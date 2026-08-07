//! JPEG Header Parsing
//!
//! Parses JPEG file headers to extract quantization tables, Huffman tables,
//! and other metadata needed for transcoding.
#![allow(dead_code)] // JPEG spec reference types (color spaces, coding processes, lookup methods)

use super::{Result, TranscoderError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpegCodingProcess {
    SequentialDCT,
    ProgressiveDCT,
    Lossless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum JpegColorSpace {
    Grayscale,
    YCbCr,
    RGB,
    CMYK,
    YCCK,
}

#[derive(Debug, Clone, Copy)]
pub struct QuantizationTable {
    pub table_id: u8,
    pub precision: u8, // 0 = 8-bit, 1 = 16-bit
    pub values: [u16; 64],
}

impl QuantizationTable {
    pub fn get(&self, index: usize) -> u16 {
        if index < 64 {
            self.values[index]
        } else {
            0
        }
    }

    pub fn scaled(&self, scale: f32) -> [u16; 64] {
        let mut result = [0u16; 64];
        for (i, &val) in self.values.iter().enumerate() {
            result[i] = ((val as f32 * scale).round() as u16).max(1);
        }
        result
    }
}

#[derive(Debug, Clone)]
pub struct HuffmanTable {
    pub table_class: u8, // 0 = DC, 1 = AC
    pub table_id: u8,
    pub counts: [u16; 16],
    pub values: Vec<u8>,
}

impl HuffmanTable {
    pub fn is_dc(&self) -> bool {
        self.table_class == 0
    }

    pub fn is_ac(&self) -> bool {
        self.table_class == 1
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScanComponent {
    pub component_id: u8,
    pub h_sampling: u8,
    pub v_sampling: u8,
    pub quant_table_id: u8,
    pub dc_table_id: u8,
    pub ac_table_id: u8,
}

#[derive(Debug, Clone)]
pub struct JpegScanSpan {
    pub sos_marker_offset: usize,
    pub sos_header_end: usize,
    pub entropy_start: usize,
    pub entropy_end: usize,
    pub terminating_marker_offset: usize,
    pub terminating_marker: u8,
}

#[derive(Debug, Clone)]
pub struct JpegStructure {
    pub scan_count: usize,
    pub has_restart_markers: bool,
    pub has_trailing_segments_after_scan: bool,
    pub eoi_offset: Option<usize>,
    pub scan_spans: Vec<JpegScanSpan>,
}

#[derive(Debug, Clone, Copy)]
struct MarkerRun {
    run_start: usize,
    marker_code_offset: usize,
    marker: u8,
}

#[derive(Debug, Clone)]
pub struct JpegHeader {
    pub width: u16,
    pub height: u16,
    pub precision: u8,
    pub coding_process: JpegCodingProcess,
    pub color_space: JpegColorSpace,

    pub quantization_tables: [Option<QuantizationTable>; 4],
    pub huffman_tables_dc: Vec<Option<HuffmanTable>>,
    pub huffman_tables_ac: Vec<Option<HuffmanTable>>,

    pub components: Vec<ScanComponent>,

    pub app0_marker: Option<Vec<u8>>,
    pub app1_markers: Vec<Vec<u8>>,
    pub com_markers: Vec<Vec<u8>>,

    pub restart_interval: u16,

    pub is_progressive: bool,
}

impl Default for JpegHeader {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            precision: 8,
            coding_process: JpegCodingProcess::SequentialDCT,
            color_space: JpegColorSpace::YCbCr,
            quantization_tables: [None; 4],
            huffman_tables_dc: vec![None, None, None, None],
            huffman_tables_ac: vec![None, None, None, None],
            components: Vec::new(),
            app0_marker: None,
            app1_markers: Vec::new(),
            com_markers: Vec::new(),
            restart_interval: 0,
            is_progressive: false,
        }
    }
}

impl JpegHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Self::parse_with_limits(data, &crate::ResourceLimits::default())
    }

    pub fn parse_with_limits(data: &[u8], limits: &crate::ResourceLimits) -> Result<Self> {
        if data.len() < 2 {
            return Err(TranscoderError::InvalidFormat("Input too short".into()));
        }

        if data[0] != 0xFF || data[1] != 0xD8 {
            return Err(TranscoderError::InvalidFormat("No SOI marker found".into()));
        }

        if data.len() <= 2 {
            return Err(TranscoderError::InvalidFormat(
                "Truncated JPEG header".into(),
            ));
        }

        let mut header = JpegHeader::default();

        // Parse the outer JPEG stream sequentially from the leading SOI.
        // This avoids false positives from marker-like byte sequences inside
        // metadata payloads such as COM, APP1, or APP13 segments.
        let mut pos = 2;
        let mut largest_width = 0usize;
        let mut largest_height = 0usize;
        let mut segment_count: usize = 0;

        while pos < data.len() {
            // Find next marker - skip to 0xFF
            while pos < data.len() && data[pos] != 0xFF {
                pos += 1;
            }

            if pos >= data.len() {
                break;
            }

            if pos + 1 >= data.len() {
                return Err(TranscoderError::InvalidFormat(format!(
                    "Truncated marker at byte offset {}",
                    pos
                )));
            }

            let marker = data[pos + 1];

            // Handle stuffed zeros (0xFF 0x00)
            if marker == 0x00 {
                pos += 2;
                continue;
            }

            // RST markers (RST0-RST7: 0xFF 0xD0-0xFF 0xD7)
            if (0xD0..=0xD7).contains(&marker) {
                pos += 2;
                continue;
            }

            // Skip standalone 0xFF that's not a marker (shouldn't happen in valid JPEG, but handle it)
            if marker == 0xFF {
                pos += 1;
                continue;
            }

            //EOI - End of Image
            if marker == 0xD9 {
                break;
            }

            segment_count += 1;
            if segment_count > limits.max_jpeg_segments() {
                return Err(TranscoderError::InvalidFormat(format!(
                    "JPEG segment count {} exceeds limit {}",
                    segment_count,
                    limits.max_jpeg_segments()
                )));
            }

            // Get segment length
            if pos + 3 >= data.len() {
                return Err(TranscoderError::InvalidFormat(format!(
                    "Truncated segment length at byte offset {}",
                    pos
                )));
            }

            let segment_len = ((data[pos + 2] as usize) << 8) | (data[pos + 3] as usize);

            if segment_len > limits.max_jpeg_segment_bytes() {
                return Err(TranscoderError::InvalidFormat(format!(
                    "JPEG segment size {} exceeds limit {}",
                    segment_len,
                    limits.max_jpeg_segment_bytes()
                )));
            }

            let segment_data_start = pos + 4;
            let segment_data_end = (pos + 2 + segment_len)
                .min(data.len())
                .max(segment_data_start);

            let segment_data = &data[segment_data_start..segment_data_end];

            match marker {
                // APP0 (JFIF)
                0xE0 => {
                    header.app0_marker = Some(segment_data.to_vec());
                }
                // APP1 (EXIF, etc)
                0xE1 => {
                    header.app1_markers.push(segment_data.to_vec());
                }
                // DQT - Define Quantization Table
                0xDB => {
                    header.parse_dqt(segment_data)?;
                }
                // SOF0 - Start of Frame (baseline)
                0xC0 => {
                    // Parse all SOFs - we'll fix dimensions later
                    header.parse_sof(segment_data)?;
                    if (header.width as usize) * (header.height as usize)
                        > largest_width * largest_height
                    {
                        largest_width = header.width as usize;
                        largest_height = header.height as usize;
                    }
                }
                // SOF1 - Start of Frame (extended)
                0xC1 => {
                    header.parse_sof(segment_data)?;
                    if (header.width as usize) * (header.height as usize)
                        > largest_width * largest_height
                    {
                        largest_width = header.width as usize;
                        largest_height = header.height as usize;
                    }
                }
                // SOF2 - Start of Frame (progressive)
                0xC2 => {
                    header.parse_sof(segment_data)?;
                    header.is_progressive = true;
                    header.coding_process = JpegCodingProcess::ProgressiveDCT;
                    if (header.width as usize) * (header.height as usize)
                        > largest_width * largest_height
                    {
                        largest_width = header.width as usize;
                        largest_height = header.height as usize;
                    }
                }
                // DHT - Define Huffman Table
                0xC4 => {
                    header.parse_dht(segment_data)?;
                }
                // SOS - Start of Scan
                0xDA => {
                    header.parse_sos(segment_data)?;
                    // Stop parsing header - rest is scan data
                    break;
                }
                // COM - Comment
                0xFE => {
                    header.com_markers.push(segment_data.to_vec());
                }
                0xDD if segment_data.len() >= 2 => {
                    header.restart_interval =
                        ((segment_data[0] as u16) << 8) | (segment_data[1] as u16);
                }
                _ => {
                    // Unknown marker - skip
                }
            }

            pos += 2 + segment_len;
        }

        // Fix dimensions to use the largest SOF
        if largest_width > 0
            && (header.width as usize, header.height as usize) != (largest_width, largest_height)
        {
            header.width = largest_width as u16;
            header.height = largest_height as u16;
        }

        Ok(header)
    }

    fn parse_dqt(&mut self, data: &[u8]) -> Result<()> {
        let mut pos = 0;
        while pos + 64 < data.len() {
            let table_info = data[pos];
            let table_id = table_info & 0x0F;
            if table_id >= 4 {
                return Err(TranscoderError::InvalidFormat(format!(
                    "DQT table_id {} out of range (0-3)",
                    table_id
                )));
            }
            let precision = if (table_info & 0xF0) != 0 { 16 } else { 8 };

            let mut values = [0u16; 64];

            if precision == 8 {
                for i in 0..64 {
                    values[i] = data[pos + 1 + i] as u16;
                }
                pos += 65;
            } else {
                // 16-bit precision needs 128 bytes of table data
                if pos + 128 >= data.len() {
                    return Err(TranscoderError::InvalidFormat(
                        "Truncated 16-bit DQT segment".into(),
                    ));
                }
                for i in 0..64 {
                    values[i] =
                        ((data[pos + 1 + i * 2] as u16) << 8) | (data[pos + 2 + i * 2] as u16);
                }
                pos += 129;
            }

            let table = QuantizationTable {
                table_id,
                precision,
                values,
            };

            self.quantization_tables[table_id as usize] = Some(table);
        }
        Ok(())
    }

    fn parse_sof(&mut self, data: &[u8]) -> Result<()> {
        if data.len() < 6 {
            return Err(TranscoderError::InvalidFormat(
                "SOF segment too short".into(),
            ));
        }

        self.precision = data[0];
        self.height = ((data[1] as u16) << 8) | (data[2] as u16);
        self.width = ((data[3] as u16) << 8) | (data[4] as u16);

        let num_components = data[5] as usize;

        if data.len() < 6 + num_components * 3 {
            return Err(TranscoderError::InvalidFormat(
                "SOF segment too short for components".into(),
            ));
        }

        self.components.clear();
        for i in 0..num_components {
            let offset = 6 + i * 3;
            let component_id = data[offset];
            let sampling = data[offset + 1];
            let quant_table_id = data[offset + 2];

            let h_sampling = (sampling >> 4) & 0x0F;
            let v_sampling = sampling & 0x0F;

            if h_sampling == 0 || v_sampling == 0 {
                return Err(TranscoderError::InvalidFormat(format!(
                    "Component {} has zero sampling factor (h={}, v={}); \
                     JPEG spec requires 1-4",
                    component_id, h_sampling, v_sampling
                )));
            }
            if h_sampling > 4 || v_sampling > 4 {
                return Err(TranscoderError::InvalidFormat(format!(
                    "Component {} sampling factor exceeds JPEG maximum (h={}, v={})",
                    component_id, h_sampling, v_sampling
                )));
            }

            self.components.push(ScanComponent {
                component_id,
                h_sampling,
                v_sampling,
                quant_table_id,
                dc_table_id: 0,
                ac_table_id: 0,
            });
        }

        Ok(())
    }

    fn parse_dht(&mut self, data: &[u8]) -> Result<()> {
        let mut pos = 0;
        while pos + 17 < data.len() {
            let table_info = data[pos];
            let table_class = (table_info >> 4) & 0x0F;
            let table_id = table_info & 0x0F;

            if table_class > 1 {
                return Err(TranscoderError::InvalidFormat(format!(
                    "DHT table class {} invalid: only DC (0) and AC (1) are supported",
                    table_class
                )));
            }

            if table_id >= 4 {
                return Err(TranscoderError::InvalidFormat(format!(
                    "DHT table_id {} out of range (0-3)",
                    table_id
                )));
            }

            let mut counts = [0u16; 16];
            let mut total = 0u16;
            for i in 0..16 {
                counts[i] = data[pos + 1 + i] as u16;
                total = total
                    .checked_add(counts[i])
                    .ok_or_else(|| TranscoderError::InvalidFormat("DHT count overflow".into()))?;
            }

            if total > 256 {
                return Err(TranscoderError::InvalidFormat(format!(
                    "DHT table {} has {} values, exceeds maximum of 256",
                    table_id, total
                )));
            }

            let values_start = pos + 17;
            let values_end = values_start + total as usize;
            if values_end > data.len() {
                return Err(TranscoderError::InvalidFormat(
                    "Truncated DHT segment: not enough value bytes".into(),
                ));
            }
            let values = data[values_start..values_end].to_vec();

            Self::validate_huffman_table(&counts, &values, table_class, table_id)?;

            let table = HuffmanTable {
                table_class,
                table_id,
                counts,
                values,
            };

            if table_class == 0 {
                self.huffman_tables_dc[table_id as usize] = Some(table);
            } else {
                self.huffman_tables_ac[table_id as usize] = Some(table);
            }

            pos = values_end;
        }

        Ok(())
    }

    fn validate_huffman_table(
        counts: &[u16; 16],
        values: &[u8],
        table_class: u8,
        table_id: u8,
    ) -> Result<()> {
        let total: usize = counts.iter().map(|&c| c as usize).sum();
        if total != values.len() {
            return Err(TranscoderError::InvalidFormat(format!(
                "DHT table {} (class {}): count sum {} != value count {}",
                table_id,
                table_class,
                total,
                values.len()
            )));
        }

        if total == 0 {
            return Err(TranscoderError::InvalidFormat(format!(
                "DHT table {} (class {}): empty table",
                table_id, table_class
            )));
        }

        let mut seen = [false; 256];
        for &v in values {
            if seen[v as usize] {
                return Err(TranscoderError::InvalidFormat(format!(
                    "DHT table {} (class {}): duplicate symbol 0x{:02X}",
                    table_id, table_class, v
                )));
            }
            seen[v as usize] = true;
        }

        let mut available: i64 = 1;
        for &count in counts {
            available *= 2;
            available -= count as i64;
            if available < 0 {
                return Err(TranscoderError::InvalidFormat(format!(
                    "DHT table {} (class {}): code space oversubscribed",
                    table_id, table_class
                )));
            }
        }

        Ok(())
    }

    fn parse_sos(&mut self, data: &[u8]) -> Result<()> {
        if data.len() < 3 {
            return Err(TranscoderError::InvalidFormat(
                "SOS segment too short".into(),
            ));
        }

        let num_components = data[0] as usize;
        if data.len() < 4 + num_components * 2 {
            return Err(TranscoderError::InvalidFormat(
                "SOS segment too short for components".into(),
            ));
        }

        for i in 0..num_components {
            let component_id = data[1 + i * 2];
            let table_info = data[2 + i * 2];

            let dc_table_id = (table_info >> 4) & 0x0F;
            let ac_table_id = table_info & 0x0F;

            if dc_table_id > 3 || ac_table_id > 3 {
                return Err(TranscoderError::InvalidFormat(format!(
                    "SOS component {} references invalid Huffman table IDs (dc={}, ac={}); \
                     valid range is 0-3",
                    component_id, dc_table_id, ac_table_id
                )));
            }

            if let Some(comp) = self
                .components
                .iter_mut()
                .find(|c| c.component_id == component_id)
            {
                comp.dc_table_id = dc_table_id;
                comp.ac_table_id = ac_table_id;
            }
        }

        Ok(())
    }

    fn checked_marker_run(data: &[u8], run_start: usize) -> Result<MarkerRun> {
        if data.get(run_start) != Some(&0xFF) {
            return Err(TranscoderError::InvalidFormat(format!(
                "Expected marker run at byte offset {}",
                run_start
            )));
        }

        let first_marker_offset = run_start
            .checked_add(1)
            .ok_or_else(|| TranscoderError::InvalidFormat("Marker run offset overflow".into()))?;
        let mut marker_code_offset = first_marker_offset;
        while data.get(marker_code_offset) == Some(&0xFF) {
            marker_code_offset = marker_code_offset.checked_add(1).ok_or_else(|| {
                TranscoderError::InvalidFormat("Marker run offset overflow".into())
            })?;
        }

        let marker = *data.get(marker_code_offset).ok_or_else(|| {
            TranscoderError::InvalidFormat(format!(
                "Truncated marker run at byte offset {}",
                run_start
            ))
        })?;

        if marker == 0x00 && marker_code_offset != first_marker_offset {
            return Err(TranscoderError::InvalidFormat(
                "Multiple FF bytes before stuffed zero".into(),
            ));
        }

        Ok(MarkerRun {
            run_start,
            marker_code_offset,
            marker,
        })
    }

    fn checked_segment_end(data: &[u8], marker_code_offset: usize) -> Result<usize> {
        let length_start = marker_code_offset.checked_add(1).ok_or_else(|| {
            TranscoderError::InvalidFormat("JPEG segment length offset overflow".into())
        })?;
        let length_end = length_start.checked_add(2).ok_or_else(|| {
            TranscoderError::InvalidFormat("JPEG segment length offset overflow".into())
        })?;
        if length_end > data.len() {
            return Err(TranscoderError::InvalidFormat(format!(
                "Truncated segment length at byte offset {}",
                marker_code_offset
            )));
        }

        let segment_len = u16::from_be_bytes([data[length_start], data[length_start + 1]]) as usize;
        if segment_len < 2 {
            return Err(TranscoderError::InvalidFormat(format!(
                "JPEG segment at byte offset {} has invalid length {}",
                marker_code_offset, segment_len
            )));
        }

        let segment_end = length_start
            .checked_add(segment_len)
            .ok_or_else(|| TranscoderError::InvalidFormat("JPEG segment extent overflow".into()))?;
        if segment_end > data.len() {
            return Err(TranscoderError::InvalidFormat(format!(
                "JPEG segment at byte offset {} extends beyond input",
                marker_code_offset
            )));
        }

        Ok(segment_end)
    }

    /// Analyze the complete scan structure of the JPEG without decoding entropy.
    ///
    /// This compatibility wrapper returns an empty structure for malformed input.
    /// Supported-path decisions use [`Self::analyze_structure_checked`] so malformed
    /// input cannot be mistaken for a valid or partially analyzed JPEG.
    pub fn analyze_structure(data: &[u8]) -> JpegStructure {
        Self::analyze_structure_checked(data).unwrap_or_else(|_| JpegStructure {
            scan_count: 0,
            has_restart_markers: false,
            has_trailing_segments_after_scan: false,
            eoi_offset: None,
            scan_spans: Vec::new(),
        })
    }

    /// Analyze JPEG marker and entropy structure with checked bounds handling.
    pub(crate) fn analyze_structure_checked(data: &[u8]) -> Result<JpegStructure> {
        if data.len() < 2 {
            return Err(TranscoderError::InvalidFormat(
                "JPEG input is shorter than SOI".into(),
            ));
        }
        if data[0] != 0xFF || data[1] != 0xD8 {
            return Err(TranscoderError::InvalidFormat("No SOI marker found".into()));
        }

        let mut structure = JpegStructure {
            scan_count: 0,
            has_restart_markers: false,
            has_trailing_segments_after_scan: false,
            eoi_offset: None,
            scan_spans: Vec::new(),
        };
        let mut pos = 2;
        let mut current_scan: Option<(usize, usize)> = None;

        while pos < data.len() {
            if let Some((sos_marker_offset, sos_header_end)) = current_scan {
                if data[pos] != 0xFF {
                    pos += 1;
                    continue;
                }

                if pos.checked_add(1).and_then(|next| data.get(next)) == Some(&0x00) {
                    pos += 2;
                    continue;
                }

                let run = Self::checked_marker_run(data, pos)?;
                if (0xD0..=0xD7).contains(&run.marker) {
                    structure.has_restart_markers = true;
                    pos = run.marker_code_offset.checked_add(1).ok_or_else(|| {
                        TranscoderError::InvalidFormat("Marker offset overflow".into())
                    })?;
                    continue;
                }

                if run.marker == 0xD9 {
                    structure.scan_spans.push(JpegScanSpan {
                        sos_marker_offset,
                        sos_header_end,
                        entropy_start: sos_header_end,
                        entropy_end: run.run_start,
                        terminating_marker_offset: run.run_start,
                        terminating_marker: run.marker,
                    });
                    structure.eoi_offset = Some(run.run_start);
                    current_scan = None;
                    break;
                }

                structure.scan_spans.push(JpegScanSpan {
                    sos_marker_offset,
                    sos_header_end,
                    entropy_start: sos_header_end,
                    entropy_end: run.run_start,
                    terminating_marker_offset: run.run_start,
                    terminating_marker: run.marker,
                });
                structure.has_trailing_segments_after_scan = true;
                current_scan = None;
                pos = run.run_start;
                continue;
            }

            if data[pos] != 0xFF {
                return Err(TranscoderError::InvalidFormat(format!(
                    "Unexpected data outside entropy at byte offset {}",
                    pos
                )));
            }

            let run = Self::checked_marker_run(data, pos)?;
            match run.marker {
                0x00 => {
                    return Err(TranscoderError::InvalidFormat(
                        "Stuffed zero outside entropy-coded scan".into(),
                    ));
                }
                0xD8 | 0xD0..=0xD7 | 0x01 => {
                    pos = run.marker_code_offset.checked_add(1).ok_or_else(|| {
                        TranscoderError::InvalidFormat("Marker offset overflow".into())
                    })?;
                }
                0xD9 => {
                    structure.eoi_offset = Some(run.run_start);
                    break;
                }
                0xDA => {
                    let segment_end = Self::checked_segment_end(data, run.marker_code_offset)?;
                    structure.scan_count += 1;
                    current_scan = Some((run.run_start, segment_end));
                    pos = segment_end;
                }
                _ => {
                    pos = Self::checked_segment_end(data, run.marker_code_offset)?;
                }
            }
        }

        if current_scan.is_some() {
            return Err(TranscoderError::InvalidFormat(
                "Entropy-coded scan has no terminating marker".into(),
            ));
        }

        Ok(structure)
    }

    /// Return the number of scans detected in the JPEG.
    pub fn scan_count(data: &[u8]) -> usize {
        JpegHeader::analyze_structure_checked(data)
            .map(|structure| structure.scan_count)
            .unwrap_or(0)
    }

    /// Check if the JPEG has a valid terminal EOI.
    pub fn has_valid_eoi(data: &[u8]) -> bool {
        JpegHeader::analyze_structure_checked(data)
            .map(|structure| structure.eoi_offset.is_some())
            .unwrap_or(false)
    }

    pub fn get_quantization_table(&self, id: u8) -> Option<&QuantizationTable> {
        self.quantization_tables
            .get(id as usize)
            .and_then(|t| t.as_ref())
    }

    pub fn get_dc_huffman_table(&self, id: u8) -> Option<&HuffmanTable> {
        self.huffman_tables_dc
            .get(id as usize)
            .and_then(|t| t.as_ref())
    }

    pub fn get_ac_huffman_table(&self, id: u8) -> Option<&HuffmanTable> {
        self.huffman_tables_ac
            .get(id as usize)
            .and_then(|t| t.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_input_returns_error() {
        let result = JpegHeader::parse(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_single_byte_returns_error() {
        let result = JpegHeader::parse(&[0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_tiny_data_returns_error() {
        let result = JpegHeader::parse(&[0xFF, 0xD8]);
        assert!(result.is_err());
    }

    #[test]
    fn com_markers_are_preserved_through_parse() {
        let mut data = vec![0xFF, 0xD8];

        let com_payload = b"Test comment";
        let com_len = (com_payload.len() + 2) as u16;
        data.extend_from_slice(&[0xFF, 0xFE]);
        data.extend_from_slice(&com_len.to_be_bytes());
        data.extend_from_slice(com_payload);

        data.extend_from_slice(&[0xFF, 0xD9]);

        let header = JpegHeader::parse(&data).unwrap();
        assert_eq!(header.com_markers.len(), 1);
        assert_eq!(header.com_markers[0], com_payload);
    }

    #[test]
    fn parse_ignores_marker_like_bytes_inside_metadata_payloads() {
        use crate::protected::metadata_trap::RightsMetadataProtector;
        use crate::types::{ImageOutputFormat, ProtectionContext};
        use image::DynamicImage;

        let img = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(32, 32, |x, y| {
            image::Rgb([(x * 3) as u8, (y * 5) as u8, ((x + y) * 7) as u8])
        }));

        let jpeg = crate::util::image::encode_image(&img, image::ImageFormat::Jpeg).unwrap();
        let ctx = ProtectionContext::new(0.5, 42).with_format(ImageOutputFormat::Jpeg);
        let injected = RightsMetadataProtector::new()
            .inject_bytes(&jpeg, &ctx)
            .unwrap();

        let header = JpegHeader::parse(&injected).unwrap();
        assert!(header.width > 0);
        assert!(header.height > 0);
        assert!(!header.quantization_tables.iter().all(|t| t.is_none()));
    }

    /// Regression test for a divide-by-zero panic discovered by the fuzz harness
    /// in `fuzz/fuzz_targets/pipeline_bytes.rs`. A SOF segment with a component
    /// whose sampling-factor nibble is 0 must be rejected at parse time, not
    /// panic later in the entropy decoder.
    #[test]
    fn parse_rejects_zero_sampling_factor() {
        // SOF0 + 1 component with h_sampling=0 (high nibble)
        let data: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0xC0, // SOF0
            0x00, 0x11, // segment length 17
            0x08, // precision 8
            0x00, 0x10, // height 16
            0x00, 0x10, // width 16
            0x01, // 1 component
            0x01, 0x00, 0x00, // id=1, sampling=0x00 (h=0, v=0), quant=0
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let result = JpegHeader::parse(data);
        assert!(
            result.is_err(),
            "SOF with zero sampling factor must be rejected"
        );
    }

    /// Regression test for SOS table IDs > 3 causing OOB panic in entropy
    /// decoder. The 4-bit field can encode 0-15 but only 0-3 are valid per
    /// the JPEG spec. Malformed IDs are now rejected.
    #[test]
    fn parse_sos_rejects_invalid_table_ids() {
        let data: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0xC0, // SOF0
            0x00, 0x11, // segment length 17
            0x08, // precision 8
            0x00, 0x10, // height 16
            0x00, 0x10, // width 16
            0x01, // 1 component
            0x01, 0x11, 0x00, // id=1, sampling=0x11 (h=1, v=1), quant=0
            // DHT - define DC table 0
            0xFF, 0xC4, 0x00, 0x1B, 0x00, // length=27, class=DC, id=0
            0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, // counts
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, // values
            // DHT - define AC table 0
            0xFF, 0xC4, 0x00, 0xB5, 0x10, // length=181, class=AC, id=0
            0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00,
            0x01, 0x7D, // counts
            0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51,
            0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42, 0xB1, 0xC1,
            0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16, 0x17, 0x18,
            0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39,
            0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57,
            0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75,
            0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x92,
            0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7,
            0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3,
            0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8,
            0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2,
            0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, // values
            // SOS with table IDs = 0x55 (dc=5, ac=5) - should be rejected
            0xFF, 0xDA, 0x00, 0x08, // SOS, length=8
            0x01, // 1 component
            0x01, 0x55, // component_id=1, table_info=0x55 (dc=5, ac=5)
            0x00, 0x3F, 0x00, // spectral selection, approx
        ];
        let result = JpegHeader::parse(data);
        assert!(
            result.is_err(),
            "SOS with table IDs > 3 must be rejected, not clamped"
        );
    }

    #[test]
    fn parse_rejects_oversized_sampling_factor() {
        // SOF0 + 1 component with h_sampling=5 (above JPEG max of 4)
        let data: &[u8] = &[
            0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01, 0x50,
            0x00, // id=1, sampling=0x50 (h=5, v=0)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let result = JpegHeader::parse(data);
        assert!(
            result.is_err(),
            "SOF with sampling factor > 4 must be rejected"
        );
    }

    #[test]
    fn analyze_structure_single_scan() {
        let data: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0xC0, 0x00, 0x0B, // SOF0, length=11
            0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01, 0x11,
            0x00, // precision, dims, 1 component
            0xFF, 0xC4, 0x00, 0x1F, 0x00, // DHT, length=31, class=DC id=0
            0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
            0xFF, 0xC4, 0x00, 0xB5, 0x10, // DHT, length=181, class=AC id=0
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
            0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xDA, 0x00,
            0x08, // SOS, length=8
            0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x7F, 0x80, // minimal scan data
            0xFF, 0xD9, // EOI
        ];
        let structure = JpegHeader::analyze_structure_checked(data).unwrap();
        assert_eq!(structure.scan_count, 1);
        assert!(!structure.has_restart_markers);
        assert!(!structure.has_trailing_segments_after_scan);
        assert!(structure.eoi_offset.is_some());
    }

    #[test]
    fn analyze_structure_missing_eoi() {
        let data: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0xC0, 0x00, 0x0B, // SOF0
            0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xDA, 0x00,
            0x08, // SOS
            0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x7F, 0x80, // scan data, no EOI
        ];
        assert!(JpegHeader::analyze_structure_checked(data).is_err());
    }

    #[test]
    fn parse_dht_rejects_class_2() {
        let data: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0xC4, 0x00, 0x1B, // DHT, length=27, class=2, id=0
            0x20, // table_info: class=2, id=0
            0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, // counts
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, // values
            0xFF, 0xD9, // EOI
        ];
        let result = JpegHeader::parse(data);
        assert!(result.is_err(), "DHT with class 2 must be rejected");
    }

    #[test]
    fn parse_dht_rejects_class_15() {
        let data: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0xC4, 0x00, 0x1B, // DHT, length=27, class=15, id=3
            0xF3, // table_info: class=15, id=3
            0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, // counts
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, // values
            0xFF, 0xD9, // EOI
        ];
        let result = JpegHeader::parse(data);
        assert!(result.is_err(), "DHT with class 15 must be rejected");
    }

    #[test]
    fn sos_marker_offset_points_to_sos() {
        let data: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0xDA, // SOS marker
            0x00, 0x08, // length 8
            0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x7F, 0x80, // entropy bytes
            0xFF, 0xD9, // EOI
        ];
        let structure = JpegHeader::analyze_structure_checked(data).unwrap();
        assert_eq!(structure.scan_count, 1);
        let span = &structure.scan_spans[0];
        assert_eq!(span.sos_marker_offset, 2);
    }

    #[test]
    fn entropy_start_equals_sos_header_end() {
        let data: &[u8] = &[
            0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x7F, 0x80,
            0xFF, 0xD9,
        ];
        let structure = JpegHeader::analyze_structure_checked(data).unwrap();
        let span = &structure.scan_spans[0];
        assert_eq!(span.entropy_start, span.sos_header_end);
    }

    fn structural_scan(entropy: &[u8], terminator: &[u8]) -> Vec<u8> {
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x04, 0x00, 0x00];
        data.extend_from_slice(entropy);
        data.extend_from_slice(terminator);
        data
    }

    #[test]
    fn checked_structure_rejects_truncated_ff_run() {
        assert!(JpegHeader::analyze_structure_checked(&[0xFF, 0xD8, 0xFF]).is_err());
        assert!(
            JpegHeader::analyze_structure_checked(&structural_scan(&[0x7F, 0xFF], &[])).is_err()
        );
    }

    #[test]
    fn checked_structure_rejects_segment_missing_length_bytes() {
        assert!(JpegHeader::analyze_structure_checked(&[0xFF, 0xD8, 0xFF, 0xE0]).is_err());
        assert!(JpegHeader::analyze_structure_checked(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00]).is_err());
    }

    #[test]
    fn checked_structure_rejects_segment_length_zero() {
        assert!(
            JpegHeader::analyze_structure_checked(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00]).is_err()
        );
    }

    #[test]
    fn checked_structure_rejects_segment_length_one() {
        assert!(
            JpegHeader::analyze_structure_checked(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x01]).is_err()
        );
    }

    #[test]
    fn checked_structure_rejects_segment_extending_past_input() {
        assert!(
            JpegHeader::analyze_structure_checked(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00])
                .is_err()
        );
    }

    #[test]
    fn checked_structure_rejects_truncated_sos() {
        assert!(
            JpegHeader::analyze_structure_checked(&[0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x04, 0x00])
                .is_err()
        );
    }

    #[test]
    fn checked_structure_rejects_entropy_without_terminator() {
        let data = structural_scan(&[0x7F], &[]);
        assert!(JpegHeader::analyze_structure_checked(&data).is_err());
    }

    #[test]
    fn sos_marker_offset_points_to_first_sos_ff_checked() {
        let data = vec![
            0xFF, 0xD8, 0xFF, 0xFF, 0xDA, 0x00, 0x04, 0x00, 0x00, 0x7F, 0xFF, 0xD9,
        ];
        let structure = JpegHeader::analyze_structure_checked(&data).unwrap();
        assert_eq!(structure.scan_spans[0].sos_marker_offset, 2);
    }

    #[test]
    fn entropy_start_equals_sos_header_end_checked() {
        let data = structural_scan(&[0x7F], &[0xFF, 0xD9]);
        let structure = JpegHeader::analyze_structure_checked(&data).unwrap();
        let span = &structure.scan_spans[0];
        assert_eq!(span.entropy_start, span.sos_header_end);
    }

    #[test]
    fn entropy_end_excludes_single_marker_prefix() {
        let data = structural_scan(&[0x7F], &[0xFF, 0xD9]);
        let structure = JpegHeader::analyze_structure_checked(&data).unwrap();
        let span = &structure.scan_spans[0];
        assert_eq!(span.entropy_end, data.len() - 2);
        assert_eq!(span.terminating_marker_offset, data.len() - 2);
    }

    #[test]
    fn entropy_end_excludes_all_repeated_marker_fill_bytes() {
        let data = structural_scan(&[0x7F], &[0xFF, 0xFF, 0xFF, 0xD9]);
        let structure = JpegHeader::analyze_structure_checked(&data).unwrap();
        let span = &structure.scan_spans[0];
        assert_eq!(span.entropy_end, data.len() - 4);
        assert_eq!(span.terminating_marker_offset, data.len() - 4);
    }

    #[test]
    fn terminating_marker_offset_points_to_first_fill_ff() {
        let data = structural_scan(&[0x7F], &[0xFF, 0xFF, 0xD9]);
        let structure = JpegHeader::analyze_structure_checked(&data).unwrap();
        let span = &structure.scan_spans[0];
        assert_eq!(span.terminating_marker_offset, span.entropy_end);
        assert_eq!(data[span.terminating_marker_offset], 0xFF);
    }

    #[test]
    fn stuffed_ff00_remains_inside_entropy() {
        let data = structural_scan(&[0x7F, 0xFF, 0x00, 0x3F], &[0xFF, 0xD9]);
        let structure = JpegHeader::analyze_structure_checked(&data).unwrap();
        let span = &structure.scan_spans[0];
        assert_eq!(
            &data[span.entropy_start..span.entropy_end],
            &[0x7F, 0xFF, 0x00, 0x3F]
        );
    }

    #[test]
    fn multiple_ff_before_00_is_rejected() {
        let data = structural_scan(&[0x7F, 0xFF, 0xFF, 0x00], &[0xFF, 0xD9]);
        assert!(JpegHeader::analyze_structure_checked(&data).is_err());
    }

    #[test]
    fn restart_marker_inside_scan_is_recorded() {
        let data = structural_scan(&[0x7F, 0xFF, 0xD0, 0x3F], &[0xFF, 0xD9]);
        let structure = JpegHeader::analyze_structure_checked(&data).unwrap();
        assert!(structure.has_restart_markers);
        assert_eq!(structure.scan_count, 1);
    }

    #[test]
    fn restart_marker_does_not_end_scan_structure() {
        let data = structural_scan(&[0x7F, 0xFF, 0xD0, 0x3F], &[0xFF, 0xD9]);
        let structure = JpegHeader::analyze_structure_checked(&data).unwrap();
        let span = &structure.scan_spans[0];
        assert_eq!(span.terminating_marker, 0xD9);
        assert_eq!(structure.eoi_offset, Some(span.terminating_marker_offset));
    }
}
