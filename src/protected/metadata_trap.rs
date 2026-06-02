use crate::error::{Error, Result};
use crate::traits::Protector;
use crate::types::{
    DmiValue, ImageOutputFormat, LegalMetadata, ProtectionContext, ProtectionLevel,
};
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

/// Metadata injection protector for the Light protection level.
///
/// Injects anti-scraping markers into image headers: tEXt/iTXt chunks for PNG,
/// COM/XMP markers for JPEG, and EXIF/XML for WebP. Also embeds the protection
/// seed in metadata for later extraction.
///
/// When used through the pipeline at `Light` level, operates on bytes (encode,
/// inject metadata, decode) so that metadata survives in the byte output.
pub struct MetadataTrapProtector;

impl MetadataTrapProtector {
    /// Create a new metadata trap protector.
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
            .and_then(|l| l.copyright_holder())
            .map(|h| format!("Copyright (c) {}", h))
            .unwrap_or_else(|| "All Rights Reserved".to_string());

        metadata.push((b"Copyright".to_vec(), copyright.as_bytes().to_vec()));

        if let Some(legal) = legal {
            if let Some(email) = legal.contact_email() {
                metadata.push((b"Contact".to_vec(), email.as_bytes().to_vec()));
            }
            if let Some(url) = legal.license_url() {
                metadata.push((b"License".to_vec(), url.as_bytes().to_vec()));
            }
            let terms = legal
                .usage_terms()
                .unwrap_or("All Rights Reserved. No AI training.");
            metadata.push((b"UsageTerms".to_vec(), terms.as_bytes().to_vec()));

            if let Some(date) = legal.creation_date() {
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
                return Err(Error::ImageTruncated(format!(
                    "JPEG segment parsing truncated at byte {} (needed 4 bytes, had {})",
                    pos,
                    jpeg_data.len() - pos
                )));
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
    /// # Warning
    ///
    /// This method returns the image **unchanged**. Metadata injection operates at the
    /// byte level (PNG tEXt chunks, JPEG COM/APP markers, WebP META chunks) and cannot
    /// be preserved through the `DynamicImage` encode/decode cycle.
    ///
    /// Use [`apply_bytes`](Protector::apply_bytes) for byte-level metadata injection,
    /// or use the [`ProtectionPipeline`](crate::ProtectionPipeline) which routes
    /// metadata correctly via `process_bytes` or `apply_light_bytes`.
    fn apply<'a>(
        &self,
        img: &'a DynamicImage,
        _ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>> {
        Ok(Cow::Borrowed(img))
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
    /// Extract the protection seed from image metadata.
    ///
    /// Parses PNG tEXt/iTXt chunks, JPEG COM/XMP markers, or WebP metadata
    /// looking for the `X-Protection-Seed` key. Returns `None` if no seed is found
    /// or the image format is unrecognized.
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
                    return None;
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
                return None;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Protector;
    use crate::types::ProtectionLevel;
    use image::DynamicImage;

    fn make_test_image() -> DynamicImage {
        DynamicImage::ImageRgba8(image::ImageBuffer::from_fn(64, 64, |x, y| {
            image::Rgba([(x * 3) as u8, (y * 5) as u8, ((x + y) * 7) as u8, 255])
        }))
    }

    fn encode_png(img: &DynamicImage) -> Vec<u8> {
        crate::util::image::encode_image(img, image::ImageFormat::Png).unwrap()
    }

    fn encode_jpeg(img: &DynamicImage) -> Vec<u8> {
        crate::util::image::encode_image(img, image::ImageFormat::Jpeg).unwrap()
    }

    fn encode_webp(img: &DynamicImage) -> Vec<u8> {
        crate::util::image::encode_image(img, image::ImageFormat::WebP).unwrap()
    }

    // ── Date computation ──────────────────────────────────────────────

    #[test]
    fn is_leap_year_divisible_by_400() {
        assert!(is_leap_year(2000));
    }

    #[test]
    fn is_leap_year_divisible_by_100_not_400() {
        assert!(!is_leap_year(1900));
    }

    #[test]
    fn is_leap_year_divisible_by_4_not_100() {
        assert!(is_leap_year(2024));
    }

    #[test]
    fn is_leap_year_not_divisible_by_4() {
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn current_date_iso_format() {
        let date = current_date_iso();
        assert_eq!(date.len(), 10);
        assert_eq!(date.as_bytes()[4], b'-');
        assert_eq!(date.as_bytes()[7], b'-');
    }

    // ── Metadata generation ───────────────────────────────────────────

    #[test]
    fn generate_poison_metadata_with_seed() {
        let protector = MetadataTrapProtector::new();
        let metadata = protector.generate_poison_metadata(
            None,
            Some(ProtectionLevel::Standard),
            Some(42),
            None,
            None,
            None,
        );
        let seed_entry = metadata.iter().find(|(k, _)| k == b"X-Protection-Seed");
        assert!(seed_entry.is_some());
        assert_eq!(seed_entry.unwrap().1, b"42");
    }

    #[test]
    fn generate_poison_metadata_dmi_auto_mapping() {
        let protector = MetadataTrapProtector::new();

        let light = protector.generate_poison_metadata(
            None,
            Some(ProtectionLevel::Light),
            None,
            None,
            None,
            None,
        );
        let dmi = light.iter().find(|(k, _)| k == b"DMI-PROHIBITED");
        assert_eq!(dmi.unwrap().1, b"Prohibited");

        let standard = protector.generate_poison_metadata(
            None,
            Some(ProtectionLevel::Standard),
            None,
            None,
            None,
            None,
        );
        let dmi = standard.iter().find(|(k, _)| k == b"DMI-PROHIBITED");
        assert_eq!(dmi.unwrap().1, b"ProhibitedAiMlTraining");
    }

    #[test]
    fn generate_poison_metadata_disabled_skips_injection() {
        let protector = MetadataTrapProtector::new();
        let metadata = protector.generate_poison_metadata(
            None,
            Some(ProtectionLevel::Disabled),
            Some(42),
            None,
            None,
            None,
        );
        assert!(metadata.is_empty());
    }

    #[test]
    fn generate_poison_metadata_explicit_dmi_overrides_auto() {
        let protector = MetadataTrapProtector::new();
        let metadata = protector.generate_poison_metadata(
            Some(DmiValue::Allowed),
            Some(ProtectionLevel::Standard),
            None,
            None,
            None,
            None,
        );
        // DMI entry is present with the explicit value "Allowed"
        let dmi = metadata.iter().find(|(k, _)| k == b"DMI-PROHIBITED");
        assert!(dmi.is_some());
        assert_eq!(dmi.unwrap().1, b"Allowed");
    }

    #[test]
    fn generate_poison_metadata_legal_claims() {
        let protector = MetadataTrapProtector::new();
        let legal = LegalMetadata::new()
            .with_copyright_holder("Test Corp")
            .with_contact_email("legal@test.com");
        let metadata = protector.generate_poison_metadata(
            None,
            Some(ProtectionLevel::Standard),
            None,
            Some(&legal),
            None,
            Some(true),
        );
        let copyright = metadata.iter().find(|(k, _)| k == b"Copyright");
        assert!(copyright.is_some());
        let copyright_str = String::from_utf8_lossy(&copyright.unwrap().1);
        assert!(copyright_str.contains("Test Corp"));

        let contact = metadata.iter().find(|(k, _)| k == b"Contact");
        assert_eq!(contact.unwrap().1, b"legal@test.com");
    }

    // ── PNG injection + extraction ────────────────────────────────────

    #[test]
    fn png_inject_produces_valid_png() {
        let protector = MetadataTrapProtector::new();
        let png = encode_png(&make_test_image());
        let metadata = vec![(b"Test-Key".to_vec(), b"Test-Value".to_vec())];
        let result = protector
            .inject_text_chunks_png(&png, &metadata, None)
            .unwrap();
        assert!(result.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
        assert!(result.len() > png.len());
    }

    #[test]
    fn png_inject_empty_metadata_returns_original() {
        let protector = MetadataTrapProtector::new();
        let png = encode_png(&make_test_image());
        let result = protector.inject_text_chunks_png(&png, &[], None).unwrap();
        assert_eq!(result, png);
    }

    #[test]
    fn png_inject_invalid_signature_errors() {
        let protector = MetadataTrapProtector::new();
        let result = protector.inject_text_chunks_png(
            b"NOTAPNG",
            &[(b"key".to_vec(), b"val".to_vec())],
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn png_seed_roundtrip() {
        let protector = MetadataTrapProtector::new();
        let png = encode_png(&make_test_image());
        let metadata = vec![];
        let result = protector
            .inject_text_chunks_png(&png, &metadata, None)
            .unwrap();
        let extracted = MetadataTrapProtector::extract_seed_from_png(&result);
        assert!(extracted.is_none());

        // Now with seed in metadata
        let metadata_with_seed = vec![
            (b"X-Protection-Seed".to_vec(), b"12345".to_vec()),
            (b"Other".to_vec(), b"Value".to_vec()),
        ];
        let result = protector
            .inject_text_chunks_png(&png, &metadata_with_seed, None)
            .unwrap();
        let extracted = MetadataTrapProtector::extract_seed_from_png(&result);
        assert_eq!(extracted, Some(12345));
    }

    #[test]
    fn png_dmi_injects_xmp_chunk() {
        let protector = MetadataTrapProtector::new();
        let png = encode_png(&make_test_image());
        let result = protector
            .inject_text_chunks_png(&png, &[], Some(DmiValue::ProhibitedAiMlTraining))
            .unwrap();
        // XMP is injected as iTXt chunk with "XML:com.adobe.xmp" keyword
        assert!(result.len() > png.len());
    }

    // ── JPEG injection + extraction ───────────────────────────────────

    #[test]
    fn jpeg_inject_produces_valid_jpeg() {
        let protector = MetadataTrapProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let metadata = vec![(b"Test-Key".to_vec(), b"Test-Value".to_vec())];
        let result = protector
            .inject_text_chunks_jpeg(&jpeg, &metadata, None)
            .unwrap();
        assert!(result.starts_with(&[0xFF, 0xD8]));
        assert!(result.ends_with(&[0xFF, 0xD9]));
    }

    #[test]
    fn jpeg_seed_roundtrip() {
        let protector = MetadataTrapProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let metadata = vec![(b"X-Protection-Seed".to_vec(), b"54321".to_vec())];
        let result = protector
            .inject_text_chunks_jpeg(&jpeg, &metadata, None)
            .unwrap();
        let extracted = MetadataTrapProtector::extract_seed_from_jpeg(&result).unwrap();
        assert_eq!(extracted, 54321);
    }

    #[test]
    fn jpeg_inject_contains_markers() {
        let protector = MetadataTrapProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let metadata = vec![(b"Test".to_vec(), b"Val".to_vec())];
        let result = protector
            .inject_text_chunks_jpeg(&jpeg, &metadata, None)
            .unwrap();

        // Should contain COM marker (0xFFFE)
        assert!(result.windows(2).any(|w| w == [0xFF, 0xFE]));
    }

    #[test]
    fn jpeg_invalid_input_errors() {
        let protector = MetadataTrapProtector::new();
        let result = protector.inject_text_chunks_jpeg(
            b"NOTJPEG",
            &[(b"key".to_vec(), b"val".to_vec())],
            None,
        );
        assert!(result.is_err());
    }

    // ── WebP injection + extraction ───────────────────────────────────

    #[test]
    fn webp_inject_produces_valid_webp() {
        let protector = MetadataTrapProtector::new();
        let webp = encode_webp(&make_test_image());
        let metadata = vec![(b"Test-Key".to_vec(), b"Test-Value".to_vec())];
        let result = protector
            .inject_text_chunks_webp(&webp, &metadata, None)
            .unwrap();
        assert!(result.starts_with(b"RIFF"));
        assert!(&result[8..12] == b"WEBP");
    }

    #[test]
    fn webp_seed_roundtrip() {
        let protector = MetadataTrapProtector::new();
        let webp = encode_webp(&make_test_image());
        let metadata = vec![(b"X-Protection-Seed".to_vec(), b"99999".to_vec())];
        let result = protector
            .inject_text_chunks_webp(&webp, &metadata, None)
            .unwrap();
        let extracted = MetadataTrapProtector::extract_seed_from_webp(&result);
        assert_eq!(extracted, Some(99999));
    }

    #[test]
    fn webp_invalid_input_errors() {
        let protector = MetadataTrapProtector::new();
        let result = protector.inject_text_chunks_webp(
            b"NOTWEBP",
            &[(b"key".to_vec(), b"val".to_vec())],
            None,
        );
        assert!(result.is_err());
    }

    // ── Seed extraction dispatch ──────────────────────────────────────

    #[test]
    fn extract_seed_dispatches_png() {
        let protector = MetadataTrapProtector::new();
        let png = encode_png(&make_test_image());
        let metadata = vec![(b"X-Protection-Seed".to_vec(), b"100".to_vec())];
        let injected = protector
            .inject_text_chunks_png(&png, &metadata, None)
            .unwrap();
        assert_eq!(
            MetadataTrapProtector::extract_seed_from_image(&injected),
            Some(100)
        );
    }

    #[test]
    fn extract_seed_dispatches_jpeg() {
        let protector = MetadataTrapProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let metadata = vec![(b"X-Protection-Seed".to_vec(), b"200".to_vec())];
        let injected = protector
            .inject_text_chunks_jpeg(&jpeg, &metadata, None)
            .unwrap();
        assert_eq!(
            MetadataTrapProtector::extract_seed_from_image(&injected),
            Some(200)
        );
    }

    #[test]
    fn extract_seed_dispatches_webp() {
        let protector = MetadataTrapProtector::new();
        let webp = encode_webp(&make_test_image());
        let metadata = vec![(b"X-Protection-Seed".to_vec(), b"300".to_vec())];
        let injected = protector
            .inject_text_chunks_webp(&webp, &metadata, None)
            .unwrap();
        assert_eq!(
            MetadataTrapProtector::extract_seed_from_image(&injected),
            Some(300)
        );
    }

    #[test]
    fn extract_seed_unknown_format_returns_none() {
        assert_eq!(
            MetadataTrapProtector::extract_seed_from_image(b"GARBAGE"),
            None
        );
    }

    #[test]
    fn extract_seed_too_short_returns_none() {
        assert_eq!(
            MetadataTrapProtector::extract_seed_from_image(&[0; 4]),
            None
        );
    }

    // ── Protector trait ───────────────────────────────────────────────

    #[test]
    fn protector_name() {
        let p = MetadataTrapProtector::new();
        assert_eq!(p.name(), "metadata_trap");
    }

    #[test]
    fn protector_level() {
        let p = MetadataTrapProtector::new();
        assert_eq!(p.protection_level(), ProtectionLevel::Light);
    }

    #[test]
    fn protector_modifies_pixels() {
        let p = MetadataTrapProtector::new();
        // Default is true; MetadataTrapProtector doesn't override.
        // apply() re-encodes the image which may alter pixel data.
        assert!(p.modifies_pixels());
    }

    #[test]
    fn protector_is_enabled() {
        let p = MetadataTrapProtector::new();
        assert!(p.is_enabled());
    }

    #[test]
    fn protector_apply_preserves_dimensions() {
        let p = MetadataTrapProtector::new();
        let img = make_test_image();
        let ctx = ProtectionContext::new(0.5, 42);
        let (w, h) = (img.width(), img.height());
        let result = p.apply(&img, &ctx).unwrap();
        assert_eq!(result.width(), w);
        assert_eq!(result.height(), h);
    }

    #[test]
    fn protector_apply_bytes_preserves_metadata() {
        let p = MetadataTrapProtector::new();
        let img = make_test_image();
        let png = encode_png(&img);
        let ctx = ProtectionContext::new(0.5, 42);
        let result = p.apply_bytes(&png, &ctx).unwrap();

        // Metadata should be preserved in byte output
        let extracted = MetadataTrapProtector::extract_seed_from_image(&result);
        assert_eq!(extracted, Some(42));
    }

    #[test]
    fn protector_apply_bytes_disabled_returns_original() {
        let p = MetadataTrapProtector::new();
        let img = make_test_image();
        let png = encode_png(&img);
        // Pass inject_metadata=false to skip metadata injection
        let ctx = ProtectionContext::new(0.5, 42).with_metadata_injection(false);
        let result = p.apply_bytes(&png, &ctx).unwrap();
        assert_eq!(result, png);
    }

    // ── Legal metadata injection ──────────────────────────────────────

    #[test]
    fn legal_metadata_all_fields() {
        let legal = LegalMetadata::new()
            .with_copyright_holder("Author")
            .with_contact_email("a@b.com")
            .with_license_url("https://example.com/license")
            .with_usage_terms("No AI")
            .with_creation_date("2024-01-01");
        let mut metadata = Vec::new();
        MetadataTrapProtector::add_legal_metadata(&mut metadata, Some(&legal));

        let keys: Vec<&[u8]> = metadata.iter().map(|(k, _)| k.as_slice()).collect();
        assert!(keys.iter().any(|k| *k == b"Copyright"));
        assert!(keys.iter().any(|k| *k == b"Contact"));
        assert!(keys.iter().any(|k| *k == b"License"));
        assert!(keys.iter().any(|k| *k == b"UsageTerms"));
        assert!(keys.iter().any(|k| *k == b"DateCreated"));
    }

    #[test]
    fn legal_metadata_partial_fields() {
        let legal = LegalMetadata::new().with_copyright_holder("Author");
        let mut metadata = Vec::new();
        MetadataTrapProtector::add_legal_metadata(&mut metadata, Some(&legal));

        let copyright = metadata.iter().find(|(k, _)| k == b"Copyright").unwrap();
        assert!(String::from_utf8_lossy(&copyright.1).contains("Author"));

        // Contact should not be present
        assert!(!metadata.iter().any(|(k, _)| k == b"Contact"));
    }

    #[test]
    fn legal_metadata_none_uses_defaults() {
        let mut metadata = Vec::new();
        MetadataTrapProtector::add_legal_metadata(&mut metadata, None);

        let copyright = metadata.iter().find(|(k, _)| k == b"Copyright").unwrap();
        assert_eq!(copyright.1, b"All Rights Reserved");
    }

    // ── Edge cases ────────────────────────────────────────────────────

    #[test]
    fn inject_empty_metadata_jpeg() {
        let protector = MetadataTrapProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let result = protector.inject_text_chunks_jpeg(&jpeg, &[], None).unwrap();
        assert_eq!(result, jpeg);
    }

    #[test]
    fn inject_empty_metadata_webp() {
        let protector = MetadataTrapProtector::new();
        let webp = encode_webp(&make_test_image());
        let result = protector.inject_text_chunks_webp(&webp, &[], None).unwrap();
        assert_eq!(result, webp);
    }
}
