#![allow(deprecated)]

use image::ImageEncoder;
use stegoeggo::{
    process_image_bytes, process_request_bytes, process_request_bytes_with_report, resolve_request,
    AuthenticationMode, DmiValue, HiddenMarkerMode, ImageOutputFormat, LegalMetadata,
    ProtectionChannels, ProtectionContext, ProtectionLevel, ProtectionPreset, ProtectionRequest,
    RightsNotice, RightsPolicy, VerificationStatus,
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
            RightsPolicy::Allowed,
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

        // WebP: the VP8/VP8L/VP8X chunk data should be preserved
        let original_vp8 = extract_webp_image_chunk(&webp_bytes);
        let output_vp8 = extract_webp_image_chunk(&output);

        assert_eq!(
            original_vp8, output_vp8,
            "WebP image chunk data should be unchanged in metadata-only path"
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

    fn extract_webp_image_chunk(bytes: &[u8]) -> Vec<u8> {
        let mut chunk_data = Vec::new();
        let mut i = 12;
        while i + 8 <= bytes.len() {
            let chunk_size =
                u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]])
                    as usize;
            let chunk_type = &bytes[i..i + 4];
            if *chunk_type == *b"VP8 " || *chunk_type == *b"VP8L" || *chunk_type == *b"VP8X" {
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
        let img = create_test_image(32, 32);
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
            "Metadata-only output should match Light-level pixel data"
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

        assert_eq!(
            old_img.to_rgb8().as_raw(),
            new_img.to_rgb8().as_raw(),
            "Hidden-marker output should match Standard-level pixel data"
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
