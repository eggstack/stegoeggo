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
//!   bounded by `max_origins`.
//!
//! All paths share the same corrected V2 carrier model.

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

/// Maximum tile-grid coordinate probed per axis during tiled recovery.
///
/// Matches the application tiled search domain so generic framed recovery
/// remains compatible with payloads embedded by the current tiled path.
const TILED_MAX_GRID: u32 = 16;

/// Enumerate bounded crop-origin candidates for tiled recovery.
///
/// Origins step by `tile_size / 2` (minimum 1) in row-major order until
/// `max_origins` is reached. Uses saturating arithmetic so untrusted tile
/// sizes cannot overflow the scan.
fn tiled_origins(width: u32, height: u32, tile_size: u32, max_origins: u32) -> Vec<(u32, u32)> {
    let stride = (tile_size / 2).max(1);
    let mut origins = Vec::new();
    let mut y = 0u32;
    while y.saturating_add(tile_size) <= height {
        let mut x = 0u32;
        while x.saturating_add(tile_size) <= width {
            origins.push((x, y));
            if origins.len() as u32 >= max_origins {
                return origins;
            }
            x = x.saturating_add(stride);
            if x == u32::MAX {
                break;
            }
        }
        if origins.len() as u32 >= max_origins {
            break;
        }
        y = y.saturating_add(stride);
        if y == u32::MAX {
            break;
        }
    }
    origins
}

/// Derive the carrier seed for one tiled candidate.
///
/// Mirrors the tiled embed seed derivation (`tile_seed` mixed per tile
/// coordinate, then multiplied by the offset constant with a 5-pass history
/// for compatibility with payloads embedded by the current path).
fn tiled_candidate_seed(master_seed: u64, tile_x: u32, tile_y: u32, pass: u32) -> u64 {
    let local = crate::lsb_internal::tile_seed(master_seed, tile_x, tile_y);
    local.wrapping_mul(crate::constants::STEGO_OFFSET_SEED_1.wrapping_add(pass as u64))
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
    crate::types::validate_max_origins(max_origins)?;
    let payload_bits = payload_len.checked_mul(8).ok_or_else(|| {
        super::StegoError::ResourceLimitExceeded("payload length overflow".to_string())
    })?;
    let tile_size = config.tile_size();
    let (width, height) = img.dimensions();
    if width < tile_size || height < tile_size {
        let required = crate::lsb_internal::lsb_required_capacity_v2(payload_bits, 1);
        return Err(super::StegoError::InsufficientCapacity {
            required,
            available: 0,
        });
    }
    let origins = tiled_origins(width, height, tile_size, max_origins);
    if origins.is_empty() {
        let required = crate::lsb_internal::lsb_required_capacity_v2(payload_bits, 1);
        return Err(super::StegoError::InsufficientCapacity {
            required,
            available: 0,
        });
    }
    for (x0, y0) in origins {
        let sub = crate::lsb_internal::crop_rgba(img, x0, y0, tile_size, tile_size);
        let base_x = x0 / tile_size;
        let base_y = y0 / tile_size;
        for dy in 0..=2u32 {
            if base_y.saturating_add(dy) >= TILED_MAX_GRID {
                break;
            }
            for dx in 0..=2u32 {
                if base_x.saturating_add(dx) >= TILED_MAX_GRID {
                    break;
                }
                for pass in 0..5u32 {
                    let seed = tiled_candidate_seed(config.seed(), base_x + dx, base_y + dy, pass);
                    if let Some(bytes) =
                        crate::lsb_internal::extract_lsb_v2(&sub, payload_bits, seed, 1)
                    {
                        if bytes.len() >= payload_len {
                            return Ok(bytes);
                        }
                    }
                }
            }
        }
    }
    let required = crate::lsb_internal::lsb_required_capacity_v2(payload_bits, 1);
    let available = crate::lsb_internal::lsb_available_slots(tile_size, tile_size).unwrap_or(0);
    Err(super::StegoError::InsufficientCapacity {
        required,
        available,
    })
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
    crate::types::validate_max_origins(max_origins)?;
    let tile_size = config.tile_size();
    let (width, height) = img.dimensions();
    let header_bits = crate::frame::FRAME_HEADER_SIZE
        .checked_mul(8)
        .ok_or_else(|| {
            super::StegoError::ResourceLimitExceeded("frame header size overflow".to_string())
        })?;
    let header_required = crate::lsb_internal::lsb_required_capacity_v2(header_bits, 1);
    if width < tile_size || height < tile_size {
        return Err(super::StegoError::InsufficientCapacity {
            required: header_required,
            available: 0,
        });
    }
    let origins = tiled_origins(width, height, tile_size, max_origins);
    if origins.is_empty() {
        return Err(super::StegoError::InsufficientCapacity {
            required: header_required,
            available: 0,
        });
    }
    let tile_available =
        crate::lsb_internal::lsb_available_slots(tile_size, tile_size).unwrap_or(0);
    for (x0, y0) in origins {
        let sub = crate::lsb_internal::crop_rgba(img, x0, y0, tile_size, tile_size);
        let base_x = x0 / tile_size;
        let base_y = y0 / tile_size;
        for dy in 0..=2u32 {
            if base_y.saturating_add(dy) >= TILED_MAX_GRID {
                break;
            }
            for dx in 0..=2u32 {
                if base_x.saturating_add(dx) >= TILED_MAX_GRID {
                    break;
                }
                for pass in 0..5u32 {
                    let seed = tiled_candidate_seed(config.seed(), base_x + dx, base_y + dy, pass);
                    let Some(prefix) =
                        crate::lsb_internal::extract_lsb_v2(&sub, header_bits, seed, 1)
                    else {
                        continue;
                    };
                    let Ok((_, total_len)) = crate::frame::decode_prefix(&prefix) else {
                        continue;
                    };
                    let Some(total_bits) = total_len.checked_mul(8) else {
                        continue;
                    };
                    let full_required =
                        crate::lsb_internal::lsb_required_capacity_v2(total_bits, 1);
                    if full_required > tile_available {
                        continue;
                    }
                    let Some(framed) =
                        crate::lsb_internal::extract_lsb_v2(&sub, total_bits, seed, 1)
                    else {
                        continue;
                    };
                    if let Ok((_, payload)) = crate::frame::decode(&framed) {
                        return Ok(payload);
                    }
                }
            }
        }
    }
    Err(super::StegoError::FrameNotFound)
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
