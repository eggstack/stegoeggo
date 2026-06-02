//! Core traits for the protection system.
//!
//! This module defines the fundamental trait that all protectors must implement:
//! - [`Protector`] - Main trait for applying protection to images
//!
//! Implementors of this trait can be composed in a pipeline to apply
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
    ///
    /// Some protectors (e.g., [`MetadataTrapProtector`](crate::MetadataTrapProtector))
    /// operate at the byte level only and their `apply()` may return the image
    /// unchanged. For full protection, callers should use
    /// [`apply_bytes`](Self::apply_bytes).
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

    /// Returns true if this protector only operates at the byte level.
    /// When true, `apply()` may return the image unchanged, and callers
    /// should use `apply_bytes()` for full protection.
    fn requires_bytes_level(&self) -> bool {
        false
    }
}
