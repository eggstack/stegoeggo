use cloakrs::{
    process_image_async, process_image_bytes_async, process_images_bytes_parallel_async,
    process_images_parallel_async, verify_image_bytes_async, ProtectionContext, ProtectionLevel,
    SteganographyProtector,
};
use image::{DynamicImage, ImageEncoder};

fn create_test_image(width: u32, height: u32) -> DynamicImage {
    DynamicImage::new_rgb8(width, height)
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

mod single_image {
    use super::*;

    #[tokio::test]
    async fn test_process_image_async() {
        let img = create_test_image(64, 64);
        let ctx = ProtectionContext::new(0.5, 42);

        let result = process_image_async(img, ProtectionLevel::Standard, ctx).await;
        assert!(result.is_ok());

        let stego = SteganographyProtector::new();
        assert!(stego.verify_payload(&result.unwrap()));
    }

    #[tokio::test]
    async fn test_process_image_bytes_async() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 12345);

        let result = process_image_bytes_async(png_bytes, ProtectionLevel::Standard, ctx).await;
        assert!(result.is_ok());

        let protected_bytes = result.unwrap();
        let protected_img = image::load_from_memory(&protected_bytes).unwrap();

        let stego = SteganographyProtector::new();
        assert!(stego.verify_payload(&protected_img));
    }

    #[tokio::test]
    async fn test_verify_image_bytes_async() {
        let img = create_test_image(64, 64);
        let png_bytes = image_to_png_bytes(&img);
        let ctx = ProtectionContext::new(0.5, 99999);

        let protected = process_image_bytes_async(png_bytes, ProtectionLevel::Standard, ctx)
            .await
            .unwrap();

        let result = verify_image_bytes_async(protected, vec![]).await;
        assert_eq!(result.unwrap(), Some(true));
    }
}

mod parallel {
    use super::*;

    #[tokio::test]
    async fn test_process_images_parallel_async() {
        let images: Vec<DynamicImage> = (0..4).map(|_| create_test_image(32, 32)).collect();
        let ctx = ProtectionContext::new(0.5, 77777);

        let results = process_images_parallel_async(images, ProtectionLevel::Standard, ctx).await;
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 4);
    }

    #[tokio::test]
    async fn test_process_images_bytes_parallel_async() {
        let images: Vec<Vec<u8>> = (0..4)
            .map(|_| {
                let img = create_test_image(32, 32);
                image_to_png_bytes(&img)
            })
            .collect();
        let ctx = ProtectionContext::new(0.5, 88888);

        let results =
            process_images_bytes_parallel_async(images, ProtectionLevel::Standard, ctx).await;
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 4);
    }

    #[tokio::test]
    async fn test_parallel_images_all_verifiable() {
        let images: Vec<DynamicImage> = (0..4).map(|_| create_test_image(64, 64)).collect();
        let ctx = ProtectionContext::new(0.5, 42);

        let results = process_images_parallel_async(images, ProtectionLevel::Standard, ctx)
            .await
            .unwrap();

        let stego = SteganographyProtector::new();
        for img in &results {
            assert!(stego.verify_payload(img), "each image should be verifiable");
        }
    }
}

mod concurrency {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_requests_do_not_block() {
        let ctx = ProtectionContext::new(0.5, 42);

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    let bytes = image_to_png_bytes(&create_test_image(256, 256));
                    process_image_bytes_async(bytes, ProtectionLevel::Standard, ctx).await
                })
            })
            .collect();

        let mut succeeded = 0;
        for handle in handles {
            if let Ok(Ok(_)) = handle.await {
                succeeded += 1;
            }
        }
        assert_eq!(succeeded, 8, "All concurrent tasks should succeed");
    }

    #[tokio::test]
    async fn test_concurrent_requests_remain_verifiable() {
        let ctx = ProtectionContext::new(0.5, 42);

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    let bytes = image_to_png_bytes(&create_test_image(128, 128));
                    let protected =
                        process_image_bytes_async(bytes, ProtectionLevel::Standard, ctx)
                            .await
                            .unwrap();
                    let img = image::load_from_memory(&protected).unwrap();
                    let stego = SteganographyProtector::new();
                    stego.verify_payload(&img)
                })
            })
            .collect();

        for handle in handles {
            assert!(
                handle.await.unwrap(),
                "concurrent result should be verifiable"
            );
        }
    }

    #[tokio::test]
    async fn test_all_levels_async() {
        let ctx = ProtectionContext::new(0.5, 10101);

        for level in [
            ProtectionLevel::Disabled,
            ProtectionLevel::Light,
            ProtectionLevel::Standard,
        ] {
            let result = process_image_async(create_test_image(32, 32), level, ctx.clone()).await;
            assert!(result.is_ok(), "Level {:?} should succeed", level);
        }
    }
}
