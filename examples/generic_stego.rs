/// Generic carrier API example: embed and extract arbitrary bytes.
///
/// Demonstrates raw and framed round-trips for both LSB (pixel-domain)
/// and JPEG (DCT-domain) carriers using `stegoeggo::stego`.
use image::{ImageBuffer, Rgb, RgbaImage};
use stegoeggo::stego::{
    frame,
    jpeg::{self, JpegConfig},
    lsb::{self, LsbConfig},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let secret = b"hello from stegoeggo generic carrier API";
    let seed: u64 = 42;

    // --- LSB (pixel-domain) raw round-trip ---
    let img = make_lsb_image(128, 128);

    let config = LsbConfig::new(seed);
    let capacity = lsb::capacity(&img, secret.len(), &config);
    println!(
        "LSB capacity: {} available, {} required",
        capacity.available, capacity.required
    );

    let report = lsb::embed(&img, secret, &config)?;
    println!(
        "LSB embedded: {} ({} bytes)",
        report.embedded, report.payload_bytes
    );

    let recovered = lsb::extract(&report.output, secret.len(), &config)?;
    println!("LSB extracted: {:?}", String::from_utf8_lossy(&recovered));

    // --- JPEG (DCT-domain) raw round-trip ---
    let jpeg_bytes = make_test_jpeg(128, 128);
    let jpeg_config = JpegConfig::new(seed).with_redundancy(2);

    match jpeg::probe_support(&jpeg_bytes)? {
        jpeg::JpegSupport::Supported => {
            let report = jpeg::embed(&jpeg_bytes, secret, &jpeg_config)?;
            println!(
                "JPEG embedded: {} (actual redundancy: {})",
                report.embedded, report.actual_redundancy
            );

            let recovered = jpeg::extract(
                &report.output,
                secret.len(),
                &jpeg_config,
                report.actual_redundancy,
            )?;
            println!("JPEG extracted: {:?}", String::from_utf8_lossy(&recovered));
        }
        jpeg::JpegSupport::Unsupported(reason) => {
            println!("JPEG DCT not supported for this image: {reason:?}");
        }
    }

    // --- Framed round-trip (no caller-known length needed) ---
    let img2 = make_lsb_image(128, 128);
    let framed = frame::encode(secret)?;
    let lsb_config = LsbConfig::new(seed);
    let report = lsb::embed(&img2, &framed, &lsb_config)?;

    let raw = lsb::extract(&report.output, framed.len(), &lsb_config)?;
    let (_, payload) = frame::decode(&raw)?;
    println!("Framed extracted: {:?}", String::from_utf8_lossy(&payload));

    Ok(())
}

fn make_lsb_image(w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let r = ((x * 7 + y * 13) % 256) as u8;
            let g = ((x * 11 + y * 3) % 256) as u8;
            let b = ((x * 5 + y * 17) % 256) as u8;
            img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
        }
    }
    img
}

fn make_test_jpeg(w: u32, h: u32) -> Vec<u8> {
    let img = ImageBuffer::<Rgb<u8>, _>::from_fn(w, h, |x, y| Rgb([x as u8, y as u8, 128]));
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
    image::DynamicImage::ImageRgb8(img)
        .write_with_encoder(encoder)
        .unwrap();
    buf
}
