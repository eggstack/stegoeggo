#![cfg(feature = "detached-manifest")]

use stegoeggo::detached::{
    DetachedManifest, EmbeddedReference, PublicKeyEntry, SignatureRecord, MAX_MANIFEST_SIZE,
    MAX_PUBLIC_KEYS, MAX_SIGNATURES,
};
use stegoeggo::provenance::ProvenanceClaim;

fn make_test_claim() -> ProvenanceClaim {
    ProvenanceClaim::new(1)
        .with_content_code("iscc:test123".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 1920, 1080, 524288)
        .with_software("stegoeggo/0.3.0")
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
            signature: "sig_data".to_string(),
        })
        .with_public_key(PublicKeyEntry {
            key_id: vec![10],
            algorithm: "ed25519".to_string(),
            key_bytes: "pub_key_hex".to_string(),
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
