use crate::error::{Error, Result};
use crate::resource_limits::ResourceLimits;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebPImageKind {
    LossyVP8,
    LosslessVP8L,
    ExtendedVP8X,
}

#[derive(Debug, Clone)]
pub(crate) struct RiffChunk {
    pub fourcc: [u8; 4],
    pub data_start: usize,
    pub data_len: usize,
}

impl RiffChunk {
    #[must_use]
    pub fn fourcc_str(&self) -> &str {
        std::str::from_utf8(&self.fourcc).unwrap_or("????")
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct ParsedWebP {
    pub data: Vec<u8>,
    pub image_kind: WebPImageKind,
    pub chunks: Vec<RiffChunk>,
    pub vp8x_index: Option<usize>,
    pub xmp_indices: Vec<usize>,
    pub exif_indices: Vec<usize>,
    pub icc_indices: Vec<usize>,
    pub vp8_indices: Vec<usize>,
    pub vp8l_indices: Vec<usize>,
    pub anim_indices: Vec<usize>,
    pub anmf_indices: Vec<usize>,
    pub alph_indices: Vec<usize>,
    pub has_xmp: bool,
    pub has_exif: bool,
    pub has_icc: bool,
    pub has_alpha: bool,
    pub has_animation: bool,
}

pub(crate) fn parse_webp(data: &[u8], limits: Option<&ResourceLimits>) -> Result<ParsedWebP> {
    if data.len() < 12 {
        return Err(Error::Metadata(
            "WebP file too short for RIFF header".to_string(),
        ));
    }
    if &data[0..4] != b"RIFF" {
        return Err(Error::Metadata("Missing RIFF header".to_string()));
    }
    if &data[8..12] != b"WEBP" {
        return Err(Error::Metadata("Missing WEBP tag".to_string()));
    }

    let riff_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let declared_end = 8usize
        .checked_add(riff_size)
        .ok_or_else(|| Error::Metadata("RIFF size overflow".to_string()))?;

    if declared_end != data.len() {
        return Err(Error::Metadata(format!(
            "RIFF declared size {} does not match physical input length {}",
            riff_size,
            data.len() - 8
        )));
    }

    let mut chunks = Vec::new();
    let mut pos = 12;
    let mut image_kind: Option<WebPImageKind> = None;
    let mut vp8x_index: Option<usize> = None;
    let mut xmp_indices = Vec::new();
    let mut exif_indices = Vec::new();
    let mut icc_indices = Vec::new();
    let mut vp8_indices = Vec::new();
    let mut vp8l_indices = Vec::new();
    let mut anim_indices = Vec::new();
    let mut anmf_indices = Vec::new();
    let mut alph_indices = Vec::new();

    while pos + 8 <= declared_end {
        let fourcc = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
        let chunk_size =
            u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                as usize;

        let data_start = pos + 8;
        let data_end = data_start
            .checked_add(chunk_size)
            .ok_or_else(|| Error::Metadata("RIFF chunk size overflow".to_string()))?;

        if data_end > declared_end {
            return Err(Error::Metadata(format!(
                "RIFF chunk '{}' at offset {} declares {} bytes but RIFF extent ends at {}",
                std::str::from_utf8(&fourcc).unwrap_or("????"),
                pos,
                chunk_size,
                declared_end,
            )));
        }

        if let Some(lim) = limits {
            lim.check_container_count(
                "webp_riff_chunks",
                chunks.len() + 1,
                lim.max_webp_riff_chunks(),
            )?;
            lim.check_metadata_size("webp_chunk_bytes", chunk_size, lim.max_webp_riff_bytes())?;
        }

        let chunk_idx = chunks.len();

        match &fourcc {
            b"VP8 " => {
                if image_kind.is_none() {
                    image_kind = Some(WebPImageKind::LossyVP8);
                }
                vp8_indices.push(chunk_idx);
            }
            b"VP8L" => {
                if image_kind.is_none() {
                    image_kind = Some(WebPImageKind::LosslessVP8L);
                }
                vp8l_indices.push(chunk_idx);
            }
            b"VP8X" => {
                if vp8x_index.is_some() {
                    return Err(Error::Metadata(
                        "Duplicate VP8X chunk in WebP file".to_string(),
                    ));
                }
                if chunk_size != 10 {
                    return Err(Error::Metadata(format!(
                        "VP8X payload length {} invalid: expected exactly 10 bytes",
                        chunk_size
                    )));
                }
                if data_start + 10 > data.len() {
                    return Err(Error::Metadata(
                        "VP8X payload extends beyond input".to_string(),
                    ));
                }
                let flags = data[data_start];
                let reserved_bits = flags & 0xC1;
                if reserved_bits != 0 {
                    return Err(Error::Metadata(format!(
                        "VP8X flags contain reserved bits: 0x{:02X}",
                        reserved_bits
                    )));
                }
                if data[data_start + 1] != 0
                    || data[data_start + 2] != 0
                    || data[data_start + 3] != 0
                {
                    return Err(Error::Metadata(
                        "VP8X reserved bytes are non-zero".to_string(),
                    ));
                }
                let w = u32::from(data[data_start + 4])
                    | (u32::from(data[data_start + 5]) << 8)
                    | (u32::from(data[data_start + 6]) << 16);
                let h = u32::from(data[data_start + 7])
                    | (u32::from(data[data_start + 8]) << 8)
                    | (u32::from(data[data_start + 9]) << 16);
                if w == 0 || h == 0 {
                    return Err(Error::Metadata("VP8X dimensions are zero".to_string()));
                }
                image_kind = Some(WebPImageKind::ExtendedVP8X);
                vp8x_index = Some(chunk_idx);
            }
            b"ICCP" => {
                icc_indices.push(chunk_idx);
            }
            b"ANIM" => {
                anim_indices.push(chunk_idx);
            }
            b"ANMF" => {
                anmf_indices.push(chunk_idx);
            }
            b"ALPH" => {
                alph_indices.push(chunk_idx);
            }
            b"XMP " => {
                xmp_indices.push(chunk_idx);
            }
            b"EXIF" => {
                exif_indices.push(chunk_idx);
            }
            _ => {}
        }

        chunks.push(RiffChunk {
            fourcc,
            data_start,
            data_len: chunk_size,
        });

        let padded = chunk_size + (chunk_size & 1);
        let padded_end = data_start
            .checked_add(padded)
            .ok_or_else(|| Error::Metadata("RIFF chunk alignment overflow".to_string()))?;
        if padded_end > declared_end {
            return Err(Error::Metadata(format!(
                "RIFF chunk '{}' padded end {} exceeds declared RIFF extent {}",
                std::str::from_utf8(&fourcc).unwrap_or("????"),
                padded_end,
                declared_end,
            )));
        }
        pos = padded_end;
    }

    if pos != declared_end {
        return Err(Error::Metadata(format!(
            "Chunk iteration ended at {} but declared RIFF extent is {}",
            pos, declared_end,
        )));
    }

    if vp8_indices.len() > 1 {
        return Err(Error::Metadata(
            "Duplicate VP8 primary image chunks".to_string(),
        ));
    }
    if vp8l_indices.len() > 1 {
        return Err(Error::Metadata(
            "Duplicate VP8L primary image chunks".to_string(),
        ));
    }
    if !vp8_indices.is_empty() && !vp8l_indices.is_empty() {
        return Err(Error::Metadata(
            "Conflicting VP8 and VP8L primary image chunks".to_string(),
        ));
    }
    if alph_indices.len() > 1 {
        return Err(Error::Metadata("Duplicate ALPH chunks".to_string()));
    }
    if !vp8l_indices.is_empty() && !alph_indices.is_empty() {
        return Err(Error::Metadata(
            "ALPH chunk paired with VP8L (VP8L has intrinsic alpha)".to_string(),
        ));
    }

    let image_kind = image_kind.ok_or_else(|| {
        Error::Metadata("WebP missing mandatory image chunk (VP8/VP8L/VP8X)".to_string())
    })?;

    if image_kind == WebPImageKind::ExtendedVP8X
        && vp8_indices.is_empty()
        && vp8l_indices.is_empty()
        && anmf_indices.is_empty()
    {
        return Err(Error::Metadata(
            "VP8X-only container without primary image or animation payload".to_string(),
        ));
    }

    let has_xmp = !xmp_indices.is_empty();
    let has_exif = !exif_indices.is_empty();
    let has_icc = !icc_indices.is_empty();
    let has_animation = !anmf_indices.is_empty() || !anim_indices.is_empty();

    let mut has_alpha = false;
    if let Some(idx) = vp8x_index {
        let chunk = &chunks[idx];
        if chunk.data_len >= 10 {
            let flags = data[chunk.data_start];
            has_alpha = (flags & 0x10) != 0;
        }
    }
    if !has_alpha {
        for &idx in &vp8l_indices {
            let chunk = &chunks[idx];
            let payload_end = chunk.data_start + chunk.data_len;
            if payload_end <= data.len() {
                if let Ok(alpha) = vp8l_has_alpha(&data[chunk.data_start..payload_end]) {
                    if alpha {
                        has_alpha = true;
                        break;
                    }
                }
            }
        }
    }
    if !has_alpha {
        for &idx in &anmf_indices {
            let chunk = &chunks[idx];
            let frame_end = chunk.data_start + chunk.data_len;
            if frame_end > data.len() {
                continue;
            }
            let frame_data = &data[chunk.data_start..frame_end];
            if frame_data.len() < 16 {
                continue;
            }
            let mut sub_pos = 16;
            while sub_pos + 8 <= frame_data.len() {
                let sub_fourcc = [
                    frame_data[sub_pos],
                    frame_data[sub_pos + 1],
                    frame_data[sub_pos + 2],
                    frame_data[sub_pos + 3],
                ];
                let sub_size = u32::from_le_bytes([
                    frame_data[sub_pos + 4],
                    frame_data[sub_pos + 5],
                    frame_data[sub_pos + 6],
                    frame_data[sub_pos + 7],
                ]) as usize;
                let sub_data_start = sub_pos + 8;
                let sub_data_end = sub_data_start + sub_size;
                if sub_data_end > frame_data.len() {
                    break;
                }
                match &sub_fourcc {
                    b"ALPH" => {
                        has_alpha = true;
                        break;
                    }
                    b"VP8L" => {
                        if let Ok(alpha) = vp8l_has_alpha(&frame_data[sub_data_start..sub_data_end])
                        {
                            if alpha {
                                has_alpha = true;
                                break;
                            }
                        }
                    }
                    _ => {}
                }
                let padded = sub_size + (sub_size & 1);
                sub_pos = sub_data_start + padded;
            }
            if has_alpha {
                break;
            }
        }
    }

    Ok(ParsedWebP {
        data: data.to_vec(),
        image_kind,
        chunks,
        vp8x_index,
        xmp_indices,
        exif_indices,
        icc_indices,
        vp8_indices,
        vp8l_indices,
        anim_indices,
        anmf_indices,
        alph_indices,
        has_xmp,
        has_exif,
        has_icc,
        has_alpha,
        has_animation,
    })
}

pub(crate) fn vp8x_dimensions(data: &[u8], vp8x_data_start: usize) -> Option<(u32, u32)> {
    if data.len() < vp8x_data_start + 10 {
        return None;
    }
    let width = (u32::from(data[vp8x_data_start + 4])
        | (u32::from(data[vp8x_data_start + 5]) << 8)
        | (u32::from(data[vp8x_data_start + 6]) << 16))
        + 1;
    let height = (u32::from(data[vp8x_data_start + 7])
        | (u32::from(data[vp8x_data_start + 8]) << 8)
        | (u32::from(data[vp8x_data_start + 9]) << 16))
        + 1;
    Some((width, height))
}

pub(crate) fn encode_vp8x_chunk(width: u32, height: u32, flags: u8) -> Result<Vec<u8>> {
    let w = width
        .checked_sub(1)
        .ok_or_else(|| Error::Metadata("VP8X width must be at least 1".to_string()))?;
    let h = height
        .checked_sub(1)
        .ok_or_else(|| Error::Metadata("VP8X height must be at least 1".to_string()))?;
    let mut data = vec![0u8; 10];
    data[0] = flags;
    data[4] = w as u8;
    data[5] = (w >> 8) as u8;
    data[6] = (w >> 16) as u8;
    data[7] = h as u8;
    data[8] = (h >> 8) as u8;
    data[9] = (h >> 16) as u8;
    Ok(data)
}

pub(crate) fn vp8l_has_alpha(payload: &[u8]) -> Result<bool> {
    if payload.len() < 5 {
        return Err(Error::Metadata("VP8L payload too short".to_string()));
    }
    if payload[0] != 0x2F {
        return Err(Error::Metadata("VP8L signature byte mismatch".to_string()));
    }
    let bits = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
    let width = (bits >> 14) & 0x3FFF;
    let height = (bits >> 2) & 0x3FFF;
    if width == 0 || height == 0 {
        return Err(Error::Metadata("VP8L dimensions are zero".to_string()));
    }
    let alpha_bit = (bits >> 28) & 1;
    Ok(alpha_bit == 1)
}

pub(crate) fn validate_webp_output(data: &[u8]) -> Result<()> {
    let parsed = parse_webp(data, None)?;

    if parsed.vp8x_index.is_none() {
        return Err(Error::Metadata(
            "Output WebP missing VP8X chunk".to_string(),
        ));
    }

    let vp8x_idx = parsed.vp8x_index.unwrap();
    let chunk = &parsed.chunks[vp8x_idx];
    if chunk.data_len < 10 {
        return Err(Error::Metadata("VP8X too short in output".to_string()));
    }
    let flags = data[chunk.data_start];

    let vp8x_count = parsed
        .chunks
        .iter()
        .filter(|c| c.fourcc == *b"VP8X")
        .count();
    if vp8x_count != 1 {
        return Err(Error::Metadata(format!(
            "Expected exactly 1 VP8X chunk, found {}",
            vp8x_count
        )));
    }

    let has_icc_out = parsed.has_icc;
    let has_exif_out = parsed.has_exif;
    let has_xmp_out = parsed.has_xmp;
    let has_anim_out = parsed.has_animation;

    let has_alpha_out = parsed.has_alpha;

    let mut expected_flags: u8 = 0;
    if has_icc_out {
        expected_flags |= 0x20;
    }
    if has_xmp_out {
        expected_flags |= 0x04;
    }
    if has_exif_out {
        expected_flags |= 0x08;
    }
    if has_alpha_out {
        expected_flags |= 0x10;
    }
    if has_anim_out {
        expected_flags |= 0x02;
    }

    if flags != expected_flags {
        return Err(Error::Metadata(format!(
            "VP8X flags mismatch: expected 0x{:02X}, got 0x{:02X}",
            expected_flags, flags
        )));
    }

    let vp8_count = parsed.vp8_indices.len();
    let vp8l_count = parsed.vp8l_indices.len();
    let anmf_count = parsed.anmf_indices.len();
    if vp8_count + vp8l_count + anmf_count == 0 {
        return Err(Error::Metadata(
            "Output WebP has no primary image payload".to_string(),
        ));
    }
    if vp8_count > 1 {
        return Err(Error::Metadata(
            "Output WebP has duplicate VP8 chunks".to_string(),
        ));
    }
    if vp8l_count > 1 {
        return Err(Error::Metadata(
            "Output WebP has duplicate VP8L chunks".to_string(),
        ));
    }
    if vp8_count > 0 && vp8l_count > 0 {
        return Err(Error::Metadata(
            "Output WebP has conflicting VP8 and VP8L".to_string(),
        ));
    }

    if has_xmp_out {
        let xmp_count = parsed
            .chunks
            .iter()
            .filter(|c| c.fourcc == *b"XMP ")
            .count();
        if xmp_count != 1 {
            return Err(Error::Metadata(format!(
                "Expected exactly 1 XMP chunk when XMP present, found {}",
                xmp_count
            )));
        }
    }

    let riff_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let declared_end = 8usize
        .checked_add(riff_size)
        .ok_or_else(|| Error::Metadata("RIFF size overflow".to_string()))?;
    if declared_end != data.len() {
        return Err(Error::Metadata(format!(
            "Output RIFF declared size {} does not match physical length {}",
            riff_size,
            data.len() - 8
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_vp8_webp() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        data.extend_from_slice(b"VP8 ");
        let payload = vec![0u8; 30];
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&payload);
        let riff_size = (data.len() - 8) as u32;
        data[4] = riff_size as u8;
        data[5] = (riff_size >> 8) as u8;
        data[6] = (riff_size >> 16) as u8;
        data[7] = (riff_size >> 24) as u8;
        data
    }

    #[test]
    fn parse_simple_vp8() {
        let data = make_simple_vp8_webp();
        let parsed = parse_webp(&data, None).unwrap();
        assert_eq!(parsed.image_kind, WebPImageKind::LossyVP8);
        assert!(!parsed.has_xmp);
        assert_eq!(parsed.chunks.len(), 1);
    }

    #[test]
    fn parse_rejects_short_input() {
        assert!(parse_webp(b"RIFF", None).is_err());
    }

    #[test]
    fn parse_rejects_non_riff() {
        assert!(parse_webp(b"NOTR\x00\x00\x00\x00WEBP", None).is_err());
    }

    #[test]
    fn parse_rejects_non_webp() {
        assert!(parse_webp(b"RIFF\x00\x00\x00\x00NOTP", None).is_err());
    }

    #[test]
    fn parse_rejects_truncated_chunk() {
        let mut data = make_simple_vp8_webp();
        data[16] = 0xFF;
        data[17] = 0xFF;
        data[18] = 0xFF;
        data[19] = 0xFF;
        assert!(parse_webp(&data, None).is_err());
    }

    #[test]
    fn parse_rejects_riff_size_exceeding_input() {
        let mut data = make_simple_vp8_webp();
        let big_size = (data.len() + 100) as u32;
        data[4..8].copy_from_slice(&big_size.to_le_bytes());
        assert!(parse_webp(&data, None).is_err());
    }

    #[test]
    fn parse_vp8x_with_flags() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        data.extend_from_slice(b"VP8X");
        let mut vp8x_payload = vec![0x10u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        vp8x_payload[4] = 0x0F;
        vp8x_payload[5] = 0x00;
        vp8x_payload[6] = 0x00;
        vp8x_payload[7] = 0x0F;
        vp8x_payload[8] = 0x00;
        vp8x_payload[9] = 0x00;
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        data.extend_from_slice(b"VP8 ");
        let payload = vec![0u8; 30];
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());

        let parsed = parse_webp(&data, None).unwrap();
        assert_eq!(parsed.image_kind, WebPImageKind::ExtendedVP8X);
        assert!(parsed.has_alpha);
        assert!(parsed.vp8x_index.is_some());
    }

    #[test]
    fn encode_decode_vp8x_roundtrip() {
        let vp8x = encode_vp8x_chunk(1920, 1080, 0x10).unwrap();
        let dims = vp8x_dimensions(&vp8x, 0).unwrap();
        assert_eq!(dims, (1920, 1080));
    }

    #[test]
    fn encode_vp8x_rejects_zero_dimensions() {
        assert!(encode_vp8x_chunk(0, 100, 0).is_err());
        assert!(encode_vp8x_chunk(100, 0, 0).is_err());
    }

    #[test]
    fn parse_rejects_duplicate_vp8x() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let mut vp8x_payload = vec![0u8; 10];
        vp8x_payload[4] = 0x01;
        vp8x_payload[7] = 0x01;
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        data.extend_from_slice(b"VP8 ");
        let payload = vec![0u8; 30];
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());

        let result = parse_webp(&data, None);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Duplicate VP8X"),
            "Expected duplicate VP8X error, got: {}",
            err_msg
        );
    }

    #[test]
    fn parse_rejects_riff_size_smaller_than_input() {
        let data = make_simple_vp8_webp();
        let mut smaller = data.clone();
        let riff_size = u32::from_le_bytes([smaller[4], smaller[5], smaller[6], smaller[7]]);
        let new_size = riff_size.saturating_sub(10);
        smaller[4..8].copy_from_slice(&new_size.to_le_bytes());
        assert!(parse_webp(&smaller, None).is_err());
    }

    #[test]
    fn parse_rejects_vp8x_wrong_payload_length() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        data.extend_from_slice(b"VP8X");
        let vp8x_payload = vec![0u8; 9];
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        data.extend_from_slice(b"VP8 ");
        let payload = vec![0u8; 30];
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        assert!(parse_webp(&data, None).is_err());
    }

    #[test]
    fn parse_rejects_vp8x_reserved_flags() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        data.extend_from_slice(b"VP8X");
        let mut vp8x_payload = vec![0u8; 10];
        vp8x_payload[0] = 0x01;
        vp8x_payload[4] = 0x01;
        vp8x_payload[7] = 0x01;
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        data.extend_from_slice(b"VP8 ");
        let payload = vec![0u8; 30];
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        assert!(parse_webp(&data, None).is_err());
    }

    #[test]
    fn parse_rejects_vp8x_reserved_bytes() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        data.extend_from_slice(b"VP8X");
        let mut vp8x_payload = vec![0u8; 10];
        vp8x_payload[1] = 0x01;
        vp8x_payload[4] = 0x01;
        vp8x_payload[7] = 0x01;
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        data.extend_from_slice(b"VP8 ");
        let payload = vec![0u8; 30];
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        assert!(parse_webp(&data, None).is_err());
    }

    #[test]
    fn parse_rejects_vp8x_only_container() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let mut vp8x_payload = vec![0u8; 10];
        vp8x_payload[4] = 0x01;
        vp8x_payload[7] = 0x01;
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("VP8X-only"));
    }

    #[test]
    fn parse_rejects_duplicate_vp8() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        data.extend_from_slice(b"VP8 ");
        let payload = vec![0u8; 30];
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&payload);
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        assert!(parse_webp(&data, None).is_err());
    }

    #[test]
    fn parse_rejects_vp8_and_vp8l_conflict() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        data.extend_from_slice(b"VP8 ");
        let payload = vec![0u8; 30];
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&payload);
        data.extend_from_slice(b"VP8L");
        let vp8l_payload = vec![0x2Fu8, 0x00, 0x00, 0x00, 0x00];
        data.extend_from_slice(&(vp8l_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8l_payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        assert!(parse_webp(&data, None).is_err());
    }

    #[test]
    fn vp8l_detects_alpha() {
        let mut payload = vec![0x2Fu8, 0, 0, 0, 0];
        let bits = (1u32 << 28) | (15 << 14) | 15;
        payload[1..5].copy_from_slice(&bits.to_le_bytes());
        assert!(vp8l_has_alpha(&payload).unwrap());
    }

    #[test]
    fn vp8l_detects_no_alpha() {
        let mut payload = vec![0x2Fu8, 0, 0, 0, 0];
        let bits = (15u32 << 14) | 15;
        payload[1..5].copy_from_slice(&bits.to_le_bytes());
        assert!(!vp8l_has_alpha(&payload).unwrap());
    }

    #[test]
    fn vp8l_rejects_short_payload() {
        assert!(vp8l_has_alpha(&[0x2F, 0x00]).is_err());
    }

    #[test]
    fn vp8l_rejects_bad_signature() {
        let payload = vec![0x00u8, 0, 0, 0, 0];
        assert!(vp8l_has_alpha(&payload).is_err());
    }

    #[test]
    fn anmf_with_alph_detects_alpha() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let mut vp8x_payload = vec![0u8; 10];
        vp8x_payload[0] = 0x02;
        vp8x_payload[4] = 0x01;
        vp8x_payload[7] = 0x01;
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]);
        let mut anmf_payload = vec![0u8; 16];
        anmf_payload[4] = 0x20;
        anmf_payload[12] = 0x20;
        data.extend_from_slice(b"ANMF");
        let alph_data = vec![0u8; 4];
        let anmf_size = 16 + 8 + 4;
        data.extend_from_slice(&(anmf_size as u32).to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"ALPH");
        data.extend_from_slice(&(alph_data.len() as u32).to_le_bytes());
        data.extend_from_slice(&alph_data);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let parsed = parse_webp(&data, None).unwrap();
        assert!(parsed.has_alpha, "ANMF with ALPH should set has_alpha");
    }

    #[test]
    fn anmf_with_vp8l_alpha_detects_alpha() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let mut vp8x_payload = vec![0u8; 10];
        vp8x_payload[0] = 0x02;
        vp8x_payload[4] = 0x01;
        vp8x_payload[7] = 0x01;
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]);
        let mut anmf_payload = vec![0u8; 16];
        anmf_payload[4] = 0x20;
        anmf_payload[12] = 0x20;
        let mut vp8l_payload = vec![0x2Fu8, 0, 0, 0, 0];
        let bits = (1u32 << 28) | (15 << 14) | 15;
        vp8l_payload[1..5].copy_from_slice(&bits.to_le_bytes());
        let anmf_size = 16 + 8 + vp8l_payload.len();
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&(anmf_size as u32).to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8L");
        data.extend_from_slice(&(vp8l_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8l_payload);
        if anmf_size & 1 != 0 {
            data.push(0);
        }
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let parsed = parse_webp(&data, None).unwrap();
        assert!(
            parsed.has_alpha,
            "ANMF with alpha VP8L should set has_alpha"
        );
    }

    #[test]
    fn anmf_with_opaque_vp8_no_alpha() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let mut vp8x_payload = vec![0u8; 10];
        vp8x_payload[4] = 0x01;
        vp8x_payload[7] = 0x01;
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]);
        let mut anmf_payload = vec![0u8; 16];
        anmf_payload[4] = 0x20;
        anmf_payload[12] = 0x20;
        data.extend_from_slice(b"ANMF");
        let vp8_data = vec![0u8; 10];
        let anmf_size = 16 + 8 + vp8_data.len();
        data.extend_from_slice(&(anmf_size as u32).to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(vp8_data.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8_data);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let parsed = parse_webp(&data, None).unwrap();
        assert!(
            !parsed.has_alpha,
            "ANMF with opaque VP8 should not set has_alpha"
        );
    }

    #[test]
    fn validate_output_accepts_valid_webp() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let mut vp8x_payload = vec![0u8; 10];
        vp8x_payload[4] = 0x01;
        vp8x_payload[7] = 0x01;
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        data.extend_from_slice(b"VP8 ");
        let payload = vec![0u8; 30];
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        assert!(validate_webp_output(&data).is_ok());
    }

    #[test]
    fn validate_output_detects_flag_mismatch() {
        let mut data = make_simple_vp8_webp();
        let mut vp8x_payload = vec![0u8; 10];
        vp8x_payload[0] = 0x04;
        vp8x_payload[4] = 0x01;
        vp8x_payload[7] = 0x01;
        let vp8x_chunk = {
            let mut chunk = Vec::new();
            chunk.extend_from_slice(b"VP8X");
            chunk.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
            chunk.extend_from_slice(&vp8x_payload);
            chunk
        };
        let vp8_chunk = {
            let mut chunk = Vec::new();
            chunk.extend_from_slice(b"VP8 ");
            let payload = vec![0u8; 30];
            chunk.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            chunk.extend_from_slice(&payload);
            chunk
        };
        data.clear();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        data.extend_from_slice(&vp8x_chunk);
        data.extend_from_slice(&vp8_chunk);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = validate_webp_output(&data);
        assert!(result.is_err());
    }
}
