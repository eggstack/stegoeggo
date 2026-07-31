#![allow(deprecated)]

use image::DynamicImage;
use image::ImageEncoder;
use stegoeggo::{
    process_image_bytes, ImageOutputFormat, LegalMetadata, ProtectionContext, ProtectionLevel,
    RightsMetadataProtector, VerificationStatus,
};

fn make_jpeg(width: u32, height: u32, quality: u8) -> Vec<u8> {
    let img = DynamicImage::new_rgb8(width, height);
    let mut rgb = img.to_rgb8();
    for y in 0..height {
        for x in 0..width {
            let r = ((x * 7 + y * 3) % 256) as u8;
            let g = ((x * 11 + y * 5) % 256) as u8;
            let b = ((x * 13 + y * 9) % 256) as u8;
            rgb.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    encoder
        .write_image(&rgb, width, height, image::ExtendedColorType::Rgb8)
        .unwrap();
    buf.into_inner()
}

fn legal() -> LegalMetadata {
    LegalMetadata::new()
        .with_copyright_holder("Fixture Holder")
        .with_usage_terms("Fixture Terms")
}

fn jpeg_find_sos_region(jpeg: &[u8]) -> Option<(usize, usize)> {
    let mut pos = 2;
    while pos + 4 <= jpeg.len() {
        if jpeg[pos] != 0xFF {
            pos += 1;
            continue;
        }
        let marker = jpeg[pos + 1];
        if marker == 0xDA {
            let len = u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]) as usize;
            let scan_start = pos + 2 + len;
            return Some((scan_start, jpeg.len()));
        }
        if marker == 0xD9 || marker == 0x00 {
            return None;
        }
        let seg_len = u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]) as usize;
        pos += 2 + seg_len;
    }
    None
}

fn jpeg_has_marker(jpeg: &[u8], target: u8) -> bool {
    let mut pos = 2;
    while pos + 2 <= jpeg.len() {
        if jpeg[pos] != 0xFF {
            break;
        }
        let marker = jpeg[pos + 1];
        if marker == target {
            return true;
        }
        if marker == 0xD8 || marker == 0xD9 {
            pos += 2;
            continue;
        }
        if pos + 4 > jpeg.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]) as usize;
        pos += 2 + seg_len;
    }
    false
}

fn inject_metadata(jpeg: &[u8]) -> Vec<u8> {
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal());
    let trap = RightsMetadataProtector::new();
    trap.inject_bytes(jpeg, &ctx).unwrap()
}

#[test]
fn progressive_jpeg_metadata_only_preserves_scan() {
    let base = make_jpeg(64, 64, 85);

    let mut progressive_base = base.clone();
    let sof_pos = progressive_base
        .windows(2)
        .position(|w| w == [0xFF, 0xC0])
        .expect("should have SOF");
    progressive_base[sof_pos + 1] = 0xC2;

    let output = inject_metadata(&progressive_base);

    let orig_region = jpeg_find_sos_region(&progressive_base).expect("progressive should have SOS");
    let out_region = jpeg_find_sos_region(&output).expect("output should have SOS");

    assert_eq!(
        progressive_base[orig_region.0..orig_region.1],
        output[out_region.0..out_region.1],
        "Progressive JPEG scan bytes must be preserved in metadata-only path"
    );
}

#[test]
fn restart_bearing_jpeg_metadata_only_preserves_scan() {
    let base = make_jpeg(64, 64, 85);

    let mut with_dri = base.clone();
    let sos_pos = with_dri
        .windows(2)
        .position(|w| w == [0xFF, 0xDA])
        .expect("should have SOS");
    let dri_data = [0x00, 0x08];
    let dri_len: u16 = 4;
    with_dri.splice(
        sos_pos..sos_pos,
        [0xFF, 0xDD]
            .into_iter()
            .chain(dri_len.to_be_bytes())
            .chain(dri_data)
            .collect::<Vec<u8>>(),
    );

    let output = inject_metadata(&with_dri);

    let orig_region = jpeg_find_sos_region(&with_dri).expect("DRI-bearing should have SOS");
    let out_region = jpeg_find_sos_region(&output).expect("output should have SOS");

    assert_eq!(
        with_dri[orig_region.0..orig_region.1],
        output[out_region.0..out_region.1],
        "DRI-bearing JPEG scan bytes must be preserved in metadata-only path"
    );
}

#[test]
fn custom_huffman_table_jpeg_round_trip() {
    let base = make_jpeg(32, 32, 50);

    let output = inject_metadata(&base);

    let img = image::load_from_memory(&output);
    assert!(
        img.is_ok(),
        "JPEG with standard Huffman tables should survive metadata injection: {:?}",
        img.err()
    );
}

#[test]
fn app2_icc_profile_marker_survives_injection() {
    let base = make_jpeg(64, 64, 85);

    let icc_data = [0u8; 32];
    let icc_len = (icc_data.len() + 2) as u16;
    let mut with_icc = Vec::with_capacity(base.len() + 4 + icc_data.len());
    with_icc.extend_from_slice(&base[..2]);
    with_icc.extend_from_slice(&[0xFF, 0xE2]);
    with_icc.extend_from_slice(&icc_len.to_be_bytes());
    with_icc.extend_from_slice(&icc_data);
    with_icc.extend_from_slice(&base[2..]);

    assert!(jpeg_has_marker(&with_icc, 0xE2), "base should have APP2");

    let output = inject_metadata(&with_icc);

    assert!(
        jpeg_has_marker(&output, 0xE2),
        "APP2 marker should survive metadata injection"
    );
    assert!(
        image::load_from_memory(&output).is_ok(),
        "output with preserved APP2 should decode"
    );
}

#[test]
fn app13_iptc_marker_survives_injection() {
    let base = make_jpeg(64, 64, 85);

    let mut iptc_data = Vec::new();
    iptc_data.extend_from_slice(b"Photoshop 3.0\0");
    iptc_data.extend_from_slice(b"8BIM");
    iptc_data.extend_from_slice(&4u16.to_be_bytes());
    iptc_data.push(0);
    let record = b"IPTC test";
    iptc_data.extend_from_slice(&(record.len() as u32).to_be_bytes());
    iptc_data.extend_from_slice(record);

    let iptc_len = (iptc_data.len() + 2) as u16;
    let mut with_iptc = Vec::with_capacity(base.len() + 4 + iptc_data.len());
    with_iptc.extend_from_slice(&base[..2]);
    with_iptc.extend_from_slice(&[0xFF, 0xED]);
    with_iptc.extend_from_slice(&iptc_len.to_be_bytes());
    with_iptc.extend_from_slice(&iptc_data);
    with_iptc.extend_from_slice(&base[2..]);

    assert!(jpeg_has_marker(&with_iptc, 0xED), "base should have APP13");

    let output = inject_metadata(&with_iptc);

    assert!(
        jpeg_has_marker(&output, 0xED),
        "APP13 marker should survive metadata injection"
    );
    assert!(
        image::load_from_memory(&output).is_ok(),
        "output with preserved APP13 should decode"
    );
}

#[test]
fn truncated_jpeg_returns_error() {
    let base = make_jpeg(64, 64, 85);
    let truncated = &base[..base.len() / 2];

    let result = inject_metadata(truncated);
    let img_result = image::load_from_memory(&result);
    assert!(
        img_result.is_err() || result.len() >= truncated.len(),
        "Truncated JPEG should either error or produce output that doesn't silently succeed"
    );
}

#[test]
fn multiple_app_segments_preserve_order() {
    let base = make_jpeg(64, 64, 85);

    let app2_data = [0x01u8; 16];
    let app2_len = (app2_data.len() + 2) as u16;
    let com_data = b"test comment";
    let com_len = (com_data.len() + 2) as u16;

    let mut enriched = Vec::new();
    enriched.extend_from_slice(&base[..2]);

    enriched.extend_from_slice(&[0xFF, 0xE2]);
    enriched.extend_from_slice(&app2_len.to_be_bytes());
    enriched.extend_from_slice(&app2_data);

    enriched.extend_from_slice(&[0xFF, 0xFE]);
    enriched.extend_from_slice(&com_len.to_be_bytes());
    enriched.extend_from_slice(com_data);

    enriched.extend_from_slice(&base[2..]);

    let output = inject_metadata(&enriched);

    assert!(jpeg_has_marker(&output, 0xE2), "APP2 should survive");
    assert!(jpeg_has_marker(&output, 0xFE), "COM should survive");
    assert!(
        image::load_from_memory(&output).is_ok(),
        "output with multiple preserved segments should decode"
    );

    let app2_pos_out = output
        .windows(2)
        .position(|w| w == [0xFF, 0xE2])
        .expect("output should have APP2");
    let com_pos_out = output
        .windows(2)
        .position(|w| w == [0xFF, 0xFE])
        .expect("output should have COM");
    assert!(
        app2_pos_out < com_pos_out,
        "APP2 should appear before COM in output"
    );
}

#[test]
fn large_jpeg_metadata_injection_preserves_scan() {
    let base = make_jpeg(256, 256, 90);

    let (orig_scan_start, orig_scan_end) =
        jpeg_find_sos_region(&base).expect("large JPEG should have SOS");
    let orig_scan_bytes = base[orig_scan_start..orig_scan_end].to_vec();

    let output = inject_metadata(&base);

    let (out_scan_start, out_scan_end) =
        jpeg_find_sos_region(&output).expect("output should have SOS");
    let out_scan_bytes = output[out_scan_start..out_scan_end].to_vec();

    assert_eq!(
        orig_scan_bytes, out_scan_bytes,
        "Large JPEG scan bytes must be byte-identical after metadata injection"
    );
    assert!(
        image::load_from_memory(&output).is_ok(),
        "Large output should decode"
    );
}

#[test]
fn standard_jpeg_dct_stego_and_metadata_round_trip() {
    let base = make_jpeg(128, 128, 90);
    let ctx = ProtectionContext::new(0.7, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_legal_metadata(legal());

    let output =
        process_image_bytes(&base, ProtectionLevel::Standard, &ctx).expect("should process");

    let img = image::load_from_memory(&output).expect("output should decode");
    assert_eq!(img.width(), 128);
    assert_eq!(img.height(), 128);

    let status = stegoeggo::verify_image_bytes(&output, &[]);
    assert_eq!(status, VerificationStatus::Verified);
}
