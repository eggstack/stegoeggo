use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Schema version for provenance claims.
pub const PROVENANCE_CLAIM_VERSION: u8 = 1;

/// A canonical provenance claim shared by embedded and detached evidence.
///
/// This is the serializable type that represents a rights/provenance
/// assertion about an image. It is used in both embedded payloads and
/// detached manifests.
///
/// Fields are declared in lexicographic key order to ensure
/// `serde_json::to_string` produces canonical output directly.
/// The `claim_id` field is excluded from canonical serialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceClaim {
    /// Unique claim identifier (16 bytes random, hex-encoded).
    #[serde(
        serialize_with = "serialize_claim_id",
        deserialize_with = "deserialize_claim_id"
    )]
    pub claim_id: [u8; 16],
    /// Perceptual/content identifier (`"iscc:<hex>"` or `"local:<hex>"`).
    pub content_code: String,
    /// Unix epoch seconds when this claim was created.
    pub created_at: u64,
    /// File size in bytes.
    pub file_size: u64,
    /// Image format (`"png"`, `"jpeg"`, `"webp"`).
    pub format: String,
    /// Image height in pixels.
    pub height: u32,
    /// SHA-256 of the exact file bytes (`"sha256:<hex>"`).
    pub instance_digest: String,
    /// Base64url-encoded issuer or key identifier.
    pub issuer_id: String,
    /// SHA-256 of the normalized rights-notice text (`"sha256:<hex>"`).
    pub notice_digest: String,
    /// Base64url-encoded parent claim ID for claim chains.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_claim_id: Option<String>,
    /// Rights/data-mining policy discriminant byte.
    pub rights_policy: u8,
    /// Schema version (currently 1).
    pub schema_version: u8,
    /// Software identifier and version (e.g. `"stegoeggo/0.5.0"`).
    pub software: String,
    /// URI to an external rights statement or license.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_uri: Option<String>,
    /// Image width in pixels.
    pub width: u32,
}

fn serialize_claim_id<S: serde::Serializer>(
    id: &[u8; 16],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let s = hex::encode(id);
    serializer.serialize_str(&s)
}

fn deserialize_claim_id<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<[u8; 16], D::Error> {
    let s = String::deserialize(deserializer)?;
    let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
    if bytes.len() != 16 {
        return Err(serde::de::Error::custom("claim_id must be 16 bytes"));
    }
    let mut id = [0u8; 16];
    id.copy_from_slice(&bytes);
    Ok(id)
}

impl ProvenanceClaim {
    /// Creates a new claim with a random claim ID.
    #[must_use]
    pub fn new(rights_policy: u8) -> Self {
        Self {
            schema_version: PROVENANCE_CLAIM_VERSION,
            claim_id: Self::random_claim_id(),
            rights_policy,
            notice_digest: String::new(),
            content_code: String::new(),
            instance_digest: String::new(),
            format: String::new(),
            width: 0,
            height: 0,
            file_size: 0,
            created_at: 0,
            issuer_id: String::new(),
            software: String::new(),
            parent_claim_id: None,
            statement_uri: None,
        }
    }

    /// Sets the notice digest by computing SHA-256 of the normalized notice text.
    #[must_use]
    pub fn with_notice_digest(mut self, notice_text: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(notice_text);
        let hash: [u8; 32] = hasher.finalize().into();
        self.notice_digest = format!("sha256:{}", hex::encode(hash));
        self
    }

    /// Sets the notice digest from a pre-formatted `"algorithm:hex"` string.
    #[must_use]
    pub fn with_notice_digest_raw(mut self, digest: String) -> Self {
        self.notice_digest = digest;
        self
    }

    /// Sets the content code (e.g. `"iscc:<hex>"` or `"local:<hex>"`).
    #[must_use]
    pub fn with_content_code(mut self, code: String) -> Self {
        self.content_code = code;
        self
    }

    /// Sets the instance digest by computing SHA-256 of the exact file bytes.
    #[must_use]
    pub fn with_instance_digest(mut self, file_bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(file_bytes);
        let hash: [u8; 32] = hasher.finalize().into();
        self.instance_digest = format!("sha256:{}", hex::encode(hash));
        self
    }

    /// Sets the instance digest from a pre-formatted `"sha256:<hex>"` string.
    #[must_use]
    pub fn with_instance_digest_raw(mut self, digest: String) -> Self {
        self.instance_digest = digest;
        self
    }

    /// Sets image source facts.
    #[must_use]
    pub fn with_source_facts(
        mut self,
        format: &str,
        width: u32,
        height: u32,
        file_size: u64,
    ) -> Self {
        self.format = format.to_string();
        self.width = width;
        self.height = height;
        self.file_size = file_size;
        self
    }

    /// Sets the creation time (Unix epoch seconds).
    #[must_use]
    pub fn with_creation_time(mut self, time: u64) -> Self {
        self.created_at = time;
        self
    }

    /// Sets the issuer/key identifier (base64url-encoded).
    #[must_use]
    pub fn with_issuer_id(mut self, issuer_id: String) -> Self {
        self.issuer_id = issuer_id;
        self
    }

    /// Sets the software identifier and version.
    #[must_use]
    pub fn with_software(mut self, software: &str) -> Self {
        self.software = software.to_string();
        self
    }

    /// Sets an optional parent claim reference (base64url-encoded).
    #[must_use]
    pub fn with_parent_claim(mut self, parent_id: String) -> Self {
        self.parent_claim_id = Some(parent_id);
        self
    }

    /// Sets an optional external statement URI.
    #[must_use]
    pub fn with_statement_uri(mut self, uri: &str) -> Self {
        self.statement_uri = Some(uri.to_string());
        self
    }

    /// Compute canonical JSON bytes for this claim.
    ///
    /// Produces deterministic JSON with sorted keys, no whitespace,
    /// null omission, and the `claim_id` field excluded. This is the
    /// form used for hashing and signing.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut map = serde_json::Map::new();
        map.insert(
            "content_code".into(),
            serde_json::Value::String(self.content_code.clone()),
        );
        map.insert(
            "created_at".into(),
            serde_json::Value::Number(self.created_at.into()),
        );
        map.insert(
            "file_size".into(),
            serde_json::Value::Number(self.file_size.into()),
        );
        map.insert(
            "format".into(),
            serde_json::Value::String(self.format.clone()),
        );
        map.insert(
            "height".into(),
            serde_json::Value::Number(self.height.into()),
        );
        map.insert(
            "instance_digest".into(),
            serde_json::Value::String(self.instance_digest.clone()),
        );
        map.insert(
            "issuer_id".into(),
            serde_json::Value::String(self.issuer_id.clone()),
        );
        map.insert(
            "notice_digest".into(),
            serde_json::Value::String(self.notice_digest.clone()),
        );
        if let Some(ref parent) = self.parent_claim_id {
            map.insert(
                "parent_claim_id".into(),
                serde_json::Value::String(parent.clone()),
            );
        }
        map.insert(
            "rights_policy".into(),
            serde_json::Value::Number(self.rights_policy.into()),
        );
        map.insert(
            "schema_version".into(),
            serde_json::Value::Number(self.schema_version.into()),
        );
        map.insert(
            "software".into(),
            serde_json::Value::String(self.software.clone()),
        );
        if let Some(ref uri) = self.statement_uri {
            map.insert(
                "statement_uri".into(),
                serde_json::Value::String(uri.clone()),
            );
        }
        map.insert("width".into(), serde_json::Value::Number(self.width.into()));
        let canonical = serde_json::Value::Object(map);
        serde_json::to_string(&canonical)
            .expect("provenance claim canonical serialization failed")
            .into_bytes()
    }

    /// Compute SHA-256 digest of canonical bytes.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.canonical_bytes());
        hasher.finalize().into()
    }

    /// Create a claim ID from random bytes.
    #[must_use]
    pub fn random_claim_id() -> [u8; 16] {
        let mut id = [0u8; 16];
        getrandom(&mut id).expect("failed to generate random claim ID");
        id
    }
}
