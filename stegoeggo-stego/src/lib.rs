#![forbid(unsafe_code)]

pub mod constants;
pub mod error;
pub mod frame;
pub mod jpeg;
pub mod jpeg_transcoder;
pub mod lsb;
pub mod types;

pub use error::{JpegUnsupportedReason, StegoError, StegoResult};
pub use jpeg_transcoder::is_progressive_jpeg;
pub use types::{EmbedOutcome, EmbedOutcomeSummary, EmbedPath, EmbedStatus};

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
#[derive(Debug, Clone)]
pub struct EmbedReport {
    /// Whether the payload was embedded.
    pub embedded: bool,
    /// The output bytes (image or JPEG).
    pub output: Vec<u8>,
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
