//! Direct third-party consumer fixture for the optional `webp` feature.
//!
//! Compiles only with `webp` enabled and covers the still-lossless
//! byte-carrier facade end to end: RGB/RGBA round trips, alpha
//! preservation, raw/framed/tiled extraction, insufficient capacity,
//! lossy/animated rejection, malformed-container handling, limit
//! exhaustion, VP8L output shape, extended-container acceptance, and the
//! documented metadata-drop contract.

#![cfg(feature = "webp")]

use std::io::Cursor;

use image::codecs::webp::{WebPDecoder, WebPEncoder};
use image::{ExtendedColorType, ImageDecoder, ImageEncoder, Rgba, RgbaImage};
use stegoeggo_stego::lsb::LsbConfig;
use stegoeggo_stego::webp::{self, WebpSupport};
use stegoeggo_stego::{CarrierLimits, StegoError, TileConfig, WebpUnsupportedReason};

fn textured_rgba(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, y| {
        Rgba([
            ((x * 7 + y * 13) % 251) as u8,
            ((x * 11 + y * 3) % 251) as u8,
            ((x * 5 + y * 17) % 251) as u8,
            ((x + y * 2) % 256) as u8,
        ])
    })
}

fn encode_lossless(image: &RgbaImage) -> Vec<u8> {
    let mut buf = Vec::new();
    WebPEncoder::new_lossless(&mut buf)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .unwrap();
    buf
}

fn decode_pixels(bytes: &[u8]) -> RgbaImage {
    let decoder = WebPDecoder::new(Cursor::new(bytes)).unwrap();
    assert!(!decoder.has_animation());
    let (width, height) = decoder.dimensions();
    let total = usize::try_from(decoder.total_bytes()).unwrap();
    let mut buf = vec![0u8; total];
    decoder.read_image(&mut buf).unwrap();
    RgbaImage::from_raw(width, height, buf).unwrap()
}

fn lossy_fixture() -> Vec<u8> {
    let mut out = Vec::from(&b"RIFF"[..]);
    let chunks = [
        b"VP8 ".as_slice(),
        &u32::to_le_bytes(3),
        &[0x9D, 0x01, 0x2A],
    ]
    .concat();
    out.extend_from_slice(&u32::try_from(chunks.len() + 4).unwrap().to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(&chunks);
    out
}

fn chunk(fourcc: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::from(&fourcc[..]);
    out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
    out
}

fn vp8x_fixture(flags: u8, siblings: &[u8]) -> Vec<u8> {
    let header = [flags, 0, 0, 0, 127, 0, 0, 127, 0, 0];
    let mut chunks = chunk(b"VP8X", &header);
    chunks.extend_from_slice(siblings);
    let mut out = Vec::from(&b"RIFF"[..]);
    out.extend_from_slice(&u32::try_from(chunks.len() + 4).unwrap().to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(&chunks);
    out
}

fn limits() -> CarrierLimits {
    CarrierLimits::default()
}

#[test]
fn still_lossless_raw_round_trip() {
    let file = encode_lossless(&textured_rgba(128, 128));
    let config = LsbConfig::new(42);
    let payload = vec![0xA5u8; 36];
    let report = webp::embed(&file, &payload, &config, &limits()).unwrap();
    assert!(report.embedded);
    match webp::probe_support(&report.output, &limits()).unwrap() {
        WebpSupport::StillLossless { width, height, .. } => {
            assert_eq!((width, height), (128, 128));
        }
    }
    let recovered = webp::extract(&report.output, payload.len(), &config, &limits()).unwrap();
    assert_eq!(recovered, payload);
}

#[test]
fn redundancy_round_trip_uses_report_redundancy() {
    let file = encode_lossless(&textured_rgba(128, 128));
    let config = LsbConfig::new(7).with_redundancy(3);
    let payload = vec![0x3Cu8; 24];
    let report = webp::embed(&file, &payload, &config, &limits()).unwrap();
    assert!(report.embedded);
    assert_eq!(report.actual_redundancy, 3);
    let extract_config = LsbConfig::new(7).with_redundancy(report.actual_redundancy);
    let recovered =
        webp::extract(&report.output, payload.len(), &extract_config, &limits()).unwrap();
    assert_eq!(recovered, payload);
}

#[test]
fn rgb_source_without_alpha_round_trips() {
    let rgb = image::RgbImage::from_fn(96, 96, |x, y| {
        image::Rgb([((x * 3) % 251) as u8, ((y * 5) % 251) as u8, 200])
    });
    let mut file = Vec::new();
    WebPEncoder::new_lossless(&mut file)
        .write_image(rgb.as_raw(), 96, 96, ExtendedColorType::Rgb8)
        .unwrap();
    match webp::probe_support(&file, &limits()).unwrap() {
        WebpSupport::StillLossless { has_alpha, .. } => assert!(!has_alpha),
    }
    let config = LsbConfig::new(42);
    let payload = b"rgb still lossless";
    let report = webp::embed(&file, payload, &config, &limits()).unwrap();
    assert!(report.embedded);
    let recovered = webp::extract(&report.output, payload.len(), &config, &limits()).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn alpha_channel_survives_embed() {
    let source = textured_rgba(64, 64);
    let file = encode_lossless(&source);
    let config = LsbConfig::new(42);
    let report = webp::embed(&file, b"alpha check", &config, &limits()).unwrap();
    assert!(report.embedded);
    let decoded = decode_pixels(&report.output);
    assert_eq!(decoded.dimensions(), source.dimensions());
    for (before, after) in source.pixels().zip(decoded.pixels()) {
        assert_eq!(before[3], after[3]);
    }
}

#[test]
fn framed_round_trip_needs_no_length() {
    let file = encode_lossless(&textured_rgba(128, 128));
    let config = LsbConfig::new(42);
    let payload = b"framed webp recovery";
    let report = webp::embed_framed(&file, payload, &config, &limits()).unwrap();
    assert!(report.embedded);
    let recovered = webp::extract_framed(&report.output, &config, &limits()).unwrap();
    assert_eq!(&recovered, payload);
}

#[test]
fn tiled_raw_and_framed_round_trip() {
    let file = encode_lossless(&textured_rgba(128, 128));
    let tile = TileConfig::try_new(42, 64).unwrap();
    let payload = vec![0x5Au8; 36];
    let report = webp::embed_tiled(&file, &payload, &tile, &limits()).unwrap();
    assert!(report.embedded);
    let recovered =
        webp::extract_tiled(&report.output, payload.len(), &tile, 64, &limits()).unwrap();
    assert_eq!(recovered, payload);
    let framed = webp::embed_tiled_framed(&file, &payload, &tile, &limits()).unwrap();
    assert!(framed.embedded);
    let recovered_framed =
        webp::extract_tiled_framed(&framed.output, &tile, 64, &limits()).unwrap();
    assert_eq!(recovered_framed, payload);
}

#[test]
fn insufficient_capacity_reports_units() {
    let file = encode_lossless(&textured_rgba(16, 16));
    let config = LsbConfig::new(42);
    let report = webp::embed(&file, &vec![0xA5u8; 256], &config, &limits()).unwrap();
    assert!(!report.embedded);
    assert!(report.required_capacity > report.available_capacity);
    assert!(!webp::capacity(&file, 256, &config, &limits())
        .unwrap()
        .is_sufficient());
}

#[test]
fn lossy_vp8_rejected_everywhere() {
    let file = lossy_fixture();
    let config = LsbConfig::new(42);
    let is_lossy = |result: Result<(), StegoError>| {
        matches!(
            result,
            Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::LossyVp8))
        )
    };
    assert!(is_lossy(webp::probe_support(&file, &limits()).map(|_| ())));
    assert!(is_lossy(
        webp::capacity(&file, 4, &config, &limits()).map(|_| ())
    ));
    assert!(is_lossy(
        webp::embed(&file, b"data", &config, &limits()).map(|_| ())
    ));
    assert!(is_lossy(
        webp::extract(&file, 4, &config, &limits()).map(|_| ())
    ));
    assert!(is_lossy(
        webp::extract_framed(&file, &config, &limits()).map(|_| ())
    ));
}

#[test]
fn animated_vp8x_rejected() {
    let file = vp8x_fixture(0x02, &[]);
    let config = LsbConfig::new(42);
    assert!(matches!(
        webp::probe_support(&file, &limits()),
        Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::Animated))
    ));
    assert!(matches!(
        webp::embed(&file, b"data", &config, &limits()),
        Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::Animated))
    ));
}

#[test]
fn malformed_containers_rejected() {
    let file = encode_lossless(&textured_rgba(64, 64));
    let config = LsbConfig::new(42);
    let mut overrun = file.clone();
    overrun[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut resized = Vec::from(&b"RIFF"[..]);
    resized.extend_from_slice(&u32::try_from(overrun.len() - 8).unwrap().to_le_bytes());
    resized.extend_from_slice(&overrun[8..]);
    for input in [
        Vec::new(),
        b"definitely not a webp file".to_vec(),
        file[..24].to_vec(),
        resized,
    ] {
        assert!(
            matches!(
                webp::probe_support(&input, &limits()),
                Err(StegoError::MalformedInput(_))
            ),
            "probe accepted malformed input of len {}",
            input.len()
        );
    }
    assert!(matches!(
        webp::embed(&file[..24], b"data", &config, &limits()),
        Err(StegoError::MalformedInput(_))
    ));
}

#[test]
fn limit_exhaustion_maps_to_resource_limit() {
    let file = encode_lossless(&textured_rgba(128, 128));
    let config = LsbConfig::new(42);
    let tiny_input = CarrierLimits::builder().max_input_bytes(16).build();
    assert!(matches!(
        webp::embed(&file, b"data", &config, &tiny_input),
        Err(StegoError::ResourceLimitExceeded(_))
    ));
    let tiny_dims = CarrierLimits::builder().max_width(64).build();
    assert!(matches!(
        webp::probe_support(&file, &tiny_dims),
        Err(StegoError::ResourceLimitExceeded(_))
    ));
    assert!(matches!(
        webp::embed(&file, b"data", &config, &tiny_dims),
        Err(StegoError::ResourceLimitExceeded(_))
    ));
}

#[test]
fn output_is_still_vp8l() {
    let file = encode_lossless(&textured_rgba(64, 64));
    let config = LsbConfig::new(42);
    let report = webp::embed(&file, b"vp8l check", &config, &limits()).unwrap();
    assert!(report.embedded);
    assert!(report.output.len() >= 16);
    assert_eq!(&report.output[0..4], b"RIFF");
    assert_eq!(&report.output[8..12], b"WEBP");
    assert_eq!(&report.output[12..16], b"VP8L");
    decode_pixels(&report.output);
}

#[test]
fn still_vp8x_wrapped_input_round_trips() {
    let plain = encode_lossless(&textured_rgba(128, 128));
    let wrapped = vp8x_fixture(0x00, &plain[12..]);
    let config = LsbConfig::new(42);
    match webp::probe_support(&wrapped, &limits()).unwrap() {
        WebpSupport::StillLossless { width, height, .. } => {
            assert_eq!((width, height), (128, 128));
        }
    }
    let payload = vec![0x77u8; 36];
    let report = webp::embed(&wrapped, &payload, &config, &limits()).unwrap();
    assert!(report.embedded);
    let recovered = webp::extract(&report.output, payload.len(), &config, &limits()).unwrap();
    assert_eq!(recovered, payload);
}

#[test]
fn unrelated_metadata_chunks_are_dropped() {
    let plain = encode_lossless(&textured_rgba(128, 128));
    let mut siblings = chunk(b"XMP ", b"<xmp/>!");
    siblings.extend_from_slice(&plain[12..]);
    let wrapped = vp8x_fixture(0x04, &siblings);
    let config = LsbConfig::new(42);
    let report = webp::embed(&wrapped, b"drop check", &config, &limits()).unwrap();
    assert!(report.embedded);
    assert!(!report.output.windows(4).any(|w| w == b"XMP "));
}

#[test]
fn tiled_origins_checked_before_decode() {
    let file = encode_lossless(&textured_rgba(128, 128));
    let tile = TileConfig::try_new(42, 64).unwrap();
    let tight = CarrierLimits::builder().max_tiled_origins(8).build();
    assert!(matches!(
        webp::extract_tiled(&file, 4, &tile, 64, &tight),
        Err(StegoError::ResourceLimitExceeded(_))
    ));
    assert!(matches!(
        webp::extract_tiled(&file, 4, &tile, 0, &limits()),
        Err(StegoError::InvalidConfig(_))
    ));
}

#[test]
fn capacity_report_matches_lsb_units() {
    let file = encode_lossless(&textured_rgba(64, 64));
    let config = LsbConfig::new(42);
    let small = webp::capacity(&file, 8, &config, &limits()).unwrap();
    assert!(small.is_sufficient());
    let huge = webp::capacity(&file, 1024 * 1024, &config, &limits()).unwrap();
    assert!(!huge.is_sufficient());
    assert!(huge.required > huge.available);
}
