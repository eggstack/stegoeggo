//! Async wrappers for WAF/CDN edge integration.
//!
//! Uses `tokio::task::spawn_blocking` to run CPU-bound image protection
//! on the blocking thread pool, keeping the async runtime responsive.
//!
//! All functions take owned data (`Vec<u8>`, `DynamicImage`) rather than
//! borrows to satisfy `spawn_blocking`'s `Send + 'static` requirement.
//!
//! # Usage
//!
//! ## Single image processing (WAF hot path)
//!
//! ```no_run
//! use cloakrs::{process_image_bytes_async, ProtectionContext, ProtectionLevel};
//!
//! # #[tokio::main] async fn main() -> Result<(), cloakrs::Error> {
//! let ctx = ProtectionContext::new(0.5, 42);
//! let bytes: Vec<u8> = std::fs::read("input.png")?;
//! let protected = process_image_bytes_async(bytes, ProtectionLevel::Standard, ctx).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Parallel batch processing (CDN origin)
//!
//! ```no_run
//! use cloakrs::{process_images_bytes_parallel_async, ProtectionContext, ProtectionLevel};
//!
//! # #[tokio::main] async fn main() -> Result<(), cloakrs::Error> {
//! let ctx = ProtectionContext::new(0.5, 42);
//! let images: Vec<Vec<u8>> = vec![std::fs::read("a.png")?, std::fs::read("b.png")?];
//! let protected = process_images_bytes_parallel_async(
//!     images,
//!     ProtectionLevel::Standard,
//!     ctx,
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Verification endpoint
//!
//! ```no_run
//! use cloakrs::verify_image_bytes_async;
//!
//! # #[tokio::main] async fn main() -> Result<(), cloakrs::Error> {
//! let img_bytes: Vec<u8> = std::fs::read("suspect.png")?;
//! let key = hex::decode("deadbeef").unwrap();
//! match verify_image_bytes_async(img_bytes, key).await? {
//!     Some(true) => println!("Verified: image is protected"),
//!     Some(false) => println!("Invalid: protection signature mismatch"),
//!     None => println!("Unprotected: no signature found"),
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use crate::types::{ProtectionContext, ProtectionLevel};
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

/// Process multiple images asynchronously and in parallel.
///
/// Spawns each image on a separate blocking thread. Avoids double-pooling
/// by not using rayon internally — tokio's blocking pool handles parallelism.
#[must_use = "the protected images should be saved or used"]
pub async fn process_images_parallel_async(
    images: Vec<DynamicImage>,
    level: ProtectionLevel,
    ctx: ProtectionContext,
) -> Result<Vec<DynamicImage>> {
    let handles: Vec<_> = images
        .into_iter()
        .map(|img| {
            let ctx = ctx.clone();
            tokio::task::spawn_blocking(move || crate::process_image(img, level, &ctx))
        })
        .collect();

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        results.push(handle.await.map_err(join_err)??);
    }
    Ok(results)
}

/// Process multiple image bytes asynchronously and in parallel.
///
/// Spawns each image on a separate blocking thread. Avoids double-pooling
/// by not using rayon internally — tokio's blocking pool handles parallelism.
#[must_use = "the protected image bytes should be saved or used"]
pub async fn process_images_bytes_parallel_async(
    images: Vec<Vec<u8>>,
    level: ProtectionLevel,
    ctx: ProtectionContext,
) -> Result<Vec<Vec<u8>>> {
    let handles: Vec<_> = images
        .into_iter()
        .map(|img_bytes| {
            let ctx = ctx.clone();
            tokio::task::spawn_blocking(move || crate::process_image_bytes(&img_bytes, level, &ctx))
        })
        .collect();

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        results.push(handle.await.map_err(join_err)??);
    }
    Ok(results)
}

/// Verify image bytes asynchronously.
///
/// Checks for protection signatures via metadata, DCT stego, and LSB stego.
/// Returns `Ok(Some(true))` if verified, `Ok(Some(false))` if checked but invalid,
/// `Ok(None)` if no protection data was found, or `Err` if the task failed.
#[must_use = "the verification result should be checked"]
pub async fn verify_image_bytes_async(
    img_bytes: Vec<u8>,
    mac_key: Vec<u8>,
) -> Result<Option<bool>> {
    tokio::task::spawn_blocking(move || crate::verify_image_bytes(&img_bytes, &mac_key))
        .await
        .map_err(join_err)
}
