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

        let seed = ctx.seed;

        // Try to decode DCT coefficients (works for baseline JPEG)
        match JpegTranscoder::decode_coefficients(jpeg_bytes) {
            Ok((mut header, mut coefficients)) => {
                // Baseline JPEG: full F5 DCT stego + seed in Q-tables
                let payload = self.generate_payload(ctx);
                let redundancy = ctx.stego_redundancy.max(1);

                let available_coeffs: usize = coefficients
                    .values()
                    .flat_map(|v| v.iter())
                    .map(|block| block.iter().skip(1).filter(|&&c| c != 0).count())
                    .sum();

                if available_coeffs >= payload.len() * 8 {
                    for r in [1, redundancy.min(2), redundancy].iter().copied() {
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

    fn extract_with_redundancy(&self, img: &RgbaImage, seed: u64) -> Option<Vec<u8>> {
        for pass in 0..5 {
            let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));

            if let Some(payload) = self.extract_lsb(img, 256, offset_seed) {
                if Self::verify_checksum(&payload) {
                    return Some(payload);
                }
            }
        }

        if let Some(jpeg_payload) = self.extract_jpeg_stego(img, 256, seed) {
            if Self::verify_checksum(&jpeg_payload) {
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

        if let Some(payload) = self.extract_with_redundancy(&rgba, seed) {
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
                    if let Some(payload) = self.extract_with_redundancy(&rgba, metadata_seed) {
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

    fn extract_payload_with_seed_and_key(
        &self,
        img: &DynamicImage,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<StegoPayload> {
        let rgba = img.to_rgba8();

        let payload = self.extract_with_redundancy(&rgba, seed)?;

        if !Self::verify_payload_integrity(&payload, mac_key) {
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

    pub fn extract_payload_with_seed(&self, img: &DynamicImage, seed: u64) -> Option<StegoPayload> {
        let rgba = img.to_rgba8();

        if let Some(payload) = self.extract_with_redundancy(&rgba, seed) {
            if payload.len() < 25 {
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

            return Some(StegoPayload {
                protection_level,
                seed: extracted_seed,
                intensity,
                version,
            });
        }

        None
    }

    fn verify_dct_stego(&self, jpeg_bytes: &[u8], mac_key: &[u8]) -> Option<bool> {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return None;
        }

        // Try baseline JPEG DCT extraction
        if let Ok((header, coefficients)) = JpegTranscoder::decode_coefficients(jpeg_bytes) {
            let stego_f5 = DctStegoF5::new();

            if let Some(extracted_seed) = stego_f5.extract_seed_from_quantization_tables(&header) {
                let bits_needed = 256;
                let extracted = stego_f5.extract_f5(&coefficients, bits_needed, extracted_seed);

                if extracted.len() >= 25 * 8 {
                    let bits_to_check = extracted.len().min(200);
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
    fn compute_checksum(data: &[u8]) -> u8 {
        data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
    }

    /// # Security
    ///
    /// Returns `true` if the simple additive checksum is valid. This does NOT
    /// provide cryptographic assurance. Use `verify_payload_integrity` with a
    /// non-empty `mac_key` for HMAC-based verification.
    fn verify_checksum(payload: &[u8]) -> bool {
        payload.len() >= 25 && payload[24] == Self::compute_checksum(&payload[..24])
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
    /// - HMAC-SHA256 of the first 24 bytes (if mac_key is set), OR
    /// - Simple additive checksum (if no mac_key)
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

        let level_byte = ctx.protection_level.map(|l| l.to_byte()).unwrap_or(2);

        payload.push(level_byte);

        payload.extend_from_slice(&ctx.seed.to_le_bytes());

        let intensity_val = (ctx.intensity * 100.0) as u16;
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
            // Intentional: Simple additive checksum for this visibility-focused design.
            // See docstring above for rationale.
            payload.push(Self::compute_checksum(&payload));
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

            if bits.len() >= 25 * 8 {
                let bytes = Self::bits_to_bytes(&bits);
                if bytes.len() >= 25 {
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
            if candidate.len() >= 25 {
                return Some(candidate.clone());
            }
        }

        all_candidates.into_iter().next()
    }

    fn apply_to_image_owned(&self, img: &DynamicImage, ctx: &ProtectionContext) -> DynamicImage {
        let payload = self.generate_payload(ctx);
        let rgba = img.to_rgba8();

        let format = ctx
            .input_format
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        let redundancy = ctx.stego_redundancy;

        let processed = match format {
            crate::types::ImageOutputFormat::Png => {
                self.embed_lsb(&rgba, &payload, ctx.seed, redundancy)
            }
            crate::types::ImageOutputFormat::Jpeg => {
                self.embed_jpeg_stego(&rgba, &payload, ctx.seed, redundancy)
            }
            crate::types::ImageOutputFormat::WebP => {
                self.embed_lsb(&rgba, &payload, ctx.seed, redundancy)
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
        let format = ctx.input_format.unwrap_or_else(|| {
            crate::types::ImageOutputFormat::from_magic_bytes(img_bytes)
                .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT)
        });

        if format == crate::types::ImageOutputFormat::Jpeg {
            return self.apply_dct_stego_bytes(img_bytes, ctx);
        }

        let img = image::load_from_memory(img_bytes)
            .map_err(|e| crate::error::Error::ImageDecode(e.to_string()))?;

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
    pub protection_level: u8,
    pub seed: u64,
    pub intensity: f32,
    pub version: u8,
}
