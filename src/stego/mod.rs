/// Generic steganographic carrier operations for arbitrary payload bytes.
///
/// This module provides a low-level API for embedding and extracting
/// arbitrary byte payloads into images using steganographic carrier
/// techniques. It is independent of the rights-protection pipeline.
///
/// # Carrier Types
///
/// - [`lsb`] — Pixel-domain LSB embedding for PNG/WebP (`RgbaImage`)
/// - [`jpeg`] — DCT-domain F5 embedding for JPEG (encoded bytes)
/// - [`frame`] — Optional self-describing frame with length + CRC32
///
/// # Security Considerations
///
/// This is best-effort steganography, not encryption. Seed knowledge
/// is not equivalent to cryptographic secrecy. LSB payloads are fragile
/// under lossy re-encoding. JPEG DCT payloads are not guaranteed across
/// arbitrary recompression.
pub mod error;
/// Generic self-describing payload frame with version, length, and CRC32.
pub mod frame;
/// JPEG DCT carrier operations for encoded byte data.
pub mod jpeg;
/// Pixel-domain LSB carrier operations for RGBA images.
pub mod lsb;

pub use error::{JpegUnsupportedReason, StegoError, StegoResult};

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
