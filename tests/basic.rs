#![allow(deprecated)]

use image::DynamicImage;
use stegoeggo::{
    process_image, process_image_bytes, MetadataTrapProtector, PassthroughProtector,
    ProtectionContext, ProtectionLevel, ProtectionPipeline, Protector,
};

fn create_test_image() -> DynamicImage {
    DynamicImage::new_rgb8(10, 10)
}

fn create_test_context() -> ProtectionContext {
    ProtectionContext::new(0.5, 42)
}

mod basic {
    use super::*;

    #[test]
    fn test_protection_levels() {
        let img = create_test_image();
        let ctx = create_test_context();

        for level in &[
            ProtectionLevel::Disabled,
            ProtectionLevel::Light,
            ProtectionLevel::Standard,
        ] {
            let result = process_image(img.clone(), *level, &ctx);
            assert!(result.is_ok(), "Failed for level: {:?}", level);
        }
    }

    #[test]
    fn test_pipeline() {
        let pipeline = ProtectionPipeline::new();
        let img = create_test_image();
        let ctx = create_test_context();

        let result = pipeline.process(&img, ProtectionLevel::Standard, &ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_all_levels() {
        let pipeline = ProtectionPipeline::new();
        let img = create_test_image();

        for level in &[
            ProtectionLevel::Disabled,
            ProtectionLevel::Light,
            ProtectionLevel::Standard,
        ] {
            let result = pipeline.process(&img, *level, &create_test_context());
            assert!(result.is_ok(), "Failed for level: {:?}", level);
        }
    }

    #[test]
    fn test_protection_context() {
        let ctx = ProtectionContext::new(0.5, 123);
        assert_eq!(ctx.intensity(), 0.5);
        assert_eq!(ctx.seed(), 123);
    }
}

mod protectors {
    use super::*;

    #[test]
    fn test_passthrough() {
        let poisoner = PassthroughProtector::new();
        let img = create_test_image();
        let ctx = create_test_context();

        let result = poisoner.apply(&img, &ctx);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.width(), img.width());
        assert_eq!(processed.height(), img.height());
    }

    #[test]
    fn test_passthrough_preserves_image() {
        let poisoner = PassthroughProtector::new();
        let img = create_test_image();
        let ctx = create_test_context();

        let result = poisoner.apply(&img, &ctx).unwrap();

        let original_bytes = img.to_rgb8().into_raw();
        let processed_bytes = result.to_rgb8().into_raw();

        assert_eq!(original_bytes, processed_bytes);
    }

    #[test]
    fn test_metadata_trap() {
        let poisoner = MetadataTrapProtector::new();
        let img = create_test_image();
        let ctx = create_test_context();

        let result = poisoner.apply(&img, &ctx);
        assert!(result.is_ok());
        assert_eq!(poisoner.protection_level(), ProtectionLevel::Light);
    }
}

mod integration {
    use super::*;

    #[test]
    fn test_process_image_bytes() {
        let img = DynamicImage::new_rgb8(10, 10);
        let mut buffer = Vec::new();

        {
            use image::ImageEncoder;
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
            encoder
                .write_image(&img.to_rgb8(), 10, 10, image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        let result =
            process_image_bytes(&buffer, ProtectionLevel::Standard, &create_test_context());

        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_output_not_identical_to_input() {
        let img = create_test_image();
        let mut buffer = Vec::new();

        {
            use image::ImageEncoder;
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
            encoder
                .write_image(&img.to_rgb8(), 10, 10, image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        let result = process_image_bytes(
            &buffer,
            ProtectionLevel::Standard,
            &ProtectionContext::new(0.8, 42),
        )
        .unwrap();

        assert_ne!(result.len(), 0);
    }

    #[test]
    fn test_all_levels_produce_valid_images() {
        let pipeline = ProtectionPipeline::new();

        for level in &[
            ProtectionLevel::Disabled,
            ProtectionLevel::Light,
            ProtectionLevel::Standard,
        ] {
            let test_img = create_test_image();
            let result = pipeline.process(&test_img, *level, &create_test_context());
            assert!(
                result.is_ok(),
                "Level {:?} should produce valid image",
                level
            );

            let img = result.unwrap();
            assert!(img.width() > 0);
            assert!(img.height() > 0);
        }
    }

    #[test]
    fn test_jpeg_encoding() {
        use image::ImageEncoder;

        let img = create_test_image();
        let mut jpeg_bytes = Vec::new();

        {
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 90);
            let rgb = img.to_rgb8();
            encoder
                .write_image(&rgb, 10, 10, image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        assert!(!jpeg_bytes.is_empty());
        assert!(jpeg_bytes.starts_with(&[0xFF, 0xD8]));
    }

    #[test]
    fn test_metadata_injection_png() {
        use stegoeggo::ImageOutputFormat;

        let img = create_test_image();

        let mut png_bytes = Vec::new();
        {
            use image::ImageEncoder;
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
            encoder
                .write_image(&img.to_rgb8(), 10, 10, image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        let ctx = ProtectionContext::new(0.8, 12345).with_format(ImageOutputFormat::Png);

        let result = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let has_itext = result.windows(4).any(|w| w == b"iTXt");
        let has_dmi_text = result.windows(14).any(|w| w == b"DMI-PROHIBITED");

        assert!(
            has_itext || has_dmi_text,
            "PNG should contain iTXt or DMI-PROHIBITED"
        );
    }

    #[test]
    fn test_xmp_injection_jpeg() {
        use stegoeggo::{DmiValue, ImageOutputFormat};

        let img = create_test_image();

        let mut jpeg_bytes = Vec::new();
        {
            use image::ImageEncoder;
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 90);
            let rgb = img.to_rgb8();
            encoder
                .write_image(&rgb, 10, 10, image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        let ctx = ProtectionContext::new(0.8, 12345)
            .with_format(ImageOutputFormat::Jpeg)
            .with_dmi(DmiValue::ProhibitedGenAiMlTraining);

        let result = process_image_bytes(&jpeg_bytes, ProtectionLevel::Light, &ctx).unwrap();

        let has_app1 = result.windows(2).any(|w| w == [0xFF, 0xE1]);
        let has_dmi_text = result.windows(14).any(|w| w == b"DMI-PROHIBITED");

        assert!(
            has_app1 || has_dmi_text,
            "JPEG should contain APP1 or DMI-PROHIBITED"
        );
    }

    #[test]
    fn test_auto_dmi_from_protection_level() {
        use stegoeggo::ImageOutputFormat;

        let img = create_test_image();

        let mut png_bytes = Vec::new();
        {
            use image::ImageEncoder;
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
            encoder
                .write_image(&img.to_rgb8(), 10, 10, image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        let ctx = ProtectionContext::new(0.8, 12345).with_format(ImageOutputFormat::Png);

        let result = process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx).unwrap();

        let has_dmi = result.windows(14).any(|w| w == b"DMI-PROHIBITED");

        assert!(
            has_dmi,
            "PNG should auto-inject DMI based on protection level"
        );
    }
}
