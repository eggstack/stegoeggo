//! JPEG DCT Transcoder for steganography
//!
//! This module provides JPEG transcoding capabilities that preserve DCT coefficients,
//! allowing steganographic embedding that survives re-encoding.

pub mod entropy;
pub mod header;
pub mod stego_f5;

use std::collections::HashMap;

use thiserror::Error;

pub type Coefficients = HashMap<u8, Vec<[i16; 64]>>;

#[derive(Error, Debug)]
pub enum TranscoderError {
    #[error("Invalid JPEG format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported JPEG feature: {0}")]
    Unsupported(String),

    #[error("Huffman decoding error: {0}")]
    HuffmanDecode(String),

    #[error("Huffman encoding error: {0}")]
    HuffmanEncode(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Embedding failed: {0}")]
    EmbeddingFailed(String),
}

pub type Result<T> = std::result::Result<T, TranscoderError>;

pub use entropy::{CoefficientDecoder, CoefficientEncoder};
pub use header::{HuffmanTable, JpegHeader};
pub use stego_f5::DctStegoF5;

/// Main JPEG DCT Transcoder
/// Provides lossless JPEG transcoding that preserves DCT coefficients
pub struct JpegTranscoder;

impl JpegTranscoder {
    /// Decode JPEG and extract raw DCT coefficients
    pub fn decode_coefficients(jpeg_data: &[u8]) -> Result<(JpegHeader, Coefficients)> {
        // Parse header first to check if progressive
        let header = match JpegHeader::parse(jpeg_data) {
            Ok(h) => h,
            Err(e) => {
                return Err(e);
            }
        };

        // If progressive, return error — caller should handle via metadata-only path
        if header.is_progressive {
            return Err(TranscoderError::Unsupported(
                "Progressive JPEG: DCT coefficient manipulation not supported, \
                 falling back to metadata-only protection"
                    .into(),
            ));
        }

        // Find scan data start
        let scan_start = scan_utils::get_scan_data_start(jpeg_data)
            .ok_or_else(|| TranscoderError::InvalidFormat("Could not find scan data".into()))?;

        let scan_data = jpeg_data[scan_start..].to_vec();

        // Decode coefficients
        let decoder = CoefficientDecoder::new(header.clone(), scan_data);
        let coefficients = decoder.decode()?;

        Ok((header, coefficients))
    }

    /// Encode DCT coefficients back to JPEG
    pub fn encode_coefficients(
        header: &JpegHeader,
        coefficients: &std::collections::HashMap<u8, Vec<[i16; 64]>>,
    ) -> Result<Vec<u8>> {
        let encoder = CoefficientEncoder::new(header.clone());
        let scan_data = encoder.encode(coefficients)?;

        // Assemble final JPEG
        Self::assemble_jpeg(header, &scan_data)
    }

    /// Assemble JPEG from header and scan data
    fn assemble_jpeg(header: &JpegHeader, scan_data: &[u8]) -> Result<Vec<u8>> {
        let mut output = Vec::new();

        // SOI
        output.push(0xFF);
        output.push(0xD8);

        // APP0 (if present)
        if let Some(ref app0) = header.app0_marker {
            output.push(0xFF);
            output.push(0xE0);
            let len = (app0.len() + 2) as u16;
            output.push((len >> 8) as u8);
            output.push((len & 0xFF) as u8);
            output.extend_from_slice(app0);
        }

        // APP1 (EXIF, XMP, ICC profiles)
        for app1 in &header.app1_markers {
            output.push(0xFF);
            output.push(0xE1);
            let len = (app1.len() + 2) as u16;
            output.push((len >> 8) as u8);
            output.push((len & 0xFF) as u8);
            output.extend_from_slice(app1);
        }

        // COM - Comment markers (preserved from original)
        for com in &header.com_markers {
            output.push(0xFF);
            output.push(0xFE);
            let len = (com.len() + 2) as u16;
            output.push((len >> 8) as u8);
            output.push((len & 0xFF) as u8);
            output.extend_from_slice(com);
        }

        // DQT - Quantization tables
        for table in header.quantization_tables.iter().flatten() {
            output.push(0xFF);
            output.push(0xDB);

            let table_data_len = if table.precision == 16 { 129 } else { 65 };
            let total_len = table_data_len + 2;
            output.push((total_len >> 8) as u8);
            output.push((total_len & 0xFF) as u8);

            // Precision: 0 = 8-bit, 1 = 16-bit
            let precision_bit = if table.precision == 16 { 1 } else { 0 };
            let table_info = (precision_bit << 4) | table.table_id;
            output.push(table_info);

            if table.precision == 8 {
                for &val in &table.values {
                    debug_assert!(
                        val <= 255,
                        "Quantization value exceeds 8-bit range: {}",
                        val
                    );
                    output.push(val as u8);
                }
            } else {
                for &val in &table.values {
                    output.push((val >> 8) as u8);
                    output.push((val & 0xFF) as u8);
                }
            }
        }

        // SOF0 - Start of Frame
        output.push(0xFF);
        output.push(0xC0);
        // Length = 8 + 3 * num_components (includes the 2-byte length field itself)
        let sof_len = 8 + header.components.len() as u16 * 3;
        output.push((sof_len >> 8) as u8);
        output.push((sof_len & 0xFF) as u8);
        output.push(header.precision);
        output.push((header.height >> 8) as u8);
        output.push((header.height & 0xFF) as u8);
        output.push((header.width >> 8) as u8);
        output.push((header.width & 0xFF) as u8);
        output.push(header.components.len() as u8);

        for comp in &header.components {
            output.push(comp.component_id);
            output.push((comp.h_sampling << 4) | comp.v_sampling);
            output.push(comp.quant_table_id);
        }

        // DHT - Huffman tables
        for t in header.huffman_tables_dc.iter().flatten() {
            Self::write_huffman_table(&mut output, 0, t.table_id, t)?;
        }
        for t in header.huffman_tables_ac.iter().flatten() {
            Self::write_huffman_table(&mut output, 1, t.table_id, t)?;
        }

        // SOS - Start of Scan
        output.push(0xFF);
        output.push(0xDA);
        let sos_len = 6 + header.components.len() * 2;
        output.push((sos_len >> 8) as u8);
        output.push((sos_len & 0xFF) as u8);
        output.push(header.components.len() as u8);

        for comp in &header.components {
            output.push(comp.component_id);
            output.push((comp.dc_table_id << 4) | comp.ac_table_id);
        }

        // Spectral selection (baseline)
        output.push(0); // Ss
        output.push(63); // Se
        output.push(0); // Ah/Al

        // Scan data
        output.extend_from_slice(scan_data);

        // EOI
        output.push(0xFF);
        output.push(0xD9);

        Ok(output)
    }

    fn write_huffman_table(
        output: &mut Vec<u8>,
        class: u8,
        id: u8,
        table: &HuffmanTable,
    ) -> Result<()> {
        output.push(0xFF);
        output.push(0xC4);

        // Calculate length: 2 (length field itself) + 1 (info byte) + 16 (counts) + values
        let mut len = 19;
        for &c in &table.counts {
            len += c as usize;
        }

        output.push((len >> 8) as u8);
        output.push((len & 0xFF) as u8);

        output.push((class << 4) | id);

        for &c in &table.counts {
            output.push(c as u8);
        }

        output.extend_from_slice(&table.values);

        Ok(())
    }
}

mod scan_utils {
    pub fn get_scan_data_start(data: &[u8]) -> Option<usize> {
        // Properly walk JPEG markers to find SOS (0xFFDA).
        // Skips marker segments using their declared lengths to avoid
        // false positives from 0xFF bytes in entropy-coded data.
        let mut pos = 2; // skip SOI
        while pos + 4 <= data.len() {
            if data[pos] != 0xFF {
                pos += 1;
                continue;
            }

            let marker = data[pos + 1];

            // SOS marker found — scan data starts after the SOS segment header
            if marker == 0xDA {
                let len = ((data[pos + 2] as usize) << 8) | (data[pos + 3] as usize);
                return Some(pos + 2 + len);
            }

            // Standalone markers (no length field): RSTm (0xD0-0xD7), SOI (0xD8), EOI (0xD9)
            if marker == 0xD9 {
                return None; // EOI before SOS — no scan data
            }
            if (0xD0..=0xD7).contains(&marker) || marker == 0xD8 {
                pos += 2;
                continue;
            }

            // Stuffed byte (0xFF 0x00) in entropy data — skip the pair
            if marker == 0x00 {
                pos += 2;
                continue;
            }

            // All other markers have a 2-byte length field after the marker byte
            if pos + 4 > data.len() {
                return None;
            }
            let seg_len = ((data[pos + 2] as usize) << 8) | (data[pos + 3] as usize);
            let next_pos = pos.checked_add(2).and_then(|p| p.checked_add(seg_len))?;
            if next_pos > data.len() {
                return None;
            }
            pos = next_pos;
        }
        None
    }
}

/// Check if JPEG bytes represent a progressive-encoded image.
///
/// Parses the JPEG header to detect progressive mode. Returns `false`
/// for invalid or non-JPEG input.
pub fn is_progressive_jpeg(jpeg_data: &[u8]) -> bool {
    JpegHeader::parse(jpeg_data)
        .map(|h| h.is_progressive)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_progressive_non_jpeg_returns_false() {
        assert!(!is_progressive_jpeg(&[]));
        assert!(!is_progressive_jpeg(&[0x89, 0x50, 0x4E, 0x47]));
        assert!(!is_progressive_jpeg(&[0xFF, 0xD8]));
    }

    #[test]
    fn get_scan_data_start_empty_returns_none() {
        assert!(scan_utils::get_scan_data_start(&[]).is_none());
    }

    #[test]
    fn get_scan_data_start_truncated_returns_none() {
        // SOI + partial marker
        assert!(scan_utils::get_scan_data_start(&[0xFF, 0xD8, 0xFF]).is_none());
    }

    #[test]
    fn assemble_jpeg_with_debug_assert() {
        // Verify the assemble path produces valid output with debug assertions
        let header = JpegHeader::default();
        let scan_data = Vec::new();
        let result = JpegTranscoder::assemble_jpeg(&header, &scan_data);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(bytes.starts_with(&[0xFF, 0xD8]));
        assert!(bytes.ends_with(&[0xFF, 0xD9]));
    }
}
