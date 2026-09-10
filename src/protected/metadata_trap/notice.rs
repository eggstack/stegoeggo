use crate::error::{Error, Result};
use crate::types::{
    DmiValue, LegalMetadata, ProtectionContext, ProtectionLevel, RightsNotice,
    PLUS_DATA_MINING_PROPERTY, PLUS_NAMESPACE,
};
use crc32fast::Hasher as Crc32Hasher;

pub(super) fn xml_escape(s: &str) -> String {
    crate::xmp::escape_metadata_value(s)
}

pub(super) fn has_copyright_prefix(value: &str) -> bool {
    let Some(prefix) = value.get(.."Copyright".len()) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case("Copyright") {
        return false;
    }
    value
        .get("Copyright".len()..)
        .and_then(|rest| rest.chars().next())
        .is_some_and(|next| next.is_whitespace() || matches!(next, '(' | ':'))
}

impl super::RightsMetadataProtector {
    pub(super) fn should_inject_metadata(
        inject_metadata: Option<bool>,
        protection_level: Option<ProtectionLevel>,
    ) -> bool {
        inject_metadata.unwrap_or(!matches!(protection_level, Some(ProtectionLevel::Disabled)))
    }

    #[allow(dead_code)]
    pub(super) fn generate_rights_metadata(
        &self,
        _dmi_value: Option<DmiValue>,
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
        }

        if should_inject_claims && legal.is_some() {
            Self::add_legal_metadata(&mut metadata, legal);
        }

        metadata
    }

    #[allow(dead_code)]
    pub(super) fn add_legal_metadata(
        metadata: &mut Vec<(Vec<u8>, Vec<u8>)>,
        legal: Option<&LegalMetadata>,
    ) {
        let Some(legal) = legal else {
            return;
        };

        if let Some(holder) = legal.copyright_holder() {
            let copyright = if has_copyright_prefix(holder) {
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

    pub(super) fn generate_rights_metadata_from_notice(
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
        }

        let should_inject_claims = inject_legal_claims.unwrap_or(notice.has_legal_content());
        if should_inject_claims && notice.has_legal_content() {
            Self::add_legal_metadata_from_notice(&mut metadata, notice);
        }

        metadata
    }

    pub(super) fn add_legal_metadata_from_notice(
        metadata: &mut Vec<(Vec<u8>, Vec<u8>)>,
        notice: &RightsNotice,
    ) {
        if let Some(holder) = notice.copyright_holder() {
            let copyright = if has_copyright_prefix(holder) {
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

    pub(super) fn build_legal_props_from_notice(notice: &RightsNotice) -> String {
        let mut props = String::new();
        if !notice.has_legal_content() {
            return props;
        }
        if let Some(creator) = notice.creator() {
            props.push_str(&format!(
                "\n   <dc:creator>\n    <rdf:Seq>\n     <rdf:li>{}</rdf:li>\n    </rdf:Seq>\n   </dc:creator>",
                xml_escape(creator)
            ));
        }
        if let Some(statement) = notice.web_statement_of_rights() {
            props.push_str(&format!(
                "\n   <xmpRights:WebStatement>{}</xmpRights:WebStatement>",
                xml_escape(statement)
            ));
        } else if let Some(url) = notice.license_url() {
            props.push_str(&format!(
                "\n   <xmpRights:WebStatement>{}</xmpRights:WebStatement>",
                xml_escape(url)
            ));
        }
        if let Some(terms) = notice.usage_terms() {
            let lang = notice.usage_terms_lang().unwrap_or("x-default");
            props.push_str(&format!(
                "\n   <xmpRights:UsageTerms>\n    <rdf:Alt>\n     <rdf:li xml:lang=\"{}\">{}</rdf:li>\n    </rdf:Alt>\n   </xmpRights:UsageTerms>",
                xml_escape(lang),
                xml_escape(terms)
            ));
        }
        if notice.dmi() == Some(DmiValue::ProhibitedSeeConstraints) {
            if let Some(constraints) = notice.ai_constraints() {
                props.push_str(&format!(
                    "\n   <plus:OtherConstraints>\n    <rdf:Alt>\n     <rdf:li xml:lang=\"x-default\">{}</rdf:li>\n    </rdf:Alt>\n   </plus:OtherConstraints>",
                    xml_escape(constraints)
                ));
            }
        }
        if let Some(constraints) = notice.ai_constraints() {
            props.push_str(&format!(
                "\n   <stegoeggo:AIConstraints>{}</stegoeggo:AIConstraints>",
                xml_escape(constraints)
            ));
        }
        if let Some(holder) = notice.copyright_holder() {
            let copyright = if has_copyright_prefix(holder) {
                holder.to_string()
            } else {
                format!("Copyright (c) {}", holder)
            };
            props.push_str(&format!(
                "\n   <dc:rights>\n    <rdf:Alt>\n     <rdf:li xml:lang=\"x-default\">{}</rdf:li>\n    </rdf:Alt>\n   </dc:rights>",
                xml_escape(&copyright)
            ));
        }
        if let Some(line) = notice.credit_line() {
            props.push_str(&format!(
                "\n   <photoshop:Credit>{}</photoshop:Credit>",
                xml_escape(line)
            ));
        }
        if let Some(date) = notice.creation_date() {
            props.push_str(&format!(
                "\n   <photoshop:DateCreated>{}</photoshop:DateCreated>",
                xml_escape(date)
            ));
        }
        if let Some(owner) = notice.copyright_owner() {
            props.push_str(&format!(
                "\n   <stegoeggo:CopyrightOwner>{}</stegoeggo:CopyrightOwner>",
                xml_escape(owner)
            ));
        }
        if let Some(name) = notice.licensor_name() {
            props.push_str(&format!(
                "\n   <stegoeggo:LicensorName>{}</stegoeggo:LicensorName>",
                xml_escape(name)
            ));
        }
        if let Some(email) = notice.licensor_email() {
            props.push_str(&format!(
                "\n   <stegoeggo:LicensorEmail>{}</stegoeggo:LicensorEmail>",
                xml_escape(email)
            ));
        }
        if let Some(url) = notice.licensor_url() {
            props.push_str(&format!(
                "\n   <stegoeggo:LicensorURL>{}</stegoeggo:LicensorURL>",
                xml_escape(url)
            ));
        }
        if let Some(date) = notice.metadata_date() {
            props.push_str(&format!(
                "\n   <xmp:MetadataDate>{}</xmp:MetadataDate>",
                xml_escape(date)
            ));
        }
        if let Some(ts) = notice.notice_applied_at() {
            props.push_str(&format!(
                "\n   <stegoeggo:NoticeAppliedAt>{}</stegoeggo:NoticeAppliedAt>",
                xml_escape(ts)
            ));
        }
        props
    }

    pub(super) fn build_xmp_packet(dmi: DmiValue, seed: Option<u64>, legal_props: &str) -> Vec<u8> {
        let vocab_uri = dmi.plus_vocab_uri();
        let bom = "\u{feff}";
        let seed_attr = seed
            .map(|s| format!("\n             stegoeggo:ProtectionSeed=\"{}\"", s))
            .unwrap_or_default();
        let dmi_attr = match vocab_uri {
            Some(uri) => format!("\n             {PLUS_DATA_MINING_PROPERTY}=\"{}\"", uri),
            None => String::new(),
        };
        format!(
            "<?xpacket begin=\"{bom}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" \
             xmlns:plus=\"{PLUS_NAMESPACE}\" \
             xmlns:stegoeggo=\"https://github.com/eggstack/stegoeggo\" \
             xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
             xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\" \
             xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" \
             xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\">\n\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
             <rdf:Description rdf:about=\"\"{dmi_attr}{seed_attr}>{legal_props}\n   </rdf:Description>\n\
             </rdf:RDF>\n\
             </x:xmpmeta>\n\
             <?xpacket end=\"w\"?>"
        )
        .into_bytes()
    }

    pub(super) fn generate_xmp_notice_from_notice(dmi: DmiValue, notice: &RightsNotice) -> Vec<u8> {
        let legal_props = Self::build_legal_props_from_notice(notice);
        Self::build_xmp_packet(dmi, notice.seed(), &legal_props)
    }

    pub(super) fn generate_xmp_dmi(dmi: DmiValue, seed: Option<u64>) -> Vec<u8> {
        Self::build_xmp_packet(dmi, seed, "")
    }

    #[allow(dead_code)]
    pub(super) fn generate_xmp_notice(
        dmi: DmiValue,
        seed: Option<u64>,
        legal: Option<&LegalMetadata>,
    ) -> Vec<u8> {
        let legal_props = match legal {
            Some(l) => {
                let notice = RightsNotice::new()
                    .with_dmi(dmi)
                    .with_seed(seed.unwrap_or(0));
                let notice = if let Some(v) = l.copyright_holder() {
                    notice.with_copyright_holder(v)
                } else {
                    notice
                };
                let notice = if let Some(v) = l.creator() {
                    notice.with_creator(v)
                } else {
                    notice
                };
                let notice = if let Some(v) = l.usage_terms() {
                    notice.with_usage_terms(v)
                } else {
                    notice
                };
                let notice = if let Some(v) = l.ai_constraints() {
                    notice.with_ai_constraints(v)
                } else {
                    notice
                };
                let notice = if let Some(v) = l.web_statement_of_rights() {
                    notice.with_web_statement_of_rights(v)
                } else {
                    notice
                };
                let notice = if let Some(v) = l.contact_email() {
                    notice.with_contact_email(v)
                } else {
                    notice
                };
                let notice = if let Some(v) = l.license_url() {
                    notice.with_license_url(v)
                } else {
                    notice
                };
                let notice = if let Some(v) = l.credit_line() {
                    notice.with_credit_line(v)
                } else {
                    notice
                };
                let notice = if let Some(v) = l.creation_date() {
                    notice.with_creation_date(v)
                } else {
                    notice
                };
                Self::build_legal_props_from_notice(&notice)
            }
            None => String::new(),
        };
        Self::build_xmp_packet(dmi, seed, &legal_props)
    }

    /// Generates EXIF UserComment tag (0x9286) containing the DMI value.
    /// EXIF UserComment is a common location for AI/ML opt-out markers.
    /// Format: ASCII charset (8 bytes) + null-terminated text.
    pub(super) fn generate_exif_dmi(dmi: DmiValue) -> Vec<u8> {
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
    pub(super) fn generate_iptc_iim_dmi(dmi: DmiValue, seed: Option<u64>) -> Vec<u8> {
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

    pub(super) fn create_png_text_chunk(
        key: &[u8],
        value: &[u8],
        limits: Option<&crate::ResourceLimits>,
    ) -> Result<Vec<u8>> {
        if key.is_empty() || key.len() > 79 {
            return Err(Error::Metadata(format!(
                "PNG tEXt keyword length must be between 1 and 79 bytes, got {}",
                key.len()
            )));
        }
        if let Some(lim) = limits {
            let total = key.len() + 1 + value.len();
            lim.check_metadata_size("tEXt field", total, lim.max_metadata_field_bytes())?;
        }
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

    pub(super) fn create_jpeg_comment(
        key: &[u8],
        value: &[u8],
        limits: Option<&crate::ResourceLimits>,
    ) -> Result<Vec<u8>> {
        if let Some(lim) = limits {
            let total = key.len() + 2 + value.len();
            lim.check_metadata_size("COM field", total, lim.max_metadata_field_bytes())?;
        }
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

    pub(super) const STRUCTURED_COM_MAGIC: &'static [u8] = b"cloakrs:v1:";

    #[cfg(test)]
    pub(super) fn generate_structured_com_marker(
        dmi: Option<DmiValue>,
        _seed: Option<u64>,
        ctx: &ProtectionContext,
    ) -> Vec<u8> {
        Self::generate_structured_com_marker_with_timestamp(dmi, ctx, None)
    }

    pub(super) fn generate_structured_com_marker_with_timestamp(
        dmi: Option<DmiValue>,
        ctx: &ProtectionContext,
        timestamp: Option<&str>,
    ) -> Vec<u8> {
        let mut payload = Vec::with_capacity(48);
        payload.extend_from_slice(Self::STRUCTURED_COM_MAGIC);

        payload.push(1); // version

        let level_byte = ctx.protection_level().map(|l| l.to_byte()).unwrap_or(2);
        payload.push(level_byte);

        payload.extend_from_slice(&ctx.seed().to_le_bytes());

        let intensity_val = (ctx.intensity() * 100.0) as u16;
        payload.extend_from_slice(&intensity_val.to_le_bytes());

        let timestamp_secs = timestamp
            .and_then(super::common::unix_seconds_from_timestamp)
            .unwrap_or_else(super::common::current_unix_seconds);
        payload.extend_from_slice(&timestamp_secs.to_le_bytes());

        let dmi_byte = dmi
            .map(|d| match d {
                DmiValue::Unspecified => 0u8,
                DmiValue::Allowed => 1,
                DmiValue::ProhibitedAiMlTraining => 2,
                DmiValue::ProhibitedGenAiMlTraining => 3,
                DmiValue::ProhibitedExceptSearchEngineIndexing => 4,
                DmiValue::Prohibited => 5,
                DmiValue::ProhibitedSeeConstraints => 6,
            })
            .unwrap_or(0);
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

    pub(super) fn crc32(chunk_type: &[u8], data: &[u8]) -> u32 {
        let mut hasher = Crc32Hasher::new();
        hasher.update(chunk_type);
        hasher.update(data);
        hasher.finalize()
    }

    pub(super) fn crc16(data: &[u8]) -> u16 {
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
