use image::ImageEncoder;
use stegoeggo::{
    process_request_bytes, process_request_bytes_with_report, AuthenticationMode, HiddenMarkerMode,
    ImageOutputFormat, ProtectionChannels, ProtectionRequest, RightsMetadataProtector,
    RightsNotice, RightsPolicy, VerificationStatus,
};

fn textured_image(width: u32, height: u32) -> image::DynamicImage {
    let rgb = image::ImageBuffer::from_fn(width, height, |x, y| {
        let r = ((x * 7 + y * 3) % 256) as u8;
        let g = ((x * 11 + y * 5) % 256) as u8;
        let b = ((x * 13 + y * 9) % 256) as u8;
        image::Rgb([r, g, b])
    });
    image::DynamicImage::ImageRgb8(rgb)
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

fn notice() -> RightsNotice {
    RightsNotice::new().with_copyright_holder("Output Domain Test")
}

fn best_effort_request(output: ImageOutputFormat) -> ProtectionRequest {
    ProtectionRequest::with_hidden_marker(notice(), RightsPolicy::Allowed)
        .with_seed(42)
        .with_output_format(output)
}

fn tiled_request(output: ImageOutputFormat) -> ProtectionRequest {
    ProtectionRequest::new(
        notice(),
        RightsPolicy::Allowed,
        ProtectionChannels {
            rights_metadata: true,
            hidden_marker: HiddenMarkerMode::Tiled { tile_size: 64 },
            authentication: AuthenticationMode::None,
        },
    )
    .with_seed(42)
    .with_output_format(output)
}

fn assert_magic(output: &[u8], format: ImageOutputFormat) {
    match format {
        ImageOutputFormat::Png => assert!(
            ImageOutputFormat::is_png(output),
            "expected PNG magic, got {:02X?}",
            &output[..output.len().min(8)]
        ),
        ImageOutputFormat::Jpeg => assert!(
            ImageOutputFormat::is_jpeg(output),
            "expected JPEG magic, got {:02X?}",
            &output[..output.len().min(8)]
        ),
        ImageOutputFormat::WebP => assert!(
            ImageOutputFormat::is_webp(output),
            "expected WebP magic, got {:02X?}",
            &output[..output.len().min(12)]
        ),
        _ => panic!("unsupported output format for test"),
    }
}

#[test]
fn jpeg_to_png_best_effort_verifies_lsb_output() {
    let img = textured_image(256, 256);
    let jpeg_bytes = image_to_jpeg_bytes(&img, 95);
    let request = best_effort_request(ImageOutputFormat::Png);
    let (output, report) = process_request_bytes_with_report(&jpeg_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::Png);
    assert!(report.format_transcoded);
    let summary = report.embed_summary().expect("embed summary present");
    assert_eq!(summary.path, stegoeggo::EmbedPath::Lsb);
    assert!(image::load_from_memory(&output).is_ok());
    assert_eq!(
        stegoeggo::verify_image_bytes(&output, &[]),
        VerificationStatus::Verified
    );
}

#[test]
fn jpeg_to_webp_best_effort_verifies_lsb_output() {
    let img = textured_image(256, 256);
    let jpeg_bytes = image_to_jpeg_bytes(&img, 95);
    let request = best_effort_request(ImageOutputFormat::WebP);
    let (output, report) = process_request_bytes_with_report(&jpeg_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::WebP);
    assert!(report.format_transcoded);
    let summary = report.embed_summary().expect("embed summary present");
    assert_eq!(summary.path, stegoeggo::EmbedPath::Lsb);
    assert!(image::load_from_memory(&output).is_ok());
    assert_eq!(
        stegoeggo::verify_image_bytes(&output, &[]),
        VerificationStatus::Verified
    );
}

#[test]
fn jpeg_to_png_tiled_verifies_lsb_output() {
    let img = textured_image(256, 256);
    let jpeg_bytes = image_to_jpeg_bytes(&img, 95);
    let request = tiled_request(ImageOutputFormat::Png);
    let (output, report) = process_request_bytes_with_report(&jpeg_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::Png);
    assert!(report.format_transcoded);
    let summary = report.embed_summary().expect("embed summary present");
    assert_eq!(summary.path, stegoeggo::EmbedPath::LsbTiled);
    assert!(image::load_from_memory(&output).is_ok());
    assert_eq!(
        stegoeggo::verify_image_bytes(&output, &[]),
        VerificationStatus::Verified
    );
}

#[test]
fn jpeg_to_webp_tiled_verifies_lsb_output() {
    let img = textured_image(256, 256);
    let jpeg_bytes = image_to_jpeg_bytes(&img, 95);
    let request = tiled_request(ImageOutputFormat::WebP);
    let (output, report) = process_request_bytes_with_report(&jpeg_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::WebP);
    assert!(report.format_transcoded);
    let summary = report.embed_summary().expect("embed summary present");
    assert_eq!(summary.path, stegoeggo::EmbedPath::LsbTiled);
    assert!(image::load_from_memory(&output).is_ok());
    assert_eq!(
        stegoeggo::verify_image_bytes(&output, &[]),
        VerificationStatus::Verified
    );
}

#[test]
fn png_to_jpeg_best_effort_reports_dct_path() {
    let img = textured_image(256, 256);
    let png_bytes = image_to_png_bytes(&img);
    let request = best_effort_request(ImageOutputFormat::Jpeg);
    let (output, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::Jpeg);
    assert!(report.format_transcoded);
    let summary = report.embed_summary().expect("embed summary present");
    assert_eq!(summary.path, stegoeggo::EmbedPath::DctF5);
    assert!(image::load_from_memory(&output).is_ok());
    assert_eq!(
        stegoeggo::verify_image_bytes(&output, &[]),
        VerificationStatus::Verified
    );
}

#[test]
fn webp_to_jpeg_best_effort_reports_dct_path() {
    let img = textured_image(256, 256);
    let webp_bytes = image_to_webp_bytes(&img);
    let request = best_effort_request(ImageOutputFormat::Jpeg);
    let (output, report) = process_request_bytes_with_report(&webp_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::Jpeg);
    assert!(report.format_transcoded);
    let summary = report.embed_summary().expect("embed summary present");
    assert_eq!(summary.path, stegoeggo::EmbedPath::DctF5);
    assert!(image::load_from_memory(&output).is_ok());
    assert_eq!(
        stegoeggo::verify_image_bytes(&output, &[]),
        VerificationStatus::Verified
    );
}

#[test]
fn jpeg_to_jpeg_best_effort_reports_dct_path() {
    let img = textured_image(256, 256);
    let jpeg_bytes = image_to_jpeg_bytes(&img, 95);
    let request = best_effort_request(ImageOutputFormat::Jpeg);
    let (output, report) = process_request_bytes_with_report(&jpeg_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::Jpeg);
    assert!(!report.format_transcoded);
    let summary = report.embed_summary().expect("embed summary present");
    assert_eq!(summary.path, stegoeggo::EmbedPath::DctF5);
    assert!(image::load_from_memory(&output).is_ok());
    assert_eq!(
        stegoeggo::verify_image_bytes(&output, &[]),
        VerificationStatus::Verified
    );
}

#[test]
fn png_to_png_best_effort_reports_lsb_path() {
    let img = textured_image(256, 256);
    let png_bytes = image_to_png_bytes(&img);
    let request = best_effort_request(ImageOutputFormat::Png);
    let (output, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::Png);
    assert!(!report.format_transcoded);
    let summary = report.embed_summary().expect("embed summary present");
    assert_eq!(summary.path, stegoeggo::EmbedPath::Lsb);
    assert!(image::load_from_memory(&output).is_ok());
    assert_eq!(
        stegoeggo::verify_image_bytes(&output, &[]),
        VerificationStatus::Verified
    );
}

#[test]
fn webp_to_webp_best_effort_reports_lsb_path() {
    let img = textured_image(256, 256);
    let webp_bytes = image_to_webp_bytes(&img);
    let request = best_effort_request(ImageOutputFormat::WebP);
    let (output, report) = process_request_bytes_with_report(&webp_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::WebP);
    assert!(!report.format_transcoded);
    let summary = report.embed_summary().expect("embed summary present");
    assert_eq!(summary.path, stegoeggo::EmbedPath::Lsb);
    assert!(image::load_from_memory(&output).is_ok());
    assert_eq!(
        stegoeggo::verify_image_bytes(&output, &[]),
        VerificationStatus::Verified
    );
}

fn seed_only_request(output: ImageOutputFormat) -> ProtectionRequest {
    ProtectionRequest::new(
        notice(),
        RightsPolicy::Allowed,
        ProtectionChannels {
            rights_metadata: true,
            hidden_marker: HiddenMarkerMode::SeedOnly,
            authentication: AuthenticationMode::None,
        },
    )
    .with_seed(777)
    .with_output_format(output)
}

fn metadata_only_request(output: ImageOutputFormat) -> ProtectionRequest {
    ProtectionRequest::metadata_only(notice(), RightsPolicy::Allowed)
        .with_seed(777)
        .with_output_format(output)
}

fn encode_fixture(img: &image::DynamicImage, format: ImageOutputFormat) -> Vec<u8> {
    match format {
        ImageOutputFormat::Png => image_to_png_bytes(img),
        ImageOutputFormat::Jpeg => image_to_jpeg_bytes(img, 95),
        ImageOutputFormat::WebP => image_to_webp_bytes(img),
        _ => panic!("unsupported fixture format"),
    }
}

fn expected_best_effort_path(output: ImageOutputFormat) -> stegoeggo::EmbedPath {
    match output {
        ImageOutputFormat::Jpeg => stegoeggo::EmbedPath::DctF5,
        _ => stegoeggo::EmbedPath::Lsb,
    }
}

fn expected_tiled_path(output: ImageOutputFormat) -> stegoeggo::EmbedPath {
    match output {
        ImageOutputFormat::Jpeg => stegoeggo::EmbedPath::DctF5Tiled,
        _ => stegoeggo::EmbedPath::LsbTiled,
    }
}

#[test]
fn best_effort_full_matrix_carrier_follows_output() {
    let img = textured_image(256, 256);
    let inputs = [
        (ImageOutputFormat::Png, "png"),
        (ImageOutputFormat::Jpeg, "jpeg"),
        (ImageOutputFormat::WebP, "webp"),
    ];
    let outputs = [
        ImageOutputFormat::Png,
        ImageOutputFormat::Jpeg,
        ImageOutputFormat::WebP,
    ];
    for (input_format, label) in inputs {
        let input_bytes = encode_fixture(&img, input_format);
        for output_format in outputs {
            let request = best_effort_request(output_format);
            let (output, report) =
                process_request_bytes_with_report(&input_bytes, &request).unwrap();
            assert_magic(&output, output_format);
            assert_eq!(
                report.format_transcoded,
                input_format != output_format,
                "{label}->{output_format:?}: transcoded flag"
            );
            let summary = report.embed_summary().expect("embed summary present");
            assert_eq!(
                summary.path,
                expected_best_effort_path(output_format),
                "{label}->{output_format:?}: carrier path"
            );
            assert!(
                image::load_from_memory(&output).is_ok(),
                "{label}->{output_format:?}: decodable"
            );
            assert_eq!(
                stegoeggo::verify_image_bytes(&output, &[]),
                VerificationStatus::Verified,
                "{label}->{output_format:?}: marker verifies"
            );
        }
    }
}

#[test]
fn png_to_webp_best_effort_reports_lsb_path() {
    let img = textured_image(256, 256);
    let png_bytes = image_to_png_bytes(&img);
    let request = best_effort_request(ImageOutputFormat::WebP);
    let (output, report) = process_request_bytes_with_report(&png_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::WebP);
    assert!(report.format_transcoded);
    let summary = report.embed_summary().expect("embed summary present");
    assert_eq!(summary.path, stegoeggo::EmbedPath::Lsb);
    assert_eq!(
        stegoeggo::verify_image_bytes(&output, &[]),
        VerificationStatus::Verified
    );
}

#[test]
fn webp_to_png_best_effort_reports_lsb_path() {
    let img = textured_image(256, 256);
    let webp_bytes = image_to_webp_bytes(&img);
    let request = best_effort_request(ImageOutputFormat::Png);
    let (output, report) = process_request_bytes_with_report(&webp_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::Png);
    assert!(report.format_transcoded);
    let summary = report.embed_summary().expect("embed summary present");
    assert_eq!(summary.path, stegoeggo::EmbedPath::Lsb);
    assert_eq!(
        stegoeggo::verify_image_bytes(&output, &[]),
        VerificationStatus::Verified
    );
}

#[test]
fn tiled_matrix_carrier_follows_output() {
    let img = textured_image(256, 256);
    let cases = [
        (ImageOutputFormat::Jpeg, ImageOutputFormat::Png),
        (ImageOutputFormat::Jpeg, ImageOutputFormat::WebP),
        (ImageOutputFormat::Png, ImageOutputFormat::Jpeg),
        (ImageOutputFormat::WebP, ImageOutputFormat::Jpeg),
        (ImageOutputFormat::Png, ImageOutputFormat::Png),
        (ImageOutputFormat::WebP, ImageOutputFormat::WebP),
        (ImageOutputFormat::Jpeg, ImageOutputFormat::Jpeg),
        (ImageOutputFormat::Png, ImageOutputFormat::WebP),
        (ImageOutputFormat::WebP, ImageOutputFormat::Png),
    ];
    for (input_format, output_format) in cases {
        let input_bytes = encode_fixture(&img, input_format);
        let request = tiled_request(output_format);
        let (output, report) = process_request_bytes_with_report(&input_bytes, &request).unwrap();
        assert_magic(&output, output_format);
        assert_eq!(
            report.format_transcoded,
            input_format != output_format,
            "{input_format:?}->{output_format:?}: transcoded flag"
        );
        let summary = report.embed_summary().expect("embed summary present");
        assert_eq!(
            summary.path,
            expected_tiled_path(output_format),
            "{input_format:?}->{output_format:?}: carrier path"
        );
        assert!(
            image::load_from_memory(&output).is_ok(),
            "{input_format:?}->{output_format:?}: decodable"
        );
        assert_eq!(
            stegoeggo::verify_image_bytes(&output, &[]),
            VerificationStatus::Verified,
            "{input_format:?}->{output_format:?}: marker verifies"
        );
    }
}

#[test]
fn seed_only_jpeg_to_jpeg_preserves_seed_hint() {
    let img = textured_image(256, 256);
    let jpeg_bytes = image_to_jpeg_bytes(&img, 95);
    let request = seed_only_request(ImageOutputFormat::Jpeg);
    let output = process_request_bytes(&jpeg_bytes, &request).unwrap();
    assert_magic(&output, ImageOutputFormat::Jpeg);
    let hint = stegoeggo::stego::jpeg::extract_seed_hint(&output).unwrap();
    assert_eq!(hint, Some(777));
    assert!(image::load_from_memory(&output).is_ok());
}

#[test]
fn seed_only_raster_to_jpeg_uses_jpeg_seed_hint() {
    for input_format in [ImageOutputFormat::Png, ImageOutputFormat::WebP] {
        let img = textured_image(256, 256);
        let input_bytes = encode_fixture(&img, input_format);
        let request = seed_only_request(ImageOutputFormat::Jpeg);
        let output = process_request_bytes(&input_bytes, &request).unwrap();
        assert_magic(&output, ImageOutputFormat::Jpeg);
        let hint = stegoeggo::stego::jpeg::extract_seed_hint(&output).unwrap();
        assert_eq!(hint, Some(777), "{input_format:?}->Jpeg seed hint");
        assert!(image::load_from_memory(&output).is_ok());
    }
}

#[test]
fn seed_only_jpeg_to_raster_uses_raster_seed_fallback() {
    let img = textured_image(256, 256);
    let jpeg_bytes = image_to_jpeg_bytes(&img, 95);
    for output_format in [ImageOutputFormat::Png, ImageOutputFormat::WebP] {
        let seed_request = seed_only_request(output_format);
        let seed_output = process_request_bytes(&jpeg_bytes, &seed_request).unwrap();
        assert_magic(&seed_output, output_format);
        assert!(image::load_from_memory(&seed_output).is_ok());
        let seed_img = image::load_from_memory(&seed_output).unwrap().to_rgb8();
        let seed_raw = seed_img.as_raw();

        let meta_request = metadata_only_request(output_format);
        let meta_output = process_request_bytes(&jpeg_bytes, &meta_request).unwrap();
        let meta_img = image::load_from_memory(&meta_output).unwrap().to_rgb8();
        let meta_raw = meta_img.as_raw();

        let mut diff = 0usize;
        for i in 0..seed_raw.len().min(192) {
            if seed_raw[i] != meta_raw[i] {
                diff += 1;
            }
        }
        assert!(
            diff > 0,
            "Jpeg->{output_format:?} SeedOnly must write raster seed marker (diffs={diff})"
        );
        let seed = RightsMetadataProtector::extract_seed_from_image(&seed_output);
        assert_eq!(seed, Some(777));
    }
}

#[test]
fn seed_only_same_format_raster_preserves_seed_behavior() {
    let img = textured_image(256, 256);
    for format in [ImageOutputFormat::Png, ImageOutputFormat::WebP] {
        let input_bytes = encode_fixture(&img, format);
        let request = seed_only_request(format);
        let output = process_request_bytes(&input_bytes, &request).unwrap();
        assert_magic(&output, format);
        assert!(image::load_from_memory(&output).is_ok());
        let seed = RightsMetadataProtector::extract_seed_from_image(&output);
        assert_eq!(seed, Some(777), "{format:?} metadata seed");
    }
}
