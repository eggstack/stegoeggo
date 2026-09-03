//! JPEG Entropy Decoding and Encoding
//!
//! Handles Huffman entropy decoding of DCT coefficients from JPEG scan data
//! and encoding coefficients back to JPEG format.
#![allow(dead_code)]

use super::{JpegHeader, Result, TranscoderError};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
struct CanonicalHuffmanEntry {
    symbol: u8,
    code: u16,
    bit_len: u8,
}

fn build_canonical_huffman_entries(
    counts: &[u16; 16],
    values: &[u8],
) -> Result<Vec<CanonicalHuffmanEntry>> {
    let total: usize = counts.iter().map(|&c| c as usize).sum();
    if total != values.len() {
        return Err(TranscoderError::HuffmanDecode(format!(
            "Canonical Huffman: count sum {} != value count {}",
            total,
            values.len()
        )));
    }
    if total == 0 {
        return Err(TranscoderError::HuffmanDecode(
            "Canonical Huffman: empty table".into(),
        ));
    }
    if total > 256 {
        return Err(TranscoderError::HuffmanDecode(format!(
            "Canonical Huffman: {} symbols exceeds maximum of 256",
            total
        )));
    }

    let mut seen = [false; 256];
    for &v in values {
        if seen[v as usize] {
            return Err(TranscoderError::HuffmanDecode(format!(
                "Canonical Huffman: duplicate symbol 0x{:02X}",
                v
            )));
        }
        seen[v as usize] = true;
    }

    let mut available: i64 = 1;
    for &count in counts {
        available = available
            .checked_mul(2)
            .ok_or_else(|| TranscoderError::HuffmanDecode("Huffman code space overflow".into()))?;
        available = available.checked_sub(count as i64).ok_or_else(|| {
            TranscoderError::HuffmanDecode("Huffman code space oversubscribed".into())
        })?;
        if available < 0 {
            return Err(TranscoderError::HuffmanDecode(
                "Huffman code space oversubscribed".into(),
            ));
        }
    }

    let mut entries = Vec::with_capacity(total);
    let mut code: u16 = 0;
    let mut value_idx = 0;

    for (len, &count) in counts.iter().enumerate() {
        for _ in 0..count {
            if value_idx >= values.len() {
                return Err(TranscoderError::HuffmanDecode(format!(
                    "Canonical Huffman: value index {} exceeds value count {}",
                    value_idx,
                    values.len()
                )));
            }
            entries.push(CanonicalHuffmanEntry {
                symbol: values[value_idx],
                code,
                bit_len: (len + 1) as u8,
            });
            code = code.checked_add(1).ok_or_else(|| {
                TranscoderError::HuffmanDecode("Huffman code increment overflow".into())
            })?;
            value_idx += 1;
        }
        code = code
            .checked_shl(1)
            .ok_or_else(|| TranscoderError::HuffmanDecode("Huffman code shift overflow".into()))?;
    }

    Ok(entries)
}

/// Zigzag ordering for 8x8 DCT blocks
pub const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// Reverse zigzag ordering
pub const REVERSE_ZIGZAG: [usize; 64] = [
    0, 1, 5, 6, 14, 15, 27, 28, 2, 4, 7, 13, 16, 26, 29, 42, 3, 8, 12, 17, 25, 30, 41, 43, 9, 11,
    18, 24, 31, 40, 44, 53, 10, 19, 23, 32, 39, 45, 52, 54, 20, 22, 33, 38, 46, 51, 55, 60, 21, 34,
    37, 47, 50, 56, 59, 61, 35, 36, 48, 49, 57, 58, 62, 63,
];

/// Standard JPEG Huffman tables (luminance)
pub const STD_LUMINANCE_DC: ([u8; 16], &[u8]) = (
    [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
);

pub const STD_LUMINANCE_AC: ([u8; 16], &[u8]) = (
    [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125],
    &[
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61,
        0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52,
        0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x25,
        0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45,
        0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64,
        0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83,
        0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99,
        0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6,
        0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3,
        0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8,
        0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA,
    ],
);

/// Standard JPEG Huffman tables (chrominance)
pub const STD_CHROMINANCE_DC: ([u8; 16], &[u8]) = (
    [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
);
pub const STD_CHROMINANCE_AC: ([u8; 16], &[u8]) = (
    [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 119],
    &[
        0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x61, 0x07,
        0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1,
        0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x25, 0x26,
        0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46,
        0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65,
        0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84,
        0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A,
        0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,
        0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4,
        0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9,
        0xEA, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA,
    ],
);

struct HuffmanDecoder {
    by_len: [Vec<(u16, u8)>; 16],
}

struct HuffmanEncoderTable {
    entries: [(u16, u8); 256],
}

impl HuffmanEncoderTable {
    fn build(table: &super::HuffmanTable) -> Result<Self> {
        let entries_canonical = build_canonical_huffman_entries(&table.counts, &table.values)?;
        let mut entries = [(0u16, u8::MAX); 256];
        for entry in &entries_canonical {
            entries[entry.symbol as usize] = (entry.code, entry.bit_len);
        }
        Ok(Self { entries })
    }
}

impl HuffmanDecoder {
    fn from_table(counts: &[u16; 16], values: &[u8]) -> Result<Self> {
        let canonical = build_canonical_huffman_entries(counts, values)?;
        let mut by_len: [Vec<(u16, u8)>; 16] = Default::default();
        for entry in canonical {
            let idx = (entry.bit_len as usize).saturating_sub(1);
            if idx < 16 {
                by_len[idx].push((entry.code, entry.symbol));
            }
        }
        for bucket in &mut by_len {
            bucket.sort_by_key(|&(code, _)| code);
        }
        Ok(Self { by_len })
    }

    fn decode_symbol(&self, bit_reader: &mut BitReader) -> Option<u8> {
        let mut code: u16 = 0;
        for i in 0..16 {
            let bit = bit_reader.read_bit()?;
            code = (code << 1) | (bit as u16);
            let bucket = &self.by_len[i];
            if let Ok(pos) = bucket.binary_search_by_key(&code, |&(c, _)| c) {
                return Some(bucket[pos].1);
            }
        }
        None
    }
}

struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
    eoi_reached: bool,
    restart_seen: bool,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 7,
            eoi_reached: false,
            restart_seen: false,
        }
    }

    fn bytes_consumed(&self) -> usize {
        self.byte_pos
    }

    fn finish_scan(&self, expected_blocks: usize, decoded_blocks: usize) -> Result<()> {
        if decoded_blocks < expected_blocks {
            return Err(TranscoderError::HuffmanDecode(format!(
                "Early scan termination: decoded {} of {} expected blocks",
                decoded_blocks, expected_blocks
            )));
        }
        if decoded_blocks > expected_blocks {
            return Err(TranscoderError::HuffmanDecode(format!(
                "Block count overflow: decoded {} but only {} expected",
                decoded_blocks, expected_blocks
            )));
        }
        if self.bit_pos < 7 {
            let remaining_bits = self.bit_pos + 1;
            let current_byte = self.data.get(self.byte_pos).copied().unwrap_or(0);
            let mask = (1u8 << remaining_bits) - 1;
            let pad_value = current_byte & mask;
            if pad_value != mask {
                return Err(TranscoderError::HuffmanDecode(format!(
                    "Invalid pad bits: expected all-one padding, got 0x{:02X}",
                    pad_value
                )));
            }
            let next_byte_pos = self.byte_pos + 1;
            if next_byte_pos < self.data.len() {
                return Err(TranscoderError::HuffmanDecode(format!(
                    "Extra entropy bytes remain after pad bits: {} bytes at offset {}",
                    self.data.len() - next_byte_pos,
                    next_byte_pos
                )));
            }
        } else {
            if self.eoi_reached {
                return Ok(());
            }
            if self.byte_pos < self.data.len() {
                return Err(TranscoderError::HuffmanDecode(format!(
                    "Extra complete entropy byte at offset {}",
                    self.byte_pos
                )));
            }
        }
        Ok(())
    }

    fn read_bit(&mut self) -> Option<u8> {
        if self.eoi_reached {
            return None;
        }

        if self.byte_pos < self.data.len() {
            if self.bit_pos == 7
                && self.data[self.byte_pos] == 0xFF
                && self.byte_pos + 1 < self.data.len()
            {
                let next = self.data[self.byte_pos + 1];
                if next != 0x00 {
                    if next == 0xD9 {
                        self.eoi_reached = true;
                        return None;
                    }
                    if (0xD0..=0xD7).contains(&next) {
                        self.byte_pos += 2;
                        self.restart_seen = true;
                        return self.read_bit();
                    }
                    self.eoi_reached = true;
                    return None;
                }
            }

            let bit = (self.data[self.byte_pos] >> self.bit_pos) & 1;

            if self.bit_pos == 0 {
                let current = self.data[self.byte_pos];
                self.byte_pos += 1;
                if current == 0xFF
                    && self.byte_pos < self.data.len()
                    && self.data[self.byte_pos] == 0x00
                {
                    self.byte_pos += 1;
                }
                self.bit_pos = 7;
            } else {
                self.bit_pos -= 1;
            }

            return Some(bit);
        }

        None
    }
}

fn read_magnitude(bit_reader: &mut BitReader<'_>, size: usize) -> Option<i16> {
    if size == 0 {
        return Some(0);
    }
    if size >= 16 {
        return None;
    }

    let mut magnitude: i16 = 0;
    for _ in 0..size {
        magnitude = (magnitude << 1) | bit_reader.read_bit()? as i16;
    }

    let threshold = 1i16 << (size - 1);
    let adjustment = (1i16 << size) - 1;
    if magnitude < threshold {
        magnitude -= adjustment;
    }

    Some(magnitude)
}

pub struct CoefficientDecoder {
    header: JpegHeader,
    scan_data: Vec<u8>,
}

impl CoefficientDecoder {
    pub fn new(header: JpegHeader, scan_data: Vec<u8>) -> Self {
        Self { header, scan_data }
    }

    /// Decode all DCT coefficients from the JPEG
    /// Returns a map of component_id -> Vec<[i16; 64]> (blocks in MCU order)
    pub fn decode(&self) -> Result<HashMap<u8, Vec<[i16; 64]>>> {
        if self.header.is_progressive {
            return Err(TranscoderError::Unsupported(
                "Progressive JPEG decoding not yet implemented - use baseline JPEG".into(),
            ));
        }

        let mut bit_reader = BitReader::new(&self.scan_data);
        let mut result = HashMap::new();

        let max_h_sampling = self
            .header
            .components
            .iter()
            .map(|c| c.h_sampling)
            .max()
            .unwrap_or(1);
        let max_v_sampling = self
            .header
            .components
            .iter()
            .map(|c| c.v_sampling)
            .max()
            .unwrap_or(1);

        let mcu_width = (self.header.width as usize).div_ceil((max_h_sampling as usize) * 8);
        let mcu_height = (self.header.height as usize).div_ceil((max_v_sampling as usize) * 8);
        let mcus_per_row = mcu_width;

        let mut dc_predictors: HashMap<u8, i16> = HashMap::new();

        let mut dc_decoders: [Option<HuffmanDecoder>; 4] = [None, None, None, None];
        let mut ac_decoders: [Option<HuffmanDecoder>; 4] = [None, None, None, None];

        for comp in &self.header.components {
            let dc_id = comp.dc_table_id as usize;
            if dc_decoders[dc_id].is_none() {
                let table = self
                    .header
                    .get_dc_huffman_table(comp.dc_table_id)
                    .ok_or_else(|| {
                        TranscoderError::HuffmanDecode(format!(
                            "Missing DC Huffman table {} referenced by component {}",
                            comp.dc_table_id, comp.component_id
                        ))
                    })?;
                dc_decoders[dc_id] =
                    Some(HuffmanDecoder::from_table(&table.counts, &table.values)?);
            }

            let ac_id = comp.ac_table_id as usize;
            if ac_decoders[ac_id].is_none() {
                let table = self
                    .header
                    .get_ac_huffman_table(comp.ac_table_id)
                    .ok_or_else(|| {
                        TranscoderError::HuffmanDecode(format!(
                            "Missing AC Huffman table {} referenced by component {}",
                            comp.ac_table_id, comp.component_id
                        ))
                    })?;
                ac_decoders[ac_id] =
                    Some(HuffmanDecoder::from_table(&table.counts, &table.values)?);
            }
        }

        let mut decoded_blocks: usize = 0;

        for _mcu_y in 0..mcu_height {
            for _mcu_x in 0..mcus_per_row {
                for comp in &self.header.components {
                    let dc_decoder =
                        dc_decoders[comp.dc_table_id as usize]
                            .as_ref()
                            .ok_or_else(|| {
                                TranscoderError::HuffmanDecode(format!(
                                    "Missing pre-built DC decoder table {}",
                                    comp.dc_table_id
                                ))
                            })?;
                    let ac_decoder =
                        ac_decoders[comp.ac_table_id as usize]
                            .as_ref()
                            .ok_or_else(|| {
                                TranscoderError::HuffmanDecode(format!(
                                    "Missing pre-built AC decoder table {}",
                                    comp.ac_table_id
                                ))
                            })?;

                    for _by in 0..comp.v_sampling {
                        for _bx in 0..comp.h_sampling {
                            let mut block = [0i16; 64];

                            let dc_predictor = dc_predictors.entry(comp.component_id).or_insert(0);
                            let size =
                                dc_decoder.decode_symbol(&mut bit_reader).ok_or_else(|| {
                                    TranscoderError::HuffmanDecode(format!(
                                    "Missing or truncated DC symbol at MCU ({},{}) component {}",
                                    _mcu_x, _mcu_y, comp.component_id
                                ))
                                })?;
                            let diff = read_magnitude(&mut bit_reader, size as usize).ok_or_else(
                                || {
                                    TranscoderError::HuffmanDecode(format!(
                                        "Truncated DC magnitude at MCU ({},{}) component {}",
                                        _mcu_x, _mcu_y, comp.component_id
                                    ))
                                },
                            )?;
                            let new_val = (*dc_predictor as i32) + (diff as i32);
                            let clamped = new_val.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                            *dc_predictor = clamped;
                            block[0] = *dc_predictor;

                            let mut k = 1;
                            while k < 64 {
                                let ss = ac_decoder
                                    .decode_symbol(&mut bit_reader)
                                    .ok_or_else(|| TranscoderError::HuffmanDecode(format!(
                                        "Missing or truncated AC symbol at MCU ({},{}) component {} coefficient {}",
                                        _mcu_x, _mcu_y, comp.component_id, k
                                    )))?;
                                let run = (ss >> 4) & 0x0F;
                                let size = ss & 0x0F;

                                if size == 0 {
                                    if run == 0 {
                                        break;
                                    }
                                    if run == 0x0F {
                                        let new_k = k + 16;
                                        if new_k > 64 {
                                            return Err(TranscoderError::HuffmanDecode(format!(
                                                "ZRL run overflows block at MCU ({},{}) component {}",
                                                _mcu_x, _mcu_y, comp.component_id
                                            )));
                                        }
                                        k = new_k;
                                    } else {
                                        return Err(TranscoderError::HuffmanDecode(format!(
                                            "Invalid zero-size symbol 0x{:02X} at MCU ({},{}) component {}",
                                            ss, _mcu_x, _mcu_y, comp.component_id
                                        )));
                                    }
                                } else {
                                    let new_k = k + run as usize;
                                    if new_k >= 64 {
                                        return Err(TranscoderError::HuffmanDecode(format!(
                                            "AC run overflows block at MCU ({},{}) component {}",
                                            _mcu_x, _mcu_y, comp.component_id
                                        )));
                                    }

                                    let magnitude =
                                        read_magnitude(&mut bit_reader, size as usize)
                                            .ok_or_else(|| {
                                                TranscoderError::HuffmanDecode(format!(
                                                    "Truncated AC magnitude at MCU ({},{}) component {}",
                                                    _mcu_x, _mcu_y, comp.component_id
                                                ))
                                            })?;

                                    block[ZIGZAG[new_k]] = magnitude;
                                    k = new_k + 1;
                                }
                            }

                            result
                                .entry(comp.component_id)
                                .or_insert_with(Vec::new)
                                .push(block);
                            decoded_blocks += 1;
                        }
                    }
                }

                if bit_reader.restart_seen {
                    bit_reader.restart_seen = false;
                    dc_predictors.clear();
                }
            }
        }

        let total_mcu_count = mcu_height * mcus_per_row;
        let mut expected_blocks: usize = 0;
        for comp in &self.header.components {
            expected_blocks +=
                total_mcu_count * (comp.h_sampling as usize) * (comp.v_sampling as usize);
        }
        bit_reader.finish_scan(expected_blocks, decoded_blocks)?;

        Ok(result)
    }
}

/// Bit writer for encoding
struct BitWriter {
    data: Vec<u8>,
    current_byte: u8,
    bits_in_byte: u8,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            current_byte: 0,
            bits_in_byte: 0,
        }
    }

    fn write_bits(&mut self, code: u16, size: u8) {
        // Handle size 0 (special case)
        if size == 0 {
            return;
        }

        let mut bits_remaining = size;

        while bits_remaining > 0 {
            // Get next bit
            let bit = ((code >> (bits_remaining - 1)) & 1) as u8;
            self.current_byte = (self.current_byte << 1) | bit;
            self.bits_in_byte += 1;
            bits_remaining -= 1;

            if self.bits_in_byte == 8 {
                self.data.push(self.current_byte);

                // Handle byte stuffing
                if self.current_byte == 0xFF {
                    self.data.push(0x00);
                }

                self.current_byte = 0;
                self.bits_in_byte = 0;
            }
        }
    }

    fn align_byte(&mut self) {
        if self.bits_in_byte > 0 {
            // Write any remaining bits padded with 1s
            while self.bits_in_byte < 8 {
                self.current_byte = (self.current_byte << 1) | 1;
                self.bits_in_byte += 1;
            }
            self.data.push(self.current_byte);

            if self.current_byte == 0xFF {
                self.data.push(0x00);
            }

            self.current_byte = 0;
            self.bits_in_byte = 0;
        }
    }

    fn finalize(mut self) -> Vec<u8> {
        self.align_byte();
        self.data
    }
}

pub struct CoefficientEncoder {
    header: JpegHeader,
}

impl CoefficientEncoder {
    pub fn new(header: JpegHeader) -> Self {
        Self { header }
    }

    /// Encode coefficient blocks back to JPEG scan data
    pub fn encode(&self, coefficients: &HashMap<u8, Vec<[i16; 64]>>) -> Result<Vec<u8>> {
        if self.header.is_progressive {
            return Err(TranscoderError::Unsupported(
                "Progressive JPEG encoding not yet implemented".into(),
            ));
        }

        let mut bit_writer = BitWriter::new();

        // Calculate MCU dimensions
        let max_h_sampling = self
            .header
            .components
            .iter()
            .map(|c| c.h_sampling)
            .max()
            .unwrap_or(1);
        let max_v_sampling = self
            .header
            .components
            .iter()
            .map(|c| c.v_sampling)
            .max()
            .unwrap_or(1);

        let mcu_width = (self.header.width as usize).div_ceil((max_h_sampling as usize) * 8);
        let mcu_height = (self.header.height as usize).div_ceil((max_v_sampling as usize) * 8);
        let mcus_per_row = mcu_width;

        // Initialize DC predictors
        let mut dc_predictors: HashMap<u8, i16> = HashMap::new();

        // Pre-build Huffman encode tables once
        let mut dc_enc_tables: [Option<HuffmanEncoderTable>; 4] = [None, None, None, None];
        let mut ac_enc_tables: [Option<HuffmanEncoderTable>; 4] = [None, None, None, None];

        for comp in &self.header.components {
            let dc_id = comp.dc_table_id as usize;
            if dc_enc_tables[dc_id].is_none() {
                let table = self
                    .header
                    .get_dc_huffman_table(comp.dc_table_id)
                    .ok_or_else(|| {
                        TranscoderError::HuffmanEncode(format!(
                            "Missing DC Huffman table {} referenced by component {}",
                            comp.dc_table_id, comp.component_id
                        ))
                    })?;
                dc_enc_tables[dc_id] = Some(HuffmanEncoderTable::build(table)?);
            }

            let ac_id = comp.ac_table_id as usize;
            if ac_enc_tables[ac_id].is_none() {
                let table = self
                    .header
                    .get_ac_huffman_table(comp.ac_table_id)
                    .ok_or_else(|| {
                        TranscoderError::HuffmanEncode(format!(
                            "Missing AC Huffman table {} referenced by component {}",
                            comp.ac_table_id, comp.component_id
                        ))
                    })?;
                ac_enc_tables[ac_id] = Some(HuffmanEncoderTable::build(table)?);
            }
        }

        // Process each MCU
        for mcu_y in 0..mcu_height {
            for mcu_x in 0..mcus_per_row {
                // Process each component
                for comp in &self.header.components {
                    let dc_enc = dc_enc_tables[comp.dc_table_id as usize]
                        .as_ref()
                        .ok_or_else(|| {
                            TranscoderError::HuffmanEncode(format!(
                                "Missing pre-built DC encoder table {}",
                                comp.dc_table_id
                            ))
                        })?;
                    let ac_enc = ac_enc_tables[comp.ac_table_id as usize]
                        .as_ref()
                        .ok_or_else(|| {
                            TranscoderError::HuffmanEncode(format!(
                                "Missing pre-built AC encoder table {}",
                                comp.ac_table_id
                            ))
                        })?;

                    // Get blocks for this component
                    let Some(blocks) = coefficients.get(&comp.component_id) else {
                        continue;
                    };

                    // Block index within this component's block vector.
                    // The decoder pushes blocks in MCU order: for each MCU, each
                    // component's sub-blocks are pushed in (by, bx) order.
                    // Within each MCU, this component's blocks start at
                    // (mcu_y * mcus_per_row + mcu_x) * h * v.
                    for by in 0..comp.v_sampling {
                        for bx in 0..comp.h_sampling {
                            let block_idx = (mcu_y * mcus_per_row + mcu_x)
                                * (comp.h_sampling as usize)
                                * (comp.v_sampling as usize)
                                + (by as usize) * (comp.h_sampling as usize)
                                + (bx as usize);

                            if block_idx >= blocks.len() {
                                continue;
                            }

                            let block = &blocks[block_idx];

                            // Encode DC coefficient
                            let dc_predictor = dc_predictors.entry(comp.component_id).or_insert(0);
                            let diff = (block[0] as i32) - (*dc_predictor as i32);
                            *dc_predictor = block[0];

                            self.encode_dc_coefficient(&mut bit_writer, diff, dc_enc)?;

                            // Encode AC coefficients
                            self.encode_ac_coefficients(&mut bit_writer, block, ac_enc)?;
                        }
                    }
                }
            }
        }

        Ok(bit_writer.finalize())
    }

    fn encode_dc_coefficient(
        &self,
        writer: &mut BitWriter,
        diff: i32,
        table: &HuffmanEncoderTable,
    ) -> Result<()> {
        if diff == 0 {
            // Size 0
            self.write_huffman_code(writer, table, 0)?;
        } else {
            let abs_diff = diff.unsigned_abs();
            let size = self.magnitude_size(u16::try_from(abs_diff).unwrap_or(u16::MAX));

            // Clamp to max encodable size (standard DC tables support up to 11)
            let encodable_size = size.min(11);
            let clamped_diff = if size > 11 {
                let max_val = (1i32 << 11) - 1;
                if diff > 0 {
                    max_val
                } else {
                    -max_val
                }
            } else {
                diff
            };

            // Write Huffman code for size
            self.write_huffman_code(writer, table, encodable_size)?;

            // Write magnitude bits (without sign extension)
            if clamped_diff > 0 {
                writer.write_bits(clamped_diff as u16, encodable_size);
            } else {
                // JPEG negative-value representation: add the max positive value
                // for this size and write the resulting codeword bits.
                let magnitude = (clamped_diff + ((1i32 << encodable_size) - 1)) as u16;
                writer.write_bits(magnitude, encodable_size);
            }
        }

        Ok(())
    }

    fn encode_ac_coefficients(
        &self,
        writer: &mut BitWriter,
        block: &[i16; 64],
        table: &HuffmanEncoderTable,
    ) -> Result<()> {
        let mut k = 1;

        while k < 64 {
            let value = block[ZIGZAG[k]];

            if value == 0 {
                let mut zero_count = 0;
                while k < 64 && block[ZIGZAG[k]] == 0 {
                    zero_count += 1;
                    k += 1;
                }

                if k >= 64 {
                    self.write_huffman_code(writer, table, 0x00)?;
                    break;
                }

                while zero_count >= 16 {
                    self.write_huffman_code(writer, table, 0xF0)?;
                    zero_count -= 16;
                }

                let non_zero_value = block[ZIGZAG[k]];
                let size = self.magnitude_size(non_zero_value.unsigned_abs());
                let encodable_size = size.min(10);
                let clamped_value = if size > 10 {
                    let max_val = ((1i32 << 10) - 1) as i16;
                    if non_zero_value > 0 {
                        max_val
                    } else {
                        -max_val
                    }
                } else {
                    non_zero_value
                };

                let rs = (zero_count << 4) | encodable_size;
                self.write_huffman_code(writer, table, rs)?;

                if clamped_value > 0 {
                    writer.write_bits(clamped_value as u16, encodable_size);
                } else {
                    let magnitude = (clamped_value as i32 + ((1i32 << encodable_size) - 1)) as u16;
                    writer.write_bits(magnitude, encodable_size);
                }

                k += 1;
            } else {
                let size = self.magnitude_size(value.unsigned_abs());
                let encodable_size = size.min(10);
                let clamped_value = if size > 10 {
                    let max_val = ((1i32 << 10) - 1) as i16;
                    if value > 0 {
                        max_val
                    } else {
                        -max_val
                    }
                } else {
                    value
                };

                let rs = encodable_size;
                self.write_huffman_code(writer, table, rs)?;

                if clamped_value > 0 {
                    writer.write_bits(clamped_value as u16, encodable_size);
                } else {
                    let magnitude = (clamped_value as i32 + ((1i32 << encodable_size) - 1)) as u16;
                    writer.write_bits(magnitude, encodable_size);
                }

                k += 1;
            }
        }

        Ok(())
    }

    fn magnitude_size(&self, magnitude: u16) -> u8 {
        if magnitude == 0 {
            return 0;
        }

        let mut size = 0;
        let mut m = magnitude;
        while m > 0 {
            size += 1;
            m >>= 1;
        }
        size
    }

    fn write_huffman_code(
        &self,
        writer: &mut BitWriter,
        table: &HuffmanEncoderTable,
        symbol: u8,
    ) -> Result<()> {
        let (code, length) = table.entries[symbol as usize];
        if length == 0 {
            return Err(TranscoderError::HuffmanEncode(format!(
                "Symbol 0x{:02X} not found in Huffman table",
                symbol
            )));
        }
        writer.write_bits(code, length);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_lookup_derived_from_canonical_entries() {
        let counts: [u16; 16] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
        let values: Vec<u8> = (0..16).collect();
        let decoder = HuffmanDecoder::from_table(&counts, &values).expect("decoder");
        assert_eq!(decoder.by_len.len(), 16);
        let total: usize = decoder.by_len.iter().map(|b| b.len()).sum();
        assert_eq!(total, 16, "decoder should have 16 entries");
        for bucket in &decoder.by_len {
            assert_eq!(bucket.len(), 1);
        }
    }

    #[test]
    fn valid_zero_count_intermediate_lengths_decode() {
        let counts: [u16; 16] = [1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0];
        let values: Vec<u8> = (0..8).collect();
        let decoder =
            HuffmanDecoder::from_table(&counts, &values).expect("decoder with zero-count slot");
        let total: usize = decoder.by_len.iter().map(|b| b.len()).sum();
        assert_eq!(total, 8);
        for (i, bucket) in decoder.by_len.iter().enumerate() {
            if i % 2 == 0 {
                assert_eq!(bucket.len(), 1);
            } else {
                assert_eq!(bucket.len(), 0);
            }
        }
    }

    #[test]
    fn test_zigzag_is_valid_permutation() {
        let mut seen = [false; 64];
        for &idx in &ZIGZAG {
            assert!(idx < 64, "ZIGZAG value {} out of range", idx);
            assert!(!seen[idx], "ZIGZAG has duplicate index {}", idx);
            seen[idx] = true;
        }
    }

    #[test]
    fn test_reverse_zigzag_is_inverse() {
        for i in 0..64 {
            assert_eq!(
                REVERSE_ZIGZAG[ZIGZAG[i]], i,
                "REVERSE_ZIGZAG is not the inverse of ZIGZAG at index {}",
                i
            );
        }
    }

    #[test]
    fn test_block_storage_is_natural_order() {
        // Verify that block[ZIGZAG[k]] stores coefficient at natural position.
        // ZIGZAG[k] maps zigzag index k to natural (row-major) position.
        // The standard zigzag for the first few positions:
        //   zigzag 0 -> natural 0  (0,0) DC
        //   zigzag 1 -> natural 1  (0,1)
        //   zigzag 2 -> natural 8  (1,0)
        //   zigzag 3 -> natural 16 (2,0)
        assert_eq!(ZIGZAG[0], 0);
        assert_eq!(ZIGZAG[1], 1);
        assert_eq!(ZIGZAG[2], 8);
        assert_eq!(ZIGZAG[3], 16);
    }
}
