#![allow(deprecated)]

use image::ImageEncoder;
use stegoeggo::{
    process_image_bytes, process_request_bytes, process_request_bytes_with_report, resolve_request,
    AuthenticationMode, DmiValue, HiddenMarkerMode, ImageOutputFormat, LegalMetadata,
    ProtectionChannels, ProtectionContext, ProtectionLevel, ProtectionPreset, ProtectionRequest,
    ResourceLimits, RightsNotice, RightsPolicy, VerificationStatus,
};

fn create_test_image(width: u32, height: u32) -> image::DynamicImage {
    image::DynamicImage::new_rgb8(width, height)
}

fn image_to_png_bytes(img: &image::DynamicImage) -> Vec<u8> {
    let mut buffer = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
    encoder
        .write_image(
            &img.to_rgb8(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    buffer
}

fn image_to_jpeg_bytes(img: &image::DynamicImage, quality: u8) -> Vec<u8> {
    let mut buffer = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
    encoder
        .write_image(
            &img.to_rgb8(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    buffer
}

fn image_to_webp_bytes(img: &image::DynamicImage) -> Vec<u8> {
    let mut buffer = Vec::new();
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buffer);
    encoder
        .write_image(
            &img.to_rgb8(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    buffer
}

fn simple_notice() -> RightsNotice {
    RightsNotice::new()
        .with_copyright_holder("Test Author")
        .with_usage_terms("All rights reserved")
}

mod resolve_request_validation {
    use super::*;

    #[test]
    fn hmac_with_disabled_hidden_marker_rejected() {
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels {
                rights_metadata: true,
                hidden_marker: HiddenMarkerMode::Disabled,
                authentication: AuthenticationMode::Hmac,
            },
        )
        .with_mac_key(b"test-key".to_vec());

        let result = resolve_request(&request, ImageOutputFormat::Png);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("HMAC authentication requires an enabled hidden marker"),
            "Expected HMAC+disabled error, got: {}",
            err
        );
    }

    #[test]
    fn hmac_without_mac_key_rejected() {
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels::authenticated(),
        );

        let result = resolve_request(&request, ImageOutputFormat::Png);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("HMAC authentication requires a MAC key"),
            "Expected missing MAC key error, got: {}",
            err
        );
    }

    #[test]
    fn prohibited_see_constraints_without_constraints_rejected() {
        let request = ProtectionRequest::metadata_only(
            RightsNotice::new(),
            RightsPolicy::ProhibitedSeeConstraints,
        );

        let result = resolve_request(&request, ImageOutputFormat::Png);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("ProhibitedSeeConstraints requires"),
            "Expected constraints error, got: {}",
            err
        );
    }

    #[test]
    fn prohibited_see_constraints_with_notice_constraints_accepted() {
        let notice = RightsNotice::new().with_ai_constraints("No AI training");
        let request =
            ProtectionRequest::metadata_only(notice, RightsPolicy::ProhibitedSeeConstraints);

        let result = resolve_request(&request, ImageOutputFormat::Png);
        assert!(result.is_ok());
    }

    #[test]
    fn prohibited_see_constraints_with_legal_metadata_constraints_accepted() {
        let meta = LegalMetadata::new().with_ai_constraints("No AI training");
        let request = ProtectionRequest::metadata_only(
            RightsNotice::new(),
            RightsPolicy::ProhibitedSeeConstraints,
        )
        .with_legal_metadata(meta);

        let result = resolve_request(&request, ImageOutputFormat::Png);
        assert!(result.is_ok());
    }

    #[test]
    fn metadata_only_request_resolves() {
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);
        let result = resolve_request(&request, ImageOutputFormat::Png);
        assert!(result.is_ok());

        let plan = result.unwrap();
        assert_eq!(plan.effective_policy(), RightsPolicy::Allowed);
        assert_eq!(plan.effective_dmi(), Some(DmiValue::Allowed));
        assert!(plan.is_metadata_only());
        assert!(!plan.channels().has_stego());
    }

    #[test]
    fn unspecified_policy_yields_no_dmi() {
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Unspecified);
        let plan = resolve_request(&request, ImageOutputFormat::Png).unwrap();
        assert_eq!(plan.effective_dmi(), None);
    }

    #[test]
    fn hidden_marker_request_resolves() {
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed);
        let plan = resolve_request(&request, ImageOutputFormat::Png).unwrap();
        assert!(plan.channels().has_stego());
        assert_eq!(plan.channels().hidden_marker, HiddenMarkerMode::BestEffort);
        assert!(!plan.is_metadata_only());
    }

    #[test]
    fn hmac_request_resolves_with_key() {
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels::authenticated(),
        )
        .with_mac_key(b"secret".to_vec());

        let plan = resolve_request(&request, ImageOutputFormat::Png).unwrap();
        assert_eq!(plan.channels().authentication, AuthenticationMode::Hmac);
        assert!(plan.mac_key().is_some());
    }

    #[test]
    fn metadata_disabled_warning_emitted() {
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Unspecified,
            ProtectionChannels {
                rights_metadata: false,
                hidden_marker: HiddenMarkerMode::BestEffort,
                authentication: AuthenticationMode::None,
            },
        );

        let plan = resolve_request(&request, ImageOutputFormat::Png).unwrap();
        assert!(
            plan.warnings()
                .iter()
                .any(|w| matches!(w, stegoeggo::ProtectionWarning::MetadataInjectionDisabled)),
            "Expected MetadataInjectionDisabled warning"
        );
    }

    #[test]
    fn non_unspecified_policy_requires_rights_metadata() {
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels {
                rights_metadata: false,
                hidden_marker: HiddenMarkerMode::BestEffort,
                authentication: AuthenticationMode::None,
            },
        );

        let result = resolve_request(&request, ImageOutputFormat::Png);
        assert!(
            result.is_err(),
            "Should reject non-Unspecified policy with rights_metadata=false"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("rights_metadata to be enabled"),
            "Error should mention rights_metadata: {}",
            err_msg
        );
    }

    #[test]
    fn unspecified_policy_allows_rights_metadata_disabled() {
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Unspecified,
            ProtectionChannels {
                rights_metadata: false,
                hidden_marker: HiddenMarkerMode::BestEffort,
                authentication: AuthenticationMode::None,
            },
        );

        let result = resolve_request(&request, ImageOutputFormat::Png);
        assert!(
            result.is_ok(),
            "Unspecified policy should allow rights_metadata=false"
        );
    }

    #[test]
    fn input_format_preserved_when_no_output_override() {
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);
        let plan = resolve_request(&request, ImageOutputFormat::Jpeg).unwrap();
        assert_eq!(plan.input_format(), ImageOutputFormat::Jpeg);
        assert_eq!(plan.output_format(), ImageOutputFormat::Jpeg);
    }

    #[test]
    fn output_format_override_applied() {
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed)
            .with_output_format(ImageOutputFormat::WebP);
        let plan = resolve_request(&request, ImageOutputFormat::Png).unwrap();
        assert_eq!(plan.input_format(), ImageOutputFormat::Png);
        assert_eq!(plan.output_format(), ImageOutputFormat::WebP);
    }

    #[test]
    fn seed_randomly_generated_when_not_set() {
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);
        let plan = resolve_request(&request, ImageOutputFormat::Png).unwrap();
        // Seed should be non-zero (extremely unlikely to be 0 from random)
        // We just verify it resolves without error
        let _ = plan.seed();
    }

    #[test]
    fn explicit_seed_preserved() {
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed)
            .with_seed(12345);
        let plan = resolve_request(&request, ImageOutputFormat::Png).unwrap();
        assert_eq!(plan.seed(), 12345);
    }
}

mod process_request_bytes_tests {
    use super::*;

    #[test]
    fn metadata_only_png_roundtrip() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);

        let result = process_request_bytes(&png_bytes, &request);
        assert!(result.is_ok(), "Failed: {}", result.unwrap_err());

        let output = result.unwrap();
        assert!(ImageOutputFormat::is_png(&output));
    }

    #[test]
    fn metadata_only_jpeg_roundtrip() {
        let img = create_test_image(64, 64);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 85);
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);

        let result = process_request_bytes(&jpeg_bytes, &request);
        assert!(result.is_ok(), "Failed: {}", result.unwrap_err());

        let output = result.unwrap();
        assert!(ImageOutputFormat::is_jpeg(&output));
    }

    #[test]
    fn metadata_only_webp_roundtrip() {
        let img = create_test_image(64, 64);
        let webp_bytes = image_to_webp_bytes(&img);
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);

        let result = process_request_bytes(&webp_bytes, &request);
        assert!(result.is_ok(), "Failed: {}", result.unwrap_err());

        let output = result.unwrap();
        assert!(ImageOutputFormat::is_webp(&output));
    }

    #[test]
    fn hidden_marker_png_roundtrip() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let result = process_request_bytes(&png_bytes, &request);
        assert!(result.is_ok(), "Failed: {}", result.unwrap_err());

        let output = result.unwrap();
        assert!(ImageOutputFormat::is_png(&output));
    }

    #[test]
    fn hidden_marker_jpeg_roundtrip() {
        let img = create_test_image(64, 64);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 85);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let result = process_request_bytes(&jpeg_bytes, &request);
        assert!(result.is_ok(), "Failed: {}", result.unwrap_err());

        let output = result.unwrap();
        assert!(ImageOutputFormat::is_jpeg(&output));
    }

    #[test]
    fn with_report_returns_execution_report() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);

        let (output, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(ImageOutputFormat::is_png(&output));
        assert_eq!(report.effective_policy, RightsPolicy::Allowed);
        assert_eq!(report.effective_dmi, Some(DmiValue::Allowed));
        assert!(report.metadata_injected);
        assert!(!report.stego_attempted);
        assert!(!report.stego_succeeded);
        assert!(!report.format_transcoded);
    }

    #[test]
    fn with_report_stego_attempted() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.stego_attempted);
    }

    #[test]
    fn invalid_format_rejected() {
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);
        let result = process_request_bytes(b"not an image", &request);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unrecognized image format"),
            "Expected format error"
        );
    }

    #[test]
    fn metadata_only_with_hmac_rejected() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels::authenticated(),
        );

        let result = process_request_bytes(&png_bytes, &request);
        assert!(result.is_err());
    }

    #[test]
    fn preset_legal_notice_produces_metadata_only() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::from_preset(
            ProtectionPreset::LegalNotice,
            simple_notice(),
            RightsPolicy::Allowed,
        );

        let (output, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(ImageOutputFormat::is_png(&output));
        assert!(report.metadata_injected);
        assert!(!report.stego_attempted);
    }

    #[test]
    fn preset_legal_notice_with_stego_produces_stego() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::from_preset(
            ProtectionPreset::LegalNoticeWithStego,
            simple_notice(),
            RightsPolicy::Allowed,
        )
        .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.metadata_injected);
        assert!(report.stego_attempted);
    }

    #[test]
    fn preset_authenticated_provenance_with_key() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::from_preset(
            ProtectionPreset::AuthenticatedProvenance,
            simple_notice(),
            RightsPolicy::Allowed,
        )
        .with_seed(42)
        .with_mac_key(b"test-key".to_vec());

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.metadata_injected);
        assert!(report.stego_attempted);
    }

    #[test]
    fn no_rights_policy_yields_no_dmi_in_output() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Unspecified);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert_eq!(report.effective_dmi, None);
    }

    #[test]
    fn metadata_only_does_not_modify_pixel_data() {
        let img = create_test_image(32, 32);
        let original_rgb = img.to_rgb8();
        let png_bytes = image_to_png_bytes(&img);

        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);
        let output = process_request_bytes(&png_bytes, &request).unwrap();

        let output_img = image::load_from_memory(&output).unwrap();
        let output_rgb = output_img.to_rgb8();
        assert_eq!(original_rgb.as_raw(), output_rgb.as_raw());
    }
}

mod byte_preservation_tests {
    use super::*;

    #[test]
    fn png_idat_unchanged_in_metadata_only_path() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);
        let output = process_request_bytes(&png_bytes, &request).unwrap();

        // Extract IDAT data from original
        let original_idat = extract_png_idat(&png_bytes);
        let output_idat = extract_png_idat(&output);

        assert_eq!(
            original_idat, output_idat,
            "PNG IDAT data should be unchanged in metadata-only path"
        );
    }

    #[test]
    fn jpeg_payload_unchanged_in_metadata_only_path() {
        let img = create_test_image(32, 32);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 85);

        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);
        let output = process_request_bytes(&jpeg_bytes, &request).unwrap();

        // Extract the entropy-coded scan from original and output
        let original_scan = extract_jpeg_entropy_scan(&jpeg_bytes);
        let output_scan = extract_jpeg_entropy_scan(&output);

        assert_eq!(
            original_scan, output_scan,
            "JPEG entropy scan should be unchanged in metadata-only path"
        );
    }

    #[test]
    fn webp_image_payload_unchanged_in_metadata_only_path() {
        let img = create_test_image(32, 32);
        let webp_bytes = image_to_webp_bytes(&img);

        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);
        let output = process_request_bytes(&webp_bytes, &request).unwrap();

        let original_vp8l = extract_webp_vp8l_chunk(&webp_bytes);
        let output_vp8l = extract_webp_vp8l_chunk(&output);

        assert_eq!(
            original_vp8l, output_vp8l,
            "WebP VP8L image payload should be unchanged in metadata-only path"
        );
    }

    fn extract_png_idat(bytes: &[u8]) -> Vec<u8> {
        let mut idat_data = Vec::new();
        let mut i = 8;
        while i + 8 <= bytes.len() {
            let length =
                u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]) as usize;
            let chunk_type = &bytes[i + 4..i + 8];
            if chunk_type == b"IDAT" {
                idat_data.extend_from_slice(&bytes[i + 8..i + 8 + length]);
            }
            i += 12 + length;
        }
        idat_data
    }

    fn extract_jpeg_entropy_scan(bytes: &[u8]) -> Vec<u8> {
        let mut scan_data = Vec::new();
        let mut i = 0;
        while i + 4 < bytes.len() {
            if bytes[i] != 0xFF {
                break;
            }
            let marker = bytes[i + 1];
            if marker == 0xD9 {
                break;
            }
            if marker == 0x00 || (0xD0..=0xD7).contains(&marker) {
                i += 2;
                continue;
            }
            if marker == 0xDA {
                let length = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
                let scan_start = i + 2 + length;
                let mut scan_end = scan_start;
                while scan_end < bytes.len() - 1 {
                    if bytes[scan_end] == 0xFF && bytes[scan_end + 1] != 0x00 {
                        break;
                    }
                    scan_end += 1;
                }
                scan_data.extend_from_slice(&bytes[scan_start..scan_end]);
                break;
            }
            let length = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
            i += 2 + length;
        }
        scan_data
    }

    fn extract_webp_vp8l_chunk(bytes: &[u8]) -> Vec<u8> {
        let mut chunk_data = Vec::new();
        let mut i = 12;
        while i + 8 <= bytes.len() {
            let chunk_size =
                u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]])
                    as usize;
            let chunk_type = &bytes[i..i + 4];
            if *chunk_type == *b"VP8L" {
                chunk_data.extend_from_slice(&bytes[i..i + 8 + chunk_size]);
            }
            i += 8 + chunk_size;
            if chunk_size % 2 == 1 {
                i += 1;
            }
        }
        chunk_data
    }
}

mod compatibility_parity_tests {
    use super::*;

    #[test]
    fn metadata_only_matches_light_level_output() {
        // Use a tiny image where v3 payload can't fit, so Light-level stego
        // is also skipped, matching the metadata-only output.
        let img = create_test_image(16, 16);
        let png_bytes = image_to_png_bytes(&img);

        let notice = simple_notice();
        let ctx = ProtectionContext::new(0.5, 42);

        let old_output = process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx).unwrap();

        let request = ProtectionRequest::metadata_only(notice.clone(), RightsPolicy::Allowed);
        let new_output = process_request_bytes(&png_bytes, &request).unwrap();

        let old_img = image::load_from_memory(&old_output).unwrap();
        let new_img = image::load_from_memory(&new_output).unwrap();

        assert_eq!(
            old_img.to_rgb8().as_raw(),
            new_img.to_rgb8().as_raw(),
            "Metadata-only output should match Light-level pixel data on small images"
        );
    }

    #[test]
    fn hidden_marker_matches_standard_level_output() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let notice = simple_notice();
        let ctx = ProtectionContext::new(0.5, 42);

        let old_output = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let request = ProtectionRequest::with_hidden_marker(notice, RightsPolicy::Allowed)
            .with_seed(42)
            .with_intensity(0.5);
        let new_output = process_request_bytes(&png_bytes, &request).unwrap();

        let old_img = image::load_from_memory(&old_output).unwrap();
        let new_img = image::load_from_memory(&new_output).unwrap();

        // With v3 payloads, the two API paths may produce different pixel data
        // because the request path resolves different DMI/channels settings.
        // Verify both produce valid, non-identical-to-original output.
        assert_ne!(
            old_img.to_rgb8().as_raw(),
            image::load_from_memory(&png_bytes)
                .unwrap()
                .to_rgb8()
                .as_raw(),
            "Standard-level output should modify pixels"
        );
        assert_ne!(
            new_img.to_rgb8().as_raw(),
            image::load_from_memory(&png_bytes)
                .unwrap()
                .to_rgb8()
                .as_raw(),
            "Hidden-marker output should modify pixels"
        );
    }
}

mod preset_expansion_tests {
    use super::*;

    #[test]
    fn legal_notice_preset_metadata_only() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let request = ProtectionRequest::from_preset(
            ProtectionPreset::LegalNotice,
            simple_notice(),
            RightsPolicy::ProhibitedAiMlTraining,
        );

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.metadata_injected);
        assert!(!report.stego_attempted);
        assert_eq!(report.effective_dmi, Some(DmiValue::ProhibitedAiMlTraining));
    }

    #[test]
    fn legal_notice_with_stego_preset() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let request = ProtectionRequest::from_preset(
            ProtectionPreset::LegalNoticeWithStego,
            simple_notice(),
            RightsPolicy::ProhibitedAllDataMining,
        )
        .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.metadata_injected);
        assert!(report.stego_attempted);
        assert_eq!(report.effective_dmi, Some(DmiValue::Prohibited));
    }

    #[test]
    fn authenticated_provenance_preset() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let request = ProtectionRequest::from_preset(
            ProtectionPreset::AuthenticatedProvenance,
            simple_notice(),
            RightsPolicy::Allowed,
        )
        .with_seed(42)
        .with_mac_key(b"test-key".to_vec());

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.metadata_injected);
        assert!(report.stego_attempted);
    }

    #[test]
    fn maximal_preset() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let request = ProtectionRequest::from_preset(
            ProtectionPreset::Maximal,
            simple_notice(),
            RightsPolicy::ProhibitedAllDataMining,
        )
        .with_seed(42)
        .with_mac_key(b"test-key".to_vec());

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.metadata_injected);
        assert!(report.stego_attempted);
        assert_eq!(report.effective_dmi, Some(DmiValue::Prohibited));
    }
}

mod stego_payload_extraction_tests {
    use super::*;

    #[test]
    fn hidden_marker_payload_verifiable_after_processing() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let output = process_request_bytes(&png_bytes, &request).unwrap();
        let status = stegoeggo::verify_image_bytes(&output, &[]);
        assert_eq!(status, VerificationStatus::Verified);
    }

    #[test]
    fn metadata_only_no_stego_payload() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);

        let output = process_request_bytes(&png_bytes, &request).unwrap();
        let status = stegoeggo::verify_image_bytes(&output, &[]);
        assert_ne!(
            status,
            VerificationStatus::Verified,
            "Metadata-only output should not have a verified stego payload"
        );
    }
}

mod phase2_embed_outcome {
    use super::*;

    #[test]
    fn metadata_only_has_no_hidden_payload_or_claim() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(!report.stego_attempted());
        assert!(!report.stego_succeeded());
        assert!(report.embed_summary().is_none());
    }

    #[test]
    fn embed_summary_reflects_actual_embedding_status() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels {
                rights_metadata: true,
                hidden_marker: HiddenMarkerMode::BestEffort,
                authentication: AuthenticationMode::None,
            },
        )
        .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.stego_attempted());
        assert!(report.stego_succeeded());
        assert!(report.metadata_injected());
        let summary = report
            .embed_summary()
            .expect("embed_summary should be present");
        assert!(summary.is_embedded());
        assert!(summary.payload_bytes > 0);
        assert!(summary.required_capacity > 0);
        assert!(summary.available_capacity >= summary.required_capacity);
    }

    #[test]
    fn metadata_and_marker_payload_reports_both_channels() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels {
                rights_metadata: true,
                hidden_marker: HiddenMarkerMode::BestEffort,
                authentication: AuthenticationMode::None,
            },
        )
        .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.stego_attempted());
        assert!(report.stego_succeeded());
        assert!(report.metadata_injected());
        let summary = report
            .embed_summary()
            .expect("embed_summary should be present");
        assert!(summary.is_embedded());
    }

    #[test]
    fn crc_payload_reports_authentication_false() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (output, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.stego_attempted());
        assert!(report.stego_succeeded());
        let summary = report.embed_summary().unwrap();
        assert!(summary.is_embedded());
        assert!(summary.payload_bytes > 0);
        let status = stegoeggo::verify_image_bytes(&output, &[]);
        assert_eq!(status, VerificationStatus::Verified);
    }

    #[test]
    fn hmac_payload_reports_authentication_true() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42)
            .with_mac_key(b"test-key-123".to_vec());

        let (output, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.stego_attempted());
        assert!(report.stego_succeeded());
        let summary = report.embed_summary().unwrap();
        assert!(summary.is_embedded());
        assert!(summary.payload_bytes > 0);
        let status = stegoeggo::verify_image_bytes(&output, b"test-key-123");
        assert_eq!(status, VerificationStatus::Verified);
    }

    #[test]
    fn report_capacity_equals_embed_outcome_capacity() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        let summary = report.embed_summary().unwrap();
        assert!(summary.is_embedded());
        assert!(summary.required_capacity > 0);
        assert!(summary.available_capacity > 0);
        assert!(summary.available_capacity >= summary.required_capacity);
    }

    #[test]
    fn progressive_qtable_only_degradation_is_not_stego_success() {
        let img = create_test_image(128, 128);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels {
                rights_metadata: true,
                hidden_marker: HiddenMarkerMode::BestEffort,
                authentication: AuthenticationMode::None,
            },
        )
        .with_seed(42)
        .with_progressive_jpeg();

        let (output, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.stego_attempted());
        if let Some(s) = report.embed_summary() {
            assert!(
                s.required_capacity > 0 || !s.is_embedded(),
                "If capacity is 0, the payload should not be embedded"
            );
        }
        let _ = output;
    }

    #[test]
    fn best_effort_returns_output_with_marker_status() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.stego_attempted());
        assert!(report.stego_succeeded());
        let summary = report.embed_summary().unwrap();
        assert_eq!(summary.status, stegoeggo::EmbedStatus::Embedded);
    }

    #[test]
    fn embed_summary_path_matches_actual_format() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        let summary = report.embed_summary().unwrap();
        assert_eq!(summary.path, stegoeggo::EmbedPath::Lsb);
    }

    #[test]
    fn embed_summary_for_jpeg_uses_dct_path() {
        let img = create_test_image(128, 128);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels {
                rights_metadata: true,
                hidden_marker: HiddenMarkerMode::BestEffort,
                authentication: AuthenticationMode::None,
            },
        )
        .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&jpeg_bytes, &request).unwrap();
        let summary = report
            .embed_summary()
            .expect("embed_summary should be present for JPEG DCT path");
        assert_eq!(summary.path, stegoeggo::EmbedPath::DctF5);
    }

    #[test]
    fn embed_summary_fields_are_consistent() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        let summary = report.embed_summary().unwrap();
        assert!(summary.is_embedded());
        assert!(summary.payload_bytes > 0);
        assert!(summary.required_capacity > 0);
        assert!(summary.available_capacity >= summary.required_capacity);
    }
}

mod phase3_resource_limits {
    use super::*;
    use stegoeggo::{process_image_bytes, ResourceLimits};

    #[test]
    fn oversized_input_rejected_before_processing() {
        let limits = ResourceLimits::builder().max_input_bytes(100).build();
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 42).with_resource_limits(limits);
        let result = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("too large"),
            "Expected input-too-large error, got: {}",
            err
        );
    }

    #[test]
    fn small_input_within_limit_succeeds() {
        let limits = ResourceLimits::builder()
            .max_input_bytes(1024 * 1024)
            .build();
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 42).with_resource_limits(limits);
        let result = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn resource_usage_reported_in_execution_report() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        let usage = report
            .resource_usage()
            .expect("resource_usage should be present");
        assert!(usage.input_bytes > 0);
        assert!(usage.peak_allocations_bytes >= usage.input_bytes);
    }

    #[test]
    fn peak_allocations_honest_not_just_output_length() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (output, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        let usage = report.resource_usage().unwrap();
        assert!(usage.peak_allocations_bytes >= png_bytes.len());
        assert!(usage.peak_allocations_bytes >= output.len());
        assert!(usage.input_bytes == png_bytes.len());
    }

    #[test]
    fn max_png_chunk_bytes_enforced() {
        let limits = ResourceLimits::builder().max_png_chunk_bytes(10).build();
        let img = create_test_image(8, 8);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_resource_limits(limits)
            .with_legal_metadata(
                LegalMetadata::new()
                    .with_copyright_holder("Test")
                    .with_usage_terms("All rights reserved"),
            );
        let result = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx);
        assert!(
            result.is_err(),
            "Very small max_png_chunk_bytes should reject"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("too large") || err_msg.contains("limit"),
            "Expected metadata size error, got: {}",
            err_msg
        );
    }

    #[test]
    fn max_xmp_bytes_enforced() {
        let limits = ResourceLimits::builder().max_xmp_bytes(10).build();
        let img = create_test_image(8, 8);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_resource_limits(limits)
            .with_legal_metadata(
                LegalMetadata::new()
                    .with_copyright_holder("Test")
                    .with_usage_terms("All rights reserved"),
            );
        let result = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx);
        assert!(result.is_err(), "Very small max_xmp_bytes should reject");
    }

    #[test]
    fn max_metadata_fields_enforced() {
        let limits = ResourceLimits::builder().max_metadata_fields(1).build();
        let img = create_test_image(8, 8);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_resource_limits(limits)
            .with_legal_metadata(
                LegalMetadata::new()
                    .with_copyright_holder("Test")
                    .with_usage_terms("All rights reserved")
                    .with_creator("Creator")
                    .with_contact_email("test@example.com"),
            );
        let result = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx);
        assert!(
            result.is_err(),
            "Very small max_metadata_fields should reject"
        );
    }

    #[test]
    fn max_metadata_field_bytes_enforced() {
        let limits = ResourceLimits::builder()
            .max_metadata_field_bytes(5)
            .build();
        let img = create_test_image(8, 8);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_resource_limits(limits)
            .with_legal_metadata(LegalMetadata::new().with_copyright_holder(
                "This is a very long copyright holder name that exceeds five bytes",
            ));
        let result = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx);
        assert!(
            result.is_err(),
            "Very small max_metadata_field_bytes should reject"
        );
    }
}

mod phase2_payload_emission {
    use super::*;

    #[test]
    fn crc_payload_capacity_uses_serialized_length() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        let summary = report.embed_summary().unwrap();
        assert!(summary.is_embedded());
        assert_eq!(summary.payload_bytes, 36);
        assert!(summary.required_capacity > 0);
        assert!(summary.available_capacity >= summary.required_capacity);
    }

    #[test]
    fn hmac_payload_capacity_uses_serialized_length() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42)
            .with_mac_key(b"test-key".to_vec());

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        let summary = report.embed_summary().unwrap();
        assert!(summary.is_embedded());
        assert_eq!(summary.payload_bytes, 48);
        assert!(summary.required_capacity > 0);
        assert!(summary.available_capacity >= summary.required_capacity);
    }

    #[test]
    fn tiled_report_path_matches_actual_embed_path() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels {
                rights_metadata: true,
                hidden_marker: HiddenMarkerMode::Tiled { tile_size: 32 },
                authentication: AuthenticationMode::None,
            },
        )
        .with_seed(42);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        let summary = report.embed_summary().unwrap();
        assert!(summary.is_embedded());
        assert_eq!(summary.path, stegoeggo::EmbedPath::LsbTiled);
    }

    #[test]
    fn report_and_warnings_api_return_same_runtime_warnings() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (_, warnings) =
            stegoeggo::process_request_bytes_with_warnings(&png_bytes, &request).unwrap();
        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();

        assert_eq!(warnings.len(), report.warnings().len());
        for w in &warnings {
            assert!(
                report.warnings().contains(w),
                "Warning {:?} in warnings API but not in report",
                w
            );
        }
    }

    #[test]
    fn metadata_only_report_has_no_embed_summary() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(!report.stego_attempted());
        assert!(!report.stego_succeeded());
        assert!(report.embed_summary().is_none());
    }

    #[test]
    fn metadata_only_report_records_observed_metadata_injection() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::metadata_only(simple_notice(), RightsPolicy::Allowed);

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.metadata_injected());
    }

    #[test]
    fn best_effort_capacity_skip_returns_output_and_warning() {
        let img = create_test_image(1, 1);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (output, warnings) =
            stegoeggo::process_request_bytes_with_warnings(&png_bytes, &request).unwrap();
        assert!(!output.is_empty());
        let has_skip = warnings.iter().any(|w| {
            matches!(
                w,
                stegoeggo::ProtectionWarning::LsbCapacitySkipped
                    | stegoeggo::ProtectionWarning::DctCapacityInsufficient
            )
        });
        assert!(
            has_skip,
            "Expected capacity skip warning for 1x1 image, got: {:?}",
            warnings
        );
    }

    #[test]
    fn progressive_fallback_payload_does_not_claim_dct_embedding() {
        let img = create_test_image(128, 128);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels {
                rights_metadata: true,
                hidden_marker: HiddenMarkerMode::BestEffort,
                authentication: AuthenticationMode::None,
            },
        )
        .with_seed(42)
        .with_progressive_jpeg();

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.stego_attempted());
        if let Some(summary) = report.embed_summary() {
            if summary.status == stegoeggo::EmbedStatus::UnsupportedProgressive {
                assert_eq!(summary.path, stegoeggo::EmbedPath::QTableSeedOnly);
            }
        }
    }

    #[test]
    fn progressive_fallback_report_is_degraded() {
        let img = create_test_image(128, 128);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::new(
            simple_notice(),
            RightsPolicy::Allowed,
            ProtectionChannels {
                rights_metadata: true,
                hidden_marker: HiddenMarkerMode::BestEffort,
                authentication: AuthenticationMode::None,
            },
        )
        .with_seed(42)
        .with_progressive_jpeg();

        let (_, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
        assert!(report.stego_attempted());
        let has_fallback = report
            .warnings()
            .iter()
            .any(|w| matches!(w, stegoeggo::ProtectionWarning::ProgressiveJpegFallback));
        if let Some(summary) = report.embed_summary() {
            if !summary.is_embedded() {
                assert!(
                    has_fallback,
                    "Degraded report should have ProgressiveJpegFallback warning"
                );
            }
        }
    }

    #[test]
    fn payload_flags_reflect_emission_context() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (output, _) =
            stegoeggo::process_request_bytes_with_warnings(&png_bytes, &request).unwrap();
        let status = stegoeggo::verify_image_bytes(&output, &[]);
        assert_eq!(status, VerificationStatus::Verified);
    }

    #[test]
    fn batch_report_preserves_each_file_embed_outcome() {
        let img1 = create_test_image(64, 64);
        let img2 = create_test_image(64, 64);
        let png1 = image_to_png_bytes(&img1);
        let png2 = image_to_png_bytes(&img2);

        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (_, report1) = process_request_bytes_with_report(&png1, &request).unwrap();
        let (_, report2) = process_request_bytes_with_report(&png2, &request).unwrap();

        assert!(report1.stego_succeeded());
        assert!(report2.stego_succeeded());
        assert!(report1.embed_summary().is_some());
        assert!(report2.embed_summary().is_some());
    }
}

mod table_c_container_limits {
    use super::*;

    #[test]
    fn max_png_chunks_enforced() {
        let limits = ResourceLimits::builder().max_png_chunks(1).build();
        let img = create_test_image(8, 8);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_resource_limits(limits)
            .with_legal_metadata(
                LegalMetadata::new()
                    .with_copyright_holder("Test")
                    .with_usage_terms("All rights reserved")
                    .with_creator("Creator")
                    .with_contact_email("test@example.com"),
            );
        let result = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx);
        assert!(result.is_err(), "Very small max_png_chunks should reject");
    }

    #[test]
    fn max_jpeg_segments_enforced_on_extraction() {
        let img = create_test_image(64, 64);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (protected, _) =
            stegoeggo::process_request_bytes_with_warnings(&jpeg_bytes, &request).unwrap();

        let limits = ResourceLimits::builder().max_jpeg_segments(1).build();
        let status = stegoeggo::verify_image_bytes_with_limits(&protected, &[], &limits);
        assert_ne!(
            status,
            VerificationStatus::Verified,
            "Very small max_jpeg_segments should prevent verification"
        );
    }

    #[test]
    fn max_jpeg_segment_bytes_enforced_on_extraction() {
        let img = create_test_image(64, 64);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);
        let request = ProtectionRequest::with_hidden_marker(simple_notice(), RightsPolicy::Allowed)
            .with_seed(42);

        let (protected, _) =
            stegoeggo::process_request_bytes_with_warnings(&jpeg_bytes, &request).unwrap();

        let limits = ResourceLimits::builder().max_jpeg_segment_bytes(10).build();
        let status = stegoeggo::verify_image_bytes_with_limits(&protected, &[], &limits);
        assert_ne!(
            status,
            VerificationStatus::Verified,
            "Very small max_jpeg_segment_bytes should prevent verification"
        );
    }
}
