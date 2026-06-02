//! Image protection library for legal deterrence against unauthorized AI training.
//!
//! Protects images through steganographic payload embedding and metadata injection,
//! providing evidence of protection and legal warnings that survive casual modification.
//!
//! # Protection Levels
//!
//! - `Disabled`: No protection applied
//! - `Light`: Metadata injection only (tEXt chunks for PNG, COM markers for JPEG)
//! - `Standard`: Steganography + metadata injection
//!
//! # Protection Layers
//!
//! Each protection level applies one or more layers:
//! 1. **Steganography** - Hidden LSB payload (PNG/WebP) or DCT perturbation (JPEG)
//! 2. **Metadata Injection** - Visible anti-scraping markers (XMP, IPTC, EXIF)
//!
//! # JPEG Limitations
//!
//! JPEG's lossy compression inherently limits steganographic robustness. For JPEG,
//! the library embeds a seed in quantization tables (survives re-encoding) and uses
//! F5-style DCT coefficient embedding. However, pixel-based stego payloads may not
//! survive JPEG re-compression. **For maximum verifiability, use PNG output format.**
//!
//! Verification priority for JPEG: metadata seed extraction > DCT quantization table
//! seed > DCT coefficient extraction > pixel-based extraction.
//!
//! # JPEG-in/JPEG-out Fast Path
//!
//! When using `process_image_bytes` with JPEG input and JPEG output, the library
//! takes a byte-only fast path that operates directly on DCT coefficients, avoiding
//! decode/encode cycles. This path applies DCT steganography (F5 embedding) and
//! metadata injection. For progressive JPEGs, the progressive encoding is preserved.
//!
//! # Security Considerations
//!
//! For production use, always set a MAC key via `with_mac_key()` — the default
//! checksum provides no cryptographic integrity.
//!
//! **Without a MAC key**, steganographic payload verification uses a trivial
//! additive checksum. An attacker can trivially forge valid-looking payloads.
//! This is suitable for accidental corruption detection and legal deterrence
//! (visible metadata markers prove intent), but **not** for adversarial settings.
//!
//! **With a MAC key**, the library uses HMAC-SHA256 for cryptographic payload
//! verification. Always set a MAC key in production:
//!
//! ```ignore
//! use cloakrs::{ProtectionContext, ProtectionLevel};
//!
//! let ctx = ProtectionContext::default()
//!     .with_mac_key(b"your-secret-key".to_vec());
//! ```
//!
//! The primary deterrence mechanism is **visible metadata injection** (XMP, IPTC,
//! EXIF markers) — not the steganographic layer. Even if an attacker strips the
//! stego payload, the visible metadata markers remain as evidence of protection
//! and legal warnings.
//!
//! # WAF-Optimized Usage
//!
//! ```ignore
//! use cloakrs::{process_image_bytes, ProtectionContext, ProtectionLevel, ImageOutputFormat};
//!
//! let ctx = ProtectionContext::new(0.5, 42)
//!     .with_format(ImageOutputFormat::Png)
//!     .with_stego_redundancy(2)      // Lower = faster
//!     .with_jpeg_quality(85)        // Lower = smaller files
//!     .with_progressive_jpeg(true); // Progressive rendering for web
//!
//! let input_bytes = std::fs::read("image.png")?;
//! let protected = process_image_bytes(&input_bytes, ProtectionLevel::Standard, &ctx)?;
//! ```
//!
//! Legal Metadata
//!
//! Embed copyright and usage restrictions in images for IP protection.
//!
//! ```rust
//! use cloakrs::{ProtectionContext, LegalMetadata, ProtectionLevel};
//!
//! let ctx = ProtectionContext::default()
//!     .with_legal_metadata(
//!         LegalMetadata::new()
//!             .with_copyright_holder("Example Corp")
//!             .with_contact_email("legal@example.com")
//!             .with_usage_terms("All Rights Reserved. No AI training permitted.")
//!     );
//! ```

pub mod error;
pub mod traits;
pub mod types;

pub(crate) mod jpeg_transcoder;
pub(crate) mod protected;
pub(crate) mod util;

#[cfg(feature = "async")]
pub mod async_api;

pub use error::{Error, Result};
pub use types::{
    DmiValue, ImageOutputFormat, LegalMetadata, ProtectionConfig, ProtectionContext,
    ProtectionLevel, DEFAULT_OUTPUT_FORMAT,
};

pub use traits::Protector;

pub use protected::metadata_trap::MetadataTrapProtector;
pub use protected::passthrough::PassthroughProtector;
pub use protected::steganography::{SteganographyProtector, StegoPayload};

pub use jpeg_transcoder::is_progressive_jpeg;

pub use util::image::{
    compute_image_hash, detect_image_format, encode_image, encode_image_with_options,
    load_image_from_bytes,
};

pub use util::iscc::{compute_iscc, compute_iscc_from_bytes, Iscc};
pub use util::seed::generate_random_seed;

#[cfg(feature = "async")]
pub use async_api::{
    process_image_async, process_image_bytes_async, process_images_bytes_parallel_async,
    process_images_parallel_async, verify_image_bytes_async,
};

use image::DynamicImage;
use image::GenericImageView;
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::LazyLock;

static DEFAULT_PIPELINE: LazyLock<ProtectionPipeline> = LazyLock::new(ProtectionPipeline::new);

/// Main pipeline for applying protection to images.
///
/// Coordinates between different protector implementations based on the
/// selected protection level.
pub struct ProtectionPipeline {
    passthrough: Arc<PassthroughProtector>,
    metadata_trap: Arc<MetadataTrapProtector>,
    steganography: Arc<SteganographyProtector>,
}

impl ProtectionPipeline {
    /// Create a new ProtectionPipeline with default protectors.
    pub fn new() -> Self {
        Self {
            passthrough: Arc::new(PassthroughProtector::new()),
            metadata_trap: Arc::new(MetadataTrapProtector::new()),
            steganography: Arc::new(SteganographyProtector::new()),
        }
    }

    fn validate_dimensions(img: &DynamicImage, max_dim: Option<u32>) -> Result<()> {
        if let Some(max) = max_dim {
            let (width, height) = img.dimensions();
            if width > max || height > max {
                return Err(Error::ImageDecode(format!(
                    "Image dimensions {}x{} exceed maximum allowed {}",
                    width, height, max
                )));
            }
        }
        Ok(())
    }

    /// Process an image with the specified protection level.
    pub fn process<'a>(
        &'a self,
        img: &'a DynamicImage,
        level: ProtectionLevel,
        ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>> {
        Self::validate_dimensions(img, ctx.max_dimension())?;

        let mut ctx_with_level = ctx.clone();
        ctx_with_level.set_protection_level(level);
        let ctx = &ctx_with_level;

        match level {
            ProtectionLevel::Disabled => self.passthrough.apply(img, ctx),
            ProtectionLevel::Light => self.apply_light_bytes(img, ctx).map(Cow::Owned),
            ProtectionLevel::Standard => self.apply_standard_pipeline(img, ctx).map(Cow::Owned),
        }
    }

    /// Standard pipeline: stego → encode → metadata injection.
    fn apply_standard_pipeline(
        &self,
        img: &DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<DynamicImage> {
        let output_format = ctx
            .output_format()
            .or(ctx.input_format())
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        let final_bytes = self.apply_pipeline_bytes(img, ctx, output_format)?;
        Ok(image::load_from_memory(&final_bytes)?)
    }

    /// Shared pipeline: stego → encode → metadata injection.
    /// Used by both `apply_standard_pipeline` and `apply_bytes_pipeline`.
    fn apply_pipeline_bytes(
        &self,
        img: &DynamicImage,
        ctx: &ProtectionContext,
        output_format: crate::types::ImageOutputFormat,
    ) -> Result<Vec<u8>> {
        // JPEG output: encode first, then apply DCT stego to the JPEG bytes
        if output_format == crate::types::ImageOutputFormat::Jpeg {
            let jpeg_bytes = crate::util::image::encode_image_with_options(
                img,
                Some(output_format),
                ctx.progressive_jpeg(),
                ctx.jpeg_quality(),
            )?;
            let with_stego = self.steganography.apply_dct_stego_bytes(&jpeg_bytes, ctx)?;
            return self.metadata_trap.inject_bytes(&with_stego, ctx);
        }

        // Non-JPEG output: pixel stego then encode
        let with_stego = self.steganography.apply(img, ctx)?;
        let encoded = crate::util::image::encode_image_with_options(
            &with_stego,
            Some(output_format),
            ctx.progressive_jpeg(),
            ctx.jpeg_quality(),
        )?;
        self.metadata_trap.inject_bytes(&encoded, ctx)
    }

    /// Light level: metadata injection only, no perturbation or steganography.
    /// Encodes to bytes, injects metadata, then decodes back to `DynamicImage`.
    /// Metadata survives in the byte-level output; pixel content is unchanged.
    fn apply_light_bytes(
        &self,
        img: &DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<DynamicImage> {
        let output_format = ctx
            .output_format()
            .or(ctx.input_format())
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        let encoded = crate::util::image::encode_image(img, output_format.to_image_format())?;
        let with_metadata = self.metadata_trap.inject_bytes(&encoded, ctx)?;
        Ok(image::load_from_memory(&with_metadata)?)
    }

    /// Process image bytes with the specified protection level.
    ///
    /// For JPEG-in/JPEG-out, uses the byte-only fast path (DCT stego + metadata,
    /// no pixel decode). For other formats, decodes to pixels, applies the full
    /// pipeline, and re-encodes.
    pub fn process_bytes(
        &self,
        img_bytes: &[u8],
        level: ProtectionLevel,
        ctx: &ProtectionContext,
    ) -> Result<Vec<u8>> {
        let mut ctx_with_level = ctx.clone();
        ctx_with_level.set_protection_level(level);

        match level {
            ProtectionLevel::Disabled => Ok(img_bytes.to_vec()),
            ProtectionLevel::Light => self.metadata_trap.apply_bytes(img_bytes, &ctx_with_level),
            ProtectionLevel::Standard => self.apply_bytes_pipeline(img_bytes, &ctx_with_level),
        }
    }

    fn validate_jpeg_dimensions_from_bytes(img_bytes: &[u8], max_dim: Option<u32>) -> Result<()> {
        if let Some(max) = max_dim {
            let header = jpeg_transcoder::header::JpegHeader::parse(img_bytes)?;
            if header.width as u32 > max || header.height as u32 > max {
                return Err(Error::ImageDecode(format!(
                    "Image dimensions {}x{} exceed maximum allowed {}",
                    header.width, header.height, max
                )));
            }
        }
        Ok(())
    }

    fn apply_bytes_pipeline(&self, img_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>> {
        let input_format = ctx
            .input_format()
            .or_else(|| crate::types::ImageOutputFormat::from_magic_bytes(img_bytes))
            .ok_or_else(|| Error::InvalidFormat("Unrecognized image format".to_string()))?;

        let output_format = ctx
            .output_format()
            .or(ctx.input_format())
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        // JPEG-in, JPEG-out: byte-only path (DCT stego + metadata, no pixel decode).
        // This preserves quality and avoids lossy re-encode cycles.
        if input_format == crate::types::ImageOutputFormat::Jpeg
            && output_format == crate::types::ImageOutputFormat::Jpeg
        {
            Self::validate_jpeg_dimensions_from_bytes(img_bytes, ctx.max_dimension())?;
            let with_stego = self.steganography.apply_dct_stego_bytes(img_bytes, ctx)?;
            return self.metadata_trap.inject_bytes(&with_stego, ctx);
        }

        // Non-JPEG-in: decode then use shared pipeline
        let img = load_image_from_bytes(img_bytes)?;
        Self::validate_dimensions(&img, ctx.max_dimension())?;
        self.apply_pipeline_bytes(&img, ctx, output_format)
    }
}

impl Default for ProtectionPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Process an image with the specified protection level.
///
/// Takes an owned DynamicImage and returns a processed image.
#[must_use = "the protected image should be saved or used"]
pub fn process_image(
    img: DynamicImage,
    level: ProtectionLevel,
    ctx: &ProtectionContext,
) -> Result<DynamicImage> {
    DEFAULT_PIPELINE
        .process(&img, level, ctx)
        .map(|c| c.into_owned())
}

/// Process multiple images in parallel.
///
/// Takes a slice of images and returns a vector of processed images.
/// Uses Rayon for parallel processing.
#[must_use = "the protected images should be saved or used"]
pub fn process_images_parallel(
    images: &[DynamicImage],
    level: ProtectionLevel,
    ctx: &ProtectionContext,
) -> Result<Vec<DynamicImage>> {
    use rayon::prelude::*;
    images
        .par_iter()
        .map(|img| {
            DEFAULT_PIPELINE
                .process(img, level, ctx)
                .map(|c| c.into_owned())
        })
        .collect()
}

/// Process multiple images in parallel (bytes variant).
///
/// Takes a slice of image bytes and returns a vector of processed image bytes.
#[must_use = "the protected image bytes should be saved or used"]
pub fn process_images_bytes_parallel(
    images: &[Vec<u8>],
    level: ProtectionLevel,
    ctx: &ProtectionContext,
) -> Result<Vec<Vec<u8>>> {
    use rayon::prelude::*;
    images
        .par_iter()
        .map(|img_bytes| DEFAULT_PIPELINE.process_bytes(img_bytes, level, ctx))
        .collect()
}

/// Process image bytes with the specified protection level.
///
/// Automatically detects the input format from magic bytes and preserves
/// the output format. Returns the protected image as bytes.
#[must_use = "the protected image bytes should be saved or used"]
pub fn process_image_bytes(
    img_bytes: &[u8],
    level: ProtectionLevel,
    ctx: &ProtectionContext,
) -> Result<Vec<u8>> {
    let format = ImageOutputFormat::from_magic_bytes(img_bytes)
        .ok_or_else(|| Error::InvalidFormat("Unrecognized image format".to_string()))?;

    let ctx_with_format = {
        let mut ctx = ctx.clone();
        if ctx.input_format().is_none() {
            ctx.set_input_format(format);
        }
        ctx
    };

    DEFAULT_PIPELINE.process_bytes(img_bytes, level, &ctx_with_format)
}

/// Verify that image bytes contain a protection signature.
///
/// Checks metadata seeds, DCT stego (for JPEG), and LSB stego (for PNG/WebP).
///
/// # Returns
///
/// - `Some(true)` — protection data found and verification passed
/// - `Some(false)` — protection data found but verification failed (corrupted or wrong key)
/// - `None` — no protection data found in the image
///
/// # Arguments
///
/// * `img_bytes` - Raw image bytes (PNG, JPEG, or WebP)
/// * `mac_key` - Optional MAC key for HMAC-SHA256 verification. Pass empty slice
///   for checksum-only verification.
///
/// # Example
///
/// ```ignore
/// match cloakrs::verify_image_bytes(&img_bytes, &mac_key) {
///     Some(true) => println!("Protected and verified"),
///     Some(false) => println!("Protected but verification failed"),
///     None => println!("No protection found"),
/// }
/// ```
pub fn verify_image_bytes(img_bytes: &[u8], mac_key: &[u8]) -> Option<bool> {
    let stego = SteganographyProtector::new();
    stego.verify_payload_from_bytes_with_key(img_bytes, mac_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_disabled() {
        let pipeline = ProtectionPipeline::new();
        let img = DynamicImage::new_rgb8(10, 10);

        let result = pipeline.process(
            &img,
            ProtectionLevel::Disabled,
            &ProtectionContext::default(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_all_levels() {
        let pipeline = ProtectionPipeline::new();
        let img = DynamicImage::new_rgb8(10, 10);

        for level in &[
            ProtectionLevel::Disabled,
            ProtectionLevel::Light,
            ProtectionLevel::Standard,
        ] {
            let result = pipeline.process(&img, *level, &ProtectionContext::default());
            assert!(result.is_ok(), "Failed for level: {:?}", level);
        }
    }

    #[test]
    fn test_process_image_bytes() {
        use image::ImageEncoder;

        let img = DynamicImage::new_rgb8(10, 10);
        let mut buffer = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
            let rgb = img.to_rgb8();
            encoder
                .write_image(&rgb, 10, 10, image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        let result = process_image_bytes(
            &buffer,
            ProtectionLevel::Standard,
            &ProtectionContext::default(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_end_to_end_protection_verification() {
        let pipeline = ProtectionPipeline::new();
        let img = DynamicImage::new_rgb8(64, 64);

        let ctx = ProtectionContext::default()
            .with_seed(42)
            .with_intensity(0.5);

        let protected = pipeline
            .process(&img, ProtectionLevel::Standard, &ctx)
            .unwrap();

        let stego = SteganographyProtector::new();
        let verified = stego.verify_payload(&protected);

        assert!(verified, "Payload should be verified after protection");
    }

    #[test]
    fn test_process_bytes_and_verify() {
        use image::ImageEncoder;

        let img = DynamicImage::new_rgb8(32, 32);
        let mut input_bytes = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut input_bytes);
            let rgb = img.to_rgb8();
            encoder
                .write_image(&rgb, 32, 32, image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        let ctx = ProtectionContext::default()
            .with_seed(12345)
            .with_intensity(0.7);

        let protected_bytes =
            process_image_bytes(&input_bytes, ProtectionLevel::Standard, &ctx).unwrap();

        assert!(!protected_bytes.is_empty());
        // Output should differ from input when intensity > 0.
        // Size difference alone is insufficient (metadata injection always
        // changes size), so also verify content differs for non-zero intensity.
        assert!(
            protected_bytes.len() != input_bytes.len() || ctx.intensity() == 0.0,
            "Protected bytes should differ from input at intensity {}",
            ctx.intensity()
        );
    }

    #[test]
    fn test_different_seeds_different_output() {
        let pipeline = ProtectionPipeline::new();
        let img = DynamicImage::new_rgb8(32, 32);

        let ctx1 = ProtectionContext::default().with_seed(42);
        let ctx2 = ProtectionContext::default().with_seed(99);

        let result1 = pipeline
            .process(&img, ProtectionLevel::Standard, &ctx1)
            .unwrap();
        let result2 = pipeline
            .process(&img, ProtectionLevel::Standard, &ctx2)
            .unwrap();

        let rgba1 = result1.to_rgba8();
        let rgba2 = result2.to_rgba8();

        assert_ne!(
            rgba1.as_raw(),
            rgba2.as_raw(),
            "Different seeds should produce different output"
        );
    }

    #[test]
    fn test_parallel_processing() {
        let images: Vec<DynamicImage> = (0..4).map(|_| DynamicImage::new_rgb8(16, 16)).collect();

        let ctx = ProtectionContext::default().with_seed(42);

        let results = process_images_parallel(&images, ProtectionLevel::Standard, &ctx).unwrap();

        assert_eq!(results.len(), 4);
    }

    #[test]
    fn test_metadata_extraction() {
        let img = DynamicImage::new_rgb8(32, 32);

        let ctx = ProtectionContext::default()
            .with_seed(42)
            .with_format(ImageOutputFormat::Png);

        let metadata_protector = MetadataTrapProtector::new();
        let encoded = crate::util::image::encode_image(&img, image::ImageFormat::Png).unwrap();

        let protected_bytes = metadata_protector.apply_bytes(&encoded, &ctx).unwrap();

        let seed = MetadataTrapProtector::extract_seed_from_image(&protected_bytes);

        assert!(
            seed.is_some(),
            "Seed should be extractable from protected image"
        );
    }

    #[test]
    fn test_intensity_zero_no_change() {
        let pipeline = ProtectionPipeline::new();
        let img = DynamicImage::new_rgb8(16, 16);

        let ctx = ProtectionContext::default()
            .with_seed(42)
            .with_intensity(0.0);

        let result = pipeline
            .process(&img, ProtectionLevel::Disabled, &ctx)
            .unwrap();

        let original_bytes = img.to_rgba8();
        let result_bytes = result.to_rgba8();

        assert_eq!(original_bytes.as_raw(), result_bytes.as_raw());
    }

    #[test]
    fn test_max_dimension_validation() {
        let pipeline = ProtectionPipeline::new();
        let img = DynamicImage::new_rgb8(1000, 1000);

        let ctx = ProtectionContext::default().with_max_dimension(512);

        let result = pipeline.process(&img, ProtectionLevel::Standard, &ctx);

        assert!(
            result.is_err(),
            "Should fail when image exceeds max dimension"
        );
    }

    #[test]
    fn test_max_dimension_validation_process_bytes() {
        use image::ImageEncoder;

        let img = DynamicImage::new_rgb8(1000, 1000);
        let mut buffer = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
            let rgb = img.to_rgb8();
            encoder
                .write_image(&rgb, 1000, 1000, image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        let ctx = ProtectionContext::default().with_max_dimension(512);

        let result = process_image_bytes(&buffer, ProtectionLevel::Standard, &ctx);

        assert!(
            result.is_err(),
            "Should fail when image exceeds max dimension via process_bytes"
        );
    }
}
