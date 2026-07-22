use stegoeggo::{
    process_image_bytes, process_request_bytes, ProtectionContext, ProtectionLevel,
    ProtectionRequest, RightsNotice, RightsPolicy,
};

const SOAK_ITERATIONS: usize = 200;

#[test]
fn soak_test_png_round_trip_no_memory_growth() {
    let ctx = ProtectionContext::new(0.5, 42);
    let png_bytes = create_test_png(512, 512);

    let mut prev_size = 0usize;

    for i in 0..SOAK_ITERATIONS {
        let (output, _warnings) = stegoeggo::process_image_bytes_with_warnings(
            &png_bytes,
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        if i > 0 && i % 50 == 0 {
            let current_size = output.len();
            if prev_size > 0 {
                let delta = current_size as f64 / prev_size as f64;
                assert!(
                    delta < 1.1,
                    "Output size grew {delta:.2}x at iteration {i}: {prev_size} -> {current_size}",
                );
            }
            prev_size = current_size;
        }
    }
}

#[test]
fn soak_test_jpeg_fast_path_no_memory_growth() {
    let jpeg_bytes = create_test_jpeg(512, 512);
    let ctx = ProtectionContext::new(0.5, 42);

    let mut prev_size = 0usize;

    for i in 0..SOAK_ITERATIONS {
        let output = process_image_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        if i > 0 && i % 50 == 0 {
            let current_size = output.len();
            if prev_size > 0 {
                let delta = current_size as f64 / prev_size as f64;
                assert!(
                    delta < 1.1,
                    "Output size grew {delta:.2}x at iteration {i}: {prev_size} -> {current_size}",
                );
            }
            prev_size = current_size;
        }
    }
}

#[test]
fn soak_test_request_api_metadata_only() {
    let png_bytes = create_test_png(256, 256);
    let notice = RightsNotice::new().with_copyright_holder("Test");
    let request = ProtectionRequest::metadata_only(notice, RightsPolicy::ProhibitedAiMlTraining);

    for _ in 0..SOAK_ITERATIONS {
        let output = process_request_bytes(&png_bytes, &request).unwrap();
        assert!(!output.is_empty());
    }
}

#[test]
fn soak_test_verify_cycle() {
    let png_bytes = create_test_png(256, 256);
    let ctx = ProtectionContext::new(0.5, 42);

    let protected = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();

    for _ in 0..SOAK_ITERATIONS {
        let status = stegoeggo::verify_image_bytes(&protected, &[]);
        assert_eq!(status, stegoeggo::VerificationStatus::Verified);
    }
}

#[test]
fn soak_test_mixed_formats() {
    let ctx = ProtectionContext::new(0.5, 42);
    let formats: Vec<(&str, Vec<u8>)> = vec![
        ("png", create_test_png(256, 256)),
        ("jpeg", create_test_jpeg(256, 256)),
    ];

    for i in 0..SOAK_ITERATIONS {
        let (_, ref bytes) = formats[i % formats.len()];
        let output = process_image_bytes(bytes, ProtectionLevel::Standard, &ctx).unwrap();
        assert!(!output.is_empty());
    }
}

fn create_test_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn create_test_jpeg(width: u32, height: u32) -> Vec<u8> {
    let img = image::DynamicImage::new_rgb8(width, height);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
    buf.into_inner()
}
