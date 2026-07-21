use serde::{Deserialize, Serialize};
use sha2::Digest;

/// A typed digest with algorithm identifier.
///
/// Encodes digests as `"algorithm:hex"` strings (e.g. `"sha256:e3b0c4..."`)
/// so consumers never guess the hash function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypedDigest {
    /// Algorithm identifier (e.g. `"sha256"`, `"iscc"`, `"local"`).
    pub algorithm: String,
    /// Hex-encoded digest value.
    pub value: String,
}

impl TypedDigest {
    /// Creates a SHA-256 digest from bytes.
    #[must_use]
    pub fn sha256(data: &[u8]) -> Self {
        let hash = sha2::Sha256::digest(data);
        Self {
            algorithm: "sha256".into(),
            value: hex::encode(hash),
        }
    }

    /// Creates an ISCC code digest (8-byte truncated content code).
    #[must_use]
    pub fn iscc(code: &[u8]) -> Self {
        Self {
            algorithm: "iscc".into(),
            value: hex::encode(code),
        }
    }

    /// Creates a project-local fingerprint digest.
    #[must_use]
    pub fn local_fingerprint(data: &[u8]) -> Self {
        Self {
            algorithm: "local".into(),
            value: hex::encode(data),
        }
    }

    /// Parses an `"algorithm:value"` string.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let (algorithm, value) = s.split_once(':')?;
        if algorithm.is_empty() || value.is_empty() {
            return None;
        }
        Some(Self {
            algorithm: algorithm.to_string(),
            value: value.to_string(),
        })
    }

    /// Format as `"algorithm:value"`.
    #[must_use]
    pub fn to_string_value(&self) -> String {
        format!("{}:{}", self.algorithm, self.value)
    }
}
