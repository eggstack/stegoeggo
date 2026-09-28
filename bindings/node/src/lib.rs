//! Node.js frontend for the canonical `stegoeggo` byte API.
//!
//! This crate is a leaf napi-rs v3 binding. It owns no protection logic: every
//! public operation delegates to `ProtectionRequest` + `process_request_bytes*`
//! or to `verify_image_bytes_report`. Deprecated `ProtectionLevel`,
//! `ProtectionContext`, and `EvidenceProfile` adapters are deliberately not
//! projected.
//!
//! The binding is built with `panic = "unwind"` so a malformed-input or
//! untrusted-input failure can never abort the host Node process.

#![deny(missing_docs)]

mod enums;
mod error;
mod limits;
mod notice;
mod numeric;
mod report;
mod request;
mod tasks;

pub use enums::{
    AuthenticationMode, DiagnosticLevel, DmiValue, EvidenceChannel, EvidenceStrength, FieldSource,
    HiddenMarkerKind, HiddenMarkerMode, ImageOutputFormat, MetadataUpdatePolicy, ProtectionPreset,
    ProtectionWarning, RightsPolicy, VerificationStatus,
};
pub use error::{ErrorCode, NativeError};
pub use limits::{ResourceLimits, ResourceLimitsBuilder};
pub use notice::{ProcessingOptions, RightsNotice};
pub use report::{
    AuthenticationVerification, BindingVerification, Diagnostic, ExecutionReport,
    HiddenMarkerVerification, ResourceUsage, RightsVerification, TrustEvaluation,
    VerificationReport,
};
pub use request::ProtectionRequest;
pub use tasks::{
    owned_bytes, protect_internal, protect_with_report_internal, protect_with_warnings_internal,
    verify_internal, ProtectOutcome, ProtectWithReportOutcome, ProtectWithWarningsOutcome,
    VerifyOutcome,
};

use napi_derive::napi;
use stegoeggo::ImageOutputFormat as RustFormat;

/// The wrapped StegoEggo library version, identical to the package version.
#[napi(js_name = "stegoeggoVersion")]
pub fn stegoeggo_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Detects the encoded image format from its magic bytes.
///
/// This is a bounded header inspection, not a decode, so it stays synchronous
/// on the JavaScript event-loop thread. Returns the canonical lowercase
/// extension (`"png"`, `"jpg"`, `"webp"`) or `null` when unrecognized.
#[napi(ts_return_type = "string | null")]
pub fn detect_format(data: &[u8]) -> Option<String> {
    RustFormat::from_magic_bytes(data).map(|format| format.extension().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_tracks_the_package_version() {
        assert_eq!(stegoeggo_version(), env!("CARGO_PKG_VERSION"));
        assert!(!stegoeggo_version().is_empty());
    }

    #[test]
    fn detects_png_jpeg_and_webp() {
        assert_eq!(
            detect_format(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]).as_deref(),
            Some("png")
        );
        assert_eq!(
            detect_format(&[0xFF, 0xD8, 0xFF, 0xE0]).as_deref(),
            Some("jpg")
        );
        assert_eq!(
            detect_format(b"RIFF\x00\x00\x00\x00WEBPVP8 ").as_deref(),
            Some("webp")
        );
    }

    #[test]
    fn rejects_unknown_and_short_input() {
        assert_eq!(detect_format(b"not a real image"), None);
        assert_eq!(detect_format(&[0x89, 0x50]), None);
        assert_eq!(detect_format(&[]), None);
    }

    #[test]
    fn detection_is_bounded() {
        let mut large = vec![0_u8; 1_000_000];
        large[0..4].copy_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]);
        assert_eq!(detect_format(&large).as_deref(), Some("jpg"));
    }
}
