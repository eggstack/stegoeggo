#![allow(deprecated)]

use image::ImageEncoder;
use stegoeggo::{
    process_image_bytes, process_image_bytes_with_warnings, process_request_bytes,
    process_request_bytes_with_report, process_request_bytes_with_warnings, AuthenticationMode,
    DmiValue, HiddenMarkerMode, LegalMetadata, ProtectionChannels, ProtectionContext,
    ProtectionLevel, ProtectionRequest, ProtectionWarning, ResourceLimits, RightsNotice,
    RightsPolicy,
};

const TS: &str = "2025-01-01T00:00:00Z";

fn png_bytes(size: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(size, size);
    let mut buf = Vec::new();
    let enc = image::codecs::png::PngEncoder::new(&mut buf);
    enc.write_image(&img.to_rgb8(), size, size, image::ExtendedColorType::Rgb8)
        .unwrap();
    buf
}

fn jpeg_bytes(size: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(size, size);
    let mut buf = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
    enc.write_image(&img.to_rgb8(), size, size, image::ExtendedColorType::Rgb8)
        .unwrap();
    buf
}

fn webp_bytes(size: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(size, size);
    let mut buf = Vec::new();
    let enc = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
    enc.write_image(&img.to_rgb8(), size, size, image::ExtendedColorType::Rgb8)
        .unwrap();
    buf
}

fn notice_with_seed(seed: u64) -> RightsNotice {
    RightsNotice::new()
        .with_copyright_holder("Convergence")
        .with_seed(seed)
}

fn metadata_only_request() -> ProtectionRequest {
    ProtectionRequest::metadata_only(notice_with_seed(42), RightsPolicy::ProhibitedAiMlTraining)
        .with_seed(42)
        .with_timestamp_override(TS)
}

fn seed_only_request() -> ProtectionRequest {
    ProtectionRequest::new(
        notice_with_seed(42),
        RightsPolicy::Unspecified,
        ProtectionChannels {
            rights_metadata: true,
            hidden_marker: HiddenMarkerMode::SeedOnly,
            authentication: AuthenticationMode::None,
        },
    )
    .with_seed(42)
    .with_intensity(0.5)
    .with_timestamp_override(TS)
}

fn best_effort_request() -> ProtectionRequest {
    ProtectionRequest::with_hidden_marker(
        notice_with_seed(42),
        RightsPolicy::ProhibitedAiMlTraining,
    )
    .with_seed(42)
    .with_intensity(0.5)
    .with_timestamp_override(TS)
}

fn tiled_request() -> ProtectionRequest {
    ProtectionRequest::new(
        notice_with_seed(42),
        RightsPolicy::ProhibitedAiMlTraining,
        ProtectionChannels {
            rights_metadata: true,
            hidden_marker: HiddenMarkerMode::Tiled { tile_size: 32 },
            authentication: AuthenticationMode::None,
        },
    )
    .with_seed(42)
    .with_intensity(0.5)
    .with_timestamp_override(TS)
}

fn authenticated_request() -> ProtectionRequest {
    ProtectionRequest::new(
        notice_with_seed(42),
        RightsPolicy::Allowed,
        ProtectionChannels::authenticated(),
    )
    .with_seed(42)
    .with_intensity(0.5)
    .with_mac_key(b"convergence-key".to_vec())
    .with_timestamp_override(TS)
}

fn sync_variants_equal(input: &[u8], request: &ProtectionRequest) {
    let plain = process_request_bytes(input, request).unwrap();
    let (with_warnings_bytes, warnings) =
        process_request_bytes_with_warnings(input, request).unwrap();
    let (with_report_bytes, report) = process_request_bytes_with_report(input, request).unwrap();
    assert_eq!(plain, with_warnings_bytes);
    assert_eq!(plain, with_report_bytes);
    assert_eq!(warnings, *report.warnings());
}

#[test]
fn sync_variants_agree_png() {
    let input = png_bytes(64);
    sync_variants_equal(&input, &metadata_only_request());
    sync_variants_equal(&input, &seed_only_request());
    sync_variants_equal(&input, &best_effort_request());
    sync_variants_equal(&input, &tiled_request());
    sync_variants_equal(&input, &authenticated_request());
}

#[test]
fn sync_variants_agree_jpeg() {
    let input = jpeg_bytes(128);
    sync_variants_equal(&input, &metadata_only_request());
    sync_variants_equal(&input, &seed_only_request());
    sync_variants_equal(&input, &best_effort_request());
    sync_variants_equal(&input, &authenticated_request());
}

#[test]
fn sync_variants_agree_webp() {
    let input = webp_bytes(64);
    sync_variants_equal(&input, &metadata_only_request());
    sync_variants_equal(&input, &seed_only_request());
    sync_variants_equal(&input, &best_effort_request());
    sync_variants_equal(&input, &tiled_request());
    sync_variants_equal(&input, &authenticated_request());
}

#[test]
fn legacy_standard_matches_explicit_request_png() {
    let input = png_bytes(64);
    let ctx = ProtectionContext::new(0.5, 42).with_timestamp_override(TS);
    let legacy = process_image_bytes(&input, ProtectionLevel::Standard, &ctx).unwrap();
    let notice = RightsNotice::default()
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_seed(42);
    let request =
        ProtectionRequest::with_hidden_marker(notice, RightsPolicy::ProhibitedAiMlTraining)
            .with_seed(42)
            .with_intensity(0.5)
            .with_timestamp_override(TS);
    let canonical = process_request_bytes(&input, &request).unwrap();
    assert_eq!(legacy, canonical);
}

#[test]
fn legacy_light_matches_explicit_request_png() {
    let input = png_bytes(64);
    let ctx = ProtectionContext::new(0.5, 42).with_timestamp_override(TS);
    let legacy = process_image_bytes(&input, ProtectionLevel::Light, &ctx).unwrap();
    let notice = RightsNotice::default()
        .with_dmi(DmiValue::Unspecified)
        .with_seed(42);
    let request = ProtectionRequest::new(
        notice,
        RightsPolicy::Unspecified,
        ProtectionChannels {
            rights_metadata: true,
            hidden_marker: HiddenMarkerMode::SeedOnly,
            authentication: AuthenticationMode::None,
        },
    )
    .with_seed(42)
    .with_intensity(0.5)
    .with_timestamp_override(TS);
    let canonical = process_request_bytes(&input, &request).unwrap();
    assert_eq!(legacy, canonical);
}

#[test]
fn legacy_warnings_are_canonical_plus_compat_jpeg() {
    let input = jpeg_bytes(128);
    let ctx = ProtectionContext::new(0.5, 42).with_timestamp_override(TS);
    let (legacy_bytes, legacy_warnings) =
        process_image_bytes_with_warnings(&input, ProtectionLevel::Standard, &ctx).unwrap();
    let notice = RightsNotice::default()
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_seed(42);
    let request =
        ProtectionRequest::with_hidden_marker(notice, RightsPolicy::ProhibitedAiMlTraining)
            .with_seed(42)
            .with_intensity(0.5)
            .with_timestamp_override(TS);
    let (canonical_bytes, canonical_warnings) =
        process_request_bytes_with_warnings(&input, &request).unwrap();
    assert_eq!(legacy_bytes, canonical_bytes);
    for w in &canonical_warnings {
        assert!(legacy_warnings.contains(w));
    }
    assert!(legacy_warnings.contains(&ProtectionWarning::JpegReencodeFragile));
}

#[test]
fn legacy_missing_mac_key_is_compat_only() {
    let input = png_bytes(64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_timestamp_override(TS)
        .with_evidence_profile(stegoeggo::EvidenceProfile::Maximal);
    let (_, legacy_warnings) =
        process_image_bytes_with_warnings(&input, ProtectionLevel::Standard, &ctx).unwrap();
    assert!(legacy_warnings.contains(&ProtectionWarning::MissingMacKey));
    let notice = RightsNotice::default()
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_seed(42);
    let request =
        ProtectionRequest::with_hidden_marker(notice, RightsPolicy::ProhibitedAiMlTraining)
            .with_seed(42)
            .with_intensity(0.5)
            .with_timestamp_override(TS);
    let (_, canonical_warnings) = process_request_bytes_with_warnings(&input, &request).unwrap();
    assert!(!canonical_warnings.contains(&ProtectionWarning::MissingMacKey));
}

#[test]
fn legacy_contradictory_claims_is_compat_only() {
    let input = png_bytes(64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_timestamp_override(TS)
        .with_legal_metadata(LegalMetadata::new().with_copyright_holder("Owner"))
        .with_legal_claims(false);
    let (legacy_bytes, legacy_warnings) =
        process_image_bytes_with_warnings(&input, ProtectionLevel::Standard, &ctx).unwrap();
    assert!(legacy_warnings.contains(&ProtectionWarning::ContradictoryLegalClaims));
    let notice = RightsNotice::default()
        .with_dmi(DmiValue::ProhibitedAiMlTraining)
        .with_seed(42);
    let request =
        ProtectionRequest::with_hidden_marker(notice, RightsPolicy::ProhibitedAiMlTraining)
            .with_seed(42)
            .with_intensity(0.5)
            .with_timestamp_override(TS);
    let (canonical_bytes, _) = process_request_bytes_with_warnings(&input, &request).unwrap();
    assert_eq!(legacy_bytes, canonical_bytes);
}

#[test]
fn invalid_format_errors_match() {
    let garbage = b"not an image";
    let request = best_effort_request();
    assert!(process_request_bytes(garbage, &request).is_err());
    let ctx = ProtectionContext::new(0.5, 42);
    assert!(process_image_bytes(garbage, ProtectionLevel::Standard, &ctx).is_err());
}

#[test]
fn hmac_without_key_errors_match() {
    let request = ProtectionRequest::new(
        notice_with_seed(42),
        RightsPolicy::Allowed,
        ProtectionChannels::authenticated(),
    )
    .with_seed(42)
    .with_timestamp_override(TS);
    let input = png_bytes(32);
    assert!(process_request_bytes(&input, &request).is_err());
    assert!(process_request_bytes_with_warnings(&input, &request).is_err());
    assert!(process_request_bytes_with_report(&input, &request).is_err());
}

#[test]
fn tiny_image_capacity_warning_in_both_paths() {
    let input = png_bytes(8);
    let request = best_effort_request();
    let (_, canonical_warnings) = process_request_bytes_with_warnings(&input, &request).unwrap();
    assert!(canonical_warnings.contains(&ProtectionWarning::LsbCapacitySkipped));
    let ctx = ProtectionContext::new(0.5, 42).with_timestamp_override(TS);
    let (_, legacy_warnings) =
        process_image_bytes_with_warnings(&input, ProtectionLevel::Standard, &ctx).unwrap();
    assert!(legacy_warnings.contains(&ProtectionWarning::LsbCapacitySkipped));
}

#[test]
fn resource_limit_errors_match() {
    let limits = ResourceLimits::builder().max_input_bytes(1).build();
    let request = ProtectionRequest::metadata_only(notice_with_seed(42), RightsPolicy::Allowed)
        .with_seed(42)
        .with_timestamp_override(TS)
        .with_resource_limits(limits);
    let input = png_bytes(32);
    let err = process_request_bytes(&input, &request).unwrap_err();
    assert!(err.to_string().contains("too large"));
    let ctx = ProtectionContext::new(0.5, 42)
        .with_resource_limits(ResourceLimits::builder().max_input_bytes(1).build());
    let legacy_err = process_image_bytes(&input, ProtectionLevel::Standard, &ctx).unwrap_err();
    assert!(legacy_err.to_string().contains("too large"));
}

#[test]
fn metadata_disabled_warning_matches() {
    let request = ProtectionRequest::new(
        notice_with_seed(42),
        RightsPolicy::Unspecified,
        ProtectionChannels {
            rights_metadata: false,
            hidden_marker: HiddenMarkerMode::BestEffort,
            authentication: AuthenticationMode::None,
        },
    )
    .with_seed(42)
    .with_timestamp_override(TS);
    let input = png_bytes(32);
    let (_, warnings) = process_request_bytes_with_warnings(&input, &request).unwrap();
    assert!(warnings.contains(&ProtectionWarning::MetadataInjectionDisabled));
}

#[cfg(feature = "parallel")]
mod parallel_convergence {
    use super::*;
    use stegoeggo::{
        process_request_bytes_parallel, process_request_bytes_with_report_parallel,
        process_request_bytes_with_warnings_parallel,
    };

    #[test]
    fn parallel_matches_sync_per_item() {
        let inputs = vec![png_bytes(64), png_bytes(64), png_bytes(64)];
        let request = best_effort_request();
        let parallel = process_request_bytes_parallel(&inputs, &request).unwrap();
        assert_eq!(parallel.len(), inputs.len());
        for (i, out) in parallel.iter().enumerate() {
            let expected = process_request_bytes(&inputs[i], &request).unwrap();
            assert_eq!(out, &expected);
        }
    }

    #[test]
    fn parallel_warnings_match_sync() {
        let inputs = vec![png_bytes(64), jpeg_bytes(128), webp_bytes(64)];
        let request = best_effort_request();
        let parallel = process_request_bytes_with_warnings_parallel(&inputs, &request).unwrap();
        assert_eq!(parallel.len(), inputs.len());
        for (i, (bytes, warnings)) in parallel.iter().enumerate() {
            let (expected_bytes, expected_warnings) =
                process_request_bytes_with_warnings(&inputs[i], &request).unwrap();
            assert_eq!(bytes, &expected_bytes);
            assert_eq!(warnings, &expected_warnings);
        }
    }

    #[test]
    fn parallel_report_matches_sync() {
        let inputs = vec![png_bytes(64), png_bytes(64)];
        let request = authenticated_request();
        let parallel = process_request_bytes_with_report_parallel(&inputs, &request).unwrap();
        for (i, (bytes, report)) in parallel.iter().enumerate() {
            let (expected_bytes, expected_report) =
                process_request_bytes_with_report(&inputs[i], &request).unwrap();
            assert_eq!(bytes, &expected_bytes);
            assert_eq!(report.warnings(), expected_report.warnings());
            assert_eq!(report.stego_succeeded(), expected_report.stego_succeeded());
        }
    }

    #[test]
    fn parallel_preserves_order() {
        let small = png_bytes(32);
        let large_input = png_bytes(96);
        let inputs = vec![small.clone(), large_input.clone(), small.clone()];
        let request = metadata_only_request();
        let out = process_request_bytes_parallel(&inputs, &request).unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], process_request_bytes(&small, &request).unwrap());
        assert_eq!(
            out[1],
            process_request_bytes(&large_input, &request).unwrap()
        );
        assert_eq!(out[2], process_request_bytes(&small, &request).unwrap());
    }

    #[test]
    fn parallel_error_matches_sync_error() {
        let good = png_bytes(32);
        let inputs = vec![good, b"not an image".to_vec()];
        let request = best_effort_request();
        assert!(process_request_bytes_parallel(&inputs, &request).is_err());
    }

    #[test]
    fn parallel_tiled_matches_sync() {
        let inputs = vec![png_bytes(64), png_bytes(64)];
        let request = tiled_request();
        let parallel = process_request_bytes_parallel(&inputs, &request).unwrap();
        for (i, out) in parallel.iter().enumerate() {
            assert_eq!(out, &process_request_bytes(&inputs[i], &request).unwrap());
        }
    }
}

#[cfg(feature = "async")]
mod async_convergence {
    use super::*;
    use stegoeggo::{
        process_request_bytes_async, process_request_bytes_with_report_async,
        process_request_bytes_with_warnings_async,
    };

    #[tokio::test]
    async fn async_matches_sync_png() {
        let input = png_bytes(64);
        for request in [
            metadata_only_request(),
            seed_only_request(),
            best_effort_request(),
            tiled_request(),
            authenticated_request(),
        ] {
            let expected = process_request_bytes(&input, &request).unwrap();
            let actual = process_request_bytes_async(input.clone(), request.clone())
                .await
                .unwrap();
            assert_eq!(expected, actual);
        }
    }

    #[tokio::test]
    async fn async_matches_sync_jpeg_webp() {
        let jpeg = jpeg_bytes(128);
        let webp = webp_bytes(64);
        let request = best_effort_request();
        assert_eq!(
            process_request_bytes(&jpeg, &request).unwrap(),
            process_request_bytes_async(jpeg.clone(), request.clone())
                .await
                .unwrap()
        );
        assert_eq!(
            process_request_bytes(&webp, &request).unwrap(),
            process_request_bytes_async(webp.clone(), request.clone())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn async_warnings_and_report_match_sync() {
        let input = png_bytes(64);
        let request = best_effort_request();
        let (expected_bytes, expected_warnings) =
            process_request_bytes_with_warnings(&input, &request).unwrap();
        let (actual_bytes, actual_warnings) =
            process_request_bytes_with_warnings_async(input.clone(), request.clone())
                .await
                .unwrap();
        assert_eq!(expected_bytes, actual_bytes);
        assert_eq!(expected_warnings, actual_warnings);

        let (expected_bytes, expected_report) =
            process_request_bytes_with_report(&input, &request).unwrap();
        let (actual_bytes, actual_report) =
            process_request_bytes_with_report_async(input.clone(), request.clone())
                .await
                .unwrap();
        assert_eq!(expected_bytes, actual_bytes);
        assert_eq!(expected_report.warnings(), actual_report.warnings());
        assert_eq!(
            expected_report.stego_succeeded(),
            actual_report.stego_succeeded()
        );
    }

    #[tokio::test]
    async fn async_errors_match_sync() {
        let request = best_effort_request();
        assert!(
            process_request_bytes_async(b"not an image".to_vec(), request.clone())
                .await
                .is_err()
        );
        let hmac_request = ProtectionRequest::new(
            notice_with_seed(42),
            RightsPolicy::Allowed,
            ProtectionChannels::authenticated(),
        )
        .with_seed(42)
        .with_timestamp_override(TS);
        let input = png_bytes(32);
        assert!(process_request_bytes(&input, &hmac_request).is_err());
        assert!(
            process_request_bytes_async(input.clone(), hmac_request.clone())
                .await
                .is_err()
        );
    }

    #[cfg(feature = "parallel")]
    #[tokio::test]
    async fn async_parallel_matches_sync_parallel() {
        use stegoeggo::{
            process_request_bytes_parallel, process_request_bytes_parallel_async,
            process_request_bytes_with_report_parallel_async,
            process_request_bytes_with_warnings_parallel_async,
        };
        let inputs = vec![png_bytes(64), png_bytes(64)];
        let request = best_effort_request();
        let expected = process_request_bytes_parallel(&inputs, &request).unwrap();
        let actual = process_request_bytes_parallel_async(inputs.clone(), request.clone())
            .await
            .unwrap();
        assert_eq!(expected, actual);

        let (expected_bytes, expected_warnings) =
            process_request_bytes_with_warnings(&inputs[0], &request).unwrap();
        let parallel_warnings =
            process_request_bytes_with_warnings_parallel_async(inputs.clone(), request.clone())
                .await
                .unwrap();
        assert_eq!(parallel_warnings[0].0, expected_bytes);
        assert_eq!(parallel_warnings[0].1, expected_warnings);

        let (expected_bytes, expected_report) =
            process_request_bytes_with_report(&inputs[0], &request).unwrap();
        let parallel_reports =
            process_request_bytes_with_report_parallel_async(inputs.clone(), request.clone())
                .await
                .unwrap();
        assert_eq!(parallel_reports[0].0, expected_bytes);
        assert_eq!(parallel_reports[0].1.warnings(), expected_report.warnings());
    }
}
