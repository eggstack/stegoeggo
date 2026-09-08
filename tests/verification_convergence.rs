use image::{ImageBuffer, Rgba, RgbaImage};
use stegoeggo::{
    process_request_bytes, verify_image_bytes, verify_image_bytes_detailed,
    verify_image_bytes_report, verify_legal_notice, AuthenticationMode, HiddenMarkerMode,
    ProtectionChannels, ProtectionRequest, ResourceLimits, RightsNotice, RightsPolicy,
    VerificationResult, VerificationStatus,
};

fn make_png(width: u32, height: u32, value: u8) -> Vec<u8> {
    let img = RgbaImage::from_fn(width, height, |x, y| {
        Rgba([
            value.wrapping_add((x * 3) as u8),
            value.wrapping_add((y * 5) as u8),
            value.wrapping_add(((x + y) * 7) as u8),
            255,
        ])
    });
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::DynamicImage::ImageRgba8(img)
        .write_with_encoder(encoder)
        .unwrap();
    buf
}

fn crc_request(seed: u64) -> ProtectionRequest {
    let notice = RightsNotice::new().with_copyright_holder("Convergence");
    ProtectionRequest::with_hidden_marker(notice, RightsPolicy::ProhibitedAiMlTraining)
        .with_seed(seed)
        .with_intensity(0.5)
}

fn hmac_request(seed: u64, key: &[u8]) -> ProtectionRequest {
    let notice = RightsNotice::new().with_copyright_holder("Convergence");
    let channels = ProtectionChannels {
        rights_metadata: true,
        hidden_marker: HiddenMarkerMode::BestEffort,
        authentication: AuthenticationMode::Hmac,
    };
    ProtectionRequest::new(notice, RightsPolicy::ProhibitedAiMlTraining, channels)
        .with_seed(seed)
        .with_intensity(0.5)
        .with_mac_key(key.to_vec())
}

fn metadata_only_request() -> ProtectionRequest {
    let notice = RightsNotice::new().with_copyright_holder("Convergence");
    ProtectionRequest::metadata_only(notice, RightsPolicy::ProhibitedAiMlTraining)
}

fn tiled_request(seed: u64) -> ProtectionRequest {
    let notice = RightsNotice::new().with_copyright_holder("Convergence");
    let channels = ProtectionChannels {
        rights_metadata: true,
        hidden_marker: HiddenMarkerMode::Tiled { tile_size: 64 },
        authentication: AuthenticationMode::None,
    };
    ProtectionRequest::new(notice, RightsPolicy::ProhibitedAiMlTraining, channels)
        .with_seed(seed)
        .with_intensity(0.5)
}

fn corrupt_lsb(bytes: &[u8]) -> Vec<u8> {
    let img = image::load_from_memory(bytes).unwrap().to_rgba8();
    let (w, h) = img.dimensions();
    let mut out = img.clone();
    for y in 0..h {
        for x in 0..w {
            let p = out.get_pixel_mut(x, y);
            p[0] ^= 0x01;
            p[1] ^= 0x01;
            p[2] ^= 0x01;
        }
    }
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::DynamicImage::ImageRgba8(out)
        .write_with_encoder(encoder)
        .unwrap();
    buf
}

fn strip_metadata(bytes: &[u8]) -> Vec<u8> {
    let img = image::load_from_memory(bytes).unwrap();
    let rgba = img.to_rgba8();
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::DynamicImage::ImageRgba8(rgba)
        .write_with_encoder(encoder)
        .unwrap();
    buf
}

struct MatrixCase {
    name: &'static str,
    bytes: Vec<u8>,
    key: Vec<u8>,
    limits: Option<ResourceLimits>,
}

fn assert_consistent(case: &MatrixCase) {
    let status = if let Some(limits) = &case.limits {
        stegoeggo::verify_image_bytes_with_limits(&case.bytes, &case.key, limits)
    } else {
        verify_image_bytes(&case.bytes, &case.key)
    };
    let detailed = if let Some(limits) = &case.limits {
        stegoeggo::verify_image_bytes_detailed_with_limits(&case.bytes, &case.key, limits)
    } else {
        verify_image_bytes_detailed(&case.bytes, &case.key)
    };
    let report = if let Some(limits) = &case.limits {
        stegoeggo::verify_image_bytes_report_with_limits(&case.bytes, &case.key, limits)
    } else {
        verify_image_bytes_report(&case.bytes, &case.key)
    };
    let notice = if let Some(limits) = &case.limits {
        stegoeggo::verify_legal_notice_with_limits(&case.bytes, &case.key, limits)
    } else {
        verify_legal_notice(&case.bytes, &case.key)
    };

    assert_eq!(
        report.hidden_marker().status(),
        status,
        "{}: report hidden status must match VerificationStatus",
        case.name
    );
    assert_eq!(
        notice.stego_status(),
        status,
        "{}: NoticeVerification stego_status must match VerificationStatus",
        case.name
    );
    assert_eq!(
        report.rights().found(),
        notice.has_notice(),
        "{}: report rights.found must match notice.has_notice",
        case.name
    );
    assert_eq!(
        report.evidence_strength(),
        notice.evidence_strength(),
        "{}: evidence strength must agree",
        case.name
    );

    match status {
        VerificationStatus::Verified => {
            assert!(
                matches!(detailed, VerificationResult::Verified { .. }),
                "{}: Verified status must project to Verified result, got {detailed:?}",
                case.name
            );
            assert!(
                detailed.is_verified(),
                "{}: result must be verified",
                case.name
            );
        }
        VerificationStatus::NotFound => {
            assert!(
                !matches!(detailed, VerificationResult::Verified { .. })
                    && !matches!(detailed, VerificationResult::Corrupted { .. }),
                "{}: NotFound status must not project to Verified/Corrupted, got {detailed:?}",
                case.name
            );
        }
        VerificationStatus::Invalid => {
            assert!(
                !matches!(detailed, VerificationResult::Verified { .. }),
                "{}: Invalid status must never project to Verified, got {detailed:?}",
                case.name
            );
        }
    }

    let cli_stego = format!("{:?}", notice.stego_status());
    let report_stego = format!("{:?}", report.hidden_marker().status());
    assert_eq!(
        cli_stego, report_stego,
        "{}: CLI JSON stego_status must match report",
        case.name
    );
    let cli_evidence = notice.evidence_strength().to_string();
    let report_evidence = report.evidence_strength().to_string();
    assert_eq!(
        cli_evidence, report_evidence,
        "{}: CLI JSON evidence_strength must match report",
        case.name
    );
    assert_eq!(
        notice.copyright_holder().map(str::to_string),
        report.rights().copyright_holder().map(str::to_string),
        "{}: CLI JSON copyright_holder must match report rights",
        case.name
    );
}

#[test]
fn verification_matrix_cross_api_consistency() {
    let plain = make_png(64, 64, 128);
    let crc = process_request_bytes(&plain, &crc_request(42)).unwrap();
    let hmac_key = b"matrix-key";
    let hmac = process_request_bytes(&plain, &hmac_request(42, hmac_key)).unwrap();
    let corrupted = corrupt_lsb(&crc);
    let metadata_only = process_request_bytes(&plain, &metadata_only_request()).unwrap();
    let tiled_base = make_png(256, 256, 77);
    let tiled = process_request_bytes(&tiled_base, &tiled_request(42)).unwrap();
    let stripped = strip_metadata(&crc);
    let truncated: Vec<u8> = crc.iter().take(200).cloned().collect();
    let tiny_limits = ResourceLimits::builder().max_payload_bytes(1).build();

    let cases = vec![
        MatrixCase {
            name: "clean_crc",
            bytes: crc.clone(),
            key: Vec::new(),
            limits: None,
        },
        MatrixCase {
            name: "hmac_correct_key",
            bytes: hmac.clone(),
            key: hmac_key.to_vec(),
            limits: None,
        },
        MatrixCase {
            name: "hmac_missing_key",
            bytes: hmac.clone(),
            key: Vec::new(),
            limits: None,
        },
        MatrixCase {
            name: "hmac_wrong_key",
            bytes: hmac.clone(),
            key: b"wrong-key".to_vec(),
            limits: None,
        },
        MatrixCase {
            name: "corrupted_payload",
            bytes: corrupted,
            key: Vec::new(),
            limits: None,
        },
        MatrixCase {
            name: "metadata_only",
            bytes: metadata_only.clone(),
            key: Vec::new(),
            limits: None,
        },
        MatrixCase {
            name: "no_protection",
            bytes: plain.clone(),
            key: Vec::new(),
            limits: None,
        },
        MatrixCase {
            name: "malformed_truncated",
            bytes: truncated,
            key: Vec::new(),
            limits: None,
        },
        MatrixCase {
            name: "unsupported_garbage_png_magic",
            bytes: {
                let mut v = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
                v.extend(vec![0xFF; 512]);
                v
            },
            key: Vec::new(),
            limits: None,
        },
        MatrixCase {
            name: "seed_only_light_like",
            bytes: {
                let req = ProtectionRequest::metadata_only(
                    RightsNotice::new().with_copyright_holder("Convergence"),
                    RightsPolicy::Unspecified,
                );
                process_request_bytes(&plain, &req).unwrap()
            },
            key: Vec::new(),
            limits: None,
        },
        MatrixCase {
            name: "tiled_lsb",
            bytes: tiled,
            key: Vec::new(),
            limits: None,
        },
        MatrixCase {
            name: "resource_limit_exhaustion",
            bytes: crc.clone(),
            key: Vec::new(),
            limits: Some(tiny_limits),
        },
        MatrixCase {
            name: "stripped_marker_survives",
            bytes: stripped,
            key: Vec::new(),
            limits: None,
        },
    ];

    assert_eq!(cases.len(), 13, "matrix must cover 13 cases");
    for case in &cases {
        assert_consistent(case);
    }

    let crc_status = verify_image_bytes(&crc, &[]);
    assert_eq!(crc_status, VerificationStatus::Verified);
    let hmac_status = verify_image_bytes(&hmac, hmac_key);
    assert_eq!(hmac_status, VerificationStatus::Verified);
    let missing = verify_image_bytes(&hmac, &[]);
    assert_eq!(missing, VerificationStatus::Invalid);
    let wrong = verify_image_bytes(&hmac, b"wrong-key");
    assert_eq!(wrong, VerificationStatus::Invalid);
    let none_status = verify_image_bytes(&plain, &[]);
    assert_eq!(none_status, VerificationStatus::NotFound);
    let meta_report = verify_image_bytes_report(&metadata_only, &[]);
    assert_eq!(
        meta_report.hidden_marker().status(),
        VerificationStatus::NotFound
    );
    assert!(meta_report.rights().found());
    assert_eq!(meta_report.summary_status(), VerificationStatus::Verified);
}

#[test]
fn tiled_crop_recovery_is_consistent() {
    let base = make_png(256, 256, 77);
    let protected = process_request_bytes(&base, &tiled_request(42)).unwrap();
    let img = image::load_from_memory(&protected).unwrap();
    let cropped = img.crop_imm(64, 0, 64, 64);
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    cropped.write_with_encoder(encoder).unwrap();

    let status = verify_image_bytes(&buf, &[]);
    let detailed = verify_image_bytes_detailed(&buf, &[]);
    let report = verify_image_bytes_report(&buf, &[]);
    let notice = verify_legal_notice(&buf, &[]);
    assert_eq!(report.hidden_marker().status(), status);
    assert_eq!(notice.stego_status(), status);
    assert_eq!(report.evidence_strength(), notice.evidence_strength());
    assert!(
        !matches!(detailed, VerificationResult::Verified { .. })
            || status == VerificationStatus::Verified,
        "cropped tiled result must be consistent with status"
    );
}

#[test]
fn jpeg_tiled_recovery_is_consistent() {
    let img: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(128, 128, |x, y| image::Rgb([x as u8, y as u8, 128]));
    let mut jpeg_in = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_in, 90);
    image::DynamicImage::ImageRgb8(img)
        .write_with_encoder(encoder)
        .unwrap();
    let notice = RightsNotice::new().with_copyright_holder("Convergence");
    let channels = ProtectionChannels {
        rights_metadata: true,
        hidden_marker: HiddenMarkerMode::Tiled { tile_size: 64 },
        authentication: AuthenticationMode::None,
    };
    let request = ProtectionRequest::new(notice, RightsPolicy::ProhibitedAiMlTraining, channels)
        .with_seed(42)
        .with_intensity(0.5)
        .with_output_format(stegoeggo::ImageOutputFormat::Jpeg);
    let protected = process_request_bytes(&jpeg_in, &request).unwrap();
    let status = verify_image_bytes(&protected, &[]);
    let report = verify_image_bytes_report(&protected, &[]);
    let notice_out = verify_legal_notice(&protected, &[]);
    assert_eq!(status, report.hidden_marker().status());
    assert_eq!(status, notice_out.stego_status());
    assert_eq!(report.evidence_strength(), notice_out.evidence_strength());
}
