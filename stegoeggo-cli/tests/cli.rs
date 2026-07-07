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
    assert!(stdout.contains("Image protection CLI"));
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
fn test_protect_jpeg_without_format_defaults_to_png_extension() {
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

    let output_file = output_dir.join("input_protected.png");
    assert!(output_file.exists());

    let output_bytes = fs::read(&output_file).unwrap();
    assert!(
        output_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "Default output should be PNG"
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
        stdout.contains("Protected: Yes"),
        "Should report protected: {}",
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
        stdout.contains("Protected: No"),
        "Should report not protected: {}",
        stdout
    );
}

#[test]
fn test_verify_metadata_only_does_not_report_verified() {
    use stegoeggo::{MetadataTrapProtector, ProtectionContext};

    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.png");
    create_test_png(&input);
    let raw = fs::read(&input).unwrap();

    let ctx = ProtectionContext::new(0.5, 42);
    let metadata_only = MetadataTrapProtector::new()
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
        !stdout.contains("verified"),
        "Metadata-only file must not be reported as verified: {}",
        stdout
    );
    assert!(
        stdout.contains("metadata-only"),
        "Metadata-only evidence should be reported explicitly: {}",
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

    create_test_png(&input);

    let result = Command::new(cli_bin())
        .arg(&input)
        .arg("-s")
        .arg("42")
        .output()
        .expect("Failed to execute CLI");

    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("protected"),
        "Should output protected filename: {}",
        stdout
    );
}
