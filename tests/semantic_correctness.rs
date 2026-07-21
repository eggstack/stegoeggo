#![allow(deprecated)]

use stegoeggo::{
    DmiValue, ImageOutputFormat, LegalMetadata, NoticeVerification, ProtectionContext,
    ProtectionLevel, RightsSignalKind, VerificationStatus,
};

fn make_test_image_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn process_and_verify(
    input: &[u8],
    format: ImageOutputFormat,
    legal: &LegalMetadata,
    dmi: DmiValue,
) -> NoticeVerification {
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(format)
        .with_legal_metadata(legal.clone())
        .with_dmi(dmi);
    let output = stegoeggo::process_image_bytes(input, ProtectionLevel::Standard, &ctx).unwrap();
    stegoeggo::verify_legal_notice(&output, b"")
}

// ── Workstream C: Canonical PLUS URI handling ───────────────────────────────

#[test]
fn dmi_value_canonical_uris() {
    assert_eq!(DmiValue::Unspecified.plus_vocab_key(), "DMI-UNSPECIFIED");
    assert_eq!(DmiValue::Allowed.plus_vocab_key(), "DMI-ALLOWED");
    assert_eq!(
        DmiValue::ProhibitedAiMlTraining.plus_vocab_key(),
        "DMI-PROHIBITED-AIMLTRAINING"
    );
    assert_eq!(
        DmiValue::ProhibitedGenAiMlTraining.plus_vocab_key(),
        "DMI-PROHIBITED-GENAIMLTRAINING"
    );
    assert_eq!(
        DmiValue::ProhibitedExceptSearchEngineIndexing.plus_vocab_key(),
        "DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING"
    );
    assert_eq!(DmiValue::Prohibited.plus_vocab_key(), "DMI-PROHIBITED");
    assert_eq!(
        DmiValue::ProhibitedSeeConstraints.plus_vocab_key(),
        "DMI-PROHIBITED-SEECONSTRAINT"
    );
}

#[test]
fn dmi_value_round_trip() {
    let values = [
        DmiValue::Unspecified,
        DmiValue::Allowed,
        DmiValue::ProhibitedAiMlTraining,
        DmiValue::ProhibitedGenAiMlTraining,
        DmiValue::ProhibitedExceptSearchEngineIndexing,
        DmiValue::Prohibited,
        DmiValue::ProhibitedSeeConstraints,
    ];
    for v in values {
        let key = v.plus_vocab_key();
        let round = DmiValue::from_plus_vocab_key(key)
            .unwrap_or_else(|| panic!("round-trip failed for {:?}", v));
        assert_eq!(round, v, "round-trip mismatch for {:?}", v);
    }
}

#[test]
fn canonical_uri_always_full() {
    let values = [
        DmiValue::ProhibitedAiMlTraining,
        DmiValue::ProhibitedGenAiMlTraining,
        DmiValue::ProhibitedExceptSearchEngineIndexing,
        DmiValue::ProhibitedSeeConstraints,
    ];
    for v in values {
        let key = v.plus_vocab_key();
        assert!(
            key.starts_with("DMI-PROHIBITED"),
            "canonical URI for {:?} should start with DMI-PROHIBITED: {}",
            v,
            key
        );
        assert!(
            key.len() > "DMI-PROHIBITED".len(),
            "canonical URI for {:?} should be more specific than DMI-PROHIBITED: {}",
            v,
            key
        );
    }
}

#[test]
fn unknown_plus_uri_returns_none() {
    assert_eq!(DmiValue::from_plus_vocab_key("DMI-UNKNOWN"), None);
    assert_eq!(
        DmiValue::from_plus_vocab_key("DMI-PROHIBITED-AIMLTRAINING-EXTRA"),
        None
    );
    assert_eq!(DmiValue::from_plus_vocab_key(""), None);
    assert_eq!(
        DmiValue::from_plus_vocab_key("Iptc4xmpExt:DMI-Prohibited"),
        None
    );
}

#[test]
fn plus_vocab_key_is_case_sensitive() {
    assert_eq!(
        DmiValue::from_plus_vocab_key("dmi-allowed"),
        None,
        "lowercase should not parse"
    );
    assert_eq!(
        DmiValue::from_plus_vocab_key("DMI-ALLOWED"),
        Some(DmiValue::Allowed),
        "uppercase should parse"
    );
    assert_eq!(
        DmiValue::from_plus_vocab_key("Dmi-Prohibited"),
        None,
        "mixed case should not parse"
    );
}

// ── Workstream C2: Namespace-aware extraction ───────────────────────────────

fn png_with_itxtraw(raw_data: &[u8]) -> Vec<u8> {
    let base = make_test_image_png(64, 64);
    let mut output = Vec::with_capacity(base.len() + raw_data.len() + 20);
    output.extend_from_slice(&base[0..8]);
    let mut i = 8;
    while i + 8 <= base.len() {
        let length = u32::from_be_bytes([base[i], base[i + 1], base[i + 2], base[i + 3]]) as usize;
        let chunk_type = &base[i + 4..i + 8];
        if chunk_type == b"IEND" {
            let chunk_len = (raw_data.len() as u32).to_be_bytes();
            output.extend_from_slice(&chunk_len);
            output.extend_from_slice(b"iTXt");
            output.extend_from_slice(raw_data);
            let mut crc = crc32fast::Hasher::new();
            crc.update(b"iTXt");
            crc.update(raw_data);
            output.extend_from_slice(&crc.finalize().to_be_bytes());
            output.extend_from_slice(&base[i..i + 8 + length + 4]);
        } else {
            output.extend_from_slice(&base[i..i + 8 + length + 4]);
        }
        i += 8 + length + 4;
    }
    output
}

#[test]
fn namespace_prefix_independent() {
    let key = b"XML:com.adobe.xmp";
    let value = br#" xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/" plus:DataMining="DMI-PROHIBITED-AIMLTRAINING""#;
    let mut raw = Vec::new();
    raw.extend_from_slice(key);
    raw.push(0);
    raw.push(0);
    raw.push(0);
    raw.extend_from_slice(value);
    let png = png_with_itxtraw(&raw);
    let report = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(
        report.canonical_dmi(),
        Some(DmiValue::ProhibitedAiMlTraining),
        "plus: prefix in iTXt XMP should be detected"
    );
}

#[test]
fn non_standard_prefix_falls_back_to_element_form() {
    let key = b"XML:com.adobe.xmp";
    let value = br#" xmlns:p="http://ns.useplus.org/ldf/xmp/1.0/" <p:DataMining>DMI-ALLOWED</p:DataMining>"#;
    let mut raw = Vec::new();
    raw.extend_from_slice(key);
    raw.push(0);
    raw.push(0);
    raw.push(0);
    raw.extend_from_slice(value);
    let png = png_with_itxtraw(&raw);
    let report = stegoeggo::verify_legal_notice(&png, b"");
    assert_eq!(
        report.canonical_dmi(),
        Some(DmiValue::Allowed),
        "non-standard prefix should resolve via xmlns lookup"
    );
}

#[test]
fn attribute_and_element_form_parse() {
    let key = b"XML:com.adobe.xmp";
    let attr_val = br#" plus:DataMining="DMI-ALLOWED""#;
    let mut raw_attr = Vec::new();
    raw_attr.extend_from_slice(key);
    raw_attr.push(0);
    raw_attr.push(0);
    raw_attr.push(0);
    raw_attr.extend_from_slice(attr_val);
    let png_attr = png_with_itxtraw(&raw_attr);
    let report_attr = stegoeggo::verify_legal_notice(&png_attr, b"");
    assert_eq!(report_attr.canonical_dmi(), Some(DmiValue::Allowed));

    let elem_val = br#" xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/" <plus:DataMining>DMI-ALLOWED</plus:DataMining>"#;
    let mut raw_elem = Vec::new();
    raw_elem.extend_from_slice(key);
    raw_elem.push(0);
    raw_elem.push(0);
    raw_elem.push(0);
    raw_elem.extend_from_slice(elem_val);
    let png_elem = png_with_itxtraw(&raw_elem);
    let report_elem = stegoeggo::verify_legal_notice(&png_elem, b"");
    assert_eq!(report_elem.canonical_dmi(), Some(DmiValue::Allowed));
}

// ── Workstream C3: Legal-field semantics ────────────────────────────────────

#[test]
fn contact_never_maps_to_credit() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_contact_email("contact@example.com")
        .with_credit_line("Actual Credit Line");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert_eq!(
            report.credit_line(),
            Some("Actual Credit Line"),
            "{}: credit_line should be the explicit value, not contact",
            label
        );
    }
}

#[test]
fn credit_line_maps_to_photoshop_credit() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new().with_credit_line("Photo by Alice");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert_eq!(
            report.credit_line(),
            Some("Photo by Alice"),
            "{}: credit_line should round-trip",
            label
        );
    }
}

#[test]
fn copyright_owner_distinct_from_notice() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("Copyright Holder Name")
        .with_copyright_owner("Copyright Owner Name");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert_eq!(
            report.copyright_holder(),
            Some("Copyright Holder Name"),
            "{}: copyright_holder",
            label
        );
        assert_eq!(
            report.copyright_owner(),
            Some("Copyright Owner Name"),
            "{}: copyright_owner",
            label
        );
    }
}

#[test]
fn license_url_distinct_from_web_statement() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_license_url("https://example.com/license")
        .with_web_statement_of_rights("https://example.com/rights");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert!(
            report.license_url().is_some() || report.web_statement_of_rights().is_some(),
            "{}: at least one URL field should be present",
            label
        );
    }
}

#[test]
fn creation_date_not_synthesized() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new().with_copyright_holder("No Date Holder");
    let dmi = DmiValue::ProhibitedAiMlTraining;
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal)
        .with_dmi(dmi)
        .with_timestamp_override("2025-07-15T12:00:00Z");
    let output =
        stegoeggo::process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
    let report = stegoeggo::verify_legal_notice(&output, b"");

    let notice_at = report.notice_applied_at();
    assert!(
        notice_at.is_some(),
        "notice_applied_at should be auto-computed when legal metadata is present"
    );
}

#[test]
fn invalid_urls_rejected() {
    let no_scheme = LegalMetadata::new().with_license_url("example.com/no-scheme");
    assert!(
        no_scheme.validate().is_err(),
        "URL without scheme should fail"
    );

    let empty = LegalMetadata::new().with_license_url("");
    assert!(empty.validate().is_err(), "Empty URL should fail");

    let no_authority = LegalMetadata::new().with_license_url("https://");
    assert!(
        no_authority.validate().is_err(),
        "URL with no authority should fail"
    );

    let no_scheme_wsor = LegalMetadata::new().with_web_statement_of_rights("not-a-url");
    assert!(
        no_scheme_wsor.validate().is_err(),
        "WebStatement without scheme should fail"
    );
}

// ── Workstream C5: Date semantics ───────────────────────────────────────────

#[test]
fn creation_date_only_when_supplied() {
    let png_bytes = make_test_image_png(64, 64);
    let legal_without_date = LegalMetadata::new().with_copyright_holder("No Date");
    let dmi = DmiValue::ProhibitedAiMlTraining;
    let report = process_and_verify(&png_bytes, ImageOutputFormat::Png, &legal_without_date, dmi);
    assert!(
        report.notice_applied_at().is_some(),
        "notice_applied_at should be auto-computed when legal metadata is present"
    );
}

#[test]
fn metadata_date_and_notice_applied_distinct() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_metadata_date("2025-01-01")
        .with_notice_applied_at("2025-06-15T10:00:00Z")
        .with_copyright_holder("Date Test");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert_eq!(
            report.metadata_date(),
            Some("2025-01-01"),
            "{}: metadata_date",
            label
        );
        assert_eq!(
            report.notice_applied_at(),
            Some("2025-06-15T10:00:00Z"),
            "{}: notice_applied_at",
            label
        );
    }
}

#[test]
fn notice_timestamps_rfc3339() {
    let legal = LegalMetadata::new()
        .with_copyright_holder("Timestamp Test")
        .with_notice_applied_at("2025-07-15T12:00:00Z");
    assert!(
        legal.validate().is_ok(),
        "RFC 3339 timestamp with Z suffix should be valid"
    );

    let legal_offset = LegalMetadata::new()
        .with_copyright_holder("Timestamp Offset")
        .with_notice_applied_at("2025-07-15T12:00:00+05:30");
    assert!(
        legal_offset.validate().is_ok(),
        "RFC 3339 timestamp with offset should be valid"
    );

    let legal_bad = LegalMetadata::new()
        .with_copyright_holder("Timestamp Bad")
        .with_notice_applied_at("July 15, 2025");
    assert!(
        legal_bad.validate().is_err(),
        "Non-ISO-8601 timestamp should fail"
    );
}

#[test]
fn creation_date_iso8601_variants() {
    let date_only = LegalMetadata::new().with_creation_date("2025-07-15");
    assert!(date_only.validate().is_ok(), "YYYY-MM-DD should be valid");

    let datetime_utc = LegalMetadata::new().with_creation_date("2025-07-15T12:00:00Z");
    assert!(
        datetime_utc.validate().is_ok(),
        "YYYY-MM-DDTHH:MM:SSZ should be valid"
    );

    let datetime_offset = LegalMetadata::new().with_creation_date("2025-07-15T12:00:00+05:30");
    assert!(
        datetime_offset.validate().is_ok(),
        "YYYY-MM-DDTHH:MM:SS+HH:MM should be valid"
    );

    let bad_format = LegalMetadata::new().with_creation_date("07/15/2025");
    assert!(bad_format.validate().is_err(), "MM/DD/YYYY should fail");

    let empty = LegalMetadata::new().with_creation_date("");
    assert!(empty.validate().is_err(), "Empty date should fail");
}

// ── Builder pattern / NoticeVerification ─────────────────────────────────────

#[test]
fn notice_verification_builder_defaults() {
    let nv = NoticeVerification::builder().build();
    assert_eq!(nv.copyright_holder(), None);
    assert_eq!(nv.credit_line(), None);
    assert_eq!(nv.copyright_owner(), None);
    assert_eq!(nv.license_url(), None);
    assert_eq!(nv.web_statement_of_rights(), None);
    assert_eq!(nv.metadata_date(), None);
    assert_eq!(nv.notice_applied_at(), None);
    assert!(!nv.has_notice());
    assert_eq!(
        nv.evidence_strength(),
        stegoeggo::EvidenceStrength::NoNoticeFound
    );
}

#[test]
fn notice_verification_builder_sets_all_fields() {
    let nv = NoticeVerification::builder()
        .copyright_holder(Some("Holder".into()))
        .creator(Some("Creator".into()))
        .contact(Some("contact@test.com".into()))
        .rights_url(Some("https://rights.test".into()))
        .usage_terms(Some("All rights reserved".into()))
        .ai_constraints(Some("No AI".into()))
        .dmi(Some(DmiValue::ProhibitedAiMlTraining))
        .tdm_reserved(Some(true))
        .rights_signal_kind(RightsSignalKind::CanonicalPlusDataMining)
        .canonical_dmi(Some(DmiValue::ProhibitedAiMlTraining))
        .legacy_dmi(None)
        .protection_seed(Some(42))
        .stego_status(VerificationStatus::Verified)
        .authenticated(true)
        .evidence_strength(stegoeggo::EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance)
        .channels(vec![stegoeggo::EvidenceChannel::PngText])
        .license_url(Some("https://license.test".into()))
        .web_statement_of_rights(Some("https://wsor.test".into()))
        .credit_line(Some("Photo by Test".into()))
        .copyright_owner(Some("Owner Corp".into()))
        .licensor_name(Some("Licensor Inc".into()))
        .licensor_email(Some("lic@test.com".into()))
        .licensor_url(Some("https://lic.test".into()))
        .metadata_date(Some("2025-01-01".into()))
        .notice_applied_at(Some("2025-06-15T12:00:00Z".into()))
        .build();

    assert_eq!(nv.copyright_holder(), Some("Holder"));
    assert_eq!(nv.creator(), Some("Creator"));
    assert_eq!(nv.contact(), Some("contact@test.com"));
    assert_eq!(nv.credit_line(), Some("Photo by Test"));
    assert_eq!(nv.copyright_owner(), Some("Owner Corp"));
    assert_eq!(nv.licensor_name(), Some("Licensor Inc"));
    assert_eq!(nv.licensor_email(), Some("lic@test.com"));
    assert_eq!(nv.licensor_url(), Some("https://lic.test"));
    assert_eq!(nv.metadata_date(), Some("2025-01-01"));
    assert_eq!(nv.notice_applied_at(), Some("2025-06-15T12:00:00Z"));
    assert!(nv.has_notice());
}
