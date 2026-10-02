//! Still-lossless WebP encoded-byte carrier (feature `webp`, off by default).
//!
//! Convenience facade that decodes a still VP8L file to RGBA, embeds or
//! extracts through the existing LSB pixel carrier, and re-encodes
//! losslessly with [`WebPEncoder::new_lossless`]. Permutation,
//! slot-mapping, framing, and tiling logic are reused from
//! [`crate::lsb`]; nothing is forked here.
//!
//! Only still lossless WebP is accepted. Lossy VP8 and animated images
//! are rejected with a structured [`StegoError::UnsupportedWebP`]
//! classification, never silently transcoded into the lossless carrier.
//! Every operation takes a [`CarrierLimits`] that bounds input bytes,
//! decoded dimensions, framed sizes, and tiled-search extent before any
//! large allocation.
//!
//! Output is a newly encoded lossless carrier. Unrelated container
//! metadata (ICC, EXIF, XMP) is not preserved and no preservation is
//! claimed. Rights metadata rendering stays with the parent crate; its
//! byte paths never switch to this facade implicitly.
//!
//! # Examples
//!
//! ```rust,no_run
//! use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage};
//! use stegoeggo_stego::lsb::LsbConfig;
//! use stegoeggo_stego::{webp, CarrierLimits};
//!
//! let image = RgbaImage::from_pixel(128, 128, Rgba([120, 130, 140, 255]));
//! let mut encoded = Vec::new();
//! image::codecs::webp::WebPEncoder::new_lossless(&mut encoded)
//!     .write_image(image.as_raw(), 128, 128, ExtendedColorType::Rgba8)
//!     .unwrap();
//! let limits = CarrierLimits::default();
//! let report = webp::embed(&encoded, b"payload", &LsbConfig::new(42), &limits).unwrap();
//! assert!(report.embedded);
//! let recovered = webp::extract(&report.output, b"payload".len(), &LsbConfig::new(42), &limits)
//!     .unwrap();
//! assert_eq!(&recovered, b"payload");
//! ```

use std::io::Cursor;

use image::codecs::webp::{WebPDecoder, WebPEncoder};
use image::{ExtendedColorType, ImageDecoder, ImageEncoder, RgbaImage};

use crate::error::{StegoError, WebpUnsupportedReason};
use crate::frame;
use crate::lsb::{self, LsbConfig};
use crate::types::TileConfig;
use crate::{CapacityReport, CarrierLimits, EmbedReport};

const CHUNK_ANMF: [u8; 4] = *b"ANMF";
const CHUNK_VP8: [u8; 4] = *b"VP8 ";
const CHUNK_VP8L: [u8; 4] = *b"VP8L";
const CHUNK_VP8X: [u8; 4] = *b"VP8X";
const RIFF_MAGIC: [u8; 4] = *b"RIFF";
const WEBP_MAGIC: [u8; 4] = *b"WEBP";
const VP8X_ANIMATION_FLAG: u8 = 0x02;
const VP8X_HEADER_LEN: usize = 10;
const VP8L_HEADER_LEN: usize = 5;
const VP8L_SIGNATURE: u8 = 0x2F;

/// Classification of still-lossless WebP support for an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebpSupport {
    /// Still VP8L image data usable by this facade.
    StillLossless {
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
        /// Whether the bitstream carries an alpha channel.
        has_alpha: bool,
    },
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|window| u32::from_le_bytes([window[0], window[1], window[2], window[3]]))
}

fn malformed(detail: &str) -> StegoError {
    StegoError::MalformedInput(format!("invalid WebP container: {detail}"))
}

fn chunk_at(
    data: &[u8],
    offset: usize,
    bound: usize,
) -> Result<([u8; 4], u32, usize, usize), StegoError> {
    let header = data
        .get(offset..offset + 8)
        .ok_or_else(|| malformed("truncated chunk header"))?;
    let size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    let data_start = offset + 8;
    let data_end = data_start
        .checked_add(size as usize)
        .ok_or_else(|| malformed("chunk size overflows"))?;
    if data_end > bound {
        return Err(malformed("truncated chunk data"));
    }
    Ok((
        [header[0], header[1], header[2], header[3]],
        size,
        data_start,
        data_end,
    ))
}

fn chunk_next(data_end: usize, size: u32, bound: usize) -> Result<Option<usize>, StegoError> {
    let next = data_end
        .checked_add((size & 1) as usize)
        .ok_or_else(|| malformed("chunk end overflows"))?;
    if next > bound {
        if data_end == bound {
            return Ok(None);
        }
        return Err(malformed("chunk overruns its container"));
    }
    if next == bound {
        return Ok(None);
    }
    Ok(Some(next))
}

fn vp8l_support(
    data: &[u8],
    data_start: usize,
    size: u32,
    limits: &CarrierLimits,
) -> Result<WebpSupport, StegoError> {
    if (size as usize) < VP8L_HEADER_LEN {
        return Err(malformed("truncated VP8L header"));
    }
    if data[data_start] != VP8L_SIGNATURE {
        return Err(malformed("bad VP8L signature"));
    }
    let bits =
        read_u32_le(data, data_start + 1).ok_or_else(|| malformed("truncated VP8L header"))?;
    if bits >> 29 != 0 {
        return Err(malformed("unsupported VP8L version"));
    }
    let width = (bits & 0x3FFF) + 1;
    let height = ((bits >> 14) & 0x3FFF) + 1;
    let has_alpha = (bits >> 28) & 1 == 1;
    limits.check_dimensions(width, height)?;
    Ok(WebpSupport::StillLossless {
        width,
        height,
        has_alpha,
    })
}

fn read_u24_le(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 3).map(|window| {
        u32::from(window[0]) | (u32::from(window[1]) << 8) | (u32::from(window[2]) << 16)
    })
}

fn extended_support(
    data: &[u8],
    data_start: usize,
    data_end: usize,
    size: u32,
    limits: &CarrierLimits,
) -> Result<WebpSupport, StegoError> {
    if data_end - data_start < VP8X_HEADER_LEN {
        return Err(malformed("truncated VP8X header"));
    }
    if data[data_start] & VP8X_ANIMATION_FLAG != 0 {
        return Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::Animated));
    }
    let canvas_width =
        read_u24_le(data, data_start + 4).ok_or_else(|| malformed("truncated VP8X header"))? + 1;
    let canvas_height =
        read_u24_le(data, data_start + 7).ok_or_else(|| malformed("truncated VP8X header"))? + 1;
    limits.check_dimensions(canvas_width, canvas_height)?;
    let mut offset = match chunk_next(data_end, size, data.len())? {
        Some(next) => next,
        None => {
            return Err(StegoError::UnsupportedWebP(
                WebpUnsupportedReason::MissingLosslessImageData,
            ));
        }
    };
    let mut found = None;
    loop {
        let (fourcc, chunk_size, chunk_data, chunk_end) = chunk_at(data, offset, data.len())?;
        if fourcc == CHUNK_VP8 {
            return Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::LossyVp8));
        }
        if fourcc == CHUNK_ANMF {
            return Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::Animated));
        }
        if fourcc == CHUNK_VP8L {
            if found.is_some() {
                return Err(malformed("multiple WebP image chunks"));
            }
            found = Some(vp8l_support(data, chunk_data, chunk_size, limits)?);
        }
        match chunk_next(chunk_end, chunk_size, data.len())? {
            Some(next) => offset = next,
            None => break,
        }
    }
    found.ok_or(StegoError::UnsupportedWebP(
        WebpUnsupportedReason::MissingLosslessImageData,
    ))
}

fn probe_still(data: &[u8], limits: &CarrierLimits) -> Result<WebpSupport, StegoError> {
    limits.check_input_bytes(data.len())?;
    if data.len() < 12 || data[0..4] != RIFF_MAGIC || data[8..12] != WEBP_MAGIC {
        return Err(malformed("not a RIFF/WEBP container"));
    }
    let riff_size = read_u32_le(data, 4).ok_or_else(|| malformed("truncated RIFF header"))?;
    if riff_size as u64 + 8 != data.len() as u64 {
        return Err(malformed("RIFF size does not match input length"));
    }
    let (fourcc, size, data_start, data_end) = chunk_at(data, 12, data.len())?;
    let support = if fourcc == CHUNK_VP8 {
        return Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::LossyVp8));
    } else if fourcc == CHUNK_VP8L {
        vp8l_support(data, data_start, size, limits)?
    } else if fourcc == CHUNK_VP8X {
        return extended_support(data, data_start, data_end, size, limits);
    } else {
        return Err(malformed("unsupported first WebP chunk"));
    };
    if chunk_next(data_end, size, data.len())?.is_some() {
        return Err(malformed("trailing data after WebP image chunk"));
    }
    Ok(support)
}

fn decode_still(data: &[u8], limits: &CarrierLimits) -> Result<RgbaImage, StegoError> {
    probe_still(data, limits)?;
    let decoder = WebPDecoder::new(Cursor::new(data))
        .map_err(|error| StegoError::MalformedInput(format!("WebP decode failed: {error}")))?;
    if decoder.has_animation() {
        return Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::Animated));
    }
    let (width, height) = decoder.dimensions();
    limits.check_dimensions(width, height)?;
    let color = decoder.color_type();
    let total = usize::try_from(decoder.total_bytes())
        .map_err(|_| StegoError::MalformedInput("WebP image too large".to_string()))?;
    let mut bytes = vec![0u8; total];
    decoder
        .read_image(&mut bytes)
        .map_err(|error| StegoError::MalformedInput(format!("WebP decode failed: {error}")))?;
    let mismatch = || StegoError::MalformedInput("WebP pixel buffer mismatch".to_string());
    match color {
        image::ColorType::Rgba8 => RgbaImage::from_raw(width, height, bytes).ok_or_else(mismatch),
        image::ColorType::Rgb8 => image::RgbImage::from_raw(width, height, bytes)
            .map(|rgb| image::DynamicImage::ImageRgb8(rgb).to_rgba8())
            .ok_or_else(mismatch),
        _ => Err(StegoError::MalformedInput(
            "unexpected WebP color type".to_string(),
        )),
    }
}

fn encode_still(image: &RgbaImage) -> Result<Vec<u8>, StegoError> {
    let mut output = Vec::new();
    WebPEncoder::new_lossless(&mut output)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|error| StegoError::MalformedInput(format!("WebP encode failed: {error}")))?;
    if output.len() < 16 || output[0..4] != RIFF_MAGIC || output[8..16] != *b"WEBPVP8L" {
        return Err(StegoError::MalformedInput(
            "WebP encoder did not emit still VP8L".to_string(),
        ));
    }
    Ok(output)
}

fn finish(report: EmbedReport<RgbaImage>) -> Result<EmbedReport, StegoError> {
    Ok(EmbedReport {
        embedded: report.embedded,
        output: encode_still(&report.output)?,
        payload_bytes: report.payload_bytes,
        required_capacity: report.required_capacity,
        available_capacity: report.available_capacity,
        actual_redundancy: report.actual_redundancy,
    })
}

/// Probe whether WebP bytes are a still lossless image usable by this facade.
///
/// Bounds input bytes and declared dimensions before classification.
/// Lossy VP8 and animated inputs are classified, never transcoded.
///
/// # Errors
///
/// Returns [`StegoError::MalformedInput`] for non-WebP, truncated, or
/// structurally invalid input, [`StegoError::UnsupportedWebP`] for
/// well-formed lossy/animated/lossless-less input, and
/// [`StegoError::ResourceLimitExceeded`] when input or declared
/// dimensions exceed `limits`.
pub fn probe_support(webp_bytes: &[u8], limits: &CarrierLimits) -> Result<WebpSupport, StegoError> {
    probe_still(webp_bytes, limits)
}

/// Query raw LSB capacity of the decoded still image for `payload_len` bytes.
pub fn capacity(
    webp_bytes: &[u8],
    payload_len: usize,
    config: &LsbConfig,
    limits: &CarrierLimits,
) -> Result<CapacityReport, StegoError> {
    let image = decode_still(webp_bytes, limits)?;
    lsb::capacity(&image, payload_len, config)
}

/// Embed raw bytes when the payload length is known to the extractor.
///
/// Decodes, embeds through the LSB carrier, and re-encodes losslessly.
/// All-or-error: failures return [`StegoError`] with no partial output.
///
/// # Errors
///
/// Returns [`StegoError::EmptyCarrier`] for a zero-size image and
/// [`StegoError::InsufficientCapacity`] when the payload does not fit,
/// matching [`lsb::embed`] semantics on the decoded pixels.
pub fn embed(
    webp_bytes: &[u8],
    payload: &[u8],
    config: &LsbConfig,
    limits: &CarrierLimits,
) -> Result<EmbedReport, StegoError> {
    let image = decode_still(webp_bytes, limits)?;
    finish(lsb::embed(&image, payload, config)?)
}

/// Extract raw bytes when the payload length is known.
///
/// Pass the `actual_redundancy` from the embed report in `config`; see
/// [`lsb::extract`].
pub fn extract(
    webp_bytes: &[u8],
    payload_len: usize,
    config: &LsbConfig,
    limits: &CarrierLimits,
) -> Result<Vec<u8>, StegoError> {
    let image = decode_still(webp_bytes, limits)?;
    lsb::extract(&image, payload_len, config)
}

/// Embed a self-describing framed payload; the report's `payload_bytes`
/// includes frame overhead.
pub fn embed_framed(
    webp_bytes: &[u8],
    payload: &[u8],
    config: &LsbConfig,
    limits: &CarrierLimits,
) -> Result<EmbedReport, StegoError> {
    let framed = frame::encode(payload)?;
    embed(webp_bytes, &framed, config, limits)
}

/// Extract and validate a framed payload without caller-known length.
///
/// Validates the declared frame length against frame bounds and carrier
/// capacity before full extraction, matching [`lsb::extract_framed`].
///
/// # Errors
///
/// Returns [`StegoError::InsufficientCapacity`] when the frame header or
/// the declared frame does not fit the decoded carrier.
pub fn extract_framed(
    webp_bytes: &[u8],
    config: &LsbConfig,
    limits: &CarrierLimits,
) -> Result<Vec<u8>, StegoError> {
    let image = decode_still(webp_bytes, limits)?;
    let prefix_capacity = lsb::capacity(&image, frame::FRAME_HEADER_SIZE, config)?;
    if !prefix_capacity.is_sufficient() {
        return Err(StegoError::InsufficientCapacity {
            required: prefix_capacity.required,
            available: prefix_capacity.available,
        });
    }
    let prefix = lsb::extract(&image, frame::FRAME_HEADER_SIZE, config)?;
    let (_, total_len) = frame::decode_prefix(&prefix)?;
    limits.check_frame_bytes(total_len)?;
    let frame_capacity = lsb::capacity(&image, total_len, config)?;
    if !frame_capacity.is_sufficient() {
        return Err(StegoError::InsufficientCapacity {
            required: frame_capacity.required,
            available: frame_capacity.available,
        });
    }
    let framed = lsb::extract(&image, total_len, config)?;
    let (_, payload) = frame::decode(&framed)?;
    Ok(payload)
}

/// Embed raw bytes once per tile for crop resistance.
///
/// Each full `tile_size x tile_size` region embeds the full payload with
/// redundancy 1; the tile grid itself is the redundancy. Delegates to
/// [`lsb::embed_tiled`].
pub fn embed_tiled(
    webp_bytes: &[u8],
    payload: &[u8],
    tile: &TileConfig,
    limits: &CarrierLimits,
) -> Result<EmbedReport, StegoError> {
    let image = decode_still(webp_bytes, limits)?;
    finish(lsb::embed_tiled(&image, payload, tile)?)
}

/// Extract tiled raw bytes, searching at most `max_origins` crop origins.
///
/// `max_origins` is checked against `limits` before decoding. Prefer
/// [`extract_tiled_framed`] for self-validating crop recovery.
pub fn extract_tiled(
    webp_bytes: &[u8],
    payload_len: usize,
    tile: &TileConfig,
    max_origins: u32,
    limits: &CarrierLimits,
) -> Result<Vec<u8>, StegoError> {
    limits.check_tiled_origins(max_origins)?;
    let image = decode_still(webp_bytes, limits)?;
    lsb::extract_tiled(&image, payload_len, tile, max_origins)
}

/// Embed a self-describing framed payload once per tile.
pub fn embed_tiled_framed(
    webp_bytes: &[u8],
    payload: &[u8],
    tile: &TileConfig,
    limits: &CarrierLimits,
) -> Result<EmbedReport, StegoError> {
    let framed = frame::encode(payload)?;
    embed_tiled(webp_bytes, &framed, tile, limits)
}

/// Extract and validate a framed tiled payload without caller-known length.
///
/// `max_origins` is checked against `limits` before decoding.
pub fn extract_tiled_framed(
    webp_bytes: &[u8],
    tile: &TileConfig,
    max_origins: u32,
    limits: &CarrierLimits,
) -> Result<Vec<u8>, StegoError> {
    limits.check_tiled_origins(max_origins)?;
    let image = decode_still(webp_bytes, limits)?;
    lsb::extract_tiled_framed(&image, tile, max_origins)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> CarrierLimits {
        CarrierLimits::default()
    }

    fn riff_wrap(chunks: &[u8]) -> Vec<u8> {
        let mut out = Vec::from(&b"RIFF"[..]);
        let size = u32::try_from(chunks.len() + 4).unwrap();
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(b"WEBP");
        out.extend_from_slice(chunks);
        out
    }

    fn chunk(fourcc: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::from(&fourcc[..]);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        if data.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    fn vp8l_data(width_minus_one: u16, height_minus_one: u16) -> Vec<u8> {
        let bits = u32::from(width_minus_one) | (u32::from(height_minus_one) << 14) | (1 << 28);
        let mut out = vec![VP8L_SIGNATURE];
        out.extend_from_slice(&bits.to_le_bytes());
        out.push(0);
        out
    }

    fn vp8x_file(flags: u8, siblings: &[u8]) -> Vec<u8> {
        let header = [flags, 0, 0, 0, 7, 0, 0, 7, 0, 0];
        let mut chunks = chunk(b"VP8X", &header);
        chunks.extend_from_slice(siblings);
        riff_wrap(&chunks)
    }

    #[test]
    fn probe_rejects_short_input() {
        assert!(matches!(
            probe_support(b"RIFF", &limits()),
            Err(StegoError::MalformedInput(_))
        ));
    }

    #[test]
    fn probe_rejects_wrong_magic() {
        let mut data = riff_wrap(&chunk(b"VP8L", &vp8l_data(7, 7)));
        data[0] = b'X';
        assert!(matches!(
            probe_support(&data, &limits()),
            Err(StegoError::MalformedInput(_))
        ));
    }

    #[test]
    fn probe_rejects_riff_size_mismatch() {
        let mut data = riff_wrap(&chunk(b"VP8L", &vp8l_data(7, 7)));
        data.truncate(data.len() - 1);
        assert!(matches!(
            probe_support(&data, &limits()),
            Err(StegoError::MalformedInput(_))
        ));
    }

    #[test]
    fn probe_classifies_lossy_vp8() {
        let data = riff_wrap(&chunk(b"VP8 ", &[0x9D, 0x01, 0x2A]));
        assert!(matches!(
            probe_support(&data, &limits()),
            Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::LossyVp8))
        ));
    }

    #[test]
    fn probe_accepts_plain_vp8l() {
        let data = riff_wrap(&chunk(b"VP8L", &vp8l_data(7, 7)));
        match probe_support(&data, &limits()) {
            Ok(WebpSupport::StillLossless {
                width,
                height,
                has_alpha,
            }) => assert_eq!((width, height, has_alpha), (8, 8, true)),
            other => panic!("unexpected probe result: {other:?}"),
        }
    }

    #[test]
    fn probe_rejects_bad_vp8l_signature() {
        let mut payload = vp8l_data(7, 7);
        payload[0] = 0x2E;
        let data = riff_wrap(&chunk(b"VP8L", &payload));
        assert!(matches!(
            probe_support(&data, &limits()),
            Err(StegoError::MalformedInput(_))
        ));
    }

    #[test]
    fn probe_rejects_truncated_chunk_data() {
        let mut data = riff_wrap(&chunk(b"VP8L", &vp8l_data(7, 7)));
        let len = data.len();
        data[12 + 4..12 + 8].copy_from_slice(&u32::to_le_bytes(64));
        data.truncate(len);
        let mut fixed = Vec::from(&b"RIFF"[..]);
        fixed.extend_from_slice(&u32::try_from(data.len() - 8).unwrap().to_le_bytes());
        fixed.extend_from_slice(&data[8..]);
        assert!(matches!(
            probe_support(&fixed, &limits()),
            Err(StegoError::MalformedInput(_))
        ));
    }

    #[test]
    fn probe_rejects_animated_vp8x() {
        let siblings = chunk(b"VP8L", &vp8l_data(7, 7));
        let data = vp8x_file(0x02, &siblings);
        assert!(matches!(
            probe_support(&data, &limits()),
            Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::Animated))
        ));
    }

    #[test]
    fn probe_rejects_animation_frames_without_flag() {
        let mut siblings = chunk(b"ANMF", &[0u8; 24]);
        siblings.extend_from_slice(&chunk(b"VP8L", &vp8l_data(7, 7)));
        let data = vp8x_file(0x00, &siblings);
        assert!(matches!(
            probe_support(&data, &limits()),
            Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::Animated))
        ));
    }

    #[test]
    fn probe_accepts_still_vp8x_with_vp8l() {
        let siblings = chunk(b"VP8L", &vp8l_data(7, 7));
        let data = vp8x_file(0x00, &siblings);
        match probe_support(&data, &limits()) {
            Ok(WebpSupport::StillLossless {
                width,
                height,
                has_alpha,
            }) => assert_eq!((width, height, has_alpha), (8, 8, true)),
            other => panic!("unexpected probe result: {other:?}"),
        }
    }

    #[test]
    fn probe_skips_metadata_chunks_in_vp8x() {
        let mut siblings = chunk(b"ICCP", &[0u8; 8]);
        siblings.extend_from_slice(&chunk(b"VP8L", &vp8l_data(7, 7)));
        siblings.extend_from_slice(&chunk(b"XMP ", &[0u8; 7]));
        let data = vp8x_file(0x2C, &siblings);
        match probe_support(&data, &limits()) {
            Ok(WebpSupport::StillLossless {
                width,
                height,
                has_alpha,
            }) => assert_eq!((width, height, has_alpha), (8, 8, true)),
            other => panic!("unexpected probe result: {other:?}"),
        }
    }

    #[test]
    fn probe_rejects_vp8x_without_lossless_data() {
        let data = vp8x_file(0x10, &[]);
        assert!(matches!(
            probe_support(&data, &limits()),
            Err(StegoError::UnsupportedWebP(
                WebpUnsupportedReason::MissingLosslessImageData
            ))
        ));
    }

    #[test]
    fn probe_rejects_lossy_inside_vp8x() {
        let siblings = chunk(b"VP8 ", &[0x9D, 0x01, 0x2A]);
        let data = vp8x_file(0x10, &siblings);
        assert!(matches!(
            probe_support(&data, &limits()),
            Err(StegoError::UnsupportedWebP(WebpUnsupportedReason::LossyVp8))
        ));
    }

    #[test]
    fn probe_rejects_unknown_first_chunk() {
        let data = riff_wrap(&chunk(b"ICCP", &[0u8; 4]));
        assert!(matches!(
            probe_support(&data, &limits()),
            Err(StegoError::MalformedInput(_))
        ));
    }

    #[test]
    fn probe_enforces_input_limit() {
        let data = riff_wrap(&chunk(b"VP8L", &vp8l_data(7, 7)));
        let tight = CarrierLimits::builder().max_input_bytes(16).build();
        assert!(matches!(
            probe_support(&data, &tight),
            Err(StegoError::ResourceLimitExceeded(_))
        ));
    }

    #[test]
    fn probe_enforces_dimension_limits() {
        let data = riff_wrap(&chunk(b"VP8L", &vp8l_data(300, 300)));
        let tight = CarrierLimits::builder().max_width(64).build();
        assert!(matches!(
            probe_support(&data, &tight),
            Err(StegoError::ResourceLimitExceeded(_))
        ));
    }
}
