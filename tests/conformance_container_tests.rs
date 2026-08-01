#![allow(deprecated)]

use image::GenericImageView;
use stegoeggo::{
    process_image_bytes_with_warnings, DmiValue, ImageOutputFormat, LegalMetadata,
    ProtectionContext, ProtectionLevel, RightsMetadataProtector,
};

fn make_test_image_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn legal() -> LegalMetadata {
    LegalMetadata::new()
        .with_copyright_holder("Container Test Holder")
        .with_usage_terms("Container Test Terms")
        .with_creator("Container Creator")
}

fn png_with_text_chunk(key: &str, value: &str) -> Vec<u8> {
    let base = make_test_image_png(64, 64);
    let mut out = Vec::with_capacity(base.len() + 100);
    out.extend_from_slice(&base[0..8]);
    let mut i = 8;
    while i + 8 <= base.len() {
        let length = u32::from_be_bytes([base[i], base[i + 1], base[i + 2], base[i + 3]]) as usize;
        let chunk_type = &base[i + 4..i + 8];
        if chunk_type == b"IEND" {
            out.extend_from_slice(&base[i..i + 8 + length + 4]);
            let chunk_data = format!("{}\0{}", key, value);
            let chunk_bytes = chunk_data.as_bytes();
            let chunk_len = (chunk_bytes.len() as u32).to_be_bytes();
            out.extend_from_slice(&chunk_len);
            out.extend_from_slice(b"tEXt");
            out.extend_from_slice(chunk_bytes);
            let mut crc = crc32fast::Hasher::new();
            crc.update(b"tEXt");
            crc.update(chunk_bytes);
            out.extend_from_slice(&crc.finalize().to_be_bytes());
        } else {
            out.extend_from_slice(&base[i..i + 8 + length + 4]);
        }
        i += 8 + length + 4;
    }
    out
}

fn count_png_chunks(png: &[u8], chunk_type: &[u8; 4]) -> usize {
    let mut count = 0;
    let mut i = 8;
    while i + 8 <= png.len() {
        let length = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]) as usize;
        let ct = &png[i + 4..i + 8];
        if ct == chunk_type {
            count += 1;
        }
        i += 12 + length;
    }
    count
}

#[test]
fn png_chunk_integrity_after_standard_processing() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes_with_warnings(&base, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    assert!(
        output.starts_with(b"\x89PNG"),
        "Output should start with PNG signature"
    );
    let ihdr_count = count_png_chunks(&output, b"IHDR");
    assert_eq!(ihdr_count, 1, "Should have exactly one IHDR chunk");
    let iend_count = count_png_chunks(&output, b"IEND");
    assert_eq!(iend_count, 1, "Should have exactly one IEND chunk");

    let img = image::load_from_memory(&output).expect("Output should decode as valid PNG");
    let (w, h) = img.dimensions();
    assert_eq!(w, 64);
    assert_eq!(h, 64);
}

#[test]
fn png_independent_decode_after_processing() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes_with_warnings(&base, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    let img = image::load_from_memory(&output).expect("Independent decode should succeed");
    assert_eq!(img.dimensions(), (64, 64));
}

#[test]
fn jpeg_independent_decode_after_processing() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes_with_warnings(&base, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    let img = image::load_from_memory(&output).expect("JPEG decode should succeed");
    assert_eq!(img.dimensions(), (64, 64));
}

#[test]
fn webp_independent_decode_after_processing() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes_with_warnings(&base, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    let img = image::load_from_memory(&output).expect("WebP decode should succeed");
    assert_eq!(img.dimensions(), (64, 64));
}

#[test]
fn png_text_chunk_count_idempotent() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let output1 = process_image_bytes_with_warnings(&base, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;
    let output2 = process_image_bytes_with_warnings(&output1, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    let text_count1 = count_png_chunks(&output1, b"tEXt");
    let itxt_count1 = count_png_chunks(&output1, b"iTXt");
    let text_count2 = count_png_chunks(&output2, b"tEXt");
    let itxt_count2 = count_png_chunks(&output2, b"iTXt");

    assert_eq!(
        text_count1 + itxt_count1,
        text_count2 + itxt_count2,
        "Text chunk count should be idempotent across reprocessing"
    );
}

#[test]
fn png_existing_text_chunk_preserved_through_byte_level() {
    let base = png_with_text_chunk("Author", "Alice");
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = RightsMetadataProtector::new();
    let output = trap.inject_bytes(&base, &ctx).unwrap();

    let mut found_author = false;
    let mut i = 8;
    while i + 8 <= output.len() {
        let length =
            u32::from_be_bytes([output[i], output[i + 1], output[i + 2], output[i + 3]]) as usize;
        let chunk_type = &output[i + 4..i + 8];
        if chunk_type == b"tEXt" && length > 6 {
            let data = &output[i + 8..i + 8 + length];
            if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                if &data[..null_pos] == b"Author" {
                    found_author = true;
                }
            }
        }
        i += 12 + length;
    }
    assert!(
        found_author,
        "Unrelated 'Author' tEXt chunk should survive byte-level injection"
    );
}

#[test]
fn jpeg_markers_preserve_structure() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes_with_warnings(&base, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    assert_eq!(output[0], 0xFF, "JPEG should start with SOI marker");
    assert_eq!(output[1], 0xD8, "JPEG should start with SOI marker");

    let mut found_com = false;
    let mut pos = 2;
    while pos + 4 <= output.len() {
        if output[pos] != 0xFF {
            break;
        }
        let marker = output[pos + 1];
        if marker == 0xD8 || marker == 0xD9 {
            pos += 2;
            continue;
        }
        let length = u16::from_be_bytes([output[pos + 2], output[pos + 3]]) as usize;
        if marker == 0xFE {
            found_com = true;
        }
        pos += 2 + length;
    }
    assert!(
        found_com,
        "JPEG should contain COM markers with legal metadata"
    );
}

#[test]
fn webp_container_valid_after_processing() {
    let base = make_test_image_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes_with_warnings(&base, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    assert!(output.starts_with(b"RIFF"), "WebP should start with RIFF");
    assert!(output.len() >= 12, "WebP should have RIFF+WEBP header");
    assert_eq!(&output[8..12], b"WEBP", "WebP tag should be at offset 8");

    let mut pos = 12;
    let mut found_vpx = false;
    let mut found_xmp = false;
    let mut found_image = false;
    while pos + 8 <= output.len() {
        let fourcc = &output[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            output[pos + 4],
            output[pos + 5],
            output[pos + 6],
            output[pos + 7],
        ]) as usize;
        if fourcc == b"VP8X" {
            found_vpx = true;
            assert_eq!(chunk_size, 10, "VP8X data should be 10 bytes");
        }
        if fourcc == b"XMP " {
            found_xmp = true;
        }
        if fourcc == b"VP8 " || fourcc == b"VP8L" {
            found_image = true;
        }
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }

    assert!(found_vpx, "Output must contain VP8X chunk");
    assert!(found_xmp, "Output must contain XMP chunk");
    assert!(found_image, "Output must contain image payload chunk");

    let img = image::load_from_memory(&output).expect("WebP should decode");
    assert_eq!(img.dimensions(), (64, 64));
}

#[test]
fn png_unrelated_xmp_survives_byte_level() {
    let base = make_test_image_png(64, 64);
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      dc:creator="Unrelated Creator"/>
  </rdf:RDF>
</x:xmpmeta>"#;
    let key = "XML:com.adobe.xmp";
    let chunk_data = format!("{}\0{}", key, xmp);
    let chunk_bytes = chunk_data.as_bytes();
    let chunk_len = (chunk_bytes.len() as u32).to_be_bytes();

    let mut out = Vec::with_capacity(base.len() + chunk_bytes.len() + 100);
    out.extend_from_slice(&base[..8]);
    let mut i = 8;
    while i + 8 <= base.len() {
        let length = u32::from_be_bytes([base[i], base[i + 1], base[i + 2], base[i + 3]]) as usize;
        let chunk_type = &base[i + 4..i + 8];
        if chunk_type == b"IEND" {
            out.extend_from_slice(&chunk_len);
            out.extend_from_slice(b"iTXt");
            out.extend_from_slice(chunk_bytes);
            let mut crc = crc32fast::Hasher::new();
            crc.update(b"iTXt");
            crc.update(chunk_bytes);
            out.extend_from_slice(&crc.finalize().to_be_bytes());
            out.extend_from_slice(&base[i..i + 8 + length + 4]);
            break;
        } else {
            out.extend_from_slice(&base[i..i + 8 + length + 4]);
        }
        i += 8 + length + 4;
    }

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = RightsMetadataProtector::new();
    let output = trap.inject_bytes(&out, &ctx).unwrap();

    let notice = stegoeggo::verify_legal_notice(&output, b"");
    assert!(
        notice.has_notice(),
        "Legal notice should be present after byte-level injection"
    );
}

fn count_webp_chunks(bytes: &[u8], target: &[u8; 4]) -> usize {
    let mut count = 0;
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let fourcc = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        if fourcc == target {
            count += 1;
        }
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }
    count
}

fn webp_vp8x_flags(bytes: &[u8]) -> Option<u8> {
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let fourcc = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        if fourcc == b"VP8X" && chunk_size >= 1 {
            return Some(bytes[pos + 8]);
        }
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }
    None
}

#[test]
fn webp_simple_to_extended_has_vp8x() {
    let img = make_test_image_png(32, 32);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes_with_warnings(&img, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    assert_eq!(count_webp_chunks(&output, b"VP8X"), 1, "Exactly one VP8X");
    assert!(
        count_webp_chunks(&output, b"VP8L") + count_webp_chunks(&output, b"VP8 ") >= 1,
        "Must have image payload"
    );

    let img = image::load_from_memory(&output).expect("Simple→extended WebP should decode");
    assert_eq!(img.dimensions(), (32, 32));
}

#[test]
fn webp_xmp_not_duplicated() {
    let img = make_test_image_png(32, 32);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes_with_warnings(&img, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    assert_eq!(
        count_webp_chunks(&output, b"XMP "),
        1,
        "Exactly one XMP chunk"
    );
    assert_eq!(
        count_webp_chunks(&output, b"EXIF"),
        0,
        "No EXIF seed chunks emitted"
    );
}

#[test]
fn webp_idempotent_metadata_count() {
    let img = make_test_image_png(32, 32);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let out1 = process_image_bytes_with_warnings(&img, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;
    let out2 = process_image_bytes_with_warnings(&out1, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    assert_eq!(
        count_webp_chunks(&out1, b"XMP "),
        count_webp_chunks(&out2, b"XMP "),
        "XMP chunk count should be idempotent"
    );
    assert_eq!(
        count_webp_chunks(&out1, b"VP8X"),
        count_webp_chunks(&out2, b"VP8X"),
        "VP8X chunk count should be idempotent"
    );

    let img = image::load_from_memory(&out2).expect("Double-processed WebP should decode");
    assert_eq!(img.dimensions(), (32, 32));
}

#[test]
fn webp_vpx_flags_reflect_xmp() {
    let img = make_test_image_png(32, 32);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes_with_warnings(&img, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    let flags = webp_vp8x_flags(&output).expect("VP8X must exist");
    assert_ne!(
        flags & 0x04,
        0,
        "XMP flag must be set when XMP chunk present"
    );
    assert_eq!(
        flags & 0x08,
        0,
        "EXIF flag must not be set when no EXIF chunk"
    );
}

#[test]
fn webp_vp8l_payload_preserved_through_metadata_injection() {
    let img_bytes = make_test_image_png(32, 32);
    let img = image::load_from_memory(&img_bytes).unwrap();
    let webp = {
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::WebP).unwrap();
        buf.into_inner()
    };

    let original_vp8l = extract_webp_vp8l_chunk(&webp);

    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = RightsMetadataProtector::new();
    let output = trap.inject_bytes(&webp, &ctx).unwrap();

    let output_vp8l = extract_webp_vp8l_chunk(&output);
    assert_eq!(
        original_vp8l, output_vp8l,
        "VP8L payload must be byte-identical after metadata injection"
    );
}

fn extract_webp_vp8l_chunk(bytes: &[u8]) -> Vec<u8> {
    let mut chunk_data = Vec::new();
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let chunk_size =
            u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        let chunk_type = &bytes[i..i + 4];
        if *chunk_type == *b"VP8L" || *chunk_type == *b"VP8 " {
            chunk_data.extend_from_slice(&bytes[i..i + 8 + chunk_size]);
        }
        i += 8 + chunk_size;
        if chunk_size % 2 == 1 {
            i += 1;
        }
    }
    chunk_data
}
