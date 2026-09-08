use crate::error::{Error, Result};
use crate::types::DmiValue;

/// All PNG `tEXt` and `iTXt` keywords owned by StegoEggo.
///
/// Shared by `png_has_stego_metadata`, `strip_stego_owned_png`, and
/// `collect_stego_owned_png_keys` to ensure the three sites agree on what
/// counts as StegoEggo-owned metadata. Includes the `XML:com.adobe.xmp`
/// XMP iTXt keyword (BUG-02).
const STEGO_OWNED_PNG_KEYS: &[&[u8]] = &[
    b"X-Protection-Seed",
    b"DMI-PROHIBITED",
    b"XML:com.adobe.xmp",
    b"noai",
    b"Copyright",
    b"Contact",
    b"License",
    b"UsageTerms",
    b"DateCreated",
    b"AIConstraints",
    b"WebStatementOfRights",
    b"Creator",
    b"CreditLine",
    b"CopyrightOwner",
    b"LicensorName",
    b"LicensorEmail",
    b"LicensorURL",
    b"MetadataDate",
    b"NoticeAppliedAt",
];

impl super::RightsMetadataProtector {
    pub(super) fn png_has_stego_metadata(&self, png_data: &[u8]) -> bool {
        if png_data.len() < 8 || &png_data[0..8] != b"\x89PNG\r\n\x1a\n" {
            return false;
        }
        let mut pos = 8;
        while pos + 12 <= png_data.len() {
            let chunk_len = u32::from_be_bytes([
                png_data[pos],
                png_data[pos + 1],
                png_data[pos + 2],
                png_data[pos + 3],
            ]) as usize;
            let chunk_type = &png_data[pos + 4..pos + 8];

            if chunk_type == b"IEND" {
                break;
            }

            if chunk_type == b"tEXt" || chunk_type == b"iTXt" {
                let data_start = pos + 8;
                let Some(data_end) = data_start.checked_add(chunk_len) else {
                    break;
                };
                let data_end = data_end.min(png_data.len());
                let data = &png_data[data_start..data_end];
                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    let key = &data[..null_pos];
                    if Self::is_stego_owned_text_key(key) {
                        return true;
                    }
                }
            }

            let Some(chunk_end) = pos.checked_add(12).and_then(|p| p.checked_add(chunk_len)) else {
                break;
            };
            if chunk_end > png_data.len() {
                break;
            }
            pos = chunk_end;
        }
        false
    }

    pub(super) fn inject_text_chunks_png(
        &self,
        png_data: &[u8],
        metadata: &[(Vec<u8>, Vec<u8>)],
        dmi: Option<DmiValue>,
        seed: Option<u64>,
        limits: Option<&crate::ResourceLimits>,
    ) -> Result<Vec<u8>> {
        if metadata.is_empty() && dmi.is_none() {
            return Ok(png_data.to_vec());
        }

        if png_data.len() < 8 || &png_data[0..8] != b"\x89PNG\r\n\x1a\n" {
            return Err(Error::Metadata("Invalid PNG signature".to_string()));
        }

        let mut has_ihdr = false;
        let mut has_iend = false;
        {
            let mut p = 8;
            while p + 12 <= png_data.len() {
                let cl = u32::from_be_bytes([
                    png_data[p],
                    png_data[p + 1],
                    png_data[p + 2],
                    png_data[p + 3],
                ]) as usize;
                let ct = &png_data[p + 4..p + 8];
                if ct == b"IHDR" {
                    has_ihdr = true;
                }
                if ct == b"IEND" {
                    has_iend = true;
                    break;
                }
                let Some(end) = p.checked_add(12).and_then(|v| v.checked_add(cl)) else {
                    break;
                };
                if end > png_data.len() {
                    break;
                }
                p = end;
            }
        }
        if !has_ihdr {
            return Err(Error::Metadata(
                "PNG missing mandatory IHDR chunk".to_string(),
            ));
        }
        if !has_iend {
            return Err(Error::Metadata(
                "PNG missing mandatory IEND chunk".to_string(),
            ));
        }

        let mut output = Vec::with_capacity(png_data.len() + 1000 * metadata.len() + 500);
        output.extend_from_slice(&png_data[0..8]);

        let mut pos = 8;
        let mut chunk_count: usize = 0;

        while pos + 12 <= png_data.len() {
            chunk_count += 1;
            if let Some(lim) = limits {
                if chunk_count > lim.max_png_chunks() {
                    return Err(Error::ContainerLimitExceeded {
                        kind: "PNG chunks",
                        count: chunk_count,
                        limit: lim.max_png_chunks(),
                    });
                }
            }

            let chunk_len = u32::from_be_bytes([
                png_data[pos],
                png_data[pos + 1],
                png_data[pos + 2],
                png_data[pos + 3],
            ]) as usize;

            if let Some(lim) = limits {
                if chunk_len > lim.max_png_chunk_bytes() {
                    return Err(Error::MetadataLimitExceeded {
                        kind: "PNG chunk",
                        size: chunk_len,
                        limit: lim.max_png_chunk_bytes(),
                    });
                }
            }

            let chunk_type = &png_data[pos + 4..pos + 8];

            if chunk_type == b"IEND" {
                let injected_field_count =
                    metadata.len() + usize::from(dmi.is_some()) + usize::from(seed.is_some());
                if let Some(lim) = limits {
                    lim.check_metadata_field_count(injected_field_count)?;
                }

                if let Some(dmi_val) = dmi {
                    let xmp_bytes = Self::generate_xmp_dmi(dmi_val, seed);
                    if let Some(lim) = limits {
                        lim.check_metadata_size("XMP", xmp_bytes.len(), lim.max_xmp_bytes())?;
                    }
                    let xmp_chunk = Self::create_png_xmp_chunk(&xmp_bytes)?;
                    output.extend_from_slice(&xmp_chunk);
                }
                for (key, value) in metadata {
                    let text_chunk = Self::create_png_text_chunk(key, value, limits)?;
                    output.extend_from_slice(&text_chunk);
                }
                if let Some(s) = seed {
                    let desc_value = format!("Protected image. Seed: {}", s);
                    let desc_chunk =
                        Self::create_png_text_chunk(b"Description", desc_value.as_bytes(), limits)?;
                    output.extend_from_slice(&desc_chunk);
                }
            }

            let chunk_end = pos
                .checked_add(12)
                .and_then(|p| p.checked_add(chunk_len))
                .ok_or_else(|| Error::ImageTruncated("PNG chunk length overflow".to_string()))?;
            if chunk_end > png_data.len() {
                return Err(Error::ImageTruncated(format!(
                    "PNG chunk at offset {} claims length {} but only {} bytes remain",
                    pos,
                    chunk_len,
                    png_data.len().saturating_sub(pos)
                )));
            }
            output.extend_from_slice(&png_data[pos..chunk_end]);
            pos = chunk_end;
        }

        Ok(output)
    }

    pub(super) fn create_png_xmp_chunk(xmp_data: &[u8]) -> Result<Vec<u8>> {
        // PNG iTXt chunk format (spec-compliant):
        //   keyword\0 compression_flag(1) compression_method(1)
        //   language_tag\0 translated_keyword\0 text
        let keyword = b"XML:com.adobe.xmp";
        let mut chunk_data = Vec::new();
        chunk_data.extend_from_slice(keyword);
        chunk_data.push(0); // null separator after keyword
        chunk_data.push(0); // compression flag: 0 = uncompressed
        chunk_data.push(0); // compression method: 0 (deflate, unused when flag=0)
        chunk_data.push(0); // null separator (empty language tag)
        chunk_data.push(0); // null separator (empty translated keyword)
        chunk_data.extend_from_slice(xmp_data);

        let len = u32::try_from(chunk_data.len()).map_err(|_| {
            Error::Metadata(format!(
                "PNG iTXt chunk data length {} exceeds u32::MAX",
                chunk_data.len()
            ))
        })?;
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&len.to_be_bytes());
        chunk.extend_from_slice(b"iTXt");
        chunk.extend_from_slice(&chunk_data);
        let crc = Self::crc32(b"iTXt", &chunk_data);
        chunk.extend_from_slice(&crc.to_be_bytes());
        Ok(chunk)
    }

    pub(super) fn extract_seed_from_png(
        png_data: &[u8],
        limits: Option<&crate::ResourceLimits>,
    ) -> Option<u64> {
        let mut pos = 8;
        let mut chunk_count: usize = 0;
        while pos + 12 <= png_data.len() {
            chunk_count += 1;
            if let Some(lim) = limits {
                if chunk_count > lim.max_png_chunks() {
                    return None;
                }
            }

            let chunk_len = u32::from_be_bytes([
                png_data[pos],
                png_data[pos + 1],
                png_data[pos + 2],
                png_data[pos + 3],
            ]) as usize;

            if let Some(lim) = limits {
                if chunk_len > lim.max_png_chunk_bytes() {
                    return None;
                }
            }

            let chunk_type = &png_data[pos + 4..pos + 8];

            if chunk_type == b"IEND" {
                break;
            }

            if chunk_type == b"tEXt" || chunk_type == b"iTXt" {
                let Some(data_start) = pos.checked_add(8) else {
                    break;
                };
                let Some(raw_end) = data_start.checked_add(chunk_len) else {
                    break;
                };
                let data_end = raw_end.min(png_data.len());
                if data_start > data_end {
                    break;
                }
                let data = &png_data[data_start..data_end];

                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    let key = &data[..null_pos];
                    let value = &data[null_pos + 1..];
                    let value_str = if let Some(end) = value.iter().position(|&b| b == 0) {
                        String::from_utf8_lossy(&value[..end]).into_owned()
                    } else {
                        String::from_utf8_lossy(value).into_owned()
                    };

                    if key == b"X-Protection-Seed" {
                        if let Ok(seed) = value_str.parse() {
                            return Some(seed);
                        }
                    }

                    if key == b"Description" {
                        if let Some(seed_str) = value_str.strip_prefix("Protected image. Seed: ") {
                            if let Ok(seed) = seed_str.trim().parse() {
                                return Some(seed);
                            }
                        }
                    }
                }
            }

            let Some(next_pos) = pos.checked_add(12).and_then(|p| p.checked_add(chunk_len)) else {
                break;
            };
            pos = next_pos;
        }
        None
    }

    pub(super) fn is_stego_owned_text_key(key: &[u8]) -> bool {
        STEGO_OWNED_PNG_KEYS.contains(&key)
    }

    pub(super) fn strip_stego_owned_png(png_data: &[u8]) -> Result<Vec<u8>> {
        if png_data.len() < 8 || &png_data[0..8] != b"\x89PNG\r\n\x1a\n" {
            return Err(Error::Metadata("Invalid PNG signature".to_string()));
        }
        let mut output = Vec::with_capacity(png_data.len());
        output.extend_from_slice(&png_data[0..8]);

        let mut pos = 8;
        while pos + 12 <= png_data.len() {
            let chunk_len = u32::from_be_bytes([
                png_data[pos],
                png_data[pos + 1],
                png_data[pos + 2],
                png_data[pos + 3],
            ]) as usize;
            let chunk_type = &png_data[pos + 4..pos + 8];

            let chunk_end = pos
                .checked_add(12)
                .and_then(|p| p.checked_add(chunk_len))
                .ok_or_else(|| Error::ImageTruncated("PNG chunk length overflow".to_string()))?;
            if chunk_end > png_data.len() {
                return Err(Error::ImageTruncated(format!(
                    "PNG chunk at offset {} claims length {} but only {} bytes remain",
                    pos,
                    chunk_len,
                    png_data.len().saturating_sub(pos)
                )));
            }

            let is_stego = if chunk_type == b"tEXt" || chunk_type == b"iTXt" {
                let data = &png_data[pos + 8..chunk_end];
                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    Self::is_stego_owned_text_key(&data[..null_pos])
                } else {
                    false
                }
            } else {
                false
            };

            if !is_stego {
                output.extend_from_slice(&png_data[pos..chunk_end]);
            }

            pos = chunk_end;
        }

        Ok(output)
    }

    pub(super) fn collect_stego_owned_png_keys(png_data: &[u8]) -> Vec<Vec<u8>> {
        let mut keys = Vec::new();
        if png_data.len() < 8 || &png_data[0..8] != b"\x89PNG\r\n\x1a\n" {
            return keys;
        }
        let mut pos = 8;
        while pos + 12 <= png_data.len() {
            let chunk_len = u32::from_be_bytes([
                png_data[pos],
                png_data[pos + 1],
                png_data[pos + 2],
                png_data[pos + 3],
            ]) as usize;
            let chunk_type = &png_data[pos + 4..pos + 8];
            if chunk_type == b"IEND" {
                break;
            }
            if chunk_type == b"tEXt" || chunk_type == b"iTXt" {
                let data_start = pos + 8;
                let Some(data_end) = data_start.checked_add(chunk_len) else {
                    break;
                };
                let data_end = data_end.min(png_data.len());
                let data = &png_data[data_start..data_end];
                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    let key = &data[..null_pos];
                    if Self::is_stego_owned_text_key(key) {
                        keys.push(key.to_vec());
                    }
                }
            }
            let Some(chunk_end) = pos.checked_add(12).and_then(|p| p.checked_add(chunk_len)) else {
                break;
            };
            if chunk_end > png_data.len() {
                break;
            }
            pos = chunk_end;
        }
        keys
    }
}
