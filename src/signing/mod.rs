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
