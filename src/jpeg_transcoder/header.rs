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
        self.values[index]
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
    pub app1_marker: Option<Vec<u8>>,

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
            app1_marker: None,
            restart_interval: 0,
            is_progressive: false,
        }
    }
}

impl JpegHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        // Simple approach: find the last SOI-EOI pair in the file
        // This handles files with embedded thumbnails

        if data.len() < 2 {
            return Err(TranscoderError::InvalidFormat("Input too short".into()));
        }

        // Find all SOI positions
        let soi_positions: Vec<usize> = (0..data.len() - 1)
            .filter(|&i| data[i] == 0xFF && data[i + 1] == 0xD8)
            .collect();

        // Find all EOI positions
        let eoi_positions: Vec<usize> = (0..data.len() - 1)
            .filter(|&i| data[i] == 0xFF && data[i + 1] == 0xD9)
            .collect();

        // Use the last SOI as start (usually main image)
        let start_pos = if !soi_positions.is_empty() {
            *soi_positions.last().unwrap()
        } else {
            return Err(TranscoderError::InvalidFormat("No SOI marker found".into()));
        };

        // Find the last EOI in the file (for the main image)
        let end_pos = if !eoi_positions.is_empty() {
            *eoi_positions.last().unwrap()
        } else {
            data.len()
        };

        let mut header = JpegHeader::default();

        // Parse from start_pos
        let mut pos = start_pos + 2;
        if pos >= data.len() {
            pos = start_pos;
        }

        // We'll also find the largest SOF to get dimensions
        let mut largest_sof_pos = 0;
        let mut largest_width = 0usize;
        let mut largest_height = 0usize;

        // Find largest SOF between start_pos and end_pos
        if end_pos < 10 {
            return Err(TranscoderError::InvalidFormat(
                "JPEG too short for SOF".into(),
            ));
        }
        let mut search_pos = start_pos;
        while search_pos < end_pos - 10 {
            if data[search_pos] == 0xFF
                && (data[search_pos + 1] == 0xC0
                    || data[search_pos + 1] == 0xC1
                    || data[search_pos + 1] == 0xC2)
            {
                let h = ((data[search_pos + 5] as usize) << 8) | (data[search_pos + 6] as usize);
                let w = ((data[search_pos + 7] as usize) << 8) | (data[search_pos + 8] as usize);
                if w * h > largest_width * largest_height {
                    largest_width = w;
                    largest_height = h;
                    largest_sof_pos = search_pos;
                }
            }
            search_pos += 1;
        }

        while pos < end_pos {
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

            // Get segment length
            if pos + 3 >= data.len() {
                return Err(TranscoderError::InvalidFormat(format!(
                    "Truncated segment length at byte offset {}",
                    pos
                )));
            }

            let segment_len = ((data[pos + 2] as usize) << 8) | (data[pos + 3] as usize);

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
                    header.app1_marker = Some(segment_data.to_vec());
                }
                // DQT - Define Quantization Table
                0xDB => {
                    header.parse_dqt(segment_data)?;
                }
                // SOF0 - Start of Frame (baseline)
                0xC0 => {
                    // Parse all SOFs - we'll fix dimensions later
                    header.parse_sof(segment_data)?;
                }
                // SOF1 - Start of Frame (extended)
                0xC1 => {
                    header.parse_sof(segment_data)?;
                }
                // SOF2 - Start of Frame (progressive)
                0xC2 => {
                    header.parse_sof(segment_data)?;
                    header.is_progressive = true;
                    header.coding_process = JpegCodingProcess::ProgressiveDCT;
                }
                // DHT - Define Huffman Table
                0xC4 => {
                    header.parse_dht(segment_data)?;
                }
                // SOS - Start of Scan
                0xDA => {
                    header.parse_sos(segment_data);
                    // Stop parsing header - rest is scan data
                    break;
                }
                // COM - Comment
                0xFE => {
                    // Skip comments
                }
                // Restart interval
                0xDD => {
                    if segment_data.len() >= 2 {
                        header.restart_interval =
                            ((segment_data[0] as u16) << 8) | (segment_data[1] as u16);
                    }
                }
                _ => {
                    // Unknown marker - skip
                }
            }

            pos = pos + 2 + segment_len;
        }

        // Fix dimensions to use the largest SOF
        if largest_sof_pos > 0
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

            self.components.push(ScanComponent {
                component_id,
                h_sampling: (sampling >> 4) & 0x0F,
                v_sampling: sampling & 0x0F,
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
                total += counts[i];
            }

            let values_start = pos + 17;
            let values_end = values_start + total as usize;
            if values_end > data.len() {
                return Err(TranscoderError::InvalidFormat(
                    "Truncated DHT segment: not enough value bytes".into(),
                ));
            }
            let values = data[values_start..values_end].to_vec();

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

    fn parse_sos(&mut self, data: &[u8]) {
        if data.len() < 3 {
            return;
        }

        let num_components = data[0] as usize;
        if data.len() < 4 + num_components * 2 {
            return;
        }

        for i in 0..num_components {
            let component_id = data[1 + i * 2];
            let table_info = data[2 + i * 2];

            // Update components with their Huffman table assignments
            if let Some(comp) = self
                .components
                .iter_mut()
                .find(|c| c.component_id == component_id)
            {
                comp.dc_table_id = (table_info >> 4) & 0x0F;
                comp.ac_table_id = table_info & 0x0F;
            }
        }
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
}
