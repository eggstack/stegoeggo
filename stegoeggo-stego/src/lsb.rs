//! Public pixel-domain LSB steganography carrier.
//!
//! Application-neutral embed/extract and capacity API on top of the
//! crate-internal mechanics in `crate::lsb_internal`. Low-level helpers
//! (permutations, slot mappings, byte/bit conversions) are intentionally
//! not re-exported; they are implementation details.
//!
//! # Capacity units
//!
//! Capacity is reported in **RGB carrier slots** (`width * height * 3`).
//! The alpha channel is never a carrier. Each embedded payload bit
//! occupies `STEGO_SPREAD_FACTOR * redundancy` slots. The cloned
//! [`embed`] and in-place [`embed_in_place`] paths share the same corrected
//! V2 mutation core.
//!
//! # Raw vs framed vs in-place
//!
//! - **Raw** ([`embed`]/[`extract`]) — caller-supplied payload length and
//!   config. Use when the caller already knows the payload length.
//! - **In-place** ([`embed_in_place`]) — caller's `RgbaImage` buffer is
//!   mutated; no full-image clone. Returns [`InPlaceEmbedReport`].
//! - **Framed** ([`embed_framed`], [`extract_framed`]) — wraps the payload
//!   in a self-describing header with a CRC32 and recovers without caller
//!   knowledge of the original payload length.
//!
//! All three paths share the same corrected V2 carrier model.

pub use crate::lsb_internal::{capacity, embed, extract, LsbConfig, DEFAULT_TILE_SIZE};
pub use crate::types::InPlaceEmbedReport;

/// Embed arbitrary bytes into an RGBA image in place using V2 corrected LSB.
///
/// The caller retains ownership of the image, and the operation performs no
/// full-image clone. If capacity is insufficient, the image is unchanged and
/// the returned report has `embedded == false`.
///
/// ```rust,no_run
/// use image::RgbaImage;
/// use stegoeggo_stego::lsb::{self, LsbConfig};
///
/// let mut image = RgbaImage::new(100, 100);
/// let config = LsbConfig::new(42);
/// let report = lsb::embed_in_place(&mut image, b"payload", &config).unwrap();
/// assert!(report.embedded);
/// ```
pub fn embed_in_place(
    img: &mut image::RgbaImage,
    payload: &[u8],
    config: &LsbConfig,
) -> Result<InPlaceEmbedReport, super::StegoError> {
    crate::lsb_internal::embed_in_place(img, payload, config)
}

/// Embed a self-describing framed payload into an RGBA image.
///
/// The frame is encoded with [`crate::frame::encode`] and then embedded using
/// the raw LSB carrier. The returned report's `payload_bytes` includes the
/// frame header and CRC overhead because those bytes are placed in the
/// carrier.
///
/// # Examples
///
/// ```rust,no_run
/// use image::RgbaImage;
/// use stegoeggo_stego::lsb::{self, LsbConfig};
///
/// let image = RgbaImage::new(100, 100);
/// let config = LsbConfig::new(42);
/// let report = lsb::embed_framed(&image, b"payload", &config).unwrap();
/// let recovered = lsb::extract_framed(&report.output, &config).unwrap();
/// assert_eq!(recovered, b"payload");
/// ```
pub fn embed_framed(
    img: &image::RgbaImage,
    payload: &[u8],
    config: &LsbConfig,
) -> Result<super::EmbedReport<image::RgbaImage>, super::StegoError> {
    let framed = crate::frame::encode(payload)?;
    embed(img, &framed, config)
}

/// Extract and validate a self-describing framed payload from an RGBA image.
///
/// The carrier is read first for the fixed frame header. The declared frame
/// length is validated before the full extraction buffer is allocated or
/// requested from the carrier. The returned bytes are the frame payload, not
/// the encoded frame.
pub fn extract_framed(
    img: &image::RgbaImage,
    config: &LsbConfig,
) -> Result<Vec<u8>, super::StegoError> {
    let prefix_capacity = capacity(img, crate::frame::FRAME_HEADER_SIZE, config);
    if !prefix_capacity.is_sufficient() {
        return Err(super::StegoError::InsufficientCapacity {
            required: prefix_capacity.required,
            available: prefix_capacity.available,
        });
    }

    let prefix = extract(img, crate::frame::FRAME_HEADER_SIZE, config)?;
    let (_, total_len) = crate::frame::decode_prefix(&prefix)?;
    let frame_capacity = capacity(img, total_len, config);
    if !frame_capacity.is_sufficient() {
        return Err(super::StegoError::InsufficientCapacity {
            required: frame_capacity.required,
            available: frame_capacity.available,
        });
    }

    let framed = extract(img, total_len, config)?;
    let (_, payload) = crate::frame::decode(&framed)?;
    Ok(payload)
}
