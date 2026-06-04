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

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("Image decoding error: {0}")]
    ImageDecode(String),

    #[error("Image encoding error: {0}")]
    ImageEncode(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Metadata error: {0}")]
    Metadata(String),

    #[error("Configuration error: {0}")]
    Config(String),

    /// Wraps `image::ImageError` directly. Prefer `ImageDecode`/`ImageEncode`
    /// for application-level context; this variant is used when the error
    /// originates from `image` crate internals without additional wrapping.
    #[error("Image error: {0}")]
    Image(#[from] ImageError),

    #[error("Steganography error: {0}")]
    Steganography(String),

    #[error("Invalid image format: {0}")]
    InvalidFormat(String),

    #[error("Image data was truncated: {0}")]
    ImageTruncated(String),

    #[error("Payload verification failed: {0}")]
    PayloadVerification(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("ISCC generation error: {0}")]
    Iscc(String),

    #[cfg(feature = "async")]
    #[error("Blocking task failed: {0}")]
    Task(String),
}

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
    }
}
