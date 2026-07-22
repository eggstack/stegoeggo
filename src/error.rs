//! Error types for the stegoeggo library.
//!
//! This module defines the custom error types used throughout the library
//! for handling various failure modes including image decoding/encoding errors,
//! metadata issues, variant handling, and cryptographic operations.

use image::ImageError;
use thiserror::Error;

use crate::jpeg_transcoder::TranscoderError;

impl From<TranscoderError> for Error {
    fn from(e: TranscoderError) -> Self {
        match e {
            TranscoderError::InvalidFormat(s) => Error::InvalidFormat(s),
            TranscoderError::Unsupported(s) => Error::InvalidFormat(s),
            TranscoderError::HuffmanDecode(s) => Error::ImageDecode(s),
            TranscoderError::HuffmanEncode(s) => Error::ImageEncode(s),
            TranscoderError::Io(e) => Error::Io(e),
            TranscoderError::EmbeddingFailed(s) => Error::Steganography(s),
        }
    }
}

/// Error type for all stegoeggo operations.
///
/// This enum captures failure modes from image decoding/encoding,
/// metadata operations, steganographic embedding/extraction, and
/// cryptographic verification. All variants are non-exhaustive to
/// allow adding new error cases in future minor releases.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Image decoding failed (invalid format, corrupt data, or unsupported features).
    #[error("Image decoding error: {0}")]
    ImageDecode(String),

    /// Image encoding failed ( unsupported format, encoder error).
    #[error("Image encoding error: {0}")]
    ImageEncode(String),

    /// An I/O error occurred (file not found, permission denied, etc.).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization failed.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// A metadata operation failed (injection, extraction, or parsing).
    #[error("Metadata error: {0}")]
    Metadata(String),

    /// A configuration error occurred (invalid parameters).
    #[error("Configuration error: {0}")]
    Config(String),

    /// Wraps `image::ImageError` directly. Prefer `ImageDecode`/`ImageEncode`
    /// for application-level context; this variant is used when the error
    /// originates from `image` crate internals without additional wrapping.
    #[error("Image error: {0}")]
    Image(#[from] ImageError),

    /// A steganographic embedding or extraction operation failed.
    #[error("Steganography error: {0}")]
    Steganography(String),

    /// The image format could not be determined or is unsupported.
    #[error("Invalid image format: {0}")]
    InvalidFormat(String),

    /// The image data was truncated or incomplete.
    #[error("Image data was truncated: {0}")]
    ImageTruncated(String),

    /// Payload verification failed (CRC32 mismatch or HMAC verification failure).
    #[error("Payload verification failed: {0}")]
    PayloadVerification(String),

    /// A cryptographic operation failed (HMAC, key derivation, etc.).
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    /// ISCC (International Standard Content Code) generation failed.
    #[error("ISCC generation error: {0}")]
    Iscc(String),

    /// Input exceeds the configured maximum byte limit.
    #[error("Input too large: {size} bytes exceeds limit of {limit} bytes")]
    InputTooLarge {
        /// Actual input size in bytes.
        size: usize,
        /// Configured maximum allowed size.
        limit: usize,
    },

    /// Image dimensions exceed the configured maximum.
    #[error("Dimensions exceeded: {width}x{height} exceeds {max_width}x{max_height}")]
    DimensionsExceeded {
        /// Actual image width.
        width: u32,
        /// Actual image height.
        height: u32,
        /// Maximum allowed width.
        max_width: u32,
        /// Maximum allowed height.
        max_height: u32,
    },

    /// A container structure (PNG chunks, JPEG segments, WebP RIFF) exceeds limits.
    #[error("Container limit exceeded: {kind} count {count} exceeds limit {limit}")]
    ContainerLimitExceeded {
        /// Container type description.
        kind: &'static str,
        /// Actual count observed.
        count: usize,
        /// Configured maximum count.
        limit: usize,
    },

    /// Metadata extraction or parsing exceeds configured limits.
    #[error("Metadata limit exceeded: {kind} size {size} exceeds limit {limit}")]
    MetadataLimitExceeded {
        /// Metadata type description.
        kind: &'static str,
        /// Actual size in bytes.
        size: usize,
        /// Configured maximum size.
        limit: usize,
    },

    /// Verification budget exceeded (too many candidates, seeds, or origins).
    #[error("Verification budget exceeded: {kind} count {count} exceeds limit {limit}")]
    VerificationBudgetExceeded {
        /// Budget type description.
        kind: &'static str,
        /// Actual count observed.
        count: usize,
        /// Configured maximum count.
        limit: usize,
    },

    /// An async blocking task failed.
    #[cfg(feature = "async")]
    #[error("Blocking task failed: {0}")]
    Task(String),
}

/// A specialized `Result` type for stegoeggo operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_image_decode_display() {
        let err = Error::ImageDecode("failed to decode PNG".to_string());
        let s = err.to_string();
        assert!(s.contains("Image decoding error"));
        assert!(s.contains("failed to decode PNG"));
    }

    #[test]
    fn error_image_encode_display() {
        let err = Error::ImageEncode("encoding failed".to_string());
        let s = err.to_string();
        assert!(s.contains("Image encoding error"));
        assert!(s.contains("encoding failed"));
    }

    #[test]
    fn error_io_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        let s = err.to_string();
        assert!(s.contains("IO error"));
    }

    #[test]
    fn error_serialization_display() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err: Error = json_err.into();
        let s = err.to_string();
        assert!(s.contains("Serialization error"));
    }

    #[test]
    fn error_metadata_display() {
        let err = Error::Metadata("missing required field".to_string());
        let s = err.to_string();
        assert!(s.contains("Metadata error"));
        assert!(s.contains("missing required field"));
    }

    #[test]
    fn error_config_display() {
        let err = Error::Config("invalid key size".to_string());
        let s = err.to_string();
        assert!(s.contains("Configuration error"));
        assert!(s.contains("invalid key size"));
    }

    #[test]
    fn error_steganography_display() {
        let err = Error::Steganography("embedding failed".to_string());
        let s = err.to_string();
        assert!(s.contains("Steganography error"));
        assert!(s.contains("embedding failed"));
    }

    #[test]
    fn error_invalid_format_display() {
        let err = Error::InvalidFormat("unknown format".to_string());
        let s = err.to_string();
        assert!(s.contains("Invalid image format"));
        assert!(s.contains("unknown format"));
    }

    #[test]
    fn error_image_truncated_display() {
        let err = Error::ImageTruncated("unexpected EOF".to_string());
        let s = err.to_string();
        assert!(s.contains("Image data was truncated"));
        assert!(s.contains("unexpected EOF"));
    }

    #[test]
    fn error_payload_verification_display() {
        let err = Error::PayloadVerification("checksum mismatch".to_string());
        let s = err.to_string();
        assert!(s.contains("Payload verification failed"));
        assert!(s.contains("checksum mismatch"));
    }

    #[test]
    fn error_crypto_display() {
        let err = Error::Crypto("key derivation failed".to_string());
        let s = err.to_string();
        assert!(s.contains("Cryptographic error"));
        assert!(s.contains("key derivation failed"));
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err: Error = io_err.into();
        match err {
            Error::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied),
            _ => panic!("Expected Error::Io"),
        }
    }

    #[test]
    fn error_from_serde_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let err: Error = json_err.into();
        match err {
            Error::Serialization(_) => {}
            _ => panic!("Expected Error::Serialization"),
        }
    }

    #[test]
    fn error_iscc_display() {
        let err = Error::Iscc("image code generation failed".to_string());
        let s = err.to_string();
        assert!(s.contains("ISCC generation error"));
        assert!(s.contains("image code generation failed"));
    }

    #[test]
    fn error_all_variants_constructible() {
        let _ = Error::ImageDecode("test".to_string());
        let _ = Error::ImageEncode("test".to_string());
        let _ = Error::Metadata("test".to_string());
        let _ = Error::Config("test".to_string());
        let _ = Error::Steganography("test".to_string());
        let _ = Error::InvalidFormat("test".to_string());
        let _ = Error::ImageTruncated("test".to_string());
        let _ = Error::PayloadVerification("test".to_string());
        let _ = Error::Crypto("test".to_string());
        let _ = Error::Iscc("test".to_string());
        let _ = Error::InputTooLarge {
            size: 1000,
            limit: 500,
        };
        let _ = Error::DimensionsExceeded {
            width: 8000,
            height: 8000,
            max_width: 4096,
            max_height: 4096,
        };
        let _ = Error::ContainerLimitExceeded {
            kind: "PNG chunks",
            count: 1000,
            limit: 500,
        };
        let _ = Error::MetadataLimitExceeded {
            kind: "XMP packet",
            size: 200000,
            limit: 65535,
        };
        let _ = Error::VerificationBudgetExceeded {
            kind: "tile origins",
            count: 100,
            limit: 16,
        };
    }

    #[test]
    fn error_input_too_large_display() {
        let err = Error::InputTooLarge {
            size: 1000,
            limit: 500,
        };
        let s = err.to_string();
        assert!(s.contains("1000"));
        assert!(s.contains("500"));
    }

    #[test]
    fn error_dimensions_exceeded_display() {
        let err = Error::DimensionsExceeded {
            width: 8000,
            height: 8000,
            max_width: 4096,
            max_height: 4096,
        };
        let s = err.to_string();
        assert!(s.contains("8000x8000"));
        assert!(s.contains("4096x4096"));
    }

    #[test]
    fn error_container_limit_display() {
        let err = Error::ContainerLimitExceeded {
            kind: "JPEG segments",
            count: 200,
            limit: 100,
        };
        let s = err.to_string();
        assert!(s.contains("JPEG segments"));
        assert!(s.contains("200"));
        assert!(s.contains("100"));
    }
}
