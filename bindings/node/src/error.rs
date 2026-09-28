use napi_derive::napi;
use stegoeggo::Error as RustError;

/// Stable machine-readable failure codes for the Node binding.
///
/// These strings are the public `error.code` contract. They are
/// compatibility-sensitive once the first npm publication exists.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Configuration or request validation failure.
    #[napi(value = "ERR_STEGOEGGO_INVALID_CONFIG")]
    InvalidConfig,
    /// The image format could not be determined or is unsupported.
    #[napi(value = "ERR_STEGOEGGO_INVALID_FORMAT")]
    InvalidFormat,
    /// Image decoding or encoding failure, including truncated input.
    #[napi(value = "ERR_STEGOEGGO_ENCODE_DECODE")]
    EncodeDecode,
    /// Metadata injection or parsing failure.
    #[napi(value = "ERR_STEGOEGGO_METADATA")]
    Metadata,
    /// Steganographic embedding or extraction failure.
    #[napi(value = "ERR_STEGOEGGO_STEGANOGRAPHY")]
    Steganography,
    /// The carrier lacks capacity for the requested payload.
    #[napi(value = "ERR_STEGOEGGO_INSUFFICIENT_CAPACITY")]
    InsufficientCapacity,
    /// Payload verification (CRC32 or HMAC) failure.
    #[napi(value = "ERR_STEGOEGGO_VERIFICATION")]
    Verification,
    /// A configured resource limit was exceeded for untrusted input.
    #[napi(value = "ERR_STEGOEGGO_RESOURCE_LIMIT")]
    ResourceLimit,
    /// Safe fallback for non-exhaustive or unexpected native failures.
    #[napi(value = "ERR_STEGOEGGO_INTERNAL")]
    Internal,
}

impl ErrorCode {
    fn as_str(self) -> &'static str {
        match self {
            Self::InvalidConfig => "ERR_STEGOEGGO_INVALID_CONFIG",
            Self::InvalidFormat => "ERR_STEGOEGGO_INVALID_FORMAT",
            Self::EncodeDecode => "ERR_STEGOEGGO_ENCODE_DECODE",
            Self::Metadata => "ERR_STEGOEGGO_METADATA",
            Self::Steganography => "ERR_STEGOEGGO_STEGANOGRAPHY",
            Self::InsufficientCapacity => "ERR_STEGOEGGO_INSUFFICIENT_CAPACITY",
            Self::Verification => "ERR_STEGOEGGO_VERIFICATION",
            Self::ResourceLimit => "ERR_STEGOEGGO_RESOURCE_LIMIT",
            Self::Internal => "ERR_STEGOEGGO_INTERNAL",
        }
    }
}

impl AsRef<str> for ErrorCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// The binding's own error type. `error.code` is one of the stable
/// `ERR_STEGOEGGO_*` values rather than a raw Node-API status.
pub type BindingError = napi::Error<ErrorCode>;

/// Binding result type carrying a stable `ERR_STEGOEGGO_*` code.
pub type Result<T> = std::result::Result<T, BindingError>;

/// Builds a native error whose JavaScript `code` is one of the stable
/// `ERR_STEGOEGGO_*` values.
pub fn binding_error(code: ErrorCode, message: impl Into<String>) -> BindingError {
    napi::Error::new(code, message.into())
}

/// Builds an `ERR_STEGOEGGO_INVALID_CONFIG` error.
pub fn config_error(message: impl Into<String>) -> BindingError {
    binding_error(ErrorCode::InvalidConfig, message)
}

/// Structured native error payload projected out of a canonical Rust failure.
///
/// The public JavaScript error is built from this DTO by the package's thin
/// wrapper so that `code` and the structured fields survive Promise rejection.
/// Field values are `null` when the canonical variant does not carry them.
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct NativeError {
    /// Stable machine-readable code.
    pub code: ErrorCode,
    /// Human-readable description. Never the machine contract.
    pub message: String,
    /// Resource category for limit failures.
    pub resource: Option<String>,
    /// Required carrier units for `ERR_STEGOEGGO_INSUFFICIENT_CAPACITY`.
    pub required: Option<f64>,
    /// Available carrier units for `ERR_STEGOEGGO_INSUFFICIENT_CAPACITY`.
    pub available: Option<f64>,
    /// Observed size for `input_bytes` and `metadata` limit failures.
    pub size: Option<f64>,
    /// Configured limit for limit failures.
    pub limit: Option<f64>,
    /// Observed image width for `dimensions` limit failures.
    pub width: Option<f64>,
    /// Observed image height for `dimensions` limit failures.
    pub height: Option<f64>,
    /// Configured maximum width for `dimensions` limit failures.
    pub max_width: Option<f64>,
    /// Configured maximum height for `dimensions` limit failures.
    pub max_height: Option<f64>,
    /// Container or budget kind for `container` and `verification_budget`.
    pub kind: Option<String>,
    /// Observed count for `container` and `verification_budget` failures.
    pub count: Option<f64>,
}

impl NativeError {
    /// Projects a canonical Rust error into the structured binding DTO.
    ///
    /// Unknown future non-exhaustive variants fall back to
    /// `ERR_STEGOEGGO_INTERNAL` without inventing structured facts.
    #[must_use]
    pub fn from_rust(error: &RustError) -> Self {
        let message = error.to_string();
        match error {
            RustError::Config(_) => Self::plain(ErrorCode::InvalidConfig, message),
            RustError::InvalidFormat(_) => Self::plain(ErrorCode::InvalidFormat, message),
            RustError::ImageDecode(_)
            | RustError::ImageEncode(_)
            | RustError::Image(_)
            | RustError::Io(_)
            | RustError::ImageTruncated(_) => Self::plain(ErrorCode::EncodeDecode, message),
            RustError::Metadata(_) | RustError::Serialization(_) => {
                Self::plain(ErrorCode::Metadata, message)
            }
            RustError::Steganography(_) => Self::plain(ErrorCode::Steganography, message),
            RustError::PayloadVerification(_) | RustError::Crypto(_) => {
                Self::plain(ErrorCode::Verification, message)
            }
            RustError::Iscc(_) => Self::plain(ErrorCode::Metadata, message),
            RustError::InsufficientCapacity {
                required,
                available,
            } => Self {
                code: ErrorCode::InsufficientCapacity,
                message,
                resource: None,
                required: Some(*required as f64),
                available: Some(*available as f64),
                size: None,
                limit: None,
                width: None,
                height: None,
                max_width: None,
                max_height: None,
                kind: None,
                count: None,
            },
            RustError::InputTooLarge { size, limit } => Self {
                code: ErrorCode::ResourceLimit,
                message,
                resource: Some("input_bytes".to_string()),
                required: None,
                available: None,
                size: Some(*size as f64),
                limit: Some(*limit as f64),
                width: None,
                height: None,
                max_width: None,
                max_height: None,
                kind: None,
                count: None,
            },
            RustError::DimensionsExceeded {
                width,
                height,
                max_width,
                max_height,
            } => Self {
                code: ErrorCode::ResourceLimit,
                message,
                resource: Some("dimensions".to_string()),
                required: None,
                available: None,
                size: None,
                limit: None,
                width: Some(f64::from(*width)),
                height: Some(f64::from(*height)),
                max_width: Some(f64::from(*max_width)),
                max_height: Some(f64::from(*max_height)),
                kind: None,
                count: None,
            },
            RustError::ContainerLimitExceeded { kind, count, limit } => Self {
                code: ErrorCode::ResourceLimit,
                message,
                resource: Some("container".to_string()),
                required: None,
                available: None,
                size: None,
                limit: Some(*limit as f64),
                width: None,
                height: None,
                max_width: None,
                max_height: None,
                kind: Some((*kind).to_string()),
                count: Some(*count as f64),
            },
            RustError::MetadataLimitExceeded { kind, size, limit } => Self {
                code: ErrorCode::ResourceLimit,
                message,
                resource: Some("metadata".to_string()),
                required: None,
                available: None,
                size: Some(*size as f64),
                limit: Some(*limit as f64),
                width: None,
                height: None,
                max_width: None,
                max_height: None,
                kind: Some((*kind).to_string()),
                count: None,
            },
            RustError::VerificationBudgetExceeded { kind, count, limit } => Self {
                code: ErrorCode::ResourceLimit,
                message,
                resource: Some("verification_budget".to_string()),
                required: None,
                available: None,
                size: None,
                limit: Some(*limit as f64),
                width: None,
                height: None,
                max_width: None,
                max_height: None,
                kind: Some((*kind).to_string()),
                count: Some(*count as f64),
            },
            _ => Self::plain(ErrorCode::Internal, message),
        }
    }

    /// Projects a binding-level failure that never came from canonical Rust.
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::plain(ErrorCode::Internal, message)
    }

    fn plain(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            resource: None,
            required: None,
            available: None,
            size: None,
            limit: None,
            width: None,
            height: None,
            max_width: None,
            max_height: None,
            kind: None,
            count: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(
            ErrorCode::InvalidConfig.as_ref(),
            "ERR_STEGOEGGO_INVALID_CONFIG"
        );
        assert_eq!(
            ErrorCode::InsufficientCapacity.as_ref(),
            "ERR_STEGOEGGO_INSUFFICIENT_CAPACITY"
        );
        assert_eq!(ErrorCode::Internal.as_ref(), "ERR_STEGOEGGO_INTERNAL");
    }

    #[test]
    fn truncated_maps_to_encode_decode() {
        let err = RustError::ImageTruncated("unexpected EOF".to_string());
        let dto = NativeError::from_rust(&err);
        assert_eq!(dto.code, ErrorCode::EncodeDecode);
        assert!(dto.message.to_lowercase().contains("truncated"));
    }

    #[test]
    fn invalid_format_maps_to_invalid_format() {
        let dto = NativeError::from_rust(&RustError::InvalidFormat("nope".to_string()));
        assert_eq!(dto.code, ErrorCode::InvalidFormat);
    }

    #[test]
    fn config_maps_to_invalid_config() {
        let dto = NativeError::from_rust(&RustError::Config("bad".to_string()));
        assert_eq!(dto.code, ErrorCode::InvalidConfig);
    }

    #[test]
    fn metadata_maps_to_metadata() {
        let dto = NativeError::from_rust(&RustError::Metadata("bad".to_string()));
        assert_eq!(dto.code, ErrorCode::Metadata);
    }

    #[test]
    fn steganography_maps_to_steganography() {
        let dto = NativeError::from_rust(&RustError::Steganography("bad".to_string()));
        assert_eq!(dto.code, ErrorCode::Steganography);
    }

    #[test]
    fn crypto_maps_to_verification() {
        let dto = NativeError::from_rust(&RustError::Crypto("bad".to_string()));
        assert_eq!(dto.code, ErrorCode::Verification);
    }

    #[test]
    fn insufficient_capacity_preserves_counts() {
        let err = RustError::InsufficientCapacity {
            required: 4096,
            available: 12,
        };
        let dto = NativeError::from_rust(&err);
        assert_eq!(dto.code, ErrorCode::InsufficientCapacity);
        assert_eq!(dto.required, Some(4096.0));
        assert_eq!(dto.available, Some(12.0));
        assert!(dto.resource.is_none());
    }

    #[test]
    fn input_limit_projection() {
        let err = RustError::InputTooLarge {
            size: 1000,
            limit: 8,
        };
        let dto = NativeError::from_rust(&err);
        assert_eq!(dto.code, ErrorCode::ResourceLimit);
        assert_eq!(dto.resource.as_deref(), Some("input_bytes"));
        assert_eq!(dto.size, Some(1000.0));
        assert_eq!(dto.limit, Some(8.0));
    }

    #[test]
    fn dimension_limit_projection() {
        let err = RustError::DimensionsExceeded {
            width: 8,
            height: 8,
            max_width: 4,
            max_height: 4,
        };
        let dto = NativeError::from_rust(&err);
        assert_eq!(dto.resource.as_deref(), Some("dimensions"));
        assert_eq!(dto.width, Some(8.0));
        assert_eq!(dto.height, Some(8.0));
        assert_eq!(dto.max_width, Some(4.0));
        assert_eq!(dto.max_height, Some(4.0));
    }

    #[test]
    fn container_limit_projection() {
        let err = RustError::ContainerLimitExceeded {
            kind: "PNG chunks",
            count: 9,
            limit: 1,
        };
        let dto = NativeError::from_rust(&err);
        assert_eq!(dto.resource.as_deref(), Some("container"));
        assert_eq!(dto.kind.as_deref(), Some("PNG chunks"));
        assert_eq!(dto.count, Some(9.0));
        assert_eq!(dto.limit, Some(1.0));
    }

    #[test]
    fn metadata_limit_projection() {
        let err = RustError::MetadataLimitExceeded {
            kind: "tEXt field",
            size: 12,
            limit: 1,
        };
        let dto = NativeError::from_rust(&err);
        assert_eq!(dto.resource.as_deref(), Some("metadata"));
        assert_eq!(dto.kind.as_deref(), Some("tEXt field"));
        assert_eq!(dto.size, Some(12.0));
        assert_eq!(dto.limit, Some(1.0));
    }

    #[test]
    fn verification_budget_projection() {
        let err = RustError::VerificationBudgetExceeded {
            kind: "tile origins",
            count: 40,
            limit: 16,
        };
        let dto = NativeError::from_rust(&err);
        assert_eq!(dto.resource.as_deref(), Some("verification_budget"));
        assert_eq!(dto.kind.as_deref(), Some("tile origins"));
        assert_eq!(dto.count, Some(40.0));
        assert_eq!(dto.limit, Some(16.0));
    }

    #[test]
    fn error_projection_never_contains_key_material() {
        let err = RustError::Config("no key material here".to_string());
        let dto = NativeError::from_rust(&err);
        assert!(!dto.message.contains("super-secret-mac-key"));
    }
}
