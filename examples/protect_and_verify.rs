use image::{DynamicImage, ImageBuffer, Rgba};
/// Full pipeline example: protect an image and verify the protection.
///
/// Demonstrates the complete workflow from image creation through protection
/// to steganographic verification and payload extraction.
use stegoeggo::{process_image_bytes, ProtectionContext, ProtectionLevel, SteganographyProtector};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a test image (in practice, load from file or network)
    let img: DynamicImage = DynamicImage::ImageRgba8(ImageBuffer::from_fn(256, 256, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
    }));

    // 2. Configure protection with a known seed for reproducibility
    let ctx = ProtectionContext::new(0.5, 42);

    // 3. Protect at Standard level (noise + stego + metadata)
    let protected = stegoeggo::process_image(img, ProtectionLevel::Standard, &ctx)?;
    println!(
        "Protected image: {}x{}",
        protected.width(),
        protected.height()
    );

    // 4. Verify protection using steganographic payload
    let stego = SteganographyProtector::new();
    if stego.verify_payload(&protected) {
        println!("Protection verified via steganography");

        if let Some(payload) = stego.extract_payload(&protected) {
            println!("  Level:     {}", payload.protection_level());
            println!("  Seed:      {}", payload.seed());
            println!("  Intensity: {:.2}", payload.intensity());
            println!("  Version:   {}", payload.version());
        }
    }

    // 5. Verify from raw bytes (e.g., after saving to disk)
    let png_bytes = stegoeggo::encode_image(&protected, image::ImageFormat::Png)?;
    let verified = stegoeggo::verify_image_bytes(&png_bytes, &[]);
    println!("Verified from bytes: {:?}", verified);

    // 6. Also works with process_image_bytes for byte-level I/O
    let input_bytes = std::fs::read("test_input.png").unwrap_or_else(|_| {
        // Fallback: encode the test image to PNG bytes
        stegoeggo::encode_image(
            &DynamicImage::ImageRgba8(ImageBuffer::from_fn(128, 128, |x, y| {
                Rgba([x as u8, y as u8, 128, 255])
            })),
            image::ImageFormat::Png,
        )
        .unwrap()
    });

    let protected_bytes = process_image_bytes(&input_bytes, ProtectionLevel::Standard, &ctx)?;
    println!("Protected bytes: {} bytes", protected_bytes.len());

    Ok(())
}
