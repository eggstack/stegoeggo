//! High-performance image protection library for WAF edge deployment.
//!
//! This library provides modular protection strategies to protect images from being
//! scraped and used to train AI models. Optimized for sub-10ms latency in CDN/WAF edge deployments.
//!
//! # Protection Levels
//!
//! - `Disabled`: No protection applied
//! - `Light`: Metadata injection (tEXt chunks for PNG, COM markers for JPEG)
//! - `Standard`: Noise perturbation + steganography + metadata
//! - `Enhanced`: Enhanced perturbation + steganography + metadata
//! - `Strong`: Precomputed variants + steganography + metadata
//!
//! # Multi-Layered Defense
//!
//! Each protection level applies multiple layers of defense:
//! 1. **Noise Perturbation** - Adversarial pixel-level noise
//! 2. **Steganography** - Hidden LSB payload (PNG/WebP) or DCT perturbation (JPEG)
//! 3. **Metadata Injection** - Visible anti-scraping markers
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
//! # Security Considerations
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
//! use cloakrs::{process_image_bytes, ProtectionContext, ProtectionLevel, ImageOutputFormat, TargetModel};
//!
//! let ctx = ProtectionContext::new(TargetModel::StableDiffusionXL, 0.5, 42)
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

pub use error::{Error, Result};
pub use types::{
    DmiValue, ImageOutputFormat, LegalMetadata, ProtectedVariant, ProtectionConfig,
    ProtectionContext, ProtectionLevel, TargetModel, DEFAULT_OUTPUT_FORMAT,
};

pub use traits::{NoOpLoader, Protector, VariantLoader};

pub use protected::enhanced::EnhancedProtector;
pub use protected::metadata_trap::MetadataTrapProtector;
pub use protected::noise::NoiseProtector;
pub use protected::passthrough::PassthroughProtector;
pub use protected::precomputed::PrecomputedProtector;
pub use protected::steganography::{SteganographyProtector, StegoPayload};

pub use jpeg_transcoder::is_progressive_jpeg;

pub use util::image::{
    apply_perturbation, compute_image_hash, detect_image_format, encode_image,
    encode_image_with_options, load_image_from_bytes, NoiseGenerator,
};

pub use util::iscc::{compute_iscc, compute_iscc_from_bytes, Iscc};

use image::DynamicImage;
use image::GenericImageView;
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::LazyLock;

static DEFAULT_PIPELINE: LazyLock<ProtectionPipeline> = LazyLock::new(ProtectionPipeline::new);

#[derive(Clone, Copy)]
enum MultiProtector {
    Noise,
    Enhanced,
    Precomputed,
}

impl MultiProtector {
    fn to_protection_level(self) -> ProtectionLevel {
        match self {
            MultiProtector::Noise => ProtectionLevel::Standard,
            MultiProtector::Enhanced => ProtectionLevel::Enhanced,
            MultiProtector::Precomputed => ProtectionLevel::Strong,
        }
    }
}

/// Main pipeline for applying protection to images.
///
/// Coordinates between different protector implementations based on the
/// selected protection level.
pub struct ProtectionPipeline {
    passthrough: Arc<PassthroughProtector>,
    metadata_trap: Arc<MetadataTrapProtector>,
    noise: Arc<protected::noise::NoiseProtector>,
    enhanced: Arc<protected::enhanced::EnhancedProtector>,
    precomputed: Arc<protected::precomputed::PrecomputedProtector>,
    steganography: Arc<SteganographyProtector>,
}

impl ProtectionPipeline {
    /// Create a new ProtectionPipeline with default protectors.
    pub fn new() -> Self {
        Self {
            passthrough: Arc::new(PassthroughProtector::new()),
            metadata_trap: Arc::new(MetadataTrapProtector::new()),
            noise: Arc::new(protected::noise::NoiseProtector::new()),
            enhanced: Arc::new(protected::enhanced::EnhancedProtector::new()),
            precomputed: Arc::new(protected::precomputed::PrecomputedProtector::new()),
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

    fn apply_perturbation<'a>(
        &'a self,
        img: &'a DynamicImage,
        ctx: &ProtectionContext,
        protector: MultiProtector,
    ) -> Result<Cow<'a, DynamicImage>> {
        match protector {
            MultiProtector::Noise => self.noise.apply(img, ctx),
            MultiProtector::Enhanced => self.enhanced.apply(img, ctx),
            MultiProtector::Precomputed => self.precomputed.apply(img, ctx),
        }
    }

    pub fn process<'a>(
        &'a self,
        img: &'a DynamicImage,
        level: ProtectionLevel,
        ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>> {
        Self::validate_dimensions(img, ctx.max_dimension)?;

        match level {
            ProtectionLevel::Disabled => self.passthrough.apply(img, ctx),
            ProtectionLevel::Light => self.metadata_trap.apply(img, ctx),
            ProtectionLevel::Standard => self
                .apply_multi_protector(img, ctx, MultiProtector::Noise)
                .map(Cow::Owned),
            ProtectionLevel::Enhanced => self
                .apply_multi_protector(img, ctx, MultiProtector::Enhanced)
                .map(Cow::Owned),
            ProtectionLevel::Strong => self
                .apply_multi_protector(img, ctx, MultiProtector::Precomputed)
                .map(Cow::Owned),
        }
    }

    fn apply_multi_protector(
        &self,
        img: &DynamicImage,
        ctx: &ProtectionContext,
        protector: MultiProtector,
    ) -> Result<DynamicImage> {
        let output_format = ctx
            .output_format
            .or(ctx.input_format)
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        let final_bytes = self.apply_protector_pipeline(img, ctx, protector, output_format)?;
        image::load_from_memory(&final_bytes).map_err(|e| Error::ImageDecode(e.to_string()))
    }

    /// Shared pipeline: perturb → stego → encode → metadata injection.
    /// Used by both `apply_multi_protector` and `apply_multi_protector_bytes`.
    fn apply_protector_pipeline(
        &self,
        img: &DynamicImage,
        ctx: &ProtectionContext,
        protector: MultiProtector,
        output_format: crate::types::ImageOutputFormat,
    ) -> Result<Vec<u8>> {
        let processed = self.apply_perturbation(img, ctx, protector)?;

        // JPEG output: encode first, then apply DCT stego to the JPEG bytes
        if output_format == crate::types::ImageOutputFormat::Jpeg {
            let jpeg_bytes = crate::util::image::encode_image_with_options(
                &processed,
                Some(output_format),
                ctx.progressive_jpeg,
                ctx.jpeg_quality,
            )?;
            let with_stego = self.steganography.apply_dct_stego_bytes(&jpeg_bytes, ctx)?;
            return self.metadata_trap.inject_bytes(&with_stego, ctx);
        }

        // Non-JPEG output: pixel stego then encode
        let with_stego = self.steganography.apply(&processed, ctx)?;
        let encoded = crate::util::image::encode_image_with_options(
            &with_stego,
            Some(output_format),
            ctx.progressive_jpeg,
            ctx.jpeg_quality,
        )?;
        self.metadata_trap.inject_bytes(&encoded, ctx)
    }

    pub fn register_precomputed_variants(&self, variants: Vec<ProtectedVariant>) -> Result<()> {
        self.precomputed.register_variants(variants)
    }

    pub fn process_bytes(
        &self,
        img_bytes: &[u8],
        level: ProtectionLevel,
        ctx: &ProtectionContext,
    ) -> Result<Vec<u8>> {
        let mut ctx_with_level = ctx.clone();
        ctx_with_level.protection_level = Some(level);

        match level {
            ProtectionLevel::Disabled => Ok(img_bytes.to_vec()),
            ProtectionLevel::Light => self.metadata_trap.apply_bytes(img_bytes, &ctx_with_level),
            ProtectionLevel::Standard => {
                self.apply_multi_protector_bytes(img_bytes, &ctx_with_level, MultiProtector::Noise)
            }
            ProtectionLevel::Enhanced => self.apply_multi_protector_bytes(
                img_bytes,
                &ctx_with_level,
                MultiProtector::Enhanced,
            ),
            ProtectionLevel::Strong => self.apply_multi_protector_bytes(
                img_bytes,
                &ctx_with_level,
                MultiProtector::Precomputed,
            ),
        }
    }

    fn apply_multi_protector_bytes(
        &self,
        img_bytes: &[u8],
        ctx: &ProtectionContext,
        protector: MultiProtector,
    ) -> Result<Vec<u8>> {
        let mut ctx_with_level = ctx.clone();
        ctx_with_level.protection_level = Some(protector.to_protection_level());

        let input_format = ctx
            .input_format
            .or_else(|| crate::types::ImageOutputFormat::from_magic_bytes(img_bytes))
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        let output_format = ctx
            .output_format
            .or(ctx.input_format)
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        // JPEG-in, JPEG-out: byte-only path (DCT stego + metadata, no pixel manipulation).
        // This preserves quality and avoids decode/encode cycles.
        if input_format == crate::types::ImageOutputFormat::Jpeg
            && output_format == crate::types::ImageOutputFormat::Jpeg
        {
            let with_stego = self
                .steganography
                .apply_dct_stego_bytes(img_bytes, &ctx_with_level)?;
            return self
                .metadata_trap
                .inject_bytes(&with_stego, &ctx_with_level);
        }

        // Non-JPEG-in: decode then use shared pipeline
        let img = load_image_from_bytes(img_bytes)?;
        self.apply_protector_pipeline(&img, &ctx_with_level, protector, output_format)
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
///
/// # Example
///
/// ```ignore
/// use cloakrs::{process_images_parallel, ProtectionContext, ProtectionLevel};
/// use image::DynamicImage;
///
/// // In real usage, load actual images:
/// // let images: Vec<DynamicImage> = vec![
/// //     image::open("image1.png")?,
/// //     image::open("image2.png")?,
/// // ];
/// // let ctx = ProtectionContext::default();
/// // let results = process_images_parallel(&images, ProtectionLevel::Standard, &ctx).unwrap();
/// ```
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
/// Note: In a real application, you would load actual image files.
///
/// # Example
///
/// ```ignore
/// use cloakrs::{process_images_bytes_parallel, ProtectionContext, ProtectionLevel};
///
/// // In real usage, load actual image files:
/// // let images: Vec<Vec<u8>> = vec![
/// //     std::fs::read("image1.png")?,
/// //     std::fs::read("image2.png")?,
/// // ];
/// // let ctx = ProtectionContext::default();
/// // let results = process_images_bytes_parallel(&images, ProtectionLevel::Standard, &ctx).unwrap();
/// ```
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
pub fn process_image_bytes(
    img_bytes: &[u8],
    level: ProtectionLevel,
    ctx: &ProtectionContext,
) -> Result<Vec<u8>> {
    let format = ImageOutputFormat::from_magic_bytes(img_bytes).unwrap_or(DEFAULT_OUTPUT_FORMAT);

    let ctx_with_format = {
        let mut ctx = ctx.clone();
        if ctx.input_format.is_none() {
            ctx.input_format = Some(format);
        }
        ctx
    };

    DEFAULT_PIPELINE.process_bytes(img_bytes, level, &ctx_with_format)
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
            ProtectionLevel::Enhanced,
            ProtectionLevel::Strong,
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
    fn test_end_to_end_enhanced_level() {
        let pipeline = ProtectionPipeline::new();
        let img = DynamicImage::new_rgb8(64, 64);

        let ctx = ProtectionContext::default()
            .with_seed(42)
            .with_intensity(0.5);

        let protected = pipeline
            .process(&img, ProtectionLevel::Enhanced, &ctx)
            .unwrap();

        let stego = SteganographyProtector::new();
        let verified = stego.verify_payload(&protected);

        assert!(
            verified,
            "Payload should be verified after enhanced protection"
        );
    }

    #[test]
    fn test_end_to_end_strong_level() {
        let pipeline = ProtectionPipeline::new();
        let img = DynamicImage::new_rgb8(64, 64);

        let ctx = ProtectionContext::default()
            .with_seed(42)
            .with_intensity(0.5);

        let protected = pipeline
            .process(&img, ProtectionLevel::Strong, &ctx)
            .unwrap();

        let stego = SteganographyProtector::new();
        let verified = stego.verify_payload(&protected);

        assert!(
            verified,
            "Payload should be verified after strong protection"
        );
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
        assert!(protected_bytes.len() != input_bytes.len() || ctx.intensity == 0.0);
    }

    #[test]
    fn test_deterministic_protection() {
        let noise = NoiseProtector::new();
        let img = DynamicImage::new_rgb8(32, 32);

        let ctx1 = ProtectionContext::default().with_seed(42);
        let ctx2 = ProtectionContext::default().with_seed(42);

        let result1 = noise.apply(&img, &ctx1).unwrap();
        let result2 = noise.apply(&img, &ctx2).unwrap();

        assert_eq!(result1.width(), result2.width());
        assert_eq!(result1.height(), result2.height());
        assert_eq!(result1.to_rgba8().as_raw(), result2.to_rgba8().as_raw());
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
}
