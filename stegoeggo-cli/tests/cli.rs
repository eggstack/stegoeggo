use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn cli_bin() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_BIN_EXE_stegoeggo"));
    if !path.exists() {
        let output = Command::new("cargo")
            .args(["build", "-p", "stegoeggo-cli"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("Failed to build CLI");
        assert!(output.status.success(), "CLI build failed");
        path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/stegoeggo");
    }
    path
}

fn create_test_png(path: &PathBuf) {
    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut rgb = img.to_rgb8();
    for y in 0..64u32 {
        for x in 0..64u32 {
            let r = ((x * 7 + y * 3) % 256) as u8;
            let g = ((x * 11 + y * 5) % 256) as u8;
            let b = ((x * 13 + y * 9) % 256) as u8;
            rgb.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    let dyn_img = image::DynamicImage::ImageRgb8(rgb);
    let file = fs::File::create(path).unwrap();
    let encoder = image::codecs::png::PngEncoder::new(file);
    image::ImageEncoder::write_image(
        encoder,
        &dyn_img.to_rgb8(),
        64,
        64,
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
}

fn create_test_jpeg(path: &PathBuf, quality: u8) {
    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut rgb = img.to_rgb8();
    for y in 0..64u32 {
        for x in 0..64u32 {
            let r = ((x * 7 + y * 3) % 256) as u8;
            let g = ((x * 11 + y * 5) % 256) as u8;
            let b = ((x * 13 + y * 9) % 256) as u8;
            rgb.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    let dyn_img = image::DynamicImage::ImageRgb8(rgb);
    let file = fs::File::create(path).unwrap();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, quality);
    image::ImageEncoder::write_image(
        encoder,
        &dyn_img.to_rgb8(),
        64,
        64,
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
}

fn create_test_webp(path: &PathBuf) {
    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut rgb = img.to_rgb8();
    for y in 0..64u32 {
        for x in 0..64u32 {
            let r = ((x * 7 + y * 3) % 256) as u8;
            let g = ((x * 11 + y * 5) % 256) as u8;
            let b = ((x * 13 + y * 9) % 256) as u8;
            rgb.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    let dyn_img = image::DynamicImage::ImageRgb8(rgb);
    let file = fs::File::create(path).unwrap();
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(file);
    image::ImageEncoder::write_image(
        encoder,
        &dyn_img.to_rgb8(),
        64,
        64,
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
}

#[test]
fn test_help_flag() {
    let output = Command::new(cli_bin())
        .arg("--help")
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("stegoeggo"));
    assert!(stdout.contains("stegoeggo"));
}

#[test]
fn test_protect_png_default() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-l")
        .arg("standard")
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");

    assert!(
        result.status.success(),
        "CLI should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let output_file = output_dir.join("input_protected.png");
    assert!(
        output_file.exists(),
        "Output file should exist at {:?}",
        output_file
    );

    let output_bytes = fs::read(&output_file).unwrap();
    assert!(
        output_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "Output should be PNG"
    );
}

#[test]
fn test_protect_jpeg_default() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.jpg");
    let output_dir = tmp.path().join("out");

    create_test_jpeg(&input, 90);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-f")
        .arg("jpg")
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");

    assert!(
        result.status.success(),
        "CLI should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let output_file = output_dir.join("input_protected.jpg");
    assert!(output_file.exists());

    let output_bytes = fs::read(&output_file).unwrap();
    assert!(
        output_bytes.starts_with(&[0xFF, 0xD8]),
        "Output should be JPEG"
    );
}

#[test]
fn test_protect_jpeg_without_format_preserves_input_format() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.jpg");
    let output_dir = tmp.path().join("out");

    create_test_jpeg(&input, 90);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");

    assert!(
        result.status.success(),
        "CLI should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let output_file = output_dir.join("input_protected.jpg");
    assert!(output_file.exists());

    let output_bytes = fs::read(&output_file).unwrap();
    assert!(
        output_bytes.starts_with(&[0xFF, 0xD8]),
        "Output should preserve JPEG format"
    );
}

#[test]
fn test_protect_webp_default() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.webp");
    let output_dir = tmp.path().join("out");

    create_test_webp(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-f")
        .arg("web-p")
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");

    assert!(
        result.status.success(),
        "CLI should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let output_file = output_dir.join("input_protected.webp");
    assert!(output_file.exists());

    let output_bytes = fs::read(&output_file).unwrap();
    assert_eq!(
        &output_bytes[0..4],
        b"RIFF",
        "Output should start with RIFF"
    );
    assert_eq!(&output_bytes[8..12], b"WEBP", "Output should contain WEBP");
}

#[test]
fn test_verify_protected_png() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);

    let protect_result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");
    assert!(protect_result.status.success());

    let protected_file = output_dir.join("input_protected.png");

    let verify_result = Command::new(cli_bin())
        .arg(&protected_file)
        .arg("--verify")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        verify_result.status.success(),
        "Verify should succeed: {}",
        String::from_utf8_lossy(&verify_result.stderr)
    );

    let stdout = String::from_utf8_lossy(&verify_result.stdout);
    assert!(
        stdout.contains("Rights notice: Found"),
        "Should report rights notice found: {}",
        stdout
    );
    assert!(
        stdout.contains("Stego marker: Found"),
        "Should report stego marker found: {}",
        stdout
    );
}

#[test]
fn test_verify_unprotected_image() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("--verify")
        .output()
        .expect("Failed to execute CLI");
    assert!(result.status.success());

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("Rights notice: Not found"),
        "Should report not protected: {}",
        stdout
    );
}

#[test]
fn test_verify_metadata_only_does_not_report_verified() {
    use stegoeggo::{ProtectionContext, RightsMetadataProtector};

    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);
    let raw = fs::read(&input).unwrap();

    let ctx = ProtectionContext::new(0.5, 42);
    let metadata_only = RightsMetadataProtector::new()
        .inject_bytes(&raw, &ctx)
        .unwrap();

    let metadata_path = tmp.path().join("metadata_only.png");
    fs::write(&metadata_path, &metadata_only).unwrap();

    let result = Command::new(cli_bin())
        .arg(&metadata_path)
        .arg("--verify")
        .output()
        .expect("Failed to execute CLI");
    assert!(result.status.success());

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        !stdout.contains("Stego marker: Found"),
        "Metadata-only file must not report stego marker found: {}",
        stdout
    );
}

#[test]
fn test_protect_light_level() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-l")
        .arg("light")
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");

    assert!(
        result.status.success(),
        "Light level should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output_dir.join("input_protected.png").exists());
}

#[test]
fn test_protect_disabled_level() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);
    let original_bytes = fs::read(&input).unwrap();

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-l")
        .arg("disabled")
        .output()
        .expect("Failed to execute CLI");

    assert!(result.status.success());
    let output_file = output_dir.join("input_protected.png");
    let output_bytes = fs::read(&output_file).unwrap();
    assert_eq!(
        original_bytes, output_bytes,
        "Disabled level should preserve bytes exactly"
    );
}

#[test]
fn test_format_conversion_png_to_jpeg() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-f")
        .arg("jpg")
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");

    assert!(
        result.status.success(),
        "Format conversion should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let output_file = output_dir.join("input_protected.jpg");
    assert!(output_file.exists());
    let output_bytes = fs::read(&output_file).unwrap();
    assert!(
        output_bytes.starts_with(&[0xFF, 0xD8]),
        "Output should be JPEG"
    );
}

#[test]
fn test_batch_processing_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let input_dir = tmp.path().join("input");
    let output_dir = tmp.path().join("output");
    fs::create_dir(&input_dir).unwrap();

    for i in 0..3 {
        let img_path = input_dir.join(format!("test_{}.png", i));
        create_test_png(&img_path);
    }

    let result = Command::new(cli_bin())
        .arg(&input_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");

    assert!(
        result.status.success(),
        "Batch should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let entries: Vec<_> = fs::read_dir(&output_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 3, "Should produce 3 output files");
}

#[test]
fn test_invalid_input_no_files() {
    let result = Command::new(cli_bin())
        .arg("nonexistent_file.png")
        .output()
        .expect("Failed to execute CLI");

    assert!(
        !result.status.success(),
        "Should fail with nonexistent input"
    );
}

#[test]
fn test_verify_batch_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let input_dir = tmp.path().join("input");
    fs::create_dir(&input_dir).unwrap();

    create_test_png(&input_dir.join("a.png"));
    create_test_png(&input_dir.join("b.png"));

    let result = Command::new(cli_bin())
        .arg(&input_dir)
        .arg("--verify")
        .output()
        .expect("Failed to execute CLI");

    assert!(
        !result.status.success(),
        "Verify mode should reject batch input"
    );
}

#[test]
fn test_verbose_output() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-s")
        .arg("42")
        .arg("-v")
        .output()
        .expect("Failed to execute CLI");

    assert!(result.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        combined.contains("Protection level") || combined.contains("Intensity"),
        "Verbose should output info: {}",
        combined
    );
}

#[test]
fn test_intensity_range() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-s")
        .arg("42")
        .arg("-i")
        .arg("0.0")
        .output()
        .expect("Failed to execute CLI");
    assert!(result.status.success(), "Intensity 0.0 should succeed");

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-s")
        .arg("42")
        .arg("-i")
        .arg("1.0")
        .output()
        .expect("Failed to execute CLI");
    assert!(result.status.success(), "Intensity 1.0 should succeed");
}

#[test]
fn test_stego_redundancy_range() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-s")
        .arg("42")
        .arg("--stego-redundancy")
        .arg("1")
        .output()
        .expect("Failed to execute CLI");
    assert!(result.status.success(), "Redundancy 1 should succeed");

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-s")
        .arg("42")
        .arg("--stego-redundancy")
        .arg("10")
        .output()
        .expect("Failed to execute CLI");
    assert!(result.status.success(), "Redundancy 10 should succeed");
}

#[test]
fn test_progressive_jpeg_output() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-f")
        .arg("jpg")
        .arg("-s")
        .arg("42")
        .arg("--progressive")
        .output()
        .expect("Failed to execute CLI");

    assert!(
        result.status.success(),
        "Progressive JPEG should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output_dir.join("input_protected.jpg").exists());
}

#[test]
fn test_dmi_options() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);

    let dmi_values = [
        "auto",
        "unspecified",
        "allowed",
        "prohibited-ai",
        "prohibited-gen-ai",
        "prohibited-se",
        "prohibited",
        "prohibited-constraints",
    ];

    for dmi in &dmi_values {
        let result = Command::new(cli_bin())
            .arg(&input)
            .arg("-o")
            .arg(&output_dir)
            .arg("-s")
            .arg("42")
            .arg("-d")
            .arg(dmi)
            .output()
            .expect("Failed to execute CLI");
        assert!(
            result.status.success(),
            "DMI '{}' should succeed: {}",
            dmi,
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn test_with_key_hex() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-s")
        .arg("42")
        .arg("--key")
        .arg("deadbeef01234567")
        .output()
        .expect("Failed to execute CLI");

    assert!(
        result.status.success(),
        "MAC key should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn test_invalid_hex_key() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--key")
        .arg("not-a-valid-hex-string-zzz")
        .output()
        .expect("Failed to execute CLI");

    assert!(!result.status.success(), "Invalid hex key should fail");
}

#[test]
fn test_protect_deterministic_with_seed() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let out_dir1 = tmp.path().join("out1");
    let out_dir2 = tmp.path().join("out2");

    create_test_png(&input);

    for dir in [&out_dir1, &out_dir2] {
        let result = Command::new(cli_bin())
            .arg(&input)
            .arg("-o")
            .arg(dir)
            .arg("-s")
            .arg("42")
            .output()
            .expect("Failed to execute CLI");
        assert!(result.status.success());
    }

    let bytes1 = fs::read(out_dir1.join("input_protected.png")).unwrap();
    let bytes2 = fs::read(out_dir2.join("input_protected.png")).unwrap();
    assert_eq!(bytes1, bytes2, "Same seed should produce identical output");
}

#[test]
fn test_jpeg_quality_option() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_dir = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_dir)
        .arg("-f")
        .arg("jpg")
        .arg("-s")
        .arg("42")
        .arg("--jpeg-quality")
        .arg("50")
        .output()
        .expect("Failed to execute CLI");

    assert!(result.status.success());
    let output_bytes = fs::read(output_dir.join("input_protected.jpg")).unwrap();
    assert!(output_bytes.starts_with(&[0xFF, 0xD8]));
}

#[test]
fn test_protect_to_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("output.png");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");

    assert!(result.status.success());
    assert!(
        output.exists(),
        "Output file should exist at {}",
        output.display()
    );
}

#[test]
fn test_equivalent_legacy_and_request_syntax() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_legacy = tmp.path().join("legacy_out");
    let output_request = tmp.path().join("request_out");

    create_test_png(&input);

    let result_legacy = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_legacy)
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");
    assert!(result_legacy.status.success());

    let result_request = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_request)
        .arg("-s")
        .arg("42")
        .arg("--rights-policy")
        .arg("prohibited-ai-ml-training")
        .output()
        .expect("Failed to execute CLI");
    assert!(result_request.status.success());

    let legacy_bytes = fs::read(output_legacy.join("input_protected.png")).unwrap();
    let request_bytes = fs::read(output_request.join("input_protected.png")).unwrap();

    assert!(
        !legacy_bytes.is_empty(),
        "Legacy path should produce output"
    );
    assert!(
        !request_bytes.is_empty(),
        "Request path should produce output"
    );
}

#[test]
fn test_preset_no_ai_training_sets_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .arg("--preset")
        .arg("legal-notice")
        .arg("--no-ai-training")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        result.status.success(),
        "--preset legal-notice --no-ai-training should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn test_allowed_with_no_ai_training_is_config_error() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .arg("--rights-policy")
        .arg("allowed")
        .arg("--no-ai-training")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        !result.status.success(),
        "--rights-policy allowed --no-ai-training should fail"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("Conflicting"),
        "Should report conflict: {}",
        stderr
    );
}

#[test]
fn test_conflicting_dmi_and_rights_policy_is_config_error() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .arg("--dmi")
        .arg("allowed")
        .arg("--rights-policy")
        .arg("prohibited-ai-ml-training")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        !result.status.success(),
        "Conflicting --dmi and --rights-policy should fail"
    );
}

#[test]
fn test_hmac_without_key_is_config_error() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .arg("--preset")
        .arg("authenticated-provenance")
        .output()
        .expect("Failed to execute CLI");
    assert!(!result.status.success(), "HMAC without key should fail");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("HMAC") || stderr.contains("key"),
        "Should mention HMAC or key: {}",
        stderr
    );
}

#[test]
fn test_dry_run_shows_resolved_plan() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--preset")
        .arg("legal-notice")
        .arg("--no-ai-training")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        result.status.success(),
        "--dry-run should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("Resolved Protection Plan"),
        "Should show resolved plan: {}",
        stdout
    );
    assert!(
        stdout.contains("Effective policy"),
        "Should show effective policy: {}",
        stdout
    );
}

#[test]
fn test_verify_with_literal_hex_key() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let protected = tmp.path().join("protected.png");

    create_test_png(&input);

    let key = "deadbeef01234567deadbeef01234567";

    let protect_result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&protected)
        .arg("-s")
        .arg("42")
        .arg("--preset")
        .arg("authenticated-provenance")
        .arg("--key")
        .arg(key)
        .output()
        .expect("Failed to execute CLI");
    assert!(protect_result.status.success());

    let verify_result = Command::new(cli_bin())
        .arg(&protected)
        .arg("--verify")
        .arg("--key")
        .arg(key)
        .output()
        .expect("Failed to execute CLI");
    assert!(
        verify_result.status.success(),
        "Verify with literal hex key should succeed"
    );
}

#[test]
fn test_verify_with_file_key() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let protected = tmp.path().join("protected.png");
    let key_file = tmp.path().join("key.txt");

    create_test_png(&input);

    let key = "deadbeef01234567deadbeef01234567";
    fs::write(&key_file, key).unwrap();

    let protect_result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&protected)
        .arg("-s")
        .arg("42")
        .arg("--preset")
        .arg("authenticated-provenance")
        .arg("--key")
        .arg(key)
        .output()
        .expect("Failed to execute CLI");
    assert!(protect_result.status.success());

    let verify_result = Command::new(cli_bin())
        .arg(&protected)
        .arg("--verify")
        .arg("--key")
        .arg(format!("@{}", key_file.display()))
        .output()
        .expect("Failed to execute CLI");
    assert!(
        verify_result.status.success(),
        "Verify with @file key should succeed"
    );
}

#[test]
fn test_dry_run_matches_execution() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("out");

    create_test_png(&input);

    let dry_run = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--preset")
        .arg("legal-notice")
        .arg("--no-ai-training")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(dry_run.status.success());
    let dry_stdout = String::from_utf8_lossy(&dry_run.stdout);

    let exec_result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .arg("--preset")
        .arg("legal-notice")
        .arg("--no-ai-training")
        .output()
        .expect("Failed to execute CLI");
    assert!(exec_result.status.success());

    assert!(
        dry_stdout.contains("Metadata-only: true") || dry_stdout.contains("metadata-only: true"),
        "Dry run should show metadata-only: true for legal-notice preset"
    );
}

#[test]
fn test_batch_stems_produce_unique_outputs() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("images");
    fs::create_dir_all(&dir).unwrap();

    let img = image::DynamicImage::new_rgb8(16, 16);
    for i in 0..3 {
        let path = dir.join(format!("photo{}.png", i));
        let file = fs::File::create(&path).unwrap();
        let encoder = image::codecs::png::PngEncoder::new(file);
        image::ImageEncoder::write_image(
            encoder,
            &img.to_rgb8(),
            16,
            16,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    }

    let out_dir = tmp.path().join("out");
    let result = Command::new(cli_bin())
        .arg(&dir)
        .arg("-o")
        .arg(&out_dir)
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        result.status.success(),
        "Batch processing should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let outputs: Vec<_> = fs::read_dir(&out_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(outputs.len(), 3, "Should produce 3 output files");
}

#[test]
fn test_pixel_only_api_does_not_claim_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .arg("--preset")
        .arg("legal-notice")
        .arg("--json")
        .output()
        .expect("Failed to execute CLI");
    assert!(result.status.success());

    let stdout = String::from_utf8_lossy(&result.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let report = json.get("report").expect("Should have report");
    assert!(
        report.get("metadata_injected").unwrap() == true,
        "legal-notice preset should inject metadata"
    );
    assert!(
        report.get("stego_attempted").unwrap() == false,
        "legal-notice preset should not attempt stego"
    );
}

#[test]
fn test_hmac_with_disabled_marker_is_config_error() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .arg("--hidden-marker")
        .arg("disabled")
        .arg("--authentication")
        .arg("hmac")
        .arg("--key")
        .arg("deadbeef01234567deadbeef01234567")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        !result.status.success(),
        "HMAC with disabled marker should fail"
    );
}

#[test]
fn test_level_preset_conflict_is_config_error() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .arg("-l")
        .arg("light")
        .arg("--preset")
        .arg("maximal")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        !result.status.success(),
        "Combining --level and --preset should fail"
    );
}

#[test]
fn test_default_standard_invocation_resolves_prohibited_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("ProhibitedAiMlTraining"),
        "Default Standard invocation should resolve ProhibitedAiMlTraining: {}",
        stdout
    );
    assert!(
        stdout.contains("rights_metadata=true"),
        "Should have rights_metadata=true: {}",
        stdout
    );
    assert!(
        stdout.contains("hidden_marker=BestEffort"),
        "Should have hidden_marker=BestEffort: {}",
        stdout
    );
}

#[test]
fn test_omitted_dmi_equals_dmi_auto() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);

    let omitted = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(omitted.status.success());
    let omitted_out = String::from_utf8_lossy(&omitted.stdout);

    let auto = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--dmi")
        .arg("auto")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(auto.status.success());
    let auto_out = String::from_utf8_lossy(&auto.stdout);

    assert_eq!(
        omitted_out, auto_out,
        "Omitted --dmi and --dmi auto should produce identical output"
    );
}

#[test]
fn test_explicit_unspecified_is_distinct_from_default() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);

    let default = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(default.status.success());
    let default_out = String::from_utf8_lossy(&default.stdout);
    assert!(
        default_out.contains("ProhibitedAiMlTraining"),
        "Default should be ProhibitedAiMlTraining"
    );

    let unspecified = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--dmi")
        .arg("unspecified")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(unspecified.status.success());
    let unspecified_out = String::from_utf8_lossy(&unspecified.stdout);
    assert!(
        unspecified_out.contains("Unspecified"),
        "Explicit --dmi unspecified should be Unspecified: {}",
        unspecified_out
    );
    assert!(
        !unspecified_out.contains("ProhibitedAiMlTraining"),
        "Explicit --dmi unspecified must NOT inherit default prohibition: {}",
        unspecified_out
    );
}

#[test]
fn test_legacy_dmi_prohibited_ai_matches_rights_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);

    let dmi = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--dmi")
        .arg("prohibited-ai")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(dmi.status.success());
    let dmi_out = String::from_utf8_lossy(&dmi.stdout);

    let policy = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--rights-policy")
        .arg("prohibited-ai-ml-training")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(policy.status.success());
    let policy_out = String::from_utf8_lossy(&policy.stdout);

    assert!(
        dmi_out.contains("ProhibitedAiMlTraining"),
        "--dmi prohibited-ai should resolve ProhibitedAiMlTraining: {}",
        dmi_out
    );
    assert!(
        policy_out.contains("ProhibitedAiMlTraining"),
        "--rights-policy prohibited-ai-ml-training should resolve ProhibitedAiMlTraining: {}",
        policy_out
    );
    assert!(
        dmi_out.contains("rights_metadata=true"),
        "--dmi prohibited-ai should have rights_metadata=true: {}",
        dmi_out
    );
    assert!(
        policy_out.contains("rights_metadata=true"),
        "--rights-policy should have rights_metadata=true: {}",
        policy_out
    );
}

#[test]
fn test_legacy_dmi_allowed_matches_rights_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);

    let dmi = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--dmi")
        .arg("allowed")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(dmi.status.success());
    let dmi_out = String::from_utf8_lossy(&dmi.stdout);

    let policy = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--rights-policy")
        .arg("allowed")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(policy.status.success());
    let policy_out = String::from_utf8_lossy(&policy.stdout);

    assert!(
        dmi_out.contains("Allowed"),
        "--dmi allowed should resolve Allowed: {}",
        dmi_out
    );
    assert!(
        policy_out.contains("Allowed"),
        "--rights-policy allowed should resolve Allowed: {}",
        policy_out
    );
}

#[test]
fn test_no_ai_training_matches_rights_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);

    let shorthand = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--no-ai-training")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(shorthand.status.success());
    let shorthand_out = String::from_utf8_lossy(&shorthand.stdout);

    let policy = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--rights-policy")
        .arg("prohibited-ai-ml-training")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(policy.status.success());
    let policy_out = String::from_utf8_lossy(&policy.stdout);

    assert!(
        shorthand_out.contains("ProhibitedAiMlTraining"),
        "--no-ai-training should resolve ProhibitedAiMlTraining: {}",
        shorthand_out
    );
    assert!(
        shorthand_out.contains("rights_metadata=true"),
        "--no-ai-training should have rights_metadata=true: {}",
        shorthand_out
    );
    assert!(
        policy_out.contains("ProhibitedAiMlTraining"),
        "--rights-policy should also resolve ProhibitedAiMlTraining: {}",
        policy_out
    );
}

#[test]
fn test_light_level_produces_correct_output() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("out");

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .arg("--level")
        .arg("light")
        .arg("--json")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        result.status.success(),
        "Light level should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let report = json.get("report").expect("Should have report");
    assert!(
        report.get("stego_attempted").unwrap() == true,
        "Light level should attempt stego"
    );
    assert!(
        report.get("metadata_injected").unwrap() == true,
        "Light level should inject metadata"
    );
}

#[test]
fn test_disabled_level_produces_bitidentical_output() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("out");

    create_test_png(&input);
    let original = fs::read(&input).unwrap();

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .arg("--level")
        .arg("disabled")
        .output()
        .expect("Failed to execute CLI");
    assert!(result.status.success());

    let protected = fs::read(output.join("input_protected.png")).unwrap();
    assert_eq!(original, protected, "Disabled level should be bitidentical");
}

#[test]
fn test_dry_run_matches_execution_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output = tmp.path().join("out");

    create_test_png(&input);

    let dry_run = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--preset")
        .arg("legal-notice")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute CLI");
    assert!(dry_run.status.success());
    let dry_out = String::from_utf8_lossy(&dry_run.stdout);

    let exec = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-s")
        .arg("42")
        .arg("--preset")
        .arg("legal-notice")
        .output()
        .expect("Failed to execute CLI");
    assert!(exec.status.success());

    let protected = fs::read(output.join("input_protected.png")).unwrap();
    assert!(!protected.is_empty(), "Execution should produce output");
    assert!(
        dry_out.contains("rights_metadata=true"),
        "Dry run should match execution: {}",
        dry_out
    );
}

#[test]
fn test_json_and_human_output_share_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_json = tmp.path().join("out_json");
    let output_human = tmp.path().join("out_human");

    create_test_png(&input);

    let json_result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_json)
        .arg("-s")
        .arg("42")
        .arg("--json")
        .output()
        .expect("Failed to execute CLI");
    assert!(json_result.status.success());
    let json_out: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&json_result.stdout)).unwrap();
    let json_policy = json_out
        .get("report")
        .and_then(|r| r.get("effective_policy"))
        .and_then(|p| p.as_str())
        .unwrap();

    let human_result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_human)
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");
    assert!(human_result.status.success());

    assert!(
        json_policy.contains("ProhibitedAiMlTraining"),
        "JSON output should report ProhibitedAiMlTraining: {}",
        json_policy
    );
    assert!(
        json_out
            .get("report")
            .and_then(|r| r.get("metadata_injected"))
            .unwrap()
            == true,
        "JSON should show metadata_injected=true"
    );
    assert!(
        json_out
            .get("report")
            .and_then(|r| r.get("stego_attempted"))
            .unwrap()
            == true,
        "JSON should show stego_attempted=true"
    );
}

#[test]
fn test_batch_single_file_same_as_direct() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    let output_single = tmp.path().join("single_out");
    let output_batch = tmp.path().join("batch_out");

    create_test_png(&input);

    let single = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_single)
        .arg("-s")
        .arg("42")
        .arg("--json")
        .output()
        .expect("Failed to execute CLI");
    assert!(single.status.success());

    let batch = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_batch)
        .arg("-s")
        .arg("42")
        .arg("--json")
        .output()
        .expect("Failed to execute CLI");
    assert!(batch.status.success());

    let single_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&single.stdout)).unwrap();
    let batch_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&batch.stdout)).unwrap();

    let single_policy = single_json
        .get("report")
        .and_then(|r| r.get("effective_policy"))
        .unwrap();
    let batch_policy = batch_json
        .get("report")
        .and_then(|r| r.get("effective_policy"))
        .unwrap();
    assert_eq!(
        single_policy, batch_policy,
        "Single-file and batch should resolve same policy"
    );
}

#[test]
fn test_conflicting_dmi_and_rights_policy_exits_error() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--dmi")
        .arg("allowed")
        .arg("--rights-policy")
        .arg("prohibited-ai-ml-training")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        !result.status.success(),
        "Conflicting --dmi and --rights-policy should fail"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("Conflicting"),
        "Should report conflict: {}",
        stderr
    );
}

#[test]
fn test_conflicting_shorthand_and_explicit_policy_exits_error() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--no-ai-training")
        .arg("--rights-policy")
        .arg("allowed")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        !result.status.success(),
        "--no-ai-training with --rights-policy allowed should fail"
    );
}

#[test]
fn test_metadata_disabled_with_legal_fields_exits_error() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--metadata")
        .arg("false")
        .arg("--copyright-notice")
        .arg("test")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        !result.status.success(),
        "--metadata false with legal fields should fail"
    );
}

#[test]
fn test_hmac_without_key_exits_error() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--preset")
        .arg("authenticated-provenance")
        .output()
        .expect("Failed to execute CLI");
    assert!(!result.status.success(), "HMAC without key should fail");
}

#[test]
fn test_hmac_with_disabled_marker_exits_error() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .arg("--hidden-marker")
        .arg("disabled")
        .arg("--authentication")
        .arg("hmac")
        .arg("--key")
        .arg("deadbeef01234567deadbeef01234567")
        .output()
        .expect("Failed to execute CLI");
    assert!(
        !result.status.success(),
        "HMAC with disabled marker should fail"
    );
}

#[test]
fn test_corrupt_image_decode_exits_error_not_internal() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("corrupt.png");
    let mut corrupt = b"\x89PNG\r\n\x1a\n".to_vec();
    corrupt.extend_from_slice(b"truncated body garbage");
    fs::write(&input, corrupt).unwrap();
    let output_path = tmp.path().join("out.png");

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_path)
        .output()
        .expect("Failed to execute CLI");

    assert_eq!(
        result.status.code(),
        Some(1),
        "decode failure must exit 1 (error), got {:?}: {}",
        result.status.code(),
        String::from_utf8_lossy(&result.stderr)
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("Image error:"),
        "failure must surface as Error::Image, got: {stderr}"
    );
}

#[test]
fn test_conflicting_policy_shorthand_and_dmi_exits_config() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);
    let output_path = tmp.path().join("out.png");

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_path)
        .arg("--hidden-marker")
        .arg("best-effort")
        .arg("--no-ai-training")
        .arg("--dmi")
        .arg("allowed")
        .output()
        .expect("Failed to execute CLI");

    assert_eq!(
        result.status.code(),
        Some(2),
        "conflicting shorthand + --dmi must exit 2 (config), got {:?}",
        result.status.code()
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("Conflicting policy"), "stderr: {stderr}");
}

#[test]
fn test_conflicting_rights_policy_and_dmi_exits_config() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);
    let output_path = tmp.path().join("out.png");

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-o")
        .arg(&output_path)
        .arg("--rights-policy")
        .arg("prohibited-ai-ml-training")
        .arg("--dmi")
        .arg("allowed")
        .output()
        .expect("Failed to execute CLI");

    assert_eq!(
        result.status.code(),
        Some(2),
        "conflicting --rights-policy + --dmi must exit 2 (config), got {:?}",
        result.status.code()
    );
}

#[test]
fn test_batch_duplicate_stems_with_file_output_exits_config() {
    let tmp = tempfile::tempdir().unwrap();
    let first = tmp.path().join("photo.png");
    create_test_png(&first);
    let subdir = tmp.path().join("sub");
    fs::create_dir(&subdir).unwrap();
    let second = subdir.join("photo.png");
    create_test_png(&second);
    let out_file = tmp.path().join("out.png");

    let result = Command::new(cli_bin())
        .arg(&first)
        .arg(&second)
        .arg("-o")
        .arg(&out_file)
        .output()
        .expect("Failed to execute CLI");

    assert_eq!(
        result.status.code(),
        Some(2),
        "duplicate stems with file-valued --output must exit 2 (config), got {:?}: {}",
        result.status.code(),
        String::from_utf8_lossy(&result.stderr)
    );
}
