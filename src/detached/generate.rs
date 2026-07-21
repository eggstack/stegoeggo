use sha2::{Digest, Sha256};

use crate::detached::manifest::DetachedManifest;
use crate::provenance::ProvenanceClaim;
use image::GenericImageView;

/// Create a detached manifest from image bytes and a provenance claim.
///
/// Computes the instance digest (SHA-256 of image bytes), populates
/// source facts from the image, and assembles a manifest ready for
/// signing.
///
/// # Arguments
///
/// * `image_bytes` - Raw image bytes.
/// * `claim` - A partially populated provenance claim. The `instance_digest`,
///   `format`, `width`, `height`, and `file_size` fields will be overwritten.
///
/// # Returns
///
/// A [`DetachedManifest`] with the claim populated and ready for signing.
///
/// # Errors
///
/// Returns an error if the image cannot be decoded to determine dimensions.
pub fn create_manifest_from_image(
    image_bytes: &[u8],
    mut claim: ProvenanceClaim,
) -> Result<DetachedManifest, crate::Error> {
    // Compute instance digest
    claim = claim.with_instance_digest(image_bytes);

    // Detect format and get dimensions
    let format = crate::types::ImageOutputFormat::from_magic_bytes(image_bytes)
        .map(|f| format!("{:?}", f).to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    let img = crate::util::image::load_image_from_bytes(image_bytes)?;
    let (width, height) = img.dimensions();

    claim = claim.with_source_facts(&format, width, height, image_bytes.len() as u64);

    Ok(DetachedManifest::new(claim))
}

/// Create a detached manifest from image bytes with a pre-computed claim.
///
/// Like [`create_manifest_from_image`], but accepts a fully populated
/// claim and only adds the instance digest and source facts.
pub fn create_manifest_with_claim(
    image_bytes: &[u8],
    claim: ProvenanceClaim,
) -> Result<DetachedManifest, crate::Error> {
    let mut claim = claim.with_instance_digest(image_bytes);

    let format = crate::types::ImageOutputFormat::from_magic_bytes(image_bytes)
        .map(|f| format!("{:?}", f).to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    let img = crate::util::image::load_image_from_bytes(image_bytes)?;
    let (width, height) = img.dimensions();

    claim = claim.with_source_facts(&format, width, height, image_bytes.len() as u64);

    Ok(DetachedManifest::new(claim))
}

/// Compute the SHA-256 digest of image bytes in the standard format.
#[must_use]
pub fn compute_image_digest(image_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(image_bytes);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}
