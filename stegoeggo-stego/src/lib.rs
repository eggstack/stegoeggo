#![forbid(unsafe_code)]

pub mod constants;
pub mod error;
pub mod frame;
pub mod jpeg;
pub(crate) mod jpeg_transcoder;
pub mod lsb;
pub(crate) mod lsb_internal;
pub mod types;

pub use error::{JpegUnsupportedReason, StegoError, StegoResult};
pub use jpeg_transcoder::is_progressive_jpeg;
pub use lsb::DEFAULT_TILE_SIZE;
pub use types::{EmbedOutcome, EmbedOutcomeSummary, EmbedPath, EmbedStatus};

/// Re-exports of JPEG transcoder internals for the consuming crate
/// (`stegoeggo`). These are not part of the stable carrier API; downstream
/// users should rely on `stegoeggo_stego::jpeg::{embed, extract, ...}`.
#[doc(hidden)]
pub mod __internal_jpeg_facade {
    pub use crate::jpeg_transcoder::header::ParseLimits;
    pub use crate::jpeg_transcoder::{
        probe_dct_support, probe_dct_support_full, Coefficients, DctStegoF5, DctSupport,
        DctUnsupportedReason, JpegHeader, JpegTranscoder, TranscoderError,
    };
}

/// Re-exports of LSB internals for the consuming crate (`stegoeggo`).
/// These are not part of the stable carrier API; downstream users should
/// rely on `stegoeggo_stego::lsb::{embed, extract, capacity, LsbConfig, ...}`.
#[doc(hidden)]
pub mod __internal_lsb_facade {
    pub use crate::lsb_internal::{
        bits_to_bytes, blit_rgba, bytes_to_bits, carrier_v2_slot_to_pixel_channel, crop_rgba,
        embed_bit_in_pixel, embed_lsb, embed_lsb_tiled, embed_lsb_v2, embed_seed_lsb_fallback,
        extract_lsb, extract_lsb_range, extract_lsb_v2, extract_seed_lsb_fallback,
        lsb_available_slots, lsb_capacity_for_image, lsb_required_capacity_v2,
        lsb_required_slots_legacy, splitmix64, stego_permutation, stego_permutation_v2, tile_seed,
    };
}

/// Capacity report for a carrier query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityReport {
    /// Required capacity in carrier units (RGB slots for LSB, non-zero AC
    /// coefficients for DCT).
    pub required: usize,
    /// Available capacity in the same units.
    pub available: usize,
}

impl CapacityReport {
    /// Returns `true` if the carrier has sufficient capacity.
    #[must_use]
    pub fn is_sufficient(&self) -> bool {
        self.available >= self.required
    }
}

/// Report returned by embedding operations.
///
/// The `output` type parameter reflects the carrier domain:
/// - `RgbaImage` for pixel-domain (LSB) carriers,
/// - `Vec<u8>` for byte-stream (JPEG) carriers.
#[derive(Debug, Clone)]
pub struct EmbedReport<T = Vec<u8>> {
    /// Whether the payload was embedded.
    pub embedded: bool,
    /// The carrier output (image pixels or encoded JPEG bytes).
    pub output: T,
    /// Payload size in bytes.
    pub payload_bytes: usize,
    /// Required capacity in carrier units.
    pub required_capacity: usize,
    /// Available capacity in carrier units.
    pub available_capacity: usize,
    /// The actual redundancy level used for embedding (may differ from
    /// the requested level if capacity was insufficient).
    pub actual_redundancy: usize,
}
