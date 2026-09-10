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
//! # Raw vs framed vs in-place vs tiled
//!
//! - **Raw** ([`embed`]/[`extract`]) — caller-supplied payload length and
//!   config. Use when the caller already knows the payload length.
//! - **In-place** ([`embed_in_place`]) — caller's `RgbaImage` buffer is
//!   mutated; no full-image clone. Returns [`InPlaceEmbedReport`].
//! - **Framed** ([`embed_framed`], [`extract_framed`]) — wraps the payload
//!   in a self-describing header with a CRC32 and recovers without caller
//!   knowledge of the original payload length.
//! - **Tiled** ([`embed_tiled`], [`embed_tiled_in_place`], [`extract_tiled`],
//!   [`embed_tiled_framed`], [`extract_tiled_framed`]) — spatial/crop-oriented
//!   repetition: the full payload is embedded in each `tile_size × tile_size`
//!   region with a deterministic tile-local seed, so recovery survives crops
//!   that leave at least one intact tile. Tiled extraction is explicitly
//!   bounded by `max_origins` and reads tile windows in place without
//!   allocating cropped copies.
//!
//! All paths share the same corrected V2 carrier model.
//!
//! Callers with packed or strided RGB/RGBA bytes (camera, video, GUI, FFI
//! buffers) should use the borrowed [`crate::pixels::PixelView`] and
//! [`crate::pixels::PixelViewMut`] views instead of converting into an
//! `RgbaImage`; equivalent pixels use the identical logical carrier mapping.

pub use crate::lsb_internal::{capacity, embed, extract, LsbConfig, DEFAULT_TILE_SIZE};
pub use crate::types::{InPlaceEmbedReport, TileConfig};

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
    let prefix_capacity = capacity(img, crate::frame::FRAME_HEADER_SIZE, config)?;
    if !prefix_capacity.is_sufficient() {
        return Err(super::StegoError::InsufficientCapacity {
            required: prefix_capacity.required,
            available: prefix_capacity.available,
        });
    }

    let prefix = extract(img, crate::frame::FRAME_HEADER_SIZE, config)?;
    let (_, total_len) = crate::frame::decode_prefix(&prefix)?;
    let frame_capacity = capacity(img, total_len, config)?;
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

/// Embed arbitrary bytes once per tile for crop resistance.
///
/// Each `tile_size × tile_size` region embeds the full payload with
/// redundancy 1 using a deterministic tile-local seed; the tile grid itself
/// is the redundancy. Partial edge tiles are skipped. Delegates to the
/// shared in-place tiled core.
///
/// # Errors
///
/// Returns [`super::StegoError::EmptyCarrier`] for a zero-size image.
///
/// ```rust,no_run
/// use image::RgbaImage;
/// use stegoeggo_stego::lsb::{self, TileConfig};
///
/// let image = RgbaImage::new(128, 128);
/// let config = TileConfig::try_new(42, 64).unwrap();
/// let report = lsb::embed_tiled(&image, b"payload", &config).unwrap();
/// assert!(report.embedded);
/// ```
pub fn embed_tiled(
    img: &image::RgbaImage,
    payload: &[u8],
    config: &TileConfig,
) -> Result<super::EmbedReport<image::RgbaImage>, super::StegoError> {
    if img.dimensions() == (0, 0) {
        return Err(super::StegoError::EmptyCarrier);
    }
    let outcome =
        crate::lsb_internal::embed_lsb_tiled(img, payload, config.seed(), config.tile_size());
    let (output, payload_bytes, required_capacity, available_capacity, embedded) = match outcome {
        crate::types::EmbedOutcome::Embedded {
            output,
            payload_bytes,
            required_capacity,
            available_capacity,
            ..
        } => (
            output,
            payload_bytes,
            required_capacity,
            available_capacity,
            true,
        ),
        crate::types::EmbedOutcome::SkippedCapacity {
            output,
            payload_bytes,
            required_capacity,
            available_capacity,
            ..
        } => (
            output,
            payload_bytes,
            required_capacity,
            available_capacity,
            false,
        ),
        crate::types::EmbedOutcome::UnsupportedProgressive { output } => {
            return Ok(super::EmbedReport {
                embedded: false,
                output,
                payload_bytes: payload.len(),
                required_capacity: 0,
                available_capacity: 0,
                actual_redundancy: 0,
            });
        }
    };
    Ok(super::EmbedReport {
        embedded,
        output,
        payload_bytes,
        required_capacity,
        available_capacity,
        actual_redundancy: u8::from(embedded) as usize,
    })
}

/// Embed arbitrary bytes once per tile in place.
///
/// Mutates the caller's buffer with no full-image clone. Capacity is checked
/// before the first pixel mutation, so an insufficient carrier is left
/// unchanged.
///
/// # Errors
///
/// Returns [`super::StegoError::EmptyCarrier`] for a zero-size image.
///
/// ```rust,no_run
/// use image::RgbaImage;
/// use stegoeggo_stego::lsb::{self, TileConfig};
///
/// let mut image = RgbaImage::new(128, 128);
/// let config = TileConfig::try_new(42, 64).unwrap();
/// let report = lsb::embed_tiled_in_place(&mut image, b"payload", &config).unwrap();
/// assert!(report.embedded);
/// ```
pub fn embed_tiled_in_place(
    img: &mut image::RgbaImage,
    payload: &[u8],
    config: &TileConfig,
) -> Result<InPlaceEmbedReport, super::StegoError> {
    if img.dimensions() == (0, 0) {
        return Err(super::StegoError::EmptyCarrier);
    }
    Ok(crate::lsb_internal::embed_lsb_tiled_in_place(
        img,
        payload,
        config.seed(),
        config.tile_size(),
    ))
}

/// Extract tiled payload bytes when the payload length is known.
///
/// Searches at most `max_origins` crop origins (stride `tile_size / 2`) and,
/// per origin, the tile-grid neighbourhood (`0..=2`) with the 5-pass seed
/// history used by the current tiled path. Returns the first candidate in
/// deterministic scan order.
///
/// Raw mode cannot authenticate correctness: without frame integrity any
/// candidate with sufficient capacity yields bytes, so a misaligned crop may
/// return bytes from the wrong tile. Prefer [`extract_tiled_framed`] for
/// self-validating crop recovery.
///
/// # Errors
///
/// Returns [`super::StegoError::InvalidConfig`] if `max_origins` is `0` or
/// above [`crate::MAX_TILED_ORIGINS`]. Returns
/// [`super::StegoError::InsufficientCapacity`] when no tile can hold the
/// payload.
pub fn extract_tiled(
    img: &image::RgbaImage,
    payload_len: usize,
    config: &TileConfig,
    max_origins: u32,
) -> Result<Vec<u8>, super::StegoError> {
    crate::lsb_internal::extract_tiled_carrier(
        img,
        payload_len,
        config.seed(),
        config.tile_size(),
        max_origins,
    )
}

/// Embed a self-describing framed payload once per tile.
///
/// The frame is encoded with [`crate::frame::encode`] and then embedded with
/// [`embed_tiled`]. The report's `payload_bytes` includes frame overhead.
pub fn embed_tiled_framed(
    img: &image::RgbaImage,
    payload: &[u8],
    config: &TileConfig,
) -> Result<super::EmbedReport<image::RgbaImage>, super::StegoError> {
    let framed = crate::frame::encode(payload)?;
    embed_tiled(img, &framed, config)
}

/// Extract and validate a framed tiled payload without caller-known length.
///
/// Recovers the frame header with bounded search, validates the declared
/// length against frame bounds and per-tile carrier capacity before full
/// extraction, then recovers the full frame from the same candidate identity
/// (origin, tile-grid seed, pass) and validates CRC32. Returns only payload
/// bytes.
///
/// # Errors
///
/// Returns [`super::StegoError::InvalidConfig`] for an out-of-range
/// `max_origins`. Returns [`super::StegoError::InsufficientCapacity`] when
/// no tile can hold even the frame header. Returns
/// [`super::StegoError::FrameNotFound`] when the bounded search finds no
/// valid frame.
pub fn extract_tiled_framed(
    img: &image::RgbaImage,
    config: &TileConfig,
    max_origins: u32,
) -> Result<Vec<u8>, super::StegoError> {
    crate::lsb_internal::extract_tiled_framed_carrier(
        img,
        config.seed(),
        config.tile_size(),
        max_origins,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_image(width: u32, height: u32) -> image::RgbaImage {
        image::RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([
                ((x * 7 + y * 13) % 256) as u8,
                ((x * 11 + y * 3) % 256) as u8,
                ((x * 5 + y * 17) % 256) as u8,
                255,
            ])
        })
    }

    #[test]
    fn tile_config_rejects_zero_size() {
        assert!(TileConfig::try_new(42, 0).is_err());
        let config = TileConfig::try_new(42, 64).unwrap();
        assert_eq!(config.seed(), 42);
        assert_eq!(config.tile_size(), 64);
    }

    #[test]
    fn tiled_raw_roundtrip() {
        let img = uniform_image(128, 128);
        let payload = vec![0xA5; 36];
        let config = TileConfig::try_new(42, 64).unwrap();
        let report = embed_tiled(&img, &payload, &config).unwrap();
        assert!(report.embedded);
        let recovered = extract_tiled(&report.output, payload.len(), &config, 64).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn tiled_in_place_roundtrip_matches_cloning() {
        let img = uniform_image(128, 128);
        let payload = vec![0x3C; 24];
        let config = TileConfig::try_new(7, 64).unwrap();
        let cloned = embed_tiled(&img, &payload, &config).unwrap();
        let mut inplace = img.clone();
        let report = embed_tiled_in_place(&mut inplace, &payload, &config).unwrap();
        assert!(report.embedded);
        assert_eq!(inplace, cloned.output);
        let recovered = extract_tiled(&inplace, payload.len(), &config, 64).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn tiled_in_place_leaves_carrier_unchanged_on_insufficient_capacity() {
        let tiny = uniform_image(40, 40);
        let mut untouched = tiny.clone();
        let config = TileConfig::try_new(42, 64).unwrap();
        let report = embed_tiled_in_place(&mut untouched, &[0xA5; 36], &config).unwrap();
        assert!(!report.embedded);
        assert_eq!(untouched, tiny);
    }

    #[test]
    fn tiled_framed_recovers_without_length() {
        let img = uniform_image(128, 128);
        let payload = b"framed tiled recovery";
        let config = TileConfig::try_new(42, 64).unwrap();
        let report = embed_tiled_framed(&img, payload, &config).unwrap();
        assert!(report.embedded);
        let recovered = extract_tiled_framed(&report.output, &config, 64).unwrap();
        assert_eq!(&recovered, payload);
    }

    #[test]
    fn tiled_framed_survives_aligned_crop() {
        let img = uniform_image(128, 128);
        let payload = b"crop survival";
        let config = TileConfig::try_new(42, 64).unwrap();
        let report = embed_tiled_framed(&img, payload, &config).unwrap();
        assert!(report.embedded);
        let cropped = crate::lsb_internal::crop_rgba(&report.output, 64, 0, 64, 64);
        let recovered = extract_tiled_framed(&cropped, &config, 64).unwrap();
        assert_eq!(&recovered, payload);
    }

    #[test]
    fn tiled_extraction_rejects_zero_origins() {
        let img = uniform_image(128, 128);
        let config = TileConfig::try_new(42, 64).unwrap();
        assert!(extract_tiled(&img, 4, &config, 0).is_err());
        assert!(extract_tiled_framed(&img, &config, 0).is_err());
    }
}
