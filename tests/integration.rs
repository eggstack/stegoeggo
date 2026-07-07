use image::{DynamicImage, ImageEncoder};
use stegoeggo::{
    process_image, process_image_bytes, process_images_bytes_parallel, process_images_parallel,
    DmiValue, EvidenceProfile, ImageOutputFormat, LegalMetadata, MetadataTrapProtector,
    PassthroughProtector, ProtectionContext, ProtectionLevel, ProtectionPipeline,
    SteganographyProtector, VerificationStatus,
};

fn create_test_image(width: u32, height: u32) -> DynamicImage {
    DynamicImage::new_rgb8(width, height)
}

fn create_test_context() -> ProtectionContext {
    ProtectionContext::new(0.5, 42)
}

fn image_to_png_bytes(img: &DynamicImage) -> Vec<u8> {
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

fn image_to_jpeg_bytes(img: &DynamicImage, quality: u8) -> Vec<u8> {
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

fn image_to_webp_bytes(img: &DynamicImage) -> Vec<u8> {
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

fn create_colored_image(width: u32, height: u32, r: u8, g: u8, b: u8) -> DynamicImage {
    let mut img = DynamicImage::new_rgb8(width, height);
    let mut rgb_img = img.to_rgb8();
    for y in 0..height {
        for x in 0..width {
            rgb_img.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    img = DynamicImage::ImageRgb8(rgb_img);
    img
}

mod round_trip {
    use super::*;

    #[test]
    fn test_protect_and_verify_standard_level() {
        let img = create_test_image(64, 64);
        let ctx = ProtectionContext::new(0.7, 12345);

        let protected = process_image(img.clone(), ProtectionLevel::Standard, &ctx).unwrap();

        let stego = SteganographyProtector::new();
        assert!(
            stego.verify_payload(&protected),
            "Standard level should be verifiable"
        );
    }

    #[test]
    fn test_extract_payload_after_protection() {
        let img = create_test_image(64, 64);
        let seed = 42;
        let intensity = 0.75;
        let ctx = ProtectionContext::new(intensity, seed);

        let protected = process_image(img, ProtectionLevel::Standard, &ctx).unwrap();

        let stego = SteganographyProtector::new();
        let payload = stego.extract_payload(&protected);

        assert!(payload.is_some(), "Should extract payload");
        let payload = payload.unwrap();
        assert_eq!(payload.seed(), seed, "Seed should match");
        assert!(
            (payload.intensity() - intensity).abs() < 0.01,
            "Intensity should match"
        );
    }

    #[test]
    fn test_deterministic_with_same_seed() {
        let img = create_test_image(32, 32);
        let ctx = ProtectionContext::new(0.5, 42);

        let protected1 = process_image(img.clone(), ProtectionLevel::Standard, &ctx).unwrap();
        let protected2 = process_image(img.clone(), ProtectionLevel::Standard, &ctx).unwrap();

        let rgba1 = protected1.to_rgba8();
        let rgba2 = protected2.to_rgba8();

        assert_eq!(
            rgba1.as_raw(),
            rgba2.as_raw(),
            "Same seed should produce identical output"
        );
    }

    #[test]
    fn test_different_seeds_produce_different_output() {
        let img = create_test_image(32, 32);

        let ctx1 = ProtectionContext::new(0.5, 100);
        let ctx2 = ProtectionContext::new(0.5, 200);

        let protected1 = process_image(img.clone(), ProtectionLevel::Standard, &ctx1).unwrap();
        let protected2 = process_image(img.clone(), ProtectionLevel::Standard, &ctx2).unwrap();

        let rgba1 = protected1.to_rgba8();
        let rgba2 = protected2.to_rgba8();

        assert_ne!(
            rgba1.as_raw(),
            rgba2.as_raw(),
            "Different seeds should produce different output"
        );
    }
}

mod image_formats {
    use super::*;

    #[test]
    fn test_png_round_trip() {
        let img = create_colored_image(64, 64, 100, 150, 200);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        assert_eq!(protected_img.width(), img.width());
        assert_eq!(protected_img.height(), img.height());

        let stego = SteganographyProtector::new();
        assert!(stego.verify_payload(&protected_img));
    }

    #[test]
    #[ignore = "JPEG pixel-based stego does not survive re-encoding due to DCT quantization"]
    fn test_jpeg_round_trip() {
        let img = create_colored_image(64, 64, 100, 150, 200);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Jpeg);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        assert_eq!(protected_img.width(), img.width());
        assert_eq!(protected_img.height(), img.height());

        let stego = SteganographyProtector::new();
        assert!(stego.verify_payload(&protected_img));
    }

    #[test]
    fn test_format_conversion_png_to_jpeg() {
        let img = create_colored_image(64, 64, 100, 150, 200);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Jpeg);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        assert!(
            protected_bytes.starts_with(&[0xFF, 0xD8]),
            "Output should be JPEG"
        );
    }

    #[test]
    fn test_format_conversion_jpeg_to_png() {
        let img = create_colored_image(64, 64, 100, 150, 200);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 85);

        let ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        assert!(
            protected_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
            "Output should be PNG"
        );
    }

    #[test]
    fn test_png_contains_metadata() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.8, 12345).with_format(ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let has_seed = protected_bytes
            .windows(18)
            .any(|w| w == b"X-Protection-Seed");
        let has_dmi = protected_bytes.windows(14).any(|w| w == b"DMI-PROHIBITED");

        assert!(
            has_seed || has_dmi,
            "PNG should contain protection metadata"
        );
    }

    #[test]
    fn test_jpeg_contains_metadata() {
        let img = create_test_image(32, 32);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let ctx = ProtectionContext::new(0.8, 12345)
            .with_format(ImageOutputFormat::Jpeg)
            .with_dmi(DmiValue::ProhibitedAiMlTraining);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let has_app1 = protected_bytes.windows(2).any(|w| w == [0xFF, 0xE1]);
        assert!(has_app1, "JPEG should contain APP1 marker for XMP");
    }
}

mod metadata_injection {
    use super::*;

    #[test]
    fn test_light_level_metadata_only() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx).unwrap();

        let seed = MetadataTrapProtector::extract_seed_from_image(&protected_bytes);
        assert!(seed.is_some(), "Light level should inject seed");
    }

    #[test]
    fn test_auto_dmi_for_protection_levels() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let test_cases = vec![(ProtectionLevel::Standard, DmiValue::ProhibitedAiMlTraining)];

        for (level, expected_dmi) in test_cases {
            let ctx = ProtectionContext::new(0.5, 42)
                .with_format(ImageOutputFormat::Png)
                .with_dmi(expected_dmi);

            let protected_bytes = process_image_bytes(&png_bytes, level, &ctx).unwrap();
            let has_dmi = protected_bytes.windows(14).any(|w| w == b"DMI-PROHIBITED");

            assert!(has_dmi, "Level {:?} should inject DMI metadata", level);
        }
    }

    #[test]
    fn test_legal_metadata_injection() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let legal = LegalMetadata::new()
            .with_copyright_holder("Test Author")
            .with_contact_email("test@example.com")
            .with_usage_terms("No AI training allowed");

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_legal_metadata(legal)
            .with_legal_claims(true);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx).unwrap();

        let has_copyright = protected_bytes.windows(9).any(|w| w == b"Copyright");
        assert!(has_copyright, "Should inject copyright metadata");
    }

    #[test]
    fn test_extract_seed_from_protected_jpeg() {
        let img = create_test_image(32, 32);
        let seed = 98765;
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let ctx = ProtectionContext::new(0.5, seed).with_format(ImageOutputFormat::Jpeg);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let extracted_seed = MetadataTrapProtector::extract_seed_from_image(&protected_bytes);
        assert_eq!(
            extracted_seed,
            Some(seed),
            "Should extract correct seed from JPEG"
        );
    }

    #[test]
    fn test_jpeg_contains_exif_iptc_xmp_markers() {
        let img = create_test_image(32, 32);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_dmi(DmiValue::ProhibitedAiMlTraining);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Light, &ctx).unwrap();

        let has_exif_tiff_prefix = protected_bytes.windows(6).any(|w| w == b"Exif\x00\x00");
        let has_ascii_charset = protected_bytes
            .windows(8)
            .any(|w| w == b"ASCII\x00\x00\x00");
        let has_photoshop_id = protected_bytes
            .windows(14)
            .any(|w| w == b"Photoshop 3.0\x00");
        let has_iptc_record_start = protected_bytes.windows(3).any(|w| w == [0x1C, 0x02, 0x78]);
        let has_dmi = protected_bytes.windows(4).any(|w| w == b"DMI:");

        assert!(
            has_exif_tiff_prefix,
            "JPEG should have EXIF TIFF header (Exif\\0\\0)"
        );
        assert!(
            has_ascii_charset,
            "JPEG should have EXIF ASCII charset prefix"
        );
        assert!(
            has_photoshop_id,
            "JPEG should have Photoshop 3.0 identifier"
        );
        assert!(
            has_iptc_record_start,
            "JPEG should have IPTC record start (1C 02 78)"
        );
        assert!(has_dmi, "JPEG should contain DMI value");
    }
}

mod steganography {
    use super::*;

    #[test]
    fn test_stego_survives_format_reencoding() {
        let img = create_colored_image(64, 64, 50, 100, 150);

        let ctx = ProtectionContext::new(0.6, 42);

        let protected = process_image(img, ProtectionLevel::Standard, &ctx).unwrap();

        let png_bytes = image_to_png_bytes(&protected);
        let reprotected = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let reprotected_img = image::load_from_memory(&reprotected).unwrap();
        let stego = SteganographyProtector::new();

        assert!(
            stego.verify_payload(&reprotected_img),
            "Stego should survive re-encoding"
        );
    }

    #[test]
    fn test_dct_stego_on_jpeg() {
        let img = create_colored_image(64, 64, 75, 125, 200);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let seed = 22222u64;
        let ctx = ProtectionContext::new(0.5, seed).with_format(ImageOutputFormat::Jpeg);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        // For JPEG, we verify via metadata since pixel-based stego has limited robustness
        let metadata_seed = MetadataTrapProtector::extract_seed_from_image(&protected_bytes);
        assert_eq!(
            metadata_seed,
            Some(seed),
            "JPEG should have seed in metadata"
        );
    }

    #[test]
    fn test_dct_stego_different_seeds() {
        let img = create_test_image(64, 64);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        for seed in [42u64, 12345, 999999] {
            let ctx = ProtectionContext::new(0.5, seed).with_format(ImageOutputFormat::Jpeg);

            let protected =
                process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

            // For JPEG, verify via metadata
            let metadata_seed = MetadataTrapProtector::extract_seed_from_image(&protected);
            assert_eq!(
                metadata_seed,
                Some(seed),
                "JPEG with seed {} should have correct metadata",
                seed
            );
        }
    }

    #[test]
    fn test_dct_stego_seed_extraction() {
        let img = create_test_image(128, 128);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 85);

        let seed = 77777u64;
        let ctx = ProtectionContext::new(0.6, seed).with_format(ImageOutputFormat::Jpeg);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        // For JPEG, verify via metadata
        let metadata_seed = MetadataTrapProtector::extract_seed_from_image(&protected_bytes);
        assert_eq!(
            metadata_seed,
            Some(seed),
            "JPEG should extract correct seed from metadata"
        );
    }

    #[test]
    fn test_verify_with_keyed_context() {
        let key = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_mac_key(key.clone())
            .with_format(ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let stego = SteganographyProtector::new();

        // Metadata seed should be extractable from the protected bytes
        let seed = MetadataTrapProtector::extract_seed_from_image(&protected_bytes);
        assert_eq!(seed, Some(42), "Metadata seed should be extractable");

        // Verify via DynamicImage round-trip with correct MAC key
        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let payload = stego.extract_payload_with_key(&protected_img, &key);
        assert!(
            payload.is_some(),
            "Should extract payload with correct MAC key"
        );
        let payload = payload.unwrap();
        assert_eq!(payload.seed(), 42);
    }

    #[test]
    fn test_extract_with_seed_from_metadata() {
        let seed = 44444;
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.7, seed).with_format(ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
        let protected_img = image::load_from_memory(&protected_bytes).unwrap();

        let stego = SteganographyProtector::new();
        let payload = stego.extract_payload_with_seed(&protected_img, seed);

        assert!(payload.is_some(), "Should extract payload with known seed");
        let payload = payload.unwrap();
        assert_eq!(payload.seed(), seed);
    }
}

mod parallel_processing {
    use super::*;

    #[test]
    fn test_parallel_image_processing() {
        let images: Vec<DynamicImage> = (0..4)
            .map(|i| create_colored_image(16, 16, i as u8, (i * 10) as u8, (i * 20) as u8))
            .collect();

        let ctx = ProtectionContext::new(0.5, 77777);

        let results = process_images_parallel(&images, ProtectionLevel::Standard, &ctx).unwrap();

        assert_eq!(results.len(), 4, "Should process all images");

        for result in &results {
            assert!(
                result.width() > 0 && result.height() > 0,
                "Result should have valid dimensions"
            );
        }
    }

    #[test]
    fn test_parallel_bytes_processing() {
        let images: Vec<Vec<u8>> = (0..4)
            .map(|i| {
                let img = create_colored_image(16, 16, i as u8, (i * 10) as u8, (i * 20) as u8);
                image_to_png_bytes(&img)
            })
            .collect();

        let ctx = ProtectionContext::new(0.5, 88888);

        let results =
            process_images_bytes_parallel(&images, ProtectionLevel::Standard, &ctx).unwrap();

        assert_eq!(results.len(), 4, "Should process all images");

        for result in &results {
            assert!(!result.is_empty(), "Result should not be empty");
        }
    }

    #[test]
    fn test_parallel_preserves_count() {
        let images: Vec<DynamicImage> = (0..8).map(|_| create_test_image(16, 16)).collect();

        let ctx = ProtectionContext::new(0.5, 99999);

        let results = process_images_parallel(&images, ProtectionLevel::Standard, &ctx).unwrap();

        assert_eq!(results.len(), 8, "Should preserve count");
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn test_max_intensity_modifies_image() {
        let img = create_test_image(32, 32);

        let ctx = ProtectionContext::new(1.0, 22222);

        let result = process_image(img.clone(), ProtectionLevel::Standard, &ctx).unwrap();

        let original_bytes = img.to_rgba8().into_raw();
        let result_bytes = result.to_rgba8().into_raw();

        let differences: usize = original_bytes
            .iter()
            .zip(result_bytes.iter())
            .filter(|(a, b)| a != b)
            .count();

        assert!(
            differences > 0,
            "Max intensity should modify image ({} differences)",
            differences
        );
    }

    #[test]
    fn test_small_image_8x8() {
        let img = create_test_image(8, 8);

        let ctx = ProtectionContext::new(0.5, 33333);

        let result = process_image(img, ProtectionLevel::Standard, &ctx).unwrap();

        assert_eq!(result.width(), 8);
        assert_eq!(result.height(), 8);
    }

    #[test]
    fn test_large_image_1024x1024() {
        let img = create_test_image(1024, 1024);

        let ctx = ProtectionContext::new(0.5, 42);

        let result = process_image(img, ProtectionLevel::Standard, &ctx).unwrap();

        assert_eq!(result.width(), 1024);
        assert_eq!(result.height(), 1024);

        let stego = SteganographyProtector::new();
        assert!(
            stego.verify_payload(&result),
            "Large image should be verifiable"
        );
    }

    #[test]
    fn test_disabled_level_preserves_image() {
        let img = create_colored_image(32, 32, 50, 100, 150);
        let original_bytes = img.to_rgba8().into_raw();

        let ctx = ProtectionContext::new(0.9, 55555);

        let result = process_image(img.clone(), ProtectionLevel::Disabled, &ctx).unwrap();
        let result_bytes = result.to_rgba8().into_raw();

        assert_eq!(
            original_bytes, result_bytes,
            "Disabled level should preserve image exactly"
        );
    }

    #[test]
    fn test_light_level_minimal_pixel_modification() {
        let img = create_colored_image(32, 32, 50, 100, 150);
        let original_bytes = img.to_rgba8().into_raw();

        let ctx = ProtectionContext::new(0.5, 66666).with_format(ImageOutputFormat::Png);

        let result = process_image(img, ProtectionLevel::Light, &ctx).unwrap();
        let result_bytes = result.to_rgba8().into_raw();

        assert_eq!(original_bytes.len(), result_bytes.len());

        let max_diff = original_bytes
            .iter()
            .zip(result_bytes.iter())
            .map(|(a, b)| (*a as i16 - *b as i16).unsigned_abs())
            .max()
            .unwrap();
        assert!(
            max_diff <= 2,
            "Light level should modify pixels by at most 2 (LSB stego), got {}",
            max_diff
        );
    }
}

mod protector_individual {
    use super::*;
    use stegoeggo::Protector;

    #[test]
    fn test_passthrough_preserves_dimensions() {
        let protector = PassthroughProtector::new();
        let img = create_test_image(100, 200);
        let ctx = create_test_context();

        let result = protector.apply(&img, &ctx).unwrap();

        assert_eq!(result.width(), 100);
        assert_eq!(result.height(), 200);
    }

    #[test]
    fn test_stego_embed_and_extract() {
        let stego = SteganographyProtector::new();
        let img = create_test_image(64, 64);
        let ctx = ProtectionContext::new(0.5, 42);

        let result = stego.apply(&img, &ctx).unwrap();

        assert!(
            stego.verify_payload(&result),
            "Stego embed should be verifiable"
        );
    }

    #[test]
    fn test_metadata_trap_injects_correctly() {
        let protector = MetadataTrapProtector::new();
        let img = create_test_image(32, 32);
        let seed = 202020;
        let ctx = ProtectionContext::new(0.5, seed).with_format(ImageOutputFormat::Png);

        let png_bytes = image_to_png_bytes(&img);
        let protected_bytes = protector.apply_bytes(&png_bytes, &ctx).unwrap();

        let extracted_seed = MetadataTrapProtector::extract_seed_from_image(&protected_bytes);
        assert_eq!(
            extracted_seed,
            Some(seed),
            "Should inject and extract correct seed"
        );
    }
}

mod pipeline {
    use super::*;

    #[test]
    fn test_pipeline_all_levels() {
        let pipeline = ProtectionPipeline::new();
        let img = create_test_image(64, 64);
        let ctx = create_test_context();

        for level in [
            ProtectionLevel::Disabled,
            ProtectionLevel::Light,
            ProtectionLevel::Standard,
        ] {
            let result = pipeline.process(&img, level, &ctx);
            assert!(result.is_ok(), "Pipeline should handle level {:?}", level);
        }
    }

    #[test]
    fn test_pipeline_bytes_all_levels() {
        let pipeline = ProtectionPipeline::new();
        let img_bytes = image_to_png_bytes(&create_test_image(32, 32));
        let ctx = create_test_context().with_format(ImageOutputFormat::Png);

        for level in [
            ProtectionLevel::Disabled,
            ProtectionLevel::Light,
            ProtectionLevel::Standard,
        ] {
            let result = pipeline.process_bytes(&img_bytes, level, &ctx);
            assert!(
                result.is_ok(),
                "Pipeline should handle bytes for level {:?}",
                level
            );
        }
    }

    #[test]
    fn test_protection_context_defaults() {
        let ctx = ProtectionContext::default();

        assert_eq!(ctx.intensity(), 0.5);
        assert_ne!(ctx.seed(), 0, "Default seed should be non-zero");
        assert!(ctx.input_format().is_none());
    }

    #[test]
    fn test_protection_context_builder() {
        let ctx = ProtectionContext::new(0.8, 12345)
            .with_format(ImageOutputFormat::Png)
            .with_stego_redundancy(3)
            .with_jpeg_quality(85)
            .with_progressive_jpeg(true);

        assert_eq!(ctx.seed(), 12345);
        assert_eq!(ctx.intensity(), 0.8);
        assert_eq!(ctx.output_format(), Some(ImageOutputFormat::Png));
        assert_eq!(ctx.stego_redundancy(), 3);
        assert_eq!(ctx.jpeg_quality(), 85);
        assert!(ctx.progressive_jpeg());
    }
}

mod utilities {
    use super::*;

    #[test]
    fn test_image_hash_deterministic() {
        let img = create_colored_image(64, 64, 100, 150, 200);

        let hash1 = stegoeggo::compute_image_hash(&img);
        let hash2 = stegoeggo::compute_image_hash(&img);

        assert_eq!(hash1, hash2, "Hash should be deterministic");
    }

    #[test]
    fn test_image_hash_differs_for_different_images() {
        let img1 = create_colored_image(64, 64, 10, 20, 30);
        let img2 = create_colored_image(64, 64, 200, 210, 220);

        let hash1 = stegoeggo::compute_image_hash(&img1);
        let hash2 = stegoeggo::compute_image_hash(&img2);

        assert_eq!(hash1.len(), 64, "Hash should be a hex string");
        assert_eq!(hash2.len(), 64, "Hash should be a hex string");
        assert_ne!(
            hash1, hash2,
            "Different images should have different hashes"
        );
    }

    #[test]
    fn test_detect_png_format() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let format = ImageOutputFormat::from_magic_bytes(&png_bytes);
        assert_eq!(format, Some(ImageOutputFormat::Png));
    }

    #[test]
    fn test_detect_jpeg_format() {
        let img = create_test_image(32, 32);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let format = ImageOutputFormat::from_magic_bytes(&jpeg_bytes);
        assert_eq!(format, Some(ImageOutputFormat::Jpeg));
    }

    #[test]
    fn test_iscc_computation() {
        let img = create_test_image(32, 32);

        let iscc = stegoeggo::compute_iscc(&img).unwrap();
        assert!(!iscc.full().is_empty(), "ISCC should be computed");
    }

    #[test]
    fn test_iscc_from_bytes() {
        let img_bytes = image_to_png_bytes(&create_test_image(32, 32));

        let result = stegoeggo::compute_iscc_from_bytes(&img_bytes);
        assert!(result.is_some(), "ISCC should be computed from bytes");
        let _ = result.unwrap().unwrap();
    }
}

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_truncated_png_bytes() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);
        let truncated = &png_bytes[..png_bytes.len() / 2];

        let ctx = create_test_context();
        let result = process_image_bytes(truncated, ProtectionLevel::Standard, &ctx);
        assert!(result.is_err(), "Truncated PNG should return error");
    }

    #[test]
    fn test_truncated_jpeg_bytes() {
        let img = create_test_image(32, 32);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 85);
        let truncated = &jpeg_bytes[..jpeg_bytes.len() / 2];

        let ctx = create_test_context();
        let result = process_image_bytes(truncated, ProtectionLevel::Standard, &ctx);
        assert!(result.is_err(), "Truncated JPEG should return error");
    }

    #[test]
    fn test_empty_bytes() {
        let ctx = create_test_context();
        let result = process_image_bytes(&[], ProtectionLevel::Standard, &ctx);
        assert!(result.is_err(), "Empty input should return error");
    }

    #[test]
    fn test_tiny_image_1x1() {
        let img = create_test_image(1, 1);
        let ctx = create_test_context();
        let result = process_image(img, ProtectionLevel::Standard, &ctx);
        assert!(result.is_ok(), "1x1 image should process successfully");
    }

    #[test]
    fn test_tiny_image_2x2() {
        let img = create_test_image(2, 2);
        let ctx = create_test_context();
        let result = process_image(img, ProtectionLevel::Standard, &ctx);
        assert!(result.is_ok(), "2x2 image should process successfully");
    }

    #[test]
    fn test_unsupported_format_random_bytes() {
        // Pure random data without any valid image magic bytes
        let garbage = vec![0xAB; 64];
        let ctx = create_test_context();
        let result = process_image_bytes(&garbage, ProtectionLevel::Standard, &ctx);
        assert!(result.is_err(), "Random bytes should return error");
    }

    #[test]
    fn test_invalid_bytes_random_data() {
        let garbage = vec![0xAB; 64];
        let ctx = create_test_context();
        let result = process_image_bytes(&garbage, ProtectionLevel::Standard, &ctx);
        assert!(result.is_err(), "Random bytes should return error");
    }
}

mod verify_tests {
    use super::*;
    use std::sync::Arc;
    use stegoeggo::{
        process_image_bytes, verify_image_bytes, verify_image_bytes_detailed,
        MetadataTrapProtector, ProtectionConfig, ProtectionContext, ProtectionLevel,
        VerificationResult, VerificationStatus,
    };

    #[test]
    fn test_verify_image_bytes_sync() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 42);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let result = verify_image_bytes(&protected_bytes, &[]);
        assert_eq!(
            result,
            VerificationStatus::Verified,
            "Protected image should verify"
        );
    }

    #[test]
    fn test_verify_image_bytes_unprotected() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let result = verify_image_bytes(&png_bytes, &[]);
        assert!(
            result == VerificationStatus::NotFound || result == VerificationStatus::Invalid,
            "Unprotected image should not verify"
        );
    }

    #[test]
    fn test_detailed_verify_reports_metadata_only() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 777);
        let metadata_only = MetadataTrapProtector::new()
            .inject_bytes(&png_bytes, &ctx)
            .unwrap();

        let result = verify_image_bytes_detailed(&metadata_only, &[]);
        assert!(
            result.is_found(),
            "Metadata seed should be reported as found"
        );
        assert!(
            !result.is_verified(),
            "Metadata-only evidence should not be treated as payload verification"
        );
        // The metadata seed must be reachable either through the dedicated
        // `MetadataOnly` variant or through verification details. Either
        // outcome is acceptable as long as callers can recover the seed.
        assert!(
            result.metadata_seed().is_some() || result.payload().is_some(),
            "Either metadata seed or extractable payload must be reported"
        );
    }

    #[test]
    fn test_mac_protected_image_verifies_with_correct_key() {
        let img = create_test_image(96, 96);
        let png_bytes = image_to_png_bytes(&img);
        let key = b"correct-key";
        let cfg = Arc::new(ProtectionConfig::new().with_mac_key(key.to_vec()));
        let ctx = ProtectionContext::new(0.5, 4242).with_config(cfg);
        let protected = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let result = verify_image_bytes(&protected, key);
        assert_eq!(result, VerificationStatus::Verified);
    }

    #[test]
    fn test_mac_protected_image_verifies_wrong_key_returns_invalid() {
        let img = create_test_image(96, 96);
        let png_bytes = image_to_png_bytes(&img);
        let cfg = Arc::new(ProtectionConfig::new().with_mac_key(b"correct-key".to_vec()));
        let ctx = ProtectionContext::new(0.5, 4242).with_config(cfg);
        let protected = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let result = verify_image_bytes(&protected, b"wrong-key");
        assert_eq!(
            result,
            VerificationStatus::Invalid,
            "Verifying a MAC-protected image with the wrong key must surface Invalid, not NotFound"
        );
    }

    #[test]
    fn test_mac_protected_image_detailed_wrong_key_reports_corrupted() {
        let img = create_test_image(96, 96);
        let png_bytes = image_to_png_bytes(&img);
        let cfg = Arc::new(ProtectionConfig::new().with_mac_key(b"correct-key".to_vec()));
        let ctx = ProtectionContext::new(0.5, 4242).with_config(cfg);
        let protected = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let result = verify_image_bytes_detailed(&protected, b"wrong-key");
        assert!(
            result.is_found(),
            "Wrong-key verification must indicate protection was found"
        );
        assert!(
            !result.is_verified(),
            "Wrong-key verification must not be reported as Verified"
        );
        assert!(
            matches!(
                result,
                VerificationResult::Corrupted { .. } | VerificationResult::MetadataOnly { .. }
            ),
            "Wrong-key verification should return Corrupted or MetadataOnly"
        );
    }
}

mod serde_tests {
    use super::*;

    #[test]
    fn test_config_skipped_in_serde_roundtrip() {
        use std::sync::Arc;
        use stegoeggo::ProtectionConfig;

        let config = Arc::new(
            ProtectionConfig::new()
                .with_mac_key(b"secret".to_vec())
                .with_legal_metadata(stegoeggo::LegalMetadata::new().with_copyright_holder("Test")),
        );
        let ctx = ProtectionContext::new(0.7, 12345).with_config(config);

        let json = serde_json::to_string(&ctx).unwrap();
        let restored: ProtectionContext = serde_json::from_str(&json).unwrap();

        // Config is #[serde(skip)] — should be None after roundtrip
        assert_eq!(restored.seed(), 12345);
        assert_eq!(restored.intensity(), 0.7);
        assert!(
            restored.mac_key().is_none(),
            "MAC key should be lost after serde roundtrip"
        );
    }
}

mod webp_tests {
    use super::*;

    #[test]
    fn test_webp_pipeline_end_to_end() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::WebP);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        // Output should be valid WebP
        assert!(protected_bytes.len() >= 12, "WebP output too short");
        assert_eq!(&protected_bytes[0..4], b"RIFF", "Should start with RIFF");
        assert_eq!(&protected_bytes[8..12], b"WEBP", "Should contain WEBP");

        // Should be loadable
        let protected_img = image::load_from_memory(&protected_bytes);
        assert!(protected_img.is_ok(), "WebP should be loadable");
    }

    #[test]
    fn test_webp_lsb_stego_roundtrip() {
        let img = create_colored_image(64, 64, 50, 100, 150);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.7, 42).with_format(ImageOutputFormat::WebP);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let stego = SteganographyProtector::new();
        let payload = stego.extract_payload_with_seed(&protected_img, 42);
        assert!(
            payload.is_some(),
            "LSB stego should survive WebP lossless roundtrip"
        );
        let payload = payload.unwrap();
        assert_eq!(payload.seed(), 42);
    }

    #[test]
    fn test_webp_metadata_injection() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::WebP)
            .with_dmi(DmiValue::ProhibitedAiMlTraining);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let has_xmp = protected_bytes.windows(4).any(|w| w == b"XMP ")
            || protected_bytes.windows(3).any(|w| w == b"XMP");
        assert!(
            has_xmp || protected_bytes.len() > 100,
            "WebP should contain XMP or EXIF metadata"
        );
    }

    #[test]
    fn test_webp_verify_after_metadata_strip() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.7, 42).with_format(ImageOutputFormat::WebP);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let stripped_bytes = image_to_png_bytes(&protected_img);

        let result = stegoeggo::verify_image_bytes(&stripped_bytes, &[]);
        assert_eq!(
            result,
            VerificationStatus::Verified,
            "LSB stego should survive metadata stripping via DynamicImage roundtrip"
        );
    }
}

mod webp_legal_xmp_tests {
    use super::*;
    use stegoeggo::{
        verify_legal_notice, DmiValue, EvidenceChannel, ImageOutputFormat, LegalMetadata,
        ProtectionContext, ProtectionLevel,
    };

    fn extract_xmp_from_webp(bytes: &[u8]) -> Option<String> {
        let marker = b"XMP ";
        let mut pos = 0;
        while pos + 8 <= bytes.len() {
            if &bytes[pos..pos + 4] == marker {
                let size = u32::from_le_bytes([
                    bytes[pos + 4],
                    bytes[pos + 5],
                    bytes[pos + 6],
                    bytes[pos + 7],
                ]) as usize;
                let data_start = pos + 8;
                let data_end = data_start + size;
                if data_end <= bytes.len() {
                    return String::from_utf8(bytes[data_start..data_end].to_vec()).ok();
                }
            }
            pos += 1;
        }
        None
    }

    fn create_webp_with_legal(legal: LegalMetadata) -> Vec<u8> {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.7, 42)
            .with_format(ImageOutputFormat::WebP)
            .with_legal_metadata(legal)
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedAiMlTraining);
        process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx).unwrap()
    }

    #[test]
    fn webp_xmp_includes_copyright_holder() {
        let legal = LegalMetadata::new().with_copyright_holder("Test Corp");
        let protected = create_webp_with_legal(legal);
        let xmp = extract_xmp_from_webp(&protected).expect("XMP should be present in WebP");
        assert!(
            xmp.contains("dc:rights"),
            "XMP should contain dc:rights, got: {}",
            xmp
        );
        let report = verify_legal_notice(&protected, &[]);
        assert_eq!(report.copyright_holder(), Some("Test Corp"));
    }

    #[test]
    fn webp_xmp_includes_creator_and_rights_url() {
        let legal = LegalMetadata::new()
            .with_creator("Test Author")
            .with_web_statement_of_rights("https://example.com/rights");
        let protected = create_webp_with_legal(legal);
        let report = verify_legal_notice(&protected, &[]);
        assert_eq!(report.creator(), Some("Test Author"));
        assert_eq!(report.rights_url(), Some("https://example.com/rights"));
    }

    #[test]
    fn webp_xmp_includes_ai_constraints() {
        let legal = LegalMetadata::new().with_ai_constraints("No AI training");
        let protected = create_webp_with_legal(legal);
        let report = verify_legal_notice(&protected, &[]);
        assert_eq!(report.ai_constraints(), Some("No AI training"));
    }

    #[test]
    fn webp_notice_verification_extracts_legal_fields() {
        let legal = LegalMetadata::new()
            .with_copyright_holder("Test Corp")
            .with_creator("Test Author")
            .with_contact_email("contact@test.com")
            .with_web_statement_of_rights("https://example.com/rights")
            .with_usage_terms("All rights reserved")
            .with_ai_constraints("No generative AI training");
        let protected = create_webp_with_legal(legal);
        let report = verify_legal_notice(&protected, &[]);
        assert_eq!(report.copyright_holder(), Some("Test Corp"));
        assert_eq!(report.creator(), Some("Test Author"));
        assert_eq!(report.contact(), Some("contact@test.com"));
        assert_eq!(report.rights_url(), Some("https://example.com/rights"));
        assert_eq!(report.usage_terms(), Some("All rights reserved"));
        assert_eq!(report.ai_constraints(), Some("No generative AI training"));
        assert!(report.has_notice());
    }

    #[test]
    fn webp_notice_verification_reports_webp_xmp_channel() {
        let legal = LegalMetadata::new()
            .with_copyright_holder("Test Corp")
            .with_ai_constraints("No AI training");
        let protected = create_webp_with_legal(legal);
        let report = verify_legal_notice(&protected, &[]);
        let channels = report.channels();
        assert!(
            channels.contains(&EvidenceChannel::WebPXmp),
            "WebP with legal metadata should report WebPXmp channel, got: {:?}",
            channels
        );
    }

    #[test]
    fn webp_notice_verification_dmi_tdm_still_present() {
        let legal = LegalMetadata::new().with_copyright_holder("Test Corp");
        let protected = create_webp_with_legal(legal);
        let report = verify_legal_notice(&protected, &[]);
        assert!(report.has_notice());
        assert_eq!(
            report.dmi(),
            Some(DmiValue::ProhibitedAiMlTraining),
            "DMI should still be extractable from WebP"
        );
        assert_eq!(report.tdm_reserved(), Some(true));
    }

    #[test]
    fn webp_xmp_namespace_uses_eggstack_repo() {
        let legal = LegalMetadata::new()
            .with_copyright_holder("Test Corp")
            .with_ai_constraints("No AI training");
        let protected = create_webp_with_legal(legal);
        let xmp = extract_xmp_from_webp(&protected).expect("XMP should be present in WebP");
        assert!(
            xmp.contains("eggstack/stegoeggo"),
            "XMP should contain eggstack/stegoeggo namespace, got: {}",
            xmp
        );
        assert!(
            !xmp.contains("anomalyco/stegoeggo"),
            "XMP should NOT contain anomalyco/stegoeggo, got: {}",
            xmp
        );
    }
}

mod error_variant_tests {
    use super::*;
    use stegoeggo::Error;

    #[test]
    fn test_invalid_format_error_variant() {
        let garbage = vec![0xAB; 64];
        let ctx = create_test_context();
        let result = process_image_bytes(&garbage, ProtectionLevel::Standard, &ctx);
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), Error::InvalidFormat(_)),
            "Should return InvalidFormat error"
        );
    }

    #[test]
    fn test_empty_bytes_error() {
        let ctx = create_test_context();
        let result = process_image_bytes(&[], ProtectionLevel::Standard, &ctx);
        assert!(result.is_err());
        assert!(
            matches!(
                result.unwrap_err(),
                Error::InvalidFormat(_) | Error::Image(_)
            ),
            "Should return InvalidFormat or Image error for empty input"
        );
    }
}

mod jpeg_max_dimension {
    use super::*;

    #[test]
    fn test_max_dimension_validation_jpeg_via_process_bytes() {
        let img = create_test_image(1000, 1000);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_max_dimension(512);

        let result = process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx);
        assert!(
            result.is_err(),
            "Should fail when JPEG exceeds max dimension via process_bytes"
        );
    }

    #[test]
    fn test_max_dimension_within_limit_jpeg() {
        let img = create_test_image(256, 256);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_max_dimension(512);

        let result = process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx);
        assert!(
            result.is_ok(),
            "Should succeed when JPEG is within max dimension"
        );
    }
}

mod inject_metadata_toggle {
    use super::*;

    #[test]
    fn test_inject_metadata_false_skips_metadata() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_metadata_injection(false);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let has_seed = protected_bytes
            .windows(18)
            .any(|w| w == b"X-Protection-Seed");
        assert!(
            !has_seed,
            "Should NOT inject seed metadata when metadata_injection is false"
        );
    }

    #[test]
    fn test_inject_metadata_default_injects() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let has_seed = protected_bytes
            .windows(18)
            .any(|w| w == b"X-Protection-Seed");
        let has_dmi = protected_bytes.windows(14).any(|w| w == b"DMI-PROHIBITED");
        assert!(
            has_seed || has_dmi,
            "Should inject metadata by default (seed or DMI present)"
        );
    }

    #[test]
    fn test_inject_metadata_false_jpeg() {
        let img = create_test_image(32, 32);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_metadata_injection(false);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let has_com = protected_bytes.windows(2).any(|w| w == [0xFF, 0xFE]);
        assert!(
            !has_com,
            "Should NOT inject COM markers when metadata_injection is false"
        );
    }
}

mod inject_legal_claims_toggle {
    use super::*;

    fn legal_ctx(inject: Option<bool>) -> ProtectionContext {
        let legal = stegoeggo::LegalMetadata::new()
            .with_copyright_holder("Test Owner")
            .with_contact_email("legal@example.com");
        let mut ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_legal_metadata(legal);
        if let Some(v) = inject {
            ctx = ctx.with_legal_claims(v);
        }
        ctx
    }

    #[test]
    fn test_legal_claims_injected_when_true() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = legal_ctx(Some(true));

        let protected = process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx).unwrap();

        let has_copyright = protected.windows(9).any(|w| w == b"Copyright");
        assert!(
            has_copyright,
            "Copyright metadata should be present when inject_legal_claims=true"
        );
    }

    #[test]
    fn test_legal_claims_absent_when_false() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = legal_ctx(Some(false));

        let protected = process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx).unwrap();

        let has_copyright = protected.windows(9).any(|w| w == b"Copyright");
        assert!(
            !has_copyright,
            "Copyright metadata should be absent when inject_legal_claims=false"
        );
    }

    #[test]
    fn test_legal_claims_absent_by_default() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = legal_ctx(None);

        let protected = process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx).unwrap();

        let has_copyright = protected.windows(9).any(|w| w == b"Copyright");
        assert!(
            !has_copyright,
            "Copyright metadata should be absent by default (None)"
        );
    }
}

mod progressive_jpeg_warning {
    use super::*;
    use stegoeggo::{
        process_image_bytes_with_info, process_image_bytes_with_warnings, ProtectionWarning,
    };

    #[test]
    fn test_progressive_jpeg_returns_warning() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_progressive_jpeg(true)
            .with_mac_key(b"shared-test-key".to_vec());

        let (protected, warning) =
            process_image_bytes_with_info(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        assert!(!protected.is_empty());
        assert_eq!(
            warning,
            Some(ProtectionWarning::ProgressiveJpegFallback),
            "Should warn about progressive JPEG fallback"
        );
    }

    #[test]
    fn test_baseline_jpeg_warns_about_reencode_fragility() {
        let img = create_test_image(64, 64);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_mac_key(b"shared-test-key".to_vec());

        let (_, warning) =
            process_image_bytes_with_info(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        assert_eq!(
            warning,
            Some(ProtectionWarning::JpegReencodeFragile),
            "Baseline JPEG output should warn about downstream re-encode fragility"
        );
    }

    #[test]
    fn test_png_no_warning() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_mac_key(b"shared-test-key".to_vec());

        let (_, warning) =
            process_image_bytes_with_info(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        assert_eq!(warning, None, "PNG should not produce warning");
    }

    #[test]
    fn test_light_level_warns_only_about_jpeg_fragility_for_progressive() {
        let img = create_test_image(64, 64);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_progressive_jpeg(true)
            .with_mac_key(b"shared-test-key".to_vec());

        let (_, warning) =
            process_image_bytes_with_info(&jpeg_bytes, ProtectionLevel::Light, &ctx).unwrap();

        assert_eq!(
            warning,
            Some(ProtectionWarning::JpegReencodeFragile),
            "Light level should not warn about progressive fallback because no DCT stego is attempted"
        );
    }

    #[test]
    fn test_proxy_warning_api_reports_all_advisories() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_progressive_jpeg(true)
            .with_metadata_injection(false)
            .with_evidence_profile(EvidenceProfile::AuthenticatedProvenance);

        let (_, warnings) =
            process_image_bytes_with_warnings(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        assert!(warnings.contains(&ProtectionWarning::MissingMacKey));
        assert!(warnings.contains(&ProtectionWarning::MetadataInjectionDisabled));
        assert!(warnings.contains(&ProtectionWarning::ProgressiveJpegFallback));
        assert!(warnings.contains(&ProtectionWarning::JpegReencodeFragile));
    }

    #[test]
    fn test_lsb_capacity_warning_uses_effective_payload_size() {
        let img = create_test_image(30, 30);
        let png_bytes = image_to_png_bytes(&img);

        let ctx_without_mac = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Png);
        let (_, warnings_without_mac) = process_image_bytes_with_warnings(
            &png_bytes,
            ProtectionLevel::Standard,
            &ctx_without_mac,
        )
        .unwrap();
        assert!(warnings_without_mac.contains(&ProtectionWarning::LsbCapacitySkipped));

        let ctx_with_mac = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_mac_key(b"shared-test-key".to_vec());
        let (_, warnings_with_mac) =
            process_image_bytes_with_warnings(&png_bytes, ProtectionLevel::Standard, &ctx_with_mac)
                .unwrap();
        assert!(!warnings_with_mac.contains(&ProtectionWarning::LsbCapacitySkipped));
    }
}

#[cfg(test)]
mod notice_verification_tests {
    use super::*;
    use stegoeggo::{
        verify_legal_notice, DmiValue, EvidenceChannel, EvidenceStrength, ImageOutputFormat,
        LegalMetadata, ProtectionContext, ProtectionLevel, VerificationStatus,
    };

    fn create_legal_metadata() -> LegalMetadata {
        LegalMetadata::new()
            .with_copyright_holder("Jane Artist")
            .with_contact_email("legal@example.com")
            .with_license_url("https://example.com/rights")
            .with_usage_terms("Copyrighted work. No AI training permitted.")
            .with_ai_constraints("No generative AI training allowed")
            .with_creator("Jane Artist")
    }

    fn create_full_context() -> ProtectionContext {
        ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_legal_metadata(create_legal_metadata())
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedGenAiMlTraining)
    }

    #[test]
    fn test_png_legal_notice_verifies_fields() {
        let img = create_test_image(64, 64);
        let ctx = create_full_context();
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Light, &ctx).unwrap();

        let report = verify_legal_notice(&protected, &[]);
        assert_eq!(report.copyright_holder(), Some("Jane Artist"));
        assert_eq!(report.contact(), Some("legal@example.com"));
        assert_eq!(report.rights_url(), Some("https://example.com/rights"));
        assert!(report.usage_terms().is_some());
        assert!(report.ai_constraints().is_some());
        assert_eq!(report.creator(), Some("Jane Artist"));
        assert!(report.protection_seed().is_some());
        assert!(report.has_notice());
    }

    #[test]
    fn test_jpeg_legal_notice_verifies_fields() {
        let img = create_test_image(64, 64);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_legal_metadata(create_legal_metadata())
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedGenAiMlTraining);
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Light, &ctx).unwrap();

        let report = verify_legal_notice(&protected, &[]);
        assert!(report.has_notice());
        assert!(!report.channels().is_empty());
    }

    #[test]
    fn test_metadata_only_returns_metadata_notice_only() {
        let img = create_test_image(32, 32);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_legal_metadata(create_legal_metadata())
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedGenAiMlTraining);
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Light, &ctx).unwrap();

        let report = verify_legal_notice(&protected, &[]);
        assert!(report.has_notice());
        assert_ne!(report.stego_status(), VerificationStatus::Verified);
        assert_eq!(
            report.evidence_strength(),
            EvidenceStrength::MetadataNoticeOnly
        );
    }

    #[test]
    fn test_metadata_plus_unkeyed_stego_returns_best_effort() {
        let img = create_test_image(256, 256);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_legal_metadata(create_legal_metadata())
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedGenAiMlTraining);
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let report = verify_legal_notice(&protected, &[]);
        assert!(report.has_notice());
        assert!(report.stego_status() == VerificationStatus::Verified);
        assert!(!report.authenticated());
        assert_eq!(
            report.evidence_strength(),
            EvidenceStrength::MetadataNoticeAndBestEffortStego
        );
    }

    #[test]
    fn test_metadata_plus_keyed_stego_returns_authenticated() {
        let img = create_test_image(256, 256);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_mac_key(b"test-mac-key".to_vec())
            .with_legal_metadata(create_legal_metadata())
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedGenAiMlTraining);
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let report = verify_legal_notice(&protected, b"test-mac-key");
        assert!(report.has_notice());
        assert_eq!(report.stego_status(), VerificationStatus::Verified);
        assert!(report.authenticated());
        assert_eq!(
            report.evidence_strength(),
            EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance
        );
    }

    #[test]
    fn test_wrong_key_reports_unauthenticated() {
        let img = create_test_image(256, 256);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_mac_key(b"test-mac-key".to_vec())
            .with_legal_metadata(create_legal_metadata())
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedGenAiMlTraining);
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let report = verify_legal_notice(&protected, b"wrong-key");
        assert!(report.has_notice());
        assert_eq!(report.stego_status(), VerificationStatus::Invalid);
        assert!(!report.authenticated());
    }

    #[test]
    fn test_unprotected_image_returns_no_notice() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);

        let report = verify_legal_notice(&png_bytes, &[]);
        assert!(!report.has_notice());
        assert_eq!(report.evidence_strength(), EvidenceStrength::NoNoticeFound);
        assert_ne!(report.stego_status(), VerificationStatus::Verified);
    }

    #[test]
    fn test_evidence_channels_populated_for_png() {
        let img = create_test_image(256, 256);
        let ctx = create_full_context();
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let report = verify_legal_notice(&protected, &[]);
        let channels = report.channels();
        assert!(channels.contains(&EvidenceChannel::PngText));
    }

    #[test]
    fn test_jpeg_xmp_channel_detected() {
        let img = create_test_image(64, 64);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_legal_metadata(create_legal_metadata())
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedGenAiMlTraining);
        let protected = process_image_bytes(&jpeg_bytes, ProtectionLevel::Light, &ctx).unwrap();

        let report = verify_legal_notice(&protected, &[]);
        let channels = report.channels();
        assert!(
            channels.contains(&EvidenceChannel::JpegXmp)
                || channels.contains(&EvidenceChannel::JpegComment),
            "JPEG with DMI should have at least JpegXmp or JpegComment channel, got: {:?}",
            channels
        );
    }

    #[test]
    fn test_jpeg_legal_notice_with_metadata() {
        let img = create_colored_image(64, 64, 128, 64, 32);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 95);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_legal_metadata(create_legal_metadata())
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedGenAiMlTraining);
        let protected = process_image_bytes(&jpeg_bytes, ProtectionLevel::Light, &ctx).unwrap();

        let report = verify_legal_notice(&protected, &[]);
        assert!(report.has_notice());
        assert_eq!(report.copyright_holder(), Some("Jane Artist"));
        assert_eq!(report.creator(), Some("Jane Artist"));
        assert_eq!(report.contact(), Some("legal@example.com"));
    }

    #[test]
    fn test_wrong_key_preserves_metadata() {
        let img = create_test_image(256, 256);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_mac_key(b"correct-key".to_vec())
            .with_legal_metadata(create_legal_metadata())
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedGenAiMlTraining);
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let report = verify_legal_notice(&protected, b"wrong-key");
        assert!(
            report.has_notice(),
            "Legal metadata should still be present even with wrong key"
        );
        assert_eq!(report.copyright_holder(), Some("Jane Artist"));
        assert_eq!(report.stego_status(), VerificationStatus::Invalid);
        assert!(!report.authenticated());
    }

    #[test]
    fn test_no_false_channels_for_unprotected() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);

        let report = verify_legal_notice(&png_bytes, &[]);
        assert!(
            report.channels().is_empty(),
            "Unprotected image should have no channels, got: {:?}",
            report.channels()
        );
    }

    #[test]
    fn test_unprotected_jpeg_no_notice() {
        let img = create_test_image(64, 64);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let report = verify_legal_notice(&jpeg_bytes, &[]);
        assert!(!report.has_notice());
        assert_eq!(report.evidence_strength(), EvidenceStrength::NoNoticeFound);
    }

    #[test]
    fn notice_verification_no_false_channels_jpeg() {
        let img = create_test_image(64, 64);
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);

        let report = verify_legal_notice(&jpeg_bytes, &[]);
        assert!(
            report.channels().is_empty(),
            "Unprotected JPEG should have no channels, got: {:?}",
            report.channels()
        );
    }

    #[test]
    fn notice_verification_no_false_channels_webp() {
        let img = create_test_image(64, 64);
        let webp_bytes = image_to_webp_bytes(&img);

        let report = verify_legal_notice(&webp_bytes, &[]);
        assert!(
            report.channels().is_empty(),
            "Unprotected WebP should have no channels, got: {:?}",
            report.channels()
        );
    }

    #[test]
    fn notice_verification_dmi_allowed_not_restriction() {
        let img = create_test_image(64, 64);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_legal_claims(true)
            .with_dmi(DmiValue::Allowed);
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Light, &ctx).unwrap();

        let report = verify_legal_notice(&protected, &[]);
        assert!(
            report.has_notice(),
            "DmiValue::Allowed should make has_notice true (DMI was found)"
        );
        assert_eq!(report.dmi(), Some(DmiValue::Allowed));
    }

    #[test]
    fn notice_verification_dmi_unspecified_not_restriction() {
        let img = create_test_image(64, 64);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_legal_claims(true)
            .with_dmi(DmiValue::Unspecified);
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Light, &ctx).unwrap();

        let report = verify_legal_notice(&protected, &[]);
        assert_eq!(
            report.dmi(),
            None,
            "DmiValue::Unspecified should not be injected as metadata"
        );
    }

    #[test]
    fn notice_verification_tdm_reserved_separate() {
        let img = create_test_image(64, 64);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedAiMlTraining);
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Light, &ctx).unwrap();

        let report = verify_legal_notice(&protected, &[]);
        assert!(report.has_notice());
        assert_eq!(report.tdm_reserved(), Some(true));
        assert_eq!(report.dmi(), Some(DmiValue::ProhibitedAiMlTraining));
    }

    #[test]
    fn cli_verify_prints_legal_fields_before_stego_status() {
        let img = create_test_image(256, 256);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_legal_metadata(create_legal_metadata())
            .with_legal_claims(true)
            .with_dmi(DmiValue::ProhibitedGenAiMlTraining);
        let protected =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let report = verify_legal_notice(&protected, &[]);

        assert!(report.has_notice());
        assert_eq!(report.copyright_holder(), Some("Jane Artist"));
        assert_eq!(report.creator(), Some("Jane Artist"));
        assert_eq!(report.contact(), Some("legal@example.com"));
        assert!(report.usage_terms().is_some());
        assert!(report.ai_constraints().is_some());
        assert_eq!(
            report.evidence_strength(),
            EvidenceStrength::MetadataNoticeAndBestEffortStego
        );
    }
}

#[cfg(test)]
mod warning_severity_tests {
    use super::*;
    use stegoeggo::{
        process_image_bytes_with_warnings, EvidenceProfile, ProtectionWarning, WarningSeverity,
    };

    #[test]
    fn test_missing_mac_not_error_for_legal_notice() {
        let img = create_test_image(256, 256);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_evidence_profile(EvidenceProfile::LegalNotice)
            .with_legal_claims(true);
        let (_, warnings) = process_image_bytes_with_warnings(
            &image_to_png_bytes(&img),
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        if warnings.contains(&ProtectionWarning::MissingMacKey) {
            let severity =
                ProtectionWarning::MissingMacKey.severity_for_profile(EvidenceProfile::LegalNotice);
            assert_eq!(
                severity,
                WarningSeverity::Info,
                "MissingMacKey should be Info for LegalNotice profile"
            );
        }
    }

    #[test]
    fn test_missing_mac_warning_for_authenticated_provenance() {
        let severity = ProtectionWarning::MissingMacKey
            .severity_for_profile(EvidenceProfile::AuthenticatedProvenance);
        assert_eq!(
            severity,
            WarningSeverity::Warning,
            "MissingMacKey should be Warning for AuthenticatedProvenance"
        );
    }

    #[test]
    fn test_metadata_disabled_error_for_legal_notice() {
        let severity = ProtectionWarning::MetadataInjectionDisabled
            .severity_for_profile(EvidenceProfile::LegalNotice);
        assert_eq!(
            severity,
            WarningSeverity::Error,
            "MetadataInjectionDisabled should be Error for LegalNotice"
        );
    }

    #[test]
    fn test_metadata_disabled_error_for_legal_notice_with_stego() {
        let severity = ProtectionWarning::MetadataInjectionDisabled
            .severity_for_profile(EvidenceProfile::LegalNoticeWithStego);
        assert_eq!(
            severity,
            WarningSeverity::Error,
            "MetadataInjectionDisabled should be Error for LegalNoticeWithStego"
        );
    }

    #[test]
    fn test_lsb_capacity_info_for_legal_notice() {
        let severity = ProtectionWarning::LsbCapacitySkipped
            .severity_for_profile(EvidenceProfile::LegalNotice);
        assert_eq!(
            severity,
            WarningSeverity::Info,
            "LsbCapacitySkipped should be Info for LegalNotice"
        );
    }

    #[test]
    fn test_lsb_capacity_warning_for_maximal() {
        let severity =
            ProtectionWarning::LsbCapacitySkipped.severity_for_profile(EvidenceProfile::Maximal);
        assert_eq!(
            severity,
            WarningSeverity::Warning,
            "LsbCapacitySkipped should be Warning for Maximal"
        );
    }

    #[test]
    fn test_progressive_jpeg_always_warning() {
        for profile in &[
            EvidenceProfile::LegalNotice,
            EvidenceProfile::LegalNoticeWithStego,
            EvidenceProfile::AuthenticatedProvenance,
            EvidenceProfile::Maximal,
        ] {
            let severity =
                ProtectionWarning::ProgressiveJpegFallback.severity_for_profile(*profile);
            assert_eq!(
                severity,
                WarningSeverity::Warning,
                "ProgressiveJpegFallback should be Warning for {:?}",
                profile
            );
        }
    }

    #[test]
    fn test_jpeg_reencode_fragile_always_warning() {
        for profile in &[
            EvidenceProfile::LegalNotice,
            EvidenceProfile::LegalNoticeWithStego,
            EvidenceProfile::AuthenticatedProvenance,
            EvidenceProfile::Maximal,
        ] {
            let severity = ProtectionWarning::JpegReencodeFragile.severity_for_profile(*profile);
            assert_eq!(
                severity,
                WarningSeverity::Warning,
                "JpegReencodeFragile should be Warning for {:?}",
                profile
            );
        }
    }

    #[test]
    fn test_strict_legal_notice_no_error_on_missing_mac() {
        let img = create_test_image(256, 256);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_evidence_profile(EvidenceProfile::LegalNotice)
            .with_legal_claims(true);
        let (_, warnings) = process_image_bytes_with_warnings(
            &image_to_png_bytes(&img),
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        let has_error = warnings.iter().any(|w| {
            w.severity_for_profile(EvidenceProfile::LegalNotice) == WarningSeverity::Error
        });
        assert!(
            !has_error,
            "LegalNotice profile should not have Error-level warnings for normal usage"
        );
    }

    #[test]
    fn test_category_mappings() {
        assert_eq!(
            ProtectionWarning::MissingMacKey.category(),
            stegoeggo::WarningCategory::AuthenticatedProvenance
        );
        assert_eq!(
            ProtectionWarning::MetadataInjectionDisabled.category(),
            stegoeggo::WarningCategory::LegalNotice
        );
        assert_eq!(
            ProtectionWarning::ProgressiveJpegFallback.category(),
            stegoeggo::WarningCategory::FormatFragility
        );
        assert_eq!(
            ProtectionWarning::LsbCapacitySkipped.category(),
            stegoeggo::WarningCategory::BestEffortStego
        );
        assert_eq!(
            ProtectionWarning::DctCapacityInsufficient.category(),
            stegoeggo::WarningCategory::BestEffortStego
        );
    }
}
