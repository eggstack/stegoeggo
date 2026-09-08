use crate::error::{Error, Result};
use crate::types::{DmiValue, LegalMetadata, RightsNotice};
use image::ImageDecoder;

impl super::RightsMetadataProtector {
    pub(super) fn inject_text_chunks_webp_from_notice(
        &self,
        webp_data: &[u8],
        notice: &RightsNotice,
    ) -> Result<Vec<u8>> {
        notice.validate()?;
        if !notice.has_legal_content() && notice.dmi().is_none() {
            return Ok(webp_data.to_vec());
        }

        let parsed = crate::webp_container::parse_webp(webp_data, None)?;

        let dmi_val = notice.dmi().unwrap_or(DmiValue::Unspecified);
        let xmp_data = Self::generate_xmp_notice_from_notice(dmi_val, notice);
        if xmp_data.is_empty() {
            return Ok(webp_data.to_vec());
        }

        let mut metadata_chunks: Vec<([u8; 4], Vec<u8>)> = Vec::new();

        if parsed.has_xmp {
            self.merge_or_replace_webp_xmp(webp_data, &parsed, &xmp_data, &mut metadata_chunks)?;
        } else {
            let xmp_chunk = Self::create_webp_xmp_chunk_data(&xmp_data);
            metadata_chunks.push((*b"XMP ", xmp_chunk));
        }

        let mut output = Vec::new();
        output.extend_from_slice(b"RIFF");
        output.extend_from_slice(&[0, 0, 0, 0]);
        output.extend_from_slice(b"WEBP");

        let (width, height) = match parsed.image_kind {
            crate::webp_container::WebPImageKind::ExtendedVP8X => {
                if let Some(idx) = parsed.vp8x_index {
                    let chunk = &parsed.chunks[idx];
                    crate::webp_container::vp8x_dimensions(&parsed.data, chunk.data_start)
                        .unwrap_or((0, 0))
                } else {
                    (0, 0)
                }
            }
            _ => self.webp_image_dimensions(webp_data)?.unwrap_or((0, 0)),
        };

        let has_metadata = !metadata_chunks.is_empty();

        let derived = crate::webp_container::derive_features(&parsed);
        let features = derived.with_xmp(has_metadata);
        let vp8x_flags = features.as_vp8x_flags();
        // Note: if the original WebP was malformed (reserved VP8X bits 0xC1
        // set), `derive_features` + `validate_webp_output` silently
        // normalizes it to valid. This is intentional — injection fixes
        // reserved bits rather than preserving malformed flags.

        let vp8x_data = crate::webp_container::encode_vp8x_chunk(width, height, vp8x_flags)?;
        output.extend_from_slice(b"VP8X");
        output.extend_from_slice(&(vp8x_data.len() as u32).to_le_bytes());
        output.extend_from_slice(&vp8x_data);
        if vp8x_data.len() & 1 != 0 {
            output.push(0);
        }

        for chunk in &parsed.chunks {
            if chunk.fourcc_str() == "VP8X" || chunk.fourcc_str() == "XMP " {
                continue;
            }
            let data_end = chunk.data_start + chunk.data_len;
            if data_end > parsed.data.len() {
                return Err(Error::Metadata(format!(
                    "Truncated RIFF chunk '{}'",
                    chunk.fourcc_str()
                )));
            }
            output.extend_from_slice(&chunk.fourcc);
            output.extend_from_slice(&(chunk.data_len as u32).to_le_bytes());
            output.extend_from_slice(&parsed.data[chunk.data_start..data_end]);
            if chunk.data_len & 1 != 0 {
                output.push(0);
            }
        }

        for (fourcc, data) in &metadata_chunks {
            output.extend_from_slice(fourcc);
            output.extend_from_slice(&(data.len() as u32).to_le_bytes());
            output.extend_from_slice(data);
            if data.len() & 1 != 0 {
                output.push(0);
            }
        }

        if output.len() >= u32::MAX as usize + 8 {
            return Err(Error::Metadata(
                "WebP file would exceed 4 GiB limit after metadata injection".to_string(),
            ));
        }
        let riff_size = u32::try_from(output.len() - 8)
            .map_err(|_| Error::Metadata("WebP RIFF size exceeds the u32 limit".to_string()))?;
        output[4] = riff_size as u8;
        output[5] = (riff_size >> 8) as u8;
        output[6] = (riff_size >> 16) as u8;
        output[7] = (riff_size >> 24) as u8;

        crate::webp_container::validate_webp_output(&output)?;

        Ok(output)
    }

    pub(super) fn webp_image_dimensions(&self, webp_data: &[u8]) -> Result<Option<(u32, u32)>> {
        let cursor = std::io::Cursor::new(webp_data);
        match image::codecs::webp::WebPDecoder::new(cursor) {
            Ok(decoder) => Ok(Some(decoder.dimensions())),
            Err(_) => Ok(None),
        }
    }

    pub(super) fn merge_or_replace_webp_xmp(
        &self,
        _webp_data: &[u8],
        parsed: &crate::webp_container::ParsedWebP,
        new_xmp_data: &[u8],
        metadata_chunks: &mut Vec<([u8; 4], Vec<u8>)>,
    ) -> Result<()> {
        let mut all_descs: Vec<crate::xmp::PreservedDescription> = Vec::new();

        for &idx in &parsed.xmp_indices {
            let chunk = &parsed.chunks[idx];
            let data_end = chunk.data_start + chunk.data_len;
            if data_end > parsed.data.len() {
                return Err(Error::Metadata(format!(
                    "Truncated XMP chunk '{}' in WebP file",
                    chunk.fourcc_str()
                )));
            }
            let existing = &parsed.data[chunk.data_start..data_end];
            let descs = crate::xmp::filter_xmp_packet(existing)?;
            all_descs.extend(descs);
        }

        let canonical_descs = crate::xmp::filter_xmp_packet(new_xmp_data)?;
        let deduped = crate::xmp::deduplicate_descriptions(&all_descs, &canonical_descs);

        let merged = if deduped.is_empty() {
            new_xmp_data.to_vec()
        } else {
            crate::xmp::merge_preserved_descriptions(new_xmp_data, &deduped)?
        };

        let xmp_chunk = Self::create_webp_xmp_chunk_data(&merged);
        metadata_chunks.push((*b"XMP ", xmp_chunk));
        Ok(())
    }

    pub(super) fn webp_has_stego_metadata(&self, webp_data: &[u8]) -> bool {
        if webp_data.len() < 12 || &webp_data[0..4] != b"RIFF" || &webp_data[8..12] != b"WEBP" {
            return false;
        }
        let mut pos = 12;
        while pos + 8 <= webp_data.len() {
            let chunk_id = &webp_data[pos..pos + 4];
            let chunk_size = u32::from_le_bytes([
                webp_data[pos + 4],
                webp_data[pos + 5],
                webp_data[pos + 6],
                webp_data[pos + 7],
            ]) as usize;
            let Some(padded_size) = chunk_size.checked_add(chunk_size & 1) else {
                break;
            };
            let Some(data_start) = pos.checked_add(8) else {
                break;
            };
            let Some(data_end) = data_start.checked_add(chunk_size) else {
                break;
            };

            if chunk_id == b"XMP "
                && data_end <= webp_data.len()
                && Self::xmp_has_stego_properties(&webp_data[data_start..data_end])
            {
                return true;
            }

            let Some(next_pos) = pos.checked_add(8).and_then(|p| p.checked_add(padded_size)) else {
                break;
            };
            pos = next_pos;
        }
        false
    }

    #[allow(dead_code)]
    pub(super) fn inject_text_chunks_webp(
        &self,
        webp_data: &[u8],
        metadata: &[(Vec<u8>, Vec<u8>)],
        dmi: Option<DmiValue>,
        seed: Option<u64>,
        legal: Option<&LegalMetadata>,
    ) -> Result<Vec<u8>> {
        if metadata.is_empty() && dmi.is_none() {
            return Ok(webp_data.to_vec());
        }

        if webp_data.len() < 12 || &webp_data[0..4] != b"RIFF" || &webp_data[8..12] != b"WEBP" {
            return Err(Error::Metadata("Invalid WebP signature".to_string()));
        }

        let dmi_val = dmi.unwrap_or(DmiValue::Unspecified);

        // Build XMP chunk with DMI, seed, and legal notice embedded as XMP properties
        let xmp_chunk =
            Self::create_webp_xmp_chunk(&Self::generate_xmp_notice(dmi_val, seed, legal));

        if xmp_chunk.is_empty() {
            return Ok(webp_data.to_vec());
        }

        let exif_chunk = seed.map(Self::create_webp_exif_chunk);

        // Copy original data and append new chunks
        let extra_len = xmp_chunk.len() + exif_chunk.as_ref().map_or(0, |c| c.len());
        let new_len = webp_data.len() + extra_len;
        if new_len > u32::MAX as usize + 8 {
            return Err(Error::Metadata(
                "WebP file would exceed 4 GiB limit after metadata injection".to_string(),
            ));
        }
        let mut output = Vec::with_capacity(new_len);
        output.extend_from_slice(webp_data);
        output.extend_from_slice(&xmp_chunk);
        if let Some(exif) = &exif_chunk {
            output.extend_from_slice(exif);
        }

        if output.len() >= 21 && &output[12..16] == b"VP8X" {
            let mut flags = output[20];
            flags |= 0x04;
            if exif_chunk.is_some() {
                flags |= 0x08;
            }
            output[20] = flags;
        }

        // Update RIFF file size in header (bytes 4-8, little-endian)
        // RIFF size = total file size - 8 (RIFF header + size field)
        let new_riff_size = (output.len() - 8) as u32;
        output[4] = new_riff_size as u8;
        output[5] = (new_riff_size >> 8) as u8;
        output[6] = (new_riff_size >> 16) as u8;
        output[7] = (new_riff_size >> 24) as u8;

        Ok(output)
    }

    /// Creates a WebP XMP chunk with the standard `XMP ` FourCC.
    pub(super) fn create_webp_xmp_chunk(xmp_data: &[u8]) -> Vec<u8> {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(b"XMP ");
        chunk.extend_from_slice(&(xmp_data.len() as u32).to_le_bytes());
        chunk.extend_from_slice(xmp_data);
        if !xmp_data.len().is_multiple_of(2) {
            chunk.push(0);
        }
        chunk
    }

    pub(super) fn create_webp_xmp_chunk_data(xmp_data: &[u8]) -> Vec<u8> {
        xmp_data.to_vec()
    }

    /// Creates a WebP EXIF chunk with UserComment containing the seed.
    /// Uses the standard `EXIF` FourCC.
    pub(super) fn create_webp_exif_chunk(seed: u64) -> Vec<u8> {
        let comment = format!("Protection seed: {}", seed);
        let mut exif_data = Vec::new();
        exif_data.extend_from_slice(b"Exif\x00\x00");
        let mut tiff_header = Vec::with_capacity(8);
        tiff_header.extend_from_slice(b"II");
        tiff_header.extend_from_slice(&42u16.to_le_bytes());
        tiff_header.extend_from_slice(&8u32.to_le_bytes());
        let mut ifd = Vec::with_capacity(14);
        ifd.extend_from_slice(&1u16.to_le_bytes());
        ifd.extend_from_slice(&0x9286u16.to_le_bytes());
        ifd.extend_from_slice(&2u16.to_le_bytes());
        ifd.extend_from_slice(&(comment.len() as u32).to_le_bytes());
        ifd.extend_from_slice(&22u32.to_le_bytes());
        exif_data.extend_from_slice(&tiff_header);
        exif_data.extend_from_slice(&ifd);
        exif_data.extend_from_slice(comment.as_bytes());
        let mut chunk = Vec::new();
        chunk.extend_from_slice(b"EXIF");
        chunk.extend_from_slice(&(exif_data.len() as u32).to_le_bytes());
        chunk.extend_from_slice(&exif_data);
        if !exif_data.len().is_multiple_of(2) {
            chunk.push(0);
        }
        chunk
    }

    pub(super) fn extract_seed_from_webp(
        webp_data: &[u8],
        limits: Option<&crate::ResourceLimits>,
    ) -> Option<u64> {
        if webp_data.len() < 20 {
            return None;
        }

        let mut pos = 12;
        let mut chunk_count: usize = 0;

        while pos + 8 <= webp_data.len() {
            chunk_count += 1;
            if let Some(lim) = limits {
                if chunk_count > lim.max_webp_riff_chunks() {
                    return None;
                }
            }

            let chunk_type = &webp_data[pos..pos + 4];
            let chunk_size = u32::from_le_bytes([
                webp_data[pos + 4],
                webp_data[pos + 5],
                webp_data[pos + 6],
                webp_data[pos + 7],
            ]) as usize;

            if let Some(lim) = limits {
                if chunk_size > lim.max_webp_riff_bytes() {
                    return None;
                }
            }

            let Some(data_start) = pos.checked_add(8) else {
                break;
            };
            let Some(raw_end) = data_start.checked_add(chunk_size) else {
                break;
            };
            let data_end = raw_end.min(webp_data.len());

            if chunk_type == b"XMP " && data_end > data_start {
                let data = &webp_data[data_start..data_end];
                if let Ok(xmp_str) = std::str::from_utf8(data) {
                    if let Some(start) = xmp_str.find("stegoeggo:ProtectionSeed=\"") {
                        let value_start = start + "stegoeggo:ProtectionSeed=\"".len();
                        if let Some(end) = xmp_str[value_start..].find('"') {
                            let value_str = &xmp_str[value_start..value_start + end];
                            if let Ok(seed) = value_str.parse::<u64>() {
                                return Some(seed);
                            }
                        }
                    }
                }
            }

            if chunk_type == b"EXIF" && data_end > data_start {
                let data = &webp_data[data_start..data_end];
                let needle = b"Protection seed: ";
                if let Some(offset) = data.windows(needle.len()).position(|w| w == needle) {
                    let start = offset + needle.len();
                    let end = data[start..]
                        .iter()
                        .position(|&b| b == b'\0' || b == b'\n')
                        .map(|p| start + p)
                        .unwrap_or(data.len());
                    if let Ok(seed_str) = std::str::from_utf8(&data[start..end]) {
                        if let Ok(seed) = seed_str.trim().parse::<u64>() {
                            return Some(seed);
                        }
                    }
                }
            }

            let Some(next) = data_start
                .checked_add(chunk_size)
                .and_then(|p| p.checked_add(chunk_size & 1))
            else {
                break;
            };
            pos = next;
        }

        None
    }

    pub(super) fn strip_stego_owned_webp(webp_data: &[u8]) -> Result<Vec<u8>> {
        if webp_data.len() < 12 || &webp_data[0..4] != b"RIFF" || &webp_data[8..12] != b"WEBP" {
            return Err(Error::Metadata("Invalid WebP signature".to_string()));
        }
        let mut output = Vec::with_capacity(webp_data.len());
        output.extend_from_slice(&webp_data[0..12]);

        // XMP is intentionally left in place: injection replaces stego-owned
        // XMP at the rdf:Description level (merge_or_replace_webp_xmp),
        // preserving unrelated descriptions. Only non-XMP stego-owned
        // content — legacy EXIF seed chunks — is stripped here.
        let mut removed_exif = false;
        let mut pos = 12;
        while pos + 8 <= webp_data.len() {
            let chunk_id = &webp_data[pos..pos + 4];
            let chunk_size = u32::from_le_bytes([
                webp_data[pos + 4],
                webp_data[pos + 5],
                webp_data[pos + 6],
                webp_data[pos + 7],
            ]) as usize;
            let Some(padded_size) = chunk_size.checked_add(chunk_size & 1) else {
                return Err(Error::Metadata("RIFF chunk alignment overflow".to_string()));
            };
            let Some(data_start) = pos.checked_add(8) else {
                return Err(Error::Metadata("RIFF chunk offset overflow".to_string()));
            };
            let Some(data_end) = data_start.checked_add(chunk_size) else {
                return Err(Error::Metadata("RIFF chunk size overflow".to_string()));
            };

            if data_end <= webp_data.len() {
                let is_legacy_exif_seed = chunk_id == b"EXIF"
                    && webp_data[data_start..data_end]
                        .windows(b"Protection seed: ".len())
                        .any(|w| w == b"Protection seed: ");
                if is_legacy_exif_seed {
                    removed_exif = true;
                } else {
                    output.extend_from_slice(&webp_data[pos..data_end]);
                    if chunk_size & 1 != 0 {
                        output.push(0);
                    }
                }
            }

            let Some(next_pos) = pos.checked_add(8).and_then(|p| p.checked_add(padded_size)) else {
                return Err(Error::Metadata("RIFF chunk alignment overflow".to_string()));
            };
            pos = next_pos;
        }

        if removed_exif && output.len() >= 20 && &output[12..16] == b"VP8X" {
            output[20] &= !0x08;
        }

        let new_riff_size = u32::try_from(output.len() - 8).map_err(|_| {
            Error::Metadata(format!(
                "WebP RIFF size {} exceeds u32 limit after strip",
                output.len() - 8
            ))
        })?;
        output[4] = new_riff_size as u8;
        output[5] = (new_riff_size >> 8) as u8;
        output[6] = (new_riff_size >> 16) as u8;
        output[7] = (new_riff_size >> 24) as u8;

        Ok(output)
    }

    pub(super) fn collect_stego_owned_webp_keys(webp_data: &[u8]) -> Vec<Vec<u8>> {
        let mut keys = Vec::new();
        if webp_data.len() < 12 || &webp_data[0..4] != b"RIFF" || &webp_data[8..12] != b"WEBP" {
            return keys;
        }
        let mut pos = 12;
        while pos + 8 <= webp_data.len() {
            let chunk_id = &webp_data[pos..pos + 4];
            let chunk_size = u32::from_le_bytes([
                webp_data[pos + 4],
                webp_data[pos + 5],
                webp_data[pos + 6],
                webp_data[pos + 7],
            ]) as usize;
            let Some(padded_size) = chunk_size.checked_add(chunk_size & 1) else {
                break;
            };
            let Some(data_start) = pos.checked_add(8) else {
                break;
            };
            let Some(data_end) = data_start.checked_add(chunk_size) else {
                break;
            };
            if chunk_id == b"XMP "
                && data_end <= webp_data.len()
                && Self::xmp_has_stego_properties(&webp_data[data_start..data_end])
            {
                keys.push(b"XMP".to_vec());
            }
            let Some(next_pos) = pos.checked_add(8).and_then(|p| p.checked_add(padded_size)) else {
                break;
            };
            pos = next_pos;
        }
        keys
    }
}
