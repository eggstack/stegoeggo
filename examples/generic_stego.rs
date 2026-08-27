/// Generic carrier API example: embed and extract arbitrary bytes.
///
/// Demonstrates raw, in-place, and framed round-trips for both LSB
/// (pixel-domain) and JPEG (DCT-domain) carriers using `stegoeggo::stego`.
use image::{ImageBuffer, Rgb, RgbaImage};
use stegoeggo::stego::{
    jpeg::{self, JpegConfig},
    lsb::{self, LsbConfig},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let secret = b"hello from stegoeggo generic carrier API";
    let seed: u64 = 42;

    // --- LSB (pixel-domain) raw round-trip ---
    let img = make_lsb_image(128, 128);

    let config = LsbConfig::new(seed);
    let capacity = lsb::capacity(&img, secret.len(), &config)?;
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

    // --- LSB in-place round-trip (no full-image clone) ---
    let mut img_in_place = make_lsb_image(128, 128);
    let in_place_report = lsb::embed_in_place(&mut img_in_place, secret, &config)?;
    println!(
        "LSB in-place embedded: {} ({} bytes)",
        in_place_report.embedded, in_place_report.payload_bytes
    );

    let recovered_in_place = lsb::extract(&img_in_place, secret.len(), &config)?;
    println!(
        "LSB in-place extracted: {:?}",
        String::from_utf8_lossy(&recovered_in_place)
    );

    // --- LSB config from untrusted runtime value ---
    let runtime_redundancy: usize = 3;
    let trusted_config = LsbConfig::try_new(seed, runtime_redundancy)?;
    println!(
        "LSB config from runtime: redundancy {}",
        trusted_config.redundancy()
    );

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

    // --- Framed LSB round-trip (no caller-known length needed) ---
    let img2 = make_lsb_image(128, 128);
    let lsb_config = LsbConfig::new(seed);
    let report = lsb::embed_framed(&img2, secret, &lsb_config)?;

    let recovered = lsb::extract_framed(&report.output, &lsb_config)?;
    println!(
        "Framed LSB extracted: {:?}",
        String::from_utf8_lossy(&recovered)
    );

    // --- Framed JPEG round-trip (no caller-known length or redundancy needed) ---
    let jpeg_framed = make_test_jpeg(128, 128);
    let jpeg_framed_config = JpegConfig::new(seed).with_redundancy(3);
    if jpeg::probe_support(&jpeg_framed)? == jpeg::JpegSupport::Supported {
        let report = jpeg::embed_framed(&jpeg_framed, secret, &jpeg_framed_config)?;
        let recovered = jpeg::extract_framed(&report.output, &jpeg_framed_config)?;
        println!(
            "Framed JPEG extracted: {:?}",
            String::from_utf8_lossy(&recovered)
        );
    }

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
