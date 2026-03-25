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

pub trait Protector: Send + Sync {
    fn apply<'a>(
        &self,
        img: &'a DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>>;

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

    fn name(&self) -> &'static str;

    fn protection_level(&self) -> ProtectionLevel;

    fn estimated_latency_ms(&self) -> u32;

    fn is_enabled(&self) -> bool {
        true
    }

    /// Returns true if this protector modifies pixel data.
    /// When false, `apply_bytes` returns the input unchanged.
    fn modifies_pixels(&self) -> bool {
        true
    }
}

pub trait VariantLoader: Send + Sync {
    fn load_variant(&self, key: &str) -> Result<Option<crate::types::ProtectedVariant>>;

    fn store_variant(&self, variant: &crate::types::ProtectedVariant) -> Result<()>;
}

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
