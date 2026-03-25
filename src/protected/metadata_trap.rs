use crate::error::{Error, Result};
use crate::traits::Protector;
use crate::types::{
    DmiValue, ImageOutputFormat, LegalMetadata, ProtectionContext, ProtectionLevel,
};
use crate::util::image::encode_image;
use crc32fast::Hasher as Crc32Hasher;
use image::DynamicImage;
use std::borrow::Cow;

/// Manual date computation from Unix epoch.
/// Intentionally avoids adding a `chrono` dependency for this simple use case.
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn current_date_iso() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut remaining_days = now / 86400;
    let mut year = 1970i32;

    loop {
        let year_days: u64 = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < year_days {
            break;
        }
        remaining_days -= year_days;
        year += 1;
    }

    let month_lengths = [
        31u64,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut month = 0usize;
    for &days_in_month in &month_lengths {
        if remaining_days < days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    let day = remaining_days + 1;
    format!("{:04}-{:02}-{:02}", year, month + 1, day)
}

pub struct MetadataTrapProtector;

impl MetadataTrapProtector {
    pub fn new() -> Self {
        Self
    }

    fn generate_poison_metadata(
        &self,
        dmi_value: Option<DmiValue>,
        protection_level: Option<ProtectionLevel>,
        seed: Option<u64>,
        legal: Option<&LegalMetadata>,
        inject_metadata: Option<bool>,
        inject_legal_claims: Option<bool>,
    ) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut metadata = Vec::new();

        let should_inject_metadata =
            inject_metadata.unwrap_or(!matches!(protection_level, Some(ProtectionLevel::Disabled)));

        let should_inject_claims = inject_legal_claims.unwrap_or(false);

        if should_inject_metadata {
            if let Some(s) = seed {
                metadata.push((
                    b"X-Protection-Seed".to_vec(),
                    s.to_string().as_bytes().to_vec(),
                ));
            }

            let dmi = dmi_value.or_else(|| {
                protection_level.and_then(|level| match level {
                    ProtectionLevel::Light => Some(DmiValue::Prohibited),
                    ProtectionLevel::Standard => Some(DmiValue::ProhibitedAiMlTraining),
                    ProtectionLevel::Enhanced => Some(DmiValue::ProhibitedGenAiMlTraining),
                    ProtectionLevel::Strong => Some(DmiValue::Prohibited),
                    _ => None,
                })
            });

            if let Some(dmi) = dmi {
                if dmi != DmiValue::Unspecified {
                    metadata.push((b"DMI-PROHIBITED".to_vec(), dmi.as_str().as_bytes().to_vec()));
                }
            }
        }

        if should_inject_claims {
            Self::add_legal_metadata(&mut metadata, legal);
        }

        metadata
    }

    fn add_legal_metadata(metadata: &mut Vec<(Vec<u8>, Vec<u8>)>, legal: Option<&LegalMetadata>) {
        let copyright = legal
            .and_then(|l| l.copyright_holder.as_ref())
            .map(|h| format!("Copyright (c) {}", h))
            .unwrap_or_else(|| "All Rights Reserved".to_string());

        metadata.push((b"Copyright".to_vec(), copyright.as_bytes().to_vec()));

        if let Some(legal) = legal {
            if let Some(ref email) = legal.contact_email {
                metadata.push((b"Contact".to_vec(), email.as_bytes().to_vec()));
            }
            if let Some(ref url) = legal.license_url {
                metadata.push((b"License".to_vec(), url.as_bytes().to_vec()));
            }
            let terms = legal
                .usage_terms
                .as_deref()
                .unwrap_or("All Rights Reserved. No AI training.");
            metadata.push((b"UsageTerms".to_vec(), terms.as_bytes().to_vec()));

            if let Some(ref date) = legal.creation_date {
                metadata.push((b"DateCreated".to_vec(), date.as_bytes().to_vec()));
            } else {
                let now = current_date_iso();
                metadata.push((b"DateCreated".to_vec(), now.as_bytes().to_vec()));
            }
        } else {
            metadata.push((
                b"UsageTerms".to_vec(),
                b"All Rights Reserved. No AI training.".to_vec(),
            ));
            let now = current_date_iso();
            metadata.push((b"DateCreated".to_vec(), now.as_bytes().to_vec()));
        }
    }

    fn generate_xmp_dmi(dmi: DmiValue) -> Vec<u8> {
        let property = dmi.to_iptc_property();
        let bom = "\u{feff}";
        let xmp = format!(
            "<?xpacket begin=\"{bom}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" xmlns:iptc4xmpExt=\"http://iptc.org/std/Iptc4xmpExt/2008-02-29/\">\n\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
             <rdf:Description rdf:about=\"\"\n\
             {property}=\"{}\"/>\n\
             </rdf:RDF>\n\
             </x:xmpmeta>\n\
             <?xpacket end=\"w\"?>",
            dmi.as_str()
        );
        xmp.into_bytes()
    }

    /// Generates EXIF UserComment tag (0x9286) containing the DMI value.
    /// EXIF UserComment is a common location for AI/ML opt-out markers.
    /// Format: ASCII charset (8 bytes) + null-terminated text.
    fn generate_exif_dmi(dmi: DmiValue) -> Vec<u8> {
        let comment = format!("DMI: {}", dmi.as_str());
        let mut data = Vec::new();
        data.extend_from_slice(b"ASCII\x00\x00\x00");
        data.extend_from_slice(comment.as_bytes());
        data.push(0);
        data
    }

    /// Generates IPTC-IIM (Information Interchange Model) data record containing DMI.
    /// Uses IPTC Application Record tag 120 (Caption/Abstract) with the DMI value encoded.
    /// Includes proper record/entry header structure for maximum compatibility.
    fn generate_iptc_iim_dmi(dmi: DmiValue) -> Vec<u8> {
        let dmi_str = format!("DMI: {}", dmi.as_str());
        let mut data = vec![0x1C, 0x02, 0x78];
        data.extend_from_slice(&(dmi_str.len() as u16).to_be_bytes());
        data.extend_from_slice(dmi_str.as_bytes());
        if dmi_str.len() % 2 != 0 {
            data.push(0);
        }
        data
    }

    fn inject_text_chunks_png(
        &self,
        png_data: &[u8],
        metadata: &[(Vec<u8>, Vec<u8>)],
        dmi: Option<DmiValue>,
    ) -> Result<Vec<u8>> {
        if metadata.is_empty() && dmi.is_none() {
            return Ok(png_data.to_vec());
        }

        if png_data.len() < 8 || &png_data[0..8] != b"\x89PNG\r\n\x1a\n" {
            return Err(Error::Metadata("Invalid PNG signature".to_string()));
        }

        let mut output = Vec::with_capacity(png_data.len() + 1000 * metadata.len() + 500);
        output.extend_from_slice(&png_data[0..8]);

        let mut pos = 8;

        while pos + 12 <= png_data.len() {
            let chunk_len = u32::from_be_bytes([
                png_data[pos],
                png_data[pos + 1],
                png_data[pos + 2],
                png_data[pos + 3],
            ]) as usize;
            let chunk_type = &png_data[pos + 4..pos + 8];

            if chunk_type == b"IEND" {
                if let Some(dmi_val) = dmi {
                    let xmp_chunk = Self::create_png_xmp_chunk(&Self::generate_xmp_dmi(dmi_val));
                    output.extend_from_slice(&xmp_chunk);
                }
                for (key, value) in metadata {
                    let text_chunk = Self::create_png_text_chunk(key, value);
                    output.extend_from_slice(&text_chunk);
                }
            }

            output.extend_from_slice(&png_data[pos..pos + 12 + chunk_len]);
            pos += 12 + chunk_len;
        }

        Ok(output)
    }

    fn create_png_xmp_chunk(xmp_data: &[u8]) -> Vec<u8> {
        // PNG iTXt chunk format (spec-compliant):
        //   keyword\0 compression_flag(1) compression_method(1)
        //   language_tag\0 translated_keyword\0 text
        let keyword = b"XML:com.adobe.xmp";
        let mut chunk_data = Vec::new();
        chunk_data.extend_from_slice(keyword);
        chunk_data.push(0); // null separator after keyword
        chunk_data.push(0); // compression flag: 0 = uncompressed
        chunk_data.push(0); // compression method: 0 (deflate, unused when flag=0)
        chunk_data.push(0); // null separator (empty language tag)
        chunk_data.push(0); // null separator (empty translated keyword)
        chunk_data.extend_from_slice(xmp_data);

        let len = chunk_data.len() as u32;
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&len.to_be_bytes());
        chunk.extend_from_slice(b"iTXt");
        chunk.extend_from_slice(&chunk_data);
        let crc = Self::crc32(b"iTXt", &chunk_data);
        chunk.extend_from_slice(&crc.to_be_bytes());
        chunk
    }

    fn inject_text_chunks_jpeg(
        &self,
        jpeg_data: &[u8],
        metadata: &[(Vec<u8>, Vec<u8>)],
        dmi: Option<DmiValue>,
    ) -> Result<Vec<u8>> {
        if metadata.is_empty() && dmi.is_none() {
            return Ok(jpeg_data.to_vec());
        }

        if jpeg_data.len() < 2 || jpeg_data[0] != 0xFF || jpeg_data[1] != 0xD8 {
            return Err(Error::Metadata("Invalid JPEG signature".to_string()));
        }

        let mut output = Vec::with_capacity(jpeg_data.len() + 500 * metadata.len() + 500);
        output.extend_from_slice(&jpeg_data[0..2]);

        let mut pos = 2;
        let mut inserted = false;

        while pos + 2 <= jpeg_data.len() {
            if jpeg_data[pos] != 0xFF {
                pos += 1;
                continue;
            }

            let marker = jpeg_data[pos + 1];

            if marker == 0xD9 {
                if !inserted {
                    self.inject_all_dmi_markers(&mut output, dmi, metadata);
                    inserted = true;
                }
                output.extend_from_slice(&jpeg_data[pos..]);
                break;
            }

            if marker == 0xDA {
                if !inserted {
                    self.inject_all_dmi_markers(&mut output, dmi, metadata);
                    inserted = true;
                }
                output.extend_from_slice(&jpeg_data[pos..]);
                break;
            }

            if marker == 0x00 {
                pos += 1;
                continue;
            }

            if pos + 4 > jpeg_data.len() {
                // Truncated JPEG — emit what we have
                output.extend_from_slice(&jpeg_data[pos..]);
                break;
            }

            let segment_len = u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;

            output.extend_from_slice(&jpeg_data[pos..pos + 2 + segment_len]);
            pos += 2 + segment_len;
        }

        if !inserted {
            self.inject_all_dmi_markers(&mut output, dmi, metadata);
        }

        Ok(output)
    }

    /// Injects all DMI markers: EXIF, IPTC-IIM, XMP, and text comments.
    /// This ensures maximum compatibility across different image processing systems.
    fn inject_all_dmi_markers(
        &self,
        output: &mut Vec<u8>,
        dmi: Option<DmiValue>,
        metadata: &[(Vec<u8>, Vec<u8>)],
    ) {
        if let Some(dmi_val) = dmi {
            let exif_marker = Self::create_jpeg_exif_marker(&Self::generate_exif_dmi(dmi_val));
            output.extend_from_slice(&exif_marker);

            let iptc_marker = Self::create_jpeg_iptc_marker(&Self::generate_iptc_iim_dmi(dmi_val));
            output.extend_from_slice(&iptc_marker);

            let xmp_marker = Self::create_jpeg_xmp_marker(&Self::generate_xmp_dmi(dmi_val));
            output.extend_from_slice(&xmp_marker);
        }

        for (key, value) in metadata {
            let com_chunk = Self::create_jpeg_comment(key, value);
            output.extend_from_slice(&com_chunk);
        }
    }

    fn create_jpeg_xmp_marker(xmp_data: &[u8]) -> Vec<u8> {
        let mut marker = Vec::new();
        marker.push(0xFF);
        marker.push(0xE1);
        let len = (xmp_data.len() + 2) as u16;
        marker.extend_from_slice(&len.to_be_bytes());
        marker.extend_from_slice(xmp_data);
        marker
    }

    /// Creates an EXIF APP1 marker (0xFFE1) with DMI data in UserComment tag.
    /// Includes the standard "Exif\0\0" TIFF header prefix that JPEG parsers expect.
    fn create_jpeg_exif_marker(exif_data: &[u8]) -> Vec<u8> {
        let tiff_prefix = b"Exif\x00\x00";
        let mut marker = Vec::new();
        marker.push(0xFF);
        marker.push(0xE1);
        let len = (tiff_prefix.len() + exif_data.len() + 2) as u16;
        marker.extend_from_slice(&len.to_be_bytes());
        marker.extend_from_slice(tiff_prefix);
        marker.extend_from_slice(exif_data);
        marker
    }

    /// Creates an IPTC-IIM APP13 marker (0xFFED) containing DMI data.
    /// Uses Photoshop 3.0 identifier for compatibility with Photoshop and other tools.
    fn create_jpeg_iptc_marker(iptc_data: &[u8]) -> Vec<u8> {
        let photoshop_id = b"Photoshop 3.0\0";
        let mut marker = Vec::new();
        marker.push(0xFF);
        marker.push(0xED);
        let mut data = Vec::new();
        data.extend_from_slice(photoshop_id);
        data.extend_from_slice(iptc_data);
        let len = (data.len() + 2) as u16;
        marker.extend_from_slice(&len.to_be_bytes());
        marker.extend_from_slice(&data);
        marker
    }

    fn inject_text_chunks_webp(
        &self,
        webp_data: &[u8],
        metadata: &[(Vec<u8>, Vec<u8>)],
        dmi: Option<DmiValue>,
    ) -> Result<Vec<u8>> {
        if metadata.is_empty() && dmi.is_none() {
            return Ok(webp_data.to_vec());
        }

        if webp_data.len() < 12 || &webp_data[0..4] != b"RIFF" || &webp_data[8..12] != b"WEBP" {
            return Err(Error::Metadata("Invalid WebP signature".to_string()));
        }

        // Calculate size of new chunks to append
        let mut new_chunks = Vec::new();
        if let Some(dmi_val) = dmi {
            let xmp_chunk = Self::create_webp_xmp_chunk(&Self::generate_xmp_dmi(dmi_val));
            new_chunks.extend_from_slice(&xmp_chunk);
        }
        for (key, value) in metadata {
            let meta_chunk = Self::create_webp_metadata_chunk(key, value);
            new_chunks.extend_from_slice(&meta_chunk);
        }

        if new_chunks.is_empty() {
            return Ok(webp_data.to_vec());
        }

        // Copy original data and append new chunks
        let mut output = Vec::with_capacity(webp_data.len() + new_chunks.len());
        output.extend_from_slice(webp_data);
        output.extend_from_slice(&new_chunks);

        // Update RIFF file size in header (bytes 4-8, little-endian)
        // RIFF size = total file size - 8 (RIFF header + size field)
        let new_riff_size = (output.len() - 8) as u32;
        output[4] = new_riff_size as u8;
        output[5] = (new_riff_size >> 8) as u8;
        output[6] = (new_riff_size >> 16) as u8;
        output[7] = (new_riff_size >> 24) as u8;

        Ok(output)
    }

    fn create_webp_xmp_chunk(xmp_data: &[u8]) -> Vec<u8> {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(b"XML ");
        chunk.extend_from_slice(&(xmp_data.len() as u32).to_le_bytes());
        chunk.extend_from_slice(xmp_data);
        if !xmp_data.len().is_multiple_of(2) {
            chunk.push(0);
        }
        chunk
    }

    fn create_png_text_chunk(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(key);
        data.push(0);
        data.extend_from_slice(value);

        let len = data.len() as u32;
        let mut chunk = Vec::new();

        chunk.extend_from_slice(&len.to_be_bytes());
        chunk.extend_from_slice(b"tEXt");
        chunk.extend_from_slice(&data);

        let crc = Self::crc32(b"tEXt", &data);
        chunk.extend_from_slice(&crc.to_be_bytes());

        chunk
    }

    fn create_jpeg_comment(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut comment = Vec::new();
        comment.extend_from_slice(key);
        comment.extend_from_slice(b": ");
        comment.extend_from_slice(value);

        let len = (comment.len() + 2) as u16;

        let mut chunk = Vec::new();
        chunk.push(0xFF);
        chunk.push(0xFE);
        chunk.extend_from_slice(&len.to_be_bytes());
        chunk.extend_from_slice(&comment);

        chunk
    }

    fn create_webp_metadata_chunk(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(key);
        data.push(0);
        data.extend_from_slice(value);

        let len = data.len() as u32;

        let mut full_chunk = Vec::new();
        full_chunk.extend_from_slice(b"META");
        full_chunk.extend_from_slice(&len.to_le_bytes());
        full_chunk.extend_from_slice(&data);

        if !data.len().is_multiple_of(2) {
            full_chunk.push(0);
        }

        full_chunk
    }

    fn crc32(chunk_type: &[u8], data: &[u8]) -> u32 {
        let mut hasher = Crc32Hasher::new();
        hasher.update(chunk_type);
        hasher.update(data);
        !hasher.finalize()
    }
}

impl Default for MetadataTrapProtector {
    fn default() -> Self {
        Self::new()
    }
}

impl Protector for MetadataTrapProtector {
    fn apply<'a>(
        &self,
        img: &'a DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>> {
        let metadata = self.generate_poison_metadata(
            ctx.dmi_value(),
            ctx.protection_level(),
            Some(ctx.seed()),
            ctx.legal_metadata(),
            ctx.inject_metadata(),
            ctx.inject_legal_claims(),
        );

        if metadata.is_empty() {
            return Ok(Cow::Borrowed(img));
        }

        let format = ctx
            .input_format()
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        let encoded = encode_image(img, format.to_image_format())?;

        let with_metadata = match format {
            ImageOutputFormat::Png => {
                self.inject_text_chunks_png(&encoded, &metadata, ctx.dmi_value())?
            }
            ImageOutputFormat::Jpeg => {
                self.inject_text_chunks_jpeg(&encoded, &metadata, ctx.dmi_value())?
            }
            ImageOutputFormat::WebP => {
                self.inject_text_chunks_webp(&encoded, &metadata, ctx.dmi_value())?
            }
        };

        let result_img = image::load_from_memory(&with_metadata)?;

        Ok(Cow::Owned(result_img))
    }

    fn apply_bytes(&self, img_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>> {
        self.inject_bytes(img_bytes, ctx)
    }

    fn name(&self) -> &'static str {
        "metadata_trap"
    }

    fn protection_level(&self) -> ProtectionLevel {
        ProtectionLevel::Light
    }

    fn estimated_latency_ms(&self) -> u32 {
        2
    }
}

impl MetadataTrapProtector {
    pub fn extract_seed_from_image(img_bytes: &[u8]) -> Option<u64> {
        if img_bytes.len() < 8 {
            return None;
        }

        if img_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            Self::extract_seed_from_png(img_bytes)
        } else if img_bytes.starts_with(&[0xFF, 0xD8]) {
            Self::extract_seed_from_jpeg(img_bytes)
        } else if img_bytes.len() >= 12
            && &img_bytes[0..4] == b"RIFF"
            && &img_bytes[8..12] == b"WEBP"
        {
            Self::extract_seed_from_webp(img_bytes)
        } else {
            None
        }
    }

    fn extract_seed_from_png(png_data: &[u8]) -> Option<u64> {
        let mut pos = 8;
        while pos + 12 <= png_data.len() {
            let chunk_len = u32::from_be_bytes([
                png_data[pos],
                png_data[pos + 1],
                png_data[pos + 2],
                png_data[pos + 3],
            ]) as usize;
            let chunk_type = &png_data[pos + 4..pos + 8];

            if chunk_type == b"IEND" {
                break;
            }

            if chunk_type == b"tEXt" || chunk_type == b"iTXt" {
                let data_start = pos + 8;
                let data_end = (data_start + chunk_len).min(png_data.len());
                let data = &png_data[data_start..data_end];

                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    let key = &data[..null_pos];
                    if key == b"X-Protection-Seed" {
                        let value = &data[null_pos + 1..];
                        if let Some(end) = value.iter().position(|&b| b == 0) {
                            let value_str = String::from_utf8_lossy(&value[..end]);
                            return value_str.parse().ok();
                        } else {
                            let value_str = String::from_utf8_lossy(value);
                            return value_str.parse().ok();
                        }
                    }
                }
            }

            pos += 12 + chunk_len;
        }
        None
    }

    fn extract_seed_from_jpeg(jpeg_data: &[u8]) -> Option<u64> {
        let mut pos = 2;
        while pos + 2 <= jpeg_data.len() {
            if jpeg_data[pos] != 0xFF {
                pos += 1;
                continue;
            }

            let marker = jpeg_data[pos + 1];

            if marker == 0xD9 || marker == 0xDA {
                break;
            }

            if marker == 0x00 {
                pos += 1;
                continue;
            }

            if marker == 0xFE {
                if pos + 4 > jpeg_data.len() {
                    break;
                }
                let comment_len =
                    u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
                if comment_len < 2 {
                    pos += 2 + comment_len;
                    continue;
                }
                let comment_start = pos + 4;
                let comment_end = (comment_start + comment_len - 2).min(jpeg_data.len());
                let comment = &jpeg_data[comment_start..comment_end];

                if let Ok(comment_str) = String::from_utf8(comment.to_vec()) {
                    if let Some(seed_part) = comment_str.strip_prefix("X-Protection-Seed: ") {
                        return seed_part.trim().parse().ok();
                    }
                }
                pos += 2 + comment_len;
                continue;
            }

            if pos + 4 > jpeg_data.len() {
                break;
            }
            let segment_len = u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
            pos += 2 + segment_len;
        }
        None
    }

    fn extract_seed_from_webp(webp_data: &[u8]) -> Option<u64> {
        // WebP RIFF container: "RIFF" + u32_le file_size + "WEBP" + chunks...
        // Each chunk: FourCC (4 bytes) + u32_le chunk_size + data + optional padding byte
        if webp_data.len() < 20 {
            return None;
        }

        let mut pos = 12; // Skip RIFF header: "RIFF"(4) + size(4) + "WEBP"(4)

        while pos + 8 <= webp_data.len() {
            let chunk_type = &webp_data[pos..pos + 4];
            let chunk_size = u32::from_le_bytes([
                webp_data[pos + 4],
                webp_data[pos + 5],
                webp_data[pos + 6],
                webp_data[pos + 7],
            ]) as usize;

            let data_start = pos + 8;
            let data_end = (data_start + chunk_size).min(webp_data.len());

            if chunk_type == b"META" && data_end > data_start {
                let data = &webp_data[data_start..data_end];

                // Data format: key\0value
                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    let key = &data[..null_pos];
                    if key == b"X-Protection-Seed" {
                        let value = &data[null_pos + 1..];
                        // Value may be null-terminated
                        let value_end = value.iter().position(|&b| b == 0).unwrap_or(value.len());
                        let value_str = String::from_utf8_lossy(&value[..value_end]);
                        return value_str.parse().ok();
                    }
                }
            }

            // Advance to next chunk (chunks are 2-byte aligned)
            pos = data_start + chunk_size;
            if !chunk_size.is_multiple_of(2) {
                pos += 1;
            }
        }

        None
    }

    pub fn inject_bytes(&self, img_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>> {
        let metadata = self.generate_poison_metadata(
            ctx.dmi_value(),
            ctx.protection_level(),
            Some(ctx.seed()),
            ctx.legal_metadata(),
            ctx.inject_metadata(),
            ctx.inject_legal_claims(),
        );

        if metadata.is_empty() {
            return Ok(img_bytes.to_vec());
        }

        let format = ctx
            .output_format()
            .or(ctx.input_format())
            .unwrap_or_else(|| {
                ImageOutputFormat::from_magic_bytes(img_bytes)
                    .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT)
            });

        let with_metadata = match format {
            ImageOutputFormat::Png => {
                self.inject_text_chunks_png(img_bytes, &metadata, ctx.dmi_value())?
            }
            ImageOutputFormat::Jpeg => {
                self.inject_text_chunks_jpeg(img_bytes, &metadata, ctx.dmi_value())?
            }
            ImageOutputFormat::WebP => {
                self.inject_text_chunks_webp(img_bytes, &metadata, ctx.dmi_value())?
            }
        };

        Ok(with_metadata)
    }
}
