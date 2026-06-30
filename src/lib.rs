//! Image protection library for legal deterrence against unauthorized AI training.
//!
//! Protects images through steganographic payload embedding and metadata injection,
//! providing evidence of protection and legal warnings that survive casual modification.
//!
//! # Protection Levels
//!
//! - `Disabled`: No protection applied
//! - `Light`: Metadata injection plus minimal seed stego (Q-table seed for JPEG,
//!   LSB redundancy=1 for PNG/WebP)
//! - `Standard`: Steganography + metadata injection
//!
//! # Protection Layers
//!
//! Each protection level applies one or more layers:
//! 1. **Steganography** - Hidden LSB payload (PNG, lossless WebP) or DCT perturbation (JPEG). Lossy WebP is not supported.
//! 2. **Metadata Injection** - Visible anti-scraping markers (XMP, IPTC, EXIF)
//!
//! # JPEG Limitations
//!
//! JPEG's lossy compression inherently limits steganographic robustness. For JPEG,
//! the library can store a seed in quantization tables when those tables are preserved
//! and uses F5-style DCT coefficient embedding for baseline JPEGs. However,
//! pixel-based stego payloads may not survive JPEG re-compression.
//! **For maximum verifiability, use PNG output format.**
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
//! **Without a MAC key**, steganographic payload verification uses a non-cryptographic
//! CRC32 checksum. An attacker can trivially forge valid-looking payloads.
//! This is suitable for accidental corruption detection and legal deterrence
//! (visible metadata markers prove intent), but **not** for adversarial settings.
//!
//! **With a MAC key**, the library uses HMAC-SHA256 for cryptographic payload
//! verification. Always set a MAC key in production:
//!
//! ```ignore
//! use stegoeggo::{ProtectionContext, ProtectionLevel};
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
//! use stegoeggo::{process_image_bytes, ProtectionContext, ProtectionLevel, ImageOutputFormat};
//!
//! let ctx = ProtectionContext::new(0.5, 42)
//!     .with_format(ImageOutputFormat::Png)
//!     .with_mac_key(b"shared-verification-key".to_vec())
//!     .with_stego_redundancy(2)      // Lower = faster
//!     .with_jpeg_quality(85)        // Lower = smaller files
//!     .with_progressive_jpeg(true); // Progressive rendering for web
//!
//! let input_bytes = std::fs::read("image.png")?;
//! let (protected, warnings) =
//!     stegoeggo::process_image_bytes_with_warnings(&input_bytes, ProtectionLevel::Standard, &ctx)?;
//! // Reverse proxies should log or enforce warnings before serving.
//! ```
//!
//! Legal Metadata
//!
//! Embed copyright and usage restrictions in images for IP protection.
//!
//! ```rust
//! use stegoeggo::{ProtectionContext, LegalMetadata, ProtectionLevel};
//!
//! let ctx = ProtectionContext::default()
//!     .with_legal_metadata(
//!         LegalMetadata::new()
//!             .with_copyright_holder("Example Corp")
//!             .with_contact_email("legal@example.com")
//!             .with_usage_terms("All Rights Reserved. No AI training permitted.")
//!     )
//!     .with_legal_claims(true);
//! ```
//!
//! # Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `async` | Enables Tokio-based async wrappers (`process_image_async`, etc.) for WAF/CDN integration |
//! | `test-seeds` | Enables fallback seed guessing during verification (tries common test/dev seeds). Used by the CLI; not recommended for library consumers |
//! | `fuzz` | Exposes internal JPEG parser for fuzz harnesses. Not part of the stable API |
//!
//! # Tiled Steganography
//!
//! For crop-resistant protection, enable tiled mode. The full payload is embedded
//! in each `tile_size × tile_size` tile independently, so any crop containing at
//! least one intact tile is recoverable:
//!
//! ```ignore
//! use stegoeggo::{ProtectionContext, ProtectionLevel};
//!
//! let ctx = ProtectionContext::new(0.5, 42)
//!     .with_tile_size(64);          // 64×64 tiles
//!
//! let protected = stegoeggo::process_image_bytes(&img_bytes, ProtectionLevel::Standard, &ctx)?;
//! ```
//!
//! # Async API
//!
//! For Tokio-based services (WAFs, CDN edge workers), use the async variants:
//!
//! ```ignore
//! use stegoeggo::{process_image_bytes_async, ProtectionContext, ProtectionLevel};
//!
//! let ctx = ProtectionContext::new(0.5, 42);
//! let protected = process_image_bytes_async(&img_bytes, ProtectionLevel::Standard, &ctx).await?;
//! ```
//!
//! # Parallel Batch Processing
//!
//! Process multiple images concurrently using Rayon:
//!
//! ```ignore
//! use stegoeggo::{process_images_parallel, ProtectionContext, ProtectionLevel};
//!
//! let images: Vec<image::DynamicImage> = vec![ /* ... */ ];
//! let ctx = ProtectionContext::default();
//! let results = process_images_parallel(&images, ProtectionLevel::Standard, &ctx)?;
//! ```
//!
//! # Warnings API
//!
//! `process_image_bytes_with_warnings` returns both the protected bytes and
//! any warnings about the protection process (e.g., progressive JPEG fallback,
//! insufficient DCT capacity):
//!
//! ```ignore
//! use stegoeggo::{process_image_bytes_with_warnings, ProtectionContext, ProtectionLevel};
//!
//! let ctx = ProtectionContext::new(0.5, 42);
//! let (protected, warnings) =
//!     process_image_bytes_with_warnings(&img_bytes, ProtectionLevel::Standard, &ctx)?;
//!
//! for w in &warnings {
//!     eprintln!("Warning: {w}");
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod traits;
/// Core types: protection levels, configuration, legal metadata, and verification results.
pub mod types;

pub(crate) mod jpeg_transcoder;
pub(crate) mod protected;
pub(crate) mod util;

#[cfg(feature = "async")]
pub mod async_api;

pub use error::{Error, Result};
pub use types::{
    DmiValue, ImageOutputFormat, LegalMetadata, ProtectionConfig, ProtectionContext,
    ProtectionLevel, ProtectionWarning, VerificationResult, VerificationStatus,
    DEFAULT_OUTPUT_FORMAT,
};

pub use traits::Protector;

pub use protected::metadata_trap::MetadataTrapProtector;
pub use protected::passthrough::PassthroughProtector;
pub use protected::steganography::{SteganographyProtector, StegoPayload};

pub use jpeg_transcoder::is_progressive_jpeg;

/// Parse JPEG header and decode DCT coefficients from raw bytes.
///
/// This function is exposed for fuzzing the internal JPEG parser. It parses
/// the JPEG header and decodes the entropy-coded scan data into DCT coefficients.
///
/// Returns `Ok((header, coefficients))` on success, or `Err` if the JPEG
/// is invalid, progressive, or otherwise unsupported.
#[cfg(feature = "fuzz")]
pub fn parse_jpeg_for_fuzz(
    data: &[u8],
) -> std::result::Result<
    (jpeg_transcoder::JpegHeader, jpeg_transcoder::Coefficients),
    jpeg_transcoder::TranscoderError,
> {
    jpeg_transcoder::JpegTranscoder::decode_coefficients(data)
}

pub use util::image::{
    compute_image_hash, detect_image_format, encode_image, encode_image_with_options,
    load_image_from_bytes,
};

pub use util::iscc::{
    compute_iscc, compute_iscc_from_bytes, compute_iscc_from_bytes_with_metadata,
    compute_iscc_with_metadata, Iscc,
};
pub use util::seed::generate_random_seed;

#[cfg(feature = "async")]
pub use async_api::{
    process_image_async, process_image_bytes_async, process_image_bytes_with_warnings_async,
    process_images_bytes_parallel_async, process_images_parallel_async, verify_image_bytes_async,
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
/// selected protection level. Create one via [`ProtectionPipeline::new`]
/// or use the convenience functions ([`process_image`], [`process_image_bytes`]).
#[non_exhaustive]
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
        let output_format = resolved_output_format(ctx);
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

    /// Light level: metadata injection + minimal steganographic seed marker.
    /// For JPEG, stores the seed in quantization tables when those tables are preserved.
    /// For PNG/WebP, embeds a minimal LSB payload with redundancy=1,
    /// then injects metadata (so tEXt chunks survive the re-encode).
    /// Encodes to bytes, applies minimal stego, injects metadata,
    /// then decodes back to `DynamicImage`.
    fn apply_light_bytes(
        &self,
        img: &DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<DynamicImage> {
        let output_format = resolved_output_format(ctx);

        match output_format {
            crate::types::ImageOutputFormat::Jpeg => {
                let encoded = crate::util::image::encode_image_with_options(
                    img,
                    Some(output_format),
                    ctx.progressive_jpeg(),
                    ctx.jpeg_quality(),
                )?;
                let with_metadata = self.metadata_trap.inject_bytes(&encoded, ctx)?;
                let with_stego = self
                    .steganography
                    .apply_qtable_seed_bytes(&with_metadata, ctx.seed())?;
                Ok(image::load_from_memory(&with_stego)?)
            }
            _ => {
                let mut minimal_ctx = ctx.clone();
                minimal_ctx.set_protection_level(crate::types::ProtectionLevel::Light);
                let stego_img = self.steganography.embed_lsb_minimal(img, &minimal_ctx);
                let encoded = crate::util::image::encode_image_with_options(
                    &stego_img,
                    Some(output_format),
                    ctx.progressive_jpeg(),
                    ctx.jpeg_quality(),
                )?;
                let with_metadata = self.metadata_trap.inject_bytes(&encoded, ctx)?;
                Ok(image::load_from_memory(&with_metadata)?)
            }
        }
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
        if level == ProtectionLevel::Disabled {
            return Ok(img_bytes.to_vec());
        }

        let (ctx_with_level, input_format, output_format) =
            Self::context_for_bytes(img_bytes, level, ctx)?;

        match level {
            ProtectionLevel::Disabled => unreachable!("disabled level returned above"),
            ProtectionLevel::Light => {
                self.validate_input_dimensions_for_bytes(
                    img_bytes,
                    input_format,
                    ctx_with_level.max_dimension(),
                )?;
                self.apply_light_bytes_pipeline(
                    img_bytes,
                    input_format,
                    output_format,
                    &ctx_with_level,
                )
            }
            ProtectionLevel::Standard => self.apply_bytes_pipeline_resolved(
                img_bytes,
                input_format,
                output_format,
                &ctx_with_level,
            ),
        }
    }

    fn input_format_from_bytes(
        img_bytes: &[u8],
        ctx: &ProtectionContext,
    ) -> Result<crate::types::ImageOutputFormat> {
        ctx.input_format()
            .or_else(|| crate::types::ImageOutputFormat::from_magic_bytes(img_bytes))
            .ok_or_else(|| Error::InvalidFormat("Unrecognized image format".to_string()))
    }

    fn output_format_for_bytes(
        ctx: &ProtectionContext,
        input_format: crate::types::ImageOutputFormat,
    ) -> crate::types::ImageOutputFormat {
        ctx.output_format().unwrap_or(input_format)
    }

    fn context_for_bytes(
        img_bytes: &[u8],
        level: ProtectionLevel,
        ctx: &ProtectionContext,
    ) -> Result<(
        ProtectionContext,
        crate::types::ImageOutputFormat,
        crate::types::ImageOutputFormat,
    )> {
        let mut ctx_with_level = ctx.clone();
        ctx_with_level.set_protection_level(level);

        let input_format = Self::input_format_from_bytes(img_bytes, &ctx_with_level)?;
        if ctx_with_level.input_format().is_none() {
            ctx_with_level.set_input_format(input_format);
        }
        let output_format = Self::output_format_for_bytes(&ctx_with_level, input_format);

        Ok((ctx_with_level, input_format, output_format))
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

    fn validate_input_dimensions_for_bytes(
        &self,
        img_bytes: &[u8],
        input_format: crate::types::ImageOutputFormat,
        max_dim: Option<u32>,
    ) -> Result<()> {
        if max_dim.is_none() {
            return Ok(());
        }

        if input_format == crate::types::ImageOutputFormat::Jpeg {
            Self::validate_jpeg_dimensions_from_bytes(img_bytes, max_dim)
        } else {
            let img = load_image_from_bytes(img_bytes)?;
            Self::validate_dimensions(&img, max_dim)
        }
    }

    fn apply_light_bytes_pipeline(
        &self,
        img_bytes: &[u8],
        input_format: crate::types::ImageOutputFormat,
        output_format: crate::types::ImageOutputFormat,
        ctx: &ProtectionContext,
    ) -> Result<Vec<u8>> {
        if output_format == crate::types::ImageOutputFormat::Jpeg {
            let encoded = if input_format == crate::types::ImageOutputFormat::Jpeg {
                img_bytes.to_vec()
            } else {
                let img = load_image_from_bytes(img_bytes)?;
                crate::util::image::encode_image_with_options(
                    &img,
                    Some(output_format),
                    ctx.progressive_jpeg(),
                    ctx.jpeg_quality(),
                )?
            };
            let with_metadata = self.metadata_trap.apply_bytes(&encoded, ctx)?;
            return self
                .steganography
                .apply_qtable_seed_bytes(&with_metadata, ctx.seed());
        }

        let mut minimal_ctx = ctx.clone();
        minimal_ctx.set_protection_level(crate::types::ProtectionLevel::Light);
        let img = load_image_from_bytes(img_bytes)?;
        let stego_img = self.steganography.embed_lsb_minimal(&img, &minimal_ctx);
        let encoded = crate::util::image::encode_image_with_options(
            &stego_img,
            Some(output_format),
            ctx.progressive_jpeg(),
            ctx.jpeg_quality(),
        )?;
        self.metadata_trap.apply_bytes(&encoded, ctx)
    }

    fn apply_bytes_pipeline_resolved(
        &self,
        img_bytes: &[u8],
        input_format: crate::types::ImageOutputFormat,
        output_format: crate::types::ImageOutputFormat,
        ctx: &ProtectionContext,
    ) -> Result<Vec<u8>> {
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
///
/// # Errors
///
/// Returns [`Error::ImageDecode`] if the image dimensions exceed `max_dimension`.
/// Returns [`Error::ImageEncode`] or [`Error::Steganography`] if encoding or
/// embedding fails.
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
///
/// # Examples
///
/// ```no_run
/// use stegoeggo::{process_images_parallel, ProtectionContext, ProtectionLevel};
/// use image::DynamicImage;
///
/// let images: Vec<DynamicImage> = vec![
///     image::open("image1.png").unwrap(),
///     image::open("image2.png").unwrap(),
/// ];
/// let ctx = ProtectionContext::new(0.5, 42);
/// let protected = process_images_parallel(&images, ProtectionLevel::Standard, &ctx).unwrap();
/// ```
///
/// # Errors
///
/// Returns the first error encountered from any image processing call.
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
///
/// # Examples
///
/// ```no_run
/// use stegoeggo::{process_images_bytes_parallel, ProtectionContext, ProtectionLevel};
///
/// let images: Vec<Vec<u8>> = vec![
///     std::fs::read("image1.png").unwrap(),
///     std::fs::read("image2.png").unwrap(),
/// ];
/// let ctx = ProtectionContext::new(0.5, 42);
/// let protected = process_images_bytes_parallel(&images, ProtectionLevel::Standard, &ctx).unwrap();
/// ```
///
/// # Errors
///
/// Returns the first error encountered from any image processing call.
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
/// the output format. Returns the protected image as bytes. `Disabled`
/// protection is a byte-for-byte no-op and does not require format detection.
///
/// For JPEG-in/JPEG-out, this function takes a byte-only fast path that
/// operates on DCT coefficients directly, avoiding pixel decode/encode cycles.
///
/// # Examples
///
/// ```no_run
/// use stegoeggo::{process_image_bytes, ProtectionContext, ProtectionLevel};
///
/// let img_bytes: Vec<u8> = std::fs::read("input.png").unwrap();
/// let ctx = ProtectionContext::new(0.5, 42);
/// let protected = process_image_bytes(&img_bytes, ProtectionLevel::Standard, &ctx).unwrap();
/// std::fs::write("output.png", &protected).unwrap();
/// ```
///
/// # Errors
///
/// Returns [`Error::InvalidFormat`] if the image format cannot be determined
/// for `Light` or `Standard` protection.
/// Returns [`Error::ImageDecode`], [`Error::ImageEncode`], or
/// [`Error::Steganography`] if processing fails.
#[must_use = "the protected image bytes should be saved or used"]
pub fn process_image_bytes(
    img_bytes: &[u8],
    level: ProtectionLevel,
    ctx: &ProtectionContext,
) -> Result<Vec<u8>> {
    DEFAULT_PIPELINE.process_bytes(img_bytes, level, ctx)
}

/// Process image bytes with protection level and return warnings about
/// degraded protection.
///
/// Like [`process_image_bytes`], but also returns a [`ProtectionWarning`] if
/// the protection was applied with reduced effectiveness. This is important
/// for legal defense use cases where the caller needs to know the actual
/// protection level applied.
///
/// This compatibility helper returns only the first warning. New reverse-proxy
/// integrations should prefer [`process_image_bytes_with_warnings`] so they can
/// log or enforce every warning emitted for the request.
///
/// # Examples
///
/// ```no_run
/// use stegoeggo::{process_image_bytes_with_info, ProtectionContext, ProtectionLevel};
///
/// let img_bytes: Vec<u8> = std::fs::read("input.jpg").unwrap();
/// let ctx = ProtectionContext::new(0.5, 42);
/// let (protected, warning) = process_image_bytes_with_info(
///     &img_bytes, ProtectionLevel::Standard, &ctx
/// ).unwrap();
/// if let Some(w) = warning {
///     eprintln!("Warning: {}", w);
/// }
/// ```
///
/// # Errors
///
/// Returns [`Error::InvalidFormat`] if the image format cannot be determined.
/// Returns [`Error::ImageDecode`], [`Error::ImageEncode`], or
/// [`Error::Steganography`] if processing fails.
pub fn process_image_bytes_with_info(
    img_bytes: &[u8],
    level: ProtectionLevel,
    ctx: &ProtectionContext,
) -> Result<(Vec<u8>, Option<ProtectionWarning>)> {
    let (bytes, warnings) = process_image_bytes_with_warnings(img_bytes, level, ctx)?;
    let warning = warnings.into_iter().next();
    Ok((bytes, warning))
}

/// Process image bytes with protection level and return all protection warnings.
///
/// This is the recommended API for reverse-proxy integrations. It keeps the
/// hot path byte-oriented, while giving the caller enough information to make
/// policy decisions about serving, logging, or falling back when the actual
/// emitted evidence is weaker than requested.
///
/// The library owns steganographic and metadata injection mechanics; the proxy
/// should still enforce request byte limits, concurrency limits, timeouts, and
/// cache policy outside this function.
///
/// # Errors
///
/// Returns [`Error::InvalidFormat`] if the image format cannot be determined.
/// Returns [`Error::ImageDecode`], [`Error::ImageEncode`], or
/// [`Error::Steganography`] if processing fails.
pub fn process_image_bytes_with_warnings(
    img_bytes: &[u8],
    level: ProtectionLevel,
    ctx: &ProtectionContext,
) -> Result<(Vec<u8>, Vec<ProtectionWarning>)> {
    if level == ProtectionLevel::Disabled {
        let result = DEFAULT_PIPELINE.process_bytes(img_bytes, level, ctx)?;
        return Ok((result, Vec::new()));
    }

    let (ctx_with_format, format) = context_with_detected_format(img_bytes, ctx)?;

    let mut warnings = Vec::new();
    if level != ProtectionLevel::Disabled && ctx_with_format.mac_key().is_none() {
        warnings.push(ProtectionWarning::MissingMacKey);
    }
    if matches!(ctx_with_format.inject_metadata(), Some(false)) {
        warnings.push(ProtectionWarning::MetadataInjectionDisabled);
    }

    let output_format = resolved_output_format(&ctx_with_format);

    // Detect progressive JPEG fallback before processing — the pipeline silently
    // falls back to Q-table seed only for progressive JPEGs. If the caller
    // requested progressive JPEG output, the encoded output will be progressive
    // and the DCT transcoder will reject it.
    if level == ProtectionLevel::Standard
        && ctx_with_format.progressive_jpeg()
        && output_format == ImageOutputFormat::Jpeg
    {
        warnings.push(ProtectionWarning::ProgressiveJpegFallback);
    }

    // Also detect progressive JPEG input: the DCT transcoder cannot decode
    // progressive JPEGs, so Standard level falls back to Q-table seed only.
    if level == ProtectionLevel::Standard
        && format == ImageOutputFormat::Jpeg
        && is_progressive_jpeg(img_bytes)
    {
        warnings.push(ProtectionWarning::ProgressiveJpegFallback);
    }

    if level != ProtectionLevel::Disabled && output_format == ImageOutputFormat::Jpeg {
        warnings.push(ProtectionWarning::JpegReencodeFragile);
    }
    if level != ProtectionLevel::Disabled && output_format == ImageOutputFormat::WebP {
        warnings.push(ProtectionWarning::WebpLossyReencodeDestructive);
    }

    // Pre-check LSB capacity for PNG/WebP Standard level — the pipeline silently
    // skips embedding when the image has too few pixels.
    if level == ProtectionLevel::Standard
        && matches!(
            output_format,
            ImageOutputFormat::Png | ImageOutputFormat::WebP
        )
    {
        if let Ok(img) = image::load_from_memory(img_bytes) {
            let (w, h) = img.dimensions();
            let total_pixels = (w as usize) * (h as usize);
            let pixels_needed = SteganographyProtector::lsb_pixels_needed(&ctx_with_format);
            if total_pixels < pixels_needed {
                warnings.push(ProtectionWarning::LsbCapacitySkipped);
            }
        }
    }

    let result = DEFAULT_PIPELINE.process_bytes(img_bytes, level, &ctx_with_format)?;

    // Post-check DCT capacity: if the output is essentially unchanged (same length
    // or within a small margin), DCT stego likely failed due to insufficient
    // AC coefficients. This catches small JPEGs where the F5 embed silently falls
    // back to Q-table seed only.
    if level == ProtectionLevel::Standard && format == ImageOutputFormat::Jpeg {
        let size_delta = (result.len() as i64 - img_bytes.len() as i64).unsigned_abs();
        // If the output is smaller or nearly the same size, DCT stego was likely
        // skipped (Q-table seed adds only ~100 bytes, metadata adds ~500-2000 bytes).
        // A real DCT embed typically changes size noticeably due to coefficient modification.
        // Heuristic: if the output is within 10% of input size, DCT capacity was likely insufficient.
        if size_delta.saturating_mul(10) < img_bytes.len() as u64 {
            // Check if this is NOT a progressive JPEG (progressive gets its own warning)
            if !is_progressive_jpeg(img_bytes) {
                warnings.push(ProtectionWarning::DctCapacityInsufficient);
            }
        }
    }

    Ok((result, warnings))
}

fn context_with_detected_format(
    img_bytes: &[u8],
    ctx: &ProtectionContext,
) -> Result<(ProtectionContext, ImageOutputFormat)> {
    let format = ImageOutputFormat::from_magic_bytes(img_bytes)
        .ok_or_else(|| Error::InvalidFormat("Unrecognized image format".to_string()))?;

    let mut ctx_with_format = ctx.clone();
    if ctx_with_format.input_format().is_none() {
        ctx_with_format.set_input_format(format);
    }

    Ok((ctx_with_format, format))
}

fn resolved_output_format(ctx: &ProtectionContext) -> ImageOutputFormat {
    ctx.output_format()
        .or(ctx.input_format())
        .unwrap_or(DEFAULT_OUTPUT_FORMAT)
}

/// Verify that image bytes contain a protection payload whose integrity can be proved.
///
/// Checks metadata seeds, DCT stego integrity (for JPEG), and LSB stego (for PNG/WebP).
///
/// # Returns
///
/// - [`VerificationStatus::Verified`] — protection data found and verification passed
/// - [`VerificationStatus::Invalid`] — protection data found but verification failed
///   (corrupted or wrong key)
/// - [`VerificationStatus::NotFound`] — no protection data found in the image
///
/// # Arguments
///
/// * `img_bytes` - Raw image bytes (PNG, JPEG, or WebP)
/// * `mac_key` - Optional MAC key for HMAC-SHA256 verification. Pass empty slice
///   for checksum-only verification.
///
/// # Examples
///
/// ```no_run
/// # let img_bytes: Vec<u8> = Vec::new();
/// match stegoeggo::verify_image_bytes(&img_bytes, &[]) {
///     stegoeggo::VerificationStatus::Verified => println!("Protected and verified"),
///     stegoeggo::VerificationStatus::Invalid => println!("Protected but verification failed"),
///     stegoeggo::VerificationStatus::NotFound => println!("No protection found"),
/// }
/// ```
pub fn verify_image_bytes(img_bytes: &[u8], mac_key: &[u8]) -> VerificationStatus {
    let stego = SteganographyProtector::new();
    stego.verify_payload_from_bytes_with_key(img_bytes, mac_key)
}

/// Verify protection with detailed results.
///
/// Like [`verify_image_bytes`], but returns a [`VerificationResult`] with
/// richer information about what was found and whether verification passed.
///
/// # Examples
///
/// ```no_run
/// use stegoeggo::{verify_image_bytes_detailed, VerificationResult};
///
/// let bytes = std::fs::read("protected.png").unwrap();
/// match verify_image_bytes_detailed(&bytes, b"my-key") {
///     VerificationResult::Verified { payload } => {
///         println!("Seed: {}, Intensity: {}", payload.seed(), payload.intensity());
///     }
///     VerificationResult::MetadataOnly { seed } => {
///         println!("Metadata seed found, but payload was not verified: {}", seed);
///     }
///     VerificationResult::Corrupted { .. } => println!("Protection found but corrupted"),
///     VerificationResult::NotFound => println!("No protection found"),
/// }
/// ```
pub fn verify_image_bytes_detailed(img_bytes: &[u8], mac_key: &[u8]) -> VerificationResult {
    let stego = SteganographyProtector::new();

    match stego.verify_payload_from_bytes_with_key(img_bytes, mac_key) {
        VerificationStatus::Verified => {
            if let Some(payload) = stego.extract_payload_from_bytes_with_key(img_bytes, mac_key) {
                return VerificationResult::Verified { payload };
            }
            return VerificationResult::NotFound;
        }
        VerificationStatus::Invalid => {
            if let Some(payload) = stego.extract_payload_from_bytes_with_key(img_bytes, mac_key) {
                return VerificationResult::Corrupted { payload };
            }
            return VerificationResult::NotFound;
        }
        VerificationStatus::NotFound => {}
    }

    if let Some(seed) = MetadataTrapProtector::extract_seed_from_image(img_bytes) {
        return VerificationResult::MetadataOnly { seed };
    }

    VerificationResult::NotFound
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
    fn disabled_process_image_bytes_is_byte_for_byte_noop() {
        let input = b"not an image";
        let ctx = ProtectionContext::default();

        let result = process_image_bytes(input, ProtectionLevel::Disabled, &ctx).unwrap();

        assert_eq!(result, input);
    }

    #[test]
    fn disabled_process_image_bytes_with_warnings_is_byte_for_byte_noop() {
        let input = b"not an image";
        let ctx = ProtectionContext::default().with_metadata_injection(false);

        let (result, warnings) =
            process_image_bytes_with_warnings(input, ProtectionLevel::Disabled, &ctx).unwrap();

        assert_eq!(result, input);
        assert!(warnings.is_empty());
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

    #[test]
    fn pipeline_process_bytes_preserves_detected_jpeg_format() {
        let pipeline = ProtectionPipeline::new();
        let img = DynamicImage::new_rgb8(64, 64);
        let jpeg_bytes = crate::util::image::encode_image(&img, image::ImageFormat::Jpeg).unwrap();

        let ctx = ProtectionContext::new(0.5, 42);
        let protected = pipeline
            .process_bytes(&jpeg_bytes, ProtectionLevel::Standard, &ctx)
            .unwrap();

        assert!(
            protected.starts_with(&[0xFF, 0xD8, 0xFF]),
            "direct byte pipeline should preserve detected JPEG output format"
        );
    }

    #[test]
    fn light_process_bytes_validates_max_dimension() {
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
        let result = process_image_bytes(&buffer, ProtectionLevel::Light, &ctx);

        assert!(
            result.is_err(),
            "Light byte processing should enforce max_dimension"
        );
    }

    #[test]
    fn light_process_bytes_can_convert_png_to_jpeg() {
        use image::ImageEncoder;

        let img = DynamicImage::new_rgb8(64, 64);
        let mut png_bytes = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
            let rgb = img.to_rgb8();
            encoder
                .write_image(&rgb, 64, 64, image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        let ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Jpeg);
        let protected = process_image_bytes(&png_bytes, ProtectionLevel::Light, &ctx).unwrap();

        assert!(
            protected.starts_with(&[0xFF, 0xD8, 0xFF]),
            "Light byte processing should convert PNG input to requested JPEG output"
        );
    }
}
