use cloakrs::{
    process_image_bytes, verify_image_bytes, ImageOutputFormat, MetadataTrapProtector,
    ProtectionContext, ProtectionLevel, SteganographyProtector,
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
    fn dct_stego_verifies_through_public_api_with_redundancy_3() {
        let img = create_test_image(1024, 1024);
        let seed = 42u64;
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);
        let ctx = ProtectionContext::new(0.6, seed)
            .with_format(cloakrs::ImageOutputFormat::Jpeg)
            .with_stego_redundancy(3);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        assert_eq!(verify_image_bytes(&protected_bytes, &[]), Some(true));
    }

    #[test]
    fn qtable_seed_only_does_not_count_as_verified_payload() {
        let img = create_test_image(16, 16);
        let seed = 424242u64;
        let jpeg_bytes = image_to_jpeg_bytes(&img, 90);
        let ctx = ProtectionContext::new(0.5, seed)
            .with_format(cloakrs::ImageOutputFormat::Jpeg)
            .with_stego_redundancy(3);

        let protected_bytes =
            process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        assert_eq!(verify_image_bytes(&protected_bytes, &[]), None);
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

mod ecc_recovery {
    use super::*;

    #[test]
    fn ecc_recovers_from_bit_corruption_in_pixel_data() {
        let img = create_test_image(128, 128);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.7, seed)
            .with_format(ImageOutputFormat::Png)
            .with_stego_redundancy(3);

        let png_bytes = image_to_png_bytes(&img);
        let protected_bytes =
            process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let mut rgba = protected_img.to_rgba8();
        let raw = rgba.as_mut();

        for i in 0..12 {
            let byte_idx = 200 + i * 37;
            if byte_idx < raw.len() {
                raw[byte_idx] ^= 0xFF;
            }
        }

        let corrupted_img = DynamicImage::ImageRgba8(rgba);
        let corrupted_png = image_to_png_bytes(&corrupted_img);

        let result = verify_image_bytes(&corrupted_png, &[]);
        assert_eq!(
            result,
            Some(true),
            "ECC majority-vote decoding recovers payload from bit-corrupted pixel data"
        );
    }
}

mod jpeg_quality_reduction {
    use super::*;

    #[test]
    fn qtable_seed_embedded_and_extractable_before_recompress() {
        let img = create_test_image(128, 128);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.5, seed).with_format(ImageOutputFormat::Jpeg);

        let protected_bytes = process_image_bytes(
            &image_to_jpeg_bytes(&img, 90),
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        let seed_before = MetadataTrapProtector::extract_seed_from_image(&protected_bytes);
        assert_eq!(seed_before, Some(seed), "Seed embedded in Q-tables at Q=90");

        let recompressed =
            image_to_jpeg_bytes(&image::load_from_memory(&protected_bytes).unwrap(), 50);

        let seed_after = MetadataTrapProtector::extract_seed_from_image(&recompressed);
        assert!(
            seed_after.is_none(),
            "Q-table seed lost after image crate re-encoder creates new quantization tables (Q=90→Q=50)"
        );
    }

    #[test]
    fn qtable_seed_lost_after_multiple_recompressions() {
        let img = create_test_image(64, 64);
        let seed = 99u64;
        let ctx = ProtectionContext::new(0.5, seed).with_format(ImageOutputFormat::Jpeg);

        let protected_bytes = process_image_bytes(
            &image_to_jpeg_bytes(&img, 95),
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        let mut current_bytes = protected_bytes;
        for &quality in &[80, 60, 40] {
            let img = image::load_from_memory(&current_bytes).unwrap();
            current_bytes = image_to_jpeg_bytes(&img, quality);
        }

        let seed_after = MetadataTrapProtector::extract_seed_from_image(&current_bytes);
        assert!(
            seed_after.is_none(),
            "Q-table seed lost after image crate re-encoder creates new quantization tables (95→80→60→40)"
        );
    }
}

mod dct_payload_after_recompression {
    use super::*;

    #[test]
    fn verify_finds_no_protection_after_jpeg_recompress() {
        let img = create_test_image(128, 128);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.6, seed).with_format(ImageOutputFormat::Jpeg);

        let protected_bytes = process_image_bytes(
            &image_to_jpeg_bytes(&img, 90),
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        let recompressed =
            image_to_jpeg_bytes(&image::load_from_memory(&protected_bytes).unwrap(), 75);

        let result = verify_image_bytes(&recompressed, &[]);
        assert!(
            result.is_none(),
            "verify_image_bytes: no protection survives JPEG re-encoding (Q=90→Q=75) — image crate encoder creates new quantization tables and Huffman codes"
        );
    }

    #[test]
    fn verify_finds_no_protection_after_low_quality_recompress() {
        let img = create_test_image(128, 128);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.6, seed).with_format(ImageOutputFormat::Jpeg);

        let protected_bytes = process_image_bytes(
            &image_to_jpeg_bytes(&img, 85),
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        let recompressed =
            image_to_jpeg_bytes(&image::load_from_memory(&protected_bytes).unwrap(), 50);

        let result = verify_image_bytes(&recompressed, &[]);
        assert!(
            result.is_none(),
            "verify_image_bytes: no protection survives JPEG re-encoding (Q=85→Q=50) — image crate encoder creates new quantization tables and Huffman codes"
        );
    }
}

mod lsb_png_roundtrip {
    use super::*;

    #[test]
    fn lsb_payload_survives_png_roundtrip() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.7, seed).with_format(ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let stego = SteganographyProtector::new();
        let payload = stego.extract_payload_with_seed(&protected_img, seed);
        assert!(
            payload.is_some(),
            "LSB payload survives PNG roundtrip (lossless format preserves pixel LSBs)"
        );
        let payload = payload.unwrap();
        assert_eq!(payload.seed(), seed, "Extracted seed matches original");
        assert_eq!(
            payload.protection_level(),
            2,
            "Protection level is Standard (2)"
        );
    }

    #[test]
    fn verify_payload_after_png_roundtrip() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.6, seed).with_format(ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let result = verify_image_bytes(&protected_bytes, &[]);
        assert_eq!(
            result,
            Some(true),
            "verify_image_bytes confirms payload after PNG encode/decode roundtrip"
        );
    }

    #[test]
    fn lsb_payload_survives_multiple_png_roundtrips() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.7, seed).with_format(ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let mut current_bytes = protected_bytes;
        for _ in 0..5 {
            let img = image::load_from_memory(&current_bytes).unwrap();
            current_bytes = image_to_png_bytes(&img);
        }

        let result = verify_image_bytes(&current_bytes, &[]);
        assert_eq!(
            result,
            Some(true),
            "LSB payload survives five PNG roundtrips (all lossless)"
        );
    }
}

mod metadata_stripping_stego {
    use super::*;

    #[test]
    fn stego_detectable_after_metadata_strip() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.7, seed).with_format(ImageOutputFormat::Png);

        let protected_bytes =
            process_image_bytes(&image_to_png_bytes(&img), ProtectionLevel::Standard, &ctx)
                .unwrap();

        let protected_img = image::load_from_memory(&protected_bytes).unwrap();
        let stripped_bytes = image_to_png_bytes(&protected_img);

        let metadata_seed = MetadataTrapProtector::extract_seed_from_image(&stripped_bytes);
        assert!(
            metadata_seed.is_none(),
            "Metadata seed stripped after re-encode (DynamicImage drops tEXt chunks)"
        );

        let result = verify_image_bytes(&stripped_bytes, &[]);
        assert_eq!(
            result,
            Some(true),
            "verify_image_bytes finds stego via fallback seeds after metadata strip"
        );
    }

    #[test]
    fn stego_payload_extractable_after_metadata_strip() {
        let img = create_test_image(64, 64);
        let seed = 42u64;
        let ctx = ProtectionContext::new(0.8, seed).with_format(ImageOutputFormat::Png);

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
            "extract_payload_with_seed recovers full payload after metadata strip"
        );
        let payload = payload.unwrap();
        assert_eq!(payload.seed(), seed, "Extracted seed matches original");
        assert_eq!(payload.intensity(), ctx.intensity());
    }
}

mod robust_stego_matrix {
    //! Documents realistic survival expectations for each protection layer.
    //! These tests serve as executable documentation of the threat model.
    use super::*;

    #[test]
    fn standard_jpeg_qtable_seed_survives_single_recompression_same_q() {
        // DISCOVERED: a single JPEG re-compression at the SAME quality (Q=90)
        // does NOT preserve the Q-table seed via the image crate's encoder.
        // The image crate rebuilds standard Q-tables from scratch on each
        // encode, so any LSB modifications to quantization values are lost.
        // This is the same outcome as a different-quality recompression: the
        // Q-table channel is single-encoding only. If the test ever flips to
        // Some(seed), the image crate has been upgraded to a Q-table-stable
        // encoder and the threat model improves.
        let img = create_test_image(64, 64);
        let seed = 12345u64;
        let ctx = ProtectionContext::new(0.7, seed).with_format(ImageOutputFormat::Jpeg);
        let protected = process_image_bytes(
            &image_to_jpeg_bytes(&img, 90),
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        let recompressed_same_q =
            image_to_jpeg_bytes(&image::load_from_memory(&protected).unwrap(), 90);
        let seed_after = MetadataTrapProtector::extract_seed_from_image(&recompressed_same_q);

        assert!(
            seed_after.is_none(),
            "Q-table seed is lost even on same-quality recompression (image crate rebuilds Q-tables): got {seed_after:?}"
        );
    }

    #[test]
    fn standard_jpeg_metadata_survives_any_quality_recompression() {
        // DISCOVERED: the image crate's JpegEncoder does not preserve COM/APP1
        // segments on re-encode. A single re-encode (Q=85) already strips the
        // X-Protection-Seed marker. This means the visible metadata channel
        // is single-encoding only when going through the image crate; the
        // steganographic channel is the only one that survives recompression
        // (and even then only with LSB-preserving noise — see
        // noise_injection::stego_survives_lsb_preserving_noise).
        // If this test ever flips to Some(seed), the image crate has been
        // upgraded to preserve APP/COM segments and the threat model improves.
        let img = create_test_image(64, 64);
        let seed = 99999u64;
        let ctx = ProtectionContext::new(0.5, seed).with_format(ImageOutputFormat::Jpeg);
        let protected = process_image_bytes(
            &image_to_jpeg_bytes(&img, 90),
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        let mut current = protected;
        for &q in &[85, 75, 60, 50] {
            current = image_to_jpeg_bytes(&image::load_from_memory(&current).unwrap(), q);
        }

        let seed_after = MetadataTrapProtector::extract_seed_from_image(&current);
        assert!(
            seed_after.is_none(),
            "X-Protection-Seed metadata is lost after recompression (image crate strips APP/COM segments): got {seed_after:?}"
        );
    }
}
