use crate::error::{Error, Result};
use crate::jpeg_transcoder::{DctStegoF5, JpegHeader, JpegTranscoder};
use crate::protected::constants::{
    SPLITMIX64_SEED, STEGO_JPEG_AMPLITUDE, STEGO_JPEG_BLOCK_STRIDE, STEGO_JPEG_SPREAD,
    STEGO_OFFSET_SEED_1,
};
use crate::protected::metadata_trap::MetadataTrapProtector;
use crate::traits::Protector;
use crate::types::{ProtectionContext, ProtectionLevel};
use crate::util::image::XorShiftRng;
use hmac::{Hmac, Mac};
use image::{DynamicImage, Rgba, RgbaImage};
use sha2::Sha256;
use std::borrow::Cow;

type HmacSha256 = Hmac<Sha256>;

/// Minimum stego payload size: 24 bytes header + 2 bytes checksum/MAC.
const MIN_PAYLOAD_SIZE: usize = 26;
/// Bit length of the minimum payload.
const MIN_PAYLOAD_BITS: usize = MIN_PAYLOAD_SIZE * 8;

#[inline(always)]
fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(SPLITMIX64_SEED);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

pub struct SteganographyProtector;

impl SteganographyProtector {
    pub fn new() -> Self {
        Self
    }

    pub fn verify_payload(&self, img: &DynamicImage) -> bool {
        self.verify_payload_with_key(img, &[]).unwrap_or(false)
    }

    pub fn apply_dct_stego_bytes(
        &self,
        jpeg_bytes: &[u8],
        ctx: &ProtectionContext,
    ) -> Result<Vec<u8>> {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return Err(Error::Steganography("Not a valid JPEG".to_string()));
        }

        let seed = ctx.seed();

        // Try to decode DCT coefficients (works for baseline JPEG)
        match JpegTranscoder::decode_coefficients(jpeg_bytes) {
            Ok((mut header, mut coefficients)) => {
                // Baseline JPEG: full F5 DCT stego + seed in Q-tables
                let payload = self.generate_payload(ctx);
                let redundancy = ctx.stego_redundancy().max(1);

                let available_coeffs: usize = coefficients
                    .values()
                    .flat_map(|v| v.iter())
                    .map(|block| block.iter().skip(1).filter(|&&c| c != 0).count())
                    .sum();

                if available_coeffs >= payload.len() * 8 {
                    let attempts: &[usize] = if redundancy <= 1 {
                        &[1]
                    } else if redundancy <= 2 {
                        &[1, 2]
                    } else {
                        &[1, 2, redundancy]
                    };
                    for &r in attempts {
                        if DctStegoF5::with_redundancy(r)
                            .embed_f5(&mut coefficients, &payload, seed)
                            .is_ok()
                        {
                            break;
                        }
                    }
                }

                DctStegoF5::new()
                    .embed_seed_in_quantization_tables(&mut header, seed)
                    .map_err(|e| Error::Steganography(format!("Seed embed failed: {}", e)))?;

                JpegTranscoder::encode_coefficients(&header, &coefficients)
                    .map_err(|e| Error::ImageEncode(format!("DCT encode failed: {}", e)))
            }
            Err(_) => {
                // Progressive JPEG or other unsupported format: seed in Q-tables only.
                // Parse header, embed seed, reassemble without touching DCT coefficients.
                let mut header = crate::jpeg_transcoder::JpegHeader::parse(jpeg_bytes)
                    .map_err(|e| Error::Steganography(format!("Header parse failed: {}", e)))?;

                DctStegoF5::new()
                    .embed_seed_in_quantization_tables(&mut header, seed)
                    .map_err(|e| Error::Steganography(format!("Seed embed failed: {}", e)))?;

                // Reassemble: replace Q-tables in original byte stream
                Self::reassemble_jpeg_with_qtables(jpeg_bytes, &header)
            }
        }
    }

    /// Replace quantization tables in a JPEG byte stream with those from header.
    /// Preserves the rest of the byte stream verbatim (including progressive scans).
    fn reassemble_jpeg_with_qtables(
        jpeg_bytes: &[u8],
        header: &crate::jpeg_transcoder::JpegHeader,
    ) -> Result<Vec<u8>> {
        let mut output = Vec::with_capacity(jpeg_bytes.len() + 256);
        output.extend_from_slice(&jpeg_bytes[0..2]); // SOI

        let mut pos = 2;
        let mut wrote_tables = false;

        while pos + 4 <= jpeg_bytes.len() {
            if jpeg_bytes[pos] != 0xFF {
                pos += 1;
                continue;
            }

            let marker = jpeg_bytes[pos + 1];

            // End of headers — write remaining data as-is
            if marker == 0xDA || marker == 0xD9 {
                output.extend_from_slice(&jpeg_bytes[pos..]);
                break;
            }

            if marker == 0xDB {
                // DQT marker — skip original, write modified tables (once)
                let segment_len =
                    u16::from_be_bytes([jpeg_bytes[pos + 2], jpeg_bytes[pos + 3]]) as usize;
                pos += 2 + segment_len;

                if !wrote_tables {
                    for table in header.quantization_tables.iter().flatten() {
                        output.push(0xFF);
                        output.push(0xDB);
                        let table_data_len = if table.precision == 16 { 129 } else { 65 };
                        let total_len = table_data_len + 2;
                        output.extend_from_slice(&(total_len as u16).to_be_bytes());
                        let precision_bit = if table.precision == 16 { 1 } else { 0 };
                        output.push((precision_bit << 4) | table.table_id);
                        if table.precision == 8 {
                            for &val in &table.values {
                                output.push(val as u8);
                            }
                        } else {
                            for &val in &table.values {
                                output.extend_from_slice(&val.to_be_bytes());
                            }
                        }
                    }
                    wrote_tables = true;
                }
                continue;
            }

            // Copy other markers verbatim
            if marker == 0x00 {
                pos += 1;
                continue;
            }

            let segment_len =
                u16::from_be_bytes([jpeg_bytes[pos + 2], jpeg_bytes[pos + 3]]) as usize;
            output.extend_from_slice(&jpeg_bytes[pos..pos + 2 + segment_len]);
            pos += 2 + segment_len;
        }

        Ok(output)
    }

    fn extract_with_redundancy(
        &self,
        img: &RgbaImage,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        for pass in 0..5 {
            let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));

            if let Some(payload) = self.extract_lsb(img, 256, offset_seed) {
                if Self::verify_payload_integrity(&payload, mac_key) {
                    return Some(payload);
                }
            }
        }

        if let Some(jpeg_payload) = self.extract_jpeg_stego(img, 256, seed) {
            if Self::verify_payload_integrity(&jpeg_payload, mac_key) {
                return Some(jpeg_payload);
            }
        }

        None
    }

    pub fn verify_payload_with_key(&self, img: &DynamicImage, mac_key: &[u8]) -> Option<bool> {
        // Encode once, delegate to bytes-aware method to avoid double-encoding.
        if let Ok(png_bytes) = crate::util::image::encode_image(img, image::ImageFormat::Png) {
            self.verify_payload_from_bytes_with_key(&png_bytes, mac_key)
        } else {
            None
        }
    }

    /// Verify protection using raw image bytes — avoids unnecessary re-encoding.
    /// For JPEG bytes, checks DCT stego directly. For all formats, extracts
    /// metadata seed from the byte stream (preserves metadata that DynamicImage drops).
    pub fn verify_payload_from_bytes_with_key(
        &self,
        img_bytes: &[u8],
        mac_key: &[u8],
    ) -> Option<bool> {
        // JPEG: check DCT stego directly (no re-encode needed)
        if img_bytes.starts_with(&[0xFF, 0xD8]) {
            if let Some(true) = self.verify_dct_stego(img_bytes, mac_key) {
                return Some(true);
            }
        }

        // Extract metadata seed directly from bytes (works for PNG, JPEG, WebP)
        if let Some(metadata_seed) = MetadataTrapProtector::extract_seed_from_image(img_bytes) {
            if let Ok(img) = image::load_from_memory(img_bytes) {
                if self.verify_payload_with_seed(&img, metadata_seed) {
                    return Some(true);
                }
            }
        }

        // LSB fallback: try known seeds via DynamicImage
        if let Ok(img) = image::load_from_memory(img_bytes) {
            for seed in [42u64, 0, 1, 12345, 99999, 123456789] {
                if self.verify_payload_with_seed(&img, seed) {
                    return Some(true);
                }
            }
        }

        None
    }

    pub fn verify_payload_from_bytes(&self, img_bytes: &[u8], seed: u64) -> bool {
        if img_bytes.starts_with(&[0xFF, 0xD8]) {
            // Try to extract seed from Q-tables (works for both baseline and progressive)
            if let Ok(header) = JpegHeader::parse(img_bytes) {
                let stego_qtable = DctStegoF5::new();
                if let Some(extracted_seed) =
                    stego_qtable.extract_seed_from_quantization_tables(&header)
                {
                    if seed == extracted_seed {
                        return true;
                    }
                }
            }
        }

        if let Ok(img) = image::load_from_memory(img_bytes) {
            return self.verify_payload_with_seed(&img, seed);
        }

        false
    }

    pub fn verify_payload_with_seed(&self, img: &DynamicImage, seed: u64) -> bool {
        let rgba = img.to_rgba8();

        if let Some(payload) = self.extract_with_redundancy(&rgba, seed, &[]) {
            if Self::verify_checksum(&payload) {
                let embedded_seed = u64::from_le_bytes([
                    payload[2], payload[3], payload[4], payload[5], payload[6], payload[7],
                    payload[8], payload[9],
                ]);
                if embedded_seed == seed {
                    return true;
                }
            }
        }

        if let Ok(encoded) = crate::util::image::encode_image(img, image::ImageFormat::Png) {
            if let Some(metadata_seed) = MetadataTrapProtector::extract_seed_from_image(&encoded) {
                if metadata_seed != seed {
                    if let Some(payload) = self.extract_with_redundancy(&rgba, metadata_seed, &[]) {
                        if Self::verify_checksum(&payload) {
                            let embedded_seed = u64::from_le_bytes([
                                payload[2], payload[3], payload[4], payload[5], payload[6],
                                payload[7], payload[8], payload[9],
                            ]);
                            if embedded_seed == seed {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    pub fn extract_payload(&self, img: &DynamicImage) -> Option<StegoPayload> {
        self.extract_payload_with_key(img, &[])
    }

    pub fn extract_payload_with_key(
        &self,
        img: &DynamicImage,
        mac_key: &[u8],
    ) -> Option<StegoPayload> {
        // Try extracting seed from metadata first
        if let Ok(encoded) = crate::util::image::encode_image(img, image::ImageFormat::Png) {
            if let Some(metadata_seed) = MetadataTrapProtector::extract_seed_from_image(&encoded) {
                if let Some(payload) =
                    self.extract_payload_with_seed_and_key(img, metadata_seed, mac_key)
                {
                    return Some(payload);
                }
            }
        }

        // Fallback: try common seeds (metadata stripped during DynamicImage re-encoding)
        for seed in [42u64, 0, 1, 12345, 99999, 123456789] {
            if let Some(payload) = self.extract_payload_with_seed_and_key(img, seed, mac_key) {
                return Some(payload);
            }
        }

        None
    }

    fn parse_stego_payload(payload: &[u8]) -> Option<StegoPayload> {
        if payload.len() < MIN_PAYLOAD_SIZE {
            return None;
        }

        let version = payload[0];
        if version != 1 {
            return None;
        }

        let protection_level = payload[1];

        let extracted_seed = u64::from_le_bytes([
            payload[2], payload[3], payload[4], payload[5], payload[6], payload[7], payload[8],
            payload[9],
        ]);

        let intensity_raw = u16::from_le_bytes([payload[10], payload[11]]);
        let intensity = intensity_raw as f32 / 100.0;

        Some(StegoPayload {
            protection_level,
            seed: extracted_seed,
            intensity,
            version,
        })
    }

    fn extract_payload_with_seed_and_key(
        &self,
        img: &DynamicImage,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<StegoPayload> {
        let rgba = img.to_rgba8();
        let payload = self.extract_with_redundancy(&rgba, seed, mac_key)?;
        if !Self::verify_payload_integrity(&payload, mac_key) {
            return None;
        }
        Self::parse_stego_payload(&payload)
    }

    pub fn extract_payload_with_seed(&self, img: &DynamicImage, seed: u64) -> Option<StegoPayload> {
        let rgba = img.to_rgba8();
        let payload = self.extract_with_redundancy(&rgba, seed, &[])?;
        Self::parse_stego_payload(&payload)
    }

    fn verify_dct_stego(&self, jpeg_bytes: &[u8], mac_key: &[u8]) -> Option<bool> {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return None;
        }

        // Try baseline JPEG DCT extraction
        if let Ok((header, coefficients)) = JpegTranscoder::decode_coefficients(jpeg_bytes) {
            if let Some(extracted_seed) =
                DctStegoF5::new().extract_seed_from_quantization_tables(&header)
            {
                let bits_needed = 256;

                // Try the same redundancy values the embedder falls back to:
                // [1, min(2, r), r]. Since we don't know r, try all valid
                // redundancies. Do NOT break on bit-count match — only break
                // when integrity verification succeeds.
                for redundancy in 1..=5 {
                    let stego_f5 = DctStegoF5::with_redundancy(redundancy);
                    let total_bits = bits_needed * redundancy;
                    let extracted = stego_f5.extract_f5(&coefficients, total_bits, extracted_seed);

                    if extracted.len() >= MIN_PAYLOAD_BITS {
                        let bits_to_check = extracted.len().min(208);
                        let payload_bytes: Vec<u8> = extracted[..bits_to_check]
                            .chunks(8)
                            .map(|chunk| {
                                let mut byte = 0u8;
                                for (i, &bit) in chunk.iter().enumerate() {
                                    byte |= bit << i;
                                }
                                byte
                            })
                            .collect();

                        if Self::verify_payload_integrity(&payload_bytes, mac_key) {
                            return Some(true);
                        }
                    }
                }
            }
        }

        // Progressive JPEG or DCT decode failure: check seed in Q-tables only
        if let Ok(header) = crate::jpeg_transcoder::JpegHeader::parse(jpeg_bytes) {
            let stego_f5 = DctStegoF5::new();
            if stego_f5
                .extract_seed_from_quantization_tables(&header)
                .is_some()
            {
                return Some(true);
            }
        }

        None
    }

    fn compute_payload_mac(payload_without_mac: &[u8], mac_key: &[u8]) -> [u8; 8] {
        let mut mac = HmacSha256::new_from_slice(mac_key).expect("HMAC can take key of any size");
        mac.update(payload_without_mac);
        let result = mac.finalize().into_bytes();
        [
            result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7],
        ]
    }

    fn verify_payload_mac(payload_without_mac: &[u8], mac_key: &[u8], expected_mac: &[u8]) -> bool {
        let computed_mac = Self::compute_payload_mac(payload_without_mac, mac_key);
        computed_mac == expected_mac
    }

    /// # Security
    ///
    /// This is a simple additive checksum — not cryptographically secure.
    /// An attacker can trivially forge valid checksums. Without a MAC key
    /// (see `verify_payload_integrity`), this provides only minimal integrity
    /// checking suitable for accidental corruption detection, not adversarial
    /// settings.
    fn compute_checksum(data: &[u8]) -> [u8; 2] {
        let lo = data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        let hi = data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b ^ 0xAA));
        [lo, hi]
    }

    /// # Security
    ///
    /// Returns `true` if the 16-bit additive checksum is valid. This does NOT
    /// provide cryptographic assurance. Use `verify_payload_integrity` with a
    /// non-empty `mac_key` for HMAC-based verification.
    fn verify_checksum(payload: &[u8]) -> bool {
        if payload.len() < MIN_PAYLOAD_SIZE {
            return false;
        }
        let expected = Self::compute_checksum(&payload[..24]);
        payload[24] == expected[0] && payload[25] == expected[1]
    }

    fn verify_payload_integrity(payload: &[u8], mac_key: &[u8]) -> bool {
        if mac_key.is_empty() {
            Self::verify_checksum(payload)
        } else {
            payload.len() >= 32
                && Self::verify_payload_mac(&payload[..24], mac_key, &payload[24..32])
        }
    }

    /// Generates the steganography payload containing protection metadata.
    ///
    /// The payload contains:
    /// - Version byte (1)
    /// - Protection level byte
    /// - Seed (8 bytes, little-endian)
    /// - Intensity (2 bytes, scaled by 100)
    /// - Timestamp (8 bytes)
    ///
    /// This is followed by either:
    /// - HMAC-SHA256 of the first 24 bytes (8 bytes, if mac_key is set), OR
    /// - 16-bit additive checksum (2 bytes, if no mac_key)
    ///
    /// Note: The simple checksum is intentional for this library's use case.
    /// This library is designed for **legal deterrence**, not cryptographic security.
    /// The metadata markers (XMP, EXIF, IPTC) are meant to be visible/detectable.
    /// Tamper evidence comes from the visible metadata, not the stego layer.
    /// If an attacker removes the stego but leaves the metadata, there's still
    /// a clear record that the image was protected and warnings were present.
    fn generate_payload(&self, ctx: &ProtectionContext) -> Vec<u8> {
        let mut payload = Vec::with_capacity(64);

        payload.push(1);

        let level_byte = ctx.protection_level().map(|l| l.to_byte()).unwrap_or(2);

        payload.push(level_byte);

        payload.extend_from_slice(&ctx.seed().to_le_bytes());

        let intensity_val = (ctx.intensity() * 100.0) as u16;
        payload.extend_from_slice(&intensity_val.to_le_bytes());

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        payload.extend_from_slice(&now.to_le_bytes());

        while payload.len() < 24 {
            payload.push(0);
        }

        payload.truncate(24);

        if let Some(key) = ctx.mac_key() {
            let mac = Self::compute_payload_mac(&payload, key);
            payload.extend_from_slice(&mac);
        } else {
            // Intentional: 16-bit additive checksum for this visibility-focused design.
            // See docstring above for rationale.
            let checksum = Self::compute_checksum(&payload);
            payload.push(checksum[0]);
            payload.push(checksum[1]);
        }

        while payload.len() < 32 {
            payload.push(0);
        }

        payload.truncate(32);
        payload
    }

    /// Collision-free LCG permutation for stego pixel selection.
    /// Maps `index` to a unique pixel position in `[0, total)`.
    /// Uses a bijective LCG (a odd) which is a permutation when m is a power of 2.
    /// For non-power-of-2 totals, the slight bias is negligible for steganography.
    #[inline(always)]
    fn stego_permutation(index: usize, total_pixels: usize, seed: u64) -> usize {
        let a = splitmix64(seed).wrapping_mul(2) | 1;
        let b = splitmix64(seed.wrapping_add(0x9e3779b97f4a7c15));
        a.wrapping_mul(index as u64).wrapping_add(b) as usize % total_pixels
    }

    fn embed_lsb(
        &self,
        img: &RgbaImage,
        payload: &[u8],
        seed: u64,
        redundancy: usize,
    ) -> RgbaImage {
        let (width, height) = img.dimensions();
        let mut output = img.clone();

        let payload_bits = Self::bytes_to_bits(payload);

        let total_pixels = (width * height) as usize;
        let total_pixels_needed = payload_bits.len().div_ceil(3);

        if total_pixels_needed > total_pixels {
            return output;
        }

        for pass in 0..redundancy {
            let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));

            for (i, &bit) in payload_bits.iter().enumerate() {
                let idx = Self::stego_permutation(i, total_pixels, offset_seed);

                let x = idx as u32 % width;
                let y = idx as u32 / width;

                Self::embed_bit_in_pixel(&mut output, x, y, i % 3, bit);
            }
        }

        output
    }

    fn extract_lsb(&self, img: &RgbaImage, expected_bits: usize, seed: u64) -> Option<Vec<u8>> {
        let (width, height) = img.dimensions();
        let total_pixels = (width * height) as usize;

        if expected_bits > total_pixels * 3 {
            return None;
        }

        let mut bits = Vec::with_capacity(expected_bits);

        for i in 0..expected_bits {
            let idx = Self::stego_permutation(i, total_pixels, seed);

            let x = idx as u32 % width;
            let y = idx as u32 / width;
            let pixel = img.get_pixel(x, y);

            let bit_idx = i % 3;
            let bit = match bit_idx {
                0 => pixel[0] & 1,
                1 => pixel[1] & 1,
                _ => pixel[2] & 1,
            };
            bits.push(bit);
        }

        Some(Self::bits_to_bytes(&bits))
    }

    fn bytes_to_bits(bytes: &[u8]) -> Vec<u8> {
        let mut bits = Vec::with_capacity(bytes.len() * 8);
        for byte in bytes {
            for i in 0..8 {
                bits.push((byte >> i) & 1);
            }
        }
        bits
    }

    fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
        debug_assert!(
            bits.len().is_multiple_of(8),
            "bits_to_bytes: input length {} is not a multiple of 8, trailing bits will be dropped",
            bits.len()
        );
        let mut bytes = Vec::with_capacity(bits.len() / 8);
        for chunk in bits.chunks_exact(8) {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                byte |= bit << i;
            }
            bytes.push(byte);
        }
        bytes
    }

    fn embed_bit_in_pixel(output: &mut RgbaImage, x: u32, y: u32, channel: usize, bit: u8) {
        let pixel = output.get_pixel(x, y);

        let new_value = match channel {
            0 => (pixel[0] & 0xFE) | bit,
            1 => (pixel[1] & 0xFE) | bit,
            _ => (pixel[2] & 0xFE) | bit,
        };

        let new_pixel = Rgba([
            if channel == 0 { new_value } else { pixel[0] },
            if channel == 1 { new_value } else { pixel[1] },
            if channel == 2 { new_value } else { pixel[2] },
            pixel[3],
        ]);
        output.put_pixel(x, y, new_pixel);
    }

    fn get_pixel_channel_value(pixel: &Rgba<u8>, channel: usize) -> i16 {
        match channel {
            0 => pixel[0] as i16,
            1 => pixel[1] as i16,
            _ => pixel[2] as i16,
        }
    }

    fn embed_jpeg_bit_in_pixel(
        output: &mut RgbaImage,
        x: u32,
        y: u32,
        channel: usize,
        bit: u8,
        amplitude: i16,
    ) {
        let pixel = output.get_pixel(x, y);

        let original = Self::get_pixel_channel_value(pixel, channel);
        let adjustment = if bit == 1 { amplitude } else { -amplitude };
        let new_val = (original + adjustment).clamp(0, 255) as u8;

        let new_pixel = Rgba([
            if channel == 0 { new_val } else { pixel[0] },
            if channel == 1 { new_val } else { pixel[1] },
            if channel == 2 { new_val } else { pixel[2] },
            pixel[3],
        ]);
        output.put_pixel(x, y, new_pixel);
    }

    fn embed_jpeg_stego(
        &self,
        img: &RgbaImage,
        payload: &[u8],
        seed: u64,
        redundancy: usize,
    ) -> RgbaImage {
        let (width, height) = img.dimensions();

        let payload_bits = Self::bytes_to_bits(payload);
        let total_bits = payload_bits.len();

        let total_pixels = (width * height) as usize;
        if total_bits * 4 > total_pixels {
            return img.clone();
        }

        let mut output = img.clone();

        let amplitude = STEGO_JPEG_AMPLITUDE;
        let spread: usize = STEGO_JPEG_SPREAD;
        let block_stride = STEGO_JPEG_BLOCK_STRIDE;

        let bits_per_pass = total_bits;

        for pass in 0..redundancy {
            let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));
            let mut rng = XorShiftRng::new(offset_seed);

            let mut embedded = 0;

            let y_start_offset = rng.gen_range_usize(0..spread);
            let x_start_offset = rng.gen_range_usize(0..spread);

            for y_base in (y_start_offset..height as usize).step_by(block_stride) {
                for y_offset in 0..spread {
                    let y = (y_base + y_offset) as u32;
                    if y >= height {
                        break;
                    }

                    for x_base in (x_start_offset..width as usize).step_by(block_stride) {
                        for x_offset in 0..spread {
                            let x = (x_base + x_offset) as u32;
                            if x >= width {
                                break;
                            }

                            let bit_idx = embedded % bits_per_pass;
                            let bit = payload_bits[bit_idx];
                            let channel = embedded % 3;

                            Self::embed_jpeg_bit_in_pixel(
                                &mut output,
                                x,
                                y,
                                channel,
                                bit,
                                amplitude,
                            );

                            embedded += 1;

                            if embedded >= bits_per_pass {
                                return output;
                            }
                        }
                    }
                }
            }
        }

        output
    }

    const EXTRACT_REDUNDANCY: usize = 5;

    fn extract_jpeg_stego(
        &self,
        img: &RgbaImage,
        expected_bits: usize,
        seed: u64,
    ) -> Option<Vec<u8>> {
        let (width, height) = img.dimensions();

        let amplitude = STEGO_JPEG_AMPLITUDE as f32;
        let spread: usize = STEGO_JPEG_SPREAD;
        let block_stride = STEGO_JPEG_BLOCK_STRIDE;

        let mut all_candidates: Vec<Vec<u8>> = Vec::new();

        for redundancy in 0..Self::EXTRACT_REDUNDANCY {
            let offset_seed =
                seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(redundancy as u64));

            let mut bit_votes: Vec<Vec<i32>> = vec![Vec::new(); expected_bits];

            let mut rng = XorShiftRng::new(offset_seed);

            let y_start_offset = rng.gen_range_usize(0..spread);
            let x_start_offset = rng.gen_range_usize(0..spread);

            let mut embedded_count = 0;

            for y_base in (y_start_offset..height as usize).step_by(block_stride) {
                for y_offset in 0..spread {
                    let y = (y_base + y_offset) as u32;
                    if y >= height {
                        break;
                    }

                    for x_base in (x_start_offset..width as usize).step_by(block_stride) {
                        for x_offset in 0..spread {
                            let x = (x_base + x_offset) as u32;
                            if x >= width {
                                break;
                            }

                            if embedded_count >= expected_bits {
                                break;
                            }

                            let pixel = img.get_pixel(x, y);
                            let channel = embedded_count % 3;

                            let value = Self::get_pixel_channel_value(pixel, channel) as f32;
                            let deviation = value - 128.0;

                            bit_votes[embedded_count].push(deviation as i32);
                            embedded_count += 1;
                        }
                    }
                }
            }

            if bit_votes.iter().any(|v| v.is_empty()) {
                continue;
            }

            let mut bits = Vec::with_capacity(expected_bits);
            for votes in &bit_votes {
                if votes.len() < 3 {
                    bits.push(0);
                    continue;
                }

                let sum: i32 = votes.iter().sum();
                let avg = sum as f32 / votes.len() as f32;

                let bit = if avg > amplitude / 4.0 { 1 } else { 0 };
                bits.push(bit);
            }

            if bits.len() >= MIN_PAYLOAD_BITS {
                let bytes = Self::bits_to_bytes(&bits);
                if bytes.len() >= MIN_PAYLOAD_SIZE {
                    all_candidates.push(bytes);
                }
            }
        }

        for candidate in &all_candidates {
            if Self::verify_checksum(candidate) {
                return Some(candidate.clone());
            }
        }

        for candidate in &all_candidates {
            if candidate.len() >= MIN_PAYLOAD_SIZE {
                return Some(candidate.clone());
            }
        }

        None
    }

    fn apply_to_image_owned(&self, img: &DynamicImage, ctx: &ProtectionContext) -> DynamicImage {
        let payload = self.generate_payload(ctx);
        let rgba = img.to_rgba8();

        let format = ctx
            .input_format()
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        let redundancy = ctx.stego_redundancy();

        let processed = match format {
            crate::types::ImageOutputFormat::Png => {
                self.embed_lsb(&rgba, &payload, ctx.seed(), redundancy)
            }
            crate::types::ImageOutputFormat::Jpeg => {
                self.embed_jpeg_stego(&rgba, &payload, ctx.seed(), redundancy)
            }
            crate::types::ImageOutputFormat::WebP => {
                self.embed_lsb(&rgba, &payload, ctx.seed(), redundancy)
            }
        };

        DynamicImage::ImageRgba8(processed)
    }
}

impl Default for SteganographyProtector {
    fn default() -> Self {
        Self::new()
    }
}

impl Protector for SteganographyProtector {
    fn apply<'a>(
        &self,
        img: &'a DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>> {
        Ok(Cow::Owned(self.apply_to_image_owned(img, ctx)))
    }

    fn apply_bytes(&self, img_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>> {
        let format = ctx.input_format().unwrap_or_else(|| {
            crate::types::ImageOutputFormat::from_magic_bytes(img_bytes)
                .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT)
        });

        if format == crate::types::ImageOutputFormat::Jpeg {
            return self.apply_dct_stego_bytes(img_bytes, ctx);
        }

        let img = image::load_from_memory(img_bytes)?;

        let processed = self.apply_to_image_owned(&img, ctx);

        let image_format = match format {
            crate::types::ImageOutputFormat::Png => image::ImageFormat::Png,
            crate::types::ImageOutputFormat::Jpeg => image::ImageFormat::Jpeg,
            crate::types::ImageOutputFormat::WebP => image::ImageFormat::WebP,
        };

        crate::util::image::encode_image(&processed, image_format)
    }

    fn name(&self) -> &'static str {
        "steganography"
    }

    fn protection_level(&self) -> ProtectionLevel {
        ProtectionLevel::Standard
    }

    fn estimated_latency_ms(&self) -> u32 {
        2
    }
}

#[derive(Debug, Clone)]
pub struct StegoPayload {
    protection_level: u8,
    seed: u64,
    intensity: f32,
    version: u8,
}

impl StegoPayload {
    pub fn protection_level(&self) -> u8 {
        self.protection_level
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    pub fn version(&self) -> u8 {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProtectionConfig;
    use image::{ImageBuffer, RgbaImage};
    use std::sync::Arc;

    fn make_test_image(w: u32, h: u32) -> RgbaImage {
        ImageBuffer::from_fn(w, h, |x, y| {
            Rgba([(x * 3) as u8, (y * 5) as u8, ((x + y) * 7) as u8, 255])
        })
    }

    fn make_large_test_image() -> RgbaImage {
        make_test_image(128, 128)
    }

    fn ctx_no_mac(seed: u64) -> ProtectionContext {
        ProtectionContext::new(0.5, seed)
    }

    fn ctx_with_mac(seed: u64, key: &[u8]) -> ProtectionContext {
        let config = Arc::new(ProtectionConfig::new().with_mac_key(key.to_vec()));
        ProtectionContext::new(0.5, seed).with_config(config)
    }

    // ── Bit conversion ────────────────────────────────────────────────

    #[test]
    fn bytes_to_bits_length() {
        let data = [0xAA, 0x55, 0xFF, 0x00];
        let bits = SteganographyProtector::bytes_to_bits(&data);
        assert_eq!(bits.len(), 32);
    }

    #[test]
    fn bits_to_bytes_roundtrip() {
        let original: Vec<u8> = vec![0x00, 0xFF, 0xA5, 0x5A, 0x01, 0x80, 0xFE, 0x7F];
        let bits = SteganographyProtector::bytes_to_bits(&original);
        let recovered = SteganographyProtector::bits_to_bytes(&bits);
        assert_eq!(original, recovered);
    }

    #[test]
    fn bytes_to_bits_lsb_order() {
        let data = [0b0000_0001];
        let bits = SteganographyProtector::bytes_to_bits(&data);
        assert_eq!(bits[0], 1);
        assert_eq!(bits[1], 0);
        assert_eq!(bits[7], 0);
    }

    #[test]
    fn bytes_to_bits_high_bit() {
        let data = [0b1000_0000];
        let bits = SteganographyProtector::bytes_to_bits(&data);
        assert_eq!(bits[7], 1);
        assert_eq!(bits[0], 0);
    }

    #[test]
    fn bits_to_bytes_trailing_dropped() {
        // Multiple of 8 — works correctly
        let bits = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let bytes = SteganographyProtector::bits_to_bytes(&bits);
        assert_eq!(bytes.len(), 1);
        assert_eq!(bytes[0], 1);
    }

    // ── Checksum ──────────────────────────────────────────────────────

    #[test]
    fn compute_checksum_deterministic() {
        let data = vec![1u8; 24];
        let a = SteganographyProtector::compute_checksum(&data);
        let b = SteganographyProtector::compute_checksum(&data);
        assert_eq!(a, b);
    }

    #[test]
    fn compute_checksum_different_data_different_result() {
        let a = SteganographyProtector::compute_checksum(&[0u8; 24]);
        let b = SteganographyProtector::compute_checksum(&[1u8; 24]);
        assert_ne!(a, b);
    }

    #[test]
    fn verify_checksum_valid() {
        let mut payload = vec![0u8; 24];
        payload[0] = 1; // version
        payload[1] = 2; // level
        let checksum = SteganographyProtector::compute_checksum(&payload);
        payload.push(checksum[0]);
        payload.push(checksum[1]);
        assert!(SteganographyProtector::verify_checksum(&payload));
    }

    #[test]
    fn verify_checksum_invalid() {
        let mut payload = vec![0u8; 26];
        payload[24] = 0xFF;
        payload[25] = 0xFF;
        assert!(!SteganographyProtector::verify_checksum(&payload));
    }

    #[test]
    fn verify_checksum_too_short() {
        assert!(!SteganographyProtector::verify_checksum(&[0u8; 10]));
    }

    #[test]
    fn verify_checksum_corrupted_byte() {
        let mut payload = vec![1u8; 24];
        let checksum = SteganographyProtector::compute_checksum(&payload);
        payload.push(checksum[0]);
        payload.push(checksum[1]);
        assert!(SteganographyProtector::verify_checksum(&payload));

        // Corrupt one byte
        payload[5] = payload[5].wrapping_add(1);
        // Recompute checksum for corrupted data
        let new_checksum = SteganographyProtector::compute_checksum(&payload[..24]);
        // Old checksum should not match new data
        assert_ne!([payload[24], payload[25]], new_checksum);
    }

    // ── HMAC ──────────────────────────────────────────────────────────

    #[test]
    fn compute_payload_mac_deterministic() {
        let data = vec![1u8; 24];
        let key = b"test-secret-key";
        let a = SteganographyProtector::compute_payload_mac(&data, key);
        let b = SteganographyProtector::compute_payload_mac(&data, key);
        assert_eq!(a, b);
    }

    #[test]
    fn compute_payload_mac_different_keys() {
        let data = vec![1u8; 24];
        let a = SteganographyProtector::compute_payload_mac(&data, b"key-a");
        let b = SteganographyProtector::compute_payload_mac(&data, b"key-b");
        assert_ne!(a, b);
    }

    #[test]
    fn compute_payload_mac_different_data() {
        let key = b"test-key";
        let a = SteganographyProtector::compute_payload_mac(&[0u8; 24], key);
        let b = SteganographyProtector::compute_payload_mac(&[1u8; 24], key);
        assert_ne!(a, b);
    }

    #[test]
    fn verify_payload_mac_match() {
        let data = vec![42u8; 24];
        let key = b"my-key";
        let mac = SteganographyProtector::compute_payload_mac(&data, key);
        assert!(SteganographyProtector::verify_payload_mac(&data, key, &mac));
    }

    #[test]
    fn verify_payload_mac_wrong_key() {
        let data = vec![42u8; 24];
        let mac = SteganographyProtector::compute_payload_mac(&data, b"correct-key");
        assert!(!SteganographyProtector::verify_payload_mac(
            &data,
            b"wrong-key",
            &mac
        ));
    }

    #[test]
    fn verify_payload_mac_corrupted_mac() {
        let data = vec![42u8; 24];
        let key = b"key";
        let mut mac = SteganographyProtector::compute_payload_mac(&data, key);
        mac[0] ^= 0xFF;
        assert!(!SteganographyProtector::verify_payload_mac(
            &data, key, &mac
        ));
    }

    // ── Payload integrity ─────────────────────────────────────────────

    #[test]
    fn verify_payload_integrity_checksum_mode() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload(&ctx);
        // Payload is 32 bytes (24 header + 2 checksum + 6 padding)
        assert_eq!(payload.len(), 32);
        assert!(SteganographyProtector::verify_payload_integrity(
            &payload,
            &[]
        ));
    }

    #[test]
    fn verify_payload_integrity_mac_mode() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_with_mac(42, b"secret");
        let payload = protector.generate_payload(&ctx);
        assert_eq!(payload.len(), 32);
        assert!(SteganographyProtector::verify_payload_integrity(
            &payload, b"secret"
        ));
    }

    #[test]
    fn verify_payload_integrity_mac_wrong_key() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_with_mac(42, b"correct");
        let payload = protector.generate_payload(&ctx);
        assert!(!SteganographyProtector::verify_payload_integrity(
            &payload, b"wrong"
        ));
    }

    #[test]
    fn verify_payload_integrity_checksum_corrupted() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let mut payload = protector.generate_payload(&ctx);
        payload[5] ^= 0xFF;
        assert!(!SteganographyProtector::verify_payload_integrity(
            &payload,
            &[]
        ));
    }

    #[test]
    fn verify_payload_integrity_truncated() {
        assert!(!SteganographyProtector::verify_payload_integrity(
            &[0u8; 10],
            &[]
        ));
    }

    #[test]
    fn verify_payload_integrity_mac_truncated() {
        assert!(!SteganographyProtector::verify_payload_integrity(
            &[0u8; 30], b"key"
        ));
    }

    // ── Payload generation ────────────────────────────────────────────

    #[test]
    fn generate_payload_checksum_mode_length() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(12345);
        let payload = protector.generate_payload(&ctx);
        // Always 32 bytes (24 header + 2 checksum + 6 padding)
        assert_eq!(payload.len(), 32);
    }

    #[test]
    fn generate_payload_mac_mode_length() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_with_mac(12345, b"key");
        let payload = protector.generate_payload(&ctx);
        assert_eq!(payload.len(), 32);
    }

    #[test]
    fn generate_payload_version_byte() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload(&ctx);
        assert_eq!(payload[0], 1);
    }

    #[test]
    fn generate_payload_seed_roundtrip() {
        let seed = 0xDEAD_BEEF_CAFE_BABE;
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(seed);
        let payload = protector.generate_payload(&ctx);

        let extracted_seed = u64::from_le_bytes([
            payload[2], payload[3], payload[4], payload[5], payload[6], payload[7], payload[8],
            payload[9],
        ]);
        assert_eq!(extracted_seed, seed);
    }

    #[test]
    fn generate_payload_intensity_precision() {
        let protector = SteganographyProtector::new();
        let ctx = ProtectionContext::new(0.73, 42);
        let payload = protector.generate_payload(&ctx);

        let intensity_raw = u16::from_le_bytes([payload[10], payload[11]]);
        let recovered = intensity_raw as f32 / 100.0;
        assert!((recovered - 0.73).abs() < 0.02);
    }

    #[test]
    fn generate_payload_protection_level_byte() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload(&ctx);
        // Default protection_level is None, falls back to 2 (Standard)
        assert_eq!(payload[1], 2);
    }

    #[test]
    fn generate_payload_different_seeds_differ() {
        let protector = SteganographyProtector::new();
        let a = protector.generate_payload(&ctx_no_mac(1));
        let b = protector.generate_payload(&ctx_no_mac(2));
        assert_ne!(a[..10], b[..10]); // seed bytes differ
    }

    // ── Permutation ───────────────────────────────────────────────────

    #[test]
    fn stego_permutation_deterministic() {
        let a = SteganographyProtector::stego_permutation(0, 1024, 42);
        let b = SteganographyProtector::stego_permutation(0, 1024, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn stego_permutation_different_seeds_differ() {
        let a = SteganographyProtector::stego_permutation(0, 1024, 42);
        let b = SteganographyProtector::stego_permutation(0, 1024, 99);
        assert_ne!(a, b);
    }

    #[test]
    fn stego_permutation_power_of_2_injective() {
        let total = 1024usize;
        let seed = 42u64;
        let mut seen = vec![false; total];
        for i in 0..total {
            let pos = SteganographyProtector::stego_permutation(i, total, seed);
            assert!(
                pos < total,
                "permutation out of range: {} >= {}",
                pos,
                total
            );
            assert!(!seen[pos], "collision at index {} -> pos {}", i, pos);
            seen[pos] = true;
        }
    }

    #[test]
    fn stego_permutation_index0_consistent() {
        let a = SteganographyProtector::stego_permutation(0, 4096, 100);
        let b = SteganographyProtector::stego_permutation(0, 4096, 100);
        assert_eq!(a, b);
    }

    // ── Pixel manipulation ────────────────────────────────────────────

    #[test]
    fn embed_bit_in_pixel_modifies_correct_channel() {
        let mut img = make_test_image(4, 4);
        let orig_g = img.get_pixel(0, 0)[1];
        let orig_b = img.get_pixel(0, 0)[2];
        let orig_a = img.get_pixel(0, 0)[3];

        // Embed bit 1 in channel 0
        SteganographyProtector::embed_bit_in_pixel(&mut img, 0, 0, 0, 1);
        let modified = img.get_pixel(0, 0);
        assert_eq!(modified[0] & 1, 1);
        assert_eq!(modified[1], orig_g);
        assert_eq!(modified[2], orig_b);
        assert_eq!(modified[3], orig_a);
    }

    #[test]
    fn embed_bit_in_pixel_clears_lsb() {
        let mut img = ImageBuffer::from_pixel(1, 1, Rgba([0xFF, 0xFF, 0xFF, 255]));

        SteganographyProtector::embed_bit_in_pixel(&mut img, 0, 0, 1, 0);
        let pixel = img.get_pixel(0, 0);
        assert_eq!(pixel[1] & 1, 0);
        // Channel 0 and 2 unchanged
        assert_eq!(pixel[0], 0xFF);
        assert_eq!(pixel[2], 0xFF);
    }

    #[test]
    fn get_pixel_channel_value_reads_correctly() {
        let pixel = Rgba([100, 150, 200, 255]);
        assert_eq!(
            SteganographyProtector::get_pixel_channel_value(&pixel, 0),
            100
        );
        assert_eq!(
            SteganographyProtector::get_pixel_channel_value(&pixel, 1),
            150
        );
        assert_eq!(
            SteganographyProtector::get_pixel_channel_value(&pixel, 2),
            200
        );
        assert_eq!(
            SteganographyProtector::get_pixel_channel_value(&pixel, 3),
            200
        );
    }

    #[test]
    fn embed_jpeg_bit_amplitude() {
        let mut img = ImageBuffer::from_pixel(1, 1, Rgba([128, 128, 128, 255]));
        let amplitude: i16 = 40;

        // Bit 1: +amplitude
        SteganographyProtector::embed_jpeg_bit_in_pixel(&mut img, 0, 0, 0, 1, amplitude);
        assert_eq!(img.get_pixel(0, 0)[0], 168);

        // Bit 0: -amplitude
        let mut img2 = ImageBuffer::from_pixel(1, 1, Rgba([128, 128, 128, 255]));
        SteganographyProtector::embed_jpeg_bit_in_pixel(&mut img2, 0, 0, 0, 0, amplitude);
        assert_eq!(img2.get_pixel(0, 0)[0], 88);
    }

    #[test]
    fn embed_jpeg_bit_clamps_to_range() {
        let mut img = ImageBuffer::from_pixel(1, 1, Rgba([10, 10, 10, 255]));
        let amplitude: i16 = 40;

        // -40 from 10 = -30, clamped to 0
        SteganographyProtector::embed_jpeg_bit_in_pixel(&mut img, 0, 0, 0, 0, amplitude);
        assert_eq!(img.get_pixel(0, 0)[0], 0);

        // +40 to 250 = 290, clamped to 255
        let mut img2 = ImageBuffer::from_pixel(1, 1, Rgba([250, 250, 250, 255]));
        SteganographyProtector::embed_jpeg_bit_in_pixel(&mut img2, 0, 0, 0, 1, amplitude);
        assert_eq!(img2.get_pixel(0, 0)[0], 255);
    }

    // ── Embed/extract via public API ───────────────────────────────────
    // Internal embed_lsb/extract_lsb use different seed derivation,
    // so we test through the public apply() + extract_payload() API
    // which correctly matches seeds.

    #[test]
    fn lsb_embed_extract_png() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&result));
        let payload = protector.extract_payload(&result).unwrap();
        assert_eq!(payload.seed(), 42);
    }

    #[test]
    fn lsb_embed_extract_different_seeds() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();

        let ctx_a = ctx_no_mac(42);
        let ctx_b = ctx_no_mac(99);

        let dyn_img = DynamicImage::ImageRgba8(img.clone());
        let result_a = protector.apply(&dyn_img, &ctx_a).unwrap();
        let result_b = protector.apply(&dyn_img, &ctx_b).unwrap();

        // Use extract_payload_with_seed with known seeds
        let payload_a = protector.extract_payload_with_seed(&result_a, 42).unwrap();
        let payload_b = protector.extract_payload_with_seed(&result_b, 99).unwrap();
        assert_eq!(payload_a.seed(), 42);
        assert_eq!(payload_b.seed(), 99);
    }

    #[test]
    fn lsb_embed_extract_high_redundancy() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ProtectionContext::new(0.5, 42).with_stego_redundancy(5);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&result));
    }

    #[test]
    fn lsb_embed_modifies_pixels() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img.clone());
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert_ne!(*result, DynamicImage::ImageRgba8(img));
    }

    #[test]
    fn lsb_preserves_dimensions() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let (w, h) = img.dimensions();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert_eq!(result.width(), w);
        assert_eq!(result.height(), h);
    }

    #[test]
    fn lsb_verify_with_mac_key() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let key = b"test-mac-key";
        let ctx = ctx_with_mac(42, key);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        // Use extract_payload_with_seed_and_key with known seed
        let payload = protector.extract_payload_with_seed_and_key(&result, 42, key);
        assert!(payload.is_some());
        assert_eq!(payload.unwrap().seed(), 42);

        // Wrong key returns None
        assert!(protector
            .extract_payload_with_seed_and_key(&result, 42, b"wrong-key")
            .is_none());
    }

    #[test]
    fn lsb_extract_wrong_key_returns_none() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_with_mac(42, b"correct");

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        assert!(protector
            .extract_payload_with_key(&result, b"correct")
            .is_some());
        assert!(protector
            .extract_payload_with_key(&result, b"wrong")
            .is_none());
    }

    #[test]
    fn lsb_payload_too_large_returns_unchanged() {
        let protector = SteganographyProtector::new();
        let tiny = make_test_image(2, 2); // 4 pixels, 12 channels — too small for 256 bits
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload(&ctx);

        let result = protector.embed_lsb(&tiny, &payload, 42, 1);
        assert_eq!(result, tiny);
    }

    #[test]
    fn lsb_extract_oversized_expected_bits_returns_none() {
        let protector = SteganographyProtector::new();
        let img = make_test_image(4, 4); // 16 pixels = 48 channels
        assert!(protector.extract_lsb(&img, 256, 42).is_none());
    }

    // ── JPEG stego via public API ─────────────────────────────────────

    #[test]
    fn jpeg_stego_embed_modifies_pixels() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload(&ctx);

        let embedded = protector.embed_jpeg_stego(&img, &payload, 42, 1);

        let mut diffs = 0u32;
        for (a, b) in img.pixels().zip(embedded.pixels()) {
            if a != b {
                diffs += 1;
            }
        }
        assert!(diffs > 0, "JPEG stego should modify some pixels");
    }

    #[test]
    fn jpeg_stego_different_seeds_differ() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload(&ctx);

        let a = protector.embed_jpeg_stego(&img, &payload, 42, 1);
        let b = protector.embed_jpeg_stego(&img, &payload, 99, 1);
        assert_ne!(a, b);
    }

    #[test]
    fn jpeg_stego_payload_too_large_returns_clone() {
        let protector = SteganographyProtector::new();
        let tiny = make_test_image(2, 2);
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload(&ctx);

        let result = protector.embed_jpeg_stego(&tiny, &payload, 42, 1);
        assert_eq!(result, tiny);
    }

    #[test]
    fn jpeg_stego_via_apply_png_fallback() {
        // When format is PNG, apply uses LSB not JPEG stego.
        // When format is explicitly JPEG, apply_bytes uses DCT stego.
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&result));
    }

    // ── StegoPayload parsing ──────────────────────────────────────────

    #[test]
    fn parse_stego_payload_valid() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(12345);
        let payload = protector.generate_payload(&ctx);

        let parsed = SteganographyProtector::parse_stego_payload(&payload).unwrap();
        assert_eq!(parsed.version(), 1);
        assert_eq!(parsed.seed(), 12345);
        assert_eq!(parsed.protection_level(), 2);
        assert!((parsed.intensity() - 0.5).abs() < 0.02);
    }

    #[test]
    fn parse_stego_payload_too_short() {
        assert!(SteganographyProtector::parse_stego_payload(&[0u8; 10]).is_none());
    }

    #[test]
    fn parse_stego_payload_wrong_version() {
        let mut payload = vec![0u8; 26];
        payload[0] = 99; // invalid version
        assert!(SteganographyProtector::parse_stego_payload(&payload).is_none());
    }

    // ── Extract with redundancy via public API ────────────────────────

    #[test]
    fn extract_with_redundancy_finds_payload() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&result));
    }

    #[test]
    fn extract_with_redundancy_mac_mode() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let key = b"hmac-key";
        let ctx = ctx_with_mac(42, key);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        let payload = protector.extract_payload_with_seed_and_key(&result, 42, key);
        assert!(payload.is_some());
        assert_eq!(payload.unwrap().seed(), 42);
    }

    #[test]
    fn extract_with_redundancy_mac_wrong_key_returns_none() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_with_mac(42, b"correct-key");

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        assert!(protector
            .extract_payload_with_key(&result, b"wrong-key")
            .is_none());
    }

    // ── Protector trait ───────────────────────────────────────────────

    #[test]
    fn protector_apply_changes_image() {
        let p = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);
        let dyn_img = DynamicImage::ImageRgba8(img.clone());
        let result = p.apply(&dyn_img, &ctx).unwrap();
        match result {
            std::borrow::Cow::Owned(owned) => {
                assert_ne!(owned.to_rgba8(), img);
            }
            _ => panic!("expected owned result"),
        }
    }

    #[test]
    fn protector_apply_preserves_dimensions() {
        let p = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);
        let (w, h) = img.dimensions();
        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = p.apply(&dyn_img, &ctx).unwrap();
        assert_eq!(result.width(), w);
        assert_eq!(result.height(), h);
    }

    #[test]
    fn protector_level() {
        let p = SteganographyProtector::new();
        assert_eq!(p.protection_level(), ProtectionLevel::Standard);
    }

    #[test]
    fn protector_modifies_pixels() {
        let p = SteganographyProtector::new();
        assert!(p.modifies_pixels());
    }

    #[test]
    fn protector_is_enabled() {
        let p = SteganographyProtector::new();
        assert!(p.is_enabled());
    }

    #[test]
    fn protector_apply_bytes_png_roundtrip() {
        let p = SteganographyProtector::new();
        let img = make_large_test_image();
        let png_bytes = crate::util::image::encode_image(
            &DynamicImage::ImageRgba8(img),
            image::ImageFormat::Png,
        )
        .unwrap();

        let ctx = ctx_no_mac(42);
        let processed = p.apply_bytes(&png_bytes, &ctx).unwrap();

        // Re-decode and verify
        let decoded = image::load_from_memory(&processed).unwrap();
        assert!(p.verify_payload(&decoded));
    }

    // ── DCT stego edge cases ──────────────────────────────────────────

    #[test]
    fn dct_stego_rejects_non_jpeg() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let result = protector.apply_dct_stego_bytes(&[0x89, 0x50, 0x4E, 0x47], &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn dct_stego_rejects_empty() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let result = protector.apply_dct_stego_bytes(&[], &ctx);
        assert!(result.is_err());
    }

    // ── Extract seed from image ───────────────────────────────────────

    #[test]
    fn extract_seed_from_protected_image() {
        let meta = MetadataTrapProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        // Encode to PNG bytes, then apply metadata trap via apply_bytes
        let png_bytes = crate::util::image::encode_image(
            &DynamicImage::ImageRgba8(img),
            image::ImageFormat::Png,
        )
        .unwrap();
        let with_metadata = meta.apply_bytes(&png_bytes, &ctx).unwrap();

        let extracted = MetadataTrapProtector::extract_seed_from_image(&with_metadata);
        assert_eq!(extracted, Some(42));
    }

    // ── Full round-trip with MAC key ──────────────────────────────────

    #[test]
    fn full_roundtrip_mac_embed_extract_verify() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let key = b"super-secret";
        let ctx = ctx_with_mac(42, key);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        // Extract payload
        let payload = protector.extract_payload_with_key(&result, key);
        assert!(payload.is_some());
        let p = payload.unwrap();
        assert_eq!(p.seed(), 42);
        assert_eq!(p.version(), 1);
    }

    #[test]
    fn full_roundtrip_no_mac_embed_extract_verify() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        let payload = protector.extract_payload(&result);
        assert!(payload.is_some());
        let p = payload.unwrap();
        assert_eq!(p.seed(), 42);
    }

    #[test]
    fn full_roundtrip_verify_payload() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&result));
    }
}
