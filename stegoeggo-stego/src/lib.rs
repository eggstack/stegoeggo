#![forbid(unsafe_code)]

//! Generic image steganography carriers.
//!
//! The default API is intentionally small: callers can use the public LSB,
//! JPEG, and framed-payload modules without depending on codec implementation
//! details.
//!
//! ```compile_fail
//! use stegoeggo_stego::__internal_jpeg_facade::JpegHeader;
//! ```
//!
//! ```compile_fail
//! use stegoeggo_stego::jpeg_transcoder::JpegTranscoder;
//! ```
//!
//! ```compile_fail
//! use stegoeggo_stego::__internal_lsb_facade::stego_permutation_v2;
//! ```

pub mod constants;
pub mod error;
pub mod frame;
pub mod jpeg;
pub(crate) mod jpeg_transcoder;
pub mod lsb;
pub(crate) mod lsb_internal;
pub mod types;

#[cfg(feature = "application-support")]
#[doc(hidden)]
pub mod application_support;

pub use error::{JpegUnsupportedReason, StegoError, StegoResult};
pub use jpeg::is_progressive_jpeg;
pub use lsb::DEFAULT_TILE_SIZE;
pub use types::{EmbedOutcome, EmbedOutcomeSummary, EmbedPath, EmbedStatus, InPlaceEmbedReport};

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
