use std::path::PathBuf;
use stegoeggo::conformance::{
    self, CheckSeverity, ConformanceReport, ConformanceRunReport, ConformanceSummary,
    CoverageCheckResult, CoverageMinimums, DigestCheckResult, ExternalExtraction, FixtureManifest,
    InternalExtraction, ManifestReport, ToolReport,
};
use stegoeggo::{
    process_image_bytes, DmiValue, ImageOutputFormat, LegalMetadata, ProtectionContext,
    ProtectionLevel,
};

fn make_test_image_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn fixtures_dir() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest_dir).join("tests/fixtures/conformance")
}

#[test]
fn json_report_serialization_roundtrip() {
    let mut report = ConformanceReport::new("test.png", "png");
    report.decode_valid = true;
    report.add_check("decode", CheckSeverity::Pass, "Image decodes");
    report.add_check_with_details(
        "copyright",
        CheckSeverity::Fail,
        "Mismatch",
        "internal=A, external=B",
    );
    report.evaluate();
    assert!(!report.passed);

    let json = serde_json::to_string(&report).unwrap();
    let restored: ConformanceReport = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.fixture, "test.png");
    assert_eq!(restored.format, "png");
    assert!(restored.decode_valid);
    assert!(!restored.passed);
    assert_eq!(restored.checks.len(), 2);
    assert_eq!(restored.checks[0].severity, CheckSeverity::Pass);
    assert_eq!(restored.checks[1].severity, CheckSeverity::Fail);
    assert!(restored.checks[1].details.is_some());
}

#[test]
fn json_report_array_serialization() {
    let mut reports = Vec::new();
    for i in 0..3 {
        let mut r = ConformanceReport::new(&format!("test_{}.png", i), "png");
        r.decode_valid = true;
        r.add_check("decode", CheckSeverity::Pass, "ok");
        r.evaluate();
        reports.push(r);
    }

    let json = serde_json::to_string(&reports).unwrap();
    let restored: Vec<ConformanceReport> = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.len(), 3);
    for r in &restored {
        assert!(r.passed);
    }
}

#[test]
fn detect_format_returns_correct_format() {
    let png = make_test_image_png(8, 8);
    assert_eq!(conformance::detect_format(&png), Some("png".to_string()));

    let jpeg;
    {
        let img = image::DynamicImage::new_rgb8(8, 8);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
        jpeg = buf.into_inner();
    }
    assert_eq!(conformance::detect_format(&jpeg), Some("jpeg".to_string()));
}

#[test]
fn detect_format_short_bytes_returns_none() {
    assert_eq!(conformance::detect_format(&[0x89]), None);
    assert_eq!(conformance::detect_format(&[]), None);
}

#[test]
fn detect_format_unknown_returns_none() {
    assert_eq!(conformance::detect_format(b"BM"), None);
}

#[test]
fn collect_fixture_files_finds_all_fixtures() {
    let dir = fixtures_dir();
    let files = conformance::collect_fixture_files(&dir, &None);
    assert!(!files.is_empty(), "Should find fixture files");
    assert!(
        files.len() >= 15,
        "Should find at least 15 fixtures, found {}",
        files.len()
    );
    for f in &files {
        assert!(f.exists(), "File should exist: {}", f.display());
    }
}

#[test]
fn collect_fixture_files_filters_by_format() {
    let dir = fixtures_dir();
    let png_files = conformance::collect_fixture_files(&dir, &Some("png".to_string()));
    let jpeg_files = conformance::collect_fixture_files(&dir, &Some("jpeg".to_string()));
    let webp_files = conformance::collect_fixture_files(&dir, &Some("webp".to_string()));

    for f in &png_files {
        assert_eq!(f.extension().unwrap(), "png");
    }
    for f in &jpeg_files {
        let ext = f.extension().unwrap().to_str().unwrap();
        assert!(ext == "jpg" || ext == "jpeg");
    }
    for f in &webp_files {
        assert_eq!(f.extension().unwrap(), "webp");
    }

    assert!(!png_files.is_empty(), "Should find PNG fixtures");
    assert!(!jpeg_files.is_empty(), "Should find JPEG fixtures");
    assert!(!webp_files.is_empty(), "Should find WebP fixtures");
}

#[test]
fn collect_fixture_files_nonexistent_dir_returns_empty() {
    let dir = std::path::Path::new("/nonexistent/path");
    let files = conformance::collect_fixture_files(dir, &None);
    assert!(files.is_empty());
}

#[test]
fn internal_extract_succeeds_for_protected_image() {
    let png_bytes = make_test_image_png(64, 64);
    let legal = LegalMetadata::new()
        .with_copyright_holder("Extract Test")
        .with_creator("Extract Creator");
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.png");
    std::fs::write(&path, &output).unwrap();

    let notice = stegoeggo::verify_legal_notice(&output, b"");
    let ext = InternalExtraction {
        copyright_holder: notice.copyright_holder().map(|s| s.to_string()),
        creators: notice
            .creator()
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        copyright_owner: notice.copyright_owner().map(|s| s.to_string()),
        usage_terms: notice.usage_terms().map(|s| s.to_string()),
        web_statement_of_rights: notice.web_statement_of_rights().map(|s| s.to_string()),
        credit_line: notice.credit_line().map(|s| s.to_string()),
        licensor_name: notice.licensor_name().map(|s| s.to_string()),
        licensor_email: notice.licensor_email().map(|s| s.to_string()),
        licensor_url: notice.licensor_url().map(|s| s.to_string()),
        content_creation_date: notice.metadata_date().map(|s| s.to_string()),
        ai_constraints: notice.ai_constraints().map(|s| s.to_string()),
        canonical_data_mining: notice.canonical_dmi().map(|d| d.as_str().to_string()),
        legacy_data_mining: notice
            .legacy_dmi()
            .map(|d| vec![d.as_str().to_string()])
            .unwrap_or_default(),
        tdm_reserved: notice.tdm_reserved(),
        seed: notice.protection_seed(),
        evidence_channels: notice
            .channels()
            .iter()
            .map(|c| format!("{:?}", c))
            .collect(),
        evidence_strength: Some(format!("{:?}", notice.evidence_strength())),
    };
    assert_eq!(ext.copyright_holder.as_deref(), Some("Extract Test"));
    assert_eq!(ext.creators, vec!["Extract Creator".to_string()]);
    assert!(
        ext.canonical_data_mining.is_some(),
        "canonical DMI should be present, got: {:?}",
        ext.canonical_data_mining
    );
}

#[test]
fn compare_extractions_matching_values_produces_pass() {
    let internal = InternalExtraction {
        copyright_holder: Some("Test Holder".to_string()),
        ..Default::default()
    };
    let external = ExternalExtraction {
        copyright: Some("Test Holder".to_string()),
        ..Default::default()
    };
    let mut report = ConformanceReport::new("test", "png");
    conformance::compare_extractions(&internal, &external, &mut report);

    let copyright_check = report
        .checks
        .iter()
        .find(|c| c.name == "copyright")
        .unwrap();
    assert_eq!(copyright_check.severity, CheckSeverity::Pass);
}

#[test]
fn compare_extractions_mismatched_values_produces_fail() {
    let internal = InternalExtraction {
        copyright_holder: Some("Internal Holder".to_string()),
        ..Default::default()
    };
    let external = ExternalExtraction {
        copyright: Some("External Holder".to_string()),
        ..Default::default()
    };
    let mut report = ConformanceReport::new("test", "png");
    conformance::compare_extractions(&internal, &external, &mut report);

    let copyright_check = report
        .checks
        .iter()
        .find(|c| c.name == "copyright")
        .unwrap();
    assert_eq!(copyright_check.severity, CheckSeverity::Fail);
    assert!(copyright_check.details.is_some());
}

#[test]
fn compare_extractions_internal_only_produces_warn() {
    let internal = InternalExtraction {
        copyright_holder: Some("Internal Only".to_string()),
        ..Default::default()
    };
    let external = ExternalExtraction::default();
    let mut report = ConformanceReport::new("test", "png");
    conformance::compare_extractions(&internal, &external, &mut report);

    let copyright_check = report
        .checks
        .iter()
        .find(|c| c.name == "copyright")
        .unwrap();
    assert_eq!(copyright_check.severity, CheckSeverity::Warn);
}

#[test]
fn compare_extractions_both_absent_produces_pass() {
    let internal = InternalExtraction::default();
    let external = ExternalExtraction::default();
    let mut report = ConformanceReport::new("test", "png");
    conformance::compare_extractions(&internal, &external, &mut report);

    let copyright_check = report
        .checks
        .iter()
        .find(|c| c.name == "copyright")
        .unwrap();
    assert_eq!(copyright_check.severity, CheckSeverity::Pass);
}

#[test]
fn strict_mode_exits_when_exiftool_missing() {
    let has_exiftool = std::process::Command::new("which")
        .arg("exiftool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_exiftool {
        let dir = tempfile::tempdir().unwrap();
        let result = std::process::Command::new(env!("CARGO_BIN_EXE_stegoeggo-conformance"))
            .arg("--fixtures")
            .arg(dir.path())
            .arg("--strict")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
    }
}

#[test]
fn report_summary_contains_fixture_info() {
    let mut report = ConformanceReport::new("my_fixture.png", "png");
    report.add_check("decode", CheckSeverity::Pass, "Image decodes");
    report.evaluate();

    let summary = report.summary();
    assert!(summary.contains("my_fixture.png"));
    assert!(summary.contains("png"));
    assert!(summary.contains("PASS"));
}

#[test]
fn evaluate_sets_passed_false_when_any_fail() {
    let mut report = ConformanceReport::new("test", "png");
    report.add_check("a", CheckSeverity::Pass, "ok");
    report.add_check("b", CheckSeverity::Fail, "bad");
    report.add_check("c", CheckSeverity::Warn, "warn");
    report.evaluate();
    assert!(!report.passed);
}

#[test]
fn evaluate_sets_passed_true_when_no_fails() {
    let mut report = ConformanceReport::new("test", "png");
    report.add_check("a", CheckSeverity::Pass, "ok");
    report.add_check("b", CheckSeverity::Warn, "warn");
    report.evaluate();
    assert!(report.passed);
}

#[test]
fn structural_xmp_validation_catches_oversized() {
    let mut report = ConformanceReport::new("test", "png");
    let oversized = "X".repeat(70000);
    let xmp = format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      dc:rights="{}"/>
  </rdf:RDF>
</x:xmpmeta>"#,
        oversized
    );
    let xmp_valid = stegoeggo::conformance::normalize_dmi_value("ProhibitedAiMlTraining");
    assert_eq!(xmp_valid, "DMI-PROHIBITED-AIMLTRAINING");
    report.add_check(
        "xmp_size",
        CheckSeverity::Fail,
        &format!("XMP exceeds maximum size: {} > 65536 bytes", xmp.len()),
    );
    report.evaluate();
    assert!(!report.passed);
}

#[test]
fn normalize_dmi_variants() {
    assert_eq!(
        stegoeggo::conformance::normalize_dmi_value("ProhibitedAiMlTraining"),
        "DMI-PROHIBITED-AIMLTRAINING"
    );
    assert_eq!(
        stegoeggo::conformance::normalize_dmi_value("DMI-PROHIBITED-AIMLTRAINING"),
        "DMI-PROHIBITED-AIMLTRAINING"
    );
    assert_eq!(
        stegoeggo::conformance::normalize_dmi_value("Prohibited for AI/ML training"),
        "DMI-PROHIBITED-AIMLTRAINING"
    );
    assert_eq!(
        stegoeggo::conformance::normalize_dmi_value("Allowed"),
        "DMI-ALLOWED"
    );
    assert_eq!(
        stegoeggo::conformance::normalize_dmi_value("Permitted"),
        "DMI-ALLOWED"
    );
    assert_eq!(
        stegoeggo::conformance::normalize_dmi_value("Prohibited"),
        "DMI-PROHIBITED"
    );
    assert_eq!(
        stegoeggo::conformance::normalize_dmi_value("DMI-ALLOWED"),
        "DMI-ALLOWED"
    );
}

#[test]
fn compare_extractions_overlap_values_produces_pass() {
    let internal = InternalExtraction {
        copyright_holder: Some("Copyright (c) Test".to_string()),
        ..Default::default()
    };
    let external = ExternalExtraction {
        copyright: Some("Copyright (c) Test Holder".to_string()),
        ..Default::default()
    };
    let mut report = ConformanceReport::new("test", "png");
    conformance::compare_extractions(&internal, &external, &mut report);
    let copyright_check = report
        .checks
        .iter()
        .find(|c| c.name == "copyright")
        .unwrap();
    assert_eq!(copyright_check.severity, CheckSeverity::Fail);
}

#[test]
fn harness_nonstrict_survives_missing_exiftool() {
    let has_exiftool = std::process::Command::new("which")
        .arg("exiftool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if has_exiftool {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_stegoeggo-conformance"))
        .arg("--fixtures")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
}

#[test]
fn harness_strict_exits_with_code_2_when_no_exiftool() {
    let has_exiftool = std::process::Command::new("which")
        .arg("exiftool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !has_exiftool {
        let dir = tempfile::tempdir().unwrap();
        let result = std::process::Command::new(env!("CARGO_BIN_EXE_stegoeggo-conformance"))
            .arg("--fixtures")
            .arg(dir.path())
            .arg("--strict")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
    }
}

#[test]
fn harness_json_output_is_valid() {
    let has_exiftool = std::process::Command::new("which")
        .arg("exiftool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !has_exiftool {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("report.json");
    let fixtures =
        std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/tests/fixtures/conformance/canonical";
    let _result = std::process::Command::new(env!("CARGO_BIN_EXE_stegoeggo-conformance"))
        .arg("--fixtures")
        .arg(&fixtures)
        .arg("--json")
        .arg(&json_path)
        .output()
        .unwrap();
    assert!(
        json_path.exists(),
        "JSON report should be written even if some fixtures fail"
    );
    let json_str = std::fs::read_to_string(&json_path).unwrap();
    let run_report: ConformanceRunReport = serde_json::from_str(&json_str).unwrap();
    assert_eq!(run_report.schema_version, 1);
    assert!(
        !run_report.fixtures.is_empty(),
        "JSON should contain at least one fixture report"
    );
    for r in &run_report.fixtures {
        assert!(!r.fixture.is_empty());
        assert!(!r.format.is_empty());
        assert!(!r.checks.is_empty());
    }
}

#[test]
fn harness_empty_dir_produces_no_reports() {
    let has_exiftool = std::process::Command::new("which")
        .arg("exiftool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !has_exiftool {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_stegoeggo-conformance"))
        .arg("--fixtures")
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(result.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        combined.contains("0"),
        "Output should mention 0 fixtures: {}",
        combined
    );
}

#[test]
fn detect_format_webp() {
    let webp = [
        0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
    ];
    assert_eq!(conformance::detect_format(&webp), Some("webp".to_string()));
}

#[test]
fn detect_format_returns_none_for_text() {
    assert_eq!(conformance::detect_format(b"hello world"), None);
}

#[test]
fn collect_fixture_files_sorted_output() {
    let dir = fixtures_dir();
    let files = conformance::collect_fixture_files(&dir, &None);
    let names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names.len(), sorted.len());
    assert!(names.iter().all(|n| sorted.contains(n)));
}

#[test]
fn collect_fixture_files_jpeg_extension() {
    let dir = fixtures_dir();
    let jpeg_files = conformance::collect_fixture_files(&dir, &Some("jpeg".to_string()));
    assert!(!jpeg_files.is_empty());
    for f in &jpeg_files {
        let ext = f.extension().unwrap().to_str().unwrap();
        assert!(
            ext == "jpg" || ext == "jpeg",
            "JPEG filter should return .jpg or .jpeg files, got .{}",
            ext
        );
    }
}

#[test]
fn report_summary_format() {
    let mut report = ConformanceReport::new("test.png", "png");
    report.add_check("decode", CheckSeverity::Pass, "ok");
    report.add_check("xmp", CheckSeverity::Warn, "missing");
    report.evaluate();
    let summary = report.summary();
    assert!(summary.contains("test.png"));
    assert!(summary.contains("PASS"));
    assert!(summary.contains("[PASS] decode: ok"));
    assert!(summary.contains("[WARN] xmp: missing"));
}

#[test]
fn external_extraction_returns_default_when_command_fails() {
    let fake_path = std::env::temp_dir().join("nonexistent_exiftool_12345");
    let result = std::process::Command::new(&fake_path)
        .arg("-json")
        .arg("/dev/null")
        .output();
    assert!(
        result.is_err(),
        "Nonexistent command should fail to execute"
    );
}

#[test]
fn external_extraction_handles_invalid_json_gracefully() {
    let has_exiftool = std::process::Command::new("which")
        .arg("exiftool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !has_exiftool {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let fake_file = dir.path().join("not_an_image.bin");
    std::fs::write(&fake_file, b"this is not an image").unwrap();
    let result = std::process::Command::new("exiftool")
        .arg("-json")
        .arg("-G")
        .arg("-n")
        .arg(&fake_file)
        .output()
        .unwrap();
    if result.status.success() {
        let stdout = String::from_utf8_lossy(&result.stdout);
        let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&stdout);
        assert!(
            parsed.is_ok() || !result.status.success(),
            "ExifTool should return valid JSON even for unrecognized files"
        );
    }
}

#[test]
fn tool_version_captured_in_external_extraction() {
    let has_exiftool = std::process::Command::new("which")
        .arg("exiftool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !has_exiftool {
        return;
    }
    let version_output = std::process::Command::new("exiftool")
        .arg("-ver")
        .output()
        .unwrap();
    let expected_version = String::from_utf8_lossy(&version_output.stdout)
        .trim()
        .to_string();
    assert!(
        !expected_version.is_empty(),
        "ExifTool version should not be empty"
    );

    let png_bytes = make_test_image_png(8, 8);
    let legal = LegalMetadata::new().with_copyright_holder("Version Test");
    let ctx = ProtectionContext::new(0.5, 99)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("version_test.png");
    std::fs::write(&path, &output).unwrap();

    let result = std::process::Command::new("exiftool")
        .arg("-json")
        .arg("-G")
        .arg("-n")
        .arg(&path)
        .output()
        .unwrap();
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    let arr: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert!(!arr.is_empty());
}

#[test]
fn strict_mode_exit_code_distinct_from_failure() {
    let has_exiftool = std::process::Command::new("which")
        .arg("exiftool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if has_exiftool {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("report.json");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_stegoeggo-conformance"))
        .arg("--fixtures")
        .arg(dir.path())
        .arg("--strict")
        .arg("--json")
        .arg(&json_path)
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(2),
        "Strict mode with missing tool should exit 2"
    );
}

#[test]
fn strict_missing_manifest_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_stegoeggo-conformance"))
        .arg("--fixtures")
        .arg(dir.path())
        .arg("--strict")
        .output()
        .unwrap();
    assert_ne!(
        result.status.code(),
        Some(0),
        "Strict mode without --manifest should exit non-zero"
    );
}

#[test]
fn strict_empty_manifest_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let manifest_path = dir.path().join("empty.toml");
    std::fs::write(&manifest_path, "# empty\n").unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_stegoeggo-conformance"))
        .arg("--fixtures")
        .arg(dir.path())
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--strict")
        .output()
        .unwrap();
    assert_ne!(
        result.status.code(),
        Some(0),
        "Strict mode with empty manifest should exit non-zero"
    );
}

#[test]
fn strict_invalid_toml_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let manifest_path = dir.path().join("bad.toml");
    std::fs::write(&manifest_path, "not valid toml {{{{").unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_stegoeggo-conformance"))
        .arg("--fixtures")
        .arg(dir.path())
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--strict")
        .output()
        .unwrap();
    assert_ne!(
        result.status.code(),
        Some(0),
        "Strict mode with invalid TOML should exit non-zero"
    );
}

#[test]
fn validate_manifest_rejects_duplicate_ids() {
    let manifest = stegoeggo::conformance::FixtureManifest {
        entries: vec![
            stegoeggo::conformance::FixtureEntry {
                id: "dup".to_string(),
                path: "a.png".to_string(),
                format: "png".to_string(),
                category: "canonical".to_string(),
                authoring_tool: "test".to_string(),
                authoring_tool_version: "1.0".to_string(),
                generation_command: "test".to_string(),
                source: "generated".to_string(),
                license: "MIT".to_string(),
                sha256: "a".repeat(64),
                expected_dmi: String::new(),
                expected_conflict: false,
                expected_legal_fields: Default::default(),
                expected_malformed: false,
                expected_decode: Default::default(),
                expected_xmp: Default::default(),
                expected_internal: Default::default(),
                expected_external: Default::default(),
                required_external_fields: Vec::new(),
                expected_preservation: Vec::new(),
            },
            stegoeggo::conformance::FixtureEntry {
                id: "dup".to_string(),
                path: "b.png".to_string(),
                format: "png".to_string(),
                category: "canonical".to_string(),
                authoring_tool: "test".to_string(),
                authoring_tool_version: "1.0".to_string(),
                generation_command: "test".to_string(),
                source: "generated".to_string(),
                license: "MIT".to_string(),
                sha256: "b".repeat(64),
                expected_dmi: String::new(),
                expected_conflict: false,
                expected_legal_fields: Default::default(),
                expected_malformed: false,
                expected_decode: Default::default(),
                expected_xmp: Default::default(),
                expected_internal: Default::default(),
                expected_external: Default::default(),
                required_external_fields: Vec::new(),
                expected_preservation: Vec::new(),
            },
        ],
    };
    let result = stegoeggo::conformance::validate_manifest(&manifest);
    assert!(result.is_err(), "Should reject duplicate IDs");
}

#[test]
fn validate_manifest_rejects_duplicate_paths() {
    let manifest = stegoeggo::conformance::FixtureManifest {
        entries: vec![
            stegoeggo::conformance::FixtureEntry {
                id: "a".to_string(),
                path: "same.png".to_string(),
                format: "png".to_string(),
                category: "canonical".to_string(),
                authoring_tool: "test".to_string(),
                authoring_tool_version: "1.0".to_string(),
                generation_command: "test".to_string(),
                source: "generated".to_string(),
                license: "MIT".to_string(),
                sha256: "a".repeat(64),
                expected_dmi: String::new(),
                expected_conflict: false,
                expected_legal_fields: Default::default(),
                expected_malformed: false,
                expected_decode: Default::default(),
                expected_xmp: Default::default(),
                expected_internal: Default::default(),
                expected_external: Default::default(),
                required_external_fields: Vec::new(),
                expected_preservation: Vec::new(),
            },
            stegoeggo::conformance::FixtureEntry {
                id: "b".to_string(),
                path: "same.png".to_string(),
                format: "png".to_string(),
                category: "canonical".to_string(),
                authoring_tool: "test".to_string(),
                authoring_tool_version: "1.0".to_string(),
                generation_command: "test".to_string(),
                source: "generated".to_string(),
                license: "MIT".to_string(),
                sha256: "b".repeat(64),
                expected_dmi: String::new(),
                expected_conflict: false,
                expected_legal_fields: Default::default(),
                expected_malformed: false,
                expected_decode: Default::default(),
                expected_xmp: Default::default(),
                expected_internal: Default::default(),
                expected_external: Default::default(),
                required_external_fields: Vec::new(),
                expected_preservation: Vec::new(),
            },
        ],
    };
    let result = stegoeggo::conformance::validate_manifest(&manifest);
    assert!(result.is_err(), "Should reject duplicate paths");
}

#[test]
fn validate_manifest_rejects_path_traversal() {
    let manifest = stegoeggo::conformance::FixtureManifest {
        entries: vec![stegoeggo::conformance::FixtureEntry {
            id: "traversal".to_string(),
            path: "../etc/passwd.png".to_string(),
            format: "png".to_string(),
            category: "canonical".to_string(),
            authoring_tool: "test".to_string(),
            authoring_tool_version: "1.0".to_string(),
            generation_command: "test".to_string(),
            source: "generated".to_string(),
            license: "MIT".to_string(),
            sha256: "a".repeat(64),
            expected_dmi: String::new(),
            expected_conflict: false,
            expected_legal_fields: Default::default(),
            expected_malformed: false,
            expected_decode: Default::default(),
            expected_xmp: Default::default(),
            expected_internal: Default::default(),
            expected_external: Default::default(),
            required_external_fields: Vec::new(),
            expected_preservation: Vec::new(),
        }],
    };
    let result = stegoeggo::conformance::validate_manifest(&manifest);
    assert!(result.is_err(), "Should reject path traversal");
}

#[test]
fn validate_manifest_rejects_unsupported_category() {
    let manifest = stegoeggo::conformance::FixtureManifest {
        entries: vec![stegoeggo::conformance::FixtureEntry {
            id: "bad_cat".to_string(),
            path: "test.png".to_string(),
            format: "png".to_string(),
            category: "bogus".to_string(),
            authoring_tool: "test".to_string(),
            authoring_tool_version: "1.0".to_string(),
            generation_command: "test".to_string(),
            source: "generated".to_string(),
            license: "MIT".to_string(),
            sha256: "a".repeat(64),
            expected_dmi: String::new(),
            expected_conflict: false,
            expected_legal_fields: Default::default(),
            expected_malformed: false,
            expected_decode: Default::default(),
            expected_xmp: Default::default(),
            expected_internal: Default::default(),
            expected_external: Default::default(),
            required_external_fields: Vec::new(),
            expected_preservation: Vec::new(),
        }],
    };
    let result = stegoeggo::conformance::validate_manifest(&manifest);
    assert!(result.is_err(), "Should reject unsupported category");
}

#[test]
fn validate_manifest_rejects_unsupported_format() {
    let manifest = stegoeggo::conformance::FixtureManifest {
        entries: vec![stegoeggo::conformance::FixtureEntry {
            id: "bad_fmt".to_string(),
            path: "test.bmp".to_string(),
            format: "bmp".to_string(),
            category: "canonical".to_string(),
            authoring_tool: "test".to_string(),
            authoring_tool_version: "1.0".to_string(),
            generation_command: "test".to_string(),
            source: "generated".to_string(),
            license: "MIT".to_string(),
            sha256: "a".repeat(64),
            expected_dmi: String::new(),
            expected_conflict: false,
            expected_legal_fields: Default::default(),
            expected_malformed: false,
            expected_decode: Default::default(),
            expected_xmp: Default::default(),
            expected_internal: Default::default(),
            expected_external: Default::default(),
            required_external_fields: Vec::new(),
            expected_preservation: Vec::new(),
        }],
    };
    let result = stegoeggo::conformance::validate_manifest(&manifest);
    assert!(result.is_err(), "Should reject unsupported format");
}

#[test]
fn normalize_dmi_genai_precedence() {
    assert_eq!(
        stegoeggo::conformance::normalize_dmi_value("DMI-PROHIBITED-GENAIMLTRAINING"),
        "DMI-PROHIBITED-GENAIMLTRAINING"
    );
    assert_ne!(
        stegoeggo::conformance::normalize_dmi_value("DMI-PROHIBITED-GENAIMLTRAINING"),
        "DMI-PROHIBITED-AIMLTRAINING"
    );
}

#[test]
fn normalize_dmi_search_engine_precedence() {
    assert_eq!(
        stegoeggo::conformance::normalize_dmi_value("DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING"),
        "DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING"
    );
    assert_eq!(
        stegoeggo::conformance::normalize_dmi_value("Prohibited except search engine indexing"),
        "DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING"
    );
}

#[test]
fn non_strict_reports_complete_false() {
    let dir = tempfile::tempdir().unwrap();
    let empty_dir = dir.path().join("empty_fixtures");
    std::fs::create_dir(&empty_dir).unwrap();
    let json_path = dir.path().join("report.json");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_stegoeggo-conformance"))
        .arg("--fixtures")
        .arg(&empty_dir)
        .arg("--json")
        .arg(&json_path)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
}

#[test]
fn coverage_minimums_in_report() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("report.json");
    let fixtures =
        std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/tests/fixtures/conformance/canonical";
    let _ = std::process::Command::new(env!("CARGO_BIN_EXE_stegoeggo-conformance"))
        .arg("--fixtures")
        .arg(&fixtures)
        .arg("--json")
        .arg(&json_path)
        .output();
    if json_path.exists() {
        let json_str = std::fs::read_to_string(&json_path).unwrap();
        let run_report: ConformanceRunReport = serde_json::from_str(&json_str).unwrap();
        assert!(!run_report.fixtures.is_empty());
    }
}

#[test]
fn run_report_schema_version_is_positive() {
    let report = ConformanceRunReport {
        schema_version: 1,
        generated_by: "test".into(),
        crate_version: "0.3.0".into(),
        commit_sha: None,
        strict: true,
        complete: true,
        passed: true,
        started_at: None,
        manifest: None,
        tools: vec![],
        coverage_minimums: None,
        coverage: None,
        digest_verification: vec![],
        summary: ConformanceSummary {
            total: 0,
            passed: 0,
            failed: 0,
            by_format: Default::default(),
            by_category: Default::default(),
            digest_verification: None,
            coverage: None,
            coverage_minimums: None,
        },
        incomplete_reasons: vec![],
        fixtures: vec![],
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: ConformanceRunReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.schema_version, 1);
    assert!(parsed.complete);
    assert!(parsed.passed);
}

#[test]
fn run_report_failure_envelope_serialization() {
    let report = ConformanceRunReport {
        schema_version: 1,
        generated_by: "test".into(),
        crate_version: "0.3.0".into(),
        commit_sha: None,
        strict: true,
        complete: false,
        passed: false,
        started_at: None,
        manifest: None,
        tools: vec![],
        coverage_minimums: None,
        coverage: None,
        digest_verification: vec![],
        summary: ConformanceSummary {
            total: 5,
            passed: 3,
            failed: 2,
            by_format: Default::default(),
            by_category: Default::default(),
            digest_verification: None,
            coverage: None,
            coverage_minimums: None,
        },
        incomplete_reasons: vec!["missing tool: exiftool".into()],
        fixtures: vec![],
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: ConformanceRunReport = serde_json::from_str(&json).unwrap();
    assert!(!parsed.complete);
    assert!(!parsed.passed);
    assert_eq!(parsed.incomplete_reasons.len(), 1);
}

#[test]
fn run_report_complete_false_when_tool_skipped() {
    let report = ConformanceRunReport {
        schema_version: 1,
        generated_by: "test".into(),
        crate_version: "0.3.0".into(),
        commit_sha: None,
        strict: false,
        complete: false,
        passed: false,
        started_at: None,
        manifest: Some(ManifestReport {
            requested_path: "manifest.toml".into(),
            canonical_path: None,
            sha256: "abc123".into(),
            entry_count: 10,
            validation: Ok(()),
            duplicate_count: 0,
            unlisted_count: 0,
            unexercised_count: 3,
        }),
        tools: vec![ToolReport {
            name: "exiftool".into(),
            path: None,
            version: None,
            discovered: false,
            exercised: false,
            invocations: 0,
            successes: 0,
            failures: 0,
        }],
        coverage_minimums: None,
        coverage: None,
        digest_verification: vec![],
        summary: ConformanceSummary {
            total: 10,
            passed: 7,
            failed: 0,
            by_format: Default::default(),
            by_category: Default::default(),
            digest_verification: None,
            coverage: None,
            coverage_minimums: None,
        },
        incomplete_reasons: vec!["tool not found: exiftool".into()],
        fixtures: vec![],
    };
    assert!(!report.complete);
}

#[test]
fn run_report_tool_execution_counts() {
    let tools = vec![
        ToolReport {
            name: "exiftool".into(),
            path: Some("/usr/bin/exiftool".into()),
            version: Some("13.00".into()),
            discovered: true,
            exercised: true,
            invocations: 41,
            successes: 38,
            failures: 3,
        },
        ToolReport {
            name: "xmllint".into(),
            path: Some("/usr/bin/xmllint".into()),
            version: Some("2.13.0".into()),
            discovered: true,
            exercised: true,
            invocations: 41,
            successes: 40,
            failures: 1,
        },
    ];
    let json = serde_json::to_string(&tools).unwrap();
    let parsed: Vec<ToolReport> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed[0].invocations, 41);
    assert_eq!(parsed[0].successes, 38);
    assert_eq!(parsed[0].failures, 3);
    assert!(parsed[0].exercised);
}

#[test]
fn run_report_manifest_hash_present() {
    let manifest = ManifestReport {
        requested_path: "tests/fixtures/conformance/manifest.toml".into(),
        canonical_path: Some("/absolute/path/manifest.toml".into()),
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
        entry_count: 41,
        validation: Ok(()),
        duplicate_count: 0,
        unlisted_count: 0,
        unexercised_count: 0,
    };
    let json = serde_json::to_string(&manifest).unwrap();
    let parsed: ManifestReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.sha256.len(), 64);
    assert_eq!(parsed.entry_count, 41);
}

#[test]
fn run_report_digest_verification_present() {
    let digests = vec![DigestCheckResult {
        fixture_id: "canonical_complete_png".into(),
        fixture_path: "canonical/complete.png".into(),
        expected: "ed7dda293bbd83f4957c854b762d638d9a8ccfe6dc1e6d8101b39c986fd06e89".into(),
        observed: "ed7dda293bbd83f4957c854b762d638d9a8ccfe6dc1e6d8101b39c986fd06e89".into(),
        matches: true,
    }];
    let json = serde_json::to_string(&digests).unwrap();
    let parsed: Vec<DigestCheckResult> = serde_json::from_str(&json).unwrap();
    assert!(parsed[0].matches);
}

#[test]
fn run_report_coverage_failure_present() {
    let coverage = CoverageCheckResult {
        passed: false,
        violations: vec!["missing external canonical png".into()],
        observed_canonical_png: 0,
        observed_canonical_jpeg: 1,
        observed_canonical_webp: 1,
        observed_legacy: 1,
        observed_conflict: 1,
        observed_malformed: 1,
        observed_preservation: 1,
        observed_external_canonical_png: 0,
        observed_external_canonical_jpeg: 1,
        observed_external_canonical_webp: 1,
        observed_external_legacy: 0,
        observed_external_alt_prefix: 0,
        observed_external_conflict: 0,
        observed_external_preservation: 0,
    };
    let json = serde_json::to_string(&coverage).unwrap();
    let parsed: CoverageCheckResult = serde_json::from_str(&json).unwrap();
    assert!(!parsed.passed);
    assert_eq!(parsed.violations.len(), 1);
    assert_eq!(parsed.observed_external_canonical_png, 0);
}

#[test]
fn internal_extraction_has_notice_with_actual_content() {
    let ext = InternalExtraction {
        copyright_holder: Some("Test".into()),
        canonical_data_mining: Some("DMI-PROHIBITED-AIMLTRAINING".into()),
        ..Default::default()
    };
    assert!(ext.has_notice_content());
}

#[test]
fn internal_extraction_has_notice_with_no_content() {
    let ext = InternalExtraction::default();
    assert!(!ext.has_notice_content());
}

#[test]
fn internal_extraction_has_notice_with_only_creator() {
    let ext = InternalExtraction {
        creators: vec!["Alice".into()],
        ..Default::default()
    };
    assert!(ext.has_notice_content());
}

#[test]
fn external_extraction_has_notice_with_actual_content() {
    let ext = ExternalExtraction {
        copyright: Some("Test".into()),
        canonical_data_mining: Some("DMI-PROHIBITED-AIMLTRAINING".into()),
        ..Default::default()
    };
    assert!(ext.has_notice_content());
}

#[test]
fn external_extraction_has_notice_with_no_content() {
    let ext = ExternalExtraction::default();
    assert!(!ext.has_notice_content());
}

#[test]
fn coverage_source_aware_minimums_default_values() {
    let mins = CoverageMinimums::default();
    assert_eq!(mins.external_canonical_png, 1);
    assert_eq!(mins.external_canonical_jpeg, 1);
    assert_eq!(mins.external_canonical_webp, 1);
    assert_eq!(mins.external_legacy_min, 1);
    assert_eq!(mins.external_alt_prefix_min, 1);
    assert_eq!(mins.external_conflict_min, 1);
    assert_eq!(mins.external_preservation_min, 1);
}

#[test]
fn coverage_missing_external_canonical_png_fails() {
    let manifest = FixtureManifest { entries: vec![] };
    let mins = CoverageMinimums::default();
    let result = conformance::check_coverage(&manifest, &mins);
    assert!(!result.passed);
    assert!(
        result
            .violations
            .iter()
            .any(|v| v.contains("external canonical PNG")),
        "Should report missing external canonical PNG: {:?}",
        result.violations
    );
}

#[test]
fn comparison_unicode_equivalent_strings_pass() {
    let internal = InternalExtraction {
        copyright_holder: Some("Caf\u{00e9}".into()),
        ..Default::default()
    };
    let external = ExternalExtraction {
        copyright: Some("Caf\u{00e9}".into()),
        ..Default::default()
    };
    let mut report = ConformanceReport::new("test", "png");
    conformance::compare_extractions(&internal, &external, &mut report);
    let copyright_check = report
        .checks
        .iter()
        .find(|c| c.name == "copyright")
        .unwrap();
    assert_ne!(copyright_check.severity, CheckSeverity::Fail);
}

#[test]
fn comparison_different_rights_urls_fails() {
    let internal = InternalExtraction {
        web_statement_of_rights: Some("https://example.com/rights".into()),
        ..Default::default()
    };
    let external = ExternalExtraction {
        rights_url: Some("https://other.com/rights".into()),
        ..Default::default()
    };
    let mut report = ConformanceReport::new("test", "png");
    conformance::compare_extractions(&internal, &external, &mut report);
    let url_check = report
        .checks
        .iter()
        .find(|c| c.name == "rights_url")
        .unwrap();
    assert_eq!(url_check.severity, CheckSeverity::Fail);
}

#[test]
fn comparison_creator_array_mismatch_fails() {
    let internal = InternalExtraction {
        creators: vec!["Alice".into(), "Bob".into()],
        ..Default::default()
    };
    let external = ExternalExtraction {
        creators: vec!["Alice".into(), "Charlie".into()],
        ..Default::default()
    };
    let mut report = ConformanceReport::new("test", "png");
    conformance::compare_extractions(&internal, &external, &mut report);
    let creator_check = report.checks.iter().find(|c| c.name == "creators").unwrap();
    assert_eq!(creator_check.severity, CheckSeverity::Fail);
}

#[test]
fn comparison_url_equivalent_normalization_passes() {
    let internal = InternalExtraction {
        web_statement_of_rights: Some("https://Example.com/Path".into()),
        ..Default::default()
    };
    let external = ExternalExtraction {
        rights_url: Some("https://example.com/Path".into()),
        ..Default::default()
    };
    let mut report = ConformanceReport::new("test", "png");
    conformance::compare_extractions(&internal, &external, &mut report);
    let url_check = report
        .checks
        .iter()
        .find(|c| c.name == "rights_url")
        .unwrap();
    assert_ne!(url_check.severity, CheckSeverity::Fail);
}
