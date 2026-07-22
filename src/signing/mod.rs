//! Ed25519 signing support for provenance claims.
//!
//! This module provides Ed25519 signing and verification behind the `signatures` feature flag.
//! When the feature is disabled, this module is empty and signing functionality are unavailable.

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "signatures")]
mod ed25519_impl;

#[cfg(feature = "signatures")]
/// Signing configuration bundling key material, key identifier, and placement preference.
pub mod config;

#[cfg(feature = "signatures")]
pub use ed25519_impl::*;

#[cfg(feature = "signatures")]
pub use config::{SignaturePlacement, SigningConfig};

/// Byte overhead for embedding an Ed25519 signature in a v3 payload.
///
/// This includes:
/// - 64-byte authentication tag (the signature itself)
/// - 36-byte `Ed25519PublicKey` extension (2 type + 2 len + 32 key)
/// - 68-byte `Ed25519DetachedSig` extension (2 type + 2 len + 64 sig)
///
/// Total: 168 bytes of overhead beyond the core header and key ID.
#[cfg(feature = "signatures")]
pub const ED25519_OVERHEAD_BYTES: usize = 64 + 36 + 68;

/// Result of checking whether an Ed25519 signature fits within payload capacity.
#[cfg(feature = "signatures")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureCapacity {
    /// The signature fits; use embedded placement.
    FitsEmbedded,
    /// The signature does not fit; use detached placement.
    NeedsDetached,
}

/// Check whether an Ed25519 signature can fit within the available payload capacity.
///
/// # Arguments
///
/// * `available_bytes` — Total byte budget for the payload (header + key ID + extensions + tag).
/// * `key_id_len` — Length of the key identifier in bytes (0–32).
/// * `placement` — The caller's placement preference.
///
/// # Returns
///
/// - `FitsEmbedded` if the capacity is sufficient for embedded placement.
/// - `NeedsDetached` if the capacity is insufficient.
/// - The original placement if it was `Detached` (always returns `Detached`).
#[cfg(feature = "signatures")]
#[must_use]
pub fn check_signature_capacity(
    available_bytes: usize,
    key_id_len: usize,
    placement: SignaturePlacement,
) -> SignatureCapacity {
    match placement {
        SignaturePlacement::Detached => SignatureCapacity::NeedsDetached,
        SignaturePlacement::Embedded | SignaturePlacement::PreferredEmbedded => {
            let needed =
                crate::payload_v3::types::V3_CORE_SIZE + key_id_len + ED25519_OVERHEAD_BYTES;
            if available_bytes >= needed {
                SignatureCapacity::FitsEmbedded
            } else {
                SignatureCapacity::NeedsDetached
            }
        }
    }
}
