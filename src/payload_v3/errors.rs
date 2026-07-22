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
    /// Authentication key was not provided but the payload requires one.
    #[error("Authentication key required but not provided")]
    MissingKey,
    /// Authentication key was provided but HMAC verification failed.
    ///
    /// This indicates the supplied key does not match the key used during
    /// embedding. It is distinct from [`PayloadV3ParseError::CorruptTag`],
    /// which indicates the tag itself is structurally malformed.
    #[error("Authentication failed: wrong key")]
    WrongKey,
    /// Authentication tag is present but structurally invalid.
    ///
    /// The tag length does not match the expected length for the declared
    /// algorithm, or the tag is truncated. This is distinct from
    /// [`PayloadV3ParseError::WrongKey`], where the tag structure is correct
    /// but the HMAC does not match.
    #[error("Authentication tag is corrupt or truncated")]
    CorruptTag,
}
