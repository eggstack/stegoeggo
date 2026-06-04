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

            metadata.push((b"noai".to_vec(), b"noindex".to_vec()));
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

            if let Some(constraints) = legal.ai_constraints() {
                metadata.push((b"AIConstraints".to_vec(), constraints.as_bytes().to_vec()));
            }
            if let Some(statement) = legal.web_statement_of_rights() {
                metadata.push((
                    b"WebStatementOfRights".to_vec(),
                    statement.as_bytes().to_vec(),
                ));
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

    fn generate_xmp_dmi(dmi: DmiValue, seed: Option<u64>) -> Vec<u8> {
        let property = dmi.to_iptc_property();
        let bom = "\u{feff}";
        let seed_attr = seed
            .map(|s| format!("\n             stegoeggo:ProtectionSeed=\"{}\"", s))
            .unwrap_or_default();
        let tdm_value = if dmi == DmiValue::Allowed { "0" } else { "1" };
        let xmp = format!(
            "<?xpacket begin=\"{bom}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" \
             xmlns:iptc4xmpExt=\"http://iptc.org/std/Iptc4xmpExt/2008-02-29/\" \
             xmlns:tdm=\"http://www.niso.org/schemas/tdm/\" \
             xmlns:stegoeggo=\"https://github.com/anomalyco/stegoeggo\">\n\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
             <rdf:Description rdf:about=\"\"\n\
             {property}=\"{}\"\n\
             tdm:reserve_tdm=\"{tdm_value}\"{seed_attr}/>\n\
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

    /// Generates IPTC-IIM (Information Interchange Model) data records containing DMI
    /// and optionally the protection seed.
    ///
    /// Includes:
    /// - Tag 5 (Object Name): protection seed for redundant recovery
    /// - Tag 120 (Caption/Abstract): DMI value
    ///
    /// Returns the raw IPTC record bytes (without the Photoshop resource envelope).
    fn generate_iptc_iim_dmi(dmi: DmiValue, seed: Option<u64>) -> Vec<u8> {
        let mut data = Vec::new();

        if let Some(s) = seed {
            let seed_str = s.to_string();
            data.extend_from_slice(&[0x1C, 0x02, 0x05]); // record 2, tag 5 (Object Name)
            data.extend_from_slice(&(seed_str.len() as u16).to_be_bytes());
            data.extend_from_slice(seed_str.as_bytes());
            if seed_str.len() % 2 != 0 {
                data.push(0);
            }
        }

        let dmi_str = format!("DMI: {}", dmi.as_str());
        data.extend_from_slice(&[0x1C, 0x02, 0x78]); // record 2, tag 120 (Caption/Abstract)
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
        seed: Option<u64>,
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
                    let xmp_chunk =
                        Self::create_png_xmp_chunk(&Self::generate_xmp_dmi(dmi_val, seed));
                    output.extend_from_slice(&xmp_chunk);
                }
                for (key, value) in metadata {
                    let text_chunk = Self::create_png_text_chunk(key, value);
                    output.extend_from_slice(&text_chunk);
                }
                if let Some(s) = seed {
                    let desc_value = format!("Protected image. Seed: {}", s);
                    let desc_chunk =
                        Self::create_png_text_chunk(b"Description", desc_value.as_bytes());
                    output.extend_from_slice(&desc_chunk);
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
        seed: Option<u64>,
        ctx: Option<&ProtectionContext>,
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
                    self.inject_all_dmi_markers(&mut output, dmi, metadata, seed, ctx);
                    inserted = true;
                }
                output.extend_from_slice(&jpeg_data[pos..]);
                break;
            }

            if marker == 0xDA {
                if !inserted {
                    self.inject_all_dmi_markers(&mut output, dmi, metadata, seed, ctx);
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
            let segment_end = pos
                .checked_add(2)
                .and_then(|v| v.checked_add(segment_len))
                .ok_or_else(|| {
                    Error::Metadata(format!(
                        "JPEG segment length overflow at byte {} (segment_len={})",
                        pos, segment_len
                    ))
                })?;

            if segment_end > jpeg_data.len() {
                return Err(Error::ImageTruncated(format!(
                    "JPEG segment at byte {} claims length {} but only {} bytes remain",
                    pos,
                    segment_len,
                    jpeg_data.len().saturating_sub(pos + 2)
                )));
            }

            output.extend_from_slice(&jpeg_data[pos..segment_end]);
            pos = segment_end;
        }

        if !inserted {
            self.inject_all_dmi_markers(&mut output, dmi, metadata, seed, ctx);
        }

        Ok(output)
    }

    /// Injects all DMI markers: EXIF, IPTC-IIM, XMP, structured COM, and text comments.
    /// This ensures maximum compatibility across different image processing systems.
    fn inject_all_dmi_markers(
        &self,
        output: &mut Vec<u8>,
        dmi: Option<DmiValue>,
        metadata: &[(Vec<u8>, Vec<u8>)],
        seed: Option<u64>,
        ctx: Option<&ProtectionContext>,
    ) {
        if let Some(dmi_val) = dmi {
            let exif_marker = Self::create_jpeg_exif_marker(&Self::generate_exif_dmi(dmi_val));
            output.extend_from_slice(&exif_marker);

            let iptc_marker =
                Self::create_jpeg_iptc_marker(&Self::generate_iptc_iim_dmi(dmi_val, seed));
            output.extend_from_slice(&iptc_marker);

            let xmp_marker = Self::create_jpeg_xmp_marker(&Self::generate_xmp_dmi(dmi_val, seed));
            output.extend_from_slice(&xmp_marker);
        }

        for (key, value) in metadata {
            let com_chunk = Self::create_jpeg_comment(key, value);
            output.extend_from_slice(&com_chunk);
        }

        if let Some(context) = ctx {
            let structured_com = Self::generate_structured_com_marker(dmi, seed, context);
            output.extend_from_slice(&structured_com);
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
    /// Includes a valid minimal TIFF structure so EXIF parsers can locate the tag.
    ///
    /// Structure:
    /// - APP1 marker (0xFFE1) + length
    /// - "Exif\0\0" prefix
    /// - TIFF header: byte order "II" (little-endian) + magic 42 + IFD0 offset (8)
    /// - IFD0: 1 entry for UserComment (tag 0x9286)
    /// - UserComment data: ASCII charset identifier (8 bytes) + text
    fn create_jpeg_exif_marker(exif_data: &[u8]) -> Vec<u8> {
        let tiff_prefix = b"Exif\x00\x00";

        // TIFF header: "II" (little-endian) + magic 42 + IFD0 offset
        let mut tiff_header = Vec::with_capacity(8);
        tiff_header.extend_from_slice(b"II"); // byte order
        tiff_header.extend_from_slice(&42u16.to_le_bytes()); // magic
        tiff_header.extend_from_slice(&8u32.to_le_bytes()); // IFD0 offset

        // IFD0: entry count (1) + one entry for UserComment (12 bytes)
        let mut ifd = Vec::with_capacity(14);
        ifd.extend_from_slice(&1u16.to_le_bytes()); // entry count
        ifd.extend_from_slice(&0x9286u16.to_le_bytes()); // tag: UserComment
        ifd.extend_from_slice(&2u16.to_le_bytes()); // type: ASCII (2)
        ifd.extend_from_slice(&(exif_data.len() as u32).to_le_bytes()); // count
                                                                        // Value: if <= 4 bytes, stored inline; otherwise offset from TIFF start.
                                                                        // Our data is always > 4 bytes, so use offset = 8 (header) + 2 (count) + 12 (entry) = 22
        ifd.extend_from_slice(&22u32.to_le_bytes()); // offset to data

        let total_len = tiff_prefix.len() + tiff_header.len() + ifd.len() + exif_data.len();
        let mut marker = Vec::with_capacity(4 + total_len);
        marker.push(0xFF);
        marker.push(0xE1);
        marker.extend_from_slice(&(total_len as u16).to_be_bytes());
        marker.extend_from_slice(tiff_prefix);
        marker.extend_from_slice(&tiff_header);
        marker.extend_from_slice(&ifd);
        marker.extend_from_slice(exif_data);
        marker
    }

    /// Creates an IPTC-IIM APP13 marker (0xFFED) containing DMI data.
    /// Wraps the IPTC data in a valid Photoshop resource envelope:
    /// "Photoshop 3.0\0" + Resource ID (0x0404) + Pascal string + data size + data.
    fn create_jpeg_iptc_marker(iptc_data: &[u8]) -> Vec<u8> {
        let photoshop_id = b"Photoshop 3.0\0";

        // Photoshop resource envelope
        let mut resource = Vec::new();
        resource.extend_from_slice(&0x0404u16.to_be_bytes()); // Resource ID: IPTC-IIM
        resource.push(0x00); // Pascal string length (0 = empty)
        resource.push(0x00); // padding byte (even alignment)
        resource.extend_from_slice(&(iptc_data.len() as u32).to_be_bytes()); // data size
        resource.extend_from_slice(iptc_data); // IPTC-IIM data

        let mut data = Vec::new();
        data.extend_from_slice(photoshop_id);
        data.extend_from_slice(&resource);

        let mut marker = Vec::new();
        marker.push(0xFF);
        marker.push(0xED);
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
        seed: Option<u64>,
    ) -> Result<Vec<u8>> {
        if metadata.is_empty() && dmi.is_none() {
            return Ok(webp_data.to_vec());
        }

        if webp_data.len() < 12 || &webp_data[0..4] != b"RIFF" || &webp_data[8..12] != b"WEBP" {
            return Err(Error::Metadata("Invalid WebP signature".to_string()));
        }

        let dmi_val = dmi.unwrap_or(DmiValue::Unspecified);

        // Build XMP chunk with DMI and seed embedded as XMP properties
        let xmp_chunk = Self::create_webp_xmp_chunk(&Self::generate_xmp_dmi(dmi_val, seed));

        if xmp_chunk.is_empty() {
            return Ok(webp_data.to_vec());
        }

        let exif_chunk = seed.map(Self::create_webp_exif_chunk);

        // Copy original data and append new chunks
        let extra_len = xmp_chunk.len() + exif_chunk.as_ref().map_or(0, |c| c.len());
        let new_len = webp_data.len() + extra_len;
        if new_len > u32::MAX as usize + 8 {
            return Err(Error::Metadata(
                "WebP file would exceed 4 GiB limit after metadata injection".to_string(),
            ));
        }
        let mut output = Vec::with_capacity(new_len);
        output.extend_from_slice(webp_data);
        output.extend_from_slice(&xmp_chunk);
        if let Some(exif) = exif_chunk {
            output.extend_from_slice(&exif);
        }

        // Update RIFF file size in header (bytes 4-8, little-endian)
        // RIFF size = total file size - 8 (RIFF header + size field)
        let new_riff_size = (output.len() - 8) as u32;
        output[4] = new_riff_size as u8;
        output[5] = (new_riff_size >> 8) as u8;
        output[6] = (new_riff_size >> 16) as u8;
        output[7] = (new_riff_size >> 24) as u8;

        Ok(output)
    }

    /// Creates a WebP XMP chunk with the standard `XMP ` FourCC.
    fn create_webp_xmp_chunk(xmp_data: &[u8]) -> Vec<u8> {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(b"XMP ");
        chunk.extend_from_slice(&(xmp_data.len() as u32).to_le_bytes());
        chunk.extend_from_slice(xmp_data);
        if !xmp_data.len().is_multiple_of(2) {
            chunk.push(0);
        }
        chunk
    }

    /// Creates a WebP EXIF chunk with UserComment containing the seed.
    /// Uses the standard `EXIF` FourCC.
    fn create_webp_exif_chunk(seed: u64) -> Vec<u8> {
        let comment = format!("Protection seed: {}", seed);
        let mut exif_data = Vec::new();
        exif_data.extend_from_slice(b"Exif\x00\x00");
        let mut tiff_header = Vec::with_capacity(8);
        tiff_header.extend_from_slice(b"II");
        tiff_header.extend_from_slice(&42u16.to_le_bytes());
        tiff_header.extend_from_slice(&8u32.to_le_bytes());
        let mut ifd = Vec::with_capacity(14);
        ifd.extend_from_slice(&1u16.to_le_bytes());
        ifd.extend_from_slice(&0x9286u16.to_le_bytes());
        ifd.extend_from_slice(&2u16.to_le_bytes());
        ifd.extend_from_slice(&(comment.len() as u32).to_le_bytes());
        ifd.extend_from_slice(&22u32.to_le_bytes());
        exif_data.extend_from_slice(&tiff_header);
        exif_data.extend_from_slice(&ifd);
        exif_data.extend_from_slice(comment.as_bytes());
        let mut chunk = Vec::new();
        chunk.extend_from_slice(b"EXIF");
        chunk.extend_from_slice(&(exif_data.len() as u32).to_le_bytes());
        chunk.extend_from_slice(&exif_data);
        if !exif_data.len().is_multiple_of(2) {
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

    const STRUCTURED_COM_MAGIC: &'static [u8] = b"cloakrs:v1:";

    fn generate_structured_com_marker(
        dmi: Option<DmiValue>,
        _seed: Option<u64>,
        ctx: &ProtectionContext,
    ) -> Vec<u8> {
        let mut payload = Vec::with_capacity(48);
        payload.extend_from_slice(Self::STRUCTURED_COM_MAGIC);

        payload.push(1); // version

        let level_byte = ctx.protection_level().map(|l| l.to_byte()).unwrap_or(2);
        payload.push(level_byte);

        payload.extend_from_slice(&ctx.seed().to_le_bytes());

        let intensity_val = (ctx.intensity() * 100.0) as u16;
        payload.extend_from_slice(&intensity_val.to_le_bytes());

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        payload.extend_from_slice(&now.to_le_bytes());

        let dmi_byte = dmi.map(|d| d as u8).unwrap_or(0);
        payload.push(dmi_byte);

        let checksum = Self::crc16(&payload);
        payload.extend_from_slice(&checksum.to_le_bytes());

        let mut marker = Vec::with_capacity(4 + payload.len());
        marker.push(0xFF);
        marker.push(0xFE);
        let len = (payload.len() + 2) as u16;
        marker.extend_from_slice(&len.to_be_bytes());
        marker.extend_from_slice(&payload);
        marker
    }

    fn parse_structured_com_payload(data: &[u8]) -> Option<(u64, u8, u16)> {
        if data.len() < 34 {
            return None;
        }
        if !data.starts_with(Self::STRUCTURED_COM_MAGIC) {
            return None;
        }
        let version = data[11];
        if version != 1 {
            return None;
        }
        let level = data[12];
        let seed = u64::from_le_bytes(data[13..21].try_into().ok()?);
        let intensity = u16::from_le_bytes(data[21..23].try_into().ok()?);
        let stored_checksum = u16::from_le_bytes(data[32..34].try_into().ok()?);
        let computed_checksum = Self::crc16(&data[0..32]);
        if stored_checksum != computed_checksum {
            return None;
        }
        Some((seed, level, intensity))
    }

    fn crc32(chunk_type: &[u8], data: &[u8]) -> u32 {
        let mut hasher = Crc32Hasher::new();
        hasher.update(chunk_type);
        hasher.update(data);
        !hasher.finalize()
    }

    fn crc16(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &byte in data {
            crc ^= byte as u16;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
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
    /// byte level (PNG tEXt chunks, JPEG COM/APP markers, WebP XMP chunks) and cannot
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

    fn requires_bytes_level(&self) -> bool {
        true
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
                    let value = &data[null_pos + 1..];
                    let value_str = if let Some(end) = value.iter().position(|&b| b == 0) {
                        String::from_utf8_lossy(&value[..end]).into_owned()
                    } else {
                        String::from_utf8_lossy(value).into_owned()
                    };

                    if key == b"X-Protection-Seed" {
                        if let Ok(seed) = value_str.parse() {
                            return Some(seed);
                        }
                    }

                    if key == b"Description" {
                        if let Some(seed_str) = value_str.strip_prefix("Protected image. Seed: ") {
                            if let Ok(seed) = seed_str.trim().parse() {
                                return Some(seed);
                            }
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

                if comment.starts_with(Self::STRUCTURED_COM_MAGIC) {
                    if let Some((seed, _level, _intensity)) =
                        Self::parse_structured_com_payload(comment)
                    {
                        return Some(seed);
                    }
                }

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

            if marker == 0xED {
                let seg_start = pos + 4;
                let seg_end = (seg_start + segment_len).min(jpeg_data.len());
                let seg_data = &jpeg_data[seg_start..seg_end];
                if let Some(seed) = Self::extract_seed_from_iptc(seg_data) {
                    return Some(seed);
                }
            }

            pos += 2 + segment_len;
        }
        None
    }

    fn extract_seed_from_iptc(iptc_data: &[u8]) -> Option<u64> {
        let photoshop_header = b"Photoshop 3.0\x00";
        let search_start = iptc_data
            .windows(photoshop_header.len())
            .position(|w| w == photoshop_header)
            .map(|p| p + photoshop_header.len())
            .unwrap_or(0);

        let mut i = search_start;
        while i + 5 <= iptc_data.len() {
            if iptc_data[i] != 0x1C || iptc_data[i + 1] != 0x02 {
                i += 1;
                continue;
            }
            let tag = iptc_data[i + 2];
            let data_len = u16::from_be_bytes([iptc_data[i + 3], iptc_data[i + 4]]) as usize;
            let data_start = i + 5;
            let data_end = (data_start + data_len).min(iptc_data.len());
            if tag == 0x05 {
                let data = &iptc_data[data_start..data_end];
                if let Ok(s) = std::str::from_utf8(data) {
                    if let Ok(seed) = s.trim().trim_end_matches('\0').parse::<u64>() {
                        return Some(seed);
                    }
                }
            }
            i = data_end;
            if !data_len.is_multiple_of(2) && i < iptc_data.len() {
                i += 1;
            }
        }
        None
    }

    fn extract_seed_from_webp(webp_data: &[u8]) -> Option<u64> {
        if webp_data.len() < 20 {
            return None;
        }

        let mut pos = 12;

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

            if chunk_type == b"XMP " && data_end > data_start {
                let data = &webp_data[data_start..data_end];
                if let Ok(xmp_str) = std::str::from_utf8(data) {
                    if let Some(start) = xmp_str.find("stegoeggo:ProtectionSeed=\"") {
                        let value_start = start + "stegoeggo:ProtectionSeed=\"".len();
                        if let Some(end) = xmp_str[value_start..].find('"') {
                            let value_str = &xmp_str[value_start..value_start + end];
                            if let Ok(seed) = value_str.parse::<u64>() {
                                return Some(seed);
                            }
                        }
                    }
                }
            }

            if chunk_type == b"EXIF" && data_end > data_start {
                let data = &webp_data[data_start..data_end];
                let needle = b"Protection seed: ";
                if let Some(offset) = data.windows(needle.len()).position(|w| w == needle) {
                    let start = offset + needle.len();
                    let end = data[start..]
                        .iter()
                        .position(|&b| b == b'\0' || b == b'\n')
                        .map(|p| start + p)
                        .unwrap_or(data.len());
                    if let Ok(seed_str) = std::str::from_utf8(&data[start..end]) {
                        if let Ok(seed) = seed_str.trim().parse::<u64>() {
                            return Some(seed);
                        }
                    }
                }
            }

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

        let seed = Some(ctx.seed());
        let with_metadata = match format {
            ImageOutputFormat::Png => {
                self.inject_text_chunks_png(img_bytes, &metadata, ctx.dmi_value(), seed)?
            }
            ImageOutputFormat::Jpeg => self.inject_text_chunks_jpeg(
                img_bytes,
                &metadata,
                ctx.dmi_value(),
                seed,
                Some(ctx),
            )?,
            ImageOutputFormat::WebP => {
                self.inject_text_chunks_webp(img_bytes, &metadata, ctx.dmi_value(), seed)?
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
    fn generate_poison_metadata_includes_noai() {
        let protector = MetadataTrapProtector::new();
        let metadata = protector.generate_poison_metadata(
            None,
            Some(ProtectionLevel::Light),
            Some(42),
            None,
            None,
            None,
        );
        let noai = metadata.iter().find(|(k, _)| k == b"noai");
        assert_eq!(noai.unwrap().1, b"noindex");
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

    #[test]
    fn generate_poison_metadata_ai_constraints_and_web_statement() {
        let protector = MetadataTrapProtector::new();
        let legal = LegalMetadata::new()
            .with_ai_constraints("no-training")
            .with_web_statement_of_rights("https://example.com/rights");
        let metadata = protector.generate_poison_metadata(
            None,
            Some(ProtectionLevel::Standard),
            None,
            Some(&legal),
            None,
            Some(true),
        );
        let constraints = metadata.iter().find(|(k, _)| k == b"AIConstraints");
        assert_eq!(constraints.unwrap().1, b"no-training");

        let statement = metadata.iter().find(|(k, _)| k == b"WebStatementOfRights");
        assert_eq!(statement.unwrap().1, b"https://example.com/rights");
    }

    #[test]
    fn generate_poison_metadata_omits_constraints_when_none() {
        let protector = MetadataTrapProtector::new();
        let legal = LegalMetadata::new().with_copyright_holder("Test");
        let metadata = protector.generate_poison_metadata(
            None,
            Some(ProtectionLevel::Standard),
            None,
            Some(&legal),
            None,
            Some(true),
        );
        assert!(
            !metadata.iter().any(|(k, _)| k == b"AIConstraints"),
            "AIConstraints should not be present when not set"
        );
        assert!(
            !metadata.iter().any(|(k, _)| k == b"WebStatementOfRights"),
            "WebStatementOfRights should not be present when not set"
        );
    }

    // ── PNG injection + extraction ────────────────────────────────────

    #[test]
    fn png_inject_produces_valid_png() {
        let protector = MetadataTrapProtector::new();
        let png = encode_png(&make_test_image());
        let metadata = vec![(b"Test-Key".to_vec(), b"Test-Value".to_vec())];
        let result = protector
            .inject_text_chunks_png(&png, &metadata, None, None)
            .unwrap();
        assert!(result.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
        assert!(result.len() > png.len());
    }

    #[test]
    fn png_inject_empty_metadata_returns_original() {
        let protector = MetadataTrapProtector::new();
        let png = encode_png(&make_test_image());
        let result = protector
            .inject_text_chunks_png(&png, &[], None, None)
            .unwrap();
        assert_eq!(result, png);
    }

    #[test]
    fn png_inject_invalid_signature_errors() {
        let protector = MetadataTrapProtector::new();
        let result = protector.inject_text_chunks_png(
            b"NOTAPNG",
            &[(b"key".to_vec(), b"val".to_vec())],
            None,
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
            .inject_text_chunks_png(&png, &metadata, None, None)
            .unwrap();
        let extracted = MetadataTrapProtector::extract_seed_from_png(&result);
        assert!(extracted.is_none());

        // Now with seed in metadata
        let metadata_with_seed = vec![
            (b"X-Protection-Seed".to_vec(), b"12345".to_vec()),
            (b"Other".to_vec(), b"Value".to_vec()),
        ];
        let result = protector
            .inject_text_chunks_png(&png, &metadata_with_seed, None, Some(12345))
            .unwrap();
        let extracted = MetadataTrapProtector::extract_seed_from_png(&result);
        assert_eq!(extracted, Some(12345));
    }

    #[test]
    fn png_dmi_injects_xmp_chunk() {
        let protector = MetadataTrapProtector::new();
        let png = encode_png(&make_test_image());
        let result = protector
            .inject_text_chunks_png(&png, &[], Some(DmiValue::ProhibitedAiMlTraining), None)
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
            .inject_text_chunks_jpeg(&jpeg, &metadata, None, None, None)
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
            .inject_text_chunks_jpeg(&jpeg, &metadata, None, Some(54321), None)
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
            .inject_text_chunks_jpeg(&jpeg, &metadata, None, None, None)
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
            None,
            None,
        );
        assert!(result.is_err());
    }

    /// Regression test for an out-of-bounds slice panic discovered by the fuzz
    /// harness in `fuzz/fuzz_targets/pipeline_bytes.rs`. A JPEG with an unknown
    /// marker (FF 0A) whose segment_len field (0x3100) exceeds the buffer must
    /// return an `Error::ImageTruncated` / `Error::Metadata`, not panic.
    #[test]
    fn jpeg_malformed_segment_length_does_not_panic() {
        let protector = MetadataTrapProtector::new();
        let malformed: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0x0A, // unknown marker
            0x31, 0x00, // segment_len = 0x3100 (12544), but only 3 bytes remain
            0x08, 0x00, 0x7A, 0x00, 0xEF,
        ];
        let result = protector.inject_text_chunks_jpeg(
            malformed,
            &[(b"key".to_vec(), b"val".to_vec())],
            None,
            None,
            None,
        );
        assert!(result.is_err(), "malformed segment length must error");
    }

    #[test]
    fn jpeg_segment_length_at_u16_max_does_not_panic() {
        let protector = MetadataTrapProtector::new();
        let malformed: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0xFE, // COM marker
            0xFF, 0xFF, // segment_len = 0xFFFF (max u16)
            0x00,
        ];
        let result = protector.inject_text_chunks_jpeg(
            malformed,
            &[(b"key".to_vec(), b"val".to_vec())],
            None,
            None,
            None,
        );
        assert!(result.is_err(), "segment_len near u16::MAX must error");
    }

    // ── WebP injection + extraction ───────────────────────────────────

    #[test]
    fn webp_inject_produces_valid_webp() {
        let protector = MetadataTrapProtector::new();
        let webp = encode_webp(&make_test_image());
        let metadata = vec![(b"Test-Key".to_vec(), b"Test-Value".to_vec())];
        let result = protector
            .inject_text_chunks_webp(&webp, &metadata, None, None)
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
            .inject_text_chunks_webp(&webp, &metadata, None, Some(99999))
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
            .inject_text_chunks_png(&png, &metadata, None, Some(100))
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
            .inject_text_chunks_jpeg(&jpeg, &metadata, None, Some(200), None)
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
            .inject_text_chunks_webp(&webp, &metadata, None, Some(300))
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
        let result = protector
            .inject_text_chunks_jpeg(&jpeg, &[], None, None, None)
            .unwrap();
        assert_eq!(result, jpeg);
    }

    #[test]
    fn inject_empty_metadata_webp() {
        let protector = MetadataTrapProtector::new();
        let webp = encode_webp(&make_test_image());
        let result = protector
            .inject_text_chunks_webp(&webp, &[], None, None)
            .unwrap();
        assert_eq!(result, webp);
    }

    // ── Standards compliance ──────────────────────────────────────────

    #[test]
    fn webp_xmp_chunk_uses_correct_fourcc() {
        let xmp_data = b"test xmp data";
        let chunk = MetadataTrapProtector::create_webp_xmp_chunk(xmp_data);
        assert_eq!(&chunk[0..4], b"XMP ");
    }

    #[test]
    fn webp_xmp_chunk_contains_seed_in_xmp_content() {
        let dmi = DmiValue::ProhibitedAiMlTraining;
        let seed = Some(42u64);
        let xmp = MetadataTrapProtector::generate_xmp_dmi(dmi, seed);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(xmp_str.contains("stegoeggo:ProtectionSeed=\"42\""));
    }

    #[test]
    fn webp_xmp_chunk_no_seed_omits_attribute() {
        let dmi = DmiValue::ProhibitedAiMlTraining;
        let xmp = MetadataTrapProtector::generate_xmp_dmi(dmi, None);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(!xmp_str.contains("stegoeggo:ProtectionSeed"));
    }

    #[test]
    fn jpeg_exif_marker_has_tiff_header() {
        let exif_data = b"Exif test data";
        let marker = MetadataTrapProtector::create_jpeg_exif_marker(exif_data);
        assert_eq!(marker[0], 0xFF);
        assert_eq!(marker[1], 0xE1);
        let exif_pos = marker
            .windows(6)
            .position(|w| w == b"Exif\x00\x00")
            .unwrap();
        let tiff_start = exif_pos + 6;
        assert_eq!(&marker[tiff_start..tiff_start + 2], b"II");
        assert_eq!(marker[tiff_start + 2], 42);
        assert_eq!(marker[tiff_start + 3], 0);
    }

    #[test]
    fn jpeg_exif_marker_has_ifd_usercomment() {
        let exif_data = b"DMI: ProhibitedAiMlTraining";
        let marker = MetadataTrapProtector::create_jpeg_exif_marker(exif_data);
        let exif_pos = marker
            .windows(6)
            .position(|w| w == b"Exif\x00\x00")
            .unwrap();
        let ifd_start = exif_pos + 6 + 8;
        let entry_count = u16::from_le_bytes([marker[ifd_start], marker[ifd_start + 1]]);
        assert_eq!(entry_count, 1);
        let tag = u16::from_le_bytes([marker[ifd_start + 2], marker[ifd_start + 3]]);
        assert_eq!(tag, 0x9286);
    }

    #[test]
    fn jpeg_iptc_marker_has_photoshop_resource_envelope() {
        let iptc_data = vec![0x1C, 0x02, 0x78, 0x00, 0x05, b'D', b'M', b'I', b':', b' '];
        let marker = MetadataTrapProtector::create_jpeg_iptc_marker(&iptc_data);
        assert_eq!(marker[0], 0xFF);
        assert_eq!(marker[1], 0xED);
        let photoshop_pos = marker
            .windows(14)
            .position(|w| w == b"Photoshop 3.0\x00")
            .unwrap();
        let resource_start = photoshop_pos + 14;
        let resource_id = u16::from_be_bytes([marker[resource_start], marker[resource_start + 1]]);
        assert_eq!(resource_id, 0x0404);
    }

    #[test]
    fn xmp_contains_stegoeggo_namespace() {
        let xmp = MetadataTrapProtector::generate_xmp_dmi(DmiValue::ProhibitedAiMlTraining, None);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(xmp_str.contains("xmlns:stegoeggo=\"https://github.com/anomalyco/stegoeggo\""));
    }

    #[test]
    fn xmp_contains_tdm_reservation_prohibit() {
        let xmp = MetadataTrapProtector::generate_xmp_dmi(DmiValue::ProhibitedAiMlTraining, None);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(
            xmp_str.contains("tdm:reserve_tdm=\"1\""),
            "TDM reservation should be '1' for prohibited values"
        );
        assert!(xmp_str.contains("xmlns:tdm=\"http://www.niso.org/schemas/tdm/\""));
    }

    #[test]
    fn xmp_contains_tdm_reservation_allow() {
        let xmp = MetadataTrapProtector::generate_xmp_dmi(DmiValue::Allowed, None);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(
            xmp_str.contains("tdm:reserve_tdm=\"0\""),
            "TDM reservation should be '0' for Allowed"
        );
    }

    #[test]
    fn structured_com_marker_roundtrip() {
        let protector = MetadataTrapProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let ctx =
            ProtectionContext::new(0.7, 42).with_format(crate::types::ImageOutputFormat::Jpeg);
        let metadata = protector.generate_poison_metadata(
            Some(DmiValue::ProhibitedAiMlTraining),
            Some(ProtectionLevel::Standard),
            Some(42),
            None,
            None,
            None,
        );
        let injected = protector
            .inject_text_chunks_jpeg(
                &jpeg,
                &metadata,
                Some(DmiValue::ProhibitedAiMlTraining),
                Some(42),
                Some(&ctx),
            )
            .unwrap();

        let extracted_seed = MetadataTrapProtector::extract_seed_from_image(&injected);
        assert_eq!(extracted_seed, Some(42));
    }

    #[test]
    fn structured_com_marker_parse_roundtrip() {
        let ctx =
            ProtectionContext::new(0.5, 12345).with_format(crate::types::ImageOutputFormat::Jpeg);
        let marker = MetadataTrapProtector::generate_structured_com_marker(
            Some(DmiValue::Prohibited),
            None,
            &ctx,
        );
        let payload = &marker[4..];
        let parsed = MetadataTrapProtector::parse_structured_com_payload(payload);
        assert!(parsed.is_some());
        let (seed, level, intensity) = parsed.unwrap();
        assert_eq!(seed, 12345);
        assert_eq!(level, 2);
        assert_eq!(intensity, 50);
    }

    // ── Redundant metadata injection ─────────────────────────────────

    #[test]
    fn png_seed_extractable_from_description_chunk() {
        let protector = MetadataTrapProtector::new();
        let img = make_test_image();
        let png = encode_png(&img);
        let ctx = ProtectionContext::new(0.5, 42);
        let injected = protector.inject_bytes(&png, &ctx).unwrap();

        let mut pos = 8;
        let mut found_description = false;
        while pos + 12 <= injected.len() {
            let chunk_len = u32::from_be_bytes([
                injected[pos],
                injected[pos + 1],
                injected[pos + 2],
                injected[pos + 3],
            ]) as usize;
            let chunk_type = &injected[pos + 4..pos + 8];
            if chunk_type == b"IEND" {
                break;
            }
            if chunk_type == b"tEXt" {
                let data_start = pos + 8;
                let data_end = (data_start + chunk_len).min(injected.len());
                let data = &injected[data_start..data_end];
                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    let key = &data[..null_pos];
                    if key == b"Description" {
                        found_description = true;
                        let value = &data[null_pos + 1..];
                        let value_str = String::from_utf8_lossy(value);
                        assert!(value_str.contains("Protected image. Seed: 42"));
                    }
                }
            }
            pos += 12 + chunk_len;
        }
        assert!(found_description, "Description chunk not found");
    }

    #[test]
    fn jpeg_iptc_has_object_name_seed() {
        let protector = MetadataTrapProtector::new();
        let img = make_test_image();
        let jpeg = encode_jpeg(&img);
        let ctx = ProtectionContext::new(0.5, 42).with_dmi(DmiValue::ProhibitedAiMlTraining);
        let injected = protector.inject_bytes(&jpeg, &ctx).unwrap();

        assert_eq!(
            MetadataTrapProtector::extract_seed_from_image(&injected),
            Some(42)
        );
    }

    #[test]
    fn webp_exif_chunk_has_seed() {
        let protector = MetadataTrapProtector::new();
        let img = make_test_image();
        let webp = encode_webp(&img);
        let ctx = ProtectionContext::new(0.5, 42).with_dmi(DmiValue::ProhibitedAiMlTraining);
        let injected = protector.inject_bytes(&webp, &ctx).unwrap();

        assert_eq!(
            MetadataTrapProtector::extract_seed_from_image(&injected),
            Some(42)
        );
    }

    #[test]
    fn png_seed_survives_description_only() {
        let protector = MetadataTrapProtector::new();
        let img = make_test_image();
        let png = encode_png(&img);
        let ctx = ProtectionContext::new(0.5, 99);
        let injected = protector.inject_bytes(&png, &ctx).unwrap();

        let mut output = Vec::new();
        output.extend_from_slice(&injected[0..8]);
        let mut pos = 8;
        while pos + 12 <= injected.len() {
            let chunk_len = u32::from_be_bytes([
                injected[pos],
                injected[pos + 1],
                injected[pos + 2],
                injected[pos + 3],
            ]) as usize;
            let chunk_type = &injected[pos + 4..pos + 8];
            let keep = chunk_type == b"IEND"
                || chunk_type == b"tEXt" && {
                    let data_start = pos + 8;
                    let data_end = (data_start + chunk_len).min(injected.len());
                    let data = &injected[data_start..data_end];
                    if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                        &data[..null_pos] == b"Description"
                    } else {
                        false
                    }
                };
            if chunk_type == b"IEND" {
                let text_chunk = MetadataTrapProtector::create_png_text_chunk(
                    b"Description",
                    b"Protected image. Seed: 99",
                );
                output.extend_from_slice(&text_chunk);
            }
            if keep || chunk_type == b"IEND" {
                output.extend_from_slice(&injected[pos..pos + 12 + chunk_len]);
            }
            pos += 12 + chunk_len;
        }
        assert_eq!(
            MetadataTrapProtector::extract_seed_from_image(&output),
            Some(99)
        );
    }
}
