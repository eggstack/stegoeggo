//! Core traits for the protection system.
//!
//! This module defines the fundamental traits that all protectors must implement:
//! - [`Protector`] - Main trait for applying protection to images
//! - [`VariantLoader`] - Trait for loading/storing precomputed variants
//!
//! Implementors of these traits can be composed in a pipeline to apply
//! multiple layers of protection to images.

use crate::error::Result;
use crate::types::{ImageOutputFormat, ProtectionContext, ProtectionLevel, DEFAULT_OUTPUT_FORMAT};
use image::DynamicImage;
use image::ImageEncoder;
use std::borrow::Cow;

/// Strategy trait for image protection.
///
/// All protection strategies implement this trait. The pipeline selects
/// the appropriate protector based on the requested [`ProtectionLevel`].
pub trait Protector: Send + Sync {
    /// Apply protection to an image, returning either the original (borrowed)
    /// or a new owned image.
    fn apply<'a>(
        &self,
        img: &'a DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>>;

    /// Apply protection to raw image bytes.
    ///
    /// Default implementation decodes, calls [`apply`](Self::apply), and re-encodes.
    /// Override for byte-level optimizations (e.g., JPEG DCT fast path).
    fn apply_bytes(&self, img_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>> {
        if !self.modifies_pixels() {
            return Ok(img_bytes.to_vec());
        }

        let format = ctx.input_format().unwrap_or_else(|| {
            ImageOutputFormat::from_magic_bytes(img_bytes).unwrap_or(DEFAULT_OUTPUT_FORMAT)
        });

        let img = image::load_from_memory(img_bytes)?;
        let processed = self.apply(&img, ctx)?;

        let quality = ctx.jpeg_quality();

        let mut bytes = Vec::new();
        match format {
            ImageOutputFormat::Png => {
                let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
                encoder.write_image(
                    &processed.to_rgba8(),
                    processed.width(),
                    processed.height(),
                    image::ExtendedColorType::Rgba8,
                )?;
            }
            ImageOutputFormat::Jpeg => {
                let encoder =
                    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality);
                encoder.write_image(
                    &processed.to_rgb8(),
                    processed.width(),
                    processed.height(),
                    image::ExtendedColorType::Rgb8,
                )?;
            }
            ImageOutputFormat::WebP => {
                let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut bytes);
                encoder.write_image(
                    &processed.to_rgba8(),
                    processed.width(),
                    processed.height(),
                    image::ExtendedColorType::Rgba8,
                )?;
            }
        }
        Ok(bytes)
    }

    /// Human-readable name for this protector (e.g., "noise", "steganography").
    fn name(&self) -> &'static str;

    /// The protection level this protector implements.
    fn protection_level(&self) -> ProtectionLevel;

    /// Estimated latency in milliseconds for this protector.
    fn estimated_latency_ms(&self) -> u32;

    /// Whether this protector is currently active.
    fn is_enabled(&self) -> bool {
        true
    }

    /// Returns true if this protector modifies pixel data.
    /// When false, `apply_bytes` returns the input unchanged.
    fn modifies_pixels(&self) -> bool {
        true
    }
}

/// Trait for persistent storage of precomputed protection variants.
///
/// Implement this to back [`PrecomputedProtector`](crate::PrecomputedProtector)
/// with Redis, a database, filesystem, or any other storage backend.
pub trait VariantLoader: Send + Sync {
    /// Load a variant by its cache key. Returns `None` if not found.
    fn load_variant(&self, key: &str) -> Result<Option<crate::types::ProtectedVariant>>;

    /// Persist a variant for later retrieval.
    fn store_variant(&self, variant: &crate::types::ProtectedVariant) -> Result<()>;
}

/// No-op variant loader that always returns `None` and discards stores.
///
/// Used as the default when no persistent storage is configured.
pub struct NoOpLoader;

impl VariantLoader for NoOpLoader {
    fn load_variant(&self, _key: &str) -> Result<Option<crate::types::ProtectedVariant>> {
        Ok(None)
    }

    fn store_variant(&self, _variant: &crate::types::ProtectedVariant) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_loader_returns_none() {
        let loader = NoOpLoader;
        let result = loader.load_variant("any_key").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn noop_store_returns_ok() {
        let loader = NoOpLoader;
        let variant = crate::types::ProtectedVariant::new(
            "hash".to_string(),
            crate::types::ProtectionLevel::Strong,
            vec![],
            0.5,
            10,
            10,
        );
        let result = loader.store_variant(&variant);
        assert!(result.is_ok());
    }
}
