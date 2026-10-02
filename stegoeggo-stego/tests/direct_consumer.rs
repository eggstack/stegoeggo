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
    CapacityReport, CarrierLimits, EmbedReport, Redundancy, StegoError, TileConfig,
    MAX_TILED_ORIGINS,
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
    let best_effort = jpeg::embed_best_effort(&jpeg_bytes, &oversized, &config).unwrap();
    assert!(!best_effort.embedded);
    assert!(!best_effort.is_embedded());
    assert_eq!(best_effort.actual_redundancy, 0);
    assert_eq!(best_effort.actual_redundancy(), 0);
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

    let framed = jpeg::embed_framed_best_effort(&jpeg_bytes, payload, &config).unwrap();
    assert!(framed.embedded);
    assert!(framed.is_embedded());
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
fn direct_consumer_best_effort_alias_parity() {
    let jpeg_bytes = encode_jpeg(&textured_rgb(256, 256), 90);
    let config = JpegConfig::new(4242);
    let payload = b"best-effort alias parity";

    let legacy = jpeg::embed(&jpeg_bytes, payload, &config).unwrap();
    let explicit = jpeg::embed_best_effort(&jpeg_bytes, payload, &config).unwrap();
    assert_eq!(legacy.embedded, explicit.embedded);
    assert_eq!(legacy.output, explicit.output);
    assert_eq!(legacy.payload_bytes, explicit.payload_bytes);
    assert_eq!(legacy.required_capacity, explicit.required_capacity);
    assert_eq!(legacy.available_capacity, explicit.available_capacity);
    assert_eq!(legacy.actual_redundancy, explicit.actual_redundancy);

    let legacy_framed = jpeg::embed_framed(&jpeg_bytes, payload, &config).unwrap();
    let explicit_framed = jpeg::embed_framed_best_effort(&jpeg_bytes, payload, &config).unwrap();
    assert_eq!(legacy_framed.output, explicit_framed.output);
    assert_eq!(
        legacy_framed.actual_redundancy,
        explicit_framed.actual_redundancy
    );

    let available = jpeg::capacity(&jpeg_bytes, 1, &config).unwrap().available;
    let framed_len = available / 16;
    assert!(framed_len > frame::FRAME_HEADER_SIZE);
    let downgrade_payload = vec![0xA5; framed_len - frame::FRAME_HEADER_SIZE];
    let requested = JpegConfig::new(4242).try_with_redundancy(3).unwrap();
    let legacy_downgrade = jpeg::embed_framed(&jpeg_bytes, &downgrade_payload, &requested).unwrap();
    let explicit_downgrade =
        jpeg::embed_framed_best_effort(&jpeg_bytes, &downgrade_payload, &requested).unwrap();
    assert!(legacy_downgrade.embedded);
    assert!(legacy_downgrade.actual_redundancy < requested.redundancy());
    assert_eq!(legacy_downgrade.output, explicit_downgrade.output);
    assert_eq!(
        jpeg::extract_framed(&explicit_downgrade.output, &requested).unwrap(),
        downgrade_payload
    );

    let oversized = vec![0xA5u8; available + 128];
    let legacy_seed_only = jpeg::embed(&jpeg_bytes, &oversized, &config).unwrap();
    let explicit_seed_only = jpeg::embed_best_effort(&jpeg_bytes, &oversized, &config).unwrap();
    assert!(!legacy_seed_only.embedded);
    assert!(!explicit_seed_only.embedded);
    assert_eq!(legacy_seed_only.output, explicit_seed_only.output);
    assert_eq!(legacy_seed_only.actual_redundancy, 0);
    assert_eq!(explicit_seed_only.actual_redundancy(), 0);
}

#[test]
fn direct_consumer_report_accessors_cover_stable_facts() {
    let image = textured_rgba(64, 64);
    let config = LsbConfig::new(99);
    let payload = b"report accessors";

    let report: EmbedReport<image::RgbaImage> = lsb::embed(&image, payload, &config).unwrap();
    assert_eq!(report.embedded(), report.embedded);
    assert_eq!(report.is_embedded(), report.embedded);
    assert_eq!(report.payload_bytes(), report.payload_bytes);
    assert_eq!(report.required_capacity(), report.required_capacity);
    assert_eq!(report.available_capacity(), report.available_capacity);
    assert_eq!(report.actual_redundancy(), report.actual_redundancy);
    assert_eq!(report.capacity().required, report.required_capacity);
    assert_eq!(report.output().width(), 64);
    assert_eq!(report.output().height(), 64);

    let mut owned = image.clone();
    let in_place = lsb::embed_in_place(&mut owned, payload, &config).unwrap();
    assert_eq!(in_place.embedded(), in_place.embedded);
    assert_eq!(in_place.is_embedded(), in_place.embedded);
    assert_eq!(in_place.payload_bytes(), in_place.payload_bytes);
    assert_eq!(in_place.required_capacity(), in_place.required_capacity);
    assert_eq!(in_place.available_capacity(), in_place.available_capacity);
    assert_eq!(in_place.actual_redundancy(), in_place.actual_redundancy);
    assert_eq!(in_place.capacity().required, in_place.required_capacity);

    let jpeg_bytes = encode_jpeg(&textured_rgb(128, 128), 90);
    let jpeg_config = JpegConfig::new(7);
    let jpeg_report = jpeg::embed_strict(&jpeg_bytes, payload, &jpeg_config).unwrap();
    assert!(jpeg_report.is_embedded());
    assert!(jpeg_report.embedded());
    assert_eq!(jpeg_report.payload_bytes(), payload.len());
    assert_eq!(jpeg_report.actual_redundancy(), jpeg_config.redundancy());
    assert!(jpeg_report.capacity().is_sufficient());
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

#[test]
fn direct_consumer_bounded_variants_match_one_shot_with_defaults() {
    let jpeg_bytes = encode_jpeg(&textured_rgb(256, 256), 90);
    let config = JpegConfig::new(777);
    let payload = b"bounded defaults parity";
    let limits = CarrierLimits::default();

    assert_eq!(
        jpeg::capacity_with_limits(&jpeg_bytes, payload.len(), &config, &limits).unwrap(),
        jpeg::capacity(&jpeg_bytes, payload.len(), &config).unwrap()
    );
    let strict = jpeg::embed_strict_with_limits(&jpeg_bytes, payload, &config, &limits).unwrap();
    let one_shot = jpeg::embed_strict(&jpeg_bytes, payload, &config).unwrap();
    assert_eq!(strict.output, one_shot.output);
    assert_eq!(
        jpeg::extract_with_limits(
            &strict.output,
            payload.len(),
            &config,
            strict.actual_redundancy,
            &limits
        )
        .unwrap(),
        payload
    );
    let framed =
        jpeg::embed_framed_strict_with_limits(&jpeg_bytes, payload, &config, &limits).unwrap();
    assert_eq!(
        jpeg::extract_framed_with_limits(&framed.output, &config, &limits).unwrap(),
        payload
    );
    assert_eq!(
        jpeg::probe_support_with_limits(&jpeg_bytes, &limits).unwrap(),
        jpeg::probe_support(&jpeg_bytes).unwrap()
    );
    assert_eq!(
        jpeg::inspect_with_limits(&jpeg_bytes, &limits)
            .unwrap()
            .width,
        jpeg::inspect(
            &jpeg_bytes,
            limits.max_jpeg_segments(),
            limits.max_jpeg_segment_bytes()
        )
        .unwrap()
        .width
    );

    let tile = TileConfig::try_new(777, 64).unwrap();
    let tiled = jpeg::embed_tiled_with_limits(&jpeg_bytes, payload, &tile, &limits).unwrap();
    assert_eq!(
        tiled.output,
        jpeg::embed_tiled(&jpeg_bytes, payload, &tile)
            .unwrap()
            .output
    );
    assert_eq!(
        jpeg::extract_tiled_with_limits(&tiled.output, payload.len(), &tile, 64, &limits).unwrap(),
        payload
    );
    let tiled_framed =
        jpeg::embed_tiled_framed_with_limits(&jpeg_bytes, payload, &tile, &limits).unwrap();
    assert_eq!(
        jpeg::extract_tiled_framed_with_limits(&tiled_framed.output, &tile, 64, &limits).unwrap(),
        payload
    );

    let best_effort =
        jpeg::embed_best_effort_with_limits(&jpeg_bytes, payload, &config, &limits).unwrap();
    assert_eq!(
        best_effort.output,
        jpeg::embed_best_effort(&jpeg_bytes, payload, &config)
            .unwrap()
            .output
    );
    let framed_best =
        jpeg::embed_framed_best_effort_with_limits(&jpeg_bytes, payload, &config, &limits).unwrap();
    assert_eq!(
        framed_best.output,
        jpeg::embed_framed_best_effort(&jpeg_bytes, payload, &config)
            .unwrap()
            .output
    );
}

#[test]
fn direct_consumer_carrier_limits_bound_adversarial_inputs() {
    let jpeg_bytes = encode_jpeg(&textured_rgb(256, 256), 90);
    let config = JpegConfig::new(999);
    let tile = TileConfig::try_new(999, 64).unwrap();

    let tiny_input = CarrierLimits::builder().max_input_bytes(8).build();
    assert!(matches!(
        jpeg::capacity_with_limits(&jpeg_bytes, 4, &config, &tiny_input),
        Err(StegoError::ResourceLimitExceeded(_))
    ));
    assert!(matches!(
        PreparedJpeg::new_with_limits(&jpeg_bytes, &tiny_input),
        Err(StegoError::ResourceLimitExceeded(_))
    ));

    let tight_segments = CarrierLimits::builder().max_jpeg_segments(1).build();
    assert!(matches!(
        jpeg::capacity_with_limits(&jpeg_bytes, 4, &config, &tight_segments),
        Err(StegoError::ResourceLimitExceeded(_))
    ));
    assert!(matches!(
        jpeg::probe_support_with_limits(&jpeg_bytes, &tight_segments),
        Err(StegoError::ResourceLimitExceeded(_))
    ));

    let tight_dims = CarrierLimits::builder()
        .max_width(8)
        .max_height(8)
        .max_pixels(64)
        .build();
    assert!(matches!(
        jpeg::capacity_with_limits(&jpeg_bytes, 4, &config, &tight_dims),
        Err(StegoError::ResourceLimitExceeded(_))
    ));

    let tight_frame = CarrierLimits::builder().max_frame_bytes(12).build();
    let framed = jpeg::embed_framed_strict(&jpeg_bytes, b"frame bound payload", &config).unwrap();
    assert!(matches!(
        jpeg::extract_framed_with_limits(&framed.output, &config, &tight_frame),
        Err(StegoError::ResourceLimitExceeded(_))
    ));

    let tight_origins = CarrierLimits::builder().max_tiled_origins(2).build();
    assert!(matches!(
        jpeg::extract_tiled_with_limits(&jpeg_bytes, 4, &tile, 64, &tight_origins),
        Err(StegoError::ResourceLimitExceeded(_))
    ));
    assert!(matches!(
        jpeg::extract_tiled_framed_with_limits(&jpeg_bytes, &tile, 64, &tight_origins),
        Err(StegoError::ResourceLimitExceeded(_))
    ));
    assert!(matches!(
        jpeg::extract_tiled(&jpeg_bytes, 4, &tile, 0),
        Err(StegoError::InvalidConfig(_))
    ));
    assert!(matches!(
        jpeg::extract_tiled(&jpeg_bytes, 4, &tile, MAX_TILED_ORIGINS + 1),
        Err(StegoError::InvalidConfig(_))
    ));

    assert!(matches!(
        jpeg::capacity_with_limits(&jpeg_bytes, usize::MAX, &config, &CarrierLimits::default()),
        Err(StegoError::InvalidConfig(_))
    ));
    assert!(matches!(
        PreparedJpeg::new(b"not a jpeg"),
        Err(StegoError::MalformedInput(_))
    ));
    assert!(matches!(
        PreparedJpeg::new(&jpeg_bytes[..64]),
        Err(StegoError::MalformedInput(_)) | Err(StegoError::UnsupportedJpeg(_))
    ));
    assert!(matches!(
        jpeg::capacity(&jpeg_bytes[..64], 4, &config),
        Err(StegoError::MalformedInput(_)) | Err(StegoError::UnsupportedJpeg(_))
    ));
    assert!(matches!(
        jpeg::extract_tiled_framed(&jpeg_bytes, &tile, 0),
        Err(StegoError::InvalidConfig(_))
    ));
}

#[test]
fn direct_consumer_prepared_tiled_embed_matches_one_shot_and_reuses() {
    let jpeg_bytes = encode_jpeg(&textured_rgb(256, 256), 90);
    let tile = TileConfig::try_new(31337, 64).unwrap();
    let payload = b"prepared tiled embed parity";

    let prepared = PreparedJpeg::new(&jpeg_bytes).unwrap();
    let prepared_report = prepared.embed_tiled(payload, &tile).unwrap();
    let one_shot = jpeg::embed_tiled(&jpeg_bytes, payload, &tile).unwrap();
    assert_eq!(prepared_report.embedded, one_shot.embedded);
    assert_eq!(prepared_report.output, one_shot.output);
    assert_eq!(prepared_report.actual_redundancy(), 1);

    let prepared_framed = prepared.embed_tiled_framed(payload, &tile).unwrap();
    let one_shot_framed = jpeg::embed_tiled_framed(&jpeg_bytes, payload, &tile).unwrap();
    assert_eq!(prepared_framed.output, one_shot_framed.output);
    assert_eq!(
        jpeg::extract_tiled_framed(&prepared_framed.output, &tile, 64).unwrap(),
        payload
    );

    let small = encode_jpeg(&textured_rgb(64, 64), 90);
    let small_prepared = PreparedJpeg::new(&small).unwrap();
    let oversized = vec![0xA5; 4096];
    let before = small_prepared.capacity(4, &JpegConfig::new(1)).unwrap();
    let attempt = small_prepared.embed_tiled(&oversized, &tile).unwrap();
    assert_eq!(attempt.payload_bytes(), oversized.len());
    assert_eq!(
        small_prepared.capacity(4, &JpegConfig::new(1)).unwrap(),
        before
    );
    assert_eq!(
        small_prepared.extract_tiled_framed(&tile, 1).is_ok(),
        jpeg::extract_tiled_framed(&small, &tile, 1).is_ok()
    );

    let limits = CarrierLimits::builder().max_input_bytes(8).build();
    assert!(matches!(
        PreparedJpeg::new_with_limits(&jpeg_bytes, &limits),
        Err(StegoError::ResourceLimitExceeded(_))
    ));
}
