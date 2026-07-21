//! Provenance claim model for rights/provenance assertions about images.
//!
//! Provides a canonical, deterministic serializable type shared by
//! embedded payloads and detached manifests.

mod canonical;
mod claim;
mod digest;

pub use canonical::{canonical_json, verify_canonical_stability};
pub use claim::{ProvenanceClaim, PROVENANCE_CLAIM_VERSION};
pub use digest::TypedDigest;
