use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::provenance::ProvenanceClaim;

/// Current detached manifest schema version.
pub const MANIFEST_SCHEMA_VERSION: u8 = 1;
/// Maximum manifest size in bytes.
pub const MAX_MANIFEST_SIZE: usize = 64 * 1024;
/// Maximum number of signature records.
pub const MAX_SIGNATURES: usize = 16;
/// Maximum number of public key entries.
pub const MAX_PUBLIC_KEYS: usize = 16;

/// A signed detached manifest containing a provenance claim and its signatures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetachedManifest {
    /// Schema version.
    pub schema_version: u8,
    /// The provenance claim.
    pub claim: ProvenanceClaim,
    /// Signature records.
    pub signatures: Vec<SignatureRecord>,
    /// Public key entries for signature verification.
    pub public_keys: Vec<PublicKeyEntry>,
    /// Optional reference to an embedded payload.
    pub embedded_reference: Option<EmbeddedReference>,
    /// Optional trust metadata for certificate chains and trust policies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_metadata: Option<TrustMetadata>,
}

/// Trust metadata for certificate chains and trust policies.
///
/// Bounded to prevent unbounded expansion. The library does not
/// ship an implicit trust store — this metadata is informational
/// and caller-validated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustMetadata {
    /// Trust model name (e.g., "local", "web-of-trust", "pki").
    pub trust_model: String,
    /// Whether the claim is trusted under the specified model.
    pub trusted: bool,
    /// Human-readable reason for the trust decision.
    pub reason: String,
    /// Optional certificate chain (DER-encoded, base64).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate_chain: Option<Vec<String>>,
}

/// A single signature record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureRecord {
    /// Signature algorithm name.
    pub algorithm: String,
    /// Key identifier.
    pub key_id: Vec<u8>,
    /// Hex-encoded signature bytes.
    pub signature: String,
}

/// A public key entry for signature verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyEntry {
    /// Key identifier.
    pub key_id: Vec<u8>,
    /// Key algorithm name.
    pub algorithm: String,
    /// Base64-encoded public key bytes.
    pub key_bytes: String,
}

/// Reference to an embedded stego payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedReference {
    /// Digest of the embedded payload.
    pub payload_digest: String,
    /// Payload format version.
    pub payload_version: u8,
}

impl DetachedManifest {
    /// Create a new manifest for the given provenance claim.
    pub fn new(claim: ProvenanceClaim) -> Self {
        Self {
            schema_version: MANIFEST_SCHEMA_VERSION,
            claim,
            signatures: Vec::new(),
            public_keys: Vec::new(),
            embedded_reference: None,
            trust_metadata: None,
        }
    }

    /// Add a signature record (up to [`MAX_SIGNATURES`]).
    #[must_use]
    pub fn with_signature(mut self, sig: SignatureRecord) -> Self {
        if self.signatures.len() < MAX_SIGNATURES {
            self.signatures.push(sig);
        }
        self
    }

    /// Add a public key entry (up to [`MAX_PUBLIC_KEYS`]).
    #[must_use]
    pub fn with_public_key(mut self, key: PublicKeyEntry) -> Self {
        if self.public_keys.len() < MAX_PUBLIC_KEYS {
            self.public_keys.push(key);
        }
        self
    }

    /// Set the embedded payload reference.
    #[must_use]
    pub fn with_embedded_reference(mut self, reference: EmbeddedReference) -> Self {
        self.embedded_reference = Some(reference);
        self
    }

    /// Set the trust metadata.
    #[must_use]
    pub fn with_trust_metadata(mut self, trust: TrustMetadata) -> Self {
        self.trust_metadata = Some(trust);
        self
    }

    /// Serialize the manifest to canonical JSON bytes.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self)
            .expect("detached manifest canonical serialization failed")
            .into_bytes()
    }

    /// Compute SHA-256 digest of the canonical bytes.
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.canonical_bytes());
        hasher.finalize().into()
    }

    /// Deserialize a manifest from JSON bytes.
    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}
