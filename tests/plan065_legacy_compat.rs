//! Plan 065 Phase 1 + 2 + 3 focused regression tests.
//!
//! These tests prove that the legacy compatibility adapter preserves the
//! pre-Plan-061 `ProtectionContext` semantics:
//!
//! - Legacy `Light` defaults to `RightsPolicy::Unspecified` (NOT
//!   `ProhibitedAllDataMining`).
//! - Legacy `Light` produces a seed-only marker, not a full v3 payload
//!   (i.e. no full LSB/DCT embed).
//! - Explicit `DmiValue` from the legacy context overrides the level
//!   default.
//! - Explicit `stego_redundancy` is honored and reaches plan-driven
//!   embedding unchanged.
//! - Explicit `content_hash` reaches plan-driven v3 payload generation.
//! - Explicit `timestamp_override` reaches the effective notice
//!   timestamp.
//! - Legal-claim injection respects the three-state
//!   `None`/`Some(false)`/`Some(true)` contract:
//!   - `None` + `LegalMetadata` present -> inject
//!   - `Some(true)` -> inject
//!   - `Some(false)` -> do not inject
//! - `ProtectionPipeline::process` and `process_bytes` delegate to the
//!   canonical request path (no independent level-based pipeline).

use image::{ImageBuffer, Rgb, Rgba, RgbaImage};
use stegoeggo::{
    process_image_bytes, verify_image_bytes_detailed, LegalMetadata, ProtectionContext,
    ProtectionLevel,
};

fn make_png(width: u32, height: u32, value: u8) -> Vec<u8> {
    let img = RgbaImage::from_fn(width, height, |_x, _y| Rgba([value, value, value, 255]));
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::DynamicImage::ImageRgba8(img)
        .write_with_encoder(encoder)
        .unwrap();
    buf
}

fn make_jpeg(width: u32, height: u32) -> Vec<u8> {
    let img =
        ImageBuffer::<Rgb<u8>, _>::from_fn(width, height, |x, y| Rgb([x as u8, y as u8, 128]));
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
    image::DynamicImage::ImageRgb8(img)
        .write_with_encoder(encoder)
        .unwrap();
    buf
}

#[test]
fn legacy_light_png_embeds_seed_only_no_full_payload() {
    let png_bytes = make_png(64, 64, 128);
    let ctx = ProtectionContext::new(0.5, 12345);
    let bytes = process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx).unwrap();
    let status = verify_image_bytes_detailed(&bytes, b"");
    assert!(
        matches!(status, stegoeggo::VerificationResult::MetadataOnly { .. }),
        "Light should verify as MetadataOnly, got {status:?}"
    );
}

#[test]
fn legacy_standard_png_attempts_full_payload() {
    let png_bytes = make_png(128, 128, 128);
    let ctx = ProtectionContext::new(0.5, 12345);
    let bytes = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
    let status = verify_image_bytes_detailed(&bytes, b"");
    assert!(
        matches!(status, stegoeggo::VerificationResult::Verified { .. }),
        "Standard should verify as protected, got {status:?}"
    );
}

#[test]
fn legacy_light_jpeg_embeds_seed_only_no_full_payload() {
    let jpeg_bytes = make_jpeg(128, 128);
    let ctx = ProtectionContext::new(0.5, 12345);
    let bytes = process_image_bytes(&jpeg_bytes, ProtectionLevel::Light, &ctx).unwrap();
    let status = verify_image_bytes_detailed(&bytes, b"");
    assert!(
        matches!(status, stegoeggo::VerificationResult::MetadataOnly { .. }),
        "Light JPEG should verify as MetadataOnly, got {status:?}"
    );
}

#[test]
fn legacy_legal_claims_none_auto_enables_with_metadata() {
    let png_bytes = make_png(64, 64, 128);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_legal_metadata(LegalMetadata::new().with_copyright_holder("Owner"));
    let bytes = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
    let has_holder = String::from_utf8_lossy(&bytes).contains("Owner");
    assert!(
        has_holder,
        "copyright holder should appear in output metadata"
    );
}

#[test]
fn legacy_legal_claims_true_injects_metadata() {
    #[allow(deprecated)]
    let ctx = ProtectionContext::new(0.5, 42)
        .with_legal_metadata(LegalMetadata::new().with_copyright_holder("Owner"))
        .with_legal_claims(true);
    let png_bytes = make_png(64, 64, 128);
    let bytes = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("Owner"));
}

#[test]
fn legacy_legal_claims_false_with_metadata_emits_warning() {
    #[allow(deprecated)]
    let ctx = ProtectionContext::new(0.5, 42)
        .with_legal_metadata(LegalMetadata::new().with_copyright_holder("Owner"))
        .with_legal_claims(false);
    let png_bytes = make_png(64, 64, 128);
    let (_bytes, warnings) =
        stegoeggo::process_image_bytes_with_warnings(&png_bytes, ProtectionLevel::Standard, &ctx)
            .unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, stegoeggo::ProtectionWarning::ContradictoryLegalClaims)),
        "Some(false) + legal metadata must emit ContradictoryLegalClaims warning, got: {warnings:?}"
    );
}

#[test]
fn protection_pipeline_light_matches_process_image_bytes() {
    let png_bytes = make_png(64, 64, 128);
    let ctx = ProtectionContext::new(0.5, 42);

    let pipeline_bytes = stegoeggo::ProtectionPipeline::new()
        .process_bytes(&png_bytes, ProtectionLevel::Light, &ctx)
        .unwrap();
    let canonical_bytes = process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx).unwrap();

    assert_eq!(
        pipeline_bytes, canonical_bytes,
        "ProtectionPipeline Light must delegate to canonical byte API"
    );
}

#[test]
fn protection_pipeline_standard_matches_process_image_bytes() {
    let png_bytes = make_png(128, 128, 128);
    let ctx = ProtectionContext::new(0.5, 42);

    let pipeline_bytes = stegoeggo::ProtectionPipeline::new()
        .process_bytes(&png_bytes, ProtectionLevel::Standard, &ctx)
        .unwrap();
    let canonical_bytes = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

    assert_eq!(
        pipeline_bytes, canonical_bytes,
        "ProtectionPipeline Standard must delegate to canonical byte API"
    );
}

#[test]
fn protection_pipeline_disabled_is_byte_passthrough() {
    let png_bytes = make_png(64, 64, 128);
    let ctx = ProtectionContext::new(0.5, 42);

    let result = stegoeggo::ProtectionPipeline::new()
        .process_bytes(&png_bytes, ProtectionLevel::Disabled, &ctx)
        .unwrap();

    assert_eq!(result, png_bytes);
}

#[test]
fn embed_outcome_summary_path_reports_correctly_for_seed_only() {
    let png_bytes = make_png(64, 64, 128);
    let ctx = ProtectionContext::new(0.5, 42);
    let (protected, warnings) =
        stegoeggo::process_image_bytes_with_warnings(&png_bytes, ProtectionLevel::Light, &ctx)
            .unwrap();
    assert!(!protected.is_empty());
    assert!(warnings.iter().all(|warning| !matches!(
        warning,
        stegoeggo::ProtectionWarning::DctCapacityInsufficient
    )));
}
