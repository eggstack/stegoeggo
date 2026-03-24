use cloakrs::{ProtectionContext, Protector, SteganographyProtector};
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};

fn main() {
    let stego = SteganographyProtector::new();

    // Create a simple test image
    let img: RgbaImage = ImageBuffer::from_fn(100, 100, |_x, _y| Rgba([128, 128, 128, 255]));
    let dyn_img = DynamicImage::ImageRgba8(img);

    let ctx = ProtectionContext::new(cloakrs::types::TargetModel::default(), 0.5, 42)
        .with_format(cloakrs::types::ImageOutputFormat::Png);

    println!("Embedding stego...");
    let processed = stego.apply(&dyn_img, &ctx).unwrap();

    println!("Verifying stego...");
    if stego.verify_payload(&processed) {
        println!("✓ Stego verified!");
    } else {
        println!("✗ No stego found");
    }
}
