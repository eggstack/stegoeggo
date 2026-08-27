/// Errors encountered when parsing a stego payload.
///
/// New variants may be added in minor releases; downstream code should
/// treat this enum as non-exhaustive (no wildcard match required to
/// compile against future additions, but exhaustive `match` expressions
/// must add a wildcard arm).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
    /// Header length is shorter than the required core + key ID size.
    ///
    /// Raised when the declared `header_length` cannot hold the 32-byte core
    /// plus the declared key ID.
    ///
    /// **Note on naming:** The variant name `HeaderExceedsTotal` is a
    /// historical misname — the check compares the header length against
    /// the minimum required `total_core = V3_CORE_SIZE + key_id_len`, not
    /// against `total_length`. Renaming the variant would be a breaking
    /// change, so it is preserved for semver compatibility. New callers
    /// should pattern-match on the fields directly: `header < total` means
    /// the header is too short to hold the core + key ID, not that the
    /// header exceeds the total payload.
    #[error("Header length {header} is shorter than the required core+key-id size {total}")]
    HeaderExceedsTotal {
        /// Declared header length.
        header: usize,
        /// Minimum required core + key ID length.
        total: usize,
    },
    /// Header length and extension flag do not describe the same layout.
    #[error("Header length {header} is inconsistent with the extension flag")]
    HeaderLengthMismatch {
        /// Declared header length.
        header: usize,
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
