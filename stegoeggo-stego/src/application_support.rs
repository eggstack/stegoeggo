use crate::error::{StegoError, StegoResult};
use crate::jpeg;
use crate::lsb_internal;
use crate::types::{EmbedOutcome, EmbedPath};
use image::RgbaImage;

pub fn legacy_lsb_required_slots(payload_bits: usize) -> usize {
    lsb_internal::lsb_required_slots_legacy(payload_bits)
}

pub fn corrected_lsb_embed(
    image: &RgbaImage,
    payload: &[u8],
    seed: u64,
    redundancy: usize,
) -> EmbedOutcome<RgbaImage> {
    lsb_internal::embed_lsb_v2(image, payload, seed, redundancy)
}

pub fn corrected_lsb_extract(
    image: &RgbaImage,
    expected_bits: usize,
    seed: u64,
    base_slot: usize,
    redundancy: usize,
) -> Option<Vec<u8>> {
    lsb_internal::extract_lsb_v2(image, expected_bits, seed, base_slot, redundancy)
}

pub fn legacy_lsb_extract(image: &RgbaImage, expected_bits: usize, seed: u64) -> Option<Vec<u8>> {
    lsb_internal::extract_lsb(image, expected_bits, seed)
}

pub fn legacy_lsb_extract_range(
    image: &RgbaImage,
    expected_bits: usize,
    offset: usize,
    count: usize,
    seed: u64,
) -> Option<Vec<u8>> {
    lsb_internal::extract_lsb_range(image, expected_bits, offset, count, seed)
}

pub fn tiled_lsb_embed(
    image: &RgbaImage,
    payload: &[u8],
    seed: u64,
    tile_size: u32,
) -> EmbedOutcome<RgbaImage> {
    lsb_internal::embed_lsb_tiled(image, payload, seed, tile_size)
}

pub fn seed_fallback_embed(image: &mut RgbaImage, seed: u64) {
    lsb_internal::embed_seed_lsb_fallback(image, seed);
}

pub fn seed_fallback_extract(image: &RgbaImage) -> Option<u64> {
    lsb_internal::extract_seed_lsb_fallback(image)
}

pub fn tile_seed(master_seed: u64, tile_x: u32, tile_y: u32) -> u64 {
    lsb_internal::tile_seed(master_seed, tile_x, tile_y)
}

pub fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    lsb_internal::bits_to_bytes(bits)
}

pub fn crop_image_region(image: &RgbaImage, x: u32, y: u32, width: u32, height: u32) -> RgbaImage {
    lsb_internal::crop_rgba(image, x, y, width, height)
}

pub fn jpeg_embed(
    jpeg_bytes: &[u8],
    payload: &[u8],
    seed: u64,
    redundancy: usize,
) -> StegoResult<EmbedOutcome<Vec<u8>>> {
    let config = jpeg::JpegConfig::new(seed).with_redundancy(redundancy);
    let report = jpeg::embed(jpeg_bytes, payload, &config)?;
    let path = EmbedPath::DctF5;
    if report.embedded {
        Ok(EmbedOutcome::Embedded {
            output: report.output,
            payload_bytes: report.payload_bytes,
            required_capacity: report.required_capacity,
            available_capacity: report.available_capacity,
            path,
        })
    } else {
        Ok(EmbedOutcome::SkippedCapacity {
            output: report.output,
            payload_bytes: report.payload_bytes,
            required_capacity: report.required_capacity,
            available_capacity: report.available_capacity,
            path,
        })
    }
}

pub fn jpeg_extract(
    jpeg_bytes: &[u8],
    payload_len: usize,
    seed: u64,
    redundancy: usize,
) -> StegoResult<Vec<u8>> {
    let config = jpeg::JpegConfig::new(seed).with_redundancy(redundancy);
    jpeg::extract(jpeg_bytes, payload_len, &config, redundancy)
}

pub fn jpeg_embed_tiled(
    jpeg_bytes: &[u8],
    payload: &[u8],
    seed: u64,
    tile_size: u32,
) -> StegoResult<EmbedOutcome<Vec<u8>>> {
    if tile_size == 0 {
        return Err(StegoError::InvalidConfig(
            "tile size must be non-zero".into(),
        ));
    }
    jpeg::probe_support(jpeg_bytes)?;
    let (header, mut coefficients) =
        crate::jpeg_transcoder::JpegTranscoder::decode_coefficients(jpeg_bytes)
            .map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    let mut header = header;
    crate::jpeg_transcoder::DctStegoF5::new()
        .embed_seed_in_quantization_tables(&mut header, seed)
        .map_err(|e| StegoError::MalformedInput(e.to_string()))?;

    let max_h = header
        .components
        .iter()
        .map(|c| c.h_sampling as u32)
        .max()
        .unwrap_or(1);
    let max_v = header
        .components
        .iter()
        .map(|c| c.v_sampling as u32)
        .max()
        .unwrap_or(1);
    let luma_blocks_x = (header.width as u32 + max_h * 7) / (max_h * 8);
    let luma_blocks_y = (header.height as u32 + max_v * 7) / (max_v * 8);
    let blocks_per_tile = tile_size / 8;
    if blocks_per_tile == 0 {
        return Err(StegoError::InvalidConfig(
            "tile size must be at least 8".into(),
        ));
    }
    let tiles_x = luma_blocks_x / blocks_per_tile;
    let tiles_y = luma_blocks_y / blocks_per_tile;
    let payload_bits = payload.len().saturating_mul(8);
    let mut embedded_any = false;

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let tile_blocks = crate::jpeg_transcoder::DctStegoF5::tile_block_set(
                &header,
                &coefficients,
                tx,
                ty,
                tile_size,
            );
            if tile_blocks.is_empty() {
                continue;
            }
            if crate::jpeg_transcoder::DctStegoF5::with_redundancy(1)
                .embed_f5_in_blocks(
                    &mut coefficients,
                    payload,
                    crate::lsb_internal::tile_seed(seed, tx, ty),
                    &tile_blocks,
                )
                .is_ok()
            {
                embedded_any = true;
            }
        }
    }

    let output = crate::jpeg_transcoder::JpegTranscoder::encode_coefficients(
        &header,
        &coefficients,
        Some(jpeg_bytes),
    )
    .map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    if !embedded_any {
        return Ok(EmbedOutcome::SkippedCapacity {
            output,
            payload_bytes: payload.len(),
            required_capacity: payload_bits,
            available_capacity: 0,
            path: EmbedPath::DctF5Tiled,
        });
    }

    Ok(EmbedOutcome::Embedded {
        output,
        payload_bytes: payload.len(),
        required_capacity: payload_bits,
        available_capacity: payload_bits,
        path: EmbedPath::DctF5Tiled,
    })
}

pub fn jpeg_extract_tiled_candidates(
    jpeg_bytes: &[u8],
    master_seed: u64,
    tile_size: u32,
    max_origins: u32,
    payload_bits: usize,
) -> Option<Vec<Vec<u8>>> {
    if tile_size == 0 || max_origins == 0 {
        return None;
    }
    let (header, coefficients) =
        crate::jpeg_transcoder::JpegTranscoder::decode_coefficients(jpeg_bytes).ok()?;
    let max_h = header
        .components
        .iter()
        .map(|c| c.h_sampling as u32)
        .max()
        .unwrap_or(1);
    let max_v = header
        .components
        .iter()
        .map(|c| c.v_sampling as u32)
        .max()
        .unwrap_or(1);
    let luma_blocks_x = (header.width as u32 + max_h * 7) / (max_h * 8);
    let luma_blocks_y = (header.height as u32 + max_v * 7) / (max_v * 8);
    let blocks_per_tile = tile_size / 8;
    if blocks_per_tile == 0 {
        return None;
    }
    let tiles_x = luma_blocks_x / blocks_per_tile;
    let tiles_y = luma_blocks_y / blocks_per_tile;
    let mut origins_tried = 0u32;
    let mut candidates = Vec::new();

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            if origins_tried >= max_origins {
                return (!candidates.is_empty()).then_some(candidates);
            }
            origins_tried += 1;
            let tile_blocks = crate::jpeg_transcoder::DctStegoF5::tile_block_set(
                &header,
                &coefficients,
                tx,
                ty,
                tile_size,
            );
            if tile_blocks.is_empty() {
                continue;
            }
            for dy in 0..=2u32 {
                if ty + dy >= 16 {
                    break;
                }
                for dx in 0..=2u32 {
                    if tx + dx >= 16 {
                        break;
                    }
                    let local_seed = lsb_internal::tile_seed(master_seed, tx + dx, ty + dy);
                    for redundancy in 1..=10 {
                        let bits = crate::jpeg_transcoder::DctStegoF5::with_redundancy(redundancy)
                            .extract_f5_from_blocks(
                                &coefficients,
                                payload_bits,
                                local_seed,
                                &tile_blocks,
                            );
                        if bits.len() >= payload_bits {
                            let bytes = lsb_internal::bits_to_bytes(&bits);
                            if !candidates.iter().any(|candidate| candidate == &bytes) {
                                candidates.push(bytes);
                            }
                        }
                    }
                }
            }
        }
    }
    (!candidates.is_empty()).then_some(candidates)
}
