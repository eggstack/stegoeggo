use crate::error::{Error, Result};
use crate::jpeg_transcoder::{DctStegoF5, JpegHeader};

pub(crate) fn dct_payload_capacity(coefficients: &crate::jpeg_transcoder::Coefficients) -> usize {
    coefficients
        .values()
        .flat_map(|blocks| blocks.iter())
        .map(|block| {
            block
                .iter()
                .skip(1)
                .filter(|&&coef| coef.abs() >= 2)
                .count()
        })
        .sum()
}

#[allow(dead_code)]
pub(crate) fn embed_seed_hint(jpeg_bytes: &[u8], seed: u64) -> Result<Vec<u8>> {
    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(Error::Steganography("Not a valid JPEG".to_string()));
    }
    let mut header = JpegHeader::parse(jpeg_bytes)?;
    DctStegoF5::new().embed_seed_in_quantization_tables(&mut header, seed)?;
    reassemble_jpeg_with_qtables(jpeg_bytes, &header)
}

#[allow(dead_code)]
pub(crate) fn extract_seed_hint(jpeg_bytes: &[u8]) -> Result<Option<u64>> {
    if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(Error::Steganography("Not a valid JPEG".to_string()));
    }
    let header = JpegHeader::parse(jpeg_bytes)?;
    Ok(DctStegoF5::new().extract_seed_from_quantization_tables(&header))
}

pub(crate) fn reassemble_jpeg_with_qtables(
    jpeg_bytes: &[u8],
    header: &JpegHeader,
) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(jpeg_bytes.len() + 256);
    output.extend_from_slice(&jpeg_bytes[0..2]);

    let mut pos = 2;
    let mut wrote_tables = false;

    while pos + 4 <= jpeg_bytes.len() {
        if jpeg_bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }

        let marker = jpeg_bytes[pos + 1];

        if marker == 0xDA || marker == 0xD9 {
            output.extend_from_slice(&jpeg_bytes[pos..]);
            break;
        }

        if marker == 0xDB {
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

        if marker == 0x00 {
            pos += 1;
            continue;
        }

        let segment_len = u16::from_be_bytes([jpeg_bytes[pos + 2], jpeg_bytes[pos + 3]]) as usize;
        if pos + 2 + segment_len > jpeg_bytes.len() {
            return Err(Error::Steganography(
                "Malformed JPEG segment length exceeds buffer".into(),
            ));
        }
        output.extend_from_slice(&jpeg_bytes[pos..pos + 2 + segment_len]);
        pos += 2 + segment_len;
    }

    Ok(output)
}
