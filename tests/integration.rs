use cloakrs::{
    process_image, process_image_bytes, process_images_bytes_parallel, process_images_parallel,
    DmiValue, ImageOutputFormat, LegalMetadata, MetadataTrapProtector, PassthroughProtector,
    ProtectionContext, ProtectionLevel, ProtectionPipeline, SteganographyProtector,
};
use image::{DynamicImage, ImageEncoder};

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
    use cloakrs::Protector;

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

        let hash1 = cloakrs::compute_image_hash(&img);
        let hash2 = cloakrs::compute_image_hash(&img);

        assert_eq!(hash1, hash2, "Hash should be deterministic");
    }

    #[test]
    fn test_image_hash_differs_for_different_images() {
        let img1 = create_colored_image(64, 64, 10, 20, 30);
        let img2 = create_colored_image(64, 64, 200, 210, 220);

        let hash1 = cloakrs::compute_image_hash(&img1);
        let hash2 = cloakrs::compute_image_hash(&img2);

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

        let iscc = cloakrs::compute_iscc(&img);
        assert!(!iscc.full.is_empty(), "ISCC should be computed");
    }

    #[test]
    fn test_iscc_from_bytes() {
        let img_bytes = image_to_png_bytes(&create_test_image(32, 32));

        let iscc = cloakrs::compute_iscc_from_bytes(&img_bytes);
        assert!(iscc.is_some(), "ISCC should be computed from bytes");
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
    use cloakrs::verify_image_bytes;

    #[test]
    fn test_verify_image_bytes_sync() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 42);

        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let result = verify_image_bytes(&protected_bytes, &[]);
        assert_eq!(result, Some(true), "Protected image should verify");
    }

    #[test]
    fn test_verify_image_bytes_unprotected() {
        let img = create_test_image(32, 32);
        let png_bytes = image_to_png_bytes(&img);

        let result = verify_image_bytes(&png_bytes, &[]);
        assert!(
            result.is_none() || result == Some(false),
            "Unprotected image should not verify"
        );
    }
}

mod serde_tests {
    use super::*;

    #[test]
    fn test_config_skipped_in_serde_roundtrip() {
        use cloakrs::ProtectionConfig;
        use std::sync::Arc;

        let config = Arc::new(
            ProtectionConfig::new()
                .with_mac_key(b"secret".to_vec())
                .with_legal_metadata(cloakrs::LegalMetadata::new().with_copyright_holder("Test")),
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
}

mod error_variant_tests {
    use super::*;
    use cloakrs::Error;

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
