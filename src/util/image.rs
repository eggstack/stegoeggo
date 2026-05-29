use crate::error::{Error, Result};
use crate::protected::constants::XORSHIFT_SEED_OFFSET;
use digest::Digest;
use hmac::{Hmac, Mac};
use image::RgbaImage;
use image::{DynamicImage, GenericImageView, ImageEncoder, ImageFormat};
use rayon::prelude::*;
use sha2::Sha256;
use std::sync::LazyLock;

use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

const SIN_TABLE_SIZE: usize = 1024;
static SIN_TABLE: LazyLock<[f32; SIN_TABLE_SIZE]> = LazyLock::new(|| {
    let mut table = [0.0f32; SIN_TABLE_SIZE];
    for (i, entry) in table.iter_mut().enumerate() {
        let angle = (i as f32 / SIN_TABLE_SIZE as f32) * std::f32::consts::TAU;
        *entry = angle.sin();
    }
    table
});

#[inline(always)]
fn fast_sin(angle: f32) -> f32 {
    let normalized = angle.rem_euclid(std::f32::consts::TAU);
    let index = ((normalized / std::f32::consts::TAU) * SIN_TABLE_SIZE as f32 + 0.5) as usize;
    let index = index % SIN_TABLE_SIZE;
    SIN_TABLE[index]
}

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
    pub fn gen_f32(&mut self) -> f32 {
        // Use top 24 bits for full f32 mantissa precision
        (self.next_u64() >> 40) as f32 / 16777216.0 * 2.0 - 1.0
    }

    #[inline]
    pub fn gen_range(&mut self, range: std::ops::Range<f32>) -> f32 {
        // Use top 24 bits for full f32 mantissa precision
        let t = (self.next_u64() >> 40) as f32 / 16777216.0;
        range.start + t * (range.end - range.start)
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

/// Noise generation engine for adversarial perturbation.
///
/// Pre-computes the HMAC key (if configured) and spatial seed once,
/// then reuses them across multiple perturbation passes. Shared
/// between serial and parallel perturbation paths.
pub struct NoiseGenerator {
    seed: u64,
    mac_key: Option<Arc<[u8]>>,
}

impl NoiseGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            mac_key: None,
        }
    }

    pub fn with_mac_key(seed: u64, mac_key: impl Into<Arc<[u8]>>) -> Self {
        Self {
            seed,
            mac_key: Some(mac_key.into()),
        }
    }

    pub fn derive_keyed_seed(&self, pixel_pos: u64) -> u64 {
        let mac_key = self.mac_key.as_deref().unwrap_or(&[]);
        if mac_key.is_empty() {
            return self.seed;
        }

        let mut mac = HmacSha256::new_from_slice(mac_key).expect("HMAC can take key of any size");
        mac.update(&self.seed.to_le_bytes());
        mac.update(&pixel_pos.to_le_bytes());
        let result = mac.finalize().into_bytes();
        u64::from_le_bytes([
            result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7],
        ])
    }
}

const VARIATION_MIN: f32 = 0.98;
const VARIATION_RANGE: f32 = 0.04;
const SPATIAL_SEED_TAG: u64 = 0x12345678;

/// Pre-computed parameters shared between serial and parallel perturbation paths.
/// Retains the `NoiseGenerator` so callers can derive additional seeds (e.g. spatial)
/// without recomputing HMAC.
struct PerturbationParams {
    intensity: f32,
    blocks_x: usize,
    keyed_seed_base: u64,
    inv_pattern_scale: f32,
    intensity_factor: f32,
    phase_offset: f32,
    noise_gen: NoiseGenerator,
}

impl PerturbationParams {
    fn new(
        seed: u64,
        intensity: f32,
        intensity_multiplier: f32,
        mac_key: &[u8],
        width: u32,
    ) -> Self {
        let noise_gen = if mac_key.is_empty() {
            NoiseGenerator::new(seed)
        } else {
            NoiseGenerator::with_mac_key(seed, mac_key)
        };
        let frequency_seed = noise_gen.derive_keyed_seed(0xDEADBEEF);
        let mut frequency_rng = XorShiftRng::new(frequency_seed);

        let pattern_scale = ((frequency_rng.next_u64() % 8) as f32) + 8.0;
        let two_pi = std::f32::consts::TAU;

        Self {
            intensity,
            blocks_x: width.div_ceil(4) as usize,
            keyed_seed_base: noise_gen.derive_keyed_seed(0),
            inv_pattern_scale: two_pi / pattern_scale,
            intensity_factor: intensity * intensity_multiplier,
            phase_offset: (frequency_rng.next_u64() as f32 / u64::MAX as f32) * two_pi,
            noise_gen,
        }
    }

    fn derive_spatial_seed(&self) -> u64 {
        self.noise_gen.derive_keyed_seed(SPATIAL_SEED_TAG)
    }

    /// Compute the perturbed RGB values for a single pixel.
    #[inline(always)]
    fn perturb_pixel(
        &self,
        x: usize,
        y: usize,
        y_phase: f32,
        y_variation: f32,
        orig: (i32, i32, i32),
    ) -> (u8, u8, u8) {
        let (orig_r, orig_g, orig_b) = orig;
        let block_idx = (y / 4) * self.blocks_x + (x / 4);
        let keyed_seed = self.keyed_seed_base.wrapping_add(block_idx as u64);

        let mut block_rng = XorShiftRng::new(keyed_seed);
        let noise_r =
            (block_rng.gen_f32() * self.intensity * 64.0 + 128.0).clamp(0.0, 255.0) as i16;
        let noise_g =
            (block_rng.gen_f32() * self.intensity * 64.0 + 128.0).clamp(0.0, 255.0) as i16;
        let noise_b =
            (block_rng.gen_f32() * self.intensity * 64.0 + 128.0).clamp(0.0, 255.0) as i16;

        let blended_r = (orig_r + i32::from(noise_r - 128) / 4).clamp(0, 255) as u8;
        let blended_g = (orig_g + i32::from(noise_g - 128) / 4).clamp(0, 255) as u8;
        let blended_b = (orig_b + i32::from(noise_b - 128) / 4).clamp(0, 255) as u8;

        let varied_r = ((blended_r as f32) * y_variation).clamp(0.0, 255.0) as u8;
        let varied_g = ((blended_g as f32) * y_variation).clamp(0.0, 255.0) as u8;
        let varied_b = ((blended_b as f32) * y_variation).clamp(0.0, 255.0) as u8;

        let phase = x as f32 * self.inv_pattern_scale + y_phase + self.phase_offset;
        let perturbation = (fast_sin(phase) * self.intensity_factor) as i16;

        (
            (i16::from(varied_r) + perturbation).clamp(0, 255) as u8,
            (i16::from(varied_g) + perturbation).clamp(0, 255) as u8,
            (i16::from(varied_b) + perturbation).clamp(0, 255) as u8,
        )
    }
}

/// Pre-computed runtime state shared between serial and parallel perturbation paths.
/// Holds per-row variation factors and the shared perturbation parameters.
struct PerturbationRuntime {
    params: PerturbationParams,
    y_variations: Vec<f32>,
}

impl PerturbationRuntime {
    fn new(
        seed: u64,
        intensity: f32,
        intensity_multiplier: f32,
        mac_key: &[u8],
        width: u32,
        height: usize,
    ) -> Self {
        let params = PerturbationParams::new(seed, intensity, intensity_multiplier, mac_key, width);

        let spatial_seed = params.derive_spatial_seed();
        let mut spatial_rng = XorShiftRng::new(spatial_seed);

        let y_variations: Vec<f32> = (0..height)
            .map(|_| {
                VARIATION_MIN + (spatial_rng.next_u64() as f32 / u64::MAX as f32) * VARIATION_RANGE
            })
            .collect();

        Self {
            params,
            y_variations,
        }
    }
}

/// Parallel version of single-pass perturbation for large images.
/// Pre-computes per-row spatial seed values then parallelizes across rows.
pub fn apply_perturbation_single_pass_keyed_par(
    img: &RgbaImage,
    seed: u64,
    intensity: f32,
    intensity_multiplier: f32,
    mac_key: &[u8],
) -> DynamicImage {
    let (width, height) = img.dimensions();
    let width_usize = width as usize;
    let height_usize = height as usize;

    let img_raw = img.as_raw();
    let total_pixels = width_usize * height_usize;
    let mut output_raw = vec![0u8; total_pixels * 4];

    let runtime = PerturbationRuntime::new(
        seed,
        intensity,
        intensity_multiplier,
        mac_key,
        width,
        height_usize,
    );

    output_raw
        .par_chunks_mut(width_usize * 4)
        .enumerate()
        .for_each(|(y, row)| {
            let y_variation = runtime.y_variations[y];
            let y_phase = y as f32 * runtime.params.inv_pattern_scale;

            for x in 0..width_usize {
                let orig_r = img_raw[(y * width_usize + x) * 4] as i32;
                let orig_g = img_raw[(y * width_usize + x) * 4 + 1] as i32;
                let orig_b = img_raw[(y * width_usize + x) * 4 + 2] as i32;
                let orig_a = img_raw[(y * width_usize + x) * 4 + 3];

                let (r, g, b) = runtime.params.perturb_pixel(
                    x,
                    y,
                    y_phase,
                    y_variation,
                    (orig_r, orig_g, orig_b),
                );

                let idx = x * 4;
                row[idx] = r;
                row[idx + 1] = g;
                row[idx + 2] = b;
                row[idx + 3] = orig_a;
            }
        });

    DynamicImage::ImageRgba8(
        RgbaImage::from_raw(width, height, output_raw)
            .unwrap_or_else(|| RgbaImage::new(width, height)),
    )
}

/// Convenience wrapper that selects serial or parallel based on image size.
pub fn apply_perturbation_single_pass(
    img: &RgbaImage,
    seed: u64,
    intensity: f32,
    intensity_multiplier: f32,
) -> DynamicImage {
    apply_perturbation_single_pass_keyed(img, seed, intensity, intensity_multiplier, &[])
}

/// Convenience wrapper that selects serial or parallel based on image size.
pub fn apply_perturbation_single_pass_keyed(
    img: &RgbaImage,
    seed: u64,
    intensity: f32,
    intensity_multiplier: f32,
    mac_key: &[u8],
) -> DynamicImage {
    let (width, height) = img.dimensions();
    let total_pixels = (width as usize) * (height as usize);

    if total_pixels >= parallel_threshold() {
        apply_perturbation_single_pass_keyed_par(
            img,
            seed,
            intensity,
            intensity_multiplier,
            mac_key,
        )
    } else {
        apply_perturbation_single_pass_keyed_serial(
            img,
            seed,
            intensity,
            intensity_multiplier,
            mac_key,
        )
    }
}

fn apply_perturbation_single_pass_keyed_serial(
    img: &RgbaImage,
    seed: u64,
    intensity: f32,
    intensity_multiplier: f32,
    mac_key: &[u8],
) -> DynamicImage {
    let (width, height) = img.dimensions();
    let width_usize = width as usize;
    let height_usize = height as usize;

    let img_raw = img.as_raw();
    let total_pixels = width_usize * height_usize;
    let mut output_raw = vec![0u8; total_pixels * 4];

    let runtime = PerturbationRuntime::new(
        seed,
        intensity,
        intensity_multiplier,
        mac_key,
        width,
        height_usize,
    );

    for y in 0..height_usize {
        let y_variation = runtime.y_variations[y];
        let y_phase = y as f32 * runtime.params.inv_pattern_scale;
        let row_offset = y * width_usize * 4;

        for x in 0..width_usize {
            let idx = row_offset + x * 4;
            let orig_r = img_raw[idx] as i32;
            let orig_g = img_raw[idx + 1] as i32;
            let orig_b = img_raw[idx + 2] as i32;
            let orig_a = img_raw[idx + 3];

            let (r, g, b) =
                runtime
                    .params
                    .perturb_pixel(x, y, y_phase, y_variation, (orig_r, orig_g, orig_b));

            output_raw[idx] = r;
            output_raw[idx + 1] = g;
            output_raw[idx + 2] = b;
            output_raw[idx + 3] = orig_a;
        }
    }

    DynamicImage::ImageRgba8(
        RgbaImage::from_raw(width, height, output_raw)
            .unwrap_or_else(|| RgbaImage::new(width, height)),
    )
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

/// Apply a precomputed RGBA perturbation to an image (serial).
///
/// The `perturbation` buffer must be `width * height * 4` bytes (RGBA per pixel).
/// Each channel is adjusted by `(perturbation - 128) / divisor`, clamped to [0, 255].
pub fn apply_perturbation(img: &RgbaImage, perturbation: &[u8], divisor: i16) -> Result<RgbaImage> {
    let (width, height) = img.dimensions();

    if perturbation.len() != (width * height * 4) as usize {
        return Err(Error::ImageDecode("Perturbation size mismatch".to_string()));
    }

    debug_assert!(
        divisor != 0,
        "divisor must not be zero in apply_perturbation"
    );

    let img_raw = img.as_raw();
    let mut output_raw = vec![0u8; img_raw.len()];

    for i in (0..img_raw.len()).step_by(4) {
        let px_r = img_raw[i] as i16;
        let px_g = img_raw[i + 1] as i16;
        let px_b = img_raw[i + 2] as i16;
        let px_a = img_raw[i + 3];

        let perturbation_offset = [
            perturbation[i] as i16 - 128,
            perturbation[i + 1] as i16 - 128,
            perturbation[i + 2] as i16 - 128,
        ];

        output_raw[i] = (px_r + perturbation_offset[0] / divisor).clamp(0, 255) as u8;
        output_raw[i + 1] = (px_g + perturbation_offset[1] / divisor).clamp(0, 255) as u8;
        output_raw[i + 2] = (px_b + perturbation_offset[2] / divisor).clamp(0, 255) as u8;
        output_raw[i + 3] = px_a;
    }

    RgbaImage::from_raw(width, height, output_raw)
        .ok_or_else(|| Error::ImageDecode("Failed to create image".to_string()))
}

/// Returns the pixel count threshold at which parallelism is worthwhile.
/// Scales with rayon's thread pool size to avoid unnecessary parallel
/// overhead on few-core machines or over-parallelization on many cores.
/// Returns the pixel count threshold above which parallel processing is used.
///
/// Scales with `rayon::current_num_threads()` — returns `cores * 64 * 64`.
/// At 4 cores: 256x256 = 65536. At 1 core: 4096.
pub fn parallel_threshold() -> usize {
    let cores = rayon::current_num_threads().max(1);
    cores * 64 * 64
}

pub fn apply_perturbation_par(
    img: &RgbaImage,
    perturbation: &[u8],
    divisor: i16,
) -> Result<RgbaImage> {
    let (width, height) = img.dimensions();
    let total_pixels = (width * height) as usize;

    if perturbation.len() != total_pixels * 4 {
        return Err(Error::ImageDecode("Perturbation size mismatch".to_string()));
    }

    if total_pixels < parallel_threshold() {
        return apply_perturbation(img, perturbation, divisor);
    }

    debug_assert!(
        divisor != 0,
        "divisor must not be zero in apply_perturbation"
    );

    let img_raw = img.as_raw();

    let output_raw: Vec<u8> = (0..total_pixels)
        .into_par_iter()
        .with_min_len(1024)
        .flat_map(|p| {
            let i = p * 4;
            let px_r = img_raw[i] as i16;
            let px_g = img_raw[i + 1] as i16;
            let px_b = img_raw[i + 2] as i16;
            let px_a = img_raw[i + 3];

            let perturbation_offset = [
                perturbation[i] as i16 - 128,
                perturbation[i + 1] as i16 - 128,
                perturbation[i + 2] as i16 - 128,
            ];

            let r = (px_r + perturbation_offset[0] / divisor).clamp(0, 255) as u8;
            let g = (px_g + perturbation_offset[1] / divisor).clamp(0, 255) as u8;
            let b = (px_b + perturbation_offset[2] / divisor).clamp(0, 255) as u8;
            [r, g, b, px_a]
        })
        .collect();

    RgbaImage::from_raw(width, height, output_raw)
        .ok_or_else(|| Error::ImageDecode("Failed to create image".to_string()))
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
    fn test_noise_generator_derive_keyed_seed() {
        let generator = NoiseGenerator::new(42);
        let seed1 = generator.derive_keyed_seed(100);
        let seed2 = generator.derive_keyed_seed(100);

        assert_eq!(seed1, seed2);
    }

    #[test]
    fn test_apply_perturbation() {
        let img = RgbaImage::from_pixel(4, 4, image::Rgba([128, 128, 128, 255]));
        let perturbation: Vec<u8> = [200, 200, 200, 255].repeat(16);

        let result = apply_perturbation(&img, &perturbation, 4).unwrap();

        assert_eq!(result.width(), 4);
        assert_eq!(result.height(), 4);
    }

    #[test]
    fn test_parallel_threshold_sane() {
        let threshold = parallel_threshold();
        assert!(threshold > 0);
        assert!(threshold <= 16 * 64 * 64);
    }

    #[test]
    fn test_serial_parallel_identical_output() {
        let img = RgbaImage::from_fn(32, 32, |x, y| {
            image::Rgba([(x * 7) as u8, (y * 11) as u8, 128, 255])
        });

        let serial = apply_perturbation_single_pass_keyed_serial(&img, 42, 0.5, 1.0, &[]);
        let par = apply_perturbation_single_pass_keyed_par(&img, 42, 0.5, 1.0, &[]);

        assert_eq!(serial.to_rgba8(), par.to_rgba8());
    }
}
