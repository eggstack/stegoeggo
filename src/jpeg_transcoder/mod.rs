//! JPEG DCT Transcoder for steganography
//!
//! This module provides JPEG transcoding capabilities that preserve DCT coefficients,
//! allowing steganographic embedding that survives re-encoding.

#[allow(dead_code)]
pub mod entropy;
#[allow(dead_code)]
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

    #[error("Extraction failed: {0}")]
    #[allow(dead_code)]
    ExtractionFailed(String),
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

        // APP1 (XMP/exif metadata) is intentionally omitted here.
        // The protection pipeline injects metadata in a separate step after DCT stego,
        // so stripping it during reassembly is correct.

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
        let mut len = 17;
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
        // Find SOS marker - skip to after SOI first
        let mut pos = 2;
        while pos + 1 < data.len() {
            if data[pos] == 0xFF && data[pos + 1] == 0xDA && pos + 3 < data.len() {
                let len = ((data[pos + 2] as usize) << 8) | (data[pos + 3] as usize);
                return Some(pos + 2 + len);
            }
            // Skip 0xFF bytes to avoid false positives
            if data[pos] == 0xFF {
                pos += 2;
            } else {
                pos += 1;
            }
        }
        None
    }
}

/// Check if JPEG is progressive
pub fn is_progressive_jpeg(jpeg_data: &[u8]) -> bool {
    if !jpeg_data.starts_with(&[0xFF, 0xD8]) {
        return false;
    }

    let mut pos = 2;
    while pos + 1 < jpeg_data.len() {
        if jpeg_data[pos] != 0xFF {
            pos += 1;
            continue;
        }

        let marker = jpeg_data[pos + 1];

        // SOF2 = Progressive JPEG
        if marker == 0xC2 {
            return true;
        }

        if marker == 0xD9 || marker == 0xDA {
            break;
        }

        if pos + 3 >= jpeg_data.len() {
            break;
        }

        let len = ((jpeg_data[pos + 2] as usize) << 8) | (jpeg_data[pos + 3] as usize);
        pos += 2 + len;
    }

    false
}
