#![allow(deprecated)]

use stegoeggo::{
    process_image_bytes, process_image_bytes_with_warnings, verify_legal_notice, DmiValue,
    ImageOutputFormat, LegalMetadata, MetadataUpdatePolicy, ProtectionContext, ProtectionLevel,
};

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

fn legal_a() -> LegalMetadata {
    LegalMetadata::new()
        .with_copyright_holder("Holder A")
        .with_usage_terms("Terms A")
        .with_creator("Creator A")
        .with_credit_line("Credit A")
        .with_copyright_owner("Owner A")
        .with_ai_constraints("No AI A")
        .with_contact_email("a@example.com")
        .with_web_statement_of_rights("https://a.example.com/rights")
}

fn legal_b() -> LegalMetadata {
    LegalMetadata::new()
        .with_copyright_holder("Holder B")
        .with_usage_terms("Terms B")
        .with_creator("Creator B")
        .with_credit_line("Credit B")
        .with_copyright_owner("Owner B")
        .with_ai_constraints("No AI B")
        .with_contact_email("b@example.com")
        .with_web_statement_of_rights("https://b.example.com/rights")
}

// ── Workstream D: Preservation and idempotence ──────────────────────────────

#[test]
fn idempotent_png_replace_stego_owned() {
    let base = make_test_image_png(64, 64);
    let legal = legal_a();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal.clone())
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let out2 = process_image_bytes(&out1, ProtectionLevel::Standard, &ctx).unwrap();

    let r1 = verify_legal_notice(&out1, b"");
    let r2 = verify_legal_notice(&out2, b"");
    assert_eq!(r1.copyright_holder(), r2.copyright_holder());
    assert_eq!(r1.usage_terms(), r2.usage_terms());
    assert_eq!(r1.creator(), r2.creator());
    assert_eq!(r1.credit_line(), r2.credit_line());
    assert_eq!(r1.dmi(), r2.dmi());
}

#[test]
fn idempotent_jpeg_replace_stego_owned() {
    let base = make_test_image_jpeg(64, 64);
    let legal = legal_a();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal.clone())
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let out2 = process_image_bytes(&out1, ProtectionLevel::Standard, &ctx).unwrap();

    let r1 = verify_legal_notice(&out1, b"");
    let r2 = verify_legal_notice(&out2, b"");
    assert_eq!(r1.copyright_holder(), r2.copyright_holder());
    assert_eq!(r1.usage_terms(), r2.usage_terms());
}

#[test]
fn idempotent_webp_replace_stego_owned() {
    let base = make_test_image_png(64, 64);
    let legal = legal_a();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal.clone())
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let out2 = process_image_bytes(&out1, ProtectionLevel::Standard, &ctx).unwrap();

    let r1 = verify_legal_notice(&out1, b"");
    let r2 = verify_legal_notice(&out2, b"");
    assert_eq!(r1.copyright_holder(), r2.copyright_holder());
    assert_eq!(r1.usage_terms(), r2.usage_terms());
}

#[test]
fn fail_on_conflict_returns_error() {
    let base = make_test_image_jpeg(64, 64);
    let legal = legal_a();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal.clone())
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let first = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();

    let new_legal = LegalMetadata::new().with_copyright_holder("Conflict");
    let ctx_conflict = ProtectionContext::new(0.5, 43)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(new_legal)
        .with_dmi(DmiValue::Allowed)
        .with_metadata_update_policy(MetadataUpdatePolicy::FailOnConflict);

    let result = process_image_bytes(&first, ProtectionLevel::Standard, &ctx_conflict);
    assert!(result.is_err());

    let first_notice = verify_legal_notice(&first, b"");
    assert_eq!(
        first_notice.copyright_holder(),
        Some("Holder A"),
        "first output should be unchanged after error"
    );
}

#[test]
fn preserve_existing_on_jpeg_byte_path() {
    let base = make_test_image_jpeg(64, 64);
    let legal_a = legal_a();
    let ctx_a = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal_a)
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::PreserveExisting);

    let first = process_image_bytes(&base, ProtectionLevel::Standard, &ctx_a).unwrap();
    let first_notice = verify_legal_notice(&first, b"");

    let legal_b = legal_b();
    let ctx_b = ProtectionContext::new(0.5, 99)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal_b)
        .with_dmi(DmiValue::Allowed)
        .with_metadata_update_policy(MetadataUpdatePolicy::PreserveExisting);

    let second = process_image_bytes(&first, ProtectionLevel::Standard, &ctx_b).unwrap();
    let second_notice = verify_legal_notice(&second, b"");

    assert_eq!(
        second_notice.copyright_holder(),
        first_notice.copyright_holder(),
        "PreserveExisting should retain original copyright on JPEG byte path"
    );
    assert_eq!(
        second_notice.usage_terms(),
        first_notice.usage_terms(),
        "PreserveExisting should retain original usage_terms on JPEG byte path"
    );
}

#[test]
fn replace_stego_owned_field_isolation() {
    let base = make_test_image_png(64, 64);
    let legal_a = LegalMetadata::new()
        .with_copyright_holder("Holder A")
        .with_usage_terms("Terms A")
        .with_credit_line("Credit A")
        .with_copyright_owner("Owner A")
        .with_ai_constraints("No AI A");
    let ctx_a = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_a)
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let first = process_image_bytes(&base, ProtectionLevel::Standard, &ctx_a).unwrap();

    let legal_b = LegalMetadata::new()
        .with_copyright_holder("Holder B")
        .with_usage_terms("Terms B")
        .with_credit_line("Credit B")
        .with_copyright_owner("Owner B")
        .with_ai_constraints("No AI B");
    let ctx_b = ProtectionContext::new(0.5, 99)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_b)
        .with_dmi(DmiValue::Allowed)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let second = process_image_bytes(&first, ProtectionLevel::Standard, &ctx_b).unwrap();
    let notice = verify_legal_notice(&second, b"");

    assert_eq!(notice.copyright_holder(), Some("Holder B"));
    assert_eq!(notice.usage_terms(), Some("Terms B"));
    assert_eq!(notice.credit_line(), Some("Credit B"));
    assert_eq!(notice.copyright_owner(), Some("Owner B"));
    assert_eq!(notice.ai_constraints(), Some("No AI B"));
    assert_eq!(notice.dmi(), Some(DmiValue::Allowed));
}

#[test]
fn updated_assets_decodable() {
    for fmt in [
        ImageOutputFormat::Png,
        ImageOutputFormat::Jpeg,
        ImageOutputFormat::WebP,
    ] {
        let base = make_test_image_png(64, 64);
        let legal = legal_a();
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(fmt)
            .with_legal_metadata(legal)
            .with_dmi(DmiValue::ProhibitedAiMlTraining);

        let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
        let out2 = process_image_bytes(&out1, ProtectionLevel::Standard, &ctx).unwrap();
        let out3 = process_image_bytes(&out2, ProtectionLevel::Standard, &ctx).unwrap();

        assert!(
            image::load_from_memory(&out1).is_ok(),
            "{:?}: out1 should decode",
            fmt
        );
        assert!(
            image::load_from_memory(&out2).is_ok(),
            "{:?}: out2 should decode",
            fmt
        );
        assert!(
            image::load_from_memory(&out3).is_ok(),
            "{:?}: out3 should decode",
            fmt
        );
    }
}

#[test]
fn three_rounds_no_unbounded_growth() {
    for fmt in [ImageOutputFormat::Png, ImageOutputFormat::WebP] {
        let base = make_test_image_png(64, 64);
        let legal = legal_a();
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(fmt)
            .with_legal_metadata(legal)
            .with_dmi(DmiValue::ProhibitedAiMlTraining);

        let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
        let out2 = process_image_bytes(&out1, ProtectionLevel::Standard, &ctx).unwrap();
        let out3 = process_image_bytes(&out2, ProtectionLevel::Standard, &ctx).unwrap();

        let ratio_1_2 = out2.len() as f64 / out1.len() as f64;
        let ratio_2_3 = out3.len() as f64 / out2.len() as f64;
        assert!(
            ratio_1_2 < 1.15,
            "{:?}: out2/out1 size ratio too large: {:.3} ({} / {})",
            fmt,
            ratio_1_2,
            out2.len(),
            out1.len()
        );
        assert!(
            ratio_2_3 < 1.15,
            "{:?}: out3/out2 size ratio too large: {:.3} ({} / {})",
            fmt,
            ratio_2_3,
            out3.len(),
            out2.len()
        );
    }
}

#[test]
fn normalized_fields_unchanged_across_rounds() {
    let base = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("Stable Holder")
        .with_usage_terms("Stable Terms")
        .with_creator("Stable Creator")
        .with_credit_line("Stable Credit")
        .with_copyright_owner("Stable Owner")
        .with_ai_constraints("Stable Constraints")
        .with_contact_email("stable@example.com")
        .with_licensor_name("Stable Licensor")
        .with_licensor_email("lic@stable.com")
        .with_licensor_url("https://stable.com/license")
        .with_web_statement_of_rights("https://stable.com/rights")
        .with_creation_date("2025-01-01")
        .with_metadata_date("2025-06-01")
        .with_notice_applied_at("2025-06-15T12:00:00Z");
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let out2 = process_image_bytes(&out1, ProtectionLevel::Standard, &ctx).unwrap();

    let r1 = verify_legal_notice(&out1, b"");
    let r2 = verify_legal_notice(&out2, b"");

    assert_eq!(r1.copyright_holder(), r2.copyright_holder());
    assert_eq!(r1.usage_terms(), r2.usage_terms());
    assert_eq!(r1.creator(), r2.creator());
    assert_eq!(r1.credit_line(), r2.credit_line());
    assert_eq!(r1.copyright_owner(), r2.copyright_owner());
    assert_eq!(r1.ai_constraints(), r2.ai_constraints());
    assert_eq!(r1.contact(), r2.contact());
    assert_eq!(r1.licensor_name(), r2.licensor_name());
    assert_eq!(r1.licensor_email(), r2.licensor_email());
    assert_eq!(r1.licensor_url(), r2.licensor_url());
    assert_eq!(r1.web_statement_of_rights(), r2.web_statement_of_rights());
    assert_eq!(r1.dmi(), r2.dmi());
}

#[test]
fn with_warnings_matches_without_for_idempotence() {
    let base = make_test_image_png(64, 64);
    let legal = legal_a();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let (out1_w, _) =
        process_image_bytes_with_warnings(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let (out2_w, _) =
        process_image_bytes_with_warnings(&out1_w, ProtectionLevel::Standard, &ctx).unwrap();

    let r1 = verify_legal_notice(&out1_w, b"");
    let r2 = verify_legal_notice(&out2_w, b"");

    assert_eq!(r1.copyright_holder(), r2.copyright_holder());
    assert_eq!(r1.usage_terms(), r2.usage_terms());
    assert_eq!(r1.dmi(), r2.dmi());
}

#[test]
fn no_legal_metadata_idempotent() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let out1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let out2 = process_image_bytes(&out1, ProtectionLevel::Standard, &ctx).unwrap();

    let r1 = verify_legal_notice(&out1, b"");
    let r2 = verify_legal_notice(&out2, b"");

    assert_eq!(r1.copyright_holder(), None);
    assert_eq!(r2.copyright_holder(), None);
    assert_eq!(r1.dmi(), r2.dmi());
}

#[test]
fn jpeg_fail_on_conflict_before_mutation() {
    let base = make_test_image_jpeg(64, 64);
    let legal_a = legal_a();
    let ctx_a = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal_a)
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::ReplaceStegoOwned);

    let first = process_image_bytes(&base, ProtectionLevel::Standard, &ctx_a).unwrap();

    let legal_b = legal_b();
    let ctx_b = ProtectionContext::new(0.5, 99)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal_b)
        .with_dmi(DmiValue::Allowed)
        .with_metadata_update_policy(MetadataUpdatePolicy::FailOnConflict);

    let result = process_image_bytes(&first, ProtectionLevel::Standard, &ctx_b);
    assert!(
        result.is_err(),
        "FailOnConflict should error on JPEG re-process"
    );

    let first_notice = verify_legal_notice(&first, b"");
    assert_eq!(first_notice.copyright_holder(), Some("Holder A"));
    assert_eq!(first_notice.dmi(), Some(DmiValue::ProhibitedAiMlTraining));
}

#[test]
fn jpeg_preserve_existing_retains_conflicts() {
    let base = make_test_image_jpeg(64, 64);
    let legal_a = legal_a();
    let ctx_a = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal_a)
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_metadata_update_policy(MetadataUpdatePolicy::PreserveExisting);

    let first = process_image_bytes(&base, ProtectionLevel::Standard, &ctx_a).unwrap();
    let first_notice = verify_legal_notice(&first, b"");

    let legal_b = legal_b();
    let ctx_b = ProtectionContext::new(0.5, 99)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal_b)
        .with_dmi(DmiValue::Allowed)
        .with_metadata_update_policy(MetadataUpdatePolicy::PreserveExisting);

    let second = process_image_bytes(&first, ProtectionLevel::Standard, &ctx_b).unwrap();
    let second_notice = verify_legal_notice(&second, b"");

    assert_eq!(
        second_notice.copyright_holder(),
        first_notice.copyright_holder(),
        "PreserveExisting should retain original copyright on JPEG"
    );
}
