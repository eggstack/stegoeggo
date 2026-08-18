#![forbid(unsafe_code)]

//! Generic image steganography carriers.
//!
//! The default API is intentionally small: callers can use the public LSB,
//! JPEG, and framed-payload modules without depending on codec implementation
//! details.
//!
//! # Stable public API
//!
//! The default public surface is operation-level:
//!
//! - [`lsb`] — pixel-domain LSB capacity, raw embed/extract, in-place embed,
//!   and framed convenience.
//! - [`jpeg`] — encoded-JPEG DCT capacity, raw embed/extract, framed
//!   convenience, support probing, and seed hints.
//! - [`frame`] — self-describing framed payload with CRC32.
//! - [`error`] — [`StegoError`] and [`JpegUnsupportedReason`].
//! - [`CapacityReport`], [`EmbedReport`], [`InPlaceEmbedReport`] — structured
//!   reports describing capacity and embed outcomes.
//!
//! # What is intentionally NOT public
//!
//! The following private modules prove compile-fail when imported from
//! outside the crate:
//!
//! ```compile_fail
//! use stegoeggo_stego::jpeg_transcoder::JpegTranscoder;
//! ```
//!
//! ```compile_fail
//! use stegoeggo_stego::lsb_internal::stego_permutation_v2;
//! ```
//!
//! The optional `application-support` feature exposes a narrow
//! `application_support` module for the parent rights-protection crate. It
//! stays `#[doc(hidden)]` and is not part of the default facade.

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
///
/// Both fields are in **carrier units** that depend on the carrier family:
/// - LSB: RGB carrier slots (one slot per non-alpha channel byte).
/// - JPEG: eligible non-zero AC coefficients across all components.
///
/// The same unit applies to both `required` and `available` for a given
/// use, so [`CapacityReport::is_sufficient`] is a direct comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityReport {
    /// Required capacity in carrier units.
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
///
/// Capacity fields use the same carrier-specific units as
/// [`CapacityReport`]. `payload_bytes` is the raw payload size in bytes
/// placed in the carrier; for framed operations it includes the frame
/// header and CRC32 overhead.
#[derive(Debug, Clone)]
pub struct EmbedReport<T = Vec<u8>> {
    /// Whether the payload was embedded.
    pub embedded: bool,
    /// The carrier output (image pixels or encoded JPEG bytes).
    pub output: T,
    /// Payload size in bytes placed in the carrier (raw, or framed
    /// header + payload when framed).
    pub payload_bytes: usize,
    /// Required capacity in carrier units.
    pub required_capacity: usize,
    /// Available capacity in carrier units.
    pub available_capacity: usize,
    /// The actual redundancy level used for embedding (may differ from
    /// the requested level if capacity was insufficient).
    pub actual_redundancy: usize,
}
