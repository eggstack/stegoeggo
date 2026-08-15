//! Public pixel-domain LSB steganography carrier.
//!
//! Application-neutral embed/extract and capacity API on top of the
//! crate-internal mechanics in [`crate::lsb_internal`]. Low-level helpers
//! (permutations, slot mappings, byte/bit conversions) are intentionally
//! not re-exported; they are implementation details.

pub use crate::lsb_internal::{capacity, embed, extract, LsbConfig, DEFAULT_TILE_SIZE};

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
