use crate::error::{Error, Result};
use crate::protected::constants::XORSHIFT_SEED_OFFSET;
use digest::Digest;
use image::{DynamicImage, GenericImageView, ImageEncoder, ImageFormat};
use sha2::Sha256;

const MAX_JPEG_DIMENSION: u32 = u16::MAX as u32;

/// XorShift64 PRNG for stego pixel selection.
/// Not interchangeable with the DCT-specific PRNG in `jpeg_transcoder/stego_f5.rs`.
///
/// **WARNING:** These two PRNG implementations use different algorithms. Do NOT
/// swap one for the other — they produce different sequences for the same seed
/// and are each paired with their respective embed/extract code paths.
#[allow(dead_code)]
pub struct PixelSelectionRng {
    state: u64,
}

#[allow(dead_code)]
impl PixelSelectionRng {
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
        let size_u64 = size as u64;
        let zone = u64::MAX - (u64::MAX % size_u64);
        loop {
            let v = self.next_u64();
            if v < zone {
                return range.start + (v % size_u64) as usize;
            }
        }
    }
}

/// Compute a SHA-256 hash of an image's raw RGBA pixel data.
///
/// Returns the hex-encoded hash string. Used for cache key generation
/// in precomputed variant lookup.
pub fn compute_image_hash(img: &DynamicImage) -> String {
    let mut hasher = Sha256::new();
    if let Some(rgba) = img.as_rgba8() {
        hasher.update(rgba.as_raw());
    } else {
        let rgba = img.to_rgba8();
        hasher.update(rgba.as_raw());
    }
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
    if format == ImageFormat::Jpeg {
        return encode_jpeg(img, quality, false);
    }

    let (width, height) = img.dimensions();
    let mut buffer = Vec::with_capacity(match format {
        ImageFormat::Png => initial_capacity(width, height, 4),
        ImageFormat::WebP => initial_capacity(width, height, 4),
        _ => initial_capacity(width, height, 4),
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

fn initial_capacity(width: u32, height: u32, channels: usize) -> usize {
    const MAX_INITIAL_CAPACITY: usize = 8 * 1024 * 1024;
    let needed = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(channels);
    needed.min(MAX_INITIAL_CAPACITY)
}

fn ensure_jpeg_dimensions_supported(width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0 {
        return Err(Error::ImageEncode(
            "JPEG encoder requires non-zero image dimensions".to_string(),
        ));
    }
    if width > MAX_JPEG_DIMENSION || height > MAX_JPEG_DIMENSION {
        return Err(Error::DimensionsExceeded {
            width,
            height,
            max_width: MAX_JPEG_DIMENSION,
            max_height: MAX_JPEG_DIMENSION,
        });
    }
    crate::resource_limits::ResourceLimits::default().check_dimensions(width, height)?;
    Ok(())
}

fn encode_jpeg(img: &DynamicImage, quality: u8, is_progressive: bool) -> Result<Vec<u8>> {
    use jpeg_encoder::{ColorType, Encoder};

    let rgb = img.to_rgb8();
    let (width, height) = (rgb.width(), rgb.height());
    ensure_jpeg_dimensions_supported(width, height)?;

    let mut output = Vec::with_capacity(initial_capacity(width, height, 3) / 2);
    let mut encoder = Encoder::new(&mut output, quality);
    if is_progressive {
        encoder.set_progressive(true);
    }
    encoder
        .encode(rgb.as_raw(), width as u16, height as u16, ColorType::Rgb)
        .map_err(|e| Error::ImageEncode(e.to_string()))?;
    Ok(output)
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
        crate::types::ImageOutputFormat::Jpeg => encode_jpeg(img, quality, is_progressive),
        crate::types::ImageOutputFormat::Png => encode_image(img, ImageFormat::Png),
        crate::types::ImageOutputFormat::WebP => encode_image(img, ImageFormat::WebP),
    }
}

/// Load a `DynamicImage` from raw bytes.
///
/// Delegates to `image::load_from_memory` with automatic format detection.
pub fn load_image_from_bytes(bytes: &[u8]) -> Result<DynamicImage> {
    #[cfg(test)]
    LOAD_IMAGE_DECODE_COUNT.with(|count| count.set(count.get() + 1));
    Ok(image::load_from_memory(bytes)?)
}

#[cfg(test)]
thread_local! {
    static LOAD_IMAGE_DECODE_COUNT: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_load_decode_count() {
    LOAD_IMAGE_DECODE_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn load_decode_count() -> usize {
    LOAD_IMAGE_DECODE_COUNT.with(std::cell::Cell::get)
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

    #[test]
    fn jpeg_encoding_rejects_dimensions_that_do_not_fit_jpeg_encoder() {
        let too_wide = DynamicImage::new_rgb8(MAX_JPEG_DIMENSION + 1, 1);
        let err = encode_image(&too_wide, ImageFormat::Jpeg).unwrap_err();
        assert!(matches!(err, crate::Error::DimensionsExceeded { .. }));
        assert!(err.to_string().contains("exceeds"));

        let too_tall = DynamicImage::new_rgb8(1, MAX_JPEG_DIMENSION + 1);
        let err = encode_image_with_options(
            &too_tall,
            Some(crate::types::ImageOutputFormat::Jpeg),
            true,
            90,
        )
        .unwrap_err();
        assert!(matches!(err, crate::Error::DimensionsExceeded { .. }));
        assert!(err.to_string().contains("exceeds"));
    }

    #[test]
    fn pixel_selection_rng_zero_seed_is_stable_and_nonzero() {
        let mut a = PixelSelectionRng::new(0);
        let mut b = PixelSelectionRng::new(0);
        let first_a = a.next_u64();
        let first_b = b.next_u64();
        assert_eq!(
            first_a, first_b,
            "PixelSelectionRng(0) must be deterministic"
        );
        assert_ne!(first_a, 0, "zero seed must not produce zero state");

        let mut c = PixelSelectionRng::new(1);
        assert_ne!(
            first_a,
            c.next_u64(),
            "different seeds must produce different sequences"
        );

        let mut d = PixelSelectionRng::new(0);
        let second = {
            d.next_u64();
            d.next_u64()
        };
        assert_ne!(first_a, second);
    }

    #[test]
    fn pixel_and_dct_rngs_produce_different_sequences_for_same_seed() {
        let mut pix = PixelSelectionRng::new(0);
        let pix_first = pix.next_u64();
        const DCT_FIRST_FOR_ZERO: u64 = 0x40822041;
        assert_ne!(
            pix_first, DCT_FIRST_FOR_ZERO,
            "PixelSelectionRng(0) {pix_first:#x} must differ from DctCoefficientRng(0) {DCT_FIRST_FOR_ZERO:#x} — do not unify the two RNGs"
        );
        let mut pix42 = PixelSelectionRng::new(42);
        let pix42_first = pix42.next_u64();
        const DCT_FIRST_FOR_42: u64 = 0xa95514aaa;
        assert_ne!(
            pix42_first, DCT_FIRST_FOR_42,
            "PixelSelectionRng(42) must differ from DctCoefficientRng(42)"
        );
    }
}
