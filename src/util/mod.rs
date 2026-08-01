//! Utility modules for image processing and metadata.
//!
//! This module contains:
//! - [`image`] - Image encoding, decoding, and perturbation utilities
//! - [`iscc`] - Image Signature Calculus Computation for content identification
//! - [`seed`] - Random seed generation

pub mod image;
#[cfg(feature = "iscc")]
pub mod iscc;
pub mod seed;
