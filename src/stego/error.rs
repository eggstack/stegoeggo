use std::fmt;

/// Error type for generic steganographic carrier operations.
///
/// These errors are specific to the `stego` public API and cover
/// configuration, capacity, input validation, and framing failures.
/// They convert into the crate's root [`Error`](crate::Error) via
/// [`From`] for callers that use the unified error type.
#[derive(Debug)]
#[non_exhaustive]
pub enum StegoError {
    /// Invalid configuration (e.g., zero seed, redundancy out of range).
    InvalidConfig(String),

    /// Insufficient carrier capacity for the requested payload.
    InsufficientCapacity {
        /// Required capacity in carrier units (RGB slots for LSB, non-zero AC
        /// coefficients for DCT).
        required: usize,
        /// Available capacity in the same units.
        available: usize,
    },

    /// The input image/JPEG data is malformed or could not be decoded.
    MalformedInput(String),

    /// The JPEG structure is not supported for DCT embedding.
    UnsupportedJpeg(JpegUnsupportedReason),

    /// No frame was found at the start of the extracted carrier data.
    FrameNotFound,

    /// The frame header is malformed (bad magic, bad version, length exceeds
    /// bounds).
    MalformedFrame(String),

    /// Frame CRC32 checksum does not match the payload bytes.
    FrameChecksumMismatch,

    /// A resource limit was exceeded during parsing or allocation.
    ResourceLimitExceeded(String),

    /// The carrier returned no data (internal error or empty image).
    EmptyCarrier,
}

/// Reason a JPEG structure is not supported for DCT embedding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum JpegUnsupportedReason {
    /// Progressive JPEG encoding.
    Progressive,
    /// Multiple scans in the JPEG stream.
    MultipleScans,
    /// Arithmetic entropy coding (not Huffman).
    ArithmeticCoding,
    /// Lossless coding process.
    LosslessCoding,
    /// Unsupported precision (not 8-bit).
    UnsupportedPrecision,
    /// Restart intervals present.
    RestartIntervals,
    /// Unsupported color process or sampling factor.
    UnsupportedColorProcess,
    /// Missing required Huffman tables.
    MissingTables,
    /// Malformed JPEG header.
    MalformedHeader,
    /// Trailing segments found after the scan data.
    TrailingSegmentsAfterScan,
}

impl fmt::Display for StegoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StegoError::InvalidConfig(msg) => write!(f, "invalid stego configuration: {msg}"),
            StegoError::InsufficientCapacity {
                required,
                available,
            } => write!(
                f,
                "insufficient capacity: need {required} carrier units, have {available}"
            ),
            StegoError::MalformedInput(msg) => write!(f, "malformed input: {msg}"),
            StegoError::UnsupportedJpeg(reason) => {
                write!(f, "unsupported JPEG structure: {reason}")
            }
            StegoError::FrameNotFound => write!(f, "frame not found in carrier data"),
            StegoError::MalformedFrame(msg) => write!(f, "malformed frame: {msg}"),
            StegoError::FrameChecksumMismatch => write!(f, "frame CRC32 checksum mismatch"),
            StegoError::ResourceLimitExceeded(msg) => {
                write!(f, "resource limit exceeded: {msg}")
            }
            StegoError::EmptyCarrier => write!(f, "carrier produced no data"),
        }
    }
}

impl std::error::Error for StegoError {}

impl fmt::Display for JpegUnsupportedReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Progressive => write!(f, "progressive JPEG"),
            Self::MultipleScans => write!(f, "multiple scans"),
            Self::ArithmeticCoding => write!(f, "arithmetic coding"),
            Self::LosslessCoding => write!(f, "lossless coding"),
            Self::UnsupportedPrecision => write!(f, "unsupported precision"),
            Self::RestartIntervals => write!(f, "restart intervals"),
            Self::UnsupportedColorProcess => write!(f, "unsupported color process"),
            Self::MissingTables => write!(f, "missing Huffman tables"),
            Self::MalformedHeader => write!(f, "malformed header"),
            Self::TrailingSegmentsAfterScan => write!(f, "trailing segments after scan"),
        }
    }
}

impl From<StegoError> for crate::Error {
    fn from(e: StegoError) -> Self {
        match &e {
            StegoError::InvalidConfig(msg) => crate::Error::Config(msg.clone()),
            StegoError::InsufficientCapacity { .. } => crate::Error::Steganography(e.to_string()),
            StegoError::MalformedInput(msg) => crate::Error::InvalidFormat(msg.clone()),
            StegoError::UnsupportedJpeg(_) => crate::Error::InvalidFormat(e.to_string()),
            StegoError::FrameNotFound => crate::Error::Steganography(e.to_string()),
            StegoError::MalformedFrame(_) => crate::Error::Steganography(e.to_string()),
            StegoError::FrameChecksumMismatch => crate::Error::PayloadVerification(e.to_string()),
            StegoError::ResourceLimitExceeded(msg) => crate::Error::Config(msg.clone()),
            StegoError::EmptyCarrier => crate::Error::Steganography(e.to_string()),
        }
    }
}

/// A specialized `Result` type for generic stego operations.
pub type StegoResult<T> = std::result::Result<T, StegoError>;
