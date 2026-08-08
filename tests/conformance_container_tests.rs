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
        if !chunk_size.is_multiple_of(2) {
            i += 1;
        }
    }
    chunk_data
}

fn make_simple_vp8_webp() -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(32, 32);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::WebP).unwrap();
    buf.into_inner()
}

fn encode_vp8x_chunk(width: u32, height: u32, flags: u8) -> Vec<u8> {
    let w = width - 1;
    let h = height - 1;
    let mut data = vec![0u8; 10];
    data[0] = flags;
    data[4] = w as u8;
    data[5] = (w >> 8) as u8;
    data[6] = (w >> 16) as u8;
    data[7] = h as u8;
    data[8] = (h >> 8) as u8;
    data[9] = (h >> 16) as u8;
    data
}

fn make_webp_with_extra_chunks(
    extra_chunks: &[(&[u8], &[u8])],
    width: u32,
    height: u32,
) -> Vec<u8> {
    let base = make_simple_vp8_webp();
    let mut data = Vec::new();
    data.extend_from_slice(&base[0..12]);
    let vp8x_flags: u8 = extra_chunks
        .iter()
        .map(|(fcc, _)| match *fcc {
            b"EXIF" => 0x08,
            b"ICCP" => 0x20,
            b"XMP " => 0x04,
            _ => 0,
        })
        .fold(0u8, |a, b| a | b);
    let vp8x_payload = encode_vp8x_chunk(width, height, vp8x_flags);
    data.extend_from_slice(b"VP8X");
    data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
    data.extend_from_slice(&vp8x_payload);
    for (fcc, payload) in extra_chunks {
        data.extend_from_slice(fcc);
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(payload);
        if !payload.len().is_multiple_of(2) {
            data.push(0);
        }
    }
    let mut pos = 12;
    while pos + 8 <= base.len() {
        let fourcc = &base[pos..pos + 4];
        let chunk_size =
            u32::from_le_bytes([base[pos + 4], base[pos + 5], base[pos + 6], base[pos + 7]])
                as usize;
        if fourcc != b"VP8X" {
            data.extend_from_slice(&base[pos..pos + 8 + chunk_size]);
            if !chunk_size.is_multiple_of(2) {
                data.push(0);
            }
        }
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }
    let riff_size = (data.len() - 8) as u32;
    data[4..8].copy_from_slice(&riff_size.to_le_bytes());
    data
}

fn make_webp_with_vp8x_and_exif() -> Vec<u8> {
    let exif_payload = vec![
        0x45, 0x78, 0x69, 0x66, 0x00, 0x00, 0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x54, 0x65, 0x73, 0x74, 0x45, 0x78, 0x69, 0x66, 0x00,
    ];
    make_webp_with_extra_chunks(&[(b"EXIF", &exif_payload)], 32, 32)
}

fn make_webp_with_unrelated_xmp() -> Vec<u8> {
    let xmp_data = br#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    dc:subject="unrelated-subject"
    xmp:Rating="5"/>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;
    make_webp_with_extra_chunks(&[(b"XMP ", xmp_data)], 32, 32)
}

fn make_webp_with_duplicate_identical_xmp() -> Vec<u8> {
    let xmp_data = br#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    dc:subject="test"/>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;
    let base = make_simple_vp8_webp();
    let mut data = Vec::new();
    data.extend_from_slice(&base[0..12]);
    let vp8x_flags: u8 = 0x04;
    let vp8x_payload = encode_vp8x_chunk(32, 32, vp8x_flags);
    data.extend_from_slice(b"VP8X");
    data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
    data.extend_from_slice(&vp8x_payload);
    for _ in 0..2 {
        data.extend_from_slice(b"XMP ");
        data.extend_from_slice(&(xmp_data.len() as u32).to_le_bytes());
        data.extend_from_slice(xmp_data);
        if !xmp_data.len().is_multiple_of(2) {
            data.push(0);
        }
    }
    let mut pos = 12;
    while pos + 8 <= base.len() {
        let fourcc = &base[pos..pos + 4];
        let chunk_size =
            u32::from_le_bytes([base[pos + 4], base[pos + 5], base[pos + 6], base[pos + 7]])
                as usize;
        if fourcc != b"VP8X" {
            data.extend_from_slice(&base[pos..pos + 8 + chunk_size]);
            if !chunk_size.is_multiple_of(2) {
                data.push(0);
            }
        }
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }
    let riff_size = (data.len() - 8) as u32;
    data[4..8].copy_from_slice(&riff_size.to_le_bytes());
    data
}

fn make_webp_with_iccp() -> Vec<u8> {
    let icc_payload = vec![
        0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    make_webp_with_extra_chunks(&[(b"ICCP", &icc_payload)], 32, 32)
}

fn make_webp_with_unknown_chunk() -> Vec<u8> {
    let unknown_payload = vec![0xAA, 0xBB, 0xCC, 0xDD];
    make_webp_with_extra_chunks(&[(b"TEST", &unknown_payload)], 32, 32)
}

fn get_webp_xmp_raw(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut xmps = Vec::new();
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let fourcc = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        if fourcc == b"XMP " {
            let data_start = pos + 8;
            let data_end = data_start + chunk_size;
            if data_end <= bytes.len() {
                xmps.push(bytes[data_start..data_end].to_vec());
            }
        }
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }
    xmps
}

// ── Phase 5: WebP XMP replacement and feature-flag consistency tests ─────

#[test]
fn webp_exif_chunk_preserved_and_flag_set() {
    let webp = make_webp_with_vp8x_and_exif();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = RightsMetadataProtector::new();
    let output = trap.inject_bytes(&webp, &ctx).unwrap();

    assert_eq!(count_webp_chunks(&output, b"VP8X"), 1, "Exactly one VP8X");
    assert_eq!(count_webp_chunks(&output, b"XMP "), 1, "Exactly one XMP");
    assert_eq!(
        count_webp_chunks(&output, b"EXIF"),
        1,
        "EXIF chunk preserved"
    );

    let flags = webp_vp8x_flags(&output).expect("VP8X must exist");
    assert_ne!(
        flags & 0x08,
        0,
        "EXIF bit must be set when EXIF chunk present"
    );
    assert_ne!(
        flags & 0x04,
        0,
        "XMP bit must be set when XMP chunk present"
    );

    let img = image::load_from_memory(&output).expect("WebP with EXIF should decode");
    assert_eq!(img.dimensions(), (32, 32));
}

#[test]
fn webp_unrelated_xmp_preserved_in_sole_packet() {
    let webp = make_webp_with_unrelated_xmp();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = RightsMetadataProtector::new();
    let output = match trap.inject_bytes(&webp, &ctx) {
        Ok(o) => o,
        Err(e) => {
            let xmps = get_webp_xmp_raw(&webp);
            panic!(
                "inject_bytes failed: {}. Existing XMP chunks: {}. Original XMP:\n{}",
                e,
                xmps.len(),
                String::from_utf8_lossy(&xmps[0])
            );
        }
    };

    assert_eq!(
        count_webp_chunks(&output, b"XMP "),
        1,
        "Exactly one XMP chunk"
    );
    let xmps = get_webp_xmp_raw(&output);
    assert_eq!(xmps.len(), 1);
    let xmp_str = String::from_utf8_lossy(&xmps[0]);
    assert!(
        xmp_str.contains("dc:subject"),
        "Unrelated dc:subject must survive in sole XMP packet. Got: {}",
        xmp_str
    );
    assert!(
        xmp_str.contains("xmp:Rating"),
        "Unrelated xmp:Rating must survive in sole XMP packet"
    );
    assert!(
        xmp_str.contains("plus:DataMining"),
        "StegoEggo rights metadata must be present"
    );

    let img = image::load_from_memory(&output).expect("WebP should decode");
    assert_eq!(img.dimensions(), (32, 32));
}

#[test]
fn webp_duplicate_identical_xmp_collapses_to_one() {
    let webp = make_webp_with_duplicate_identical_xmp();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = RightsMetadataProtector::new();
    let output = trap.inject_bytes(&webp, &ctx).unwrap();

    assert_eq!(
        count_webp_chunks(&output, b"XMP "),
        1,
        "Duplicate XMP chunks must collapse to exactly one"
    );
    let flags = webp_vp8x_flags(&output).expect("VP8X must exist");
    assert_ne!(flags & 0x04, 0, "XMP bit must be set");

    let img = image::load_from_memory(&output).expect("WebP should decode");
    assert_eq!(img.dimensions(), (32, 32));
}

#[test]
fn webp_iccp_preserved_and_flag_set() {
    let webp = make_webp_with_iccp();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = RightsMetadataProtector::new();
    let output = trap.inject_bytes(&webp, &ctx).unwrap();

    assert_eq!(
        count_webp_chunks(&output, b"ICCP"),
        1,
        "ICCP chunk preserved"
    );
    let flags = webp_vp8x_flags(&output).expect("VP8X must exist");
    assert_ne!(flags & 0x20, 0, "ICC bit must be set when ICCP present");
    assert_ne!(flags & 0x04, 0, "XMP bit must be set");
}

#[test]
fn webp_unknown_chunk_preserved() {
    let webp = make_webp_with_unknown_chunk();
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = RightsMetadataProtector::new();
    let output = trap.inject_bytes(&webp, &ctx).unwrap();

    let mut has_unknown = false;
    let mut pos = 12;
    while pos + 8 <= output.len() {
        let fourcc = &output[pos..pos + 4];
        if fourcc == b"TEST" {
            has_unknown = true;
            break;
        }
        let chunk_size = u32::from_le_bytes([
            output[pos + 4],
            output[pos + 5],
            output[pos + 6],
            output[pos + 7],
        ]) as usize;
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }
    assert!(has_unknown, "Unknown RIFF chunk must be preserved");

    let img = image::load_from_memory(&output).expect("WebP should decode");
    assert_eq!(img.dimensions(), (32, 32));
}

#[test]
fn webp_three_rounds_no_unbounded_growth() {
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
    let out3 = process_image_bytes_with_warnings(&out2, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    assert_eq!(
        count_webp_chunks(&out1, b"XMP "),
        1,
        "Round 1: exactly 1 XMP"
    );
    assert_eq!(
        count_webp_chunks(&out2, b"XMP "),
        1,
        "Round 2: exactly 1 XMP"
    );
    assert_eq!(
        count_webp_chunks(&out3, b"XMP "),
        1,
        "Round 3: exactly 1 XMP"
    );
    assert_eq!(
        count_webp_chunks(&out1, b"VP8X"),
        1,
        "Round 1: exactly 1 VP8X"
    );
    assert_eq!(
        count_webp_chunks(&out2, b"VP8X"),
        1,
        "Round 2: exactly 1 VP8X"
    );
    assert_eq!(
        count_webp_chunks(&out3, b"VP8X"),
        1,
        "Round 3: exactly 1 VP8X"
    );

    let ratio = out3.len() as f64 / out1.len() as f64;
    assert!(
        ratio < 1.5,
        "Three rounds should not cause unbounded growth (ratio={:.2})",
        ratio
    );

    let img = image::load_from_memory(&out3).expect("Triple-processed WebP should decode");
    assert_eq!(img.dimensions(), (32, 32));
}

#[test]
fn webp_vp8x_flags_consistent_with_chunks() {
    let img = make_test_image_png(32, 32);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes_with_warnings(&img, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    let flags = webp_vp8x_flags(&output).expect("VP8X must exist");
    let xmp_count = count_webp_chunks(&output, b"XMP ");
    let exif_count = count_webp_chunks(&output, b"EXIF");
    let icc_count = count_webp_chunks(&output, b"ICCP");

    if xmp_count > 0 {
        assert_ne!(
            flags & 0x04,
            0,
            "XMP bit must be set when XMP chunks present"
        );
    }
    if exif_count > 0 {
        assert_ne!(
            flags & 0x08,
            0,
            "EXIF bit must be set when EXIF chunks present"
        );
    }
    if icc_count > 0 {
        assert_ne!(
            flags & 0x20,
            0,
            "ICC bit must be set when ICCP chunks present"
        );
    }
    assert_eq!(count_webp_chunks(&output, b"VP8X"), 1, "Exactly one VP8X");
}

#[test]
fn webp_vp8x_no_exif_when_no_exif_chunk() {
    let img = make_test_image_png(32, 32);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = process_image_bytes_with_warnings(&img, ProtectionLevel::Standard, &ctx)
        .unwrap()
        .0;

    let flags = webp_vp8x_flags(&output).expect("VP8X must exist");
    assert_eq!(
        flags & 0x08,
        0,
        "EXIF bit must not be set when no EXIF chunk"
    );
    assert_eq!(count_webp_chunks(&output, b"EXIF"), 0, "No EXIF chunks");
}

#[test]
fn webp_unrelated_plus_field_survives() {
    let xmp_data = br#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
    plus:License="https://creativecommons.org/licenses/by/4.0/"
    plus:DataSource="test-source"/>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;
    let webp = make_webp_with_extra_chunks(&[(b"XMP ", xmp_data)], 32, 32);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = RightsMetadataProtector::new();
    let output = trap.inject_bytes(&webp, &ctx).unwrap();

    assert_eq!(
        count_webp_chunks(&output, b"XMP "),
        1,
        "Exactly one XMP chunk"
    );
    let xmps = get_webp_xmp_raw(&output);
    assert_eq!(xmps.len(), 1);
    let xmp_str = String::from_utf8_lossy(&xmps[0]);
    assert!(
        xmp_str.contains("plus:License"),
        "Unrelated plus:License must survive stripping"
    );
    assert!(
        xmp_str.contains("plus:DataSource"),
        "Unrelated plus:DataSource must survive stripping"
    );
    assert!(
        xmp_str.contains("plus:DataMining"),
        "StegoEggo plus:DataMining must be present"
    );
}

fn build_webp_with_xmp(xmp_bytes: &[u8]) -> Vec<u8> {
    let img = make_test_image_png(32, 32);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("XMP Holder")
                .with_usage_terms("XMP Terms"),
        )
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let webp = stegoeggo::process_image_bytes_with_warnings(&img, ProtectionLevel::Standard, &ctx)
        .expect("initial WebP encode")
        .0;

    let mut data = Vec::new();
    let mut pos = 12;
    let mut webp_chunks = Vec::new();
    while pos + 8 <= webp.len() {
        let fourcc = &webp[pos..pos + 4];
        let chunk_size =
            u32::from_le_bytes([webp[pos + 4], webp[pos + 5], webp[pos + 6], webp[pos + 7]])
                as usize;
        webp_chunks.push((
            fourcc.to_vec(),
            webp[pos + 8..pos + 8 + chunk_size].to_vec(),
        ));
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }

    let mut vp8x_flags = 0x04u8;
    let mut has_image = false;
    for (fourcc, payload) in &webp_chunks {
        if fourcc == b"VP8X" {
            vp8x_flags |= payload[0] & 0x3D;
        } else if fourcc == b"VP8 " || fourcc == b"VP8L" {
            has_image = true;
        }
    }
    if !has_image {
        panic!("expected VP8/VP8L in WebP fixture");
    }

    let mut vp8x_payload = [vp8x_flags, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    vp8x_payload[4] = 0x1F;
    vp8x_payload[5] = 0x00;
    vp8x_payload[6] = 0x00;
    vp8x_payload[7] = 0x1F;
    vp8x_payload[8] = 0x00;
    vp8x_payload[9] = 0x00;

    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(b"WEBP");
    data.extend_from_slice(b"VP8X");
    data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
    data.extend_from_slice(&vp8x_payload);
    for (fourcc, payload) in &webp_chunks {
        if fourcc == b"VP8X" || fourcc == b"XMP " {
            continue;
        }
        data.extend_from_slice(fourcc);
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(payload);
        if payload.len() & 1 != 0 {
            data.push(0);
        }
    }
    data.extend_from_slice(b"XMP ");
    data.extend_from_slice(&(xmp_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(xmp_bytes);
    if xmp_bytes.len() & 1 != 0 {
        data.push(0);
    }
    let riff_size = (data.len() - 8) as u32;
    data[4..8].copy_from_slice(&riff_size.to_le_bytes());
    data
}

fn webp_xmp_bytes(webp: &[u8]) -> Vec<u8> {
    let mut pos = 12;
    let mut out = Vec::new();
    while pos + 8 <= webp.len() {
        let fourcc = &webp[pos..pos + 4];
        let chunk_size =
            u32::from_le_bytes([webp[pos + 4], webp[pos + 5], webp[pos + 6], webp[pos + 7]])
                as usize;
        if fourcc == b"XMP " {
            out.extend_from_slice(&webp[pos + 8..pos + 8 + chunk_size]);
        }
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }
    out
}

#[test]
fn webp_mixed_owned_and_unrelated_attributes_preserved() {
    let xmp = br#"<?xml version="1.0"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"
dc:creator="Alice"/></rdf:RDF>
</x:xmpmeta>"#;
    let webp = build_webp_with_xmp(xmp);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Holder")
                .with_usage_terms("Terms"),
        )
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = RightsMetadataProtector::new()
        .inject_bytes(&webp, &ctx)
        .expect("rewrite succeeds");
    let rewritten_xmp = webp_xmp_bytes(&output);
    let rewritten_str = std::str::from_utf8(&rewritten_xmp).expect("utf8");
    assert!(
        rewritten_str.contains("Alice"),
        "unrelated dc:creator preserved: {}",
        rewritten_str
    );
}

#[test]
fn webp_unrelated_dc_creator_only_preserved() {
    let xmp = br#"<?xml version="1.0"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"
dc:creator="Alice"/></rdf:RDF>
</x:xmpmeta>"#;
    let webp = build_webp_with_xmp(xmp);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Holder")
                .with_usage_terms("Terms"),
        )
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = RightsMetadataProtector::new()
        .inject_bytes(&webp, &ctx)
        .expect("rewrite succeeds");
    let rewritten_xmp = webp_xmp_bytes(&output);
    let rewritten_str = std::str::from_utf8(&rewritten_xmp).expect("utf8");
    assert!(
        rewritten_str.contains("Alice"),
        "unrelated dc:creator preserved: {}",
        rewritten_str
    );
}

#[test]
fn webp_owned_fields_under_alternate_prefix_replaced() {
    let xmp = br#"<?xml version="1.0"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"
dc:creator="Alice"/></rdf:RDF>
</x:xmpmeta>"#;
    let webp = build_webp_with_xmp(xmp);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Holder")
                .with_usage_terms("Terms"),
        )
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = RightsMetadataProtector::new()
        .inject_bytes(&webp, &ctx)
        .expect("rewrite succeeds");
    let rewritten_xmp = webp_xmp_bytes(&output);
    let rewritten_str = std::str::from_utf8(&rewritten_xmp).expect("utf8");
    assert!(
        rewritten_str.contains("Alice"),
        "unrelated dc:creator preserved: {}",
        rewritten_str
    );
}

#[test]
fn webp_same_local_name_wrong_namespace_preserved() {
    let xmp = br#"<?xml version="1.0"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF><rdf:Description xmlns:other="http://example.com/other/"
other:DataMining="UnrelatedSameLocal"/></rdf:RDF>
</x:xmpmeta>"#;
    let webp = build_webp_with_xmp(xmp);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Holder")
                .with_usage_terms("Terms"),
        )
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = RightsMetadataProtector::new()
        .inject_bytes(&webp, &ctx)
        .expect("rewrite succeeds");
    let rewritten_xmp = webp_xmp_bytes(&output);
    let rewritten_str = std::str::from_utf8(&rewritten_xmp).expect("utf8");
    assert!(
        rewritten_str.contains("UnrelatedSameLocal"),
        "same local in unrelated namespace must survive: {}",
        rewritten_str
    );
}

#[test]
fn webp_rewrite_emits_exactly_one_xmp_chunk() {
    let img = make_test_image_png(32, 32);
    let initial_ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Holder")
                .with_usage_terms("Terms"),
        )
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let webp =
        stegoeggo::process_image_bytes_with_warnings(&img, ProtectionLevel::Standard, &initial_ctx)
            .expect("initial WebP encode")
            .0;

    let rewrite_ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Holder2")
                .with_usage_terms("Terms2"),
        )
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let output = RightsMetadataProtector::new()
        .inject_bytes(&webp, &rewrite_ctx)
        .expect("rewrite succeeds");
    assert_eq!(
        count_webp_chunks(&output, b"XMP "),
        1,
        "output WebP must have exactly one XMP chunk"
    );
}

#[test]
fn webp_three_round_rewrite_is_semantically_idempotent() {
    let img = make_test_image_png(32, 32);
    let initial_ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Holder")
                .with_usage_terms("Terms"),
        )
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let mut current =
        stegoeggo::process_image_bytes_with_warnings(&img, ProtectionLevel::Standard, &initial_ctx)
            .expect("initial WebP encode")
            .0;

    let rewrite_ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Holder2")
                .with_usage_terms("Terms2"),
        )
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    for round in 0..3 {
        current = RightsMetadataProtector::new()
            .inject_bytes(&current, &rewrite_ctx)
            .expect("round succeeds");
        assert_eq!(
            count_webp_chunks(&current, b"XMP "),
            1,
            "round {} XMP chunks",
            round
        );
        let xmp = webp_xmp_bytes(&current);
        let xmp_str = std::str::from_utf8(&xmp).expect("utf8");
        assert!(
            xmp_str.contains("plus:DataMining"),
            "round {} should retain current plus:DataMining",
            round
        );
    }
}

#[derive(Debug, Default)]
struct XmpSemanticFacts {
    #[allow(dead_code)]
    data_mining_count: usize,
    data_mining_uris: Vec<String>,
    owned_field_counts: std::collections::BTreeMap<(String, String), usize>,
    unrelated_values: Vec<XmpAttribute>,
    rdf_description_count: usize,
    #[allow(dead_code)]
    preserved_rdf_description_count: usize,
}

#[derive(Debug, Clone)]
struct XmpAttribute {
    #[allow(dead_code)]
    namespace_uri: String,
    #[allow(dead_code)]
    local_name: String,
    value: String,
}

fn parse_xmp_semantic_facts(xml: &[u8]) -> XmpSemanticFacts {
    use quick_xml::events::Event;
    use quick_xml::name::ResolveResult;
    use quick_xml::reader::NsReader;

    let packet_str = std::str::from_utf8(xml).expect("utf8");
    let mut reader = NsReader::from_str(packet_str);
    let mut buf = Vec::new();
    let mut facts = XmpSemanticFacts::default();
    let mut current_element_ns: Option<(String, String)> = None;

    loop {
        let event = reader.read_event_into(&mut buf).expect("parse");
        match event {
            Event::Start(ref s) | Event::Empty(ref s) => {
                let (resolve, _) = reader.resolver().resolve_element(s.name());
                let ns_uri = if let ResolveResult::Bound(ns) = resolve {
                    String::from_utf8_lossy(ns.as_ref()).into_owned()
                } else {
                    String::new()
                };
                let local = String::from_utf8_lossy(s.local_name().as_ref()).into_owned();
                let is_rdf_desc = ns_uri == "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                    && local == "Description";
                if is_rdf_desc {
                    facts.rdf_description_count += 1;
                }
                let stego_ns = "https://github.com/eggstack/stegoeggo";
                let stego_owned: &[&str] = &[
                    "ProtectionSeed",
                    "ProtectionLevel",
                    "RightsPolicy",
                    "AIConstraints",
                    "CopyrightOwner",
                    "LicensorName",
                    "LicensorEmail",
                    "LicensorURL",
                    "NoticeAppliedAt",
                ];
                if ns_uri == stego_ns {
                    for n in stego_owned {
                        if local == *n {
                            *facts
                                .owned_field_counts
                                .entry((ns_uri.clone(), local.clone()))
                                .or_insert(0) += 1;
                        }
                    }
                }
                if ns_uri == "http://ns.useplus.org/ldf/xmp/1.0/" && local == "OtherConstraints" {
                    *facts
                        .owned_field_counts
                        .entry((ns_uri.clone(), local.clone()))
                        .or_insert(0) += 1;
                }
                for attr_res in s.attributes() {
                    let Ok(attr) = attr_res else { continue };
                    let key = attr.key.as_ref();
                    if key.starts_with(b"xmlns") || key.starts_with(b"rdf:") {
                        continue;
                    }
                    let attr_uri = resolve_attr_uri(&reader, key);
                    let attr_local = String::from_utf8_lossy(
                        if let Some(colon) = key.iter().position(|&b| b == b':') {
                            &key[colon + 1..]
                        } else {
                            key
                        },
                    )
                    .into_owned();
                    let value = attr
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .expect("attribute value should decode")
                        .into_owned();
                    if attr_uri == "http://ns.useplus.org/ldf/xmp/1.0/"
                        && attr_local == "DataMining"
                    {
                        facts.data_mining_count += 1;
                        if !value.is_empty() {
                            facts.data_mining_uris.push(value.clone());
                        }
                    }
                    let skip_attr = is_stegoeggo_owned_attr(key, &attr_uri)
                        || (attr_uri == "http://ns.useplus.org/ldf/xmp/1.0/"
                            && (attr_local == "DataMining" || attr_local == "OtherConstraints"));
                    if skip_attr {
                        continue;
                    }
                    facts.unrelated_values.push(XmpAttribute {
                        namespace_uri: attr_uri,
                        local_name: attr_local,
                        value,
                    });
                }
                let skip_element = ns_uri == "http://ns.useplus.org/ldf/xmp/1.0/"
                    && (local == "DataMining" || local == "OtherConstraints");
                if !skip_element && !is_rdf_desc && !ns_uri.is_empty() {
                    current_element_ns = Some((ns_uri.clone(), local.clone()));
                } else {
                    current_element_ns = None;
                }
            }
            Event::End(_) => {
                current_element_ns = None;
            }
            Event::Text(ref t) => {
                let text = String::from_utf8_lossy(t.as_ref()).into_owned();
                if let Some((ns, local)) = &current_element_ns {
                    if !text.trim().is_empty() {
                        facts.unrelated_values.push(XmpAttribute {
                            namespace_uri: ns.clone(),
                            local_name: local.clone(),
                            value: text,
                        });
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    facts
}

fn resolve_attr_uri(reader: &quick_xml::reader::NsReader<&[u8]>, key: &[u8]) -> String {
    let resolver = reader.resolver();
    let (resolve, _) = resolver.resolve_attribute(quick_xml::name::QName(key));
    match resolve {
        quick_xml::name::ResolveResult::Bound(ns) => {
            String::from_utf8_lossy(ns.as_ref()).into_owned()
        }
        _ => String::new(),
    }
}

fn is_stegoeggo_owned_attr(key: &[u8], ns_uri: &str) -> bool {
    if ns_uri != "https://github.com/eggstack/stegoeggo" {
        return false;
    }
    let local = if let Some(colon) = key.iter().position(|&b| b == b':') {
        &key[colon + 1..]
    } else {
        key
    };
    let local_str = String::from_utf8_lossy(local).into_owned();
    matches!(
        local_str.as_str(),
        "ProtectionSeed"
            | "ProtectionLevel"
            | "RightsPolicy"
            | "AIConstraints"
            | "CopyrightOwner"
            | "LicensorName"
            | "LicensorEmail"
            | "LicensorURL"
            | "NoticeAppliedAt"
    )
}

fn get_webp_xmp_packets(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut xmps = Vec::new();
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let fourcc = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        if fourcc == b"XMP " {
            let data_start = pos + 8;
            let data_end = data_start + chunk_size;
            if data_end <= bytes.len() {
                xmps.push(bytes[data_start..data_end].to_vec());
            }
        }
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }
    xmps
}

#[test]
fn webp_xmp_reference_values_are_semantically_stable_three_rounds() {
    let xmp = br#"<?xml version="1.0"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF><rdf:Description xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/" xmlns:dc="http://purl.org/dc/elements/1.1/" dc:title="A &amp; B">
<plus:OtherConstraints><rdf:Description><rdf:value>owned &amp; nested</rdf:value><!-- owned comment --><?owned test?></rdf:Description></plus:OtherConstraints>
<dc:creator>decimal &#169; and hex &#x1F642;</dc:creator>
</rdf:Description></rdf:RDF>
</x:xmpmeta>"#;
    let webp = build_webp_with_xmp(xmp);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(legal())
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let trap = RightsMetadataProtector::new();
    let mut current = webp;
    let mut facts = Vec::new();

    for _ in 0..3 {
        current = trap.inject_bytes(&current, &ctx).expect("rewrite succeeds");
        let packets = get_webp_xmp_packets(&current);
        assert_eq!(packets.len(), 1, "exactly one XMP chunk");
        facts.push(parse_xmp_semantic_facts(&packets[0]));
    }

    for (round, fact) in facts.iter().enumerate() {
        assert_eq!(fact.rdf_description_count, 2, "round {round}");
        assert_eq!(fact.data_mining_count, 1, "round {round}");
        assert!(
            fact.unrelated_values.iter().any(|value| {
                value.namespace_uri == "http://purl.org/dc/elements/1.1/"
                    && value.local_name == "title"
                    && value.value == "A & B"
            }),
            "round {round} must preserve the decoded attribute"
        );
        assert!(
            fact.unrelated_values.iter().any(|value| {
                value.namespace_uri == "http://purl.org/dc/elements/1.1/"
                    && value.local_name == "creator"
                    && value.value == "decimal © and hex 🙂"
            }),
            "round {round} must preserve decoded text"
        );
        assert!(
            fact.unrelated_values
                .iter()
                .all(|value| !value.value.contains("owned")),
            "round {round} must suppress owned nested content"
        );
    }
}

#[test]
fn webp_three_round_rewrite_xmp_is_parser_idempotent() {
    let img = make_test_image_png(32, 32);
    let initial_ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Holder")
                .with_usage_terms("Terms")
                .with_creator("Creator")
                .with_ai_constraints("No AI training"),
        )
        .with_dmi(DmiValue::ProhibitedAiMlTraining);
    let initial_webp =
        stegoeggo::process_image_bytes_with_warnings(&img, ProtectionLevel::Standard, &initial_ctx)
            .expect("initial WebP encode")
            .0;
    let mut current = initial_webp;

    let rewrite_ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Holder2")
                .with_usage_terms("Terms2"),
        )
        .with_dmi(DmiValue::ProhibitedAiMlTraining);

    let mut round_xml: Vec<Vec<u8>> = Vec::new();
    for _round in 0..3 {
        current = RightsMetadataProtector::new()
            .inject_bytes(&current, &rewrite_ctx)
            .expect("round succeeds");
        let xmps = get_webp_xmp_packets(&current);
        assert_eq!(xmps.len(), 1, "exactly one XMP chunk per round");
        round_xml.push(xmps[0].clone());
    }

    let facts: Vec<XmpSemanticFacts> = round_xml
        .iter()
        .map(|x| parse_xmp_semantic_facts(x))
        .collect();

    for (i, facts) in facts.iter().enumerate() {
        assert_eq!(
            facts.rdf_description_count, 2,
            "Round {} should have 2 rdf:Description (preserved + current): {:?}",
            i, facts
        );
        assert_eq!(
            facts.data_mining_count, 1,
            "Round {} should have exactly one DataMining (PLUS): {:?}",
            i, facts
        );
        assert_eq!(
            facts.data_mining_uris.len(),
            1,
            "Round {} should have exactly one DataMining URI: {:?}",
            i,
            facts
        );
        assert!(
            facts.data_mining_uris[0].contains("DMI-PROHIBITED-AIMLTRAINING"),
            "Round {} current DataMining URI must remain: {:?}",
            i,
            facts
        );
    }

    for (label, facts) in [
        ("round1", &facts[0]),
        ("round2", &facts[1]),
        ("round3", &facts[2]),
    ] {
        let notice_count = facts
            .owned_field_counts
            .get(&(
                "https://github.com/eggstack/stegoeggo".to_string(),
                "NoticeAppliedAt".to_string(),
            ))
            .copied()
            .unwrap_or(0);
        assert_eq!(
            notice_count, 1,
            "{} must have exactly one NoticeAppliedAt (current canonical): {:?}",
            label, facts.owned_field_counts
        );
    }

    let has_creator = facts[0]
        .unrelated_values
        .iter()
        .any(|a| a.value == "Creator");
    assert!(
        has_creator,
        "Creator value from round 0 must survive in round 1: {:?}",
        facts[0].unrelated_values
    );
}

fn build_animation_webp(
    canvas_width: u32,
    canvas_height: u32,
    anmf_payloads: &[[u8; 16]],
    frames: &[Vec<u8>],
    with_xmp: bool,
) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(b"WEBP");
    let mut vp8x_payload = vec![0u8; 10];
    vp8x_payload[0] = if with_xmp { 0x06 } else { 0x02 };
    vp8x_payload[4] = (canvas_width - 1) as u8;
    vp8x_payload[7] = (canvas_height - 1) as u8;
    data.extend_from_slice(b"VP8X");
    data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
    data.extend_from_slice(&vp8x_payload);
    data.extend_from_slice(b"ANIM");
    data.extend_from_slice(&6u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 6]);
    for (i, payload) in anmf_payloads.iter().enumerate() {
        let frame = &frames[i];
        let padded = frame.len() + (frame.len() & 1);
        let anmf_size = (16 + 8 + padded) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(payload);
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        data.extend_from_slice(frame);
        if frame.len() & 1 != 0 {
            data.push(0);
        }
    }
    if with_xmp {
        let xmp = b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\"/></rdf:RDF></x:xmpmeta>";
        data.extend_from_slice(b"XMP ");
        data.extend_from_slice(&(xmp.len() as u32).to_le_bytes());
        data.extend_from_slice(xmp);
        if xmp.len() & 1 != 0 {
            data.push(0);
        }
    }
    let riff_size = (data.len() - 8) as u32;
    data[4..8].copy_from_slice(&riff_size.to_le_bytes());
    data
}

fn extract_anmf_payloads(webp: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 12;
    while pos + 8 <= webp.len() {
        let fourcc = &webp[pos..pos + 4];
        let chunk_size =
            u32::from_le_bytes([webp[pos + 4], webp[pos + 5], webp[pos + 6], webp[pos + 7]])
                as usize;
        if fourcc == b"ANMF" {
            let data_start = pos + 8;
            let data_end = data_start + chunk_size;
            if data_end <= webp.len() {
                out.push(webp[data_start..data_end].to_vec());
            }
        }
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }
    out
}

fn rewrite_animation(webp: &[u8]) -> Vec<u8> {
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::WebP)
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Container Test Holder")
                .with_usage_terms("Container Test Terms")
                .with_creator("Container Creator"),
        );
    RightsMetadataProtector::new()
        .inject_bytes(webp, &ctx)
        .expect("rewrite succeeds")
}

fn assert_animated_rewrite_correct(label: &str, webp_before: &[u8], expected_anmf: &[Vec<u8>]) {
    let output = rewrite_animation(webp_before);

    assert_eq!(
        count_webp_chunks(&output, b"VP8X"),
        1,
        "{}: exactly one VP8X",
        label
    );
    assert_eq!(
        count_webp_chunks(&output, b"XMP "),
        1,
        "{}: exactly one XMP",
        label
    );
    assert_eq!(
        count_webp_chunks(&output, b"ANIM"),
        1,
        "{}: exactly one ANIM",
        label
    );
    let output_anmfs = extract_anmf_payloads(&output);
    assert_eq!(
        output_anmfs.len(),
        expected_anmf.len(),
        "{}: ANMF count",
        label
    );
    for (i, expected) in expected_anmf.iter().enumerate() {
        let output_header = &output_anmfs[i][..16];
        assert_eq!(
            output_header,
            expected.as_slice(),
            "{}: ANMF frame {} 16-byte header must be byte-identical",
            label,
            i
        );
    }
    let flags = webp_vp8x_flags(&output).expect("VP8X must exist");
    assert_ne!(flags & 0x02, 0, "{}: animation flag must be set", label);
    let xmp = get_webp_xmp_raw(&output);
    let xmp_str = String::from_utf8_lossy(&xmp[0]);
    assert!(
        xmp_str.contains("plus:DataMining") || xmp_str.contains("DMI-PROHIBITED-AIMLTRAINING"),
        "{}: DataMining present: {}",
        label,
        xmp_str
    );
}

#[test]
fn webp_animated_opaque_vp8_rewrite_succeeds() {
    let canvas_width = 8u32;
    let canvas_height = 8u32;
    let anmf_payloads = vec![vec![0u8; 16], vec![0u8; 16]];
    let frames = vec![vec![0u8; 4], vec![0u8; 4]];
    let anmf_byte_arrays: Vec<[u8; 16]> = anmf_payloads
        .iter()
        .map(|v| {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(v);
            arr
        })
        .collect();
    let webp = build_animation_webp(
        canvas_width,
        canvas_height,
        &anmf_byte_arrays,
        &frames,
        false,
    );
    assert_animated_rewrite_correct("opaque", &webp, &anmf_payloads);
}

#[test]
fn webp_animated_multiple_frame_rewrite_byte_identical() {
    let canvas_width = 8u32;
    let canvas_height = 8u32;
    let anmf_payloads = vec![vec![0u8; 16], vec![0u8; 16], vec![0u8; 16]];
    let frames = vec![vec![0u8; 4], vec![0u8; 4], vec![0u8; 4]];
    let anmf_byte_arrays: Vec<[u8; 16]> = anmf_payloads
        .iter()
        .map(|v| {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(v);
            arr
        })
        .collect();
    let webp = build_animation_webp(
        canvas_width,
        canvas_height,
        &anmf_byte_arrays,
        &frames,
        false,
    );
    assert_animated_rewrite_correct("multi", &webp, &anmf_payloads);
}

#[test]
fn webp_animated_with_xmp_rewrite_succeeds() {
    let canvas_width = 8u32;
    let canvas_height = 8u32;
    let anmf_payloads = vec![vec![0u8; 16], vec![0u8; 16]];
    let frames = vec![vec![0u8; 4], vec![0u8; 4]];
    let anmf_byte_arrays: Vec<[u8; 16]> = anmf_payloads
        .iter()
        .map(|v| {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(v);
            arr
        })
        .collect();
    let webp = build_animation_webp(
        canvas_width,
        canvas_height,
        &anmf_byte_arrays,
        &frames,
        true,
    );
    assert_animated_rewrite_correct("with_xmp", &webp, &anmf_payloads);
}

#[test]
fn webp_animated_unknown_top_level_chunk_preserved() {
    let canvas_width = 8u32;
    let canvas_height = 8u32;
    let anmf_payloads = vec![[0u8; 16]];
    let frames = vec![vec![0u8; 4]];
    let mut webp =
        build_animation_webp(canvas_width, canvas_height, &anmf_payloads, &frames, false);
    webp.extend_from_slice(b"TEST");
    webp.extend_from_slice(&4u32.to_le_bytes());
    webp.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0]);
    let riff_size = (webp.len() - 8) as u32;
    webp[4..8].copy_from_slice(&riff_size.to_le_bytes());

    let output = rewrite_animation(&webp);
    let mut found = false;
    let mut pos = 12;
    while pos + 8 <= output.len() {
        let fourcc = &output[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            output[pos + 4],
            output[pos + 5],
            output[pos + 6],
            output[pos + 7],
        ]) as usize;
        if fourcc == b"TEST" {
            found = true;
        }
        let padded = chunk_size + (chunk_size & 1);
        pos += 8 + padded;
    }
    assert!(found, "unknown TEST chunk must be preserved");
}
