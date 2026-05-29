/// CDN precomputed variant example.
///
/// Demonstrates the two-phase workflow for CDN/WAF edge deployment:
/// 1. At upload time: generate and register perturbation data
/// 2. At serve time: look up and apply precomputed variants
use cloakrs::{
    compute_image_hash, PrecomputedProtector, ProtectedVariant, ProtectionContext, ProtectionLevel,
    Protector,
};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // === Phase 1: Upload-time precomputation ===

    // Load the original image
    let original: DynamicImage =
        DynamicImage::ImageRgba8(ImageBuffer::from_fn(128, 128, |x, y| {
            Rgba([(x * 2) as u8, (y * 3) as u8, 128, 255])
        }));

    // Compute hash for cache key lookup
    let hash = compute_image_hash(&original);
    let (width, height) = original.dimensions();

    // Configure protection
    let ctx = ProtectionContext::new(0.5, 42);

    // Generate perturbation data
    let precomputed = PrecomputedProtector::new();
    let perturbation = precomputed.generate_perturbation_data(width, height, &ctx)?;
    println!(
        "Generated perturbation: {} bytes ({}x{})",
        perturbation.len(),
        width,
        height
    );

    // Register the variant for later lookup
    let variant = ProtectedVariant::new(
        hash.clone(),
        ProtectionLevel::Strong,
        perturbation,
        0.5,
        width,
        height,
    );
    precomputed.register_variant(variant.clone())?;

    // Serialize for storage (Redis, database, filesystem)
    let json = serde_json::to_string(&variant)?;
    println!("Variant JSON: {} bytes", json.len());

    // === Phase 2: Serve-time edge application ===

    // Deserialize from storage
    let loaded_variant: ProtectedVariant = serde_json::from_str(&json)?;

    // Create a fresh protector and register the loaded variant
    let edge_protector = PrecomputedProtector::new();
    edge_protector.register_variant(loaded_variant)?;

    // Apply protection to the original image
    let ctx = ProtectionContext::new(0.5, 42);
    let protected = edge_protector.apply(&original, &ctx)?;
    println!(
        "Protected: {}x{}, hash: {}",
        protected.width(),
        protected.height(),
        compute_image_hash(&protected)
    );

    // Verify that same seed + same image = deterministic output
    let protected2 = edge_protector.apply(&original, &ctx)?;
    assert_eq!(
        compute_image_hash(&protected),
        compute_image_hash(&protected2),
        "Same seed should produce identical output"
    );
    println!("Determinism verified");

    Ok(())
}
