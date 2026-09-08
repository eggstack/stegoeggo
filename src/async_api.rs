//! Async wrappers for WAF/CDN edge integration.
//!
//! Uses `tokio::task::spawn_blocking` to run CPU-bound image protection
//! on the blocking thread pool, keeping the async runtime responsive.
//!
//! The canonical surface is request-based (`process_request_bytes_async`,
//! `process_request_bytes_with_warnings_async`,
//! `process_request_bytes_with_report_async`, plus the `parallel` batch
//! variants). Each calls its synchronous canonical counterpart inside one
//! `spawn_blocking` closure with no policy, routing, or warning duplication.
//! Level/context async wrappers remain as compatibility adapters.
//!
//! All functions take owned data (`Vec<u8>`, `DynamicImage`, `ProtectionRequest`)
//! rather than borrows to satisfy `spawn_blocking`'s `Send + 'static` requirement.
//!
//! # Usage
//!
//! ## Single image processing (WAF hot path)
//!
//! ```no_run
//! use stegoeggo::{process_request_bytes_with_warnings_async, ProtectionRequest, RightsNotice, RightsPolicy};
//!
//! # #[tokio::main] async fn main() -> Result<(), stegoeggo::Error> {
//! let request = ProtectionRequest::with_hidden_marker(
//!     RightsNotice::new(),
//!     RightsPolicy::ProhibitedAiMlTraining,
//! )
//! .with_seed(42);
//! let bytes: Vec<u8> = std::fs::read("input.png")?;
//! let (protected, warnings) =
//!     process_request_bytes_with_warnings_async(bytes, request).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Parallel batch processing (CDN origin)
//!
//! ```no_run
//! use stegoeggo::{process_request_bytes_parallel_async, ProtectionRequest, RightsNotice, RightsPolicy};
//!
//! # #[tokio::main] async fn main() -> Result<(), stegoeggo::Error> {
//! let request = ProtectionRequest::with_hidden_marker(
//!     RightsNotice::new(),
//!     RightsPolicy::ProhibitedAiMlTraining,
//! )
//! .with_seed(42);
//! let images: Vec<Vec<u8>> = vec![std::fs::read("a.png")?, std::fs::read("b.png")?];
//! let protected = process_request_bytes_parallel_async(images, request).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Verification endpoint
//!
//! ```no_run
//! use stegoeggo::verify_image_bytes_async;
//!
//! # #[tokio::main] async fn main() -> Result<(), stegoeggo::Error> {
//! let img_bytes: Vec<u8> = std::fs::read("suspect.png")?;
//! let key = hex::decode("deadbeef").unwrap();
//! match verify_image_bytes_async(img_bytes, key).await? {
//!     stegoeggo::VerificationStatus::Verified => println!("Verified: image is protected"),
//!     stegoeggo::VerificationStatus::Invalid => println!("Invalid: protection signature mismatch"),
//!     stegoeggo::VerificationStatus::NotFound => println!("Unprotected: no signature found"),
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use crate::types::{
    ExecutionReport, ProtectionContext, ProtectionLevel, ProtectionRequest, ProtectionWarning,
    VerificationStatus,
};
use image::DynamicImage;

fn join_err(e: tokio::task::JoinError) -> Error {
    if e.is_cancelled() {
        Error::Task(format!("cancelled: {e}"))
    } else if e.is_panic() {
        Error::Task(format!("panicked: {e}"))
    } else {
        Error::Task(format!("{e}"))
    }
}

/// Process a single image asynchronously.
///
/// Runs the protection pipeline on a blocking thread.
#[must_use = "the protected image should be saved or used"]
pub async fn process_image_async(
    img: DynamicImage,
    level: ProtectionLevel,
    ctx: ProtectionContext,
) -> Result<DynamicImage> {
    tokio::task::spawn_blocking(move || crate::process_image(img, level, &ctx))
        .await
        .map_err(join_err)?
}

/// Process image bytes asynchronously.
///
/// Automatically detects input format and applies protection. Returns
/// the protected image as bytes. This is the primary WAF hot path.
#[must_use = "the protected image bytes should be saved or used"]
pub async fn process_image_bytes_async(
    img_bytes: Vec<u8>,
    level: ProtectionLevel,
    ctx: ProtectionContext,
) -> Result<Vec<u8>> {
    tokio::task::spawn_blocking(move || crate::process_image_bytes(&img_bytes, level, &ctx))
        .await
        .map_err(join_err)?
}

/// Process image bytes asynchronously and return all protection warnings.
///
/// This is the preferred async API for reverse proxies because it keeps image
/// work on the blocking pool and lets the proxy log or enforce degraded
/// protection states before serving the bytes.
#[must_use = "the protected image bytes and warnings should be used"]
pub async fn process_image_bytes_with_warnings_async(
    img_bytes: Vec<u8>,
    level: ProtectionLevel,
    ctx: ProtectionContext,
) -> Result<(Vec<u8>, Vec<ProtectionWarning>)> {
    tokio::task::spawn_blocking(move || {
        crate::process_image_bytes_with_warnings(&img_bytes, level, &ctx)
    })
    .await
    .map_err(join_err)?
}

/// Process image bytes asynchronously using a [`ProtectionRequest`].
///
/// Canonical request-based async API. Calls [`crate::process_request_bytes`]
/// inside one `spawn_blocking` closure with no policy, routing, or warning
/// duplication.
#[must_use = "the protected image bytes should be saved or used"]
pub async fn process_request_bytes_async(
    img_bytes: Vec<u8>,
    request: ProtectionRequest,
) -> Result<Vec<u8>> {
    tokio::task::spawn_blocking(move || crate::process_request_bytes(&img_bytes, &request))
        .await
        .map_err(join_err)?
}

/// Process image bytes asynchronously using a [`ProtectionRequest`], returning warnings.
///
/// Canonical request-based async API. Calls
/// [`crate::process_request_bytes_with_warnings`] inside one `spawn_blocking`
/// closure with no policy, routing, or warning duplication.
#[must_use = "the protected image bytes and warnings should be used"]
pub async fn process_request_bytes_with_warnings_async(
    img_bytes: Vec<u8>,
    request: ProtectionRequest,
) -> Result<(Vec<u8>, Vec<ProtectionWarning>)> {
    tokio::task::spawn_blocking(move || {
        crate::process_request_bytes_with_warnings(&img_bytes, &request)
    })
    .await
    .map_err(join_err)?
}

/// Process image bytes asynchronously using a [`ProtectionRequest`], returning a report.
///
/// Canonical request-based async API. Calls
/// [`crate::process_request_bytes_with_report`] inside one `spawn_blocking`
/// closure with no policy, routing, or warning duplication.
#[must_use = "the protected image bytes and report should be used"]
pub async fn process_request_bytes_with_report_async(
    img_bytes: Vec<u8>,
    request: ProtectionRequest,
) -> Result<(Vec<u8>, ExecutionReport)> {
    tokio::task::spawn_blocking(move || {
        crate::process_request_bytes_with_report(&img_bytes, &request)
    })
    .await
    .map_err(join_err)?
}

/// Process multiple images asynchronously and in parallel.
///
/// Compatibility adapter over the legacy level/context API. Runs the entire
/// batch on a single blocking thread. New code should prefer
/// [`process_request_bytes_parallel_async`].
#[cfg(feature = "parallel")]
#[cfg_attr(docsrs, doc(cfg(feature = "parallel")))]
#[must_use = "the protected images should be saved or used"]
pub async fn process_images_parallel_async(
    images: Vec<DynamicImage>,
    level: ProtectionLevel,
    ctx: ProtectionContext,
) -> Result<Vec<DynamicImage>> {
    tokio::task::spawn_blocking(move || crate::process_images_parallel(&images, level, &ctx))
        .await
        .map_err(join_err)?
}

/// Process multiple image bytes asynchronously and in parallel.
///
/// Compatibility adapter over the legacy level/context API. Runs the entire
/// batch on a single blocking thread. New code should prefer
/// [`process_request_bytes_parallel_async`].
#[cfg(feature = "parallel")]
#[cfg_attr(docsrs, doc(cfg(feature = "parallel")))]
#[must_use = "the protected image bytes should be saved or used"]
pub async fn process_images_bytes_parallel_async(
    images: Vec<Vec<u8>>,
    level: ProtectionLevel,
    ctx: ProtectionContext,
) -> Result<Vec<Vec<u8>>> {
    tokio::task::spawn_blocking(move || crate::process_images_bytes_parallel(&images, level, &ctx))
        .await
        .map_err(join_err)?
}

/// Process multiple image byte buffers asynchronously using one shared [`ProtectionRequest`].
///
/// Canonical request-based async batch API. Runs the entire batch on a single
/// `spawn_blocking` closure that delegates to the synchronous Rayon batch
/// [`crate::process_request_bytes_parallel`], preserving input order.
#[cfg(feature = "parallel")]
#[cfg_attr(docsrs, doc(cfg(feature = "parallel")))]
#[must_use = "the protected image bytes should be saved or used"]
pub async fn process_request_bytes_parallel_async(
    images: Vec<Vec<u8>>,
    request: ProtectionRequest,
) -> Result<Vec<Vec<u8>>> {
    tokio::task::spawn_blocking(move || crate::process_request_bytes_parallel(&images, &request))
        .await
        .map_err(join_err)?
}

/// Process multiple image byte buffers asynchronously using one shared request, with warnings.
///
/// Canonical request-based async batch API. Delegates to
/// [`crate::process_request_bytes_with_warnings_parallel`] on a single
/// blocking thread, preserving input order.
#[cfg(feature = "parallel")]
#[cfg_attr(docsrs, doc(cfg(feature = "parallel")))]
#[must_use = "the protected image bytes and warnings should be used"]
pub async fn process_request_bytes_with_warnings_parallel_async(
    images: Vec<Vec<u8>>,
    request: ProtectionRequest,
) -> Result<Vec<(Vec<u8>, Vec<ProtectionWarning>)>> {
    tokio::task::spawn_blocking(move || {
        crate::process_request_bytes_with_warnings_parallel(&images, &request)
    })
    .await
    .map_err(join_err)?
}

/// Process multiple image byte buffers asynchronously using one shared request, with reports.
///
/// Canonical request-based async batch API. Delegates to
/// [`crate::process_request_bytes_with_report_parallel`] on a single blocking
/// thread, preserving input order.
#[cfg(feature = "parallel")]
#[cfg_attr(docsrs, doc(cfg(feature = "parallel")))]
#[must_use = "the protected image bytes and reports should be used"]
pub async fn process_request_bytes_with_report_parallel_async(
    images: Vec<Vec<u8>>,
    request: ProtectionRequest,
) -> Result<Vec<(Vec<u8>, ExecutionReport)>> {
    tokio::task::spawn_blocking(move || {
        crate::process_request_bytes_with_report_parallel(&images, &request)
    })
    .await
    .map_err(join_err)?
}

/// Verify image bytes asynchronously.
///
/// Checks for protection signatures via metadata, DCT stego, and LSB stego.
/// Returns `Ok(VerificationStatus::Verified)` if verified,
/// `Ok(VerificationStatus::Invalid)` if checked but invalid,
/// `Ok(VerificationStatus::NotFound)` if no protection data was found,
/// or `Err` if the task failed.
#[must_use = "the verification result should be checked"]
pub async fn verify_image_bytes_async(
    img_bytes: Vec<u8>,
    mac_key: Vec<u8>,
) -> Result<VerificationStatus> {
    tokio::task::spawn_blocking(move || crate::verify_image_bytes(&img_bytes, &mac_key))
        .await
        .map_err(join_err)
}
