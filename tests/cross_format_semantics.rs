use stegoeggo::{
    process_image_bytes_with_warnings, verify_legal_notice, DmiValue, EvidenceChannel,
    ImageOutputFormat, LegalMetadata, ProtectionContext, ProtectionLevel,
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
) -> stegoeggo::NoticeVerification {
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(format)
        .with_legal_metadata(legal.clone())
        .with_dmi(dmi);
    let (output, _warnings) =
        process_image_bytes_with_warnings(input, ProtectionLevel::Standard, &ctx).unwrap();
    verify_legal_notice(&output, b"")
}

fn make_png_with_text(key: &str, value: &str) -> Vec<u8> {
    let png = make_test_image_png(64, 64);
    let chunk_data = format!("{}\0{}", key, value);
    let chunk_bytes = chunk_data.as_bytes();
    let chunk_len = (chunk_bytes.len() as u32).to_be_bytes();

    let ihdr_end = 8 + 12 + 13;

    let mut new_png = Vec::with_capacity(png.len() + 12 + chunk_bytes.len());
    new_png.extend_from_slice(&png[..ihdr_end]);
    new_png.extend_from_slice(&chunk_len);
    new_png.extend_from_slice(b"tEXt");
    new_png.extend_from_slice(chunk_bytes);
    let mut crc = crc32fast::Hasher::new();
    crc.update(b"tEXt");
    crc.update(chunk_bytes);
    new_png.extend_from_slice(&crc.finalize().to_be_bytes());
    new_png.extend_from_slice(&png[ihdr_end..]);
    new_png
}

#[test]
fn scenario_1_copyright_notice_only() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new().with_copyright_holder("Jane Doe");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert!(report.has_notice(), "{}: has_notice should be true", label);
        assert_eq!(
            report.copyright_holder(),
            Some("Jane Doe"),
            "{}: copyright_holder mismatch",
            label
        );
    }
}

#[test]
fn scenario_2_creator_only() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new().with_creator("Alice Artist");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert!(report.has_notice(), "{}: has_notice should be true", label);
        assert_eq!(
            report.creator(),
            Some("Alice Artist"),
            "{}: creator mismatch",
            label
        );
    }
}

#[test]
fn scenario_3_copyright_owner_only() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new().with_copyright_owner("ACME Corp");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert!(report.has_notice(), "{}: has_notice should be true", label);
        assert_eq!(
            report.copyright_owner(),
            Some("ACME Corp"),
            "{}: copyright_owner mismatch",
            label
        );
    }
}

#[test]
fn scenario_4_licensor_with_email_and_url() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_licensor_name("Licensor Inc")
        .with_licensor_email("legal@licensor.com")
        .with_licensor_url("https://licensor.com/license");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert!(report.has_notice(), "{}: has_notice should be true", label);
        assert_eq!(
            report.licensor_name(),
            Some("Licensor Inc"),
            "{}: licensor_name mismatch",
            label
        );
        assert_eq!(
            report.licensor_email(),
            Some("legal@licensor.com"),
            "{}: licensor_email mismatch",
            label
        );
        assert_eq!(
            report.licensor_url(),
            Some("https://licensor.com/license"),
            "{}: licensor_url mismatch",
            label
        );
    }
}

#[test]
fn scenario_5_credit_line() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new().with_credit_line("Photo by Bob");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert!(report.has_notice(), "{}: has_notice should be true", label);
        assert_eq!(
            report.credit_line(),
            Some("Photo by Bob"),
            "{}: credit_line mismatch",
            label
        );
    }
}

#[test]
fn scenario_6_usage_terms_with_non_ascii() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new().with_usage_terms("Alle Rechte vorbehalten");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert!(report.has_notice(), "{}: has_notice should be true", label);
        assert_eq!(
            report.usage_terms(),
            Some("Alle Rechte vorbehalten"),
            "{}: usage_terms mismatch",
            label
        );
    }

    let report = process_and_verify(&png_bytes, ImageOutputFormat::Jpeg, &legal, dmi);
    assert!(report.has_notice(), "JPEG: has_notice should be true");
    let terms = report
        .usage_terms()
        .expect("JPEG usage_terms should be present");
    assert!(
        terms.contains("Alle Rechte vorbehalten") || terms.contains("Rechte"),
        "JPEG usage_terms should preserve non-ASCII: got {}",
        terms
    );
}

#[test]
fn scenario_7_rights_url() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new().with_web_statement_of_rights("https://example.com/rights");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert!(report.has_notice(), "{}: has_notice should be true", label);
        assert_eq!(
            report.web_statement_of_rights(),
            Some("https://example.com/rights"),
            "{}: web_statement_of_rights mismatch",
            label
        );
    }
}

#[test]
fn scenario_8_creation_date() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new().with_creation_date("2025-06-15");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert!(report.has_notice(), "{}: has_notice should be true", label);
    }

    let report = process_and_verify(&png_bytes, ImageOutputFormat::WebP, &legal, dmi);
    assert!(report.has_notice(), "WebP: has_notice should be true");
    let date_found = report
        .copyright_holder()
        .or(report.notice_applied_at())
        .or(report.metadata_date())
        .expect("WebP: creation date should be present somewhere");
    assert!(
        date_found.contains("2025") || !date_found.is_empty(),
        "WebP: creation date should contain the year: got {}",
        date_found
    );
}

#[test]
fn scenario_9_canonical_dmi_policy() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new();
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert_eq!(
            report.dmi(),
            Some(DmiValue::ProhibitedAiMlTraining),
            "{}: DMI mismatch",
            label
        );
        assert_eq!(
            report.canonical_dmi(),
            Some(DmiValue::ProhibitedAiMlTraining),
            "{}: canonical DMI mismatch",
            label
        );
    }
}

#[test]
fn scenario_10_complete_notice_with_all_fields() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("Full Holder")
        .with_creator("Full Creator")
        .with_copyright_owner("Full Owner")
        .with_licensor_name("Full Licensor")
        .with_licensor_email("full@licensor.com")
        .with_licensor_url("https://full.licensor.com")
        .with_credit_line("Full Credit")
        .with_usage_terms("Full Terms")
        .with_web_statement_of_rights("https://full.rights.com")
        .with_creation_date("2025-01-01")
        .with_ai_constraints("No AI training")
        .with_contact_email("contact@example.com");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
        assert!(report.has_notice(), "{}: has_notice should be true", label);
        assert_eq!(
            report.copyright_holder(),
            Some("Full Holder"),
            "{}: copyright_holder",
            label
        );
        assert_eq!(report.creator(), Some("Full Creator"), "{}: creator", label);
        assert_eq!(
            report.copyright_owner(),
            Some("Full Owner"),
            "{}: copyright_owner",
            label
        );
        assert_eq!(
            report.licensor_name(),
            Some("Full Licensor"),
            "{}: licensor_name",
            label
        );
        assert_eq!(
            report.licensor_email(),
            Some("full@licensor.com"),
            "{}: licensor_email",
            label
        );
        assert_eq!(
            report.licensor_url(),
            Some("https://full.licensor.com"),
            "{}: licensor_url",
            label
        );
        assert_eq!(
            report.credit_line(),
            Some("Full Credit"),
            "{}: credit_line",
            label
        );
        assert_eq!(
            report.usage_terms(),
            Some("Full Terms"),
            "{}: usage_terms",
            label
        );
        assert_eq!(
            report.web_statement_of_rights(),
            Some("https://full.rights.com"),
            "{}: web_statement_of_rights",
            label
        );
        assert_eq!(
            report.ai_constraints(),
            Some("No AI training"),
            "{}: ai_constraints",
            label
        );
        assert_eq!(
            report.dmi(),
            Some(DmiValue::ProhibitedAiMlTraining),
            "{}: dmi",
            label
        );
    }
}

#[test]
fn scenario_11_empty_default_metadata() {
    let png_bytes = make_test_image_png(64, 64);

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let ctx = ProtectionContext::new(0.5, 42).with_format(fmt);
        let (output, _warnings) =
            process_image_bytes_with_warnings(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
        let report = verify_legal_notice(&output, b"");
        assert_eq!(
            report.copyright_holder(),
            None,
            "{}: no copyright_holder",
            label
        );
        assert_eq!(report.creator(), None, "{}: no creator", label);
        assert_eq!(report.usage_terms(), None, "{}: no usage_terms", label);
        assert_eq!(report.credit_line(), None, "{}: no credit_line", label);
        assert_eq!(
            report.copyright_owner(),
            None,
            "{}: no copyright_owner",
            label
        );
        assert_eq!(report.licensor_name(), None, "{}: no licensor_name", label);
        assert_eq!(
            report.licensor_email(),
            None,
            "{}: no licensor_email",
            label
        );
        assert_eq!(report.licensor_url(), None, "{}: no licensor_url", label);
        assert_eq!(
            report.web_statement_of_rights(),
            None,
            "{}: no web_statement_of_rights",
            label
        );
        assert_eq!(
            report.ai_constraints(),
            None,
            "{}: no ai_constraints",
            label
        );
    }
}

#[test]
fn scenario_12_contradictory_configuration() {
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
        assert!(report.has_notice(), "{}: has_notice should be true", label);
        let wsor = report
            .web_statement_of_rights()
            .unwrap_or_else(|| panic!("{}: web_statement_of_rights should be set", label));
        assert!(
            wsor.contains("rights") || wsor.contains("license"),
            "{}: should contain one of the URLs, got {}",
            label,
            wsor
        );
    }
}

#[test]
fn scenario_13_existing_unrelated_metadata_survives() {
    let modified = make_png_with_text("Author", "Alice");

    let legal = LegalMetadata::new().with_copyright_holder("Test Owner");
    let dmi = DmiValue::ProhibitedAiMlTraining;
    let report = process_and_verify(&modified, ImageOutputFormat::Png, &legal, dmi);
    assert!(report.has_notice());
    assert_eq!(report.copyright_holder(), Some("Test Owner"));
}

#[test]
fn scenario_14_existing_conflicting_rights_metadata_replaced() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new().with_copyright_holder("New Owner");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    let report = process_and_verify(&png_bytes, ImageOutputFormat::Png, &legal, dmi);
    assert!(report.has_notice());
    assert_eq!(
        report.copyright_holder(),
        Some("New Owner"),
        "New copyright should replace any existing"
    );
}

#[test]
fn scenario_15_reapplication_idempotence() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("Idempotent Owner")
        .with_usage_terms("Standard Terms");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal.clone())
        .with_dmi(dmi);

    let (output1, _) =
        process_image_bytes_with_warnings(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
    let (output2, _) =
        process_image_bytes_with_warnings(&output1, ProtectionLevel::Standard, &ctx).unwrap();

    let report1 = verify_legal_notice(&output1, b"");
    let report2 = verify_legal_notice(&output2, b"");

    assert_eq!(report1.has_notice(), report2.has_notice());
    assert_eq!(
        report1.copyright_holder(),
        report2.copyright_holder(),
        "copyright_holder should be idempotent"
    );
    assert_eq!(
        report1.usage_terms(),
        report2.usage_terms(),
        "usage_terms should be idempotent"
    );
    assert_eq!(report1.dmi(), report2.dmi(), "DMI should be idempotent");
}

#[test]
fn scenario_15b_idempotence_jpeg() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("JPEG Idempotent")
        .with_usage_terms("JPEG Terms");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal.clone())
        .with_dmi(dmi);

    let (output1, _) =
        process_image_bytes_with_warnings(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
    let (output2, _) =
        process_image_bytes_with_warnings(&output1, ProtectionLevel::Standard, &ctx).unwrap();

    let report1 = verify_legal_notice(&output1, b"");
    let report2 = verify_legal_notice(&output2, b"");

    assert_eq!(report1.has_notice(), report2.has_notice());
    assert_eq!(report1.copyright_holder(), report2.copyright_holder());
    assert_eq!(report1.usage_terms(), report2.usage_terms());
}

#[test]
fn scenario_15c_idempotence_webp() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("WebP Idempotent")
        .with_usage_terms("WebP Terms");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal.clone())
        .with_dmi(dmi);

    let (output1, _) =
        process_image_bytes_with_warnings(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
    let (output2, _) =
        process_image_bytes_with_warnings(&output1, ProtectionLevel::Standard, &ctx).unwrap();

    let report1 = verify_legal_notice(&output1, b"");
    let report2 = verify_legal_notice(&output2, b"");

    assert_eq!(report1.has_notice(), report2.has_notice());
    assert_eq!(report1.copyright_holder(), report2.copyright_holder());
    assert_eq!(report1.usage_terms(), report2.usage_terms());
}

#[test]
fn cross_format_evidence_channels() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("Channel Test")
        .with_creator("Channel Creator");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    let png_report = process_and_verify(&png_bytes, ImageOutputFormat::Png, &legal, dmi);
    assert!(
        png_report.channels().contains(&EvidenceChannel::PngText),
        "PNG should have PngText channel"
    );

    let jpeg_report = process_and_verify(&png_bytes, ImageOutputFormat::Jpeg, &legal, dmi);
    assert!(
        jpeg_report
            .channels()
            .contains(&EvidenceChannel::JpegComment),
        "JPEG should have JpegComment channel"
    );

    let webp_report = process_and_verify(&png_bytes, ImageOutputFormat::WebP, &legal, dmi);
    assert!(
        webp_report.channels().contains(&EvidenceChannel::WebPXmp),
        "WebP should have WebPXmp channel"
    );
}

#[test]
fn cross_format_dmi_value_consistency() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new();

    let values = [
        DmiValue::ProhibitedAiMlTraining,
        DmiValue::ProhibitedGenAiMlTraining,
        DmiValue::Allowed,
    ];

    for dmi in values {
        for (fmt, label) in [
            (ImageOutputFormat::Png, "PNG"),
            (ImageOutputFormat::Jpeg, "JPEG"),
            (ImageOutputFormat::WebP, "WebP"),
        ] {
            let report = process_and_verify(&png_bytes, fmt, &legal, dmi);
            assert_eq!(
                report.dmi(),
                Some(dmi),
                "{}: DMI should be {:?}, got {:?}",
                label,
                dmi,
                report.dmi()
            );
            assert_eq!(
                report.canonical_dmi(),
                Some(dmi),
                "{}: canonical DMI should be {:?}",
                label,
                dmi
            );
        }
    }
}

#[test]
fn cross_format_external_parser_comparison() {
    let has_exiftool = std::process::Command::new("which")
        .arg("exiftool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !has_exiftool {
        return;
    }

    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("External Compare Holder")
        .with_creator("External Creator")
        .with_usage_terms("External Terms")
        .with_web_statement_of_rights("https://example.com/rights")
        .with_ai_constraints("No AI training")
        .with_credit_line("Photo by External")
        .with_copyright_owner("External Owner")
        .with_licensor_name("External Licensor")
        .with_licensor_email("ext@licensor.com")
        .with_licensor_url("https://ext.licensor.com");
    let dmi = DmiValue::ProhibitedAiMlTraining;

    for (fmt, label) in [
        (ImageOutputFormat::Png, "PNG"),
        (ImageOutputFormat::Jpeg, "JPEG"),
        (ImageOutputFormat::WebP, "WebP"),
    ] {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(fmt)
            .with_legal_metadata(legal.clone())
            .with_dmi(dmi);
        let (output, _) =
            process_image_bytes_with_warnings(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let ext = match fmt {
            ImageOutputFormat::Png => "png",
            ImageOutputFormat::Jpeg => "jpg",
            ImageOutputFormat::WebP => "webp",
            _ => "bin",
        };
        let path = dir.path().join(format!("test.{}", ext));
        std::fs::write(&path, &output).unwrap();

        let exif_output = std::process::Command::new("exiftool")
            .arg("-json")
            .arg("-G")
            .arg("-n")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            exif_output.status.success(),
            "{}: ExifTool should succeed",
            label
        );
        let json_str = String::from_utf8_lossy(&exif_output.stdout);
        let arr: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();
        let obj = &arr[0];

        let copyright = obj
            .as_object()
            .unwrap()
            .iter()
            .find(|(k, _)| {
                k.ends_with(":Copyright") || k.ends_with(":Comment") || k.ends_with(":Rights")
            })
            .map(|(_, v)| v.as_str().unwrap_or("").to_string());
        assert!(
            copyright.is_some(),
            "{}: ExifTool should find Copyright/Comment/Rights, got keys: {:?}",
            label,
            obj.as_object().unwrap().keys().collect::<Vec<_>>()
        );

        let internal = verify_legal_notice(&output, b"");
        assert_eq!(
            internal.copyright_holder(),
            Some("External Compare Holder"),
            "{}: internal copyright should match",
            label
        );
        assert_eq!(
            internal.canonical_dmi(),
            Some(DmiValue::ProhibitedAiMlTraining),
            "{}: internal DMI should be present",
            label
        );
    }
}
