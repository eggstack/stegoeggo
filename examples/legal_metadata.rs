use image::{DynamicImage, ImageBuffer, Rgba};
/// Legal metadata injection example.
///
/// Demonstrates how to inject copyright and usage restrictions
/// into images you own for IP protection.
use stegoeggo::{LegalMetadata, MetadataTrapProtector, ProtectionContext, ProtectionLevel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an image you own
    let img: DynamicImage = DynamicImage::ImageRgba8(ImageBuffer::from_fn(128, 128, |x, y| {
        Rgba([(x * 3) as u8, (y * 5) as u8, 128, 255])
    }));

    // Configure protection with legal metadata.
    // Both with_legal_metadata() AND with_legal_claims(true) are required —
    // one provides the content, the other enables injection.
    let ctx = ProtectionContext::new(0.5, 42)
        .with_dmi(stegoeggo::DmiValue::ProhibitedAiMlTraining)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Example Corp")
                .with_contact_email("legal@example.com")
                .with_usage_terms("All Rights Reserved. No AI training permitted.")
                .with_license_url("https://example.com/license"),
        )
        .with_legal_claims(true);

    // Process at Standard level (noise + stego + metadata)
    let protected = stegoeggo::process_image(img, ProtectionLevel::Standard, &ctx)?;
    println!(
        "Protected with legal metadata: {}x{}",
        protected.width(),
        protected.height()
    );

    // Encode to PNG to get byte-level output with metadata
    let png_bytes = stegoeggo::encode_image(&protected, image::ImageFormat::Png)?;

    // Verify the seed is extractable from metadata
    if let Some(seed) = MetadataTrapProtector::extract_seed_from_image(&png_bytes) {
        println!("Extracted seed from metadata: {}", seed);
    }

    // Verify steganographic protection
    let verified = stegoeggo::verify_image_bytes(&png_bytes, &[]);
    println!("Protection verified: {:?}", verified);

    println!("\nOutput contains:");
    println!("  - X-Protection-Seed metadata");
    println!("  - IPTC DMI tag: ProhibitedAiMlTraining");
    println!("  - Copyright: Example Corp");
    println!("  - Contact: legal@example.com");
    println!("  - Usage terms: All Rights Reserved. No AI training permitted.");

    Ok(())
}
