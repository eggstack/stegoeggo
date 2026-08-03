#![allow(deprecated)]

use stegoeggo::{
    process_image_bytes, process_image_bytes_with_warnings, process_request_bytes, resolve_request,
    verify_legal_notice, AuthenticationMode, DmiValue, EvidenceStrength, HiddenMarkerMode,
    ImageOutputFormat, LegalMetadata, MetadataUpdatePolicy, ProtectionChannels, ProtectionContext,
    ProtectionLevel, ProtectionRequest, RightsNotice, RightsPolicy, VerificationStatus,
};

use image::GenericImageView;

fn make_test_image_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn make_test_image_jpeg(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
    buf.into_inner()
}

fn legal_full() -> LegalMetadata {
    LegalMetadata::new()
        .with_copyright_holder("Closure Test Holder")
        .with_creator("Closure Creator")
        .with_usage_terms("All rights reserved")
        .with_credit_line("Photo by Test")
        .with_copyright_owner("Test Owner")
        .with_ai_constraints("No AI")
        .with_contact_email("test@example.com")
        .with_web_statement_of_rights("https://example.com/rights")
}

// ── Container preservation ──────────────────────────────────────────────────

#[test]
fn png_idat_preserved_through_metadata_only() {
    let base = make_test_image_png(64, 64);
    let r1 = image::load_from_memory(&base).unwrap();
    let (w1, h1) = r1.dimensions();

    let legal = legal_full();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal)
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let out = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let r2 = image::load_from_memory(&out).unwrap();
    let (w2, h2) = r2.dimensions();

    assert_eq!((w1, h1), (w2, h2), "PNG dimensions should be preserved");

    let notice = verify_legal_notice(&out, b"");
    assert_eq!(notice.copyright_holder(), Some("Closure Test Holder"));
    assert_eq!(notice.dmi(), Some(DmiValue::ProhibitedAiMlTraining));
}

#[test]
fn jpeg_segment_preserved_through_metadata_injection() {
    let base = make_test_image_jpeg(64, 64);

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal_full())
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let out2 = process_image_bytes(&out1, ProtectionLevel::Standard, &ctx).unwrap();

    let r1 = verify_legal_notice(&out1, b"");
    let r2 = verify_legal_notice(&out2, b"");
    assert_eq!(r1.copyright_holder(), r2.copyright_holder());
    assert_eq!(r1.dmi(), r2.dmi());

    assert!(
        image::load_from_memory(&out1).is_ok(),
        "JPEG out1 should decode"
    );
    assert!(
        image::load_from_memory(&out2).is_ok(),
        "JPEG out2 should decode"
    );
}

#[test]
fn webp_payload_preserved_through_metadata_injection() {
    let base = make_test_image_png(64, 64);

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal_full())
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let out2 = process_image_bytes(&out1, ProtectionLevel::Standard, &ctx).unwrap();

    let r1 = verify_legal_notice(&out1, b"");
    let r2 = verify_legal_notice(&out2, b"");
    assert_eq!(r1.copyright_holder(), r2.copyright_holder());
    assert_eq!(r1.dmi(), r2.dmi());

    assert!(
        image::load_from_memory(&out1).is_ok(),
        "WebP out1 should decode"
    );
    assert!(
        image::load_from_memory(&out2).is_ok(),
        "WebP out2 should decode"
    );
}

// ── Hidden-marker fallback and reporting ────────────────────────────────────

#[test]
fn hidden_marker_disabled_metadata_only() {
    let base = make_test_image_png(64, 64);
    let request = ProtectionRequest::metadata_only(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("Metadata Only")
            .with_usage_terms("Terms"),
    );

    let out = process_request_bytes(&base, &request).unwrap();
    let report = verify_legal_notice(&out, b"");

    assert_eq!(report.copyright_holder(), Some("Metadata Only"));
    assert_eq!(report.dmi(), Some(DmiValue::ProhibitedAiMlTraining));
    assert!(
        image::load_from_memory(&out).is_ok(),
        "metadata-only output should decode"
    );
}

#[test]
fn hidden_marker_enabled_with_key() {
    let base = make_test_image_png(64, 64);
    let key = vec![0xde, 0xad, 0xbe, 0xef, 0x12, 0x34, 0x56, 0x78];
    let request = ProtectionRequest::with_hidden_marker(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_mac_key(key.clone())
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("With Key")
            .with_usage_terms("Terms"),
    );

    let out = process_request_bytes(&base, &request).unwrap();
    let report = verify_legal_notice(&out, &key);

    assert_eq!(report.copyright_holder(), Some("With Key"));
    assert_eq!(report.dmi(), Some(DmiValue::ProhibitedAiMlTraining));
    assert!(
        image::load_from_memory(&out).is_ok(),
        "hidden marker output should decode"
    );
}

#[test]
fn progressive_jpeg_metadata_only_fallback() {
    use jpeg_encoder::Encoder as JpegEnc;

    let img = image::DynamicImage::new_rgb8(64, 64);
    let rgb = img.to_rgb8();
    let mut progressive_buf = Vec::new();
    {
        let mut enc = JpegEnc::new(&mut progressive_buf, 90);
        enc.set_progressive(true);
        enc.encode(rgb.as_raw(), 64, 64, jpeg_encoder::ColorType::Rgb)
            .unwrap();
    }

    let (out, warnings) =
        process_image_bytes_with_warnings(&progressive_buf, ProtectionLevel::Standard, &{
            let mut ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Jpeg);
            ctx = ctx.with_legal_metadata(
                LegalMetadata::new()
                    .with_copyright_holder("Progressive Fallback")
                    .with_usage_terms("Terms"),
            );
            ctx = ctx.with_dmi(DmiValue::ProhibitedAiMlTraining);
            ctx
        })
        .unwrap();

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, stegoeggo::ProtectionWarning::ProgressiveJpegFallback)),
        "should emit ProgressiveJpegFallback"
    );

    let report = verify_legal_notice(&out, b"");
    assert_eq!(report.copyright_holder(), Some("Progressive Fallback"));
}

// ── API equivalence ─────────────────────────────────────────────────────────

#[test]
fn legacy_and_request_api_equivalent_semantics() {
    let base = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("Equiv Holder")
        .with_usage_terms("Equiv Terms")
        .with_creator("Equiv Creator");

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal.clone())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let legacy_out = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();

    let request = ProtectionRequest::with_hidden_marker(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_seed(42)
    .with_intensity(0.5)
    .with_legal_metadata(legal);

    let request_out = process_request_bytes(&base, &request).unwrap();

    let r_legacy = verify_legal_notice(&legacy_out, b"");
    let r_request = verify_legal_notice(&request_out, b"");

    assert_eq!(
        r_legacy.copyright_holder(),
        r_request.copyright_holder(),
        "copyright_holder should match between legacy and request API"
    );
    assert_eq!(
        r_legacy.usage_terms(),
        r_request.usage_terms(),
        "usage_terms should match"
    );
    assert_eq!(
        r_legacy.creator(),
        r_request.creator(),
        "creator should match"
    );
    assert_eq!(r_legacy.dmi(), r_request.dmi(), "DMI should match");
}

#[test]
fn resolve_request_matches_legacy_plan() {
    let request = ProtectionRequest::new(
        RightsNotice::default()
            .with_copyright_holder("Plan Holder")
            .with_usage_terms("Plan Terms"),
        RightsPolicy::ProhibitedAiMlTraining,
        ProtectionChannels::with_hidden_marker(),
    );

    let plan = resolve_request(&request, ImageOutputFormat::Png).unwrap();
    assert!(
        plan.channels().has_stego(),
        "resolved plan should have stego enabled"
    );
}

#[test]
fn conflict_rejection_returns_config_error() {
    let request = ProtectionRequest::new(
        RightsNotice::default(),
        RightsPolicy::Allowed,
        ProtectionChannels {
            rights_metadata: true,
            hidden_marker: HiddenMarkerMode::Disabled,
            authentication: AuthenticationMode::Hmac,
        },
    )
    .with_mac_key(b"key".to_vec());

    let result = resolve_request(&request, ImageOutputFormat::Png);
    assert!(
        result.is_err(),
        "HMAC without hidden marker should be config error"
    );
}

#[test]
fn three_rounds_no_unbounded_growth_all_formats() {
    let base = make_test_image_png(64, 64);
    let legal = legal_full();

    for fmt in [
        ImageOutputFormat::Png,
        ImageOutputFormat::Jpeg,
        ImageOutputFormat::WebP,
    ] {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(fmt)
            .with_legal_metadata(legal.clone())
            .with_dmi(DmiValue::ProhibitedAiMlTraining);

        let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
        let out2 = process_image_bytes(&out1, ProtectionLevel::Standard, &ctx).unwrap();
        let out3 = process_image_bytes(&out2, ProtectionLevel::Standard, &ctx).unwrap();

        let ratio_1_2 = out2.len() as f64 / out1.len() as f64;
        let ratio_2_3 = out3.len() as f64 / out2.len() as f64;

        assert!(
            ratio_1_2 < 1.2,
            "{:?}: out2/out1 too large: {:.3} ({} / {})",
            fmt,
            ratio_1_2,
            out2.len(),
            out1.len()
        );
        assert!(
            ratio_2_3 < 1.2,
            "{:?}: out3/out2 too large: {:.3} ({} / {})",
            fmt,
            ratio_2_3,
            out3.len(),
            out2.len()
        );

        assert!(
            image::load_from_memory(&out1).is_ok(),
            "{:?}: out1 decode",
            fmt
        );
        assert!(
            image::load_from_memory(&out2).is_ok(),
            "{:?}: out2 decode",
            fmt
        );
        assert!(
            image::load_from_memory(&out3).is_ok(),
            "{:?}: out3 decode",
            fmt
        );
    }
}

#[test]
fn pixel_only_api_does_not_inject_metadata() {
    let img = image::DynamicImage::new_rgb8(64, 64);
    let ctx = ProtectionContext::new(0.5, 42);

    let protected = stegoeggo::process_image(img, ProtectionLevel::Standard, &ctx).unwrap();

    let mut buf = std::io::Cursor::new(Vec::new());
    protected
        .write_to(&mut buf, image::ImageFormat::Png)
        .unwrap();
    let out_bytes = buf.into_inner();

    let report = verify_legal_notice(&out_bytes, b"");
    assert_eq!(
        report.copyright_holder(),
        None,
        "pixel-only API should not inject metadata"
    );
}

// ── Phase 4.3: Authentication distinctions ───────────────────────────────────

#[test]
fn auth_crc_best_effort_marker_verified() {
    let base = make_test_image_png(64, 64);
    let request = ProtectionRequest::with_hidden_marker(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_seed(42)
    .with_intensity(0.5)
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("CRC Holder")
            .with_usage_terms("Terms"),
    );

    let out = process_request_bytes(&base, &request).unwrap();
    let report = verify_legal_notice(&out, b"");

    assert_eq!(report.copyright_holder(), Some("CRC Holder"));
    assert_eq!(report.dmi(), Some(DmiValue::ProhibitedAiMlTraining));
    assert!(
        !report.authenticated(),
        "CRC verification should not be authenticated"
    );
    assert_eq!(
        report.stego_status(),
        VerificationStatus::Verified,
        "CRC marker should be verified"
    );
}

#[test]
fn auth_hmac_authenticated_marker_verified() {
    let base = make_test_image_png(64, 64);
    let key = vec![0xde, 0xad, 0xbe, 0xef, 0x12, 0x34, 0x56, 0x78];
    let request = ProtectionRequest::with_hidden_marker(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_mac_key(key.clone())
    .with_seed(42)
    .with_intensity(0.5)
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("HMAC Holder")
            .with_usage_terms("Terms"),
    );

    let out = process_request_bytes(&base, &request).unwrap();
    let report = verify_legal_notice(&out, &key);

    assert_eq!(report.copyright_holder(), Some("HMAC Holder"));
    assert_eq!(report.dmi(), Some(DmiValue::ProhibitedAiMlTraining));
    assert!(
        report.authenticated(),
        "HMAC verification should be authenticated"
    );
    assert_eq!(report.stego_status(), VerificationStatus::Verified);
    assert_eq!(
        report.evidence_strength(),
        EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance,
        "HMAC should yield strongest evidence"
    );
}

#[test]
fn auth_marker_present_but_key_missing() {
    let base = make_test_image_png(64, 64);
    let key = vec![0xde, 0xad, 0xbe, 0xef, 0x12, 0x34, 0x56, 0x78];
    let request = ProtectionRequest::with_hidden_marker(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_mac_key(key.clone())
    .with_seed(42)
    .with_intensity(0.5)
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("Key Missing Holder")
            .with_usage_terms("Terms"),
    );

    let out = process_request_bytes(&base, &request).unwrap();
    let report = verify_legal_notice(&out, b"");

    assert_eq!(report.copyright_holder(), Some("Key Missing Holder"));
    assert_eq!(report.dmi(), Some(DmiValue::ProhibitedAiMlTraining));
    assert!(!report.authenticated(), "empty key should not authenticate");
}

#[test]
fn auth_marker_authentication_failed_wrong_key() {
    let base = make_test_image_png(64, 64);
    let key = vec![0xde, 0xad, 0xbe, 0xef, 0x12, 0x34, 0x56, 0x78];
    let request = ProtectionRequest::with_hidden_marker(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_mac_key(key.clone())
    .with_seed(42)
    .with_intensity(0.5)
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("Wrong Key Holder")
            .with_usage_terms("Terms"),
    );

    let out = process_request_bytes(&base, &request).unwrap();
    let wrong_key = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let report = verify_legal_notice(&out, &wrong_key);

    assert_eq!(report.copyright_holder(), Some("Wrong Key Holder"));
    assert_eq!(report.dmi(), Some(DmiValue::ProhibitedAiMlTraining));
    assert!(!report.authenticated(), "wrong key should not authenticate");
}

#[test]
fn auth_marker_not_found_on_unprotected_image() {
    let base = make_test_image_png(64, 64);
    let report = verify_legal_notice(&base, b"");

    assert_ne!(
        report.stego_status(),
        VerificationStatus::Verified,
        "unprotected image should not be Verified"
    );
    assert!(!report.authenticated());
    assert_eq!(
        report.evidence_strength(),
        EvidenceStrength::NoNoticeFound,
        "no notice on unprotected image"
    );
}

#[test]
fn auth_marker_fallback_on_unsupported_jpeg() {
    use jpeg_encoder::Encoder as JpegEnc;

    let img = image::DynamicImage::new_rgb8(64, 64);
    let rgb = img.to_rgb8();
    let mut progressive_buf = Vec::new();
    {
        let mut enc = JpegEnc::new(&mut progressive_buf, 90);
        enc.set_progressive(true);
        enc.encode(rgb.as_raw(), 64, 64, jpeg_encoder::ColorType::Rgb)
            .unwrap();
    }

    let legal = LegalMetadata::new()
        .with_copyright_holder("Fallback Auth Holder")
        .with_usage_terms("Terms");

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let (output, warnings) =
        process_image_bytes_with_warnings(&progressive_buf, ProtectionLevel::Standard, &ctx)
            .unwrap();

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, stegoeggo::ProtectionWarning::ProgressiveJpegFallback)),
        "progressive JPEG should emit fallback warning"
    );

    let report = verify_legal_notice(&output, b"");
    assert_eq!(report.copyright_holder(), Some("Fallback Auth Holder"));
    assert_eq!(report.dmi(), Some(DmiValue::ProhibitedAiMlTraining));
}

// ── Phase 3.3: PreserveExisting and FailOnConflict ──────────────────────────

#[test]
fn preserve_existing_does_not_overwrite_protected_seed() {
    let base = make_test_image_png(64, 64);
    let legal_v1 = LegalMetadata::new()
        .with_copyright_holder("Original Holder")
        .with_usage_terms("Original Terms");

    let ctx_v1 = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_v1)
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx_v1).unwrap();

    let report1 = verify_legal_notice(&out1, b"");
    assert_eq!(report1.copyright_holder(), Some("Original Holder"));
    assert_eq!(report1.usage_terms(), Some("Original Terms"));

    let legal_v2 = LegalMetadata::new()
        .with_copyright_holder("New Holder")
        .with_usage_terms("New Terms");

    let request = ProtectionRequest::metadata_only(
        RightsNotice::default(),
        RightsPolicy::ProhibitedGenerativeAiTraining,
    )
    .with_metadata_update_policy(MetadataUpdatePolicy::PreserveExisting)
    .with_legal_metadata(legal_v2);

    let out2 = process_request_bytes(&out1, &request).unwrap();

    let report2 = verify_legal_notice(&out2, b"");
    assert!(
        report2.has_notice(),
        "PreserveExisting should produce valid output"
    );
    assert!(
        image::load_from_memory(&out2).is_ok(),
        "PreserveExisting output should decode"
    );
}

#[test]
fn preserve_existing_adds_missing_fields() {
    let base = make_test_image_png(64, 64);
    let legal_v1 = LegalMetadata::new().with_copyright_holder("Original Holder");

    let ctx_v1 = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_v1)
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx_v1).unwrap();

    let report1 = verify_legal_notice(&out1, b"");
    assert_eq!(report1.copyright_holder(), Some("Original Holder"));
    assert_eq!(report1.usage_terms(), None);

    let legal_v2 = LegalMetadata::new()
        .with_copyright_holder("Original Holder")
        .with_usage_terms("Added Terms");

    let request = ProtectionRequest::metadata_only(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_metadata_update_policy(MetadataUpdatePolicy::PreserveExisting)
    .with_legal_metadata(legal_v2);

    let out2 = process_request_bytes(&out1, &request).unwrap();

    let report2 = verify_legal_notice(&out2, b"");
    assert!(
        report2.has_notice(),
        "PreserveExisting should produce valid output"
    );
    assert!(
        image::load_from_memory(&out2).is_ok(),
        "PreserveExisting output should decode"
    );
}

#[test]
fn fail_on_conflict_returns_error_when_stego_exists() {
    let base = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("Conflict Holder")
        .with_usage_terms("Terms");

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal)
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();

    let request = ProtectionRequest::metadata_only(
        RightsNotice::default(),
        RightsPolicy::ProhibitedGenerativeAiTraining,
    )
    .with_metadata_update_policy(MetadataUpdatePolicy::FailOnConflict)
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("New Holder")
            .with_usage_terms("New Terms"),
    );

    let result = process_request_bytes(&out1, &request);
    assert!(
        result.is_err(),
        "FailOnConflict should return error when StegoEggo metadata already present"
    );
}

#[test]
fn fail_on_conflict_succeeds_on_clean_image() {
    let base = make_test_image_png(64, 64);

    let request = ProtectionRequest::metadata_only(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_metadata_update_policy(MetadataUpdatePolicy::FailOnConflict)
    .with_legal_metadata(
        LegalMetadata::new()
            .with_copyright_holder("Clean Holder")
            .with_usage_terms("Terms"),
    );

    let result = process_request_bytes(&base, &request);
    assert!(
        result.is_ok(),
        "FailOnConflict should succeed on image without existing StegoEggo metadata"
    );
}

#[test]
fn preserve_existing_cross_format_jpeg() {
    let base = make_test_image_jpeg(64, 64);
    let legal_v1 = LegalMetadata::new().with_copyright_holder("JPEG Original");

    let ctx_v1 = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal_v1)
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx_v1).unwrap();

    let report1 = verify_legal_notice(&out1, b"");
    assert_eq!(report1.copyright_holder(), Some("JPEG Original"));

    let legal_v2 = LegalMetadata::new()
        .with_copyright_holder("JPEG Original")
        .with_usage_terms("JPEG Added");

    let request = ProtectionRequest::metadata_only(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_metadata_update_policy(MetadataUpdatePolicy::PreserveExisting)
    .with_legal_metadata(legal_v2);

    let out2 = process_request_bytes(&out1, &request).unwrap();

    let report2 = verify_legal_notice(&out2, b"");
    assert!(
        report2.has_notice(),
        "JPEG PreserveExisting should produce valid output"
    );
    assert!(
        image::load_from_memory(&out2).is_ok(),
        "JPEG PreserveExisting output should decode"
    );
}

#[test]
fn preserve_existing_cross_format_webp() {
    let base = make_test_image_png(64, 64);
    let legal_v1 = LegalMetadata::new().with_copyright_holder("WebP Original");

    let ctx_v1 = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal_v1)
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx_v1).unwrap();

    let report1 = verify_legal_notice(&out1, b"");
    assert_eq!(report1.copyright_holder(), Some("WebP Original"));

    let legal_v2 = LegalMetadata::new()
        .with_copyright_holder("WebP Original")
        .with_usage_terms("WebP Added");

    let request = ProtectionRequest::metadata_only(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_metadata_update_policy(MetadataUpdatePolicy::PreserveExisting)
    .with_legal_metadata(legal_v2);

    let out2 = process_request_bytes(&out1, &request).unwrap();

    let report2 = verify_legal_notice(&out2, b"");
    assert!(
        report2.has_notice(),
        "WebP PreserveExisting should produce valid output"
    );
    assert!(
        image::load_from_memory(&out2).is_ok(),
        "WebP PreserveExisting output should decode"
    );
}

// ── Phase 5.2: CLI equivalence ──────────────────────────────────────────────

#[test]
fn cli_dmi_flag_matches_api_dmi_value() {
    let base = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("CLI Test Holder")
        .with_usage_terms("CLI Terms");

    let ctx_api = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal.clone())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let api_out = process_image_bytes(&base, ProtectionLevel::Standard, &ctx_api).unwrap();

    let request = ProtectionRequest::with_hidden_marker(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_seed(42)
    .with_intensity(0.5)
    .with_legal_metadata(legal);

    let request_out = process_request_bytes(&base, &request).unwrap();

    let r_api = verify_legal_notice(&api_out, b"");
    let r_request = verify_legal_notice(&request_out, b"");

    assert_eq!(
        r_api.copyright_holder(),
        r_request.copyright_holder(),
        "API and request should produce same copyright"
    );
    assert_eq!(
        r_api.usage_terms(),
        r_request.usage_terms(),
        "API and request should produce same usage_terms"
    );
    assert_eq!(
        r_api.dmi(),
        r_request.dmi(),
        "API and request should produce same DMI"
    );
}

#[test]
fn cli_metadata_only_matches_api_metadata_only() {
    let base = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("Metadata Only Holder")
        .with_usage_terms("Metadata Only Terms");

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal.clone())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let api_out = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();

    let request = ProtectionRequest::metadata_only(
        RightsNotice::default(),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_legal_metadata(legal);

    let request_out = process_request_bytes(&base, &request).unwrap();

    let r_api = verify_legal_notice(&api_out, b"");
    let r_request = verify_legal_notice(&request_out, b"");

    assert_eq!(r_api.copyright_holder(), r_request.copyright_holder());
    assert_eq!(r_api.usage_terms(), r_request.usage_terms());
    assert_eq!(r_api.dmi(), r_request.dmi());
}
