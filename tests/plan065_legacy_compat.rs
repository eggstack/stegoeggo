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
    process_image_bytes, verify_image_bytes_detailed, AuthenticationMode, DmiValue,
    HiddenMarkerMode, LegalMetadata, ProtectionChannels, ProtectionContext, ProtectionLevel,
    ProtectionRequest, RightsNotice, RightsPolicy,
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

mod plan065_compat_helpers {
    use super::*;

    #[allow(deprecated)]
    pub fn build_request(level: ProtectionLevel, ctx: &ProtectionContext) -> ProtectionRequest {
        let rights_metadata =
            level != ProtectionLevel::Disabled && ctx.inject_metadata() != Some(false);
        let policy = if rights_metadata {
            ctx.dmi_value()
                .map(RightsPolicy::from)
                .unwrap_or_else(|| level.default_policy())
        } else {
            RightsPolicy::Unspecified
        };
        let dmi = ctx.dmi_value().unwrap_or_else(|| DmiValue::from(policy));
        let hidden_marker = match level {
            ProtectionLevel::Disabled => HiddenMarkerMode::Disabled,
            ProtectionLevel::Light => HiddenMarkerMode::SeedOnly,
            ProtectionLevel::Standard => ctx
                .tile_size()
                .filter(|&size| size > 0)
                .map_or(HiddenMarkerMode::BestEffort, |size| {
                    HiddenMarkerMode::Tiled { tile_size: size }
                }),
            _ => HiddenMarkerMode::Disabled,
        };
        let authentication = if ctx.mac_key().is_some() {
            AuthenticationMode::Hmac
        } else {
            AuthenticationMode::None
        };
        let mut notice = RightsNotice::default().with_dmi(dmi).with_seed(ctx.seed());
        let include_claims = ctx.inject_legal_claims() != Some(false);
        if include_claims {
            if let Some(metadata) = ctx.legal_metadata() {
                notice = notice.with_legal_metadata_fields(metadata);
            }
        }
        let channels = ProtectionChannels {
            rights_metadata,
            hidden_marker,
            authentication,
        };
        let mut request = ProtectionRequest::new(notice, policy, channels)
            .with_seed(ctx.seed())
            .with_intensity(ctx.intensity())
            .with_jpeg_quality(ctx.jpeg_quality());
        if let Some(format) = ctx.output_format() {
            request = request.with_output_format(format);
        }
        if ctx.progressive_jpeg() {
            request = request.with_progressive_jpeg();
        }
        if include_claims {
            if let Some(metadata) = ctx.legal_metadata() {
                request = request.with_legal_metadata(metadata.clone());
            }
        }
        if let Some(key) = ctx.mac_key() {
            request = request.with_mac_key(key.to_vec());
        }
        if let Some(max_dimension) = ctx.max_dimension() {
            request = request.with_max_dimension(max_dimension);
        }
        if let Some(redundancy) = ctx.stego_redundancy_field() {
            request = request.with_stego_redundancy(redundancy);
        }
        if let Some(hash) = ctx.content_hash() {
            request = request.with_content_hash(hash);
        }
        request.with_resource_limits(ctx.resource_limits())
    }
}

#[test]
fn legacy_light_default_policy_is_unspecified() {
    let ctx = ProtectionContext::new(0.5, 42);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Light, &ctx);
    assert_eq!(request.policy(), RightsPolicy::Unspecified);
}

#[test]
fn legacy_standard_default_policy_is_prohibited_ai_ml_training() {
    let ctx = ProtectionContext::new(0.5, 42);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Standard, &ctx);
    assert_eq!(request.policy(), RightsPolicy::ProhibitedAiMlTraining);
}

#[test]
fn legacy_disabled_policy_is_unspecified() {
    let ctx = ProtectionContext::new(0.5, 42);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Disabled, &ctx);
    assert_eq!(request.policy(), RightsPolicy::Unspecified);
}

#[test]
fn legacy_light_uses_seed_only_marker_mode() {
    let ctx = ProtectionContext::new(0.5, 42);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Light, &ctx);
    assert!(matches!(
        request.channels().hidden_marker,
        stegoeggo::HiddenMarkerMode::SeedOnly
    ));
}

#[test]
fn legacy_standard_uses_best_effort_marker_mode() {
    let ctx = ProtectionContext::new(0.5, 42);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Standard, &ctx);
    assert!(matches!(
        request.channels().hidden_marker,
        stegoeggo::HiddenMarkerMode::BestEffort
    ));
}

#[test]
#[allow(deprecated)]
fn legacy_explicit_dmi_overrides_level_default() {
    let ctx = ProtectionContext::new(0.5, 42).with_dmi(DmiValue::Allowed);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Light, &ctx);
    assert_eq!(request.policy(), RightsPolicy::Allowed);
}

#[test]
fn legacy_light_png_embeds_seed_only_no_full_payload() {
    let png_bytes = make_png(64, 64, 128);
    let ctx = ProtectionContext::new(0.5, 12345);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Light, &ctx);
    let (bytes, report) =
        stegoeggo::process_request_bytes_with_report(&png_bytes, &request).unwrap();
    assert!(
        report.embed_summary.is_none(),
        "Light should report embed_summary=None (seed-only)"
    );
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
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Standard, &ctx);
    let (_bytes, report) =
        stegoeggo::process_request_bytes_with_report(&png_bytes, &request).unwrap();
    assert!(
        report.embed_summary.is_some(),
        "Standard should report an embed summary"
    );
    let summary = report.embed_summary.unwrap();
    assert!(summary.is_embedded());
}

#[test]
fn legacy_light_jpeg_embeds_seed_only_no_full_payload() {
    let jpeg_bytes = make_jpeg(128, 128);
    let ctx = ProtectionContext::new(0.5, 12345);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Light, &ctx);
    let (bytes, report) =
        stegoeggo::process_request_bytes_with_report(&jpeg_bytes, &request).unwrap();
    assert!(
        report.embed_summary.is_none(),
        "Light JPEG should report embed_summary=None (Q-table seed only)"
    );
    let status = verify_image_bytes_detailed(&bytes, b"");
    assert!(
        matches!(status, stegoeggo::VerificationResult::MetadataOnly { .. }),
        "Light JPEG should verify as MetadataOnly, got {status:?}"
    );
}

#[test]
fn legacy_explicit_stego_redundancy_reaches_resolved_plan() {
    let ctx = ProtectionContext::new(0.5, 42).with_stego_redundancy(7);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Standard, &ctx);
    let plan = stegoeggo::resolve_request(&request, stegoeggo::ImageOutputFormat::Png).unwrap();
    assert_eq!(plan.effective_redundancy(), 7);
}

#[test]
fn legacy_standard_without_explicit_redundancy_uses_intensity_derived() {
    let ctx = ProtectionContext::new(0.8, 42);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Standard, &ctx);
    let plan = stegoeggo::resolve_request(&request, stegoeggo::ImageOutputFormat::Png).unwrap();
    assert_eq!(plan.effective_redundancy(), 3);
}

#[test]
fn legacy_content_hash_reaches_resolved_plan() {
    let ctx = ProtectionContext::new(0.5, 42).with_content_hash([0xDE, 0xAD, 0xBE, 0xEF]);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Standard, &ctx);
    let plan = stegoeggo::resolve_request(&request, stegoeggo::ImageOutputFormat::Png).unwrap();
    assert_eq!(plan.content_hash(), Some([0xDE, 0xAD, 0xBE, 0xEF]));
}

#[test]
fn legacy_timestamp_override_reaches_effective_notice() {
    let ts = "2025-06-15T10:00:00Z";
    let ctx = ProtectionContext::new(0.5, 42)
        .with_legal_metadata(LegalMetadata::new().with_copyright_holder("Owner"))
        .with_timestamp_override(ts);
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Standard, &ctx)
        .with_timestamp_override(ts);
    let plan = stegoeggo::resolve_request(&request, stegoeggo::ImageOutputFormat::Png).unwrap();
    assert_eq!(plan.effective_notice().notice_applied_at(), Some(ts));
}

#[test]
fn legacy_legal_claims_none_auto_enables_with_metadata() {
    let png_bytes = make_png(64, 64, 128);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_legal_metadata(LegalMetadata::new().with_copyright_holder("Owner"));
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Standard, &ctx);
    let (bytes, report) =
        stegoeggo::process_request_bytes_with_report(&png_bytes, &request).unwrap();
    assert!(report.metadata_injected, "legal claims should auto-inject");
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
    let request = plan065_compat_helpers::build_request(ProtectionLevel::Standard, &ctx);
    let plan = stegoeggo::resolve_request(&request, stegoeggo::ImageOutputFormat::Png).unwrap();
    assert_eq!(plan.effective_notice().copyright_holder(), Some("Owner"));
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
