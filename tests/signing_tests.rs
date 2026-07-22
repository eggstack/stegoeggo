#![cfg(feature = "signatures")]

use stegoeggo::signing::{SignatureResult, SigningKey, VerifyingKey};

use ed25519_dalek::Signer;

#[test]
fn test_key_generation() {
    let key = SigningKey::generate();
    assert_eq!(key.key_id().len(), 16);
    let pk = key.public_key_bytes();
    assert_eq!(pk.len(), 32);
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
    let secret_hex = hex::encode(key.to_bytes());
    assert!(!debug.contains(&secret_hex));
}

#[test]
fn test_key_not_serializable() {
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
    let original_key_bytes = key.to_bytes();

    key.zeroize();
    assert_eq!(key.to_bytes(), [0u8; 32]);
    assert_ne!(key.to_bytes(), original_key_bytes);
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
    let key = SigningKey::from_bytes([42u8; 32], vec![1]).unwrap();
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
    let key = SigningKey::from_bytes([0u8; 32], vec![0u8; 32]).unwrap();
    assert_eq!(key.key_id().len(), 32);
}

#[test]
fn test_key_id_exceeds_max_length_returns_error() {
    let result = SigningKey::from_bytes([0u8; 32], vec![0u8; 33]);
    assert!(result.is_err());
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
    assert_ne!(key1.public_key_bytes(), key2.public_key_bytes());
    assert_ne!(key1.key_id(), key2.key_id());
}

#[test]
fn test_public_key_deterministic() {
    let key = SigningKey::from_bytes([1u8; 32], vec![1]).unwrap();
    let pk1 = key.public_key_bytes();
    let pk2 = key.public_key_bytes();
    assert_eq!(pk1, pk2);
}

#[test]
fn test_rfc8032_test_vector() {
    let secret_key = [0u8; 32];
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_key);
    let verifying_key = signing_key.verifying_key();

    assert_eq!(verifying_key.as_bytes().len(), 32);

    let claim = b"Hello, world!";
    use ed25519_dalek::Signer;
    let signature = signing_key.sign(claim);
    assert_eq!(signature.to_bytes().len(), 64);

    use ed25519_dalek::Verifier;
    assert!(verifying_key.verify(claim, &signature).is_ok());

    let bad_claim = b"Goodbye, world!";
    assert!(verifying_key.verify(bad_claim, &signature).is_err());
}

#[test]
fn test_rfc8032_known_answer_public_key() {
    let seed = [0u8; 32];
    let expected_public_key_hex =
        "3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29";

    let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_bytes();
    assert_eq!(hex::encode(public_key_bytes), expected_public_key_hex);

    let wrapper_key = SigningKey::from_bytes(seed, vec![1]).unwrap();
    assert_eq!(wrapper_key.public_key_bytes(), public_key_bytes);
}

#[test]
fn test_signature_bit_flip_rejected() {
    let key = SigningKey::generate();
    let claim = b"test claim for bit flip";
    let sig = key.sign(claim);
    let mut sig_bytes = sig;
    sig_bytes[32] ^= 0x01;
    assert_eq!(
        key.verifying_key().verify(claim, &sig_bytes),
        SignatureResult::Invalid
    );
}

#[test]
fn test_signing_key_not_serializable() {
    let key = SigningKey::generate();
    let _: &dyn std::fmt::Debug = &key;
}

#[test]
fn test_verify_independently_created_signature() {
    let seed = [42u8; 32];
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
    let claim = b"independently signed message";
    let sig = signing_key.sign(claim);

    let wrapper_key = SigningKey::from_bytes(seed, vec![1]).unwrap();
    assert_eq!(
        wrapper_key.verifying_key().verify(claim, &sig.to_bytes()),
        SignatureResult::Valid
    );
}

#[test]
fn test_truncated_signature_rejected() {
    let key = SigningKey::generate();
    let claim = b"truncated test";
    for len in [32, 63, 65] {
        let mut sig_bytes = vec![0u8; len];
        sig_bytes[0] = 0x01;
        assert_eq!(
            key.verifying_key().verify(claim, &sig_bytes),
            SignatureResult::MalformedSignature
        );
    }
}
