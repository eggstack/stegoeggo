use std::process::Command;
use stegoeggo::{
    process_image_bytes, DmiValue, ImageOutputFormat, LegalMetadata, ProtectionContext,
    ProtectionLevel,
};

fn tool_available(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn make_test_image_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn full_legal_metadata() -> LegalMetadata {
    LegalMetadata::new()
        .with_copyright_holder("Test Holder")
        .with_creator("Test Creator")
        .with_copyright_owner("Test Owner")
        .with_contact_email("contact@example.com")
        .with_license_url("https://example.com/license")
        .with_usage_terms("All rights reserved")
        .with_web_statement_of_rights("https://example.com/rights")
        .with_creation_date("2025-01-15")
        .with_ai_constraints("No AI training")
        .with_credit_line("Photo by Test Creator")
        .with_licensor_name("Test Licensor")
        .with_licensor_email("licensor@example.com")
        .with_licensor_url("https://licensor.example.com")
}

fn process_and_write(format: ImageOutputFormat, output_path: &std::path::Path) {
    let png_bytes = make_test_image_png(64, 64);
    let legal = full_legal_metadata();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(format)
        .with_legal_metadata(legal)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
    std::fs::write(output_path, &output).unwrap();
}

fn exiftool_extract(file: &std::path::Path, tag: &str) -> Option<String> {
    let output = Command::new("exiftool")
        .arg("-s3")
        .arg(tag)
        .arg(file)
        .output()
        .ok()?;
    if output.status.success() {
        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if val.is_empty() {
            None
        } else {
            Some(val)
        }
    } else {
        None
    }
}

fn exiftool_extract_all(file: &std::path::Path) -> String {
    let output = Command::new("exiftool")
        .arg("-G")
        .arg("-a")
        .arg(file)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn xmllint_validate(file: &std::path::Path) -> bool {
    Command::new("xmllint")
        .arg("--noout")
        .arg(file)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn imagemagick_identify(file: &std::path::Path) -> bool {
    if Command::new("magick")
        .arg("identify")
        .arg(file)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return true;
    }
    Command::new("identify")
        .arg(file)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn extract_xmp_from_webp(bytes: &[u8]) -> Option<String> {
    let marker = b"XMP ";
    let mut pos = 0;
    while pos + 8 <= bytes.len() {
        if &bytes[pos..pos + 4] == marker {
            let size = u32::from_le_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]) as usize;
            if pos + 8 + size <= bytes.len() {
                let xmp_data = &bytes[pos + 8..pos + 8 + size];
                let needle = b"<?xpacket begin=";
                if let Some(start) = xmp_data.windows(needle.len()).position(|w| w == needle) {
                    return String::from_utf8(xmp_data[start..].to_vec()).ok();
                }
            }
        }
        pos += 1;
    }
    None
}

fn extract_xmp_to_tempfile(xmp: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".xmp").tempfile().unwrap();
    std::io::Write::write_all(&mut f, xmp.as_bytes()).unwrap();
    f
}

fn tmpdir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

mod exiftool_png {
    use super::*;

    #[test]
    fn extracts_copyright() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.png");
        process_and_write(ImageOutputFormat::Png, &path);

        let val = exiftool_extract(&path, "-Copyright");
        assert!(val.is_some(), "ExifTool should find Copyright in PNG");
        assert!(val.unwrap().contains("Test Holder"));
    }

    #[test]
    fn extracts_creator() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.png");
        process_and_write(ImageOutputFormat::Png, &path);

        let val = exiftool_extract(&path, "-Creator");
        assert!(val.is_some(), "ExifTool should find Creator in PNG");
        assert_eq!(val.unwrap(), "Test Creator");
    }

    #[test]
    fn extracts_dmi_from_xmp() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.png");
        process_and_write(ImageOutputFormat::Png, &path);

        let all = exiftool_extract_all(&path);
        assert!(
            all.contains("Data Mining"),
            "ExifTool should find Data Mining in PNG XMP"
        );
        assert!(
            all.contains("Prohibited for AI/ML training"),
            "DMI value should be present"
        );
    }
}

mod exiftool_jpeg {
    use super::*;

    #[test]
    fn extracts_copyright_in_comment() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.jpg");
        process_and_write(ImageOutputFormat::Jpeg, &path);

        let all = exiftool_extract_all(&path);
        assert!(
            all.contains("Copyright (c) Test Holder"),
            "ExifTool should find Copyright in JPEG COM markers. Output:\n{}",
            all
        );
    }

    #[test]
    fn extracts_creator_in_comment() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.jpg");
        process_and_write(ImageOutputFormat::Jpeg, &path);

        let all = exiftool_extract_all(&path);
        assert!(
            all.contains("Creator: Test Creator"),
            "ExifTool should find Creator in JPEG COM markers"
        );
    }

    #[test]
    fn extracts_dmi_from_xmp() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.jpg");
        process_and_write(ImageOutputFormat::Jpeg, &path);

        let all = exiftool_extract_all(&path);
        assert!(
            all.contains("Data Mining"),
            "ExifTool should find Data Mining in JPEG XMP"
        );
    }
}

mod exiftool_webp {
    use super::*;

    #[test]
    fn extracts_rights() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.webp");
        process_and_write(ImageOutputFormat::WebP, &path);

        let all = exiftool_extract_all(&path);
        assert!(
            all.contains("Rights") && all.contains("Test Holder"),
            "ExifTool should find dc:rights in WebP XMP"
        );
    }

    #[test]
    fn extracts_usage_terms() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.webp");
        process_and_write(ImageOutputFormat::WebP, &path);

        let all = exiftool_extract_all(&path);
        assert!(
            all.contains("Usage Terms") && all.contains("All rights reserved"),
            "ExifTool should find xmpRights:UsageTerms in WebP XMP"
        );
    }

    #[test]
    fn extracts_web_statement() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.webp");
        process_and_write(ImageOutputFormat::WebP, &path);

        let all = exiftool_extract_all(&path);
        assert!(
            all.contains("Web Statement") && all.contains("https://example.com/rights"),
            "ExifTool should find xmpRights:WebStatement in WebP XMP"
        );
    }

    #[test]
    fn extracts_credit() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.webp");
        process_and_write(ImageOutputFormat::WebP, &path);

        let all = exiftool_extract_all(&path);
        assert!(
            all.contains("Credit") && all.contains("Photo by Test Creator"),
            "ExifTool should find photoshop:Credit in WebP XMP"
        );
    }

    #[test]
    fn extracts_creator() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.webp");
        process_and_write(ImageOutputFormat::WebP, &path);

        let all = exiftool_extract_all(&path);
        assert!(
            all.contains("Creator") && all.contains("Test Creator"),
            "ExifTool should find dc:creator in WebP XMP"
        );
    }

    #[test]
    fn extracts_dmi() {
        if !tool_available("exiftool") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.webp");
        process_and_write(ImageOutputFormat::WebP, &path);

        let all = exiftool_extract_all(&path);
        assert!(
            all.contains("Data Mining") && all.contains("Prohibited for AI/ML training"),
            "ExifTool should find plus:DataMining in WebP XMP"
        );
    }
}

mod xml_validation {
    use super::*;

    #[test]
    fn webp_xmp_is_valid_xml() {
        if !tool_available("xmllint") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.webp");
        process_and_write(ImageOutputFormat::WebP, &path);

        let bytes = std::fs::read(&path).unwrap();
        let xmp = extract_xmp_from_webp(&bytes);
        assert!(xmp.is_some(), "WebP should contain XMP data");

        let xmp_file = extract_xmp_to_tempfile(&xmp.unwrap());
        assert!(
            xmllint_validate(xmp_file.path()),
            "WebP XMP should be valid XML"
        );
    }

    #[test]
    fn webp_xmp_has_required_namespaces() {
        if !tool_available("xmllint") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.webp");
        process_and_write(ImageOutputFormat::WebP, &path);

        let bytes = std::fs::read(&path).unwrap();
        let xmp = extract_xmp_from_webp(&bytes).unwrap();

        for ns in &[
            "xmlns:xmpRights=",
            "xmlns:dc=",
            "xmlns:photoshop=",
            "xmlns:plus=",
            "xmlns:stegoeggo=",
        ] {
            assert!(xmp.contains(ns), "WebP XMP missing namespace {}", ns);
        }
    }

    #[test]
    fn webp_xmp_has_dmi_property() {
        if !tool_available("xmllint") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.webp");
        process_and_write(ImageOutputFormat::WebP, &path);

        let bytes = std::fs::read(&path).unwrap();
        let xmp = extract_xmp_from_webp(&bytes).unwrap();

        assert!(
            xmp.contains("plus:DataMining"),
            "XMP should contain plus:DataMining"
        );
        assert!(
            xmp.contains("DMI-PROHIBITED-AIMLTRAINING"),
            "XMP should contain the DMI enum value"
        );
    }
}

mod imagemagick_smoke {
    use super::*;

    #[test]
    fn png_identifies() {
        if !tool_available("identify") && !tool_available("magick") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.png");
        process_and_write(ImageOutputFormat::Png, &path);
        assert!(imagemagick_identify(&path));
    }

    #[test]
    fn jpeg_identifies() {
        if !tool_available("identify") && !tool_available("magick") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.jpg");
        process_and_write(ImageOutputFormat::Jpeg, &path);
        assert!(imagemagick_identify(&path));
    }

    #[test]
    fn webp_identifies() {
        if !tool_available("identify") && !tool_available("magick") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.webp");
        process_and_write(ImageOutputFormat::WebP, &path);
        assert!(imagemagick_identify(&path));
    }

    #[test]
    fn png_dimensions_preserved() {
        if !tool_available("identify") && !tool_available("magick") {
            return;
        }
        let dir = tmpdir();
        let path = dir.path().join("test.png");
        process_and_write(ImageOutputFormat::Png, &path);

        let output = if tool_available("magick") {
            Command::new("magick")
                .arg("identify")
                .arg("-format")
                .arg("%wx%h")
                .arg(&path)
                .output()
                .unwrap()
        } else {
            Command::new("identify")
                .arg("-format")
                .arg("%wx%h")
                .arg(&path)
                .output()
                .unwrap()
        };
        let dims = String::from_utf8_lossy(&output.stdout).to_string();
        assert_eq!(dims, "64x64");
    }
}
