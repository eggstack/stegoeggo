#![cfg(all(feature = "signatures", feature = "detached-manifest"))]

use stegoeggo::detached::{
    DetachedManifest, EmbeddedReference, PublicKeyEntry, SignatureRecord, TrustMetadata,
};
use stegoeggo::provenance::{canonical_json, ProvenanceClaim};
use stegoeggo::signing::SigningKey;

// ============================================================================
// Known-answer test vector: Ed25519 signing with fixed key
// ============================================================================

/// Fixed 32-byte secret key for deterministic test vectors.
const FIXED_SECRET_KEY: [u8; 32] = [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
];

/// Fixed key ID for deterministic test vectors.
const FIXED_KEY_ID: &[u8] = b"test-key-001";

#[test]
fn test_known_answer_signing_deterministic() {
    let signing_key = SigningKey::from_bytes(FIXED_SECRET_KEY, FIXED_KEY_ID.to_vec()).unwrap();
    let verifying_key = signing_key.verifying_key();

    let claim = b"test provenance claim data for known-answer vector";

    let sig1 = signing_key.sign(claim);
    let sig2 = signing_key.sign(claim);

    assert_eq!(sig1, sig2, "Ed25519 signing must be deterministic");
    assert_eq!(sig1.len(), 64, "Ed25519 signature must be 64 bytes");

    assert_eq!(
        verifying_key.verify(claim, &sig1),
        stegoeggo::signing::SignatureResult::Valid,
    );
}

#[test]
fn test_known_answer_verifying_key_derivation() {
    let signing_key = SigningKey::from_bytes(FIXED_SECRET_KEY, FIXED_KEY_ID.to_vec()).unwrap();
    let vk1 = signing_key.verifying_key();
    let vk2 = signing_key.verifying_key();

    assert_eq!(vk1, vk2, "Verifying key derivation must be deterministic");
    assert_eq!(vk1.key_id(), FIXED_KEY_ID);
    assert_eq!(vk1.as_bytes().len(), 32);
}

#[test]
fn test_known_answer_wrong_key_rejected() {
    let key1 = SigningKey::from_bytes(FIXED_SECRET_KEY, FIXED_KEY_ID.to_vec()).unwrap();
    let key2_bytes: [u8; 32] = [
        0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0xF9, 0xF8, 0xF7, 0xF6, 0xF5, 0xF4, 0xF3, 0xF2, 0xF1,
        0xF0, 0xEF, 0xEE, 0xED, 0xEC, 0xEB, 0xEA, 0xE9, 0xE8, 0xE7, 0xE6, 0xE5, 0xE4, 0xE3, 0xE2,
        0xE1, 0xE0,
    ];
    let key2 = SigningKey::from_bytes(key2_bytes, b"wrong-key".to_vec()).unwrap();

    let claim = b"test claim for wrong key rejection";
    let signature = key1.sign(claim);

    assert_eq!(
        key2.verifying_key().verify(claim, &signature),
        stegoeggo::signing::SignatureResult::Invalid,
    );
}

#[test]
fn test_known_answer_altered_claim_rejected() {
    let signing_key = SigningKey::from_bytes(FIXED_SECRET_KEY, FIXED_KEY_ID.to_vec()).unwrap();
    let verifying_key = signing_key.verifying_key();

    let claim = b"original claim for alteration test";
    let signature = signing_key.sign(claim);

    let altered = b"altered claim for alteration test";
    assert_eq!(
        verifying_key.verify(altered, &signature),
        stegoeggo::signing::SignatureResult::Invalid,
    );
}

// ============================================================================
// Known-answer test vector: Canonical claim serialization
// ============================================================================

#[test]
fn test_known_answer_canonical_claim_deterministic() {
    let claim = ProvenanceClaim::new(2)
        .with_content_code("iscc:a1b2c3d4e5f6a7b8".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 1920, 1080, 524288)
        .with_issuer_id("dGVzdC1pc3N1ZXI".to_string())
        .with_software("stegoeggo/0.2.2");

    let bytes1 = claim.canonical_bytes();
    let bytes2 = claim.canonical_bytes();

    assert_eq!(bytes1, bytes2, "Canonical bytes must be deterministic");
    assert!(bytes1.len() > 50, "Canonical bytes must be non-trivial");

    let canonical_str = String::from_utf8(bytes1.clone()).unwrap();
    assert!(canonical_str.contains("content_code"));
    assert!(canonical_str.contains("created_at"));
    assert!(canonical_str.contains("rights_policy"));
    assert!(
        !canonical_str.contains("claim_id"),
        "claim_id must be excluded"
    );
}

#[test]
fn test_known_answer_canonical_json_excludes_claim_id() {
    let claim = ProvenanceClaim::new(0)
        .with_content_code("iscc:test".to_string())
        .with_software("test/1.0");

    let canonical = canonical_json(&claim);
    let canonical_str = String::from_utf8(canonical).unwrap();

    assert!(!canonical_str.contains("claim_id"));
    assert!(canonical_str.contains("content_code"));
}

#[test]
fn test_known_answer_claim_digest_deterministic() {
    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:deterministic".to_string())
        .with_creation_time(2000)
        .with_source_facts("png", 100, 100, 512)
        .with_software("test/1.0");

    let digest1 = claim.digest();
    let digest2 = claim.digest();

    assert_eq!(digest1, digest2, "Claim digest must be deterministic");
    assert_eq!(digest1.len(), 32, "SHA-256 digest must be 32 bytes");
}

// ============================================================================
// Known-answer test vector: Detached manifest
// ============================================================================

#[test]
fn test_known_answer_manifest_roundtrip() {
    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:manifest-test".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("jpeg", 640, 480, 102400)
        .with_software("stegoeggo/0.2.2");

    let manifest = DetachedManifest::new(claim)
        .with_signature(SignatureRecord {
            algorithm: "ed25519".to_string(),
            key_id: vec![0xAA, 0xBB],
            signature: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".to_string(),
        })
        .with_public_key(PublicKeyEntry {
            key_id: vec![0xAA, 0xBB],
            algorithm: "ed25519".to_string(),
            key_bytes: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        })
        .with_embedded_reference(EmbeddedReference {
            payload_digest:
                "sha256:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
                    .to_string(),
            payload_version: 3,
        });

    let bytes = manifest.canonical_bytes();
    let parsed = DetachedManifest::from_json(&bytes).unwrap();

    assert_eq!(parsed.schema_version, 1);
    assert_eq!(parsed.claim.content_code, "iscc:manifest-test");
    assert_eq!(parsed.signatures.len(), 1);
    assert_eq!(parsed.public_keys.len(), 1);
    assert!(parsed.embedded_reference.is_some());
    assert!(parsed.trust_metadata.is_none());
}

#[test]
fn test_known_answer_manifest_with_trust_metadata() {
    let claim = ProvenanceClaim::new(0)
        .with_content_code("iscc:trust-test".to_string())
        .with_software("test/1.0");

    let trust = TrustMetadata {
        trust_model: "local".to_string(),
        trusted: true,
        reason: "Key verified by operator".to_string(),
        certificate_chain: None,
    };

    let manifest = DetachedManifest::new(claim).with_trust_metadata(trust.clone());

    let bytes = manifest.canonical_bytes();
    let parsed = DetachedManifest::from_json(&bytes).unwrap();

    assert!(parsed.trust_metadata.is_some());
    let parsed_trust = parsed.trust_metadata.unwrap();
    assert_eq!(parsed_trust.trust_model, "local");
    assert!(parsed_trust.trusted);
    assert_eq!(parsed_trust.reason, "Key verified by operator");
}

#[test]
fn test_known_answer_manifest_digest_stability() {
    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:digest-test".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 800, 600, 4096)
        .with_software("stegoeggo/0.2.2");

    let manifest = DetachedManifest::new(claim);
    let digest1 = manifest.digest();
    let digest2 = manifest.digest();

    assert_eq!(digest1, digest2, "Manifest digest must be deterministic");
    assert_eq!(digest1.len(), 32);
}
