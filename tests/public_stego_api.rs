use image::RgbaImage;
use stegoeggo::stego;
use stegoeggo::stego::frame::{self, FRAMED_MAGIC, FRAME_HEADER_SIZE, FRAME_VERSION};
use stegoeggo::stego::jpeg::{self, JpegConfig, JpegSupport};
use stegoeggo::stego::lsb::{self, LsbConfig};
use stegoeggo::stego::StegoError;

fn make_lsb_image(w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let r = ((x * 7 + y * 13) % 256) as u8;
            let g = ((x * 11 + y * 3) % 256) as u8;
            let b = ((x * 5 + y * 17) % 256) as u8;
            img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
        }
    }
    img
}

fn make_jpeg_bytes(w: u32, h: u32) -> Vec<u8> {
    let mut img = image::RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let r = ((x * 7 + y * 13) % 256) as u8;
            let g = ((x * 11 + y * 3) % 256) as u8;
            let b = ((x * 5 + y * 17) % 256) as u8;
            img.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
    image::DynamicImage::ImageRgb8(img)
        .write_with_encoder(encoder)
        .unwrap();
    buf
}

#[test]
fn public_lsb_known_answer_vector() {
    let source = image::RgbaImage::from_fn(5, 3, |x, y| {
        let values = [0, 255, 2, 127, 254, 1, 128, 253, 3, 252, 4, 251, 5, 250, 6];
        let value = values[(y * 5 + x) as usize];
        image::Rgba([value, 255 - value, value, (x * 53 + y * 29 + 7) as u8])
    });
    let payload = [0xA5];
    let config = LsbConfig::new(0x0123_4567_89AB_CDEF).with_redundancy(1);

    let report = lsb::embed(&source, &payload, &config).unwrap();
    assert!(report.embedded);
    assert_eq!(
        report.output.as_raw(),
        &[
            0, 255, 1, 7, 254, 1, 254, 60, 2, 253, 2, 113, 127, 128, 127, 166, 255, 1, 255, 219, 0,
            253, 1, 36, 129, 127, 128, 89, 253, 1, 253, 142, 3, 252, 3, 195, 252, 3, 252, 248, 4,
            251, 4, 65, 251, 4, 250, 118, 5, 250, 6, 171, 250, 4, 250, 224, 6, 250, 6, 21,
        ]
    );
}

#[test]
fn public_lsb_in_place_matches_clone_and_preserves_alpha() {
    let source = image::RgbaImage::from_fn(17, 19, |x, y| {
        image::Rgba([
            (x * 17 + y * 3) as u8,
            (x * 5 + y * 19) as u8,
            (x * 23 + y * 7) as u8,
            (x * 11 + y * 13 + 1) as u8,
        ])
    });
    let payload = [0xA5, 0x3C, 0x00, 0xFF];
    let config = LsbConfig::new(42).with_redundancy(3);

    let cloned = lsb::embed(&source, &payload, &config).unwrap();
    let mut in_place = source.clone();
    let report = lsb::embed_in_place(&mut in_place, &payload, &config).unwrap();

    assert!(report.embedded);
    assert_eq!(report.payload_bytes, payload.len());
    assert_eq!(report.required_capacity, cloned.required_capacity);
    assert_eq!(report.available_capacity, cloned.available_capacity);
    assert_eq!(report.actual_redundancy, config.redundancy());
    assert_eq!(in_place, cloned.output);
    for (before, after) in source.pixels().zip(in_place.pixels()) {
        assert_eq!(before[3], after[3]);
    }
    assert_eq!(
        lsb::extract(&in_place, payload.len(), &config).unwrap(),
        payload
    );
}

#[test]
fn public_lsb_in_place_capacity_failure_is_atomic() {
    let mut image = make_lsb_image(8, 8);
    let original = image.clone();
    let config = LsbConfig::new(42);

    let report = lsb::embed_in_place(&mut image, &[0u8; 10_000], &config).unwrap();

    assert!(!report.embedded);
    assert_eq!(image, original);
}

#[test]
fn public_lsb_raw_roundtrip_arbitrary_bytes() {
    let img = make_lsb_image(64, 64);
    let payload = b"hello stegoeggo generic api";
    let config = LsbConfig::new(42);

    let report = lsb::embed(&img, payload, &config).unwrap();
    assert!(report.embedded);
    assert_eq!(report.payload_bytes, payload.len());

    let decoded = report.output.clone();
    let recovered = lsb::extract(&decoded, payload.len(), &config).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_lsb_raw_roundtrip_zero_seed() {
    let img = make_lsb_image(64, 64);
    let payload = b"zero seed test";
    let config = LsbConfig::new(0);

    let report = lsb::embed(&img, payload, &config).unwrap();
    assert!(report.embedded);

    let decoded = report.output.clone();
    let recovered = lsb::extract(&decoded, payload.len(), &config).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_lsb_raw_roundtrip_max_seed() {
    let img = make_lsb_image(64, 64);
    let payload = b"max seed test";
    let config = LsbConfig::new(u64::MAX);

    let report = lsb::embed(&img, payload, &config).unwrap();
    assert!(report.embedded);

    let decoded = report.output.clone();
    let recovered = lsb::extract(&decoded, payload.len(), &config).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_lsb_raw_roundtrip_binary_payload() {
    let img = make_lsb_image(64, 64);
    let payload: Vec<u8> = (0..=255).collect();
    let config = LsbConfig::new(99).with_redundancy(1);

    let report = lsb::embed(&img, &payload, &config).unwrap();
    assert!(report.embedded);

    let decoded = report.output.clone();
    let recovered = lsb::extract(&decoded, payload.len(), &config).unwrap();
    assert_eq!(recovered, payload);
}

#[test]
fn public_lsb_capacity_preflight() {
    let img = make_lsb_image(100, 100);
    let config = LsbConfig::new(42);

    let report = lsb::capacity(&img, 100, &config).unwrap();
    assert!(report.is_sufficient());
    assert!(report.available > 0);
    assert!(report.required > 0);

    let embed_report = lsb::embed(&img, &[0u8; 100], &config).unwrap();
    assert_eq!(report.required, embed_report.required_capacity);
    assert_eq!(report.available, embed_report.available_capacity);
}

#[test]
fn public_lsb_insufficient_capacity_structured_error() {
    let img = make_lsb_image(8, 8);
    let config = LsbConfig::new(42);
    let large_payload = vec![0u8; 10000];

    let report = lsb::embed(&img, &large_payload, &config).unwrap();
    assert!(!report.embedded);
    assert!(report.required_capacity > report.available_capacity);
}

#[test]
fn public_lsb_different_seeds_dont_interfere() {
    let img = make_lsb_image(64, 64);
    let payload_a = b"seed A payload";
    let payload_b = b"seed B payload";

    let config_a = LsbConfig::new(1);
    let config_b = LsbConfig::new(2);

    let report_a = lsb::embed(&img, payload_a, &config_a).unwrap();
    let report_b = lsb::embed(&img, payload_b, &config_b).unwrap();

    let decoded_a = report_a.output.clone();
    let decoded_b = report_b.output.clone();

    let recovered_a = lsb::extract(&decoded_a, payload_a.len(), &config_a).unwrap();
    let recovered_b = lsb::extract(&decoded_b, payload_b.len(), &config_b).unwrap();

    assert_eq!(&recovered_a, payload_a);
    assert_eq!(&recovered_b, payload_b);
}

#[test]
fn public_lsb_empty_payload() {
    let img = make_lsb_image(64, 64);
    let config = LsbConfig::new(42);

    let report = lsb::embed(&img, b"", &config).unwrap();
    assert!(report.embedded);
    assert_eq!(report.payload_bytes, 0);
}

#[test]
fn public_lsb_redundancy_affects_capacity() {
    let img = make_lsb_image(64, 64);
    let config_r1 = LsbConfig::new(42).with_redundancy(1);
    let config_r3 = LsbConfig::new(42).with_redundancy(3);

    let cap_r1 = lsb::capacity(&img, 100, &config_r1).unwrap();
    let cap_r3 = lsb::capacity(&img, 100, &config_r3).unwrap();

    assert!(cap_r1.required < cap_r3.required);
}

#[test]
fn public_jpeg_raw_roundtrip_arbitrary_bytes() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);
    let payload = b"jpeg stego test";
    let config = JpegConfig::new(42);

    let report = jpeg::embed(&jpeg_bytes, payload, &config).unwrap();
    assert!(report.embedded);

    let recovered = jpeg::extract(
        &report.output,
        payload.len(),
        &config,
        report.actual_redundancy,
    )
    .unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_jpeg_supported_container_preservation() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);
    let config = JpegConfig::new(42);
    let payload = b"preservation test";

    let report = jpeg::embed(&jpeg_bytes, payload, &config).unwrap();
    assert!(report.embedded);

    assert!(report.output.starts_with(&[0xFF, 0xD8]));
}

#[test]
fn public_jpeg_unsupported_progressive_is_explicit() {
    use jpeg_encoder::Encoder as JpegEnc;

    let img = image::DynamicImage::new_rgb8(64, 64);
    let rgb = img.to_rgb8();
    let mut progressive_buf = Vec::new();
    {
        let mut enc = JpegEnc::new(&mut progressive_buf, 90);
        enc.set_progressive(true);
        enc.encode(rgb.as_raw(), 64, 64, jpeg_encoder::ColorType::Rgb)
            .unwrap();
    }

    let support = jpeg::probe_support(&progressive_buf).unwrap();
    match support {
        JpegSupport::Unsupported(reason) => {
            assert!(
                reason == stego::JpegUnsupportedReason::Progressive
                    || reason == stego::JpegUnsupportedReason::MultipleScans
            );
        }
        JpegSupport::Supported => {
            panic!("Progressive JPEG should be unsupported");
        }
    }
}

#[test]
fn public_jpeg_capacity_preflight() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);
    let config = JpegConfig::new(42);

    let report = jpeg::capacity(&jpeg_bytes, 10, &config).unwrap();
    assert!(report.available > 0);
}

#[test]
fn public_frame_roundtrip_binary_payload() {
    let payload: Vec<u8> = (0..=255).cycle().take(1000).collect();
    let framed = frame::encode(&payload).unwrap();

    assert_eq!(&framed[0..2], &FRAMED_MAGIC);
    assert_eq!(framed[2], FRAME_VERSION);

    let (header, decoded) = frame::decode(&framed).unwrap();
    assert_eq!(header.version, FRAME_VERSION);
    assert_eq!(header.payload_len, payload.len());
    assert_eq!(decoded, payload);
}

#[test]
fn public_frame_checksum_detects_corruption() {
    let payload = b"test payload for CRC";
    let mut framed = frame::encode(payload).unwrap();

    framed[FRAME_HEADER_SIZE + 2] ^= 0xFF;

    let result = frame::decode(&framed);
    assert!(matches!(result, Err(StegoError::FrameChecksumMismatch)));
}

#[test]
fn public_frame_malformed_length_fails_before_large_allocation() {
    let mut data = vec![0u8; FRAME_HEADER_SIZE + 10];
    data[0..2].copy_from_slice(&FRAMED_MAGIC);
    data[2] = FRAME_VERSION;
    data[3..7].copy_from_slice(&(u32::MAX).to_le_bytes());

    let result = frame::decode(&data);
    assert!(matches!(result, Err(StegoError::MalformedFrame(_))));
}

#[test]
fn public_frame_trailing_bytes_rejected() {
    let payload = b"trailing test";
    let mut framed = frame::encode(payload).unwrap();
    framed.extend_from_slice(&[0xFF, 0xFE, 0xFD]);

    let result = frame::decode(&framed);
    assert!(
        matches!(result, Err(StegoError::MalformedFrame(ref msg)) if msg.contains("trailing bytes"))
    );
}

#[test]
fn public_frame_prefix_determines_total_length() {
    let payload = vec![42u8; 500];
    let framed = frame::encode(&payload).unwrap();

    let (header, total) = frame::decode_prefix(&framed).unwrap();
    assert_eq!(header.payload_len, 500);
    assert_eq!(total, FRAME_HEADER_SIZE + 500);
}

#[test]
fn public_lsb_framed_roundtrip() {
    let img = make_lsb_image(64, 64);
    let payload = b"framed lsb payload";
    let config = LsbConfig::new(42);

    let report = lsb::embed_framed(&img, payload, &config).unwrap();
    assert!(report.embedded);
    assert_eq!(
        report.payload_bytes,
        frame::FRAME_HEADER_SIZE + payload.len()
    );

    let recovered = lsb::extract_framed(&report.output, &config).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_lsb_framed_empty_payload() {
    let img = make_lsb_image(64, 64);
    let config = LsbConfig::new(42);

    let report = lsb::embed_framed(&img, b"", &config).unwrap();
    assert_eq!(report.payload_bytes, FRAME_HEADER_SIZE);
    assert!(lsb::extract_framed(&report.output, &config)
        .unwrap()
        .is_empty());
}

#[test]
fn public_lsb_framed_capacity_includes_frame_overhead() {
    let img = make_lsb_image(40, 4);
    let config = LsbConfig::new(42).with_redundancy(1);

    let exact = lsb::embed_framed(&img, b"x", &config).unwrap();
    assert!(exact.embedded);
    assert_eq!(exact.required_capacity, exact.available_capacity);

    let too_large = lsb::embed_framed(&img, b"xy", &config).unwrap();
    assert!(!too_large.embedded);
    assert!(too_large.required_capacity > too_large.available_capacity);
}

#[test]
fn public_lsb_framed_wrong_seed_returns_frame_error() {
    let img = make_lsb_image(64, 64);
    let report = lsb::embed_framed(&img, b"framed payload", &LsbConfig::new(42)).unwrap();

    let result = lsb::extract_framed(&report.output, &LsbConfig::new(43));
    assert!(matches!(
        result,
        Err(StegoError::FrameNotFound)
            | Err(StegoError::MalformedFrame(_))
            | Err(StegoError::FrameChecksumMismatch)
            | Err(StegoError::MalformedInput(_))
    ));
}

#[test]
fn public_jpeg_framed_roundtrip() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);
    let payload = b"framed jpeg payload";
    let config = JpegConfig::new(42);

    let report = jpeg::embed_framed(&jpeg_bytes, payload, &config).unwrap();
    assert!(report.embedded);
    assert_eq!(
        report.payload_bytes,
        frame::FRAME_HEADER_SIZE + payload.len()
    );

    let recovered = jpeg::extract_framed(&report.output, &config).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_jpeg_framed_extracts_after_capacity_downgrade() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);
    let requested = JpegConfig::new(42).with_redundancy(3);
    let available = jpeg::capacity(&jpeg_bytes, 1, &requested)
        .unwrap()
        .available;
    let framed_len = available / 16;
    assert!(framed_len > FRAME_HEADER_SIZE);
    let payload = vec![0xA5; framed_len - FRAME_HEADER_SIZE];

    let report = jpeg::embed_framed(&jpeg_bytes, &payload, &requested).unwrap();
    assert!(report.embedded);
    assert!(report.actual_redundancy < requested.redundancy());

    let recovered = jpeg::extract_framed(&report.output, &requested).unwrap();
    assert_eq!(recovered, payload);
}

#[test]
fn public_jpeg_framed_wrong_seed_fails() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);
    let config = JpegConfig::new(42);
    let report = jpeg::embed_framed(&jpeg_bytes, b"framed jpeg payload", &config).unwrap();

    let result = jpeg::extract_framed(&report.output, &JpegConfig::new(43));
    assert!(result.is_err());
}

#[test]
fn public_generic_api_does_not_emit_rights_metadata() {
    let img = make_lsb_image(64, 64);
    let payload = b"no rights metadata";
    let config = LsbConfig::new(42);

    let report = lsb::embed(&img, payload, &config).unwrap();
    let output_str = String::from_utf8_lossy(&report.output);

    assert!(!output_str.contains("plus:DataMining"));
    assert!(!output_str.contains("Rights"));
    assert!(!output_str.contains("Copyright"));
}

#[test]
fn public_frame_empty_payload() {
    let framed = frame::encode(b"").unwrap();
    let (header, decoded) = frame::decode(&framed).unwrap();
    assert_eq!(header.payload_len, 0);
    assert!(decoded.is_empty());
}

#[test]
fn public_frame_rejects_over_max_payload() {
    let payload = vec![0u8; stego::frame::MAX_FRAME_PAYLOAD + 1];
    let result = lsb::embed_framed(&make_lsb_image(64, 64), &payload, &LsbConfig::new(42));
    assert!(matches!(result, Err(StegoError::InvalidConfig(_))));
}

#[test]
fn public_lsb_framed_rejects_oversized_declared_length_before_full_extract() {
    let img = make_lsb_image(64, 64);
    let config = LsbConfig::new(42);
    let mut prefix = vec![0u8; FRAME_HEADER_SIZE];
    prefix[0..2].copy_from_slice(&FRAMED_MAGIC);
    prefix[2] = FRAME_VERSION;
    prefix[3..7].copy_from_slice(&(u32::MAX).to_le_bytes());

    let report = lsb::embed(&img, &prefix, &config).unwrap();
    let result = lsb::extract_framed(&report.output, &config);
    assert!(matches!(result, Err(StegoError::MalformedFrame(_))));
}

#[test]
fn public_lsb_framed_rejects_declared_frame_beyond_carrier_capacity() {
    let img = make_lsb_image(40, 4);
    let config = LsbConfig::new(42).with_redundancy(1);
    let mut prefix = vec![0u8; FRAME_HEADER_SIZE];
    prefix[0..2].copy_from_slice(&FRAMED_MAGIC);
    prefix[2] = FRAME_VERSION;
    prefix[3..7].copy_from_slice(&2u32.to_le_bytes());

    let report = lsb::embed(&img, &prefix, &config).unwrap();
    let result = lsb::extract_framed(&report.output, &config);
    assert!(matches!(
        result,
        Err(StegoError::InsufficientCapacity { .. })
    ));
}

#[test]
fn public_lsb_tiled_config() {
    let img = make_lsb_image(128, 128);
    let payload = b"tiled test";
    let config = LsbConfig::new(42);

    let report = lsb::embed(&img, payload, &config).unwrap();
    assert!(report.embedded);

    let decoded = report.output.clone();
    let recovered = lsb::extract(&decoded, payload.len(), &config).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_lsb_tiled_raw_roundtrip_via_root_reexport() {
    let img = make_lsb_image(128, 128);
    let payload = b"root tiled lsb";
    let config = stego::TileConfig::try_new(42, 64).unwrap();

    let report = lsb::embed_tiled(&img, payload, &config).unwrap();
    assert!(report.embedded);
    let recovered = lsb::extract_tiled(&report.output, payload.len(), &config, 64).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_lsb_tiled_in_place_roundtrip_via_root_reexport() {
    let mut img = make_lsb_image(128, 128);
    let payload = b"root tiled in-place";
    let config = stego::TileConfig::try_new(7, 64).unwrap();

    let report = lsb::embed_tiled_in_place(&mut img, payload, &config).unwrap();
    assert!(report.embedded);
    let recovered = lsb::extract_tiled(&img, payload.len(), &config, 64).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_lsb_tiled_framed_recovers_without_length() {
    let img = make_lsb_image(128, 128);
    let payload = b"root framed tiled";
    let config = stego::TileConfig::try_new(42, 64).unwrap();

    let report = lsb::embed_tiled_framed(&img, payload, &config).unwrap();
    assert!(report.embedded);
    let recovered = lsb::extract_tiled_framed(&report.output, &config, 64).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_lsb_tiled_extraction_rejects_zero_origins() {
    let img = make_lsb_image(128, 128);
    let config = stego::TileConfig::try_new(42, 64).unwrap();
    assert!(lsb::extract_tiled(&img, 4, &config, 0).is_err());
    assert!(lsb::extract_tiled_framed(&img, &config, 0).is_err());
}

#[test]
fn public_jpeg_tiled_raw_roundtrip_via_root_reexport() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);
    let payload = b"root tiled jpeg";
    let config = stego::TileConfig::try_new(42, 64).unwrap();

    let report = jpeg::embed_tiled(&jpeg_bytes, payload, &config).unwrap();
    assert!(report.embedded);
    let recovered = jpeg::extract_tiled(&report.output, payload.len(), &config, 64).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_jpeg_tiled_framed_recovers_without_length() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);
    let payload = b"root framed tiled jpeg";
    let config = stego::TileConfig::try_new(42, 64).unwrap();

    let report = jpeg::embed_tiled_framed(&jpeg_bytes, payload, &config).unwrap();
    assert!(report.embedded);
    let recovered = jpeg::extract_tiled_framed(&report.output, &config, 64).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn public_tile_config_rejects_zero_size() {
    assert!(stego::TileConfig::try_new(42, 0).is_err());
    assert!(stego::TileConfig::try_new(42, 64).is_ok());
}

#[test]
fn public_jpeg_probe_support_baseline() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);
    let support = jpeg::probe_support(&jpeg_bytes).unwrap();
    assert_eq!(support, JpegSupport::Supported);
}

#[test]
fn public_jpeg_probe_support_not_jpeg() {
    let result = jpeg::probe_support(&[0x89, 0x50, 0x4E, 0x47]);
    assert!(result.is_err());
}

#[test]
fn public_lsb_zero_image_returns_empty() {
    let img = RgbaImage::new(0, 0);
    let config = LsbConfig::new(42);
    let result = lsb::embed(&img, b"test", &config);
    assert!(matches!(result, Err(StegoError::EmptyCarrier)));
}

#[test]
fn public_stego_error_display() {
    let err = StegoError::InsufficientCapacity {
        required: 100,
        available: 50,
    };
    let msg = err.to_string();
    assert!(msg.contains("100"));
    assert!(msg.contains("50"));
}

#[test]
fn public_stego_error_into_crate_error() {
    let err = StegoError::FrameChecksumMismatch;
    let crate_err: stegoeggo::Error = err.into();
    assert!(crate_err.to_string().contains("checksum mismatch"));
}

#[test]
fn public_lsb_config_try_new_rejects_untrusted_redundancy() {
    let user_redundancy: usize = 99;
    let result = LsbConfig::try_new(42, user_redundancy);
    assert!(matches!(result, Err(StegoError::InvalidConfig(_))));

    let config = LsbConfig::try_new(42, 5).unwrap();
    assert_eq!(config.redundancy(), 5);
}

#[test]
fn public_lsb_config_try_with_redundancy_rejects_untrusted_redundancy() {
    let result = LsbConfig::new(42).try_with_redundancy(50);
    assert!(matches!(result, Err(StegoError::InvalidConfig(_))));

    let config = LsbConfig::new(42).try_with_redundancy(4).unwrap();
    assert_eq!(config.redundancy(), 4);
}

#[test]
fn public_jpeg_config_try_new_rejects_untrusted_redundancy() {
    let result = JpegConfig::try_new(42, 0);
    assert!(matches!(result, Err(StegoError::InvalidConfig(_))));

    let config = JpegConfig::try_new(42, 6).unwrap();
    assert_eq!(config.redundancy(), 6);
}

#[test]
fn public_jpeg_config_try_with_redundancy_rejects_untrusted_redundancy() {
    let result = JpegConfig::new(42).try_with_redundancy(11);
    assert!(matches!(result, Err(StegoError::InvalidConfig(_))));

    let config = JpegConfig::new(42).try_with_redundancy(2).unwrap();
    assert_eq!(config.redundancy(), 2);
}

#[test]
fn public_jpeg_raw_inputs_reject_overflow_and_invalid_redundancy() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);
    let config = JpegConfig::new(42);

    let capacity_result =
        std::panic::catch_unwind(|| jpeg::capacity(&jpeg_bytes, usize::MAX, &config));
    assert!(capacity_result.is_ok());
    assert!(matches!(
        capacity_result.unwrap(),
        Err(StegoError::InvalidConfig(_))
    ));

    let extract_result =
        std::panic::catch_unwind(|| jpeg::extract(&jpeg_bytes, usize::MAX, &config, 1));
    assert!(extract_result.is_ok());
    assert!(matches!(
        extract_result.unwrap(),
        Err(StegoError::InvalidConfig(_))
    ));

    for redundancy in [0, 11, usize::MAX] {
        let result = jpeg::extract(&jpeg_bytes, 1, &config, redundancy);
        assert!(matches!(result, Err(StegoError::InvalidConfig(_))));
    }
}

#[test]
fn public_jpeg_raw_redundancy_one_and_ten_remain_valid() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);

    for redundancy in [1, 10] {
        let config = JpegConfig::new(42).with_redundancy(redundancy);
        let report = jpeg::embed(&jpeg_bytes, b"raw", &config).unwrap();
        assert!(report.embedded);
        assert_eq!(report.actual_redundancy, redundancy);
        let recovered =
            jpeg::extract(&report.output, 3, &config, report.actual_redundancy).unwrap();
        assert_eq!(&recovered, b"raw");
    }
}
