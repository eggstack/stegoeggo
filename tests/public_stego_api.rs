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
fn public_lsb_raw_roundtrip_arbitrary_bytes() {
    let img = make_lsb_image(64, 64);
    let payload = b"hello stegoeggo generic api";
    let config = LsbConfig::new(42);

    let report = lsb::embed(&img, payload, &config).unwrap();
    assert!(report.embedded);
    assert_eq!(report.payload_bytes, payload.len());

    let decoded = image::load_from_memory(&report.output).unwrap().to_rgba8();
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

    let decoded = image::load_from_memory(&report.output).unwrap().to_rgba8();
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

    let decoded = image::load_from_memory(&report.output).unwrap().to_rgba8();
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

    let decoded = image::load_from_memory(&report.output).unwrap().to_rgba8();
    let recovered = lsb::extract(&decoded, payload.len(), &config).unwrap();
    assert_eq!(recovered, payload);
}

#[test]
fn public_lsb_capacity_preflight() {
    let img = make_lsb_image(100, 100);
    let config = LsbConfig::new(42);

    let report = lsb::capacity(&img, 100, &config);
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

    let decoded_a = image::load_from_memory(&report_a.output)
        .unwrap()
        .to_rgba8();
    let decoded_b = image::load_from_memory(&report_b.output)
        .unwrap()
        .to_rgba8();

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

    let cap_r1 = lsb::capacity(&img, 100, &config_r1);
    let cap_r3 = lsb::capacity(&img, 100, &config_r3);

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

    let framed_payload = frame::encode(payload).unwrap();
    let report = lsb::embed(&img, &framed_payload, &config).unwrap();
    assert!(report.embedded);

    let decoded = image::load_from_memory(&report.output).unwrap().to_rgba8();

    let prefix_data = lsb::extract(&decoded, FRAME_HEADER_SIZE, &config).unwrap();
    let (_, total_len) = frame::decode_prefix(&prefix_data).unwrap();

    let full_data = lsb::extract(&decoded, total_len, &config).unwrap();
    let (header, recovered) = frame::decode(&full_data).unwrap();
    assert_eq!(header.payload_len, payload.len());
    assert_eq!(&recovered, payload);
}

#[test]
fn public_jpeg_framed_roundtrip() {
    let jpeg_bytes = make_jpeg_bytes(256, 256);
    let payload = b"framed jpeg payload";
    let config = JpegConfig::new(42);

    let framed_payload = frame::encode(payload).unwrap();

    let report = jpeg::embed(&jpeg_bytes, &framed_payload, &config).unwrap();
    assert!(report.embedded);

    let recovered = jpeg::extract(
        &report.output,
        framed_payload.len(),
        &config,
        report.actual_redundancy,
    )
    .unwrap();
    assert_eq!(recovered, framed_payload);

    let (header, final_payload) = frame::decode(&recovered).unwrap();
    assert_eq!(header.payload_len, payload.len());
    assert_eq!(&final_payload, payload);
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
    let result = frame::encode(&payload);
    assert!(matches!(result, Err(StegoError::InvalidConfig(_))));
}

#[test]
fn public_lsb_tiled_config() {
    let img = make_lsb_image(128, 128);
    let payload = b"tiled test";
    let config = LsbConfig::new(42);

    let report = lsb::embed(&img, payload, &config).unwrap();
    assert!(report.embedded);

    let decoded = image::load_from_memory(&report.output).unwrap().to_rgba8();
    let recovered = lsb::extract(&decoded, payload.len(), &config).unwrap();
    assert_eq!(&recovered, payload);
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
