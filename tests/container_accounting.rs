use image::ImageEncoder;
use stegoeggo::{
    process_request_bytes_with_report, ImageOutputFormat, ProtectionRequest, ResourceLimits,
    RightsNotice, RightsPolicy,
};

fn textured_image() -> image::DynamicImage {
    let rgb = image::ImageBuffer::from_fn(32, 32, |x, y| {
        image::Rgb([(x * 7) as u8, (y * 11) as u8, ((x + y) * 5) as u8])
    });
    image::DynamicImage::ImageRgb8(rgb)
}

fn encode_png(img: &image::DynamicImage) -> Vec<u8> {
    let mut buf = Vec::new();
    image::codecs::png::PngEncoder::new(&mut buf)
        .write_image(
            &img.to_rgb8(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    buf
}

fn encode_jpeg(img: &image::DynamicImage) -> Vec<u8> {
    let mut buf = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90)
        .write_image(
            &img.to_rgb8(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    buf
}

fn encode_webp(img: &image::DynamicImage) -> Vec<u8> {
    let mut buf = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut buf)
        .write_image(
            &img.to_rgb8(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    buf
}

fn notice() -> RightsNotice {
    RightsNotice::new().with_copyright_holder("Container Accounting")
}

fn request_for(output: ImageOutputFormat) -> ProtectionRequest {
    ProtectionRequest::with_hidden_marker(notice(), RightsPolicy::Allowed)
        .with_seed(7)
        .with_output_format(output)
}

#[test]
fn png_report_tracks_container_counts() {
    let png = encode_png(&textured_image());
    let (_, report) =
        process_request_bytes_with_report(&png, &request_for(ImageOutputFormat::Png)).unwrap();
    let usage = report.resource_usage().expect("usage present");
    assert!(usage.png_chunks_scanned > 0);
    assert_eq!(usage.jpeg_segments_scanned, 0);
    assert_eq!(usage.webp_riff_chunks_scanned, 0);
}

#[test]
fn jpeg_report_tracks_container_counts() {
    let jpeg = encode_jpeg(&textured_image());
    let (_, report) =
        process_request_bytes_with_report(&jpeg, &request_for(ImageOutputFormat::Jpeg)).unwrap();
    let usage = report.resource_usage().expect("usage present");
    assert!(usage.jpeg_segments_scanned > 0);
    assert_eq!(usage.png_chunks_scanned, 0);
    assert_eq!(usage.webp_riff_chunks_scanned, 0);
}

#[test]
fn webp_report_tracks_container_counts() {
    let webp = encode_webp(&textured_image());
    let (_, report) =
        process_request_bytes_with_report(&webp, &request_for(ImageOutputFormat::WebP)).unwrap();
    let usage = report.resource_usage().expect("usage present");
    assert!(usage.webp_riff_chunks_scanned > 0);
    assert_eq!(usage.png_chunks_scanned, 0);
    assert_eq!(usage.jpeg_segments_scanned, 0);
}

#[test]
fn container_limit_at_threshold_passes_and_below_fails() {
    let png = encode_png(&textured_image());
    let (_, baseline) =
        process_request_bytes_with_report(&png, &request_for(ImageOutputFormat::Png)).unwrap();
    let observed = baseline
        .resource_usage()
        .expect("usage present")
        .png_chunks_scanned;
    assert!(observed > 0);

    let at_limit = ResourceLimits::builder().max_png_chunks(observed).build();
    let req = ProtectionRequest::with_hidden_marker(notice(), RightsPolicy::Allowed)
        .with_seed(7)
        .with_output_format(ImageOutputFormat::Png)
        .with_resource_limits(at_limit);
    assert!(process_request_bytes_with_report(&png, &req).is_ok());

    let below_limit = ResourceLimits::builder()
        .max_png_chunks(observed.saturating_sub(1))
        .build();
    let req = ProtectionRequest::with_hidden_marker(notice(), RightsPolicy::Allowed)
        .with_seed(7)
        .with_output_format(ImageOutputFormat::Png)
        .with_resource_limits(below_limit);
    let err = process_request_bytes_with_report(&png, &req).unwrap_err();
    assert!(
        format!("{err}").contains("PNG")
            || matches!(err, stegoeggo::Error::ContainerLimitExceeded { .. }),
        "expected PNG container limit, got: {err}"
    );
}

#[test]
fn jpeg_container_limit_enforced_end_to_end() {
    let jpeg = encode_jpeg(&textured_image());
    let (_, baseline) =
        process_request_bytes_with_report(&jpeg, &request_for(ImageOutputFormat::Jpeg)).unwrap();
    let observed = baseline
        .resource_usage()
        .expect("usage present")
        .jpeg_segments_scanned;
    assert!(observed > 0);

    let below_limit = ResourceLimits::builder()
        .max_jpeg_segments(observed.saturating_sub(1))
        .build();
    let req = ProtectionRequest::with_hidden_marker(notice(), RightsPolicy::Allowed)
        .with_seed(7)
        .with_output_format(ImageOutputFormat::Jpeg)
        .with_resource_limits(below_limit);
    assert!(process_request_bytes_with_report(&jpeg, &req).is_err());
}

#[test]
fn webp_container_limit_enforced_end_to_end() {
    let webp = encode_webp(&textured_image());
    let (_, baseline) =
        process_request_bytes_with_report(&webp, &request_for(ImageOutputFormat::WebP)).unwrap();
    let observed = baseline
        .resource_usage()
        .expect("usage present")
        .webp_riff_chunks_scanned;
    assert!(observed > 0);

    let below_limit = ResourceLimits::builder()
        .max_webp_riff_chunks(observed.saturating_sub(1))
        .build();
    let req = ProtectionRequest::with_hidden_marker(notice(), RightsPolicy::Allowed)
        .with_seed(7)
        .with_output_format(ImageOutputFormat::WebP)
        .with_resource_limits(below_limit);
    assert!(process_request_bytes_with_report(&webp, &req).is_err());
}
