//! Direct third-party consumer fixture for `stegoeggo-stego`.
//!
//! Compiles and runs against the default feature set only: no
//! `application-support`, no parent rights-protection crate. Covers the
//! recommended generic surface: validated configuration, LSB raw/framed/
//! in-place/tiled operations, packed/strided pixel views, JPEG support
//! probing, strict and best-effort JPEG embedding, framed/tiled JPEG
//! extraction, opaque prepared-JPEG reuse, and the transactional seed hint
//! including its insufficient-hint failure.

use stegoeggo_stego::{
    frame, jpeg,
    jpeg::JpegConfig,
    lsb,
    lsb::LsbConfig,
    pixels::{PixelLayout, PixelView, PixelViewMut},
    prepared::PreparedJpeg,
    CapacityReport, EmbedReport, Redundancy, StegoError, TileConfig, MAX_TILED_ORIGINS,
};

fn textured_rgb(width: u32, height: u32) -> image::RgbImage {
    image::RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([
            ((x * 7 + y * 13) % 256) as u8,
            ((x * 11 + y * 3) % 256) as u8,
            ((x * 5 + y * 17) % 256) as u8,
        ])
    })
}

fn textured_rgba(width: u32, height: u32) -> image::RgbaImage {
    image::DynamicImage::ImageRgb8(textured_rgb(width, height)).to_rgba8()
}

fn encode_jpeg(rgb: &image::RgbImage, quality: u8) -> Vec<u8> {
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    image::DynamicImage::ImageRgb8(rgb.clone())
        .write_with_encoder(encoder)
        .unwrap();
    buf
}

fn encode_progressive_jpeg(rgb: &image::RgbImage) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut buf, 90);
    encoder.set_progressive(true);
    encoder
        .encode(
            rgb.as_raw(),
            u16::try_from(rgb.width()).unwrap(),
            u16::try_from(rgb.height()).unwrap(),
            jpeg_encoder::ColorType::Rgb,
        )
        .unwrap();
    buf
}

/// Rewrite every 8-bit quantization value in every DQT segment to 1,
/// leaving segment structure intact.
fn flatten_quantization_tables(jpeg_bytes: &[u8]) -> Vec<u8> {
    let mut output = jpeg_bytes.to_vec();
    let mut pos = 2;
    while pos + 4 <= output.len() {
        if output[pos] != 0xFF {
            pos += 1;
            continue;
        }
        let marker = output[pos + 1];
        if marker == 0xFF {
            pos += 1;
            continue;
        }
        if marker == 0xDA || marker == 0xD9 {
            break;
        }
        let segment_len = u16::from_be_bytes([output[pos + 2], output[pos + 3]]) as usize;
        if marker == 0xDB {
            let mut table_pos = pos + 4;
            let segment_end = pos + 2 + segment_len;
            while table_pos < segment_end {
                let precision = output[table_pos] >> 4;
                let value_bytes = if precision == 0 { 1 } else { 2 };
                output[table_pos + 1..table_pos + 1 + 64 * value_bytes].fill(1);
                table_pos += 1 + 64 * value_bytes;
            }
        }
        if marker == 0x00 {
            pos += 2;
            continue;
        }
        pos += 2 + segment_len;
    }
    output
}

#[test]
fn direct_consumer_lsb_raw_and_in_place_round_trip() {
    let image = textured_rgba(96, 96);
    let config = LsbConfig::from_redundancy(2024, Redundancy::new(2).unwrap());
    let payload = b"direct consumer lsb";

    let report: EmbedReport<image::RgbaImage> = lsb::embed(&image, payload, &config).unwrap();
    assert!(report.embedded);
    assert_eq!(report.capacity().required, report.required_capacity);
    assert_eq!(
        lsb::extract(&report.output, payload.len(), &config).unwrap(),
        payload
    );

    let mut owned = image.clone();
    let in_place = lsb::embed_in_place(&mut owned, payload, &config).unwrap();
    assert!(in_place.embedded);
    assert_eq!(owned, report.output);
}

#[test]
fn direct_consumer_lsb_framed_and_tiled_round_trip() {
    let image = textured_rgba(128, 128);
    let config = LsbConfig::new(77);
    let payload = b"direct consumer framed";

    let framed = lsb::embed_framed(&image, payload, &config).unwrap();
    assert!(framed.embedded);
    assert_eq!(
        lsb::extract_framed(&framed.output, &config).unwrap(),
        payload
    );

    let tile = TileConfig::try_new(77, 64).unwrap();
    let tiled = lsb::embed_tiled(&image, payload, &tile).unwrap();
    assert!(tiled.embedded);
    assert_eq!(
        lsb::extract_tiled(&tiled.output, payload.len(), &tile, 64).unwrap(),
        payload
    );
    let tiled_framed = lsb::embed_tiled_framed(&image, payload, &tile).unwrap();
    assert_eq!(
        lsb::extract_tiled_framed(&tiled_framed.output, &tile, MAX_TILED_ORIGINS).unwrap(),
        payload
    );
}

#[test]
fn direct_consumer_strided_views_match_rgba_path() {
    let (width, height) = (64u32, 48u32);
    let stride = (width as usize) * 3 + 24;
    let mut strided = vec![0x11u8; (height as usize) * stride];
    for y in 0..height {
        for x in 0..width {
            let base = (y as usize) * stride + (x as usize) * 3;
            strided[base] = ((x * 7 + y * 13) % 256) as u8;
            strided[base + 1] = ((x * 11 + y * 3) % 256) as u8;
            strided[base + 2] = ((x * 5 + y * 17) % 256) as u8;
        }
    }
    let config = LsbConfig::new(555);
    let payload = b"direct consumer strided";

    {
        let mut view =
            PixelViewMut::new(&mut strided, width, height, PixelLayout::Rgb8, stride).unwrap();
        let capacity: CapacityReport = view.capacity(payload.len(), &config).unwrap();
        assert!(capacity.is_sufficient());
        assert!(view.embed(payload, &config).unwrap().embedded);
        assert_eq!(
            view.as_view().extract(payload.len(), &config).unwrap(),
            payload
        );
    }
    assert!(strided
        .chunks(stride)
        .all(|row| row[(width as usize) * 3..].iter().all(|&byte| byte == 0x11)));

    let packed: Vec<u8> = strided
        .chunks(stride)
        .flat_map(|row| row[..(width as usize) * 3].to_vec())
        .collect();
    let packed_view = PixelView::new(
        &packed,
        width,
        height,
        PixelLayout::Rgb8,
        (width as usize) * 3,
    )
    .unwrap();
    assert_eq!(
        packed_view.extract(payload.len(), &config).unwrap(),
        payload
    );
}

#[test]
fn direct_consumer_jpeg_strict_and_best_effort() {
    let jpeg_bytes = encode_jpeg(&textured_rgb(256, 256), 90);
    assert_eq!(
        jpeg::probe_support(&jpeg_bytes).unwrap(),
        jpeg::JpegSupport::Supported
    );
    let config = JpegConfig::from_redundancy(31337, Redundancy::new(2).unwrap());
    let payload = b"direct consumer jpeg";

    let strict = jpeg::embed_strict(&jpeg_bytes, payload, &config).unwrap();
    assert!(strict.embedded);
    assert_eq!(strict.actual_redundancy, config.redundancy());
    assert_eq!(
        jpeg::extract(
            &strict.output,
            payload.len(),
            &config,
            strict.actual_redundancy
        )
        .unwrap(),
        payload
    );

    let framed = jpeg::embed_framed_strict(&jpeg_bytes, payload, &config).unwrap();
    assert_eq!(
        jpeg::extract_framed(&framed.output, &config).unwrap(),
        payload
    );

    let available = jpeg::capacity(&jpeg_bytes, 1, &config).unwrap().available;
    let oversized = vec![0xA5u8; available + 128];
    match jpeg::embed_strict(&jpeg_bytes, &oversized, &config) {
        Err(StegoError::InsufficientCapacity {
            required,
            available,
        }) => {
            assert!(required > available);
        }
        other => panic!("strict oversized embed must fail with capacity, got {other:?}"),
    }
    let best_effort = jpeg::embed(&jpeg_bytes, &oversized, &config).unwrap();
    assert!(!best_effort.embedded);
    assert_eq!(best_effort.actual_redundancy, 0);
}

#[test]
fn direct_consumer_jpeg_tiled_and_prepared_reuse() {
    let jpeg_bytes = encode_jpeg(&textured_rgb(256, 256), 90);
    let config = JpegConfig::new(909);
    let tile = TileConfig::try_new(909, 64).unwrap();
    let payload = b"direct consumer tiled jpeg";

    let tiled = jpeg::embed_tiled(&jpeg_bytes, payload, &tile).unwrap();
    assert!(tiled.embedded);
    assert_eq!(
        jpeg::extract_tiled(&tiled.output, payload.len(), &tile, 64).unwrap(),
        payload
    );

    let framed = jpeg::embed(&jpeg_bytes, &frame::encode(payload).unwrap(), &config).unwrap();
    assert!(framed.embedded);
    let prepared = PreparedJpeg::new(&framed.output).unwrap();
    assert_eq!(prepared.support(), jpeg::JpegSupport::Supported);
    assert_eq!(
        prepared.capacity(payload.len(), &config).unwrap(),
        jpeg::capacity(&framed.output, payload.len(), &config).unwrap()
    );
    assert_eq!(
        prepared
            .extract(
                payload.len() + frame::FRAME_HEADER_SIZE,
                &config,
                framed.actual_redundancy
            )
            .unwrap(),
        jpeg::extract(
            &framed.output,
            payload.len() + frame::FRAME_HEADER_SIZE,
            &config,
            framed.actual_redundancy
        )
        .unwrap()
    );
    assert_eq!(prepared.extract_framed(&config).unwrap(), payload);
    assert_eq!(
        prepared.extract_tiled_framed(&tile, 8).is_ok(),
        jpeg::extract_tiled_framed(&framed.output, &tile, 8).is_ok()
    );
}

#[test]
fn direct_consumer_seed_hint_is_transactional() {
    let jpeg_bytes = encode_jpeg(&textured_rgb(128, 128), 90);
    let hinted = jpeg::embed_seed_hint(&jpeg_bytes, 0x1234_5678).unwrap();
    assert_eq!(jpeg::extract_seed_hint(&hinted).unwrap(), Some(0x1234_5678));

    let flat = flatten_quantization_tables(&jpeg_bytes);
    match jpeg::embed_seed_hint(&flat, 42) {
        Err(StegoError::InsufficientCapacity {
            required,
            available,
        }) => {
            assert_eq!(required, 96);
            assert!(available < 96);
        }
        other => panic!("flattened tables must fail the hint preflight, got {other:?}"),
    }
    assert_eq!(jpeg::extract_seed_hint(&flat).unwrap(), None);
}

#[test]
fn direct_consumer_unsupported_jpeg_has_explicit_contract() {
    let progressive = encode_progressive_jpeg(&textured_rgb(64, 64));
    let support = jpeg::probe_support(&progressive).unwrap();
    assert!(matches!(support, jpeg::JpegSupport::Unsupported(_)));

    let config = JpegConfig::new(1);
    match jpeg::embed_strict(&progressive, b"payload", &config) {
        Err(StegoError::UnsupportedJpeg(_)) | Err(StegoError::MalformedInput(_)) => {}
        other => panic!("progressive strict embed must be explicit, got {other:?}"),
    }
    let prepared = PreparedJpeg::new(&progressive).unwrap();
    assert_eq!(prepared.support(), support);
    match prepared.capacity(4, &config) {
        Err(StegoError::UnsupportedJpeg(_)) => {}
        other => panic!("prepared reads on unsupported input must fail explicitly, got {other:?}"),
    }
}

#[test]
fn direct_consumer_errors_need_no_exhaustive_match() {
    let tiny = textured_rgba(8, 8);
    let config = LsbConfig::new(3);
    let payload = vec![0xA5u8; 1024];
    let report = lsb::embed(&tiny, &payload, &config).unwrap();
    assert!(!report.embedded);
    let outcome = if report.capacity().is_sufficient() {
        "fits"
    } else {
        match lsb::extract(&tiny, payload.len(), &config) {
            Err(StegoError::InsufficientCapacity { .. }) | Err(StegoError::MalformedInput(_)) => {
                "short"
            }
            Err(_) => "other",
            Ok(_) => "unexpected",
        }
    };
    assert_eq!(outcome, "short");

    match Redundancy::from_usize(usize::MAX) {
        Err(StegoError::InvalidConfig(_)) => {}
        other => panic!("invalid redundancy must be structured, got {other:?}"),
    }
    match PixelView::new(&[0u8; 8], 4, 4, PixelLayout::Rgb8, 4) {
        Err(StegoError::InvalidConfig(_)) => {}
        other => panic!("truncated view must be structured, got {other:?}"),
    }
    match frame::decode(b"too short") {
        Err(_) => {}
        Ok(_) => panic!("frame decode of garbage must fail"),
    }
}
