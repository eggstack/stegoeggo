use super::ed25519_impl::{SigningKey, VerifyingKey};

/// Where a signature should be placed relative to the protected image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignaturePlacement {
    /// Embed the signature in the image payload (when capacity permits).
    Embedded,
    /// Store the signature in a detached manifest only.
    Detached,
    /// Embed when capacity permits, otherwise fall back to detached.
    PreferredEmbedded,
}

/// Configuration for Ed25519 signing operations.
///
/// Bundles the signing key, key identifier, and placement preference
/// into a single configuration type. This avoids scattering key
/// material across multiple API parameters.
///
/// # Security
///
/// `SigningConfig` contains secret key material and intentionally
/// does not implement `Serialize`. Use `to_bytes()` / `from_bytes()`
/// for explicit key serialization when needed.
pub struct SigningConfig {
    signing_key: SigningKey,
    placement: SignaturePlacement,
}

impl std::fmt::Debug for SigningConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningConfig")
            .field("key_id", &hex::encode(self.signing_key.key_id()))
            .field("placement", &self.placement)
            .finish_non_exhaustive()
    }
}

impl SigningConfig {
    /// Create a new signing configuration from a signing key.
    #[must_use]
    pub fn new(signing_key: SigningKey, placement: SignaturePlacement) -> Self {
        Self {
            signing_key,
            placement,
        }
    }

    /// Create a signing configuration with preferred-embedded placement.
    #[must_use]
    pub fn with_key(signing_key: SigningKey) -> Self {
        Self {
            signing_key,
            placement: SignaturePlacement::PreferredEmbedded,
        }
    }

    /// Get a reference to the signing key.
    #[must_use]
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// Get the key identifier.
    #[must_use]
    pub fn key_id(&self) -> &[u8] {
        self.signing_key.key_id()
    }

    /// Get the corresponding verifying key.
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Get the signature placement preference.
    #[must_use]
    pub fn placement(&self) -> SignaturePlacement {
        self.placement
    }

    /// Set the signature placement preference.
    #[must_use]
    pub fn with_placement(mut self, placement: SignaturePlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Get the raw secret key bytes.
    #[must_use]
    pub fn key_bytes(&self) -> &[u8; 32] {
        self.signing_key.key_bytes()
    }

    /// Check whether the signature fits within the available payload capacity.
    ///
    /// Delegates to [`check_signature_capacity`](super::check_signature_capacity)
    /// using this config's key ID length and placement preference.
    #[must_use]
    pub fn check_capacity(&self, available_bytes: usize) -> super::SignatureCapacity {
        super::check_signature_capacity(available_bytes, self.key_id().len(), self.placement)
    }
}

impl Drop for SigningConfig {
    fn drop(&mut self) {
        self.signing_key.zeroize();
    }
}
