#![allow(deprecated)]

use image::GenericImageView;
use stegoeggo::{
    process_image_bytes, DmiValue, ImageOutputFormat, LegalMetadata, MetadataTrapProtector,
    ProtectionContext, ProtectionLevel,
};

fn make_test_image_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn png_with_text_chunk(key: &str, value: &str) -> Vec<u8> {
    let base = make_test_image_png(64, 64);
    let mut output = Vec::with_capacity(base.len() + 100);
    output.extend_from_slice(&base[0..8]);
    let mut i = 8;
    while i + 8 <= base.len() {
        let length = u32::from_be_bytes([base[i], base[i + 1], base[i + 2], base[i + 3]]) as usize;
        let chunk_type = &base[i + 4..i + 8];
        if chunk_type == b"IEND" {
            output.extend_from_slice(&base[i..i + 8 + length + 4]);
            let chunk_data = format!("{}\0{}", key, value);
            let chunk_bytes = chunk_data.as_bytes();
            let chunk_len = (chunk_bytes.len() as u32).to_be_bytes();
            output.extend_from_slice(&chunk_len);
            output.extend_from_slice(b"tEXt");
            output.extend_from_slice(chunk_bytes);
            let mut crc = crc32fast::Hasher::new();
            crc.update(b"tEXt");
            crc.update(chunk_bytes);
            output.extend_from_slice(&crc.finalize().to_be_bytes());
        } else {
            output.extend_from_slice(&base[i..i + 8 + length + 4]);
        }
        i += 8 + length + 4;
    }
    output
}

fn has_text_chunk(png_bytes: &[u8], key: &str) -> bool {
    let mut i = 8;
    while i + 8 <= png_bytes.len() {
        let length = u32::from_be_bytes([
            png_bytes[i],
            png_bytes[i + 1],
            png_bytes[i + 2],
            png_bytes[i + 3],
        ]) as usize;
        let chunk_type = &png_bytes[i + 4..i + 8];
        if chunk_type == b"tEXt" && length > key.len() {
            let chunk_data = &png_bytes[i + 8..i + 8 + length];
            if let Some(null_pos) = chunk_data.iter().position(|&b| b == 0) {
                if &chunk_data[..null_pos] == key.as_bytes() {
                    return true;
                }
            }
        }
        i += 8 + length + 4;
    }
    false
}

fn get_text_value(png_bytes: &[u8], key: &str) -> Option<String> {
    let mut i = 8;
    while i + 8 <= png_bytes.len() {
        let length = u32::from_be_bytes([
            png_bytes[i],
            png_bytes[i + 1],
            png_bytes[i + 2],
            png_bytes[i + 3],
        ]) as usize;
        let chunk_type = &png_bytes[i + 4..i + 8];
        if chunk_type == b"tEXt" && length > key.len() {
            let chunk_data = &png_bytes[i + 8..i + 8 + length];
            if let Some(null_pos) = chunk_data.iter().position(|&b| b == 0) {
                if &chunk_data[..null_pos] == key.as_bytes() {
                    return Some(String::from_utf8_lossy(&chunk_data[null_pos + 1..]).into_owned());
                }
            }
        }
        i += 8 + length + 4;
    }
    None
}

fn legal() -> LegalMetadata {
    LegalMetadata::new()
        .with_copyright_holder("Preservation Holder")
        .with_usage_terms("Preservation Terms")
        .with_creator("Preservation Creator")
}

#[test]
fn png_unrelated_text_chunk_survives_byte_level_injection() {
    let base = png_with_text_chunk("Author", "Alice");
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = MetadataTrapProtector::new();
    let output = trap.inject_bytes(&base, &ctx).unwrap();

    assert!(
        has_text_chunk(&output, "Author"),
        "Unrelated 'Author' tEXt chunk should survive byte-level metadata injection"
    );
    assert_eq!(
        get_text_value(&output, "Author"),
        Some("Alice".to_string()),
        "Author value should be preserved"
    );
}

#[test]
fn png_unrelated_text_chunk_survives_double_injection() {
    let base = png_with_text_chunk("Author", "Alice");
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = MetadataTrapProtector::new();

    let output1 = trap.inject_bytes(&base, &ctx).unwrap();
    let output2 = trap.inject_bytes(&output1, &ctx).unwrap();

    assert!(
        has_text_chunk(&output2, "Author"),
        "Unrelated 'Author' tEXt chunk should survive double byte-level injection"
    );
    assert_eq!(
        get_text_value(&output2, "Author"),
        Some("Alice".to_string()),
        "Author value should be preserved after double injection"
    );
}

#[test]
fn png_pixel_content_decodes_after_processing() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();

    let img = image::load_from_memory(&output).expect("Output should decode as valid PNG");
    let (w, h) = img.dimensions();
    assert_eq!(w, 64);
    assert_eq!(h, 64);
}

#[test]
fn jpeg_pixel_content_decodes_after_processing() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();

    let img = image::load_from_memory(&output).expect("Output should decode as valid JPEG");
    let (w, h) = img.dimensions();
    assert_eq!(w, 64);
    assert_eq!(h, 64);
}

#[test]
fn webp_pixel_content_decodes_after_processing() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();

    let img = image::load_from_memory(&output).expect("Output should decode as valid WebP");
    let (w, h) = img.dimensions();
    assert_eq!(w, 64);
    assert_eq!(h, 64);
}

#[test]
fn png_idempotent_metadata_count() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let output1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let output2 = process_image_bytes(&output1, ProtectionLevel::Standard, &ctx).unwrap();

    let notice1 = stegoeggo::verify_legal_notice(&output1, b"");
    let notice2 = stegoeggo::verify_legal_notice(&output2, b"");

    assert_eq!(notice1.copyright_holder(), notice2.copyright_holder());
    assert_eq!(notice1.usage_terms(), notice2.usage_terms());
    assert_eq!(notice1.dmi(), notice2.dmi());
}

#[test]
fn jpeg_idempotent_metadata() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let output1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let output2 = process_image_bytes(&output1, ProtectionLevel::Standard, &ctx).unwrap();

    let notice1 = stegoeggo::verify_legal_notice(&output1, b"");
    let notice2 = stegoeggo::verify_legal_notice(&output2, b"");

    assert_eq!(notice1.copyright_holder(), notice2.copyright_holder());
    assert_eq!(notice1.usage_terms(), notice2.usage_terms());
}

#[test]
fn webp_idempotent_metadata() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let output1 = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();
    let output2 = process_image_bytes(&output1, ProtectionLevel::Standard, &ctx).unwrap();

    let notice1 = stegoeggo::verify_legal_notice(&output1, b"");
    let notice2 = stegoeggo::verify_legal_notice(&output2, b"");

    assert_eq!(notice1.copyright_holder(), notice2.copyright_holder());
    assert_eq!(notice1.usage_terms(), notice2.usage_terms());
}

#[test]
fn unrelated_xmp_namespace_preserved_in_png() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let output = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();

    let notice = stegoeggo::verify_legal_notice(&output, b"");
    assert!(notice.has_notice());
    assert_eq!(
        notice.copyright_holder(),
        Some("Preservation Holder"),
        "Legal metadata should be present"
    );
}

#[test]
fn existing_creator_preserved_in_png() {
    let base = png_with_text_chunk("Creator", "Original Creator");
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(LegalMetadata::new().with_copyright_holder("New Holder"))
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = MetadataTrapProtector::new();
    let output = trap.inject_bytes(&base, &ctx).unwrap();

    assert!(
        has_text_chunk(&output, "Creator"),
        "Original Creator chunk should survive byte-level injection"
    );
}

#[test]
fn image_dimensions_preserved_png() {
    let img = image::DynamicImage::new_rgb8(128, 64);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    let base = buf.into_inner();

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();

    let img = image::load_from_memory(&output).unwrap();
    let (w, h) = img.dimensions();
    assert_eq!(w, 128);
    assert_eq!(h, 64);
}

#[test]
fn image_dimensions_preserved_jpeg() {
    let img = image::DynamicImage::new_rgb8(128, 64);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    let base = buf.into_inner();

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes(&base, ProtectionLevel::Standard, &ctx).unwrap();

    let img = image::load_from_memory(&output).unwrap();
    let (w, h) = img.dimensions();
    assert_eq!(w, 128);
    assert_eq!(h, 64);
}

#[test]
fn reapplication_idempotence_on_existing_metadata() {
    let base = make_test_image_png(64, 64);
    let legal_first = LegalMetadata::new()
        .with_copyright_holder("First Holder")
        .with_usage_terms("First Terms")
        .with_creator("First Creator");
    let ctx_first = ProtectionContext::new(0.5, 80)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_first)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let first = process_image_bytes(&base, ProtectionLevel::Standard, &ctx_first).unwrap();

    let legal_same = LegalMetadata::new()
        .with_copyright_holder("First Holder")
        .with_usage_terms("First Terms")
        .with_creator("First Creator");
    let ctx_same = ProtectionContext::new(0.5, 80)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_same)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let second = process_image_bytes(&first, ProtectionLevel::Standard, &ctx_same).unwrap();

    let notice1 = stegoeggo::verify_legal_notice(&first, b"");
    let notice2 = stegoeggo::verify_legal_notice(&second, b"");

    assert_eq!(notice1.copyright_holder(), notice2.copyright_holder());
    assert_eq!(notice1.usage_terms(), notice2.usage_terms());
    assert_eq!(notice1.creator(), notice2.creator());
    assert_eq!(notice1.dmi(), notice2.dmi());
    assert_eq!(notice1.canonical_dmi(), notice2.canonical_dmi());
}

#[test]
fn reapplication_with_different_notice() {
    let base = make_test_image_png(64, 64);
    let legal_first = LegalMetadata::new()
        .with_copyright_holder("Old Holder")
        .with_usage_terms("Old Terms");
    let ctx_first = ProtectionContext::new(0.5, 81)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_first)
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let first = process_image_bytes(&base, ProtectionLevel::Standard, &ctx_first).unwrap();

    let legal_new = LegalMetadata::new()
        .with_copyright_holder("New Holder")
        .with_usage_terms("New Terms");
    let ctx_new = ProtectionContext::new(0.5, 82)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_new)
        .with_dmi(DmiValue::Allowed);
    let second = process_image_bytes(&first, ProtectionLevel::Standard, &ctx_new).unwrap();

    let notice = stegoeggo::verify_legal_notice(&second, b"");
    assert_eq!(notice.copyright_holder(), Some("New Holder"));
    assert_eq!(notice.usage_terms(), Some("New Terms"));
    assert_eq!(notice.dmi(), Some(DmiValue::Allowed));
}

fn png_with_iccp_chunk(profile_data: &[u8]) -> Vec<u8> {
    let base = make_test_image_png(64, 64);
    let keyword = b"ICC_Profile\0";
    let compression_method = 0u8;
    let mut chunk_data = Vec::with_capacity(keyword.len() + 1 + profile_data.len());
    chunk_data.extend_from_slice(keyword);
    chunk_data.push(compression_method);
    chunk_data.extend_from_slice(profile_data);

    let mut out = Vec::with_capacity(base.len() + chunk_data.len() + 12);
    out.extend_from_slice(&base[..8]);
    let mut i = 8;
    while i + 8 <= base.len() {
        let length = u32::from_be_bytes([base[i], base[i + 1], base[i + 2], base[i + 3]]) as usize;
        let chunk_type = &base[i + 4..i + 8];
        if chunk_type == b"IEND" {
            let chunk_len = (chunk_data.len() as u32).to_be_bytes();
            out.extend_from_slice(&chunk_len);
            out.extend_from_slice(b"iCCP");
            out.extend_from_slice(&chunk_data);
            let mut crc = crc32fast::Hasher::new();
            crc.update(b"iCCP");
            crc.update(&chunk_data);
            out.extend_from_slice(&crc.finalize().to_be_bytes());
            out.extend_from_slice(&base[i..i + 8 + length + 4]);
            break;
        } else {
            out.extend_from_slice(&base[i..i + 8 + length + 4]);
        }
        i += 8 + length + 4;
    }
    out
}

fn has_chunk(png: &[u8], chunk_type: &[u8; 4]) -> bool {
    let mut i = 8;
    while i + 8 <= png.len() {
        let length = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]) as usize;
        let ct = &png[i + 4..i + 8];
        if ct == chunk_type {
            return true;
        }
        i += 12 + length;
    }
    false
}

#[test]
fn png_iccp_chunk_survives_byte_level_injection() {
    let fake_profile = b"profile data for testing preservation";
    let base = png_with_iccp_chunk(fake_profile);
    assert!(has_chunk(&base, b"iCCP"), "Base should contain iCCP chunk");

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = MetadataTrapProtector::new();
    let output = trap.inject_bytes(&base, &ctx).unwrap();

    assert!(
        has_chunk(&output, b"iCCP"),
        "iCCP chunk should survive byte-level metadata injection"
    );
    assert!(
        image::load_from_memory(&output).is_ok(),
        "Output with preserved iCCP should decode"
    );
}

fn jpeg_with_exif_app1(orientation: u16) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
    let base = buf.into_inner();

    let mut exif_data = Vec::new();
    exif_data.extend_from_slice(b"Exif\0\0");
    exif_data.extend_from_slice(b"II");
    exif_data.extend_from_slice(&42u16.to_le_bytes());
    exif_data.extend_from_slice(&8u32.to_le_bytes());
    exif_data.extend_from_slice(&10u16.to_le_bytes());
    exif_data.extend_from_slice(&0x0112u16.to_le_bytes());
    exif_data.extend_from_slice(&4u32.to_le_bytes());
    exif_data.extend_from_slice(&orientation.to_le_bytes());
    exif_data.extend_from_slice(&0u32.to_le_bytes());
    exif_data.extend_from_slice(&0u32.to_le_bytes());

    let payload_len = exif_data.len();
    let marker_len = (payload_len + 2) as u16;
    let mut out = Vec::with_capacity(base.len() + 4 + payload_len);
    out.extend_from_slice(&base[..2]);
    out.extend_from_slice(&[0xFF, 0xE1]);
    out.extend_from_slice(&marker_len.to_be_bytes());
    out.extend_from_slice(&exif_data);
    out.extend_from_slice(&base[2..]);
    out
}

fn jpeg_has_app1_marker(jpeg: &[u8], prefix: &[u8]) -> bool {
    let mut pos = 2;
    while pos + 4 <= jpeg.len() {
        if jpeg[pos] != 0xFF {
            break;
        }
        let marker = jpeg[pos + 1];
        if marker == 0xD8 || marker == 0xD9 {
            pos += 2;
            continue;
        }
        let length = u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]) as usize;
        if marker == 0xE1
            && pos + 4 + prefix.len() <= jpeg.len()
            && &jpeg[pos + 4..pos + 4 + prefix.len()] == prefix
        {
            return true;
        }
        pos += 2 + length;
    }
    false
}

#[test]
fn jpeg_exif_orientation_survives_byte_level_injection() {
    let base = jpeg_with_exif_app1(6);
    assert!(
        jpeg_has_app1_marker(&base, b"Exif\0\0"),
        "Base JPEG should contain Exif APP1 marker"
    );

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = MetadataTrapProtector::new();
    let output = trap.inject_bytes(&base, &ctx).unwrap();

    assert!(
        jpeg_has_app1_marker(&output, b"Exif\0\0"),
        "Exif APP1 marker should survive byte-level metadata injection"
    );
    assert!(
        image::load_from_memory(&output).is_ok(),
        "Output with preserved EXIF should decode"
    );
}

fn jpeg_with_iptc_app13() -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
    let base = buf.into_inner();

    let mut iptc_data = Vec::new();
    iptc_data.extend_from_slice(b"Photoshop 3.0\0");
    iptc_data.extend_from_slice(b"8BIM");
    iptc_data.extend_from_slice(&4u16.to_be_bytes());
    iptc_data.push(0);
    let record_data = b"IPTC test data";
    iptc_data.extend_from_slice(&(record_data.len() as u32).to_be_bytes());
    iptc_data.extend_from_slice(record_data);

    let payload_len = iptc_data.len();
    let marker_len = (payload_len + 2) as u16;
    let mut out = Vec::with_capacity(base.len() + 4 + payload_len);
    out.extend_from_slice(&base[..2]);
    out.extend_from_slice(&[0xFF, 0xED]);
    out.extend_from_slice(&marker_len.to_be_bytes());
    out.extend_from_slice(&iptc_data);
    out.extend_from_slice(&base[2..]);
    out
}

fn jpeg_has_app13_marker(jpeg: &[u8]) -> bool {
    let mut pos = 2;
    while pos + 4 <= jpeg.len() {
        if jpeg[pos] != 0xFF {
            break;
        }
        let marker = jpeg[pos + 1];
        if marker == 0xD8 || marker == 0xD9 {
            pos += 2;
            continue;
        }
        let length = u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]) as usize;
        if marker == 0xED {
            return true;
        }
        pos += 2 + length;
    }
    false
}

#[test]
fn jpeg_iptc_app13_survives_byte_level_injection() {
    let base = jpeg_with_iptc_app13();
    assert!(
        jpeg_has_app13_marker(&base),
        "Base JPEG should contain IPTC APP13 marker"
    );

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = MetadataTrapProtector::new();
    let output = trap.inject_bytes(&base, &ctx).unwrap();

    assert!(
        jpeg_has_app13_marker(&output),
        "IPTC APP13 marker should survive byte-level metadata injection"
    );
    assert!(
        image::load_from_memory(&output).is_ok(),
        "Output with preserved IPTC should decode"
    );
}
