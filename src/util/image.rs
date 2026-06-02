use crate::error::{Error, Result};
use crate::protected::constants::XORSHIFT_SEED_OFFSET;
use digest::Digest;
use image::{DynamicImage, GenericImageView, ImageEncoder, ImageFormat};
use sha2::Sha256;

/// General-purpose XorShift64 PRNG for noise generation and pixel selection.
/// Not interchangeable with the F5-specific PRNG in `jpeg_transcoder/stego_f5.rs`.
///
/// **WARNING:** These two PRNG implementations use different algorithms. Do NOT
/// swap one for the other — they produce different sequences for the same seed
/// and are each paired with their respective embed/extract code paths.
pub struct XorShiftRng {
    state: u64,
}

impl XorShiftRng {
    #[inline]
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(XORSHIFT_SEED_OFFSET),
        }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    #[inline]
    pub fn gen_range_usize(&mut self, range: std::ops::Range<usize>) -> usize {
        if range.start >= range.end {
            return range.start;
        }
        let size = range.end - range.start;
        range.start + (self.next_u64() as usize % size)
    }
}

/// Compute a SHA-256 hash of an image's raw RGBA pixel data.
///
/// Returns the hex-encoded hash string. Used for cache key generation
/// in precomputed variant lookup.
pub fn compute_image_hash(img: &DynamicImage) -> String {
    let rgba = img.to_rgba8();
    let bytes = rgba.into_raw();

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();

    hex::encode(result)
}

/// Detect the image format from magic bytes.
///
/// Returns the `image::ImageFormat` if recognized (PNG, JPEG, or WebP), or `None`.
pub fn detect_image_format(bytes: &[u8]) -> Option<ImageFormat> {
    use crate::types::ImageOutputFormat;
    ImageOutputFormat::from_magic_bytes(bytes).map(|fmt| match fmt {
        ImageOutputFormat::Png => ImageFormat::Png,
        ImageOutputFormat::Jpeg => ImageFormat::Jpeg,
        ImageOutputFormat::WebP => ImageFormat::WebP,
    })
}

/// Encode an image to bytes in the given format with default quality (90).
///
/// For JPEG, uses [`jpeg_encoder`](jpeg_encoder) at quality 90.
/// For PNG and WebP, uses lossless encoding.
pub fn encode_image(img: &DynamicImage, format: ImageFormat) -> Result<Vec<u8>> {
    encode_image_with_quality(img, format, 90)
}

/// Core encoding function — encodes an image in the given format with quality setting.
/// Quality only affects JPEG; PNG and WebP ignore it.
pub fn encode_image_with_quality(
    img: &DynamicImage,
    format: ImageFormat,
    quality: u8,
) -> Result<Vec<u8>> {
    let (width, height) = img.dimensions();
    let mut buffer = Vec::with_capacity(match format {
        ImageFormat::Png => (width as usize) * (height as usize) * 4,
        ImageFormat::Jpeg => (width as usize) * (height as usize) * 3 / 2,
        ImageFormat::WebP => (width as usize) * (height as usize) * 4,
        _ => (width as usize) * (height as usize) * 4,
    });

    match format {
        ImageFormat::Png => {
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            encoder
                .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
                .map_err(|e| Error::ImageEncode(e.to_string()))?;
        }
        ImageFormat::Jpeg => {
            use jpeg_encoder::{ColorType, Encoder};
            let rgb = img.to_rgb8();
            let (w, h) = (rgb.width(), rgb.height());
            let encoder = Encoder::new(&mut buffer, quality);
            encoder
                .encode(rgb.as_raw(), w as u16, h as u16, ColorType::Rgb)
                .map_err(|e| Error::ImageEncode(e.to_string()))?;
        }
        ImageFormat::WebP => {
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buffer);
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            encoder
                .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
                .map_err(|e| Error::ImageEncode(e.to_string()))?;
        }
        _ => {
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            encoder
                .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
                .map_err(|e| Error::ImageEncode(e.to_string()))?;
        }
    }

    Ok(buffer)
}

/// Encode an image with format selection, progressive JPEG support, and quality control.
///
/// When `format` is `None`, defaults to [`DEFAULT_OUTPUT_FORMAT`](crate::types::DEFAULT_OUTPUT_FORMAT).
/// Progressive and quality options only affect JPEG output.
pub fn encode_image_with_options(
    img: &DynamicImage,
    format: Option<crate::types::ImageOutputFormat>,
    is_progressive: bool,
    quality: u8,
) -> Result<Vec<u8>> {
    let output_format = format.unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

    match output_format {
        crate::types::ImageOutputFormat::Jpeg => {
            use jpeg_encoder::{ColorType, Encoder};
            let rgb = img.to_rgb8();
            let (width, height) = (rgb.width(), rgb.height());
            let mut output = Vec::new();
            let mut encoder = Encoder::new(&mut output, quality);
            if is_progressive {
                encoder.set_progressive(true);
            }
            encoder
                .encode(rgb.as_raw(), width as u16, height as u16, ColorType::Rgb)
                .map_err(|e| Error::ImageEncode(e.to_string()))?;
            Ok(output)
        }
        crate::types::ImageOutputFormat::Png => encode_image(img, ImageFormat::Png),
        crate::types::ImageOutputFormat::WebP => encode_image(img, ImageFormat::WebP),
    }
}

/// Load a `DynamicImage` from raw bytes.
///
/// Delegates to `image::load_from_memory` with automatic format detection.
pub fn load_image_from_bytes(bytes: &[u8]) -> Result<DynamicImage> {
    Ok(image::load_from_memory(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    #[test]
    fn test_detect_png() {
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_image_format(&png_data), Some(ImageFormat::Png));
    }

    #[test]
    fn test_detect_jpeg() {
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_image_format(&jpeg_data), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn test_detect_webp() {
        let webp_data = vec![
            0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
        ];
        assert_eq!(detect_image_format(&webp_data), Some(ImageFormat::WebP));
    }

    #[test]
    fn test_image_hash_deterministic() {
        let img = RgbaImage::from_pixel(10, 10, image::Rgba([255, 0, 0, 255]));
        let dyn_img = DynamicImage::ImageRgba8(img);

        let hash1 = compute_image_hash(&dyn_img);
        let hash2 = compute_image_hash(&dyn_img);

        assert_eq!(hash1, hash2);
    }
}
