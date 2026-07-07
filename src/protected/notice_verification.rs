use crate::protected::steganography::SteganographyProtector;
use crate::types::{
    DmiValue, EvidenceChannel, EvidenceStrength, NoticeVerification, VerificationStatus,
};

type NoticeFields = (
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

    let (copyright_holder, creator, contact, rights_url, usage_terms, ai_constraints) = match format
    {
        Some(Format::Png) => {
            let result = extract_png_notice(img_bytes, &mut channels, &mut seed);
            extract_xmp_dmi_from_png(img_bytes, &mut dmi, &mut tdm_reserved);
            result
        }
        Some(Format::Jpeg) => {
            let result = extract_jpeg_notice(img_bytes, &mut channels, &mut seed);
            extract_xmp_dmi_from_jpeg(img_bytes, &mut dmi, &mut tdm_reserved);
            result
        }
        Some(Format::WebP) => {
            let result = extract_webp_notice(img_bytes, &mut channels, &mut seed);
            extract_xmp_dmi_from_webp(img_bytes, &mut dmi, &mut tdm_reserved);
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
        || dmi.is_some();

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

    NoticeVerification::new(
        copyright_holder,
        creator,
        contact,
        rights_url,
        usage_terms,
        ai_constraints,
        dmi,
        tdm_reserved,
        seed,
        stego_status,
        stego_payload,
        authenticated,
        evidence_strength,
        channels,
    )
}

fn empty_report() -> NoticeVerification {
    NoticeVerification::new(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        VerificationStatus::NotFound,
        None,
        false,
        EvidenceStrength::NoNoticeFound,
        Vec::new(),
    )
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
                        rights_url = Some(value_str);
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
                        rights_url = Some(value_str);
                        found_text = true;
                    }
                    b"Creator" => {
                        creator = Some(value_str);
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
    )
}

fn extract_xmp_dmi_from_png(
    png_data: &[u8],
    dmi: &mut Option<DmiValue>,
    tdm_reserved: &mut Option<bool>,
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
                            parse_xmp_for_dmi(xmp_str, dmi, tdm_reserved);
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
                        rights_url = Some(v);
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
                        rights_url = Some(v);
                        found_comment = true;
                    }
                    Some(ComField::Creator(v)) => {
                        creator = Some(v);
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
                .windows(29)
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
    )
}

fn extract_xmp_dmi_from_jpeg(
    jpeg_data: &[u8],
    dmi: &mut Option<DmiValue>,
    tdm_reserved: &mut Option<bool>,
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
                .windows(29)
                .any(|w| w == b"http://ns.adobe.com/xap/1.0/")
            {
                if let Ok(xmp_str) = std::str::from_utf8(segment_data) {
                    parse_xmp_for_dmi(xmp_str, dmi, tdm_reserved);
                }
            }
        }

        pos = segment_end;
    }
}

fn extract_webp_notice(
    webp_data: &[u8],
    channels: &mut Vec<EvidenceChannel>,
    seed: &mut Option<u64>,
) -> NoticeFields {
    // WebP legal fields (Copyright, Contact, etc.) are not injected as tEXt by stegoeggo.
    // Only DMI/TDM/seed are embedded in XMP and EXIF chunks.
    let copyright_holder: Option<String> = None;
    let creator: Option<String> = None;
    let contact: Option<String> = None;
    let rights_url: Option<String> = None;
    let usage_terms: Option<String> = None;
    let ai_constraints: Option<String> = None;

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

    // WebP legal fields are not injected as tEXt — they're in XMP only.
    // The XMP generated by stegoeggo contains DMI/TDM properties but not
    // the individual legal claim fields (Copyright, Contact, etc.).
    // So for WebP, we don't extract those fields from the image.

    (
        copyright_holder,
        creator,
        contact,
        rights_url,
        usage_terms,
        ai_constraints,
    )
}

fn extract_xmp_dmi_from_webp(
    webp_data: &[u8],
    dmi: &mut Option<DmiValue>,
    tdm_reserved: &mut Option<bool>,
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
                parse_xmp_for_dmi(xmp_str, dmi, tdm_reserved);
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
    None
}

fn parse_xmp_for_dmi(xmp_str: &str, dmi: &mut Option<DmiValue>, tdm_reserved: &mut Option<bool>) {
    if dmi.is_none() {
        if let Some(val) = extract_xmp_attr(xmp_str, "Iptc4xmpExt:DataMiningAttribute") {
            *dmi = parse_dmi_value(&val);
        }
    }
    if dmi.is_none() {
        if let Some(val) = extract_xmp_attr(xmp_str, "Iptc4xmpExt:DMI-Prohibited") {
            *dmi = parse_dmi_value(&val);
        }
    }
    if dmi.is_none() {
        if let Some(val) = extract_xmp_attr(xmp_str, "Iptc4xmpExt:DMI-Allowed") {
            *dmi = parse_dmi_value(&val);
        }
    }
    if dmi.is_none() {
        if let Some(val) = extract_xmp_attr(xmp_str, "Iptc4xmpExt:DMI") {
            *dmi = parse_dmi_value(&val);
        }
    }
    if tdm_reserved.is_none() {
        if let Some(val) = extract_xmp_attr(xmp_str, "tdm:reserve_tdm") {
            *tdm_reserved = Some(val == "1");
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
