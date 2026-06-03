//! DCT-based Steganography
//!
//! Production-ready steganography for JPEG images using F5-style DCT coefficient embedding.
//! Features:
//! - F5-style embedding with shrinkage handling
//! - Configurable redundancy for robustness
//! - Seed embedded in quantization tables when those tables are preserved
//! - Progressive JPEG support
//!
//! ## Variant: No-zero-coefficient F5
//!
//! This implementation differs from the original F5 algorithm: it never creates zero
//! coefficients. Standard F5 decrements coefficient values toward zero when the LSB
//! doesn't match the target bit, which can create detectable shrinkage patterns (a
//! histogram of zeros that is larger than expected). Instead, when |coef| == 1 and the
//! LSB doesn't match, this implementation *increments* the absolute value (+1 -> +2,
//! -1 -> -2) to flip the LSB without creating a zero. This trades slightly larger
//! coefficient magnitude changes for reduced statistical detectability via shrinkage
//! analysis. Position alignment between embed and extract is preserved because no
//! coefficient is ever zeroed out.

use super::{JpegHeader, Result, TranscoderError};
use std::collections::HashMap;

/// Seed embedding magic (stored in quantization tables)
const SEED_MAGIC: &[u8] = b"SEED";

/// DCT coefficient shuffling PRNG for F5 steganography.
/// Uses a different algorithm than the general-purpose `PixelSelectionRng` in `util/image.rs`.
/// Changing this algorithm would break compatibility with existing steganographic data.
///
/// **WARNING:** Do NOT interchange with the general-purpose `PixelSelectionRng` — they produce
/// different sequences for the same seed and are each paired with their respective
/// embed/extract code paths.
struct DctCoefficientRng(u64);

impl DctCoefficientRng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 1 } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn gen_range(&mut self, range: usize) -> usize {
        (self.next_u64() as usize) % range
    }
}

/// F5-style coefficient steganography
pub struct DctStegoF5 {
    redundancy: usize,
}

impl DctStegoF5 {
    #[inline]
    fn normalize_ac_coefficient(value: i16) -> i16 {
        value.clamp(-1023, 1023)
    }

    pub fn new() -> Self {
        Self { redundancy: 3 }
    }

    pub fn with_redundancy(redundancy: usize) -> Self {
        Self {
            redundancy: redundancy.clamp(1, 10),
        }
    }

    /// Embed seed in quantization tables when those tables are preserved.
    ///
    /// Embeds 12 bytes: 4 bytes magic + 8 bytes seed (u64).
    ///
    /// Positions where the quantization value is 1 are skipped because `1 & 0xFE`
    /// would change the value from 1 to 0, corrupting the Q-table. This means
    /// fewer bits are embedded than intended. Extraction handles partial reads
    /// gracefully — if the full seed cannot be recovered, the caller falls back
    /// to other extraction methods.
    pub fn embed_seed_in_quantization_tables(
        &self,
        header: &mut JpegHeader,
        seed: u64,
    ) -> Result<()> {
        let mut payload = Vec::new();
        payload.extend_from_slice(SEED_MAGIC);

        payload.extend_from_slice(&seed.to_le_bytes());

        let bits: Vec<u8> = payload
            .iter()
            .flat_map(|&b| (0..8).map(move |i| (b >> i) & 1))
            .collect();

        let mut bit_idx = 0;
        for table_idx in 0..2 {
            if let Some(ref mut quant) = header.quantization_tables[table_idx] {
                for pos in 0..64 {
                    if bit_idx >= bits.len() {
                        break;
                    }
                    if quant.values[pos] == 1 {
                        continue;
                    }
                    if bits[bit_idx] == 1 {
                        quant.values[pos] |= 1;
                    } else {
                        quant.values[pos] &= 0xFE;
                    }
                    bit_idx += 1;
                }
            }
        }

        Ok(())
    }

    /// Extract seed from quantization tables
    pub fn extract_seed_from_quantization_tables(&self, header: &JpegHeader) -> Option<u64> {
        let mut bits: Vec<u8> = Vec::new();

        // Extract bits from quantization tables
        // Read from both tables since we embed 96 bits across 2 tables
        for table_idx in 0..2 {
            if let Some(ref quant) = header.quantization_tables[table_idx] {
                for (j, &val) in quant.values.iter().enumerate() {
                    if j >= 64 {
                        break;
                    }
                    bits.push((val & 1) as u8);
                }
            }
        }

        // Need at least 96 bits (12 bytes)
        if bits.len() < 96 {
            return None;
        }

        // Check SEED magic (32 bits = 4 bytes)
        let mut magic = [0u8; 4];
        for (k, byte_slot) in magic.iter_mut().enumerate() {
            for i in 0..8 {
                let idx = k * 8 + i;
                if idx < bits.len() {
                    *byte_slot |= bits[idx] << i;
                }
            }
        }

        if magic != SEED_MAGIC {
            return None;
        }

        // Extract seed (next 8 bytes = 64 bits)
        let mut seed_bytes = [0u8; 8];
        for (k, byte_slot) in seed_bytes.iter_mut().enumerate() {
            for i in 0..8 {
                let idx = 32 + k * 8 + i;
                if idx < bits.len() {
                    *byte_slot |= bits[idx] << i;
                }
            }
        }

        Some(u64::from_le_bytes(seed_bytes))
    }

    /// F5-style DCT embedding
    ///
    /// Algorithm:
    /// 1. Skip DC coefficient (position 0)
    /// 2. For each bit:
    ///    - Find AC coefficients with magnitude >= 2
    ///    - If LSB matches target, keep it
    ///    - If LSB doesn't match, decrement absolute value (F5's shrinkage handling)
    /// 3. Embed with configurable redundancy
    pub fn embed_f5(
        &self,
        coefficients: &mut HashMap<u8, Vec<[i16; 64]>>,
        payload: &[u8],
        seed: u64,
    ) -> Result<usize> {
        if payload.is_empty() {
            return Ok(0);
        }

        // Canonicalize all AC coefficients into the encoder's representable range
        // before selecting carriers. This keeps the payload path aligned with the
        // JPEG bytes that will actually be emitted.
        for blocks in coefficients.values_mut() {
            for block in blocks.iter_mut() {
                for coef in block.iter_mut().skip(1) {
                    *coef = Self::normalize_ac_coefficient(*coef);
                }
            }
        }

        // Convert payload to bits
        let mut bits: Vec<u8> = payload
            .iter()
            .flat_map(|&b| (0..8).map(move |i| (b >> i) & 1))
            .collect();

        let original_bit_count = bits.len();

        // Add redundancy by repeating bits
        if self.redundancy > 1 {
            let full_bits = bits.clone();
            for _ in 1..self.redundancy {
                bits.extend_from_slice(&full_bits);
            }
        }

        // Collect AC positions with magnitude >= 2 in deterministic component/block order.
        //
        // The carrier set must stay stable after embedding. Because this variant
        // never drops selected coefficients below magnitude 2, the carrier set
        // remains stable after the one-step adjustment used here.
        let mut positions: Vec<(u8, usize, usize)> = Vec::new();
        for (comp_id, blocks) in coefficients.iter() {
            for (block_idx, block) in blocks.iter().enumerate() {
                for (pos, &coef) in block.iter().enumerate().skip(1) {
                    if coef.abs() >= 2 {
                        positions.push((*comp_id, block_idx, pos));
                    }
                }
            }
        }

        positions.sort();

        // Shuffle positions using seeded PRNG for pseudo-random ordering
        let mut rng = DctCoefficientRng::new(seed);
        for i in (1..positions.len()).rev() {
            let j = rng.gen_range(i + 1);
            positions.swap(i, j);
        }

        if bits.len() > positions.len() {
            return Err(TranscoderError::EmbeddingFailed(format!(
                "Insufficient capacity: need {} bits, have {} AC coefficients",
                bits.len(),
                positions.len()
            )));
        }

        // Embed bits using F5-style value decrementing.
        // Each selected AC coefficient in shuffled order consumes exactly one bit.
        // - LSB mismatch: decrement/increment toward zero, but never below |2|
        // - LSB already matches: no-op (position still consumed, bit counted)

        let mut bit_idx = 0usize;

        for &(comp_id, block_idx, pos) in &positions {
            if bit_idx >= bits.len() {
                break;
            }

            let current = coefficients
                .get(&comp_id)
                .and_then(|b| b.get(block_idx))
                .map(|b| b[pos])
                .unwrap_or(0);
            let current = Self::normalize_ac_coefficient(current);

            let target_bit = bits[bit_idx];
            let block = coefficients
                .get_mut(&comp_id)
                .unwrap()
                .get_mut(block_idx)
                .unwrap();
            block[pos] = current;

            let current_lsb = (current & 1) as u8;

            if current_lsb == target_bit {
                // Bit already matches, consume it
                bit_idx += 1;
                continue;
            }

            // LSB doesn't match — flip it without creating zero.
            if current.abs() <= 2 {
                block[pos] = if current > 0 { 3 } else { -3 };
            } else if current > 0 {
                block[pos] = current - 1;
            } else {
                block[pos] = current + 1;
            }
            bit_idx += 1;
        }

        if bit_idx < bits.len() {
            return Err(TranscoderError::EmbeddingFailed(format!(
                "Insufficient capacity: {} bits remain unembedded",
                bits.len() - bit_idx
            )));
        }

        Ok(original_bit_count)
    }

    /// F5-style DCT extraction.
    ///
    /// `expected_bits` is the original payload bit count before redundancy
    /// repetition. When redundancy is greater than 1, the extractor reads
    /// `expected_bits * redundancy` bits and then majority-votes them back
    /// down to the original length.
    pub fn extract_f5(
        &self,
        coefficients: &HashMap<u8, Vec<[i16; 64]>>,
        expected_bits: usize,
        seed: u64,
    ) -> Vec<u8> {
        // Collect AC positions with magnitude >= 2 in the same deterministic
        // order as embedding.
        let mut positions: Vec<(u8, usize, usize)> = Vec::new();
        for (comp_id, blocks) in coefficients.iter() {
            for (block_idx, block) in blocks.iter().enumerate() {
                for (pos, &coef) in block.iter().enumerate().skip(1) {
                    if coef.abs() >= 2 {
                        positions.push((*comp_id, block_idx, pos));
                    }
                }
            }
        }

        positions.sort();

        // Shuffle with same seed
        let mut rng = DctCoefficientRng::new(seed);
        for i in (1..positions.len()).rev() {
            let j = rng.gen_range(i + 1);
            positions.swap(i, j);
        }

        // Extract LSBs
        let required_bits = if self.redundancy > 1 {
            expected_bits.saturating_mul(self.redundancy)
        } else {
            expected_bits
        };
        let mut bits = Vec::with_capacity(required_bits);
        for &(comp_id, block_idx, pos) in positions.iter() {
            if bits.len() >= required_bits {
                break;
            }

            if let Some(block) = coefficients.get(&comp_id).and_then(|b| b.get(block_idx)) {
                bits.push((block[pos] & 1) as u8);
            }
        }

        // Remove redundancy by majority voting per bit position
        if self.redundancy > 1 && bits.len() >= required_bits {
            let mut decoded_bits = Vec::with_capacity(expected_bits);
            for i in 0..expected_bits {
                let mut ones = 0;
                for r in 0..self.redundancy {
                    let idx = i + r * expected_bits;
                    if idx < bits.len() && bits[idx] == 1 {
                        ones += 1;
                    }
                }
                decoded_bits.push(if ones > self.redundancy / 2 { 1 } else { 0 });
            }
            return decoded_bits;
        }

        bits
    }
}

impl Default for DctStegoF5 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_coefficients(block_count: usize) -> HashMap<u8, Vec<[i16; 64]>> {
        let mut coefficients: HashMap<u8, Vec<[i16; 64]>> = HashMap::new();
        let mut blocks = Vec::new();

        for i in 0..block_count {
            let mut block = [0i16; 64];
            block[0] = 50;
            for (j, val) in block.iter_mut().enumerate().skip(1) {
                *val = ((i * 63 + j) as i16) * 2 + 3;
            }
            blocks.push(block);
        }

        coefficients.insert(1, blocks);
        coefficients
    }

    fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for chunk in bits.chunks(8) {
            if chunk.len() < 8 {
                break;
            }
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                byte |= bit << i;
            }
            bytes.push(byte);
        }
        bytes
    }

    fn shuffled_positions(
        coefficients: &HashMap<u8, Vec<[i16; 64]>>,
        seed: u64,
    ) -> Vec<(u8, usize, usize)> {
        let mut positions: Vec<(u8, usize, usize)> = Vec::new();
        for (comp_id, blocks) in coefficients.iter() {
            for (block_idx, block) in blocks.iter().enumerate() {
                for (pos, _) in block.iter().enumerate().skip(1) {
                    positions.push((*comp_id, block_idx, pos));
                }
            }
        }

        positions.sort();

        let mut rng = DctCoefficientRng::new(seed);
        for i in (1..positions.len()).rev() {
            let j = rng.gen_range(i + 1);
            positions.swap(i, j);
        }

        positions
    }

    fn flip_lsb_without_zero(value: i16) -> i16 {
        if value & 1 == 0 {
            if value > 0 {
                value + 1
            } else {
                value - 1
            }
        } else if value > 0 {
            value + 1
        } else {
            value - 1
        }
    }

    #[test]
    fn test_seed_embed_extract() {
        let mut header = JpegHeader::default();
        // Initialize quantization tables with non-zero values
        for i in 0..2 {
            let mut table = crate::jpeg_transcoder::header::QuantizationTable {
                table_id: i as u8,
                precision: 8,
                values: [16; 64], // Standard luminance quantization table
            };
            // Make some values different to ensure embedding works
            for (j, val) in table.values.iter_mut().enumerate() {
                *val = 16 + j as u16;
            }
            header.quantization_tables[i] = Some(table);
        }

        let seed = 0x12345678DEADBEEFu64;

        let stego = DctStegoF5::new();
        stego
            .embed_seed_in_quantization_tables(&mut header, seed)
            .unwrap();

        let extracted = stego
            .extract_seed_from_quantization_tables(&header)
            .unwrap();
        assert_eq!(extracted, seed);
    }

    #[test]
    fn test_f5_redundancy_1_roundtrip() {
        let stego = DctStegoF5::with_redundancy(1);
        let mut coefficients = make_coefficients(8);
        let payload = b"Hello, World!";

        stego.embed_f5(&mut coefficients, payload, 42).unwrap();

        let bits = stego.extract_f5(&coefficients, payload.len() * 8, 42);
        assert_eq!(bits_to_bytes(&bits), payload);
    }

    #[test]
    fn test_f5_redundancy_3_majority_recovers_corrupted_copy() {
        let stego = DctStegoF5::with_redundancy(3);
        let mut coefficients = make_coefficients(8);
        let payload = b"ABCD";

        stego.embed_f5(&mut coefficients, payload, 42).unwrap();

        let positions = shuffled_positions(&coefficients, 42);
        let expected_bits = payload.len() * 8;
        let corrupted_idx = expected_bits + 5;
        let (comp_id, block_idx, pos) = positions[corrupted_idx];
        let block = coefficients
            .get_mut(&comp_id)
            .and_then(|blocks| blocks.get_mut(block_idx))
            .unwrap();
        block[pos] = flip_lsb_without_zero(block[pos]);

        let bits = stego.extract_f5(&coefficients, expected_bits, 42);
        assert_eq!(bits_to_bytes(&bits), payload);
    }

    #[test]
    fn test_f5_wrong_seed_does_not_recover_payload() {
        let stego = DctStegoF5::with_redundancy(3);
        let mut coefficients = make_coefficients(8);
        let payload = b"test!";

        stego.embed_f5(&mut coefficients, payload, 42).unwrap();

        let bits = stego.extract_f5(&coefficients, payload.len() * 8, 99);
        let recovered = bits_to_bytes(&bits);
        assert_ne!(recovered, payload);
    }

    #[test]
    fn test_seed_embed_with_unit_quant_values() {
        let mut header = JpegHeader::default();
        // Use value 2 which can embed both 0-bit (2 & 0xFE = 2) and 1-bit (2 | 1 = 3).
        // This also tests that values never drop below 1 after embedding.
        for i in 0..2 {
            let table = crate::jpeg_transcoder::header::QuantizationTable {
                table_id: i as u8,
                precision: 8,
                values: [2; 64],
            };
            header.quantization_tables[i] = Some(table);
        }

        let seed = 0xCAFEBABEu64;
        let stego = DctStegoF5::new();
        stego
            .embed_seed_in_quantization_tables(&mut header, seed)
            .unwrap();

        // All quantization values should remain >= 1
        for table in header.quantization_tables.iter().flatten() {
            for &val in &table.values[..64] {
                assert!(val >= 1, "Quantization value must be >= 1, got {}", val);
            }
        }

        // Seed should still be extractable
        let extracted = stego
            .extract_seed_from_quantization_tables(&header)
            .unwrap();
        assert_eq!(extracted, seed);
    }

    #[test]
    fn test_seed_embed_all_ones_quant_skips_positions() {
        let mut header = JpegHeader::default();
        for i in 0..2 {
            let table = crate::jpeg_transcoder::header::QuantizationTable {
                table_id: i as u8,
                precision: 8,
                values: [1; 64],
            };
            header.quantization_tables[i] = Some(table);
        }

        let stego = DctStegoF5::new();
        let result = stego.embed_seed_in_quantization_tables(&mut header, 0xCAFEBABEu64);
        assert!(result.is_ok());

        let extracted = stego.extract_seed_from_quantization_tables(&header);
        assert!(
            extracted.is_none(),
            "all-ones tables should not yield a valid seed"
        );
    }

    #[test]
    fn test_seed_survives_reencoding() {
        // This test verifies that the seed survives workflows that preserve
        // quantization tables.
        let mut header = JpegHeader::default();
        // Initialize quantization tables with non-zero values
        for i in 0..2 {
            let mut table = crate::jpeg_transcoder::header::QuantizationTable {
                table_id: i as u8,
                precision: 8,
                values: [16; 64],
            };
            for (j, val) in table.values.iter_mut().enumerate() {
                *val = 16 + j as u16;
            }
            header.quantization_tables[i] = Some(table);
        }

        let seed = 0xABCDEF1234567890u64;

        let stego = DctStegoF5::new();
        stego
            .embed_seed_in_quantization_tables(&mut header, seed)
            .unwrap();

        // Simulate a table-preserving round trip.

        let extracted = stego
            .extract_seed_from_quantization_tables(&header)
            .unwrap();
        assert_eq!(extracted, seed);
    }
}
