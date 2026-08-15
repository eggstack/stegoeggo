use crate::error::{StegoError, StegoResult};
use crate::jpeg;
use crate::lsb_internal;
use crate::types::{EmbedOutcome, EmbedPath, InPlaceEmbedReport};
use image::RgbaImage;

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static TILED_SEARCH_DECODE_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
fn record_tiled_search_decode() {
    TILED_SEARCH_DECODE_COUNT.with(|count| count.set(count.get() + 1));
}

#[cfg(test)]
fn reset_tiled_search_decode_count() {
    TILED_SEARCH_DECODE_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
fn tiled_search_decode_count() -> usize {
    TILED_SEARCH_DECODE_COUNT.with(Cell::get)
}

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

pub fn corrected_lsb_embed_in_place(
    image: &mut RgbaImage,
    payload: &[u8],
    seed: u64,
    redundancy: usize,
) -> StegoResult<InPlaceEmbedReport> {
    if image.dimensions() == (0, 0) {
        return Err(StegoError::EmptyCarrier);
    }
    Ok(lsb_internal::embed_lsb_v2_in_place(
        image, payload, seed, redundancy,
    ))
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
    let mut first_embedded: Option<(u32, u32, u64)> = None;

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
                first_embedded.get_or_insert((
                    tx,
                    ty,
                    crate::lsb_internal::tile_seed(seed, tx, ty),
                ));
            }
        }
    }

    let output = crate::jpeg_transcoder::JpegTranscoder::encode_coefficients(
        &header,
        &coefficients,
        Some(jpeg_bytes),
    )
    .map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    let Some((tile_x, tile_y, local_seed)) = first_embedded else {
        return Ok(EmbedOutcome::SkippedCapacity {
            output,
            payload_bytes: payload.len(),
            required_capacity: payload_bits,
            available_capacity: 0,
            path: EmbedPath::DctF5Tiled,
        });
    };

    jpeg_tiled_outcome_after_verification(output, payload, tile_size, tile_x, tile_y, local_seed)
}

fn jpeg_tiled_outcome_after_verification(
    output: Vec<u8>,
    payload: &[u8],
    tile_size: u32,
    tile_x: u32,
    tile_y: u32,
    local_seed: u64,
) -> StegoResult<EmbedOutcome<Vec<u8>>> {
    let payload_bits = payload.len().saturating_mul(8);
    if !jpeg_tiled_payload_matches(&output, payload, tile_size, tile_x, tile_y, local_seed)? {
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

fn jpeg_tiled_payload_matches(
    jpeg_bytes: &[u8],
    payload: &[u8],
    tile_size: u32,
    tile_x: u32,
    tile_y: u32,
    local_seed: u64,
) -> StegoResult<bool> {
    let (header, coefficients) =
        crate::jpeg_transcoder::JpegTranscoder::decode_coefficients(jpeg_bytes)
            .map_err(|e| StegoError::MalformedInput(e.to_string()))?;
    let tile_blocks = crate::jpeg_transcoder::DctStegoF5::tile_block_set(
        &header,
        &coefficients,
        tile_x,
        tile_y,
        tile_size,
    );
    let payload_bits = payload.len().saturating_mul(8);
    let bits = crate::jpeg_transcoder::DctStegoF5::with_redundancy(1).extract_f5_from_blocks(
        &coefficients,
        payload_bits,
        local_seed,
        &tile_blocks,
    );
    Ok(bits.len() >= payload_bits && lsb_internal::bits_to_bytes(&bits) == payload)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TiledJpegCandidateKey {
    tile_x: u32,
    tile_y: u32,
    seed_x: u32,
    seed_y: u32,
    redundancy: u8,
}

pub struct TiledJpegSearch {
    header: crate::jpeg_transcoder::JpegHeader,
    coefficients: crate::jpeg_transcoder::Coefficients,
    tile_size: u32,
    tiles_x: u32,
    tiles_y: u32,
}

fn decode_tiled_search_coefficients(
    jpeg_bytes: &[u8],
) -> crate::jpeg_transcoder::Result<(
    crate::jpeg_transcoder::JpegHeader,
    crate::jpeg_transcoder::Coefficients,
)> {
    #[cfg(test)]
    record_tiled_search_decode();
    crate::jpeg_transcoder::JpegTranscoder::decode_coefficients(jpeg_bytes)
}

impl TiledJpegSearch {
    pub fn new(jpeg_bytes: &[u8], tile_size: u32) -> Option<Self> {
        if tile_size == 0 {
            return None;
        }
        let (header, coefficients) = decode_tiled_search_coefficients(jpeg_bytes).ok()?;
        let (tiles_x, tiles_y) = tile_geometry(&header, tile_size)?;
        Some(Self {
            header,
            coefficients,
            tile_size,
            tiles_x,
            tiles_y,
        })
    }

    pub fn prefix_candidates(
        &self,
        master_seed: u64,
        max_origins: u32,
        prefix_bits: usize,
    ) -> Vec<(TiledJpegCandidateKey, Vec<u8>)> {
        if max_origins == 0 {
            return Vec::new();
        }
        let mut origins_tried = 0u32;
        let mut candidates = Vec::new();

        for ty in 0..self.tiles_y {
            for tx in 0..self.tiles_x {
                if origins_tried >= max_origins {
                    return candidates;
                }
                origins_tried += 1;
                let tile_blocks = crate::jpeg_transcoder::DctStegoF5::tile_block_set(
                    &self.header,
                    &self.coefficients,
                    tx,
                    ty,
                    self.tile_size,
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
                            let bits =
                                crate::jpeg_transcoder::DctStegoF5::with_redundancy(redundancy)
                                    .extract_f5_from_blocks(
                                        &self.coefficients,
                                        prefix_bits,
                                        local_seed,
                                        &tile_blocks,
                                    );
                            if bits.len() >= prefix_bits {
                                let bytes = lsb_internal::bits_to_bytes(&bits);
                                candidates.push((
                                    TiledJpegCandidateKey {
                                        tile_x: tx,
                                        tile_y: ty,
                                        seed_x: tx + dx,
                                        seed_y: ty + dy,
                                        redundancy: redundancy as u8,
                                    },
                                    bytes,
                                ));
                            }
                        }
                    }
                }
            }
        }
        candidates
    }

    pub fn extract_candidate(
        &self,
        master_seed: u64,
        candidate: TiledJpegCandidateKey,
        payload_bits: usize,
    ) -> Option<Vec<u8>> {
        if !self.valid_candidate(candidate) {
            return None;
        }
        let tile_blocks = crate::jpeg_transcoder::DctStegoF5::tile_block_set(
            &self.header,
            &self.coefficients,
            candidate.tile_x,
            candidate.tile_y,
            self.tile_size,
        );
        if tile_blocks.is_empty() {
            return None;
        }
        let local_seed = lsb_internal::tile_seed(master_seed, candidate.seed_x, candidate.seed_y);
        let bits =
            crate::jpeg_transcoder::DctStegoF5::with_redundancy(candidate.redundancy as usize)
                .extract_f5_from_blocks(&self.coefficients, payload_bits, local_seed, &tile_blocks);
        (bits.len() >= payload_bits).then(|| lsb_internal::bits_to_bytes(&bits))
    }

    fn valid_candidate(&self, candidate: TiledJpegCandidateKey) -> bool {
        candidate.tile_x < self.tiles_x
            && candidate.tile_y < self.tiles_y
            && candidate.seed_x < 16
            && candidate.seed_y < 16
            && (1..=10).contains(&candidate.redundancy)
    }
}

fn tile_geometry(
    header: &crate::jpeg_transcoder::JpegHeader,
    tile_size: u32,
) -> Option<(u32, u32)> {
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
    (blocks_per_tile > 0).then(|| {
        (
            luma_blocks_x / blocks_per_tile,
            luma_blocks_y / blocks_per_tile,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    const MASTER_SEED: u64 = 42;
    const TILE_SIZE: u32 = 64;

    fn test_jpeg() -> Vec<u8> {
        let image = ImageBuffer::<Rgb<u8>, _>::from_fn(256, 256, |x, y| {
            Rgb([(x as u8).wrapping_mul(3), (y as u8).wrapping_mul(5), 128])
        });
        let mut output = Vec::new();
        image::DynamicImage::ImageRgb8(image)
            .write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut output,
                95,
            ))
            .expect("test JPEG should encode");
        output
    }

    fn offset_seed_fixture(payload: &[u8]) -> Vec<u8> {
        let jpeg_bytes = test_jpeg();
        let (header, mut coefficients) =
            crate::jpeg_transcoder::JpegTranscoder::decode_coefficients(&jpeg_bytes)
                .expect("test JPEG should decode");
        let tile_blocks = crate::jpeg_transcoder::DctStegoF5::tile_block_set(
            &header,
            &coefficients,
            0,
            0,
            TILE_SIZE,
        );
        crate::jpeg_transcoder::DctStegoF5::with_redundancy(1)
            .embed_f5_in_blocks(
                &mut coefficients,
                payload,
                tile_seed(MASTER_SEED, 1, 0),
                &tile_blocks,
            )
            .expect("offset tile should have capacity");
        crate::jpeg_transcoder::JpegTranscoder::encode_coefficients(
            &header,
            &coefficients,
            Some(&jpeg_bytes),
        )
        .expect("test JPEG should encode coefficients")
    }

    #[test]
    fn tiled_jpeg_candidate_key_roundtrips_same_identity() {
        let payload = vec![0xA5; 36];
        let embedded = jpeg_embed_tiled(&test_jpeg(), &payload, MASTER_SEED, TILE_SIZE)
            .expect("tiled embed should succeed");
        assert!(embedded.is_embedded());
        let search =
            TiledJpegSearch::new(embedded.output(), TILE_SIZE).expect("tiled search should decode");
        let candidates = search.prefix_candidates(MASTER_SEED, 1, 48);
        assert!(candidates.len() > 1);
        for (key, prefix) in candidates {
            assert_eq!(
                search
                    .extract_candidate(MASTER_SEED, key, 48)
                    .expect("same candidate should extract"),
                prefix
            );
        }
    }

    #[test]
    fn tiled_jpeg_wrong_first_candidate_does_not_mask_later_v3() {
        reset_tiled_search_decode_count();
        let payload = [
            0x53, 0x45, 3, 6, 36, 0, 0xA5, 0x5A, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
            0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70,
            0x80, 0x90, 0xA0, 0xB0, 0xC0, 0xD0,
        ];
        let embedded = offset_seed_fixture(&payload);
        let search =
            TiledJpegSearch::new(&embedded, TILE_SIZE).expect("tiled search should decode");
        let candidates = search.prefix_candidates(MASTER_SEED, 1, 48);
        assert_ne!(candidates[0].1, payload[..6]);
        let (position, (key, _prefix)) = candidates
            .iter()
            .enumerate()
            .find(|(_, (_, prefix))| prefix == &payload[..6])
            .expect("later offset-seed candidate should be found");
        assert!(position > 0);
        assert_eq!(
            search
                .extract_candidate(MASTER_SEED, *key, 288)
                .expect("later candidate should extract"),
            payload
        );
        assert_eq!(tiled_search_decode_count(), 1);
    }

    #[test]
    fn tiled_jpeg_equal_prefix_bytes_do_not_collapse_distinct_candidate_keys() {
        let embedded = jpeg_embed_tiled(&test_jpeg(), &[0xA5; 36], MASTER_SEED, TILE_SIZE)
            .expect("tiled embed should succeed");
        let search =
            TiledJpegSearch::new(embedded.output(), TILE_SIZE).expect("tiled search should decode");
        let candidates = search.prefix_candidates(MASTER_SEED, 1, 0);
        let keys = candidates
            .iter()
            .map(|(key, _)| *key)
            .collect::<std::collections::HashSet<_>>();
        assert!(keys.len() > 1);
        assert!(candidates.iter().all(|(_, bytes)| bytes.is_empty()));
    }

    #[test]
    fn tiled_jpeg_candidate_extraction_uses_requested_redundancy() {
        let embedded = jpeg_embed_tiled(&test_jpeg(), &[0xA5; 36], MASTER_SEED, TILE_SIZE)
            .expect("tiled embed should succeed");
        let search =
            TiledJpegSearch::new(embedded.output(), TILE_SIZE).expect("tiled search should decode");
        let candidates = search.prefix_candidates(MASTER_SEED, 1, 48);
        let (key, prefix) = candidates
            .iter()
            .find(|(key, _)| key.redundancy == 3)
            .expect("redundancy-three candidate should be enumerated");
        assert_eq!(
            search
                .extract_candidate(MASTER_SEED, *key, 48)
                .expect("requested candidate should extract"),
            *prefix
        );
    }

    #[test]
    fn tiled_jpeg_max_origins_still_bounds_origin_scan() {
        let embedded = jpeg_embed_tiled(&test_jpeg(), &[0xA5; 36], MASTER_SEED, TILE_SIZE)
            .expect("tiled embed should succeed");
        let search =
            TiledJpegSearch::new(embedded.output(), TILE_SIZE).expect("tiled search should decode");
        let candidates = search.prefix_candidates(MASTER_SEED, 1, 48);
        assert!(candidates
            .iter()
            .all(|(key, _)| key.tile_x == 0 && key.tile_y == 0));
    }

    #[test]
    fn tiled_jpeg_search_decodes_once_for_multiple_candidate_extractions() {
        reset_tiled_search_decode_count();
        let embedded = jpeg_embed_tiled(&test_jpeg(), &[0xA5; 36], MASTER_SEED, TILE_SIZE)
            .expect("tiled embed should succeed");
        let search =
            TiledJpegSearch::new(embedded.output(), TILE_SIZE).expect("tiled search should decode");
        let candidates = search.prefix_candidates(MASTER_SEED, 1, 48);
        assert!(candidates.len() > 1);
        for (key, _) in &candidates {
            let _ = search.extract_candidate(MASTER_SEED, *key, 288);
        }
        assert_eq!(tiled_search_decode_count(), 1);
    }

    #[test]
    fn tiled_jpeg_search_no_match_broad_scan_decodes_once() {
        reset_tiled_search_decode_count();
        let search =
            TiledJpegSearch::new(&test_jpeg(), TILE_SIZE).expect("tiled search should decode");
        let candidates = search.prefix_candidates(MASTER_SEED, 4, 48);
        assert!(candidates.len() > 1);
        for (key, _) in &candidates {
            let _ = search.extract_candidate(MASTER_SEED, *key, 288);
        }
        assert_eq!(tiled_search_decode_count(), 1);
    }

    #[test]
    fn tiled_jpeg_search_legacy_fallback_lengths_decode_once() {
        reset_tiled_search_decode_count();
        let embedded = jpeg_embed_tiled(&test_jpeg(), &[0xA5; 36], MASTER_SEED, TILE_SIZE)
            .expect("tiled embed should succeed");
        let search =
            TiledJpegSearch::new(embedded.output(), TILE_SIZE).expect("tiled search should decode");
        let (key, _) = search
            .prefix_candidates(MASTER_SEED, 1, 48)
            .into_iter()
            .next()
            .expect("candidate should be available");
        let _ = search.extract_candidate(MASTER_SEED, key, 100 * 8);
        let _ = search.extract_candidate(MASTER_SEED, key, 76 * 8);
        assert_eq!(tiled_search_decode_count(), 1);
    }

    #[test]
    fn tiled_jpeg_search_rejects_key_from_incompatible_geometry() {
        let source =
            TiledJpegSearch::new(&test_jpeg(), TILE_SIZE).expect("source search should decode");
        let key = source
            .prefix_candidates(MASTER_SEED, 2, 0)
            .into_iter()
            .find(|(key, _)| key.tile_x == 1)
            .map(|(key, _)| key)
            .expect("source search should include a second tile origin");
        let target = TiledJpegSearch::new(&test_jpeg(), 256).expect("target search should decode");
        assert!(target.extract_candidate(MASTER_SEED, key, 48).is_none());
    }

    #[test]
    fn tiled_jpeg_embed_reports_embedded_only_after_encoded_roundtrip() {
        let payload = vec![0xA5; 36];
        let result = jpeg_embed_tiled(&test_jpeg(), &payload, MASTER_SEED, TILE_SIZE)
            .expect("tiled embed should succeed");
        assert!(result.is_embedded());
        assert!(jpeg_tiled_payload_matches(
            result.output(),
            &payload,
            TILE_SIZE,
            0,
            0,
            tile_seed(MASTER_SEED, 0, 0),
        )
        .expect("verification decode should succeed"));
    }

    #[test]
    fn tiled_jpeg_embed_roundtrip_uses_recorded_successful_tile() {
        let payload = vec![0xA5; 36];
        let result = jpeg_embed_tiled(&test_jpeg(), &payload, MASTER_SEED, TILE_SIZE)
            .expect("tiled embed should succeed");
        let search =
            TiledJpegSearch::new(result.output(), TILE_SIZE).expect("tiled search should decode");
        let candidates = search.prefix_candidates(MASTER_SEED, 1, 48);
        assert!(candidates.iter().any(|(key, _)| {
            key.tile_x == 0
                && key.tile_y == 0
                && key.seed_x == 0
                && key.seed_y == 0
                && key.redundancy == 1
                && search
                    .extract_candidate(MASTER_SEED, *key, payload.len() * 8)
                    .as_deref()
                    == Some(payload.as_slice())
        }));
    }

    #[test]
    fn tiled_jpeg_failed_roundtrip_is_not_reported_embedded() {
        let payload = vec![0xA5; 36];
        let result = jpeg_embed_tiled(&test_jpeg(), &payload, MASTER_SEED, TILE_SIZE)
            .expect("tiled embed should succeed");
        let (header, mut coefficients) =
            crate::jpeg_transcoder::JpegTranscoder::decode_coefficients(result.output())
                .expect("embedded JPEG should decode");
        let tile_blocks = crate::jpeg_transcoder::DctStegoF5::tile_block_set(
            &header,
            &coefficients,
            0,
            0,
            TILE_SIZE,
        );
        for (component, block_index) in tile_blocks {
            let block = coefficients
                .get_mut(&component)
                .and_then(|blocks| blocks.get_mut(block_index))
                .expect("test JPEG should have tile coefficients");
            for coefficient in block.iter_mut().skip(1) {
                if coefficient.abs() >= 2 {
                    *coefficient ^= 1;
                }
            }
        }
        let corrupted = crate::jpeg_transcoder::JpegTranscoder::encode_coefficients(
            &header,
            &coefficients,
            Some(result.output()),
        )
        .expect("corrupted JPEG should re-encode");
        let outcome = jpeg_tiled_outcome_after_verification(
            corrupted,
            &payload,
            TILE_SIZE,
            0,
            0,
            tile_seed(MASTER_SEED, 0, 0),
        )
        .expect("verification decode should succeed");
        assert!(outcome.is_skipped());
        assert!(!jpeg_tiled_payload_matches(
            outcome.output(),
            &payload,
            TILE_SIZE,
            0,
            0,
            tile_seed(MASTER_SEED, 0, 0),
        )
        .expect("skipped output should remain unverifiable"));
    }
}
