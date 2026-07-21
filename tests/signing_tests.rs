#![cfg(feature = "signatures")]

use stegoeggo::signing::{SignatureResult, SigningKey, VerifyingKey};

#[test]
fn test_key_generation() {
    let key = SigningKey::generate();
    assert_eq!(key.key_id().len(), 16);
    assert_eq!(key.key_bytes().len(), 32);
}

#[test]
fn test_sign_and_verify() {
    let signing_key = SigningKey::generate();
    let verifying_key = signing_key.verifying_key();

    let claim = b"test provenance claim data";
    let signature = signing_key.sign(claim);

    assert_eq!(
        verifying_key.verify(claim, &signature),
        SignatureResult::Valid
    );
}

#[test]
fn test_wrong_key_fails() {
    let key1 = SigningKey::generate();
    let key2 = SigningKey::generate();

    let claim = b"test claim";
    let signature = key1.sign(claim);

    assert_eq!(
        key2.verifying_key().verify(claim, &signature),
        SignatureResult::Invalid
    );
}

#[test]
fn test_altered_claim_fails() {
    let key = SigningKey::generate();
    let claim = b"original claim";
    let signature = key.sign(claim);

    assert_eq!(
        key.verifying_key().verify(b"altered claim", &signature),
        SignatureResult::Invalid
    );
}

#[test]
fn test_key_not_revealed_in_debug() {
    let key = SigningKey::generate();
    let debug = format!("{:?}", key);

    assert!(debug.contains("SigningKey"));
    assert!(debug.contains("key_id"));
    assert!(!debug.contains(&hex::encode(key.key_bytes())));
}

#[test]
fn test_key_not_serializable() {
    // SigningKey intentionally does not implement Serialize.
    // VerifyingKey does — confirm it roundtrips through JSON.
    let key = SigningKey::generate();
    let vk = key.verifying_key();

    let vk_json = serde_json::to_string(&vk).unwrap();
    assert!(vk_json.contains("key_bytes"));
    assert!(vk_json.contains("key_id"));

    let deserialized: VerifyingKey = serde_json::from_str(&vk_json).unwrap();
    assert_eq!(vk, deserialized);
}

#[test]
fn test_zeroize_on_drop() {
    let mut key = SigningKey::generate();
    let original_key_bytes = *key.key_bytes();

    key.zeroize();
    assert_eq!(key.key_bytes(), &[0u8; 32]);
    assert_ne!(*key.key_bytes(), original_key_bytes);
}

#[test]
fn test_verifying_key_serialization_roundtrip() {
    let signing_key = SigningKey::generate();
    let vk = signing_key.verifying_key();

    let json = serde_json::to_string(&vk).unwrap();
    let deserialized: VerifyingKey = serde_json::from_str(&json).unwrap();

    assert_eq!(vk, deserialized);
}

#[test]
fn test_signature_is_64_bytes() {
    let key = SigningKey::generate();
    let signature = key.sign(b"test");
    assert_eq!(signature.len(), 64);
}

#[test]
fn test_deterministic_signing() {
    let key = SigningKey::from_bytes([42u8; 32], vec![1]);
    let claim = b"deterministic test";

    let sig1 = key.sign(claim);
    let sig2 = key.sign(claim);

    assert_eq!(sig1, sig2);
}

#[test]
fn test_malformed_signature_rejected() {
    let key = SigningKey::generate();
    assert_eq!(
        key.verifying_key().verify(b"test", &[0u8; 32]),
        SignatureResult::MalformedSignature
    );
}

#[test]
fn test_key_id_at_max_length() {
    let key = SigningKey::from_bytes([0u8; 32], vec![0u8; 32]);
    assert_eq!(key.key_id().len(), 32);
}

#[test]
#[should_panic(expected = "Key ID too long")]
fn test_key_id_exceeds_max_length_panics() {
    SigningKey::from_bytes([0u8; 32], vec![0u8; 33]);
}

#[test]
fn test_signature_result_display() {
    assert_eq!(SignatureResult::Valid.to_string(), "Valid");
    assert_eq!(SignatureResult::Invalid.to_string(), "Invalid");
    assert_eq!(
        SignatureResult::MalformedSignature.to_string(),
        "MalformedSignature"
    );
    assert_eq!(
        SignatureResult::KeyNotSupplied.to_string(),
        "KeyNotSupplied"
    );
}

#[test]
fn test_generate_produces_unique_keys() {
    let key1 = SigningKey::generate();
    let key2 = SigningKey::generate();
    assert_ne!(key1.key_bytes(), key2.key_bytes());
    assert_ne!(key1.key_id(), key2.key_id());
}

#[test]
fn test_public_key_deterministic() {
    let key = SigningKey::from_bytes([1u8; 32], vec![1]);
    let pk1 = key.public_key_bytes();
    let pk2 = key.public_key_bytes();
    assert_eq!(pk1, pk2);
}
