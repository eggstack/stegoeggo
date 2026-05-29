//! Error types for the cloakrs library.
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

    #[error("Variant not found: {0}")]
    VariantNotFound(String),

    #[error("Invalid variant format: {0}")]
    InvalidVariant(String),

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

    #[error("Hash computation error: {0}")]
    HashError(String),

    #[error("Invalid image format: {0}")]
    InvalidFormat(String),

    #[error("Image data was truncated: {0}")]
    ImageTruncated(String),

    #[error("Payload verification failed: {0}")]
    PayloadVerification(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[cfg(feature = "async")]
    #[error("Blocking task failed: {0}")]
    Task(String),
}

pub type Result<T> = std::result::Result<T, Error>;
