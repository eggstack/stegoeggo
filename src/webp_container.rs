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
    pub xmp_indices: Vec<usize>,
    #[allow(dead_code)]
    pub exif_indices: Vec<usize>,
    #[allow(dead_code)]
    pub icc_indices: Vec<usize>,
    pub vp8_indices: Vec<usize>,
    pub vp8l_indices: Vec<usize>,
    #[allow(dead_code)]
    pub anim_indices: Vec<usize>,
    pub anmf_indices: Vec<usize>,
    #[allow(dead_code)]
    pub alph_indices: Vec<usize>,
    pub has_xmp: bool,
    pub has_exif: bool,
    pub has_icc: bool,
    pub has_alpha: bool,
    pub has_animation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameImageKind {
    LossyVP8,
    LosslessVP8L,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AnmfFrameInfo {
    #[allow(dead_code)]
    pub x: u32,
    #[allow(dead_code)]
    pub y: u32,
    #[allow(dead_code)]
    pub width: u32,
    #[allow(dead_code)]
    pub height: u32,
    #[allow(dead_code)]
    pub duration_ms: u32,
    #[allow(dead_code)]
    pub blend: bool,
    #[allow(dead_code)]
    pub dispose: bool,
    pub has_alpha: bool,
    pub image_kind: Option<FrameImageKind>,
}

fn read_u24_le(bytes: &[u8], offset: usize) -> Result<u32> {
    if offset
        .checked_add(3)
        .ok_or_else(|| Error::Metadata("u24 overflow".to_string()))?
        > bytes.len()
    {
        return Err(Error::Metadata("u24 read out of bounds".to_string()));
    }
    Ok(u32::from(bytes[offset])
        | (u32::from(bytes[offset + 1]) << 8)
        | (u32::from(bytes[offset + 2]) << 16))
}

pub(crate) fn parse_anmf_frame(
    frame_data: &[u8],
    canvas_width: u32,
    canvas_height: u32,
) -> Result<AnmfFrameInfo> {
    if frame_data.len() < 16 {
        return Err(Error::Metadata(format!(
            "ANMF frame is {} bytes, must be at least 16",
            frame_data.len()
        )));
    }
    let stored_x = read_u24_le(frame_data, 0)?;
    let stored_y = read_u24_le(frame_data, 3)?;
    let stored_w_minus_one = read_u24_le(frame_data, 6)?;
    let stored_h_minus_one = read_u24_le(frame_data, 9)?;
    let duration_ms = read_u24_le(frame_data, 12)?;
    let flags = frame_data[15];
    let reserved_bits = flags & 0xFC;
    if reserved_bits != 0 {
        return Err(Error::Metadata(format!(
            "ANMF flags byte 0x{:02X} contains reserved bits 0x{:02X}",
            flags, reserved_bits
        )));
    }
    let blend = (flags & 0x02) != 0;
    let dispose = (flags & 0x01) != 0;

    let x = stored_x.checked_mul(2).ok_or_else(|| {
        Error::Metadata(format!("ANMF frame x coordinate overflow: {}", stored_x))
    })?;
    let y = stored_y.checked_mul(2).ok_or_else(|| {
        Error::Metadata(format!("ANMF frame y coordinate overflow: {}", stored_y))
    })?;
    let width = stored_w_minus_one
        .checked_add(1)
        .ok_or_else(|| Error::Metadata("ANMF frame width overflow".to_string()))?;
    let height = stored_h_minus_one
        .checked_add(1)
        .ok_or_else(|| Error::Metadata("ANMF frame height overflow".to_string()))?;
    if width == 0 || height == 0 {
        return Err(Error::Metadata(
            "ANMF frame dimensions are zero".to_string(),
        ));
    }
    let right = x
        .checked_add(width)
        .ok_or_else(|| Error::Metadata("ANMF frame right edge overflow".to_string()))?;
    let bottom = y
        .checked_add(height)
        .ok_or_else(|| Error::Metadata("ANMF frame bottom edge overflow".to_string()))?;
    if right > canvas_width {
        return Err(Error::Metadata(format!(
            "ANMF frame right edge {} exceeds canvas width {}",
            right, canvas_width
        )));
    }
    if bottom > canvas_height {
        return Err(Error::Metadata(format!(
            "ANMF frame bottom edge {} exceeds canvas height {}",
            bottom, canvas_height
        )));
    }

    let mut info = AnmfFrameInfo {
        x,
        y,
        width,
        height,
        duration_ms,
        blend,
        dispose,
        has_alpha: false,
        image_kind: None,
    };
    let mut sub_pos = 16;
    let mut vp8_count = 0usize;
    let mut vp8l_count = 0usize;
    let mut alph_count = 0usize;
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
        let sub_data_end = sub_data_start
            .checked_add(sub_size)
            .ok_or_else(|| Error::Metadata("ANMF nested chunk overflow".to_string()))?;
        if sub_data_end > frame_data.len() {
            return Err(Error::Metadata(format!(
                "ANMF nested chunk '{}' at offset {} exceeds frame bounds",
                std::str::from_utf8(&sub_fourcc).unwrap_or("????"),
                sub_pos
            )));
        }
        match &sub_fourcc {
            b"VP8 " => {
                vp8_count += 1;
            }
            b"VP8L" => {
                vp8l_count += 1;
                let vp8l_payload = &frame_data[sub_data_start..sub_data_end];
                let header = parse_vp8l_header(vp8l_payload)?;
                if header.alpha {
                    info.has_alpha = true;
                }
            }
            b"ALPH" => {
                alph_count += 1;
            }
            _ => {}
        }
        let padded = sub_size
            .checked_add(sub_size & 1)
            .ok_or_else(|| Error::Metadata("ANMF nested chunk padded overflow".to_string()))?;
        let next_pos = sub_data_start
            .checked_add(padded)
            .ok_or_else(|| Error::Metadata("ANMF nested chunk padded overflow".to_string()))?;
        if next_pos > frame_data.len() {
            return Err(Error::Metadata(format!(
                "ANMF nested chunk '{}' pad byte exceeds frame bounds",
                std::str::from_utf8(&sub_fourcc).unwrap_or("????")
            )));
        }
        sub_pos = next_pos;
    }
    if vp8_count + vp8l_count != 1 {
        return Err(Error::Metadata(format!(
            "ANMF requires exactly one image payload (VP8 or VP8L), got VP8={} VP8L={}",
            vp8_count, vp8l_count
        )));
    }
    if vp8_count > 1 {
        return Err(Error::Metadata(
            "ANMF has duplicate VP8 image chunk".to_string(),
        ));
    }
    if vp8l_count > 1 {
        return Err(Error::Metadata(
            "ANMF has duplicate VP8L image chunk".to_string(),
        ));
    }
    if alph_count > 1 {
        return Err(Error::Metadata("ANMF has duplicate ALPH chunk".to_string()));
    }
    if vp8l_count == 1 && alph_count > 0 {
        return Err(Error::Metadata(
            "ANMF ALPH paired with VP8L (VP8L has intrinsic alpha)".to_string(),
        ));
    }
    if vp8_count == 1 && alph_count > 0 {
        info.has_alpha = true;
    }
    info.image_kind = if vp8_count == 1 {
        Some(FrameImageKind::LossyVP8)
    } else {
        Some(FrameImageKind::LosslessVP8L)
    };
    if sub_pos != frame_data.len() {
        return Err(Error::Metadata(format!(
            "ANMF final cursor {} does not match frame end {}",
            sub_pos,
            frame_data.len()
        )));
    }
    Ok(info)
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

    match data.len().checked_sub(declared_end) {
        Some(0) => {}
        Some(1) if data[declared_end] == 0 => {}
        _ => {
            return Err(Error::Metadata(format!(
                "RIFF declared size {} does not match physical input length {}",
                riff_size,
                data.len().saturating_sub(8)
            )));
        }
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
                let w_raw = u32::from(data[data_start + 4])
                    | (u32::from(data[data_start + 5]) << 8)
                    | (u32::from(data[data_start + 6]) << 16);
                let h_raw = u32::from(data[data_start + 7])
                    | (u32::from(data[data_start + 8]) << 8)
                    | (u32::from(data[data_start + 9]) << 16);
                let w = w_raw
                    .checked_add(1)
                    .ok_or_else(|| Error::Metadata("VP8X width overflow".to_string()))?;
                let h = h_raw
                    .checked_add(1)
                    .ok_or_else(|| Error::Metadata("VP8X height overflow".to_string()))?;
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

        let padded = chunk_size
            .checked_add(chunk_size & 1)
            .ok_or_else(|| Error::Metadata("RIFF chunk alignment overflow".to_string()))?;
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
    if icc_indices.len() > 1 {
        return Err(Error::Metadata("Duplicate ICCP chunks".to_string()));
    }
    if exif_indices.len() > 1 {
        return Err(Error::Metadata("Duplicate EXIF chunks".to_string()));
    }
    if anim_indices.len() > 1 {
        return Err(Error::Metadata("Duplicate ANIM chunks".to_string()));
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

    let is_animated = !anim_indices.is_empty() || !anmf_indices.is_empty();
    if is_animated {
        if anim_indices.len() != 1 {
            return Err(Error::Metadata(
                "Animation coherence: exactly one ANIM chunk required".to_string(),
            ));
        }
        if anmf_indices.is_empty() {
            return Err(Error::Metadata(
                "Animation coherence: ANIM requires at least one ANMF frame".to_string(),
            ));
        }
        if !vp8_indices.is_empty() || !vp8l_indices.is_empty() {
            return Err(Error::Metadata(
                "Animation coherence: top-level VP8/VP8L forbidden when ANIM/ANMF present"
                    .to_string(),
            ));
        }
        if !alph_indices.is_empty() {
            return Err(Error::Metadata(
                "Animation coherence: top-level ALPH forbidden when ANIM/ANMF present".to_string(),
            ));
        }
        if vp8x_index.is_none() {
            return Err(Error::Metadata(
                "Animation coherence: animated WebP requires VP8X".to_string(),
            ));
        }
        for &idx in &anim_indices {
            let chunk = &chunks[idx];
            if chunk.data_len != 6 {
                return Err(Error::Metadata(format!(
                    "ANIM payload length {} invalid: expected exactly 6 bytes",
                    chunk.data_len
                )));
            }
        }
    } else {
        if !anim_indices.is_empty() || !anmf_indices.is_empty() {
            return Err(Error::Metadata(
                "Non-animated container cannot contain ANIM or ANMF chunks".to_string(),
            ));
        }
    }

    let has_xmp = !xmp_indices.is_empty();
    let has_exif = !exif_indices.is_empty();
    let has_icc = !icc_indices.is_empty();
    let has_animation = !anmf_indices.is_empty() || !anim_indices.is_empty();

    let mut has_alpha = false;
    for &idx in &vp8l_indices {
        let chunk = &chunks[idx];
        let payload_end = chunk.data_start + chunk.data_len;
        if payload_end <= data.len() {
            let header = parse_vp8l_header(&data[chunk.data_start..payload_end])?;
            if header.alpha {
                has_alpha = true;
                break;
            }
        }
    }
    let (canvas_width, canvas_height) = if let Some(idx) = vp8x_index {
        match vp8x_dimensions(data, chunks[idx].data_start) {
            Some((w, h)) => (w, h),
            None => (0, 0),
        }
    } else {
        (0, 0)
    };
    if !has_alpha {
        for &idx in &anmf_indices {
            let chunk = &chunks[idx];
            let frame_end = chunk.data_start + chunk.data_len;
            if frame_end > data.len() {
                return Err(Error::Metadata(format!(
                    "ANMF frame at chunk {} exceeds RIFF extent",
                    chunk.fourcc_str()
                )));
            }
            let frame_data = &data[chunk.data_start..frame_end];
            let info = parse_anmf_frame(frame_data, canvas_width, canvas_height)?;
            if info.has_alpha {
                has_alpha = true;
                break;
            }
        }
    }
    if !has_alpha && !alph_indices.is_empty() {
        has_alpha = true;
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WebPFeatures {
    pub icc: bool,
    pub alpha: bool,
    pub exif: bool,
    pub xmp: bool,
    pub animation: bool,
}

impl WebPFeatures {
    #[must_use]
    pub fn as_vp8x_flags(&self) -> u8 {
        let mut flags = 0u8;
        if self.icc {
            flags |= 0x20;
        }
        if self.alpha {
            flags |= 0x10;
        }
        if self.exif {
            flags |= 0x08;
        }
        if self.xmp {
            flags |= 0x04;
        }
        if self.animation {
            flags |= 0x02;
        }
        flags
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn with_xmp(mut self, on: bool) -> Self {
        self.xmp = on;
        self
    }
}

pub(crate) fn derive_features(parsed: &ParsedWebP) -> WebPFeatures {
    WebPFeatures {
        icc: parsed.has_icc,
        alpha: parsed.has_alpha,
        exif: parsed.has_exif,
        xmp: parsed.has_xmp,
        animation: parsed.has_animation,
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Vp8lHeader {
    pub width: u32,
    pub height: u32,
    pub alpha: bool,
}

pub(crate) fn parse_vp8l_header(payload: &[u8]) -> Result<Vp8lHeader> {
    if payload.len() < 5 {
        return Err(Error::Metadata("VP8L payload too short".to_string()));
    }
    if payload[0] != 0x2F {
        return Err(Error::Metadata("VP8L signature byte mismatch".to_string()));
    }
    let bits = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
    let width = (bits & 0x3FFF) + 1;
    let height = ((bits >> 14) & 0x3FFF) + 1;
    let alpha = ((bits >> 28) & 1) != 0;
    let version = (bits >> 29) & 0x7;
    if version != 0 {
        return Err(Error::Metadata(format!(
            "VP8L version {} is non-zero (only version 0 is supported)",
            version
        )));
    }
    Ok(Vp8lHeader {
        width,
        height,
        alpha,
    })
}

#[allow(dead_code)]
pub(crate) fn vp8l_has_alpha(payload: &[u8]) -> Result<bool> {
    parse_vp8l_header(payload).map(|h| h.alpha)
}

pub(crate) fn validate_webp_output(data: &[u8]) -> Result<()> {
    let parsed = parse_webp(data, None)?;

    let Some(vp8x_idx) = parsed.vp8x_index else {
        return Err(Error::Metadata(
            "Output WebP missing VP8X chunk".to_string(),
        ));
    };
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

    let derived = derive_features(&parsed);
    let expected_flags = derived.as_vp8x_flags();
    if flags != expected_flags {
        return Err(Error::Metadata(format!(
            "VP8X declared flags 0x{:02X} do not match derived features 0x{:02X}",
            flags, expected_flags
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

    if derived.xmp {
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
    fn parse_accepts_single_zero_byte_after_declared_riff_extent() {
        let mut data = make_simple_vp8_webp();
        data.push(0);
        assert!(parse_webp(&data, None).is_ok());

        data.push(0);
        assert!(parse_webp(&data, None).is_err());
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
        assert!(
            !parsed.has_alpha,
            "derived alpha must be false when no ALPH or alpha-bearing VP8L/ANMF present"
        );
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
    fn vp8l_header_parses_1x1_opaque() {
        let mut payload = vec![0x2Fu8, 0, 0, 0, 0];
        let bits = 0u32;
        payload[1..5].copy_from_slice(&bits.to_le_bytes());
        let header = parse_vp8l_header(&payload).expect("parse 1x1");
        assert_eq!(header.width, 1);
        assert_eq!(header.height, 1);
        assert!(!header.alpha);
    }

    #[test]
    fn vp8l_header_parses_dimensions_and_alpha() {
        let mut payload = vec![0x2Fu8, 0, 0, 0, 0];
        let bits: u32 = (1 << 28) | (4 << 14) | 8;
        payload[1..5].copy_from_slice(&bits.to_le_bytes());
        let header = parse_vp8l_header(&payload).expect("parse");
        assert_eq!(header.width, 9);
        assert_eq!(header.height, 5);
        assert!(header.alpha);
    }

    #[test]
    fn vp8l_header_rejects_nonzero_version() {
        let mut payload = vec![0x2Fu8, 0, 0, 0, 0];
        let bits: u32 = 1 << 31;
        payload[1..5].copy_from_slice(&bits.to_le_bytes());
        let result = parse_vp8l_header(&payload);
        assert!(result.is_err());
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

    fn webp_with_vp8x_and_vp8(
        vp8x_flags: u8,
        width_minus_one: u32,
        height_minus_one: u32,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let mut vp8x = vec![vp8x_flags, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        vp8x[4] = (width_minus_one & 0xFF) as u8;
        vp8x[5] = ((width_minus_one >> 8) & 0xFF) as u8;
        vp8x[6] = ((width_minus_one >> 16) & 0xFF) as u8;
        vp8x[7] = (height_minus_one & 0xFF) as u8;
        vp8x[8] = ((height_minus_one >> 8) & 0xFF) as u8;
        vp8x[9] = ((height_minus_one >> 16) & 0xFF) as u8;
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let vp8 = vec![0u8; 30];
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(vp8.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        data
    }

    #[test]
    fn vp8x_accepts_1x1_canvas() {
        let webp = webp_with_vp8x_and_vp8(0, 0, 0);
        let parsed = parse_webp(&webp, None).expect("1x1 canvas must parse");
        let vp8x_idx = parsed.vp8x_index.expect("has VP8X");
        let (w, h) = vp8x_dimensions(&webp, parsed.chunks[vp8x_idx].data_start).expect("dims");
        assert_eq!((w, h), (1, 1));
    }

    #[test]
    fn vp8x_accepts_1xn_and_nx1_canvas() {
        let webp_1xn = webp_with_vp8x_and_vp8(0, 0, 99);
        let parsed = parse_webp(&webp_1xn, None).expect("1xN must parse");
        let vp8x_idx = parsed.vp8x_index.expect("has VP8X");
        let dims = vp8x_dimensions(&webp_1xn, parsed.chunks[vp8x_idx].data_start).expect("dims");
        assert_eq!(dims, (1, 100));

        let webp_nx1 = webp_with_vp8x_and_vp8(0, 99, 0);
        let parsed = parse_webp(&webp_nx1, None).expect("Nx1 must parse");
        let vp8x_idx = parsed.vp8x_index.expect("has VP8X");
        let dims = vp8x_dimensions(&webp_nx1, parsed.chunks[vp8x_idx].data_start).expect("dims");
        assert_eq!(dims, (100, 1));
    }

    #[test]
    fn duplicate_anim_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x02u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let anim = vec![0u8; 6];
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&(anim.len() as u32).to_le_bytes());
        data.extend_from_slice(&anim);
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&(anim.len() as u32).to_le_bytes());
        data.extend_from_slice(&anim);
        let anmf_payload = vec![0u8; 16];
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&(anmf_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "duplicate ANIM must be rejected");
    }

    #[test]
    fn duplicate_iccp_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x20u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let icc = vec![0u8; 4];
        data.extend_from_slice(b"ICCP");
        data.extend_from_slice(&(icc.len() as u32).to_le_bytes());
        data.extend_from_slice(&icc);
        data.extend_from_slice(b"ICCP");
        data.extend_from_slice(&(icc.len() as u32).to_le_bytes());
        data.extend_from_slice(&icc);
        let vp8 = vec![0u8; 30];
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(vp8.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "duplicate ICCP must be rejected");
    }

    #[test]
    fn duplicate_exif_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x08u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let exif = vec![0u8; 4];
        data.extend_from_slice(b"EXIF");
        data.extend_from_slice(&(exif.len() as u32).to_le_bytes());
        data.extend_from_slice(&exif);
        data.extend_from_slice(b"EXIF");
        data.extend_from_slice(&(exif.len() as u32).to_le_bytes());
        data.extend_from_slice(&exif);
        let vp8 = vec![0u8; 30];
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(vp8.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "duplicate EXIF must be rejected");
    }

    #[test]
    fn anim_without_anmf_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x02u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let anim = vec![0u8; 6];
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&(anim.len() as u32).to_le_bytes());
        data.extend_from_slice(&anim);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "ANIM without ANMF must be rejected");
    }

    #[test]
    fn anmf_without_anim_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x02u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let anmf_payload = vec![0u8; 16];
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&(anmf_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "ANMF without ANIM must be rejected");
    }

    #[test]
    fn anmf_short_header_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x02u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let anim = vec![0u8; 6];
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&(anim.len() as u32).to_le_bytes());
        data.extend_from_slice(&anim);
        let anmf_payload = vec![0u8; 8];
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&(anmf_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "short ANMF header must be rejected");
    }

    #[test]
    fn anmf_missing_image_payload_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x02u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let anim = vec![0u8; 6];
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&(anim.len() as u32).to_le_bytes());
        data.extend_from_slice(&anim);
        let anmf_payload = vec![0u8; 16];
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&(anmf_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "ANMF missing image must be rejected");
    }

    #[test]
    fn anmf_duplicate_vp8_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x02u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let anim = vec![0u8; 6];
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&(anim.len() as u32).to_le_bytes());
        data.extend_from_slice(&anim);
        let mut anmf_payload = vec![0u8; 16];
        let vp8 = vec![0u8; 4];
        let extra_len = 8 + vp8.len() + 8 + vp8.len();
        let anmf_size = (anmf_payload.len() + extra_len) as u32;
        anmf_payload.extend(std::iter::repeat_n(0u8, extra_len));
        let mut buf = Vec::new();
        buf.extend_from_slice(b"ANMF");
        buf.extend_from_slice(&anmf_size.to_le_bytes());
        buf.extend_from_slice(&anmf_payload);
        buf.extend_from_slice(b"VP8 ");
        buf.extend_from_slice(&(vp8.len() as u32).to_le_bytes());
        buf.extend_from_slice(&vp8);
        buf.extend_from_slice(b"VP8 ");
        buf.extend_from_slice(&(vp8.len() as u32).to_le_bytes());
        buf.extend_from_slice(&vp8);
        let riff_size = (buf.len() - 8) as u32;
        buf[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&buf, None);
        assert!(result.is_err(), "duplicate VP8 in ANMF must be rejected");
    }

    #[test]
    fn anmf_vp8_and_vp8l_conflict_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x02u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let anim = vec![0u8; 6];
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&(anim.len() as u32).to_le_bytes());
        data.extend_from_slice(&anim);
        let anmf_payload = vec![0u8; 16];
        let vp8 = vec![0u8; 4];
        let vp8l = vec![0x2Fu8, 0x00, 0x00, 0x00, 0x00];
        let anmf_size = (anmf_payload.len() + 8 + vp8.len() + 8 + vp8l.len()) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(vp8.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8);
        data.extend_from_slice(b"VP8L");
        data.extend_from_slice(&(vp8l.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8l);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "ANMF VP8+VP8L conflict must be rejected");
    }

    #[test]
    fn anmf_alph_with_vp8l_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x02u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let anim = vec![0u8; 6];
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&(anim.len() as u32).to_le_bytes());
        data.extend_from_slice(&anim);
        let anmf_payload = vec![0u8; 16];
        let vp8l = vec![0x2Fu8, 0x00, 0x00, 0x00, 0x00];
        let alph = vec![0u8; 4];
        let anmf_size = (anmf_payload.len() + 8 + vp8l.len() + 8 + alph.len()) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8L");
        data.extend_from_slice(&(vp8l.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8l);
        data.extend_from_slice(b"ALPH");
        data.extend_from_slice(&(alph.len() as u32).to_le_bytes());
        data.extend_from_slice(&alph);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(
            result.is_err(),
            "ANMF ALPH with VP8L must be rejected (VP8L has intrinsic alpha)"
        );
    }

    #[test]
    fn valid_opaque_vp8_frame_accepted() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x02u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let anim = vec![0u8; 6];
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&(anim.len() as u32).to_le_bytes());
        data.extend_from_slice(&anim);
        let anmf_payload = vec![0u8; 16];
        let vp8 = vec![0u8; 4];
        let anmf_size = (anmf_payload.len() + 8 + vp8.len()) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(vp8.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let parsed = parse_webp(&data, None).expect("valid animation should parse");
        assert!(parsed.has_animation);
    }

    #[test]
    fn valid_alpha_vp8l_frame_accepted() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let vp8x = vec![0x02u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x);
        let anim = vec![0u8; 6];
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&(anim.len() as u32).to_le_bytes());
        data.extend_from_slice(&anim);
        let anmf_payload = vec![0u8; 16];
        let mut vp8l = vec![0x2Fu8, 0, 0, 0, 0];
        let bits: u32 = 1 << 28;
        vp8l[1..5].copy_from_slice(&bits.to_le_bytes());
        let vp8l_padded = vp8l.len() + (vp8l.len() & 1);
        let anmf_size = (anmf_payload.len() + 8 + vp8l_padded) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8L");
        data.extend_from_slice(&(vp8l.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8l);
        if vp8l.len() & 1 != 0 {
            data.push(0);
        }
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let parsed = parse_webp(&data, None).expect("valid alpha VP8L animation should parse");
        assert!(parsed.has_animation);
        assert!(parsed.has_alpha);
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
        data.extend_from_slice(&6u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 6]);
        let mut anmf_payload = vec![0u8; 16];

        anmf_payload[12] = 0x20;
        let vp8_data = vec![0u8; 4];
        let alph_data = vec![0u8; 4];
        let vp8_chunk = 8 + vp8_data.len();
        let alph_chunk = 8 + alph_data.len();
        let anmf_size = (16 + vp8_chunk + alph_chunk) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(vp8_data.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8_data);
        data.extend_from_slice(b"ALPH");
        data.extend_from_slice(&(alph_data.len() as u32).to_le_bytes());
        data.extend_from_slice(&alph_data);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let parsed = parse_webp(&data, None).unwrap();
        assert!(
            parsed.has_alpha,
            "ANMF with VP8 + ALPH should set has_alpha"
        );
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
        data.extend_from_slice(&6u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 6]);
        let mut anmf_payload = vec![0u8; 16];
        anmf_payload[12] = 0x20;
        let mut vp8l_payload = vec![0x2Fu8, 0, 0, 0, 0, 0];
        let bits: u32 = 1 << 28;
        vp8l_payload[1..5].copy_from_slice(&bits.to_le_bytes());
        let vp8l_padded = vp8l_payload.len() + (vp8l_payload.len() & 1);
        let anmf_size = (16 + 8 + vp8l_padded) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8L");
        data.extend_from_slice(&(vp8l_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8l_payload);
        if vp8l_payload.len() & 1 != 0 {
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
        data.extend_from_slice(&6u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 6]);
        let mut anmf_payload = vec![0u8; 16];
        anmf_payload[12] = 0x20;
        let vp8_data = vec![0u8; 4];
        let vp8_padded = vp8_data.len();
        let anmf_size = (16 + 8 + vp8_padded) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
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

    fn build_anmf_with_frame(
        canvas_width: u32,
        canvas_height: u32,
        anmf_payload: [u8; 16],
        nested: &[(&[u8; 4], &[u8])],
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let mut vp8x_payload = vec![0u8; 10];
        vp8x_payload[0] = 0x02;
        vp8x_payload[4] = (canvas_width - 1) as u8;
        vp8x_payload[7] = (canvas_height - 1) as u8;
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        data.extend_from_slice(b"ANIM");
        data.extend_from_slice(&6u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 6]);
        let mut nested_total = 0;
        for (_fcc, payload) in nested {
            nested_total += 8 + payload.len() + (payload.len() & 1);
        }
        let anmf_size = (16 + nested_total) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        for (fcc, payload) in nested {
            data.extend_from_slice(*fcc);
            data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            data.extend_from_slice(payload);
            if payload.len() & 1 != 0 {
                data.push(0);
            }
        }
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        data
    }

    fn anmf_frame_from_data(
        parsed: &ParsedWebP,
        data: &[u8],
        canvas_width: u32,
        canvas_height: u32,
    ) -> AnmfFrameInfo {
        let idx = parsed.anmf_indices[0];
        let start = parsed.chunks[idx].data_start;
        let end = start + parsed.chunks[idx].data_len;
        parse_anmf_frame(&data[start..end], canvas_width, canvas_height).unwrap()
    }

    #[test]
    fn anmf_decodes_x_y_width_height_duration_and_flags() {
        let mut anmf_payload = [0u8; 16];
        anmf_payload[0] = 4;
        anmf_payload[3] = 2;
        anmf_payload[6] = 9;
        anmf_payload[9] = 7;
        anmf_payload[12] = 0x64;
        anmf_payload[15] = 0x02;
        let vp8 = vec![0u8; 4];
        let data = build_anmf_with_frame(64, 64, anmf_payload, &[(b"VP8 ", &vp8)]);
        let parsed = parse_webp(&data, None).unwrap();
        assert!(parsed.has_animation);
        let info = anmf_frame_from_data(&parsed, &data, 64, 64);
        assert_eq!(info.x, 8);
        assert_eq!(info.y, 4);
        assert_eq!(info.width, 10);
        assert_eq!(info.height, 8);
        assert_eq!(info.duration_ms, 100);
        assert!(info.blend);
        assert!(!info.dispose);
    }

    #[test]
    fn anmf_reserved_flag_bits_rejected() {
        let mut anmf_payload = [0u8; 16];
        anmf_payload[15] = 0x04;
        let vp8 = vec![0u8; 4];
        let data = build_anmf_with_frame(4, 4, anmf_payload, &[(b"VP8 ", &vp8)]);
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "reserved flag bits must be rejected");
    }

    #[test]
    fn anmf_1x1_frame_accepted() {
        let anmf_payload = [0u8; 16];
        let vp8 = vec![0u8; 4];
        let data = build_anmf_with_frame(4, 4, anmf_payload, &[(b"VP8 ", &vp8)]);
        let parsed = parse_webp(&data, None).unwrap();
        let info = anmf_frame_from_data(&parsed, &data, 4, 4);
        assert_eq!(info.width, 1);
        assert_eq!(info.height, 1);
    }

    #[test]
    fn anmf_frame_right_edge_exactly_on_canvas_accepted() {
        let mut anmf_payload = [0u8; 16];
        anmf_payload[0] = 5;
        anmf_payload[6] = 9;
        let vp8 = vec![0u8; 4];
        let data = build_anmf_with_frame(20, 20, anmf_payload, &[(b"VP8 ", &vp8)]);
        let parsed = parse_webp(&data, None).unwrap();
        let info = anmf_frame_from_data(&parsed, &data, 20, 20);
        assert_eq!(info.x, 10);
        assert_eq!(info.width, 10);
        assert_eq!(info.x + info.width, 20);
    }

    #[test]
    fn anmf_frame_bottom_edge_exactly_on_canvas_accepted() {
        let mut anmf_payload = [0u8; 16];
        anmf_payload[3] = 5;
        anmf_payload[9] = 9;
        let vp8 = vec![0u8; 4];
        let data = build_anmf_with_frame(20, 20, anmf_payload, &[(b"VP8 ", &vp8)]);
        let parsed = parse_webp(&data, None).unwrap();
        let info = anmf_frame_from_data(&parsed, &data, 20, 20);
        assert_eq!(info.y, 10);
        assert_eq!(info.height, 10);
        assert_eq!(info.y + info.height, 20);
    }

    #[test]
    fn anmf_frame_right_outside_canvas_rejected() {
        let mut anmf_payload = [0u8; 16];
        anmf_payload[0] = 5;
        anmf_payload[6] = 9;
        let vp8 = vec![0u8; 4];
        let data = build_anmf_with_frame(8, 8, anmf_payload, &[(b"VP8 ", &vp8)]);
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "frame outside canvas must be rejected");
    }

    #[test]
    fn anmf_frame_bottom_outside_canvas_rejected() {
        let mut anmf_payload = [0u8; 16];
        anmf_payload[3] = 5;
        anmf_payload[9] = 9;
        let vp8 = vec![0u8; 4];
        let data = build_anmf_with_frame(8, 8, anmf_payload, &[(b"VP8 ", &vp8)]);
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "frame outside canvas must be rejected");
    }

    #[test]
    fn anmf_coordinate_multiply_overflow_rejected() {
        let mut anmf_payload = [0u8; 16];
        anmf_payload[0] = 0xFF;
        anmf_payload[1] = 0xFF;
        anmf_payload[2] = 0x7F;
        let vp8 = vec![0u8; 4];
        let data = build_anmf_with_frame(65535, 65535, anmf_payload, &[(b"VP8 ", &vp8)]);
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "coordinate overflow must be rejected");
    }

    #[test]
    fn anmf_nested_chunk_overrun_rejected() {
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
        data.extend_from_slice(&6u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 6]);
        let anmf_payload = [0u8; 16];
        let vp8 = vec![0u8; 4];
        let anmf_size = (16 + 8 + vp8.len()) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&16u32.to_le_bytes());
        data.extend_from_slice(&vp8);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "chunk overrun must be rejected");
    }

    #[test]
    fn anmf_missing_pad_rejected() {
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
        data.extend_from_slice(&6u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 6]);
        let anmf_payload = [0u8; 16];
        let vp8 = vec![0u8; 3];
        let anmf_size = (16 + 8 + vp8.len() + 1) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(vp8.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "missing pad byte must be rejected");
    }

    #[test]
    fn anmf_final_cursor_mismatch_rejected() {
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
        data.extend_from_slice(&6u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 6]);
        let anmf_payload = [0u8; 16];
        let vp8 = vec![0u8; 4];
        let anmf_size = (16 + 8 + vp8.len() + 1) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(vp8.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8);
        data.push(0);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "final cursor mismatch must be rejected");
    }

    #[test]
    fn anmf_missing_image_rejected() {
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
        data.extend_from_slice(&6u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 6]);
        let anmf_payload = [0u8; 16];
        let anmf_size = 16u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "ANMF without image must be rejected");
    }

    #[test]
    fn anmf_duplicate_vp8l_rejected() {
        let vp8l1 = vec![0x2Fu8, 0, 0, 0, 0];
        let vp8l2 = vec![0x2Fu8, 0, 0, 0, 0];
        let data = build_anmf_with_frame(4, 4, [0u8; 16], &[(b"VP8L", &vp8l1), (b"VP8L", &vp8l2)]);
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "duplicate VP8L must be rejected");
    }

    #[test]
    fn anmf_vp8l_then_alph_rejected() {
        let vp8l = vec![0x2Fu8, 0, 0, 0, 0];
        let alph = vec![0u8; 4];
        let data = build_anmf_with_frame(4, 4, [0u8; 16], &[(b"VP8L", &vp8l), (b"ALPH", &alph)]);
        let result = parse_webp(&data, None);
        assert!(
            result.is_err(),
            "VP8L+ALPH in either order must be rejected"
        );
    }

    #[test]
    fn animated_top_level_alph_rejected() {
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
        data.extend_from_slice(&6u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 6]);
        let alph = vec![0u8; 4];
        data.extend_from_slice(b"ALPH");
        data.extend_from_slice(&(alph.len() as u32).to_le_bytes());
        data.extend_from_slice(&alph);
        let anmf_payload = [0u8; 16];
        let vp8 = vec![0u8; 4];
        let anmf_size = (16 + 8 + vp8.len()) as u32;
        data.extend_from_slice(b"ANMF");
        data.extend_from_slice(&anmf_size.to_le_bytes());
        data.extend_from_slice(&anmf_payload);
        data.extend_from_slice(b"VP8 ");
        data.extend_from_slice(&(vp8.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(
            result.is_err(),
            "top-level ALPH in animated container must be rejected"
        );
    }

    #[test]
    fn malformed_top_level_vp8l_header_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"WEBP");
        let mut vp8x_payload = vec![0u8; 10];
        vp8x_payload[0] = 0x12;
        vp8x_payload[4] = 0x01;
        vp8x_payload[7] = 0x01;
        data.extend_from_slice(b"VP8X");
        data.extend_from_slice(&(vp8x_payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8x_payload);
        let vp8l = vec![0u8; 5];
        data.extend_from_slice(b"VP8L");
        data.extend_from_slice(&(vp8l.len() as u32).to_le_bytes());
        data.extend_from_slice(&vp8l);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "malformed top-level VP8L must fail");
    }

    #[test]
    fn malformed_nested_vp8l_header_rejected() {
        let vp8 = vec![0u8; 4];
        let vp8l_bad = vec![0u8; 5];
        let data = build_anmf_with_frame(4, 4, [0u8; 16], &[(b"VP8 ", &vp8), (b"VP8L", &vp8l_bad)]);
        let result = parse_webp(&data, None);
        assert!(result.is_err(), "malformed nested VP8L must fail");
    }
}
