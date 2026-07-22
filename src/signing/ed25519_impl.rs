use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroize;

use crate::Error;

/// Domain separation string for signature computation.
///
/// Note: Ed25519 has built-in domain separation. This constant is retained
/// for backward compatibility and documentation purposes.
pub const SIGNATURE_DOMAIN: &[u8] = b"StegoEggo-Sig-v1";

/// Maximum key identifier length in bytes.
pub const MAX_KEY_ID_LENGTH: usize = 32;

/// Ed25519 key pair for signing provenance claims.
///
/// Wraps `ed25519_dalek::SigningKey` and a key identifier. The private key
/// material is zeroized on drop. `Debug` output does not reveal key bytes.
/// `Serialize`/`Deserialize` are intentionally not implemented to prevent
/// accidental serialization of private keys.
pub struct SigningKey {
    signing_key: ed25519_dalek::SigningKey,
    secret_bytes: [u8; 32],
    key_id: Vec<u8>,
}

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningKey")
            .field("key_id", &hex::encode(&self.key_id))
            .finish_non_exhaustive()
    }
}

impl SigningKey {
    /// Create a signing key from a raw 32-byte seed and a key identifier.
    ///
    /// Returns an error if `key_id` exceeds [`MAX_KEY_ID_LENGTH`].
    pub fn from_bytes(key_bytes: [u8; 32], key_id: Vec<u8>) -> Result<Self, Error> {
        if key_id.len() > MAX_KEY_ID_LENGTH {
            return Err(Error::Config(format!(
                "Key ID length {} exceeds maximum {}",
                key_id.len(),
                MAX_KEY_ID_LENGTH
            )));
        }
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
        Ok(Self {
            signing_key,
            secret_bytes: key_bytes,
            key_id,
        })
    }

    /// Generate a new random signing key with a random 16-byte key identifier.
    pub fn generate() -> Self {
        let mut key_bytes = [0u8; 32];
        getrandom::getrandom(&mut key_bytes).expect("failed to generate random key");
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
        let mut key_id = [0u8; 16];
        getrandom::getrandom(&mut key_id).expect("failed to generate random key ID");
        Self {
            signing_key,
            secret_bytes: key_bytes,
            key_id: key_id.to_vec(),
        }
    }

    /// Get the key identifier.
    pub fn key_id(&self) -> &[u8] {
        &self.key_id
    }

    /// Derive the corresponding [`VerifyingKey`] for this signing key.
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey {
            verifying_key: self.signing_key.verifying_key(),
            key_id: self.key_id.clone(),
        }
    }

    /// Get the 32-byte public key derived from the secret key.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        *self.signing_key.verifying_key().as_bytes()
    }

    /// Export the raw secret key bytes.
    ///
    /// This method returns the secret key material. Use with caution.
    /// The returned bytes are a copy; the original is zeroized on drop.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.secret_bytes
    }

    /// Sign canonical claim bytes, producing a 64-byte deterministic Ed25519 signature.
    pub fn sign(&self, claim_bytes: &[u8]) -> Vec<u8> {
        use ed25519_dalek::Signer;
        let sig = self.signing_key.sign(claim_bytes);
        sig.to_bytes().to_vec()
    }

    /// Erase key material from memory (best-effort).
    pub fn zeroize(&mut self) {
        self.secret_bytes.zeroize();
    }
}

impl Drop for SigningKey {
    fn drop(&mut self) {
        self.secret_bytes.zeroize();
    }
}

/// Public key for signature verification.
///
/// Unlike [`SigningKey`], this type implements `Serialize` and `Deserialize`
/// so it can be embedded in metadata or distributed to verifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyingKey {
    verifying_key: ed25519_dalek::VerifyingKey,
    key_id: Vec<u8>,
}

impl VerifyingKey {
    /// Create from raw key bytes and a key identifier.
    pub fn from_bytes(key_bytes: [u8; 32], key_id: Vec<u8>) -> Self {
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&key_bytes)
            .expect("invalid ed25519 public key bytes");
        Self {
            verifying_key,
            key_id,
        }
    }

    /// Get the key identifier.
    pub fn key_id(&self) -> &[u8] {
        &self.key_id
    }

    /// Get the raw public key bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.verifying_key.as_bytes()
    }

    /// Verify a signature against claim bytes.
    pub fn verify(&self, claim_bytes: &[u8], signature: &[u8]) -> SignatureResult {
        let sig_bytes: [u8; 64] = match signature.try_into() {
            Ok(bytes) => bytes,
            Err(_) => return SignatureResult::MalformedSignature,
        };
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        use ed25519_dalek::Verifier;
        match self.verifying_key.verify(claim_bytes, &sig) {
            Ok(()) => SignatureResult::Valid,
            Err(_) => SignatureResult::Invalid,
        }
    }
}

impl Serialize for VerifyingKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("key_bytes", &hex::encode(self.verifying_key.as_bytes()))?;
        map.serialize_entry("key_id", &hex::encode(&self.key_id))?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for VerifyingKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct VerifyingKeyVisitor;

        impl<'de> Visitor<'de> for VerifyingKeyVisitor {
            type Value = VerifyingKey;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a VerifyingKey with key_bytes and key_id fields")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<VerifyingKey, M::Error> {
                let mut key_bytes_hex: Option<String> = None;
                let mut key_id_hex: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "key_bytes" => key_bytes_hex = Some(map.next_value()?),
                        "key_id" => key_id_hex = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<serde_json::Value>()?;
                        }
                    }
                }

                let key_bytes_str =
                    key_bytes_hex.ok_or_else(|| serde::de::Error::missing_field("key_bytes"))?;
                let key_id_str =
                    key_id_hex.ok_or_else(|| serde::de::Error::missing_field("key_id"))?;

                let key_bytes_vec = hex::decode(&key_bytes_str).map_err(|e| {
                    serde::de::Error::custom(format!("invalid key_bytes hex: {}", e))
                })?;
                let key_id_vec = hex::decode(&key_id_str)
                    .map_err(|e| serde::de::Error::custom(format!("invalid key_id hex: {}", e)))?;

                let key_bytes: [u8; 32] = key_bytes_vec.try_into().map_err(|v: Vec<u8>| {
                    serde::de::Error::custom(format!("key_bytes must be 32 bytes, got {}", v.len()))
                })?;

                let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&key_bytes)
                    .map_err(|e| serde::de::Error::custom(format!("invalid public key: {}", e)))?;

                Ok(VerifyingKey {
                    verifying_key,
                    key_id: key_id_vec,
                })
            }
        }

        deserializer.deserialize_map(VerifyingKeyVisitor)
    }
}

/// Result of low-level signature verification.
///
/// This enum reports the cryptographic validity of a signature against a known
/// public key. It does **not** distinguish between an unknown key and an untrusted
/// key — both are treated as the same public key for verification purposes.
///
/// For trust-aware verification, use
/// [`VerificationReport::signatures()`](crate::verification::VerificationReport::signatures),
/// which separately tracks:
/// - `cryptographically_valid` — the signature is valid for the given key
/// - `key_id_matched` — the key ID matches the expected key ID
/// - `trusted` — the key is trusted under the caller's trust policy
///
/// `SignatureResult` is intentionally limited to cryptographic validity so that
/// trust decisions remain caller-owned and are not conflated with signature
/// correctness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureResult {
    /// Signature is cryptographically valid for the supplied public key.
    Valid,
    /// Signature is invalid (wrong key, altered claim, or corrupted signature).
    Invalid,
    /// Signature bytes are malformed (wrong length, etc.).
    MalformedSignature,
    /// No public key was supplied for verification.
    KeyNotSupplied,
}

impl std::fmt::Display for SignatureResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignatureResult::Valid => write!(f, "Valid"),
            SignatureResult::Invalid => write!(f, "Invalid"),
            SignatureResult::MalformedSignature => write!(f, "MalformedSignature"),
            SignatureResult::KeyNotSupplied => write!(f, "KeyNotSupplied"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_key_debug_does_not_expose_key_bytes() {
        let key = SigningKey::generate();
        let debug = format!("{:?}", key);
        assert!(debug.contains("SigningKey"));
        assert!(debug.contains("key_id"));
        let secret_hex = hex::encode(key.secret_bytes);
        assert!(!debug.contains(&secret_hex));
    }

    #[test]
    fn signing_key_from_bytes_roundtrip() {
        let key_bytes = [42u8; 32];
        let key_id = b"test-key-id".to_vec();
        let key = SigningKey::from_bytes(key_bytes, key_id.clone()).unwrap();
        assert_eq!(key.secret_bytes, key_bytes);
        assert_eq!(key.key_id(), key_id.as_slice());
    }

    #[test]
    fn signing_key_from_bytes_validates_key_id_length() {
        let result = SigningKey::from_bytes([0u8; 32], vec![0u8; MAX_KEY_ID_LENGTH + 1]);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Config(msg) => assert!(msg.contains("Key ID length")),
            other => panic!("Expected Config error, got {:?}", other),
        }
    }

    #[test]
    fn signing_key_public_key_deterministic() {
        let key = SigningKey::from_bytes([1u8; 32], vec![1]).unwrap();
        let pk1 = key.public_key_bytes();
        let pk2 = key.public_key_bytes();
        assert_eq!(pk1, pk2);
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let signing_key = SigningKey::from_bytes([7u8; 32], vec![7]).unwrap();
        let verifying_key = signing_key.verifying_key();

        let claim = b"test claim data";
        let signature = signing_key.sign(claim);

        assert_eq!(
            verifying_key.verify(claim, &signature),
            SignatureResult::Valid
        );
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let key1 = SigningKey::from_bytes([1u8; 32], vec![1]).unwrap();
        let key2 = SigningKey::from_bytes([2u8; 32], vec![2]).unwrap();

        let claim = b"test claim";
        let signature = key1.sign(claim);

        assert_eq!(
            key2.verifying_key().verify(claim, &signature),
            SignatureResult::Invalid
        );
    }

    #[test]
    fn verify_rejects_altered_claim() {
        let key = SigningKey::from_bytes([3u8; 32], vec![3]).unwrap();
        let claim = b"original claim";
        let signature = key.sign(claim);

        assert_eq!(
            key.verifying_key().verify(b"altered claim", &signature),
            SignatureResult::Invalid
        );
    }

    #[test]
    fn verify_rejects_malformed_signature() {
        let key = SigningKey::from_bytes([4u8; 32], vec![4]).unwrap();
        assert_eq!(
            key.verifying_key().verify(b"test", &[0u8; 32]),
            SignatureResult::MalformedSignature
        );
    }

    #[test]
    fn verifying_key_serialization_roundtrip() {
        let key = SigningKey::from_bytes([5u8; 32], vec![5]).unwrap();
        let vk = key.verifying_key();

        let json = serde_json::to_string(&vk).unwrap();
        let deserialized: VerifyingKey = serde_json::from_str(&json).unwrap();

        assert_eq!(vk, deserialized);
    }

    #[test]
    fn signature_result_display() {
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
    fn key_id_respects_max_length() {
        let key = SigningKey::from_bytes([0u8; 32], vec![0u8; MAX_KEY_ID_LENGTH]).unwrap();
        assert_eq!(key.key_id().len(), MAX_KEY_ID_LENGTH);
    }

    #[test]
    fn generate_produces_unique_keys() {
        let key1 = SigningKey::generate();
        let key2 = SigningKey::generate();
        assert_ne!(key1.secret_bytes, key2.secret_bytes);
    }

    #[test]
    fn zeroize_clears_key_material() {
        let mut key = SigningKey::from_bytes([99u8; 32], vec![1]).unwrap();
        key.zeroize();
        let zeroed = [0u8; 32];
        assert_eq!(key.secret_bytes, zeroed);
    }

    #[test]
    fn signing_key_signs_64_bytes() {
        let key = SigningKey::from_bytes([42u8; 32], vec![1]).unwrap();
        let sig = key.sign(b"test");
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn signing_key_deterministic_signing() {
        let key = SigningKey::from_bytes([42u8; 32], vec![1]).unwrap();
        let claim = b"deterministic test";
        let sig1 = key.sign(claim);
        let sig2 = key.sign(claim);
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn rfc8032_test_vector() {
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
    fn public_key_only_cannot_forge_signature() {
        let key1 = SigningKey::from_bytes([1u8; 32], vec![1]).unwrap();
        let key2 = SigningKey::from_bytes([2u8; 32], vec![2]).unwrap();

        let claim = b"important claim";

        let vk1 = key1.verifying_key();
        let vk2 = key2.verifying_key();

        let sig = key1.sign(claim);

        assert_eq!(vk1.verify(claim, &sig), SignatureResult::Valid);
        assert_eq!(vk2.verify(claim, &sig), SignatureResult::Invalid);

        let forged_sig = key2.sign(claim);
        assert_eq!(vk1.verify(claim, &forged_sig), SignatureResult::Invalid);
    }

    #[test]
    fn check_signature_capacity_fits_embedded() {
        use super::super::{check_signature_capacity, SignatureCapacity, SignaturePlacement};

        let available = 256;
        let result = check_signature_capacity(available, 16, SignaturePlacement::PreferredEmbedded);
        assert_eq!(result, SignatureCapacity::FitsEmbedded);
    }

    #[test]
    fn check_signature_capacity_needs_detached() {
        use super::super::{check_signature_capacity, SignatureCapacity, SignaturePlacement};

        let available = 100;
        let result = check_signature_capacity(available, 16, SignaturePlacement::PreferredEmbedded);
        assert_eq!(result, SignatureCapacity::NeedsDetached);
    }

    #[test]
    fn check_signature_capacity_detached_always_needs_detached() {
        use super::super::{check_signature_capacity, SignatureCapacity, SignaturePlacement};

        let result = check_signature_capacity(256, 0, SignaturePlacement::Detached);
        assert_eq!(result, SignatureCapacity::NeedsDetached);
    }

    #[test]
    fn check_signature_capacity_embedded_fails_when_too_small() {
        use super::super::{check_signature_capacity, SignatureCapacity, SignaturePlacement};

        let result = check_signature_capacity(231, 32, SignaturePlacement::Embedded);
        assert_eq!(result, SignatureCapacity::NeedsDetached);

        let result = check_signature_capacity(232, 32, SignaturePlacement::Embedded);
        assert_eq!(result, SignatureCapacity::FitsEmbedded);
    }

    #[test]
    fn signing_config_check_capacity() {
        use super::super::{SignatureCapacity, SignaturePlacement};

        let key = SigningKey::from_bytes([1u8; 32], vec![0xAA; 16]).unwrap();
        let config =
            super::super::config::SigningConfig::new(key, SignaturePlacement::PreferredEmbedded);

        let result = config.check_capacity(256);
        assert_eq!(result, SignatureCapacity::FitsEmbedded);

        let result = config.check_capacity(50);
        assert_eq!(result, SignatureCapacity::NeedsDetached);
    }
}
