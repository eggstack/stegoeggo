//! Detached signed manifest support for provenance evidence.

#![cfg_attr(docsrs, feature(doc_cfg))]

/// Generate detached manifests from image bytes and claims.
pub mod generate;
mod manifest;
/// Verification of detached manifests against image bytes.
pub mod verify;

pub use generate::{compute_image_digest, create_manifest_from_image, create_manifest_with_claim};
pub use manifest::*;
pub use verify::{
    verify_detached_manifest, verify_detached_manifest_with_keys,
    verify_detached_manifest_with_limits, EmbeddedReferenceStatus, ManifestVerification,
    TrustCallbackFn, TrustPolicy,
};
