//! JPEG DCT Transcoder for steganography
//!
//! This module provides JPEG transcoding capabilities that preserve DCT coefficients,
//! enabling steganographic embedding for byte-preserving JPEG fast paths.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DctSupport {
    Supported,
    Unsupported(DctUnsupportedReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DctUnsupportedReason {
    Progressive,
    MultipleScans,
    ArithmeticCoding,
    LosslessCoding,
    UnsupportedPrecision,
    RestartIntervals,
    UnsupportedColorProcess,
    MissingTables,
    MalformedHeader,
    TrailingSegmentsAfterScan,
}

impl std::fmt::Display for DctUnsupportedReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Progressive => write!(f, "progressive JPEG"),
            Self::MultipleScans => write!(f, "multiple scans"),
            Self::ArithmeticCoding => write!(f, "arithmetic coding"),
            Self::LosslessCoding => write!(f, "lossless coding"),
            Self::UnsupportedPrecision => write!(f, "unsupported precision"),
            Self::RestartIntervals => write!(f, "restart intervals"),
            Self::UnsupportedColorProcess => write!(f, "unsupported color process"),
            Self::MissingTables => write!(f, "missing Huffman tables"),
            Self::MalformedHeader => write!(f, "malformed header"),
            Self::TrailingSegmentsAfterScan => write!(f, "trailing segments after scan"),
        }
    }
}

pub fn probe_dct_support(header: &JpegHeader) -> DctSupport {
    if header.is_progressive {
        return DctSupport::Unsupported(DctUnsupportedReason::Progressive);
    }

    if header.precision != 8 {
        return DctSupport::Unsupported(DctUnsupportedReason::UnsupportedPrecision);
    }

    match header.coding_process {
        header::JpegCodingProcess::ProgressiveDCT => {
            return DctSupport::Unsupported(DctUnsupportedReason::Progressive);
        }
        header::JpegCodingProcess::Lossless => {
            return DctSupport::Unsupported(DctUnsupportedReason::LosslessCoding);
        }
        header::JpegCodingProcess::SequentialDCT => {}
    }

    if header.restart_interval > 0 {
        return DctSupport::Unsupported(DctUnsupportedReason::RestartIntervals);
    }

    if header.components.is_empty() {
        return DctSupport::Unsupported(DctUnsupportedReason::MalformedHeader);
    }

    for comp in &header.components {
        let has_dc = header.get_dc_huffman_table(comp.dc_table_id).is_some();
        let has_ac = header.get_ac_huffman_table(comp.ac_table_id).is_some();
        if !has_dc || !has_ac {
            return DctSupport::Unsupported(DctUnsupportedReason::MissingTables);
        }
    }

    let max_h = header
        .components
        .iter()
        .map(|c| c.h_sampling)
        .max()
        .unwrap_or(1);
    let max_v = header
        .components
        .iter()
        .map(|c| c.v_sampling)
        .max()
        .unwrap_or(1);

    if max_h > 4 || max_v > 4 {
        return DctSupport::Unsupported(DctUnsupportedReason::UnsupportedColorProcess);
    }

    DctSupport::Supported
}

/// Full DCT support probe including structural scan analysis.
///
/// Checks header properties plus scan count, EOI validity, and trailing
/// segment presence. Accepts raw JPEG bytes for structural analysis.
#[allow(dead_code)]
pub fn probe_dct_support_full(header: &JpegHeader, jpeg_data: &[u8]) -> DctSupport {
    match checked_supported_structure(header, jpeg_data) {
        Ok(_) => DctSupport::Supported,
        Err(reason) => DctSupport::Unsupported(reason),
    }
}

fn checked_supported_structure(
    header: &JpegHeader,
    jpeg_data: &[u8],
) -> std::result::Result<header::JpegStructure, DctUnsupportedReason> {
    if let DctSupport::Unsupported(reason) = probe_dct_support(header) {
        return Err(reason);
    }

    let structure = JpegHeader::analyze_structure_checked(jpeg_data)
        .map_err(|_| DctUnsupportedReason::MalformedHeader)?;

    if structure.scan_count != 1 {
        return Err(DctUnsupportedReason::MultipleScans);
    }

    if structure.eoi_offset.is_none() {
        return Err(DctUnsupportedReason::MalformedHeader);
    }

    if structure.has_restart_markers {
        return Err(DctUnsupportedReason::RestartIntervals);
    }

    if structure.has_trailing_segments_after_scan {
        return Err(DctUnsupportedReason::TrailingSegmentsAfterScan);
    }

    Ok(structure)
}

/// Main JPEG DCT Transcoder
/// Provides lossless JPEG transcoding that preserves DCT coefficients
pub struct JpegTranscoder;

impl JpegTranscoder {
    /// Decode JPEG and extract raw DCT coefficients
    pub fn decode_coefficients(jpeg_data: &[u8]) -> Result<(JpegHeader, Coefficients)> {
        let header = JpegHeader::parse(jpeg_data)?;

        let structure = checked_supported_structure(&header, jpeg_data).map_err(|reason| {
            TranscoderError::Unsupported(format!(
                "DCT embedding not supported for {}: {}",
                reason, reason
            ))
        })?;

        let scan_span = structure.scan_spans.first().ok_or_else(|| {
            TranscoderError::InvalidFormat("No scan span found in supported JPEG".into())
        })?;

        let entropy_slice = &jpeg_data[scan_span.entropy_start..scan_span.entropy_end];

        let decoder = CoefficientDecoder::new(header.clone(), entropy_slice.to_vec());
        let coefficients = decoder.decode()?;

        Ok((header, coefficients))
    }

    /// Encode DCT coefficients back to JPEG using the original byte stream
    /// for container preservation. Falls back to assemble_jpeg if no original
    /// is provided.
    pub fn encode_coefficients(
        header: &JpegHeader,
        coefficients: &std::collections::HashMap<u8, Vec<[i16; 64]>>,
        original_jpeg: Option<&[u8]>,
    ) -> Result<Vec<u8>> {
        match original_jpeg {
            Some(orig) => Self::encode_coefficients_preserving(header, coefficients, orig),
            None => {
                let encoder = CoefficientEncoder::new(header.clone());
                let scan_data = encoder.encode(coefficients)?;
                Self::assemble_jpeg(header, &scan_data)
            }
        }
    }

    /// Assemble JPEG from header and scan data.
    ///
    /// This rebuilds the JPEG from parsed fields. It preserves APP0, APP1,
    /// and COM markers stored in the header but drops unknown segments
    /// (APP2, APP3, ..., APP13, APP14, DRI, etc.). Use
    /// [`encode_coefficients_preserving`] for container-preserving output.
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

    /// Encode DCT coefficients back to JPEG, preserving the original container.
    ///
    /// Walks the original byte stream, replacing only DQT markers (with the
    /// modified quantization tables) and the SOS entropy-coded scan data (with
    /// the newly encoded scan bytes). All other segments — APP0, APP1, APP2,
    /// APP13, APP14, COM, DRI, unknown APP markers — are copied verbatim in
    /// their original order.
    pub fn encode_coefficients_preserving(
        header: &JpegHeader,
        coefficients: &Coefficients,
        original_jpeg: &[u8],
    ) -> Result<Vec<u8>> {
        let encoder = CoefficientEncoder::new(header.clone());
        let scan_data = encoder.encode(coefficients)?;

        let mut output = Vec::with_capacity(original_jpeg.len() + scan_data.len());
        output.extend_from_slice(&original_jpeg[0..2]); // SOI

        let mut pos = 2;
        let mut wrote_qtables = false;

        while pos + 2 <= original_jpeg.len() {
            if original_jpeg[pos] != 0xFF {
                pos += 1;
                continue;
            }

            let marker = original_jpeg[pos + 1];

            if marker == 0xDA {
                // Copy the original SOS header verbatim, replace scan data, append EOI.
                // The SOS header includes component table assignments and spectral
                // selection which must match the header state.
                let orig_seg_len =
                    u16::from_be_bytes([original_jpeg[pos + 2], original_jpeg[pos + 3]]) as usize;
                let sos_end = pos + 2 + orig_seg_len;
                if sos_end > original_jpeg.len() {
                    output.extend_from_slice(&original_jpeg[pos..]);
                    return Ok(output);
                }
                output.extend_from_slice(&original_jpeg[pos..sos_end]); // SOS marker + header
                output.extend_from_slice(&scan_data);
                output.push(0xFF);
                output.push(0xD9); // EOI
                return Ok(output);
            }

            if marker == 0xD9 {
                output.extend_from_slice(&original_jpeg[pos..]);
                return Ok(output);
            }

            if marker == 0x00 {
                pos += 2;
                continue;
            }

            if pos + 3 >= original_jpeg.len() {
                output.extend_from_slice(&original_jpeg[pos..]);
                return Ok(output);
            }

            let seg_len =
                u16::from_be_bytes([original_jpeg[pos + 2], original_jpeg[pos + 3]]) as usize;
            let seg_end = pos + 2 + seg_len;

            if seg_end > original_jpeg.len() {
                output.extend_from_slice(&original_jpeg[pos..]);
                return Ok(output);
            }

            if marker == 0xDB {
                pos = seg_end;
                if !wrote_qtables {
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
                    wrote_qtables = true;
                }
                continue;
            }

            output.extend_from_slice(&original_jpeg[pos..seg_end]);
            pos = seg_end;
        }

        output.extend_from_slice(&scan_data);
        output.push(0xFF);
        output.push(0xD9);
        Ok(output)
    }
}

mod scan_utils {
    #[allow(dead_code)]
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
                let scan_start = pos.checked_add(2).and_then(|p| p.checked_add(len))?;
                if scan_start > data.len() {
                    return None;
                }
                return Some(scan_start);
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
#[allow(dead_code)]
fn is_progressive_jpeg(jpeg_data: &[u8]) -> bool {
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

    /// Regression test for an out-of-bounds slice panic discovered by the fuzz
    /// harness. A JPEG with a SOS marker near the end of the buffer whose
    /// declared header length exceeds the buffer must return `None`, not panic.
    #[test]
    fn get_scan_data_start_oversized_sos_returns_none() {
        let malformed: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0xDA, // SOS
            0xFF, 0xFF, // SOS header length = 0xFFFF (way past buffer end)
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        assert!(scan_utils::get_scan_data_start(malformed).is_none());
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

    fn valid_structure_probe_header() -> JpegHeader {
        let mut header = JpegHeader::default();
        header.components.push(header::ScanComponent {
            component_id: 1,
            h_sampling: 1,
            v_sampling: 1,
            quant_table_id: 0,
            dc_table_id: 0,
            ac_table_id: 0,
        });
        let table = HuffmanTable {
            table_class: 0,
            table_id: 0,
            counts: [0; 16],
            values: Vec::new(),
        };
        header.huffman_tables_dc[0] = Some(table.clone());
        header.huffman_tables_ac[0] = Some(HuffmanTable {
            table_class: 1,
            ..table
        });
        header
    }

    fn minimal_structural_jpeg(entropy: &[u8], terminator: &[u8]) -> Vec<u8> {
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x04, 0x00, 0x00];
        data.extend_from_slice(entropy);
        data.extend_from_slice(terminator);
        data
    }

    #[test]
    fn probe_dct_support_full_rejects_restart_without_dri() {
        let header = valid_structure_probe_header();
        let data = minimal_structural_jpeg(&[0x7F, 0xFF, 0xD0, 0x3F], &[0xFF, 0xD9]);
        assert_eq!(
            probe_dct_support_full(&header, &data),
            DctSupport::Unsupported(DctUnsupportedReason::RestartIntervals)
        );
    }

    #[test]
    fn probe_dct_support_full_rejects_restart_with_dri() {
        let mut header = valid_structure_probe_header();
        header.restart_interval = 1;
        let data = minimal_structural_jpeg(&[0x7F], &[0xFF, 0xD9]);
        assert_eq!(
            probe_dct_support_full(&header, &data),
            DctSupport::Unsupported(DctUnsupportedReason::RestartIntervals)
        );
    }

    #[test]
    fn probe_dct_support_full_rejects_truncated_marker_run() {
        let header = valid_structure_probe_header();
        assert_eq!(
            probe_dct_support_full(&header, &[0xFF, 0xD8, 0xFF]),
            DctSupport::Unsupported(DctUnsupportedReason::MalformedHeader)
        );
    }

    #[test]
    fn probe_dct_support_full_rejects_short_segment() {
        let header = valid_structure_probe_header();
        assert_eq!(
            probe_dct_support_full(&header, &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x01]),
            DctSupport::Unsupported(DctUnsupportedReason::MalformedHeader)
        );
    }

    #[test]
    fn probe_dct_support_full_rejects_post_scan_segment() {
        let header = valid_structure_probe_header();
        let data = minimal_structural_jpeg(&[0x7F], &[0xFF, 0xC0, 0x00, 0x02, 0xFF, 0xD9]);
        assert_eq!(
            probe_dct_support_full(&header, &data),
            DctSupport::Unsupported(DctUnsupportedReason::TrailingSegmentsAfterScan)
        );
    }

    #[test]
    fn probe_dct_support_full_accepts_supported_baseline_structure() {
        let header = valid_structure_probe_header();
        let data = minimal_structural_jpeg(&[0x7F], &[0xFF, 0xD9]);
        assert_eq!(
            probe_dct_support_full(&header, &data),
            DctSupport::Supported
        );
    }

    #[test]
    fn probe_dct_support_full_rejects_multi_scan() {
        let mut data = vec![0xFF, 0xD8]; // SOI
        let sof = [
            0xFF, 0xC0, 0x00, 0x0B, // SOF0, length=11
            0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01, 0x11, 0x00,
        ];
        data.extend_from_slice(&sof);
        let dht = [
            0xFF, 0xC4, 0x00, 0x1F, 0x00, // DHT DC table 0
            0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
        ];
        data.extend_from_slice(&dht);
        let dht_ac = [
            0xFF, 0xC4, 0x00, 0xB5, 0x10, // DHT AC table 0
            0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00,
            0x01, 0x7D, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
            0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42,
            0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16,
            0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37,
            0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55,
            0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73,
            0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
            0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5,
            0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA,
            0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6,
            0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA,
            0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA,
        ];
        data.extend_from_slice(&dht_ac);

        // First SOS
        let sos1 = [
            0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x7F, 0x80,
        ];
        data.extend_from_slice(&sos1);

        // Second SOS (multi-scan)
        let sos2 = [
            0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x7F, 0x80,
        ];
        data.extend_from_slice(&sos2);

        data.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let header = JpegHeader::parse(&data).unwrap();
        let support = probe_dct_support_full(&header, &data);
        assert_eq!(
            support,
            DctSupport::Unsupported(DctUnsupportedReason::MultipleScans)
        );
    }

    #[test]
    fn probe_dct_support_full_rejects_missing_eoi() {
        let mut data = vec![0xFF, 0xD8]; // SOI
        let sof = [
            0xFF, 0xC0, 0x00, 0x0B, // SOF0, length=11
            0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01, 0x11, 0x00,
        ];
        data.extend_from_slice(&sof);
        let dht = [
            0xFF, 0xC4, 0x00, 0x1F, 0x00, // DHT DC table 0
            0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
        ];
        data.extend_from_slice(&dht);
        let dht_ac = [
            0xFF, 0xC4, 0x00, 0xB5, 0x10, // DHT AC table 0
            0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00,
            0x01, 0x7D, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
            0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42,
            0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16,
            0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37,
            0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55,
            0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73,
            0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
            0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5,
            0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA,
            0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6,
            0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA,
            0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA,
        ];
        data.extend_from_slice(&dht_ac);

        // SOS
        let sos = [0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00];
        data.extend_from_slice(&sos);

        // scan data, no EOI
        data.extend_from_slice(&[0x7F, 0x80]);

        let header = JpegHeader::parse(&data).unwrap();
        let support = probe_dct_support_full(&header, &data);
        assert_eq!(
            support,
            DctSupport::Unsupported(DctUnsupportedReason::MalformedHeader)
        );
    }
}
