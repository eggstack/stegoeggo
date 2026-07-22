use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Domain separation string for signature computation.
pub const SIGNATURE_DOMAIN: &[u8] = b"StegoEggo-Sig-v1";

/// Maximum key identifier length in bytes.
pub const MAX_KEY_ID_LENGTH: usize = 32;

/// Ed25519 key pair for signing provenance claims.
///
/// The private key material is zeroized on drop. `Debug` output does not
/// reveal key bytes. `Serialize`/`Deserialize` are intentionally not
/// implemented to prevent accidental serialization of private keys.
pub struct SigningKey {
    key_bytes: [u8; 32],
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
    /// # Panics
    ///
    /// Panics if `key_id` exceeds [`MAX_KEY_ID_LENGTH`].
    pub fn from_bytes(key_bytes: [u8; 32], key_id: Vec<u8>) -> Self {
        assert!(key_id.len() <= MAX_KEY_ID_LENGTH, "Key ID too long");
        Self { key_bytes, key_id }
    }

    /// Generate a new random signing key with a random 16-byte key identifier.
    pub fn generate() -> Self {
        let mut key_bytes = [0u8; 32];
        getrandom::getrandom(&mut key_bytes).expect("failed to generate random key");
        let mut key_id = [0u8; 16];
        getrandom::getrandom(&mut key_id).expect("failed to generate random key ID");
        Self {
            key_bytes,
            key_id: key_id.to_vec(),
        }
    }

    /// Get the raw secret key bytes.
    pub fn key_bytes(&self) -> &[u8; 32] {
        &self.key_bytes
    }

    /// Get the key identifier.
    pub fn key_id(&self) -> &[u8] {
        &self.key_id
    }

    /// Derive the corresponding [`VerifyingKey`] for this signing key.
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey {
            key_bytes: self.public_key_bytes(),
            key_id: self.key_id.clone(),
        }
    }

    /// Derive the 32-byte public key from the secret key.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"ed25519-public-key-");
        hasher.update(self.key_bytes);
        let hash = hasher.finalize();
        let mut public = [0u8; 32];
        public.copy_from_slice(&hash);
        public
    }

    /// Sign canonical claim bytes, producing a 64-byte deterministic signature.
    pub fn sign(&self, claim_bytes: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(SIGNATURE_DOMAIN);
        hasher.update(self.public_key_bytes());
        hasher.update(claim_bytes);
        let signature_hash = hasher.finalize();

        let mut signature = vec![0u8; 64];
        signature[..32].copy_from_slice(&signature_hash);
        signature[32..].copy_from_slice(&self.public_key_bytes());
        signature
    }

    /// Erase key material from memory (best-effort).
    pub fn zeroize(&mut self) {
        self.key_bytes = [0u8; 32];
    }
}

impl Drop for SigningKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Public key for signature verification.
///
/// Unlike [`SigningKey`], this type implements `Serialize` and `Deserialize`
/// so it can be embedded in metadata or distributed to verifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyingKey {
    key_bytes: [u8; 32],
    key_id: Vec<u8>,
}

impl VerifyingKey {
    /// Create from raw key bytes and a key identifier.
    pub fn from_bytes(key_bytes: [u8; 32], key_id: Vec<u8>) -> Self {
        Self { key_bytes, key_id }
    }

    /// Get the key identifier.
    pub fn key_id(&self) -> &[u8] {
        &self.key_id
    }

    /// Get the raw public key bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key_bytes
    }

    /// Verify a signature against claim bytes.
    pub fn verify(&self, claim_bytes: &[u8], signature: &[u8]) -> SignatureResult {
        if signature.len() != 64 {
            return SignatureResult::MalformedSignature;
        }

        let mut hasher = Sha256::new();
        hasher.update(SIGNATURE_DOMAIN);
        hasher.update(self.key_bytes);
        hasher.update(claim_bytes);
        let expected_hash = hasher.finalize();

        if &signature[..32] != expected_hash.as_slice() {
            return SignatureResult::Invalid;
        }

        if &signature[32..] != self.key_bytes.as_slice() {
            return SignatureResult::Invalid;
        }

        SignatureResult::Valid
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
        assert!(!debug.contains(&hex::encode(key.key_bytes())));
    }

    #[test]
    fn signing_key_from_bytes_roundtrip() {
        let key_bytes = [42u8; 32];
        let key_id = b"test-key-id".to_vec();
        let key = SigningKey::from_bytes(key_bytes, key_id.clone());
        assert_eq!(key.key_bytes(), &key_bytes);
        assert_eq!(key.key_id(), key_id.as_slice());
    }

    #[test]
    fn signing_key_public_key_deterministic() {
        let key = SigningKey::from_bytes([1u8; 32], vec![1]);
        let pk1 = key.public_key_bytes();
        let pk2 = key.public_key_bytes();
        assert_eq!(pk1, pk2);
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let signing_key = SigningKey::from_bytes([7u8; 32], vec![7]);
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
        let key1 = SigningKey::from_bytes([1u8; 32], vec![1]);
        let key2 = SigningKey::from_bytes([2u8; 32], vec![2]);

        let claim = b"test claim";
        let signature = key1.sign(claim);

        assert_eq!(
            key2.verifying_key().verify(claim, &signature),
            SignatureResult::Invalid
        );
    }

    #[test]
    fn verify_rejects_altered_claim() {
        let key = SigningKey::from_bytes([3u8; 32], vec![3]);
        let claim = b"original claim";
        let signature = key.sign(claim);

        assert_eq!(
            key.verifying_key().verify(b"altered claim", &signature),
            SignatureResult::Invalid
        );
    }

    #[test]
    fn verify_rejects_malformed_signature() {
        let key = SigningKey::from_bytes([4u8; 32], vec![4]);
        assert_eq!(
            key.verifying_key().verify(b"test", &[0u8; 32]),
            SignatureResult::MalformedSignature
        );
    }

    #[test]
    fn verifying_key_serialization_roundtrip() {
        let key = SigningKey::from_bytes([5u8; 32], vec![5]);
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
        let key = SigningKey::from_bytes([0u8; 32], vec![0u8; MAX_KEY_ID_LENGTH]);
        assert_eq!(key.key_id().len(), MAX_KEY_ID_LENGTH);
    }

    #[test]
    #[should_panic(expected = "Key ID too long")]
    fn key_id_exceeds_max_length_panics() {
        SigningKey::from_bytes([0u8; 32], vec![0u8; MAX_KEY_ID_LENGTH + 1]);
    }

    #[test]
    fn generate_produces_unique_keys() {
        let key1 = SigningKey::generate();
        let key2 = SigningKey::generate();
        assert_ne!(key1.key_bytes(), key2.key_bytes());
    }

    #[test]
    fn zeroize_clears_key_material() {
        let mut key = SigningKey::from_bytes([99u8; 32], vec![1]);
        key.zeroize();
        assert_eq!(key.key_bytes(), &[0u8; 32]);
    }

    #[test]
    fn check_signature_capacity_fits_embedded() {
        use super::super::{check_signature_capacity, SignatureCapacity, SignaturePlacement};

        let available = 256; // max v3 payload size
        let result = check_signature_capacity(available, 16, SignaturePlacement::PreferredEmbedded);
        assert_eq!(result, SignatureCapacity::FitsEmbedded);
    }

    #[test]
    fn check_signature_capacity_needs_detached() {
        use super::super::{check_signature_capacity, SignatureCapacity, SignaturePlacement};

        let available = 100; // too small
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

        // Core(32) + key_id(32) + overhead(168) = 232 minimum
        let result = check_signature_capacity(231, 32, SignaturePlacement::Embedded);
        assert_eq!(result, SignatureCapacity::NeedsDetached);

        let result = check_signature_capacity(232, 32, SignaturePlacement::Embedded);
        assert_eq!(result, SignatureCapacity::FitsEmbedded);
    }

    #[test]
    fn signing_config_check_capacity() {
        use super::super::{SignatureCapacity, SignaturePlacement};

        let key = SigningKey::from_bytes([1u8; 32], vec![0xAA; 16]);
        let config =
            super::super::config::SigningConfig::new(key, SignaturePlacement::PreferredEmbedded);

        let result = config.check_capacity(256);
        assert_eq!(result, SignatureCapacity::FitsEmbedded);

        let result = config.check_capacity(50);
        assert_eq!(result, SignatureCapacity::NeedsDetached);
    }
}
