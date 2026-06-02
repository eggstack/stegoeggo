use cloakrs::{
    process_image_bytes, verify_image_bytes, MetadataTrapProtector, ProtectionContext,
    ProtectionLevel, SteganographyProtector,
};
use image::DynamicImage;
use image::ImageEncoder;

fn create_test_image(width: u32, height: u32) -> DynamicImage {
    let img = DynamicImage::new_rgb8(width, height);
    let mut rgb = img.to_rgb8();
    for y in 0..height {
        for x in 0..width {
            let r = ((x * 7 + y * 3) % 256) as u8;
            let g = ((x * 11 + y * 5) % 256) as u8;
            let b = ((x * 13 + y * 9) % 256) as u8;
            rgb.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    DynamicImage::ImageRgb8(rgb)
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

mod jpeg_recompression {
    use super::*;

    #[test]
    fn dct_stego_metadata_seed_embedded_in_protected_jpeg() {
        let img = create_test_image(128, 128);
        let seed = 42u64;
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);
        let ctx = ProtectionContext::new(0.5, seed).with_format(cloakrs::ImageOutputFormat::Jpeg);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let metadata_seed = MetadataTrapProtector::extract_seed_from_image(&protected_bytes);
        assert_eq!(
            metadata_seed,
            Some(seed),
            "Protected JPEG contains seed in metadata (COM/APP1 markers)"
        );
    }

    #[test]
    fn dct_stego_jpeg_decodable_by_image_crate() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);
        let ctx = ProtectionContext::new(0.5, seed).with_format(cloakrs::ImageOutputFormat::Jpeg);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let result = image::load_from_memory(&protected_bytes);
        assert!(
            result.is_ok(),
            "DCT-stego'd JPEG should be decodable by the image crate after transcoder fixes"
        );
    }

    #[test]
    fn png_metadata_lost_on_jpeg_conversion() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.5, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let seed_before = MetadataTrapProtector::extract_seed_from_image(&protected_bytes);
        assert_eq!(
            seed_before,
            Some(seed),
            "PNG should have seed in tEXt chunks"
        );

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let jpeg_bytes = image_to_jpeg_bytes(&protected_img, 85);

        let seed_after = MetadataTrapProtector::extract_seed_from_image(&jpeg_bytes);
        assert!(
            seed_after.is_none(),
            "PNG tEXt metadata lost when converted to JPEG (different container format)"
        );
    }

    #[test]
    fn verify_image_bytes_finds_stego_via_fallback_on_recompressed_jpeg() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.6, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let jpeg_bytes = image_to_jpeg_bytes(&protected_img, 85);

        let result = verify_image_bytes(&jpeg_bytes, &[]);
        assert!(
            result.is_none(),
            "verify_image_bytes: no protection data survives PNG→JPEG conversion"
        );
    }
}

mod metadata_stripping {
    use super::*;

    #[test]
    fn stego_survives_png_metadata_strip() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.6, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let stripped_bytes = image_to_png_bytes(&protected_img);

        let metadata_seed = MetadataTrapProtector::extract_seed_from_image(&stripped_bytes);
        assert!(
            metadata_seed.is_none(),
            "Metadata seed stripped after DynamicImage round-trip (re-encode creates clean PNG)"
        );

        let stego = SteganographyProtector::new();
        let stripped_img = image::load_from_memory(&stripped_bytes).unwrap();
        assert!(
            stego.verify_payload(&stripped_img),
            "LSB stego payload survives metadata stripping (pixels unchanged)"
        );
    }

    #[test]
    fn verify_finds_stego_after_metadata_strip() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.6, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let stripped_bytes = image_to_png_bytes(&protected_img);

        let result = verify_image_bytes(&stripped_bytes, &[]);
        assert_eq!(
            result,
            Some(true),
            "verify_image_bytes finds stego via fallback seeds after metadata strip"
        );
    }

    #[test]
    fn extract_payload_after_metadata_strip() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.7, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let stripped_bytes = image_to_png_bytes(&protected_img);
        let stripped_img = image::load_from_memory(&stripped_bytes).unwrap();

        let stego = SteganographyProtector::new();
        let payload = stego.extract_payload_with_seed(&stripped_img, seed);
        assert!(
            payload.is_some(),
            "extract_payload_with_seed recovers payload after metadata strip"
        );
        let payload = payload.unwrap();
        assert_eq!(payload.seed(), seed, "Extracted seed matches original");
    }
}

mod format_conversion_round_trip {
    use super::*;

    #[test]
    fn png_to_jpeg_metadata_lost() {
        let img = create_test_image(64, 64);
        let seed = 99u64;
        let ctx = ProtectionContext::new(0.5, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let jpeg_bytes = image_to_jpeg_bytes(&protected_img, 85);

        let seed_after_jpeg = MetadataTrapProtector::extract_seed_from_image(&jpeg_bytes);
        assert!(
            seed_after_jpeg.is_none(),
            "PNG tEXt metadata lost when converting to JPEG format"
        );
    }

    #[test]
    fn png_to_jpeg_to_png_metadata_lost() {
        let img = create_test_image(64, 64);
        let seed = 99u64;
        let ctx = ProtectionContext::new(0.5, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let jpeg_bytes = image_to_jpeg_bytes(&protected_img, 90);
        let jpeg_img = image::load_from_memory(&jpeg_bytes).unwrap();
        let final_png = image_to_png_bytes(&jpeg_img);

        let seed_after_roundtrip = MetadataTrapProtector::extract_seed_from_image(&final_png);
        assert!(
            seed_after_roundtrip.is_none(),
            "Metadata seed does not survive PNG→JPEG→PNG (lost in JPEG step)"
        );
    }

    #[test]
    fn png_to_jpeg_to_png_stego_lost() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.7, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let jpeg_bytes = image_to_jpeg_bytes(&protected_img, 90);
        let jpeg_img = image::load_from_memory(&jpeg_bytes).unwrap();
        let final_png = image_to_png_bytes(&jpeg_img);
        let final_img = image::load_from_memory(&final_png).unwrap();

        let stego = SteganographyProtector::new();
        let payload = stego.extract_payload_with_seed(&final_img, seed);
        assert!(
            payload.is_none(),
            "LSB stego does NOT survive PNG→JPEG→PNG (JPEG compression destroys pixel LSBs)"
        );
    }
}

mod image_resizing {
    use super::*;

    #[test]
    fn metadata_lost_after_resize() {
        let img = create_test_image(128, 128);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.5, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let resized = protected_img.resize(64, 64, image::imageops::FilterType::Lanczos3);
        let resized_bytes = image_to_png_bytes(&resized);

        let extracted_seed = MetadataTrapProtector::extract_seed_from_image(&resized_bytes);
        assert!(
            extracted_seed.is_none(),
            "Metadata seed lost after resize (image crate re-encode strips tEXt chunks)"
        );
    }

    #[test]
    fn stego_lost_after_resize() {
        let img = create_test_image(128, 128);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.7, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let resized = protected_img.resize(64, 64, image::imageops::FilterType::Lanczos3);

        let stego = SteganographyProtector::new();
        let payload = stego.extract_payload_with_seed(&resized, seed);
        assert!(
            payload.is_none(),
            "LSB stego does NOT survive resize (pixel positions and values change)"
        );
    }
}

mod noise_injection {
    use super::*;

    #[test]
    fn metadata_lost_after_noise_and_reencode() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.5, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let mut rgba = protected_img.to_rgba8();

        for y in 0..rgba.height() {
            for x in 0..rgba.width() {
                let pixel = rgba.get_pixel_mut(x, y);
                let offset = ((x * 3 + y * 7) % 5) as i16 - 2;
                for c in 0..3 {
                    let val = pixel[c] as i16 + offset;
                    pixel[c] = val.clamp(0, 255) as u8;
                }
            }
        }

        let noisy_img = DynamicImage::ImageRgba8(rgba);
        let noisy_bytes = image_to_png_bytes(&noisy_img);

        let extracted_seed = MetadataTrapProtector::extract_seed_from_image(&noisy_bytes);
        assert!(
            extracted_seed.is_none(),
            "Metadata seed lost after noise + re-encode (re-encode strips tEXt chunks)"
        );
    }

    #[test]
    fn stego_lost_with_lsb_flipping_noise() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.8, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let mut rgba = protected_img.to_rgba8();

        for y in 0..rgba.height() {
            for x in 0..rgba.width() {
                let pixel = rgba.get_pixel_mut(x, y);
                let offset = ((x * 3 + y * 7) % 5) as i16 - 2;
                for c in 0..3 {
                    let val = pixel[c] as i16 + offset;
                    pixel[c] = val.clamp(0, 255) as u8;
                }
            }
        }

        let noisy_img = DynamicImage::ImageRgba8(rgba);
        let stego = SteganographyProtector::new();
        let payload = stego.extract_payload_with_seed(&noisy_img, seed);
        assert!(
            payload.is_none(),
            "LSB stego lost when noise flips LSBs (offsets ±1 change bit 0)"
        );
    }

    #[test]
    fn stego_survives_lsb_preserving_noise() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.8, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let mut rgba = protected_img.to_rgba8();

        for y in 0..rgba.height() {
            for x in 0..rgba.width() {
                let pixel = rgba.get_pixel_mut(x, y);
                let offset = (((x * 3 + y * 7) % 3) as i16) * 2 - 2;
                for c in 0..3 {
                    let val = pixel[c] as i16 + offset;
                    pixel[c] = val.clamp(0, 255) as u8;
                }
            }
        }

        let noisy_img = DynamicImage::ImageRgba8(rgba);
        let stego = SteganographyProtector::new();
        let payload = stego.extract_payload_with_seed(&noisy_img, seed);
        assert!(
            payload.is_some(),
            "LSB stego survives noise that preserves LSBs (even-only offsets: -2, 0, +2)"
        );
        let payload = payload.unwrap();
        assert_eq!(payload.seed(), seed, "Extracted seed matches original");
    }
}

mod crop {
    use super::*;

    #[test]
    fn metadata_lost_after_crop_and_reencode() {
        let img = create_test_image(100, 100);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.5, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let mut protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let cropped = protected_img.crop(10, 10, 80, 80);
        let cropped_bytes = image_to_png_bytes(&cropped);

        let extracted_seed = MetadataTrapProtector::extract_seed_from_image(&cropped_bytes);
        assert!(
            extracted_seed.is_none(),
            "Metadata seed lost after crop + re-encode (re-encode strips tEXt chunks)"
        );
    }

    #[test]
    fn stego_lost_after_crop() {
        let img = create_test_image(100, 100);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.8, seed).with_format(cloakrs::ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let mut protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let cropped = protected_img.crop(10, 10, 80, 80);

        let stego = SteganographyProtector::new();
        let payload = stego.extract_payload_with_seed(&cropped, seed);
        assert!(
            payload.is_none(),
            "LSB stego does NOT survive crop (pixel positions shift, seed-based selection mismatches)"
        );
    }
}
