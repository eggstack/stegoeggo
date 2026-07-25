#![cfg(feature = "detached-manifest")]

use stegoeggo::detached::{
    verify_detached_manifest, verify_detached_manifest_with_keys,
    verify_detached_manifest_with_limits, DetachedManifest, EmbeddedReference,
    EmbeddedReferenceStatus, PublicKeyEntry, SignatureRecord, TrustPolicy, MAX_KEY_ID_LEN,
    MAX_MANIFEST_SIZE, MAX_PUBLIC_KEYS, MAX_SIGNATURES,
};
use stegoeggo::provenance::ProvenanceClaim;
#[cfg(feature = "signatures")]
use stegoeggo::signing::SigningKey;

fn make_test_claim() -> ProvenanceClaim {
    ProvenanceClaim::new(1)
        .with_content_code("iscc:test123".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 1920, 1080, 524288)
        .with_software("stegoeggo/0.2.2")
}

#[test]
fn test_manifest_new() {
    let claim = make_test_claim();
    let manifest = DetachedManifest::new(claim.clone());

    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.claim.content_code, "iscc:test123");
    assert!(manifest.signatures.is_empty());
    assert!(manifest.public_keys.is_empty());
    assert!(manifest.embedded_reference.is_none());
}

#[test]
fn test_manifest_with_signature() {
    let sig = SignatureRecord {
        algorithm: "ed25519".to_string(),
        key_id: vec![1, 2, 3],
        signature: "abc123def456".to_string(),
    };

    let manifest = DetachedManifest::new(make_test_claim()).with_signature(sig);

    assert_eq!(manifest.signatures.len(), 1);
    assert_eq!(manifest.signatures[0].algorithm, "ed25519");
    assert_eq!(manifest.signatures[0].key_id, vec![1, 2, 3]);
}

#[test]
fn test_manifest_with_public_key() {
    let key = PublicKeyEntry {
        key_id: vec![4, 5, 6],
        algorithm: "ed25519".to_string(),
        key_bytes: "hex_encoded_key".to_string(),
    };

    let manifest = DetachedManifest::new(make_test_claim()).with_public_key(key);

    assert_eq!(manifest.public_keys.len(), 1);
    assert_eq!(manifest.public_keys[0].algorithm, "ed25519");
    assert_eq!(manifest.public_keys[0].key_id, vec![4, 5, 6]);
}

#[test]
fn test_manifest_canonical_bytes() {
    let manifest = DetachedManifest::new(make_test_claim());

    let bytes1 = manifest.canonical_bytes();
    let bytes2 = manifest.canonical_bytes();

    assert_eq!(bytes1, bytes2);
    assert!(!bytes1.is_empty());

    let json_str = String::from_utf8(bytes1).unwrap();
    assert!(json_str.contains("schema_version"));
    assert!(json_str.contains("claim"));
}

#[test]
fn test_manifest_digest() {
    let manifest = DetachedManifest::new(make_test_claim());

    let digest1 = manifest.digest();
    let digest2 = manifest.digest();

    assert_eq!(digest1, digest2);
    assert_eq!(digest1.len(), 32);
}

#[test]
fn test_manifest_from_json_roundtrip() {
    let manifest = DetachedManifest::new(make_test_claim())
        .with_signature(SignatureRecord {
            algorithm: "ed25519".to_string(),
            key_id: vec![10],
            signature: "a".repeat(128),
        })
        .with_public_key(PublicKeyEntry {
            key_id: vec![10],
            algorithm: "ed25519".to_string(),
            key_bytes: "b".repeat(64),
        })
        .with_embedded_reference(EmbeddedReference {
            payload_digest: "sha256:abcdef".to_string(),
            payload_version: 3,
        });

    let json_bytes = manifest.canonical_bytes();
    let parsed = DetachedManifest::from_json(&json_bytes).unwrap();

    assert_eq!(parsed.schema_version, manifest.schema_version);
    assert_eq!(parsed.claim.content_code, manifest.claim.content_code);
    assert_eq!(parsed.signatures.len(), 1);
    assert_eq!(parsed.signatures[0].algorithm, "ed25519");
    assert_eq!(parsed.public_keys.len(), 1);
    assert!(parsed.embedded_reference.is_some());
    assert_eq!(parsed.embedded_reference.unwrap().payload_version, 3);
}

#[test]
fn test_manifest_size_limits() {
    let mut manifest = DetachedManifest::new(make_test_claim());

    for i in 0..MAX_SIGNATURES + 5 {
        manifest = manifest.with_signature(SignatureRecord {
            algorithm: "ed25519".to_string(),
            key_id: vec![i as u8],
            signature: format!("sig_{}", i),
        });
    }
    assert_eq!(manifest.signatures.len(), MAX_SIGNATURES);

    let mut manifest = DetachedManifest::new(make_test_claim());
    for i in 0..MAX_PUBLIC_KEYS + 5 {
        manifest = manifest.with_public_key(PublicKeyEntry {
            key_id: vec![i as u8],
            algorithm: "ed25519".to_string(),
            key_bytes: format!("key_{}", i),
        });
    }
    assert_eq!(manifest.public_keys.len(), MAX_PUBLIC_KEYS);
}

#[test]
fn test_manifest_digest_differs_for_different_claims() {
    let claim1 = ProvenanceClaim::new(0).with_content_code("iscc:aaa".to_string());
    let claim2 = ProvenanceClaim::new(0).with_content_code("iscc:bbb".to_string());

    let manifest1 = DetachedManifest::new(claim1);
    let manifest2 = DetachedManifest::new(claim2);

    assert_ne!(manifest1.digest(), manifest2.digest());
}

#[test]
fn test_manifest_with_embedded_reference() {
    let reference = EmbeddedReference {
        payload_digest: "sha256:1234567890abcdef".to_string(),
        payload_version: 2,
    };

    let manifest = DetachedManifest::new(make_test_claim()).with_embedded_reference(reference);

    assert!(manifest.embedded_reference.is_some());
    let ref_inner = manifest.embedded_reference.unwrap();
    assert_eq!(ref_inner.payload_version, 2);
    assert_eq!(ref_inner.payload_digest, "sha256:1234567890abcdef");
}

#[test]
fn test_manifest_json_is_valid_json() {
    let manifest = DetachedManifest::new(make_test_claim());
    let bytes = manifest.canonical_bytes();

    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(parsed.is_object());
    assert!(parsed.get("schema_version").is_some());
    assert!(parsed.get("claim").is_some());
    assert!(parsed.get("signatures").is_some());
    assert!(parsed.get("public_keys").is_some());
}

#[test]
fn test_manifest_max_manifest_size_constant() {
    assert_eq!(MAX_MANIFEST_SIZE, 64 * 1024);
}

#[cfg(feature = "signatures")]
fn make_signed_manifest() -> (DetachedManifest, Vec<u8>, SigningKey) {
    let sk = SigningKey::from_bytes([42u8; 32], b"test-key-id".to_vec()).unwrap();
    let vk = sk.verifying_key();
    let image_bytes = b"fake image content for testing";

    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:test-sign".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 100, 100, 10000)
        .with_software("stegoeggo/0.2.2")
        .with_instance_digest(image_bytes);

    let claim_bytes = claim.canonical_bytes();
    let sig_bytes = sk.sign(&claim_bytes);
    let sig_hex = hex::encode(&sig_bytes);

    let manifest = DetachedManifest::new(claim)
        .with_signature(SignatureRecord {
            algorithm: "ed25519".to_string(),
            key_id: b"test-key-id".to_vec(),
            signature: sig_hex,
        })
        .with_public_key(PublicKeyEntry {
            key_id: b"test-key-id".to_vec(),
            algorithm: "ed25519".to_string(),
            key_bytes: hex::encode(vk.as_bytes()),
        });

    (manifest, image_bytes.to_vec(), sk)
}

#[cfg(feature = "signatures")]
#[test]
fn test_trust_none_untrusted_even_with_valid_sig() {
    let (manifest, image_bytes, _) = make_signed_manifest();
    let result = verify_detached_manifest(&image_bytes, &manifest, &TrustPolicy::TrustNone);

    assert!(result.report.signatures().len() == 1);
    assert!(result.report.signatures()[0].cryptographically_valid());
    assert!(!result.report.signatures()[0].trusted());
}

#[cfg(feature = "signatures")]
#[test]
fn test_trust_keys_trusted_when_key_matches() {
    let (manifest, image_bytes, _) = make_signed_manifest();
    let trusted_keys = vec![b"test-key-id".to_vec()];
    let result = verify_detached_manifest(
        &image_bytes,
        &manifest,
        &TrustPolicy::TrustKeys(trusted_keys),
    );

    assert!(result.report.signatures()[0].cryptographically_valid());
    assert!(result.report.signatures()[0].key_id_matched());
    assert!(result.report.signatures()[0].trusted());
}

#[cfg(feature = "signatures")]
#[test]
fn test_trust_keys_untrusted_when_key_does_not_match() {
    let (manifest, image_bytes, _) = make_signed_manifest();
    let trusted_keys = vec![b"wrong-key-id".to_vec()];
    let result = verify_detached_manifest(
        &image_bytes,
        &manifest,
        &TrustPolicy::TrustKeys(trusted_keys),
    );

    assert!(result.report.signatures()[0].cryptographically_valid());
    assert!(!result.report.signatures()[0].key_id_matched());
    assert!(!result.report.signatures()[0].trusted());
}

#[cfg(feature = "signatures")]
#[test]
fn test_trust_callback_trusted_when_cb_returns_true() {
    let (manifest, image_bytes, _) = make_signed_manifest();
    let result = verify_detached_manifest(
        &image_bytes,
        &manifest,
        &TrustPolicy::TrustCallback(Box::new(|_key_id| true)),
    );

    assert!(result.report.signatures()[0].cryptographically_valid());
    assert!(result.report.signatures()[0].trusted());
}

#[cfg(feature = "signatures")]
#[test]
fn test_trust_callback_untrusted_when_cb_returns_false() {
    let (manifest, image_bytes, _) = make_signed_manifest();
    let result = verify_detached_manifest(
        &image_bytes,
        &manifest,
        &TrustPolicy::TrustCallback(Box::new(|_key_id| false)),
    );

    assert!(result.report.signatures()[0].cryptographically_valid());
    assert!(!result.report.signatures()[0].trusted());
}

#[cfg(feature = "signatures")]
#[test]
fn test_trust_callback_custom_logic() {
    let (manifest, image_bytes, _) = make_signed_manifest();
    let result = verify_detached_manifest(
        &image_bytes,
        &manifest,
        &TrustPolicy::TrustCallback(Box::new(|key_id| key_id.starts_with(b"test"))),
    );

    assert!(result.report.signatures()[0].trusted());

    let result2 = verify_detached_manifest(
        &image_bytes,
        &manifest,
        &TrustPolicy::TrustCallback(Box::new(|key_id| key_id.starts_with(b"other"))),
    );

    assert!(!result2.report.signatures()[0].trusted());
}

#[cfg(feature = "signatures")]
#[test]
fn test_backward_compat_wrapper_trust_none() {
    let (manifest, image_bytes, _) = make_signed_manifest();
    let result = verify_detached_manifest_with_keys(&image_bytes, &manifest, None);

    assert!(result.report.signatures()[0].cryptographically_valid());
    assert!(!result.report.signatures()[0].trusted());
}

#[cfg(feature = "signatures")]
#[test]
fn test_backward_compat_wrapper_trust_keys() {
    let (manifest, image_bytes, _) = make_signed_manifest();
    let keys = vec![b"test-key-id".to_vec()];
    let result = verify_detached_manifest_with_keys(&image_bytes, &manifest, Some(&keys));

    assert!(result.report.signatures()[0].trusted());
}

#[cfg(feature = "signatures")]
#[test]
fn test_instance_digest_match() {
    let (manifest, image_bytes, _) = make_signed_manifest();
    let result = verify_detached_manifest(&image_bytes, &manifest, &TrustPolicy::TrustNone);

    assert!(result.instance_digest_match);
}

#[cfg(feature = "signatures")]
#[test]
fn test_instance_digest_mismatch() {
    let (manifest, _, _) = make_signed_manifest();
    let different_image = b"completely different image content";
    let result = verify_detached_manifest(different_image, &manifest, &TrustPolicy::TrustNone);

    assert!(!result.instance_digest_match);
}

#[cfg(feature = "signatures")]
#[test]
fn test_embedded_reference_not_provided() {
    let (manifest, image_bytes, _) = make_signed_manifest();
    assert!(manifest.embedded_reference.is_none());

    let result = verify_detached_manifest(&image_bytes, &manifest, &TrustPolicy::TrustNone);
    assert_eq!(
        result.embedded_reference_status,
        EmbeddedReferenceStatus::NotProvided
    );
}

#[cfg(feature = "signatures")]
#[test]
fn test_unknown_algorithm_rejected() {
    let sk = SigningKey::from_bytes([42u8; 32], b"test-key-id".to_vec()).unwrap();
    let vk = sk.verifying_key();
    let image_bytes = b"fake image content for testing";

    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:test-algo".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 100, 100, 10000)
        .with_software("stegoeggo/0.2.2")
        .with_instance_digest(image_bytes);

    let claim_bytes = claim.canonical_bytes();
    let sig_bytes = sk.sign(&claim_bytes);

    let manifest = DetachedManifest::new(claim)
        .with_signature(SignatureRecord {
            algorithm: "rsa-sha256".to_string(),
            key_id: b"test-key-id".to_vec(),
            signature: hex::encode(&sig_bytes),
        })
        .with_public_key(PublicKeyEntry {
            key_id: b"test-key-id".to_vec(),
            algorithm: "ed25519".to_string(),
            key_bytes: hex::encode(vk.as_bytes()),
        });

    let result = verify_detached_manifest(image_bytes, &manifest, &TrustPolicy::TrustNone);
    assert_eq!(result.report.signatures().len(), 1);
    assert!(!result.report.signatures()[0].structurally_valid());
}

#[cfg(feature = "signatures")]
#[test]
fn test_encoding_mismatch_rejected() {
    let sk = SigningKey::from_bytes([42u8; 32], b"test-key-id".to_vec()).unwrap();
    let image_bytes = b"fake image content for testing";

    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:test-encoding".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 100, 100, 10000)
        .with_software("stegoeggo/0.2.2")
        .with_instance_digest(image_bytes);

    let claim_bytes = claim.canonical_bytes();
    let sig_bytes = sk.sign(&claim_bytes);

    let manifest = DetachedManifest::new(claim)
        .with_signature(SignatureRecord {
            algorithm: "ed25519".to_string(),
            key_id: b"test-key-id".to_vec(),
            signature: hex::encode(sig_bytes),
        })
        .with_public_key(PublicKeyEntry {
            key_id: b"test-key-id".to_vec(),
            algorithm: "ed25519".to_string(),
            key_bytes: "++invalid-hex++".to_string(),
        });

    let result = verify_detached_manifest(image_bytes, &manifest, &TrustPolicy::TrustNone);
    assert_eq!(result.report.signatures().len(), 1);
    assert!(!result.report.signatures()[0].structurally_valid());
}

#[test]
fn test_wrong_key_content_fails() {
    let image_bytes = b"fake image content for testing";

    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:test-wrong-key".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 100, 100, 10000)
        .with_software("stegoeggo/0.2.2")
        .with_instance_digest(image_bytes);

    let sig_bytes = [0u8; 64];

    let manifest = DetachedManifest::new(claim)
        .with_signature(SignatureRecord {
            algorithm: "ed25519".to_string(),
            key_id: b"test-key-id".to_vec(),
            signature: hex::encode(sig_bytes),
        })
        .with_public_key(PublicKeyEntry {
            key_id: b"test-key-id".to_vec(),
            algorithm: "ed25519".to_string(),
            key_bytes: hex::encode([0xFFu8; 32]),
        });

    let result = verify_detached_manifest(image_bytes, &manifest, &TrustPolicy::TrustNone);
    assert_eq!(result.report.signatures().len(), 1);
    assert!(result.report.signatures()[0].structurally_valid());
    assert!(!result.report.signatures()[0].cryptographically_valid());
}

#[test]
fn test_embedded_reference_raw_payload_none() {
    use image::ImageEncoder;

    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut image_bytes = Vec::new();
    {
        let encoder = image::codecs::png::PngEncoder::new(&mut image_bytes);
        let rgb = img.to_rgb8();
        encoder
            .write_image(&rgb, 64, 64, image::ExtendedColorType::Rgb8)
            .unwrap();
    }

    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:raw-payload-test".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 64, 64, image_bytes.len() as u64)
        .with_software("stegoeggo/0.2.2")
        .with_instance_digest(&image_bytes);

    let manifest = DetachedManifest::new(claim).with_embedded_reference(EmbeddedReference {
        payload_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        payload_version: 3,
    });

    let result = verify_detached_manifest(&image_bytes, &manifest, &TrustPolicy::TrustNone);
    assert_eq!(
        result.embedded_reference_status,
        EmbeddedReferenceStatus::Stripped
    );
}

#[test]
fn test_resource_limits_rejects_oversized_input() {
    use stegoeggo::ResourceLimits;

    let limits = ResourceLimits::builder().max_input_bytes(100).build();
    let image_bytes = vec![0u8; 200];

    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:limits-test".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 100, 100, 10000)
        .with_software("stegoeggo/0.2.2");

    let manifest = DetachedManifest::new(claim);
    let result = verify_detached_manifest_with_limits(
        &image_bytes,
        &manifest,
        &TrustPolicy::TrustNone,
        Some(&limits),
    );

    assert!(!result.manifest_valid);
    assert!(!result.instance_digest_match);
}

#[cfg(feature = "signatures")]
#[test]
fn test_embedded_reference_stripped_when_no_payload() {
    let (mut manifest, image_bytes, _) = make_signed_manifest();
    manifest = manifest.with_embedded_reference(EmbeddedReference {
        payload_digest: "sha256:abcdef".to_string(),
        payload_version: 3,
    });

    let result = verify_detached_manifest(&image_bytes, &manifest, &TrustPolicy::TrustNone);
    assert_eq!(
        result.embedded_reference_status,
        EmbeddedReferenceStatus::Stripped
    );
}

#[test]
fn test_embedded_reference_stripped_for_invalid_image_bytes() {
    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:test-stripped".to_string())
        .with_instance_digest(b"not-a-real-image")
        .with_source_facts("png", 100, 100, 10000)
        .with_software("stegoeggo/0.2.2");

    let manifest = DetachedManifest::new(claim).with_embedded_reference(EmbeddedReference {
        payload_digest: "sha256:test".to_string(),
        payload_version: 3,
    });

    let result = verify_detached_manifest(b"not-a-real-image", &manifest, &TrustPolicy::TrustNone);
    assert_eq!(
        result.embedded_reference_status,
        EmbeddedReferenceStatus::Stripped
    );
}

#[test]
fn test_oversized_manifest_fails_parsing() {
    let oversized = vec![0u8; MAX_MANIFEST_SIZE + 1];
    let result = DetachedManifest::from_json(&oversized);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("exceeds maximum"),
        "Error should mention size limit: {}",
        err_msg
    );
}

#[test]
fn test_unknown_schema_version_fails_parsing() {
    let manifest = DetachedManifest::new(make_test_claim());
    let mut json_bytes = manifest.canonical_bytes();
    let mut parsed: serde_json::Value = serde_json::from_slice(&json_bytes).unwrap();
    parsed["schema_version"] = serde_json::Value::from(99);
    json_bytes = serde_json::to_vec(&parsed).unwrap();

    let result = DetachedManifest::from_json(&json_bytes);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("schema version"),
        "Error should mention schema version: {}",
        err_msg
    );
}

#[test]
fn test_schema_version_zero_fails_parsing() {
    let manifest = DetachedManifest::new(make_test_claim());
    let mut json_bytes = manifest.canonical_bytes();
    let mut parsed: serde_json::Value = serde_json::from_slice(&json_bytes).unwrap();
    parsed["schema_version"] = serde_json::Value::from(0);
    json_bytes = serde_json::to_vec(&parsed).unwrap();

    let result = DetachedManifest::from_json(&json_bytes);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("schema version"),
        "Error should mention schema version: {}",
        err_msg
    );
}

#[cfg(feature = "signatures")]
#[test]
fn test_duplicate_key_ids_in_signatures_are_handled() {
    let sk = SigningKey::from_bytes([42u8; 32], b"dup-key".to_vec()).unwrap();
    let vk = sk.verifying_key();
    let image_bytes = b"test image for dup keys";

    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:dup-test".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 100, 100, 10000)
        .with_software("stegoeggo/0.2.2")
        .with_instance_digest(image_bytes);

    let claim_bytes = claim.canonical_bytes();
    let sig_bytes = sk.sign(&claim_bytes);
    let sig_hex = hex::encode(&sig_bytes);

    let manifest = DetachedManifest::new(claim)
        .with_signature(SignatureRecord {
            algorithm: "ed25519".to_string(),
            key_id: b"dup-key".to_vec(),
            signature: sig_hex.clone(),
        })
        .with_signature(SignatureRecord {
            algorithm: "ed25519".to_string(),
            key_id: b"dup-key".to_vec(),
            signature: sig_hex,
        })
        .with_public_key(PublicKeyEntry {
            key_id: b"dup-key".to_vec(),
            algorithm: "ed25519".to_string(),
            key_bytes: hex::encode(vk.as_bytes()),
        });

    assert_eq!(manifest.signatures.len(), 1);
    let result = verify_detached_manifest(image_bytes, &manifest, &TrustPolicy::TrustNone);
    assert_eq!(result.report.signatures().len(), 1);
    assert!(result.report.signatures()[0].cryptographically_valid());
}

#[cfg(feature = "signatures")]
#[test]
fn test_wrong_image_digest_fails_verification() {
    let (manifest, _image_bytes, _) = make_signed_manifest();
    let different_image = b"this is not the original image content at all";

    let result = verify_detached_manifest(different_image, &manifest, &TrustPolicy::TrustNone);
    assert!(!result.instance_digest_match);
}

#[cfg(feature = "signatures")]
#[test]
fn test_untrusted_key_produces_invalid_trust_result() {
    let (manifest, image_bytes, _) = make_signed_manifest();

    let result = verify_detached_manifest(
        &image_bytes,
        &manifest,
        &TrustPolicy::TrustKeys(vec![b"unknown-key-id".to_vec()]),
    );

    assert!(result.report.signatures()[0].cryptographically_valid());
    assert!(!result.report.signatures()[0].key_id_matched());
    assert!(!result.report.signatures()[0].trusted());
}

#[cfg(feature = "signatures")]
#[test]
fn test_trust_metadata_is_informational_only() {
    let (mut manifest, image_bytes, _) = make_signed_manifest();
    manifest.trust_metadata = Some(stegoeggo::detached::TrustMetadata {
        trust_model: "pki".to_string(),
        trusted: true,
        reason: "manifest says trusted".to_string(),
        certificate_chain: None,
    });

    // TrustNone should NOT trust even though trust_metadata.trusted = true
    let result = verify_detached_manifest(&image_bytes, &manifest, &TrustPolicy::TrustNone);
    assert!(!result.report.signatures()[0].trusted());

    // TrustKeys with the correct key SHOULD trust, regardless of trust_metadata
    let result = verify_detached_manifest(
        &image_bytes,
        &manifest,
        &TrustPolicy::TrustKeys(vec![b"test-key-id".to_vec()]),
    );
    assert!(result.report.signatures()[0].trusted());
}

#[cfg(feature = "signatures")]
#[test]
fn test_embedded_reference_version_mismatch_reports_stripped() {
    let (mut manifest, image_bytes, _) = make_signed_manifest();
    manifest = manifest.with_embedded_reference(EmbeddedReference {
        payload_digest: "sha256:does_not_matter".to_string(),
        payload_version: 99,
    });

    let result = verify_detached_manifest(&image_bytes, &manifest, &TrustPolicy::TrustNone);
    assert_eq!(
        result.embedded_reference_status,
        EmbeddedReferenceStatus::Stripped
    );
}

#[test]
fn test_max_key_id_len_constant() {
    assert_eq!(MAX_KEY_ID_LEN, 64);
}

#[test]
fn test_invalid_json_fails_parsing() {
    let result = DetachedManifest::from_json(b"not valid json {{{");
    assert!(result.is_err());
}

#[cfg(feature = "signatures")]
#[test]
fn test_trust_metadata_not_present_reported() {
    let (manifest, image_bytes, _) = make_signed_manifest();
    let result = verify_detached_manifest(&image_bytes, &manifest, &TrustPolicy::TrustNone);
    // trust_metadata is None, so the report trust should have default values
    assert!(!result.report.trust().trusted());
}

#[test]
fn test_embedded_reference_wrong_digest_reports_stripped_without_payload() {
    use image::ImageEncoder;

    let img = image::DynamicImage::new_rgb8(64, 64);
    let mut image_bytes = Vec::new();
    {
        let encoder = image::codecs::png::PngEncoder::new(&mut image_bytes);
        let rgb = img.to_rgb8();
        encoder
            .write_image(&rgb, 64, 64, image::ExtendedColorType::Rgb8)
            .unwrap();
    }

    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:digest-test".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 64, 64, image_bytes.len() as u64)
        .with_software("stegoeggo/0.2.2")
        .with_instance_digest(&image_bytes);

    let manifest = DetachedManifest::new(claim).with_embedded_reference(EmbeddedReference {
        payload_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        payload_version: 3,
    });

    let result = verify_detached_manifest(&image_bytes, &manifest, &TrustPolicy::TrustNone);
    assert_eq!(
        result.embedded_reference_status,
        EmbeddedReferenceStatus::Stripped
    );
}

#[cfg(feature = "signatures")]
mod trust_verifying_key_tests {
    use super::*;
    use stegoeggo::detached::verify::verify_detached_manifest_with_limits_and_mac;
    use stegoeggo::detached::TrustedVerifyingKey;
    use stegoeggo::resource_limits::ResourceLimits;

    fn build_manifest(
        claim: &ProvenanceClaim,
        signing_key: &SigningKey,
        sig_key_id: &[u8],
        manifest_key_id: &[u8],
        pub_key_bytes: &[u8; 32],
    ) -> DetachedManifest {
        let claim_bytes = claim.canonical_bytes();
        let sig_bytes = signing_key.sign(&claim_bytes);
        let sig_hex = hex::encode(&sig_bytes);

        DetachedManifest::new(claim.clone())
            .with_signature(SignatureRecord {
                algorithm: "ed25519".to_string(),
                key_id: sig_key_id.to_vec(),
                signature: sig_hex,
            })
            .with_public_key(PublicKeyEntry {
                key_id: manifest_key_id.to_vec(),
                algorithm: "ed25519".to_string(),
                key_bytes: hex::encode(pub_key_bytes),
            })
    }

    fn make_claim(image_bytes: &[u8]) -> ProvenanceClaim {
        ProvenanceClaim::new(1)
            .with_content_code("iscc:attack-test".to_string())
            .with_creation_time(1700000000)
            .with_source_facts("png", 64, 64, image_bytes.len() as u64)
            .with_software("stegoeggo/0.2.3")
            .with_instance_digest(image_bytes)
    }

    #[test]
    fn a_id_matching_a_bytes_a_signature_succeeds() {
        let image_bytes = b"attack test image content";
        let claim = make_claim(image_bytes);

        let key_a = SigningKey::from_bytes([0xAA; 32], b"key-a-id".to_vec()).unwrap();
        let vk_a = key_a.verifying_key();

        let manifest = build_manifest(&claim, &key_a, b"key-a-id", b"key-a-id", vk_a.as_bytes());

        let trusted = vec![TrustedVerifyingKey {
            key_id: b"key-a-id".to_vec(),
            key: vk_a.clone(),
        }];
        let result = verify_detached_manifest(
            image_bytes,
            &manifest,
            &TrustPolicy::TrustVerifyingKeys(trusted),
        );

        assert!(result.report.signatures()[0].cryptographically_valid());
        assert!(result.report.signatures()[0].key_id_matched());
        assert!(result.report.signatures()[0].key_material_matched());
        assert!(result.report.signatures()[0].trusted());
        assert_eq!(
            result.overall_status(),
            stegoeggo::detached::DetachedOverallStatus::VerifiedTrusted
        );
    }

    #[test]
    fn a_id_with_b_bytes_and_a_signature_fails_key_material() {
        let image_bytes = b"attack test image content";
        let claim = make_claim(image_bytes);

        let key_a = SigningKey::from_bytes([0xAA; 32], b"key-a-id".to_vec()).unwrap();
        let key_b = SigningKey::from_bytes([0xBB; 32], b"key-b-id".to_vec()).unwrap();
        let vk_a = key_a.verifying_key();

        // Manifest stores B's public bytes under A's key ID.
        let manifest = build_manifest(
            &claim,
            &key_a,
            b"key-a-id",
            b"key-a-id",
            key_b.verifying_key().as_bytes(),
        );

        let trusted = vec![TrustedVerifyingKey {
            key_id: b"key-a-id".to_vec(),
            key: vk_a.clone(),
        }];
        let result = verify_detached_manifest(
            image_bytes,
            &manifest,
            &TrustPolicy::TrustVerifyingKeys(trusted),
        );

        // Signature is cryptographically valid (signed with A's private key).
        assert!(result.report.signatures()[0].cryptographically_valid());
        assert!(result.report.signatures()[0].key_id_matched());
        // But key material does NOT match — manifest has B's bytes.
        assert!(!result.report.signatures()[0].key_material_matched());
        assert!(!result.report.signatures()[0].trusted());
        assert_eq!(
            result.overall_status(),
            stegoeggo::detached::DetachedOverallStatus::KeyMaterialMismatch
        );
    }

    #[test]
    fn a_id_with_a_bytes_and_b_signature_fails_crypto() {
        let image_bytes = b"attack test image content";
        let claim = make_claim(image_bytes);

        let key_a = SigningKey::from_bytes([0xAA; 32], b"key-a-id".to_vec()).unwrap();
        let key_b = SigningKey::from_bytes([0xBB; 32], b"key-b-id".to_vec()).unwrap();
        let vk_a = key_a.verifying_key();

        // Manifest stores A's public bytes under A's key ID, but signature
        // is made with B's private key.
        let manifest = build_manifest(&claim, &key_b, b"key-a-id", b"key-a-id", vk_a.as_bytes());

        let trusted = vec![TrustedVerifyingKey {
            key_id: b"key-a-id".to_vec(),
            key: vk_a.clone(),
        }];
        let result = verify_detached_manifest(
            image_bytes,
            &manifest,
            &TrustPolicy::TrustVerifyingKeys(trusted),
        );

        assert!(!result.report.signatures()[0].cryptographically_valid());
        assert!(result.report.signatures()[0].key_id_matched());
        assert!(result.report.signatures()[0].key_material_matched());
        assert!(!result.report.signatures()[0].trusted());
    }

    #[test]
    fn wrong_external_key_returns_untrusted() {
        let image_bytes = b"attack test image content";
        let claim = make_claim(image_bytes);

        let key_a = SigningKey::from_bytes([0xAA; 32], b"key-a-id".to_vec()).unwrap();
        let key_wrong = SigningKey::from_bytes([0xCC; 32], b"key-a-id".to_vec()).unwrap();
        let vk_a = key_a.verifying_key();

        let manifest = build_manifest(&claim, &key_a, b"key-a-id", b"key-a-id", vk_a.as_bytes());

        // Caller supplies wrong key under the same key ID.
        let trusted = vec![TrustedVerifyingKey {
            key_id: b"key-a-id".to_vec(),
            key: key_wrong.verifying_key(),
        }];
        let result = verify_detached_manifest(
            image_bytes,
            &manifest,
            &TrustPolicy::TrustVerifyingKeys(trusted),
        );

        assert!(!result.report.signatures()[0].trusted());
        assert!(!result.report.signatures()[0].key_material_matched());
    }

    #[test]
    fn trusted_key_without_manifest_entry_verifies_directly() {
        let image_bytes = b"attack test image content";
        let claim = make_claim(image_bytes);

        let key_a = SigningKey::from_bytes([0xAA; 32], b"key-a-id".to_vec()).unwrap();
        let vk_a = key_a.verifying_key();

        let claim_bytes = claim.canonical_bytes();
        let sig_bytes = key_a.sign(&claim_bytes);

        // Manifest has NO public key entry — caller-owned key is used directly.
        let manifest = DetachedManifest::new(claim.clone()).with_signature(SignatureRecord {
            algorithm: "ed25519".to_string(),
            key_id: b"key-a-id".to_vec(),
            signature: hex::encode(&sig_bytes),
        });

        let trusted = vec![TrustedVerifyingKey {
            key_id: b"key-a-id".to_vec(),
            key: vk_a.clone(),
        }];
        let result = verify_detached_manifest(
            image_bytes,
            &manifest,
            &TrustPolicy::TrustVerifyingKeys(trusted),
        );

        assert!(result.report.signatures()[0].cryptographically_valid());
        assert!(result.report.signatures()[0].key_id_matched());
        assert!(result.report.signatures()[0].key_material_matched());
        assert!(result.report.signatures()[0].trusted());
    }

    #[test]
    fn key_material_mismatch_reported_in_json_and_diagnostics() {
        let image_bytes = b"attack test image content";
        let claim = make_claim(image_bytes);

        let key_a = SigningKey::from_bytes([0xAA; 32], b"key-a-id".to_vec()).unwrap();
        let key_b = SigningKey::from_bytes([0xBB; 32], b"key-b-id".to_vec()).unwrap();
        let vk_a = key_a.verifying_key();

        let manifest = build_manifest(
            &claim,
            &key_a,
            b"key-a-id",
            b"key-a-id",
            key_b.verifying_key().as_bytes(),
        );

        let trusted = vec![TrustedVerifyingKey {
            key_id: b"key-a-id".to_vec(),
            key: vk_a.clone(),
        }];
        let result = verify_detached_manifest(
            image_bytes,
            &manifest,
            &TrustPolicy::TrustVerifyingKeys(trusted),
        );

        let sig = &result.report.signatures()[0];
        assert!(!sig.key_material_matched());
        assert!(!sig.trusted());

        // JSON serialization must include key_material_matched.
        let json = serde_json::to_string(&result.report).unwrap();
        assert!(json.contains("key_material_matched"));
        assert!(json.contains("false"));
    }

    #[test]
    fn key_id_only_trust_distinguished_from_caller_key_trust() {
        let image_bytes = b"attack test image content";
        let claim = make_claim(image_bytes);

        let key_a = SigningKey::from_bytes([0xAA; 32], b"key-a-id".to_vec()).unwrap();
        let key_b = SigningKey::from_bytes([0xBB; 32], b"key-b-id".to_vec()).unwrap();
        let vk_a = key_a.verifying_key();

        // Manifest stores A's public bytes under A's key ID (correct).
        let manifest = build_manifest(&claim, &key_a, b"key-a-id", b"key-a-id", vk_a.as_bytes());

        // Key-ID-only trust: trusted because key ID matches and signature
        // verifies with the manifest's key bytes.
        let key_id_result = verify_detached_manifest(
            image_bytes,
            &manifest,
            &TrustPolicy::TrustKeys(vec![b"key-a-id".to_vec()]),
        );
        assert!(key_id_result.report.signatures()[0].cryptographically_valid());
        assert!(key_id_result.report.signatures()[0].trusted());
        assert!(key_id_result.report.signatures()[0].key_material_matched());

        // Caller-key trust with B's key under A's ID: NOT trusted because
        // the caller-owned key bytes (B) don't match the manifest's bytes (A),
        // even though the key ID matches.
        let trusted = vec![TrustedVerifyingKey {
            key_id: b"key-a-id".to_vec(),
            key: key_b.verifying_key(),
        }];
        let caller_result = verify_detached_manifest(
            image_bytes,
            &manifest,
            &TrustPolicy::TrustVerifyingKeys(trusted),
        );
        assert!(!caller_result.report.signatures()[0].trusted());
        assert!(!caller_result.report.signatures()[0].key_material_matched());
    }

    #[test]
    fn duplicate_manifest_key_records_rejected_before_signature_eval() {
        let image_bytes = b"attack test image content";
        let claim = make_claim(image_bytes);

        let key_a = SigningKey::from_bytes([0xAA; 32], b"key-a-id".to_vec()).unwrap();
        let vk_a = key_a.verifying_key();

        let claim_bytes = claim.canonical_bytes();
        let sig_bytes = key_a.sign(&claim_bytes);

        let mut manifest = DetachedManifest::new(claim.clone())
            .with_signature(SignatureRecord {
                algorithm: "ed25519".to_string(),
                key_id: b"key-a-id".to_vec(),
                signature: hex::encode(&sig_bytes),
            })
            .with_public_key(PublicKeyEntry {
                key_id: b"key-a-id".to_vec(),
                algorithm: "ed25519".to_string(),
                key_bytes: hex::encode(vk_a.as_bytes()),
            });

        // Add a conflicting public key entry under the same key ID.
        manifest.public_keys.push(PublicKeyEntry {
            key_id: b"key-a-id".to_vec(),
            algorithm: "ed25519".to_string(),
            key_bytes: hex::encode(key_a.public_key_bytes()),
        });

        // Validation should reject duplicate key IDs.
        let validation = manifest.validate();
        assert!(validation.is_err(), "duplicate key IDs must be rejected");

        let trusted = vec![TrustedVerifyingKey {
            key_id: b"key-a-id".to_vec(),
            key: vk_a.clone(),
        }];
        let result = verify_detached_manifest(
            image_bytes,
            &manifest,
            &TrustPolicy::TrustVerifyingKeys(trusted),
        );

        // Manifest is structurally invalid due to duplicate key IDs.
        assert!(!result.manifest_valid);
        assert_eq!(
            result.overall_status(),
            stegoeggo::detached::DetachedOverallStatus::InvalidConfiguration
        );
    }

    #[test]
    fn trust_verifying_keys_debug_does_not_expose_key_bytes() {
        let key_a = SigningKey::from_bytes([0xAA; 32], b"key-a-id".to_vec()).unwrap();
        let vk_a = key_a.verifying_key();
        let trusted = TrustedVerifyingKey {
            key_id: b"key-a-id".to_vec(),
            key: vk_a.clone(),
        };
        let debug = format!("{:?}", trusted);
        assert!(debug.contains("TrustedVerifyingKey"));
        assert!(debug.contains("key_id"));
        let key_hex = hex::encode(vk_a.as_bytes());
        assert!(!debug.contains(&key_hex));
    }

    #[test]
    fn trust_policy_debug_shows_trust_verifying_keys() {
        let key_a = SigningKey::from_bytes([0xAA; 32], b"key-a-id".to_vec()).unwrap();
        let vk_a = key_a.verifying_key();
        let trusted = vec![TrustedVerifyingKey {
            key_id: b"key-a-id".to_vec(),
            key: vk_a.clone(),
        }];
        let policy = TrustPolicy::TrustVerifyingKeys(trusted);
        let debug = format!("{:?}", policy);
        assert!(debug.contains("TrustVerifyingKeys"));
    }

    #[test]
    fn resource_limits_enforced_before_verification() {
        let image_bytes = b"attack test image content";
        let claim = make_claim(image_bytes);

        let key_a = SigningKey::from_bytes([0xAA; 32], b"key-a-id".to_vec()).unwrap();
        let vk_a = key_a.verifying_key();

        let manifest = build_manifest(&claim, &key_a, b"key-a-id", b"key-a-id", vk_a.as_bytes());

        let trusted = vec![TrustedVerifyingKey {
            key_id: b"key-a-id".to_vec(),
            key: vk_a.clone(),
        }];

        // Tiny limit should reject before verification.
        let limits = ResourceLimits::builder()
            .max_input_bytes(image_bytes.len() - 1)
            .build();
        let result = verify_detached_manifest_with_limits_and_mac(
            image_bytes,
            &manifest,
            &TrustPolicy::TrustVerifyingKeys(trusted),
            Some(&limits),
            None,
        );

        assert!(!result.manifest_valid);
        assert!(!result.instance_digest_match);
    }
}
