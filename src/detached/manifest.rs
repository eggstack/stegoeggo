use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

use crate::provenance::ProvenanceClaim;
use crate::resource_limits::ResourceLimits;

/// Current detached manifest schema version.
pub const MANIFEST_SCHEMA_VERSION: u8 = 1;
/// Maximum manifest size in bytes.
pub const MAX_MANIFEST_SIZE: usize = 64 * 1024;
/// Maximum number of signature records.
pub const MAX_SIGNATURES: usize = 16;
/// Maximum number of public key entries.
pub const MAX_PUBLIC_KEYS: usize = 16;
/// Maximum key identifier length in bytes.
pub const MAX_KEY_ID_LEN: usize = 64;

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
    /// Hex-encoded public key bytes.
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
    ///
    /// Silently skips if a signature with the same (algorithm, key_id) already exists.
    #[must_use]
    pub fn with_signature(mut self, sig: SignatureRecord) -> Self {
        if self.signatures.len() < MAX_SIGNATURES {
            let is_dup = self
                .signatures
                .iter()
                .any(|s| s.algorithm == sig.algorithm && s.key_id == sig.key_id);
            if !is_dup {
                self.signatures.push(sig);
            }
        }
        self
    }

    /// Add a public key entry (up to [`MAX_PUBLIC_KEYS`]).
    ///
    /// Silently skips if a public key with the same key_id already exists.
    #[must_use]
    pub fn with_public_key(mut self, key: PublicKeyEntry) -> Self {
        if self.public_keys.len() < MAX_PUBLIC_KEYS {
            let is_dup = self.public_keys.iter().any(|k| k.key_id == key.key_id);
            if !is_dup {
                self.public_keys.push(key);
            }
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

    /// Validate the manifest structure.
    ///
    /// Checks for duplicate keys, duplicate signatures, empty key IDs,
    /// valid hex encodings, correct byte lengths, and that each signature
    /// references an existing public key.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check for empty key IDs and validate key_bytes hex/length
        for (i, key) in self.public_keys.iter().enumerate() {
            if key.key_id.is_empty() {
                errors.push(format!("Public key {} has empty key ID", i));
            }
            match hex::decode(&key.key_bytes) {
                Ok(bytes) => {
                    if bytes.len() != 32 {
                        errors.push(format!(
                            "Public key {} has invalid byte length {} (expected 32)",
                            i,
                            bytes.len()
                        ));
                    }
                }
                Err(e) => {
                    errors.push(format!("Public key {} has invalid hex: {}", i, e));
                }
            }
        }

        // Check for duplicate public key IDs
        let mut seen_key_ids = HashSet::new();
        for key in &self.public_keys {
            if !seen_key_ids.insert(&key.key_id) {
                errors.push(format!(
                    "Duplicate public key ID: {:?}",
                    String::from_utf8_lossy(&key.key_id)
                ));
            }
        }

        // Validate signatures
        for (i, sig) in self.signatures.iter().enumerate() {
            if sig.key_id.is_empty() {
                errors.push(format!("Signature {} has empty key ID", i));
            }
            match hex::decode(&sig.signature) {
                Ok(bytes) => {
                    if bytes.len() != 64 {
                        errors.push(format!(
                            "Signature {} has invalid byte length {} (expected 64 for ed25519)",
                            i,
                            bytes.len()
                        ));
                    }
                }
                Err(e) => {
                    errors.push(format!("Signature {} has invalid hex: {}", i, e));
                }
            }
            // Check that signature has a matching public key
            if !self.public_keys.iter().any(|k| k.key_id == sig.key_id) {
                errors.push(format!(
                    "Signature {} references unknown key ID {:?}",
                    i,
                    String::from_utf8_lossy(&sig.key_id)
                ));
            }
        }

        // Check for duplicate signature identities (algorithm + key_id)
        let mut seen_sigs = HashSet::new();
        for sig in &self.signatures {
            let identity = (sig.algorithm.clone(), sig.key_id.clone());
            if !seen_sigs.insert(identity) {
                errors.push(format!(
                    "Duplicate signature for ({}, {:?})",
                    sig.algorithm,
                    String::from_utf8_lossy(&sig.key_id)
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Serialize the manifest to canonical JSON bytes.
    ///
    /// Produces deterministic JSON with sorted keys and no whitespace.
    /// This is used for digest computation. Signing uses the claim-level
    /// canonical bytes, not the full manifest.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut map = serde_json::Map::new();
        map.insert("claim".into(), {
            let mut claim_map = serde_json::Map::new();
            claim_map.insert(
                "claim_id".into(),
                serde_json::Value::String(hex::encode(self.claim.claim_id)),
            );
            claim_map.insert(
                "content_code".into(),
                serde_json::Value::String(self.claim.content_code.clone()),
            );
            claim_map.insert(
                "created_at".into(),
                serde_json::Value::Number(self.claim.created_at.into()),
            );
            claim_map.insert(
                "file_size".into(),
                serde_json::Value::Number(self.claim.file_size.into()),
            );
            claim_map.insert(
                "format".into(),
                serde_json::Value::String(self.claim.format.clone()),
            );
            claim_map.insert(
                "height".into(),
                serde_json::Value::Number(self.claim.height.into()),
            );
            claim_map.insert(
                "instance_digest".into(),
                serde_json::Value::String(self.claim.instance_digest.clone()),
            );
            claim_map.insert(
                "issuer_id".into(),
                serde_json::Value::String(self.claim.issuer_id.clone()),
            );
            claim_map.insert(
                "notice_digest".into(),
                serde_json::Value::String(self.claim.notice_digest.clone()),
            );
            if let Some(ref parent) = self.claim.parent_claim_id {
                claim_map.insert(
                    "parent_claim_id".into(),
                    serde_json::Value::String(parent.clone()),
                );
            }
            claim_map.insert(
                "rights_policy".into(),
                serde_json::Value::Number(self.claim.rights_policy.into()),
            );
            claim_map.insert(
                "schema_version".into(),
                serde_json::Value::Number(self.claim.schema_version.into()),
            );
            claim_map.insert(
                "software".into(),
                serde_json::Value::String(self.claim.software.clone()),
            );
            if let Some(ref uri) = self.claim.statement_uri {
                claim_map.insert(
                    "statement_uri".into(),
                    serde_json::Value::String(uri.clone()),
                );
            }
            claim_map.insert(
                "width".into(),
                serde_json::Value::Number(self.claim.width.into()),
            );
            serde_json::Value::Object(claim_map)
        });
        map.insert(
            "embedded_reference".into(),
            match &self.embedded_reference {
                Some(r) => {
                    let mut m = serde_json::Map::new();
                    m.insert(
                        "payload_digest".into(),
                        serde_json::Value::String(r.payload_digest.clone()),
                    );
                    m.insert(
                        "payload_version".into(),
                        serde_json::Value::Number(r.payload_version.into()),
                    );
                    serde_json::Value::Object(m)
                }
                None => serde_json::Value::Null,
            },
        );
        map.insert(
            "public_keys".into(),
            serde_json::json!(self
                .public_keys
                .iter()
                .map(|k| {
                    let mut m = serde_json::Map::new();
                    m.insert(
                        "algorithm".into(),
                        serde_json::Value::String(k.algorithm.clone()),
                    );
                    m.insert(
                        "key_bytes".into(),
                        serde_json::Value::String(k.key_bytes.clone()),
                    );
                    m.insert("key_id".into(), serde_json::json!(k.key_id));
                    serde_json::Value::Object(m)
                })
                .collect::<Vec<_>>()),
        );
        map.insert(
            "schema_version".into(),
            serde_json::Value::Number(self.schema_version.into()),
        );
        map.insert(
            "signatures".into(),
            serde_json::json!(self
                .signatures
                .iter()
                .map(|s| {
                    let mut m = serde_json::Map::new();
                    m.insert(
                        "algorithm".into(),
                        serde_json::Value::String(s.algorithm.clone()),
                    );
                    m.insert("key_id".into(), serde_json::json!(s.key_id));
                    m.insert(
                        "signature".into(),
                        serde_json::Value::String(s.signature.clone()),
                    );
                    serde_json::Value::Object(m)
                })
                .collect::<Vec<_>>()),
        );
        if let Some(ref trust) = self.trust_metadata {
            let mut trust_map = serde_json::Map::new();
            trust_map.insert(
                "trust_model".into(),
                serde_json::Value::String(trust.trust_model.clone()),
            );
            trust_map.insert("trusted".into(), serde_json::Value::Bool(trust.trusted));
            trust_map.insert(
                "reason".into(),
                serde_json::Value::String(trust.reason.clone()),
            );
            if let Some(ref chain) = trust.certificate_chain {
                trust_map.insert("certificate_chain".into(), serde_json::json!(chain));
            }
            map.insert(
                "trust_metadata".into(),
                serde_json::Value::Object(trust_map),
            );
        }
        let canonical = serde_json::Value::Object(map);
        serde_json::to_string(&canonical)
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
    ///
    /// Validates bounded resource limits before and after deserialization:
    /// - Maximum manifest size ([`MAX_MANIFEST_SIZE`])
    /// - Schema version must be exactly [`MANIFEST_SCHEMA_VERSION`]
    /// - Maximum signatures ([`MAX_SIGNATURES`])
    /// - Maximum public keys ([`MAX_PUBLIC_KEYS`])
    /// - Maximum key ID length ([`MAX_KEY_ID_LEN`])
    pub fn from_json(bytes: &[u8]) -> Result<Self, crate::Error> {
        if bytes.len() > MAX_MANIFEST_SIZE {
            return Err(crate::Error::Config(format!(
                "Manifest size {} exceeds maximum {}",
                bytes.len(),
                MAX_MANIFEST_SIZE
            )));
        }

        let manifest: Self = serde_json::from_slice(bytes)?;

        if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err(crate::Error::Config(format!(
                "Unsupported manifest schema version {} (expected {})",
                manifest.schema_version, MANIFEST_SCHEMA_VERSION
            )));
        }

        if manifest.signatures.len() > MAX_SIGNATURES {
            return Err(crate::Error::Config(format!(
                "Signature count {} exceeds maximum {}",
                manifest.signatures.len(),
                MAX_SIGNATURES
            )));
        }

        if manifest.public_keys.len() > MAX_PUBLIC_KEYS {
            return Err(crate::Error::Config(format!(
                "Public key count {} exceeds maximum {}",
                manifest.public_keys.len(),
                MAX_PUBLIC_KEYS
            )));
        }

        for key in &manifest.public_keys {
            if key.key_id.len() > MAX_KEY_ID_LEN {
                return Err(crate::Error::Config(format!(
                    "Key ID length {} exceeds maximum {}",
                    key.key_id.len(),
                    MAX_KEY_ID_LEN
                )));
            }
        }

        for sig in &manifest.signatures {
            if sig.key_id.len() > MAX_KEY_ID_LEN {
                return Err(crate::Error::Config(format!(
                    "Signature key ID length {} exceeds maximum {}",
                    sig.key_id.len(),
                    MAX_KEY_ID_LEN
                )));
            }
        }

        manifest.validate().map_err(|errors| {
            crate::Error::Config(format!("Manifest validation failed: {}", errors.join("; ")))
        })?;

        Ok(manifest)
    }

    /// Deserialize a manifest from JSON bytes with explicit resource limits.
    ///
    /// Like [`from_json`](Self::from_json), but uses the provided [`ResourceLimits`]
    /// for the maximum manifest size instead of the hardcoded default.
    pub fn from_json_with_limits(
        bytes: &[u8],
        limits: &ResourceLimits,
    ) -> Result<Self, crate::Error> {
        if bytes.len() > limits.max_detached_manifest_bytes() {
            return Err(crate::Error::Config(format!(
                "Manifest size {} exceeds maximum {}",
                bytes.len(),
                limits.max_detached_manifest_bytes()
            )));
        }

        let manifest: Self = serde_json::from_slice(bytes)?;

        if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err(crate::Error::Config(format!(
                "Unsupported manifest schema version {} (expected {})",
                manifest.schema_version, MANIFEST_SCHEMA_VERSION
            )));
        }

        if manifest.signatures.len() > MAX_SIGNATURES {
            return Err(crate::Error::Config(format!(
                "Signature count {} exceeds maximum {}",
                manifest.signatures.len(),
                MAX_SIGNATURES
            )));
        }

        if manifest.public_keys.len() > MAX_PUBLIC_KEYS {
            return Err(crate::Error::Config(format!(
                "Public key count {} exceeds maximum {}",
                manifest.public_keys.len(),
                MAX_PUBLIC_KEYS
            )));
        }

        for key in &manifest.public_keys {
            if key.key_id.len() > MAX_KEY_ID_LEN {
                return Err(crate::Error::Config(format!(
                    "Key ID length {} exceeds maximum {}",
                    key.key_id.len(),
                    MAX_KEY_ID_LEN
                )));
            }
        }

        for sig in &manifest.signatures {
            if sig.key_id.len() > MAX_KEY_ID_LEN {
                return Err(crate::Error::Config(format!(
                    "Signature key ID length {} exceeds maximum {}",
                    sig.key_id.len(),
                    MAX_KEY_ID_LEN
                )));
            }
        }

        manifest.validate().map_err(|errors| {
            crate::Error::Config(format!("Manifest validation failed: {}", errors.join("; ")))
        })?;

        Ok(manifest)
    }
}
