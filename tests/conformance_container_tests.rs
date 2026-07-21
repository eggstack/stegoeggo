#![allow(deprecated)]

use image::GenericImageView;
use stegoeggo::{
    process_image_bytes_with_warnings, DmiValue, ImageOutputFormat, LegalMetadata,
    MetadataTrapProtector, ProtectionContext, ProtectionLevel,
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
    let trap = MetadataTrapProtector::new();
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
    let trap = MetadataTrapProtector::new();
    let output = trap.inject_bytes(&out, &ctx).unwrap();

    let notice = stegoeggo::verify_legal_notice(&output, b"");
    assert!(
        notice.has_notice(),
        "Legal notice should be present after byte-level injection"
    );
}
