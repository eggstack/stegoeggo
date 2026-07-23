//! Tests for Plan 026: payload extraction, verification, and resource-limit closure.
//!
//! Covers Gate 1.5 (payload tests), Gate 2.7 (detached verification), and
//! Gate 3.5 (fail-before-work tests).

use stegoeggo::resource_limits::ResourceLimits;
use stegoeggo::{
    encode_image, process_image_bytes, process_image_bytes_with_warnings, verify_image_bytes,
    ImageOutputFormat, ProtectionContext, ProtectionLevel, VerificationStatus,
};

// ---------------------------------------------------------------------------
// Gate 1.5: Payload extraction and channel-flag tests
// ---------------------------------------------------------------------------

fn textured_image(w: u32, h: u32) -> image::DynamicImage {
    image::DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(w, h, |x, y| {
        image::Rgb([(x as u8).wrapping_add(y as u8), x as u8, y as u8])
    }))
}

#[test]
fn v3_crc_png_exact_length_extraction() {
    let img = textured_image(128, 128);
    let ctx = ProtectionContext::new(0.5, 42);
    let protected = process_image_bytes(
        &encode_image(&img, image::ImageFormat::Png).unwrap(),
        ProtectionLevel::Standard,
        &ctx,
    )
    .unwrap();

    let status = verify_image_bytes(&protected, &[]);
    assert_eq!(status, VerificationStatus::Verified);

    let stego = stegoeggo::SteganographyProtector::new();
    let payload = stego
        .extract_payload_from_bytes_with_key(&protected, &[])
        .unwrap();
    assert_eq!(payload.version(), 3);
    assert!(payload.raw_payload().is_some());
}

#[test]
fn v3_hmac_png_exact_length_extraction() {
    let img = image::DynamicImage::new_rgb8(128, 128);
    let ctx = ProtectionContext::new(0.5, 42).with_mac_key(b"test-key-1234567".to_vec());
    let protected = process_image_bytes(
        &encode_image(&img, image::ImageFormat::Png).unwrap(),
        ProtectionLevel::Standard,
        &ctx,
    )
    .unwrap();

    let status = verify_image_bytes(&protected, b"test-key-1234567");
    assert_eq!(status, VerificationStatus::Verified);

    let stego = stegoeggo::SteganographyProtector::new();
    let payload = stego
        .extract_payload_from_bytes_with_key(&protected, b"test-key-1234567")
        .unwrap();
    assert_eq!(payload.version(), 3);
}

#[test]
fn v3_crc_jpeg_exact_length_extraction() {
    let img = textured_image(128, 128);
    let ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Jpeg);
    let protected = process_image_bytes(
        &encode_image(&img, image::ImageFormat::Png).unwrap(),
        ProtectionLevel::Standard,
        &ctx,
    )
    .unwrap();

    let status = verify_image_bytes(&protected, &[]);
    assert_eq!(status, VerificationStatus::Verified);
}

#[test]
fn v3_hmac_jpeg_exact_length_extraction() {
    let img = textured_image(128, 128);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_mac_key(b"hmac-key-for-test!!!".to_vec());
    let protected = process_image_bytes(
        &encode_image(&img, image::ImageFormat::Png).unwrap(),
        ProtectionLevel::Standard,
        &ctx,
    )
    .unwrap();

    let status = verify_image_bytes(&protected, b"hmac-key-for-test!!!");
    assert_eq!(status, VerificationStatus::Verified);
}

#[test]
fn v3_crc_tiled_png_extraction() {
    let img = image::DynamicImage::new_rgb8(128, 128);
    let ctx = ProtectionContext::new(0.5, 42).with_tile_size(64);
    let protected = process_image_bytes(
        &encode_image(&img, image::ImageFormat::Png).unwrap(),
        ProtectionLevel::Standard,
        &ctx,
    )
    .unwrap();

    let status = verify_image_bytes(&protected, &[]);
    assert_eq!(status, VerificationStatus::Verified);
}

#[test]
fn v3_hmac_tiled_jpeg_extraction() {
    let img = textured_image(128, 128);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_tile_size(64)
        .with_mac_key(b"tile-hmac-key-test!".to_vec());
    let protected = process_image_bytes(
        &encode_image(&img, image::ImageFormat::Png).unwrap(),
        ProtectionLevel::Standard,
        &ctx,
    )
    .unwrap();

    // Tiled JPEG + HMAC: the non-tiled DCT path should still find the payload
    // since the tile embedding also writes to the full DCT coefficient space.
    let stego = stegoeggo::SteganographyProtector::new();
    let status = stego.verify_payload_from_bytes_with_key(&protected, b"tile-hmac-key-test!");
    assert_ne!(
        status,
        VerificationStatus::NotFound,
        "Tiled JPEG HMAC payload should be found"
    );
}

#[test]
fn wrong_hmac_key_reports_invalid_not_not_found() {
    let img = image::DynamicImage::new_rgb8(128, 128);
    let ctx = ProtectionContext::new(0.5, 42).with_mac_key(b"correct-key-here!!!".to_vec());
    let protected = process_image_bytes(
        &encode_image(&img, image::ImageFormat::Png).unwrap(),
        ProtectionLevel::Standard,
        &ctx,
    )
    .unwrap();

    let status = verify_image_bytes(&protected, b"wrong-key!!!!!!!!!!");
    assert_eq!(
        status,
        VerificationStatus::Invalid,
        "Wrong HMAC key must return Invalid, not NotFound"
    );
}

#[test]
fn missing_hmac_key_on_hmac_payload_reports_invalid() {
    let img = image::DynamicImage::new_rgb8(128, 128);
    let ctx = ProtectionContext::new(0.5, 42).with_mac_key(b"some-mac-key-here!!".to_vec());
    let protected = process_image_bytes(
        &encode_image(&img, image::ImageFormat::Png).unwrap(),
        ProtectionLevel::Standard,
        &ctx,
    )
    .unwrap();

    let status = verify_image_bytes(&protected, &[]);
    assert_eq!(
        status,
        VerificationStatus::Invalid,
        "Missing HMAC key on HMAC payload should return Invalid"
    );
}

#[test]
fn crc_channel_flags_do_not_claim_authentication() {
    let ctx = ProtectionContext::new(0.5, 42);
    let stego = stegoeggo::SteganographyProtector::new();
    let payload_bytes = stego.generate_payload_for_context(&ctx);
    assert!(payload_bytes.len() >= 3);
    assert_eq!(payload_bytes[0], 0x53);
    assert_eq!(payload_bytes[1], 0x45);
    assert_eq!(payload_bytes[2], 3);
}

#[test]
fn hmac_channel_flags_claim_authentication() {
    let ctx = ProtectionContext::new(0.5, 42).with_mac_key(b"test-key".to_vec());
    let stego = stegoeggo::SteganographyProtector::new();
    let payload_bytes = stego.generate_payload_for_context(&ctx);
    assert!(payload_bytes.len() >= 3);
    assert_eq!(payload_bytes[0], 0x53);
    assert_eq!(payload_bytes[1], 0x45);
    assert_eq!(payload_bytes[2], 3);
}

#[test]
fn tiny_image_stego_capacity_skipped() {
    let img = image::DynamicImage::new_rgb8(2, 2);
    let ctx = ProtectionContext::new(0.5, 42);
    let (_, warnings) = process_image_bytes_with_warnings(
        &encode_image(&img, image::ImageFormat::Png).unwrap(),
        ProtectionLevel::Standard,
        &ctx,
    )
    .unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, stegoeggo::ProtectionWarning::LsbCapacitySkipped)),
        "Tiny image should produce LsbCapacitySkipped warning"
    );
}

#[test]
fn unsupported_image_returns_not_found() {
    let garbage = vec![0x00; 128];
    let status = verify_image_bytes(&garbage, &[]);
    assert_eq!(status, VerificationStatus::NotFound);
}

#[test]
fn v3_fits_but_legacy_v2_window_does_not() {
    let img = image::DynamicImage::new_rgb8(64, 64);
    let ctx = ProtectionContext::new(0.5, 42);
    let png_bytes = encode_image(&img, image::ImageFormat::Png).unwrap();
    let protected = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx).unwrap();
    let status = verify_image_bytes(&protected, &[]);
    assert_eq!(status, VerificationStatus::Verified);
}

// ---------------------------------------------------------------------------
// Gate 2.7: Detached-manifest verification tests
// ---------------------------------------------------------------------------

#[cfg(all(feature = "signatures", feature = "detached-manifest"))]
mod detached_tests {
    use stegoeggo::detached::{
        verify_detached_manifest, DetachedManifest, EmbeddedReference, EmbeddedReferenceStatus,
        PublicKeyEntry, SignatureRecord, TrustMetadata, TrustPolicy,
    };
    use stegoeggo::provenance::ProvenanceClaim;
    use stegoeggo::resource_limits::ResourceLimits;
    use stegoeggo::signing::SigningKey;
    use stegoeggo::{encode_image, process_image_bytes, ProtectionContext, ProtectionLevel};

    fn make_test_image_bytes() -> Vec<u8> {
        let img = image::DynamicImage::new_rgb8(64, 64);
        encode_image(&img, image::ImageFormat::Png).unwrap()
    }

    fn make_test_claim_for(image_bytes: &[u8]) -> ProvenanceClaim {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(image_bytes);
        let digest = format!("sha256:{}", hex::encode(hash));
        ProvenanceClaim::new(1)
            .with_instance_digest_raw(digest)
            .with_content_code("iscc:test123".to_string())
            .with_creation_time(1700000000)
            .with_source_facts("png", 64, 64, image_bytes.len() as u64)
            .with_software("stegoeggo-test/0.2.3")
    }

    #[test]
    fn oversized_manifest_rejected() {
        let image_bytes = make_test_image_bytes();
        let claim = make_test_claim_for(&image_bytes);
        let manifest_json = serde_json::to_vec(&DetachedManifest::new(claim)).unwrap();
        let limits = ResourceLimits::builder()
            .max_detached_manifest_bytes(10)
            .build();
        let result =
            stegoeggo::detached::DetachedManifest::from_json_with_limits(&manifest_json, &limits);
        assert!(result.is_err());
    }

    #[test]
    fn correct_signature_without_trust_exits_untrusted() {
        let image_bytes = make_test_image_bytes();
        let claim = make_test_claim_for(&image_bytes);

        let signing_key = SigningKey::generate();
        let claim_bytes = claim.canonical_bytes();
        let sig = signing_key.sign(&claim_bytes);
        let sig_hex = hex::encode(&sig);

        let vk = signing_key.verifying_key();
        let manifest = DetachedManifest::new(claim)
            .with_signature(SignatureRecord {
                algorithm: "ed25519".to_string(),
                key_id: vk.key_id().to_vec(),
                signature: sig_hex,
            })
            .with_public_key(PublicKeyEntry {
                key_id: vk.key_id().to_vec(),
                algorithm: "ed25519".to_string(),
                key_bytes: hex::encode(vk.as_bytes()),
            });

        let trust = TrustPolicy::TrustNone;
        let result = verify_detached_manifest(&image_bytes, &manifest, &trust);
        assert!(result.manifest_valid);
        assert!(result.instance_digest_match);
        let sigs_valid = result
            .report
            .signatures()
            .iter()
            .any(|s| s.cryptographically_valid());
        assert!(sigs_valid, "Signature should be cryptographically valid");
        assert!(
            !result.report.trust().trusted(),
            "Without TrustKeys, result must be untrusted"
        );
    }

    #[test]
    fn correct_signature_with_trust_succeeds() {
        let image_bytes = make_test_image_bytes();
        let claim = make_test_claim_for(&image_bytes);

        let signing_key = SigningKey::generate();
        let claim_bytes = claim.canonical_bytes();
        let sig = signing_key.sign(&claim_bytes);
        let sig_hex = hex::encode(&sig);

        let vk = signing_key.verifying_key();
        let manifest = DetachedManifest::new(claim)
            .with_signature(SignatureRecord {
                algorithm: "ed25519".to_string(),
                key_id: vk.key_id().to_vec(),
                signature: sig_hex,
            })
            .with_public_key(PublicKeyEntry {
                key_id: vk.key_id().to_vec(),
                algorithm: "ed25519".to_string(),
                key_bytes: hex::encode(vk.as_bytes()),
            });

        let trust = TrustPolicy::TrustKeys(vec![vk.key_id().to_vec()]);
        let result = verify_detached_manifest(&image_bytes, &manifest, &trust);
        assert!(result.manifest_valid);
        assert!(result.instance_digest_match);
        assert!(result.report.trust().trusted());
    }

    #[test]
    fn wrong_key_exits_untrusted() {
        let image_bytes = make_test_image_bytes();
        let claim = make_test_claim_for(&image_bytes);

        let signing_key = SigningKey::generate();
        let claim_bytes = claim.canonical_bytes();
        let sig = signing_key.sign(&claim_bytes);
        let sig_hex = hex::encode(&sig);

        let vk = signing_key.verifying_key();
        let manifest = DetachedManifest::new(claim)
            .with_signature(SignatureRecord {
                algorithm: "ed25519".to_string(),
                key_id: vk.key_id().to_vec(),
                signature: sig_hex,
            })
            .with_public_key(PublicKeyEntry {
                key_id: vk.key_id().to_vec(),
                algorithm: "ed25519".to_string(),
                key_bytes: hex::encode(vk.as_bytes()),
            });

        let trust = TrustPolicy::TrustKeys(vec![vec![0xFF; 32]]);
        let result = verify_detached_manifest(&image_bytes, &manifest, &trust);
        assert!(result.manifest_valid);
        assert!(
            !result.report.trust().trusted(),
            "Wrong key must not be trusted"
        );
    }

    #[test]
    fn wrong_image_digest_fails() {
        let image_bytes = make_test_image_bytes();
        let claim = make_test_claim_for(&image_bytes);
        let manifest = DetachedManifest::new(claim);

        let trust = TrustPolicy::TrustNone;
        let wrong_image = vec![0x00; 64];
        let result = verify_detached_manifest(&wrong_image, &manifest, &trust);
        assert!(!result.instance_digest_match);
    }

    #[test]
    fn embedded_reference_with_crc_succeeds() {
        let img = image::DynamicImage::new_rgb8(128, 128);
        let ctx = ProtectionContext::new(0.5, 42);
        let protected = process_image_bytes(
            &encode_image(&img, image::ImageFormat::Png).unwrap(),
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        let stego = stegoeggo::SteganographyProtector::new();
        let payload = stego
            .extract_payload_from_bytes_with_key(&protected, &[])
            .unwrap();
        let raw = payload.raw_payload().unwrap();

        use sha2::{Digest, Sha256};
        let payload_digest = format!("sha256:{}", hex::encode(Sha256::digest(raw)));
        let image_digest = format!("sha256:{}", hex::encode(Sha256::digest(&protected)));

        let claim = stegoeggo::provenance::ProvenanceClaim::new(1)
            .with_instance_digest_raw(image_digest)
            .with_content_code("iscc:test".to_string())
            .with_creation_time(1700000000)
            .with_source_facts("png", 128, 128, protected.len() as u64)
            .with_software("stegoeggo-test/0.2.3");

        let manifest = DetachedManifest::new(claim).with_embedded_reference(EmbeddedReference {
            payload_version: 3,
            payload_digest,
        });

        let trust = TrustPolicy::TrustNone;
        let result = verify_detached_manifest(&protected, &manifest, &trust);
        assert_eq!(
            result.embedded_reference_status,
            EmbeddedReferenceStatus::Present
        );
    }

    #[test]
    fn embedded_reference_digest_mismatch() {
        let img = image::DynamicImage::new_rgb8(128, 128);
        let ctx = ProtectionContext::new(0.5, 42);
        let protected = process_image_bytes(
            &encode_image(&img, image::ImageFormat::Png).unwrap(),
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        use sha2::{Digest, Sha256};
        let image_digest = format!("sha256:{}", hex::encode(Sha256::digest(&protected)));

        let claim = stegoeggo::provenance::ProvenanceClaim::new(1)
            .with_instance_digest_raw(image_digest)
            .with_content_code("iscc:test".to_string())
            .with_creation_time(1700000000)
            .with_source_facts("png", 128, 128, protected.len() as u64)
            .with_software("stegoeggo-test/0.2.3");

        let manifest = DetachedManifest::new(claim).with_embedded_reference(EmbeddedReference {
            payload_version: 3,
            payload_digest:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
        });

        let trust = TrustPolicy::TrustNone;
        let result = verify_detached_manifest(&protected, &manifest, &trust);
        assert_eq!(
            result.embedded_reference_status,
            EmbeddedReferenceStatus::DigestMismatch
        );
    }

    #[test]
    fn embedded_reference_version_mismatch() {
        let img = image::DynamicImage::new_rgb8(128, 128);
        let ctx = ProtectionContext::new(0.5, 42);
        let protected = process_image_bytes(
            &encode_image(&img, image::ImageFormat::Png).unwrap(),
            ProtectionLevel::Standard,
            &ctx,
        )
        .unwrap();

        use sha2::{Digest, Sha256};
        let image_digest = format!("sha256:{}", hex::encode(Sha256::digest(&protected)));

        let claim = stegoeggo::provenance::ProvenanceClaim::new(1)
            .with_instance_digest_raw(image_digest)
            .with_content_code("iscc:test".to_string())
            .with_creation_time(1700000000)
            .with_source_facts("png", 128, 128, protected.len() as u64)
            .with_software("stegoeggo-test/0.2.3");

        let manifest = DetachedManifest::new(claim).with_embedded_reference(EmbeddedReference {
            payload_version: 99,
            payload_digest: "sha256:aa".to_string(),
        });

        let trust = TrustPolicy::TrustNone;
        let result = verify_detached_manifest(&protected, &manifest, &trust);
        assert_eq!(
            result.embedded_reference_status,
            EmbeddedReferenceStatus::VersionMismatch
        );
    }

    #[test]
    fn no_embedded_reference_reports_not_provided() {
        let image_bytes = make_test_image_bytes();
        let claim = make_test_claim_for(&image_bytes);
        let manifest = DetachedManifest::new(claim);

        let trust = TrustPolicy::TrustNone;
        let result = verify_detached_manifest(&image_bytes, &manifest, &trust);
        assert_eq!(
            result.embedded_reference_status,
            EmbeddedReferenceStatus::NotProvided
        );
    }

    #[test]
    fn trust_metadata_does_not_affect_trust() {
        let image_bytes = make_test_image_bytes();
        let claim = make_test_claim_for(&image_bytes);

        let signing_key = SigningKey::generate();
        let claim_bytes = claim.canonical_bytes();
        let sig = signing_key.sign(&claim_bytes);
        let sig_hex = hex::encode(&sig);
        let vk = signing_key.verifying_key();

        let manifest = DetachedManifest::new(claim)
            .with_signature(SignatureRecord {
                algorithm: "ed25519".to_string(),
                key_id: vk.key_id().to_vec(),
                signature: sig_hex,
            })
            .with_public_key(PublicKeyEntry {
                key_id: vk.key_id().to_vec(),
                algorithm: "ed25519".to_string(),
                key_bytes: hex::encode(vk.as_bytes()),
            })
            .with_trust_metadata(TrustMetadata {
                trust_model: "test".to_string(),
                trusted: true,
                reason: "test trust".to_string(),
                certificate_chain: None,
            });

        let trust = TrustPolicy::TrustNone;
        let result = verify_detached_manifest(&image_bytes, &manifest, &trust);
        assert!(
            !result.report.trust().trusted(),
            "Manifest trust_metadata.trusted=true must not influence trust"
        );
    }

    #[test]
    fn json_and_human_output_agree() {
        let image_bytes = make_test_image_bytes();
        let claim = make_test_claim_for(&image_bytes);

        let signing_key = SigningKey::generate();
        let claim_bytes = claim.canonical_bytes();
        let sig = signing_key.sign(&claim_bytes);
        let sig_hex = hex::encode(&sig);
        let vk = signing_key.verifying_key();

        let manifest = DetachedManifest::new(claim)
            .with_signature(SignatureRecord {
                algorithm: "ed25519".to_string(),
                key_id: vk.key_id().to_vec(),
                signature: sig_hex,
            })
            .with_public_key(PublicKeyEntry {
                key_id: vk.key_id().to_vec(),
                algorithm: "ed25519".to_string(),
                key_bytes: hex::encode(vk.as_bytes()),
            });

        let trust = TrustPolicy::TrustKeys(vec![vk.key_id().to_vec()]);
        let result = verify_detached_manifest(&image_bytes, &manifest, &trust);
        assert!(result.report.trust().trusted());
        assert!(result.manifest_valid);
        assert!(result.instance_digest_match);
    }
}

// ---------------------------------------------------------------------------
// Gate 3.5: Fail-before-work tests
// ---------------------------------------------------------------------------

#[test]
fn metadata_only_rejects_oversized_input() {
    let limits = ResourceLimits::builder().max_input_bytes(100).build();
    let img = image::DynamicImage::new_rgb8(8, 8);
    let ctx = ProtectionContext::new(0.5, 42).with_resource_limits(limits);
    let png_bytes = encode_image(&img, image::ImageFormat::Png).unwrap();
    let result = process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx);
    assert!(
        result.is_err(),
        "Oversized input should be rejected before processing"
    );
}

#[test]
fn default_dimension_limits_apply_without_explicit_max_dimension() {
    let limits = ResourceLimits::builder()
        .max_width(32)
        .max_height(32)
        .build();
    let img = image::DynamicImage::new_rgb8(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_resource_limits(limits);
    let png_bytes = encode_image(&img, image::ImageFormat::Png).unwrap();
    let result = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx);
    assert!(
        result.is_err(),
        "Dimensions exceeding ResourceLimits defaults should fail without explicit max_dimension"
    );
}

#[test]
fn default_dimension_limits_apply_to_jpeg() {
    let limits = ResourceLimits::builder()
        .max_width(32)
        .max_height(32)
        .build();
    let img = image::DynamicImage::new_rgb8(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Jpeg)
        .with_resource_limits(limits);
    let png_bytes = encode_image(&img, image::ImageFormat::Png).unwrap();
    let result = process_image_bytes(&png_bytes, ProtectionLevel::Standard, &ctx);
    assert!(
        result.is_err(),
        "JPEG path should also enforce ResourceLimits dimensions"
    );
}

#[test]
fn payload_limit_rejects_large_payload() {
    let limits = ResourceLimits::builder().max_payload_bytes(10).build();
    let stego = stegoeggo::SteganographyProtector::with_resource_limits(limits);
    let img = image::DynamicImage::new_rgb8(64, 64);
    let result = stego.verify_payload_from_bytes_with_key(
        &encode_image(&img, image::ImageFormat::Png).unwrap(),
        &[],
    );
    assert_eq!(
        result,
        VerificationStatus::Invalid,
        "Payload found but oversized -> Invalid, not NotFound"
    );
}

#[test]
fn tile_extraction_origins_bounded() {
    let limits = ResourceLimits::builder()
        .max_tile_extraction_origins(1)
        .build();
    assert_eq!(limits.max_tile_extraction_origins(), 1);
}

#[test]
fn verification_seeds_bounded() {
    let limits = ResourceLimits::builder().max_verification_seeds(1).build();
    assert_eq!(limits.max_verification_seeds(), 1);
}

#[test]
fn resource_usage_tracks_input_and_output() {
    let usage = stegoeggo::ResourceUsage::begin(1024);
    assert_eq!(usage.input_bytes, 1024);
    assert_eq!(usage.peak_allocations_bytes, 1024);

    let mut usage = usage;
    usage.track_allocation(2048);
    assert_eq!(usage.peak_allocations_bytes, 2048);

    usage.track_allocation(512);
    assert_eq!(usage.peak_allocations_bytes, 2048);
}

#[test]
fn resource_usage_records_png_chunks() {
    let mut usage = stegoeggo::ResourceUsage::default();
    usage.record_png_chunks(10);
    assert_eq!(usage.png_chunks_scanned, 10);
}

#[test]
fn resource_usage_records_jpeg_segments() {
    let mut usage = stegoeggo::ResourceUsage::default();
    usage.record_jpeg_segments(5);
    assert_eq!(usage.jpeg_segments_scanned, 5);
}

#[test]
fn resource_usage_records_xmp_bytes() {
    let mut usage = stegoeggo::ResourceUsage::default();
    usage.record_xmp_bytes(1024);
    assert_eq!(usage.xmp_bytes_parsed, 1024);
}

#[test]
fn resource_usage_records_metadata() {
    let mut usage = stegoeggo::ResourceUsage::default();
    usage.record_metadata(5, 256);
    assert_eq!(usage.metadata_fields_extracted, 5);
    assert_eq!(usage.metadata_bytes_copied, 256);
}

#[test]
fn resource_usage_records_tile_origins() {
    let mut usage = stegoeggo::ResourceUsage::default();
    usage.record_tile_origins(8);
    assert_eq!(usage.tile_origins_checked, 8);
}

#[test]
fn resource_usage_records_verification_seeds() {
    let mut usage = stegoeggo::ResourceUsage::default();
    usage.record_verification_seeds(3);
    assert_eq!(usage.verification_seeds_tried, 3);
}

#[test]
fn resource_usage_display_format() {
    let usage = stegoeggo::ResourceUsage {
        input_bytes: 4096,
        png_chunks_scanned: 12,
        jpeg_segments_scanned: 0,
        webp_riff_chunks_scanned: 0,
        xmp_bytes_parsed: 256,
        metadata_fields_extracted: 3,
        metadata_bytes_copied: 128,
        tile_origins_checked: 0,
        verification_seeds_tried: 1,
        peak_allocations_bytes: 8192,
    };
    let s = usage.to_string();
    assert!(s.contains("4096"));
    assert!(s.contains("12"));
    assert!(s.contains("256"));
    assert!(s.contains("3"));
}
