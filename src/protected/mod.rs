//! Protection modules for applying various protection strategies to images.
//!
//! This module contains the core protector implementations:
//! - [`SteganographyProtector`] - LSB/DCT steganographic embedding
//! - [`MetadataTrapProtector`] - Metadata injection for AI exclusion
//! - [`PassthroughProtector`] - No-op protector for disabled protection

pub mod constants;
pub(crate) mod ecc;
pub mod metadata_trap;
pub(crate) mod notice_verification;
pub mod passthrough;
pub(crate) mod resolve;
pub mod steganography;
#[cfg(feature = "fuzz")]
pub(crate) mod stego_cost;

// Re-exports are handled at the crate root level in lib.rs
