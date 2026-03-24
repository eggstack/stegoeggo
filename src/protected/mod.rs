//! Protection modules for applying various protection strategies to images.
//!
//! This module contains the core protector implementations:
//! - [`NoiseProtector`] - Applies adversarial noise perturbations
//! - [`EnhancedProtector`] - Enhanced noise with higher intensity
//! - [`SteganographyProtector`] - LSB/DCT steganographic embedding
//! - [`MetadataTrapProtector`] - Metadata injection for AI exclusion
//! - [`PrecomputedProtector`] - Precomputed variants for CDN edge deployment
//! - [`PassthroughProtector`] - No-op protector for disabled protection

pub mod constants;
pub mod enhanced;
pub mod metadata_trap;
pub mod noise;
pub mod passthrough;
pub mod precomputed;
pub mod steganography;

// Re-exports are handled at the crate root level in lib.rs
