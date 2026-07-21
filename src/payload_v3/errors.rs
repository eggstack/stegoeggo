/// Errors encountered when parsing a stego payload.
#[derive(Debug, thiserror::Error)]
pub enum PayloadV3ParseError {
    /// Payload is shorter than the minimum required size.
    #[error("Payload too short: need at least {min} bytes, got {actual}")]
    TooShort {
        /// Minimum required bytes.
        min: usize,
        /// Actual bytes provided.
        actual: usize,
    },
    /// Magic bytes do not match the expected `SE` marker.
    #[error("Invalid magic bytes: expected [0x53, 0x45], got {0:?}")]
    InvalidMagic([u8; 2]),
    /// Payload version is not supported.
    #[error("Unsupported payload version: {0}")]
    UnsupportedVersion(u8),
    /// Header length exceeds total payload length.
    #[error("Header length {header} exceeds total payload length {total}")]
    HeaderExceedsTotal {
        /// Declared header length.
        header: usize,
        /// Total payload length.
        total: usize,
    },
    /// DMI policy byte is out of range.
    #[error("Invalid DMI policy byte: {0}")]
    InvalidDmiPolicy(u8),
    /// Authentication algorithm byte is not recognized.
    #[error("Invalid authentication algorithm: {0}")]
    InvalidAuthAlgorithm(u8),
    /// Key ID length exceeds the maximum allowed.
    #[error("Key ID length {key_id_len} exceeds maximum {max}")]
    KeyIdTooLong {
        /// Declared key ID length.
        key_id_len: usize,
        /// Maximum allowed key ID length.
        max: usize,
    },
    /// Extension section exceeds maximum size.
    #[error("Extension section exceeds maximum size")]
    ExtensionsTooLarge,
    /// Unknown critical extension type encountered.
    #[error("Unknown critical extension type: 0x{0:04X}")]
    UnknownCriticalExtension(u16),
    /// Duplicate singleton extension type encountered.
    #[error("Duplicate singleton extension: 0x{0:04X}")]
    DuplicateExtension(u16),
    /// Payload exceeds maximum embedded size.
    #[error("Payload exceeds maximum embedded size: {size} > {max}")]
    Oversized {
        /// Actual size.
        size: usize,
        /// Maximum allowed size.
        max: usize,
    },
    /// Intensity value is out of range.
    #[error("Invalid intensity value: {0}")]
    InvalidIntensity(u16),
}
