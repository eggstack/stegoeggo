//! Hidden parent-crate compatibility and search support.
//!
//! This module is `#[doc(hidden)]` behind the optional `application-support`
//! feature. It is not part of the stable generic API.
//!
//! Retained exports and why each is not stable generic API:
//!
//! - `legacy_lsb_required_slots`, `legacy_lsb_extract`,
//!   `legacy_lsb_extract_range`: legacy V1 compatibility only. Promoting them
//!   would misrepresent a superseded format as recommended generic API.
//! - `seed_fallback_embed`, `seed_fallback_extract`: historical StegoEggo LSB
//!   seed fallback needed for verification of existing images. No generic
//!   user-facing contract justifies stable promotion.
//! - `tile_seed`: tile-coordinate seed derivation needed by the application's
//!   V3/legacy candidate classification, which cannot be expressed as generic
//!   carrier recovery. Generic tiled operations derive seeds internally.
//! - `TiledJpegSearch`, `TiledJpegCandidateKey`: application-specific bounded
//!   candidate search for V3/legacy classification. Generic tiled extraction
//!   uses its own private bounded search without exposing candidate keys.
//! - `JpegSearchContext`: single-decode application verification reuse
//!   (standard probing plus tiled fallback share one decode). Plan 078
//!   evidence disposition is `PRIVATE-REUSE-SUFFICIENT`, so no public
//!   prepared type exists; this context stays hidden with all codec types
//!   private.
//!
//! Ordinary current LSB/JPEG/tiled embedding uses the stable `lsb`/`jpeg`
//! APIs directly. No hidden wrapper duplicates those paths.

use crate::error::StegoResult;
use crate::lsb_internal;
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

pub fn seed_fallback_embed(image: &mut RgbaImage, seed: u64) {
    lsb_internal::embed_seed_lsb_fallback(image, seed);
}

pub fn seed_fallback_extract(image: &RgbaImage) -> Option<u64> {
    lsb_internal::extract_seed_lsb_fallback(image)
}

pub fn tile_seed(master_seed: u64, tile_x: u32, tile_y: u32) -> u64 {
    lsb_internal::tile_seed(master_seed, tile_x, tile_y)
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

pub struct JpegSearchContext {
    decoded: crate::jpeg::DecodedJpegCarrier,
}

impl JpegSearchContext {
    pub fn new(jpeg_bytes: &[u8]) -> StegoResult<Self> {
        Ok(Self {
            decoded: crate::jpeg::decode_supported_carrier(jpeg_bytes)?,
        })
    }

    pub fn extract(
        &self,
        payload_len: usize,
        seed: u64,
        redundancy: usize,
    ) -> StegoResult<Vec<u8>> {
        self.decoded.extract_raw(payload_len, seed, redundancy)
    }

    pub fn capacity(
        &self,
        payload_len: usize,
        redundancy: usize,
    ) -> StegoResult<crate::CapacityReport> {
        self.decoded.capacity_for(payload_len, redundancy)
    }

    pub fn available_capacity(&self) -> usize {
        self.decoded.available_capacity()
    }

    pub fn tiled_prefix_candidates(
        &self,
        master_seed: u64,
        tile_size: u32,
        max_origins: u32,
        prefix_bits: usize,
    ) -> Vec<(TiledJpegCandidateKey, Vec<u8>)> {
        let Some(grid) = TiledGrid::for_decoded(&self.decoded, tile_size) else {
            return Vec::new();
        };
        grid.prefix_candidates(master_seed, max_origins, prefix_bits)
    }

    pub fn tiled_extract_candidate(
        &self,
        master_seed: u64,
        tile_size: u32,
        candidate: TiledJpegCandidateKey,
        payload_bits: usize,
    ) -> Option<Vec<u8>> {
        let grid = TiledGrid::for_decoded(&self.decoded, tile_size)?;
        grid.extract_candidate(master_seed, candidate, payload_bits)
    }
}

struct TiledGrid<'a> {
    header: &'a crate::jpeg_transcoder::JpegHeader,
    coefficients: &'a crate::jpeg_transcoder::Coefficients,
    tile_size: u32,
    tiles_x: u32,
    tiles_y: u32,
}

impl<'a> TiledGrid<'a> {
    fn new(
        header: &'a crate::jpeg_transcoder::JpegHeader,
        coefficients: &'a crate::jpeg_transcoder::Coefficients,
        tile_size: u32,
        tiles_x: u32,
        tiles_y: u32,
    ) -> Self {
        Self {
            header,
            coefficients,
            tile_size,
            tiles_x,
            tiles_y,
        }
    }

    fn for_decoded(decoded: &'a crate::jpeg::DecodedJpegCarrier, tile_size: u32) -> Option<Self> {
        let (tiles_x, tiles_y) = tile_geometry(decoded.header(), tile_size)?;
        Some(Self::new(
            decoded.header(),
            decoded.coefficients(),
            tile_size,
            tiles_x,
            tiles_y,
        ))
    }

    fn prefix_candidates(
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
                    self.header,
                    self.coefficients,
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
                                        self.coefficients,
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

    fn extract_candidate(
        &self,
        master_seed: u64,
        candidate: TiledJpegCandidateKey,
        payload_bits: usize,
    ) -> Option<Vec<u8>> {
        if !valid_tiled_candidate(candidate, self.tiles_x, self.tiles_y) {
            return None;
        }
        let tile_blocks = crate::jpeg_transcoder::DctStegoF5::tile_block_set(
            self.header,
            self.coefficients,
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
                .extract_f5_from_blocks(self.coefficients, payload_bits, local_seed, &tile_blocks);
        (bits.len() >= payload_bits).then(|| lsb_internal::bits_to_bytes(&bits))
    }
}

fn valid_tiled_candidate(candidate: TiledJpegCandidateKey, tiles_x: u32, tiles_y: u32) -> bool {
    candidate.tile_x < tiles_x
        && candidate.tile_y < tiles_y
        && candidate.seed_x < 16
        && candidate.seed_y < 16
        && (1..=10).contains(&candidate.redundancy)
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
        self.grid()
            .prefix_candidates(master_seed, max_origins, prefix_bits)
    }

    pub fn extract_candidate(
        &self,
        master_seed: u64,
        candidate: TiledJpegCandidateKey,
        payload_bits: usize,
    ) -> Option<Vec<u8>> {
        self.grid()
            .extract_candidate(master_seed, candidate, payload_bits)
    }

    fn grid(&self) -> TiledGrid<'_> {
        TiledGrid::new(
            &self.header,
            &self.coefficients,
            self.tile_size,
            self.tiles_x,
            self.tiles_y,
        )
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
    let luma_blocks_x = (header.width as u32).div_ceil(max_h * 8) * max_h;
    let luma_blocks_y = (header.height as u32).div_ceil(max_v * 8) * max_v;
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

    fn stable_tiled_output(payload: &[u8]) -> Vec<u8> {
        let config = crate::types::TileConfig::try_new(MASTER_SEED, TILE_SIZE).unwrap();
        let report = crate::jpeg::embed_tiled(&test_jpeg(), payload, &config)
            .expect("stable tiled embed should succeed");
        assert!(report.embedded);
        report.output
    }

    #[test]
    fn tiled_jpeg_candidate_key_roundtrips_same_identity() {
        let payload = vec![0xA5; 36];
        let output = stable_tiled_output(&payload);
        let search = TiledJpegSearch::new(&output, TILE_SIZE).expect("tiled search should decode");
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
        let output = stable_tiled_output(&[0xA5; 36]);
        let search = TiledJpegSearch::new(&output, TILE_SIZE).expect("tiled search should decode");
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
        let output = stable_tiled_output(&[0xA5; 36]);
        let search = TiledJpegSearch::new(&output, TILE_SIZE).expect("tiled search should decode");
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
        let output = stable_tiled_output(&[0xA5; 36]);
        let search = TiledJpegSearch::new(&output, TILE_SIZE).expect("tiled search should decode");
        let candidates = search.prefix_candidates(MASTER_SEED, 1, 48);
        assert!(candidates
            .iter()
            .all(|(key, _)| key.tile_x == 0 && key.tile_y == 0));
    }

    #[test]
    fn jpeg_search_context_decodes_once_for_standard_and_tiled_probes() {
        crate::jpeg::reset_decode_count();
        let payload = vec![0xA5; 36];
        let output = stable_tiled_output(&payload);
        let context = JpegSearchContext::new(&output).expect("search context should decode");
        assert_eq!(crate::jpeg::decode_count(), 1);
        for redundancy in 1..=10 {
            let _ = context.extract(6, MASTER_SEED, redundancy);
            let _ = context.extract(36, MASTER_SEED, redundancy);
            let _ = context.capacity(36, redundancy);
        }
        assert_eq!(context.available_capacity(), context.available_capacity());
        let candidates = context.tiled_prefix_candidates(MASTER_SEED, TILE_SIZE, 1, 48);
        assert!(!candidates.is_empty());
        for (key, _) in &candidates {
            let _ = context.tiled_extract_candidate(MASTER_SEED, TILE_SIZE, *key, 288);
        }
        assert_eq!(crate::jpeg::decode_count(), 1);
    }

    #[test]
    fn tiled_jpeg_search_decodes_once_for_multiple_candidate_extractions() {
        reset_tiled_search_decode_count();
        let output = stable_tiled_output(&[0xA5; 36]);
        let search = TiledJpegSearch::new(&output, TILE_SIZE).expect("tiled search should decode");
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
        let output = stable_tiled_output(&[0xA5; 36]);
        let search = TiledJpegSearch::new(&output, TILE_SIZE).expect("tiled search should decode");
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
    fn tiled_jpeg_embed_roundtrip_uses_recorded_successful_tile() {
        let payload = vec![0xA5; 36];
        let output = stable_tiled_output(&payload);
        let search = TiledJpegSearch::new(&output, TILE_SIZE).expect("tiled search should decode");
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

    fn make_header_420(width: u16, height: u16) -> crate::jpeg_transcoder::JpegHeader {
        let mut header = crate::jpeg_transcoder::JpegHeader::default();
        header.width = width;
        header.height = height;
        header.components = vec![
            crate::jpeg_transcoder::header::ScanComponent {
                component_id: 1,
                h_sampling: 2,
                v_sampling: 2,
                quant_table_id: 0,
                dc_table_id: 0,
                ac_table_id: 0,
            },
            crate::jpeg_transcoder::header::ScanComponent {
                component_id: 2,
                h_sampling: 1,
                v_sampling: 1,
                quant_table_id: 1,
                dc_table_id: 0,
                ac_table_id: 0,
            },
            crate::jpeg_transcoder::header::ScanComponent {
                component_id: 3,
                h_sampling: 1,
                v_sampling: 1,
                quant_table_id: 1,
                dc_table_id: 0,
                ac_table_id: 0,
            },
        ];
        header
    }

    #[test]
    fn tile_geometry_counts_subsampled_luma_blocks() {
        let header = make_header_420(128, 128);
        assert_eq!(tile_geometry(&header, 64), Some((2, 2)));

        let small = make_header_420(64, 64);
        assert_eq!(tile_geometry(&small, 64), Some((1, 1)));
    }
}
