use crate::error::{Error, Result};
use crate::traits::Protector;
use crate::types::{
    DmiValue, ImageOutputFormat, LegalMetadata, MetadataUpdatePolicy, ProtectionContext,
    ProtectionLevel, RightsNotice, PLUS_DATA_MINING_PROPERTY, PLUS_NAMESPACE,
};
use crc32fast::Hasher as Crc32Hasher;
use image::DynamicImage;
use std::borrow::Cow;

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Manual date computation from Unix epoch.
/// Intentionally avoids adding a `chrono` dependency for this simple use case.
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
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

pub(crate) fn current_timestamp_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let secs_in_day = 86400u64;
    let mut remaining_days = now / secs_in_day;
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
    let day_secs = now % secs_in_day;
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month + 1,
        day,
        hours,
        minutes,
        seconds
    )
}

/// Metadata injection protector for the Light protection level.
///
/// Injects rights-reservation markers into image headers: tEXt/iTXt chunks for PNG,
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

    fn should_inject_metadata(
        inject_metadata: Option<bool>,
        protection_level: Option<ProtectionLevel>,
    ) -> bool {
        inject_metadata.unwrap_or(!matches!(protection_level, Some(ProtectionLevel::Disabled)))
    }

    #[allow(dead_code)]
    fn resolved_dmi(
        dmi_value: Option<DmiValue>,
        protection_level: Option<ProtectionLevel>,
        inject_metadata: Option<bool>,
    ) -> Option<DmiValue> {
        if !Self::should_inject_metadata(inject_metadata, protection_level) {
            return None;
        }

        let dmi = dmi_value.or_else(|| {
            protection_level.and_then(|level| match level {
                ProtectionLevel::Light => Some(DmiValue::Prohibited),
                ProtectionLevel::Standard => Some(DmiValue::ProhibitedAiMlTraining),
                _ => None,
            })
        });

        dmi.filter(|value| *value != DmiValue::Unspecified)
    }

    #[allow(dead_code)]
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
            Self::should_inject_metadata(inject_metadata, protection_level);

        let should_inject_claims = inject_legal_claims.unwrap_or(legal.is_some());

        if should_inject_metadata {
            if let Some(s) = seed {
                metadata.push((
                    b"X-Protection-Seed".to_vec(),
                    s.to_string().as_bytes().to_vec(),
                ));
            }

            if let Some(dmi) = Self::resolved_dmi(dmi_value, protection_level, inject_metadata) {
                metadata.push((b"DMI-PROHIBITED".to_vec(), dmi.as_str().as_bytes().to_vec()));
            }

            metadata.push((b"noai".to_vec(), b"noindex".to_vec()));
        }

        if should_inject_claims && legal.is_some() {
            Self::add_legal_metadata(&mut metadata, legal);
        }

        metadata
    }

    #[allow(dead_code)]
    fn add_legal_metadata(metadata: &mut Vec<(Vec<u8>, Vec<u8>)>, legal: Option<&LegalMetadata>) {
        let Some(legal) = legal else {
            return;
        };

        if let Some(holder) = legal.copyright_holder() {
            let copyright = if holder.contains("Copyright") {
                holder.to_string()
            } else {
                format!("Copyright (c) {}", holder)
            };
            metadata.push((b"Copyright".to_vec(), copyright.as_bytes().to_vec()));
        }

        if let Some(email) = legal.contact_email() {
            metadata.push((b"Contact".to_vec(), email.as_bytes().to_vec()));
        }
        if let Some(url) = legal.license_url() {
            metadata.push((b"License".to_vec(), url.as_bytes().to_vec()));
        }
        if let Some(terms) = legal.usage_terms() {
            metadata.push((b"UsageTerms".to_vec(), terms.as_bytes().to_vec()));
        }
        if let Some(date) = legal.creation_date() {
            metadata.push((b"DateCreated".to_vec(), date.as_bytes().to_vec()));
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
        if let Some(creator_name) = legal.creator() {
            metadata.push((b"Creator".to_vec(), creator_name.as_bytes().to_vec()));
        }
        if let Some(line) = legal.credit_line() {
            metadata.push((b"CreditLine".to_vec(), line.as_bytes().to_vec()));
        }
        if let Some(owner) = legal.copyright_owner() {
            metadata.push((b"CopyrightOwner".to_vec(), owner.as_bytes().to_vec()));
        }
        if let Some(name) = legal.licensor_name() {
            metadata.push((b"LicensorName".to_vec(), name.as_bytes().to_vec()));
        }
        if let Some(email) = legal.licensor_email() {
            metadata.push((b"LicensorEmail".to_vec(), email.as_bytes().to_vec()));
        }
        if let Some(url) = legal.licensor_url() {
            metadata.push((b"LicensorURL".to_vec(), url.as_bytes().to_vec()));
        }
        if let Some(date) = legal.metadata_date() {
            metadata.push((b"MetadataDate".to_vec(), date.as_bytes().to_vec()));
        }
        if let Some(ts) = legal.notice_applied_at() {
            metadata.push((b"NoticeAppliedAt".to_vec(), ts.as_bytes().to_vec()));
        }
    }

    fn generate_poison_metadata_from_notice(
        &self,
        notice: &RightsNotice,
        should_inject_metadata: bool,
        inject_legal_claims: Option<bool>,
    ) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut metadata = Vec::new();

        if should_inject_metadata {
            if let Some(s) = notice.seed() {
                metadata.push((
                    b"X-Protection-Seed".to_vec(),
                    s.to_string().as_bytes().to_vec(),
                ));
            }

            if let Some(dmi) = notice.dmi() {
                metadata.push((b"DMI-PROHIBITED".to_vec(), dmi.as_str().as_bytes().to_vec()));
            }

            metadata.push((b"noai".to_vec(), b"noindex".to_vec()));
        }

        let should_inject_claims = inject_legal_claims.unwrap_or(notice.has_legal_content());
        if should_inject_claims && notice.has_legal_content() {
            Self::add_legal_metadata_from_notice(&mut metadata, notice);
        }

        metadata
    }

    fn add_legal_metadata_from_notice(
        metadata: &mut Vec<(Vec<u8>, Vec<u8>)>,
        notice: &RightsNotice,
    ) {
        if let Some(holder) = notice.copyright_holder() {
            let copyright = if holder.contains("Copyright") {
                holder.to_string()
            } else {
                format!("Copyright (c) {}", holder)
            };
            metadata.push((b"Copyright".to_vec(), copyright.as_bytes().to_vec()));
        }

        if let Some(email) = notice.contact_email() {
            metadata.push((b"Contact".to_vec(), email.as_bytes().to_vec()));
        }
        if let Some(url) = notice.license_url() {
            metadata.push((b"License".to_vec(), url.as_bytes().to_vec()));
        }
        if let Some(terms) = notice.usage_terms() {
            metadata.push((b"UsageTerms".to_vec(), terms.as_bytes().to_vec()));
        }
        if let Some(date) = notice.creation_date() {
            metadata.push((b"DateCreated".to_vec(), date.as_bytes().to_vec()));
        }
        if let Some(constraints) = notice.ai_constraints() {
            metadata.push((b"AIConstraints".to_vec(), constraints.as_bytes().to_vec()));
        }
        if let Some(statement) = notice.web_statement_of_rights() {
            metadata.push((
                b"WebStatementOfRights".to_vec(),
                statement.as_bytes().to_vec(),
            ));
        }
        if let Some(creator_name) = notice.creator() {
            metadata.push((b"Creator".to_vec(), creator_name.as_bytes().to_vec()));
        }
        if let Some(line) = notice.credit_line() {
            metadata.push((b"CreditLine".to_vec(), line.as_bytes().to_vec()));
        }
        if let Some(owner) = notice.copyright_owner() {
            metadata.push((b"CopyrightOwner".to_vec(), owner.as_bytes().to_vec()));
        }
        if let Some(name) = notice.licensor_name() {
            metadata.push((b"LicensorName".to_vec(), name.as_bytes().to_vec()));
        }
        if let Some(email) = notice.licensor_email() {
            metadata.push((b"LicensorEmail".to_vec(), email.as_bytes().to_vec()));
        }
        if let Some(url) = notice.licensor_url() {
            metadata.push((b"LicensorURL".to_vec(), url.as_bytes().to_vec()));
        }
        if let Some(date) = notice.metadata_date() {
            metadata.push((b"MetadataDate".to_vec(), date.as_bytes().to_vec()));
        }
        if let Some(ts) = notice.notice_applied_at() {
            metadata.push((b"NoticeAppliedAt".to_vec(), ts.as_bytes().to_vec()));
        }
    }

    fn inject_text_chunks_webp_from_notice(
        &self,
        webp_data: &[u8],
        notice: &RightsNotice,
    ) -> Result<Vec<u8>> {
        if !notice.has_legal_content() && notice.dmi().is_none() {
            return Ok(webp_data.to_vec());
        }

        if webp_data.len() < 12 || &webp_data[0..4] != b"RIFF" || &webp_data[8..12] != b"WEBP" {
            return Err(Error::Metadata("Invalid WebP signature".to_string()));
        }

        let dmi_val = notice.dmi().unwrap_or(DmiValue::Unspecified);

        let xmp_chunk =
            Self::create_webp_xmp_chunk(&Self::generate_xmp_notice_from_notice(dmi_val, notice));

        if xmp_chunk.is_empty() {
            return Ok(webp_data.to_vec());
        }

        let exif_chunk = notice.seed().map(Self::create_webp_exif_chunk);

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

        let new_riff_size = (output.len() - 8) as u32;
        output[4] = new_riff_size as u8;
        output[5] = (new_riff_size >> 8) as u8;
        output[6] = (new_riff_size >> 16) as u8;
        output[7] = (new_riff_size >> 24) as u8;

        Ok(output)
    }

    fn generate_xmp_notice_from_notice(dmi: DmiValue, notice: &RightsNotice) -> Vec<u8> {
        let vocab_key = dmi.plus_vocab_key();
        let bom = "\u{feff}";
        let seed_attr = notice
            .seed()
            .map(|s| format!("\n             stegoeggo:ProtectionSeed=\"{}\"", s))
            .unwrap_or_default();

        let mut legal_props = String::new();
        if notice.has_legal_content() {
            if let Some(creator) = notice.creator() {
                legal_props.push_str(&format!(
                    "\n   <dc:creator>\n    <rdf:Seq>\n     <rdf:li>{}</rdf:li>\n    </rdf:Seq>\n   </dc:creator>",
                    xml_escape(creator)
                ));
            }
            if let Some(statement) = notice.web_statement_of_rights() {
                legal_props.push_str(&format!(
                    "\n   <xmpRights:WebStatement>{}</xmpRights:WebStatement>",
                    xml_escape(statement)
                ));
            } else if let Some(url) = notice.license_url() {
                legal_props.push_str(&format!(
                    "\n   <xmpRights:WebStatement>{}</xmpRights:WebStatement>",
                    xml_escape(url)
                ));
            }
            if let Some(terms) = notice.usage_terms() {
                let lang = notice.usage_terms_lang().unwrap_or("x-default");
                legal_props.push_str(&format!(
                    "\n   <xmpRights:UsageTerms>\n    <rdf:Alt>\n     <rdf:li xml:lang=\"{}\">{}</rdf:li>\n    </rdf:Alt>\n   </xmpRights:UsageTerms>",
                    xml_escape(lang),
                    xml_escape(terms)
                ));
            }
            if let Some(constraints) = notice.ai_constraints() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:AIConstraints>{}</stegoeggo:AIConstraints>",
                    xml_escape(constraints)
                ));
            }
            if let Some(holder) = notice.copyright_holder() {
                let copyright = if holder.contains("Copyright") {
                    holder.to_string()
                } else {
                    format!("Copyright (c) {}", holder)
                };
                legal_props.push_str(&format!(
                    "\n   <dc:rights>\n    <rdf:Alt>\n     <rdf:li xml:lang=\"x-default\">{}</rdf:li>\n    </rdf:Alt>\n   </dc:rights>",
                    xml_escape(&copyright)
                ));
            }
            if let Some(line) = notice.credit_line() {
                legal_props.push_str(&format!(
                    "\n   <photoshop:Credit>{}</photoshop:Credit>",
                    xml_escape(line)
                ));
            }
            if let Some(date) = notice.creation_date() {
                legal_props.push_str(&format!(
                    "\n   <photoshop:DateCreated>{}</photoshop:DateCreated>",
                    xml_escape(date)
                ));
            }
            if let Some(owner) = notice.copyright_owner() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:CopyrightOwner>{}</stegoeggo:CopyrightOwner>",
                    xml_escape(owner)
                ));
            }
            if let Some(name) = notice.licensor_name() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:LicensorName>{}</stegoeggo:LicensorName>",
                    xml_escape(name)
                ));
            }
            if let Some(email) = notice.licensor_email() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:LicensorEmail>{}</stegoeggo:LicensorEmail>",
                    xml_escape(email)
                ));
            }
            if let Some(url) = notice.licensor_url() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:LicensorURL>{}</stegoeggo:LicensorURL>",
                    xml_escape(url)
                ));
            }
            if let Some(date) = notice.metadata_date() {
                legal_props.push_str(&format!(
                    "\n   <xmp:MetadataDate>{}</xmp:MetadataDate>",
                    xml_escape(date)
                ));
            }
            if let Some(ts) = notice.notice_applied_at() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:NoticeAppliedAt>{}</stegoeggo:NoticeAppliedAt>",
                    xml_escape(ts)
                ));
            }
        }

        let xmp = format!(
            "<?xpacket begin=\"{bom}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" \
             xmlns:plus=\"{PLUS_NAMESPACE}\" \
             xmlns:stegoeggo=\"https://github.com/eggstack/stegoeggo\" \
             xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
             xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\" \
             xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" \
             xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\">\n\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
             <rdf:Description rdf:about=\"\"\n\
              {PLUS_DATA_MINING_PROPERTY}=\"{vocab_key}\"{seed_attr}>{legal_props}\n   </rdf:Description>\n\
             </rdf:RDF>\n\
             </x:xmpmeta>\n\
             <?xpacket end=\"w\"?>"
        );
        xmp.into_bytes()
    }

    pub(crate) fn has_stego_owned_metadata(
        &self,
        img_bytes: &[u8],
        format: ImageOutputFormat,
    ) -> bool {
        match format {
            ImageOutputFormat::Png => self.png_has_stego_metadata(img_bytes),
            ImageOutputFormat::Jpeg => self.jpeg_has_stego_metadata(img_bytes),
            ImageOutputFormat::WebP => self.webp_has_stego_metadata(img_bytes),
        }
    }

    fn png_has_stego_metadata(&self, png_data: &[u8]) -> bool {
        if png_data.len() < 8 || &png_data[0..8] != b"\x89PNG\r\n\x1a\n" {
            return false;
        }
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
                    if key == b"X-Protection-Seed"
                        || key == b"DMI-PROHIBITED"
                        || key == b"noai"
                        || key == b"Copyright"
                        || key == b"Contact"
                        || key == b"License"
                        || key == b"UsageTerms"
                        || key == b"DateCreated"
                        || key == b"AIConstraints"
                        || key == b"WebStatementOfRights"
                        || key == b"Creator"
                        || key == b"CreditLine"
                        || key == b"CopyrightOwner"
                        || key == b"LicensorName"
                        || key == b"LicensorEmail"
                        || key == b"LicensorURL"
                        || key == b"MetadataDate"
                        || key == b"NoticeAppliedAt"
                    {
                        return true;
                    }
                }
            }

            pos += 12 + chunk_len;
        }
        false
    }

    fn jpeg_has_stego_metadata(&self, jpeg_data: &[u8]) -> bool {
        if jpeg_data.len() < 2 || jpeg_data[0] != 0xFF || jpeg_data[1] != 0xD8 {
            return false;
        }
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
                    return false;
                }
                let comment_len =
                    u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
                let comment_start = pos + 4;
                let comment_end = (comment_start + comment_len - 2).min(jpeg_data.len());
                let comment = &jpeg_data[comment_start..comment_end];
                if comment.starts_with(Self::STRUCTURED_COM_MAGIC) {
                    return true;
                }
                if let Ok(comment_str) = String::from_utf8(comment.to_vec()) {
                    if comment_str.starts_with("X-Protection-Seed: ")
                        || comment_str.starts_with("DMI-PROHIBITED: ")
                        || comment_str.starts_with("noai: ")
                        || comment_str.starts_with("Copyright: ")
                        || comment_str.starts_with("Contact: ")
                        || comment_str.starts_with("License: ")
                        || comment_str.starts_with("UsageTerms: ")
                        || comment_str.starts_with("DateCreated: ")
                        || comment_str.starts_with("AIConstraints: ")
                        || comment_str.starts_with("WebStatementOfRights: ")
                        || comment_str.starts_with("Creator: ")
                        || comment_str.starts_with("CreditLine: ")
                        || comment_str.starts_with("CopyrightOwner: ")
                        || comment_str.starts_with("LicensorName: ")
                        || comment_str.starts_with("LicensorEmail: ")
                        || comment_str.starts_with("LicensorURL: ")
                        || comment_str.starts_with("MetadataDate: ")
                        || comment_str.starts_with("NoticeAppliedAt: ")
                    {
                        return true;
                    }
                }
                pos += 2 + comment_len;
                continue;
            }
            if pos + 4 > jpeg_data.len() {
                return false;
            }
            let segment_len = u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
            if marker == 0xED {
                return true;
            }
            if marker == 0xE1 {
                let seg_end = pos + 2 + segment_len;
                if seg_end > jpeg_data.len() {
                    break;
                }
                let seg_data = &jpeg_data[pos + 4..seg_end];
                if seg_data.starts_with(b"http://ns.adobe.com/xap/1.0/\0") {
                    return true;
                }
            }
            pos += 2 + segment_len;
        }
        false
    }

    fn webp_has_stego_metadata(&self, webp_data: &[u8]) -> bool {
        if webp_data.len() < 12 || &webp_data[0..4] != b"RIFF" || &webp_data[8..12] != b"WEBP" {
            return false;
        }
        let mut pos = 12;
        while pos + 8 <= webp_data.len() {
            let chunk_id = &webp_data[pos..pos + 4];
            let chunk_size = u32::from_le_bytes([
                webp_data[pos + 4],
                webp_data[pos + 5],
                webp_data[pos + 6],
                webp_data[pos + 7],
            ]) as usize;
            let padded_size = chunk_size + (chunk_size & 1);

            if chunk_id == b"XMP " || chunk_id == b"EXIF" {
                return true;
            }

            pos += 8 + padded_size;
        }
        false
    }

    fn generate_xmp_dmi(dmi: DmiValue, seed: Option<u64>) -> Vec<u8> {
        let vocab_key = dmi.plus_vocab_key();
        let bom = "\u{feff}";
        let seed_attr = seed
            .map(|s| format!("\n             stegoeggo:ProtectionSeed=\"{}\"", s))
            .unwrap_or_default();
        let xmp = format!(
            "<?xpacket begin=\"{bom}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" \
             xmlns:plus=\"{PLUS_NAMESPACE}\" \
             xmlns:stegoeggo=\"https://github.com/eggstack/stegoeggo\">\n\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
             <rdf:Description rdf:about=\"\"\n\
             {PLUS_DATA_MINING_PROPERTY}=\"{vocab_key}\"{seed_attr}/>\n\
             </rdf:RDF>\n\
             </x:xmpmeta>\n\
             <?xpacket end=\"w\"?>"
        );
        xmp.into_bytes()
    }

    #[allow(dead_code)]
    fn generate_xmp_notice(
        dmi: DmiValue,
        seed: Option<u64>,
        legal: Option<&LegalMetadata>,
    ) -> Vec<u8> {
        let vocab_key = dmi.plus_vocab_key();
        let bom = "\u{feff}";
        let seed_attr = seed
            .map(|s| format!("\n             stegoeggo:ProtectionSeed=\"{}\"", s))
            .unwrap_or_default();

        let mut legal_props = String::new();
        if let Some(legal) = legal {
            if let Some(creator) = legal.creator() {
                legal_props.push_str(&format!(
                    "\n   <dc:creator>\n    <rdf:Seq>\n     <rdf:li>{}</rdf:li>\n    </rdf:Seq>\n   </dc:creator>",
                    xml_escape(creator)
                ));
            }
            if let Some(statement) = legal.web_statement_of_rights() {
                legal_props.push_str(&format!(
                    "\n   <xmpRights:WebStatement>{}</xmpRights:WebStatement>",
                    xml_escape(statement)
                ));
            } else if let Some(url) = legal.license_url() {
                legal_props.push_str(&format!(
                    "\n   <xmpRights:WebStatement>{}</xmpRights:WebStatement>",
                    xml_escape(url)
                ));
            }
            if let Some(terms) = legal.usage_terms() {
                let lang = legal.usage_terms_lang().unwrap_or("x-default");
                legal_props.push_str(&format!(
                    "\n   <xmpRights:UsageTerms>\n    <rdf:Alt>\n     <rdf:li xml:lang=\"{}\">{}</rdf:li>\n    </rdf:Alt>\n   </xmpRights:UsageTerms>",
                    xml_escape(lang),
                    xml_escape(terms)
                ));
            }
            if let Some(constraints) = legal.ai_constraints() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:AIConstraints>{}</stegoeggo:AIConstraints>",
                    xml_escape(constraints)
                ));
            }
            if let Some(holder) = legal.copyright_holder() {
                let copyright = if holder.contains("Copyright") {
                    holder.to_string()
                } else {
                    format!("Copyright (c) {}", holder)
                };
                legal_props.push_str(&format!(
                    "\n   <dc:rights>\n    <rdf:Alt>\n     <rdf:li xml:lang=\"x-default\">{}</rdf:li>\n    </rdf:Alt>\n   </dc:rights>",
                    xml_escape(&copyright)
                ));
            }
            if let Some(line) = legal.credit_line() {
                legal_props.push_str(&format!(
                    "\n   <photoshop:Credit>{}</photoshop:Credit>",
                    xml_escape(line)
                ));
            }
            if let Some(date) = legal.creation_date() {
                legal_props.push_str(&format!(
                    "\n   <photoshop:DateCreated>{}</photoshop:DateCreated>",
                    xml_escape(date)
                ));
            }
            if let Some(owner) = legal.copyright_owner() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:CopyrightOwner>{}</stegoeggo:CopyrightOwner>",
                    xml_escape(owner)
                ));
            }
            if let Some(name) = legal.licensor_name() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:LicensorName>{}</stegoeggo:LicensorName>",
                    xml_escape(name)
                ));
            }
            if let Some(email) = legal.licensor_email() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:LicensorEmail>{}</stegoeggo:LicensorEmail>",
                    xml_escape(email)
                ));
            }
            if let Some(url) = legal.licensor_url() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:LicensorURL>{}</stegoeggo:LicensorURL>",
                    xml_escape(url)
                ));
            }
            if let Some(date) = legal.metadata_date() {
                legal_props.push_str(&format!(
                    "\n   <xmp:MetadataDate>{}</xmp:MetadataDate>",
                    xml_escape(date)
                ));
            }
            if let Some(ts) = legal.notice_applied_at() {
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:NoticeAppliedAt>{}</stegoeggo:NoticeAppliedAt>",
                    xml_escape(ts)
                ));
            } else {
                let now = current_timestamp_iso8601();
                legal_props.push_str(&format!(
                    "\n   <stegoeggo:NoticeAppliedAt>{}</stegoeggo:NoticeAppliedAt>",
                    xml_escape(&now)
                ));
            }
        }

        let xmp = format!(
            "<?xpacket begin=\"{bom}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" \
             xmlns:plus=\"{PLUS_NAMESPACE}\" \
             xmlns:stegoeggo=\"https://github.com/eggstack/stegoeggo\" \
             xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
             xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\" \
             xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" \
             xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\">\n\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
             <rdf:Description rdf:about=\"\"\n\
              {PLUS_DATA_MINING_PROPERTY}=\"{vocab_key}\"{seed_attr}>{legal_props}\n   </rdf:Description>\n\
             </rdf:RDF>\n\
             </x:xmpmeta>\n\
             <?xpacket end=\"w\"?>"
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
        limits: Option<&crate::ResourceLimits>,
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
        let mut chunk_count: usize = 0;

        while pos + 12 <= png_data.len() {
            chunk_count += 1;
            if let Some(lim) = limits {
                if chunk_count > lim.max_png_chunks() {
                    return Err(Error::ContainerLimitExceeded {
                        kind: "PNG chunks",
                        count: chunk_count,
                        limit: lim.max_png_chunks(),
                    });
                }
            }

            let chunk_len = u32::from_be_bytes([
                png_data[pos],
                png_data[pos + 1],
                png_data[pos + 2],
                png_data[pos + 3],
            ]) as usize;

            if let Some(lim) = limits {
                if chunk_len > lim.max_png_chunk_bytes() {
                    return Err(Error::MetadataLimitExceeded {
                        kind: "PNG chunk",
                        size: chunk_len,
                        limit: lim.max_png_chunk_bytes(),
                    });
                }
            }

            let chunk_type = &png_data[pos + 4..pos + 8];

            if chunk_type == b"IEND" {
                if let Some(dmi_val) = dmi {
                    let xmp_chunk =
                        Self::create_png_xmp_chunk(&Self::generate_xmp_dmi(dmi_val, seed))?;
                    output.extend_from_slice(&xmp_chunk);
                }
                for (key, value) in metadata {
                    let text_chunk = Self::create_png_text_chunk(key, value)?;
                    output.extend_from_slice(&text_chunk);
                }
                if let Some(s) = seed {
                    let desc_value = format!("Protected image. Seed: {}", s);
                    let desc_chunk =
                        Self::create_png_text_chunk(b"Description", desc_value.as_bytes())?;
                    output.extend_from_slice(&desc_chunk);
                }
            }

            let chunk_end = pos
                .checked_add(12)
                .and_then(|p| p.checked_add(chunk_len))
                .ok_or_else(|| Error::ImageTruncated("PNG chunk length overflow".to_string()))?;
            if chunk_end > png_data.len() {
                return Err(Error::ImageTruncated(format!(
                    "PNG chunk at offset {} claims length {} but only {} bytes remain",
                    pos,
                    chunk_len,
                    png_data.len().saturating_sub(pos)
                )));
            }
            output.extend_from_slice(&png_data[pos..chunk_end]);
            pos = chunk_end;
        }

        Ok(output)
    }

    fn create_png_xmp_chunk(xmp_data: &[u8]) -> Result<Vec<u8>> {
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

        let len = u32::try_from(chunk_data.len()).map_err(|_| {
            Error::Metadata(format!(
                "PNG iTXt chunk data length {} exceeds u32::MAX",
                chunk_data.len()
            ))
        })?;
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&len.to_be_bytes());
        chunk.extend_from_slice(b"iTXt");
        chunk.extend_from_slice(&chunk_data);
        let crc = Self::crc32(b"iTXt", &chunk_data);
        chunk.extend_from_slice(&crc.to_be_bytes());
        Ok(chunk)
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
                    self.inject_all_dmi_markers(&mut output, dmi, metadata, seed, ctx)?;
                    inserted = true;
                }
                output.extend_from_slice(&jpeg_data[pos..]);
                break;
            }

            if marker == 0xDA {
                if !inserted {
                    self.inject_all_dmi_markers(&mut output, dmi, metadata, seed, ctx)?;
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
            self.inject_all_dmi_markers(&mut output, dmi, metadata, seed, ctx)?;
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
    ) -> Result<()> {
        if let Some(dmi_val) = dmi {
            let exif_marker = Self::create_jpeg_exif_marker(&Self::generate_exif_dmi(dmi_val))?;
            output.extend_from_slice(&exif_marker);

            let iptc_marker =
                Self::create_jpeg_iptc_marker(&Self::generate_iptc_iim_dmi(dmi_val, seed))?;
            output.extend_from_slice(&iptc_marker);

            let xmp_marker = Self::create_jpeg_xmp_marker(&Self::generate_xmp_dmi(dmi_val, seed))?;
            output.extend_from_slice(&xmp_marker);
        }

        for (key, value) in metadata {
            let com_chunk = Self::create_jpeg_comment(key, value)?;
            output.extend_from_slice(&com_chunk);
        }

        if let Some(context) = ctx {
            let structured_com = Self::generate_structured_com_marker(dmi, seed, context);
            output.extend_from_slice(&structured_com);
        }
        Ok(())
    }

    fn create_jpeg_xmp_marker(xmp_data: &[u8]) -> Result<Vec<u8>> {
        let namespace = b"http://ns.adobe.com/xap/1.0/\0";
        let total_len = namespace.len() + xmp_data.len() + 2;
        let len_u16 = u16::try_from(total_len).map_err(|_| {
            Error::Metadata(format!(
                "JPEG XMP marker length {} exceeds u16::MAX (65535)",
                total_len
            ))
        })?;
        let mut marker = Vec::with_capacity(4 + total_len);
        marker.push(0xFF);
        marker.push(0xE1);
        marker.extend_from_slice(&len_u16.to_be_bytes());
        marker.extend_from_slice(namespace);
        marker.extend_from_slice(xmp_data);
        Ok(marker)
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
    fn create_jpeg_exif_marker(exif_data: &[u8]) -> Result<Vec<u8>> {
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
        let count = u32::try_from(exif_data.len()).map_err(|_| {
            Error::Metadata(format!(
                "JPEG EXIF UserComment count {} exceeds u32::MAX",
                exif_data.len()
            ))
        })?;
        ifd.extend_from_slice(&count.to_le_bytes()); // count
                                                     // Value: if <= 4 bytes, stored inline; otherwise offset from TIFF start.
                                                     // Our data is always > 4 bytes, so use offset = 8 (header) + 2 (count) + 12 (entry) = 22
        ifd.extend_from_slice(&22u32.to_le_bytes()); // offset to data

        let total_len = tiff_prefix.len() + tiff_header.len() + ifd.len() + exif_data.len();
        let len_u16 = u16::try_from(total_len + 2).map_err(|_| {
            Error::Metadata(format!(
                "JPEG EXIF marker length {} exceeds u16::MAX (65535)",
                total_len + 2
            ))
        })?;
        let mut marker = Vec::with_capacity(4 + total_len);
        marker.push(0xFF);
        marker.push(0xE1);
        marker.extend_from_slice(&len_u16.to_be_bytes());
        marker.extend_from_slice(tiff_prefix);
        marker.extend_from_slice(&tiff_header);
        marker.extend_from_slice(&ifd);
        marker.extend_from_slice(exif_data);
        Ok(marker)
    }

    /// Creates an IPTC-IIM APP13 marker (0xFFED) containing DMI data.
    /// Wraps the IPTC data in a valid Photoshop resource envelope:
    /// "Photoshop 3.0\0" + Resource ID (0x0404) + Pascal string + data size + data.
    fn create_jpeg_iptc_marker(iptc_data: &[u8]) -> Result<Vec<u8>> {
        let photoshop_id = b"Photoshop 3.0\0";

        // Photoshop resource envelope
        let mut resource = Vec::new();
        resource.extend_from_slice(&0x0404u16.to_be_bytes()); // Resource ID: IPTC-IIM
        resource.push(0x00); // Pascal string length (0 = empty)
        resource.push(0x00); // padding byte (even alignment)
        let data_size = u32::try_from(iptc_data.len()).map_err(|_| {
            Error::Metadata(format!(
                "JPEG IPTC data size {} exceeds u32::MAX",
                iptc_data.len()
            ))
        })?;
        resource.extend_from_slice(&data_size.to_be_bytes()); // data size
        resource.extend_from_slice(iptc_data); // IPTC-IIM data

        let mut data = Vec::new();
        data.extend_from_slice(photoshop_id);
        data.extend_from_slice(&resource);

        let len = u16::try_from(data.len() + 2).map_err(|_| {
            Error::Metadata(format!(
                "JPEG IPTC marker length {} exceeds u16::MAX (65535)",
                data.len() + 2
            ))
        })?;
        let mut marker = Vec::new();
        marker.push(0xFF);
        marker.push(0xED);
        marker.extend_from_slice(&len.to_be_bytes());
        marker.extend_from_slice(&data);
        Ok(marker)
    }

    #[allow(dead_code)]
    fn inject_text_chunks_webp(
        &self,
        webp_data: &[u8],
        metadata: &[(Vec<u8>, Vec<u8>)],
        dmi: Option<DmiValue>,
        seed: Option<u64>,
        legal: Option<&LegalMetadata>,
    ) -> Result<Vec<u8>> {
        if metadata.is_empty() && dmi.is_none() {
            return Ok(webp_data.to_vec());
        }

        if webp_data.len() < 12 || &webp_data[0..4] != b"RIFF" || &webp_data[8..12] != b"WEBP" {
            return Err(Error::Metadata("Invalid WebP signature".to_string()));
        }

        let dmi_val = dmi.unwrap_or(DmiValue::Unspecified);

        // Build XMP chunk with DMI, seed, and legal notice embedded as XMP properties
        let xmp_chunk =
            Self::create_webp_xmp_chunk(&Self::generate_xmp_notice(dmi_val, seed, legal));

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

    fn create_png_text_chunk(key: &[u8], value: &[u8]) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        data.extend_from_slice(key);
        data.push(0);
        data.extend_from_slice(value);

        let len = u32::try_from(data.len()).map_err(|_| {
            Error::Metadata(format!(
                "PNG tEXt chunk data length {} exceeds u32::MAX",
                data.len()
            ))
        })?;
        let mut chunk = Vec::new();

        chunk.extend_from_slice(&len.to_be_bytes());
        chunk.extend_from_slice(b"tEXt");
        chunk.extend_from_slice(&data);

        let crc = Self::crc32(b"tEXt", &data);
        chunk.extend_from_slice(&crc.to_be_bytes());

        Ok(chunk)
    }

    fn create_jpeg_comment(key: &[u8], value: &[u8]) -> Result<Vec<u8>> {
        let mut comment = Vec::new();
        comment.extend_from_slice(key);
        comment.extend_from_slice(b": ");
        comment.extend_from_slice(value);

        let len = u16::try_from(comment.len() + 2).map_err(|_| {
            Error::Metadata(format!(
                "JPEG COM marker length {} exceeds u16::MAX (65535)",
                comment.len() + 2
            ))
        })?;

        let mut chunk = Vec::new();
        chunk.push(0xFF);
        chunk.push(0xFE);
        chunk.extend_from_slice(&len.to_be_bytes());
        chunk.extend_from_slice(&comment);

        Ok(chunk)
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

    pub(crate) fn parse_structured_com_payload(data: &[u8]) -> Option<(u64, u8, u16)> {
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
        hasher.finalize()
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
        Self::extract_seed_from_image_with_limits(img_bytes, None)
    }

    /// Extract the protection seed from image metadata, with optional resource limits.
    ///
    /// When limits are provided, chunk/segment iteration is bounded to prevent
    /// resource exhaustion on malicious inputs.
    pub fn extract_seed_from_image_with_limits(
        img_bytes: &[u8],
        limits: Option<&crate::ResourceLimits>,
    ) -> Option<u64> {
        if img_bytes.len() < 8 {
            return None;
        }

        if img_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            Self::extract_seed_from_png(img_bytes, limits)
        } else if img_bytes.starts_with(&[0xFF, 0xD8]) {
            Self::extract_seed_from_jpeg(img_bytes, limits)
        } else if img_bytes.len() >= 12
            && &img_bytes[0..4] == b"RIFF"
            && &img_bytes[8..12] == b"WEBP"
        {
            Self::extract_seed_from_webp(img_bytes, limits)
        } else {
            None
        }
    }

    fn extract_seed_from_png(
        png_data: &[u8],
        limits: Option<&crate::ResourceLimits>,
    ) -> Option<u64> {
        let mut pos = 8;
        let mut chunk_count: usize = 0;
        while pos + 12 <= png_data.len() {
            chunk_count += 1;
            if let Some(lim) = limits {
                if chunk_count > lim.max_png_chunks() {
                    return None;
                }
            }

            let chunk_len = u32::from_be_bytes([
                png_data[pos],
                png_data[pos + 1],
                png_data[pos + 2],
                png_data[pos + 3],
            ]) as usize;

            if let Some(lim) = limits {
                if chunk_len > lim.max_png_chunk_bytes() {
                    return None;
                }
            }

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

    fn extract_seed_from_jpeg(
        jpeg_data: &[u8],
        limits: Option<&crate::ResourceLimits>,
    ) -> Option<u64> {
        let mut pos = 2;
        let mut segment_count: usize = 0;
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

            segment_count += 1;
            if let Some(lim) = limits {
                if segment_count > lim.max_jpeg_segments() {
                    return None;
                }
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

            if let Some(lim) = limits {
                if segment_len > lim.max_jpeg_segment_bytes() {
                    return None;
                }
            }

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

    fn extract_seed_from_webp(
        webp_data: &[u8],
        limits: Option<&crate::ResourceLimits>,
    ) -> Option<u64> {
        if webp_data.len() < 20 {
            return None;
        }

        let mut pos = 12;
        let mut chunk_count: usize = 0;

        while pos + 8 <= webp_data.len() {
            chunk_count += 1;
            if let Some(lim) = limits {
                if chunk_count > lim.max_webp_riff_chunks() {
                    return None;
                }
            }

            let chunk_type = &webp_data[pos..pos + 4];
            let chunk_size = u32::from_le_bytes([
                webp_data[pos + 4],
                webp_data[pos + 5],
                webp_data[pos + 6],
                webp_data[pos + 7],
            ]) as usize;

            if let Some(lim) = limits {
                if chunk_size > lim.max_webp_riff_bytes() {
                    return None;
                }
            }

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

    #[doc(hidden)]
    pub fn inject_bytes(&self, img_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>> {
        let should_inject_metadata =
            Self::should_inject_metadata(ctx.inject_metadata(), ctx.protection_level());

        let notice = ctx.normalize_rights_notice();

        let metadata = self.generate_poison_metadata_from_notice(
            &notice,
            should_inject_metadata,
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

        match ctx.metadata_update_policy() {
            MetadataUpdatePolicy::FailOnConflict => {
                if self.has_stego_owned_metadata(img_bytes, format) {
                    return Err(Error::Metadata(
                        "MetadataUpdatePolicy::FailOnConflict: \
                         image already contains StegoEggo metadata"
                            .to_string(),
                    ));
                }
            }
            MetadataUpdatePolicy::PreserveExisting => {
                if self.has_stego_owned_metadata(img_bytes, format) {
                    return Ok(img_bytes.to_vec());
                }
            }
            MetadataUpdatePolicy::ReplaceStegoOwned => {}
        }

        let with_metadata = match format {
            ImageOutputFormat::Png => self.inject_text_chunks_png(
                img_bytes,
                &metadata,
                notice.dmi(),
                notice.seed(),
                Some(&ctx.resource_limits()),
            )?,
            ImageOutputFormat::Jpeg => self.inject_text_chunks_jpeg(
                img_bytes,
                &metadata,
                notice.dmi(),
                notice.seed(),
                Some(ctx),
            )?,
            ImageOutputFormat::WebP => {
                self.inject_text_chunks_webp_from_notice(img_bytes, &notice)?
            }
        };

        Ok(with_metadata)
    }
}

#[cfg(test)]
#[allow(deprecated)]
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
            .inject_text_chunks_png(&png, &metadata, None, None, None)
            .unwrap();
        assert!(result.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
        assert!(result.len() > png.len());
    }

    #[test]
    fn png_injected_chunks_have_valid_crc() {
        let protector = MetadataTrapProtector::new();
        let png = encode_png(&make_test_image());
        let metadata = vec![(b"Test-Key".to_vec(), b"Test-Value".to_vec())];
        let result = protector
            .inject_text_chunks_png(
                &png,
                &metadata,
                Some(DmiValue::ProhibitedAiMlTraining),
                Some(42),
                None,
            )
            .unwrap();

        let mut pos = 8;
        let mut checked_injected_chunk = false;
        while pos + 12 <= result.len() {
            let chunk_len = u32::from_be_bytes([
                result[pos],
                result[pos + 1],
                result[pos + 2],
                result[pos + 3],
            ]) as usize;
            let chunk_type = &result[pos + 4..pos + 8];
            let data_start = pos + 8;
            let data_end = data_start + chunk_len;
            let crc_start = data_end;
            let crc_end = crc_start + 4;
            assert!(crc_end <= result.len(), "PNG chunk extends past file end");

            if chunk_type == b"tEXt" || chunk_type == b"iTXt" {
                let stored = u32::from_be_bytes([
                    result[crc_start],
                    result[crc_start + 1],
                    result[crc_start + 2],
                    result[crc_start + 3],
                ]);
                let mut hasher = Crc32Hasher::new();
                hasher.update(chunk_type);
                hasher.update(&result[data_start..data_end]);
                assert_eq!(stored, hasher.finalize());
                checked_injected_chunk = true;
            }

            if chunk_type == b"IEND" {
                break;
            }
            pos = crc_end;
        }

        assert!(checked_injected_chunk);
    }

    #[test]
    fn png_inject_empty_metadata_returns_original() {
        let protector = MetadataTrapProtector::new();
        let png = encode_png(&make_test_image());
        let result = protector
            .inject_text_chunks_png(&png, &[], None, None, None)
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
            .inject_text_chunks_png(&png, &metadata, None, None, None)
            .unwrap();
        let extracted = MetadataTrapProtector::extract_seed_from_png(&result, None);
        assert!(extracted.is_none());

        // Now with seed in metadata
        let metadata_with_seed = vec![
            (b"X-Protection-Seed".to_vec(), b"12345".to_vec()),
            (b"Other".to_vec(), b"Value".to_vec()),
        ];
        let result = protector
            .inject_text_chunks_png(&png, &metadata_with_seed, None, Some(12345), None)
            .unwrap();
        let extracted = MetadataTrapProtector::extract_seed_from_png(&result, None);
        assert_eq!(extracted, Some(12345));
    }

    #[test]
    fn png_dmi_injects_xmp_chunk() {
        let protector = MetadataTrapProtector::new();
        let png = encode_png(&make_test_image());
        let result = protector
            .inject_text_chunks_png(
                &png,
                &[],
                Some(DmiValue::ProhibitedAiMlTraining),
                None,
                None,
            )
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
        let extracted = MetadataTrapProtector::extract_seed_from_jpeg(&result, None).unwrap();
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
            .inject_text_chunks_webp(&webp, &metadata, None, None, None)
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
            .inject_text_chunks_webp(&webp, &metadata, None, Some(99999), None)
            .unwrap();
        let extracted = MetadataTrapProtector::extract_seed_from_webp(&result, None);
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
            .inject_text_chunks_png(&png, &metadata, None, Some(100), None)
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
            .inject_text_chunks_webp(&webp, &metadata, None, Some(300), None)
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
    fn legal_metadata_none_produces_no_output() {
        let mut metadata = Vec::new();
        MetadataTrapProtector::add_legal_metadata(&mut metadata, None);

        assert!(
            metadata.is_empty(),
            "no fields should be emitted when legal metadata is None"
        );
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
            .inject_text_chunks_webp(&webp, &[], None, None, None)
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
    fn webp_xmp_legal_children_after_description_start_tag() {
        let legal = LegalMetadata::new()
            .with_copyright_holder("Test Corp")
            .with_creator("Test Author")
            .with_usage_terms("All rights reserved");
        let xmp = MetadataTrapProtector::generate_xmp_notice(
            DmiValue::ProhibitedAiMlTraining,
            Some(42),
            Some(&legal),
        );
        let xmp_str = String::from_utf8_lossy(&xmp);

        let desc_open = xmp_str
            .find("<rdf:Description")
            .expect("rdf:Description must be present");
        let desc_close_offset = xmp_str[desc_open..]
            .find('>')
            .expect("rdf:Description must have a closing >");
        let desc_close = desc_open + desc_close_offset;

        for needle in ["<dc:creator>", "<dc:rights>", "<xmpRights:UsageTerms>"] {
            let pos = xmp_str
                .find(needle)
                .unwrap_or_else(|| panic!("expected {needle} in XMP"));
            assert!(
                desc_close < pos,
                "{needle} must appear after the > of <rdf:Description ...> (desc_close={desc_close}, pos={pos})"
            );
        }

        let end_desc = xmp_str
            .find("</rdf:Description>")
            .expect("must close rdf:Description");
        for needle in ["<dc:creator>", "<dc:rights>", "<xmpRights:UsageTerms>"] {
            let pos = xmp_str.find(needle).unwrap();
            assert!(
                pos < end_desc,
                "{needle} must appear before </rdf:Description> (pos={pos}, end_desc={end_desc})"
            );
        }
    }

    #[test]
    fn webp_xmp_rdf_description_is_well_ordered() {
        let legal = LegalMetadata::new().with_copyright_holder("Test Corp");
        let xmp = MetadataTrapProtector::generate_xmp_notice(
            DmiValue::ProhibitedAiMlTraining,
            Some(7),
            Some(&legal),
        );
        let xmp_str = String::from_utf8_lossy(&xmp);

        let rdf_desc_open = xmp_str.find("<rdf:Description").unwrap();
        assert!(
            rdf_desc_open < xmp_str.find("plus:DataMining").unwrap(),
            "<rdf:Description must precede plus:DataMining"
        );

        let desc_close_rel = xmp_str[rdf_desc_open..].find('>').unwrap();
        let desc_close = rdf_desc_open + desc_close_rel;
        assert!(
            desc_close < xmp_str.find("<dc:rights>").unwrap(),
            "<rdf:Description ...> must close before any legal child element"
        );
    }

    #[test]
    fn webp_xmp_dc_rights_uses_rdf_alt() {
        let legal = LegalMetadata::new().with_copyright_holder("Test Corp");
        let xmp = MetadataTrapProtector::generate_xmp_notice(
            DmiValue::ProhibitedAiMlTraining,
            None,
            Some(&legal),
        );
        let xmp_str = String::from_utf8_lossy(&xmp);

        let rights_start = xmp_str.find("<dc:rights>").unwrap();
        let rights_end = xmp_str.find("</dc:rights>").unwrap();
        let slice = &xmp_str[rights_start..rights_end];
        assert!(
            slice.contains("<rdf:Alt>"),
            "dc:rights must contain <rdf:Alt> container"
        );
        assert!(
            slice.contains("xml:lang=\"x-default\""),
            "rdf:Alt must include xml:lang x-default"
        );
        assert!(
            slice.contains("Copyright (c) Test Corp"),
            "rdf:Alt must wrap the copyright text"
        );
    }

    #[test]
    fn webp_xmp_usage_terms_uses_rdf_alt() {
        let legal = LegalMetadata::new().with_usage_terms("All rights reserved");
        let xmp = MetadataTrapProtector::generate_xmp_notice(
            DmiValue::ProhibitedAiMlTraining,
            None,
            Some(&legal),
        );
        let xmp_str = String::from_utf8_lossy(&xmp);

        let start = xmp_str.find("<xmpRights:UsageTerms>").unwrap();
        let end = xmp_str.find("</xmpRights:UsageTerms>").unwrap();
        let slice = &xmp_str[start..end];
        assert!(slice.contains("<rdf:Alt>"));
        assert!(slice.contains("All rights reserved"));
    }

    #[test]
    fn webp_xmp_exiftool_tag_shape_regression() {
        let legal = LegalMetadata::new()
            .with_copyright_holder("Test Corp")
            .with_creator("Test Author")
            .with_contact_email("legal@test.com")
            .with_web_statement_of_rights("https://example.com/rights")
            .with_usage_terms("All rights reserved")
            .with_ai_constraints("No AI training");
        let xmp = MetadataTrapProtector::generate_xmp_notice(
            DmiValue::ProhibitedAiMlTraining,
            Some(99),
            Some(&legal),
        );
        let xmp_str = String::from_utf8_lossy(&xmp);

        for required in [
            "<rdf:Description",
            "<dc:creator>",
            "<rdf:Seq>",
            "<dc:rights>",
            "<rdf:Alt>",
            "<xmpRights:UsageTerms>",
            "<xmpRights:WebStatement>",
            "<stegoeggo:AIConstraints>",
            "stegoeggo:ProtectionSeed=\"99\"",
            "plus:DataMining=\"DMI-PROHIBITED-AIMLTRAINING\"",
            "</rdf:Description>",
        ] {
            assert!(
                xmp_str.contains(required),
                "XMP must contain {required}, got: {xmp_str}"
            );
        }

        let desc_close = xmp_str.find("</rdf:Description>").unwrap();
        let start_tag_close = xmp_str.find("<rdf:Description").unwrap();
        let after_open = xmp_str[start_tag_close..]
            .find('>')
            .map(|p| start_tag_close + p)
            .unwrap();
        assert!(
            after_open < desc_close,
            "rdf:Description opening tag must close before its closing tag"
        );
    }

    #[test]
    fn jpeg_exif_marker_has_tiff_header() {
        let exif_data = b"Exif test data";
        let marker = MetadataTrapProtector::create_jpeg_exif_marker(exif_data).unwrap();
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
    fn jpeg_exif_marker_length_matches_payload() {
        let exif_data = b"DMI: ProhibitedAiMlTraining";
        let marker = MetadataTrapProtector::create_jpeg_exif_marker(exif_data).unwrap();
        let segment_len = u16::from_be_bytes([marker[2], marker[3]]) as usize;
        assert_eq!(segment_len, marker.len() - 2);
    }

    #[test]
    fn jpeg_exif_marker_has_ifd_usercomment() {
        let exif_data = b"DMI: ProhibitedAiMlTraining";
        let marker = MetadataTrapProtector::create_jpeg_exif_marker(exif_data).unwrap();
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
        let marker = MetadataTrapProtector::create_jpeg_iptc_marker(&iptc_data).unwrap();
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
        assert!(xmp_str.contains("xmlns:stegoeggo=\"https://github.com/eggstack/stegoeggo\""));
    }

    #[test]
    fn xmp_contains_tdm_reservation_prohibit() {
        let xmp = MetadataTrapProtector::generate_xmp_dmi(DmiValue::ProhibitedAiMlTraining, None);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(
            !xmp_str.contains("tdm:reserve_tdm"),
            "TDM reservation should not be present in default output"
        );
        assert!(
            xmp_str.contains("plus:DataMining=\"DMI-PROHIBITED-AIMLTRAINING\""),
            "XMP should contain canonical plus:DataMining"
        );
    }

    #[test]
    fn xmp_contains_tdm_reservation_allow() {
        let xmp = MetadataTrapProtector::generate_xmp_dmi(DmiValue::Allowed, None);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(
            !xmp_str.contains("tdm:reserve_tdm"),
            "TDM reservation should not be present in default output"
        );
        assert!(
            xmp_str.contains("plus:DataMining=\"DMI-ALLOWED\""),
            "XMP should contain canonical plus:DataMining"
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
    fn jpeg_auto_dmi_injects_standard_markers() {
        let protector = MetadataTrapProtector::new();
        let img = make_test_image();
        let jpeg = encode_jpeg(&img);
        let mut ctx =
            ProtectionContext::new(0.5, 42).with_format(crate::types::ImageOutputFormat::Jpeg);
        ctx.set_protection_level(ProtectionLevel::Standard);

        let injected = protector.inject_bytes(&jpeg, &ctx).unwrap();

        assert!(
            injected.windows(6).any(|w| w == b"Exif\x00\x00"),
            "auto DMI should create EXIF metadata"
        );
        assert!(
            injected.windows(14).any(|w| w == b"Photoshop 3.0\x00"),
            "auto DMI should create IPTC metadata"
        );
        assert!(
            injected.windows(15).any(|w| w == b"plus:DataMining"),
            "auto DMI should create XMP/PLUS metadata"
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
                )
                .unwrap();
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

    #[test]
    fn xml_escape_ampersand() {
        assert_eq!(xml_escape("A & B"), "A &amp; B");
    }

    #[test]
    fn xml_escape_less_than() {
        assert_eq!(xml_escape("a < b"), "a &lt; b");
    }

    #[test]
    fn xml_escape_greater_than() {
        assert_eq!(xml_escape("a > b"), "a &gt; b");
    }

    #[test]
    fn xml_escape_quotes() {
        assert_eq!(
            xml_escape(r#"he said "hello""#),
            "he said &quot;hello&quot;"
        );
    }

    #[test]
    fn xml_escape_apostrophe() {
        assert_eq!(xml_escape("it's"), "it&apos;s");
    }

    #[test]
    fn xml_escape_mixed_special_chars() {
        let input = "Copyright (c) 2024 <Corp> & \"Friends\" — it's fine";
        let escaped = xml_escape(input);
        assert!(escaped.contains("&amp;"));
        assert!(escaped.contains("&lt;"));
        assert!(escaped.contains("&gt;"));
        assert!(escaped.contains("&quot;"));
        assert!(escaped.contains("&apos;"));
    }

    #[test]
    fn xml_escape_no_change_for_plain_text() {
        let input = "Simple copyright notice 2024";
        assert_eq!(xml_escape(input), input);
    }

    #[test]
    fn webp_xmp_round_trip_special_chars() {
        let protector = MetadataTrapProtector::new();
        let img = make_test_image();
        let webp = encode_webp(&img);

        let legal = LegalMetadata::new()
            .with_copyright_holder("A < B & C")
            .with_usage_terms("Terms with \"quotes\" and 'apostrophes'")
            .with_creator("O'Brien & Sons");

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::WebP)
            .with_legal_metadata(legal)
            .with_metadata_injection(true);

        let result = protector.inject_bytes(&webp, &ctx).unwrap();
        let report = crate::verify_legal_notice(&result, b"");

        assert_eq!(
            report.copyright_holder(),
            Some("A < B & C"),
            "Copyright should survive XML escaping round-trip"
        );
        assert_eq!(
            report.usage_terms(),
            Some("Terms with \"quotes\" and 'apostrophes'"),
            "Usage terms should survive XML escaping round-trip"
        );
        assert_eq!(
            report.creator(),
            Some("O'Brien & Sons"),
            "Creator should survive XML escaping round-trip"
        );
    }

    #[test]
    fn png_text_round_trip_special_chars() {
        let protector = MetadataTrapProtector::new();
        let img = make_test_image();
        let png = encode_png(&img);

        let legal = LegalMetadata::new()
            .with_copyright_holder("A < B & C")
            .with_usage_terms("Terms with \"quotes\"");

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_legal_metadata(legal)
            .with_metadata_injection(true);

        let result = protector.inject_bytes(&png, &ctx).unwrap();
        let report = crate::verify_legal_notice(&result, b"");

        assert_eq!(
            report.copyright_holder(),
            Some("A < B & C"),
            "Copyright should survive tEXt round-trip"
        );
        assert_eq!(
            report.usage_terms(),
            Some("Terms with \"quotes\""),
            "Usage terms should survive tEXt round-trip"
        );
    }

    #[test]
    fn jpeg_com_round_trip_special_chars() {
        let protector = MetadataTrapProtector::new();
        let img = make_test_image();
        let jpeg = encode_jpeg(&img);

        let legal = LegalMetadata::new()
            .with_copyright_holder("A < B & C")
            .with_usage_terms("Terms with \"quotes\"");

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_legal_metadata(legal)
            .with_metadata_injection(true);

        let result = protector.inject_bytes(&jpeg, &ctx).unwrap();
        let report = crate::verify_legal_notice(&result, b"");

        assert_eq!(
            report.copyright_holder(),
            Some("A < B & C"),
            "Copyright should survive COM round-trip"
        );
        assert_eq!(
            report.usage_terms(),
            Some("Terms with \"quotes\""),
            "Usage terms should survive COM round-trip"
        );
    }
}
