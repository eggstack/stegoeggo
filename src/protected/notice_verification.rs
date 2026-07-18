use crate::protected::steganography::SteganographyProtector;
use crate::types::{
    DmiValue, EvidenceChannel, EvidenceStrength, NoticeVerification, RightsSignalKind,
    VerificationStatus,
};

type NoticeFields = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub(crate) fn verify_notice_metadata(img_bytes: &[u8], mac_key: &[u8]) -> NoticeVerification {
    if img_bytes.len() < 8 {
        return empty_report();
    }

    let format = detect_format(img_bytes);

    let mut channels = Vec::new();
    let mut seed: Option<u64> = None;
    let mut dmi: Option<DmiValue> = None;
    let mut tdm_reserved: Option<bool> = None;
    let mut canonical_dmi: Option<DmiValue> = None;
    let mut legacy_dmi: Option<DmiValue> = None;
    let mut detected_rights_signal: Option<RightsSignalKind> = None;

    let (
        copyright_holder,
        creator,
        contact,
        rights_url,
        usage_terms,
        ai_constraints,
        license_url,
        web_statement_of_rights,
        credit_line,
        copyright_owner,
        licensor_name,
        licensor_email,
        licensor_url,
        metadata_date,
        notice_applied_at,
    ) = match format {
        Some(Format::Png) => {
            let result = extract_png_notice(img_bytes, &mut channels, &mut seed);
            extract_xmp_dmi_from_png(
                img_bytes,
                &mut dmi,
                &mut tdm_reserved,
                &mut canonical_dmi,
                &mut legacy_dmi,
                &mut detected_rights_signal,
            );
            result
        }
        Some(Format::Jpeg) => {
            let result = extract_jpeg_notice(img_bytes, &mut channels, &mut seed);
            extract_xmp_dmi_from_jpeg(
                img_bytes,
                &mut dmi,
                &mut tdm_reserved,
                &mut canonical_dmi,
                &mut legacy_dmi,
                &mut detected_rights_signal,
            );
            result
        }
        Some(Format::WebP) => {
            let result = extract_webp_notice(img_bytes, &mut channels, &mut seed);
            extract_xmp_dmi_from_webp(
                img_bytes,
                &mut dmi,
                &mut tdm_reserved,
                &mut canonical_dmi,
                &mut legacy_dmi,
                &mut detected_rights_signal,
            );
            result
        }
        None => return empty_report(),
    };

    let has_notice = copyright_holder.is_some()
        || creator.is_some()
        || contact.is_some()
        || rights_url.is_some()
        || usage_terms.is_some()
        || ai_constraints.is_some()
        || dmi.is_some()
        || license_url.is_some()
        || web_statement_of_rights.is_some()
        || credit_line.is_some()
        || copyright_owner.is_some()
        || licensor_name.is_some()
        || licensor_email.is_some()
        || licensor_url.is_some()
        || metadata_date.is_some()
        || notice_applied_at.is_some();

    let stego_status;
    let stego_payload;
    let authenticated;

    let stego = SteganographyProtector::new();

    let is_jpeg = img_bytes.starts_with(&[0xFF, 0xD8]);

    if !mac_key.is_empty() {
        let result = stego.verify_payload_from_bytes_with_key(img_bytes, mac_key);
        match result {
            VerificationStatus::Verified => {
                stego_status = VerificationStatus::Verified;
                authenticated = true;
                let payload = stego.extract_payload_from_bytes_with_key(img_bytes, mac_key);
                stego_payload = payload;
                if stego_payload.is_some() {
                    if is_jpeg {
                        channels.push(EvidenceChannel::DctPayload);
                    } else {
                        channels.push(EvidenceChannel::LsbPayload);
                    }
                }
            }
            VerificationStatus::Invalid => {
                stego_status = VerificationStatus::Invalid;
                authenticated = false;
                stego_payload = None;
            }
            VerificationStatus::NotFound => {
                stego_status = VerificationStatus::NotFound;
                authenticated = false;
                stego_payload = None;
            }
        }
    } else {
        let result = stego.verify_payload_from_bytes_with_key(img_bytes, &[]);
        stego_status = result;
        authenticated = false;
        stego_payload = None;
        if result == VerificationStatus::Verified {
            if is_jpeg {
                channels.push(EvidenceChannel::DctPayload);
            } else {
                channels.push(EvidenceChannel::LsbPayload);
            }
        }
    }

    let evidence_strength = compute_evidence_strength(has_notice, authenticated, &channels);

    let rights_signal_kind = detected_rights_signal.unwrap_or(RightsSignalKind::Unknown);

    NoticeVerification::builder()
        .copyright_holder(copyright_holder)
        .creator(creator)
        .contact(contact)
        .rights_url(rights_url)
        .usage_terms(usage_terms)
        .ai_constraints(ai_constraints)
        .dmi(dmi)
        .tdm_reserved(tdm_reserved)
        .rights_signal_kind(rights_signal_kind)
        .canonical_dmi(canonical_dmi)
        .legacy_dmi(legacy_dmi)
        .protection_seed(seed)
        .stego_status(stego_status)
        .stego_payload(stego_payload)
        .authenticated(authenticated)
        .evidence_strength(evidence_strength)
        .channels(channels)
        .license_url(license_url)
        .web_statement_of_rights(web_statement_of_rights)
        .credit_line(credit_line)
        .copyright_owner(copyright_owner)
        .licensor_name(licensor_name)
        .licensor_email(licensor_email)
        .licensor_url(licensor_url)
        .metadata_date(metadata_date)
        .notice_applied_at(notice_applied_at)
        .build()
}

fn empty_report() -> NoticeVerification {
    NoticeVerification::builder().build()
}

fn compute_evidence_strength(
    has_notice: bool,
    authenticated: bool,
    channels: &[EvidenceChannel],
) -> EvidenceStrength {
    let has_stego = channels
        .iter()
        .any(|c| matches!(c, EvidenceChannel::LsbPayload | EvidenceChannel::DctPayload));

    if has_notice && authenticated && has_stego {
        EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance
    } else if has_notice && has_stego {
        EvidenceStrength::MetadataNoticeAndBestEffortStego
    } else if has_notice {
        EvidenceStrength::MetadataNoticeOnly
    } else {
        EvidenceStrength::NoNoticeFound
    }
}

enum Format {
    Png,
    Jpeg,
    WebP,
}

fn detect_format(bytes: &[u8]) -> Option<Format> {
    if bytes.len() < 4 {
        return None;
    }
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        Some(Format::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(Format::Jpeg)
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(Format::WebP)
    } else {
        None
    }
}

fn extract_png_notice(
    png_data: &[u8],
    channels: &mut Vec<EvidenceChannel>,
    seed: &mut Option<u64>,
) -> NoticeFields {
    let mut copyright_holder: Option<String> = None;
    let mut creator: Option<String> = None;
    let mut contact: Option<String> = None;
    let mut rights_url: Option<String> = None;
    let mut usage_terms: Option<String> = None;
    let mut ai_constraints: Option<String> = None;
    let mut license_url: Option<String> = None;
    let mut web_statement_of_rights: Option<String> = None;
    let mut credit_line: Option<String> = None;
    let mut copyright_owner: Option<String> = None;
    let mut licensor_name: Option<String> = None;
    let mut licensor_email: Option<String> = None;
    let mut licensor_url: Option<String> = None;
    let mut metadata_date: Option<String> = None;
    let mut notice_applied_at: Option<String> = None;
    let mut found_text = false;

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
                let value_raw = &data[null_pos + 1..];
                let value_str = if let Some(end) = value_raw.iter().position(|&b| b == 0) {
                    String::from_utf8_lossy(&value_raw[..end]).into_owned()
                } else {
                    String::from_utf8_lossy(value_raw).into_owned()
                };

                if key == b"X-Protection-Seed" && seed.is_none() {
                    if let Ok(s) = value_str.parse() {
                        *seed = Some(s);
                    }
                }

                match key {
                    b"Copyright" => {
                        let holder = value_str
                            .strip_prefix("Copyright (c) ")
                            .unwrap_or(&value_str);
                        copyright_holder = Some(holder.to_string());
                        found_text = true;
                    }
                    b"Contact" => {
                        contact = Some(value_str);
                        found_text = true;
                    }
                    b"License" => {
                        rights_url = Some(value_str.clone());
                        license_url = Some(value_str);
                        found_text = true;
                    }
                    b"UsageTerms" => {
                        usage_terms = Some(value_str);
                        found_text = true;
                    }
                    b"AIConstraints" => {
                        ai_constraints = Some(value_str);
                        found_text = true;
                    }
                    b"WebStatementOfRights" => {
                        rights_url = Some(value_str.clone());
                        web_statement_of_rights = Some(value_str);
                        found_text = true;
                    }
                    b"Creator" => {
                        creator = Some(value_str);
                        found_text = true;
                    }
                    b"CreditLine" => {
                        credit_line = Some(value_str);
                        found_text = true;
                    }
                    b"CopyrightOwner" => {
                        copyright_owner = Some(value_str);
                        found_text = true;
                    }
                    b"LicensorName" => {
                        licensor_name = Some(value_str);
                        found_text = true;
                    }
                    b"LicensorEmail" => {
                        licensor_email = Some(value_str);
                        found_text = true;
                    }
                    b"LicensorURL" => {
                        licensor_url = Some(value_str);
                        found_text = true;
                    }
                    b"MetadataDate" => {
                        metadata_date = Some(value_str);
                        found_text = true;
                    }
                    b"NoticeAppliedAt" => {
                        notice_applied_at = Some(value_str);
                        found_text = true;
                    }
                    b"DMI-PROHIBITED" => {
                        found_text = true;
                    }
                    _ => {}
                }
            }

            if chunk_type == b"iTXt" && !found_text {
                let data_start = pos + 8;
                let data_end = (data_start + chunk_len).min(png_data.len());
                if &png_data[data_start..data_start + 18.min(data_end - data_start)]
                    == b"XML:com.adobe.xmp"
                {
                    channels.push(EvidenceChannel::PngXmp);
                }
            }
        }

        pos += 12 + chunk_len;
    }

    if found_text {
        channels.push(EvidenceChannel::PngText);
    }

    (
        copyright_holder,
        creator,
        contact,
        rights_url,
        usage_terms,
        ai_constraints,
        license_url,
        web_statement_of_rights,
        credit_line,
        copyright_owner,
        licensor_name,
        licensor_email,
        licensor_url,
        metadata_date,
        notice_applied_at,
    )
}

fn extract_xmp_dmi_from_png(
    png_data: &[u8],
    dmi: &mut Option<DmiValue>,
    tdm_reserved: &mut Option<bool>,
    canonical_dmi: &mut Option<DmiValue>,
    legacy_dmi: &mut Option<DmiValue>,
    rights_signal_kind: &mut Option<RightsSignalKind>,
) {
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

        if chunk_type == b"iTXt" {
            let data_start = pos + 8;
            let data_end = (data_start + chunk_len).min(png_data.len());
            let data = &png_data[data_start..data_end];

            if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                let key = &data[..null_pos];
                if key == b"XML:com.adobe.xmp" {
                    let value_raw = &data[null_pos + 1..];
                    if value_raw.len() >= 3 {
                        let value = &value_raw[3..]; // skip compression flag + method + null
                        if let Ok(xmp_str) = std::str::from_utf8(value) {
                            parse_xmp_for_dmi(
                                xmp_str,
                                dmi,
                                tdm_reserved,
                                canonical_dmi,
                                legacy_dmi,
                                rights_signal_kind,
                            );
                        }
                    }
                }
            }
        }

        pos += 12 + chunk_len;
    }
}

fn extract_jpeg_notice(
    jpeg_data: &[u8],
    channels: &mut Vec<EvidenceChannel>,
    seed: &mut Option<u64>,
) -> NoticeFields {
    let mut copyright_holder: Option<String> = None;
    let mut creator: Option<String> = None;
    let mut contact: Option<String> = None;
    let mut rights_url: Option<String> = None;
    let mut usage_terms: Option<String> = None;
    let mut ai_constraints: Option<String> = None;
    let mut license_url: Option<String> = None;
    let mut web_statement_of_rights: Option<String> = None;
    let mut credit_line: Option<String> = None;
    let mut copyright_owner: Option<String> = None;
    let mut licensor_name: Option<String> = None;
    let mut licensor_email: Option<String> = None;
    let mut licensor_url: Option<String> = None;
    let mut metadata_date: Option<String> = None;
    let mut notice_applied_at: Option<String> = None;
    let mut found_comment = false;
    let mut found_iptc = false;
    let mut found_xmp = false;

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
            // COM marker
            if pos + 4 > jpeg_data.len() {
                break;
            }
            let comment_len = u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
            if comment_len < 2 {
                pos += 2 + comment_len;
                continue;
            }
            let comment_start = pos + 4;
            let comment_end = (comment_start + comment_len - 2).min(jpeg_data.len());
            let comment = &jpeg_data[comment_start..comment_end];

            if comment.starts_with(b"cloakrs:v1:") {
                if let Some((s, _level, _intensity)) =
                    crate::protected::metadata_trap::MetadataTrapProtector::parse_structured_com_payload(comment)
                {
                    if seed.is_none() {
                        *seed = Some(s);
                    }
                }
            }

            if let Ok(comment_str) = String::from_utf8(comment.to_vec()) {
                if let Some(s) = comment_str.strip_prefix("X-Protection-Seed: ") {
                    if seed.is_none() {
                        *seed = s.trim().parse().ok();
                    }
                }

                match parse_com_kv(&comment_str) {
                    Some(ComField::Copyright(v)) => {
                        copyright_holder = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::Contact(v)) => {
                        contact = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::License(v)) => {
                        rights_url = Some(v.clone());
                        license_url = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::UsageTerms(v)) => {
                        usage_terms = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::AIConstraints(v)) => {
                        ai_constraints = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::WebStatementOfRights(v)) => {
                        rights_url = Some(v.clone());
                        web_statement_of_rights = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::Creator(v)) => {
                        creator = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::CreditLine(v)) => {
                        credit_line = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::CopyrightOwner(v)) => {
                        copyright_owner = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::LicensorName(v)) => {
                        licensor_name = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::LicensorEmail(v)) => {
                        licensor_email = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::LicensorURL(v)) => {
                        licensor_url = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::MetadataDate(v)) => {
                        metadata_date = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::NoticeAppliedAt(v)) => {
                        notice_applied_at = Some(v);
                        found_comment = true;
                    }
                    None => {}
                }
            }

            pos += 2 + comment_len;
            continue;
        }

        if pos + 4 > jpeg_data.len() {
            break;
        }

        let segment_len = u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
        let segment_end = pos + 2 + segment_len;
        if segment_end > jpeg_data.len() {
            break;
        }

        if marker == 0xED {
            // IPTC APP13
            let segment_data = &jpeg_data[pos + 4..segment_end];
            if segment_data.windows(13).any(|w| w == b"Photoshop 3.0\0") {
                found_iptc = true;
            }
        }

        if marker == 0xE1 {
            // APP1 — could be XMP
            let segment_data = &jpeg_data[pos + 4..segment_end];
            if segment_data
                .windows(28)
                .any(|w| w == b"http://ns.adobe.com/xap/1.0/")
            {
                found_xmp = true;
            }
        }

        pos = segment_end;
    }

    if found_comment {
        channels.push(EvidenceChannel::JpegComment);
    }
    if found_iptc {
        channels.push(EvidenceChannel::JpegIptc);
    }
    if found_xmp {
        channels.push(EvidenceChannel::JpegXmp);
    }

    (
        copyright_holder,
        creator,
        contact,
        rights_url,
        usage_terms,
        ai_constraints,
        license_url,
        web_statement_of_rights,
        credit_line,
        copyright_owner,
        licensor_name,
        licensor_email,
        licensor_url,
        metadata_date,
        notice_applied_at,
    )
}

fn extract_xmp_dmi_from_jpeg(
    jpeg_data: &[u8],
    dmi: &mut Option<DmiValue>,
    tdm_reserved: &mut Option<bool>,
    canonical_dmi: &mut Option<DmiValue>,
    legacy_dmi: &mut Option<DmiValue>,
    rights_signal_kind: &mut Option<RightsSignalKind>,
) {
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

        if pos + 4 > jpeg_data.len() {
            break;
        }

        let segment_len = u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
        let segment_end = pos + 2 + segment_len;
        if segment_end > jpeg_data.len() {
            break;
        }

        if marker == 0xE1 {
            let segment_data = &jpeg_data[pos + 4..segment_end];
            if segment_data
                .windows(28)
                .any(|w| w == b"http://ns.adobe.com/xap/1.0/")
            {
                if let Ok(xmp_str) = std::str::from_utf8(segment_data) {
                    parse_xmp_for_dmi(
                        xmp_str,
                        dmi,
                        tdm_reserved,
                        canonical_dmi,
                        legacy_dmi,
                        rights_signal_kind,
                    );
                }
            }
        }

        pos = segment_end;
    }
}

fn extract_xmp_text_property(xmp: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = xmp.find(&open) {
        let value_start = start + open.len();
        if let Some(end) = xmp[value_start..].find(&close) {
            let value = &xmp[value_start..value_start + end];
            if !value.is_empty() {
                return Some(unescape_xml(value));
            }
        }
    }
    None
}

fn extract_xmp_seq_property(xmp: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    if let Some(start) = xmp.find(&open) {
        let rest = &xmp[start..];
        let li_open = "<rdf:li>";
        let li_close = "</rdf:li>";
        if let Some(li_start) = rest.find(li_open) {
            let value_start = li_start + li_open.len();
            if let Some(li_end) = rest[value_start..].find(li_close) {
                let value = &rest[value_start..value_start + li_end];
                if !value.is_empty() {
                    return Some(unescape_xml(value));
                }
            }
        }
    }
    None
}

fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn extract_xmp_alt_property(xmp: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xmp.find(&open)?;
    let value_start = start + open.len();
    let end = xmp[value_start..].find(&close)?;
    let container = &xmp[value_start..value_start + end];
    let li_open = "<rdf:li";
    let li_close = "</rdf:li>";
    let li_start = container.find(li_open)?;
    let after_open = li_start + li_open.len();
    let li_content_start = container[after_open..]
        .find('>')
        .map(|p| after_open + p + 1)?;
    let li_end = container[li_content_start..].find(li_close)?;
    let value = &container[li_content_start..li_content_start + li_end];
    if value.is_empty() {
        None
    } else {
        Some(unescape_xml(value))
    }
}

fn extract_webp_notice(
    webp_data: &[u8],
    channels: &mut Vec<EvidenceChannel>,
    seed: &mut Option<u64>,
) -> NoticeFields {
    let mut copyright_holder: Option<String> = None;
    let mut creator: Option<String> = None;
    let mut contact: Option<String> = None;
    let mut rights_url: Option<String> = None;
    let mut usage_terms: Option<String> = None;
    let mut ai_constraints: Option<String> = None;
    let license_url: Option<String> = None;
    let mut web_statement_of_rights: Option<String> = None;
    let mut credit_line: Option<String> = None;
    let mut copyright_owner: Option<String> = None;
    let mut licensor_name: Option<String> = None;
    let mut licensor_email: Option<String> = None;
    let mut licensor_url: Option<String> = None;
    let mut metadata_date: Option<String> = None;
    let mut notice_applied_at: Option<String> = None;

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
                channels.push(EvidenceChannel::WebPXmp);

                if let Some(start) = xmp_str.find("stegoeggo:ProtectionSeed=\"") {
                    let value_start = start + "stegoeggo:ProtectionSeed=\"".len();
                    if let Some(end) = xmp_str[value_start..].find('"') {
                        let value_str = &xmp_str[value_start..value_start + end];
                        if let Ok(s) = value_str.parse::<u64>() {
                            if seed.is_none() {
                                *seed = Some(s);
                            }
                        }
                    }
                }

                copyright_holder = extract_xmp_alt_property(xmp_str, "dc:rights").map(|s| {
                    s.trim()
                        .strip_prefix("Copyright (c) ")
                        .unwrap_or(&s)
                        .to_string()
                });
                creator = extract_xmp_seq_property(xmp_str, "dc:creator");
                contact = extract_xmp_text_property(xmp_str, "photoshop:Credit");
                if let Some(v) = extract_xmp_text_property(xmp_str, "xmpRights:WebStatement") {
                    rights_url = Some(v.clone());
                    web_statement_of_rights = Some(v);
                }
                usage_terms = extract_xmp_alt_property(xmp_str, "xmpRights:UsageTerms");
                ai_constraints = extract_xmp_text_property(xmp_str, "stegoeggo:AIConstraints");
                credit_line = extract_xmp_text_property(xmp_str, "photoshop:Credit");
                if let Some(v) = extract_xmp_text_property(xmp_str, "photoshop:DateCreated") {
                    if copyright_holder.is_none() {
                        copyright_holder = Some(v);
                    }
                }
                copyright_owner = extract_xmp_text_property(xmp_str, "stegoeggo:CopyrightOwner");
                licensor_name = extract_xmp_text_property(xmp_str, "stegoeggo:LicensorName");
                licensor_email = extract_xmp_text_property(xmp_str, "stegoeggo:LicensorEmail");
                licensor_url = extract_xmp_text_property(xmp_str, "stegoeggo:LicensorURL");
                metadata_date = extract_xmp_text_property(xmp_str, "xmp:MetadataDate");
                notice_applied_at = extract_xmp_text_property(xmp_str, "stegoeggo:NoticeAppliedAt");
            }
        }

        if chunk_type == b"EXIF" && data_end > data_start {
            let data = &webp_data[data_start..data_end];
            channels.push(EvidenceChannel::WebPExif);

            let needle = b"Protection seed: ";
            if let Some(offset) = data.windows(needle.len()).position(|w| w == needle) {
                let start = offset + needle.len();
                let end = data[start..]
                    .iter()
                    .position(|&b| b == b'\0' || b == b'\n')
                    .map(|p| start + p)
                    .unwrap_or(data.len());
                if let Ok(seed_str) = std::str::from_utf8(&data[start..end]) {
                    if let Ok(s) = seed_str.trim().parse::<u64>() {
                        if seed.is_none() {
                            *seed = Some(s);
                        }
                    }
                }
            }
        }

        pos = data_start + chunk_size;
        if !chunk_size.is_multiple_of(2) {
            pos += 1;
        }
    }

    // WebP legal fields are extracted from the XMP chunk.
    // DMI/TDM properties are extracted separately via extract_xmp_dmi_from_webp.

    (
        copyright_holder,
        creator,
        contact,
        rights_url,
        usage_terms,
        ai_constraints,
        license_url,
        web_statement_of_rights,
        credit_line,
        copyright_owner,
        licensor_name,
        licensor_email,
        licensor_url,
        metadata_date,
        notice_applied_at,
    )
}

fn extract_xmp_dmi_from_webp(
    webp_data: &[u8],
    dmi: &mut Option<DmiValue>,
    tdm_reserved: &mut Option<bool>,
    canonical_dmi: &mut Option<DmiValue>,
    legacy_dmi: &mut Option<DmiValue>,
    rights_signal_kind: &mut Option<RightsSignalKind>,
) {
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
                parse_xmp_for_dmi(
                    xmp_str,
                    dmi,
                    tdm_reserved,
                    canonical_dmi,
                    legacy_dmi,
                    rights_signal_kind,
                );
            }
        }

        pos = data_start + chunk_size;
        if !chunk_size.is_multiple_of(2) {
            pos += 1;
        }
    }
}

enum ComField {
    Copyright(String),
    Contact(String),
    License(String),
    UsageTerms(String),
    AIConstraints(String),
    WebStatementOfRights(String),
    Creator(String),
    CreditLine(String),
    CopyrightOwner(String),
    LicensorName(String),
    LicensorEmail(String),
    LicensorURL(String),
    MetadataDate(String),
    NoticeAppliedAt(String),
}

fn parse_com_kv(comment: &str) -> Option<ComField> {
    if let Some(v) = comment.strip_prefix("Copyright: ") {
        return Some(ComField::Copyright(
            v.strip_prefix("Copyright (c) ").unwrap_or(v).to_string(),
        ));
    }
    if let Some(v) = comment.strip_prefix("Contact: ") {
        return Some(ComField::Contact(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("License: ") {
        return Some(ComField::License(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("UsageTerms: ") {
        return Some(ComField::UsageTerms(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("AIConstraints: ") {
        return Some(ComField::AIConstraints(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("WebStatementOfRights: ") {
        return Some(ComField::WebStatementOfRights(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("Creator: ") {
        return Some(ComField::Creator(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("CreditLine: ") {
        return Some(ComField::CreditLine(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("CopyrightOwner: ") {
        return Some(ComField::CopyrightOwner(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("LicensorName: ") {
        return Some(ComField::LicensorName(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("LicensorEmail: ") {
        return Some(ComField::LicensorEmail(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("LicensorURL: ") {
        return Some(ComField::LicensorURL(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("MetadataDate: ") {
        return Some(ComField::MetadataDate(v.to_string()));
    }
    if let Some(v) = comment.strip_prefix("NoticeAppliedAt: ") {
        return Some(ComField::NoticeAppliedAt(v.to_string()));
    }
    None
}

fn extract_xmp_element(xmp_str: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = xmp_str.find(&open) {
        let value_start = start + open.len();
        if let Some(end) = xmp_str[value_start..].find(&close) {
            let value = &xmp_str[value_start..value_start + end];
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn find_prefix_for_namespace(xmp_str: &str, namespace_url: &str) -> Option<String> {
    let ns_decl = format!("=\"{}\"", namespace_url);
    if let Some(pos) = xmp_str.find(&ns_decl) {
        let before = &xmp_str[..pos];
        if let Some(colon_pos) = before.rfind("xmlns:") {
            let prefix_start = colon_pos + 6;
            let prefix = &before[prefix_start..];
            if !prefix.is_empty() {
                return Some(prefix.to_string());
            }
        }
    }
    None
}

fn parse_xmp_for_dmi(
    xmp_str: &str,
    dmi: &mut Option<DmiValue>,
    tdm_reserved: &mut Option<bool>,
    canonical_dmi: &mut Option<DmiValue>,
    legacy_dmi: &mut Option<DmiValue>,
    rights_signal_kind: &mut Option<RightsSignalKind>,
) {
    if canonical_dmi.is_none() {
        let found = if let Some(val) = extract_xmp_attr(xmp_str, "plus:DataMining") {
            DmiValue::from_plus_vocab_key(&val)
        } else if let Some(prefix) =
            find_prefix_for_namespace(xmp_str, crate::types::PLUS_NAMESPACE)
        {
            let tag = format!("{}:DataMining", prefix);
            if let Some(val) = extract_xmp_element(xmp_str, &tag) {
                DmiValue::from_plus_vocab_key(&val)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(v) = found {
            *canonical_dmi = Some(v);
            if rights_signal_kind.is_none() {
                *rights_signal_kind = Some(RightsSignalKind::CanonicalPlusDataMining);
            }
        }
    }

    if dmi.is_none() && canonical_dmi.is_some() {
        *dmi = *canonical_dmi;
    }

    if legacy_dmi.is_none() {
        if let Some(val) = extract_xmp_attr(xmp_str, "Iptc4xmpExt:DataMiningAttribute") {
            if let Some(v) = parse_dmi_value(&val) {
                *legacy_dmi = Some(v);
                if dmi.is_none() {
                    *dmi = Some(v);
                }
                if rights_signal_kind.is_none() {
                    *rights_signal_kind = Some(RightsSignalKind::LegacyStegoEggoDmi);
                }
            }
        }
    }
    if legacy_dmi.is_none() {
        if let Some(val) = extract_xmp_attr(xmp_str, "Iptc4xmpExt:DMI-Prohibited") {
            if let Some(v) = parse_dmi_value(&val) {
                *legacy_dmi = Some(v);
                if dmi.is_none() {
                    *dmi = Some(v);
                }
                if rights_signal_kind.is_none() {
                    *rights_signal_kind = Some(RightsSignalKind::LegacyStegoEggoDmi);
                }
            }
        }
    }
    if legacy_dmi.is_none() {
        if let Some(val) = extract_xmp_attr(xmp_str, "Iptc4xmpExt:DMI-Allowed") {
            if let Some(v) = parse_dmi_value(&val) {
                *legacy_dmi = Some(v);
                if dmi.is_none() {
                    *dmi = Some(v);
                }
                if rights_signal_kind.is_none() {
                    *rights_signal_kind = Some(RightsSignalKind::LegacyStegoEggoDmi);
                }
            }
        }
    }
    if legacy_dmi.is_none() {
        if let Some(val) = extract_xmp_attr(xmp_str, "Iptc4xmpExt:DMI") {
            if let Some(v) = parse_dmi_value(&val) {
                *legacy_dmi = Some(v);
                if dmi.is_none() {
                    *dmi = Some(v);
                }
                if rights_signal_kind.is_none() {
                    *rights_signal_kind = Some(RightsSignalKind::LegacyStegoEggoDmi);
                }
            }
        }
    }

    if tdm_reserved.is_none() {
        if let Some(val) = extract_xmp_attr(xmp_str, "tdm:reserve_tdm") {
            *tdm_reserved = Some(val == "1");
            if rights_signal_kind.is_none() {
                *rights_signal_kind = Some(RightsSignalKind::LegacyTdmReservation);
            }
        }
    }
}

fn extract_xmp_attr(xmp_str: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr_name);
    if let Some(start) = xmp_str.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = xmp_str[value_start..].find('"') {
            return Some(xmp_str[value_start..value_start + end].to_string());
        }
    }
    None
}

fn parse_dmi_value(val: &str) -> Option<DmiValue> {
    match val {
        "Unspecified" => Some(DmiValue::Unspecified),
        "Allowed" => Some(DmiValue::Allowed),
        "ProhibitedAiMlTraining" => Some(DmiValue::ProhibitedAiMlTraining),
        "ProhibitedGenAiMlTraining" => Some(DmiValue::ProhibitedGenAiMlTraining),
        "ProhibitedExceptSearchEngineIndexing" => {
            Some(DmiValue::ProhibitedExceptSearchEngineIndexing)
        }
        "Prohibited" => Some(DmiValue::Prohibited),
        "ProhibitedSeeConstraints" => Some(DmiValue::ProhibitedSeeConstraints),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn helper_parse_dmi(xmp_str: &str) -> (Option<DmiValue>, Option<DmiValue>, RightsSignalKind) {
        let mut dmi = None;
        let mut tdm_reserved = None;
        let mut canonical_dmi = None;
        let mut legacy_dmi = None;
        let mut rights_signal_kind = None;
        parse_xmp_for_dmi(
            xmp_str,
            &mut dmi,
            &mut tdm_reserved,
            &mut canonical_dmi,
            &mut legacy_dmi,
            &mut rights_signal_kind,
        );
        (
            canonical_dmi,
            legacy_dmi,
            rights_signal_kind.unwrap_or(RightsSignalKind::Unknown),
        )
    }

    #[test]
    fn canonical_plus_datamining_attribute_form() {
        let xmp = r#"plus:DataMining="DMI-PROHIBITED-AIMLTRAINING""#;
        let (canonical, legacy, kind) = helper_parse_dmi(xmp);
        assert_eq!(canonical, Some(DmiValue::ProhibitedAiMlTraining));
        assert_eq!(legacy, None);
        assert_eq!(kind, RightsSignalKind::CanonicalPlusDataMining);
    }

    #[test]
    fn canonical_plus_datamining_element_form() {
        let xmp = r#"xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/" <plus:DataMining>DMI-PROHIBITED-AIMLTRAINING</plus:DataMining>"#;
        let (canonical, legacy, kind) = helper_parse_dmi(xmp);
        assert_eq!(canonical, Some(DmiValue::ProhibitedAiMlTraining));
        assert_eq!(legacy, None);
        assert_eq!(kind, RightsSignalKind::CanonicalPlusDataMining);
    }

    #[test]
    fn legacy_iptc_dmi_prohibited() {
        let xmp = r#"Iptc4xmpExt:DMI-Prohibited="ProhibitedAiMlTraining""#;
        let (canonical, legacy, kind) = helper_parse_dmi(xmp);
        assert_eq!(canonical, None);
        assert_eq!(legacy, Some(DmiValue::ProhibitedAiMlTraining));
        assert_eq!(kind, RightsSignalKind::LegacyStegoEggoDmi);
    }

    #[test]
    fn legacy_iptc_dmi_allowed() {
        let xmp = r#"Iptc4xmpExt:DMI-Allowed="Allowed""#;
        let (canonical, legacy, kind) = helper_parse_dmi(xmp);
        assert_eq!(canonical, None);
        assert_eq!(legacy, Some(DmiValue::Allowed));
        assert_eq!(kind, RightsSignalKind::LegacyStegoEggoDmi);
    }

    #[test]
    fn unknown_plus_vocab_key_ignored() {
        let xmp = r#"plus:DataMining="DMI-UNKNOWN-VALUE""#;
        let (canonical, legacy, kind) = helper_parse_dmi(xmp);
        assert_eq!(canonical, None);
        assert_eq!(legacy, None);
        assert_eq!(kind, RightsSignalKind::Unknown);
    }

    #[test]
    fn conflict_between_canonical_and_legacy() {
        let nv = NoticeVerification::builder()
            .dmi(Some(DmiValue::ProhibitedAiMlTraining))
            .rights_signal_kind(RightsSignalKind::CanonicalPlusDataMining)
            .canonical_dmi(Some(DmiValue::ProhibitedAiMlTraining))
            .legacy_dmi(Some(DmiValue::Allowed))
            .build();
        assert!(nv.has_dmi_conflict());
    }

    #[test]
    fn no_conflict_when_same_value() {
        let nv = NoticeVerification::builder()
            .dmi(Some(DmiValue::ProhibitedAiMlTraining))
            .rights_signal_kind(RightsSignalKind::CanonicalPlusDataMining)
            .canonical_dmi(Some(DmiValue::ProhibitedAiMlTraining))
            .legacy_dmi(Some(DmiValue::ProhibitedAiMlTraining))
            .build();
        assert!(!nv.has_dmi_conflict());
    }

    #[test]
    fn no_conflict_when_only_canonical() {
        let nv = NoticeVerification::builder()
            .dmi(Some(DmiValue::ProhibitedAiMlTraining))
            .rights_signal_kind(RightsSignalKind::CanonicalPlusDataMining)
            .canonical_dmi(Some(DmiValue::ProhibitedAiMlTraining))
            .build();
        assert!(!nv.has_dmi_conflict());
    }

    #[test]
    fn rights_signal_kind_canonical_preferred_over_legacy() {
        let xmp =
            r#"plus:DataMining="DMI-ALLOWED" Iptc4xmpExt:DMI-Prohibited="ProhibitedAiMlTraining""#;
        let (canonical, legacy, kind) = helper_parse_dmi(xmp);
        assert_eq!(canonical, Some(DmiValue::Allowed));
        assert_eq!(legacy, Some(DmiValue::ProhibitedAiMlTraining));
        assert_eq!(kind, RightsSignalKind::CanonicalPlusDataMining);
    }

    #[test]
    fn tdm_reservation_fallback() {
        let xmp = r#"tdm:reserve_tdm="1""#;
        let (canonical, legacy, kind) = helper_parse_dmi(xmp);
        assert_eq!(canonical, None);
        assert_eq!(legacy, None);
        assert_eq!(kind, RightsSignalKind::LegacyTdmReservation);
    }

    #[test]
    fn canonical_dmi_prefers_attribute_over_element() {
        let xmp = r#"plus:DataMining="DMI-ALLOWED""#;
        let (canonical, _, _) = helper_parse_dmi(xmp);
        assert_eq!(canonical, Some(DmiValue::Allowed));
    }

    #[test]
    fn builder_produces_correct_result_regardless_of_field_order() {
        let a = NoticeVerification::builder()
            .copyright_holder(Some("Alice".into()))
            .dmi(Some(DmiValue::ProhibitedAiMlTraining))
            .rights_signal_kind(RightsSignalKind::CanonicalPlusDataMining)
            .authenticated(true)
            .stego_status(VerificationStatus::Verified)
            .evidence_strength(EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance)
            .channels(vec![EvidenceChannel::DctPayload])
            .build();

        let b = NoticeVerification::builder()
            .evidence_strength(EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance)
            .channels(vec![EvidenceChannel::DctPayload])
            .stego_status(VerificationStatus::Verified)
            .authenticated(true)
            .rights_signal_kind(RightsSignalKind::CanonicalPlusDataMining)
            .dmi(Some(DmiValue::ProhibitedAiMlTraining))
            .copyright_holder(Some("Alice".into()))
            .build();

        assert_eq!(a.copyright_holder(), b.copyright_holder());
        assert_eq!(a.dmi(), b.dmi());
        assert_eq!(a.rights_signal_kind(), b.rights_signal_kind());
        assert_eq!(a.authenticated(), b.authenticated());
        assert_eq!(a.stego_status(), b.stego_status());
        assert_eq!(a.evidence_strength(), b.evidence_strength());
        assert_eq!(a.channels(), b.channels());
    }

    #[test]
    fn builder_defaults_are_sensible() {
        let nv = NoticeVerification::builder().build();
        assert!(nv.copyright_holder().is_none());
        assert!(nv.creator().is_none());
        assert!(nv.contact().is_none());
        assert!(nv.rights_url().is_none());
        assert!(nv.usage_terms().is_none());
        assert!(nv.ai_constraints().is_none());
        assert!(nv.dmi().is_none());
        assert!(nv.tdm_reserved().is_none());
        assert_eq!(nv.rights_signal_kind(), RightsSignalKind::Unknown);
        assert!(nv.canonical_dmi().is_none());
        assert!(nv.legacy_dmi().is_none());
        assert!(nv.protection_seed().is_none());
        assert_eq!(nv.stego_status(), VerificationStatus::NotFound);
        assert!(nv.stego_payload().is_none());
        assert!(!nv.authenticated());
        assert_eq!(nv.evidence_strength(), EvidenceStrength::NoNoticeFound);
        assert!(nv.channels().is_empty());
        assert!(!nv.has_notice());
        assert!(!nv.has_dmi_conflict());
    }
}
