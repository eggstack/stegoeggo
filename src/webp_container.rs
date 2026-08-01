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
pub(crate) struct ParsedWebP {
    pub data: Vec<u8>,
    pub image_kind: WebPImageKind,
    pub chunks: Vec<RiffChunk>,
    pub vp8x_index: Option<usize>,
    pub has_xmp: bool,
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
    if riff_size.saturating_add(8) > data.len() {
        return Err(Error::Metadata(
            "RIFF declared size exceeds input length".to_string(),
        ));
    }

    let mut chunks = Vec::new();
    let mut pos = 12;
    let mut image_kind: Option<WebPImageKind> = None;
    let mut vp8x_index: Option<usize> = None;
    let mut has_xmp = false;
    let mut has_icc = false;
    let mut has_alpha = false;
    let mut has_animation = false;

    while pos + 8 <= data.len() {
        let fourcc = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
        let chunk_size =
            u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                as usize;

        let data_start = pos + 8;
        let data_end = data_start
            .checked_add(chunk_size)
            .ok_or_else(|| Error::Metadata("RIFF chunk size overflow".to_string()))?;

        if data_end > data.len() {
            return Err(Error::Metadata(format!(
                "RIFF chunk '{}' at offset {} declares {} bytes but only {} remain",
                std::str::from_utf8(&fourcc).unwrap_or("????"),
                pos,
                chunk_size,
                data.len() - data_start,
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

        match &fourcc {
            b"VP8 " => {
                if image_kind.is_none() {
                    image_kind = Some(WebPImageKind::LossyVP8);
                }
            }
            b"VP8L" => {
                if image_kind.is_none() {
                    image_kind = Some(WebPImageKind::LosslessVP8L);
                }
            }
            b"VP8X" => {
                image_kind = Some(WebPImageKind::ExtendedVP8X);
                vp8x_index = Some(chunks.len());
                if chunk_size >= 10 {
                    let flags = data[data_start];
                    has_icc = (flags & 0x20) != 0;
                    has_alpha = (flags & 0x10) != 0;
                    has_animation = (flags & 0x02) != 0;
                }
            }
            b"ICCP" => {
                has_icc = true;
            }
            b"ANMF" => {
                has_animation = true;
            }
            b"XMP " => {
                has_xmp = true;
            }
            _ => {}
        }

        chunks.push(RiffChunk {
            fourcc,
            data_start,
            data_len: chunk_size,
        });

        let padded = chunk_size + (chunk_size & 1);
        pos = data_start
            .checked_add(padded)
            .ok_or_else(|| Error::Metadata("RIFF chunk alignment overflow".to_string()))?;
    }

    let image_kind = image_kind.ok_or_else(|| {
        Error::Metadata("WebP missing mandatory image chunk (VP8/VP8L/VP8X)".to_string())
    })?;

    Ok(ParsedWebP {
        data: data.to_vec(),
        image_kind,
        chunks,
        vp8x_index,
        has_xmp,
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
        let vp8x_payload = vec![0x10u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
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
}
