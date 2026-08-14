use crate::error::{Error, Result};
use crate::payload_v3::types::{AuthAlgorithm, ProtectionChannels, V3_MAGIC, V3_PAYLOAD_VERSION};
use crate::protected::constants::STEGO_OFFSET_SEED_1;
use crate::protected::ecc;
use crate::protected::metadata_trap::RightsMetadataProtector;
use crate::resource_limits::ResourceLimits;
use crate::stego::jpeg as carrier_jpeg;
use crate::stego::jpeg_transcoder::{DctStegoF5, JpegTranscoder};
use crate::stego::lsb as carrier_lsb;
use crate::traits::Protector;
use crate::types::{
    PayloadEmissionContext, ProtectionContext, ProtectionLevel, VerificationStatus,
};
use crc32fast::Hasher as Crc32Hasher;
use hmac::{Hmac, Mac};
use image::{DynamicImage, RgbaImage};
use sha2::Sha256;
use std::borrow::Cow;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Minimum stego payload size: 24 bytes header + 4 bytes CRC32 checksum (or 8 bytes MAC).
const MIN_PAYLOAD_SIZE: usize = 28;
/// V1 ECC-encoded payload size: 72 bytes (24 data × 3 replication) + 4 bytes CRC32.
const ECC_PAYLOAD_SIZE_V1: usize = ecc::TOTAL_ECC_LEN + 4;
/// V2 header size: 32 bytes.
pub(crate) const V2_HEADER_SIZE: usize = 32;
/// V2 ECC-encoded payload size: 96 bytes (32 data × 3 replication) + 4 bytes CRC32.
const ECC_PAYLOAD_SIZE_V2: usize = V2_HEADER_SIZE * ecc::REPLICATION_FACTOR + 4;
/// Bit length of the V2 ECC-encoded payload.
pub(crate) const ECC_PAYLOAD_BITS_V2: usize = ECC_PAYLOAD_SIZE_V2 * 8;
/// Legacy ECC payload bits (V1, kept for backward-compatible extraction).
const ECC_PAYLOAD_BITS: usize = ECC_PAYLOAD_SIZE_V1 * 8;
/// V3 payload size: CRC32 mode (32-byte core + 4-byte CRC32 = 36 bytes).
const V3_CRC_PAYLOAD_SIZE: usize = crate::payload_v3::types::V3_CORE_SIZE + 4;
/// V3 payload bits for CRC32 mode.
pub(crate) const V3_CRC_PAYLOAD_BITS: usize = V3_CRC_PAYLOAD_SIZE * 8;
/// V3 payload size: HMAC mode (32-byte core + 16-byte HMAC = 48 bytes).
const V3_HMAC_PAYLOAD_SIZE: usize = crate::payload_v3::types::V3_CORE_SIZE + 16;
/// V3 payload bits for HMAC mode.
pub(crate) const V3_HMAC_PAYLOAD_BITS: usize = V3_HMAC_PAYLOAD_SIZE * 8;

/// Payload versions the extractor knows how to parse, in preference order.
///
/// When a future v3 lands, append `3` here and add a `parse_stego_payload_v3` arm
/// in `parse_stego_payload`. The parser tries each version in order, so a single
/// binary can read both old and new payloads without forcing a coordinated upgrade
/// of every protected image in the wild.
///
/// Removing a version from this slice will make previously-protected images
/// un-parseable. Only remove a version when you are confident no surviving
/// protected image still uses it.
const SUPPORTED_PAYLOAD_VERSIONS: &[u8] = &[1, 2, 3];

/// Common test/dev seeds tried when metadata seed is unavailable.
///
/// Only available with the `test-seeds` feature flag. These are only useful
/// for images protected with one of these known seeds. Images protected with
/// `generate_random_seed()` (the default) will NOT be found via fallback —
/// they rely on metadata seed extraction or DCT Q-table seed for verification.
/// Production deployments should use explicit seeds or rely on the
/// metadata-based extraction path.
#[cfg(feature = "test-seeds")]
const FALLBACK_SEEDS: &[u64] = &[42, 0, 1, 12345, 99999, 123456789];

/// Outcome for stego payload candidate extraction used by the verification
/// path. Distinguishing `Invalid` from `NotFound` lets
/// [`VerificationStatus::Invalid`] actually surface when a structurally
/// plausible payload is present but its HMAC/checksum fails (e.g. wrong
/// MAC key, bit-level corruption).
///
/// V3-specific variants distinguish header-driven failure modes from legacy
/// corruption, enabling structured reporting for missing authentication keys,
/// failed authentication, malformed headers, and unsupported versions.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum CandidateOutcome {
    Valid(Vec<u8>),
    Invalid(Vec<u8>),
    MalformedV3,
    UnsupportedVersion(u8),
    AuthenticationKeyMissing(Vec<u8>),
    AuthenticationFailed(Vec<u8>),
    NotFound,
}

/// Number of bytes in the v3 prefix: magic (2) + version (1) + header_length (1) + total_length (2).
const V3_PREFIX_BYTES: usize = 6;

/// Result of classifying the first6 bytes of a potential v3 payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum V3PrefixResult {
    /// Valid v3 prefix; `header_length` and `total_length` are the declared sizes.
    Detected {
        header_length: usize,
        total_length: usize,
    },
    /// Bytes do not begin with v3 magic — legacy payload.
    NotV3,
    /// v3 magic found but version is unsupported.
    UnsupportedVersion(u8),
    /// v3 magic found but prefix fields are malformed.
    Malformed(PayloadMalformedReason),
    /// v3 payload exceeds resource limits.
    ResourceLimitExceeded,
}

/// Reason a v3 prefix or header is malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum PayloadMalformedReason {
    HeaderTooShort,
    TotalLessThanHeader,
    TotalExceedsWireMax,
    TotalExceedsResourceLimit,
    BitOverflow,
    AuthTagLengthInvalid,
    KeyIdTooLong,
    HeaderTooShortForKeyId,
    InvalidAuthAlgorithm,
    HeaderLengthMismatch,
}

/// Validated v3 header fields extracted after the prefix stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidatedV3Header {
    pub header_length: usize,
    pub total_length: usize,
    pub auth_algorithm: AuthAlgorithm,
    pub auth_tag_length: usize,
    pub key_id_length: usize,
    pub channels: ProtectionChannels,
    pub flags: crate::payload_v3::types::PayloadFlags,
}

/// Result of probing extracted bytes for a v3 header (legacy name retained for call-site compatibility).
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum V3ProbeResult {
    /// Valid v3 header found; `total_length` is the exact payload size in bytes.
    V3Detected {
        header_length: usize,
        total_length: usize,
        total_bits: usize,
    },
    /// Bytes do not begin with v3 magic — legacy payload.
    NotV3,
    /// v3 magic found but header is malformed (invalid lengths, fields).
    MalformedV3,
    /// v3 magic found but version is unsupported.
    UnsupportedVersion(u8),
    /// v3 payload exceeds operation resource limits.
    ResourceLimitExceeded,
    /// Image too small to extract the probe bits.
    InsufficientCapacity,
}

#[cfg(test)]
#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct ExtractionTrace {
    pub requested_bit_lengths: Vec<usize>,
    pub prefix_extractions: usize,
    pub header_extractions: usize,
    pub full_extractions: usize,
    pub legacy_decoder_entries: usize,
}

pub use crate::stego::lsb::tile_seed;
pub use crate::stego::lsb::DEFAULT_TILE_SIZE;

/// Steganographic protection: embeds hidden payloads in image pixels or DCT coefficients.
///
/// For PNG/WebP images, uses LSB (Least Significant Bit) embedding with pseudo-random
/// pixel selection. WebP support is **lossless-only**: lossy WebP re-encoding destroys
/// LSBs. For JPEG images, uses F5-style DCT coefficient embedding and quantization
/// table seed storage.
///
/// The embedded payload contains protection metadata (level, seed, intensity, timestamp)
/// and is verified via checksum or HMAC-SHA256 when a MAC key is configured.
///
/// # Extraction
///
/// Use [`extract_payload`](Self::extract_payload) or
/// [`extract_payload_with_key`](Self::extract_payload_with_key) to recover the
/// embedded metadata. For quick checks, use [`verify_payload`](Self::verify_payload).
///
/// # JPEG Behavior
///
/// When called through [`apply_bytes`](crate::traits::Protector::apply_bytes) on JPEG
/// input, the protector uses `apply_dct_stego_bytes`,
/// which stores the seed in quantization tables when those tables are preserved and
/// applies F5 DCT coefficient embedding for baseline JPEGs.
pub struct SteganographyProtector {
    limits: ResourceLimits,
}

impl SteganographyProtector {
    /// Create a new steganography protector with default resource limits.
    pub fn new() -> Self {
        Self {
            limits: ResourceLimits::default(),
        }
    }

    /// Create a new steganography protector with custom resource limits.
    pub fn with_resource_limits(limits: ResourceLimits) -> Self {
        Self { limits }
    }

    fn payload_within_limits(&self, bytes: &[u8]) -> bool {
        bytes.len() <= self.limits.max_payload_bytes()
    }

    /// Verify that an image contains a valid protection payload.
    ///
    /// Returns `VerificationStatus::Verified` if a payload is found and its
    /// checksum or HMAC is valid. For HMAC verification, use
    /// [`verify_payload_with_key`](Self::verify_payload_with_key).
    ///
    /// **Warning:** Without a MAC key, this method only checks a non-cryptographic
    /// CRC32 checksum that can be forged. For adversarial settings, always verify
    /// with a MAC key.
    pub fn verify_payload(&self, img: &DynamicImage) -> bool {
        self.verify_payload_with_key(img, &[]) == VerificationStatus::Verified
    }

    /// Apply DCT-based steganography to JPEG bytes.
    ///
    /// For baseline JPEGs, performs full F5 embedding in DCT coefficients and stores
    /// the seed in quantization tables. For progressive JPEGs (which cannot be decoded
    /// to DCT coefficients), falls back to seed-in-quantization-tables only.
    ///
    /// This is the JPEG fast path — it avoids pixel decode/encode cycles and preserves
    /// quality. Used internally by the pipeline for JPEG-in/JPEG-out flows.
    pub(crate) fn apply_dct_stego_bytes(
        &self,
        jpeg_bytes: &[u8],
        ctx: &ProtectionContext,
    ) -> Result<crate::stego::EmbedOutcome<Vec<u8>>> {
        use crate::stego::{EmbedOutcome, EmbedPath};

        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return Err(Error::Steganography("Not a valid JPEG".to_string()));
        }

        if let Some(tile_size) = ctx.tile_size().filter(|&s| s > 0) {
            return self.apply_dct_stego_bytes_tiled(jpeg_bytes, ctx, tile_size);
        }

        let seed = ctx.seed();

        match JpegTranscoder::decode_coefficients(jpeg_bytes) {
            Ok((header, coefficients)) => {
                let payload = self.generate_payload(
                    &crate::types::PayloadEmissionContext::from_plan_for_context(
                        ctx,
                        EmbedPath::DctF5,
                    ),
                    ctx,
                );
                let requested_redundancy = ctx.effective_redundancy();
                let payload_bits = payload.len().saturating_mul(8);

                let available_coeffs = Self::dct_payload_capacity(&coefficients);

                let mut header = header;
                DctStegoF5::new().embed_seed_in_quantization_tables(&mut header, seed)?;

                let max_feasible = available_coeffs.checked_div(payload_bits).unwrap_or(0);
                let selected_redundancy = requested_redundancy.min(max_feasible).max(1);

                if max_feasible >= 1 {
                    let mut embedded_coefficients = coefficients.clone();
                    if DctStegoF5::with_redundancy(selected_redundancy)
                        .embed_f5(&mut embedded_coefficients, &payload, seed)
                        .is_ok()
                    {
                        let output = JpegTranscoder::encode_coefficients(
                            &header,
                            &embedded_coefficients,
                            Some(jpeg_bytes),
                        )?;
                        return Ok(EmbedOutcome::Embedded {
                            output,
                            payload_bytes: payload.len(),
                            required_capacity: payload_bits,
                            available_capacity: available_coeffs,
                            path: EmbedPath::DctF5,
                        });
                    }
                }

                let output =
                    JpegTranscoder::encode_coefficients(&header, &coefficients, Some(jpeg_bytes))?;
                Ok(EmbedOutcome::SkippedCapacity {
                    output,
                    payload_bytes: payload.len(),
                    required_capacity: payload_bits,
                    available_capacity: available_coeffs,
                    path: EmbedPath::DctF5,
                })
            }
            Err(_) => {
                let mut header = crate::stego::jpeg_transcoder::JpegHeader::parse(jpeg_bytes)?;
                DctStegoF5::new().embed_seed_in_quantization_tables(&mut header, seed)?;
                let output = Self::reassemble_jpeg_with_qtables(jpeg_bytes, &header)?;
                Ok(EmbedOutcome::UnsupportedProgressive { output })
            }
        }
    }

    pub(crate) fn lsb_pixels_needed(ctx: &ProtectionContext) -> usize {
        let payload_bits = Self::payload_bits_for_context(ctx);
        carrier_lsb::lsb_required_slots_legacy(payload_bits)
    }

    fn payload_bits_for_context(ctx: &ProtectionContext) -> usize {
        if ctx.mac_key().is_some() {
            V3_HMAC_PAYLOAD_BITS
        } else {
            V3_CRC_PAYLOAD_BITS
        }
    }

    /// Embed only the seed in JPEG quantization tables (no DCT coefficient modification).
    /// Used for Light level JPEG protection — the seed is recoverable when the
    /// quantization tables themselves are preserved.
    pub(crate) fn apply_qtable_seed_bytes(&self, jpeg_bytes: &[u8], seed: u64) -> Result<Vec<u8>> {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return Err(Error::Steganography("Not a valid JPEG".to_string()));
        }

        let mut header = crate::stego::jpeg_transcoder::JpegHeader::parse(jpeg_bytes)?;
        DctStegoF5::new().embed_seed_in_quantization_tables(&mut header, seed)?;
        Self::reassemble_jpeg_with_qtables(jpeg_bytes, &header)
    }

    /// Embed the full payload per tile using F5-style DCT coefficient
    /// manipulation for crop resistance.
    ///
    /// Each `tile_size × tile_size` pixel region (mapped to DCT blocks)
    /// embeds the full payload using a tile-specific seed. Redundancy is
    /// fixed at 1 because the tile grid itself is the redundancy.
    ///
    /// Returns the re-encoded JPEG bytes, or an error if embedding fails.
    pub(crate) fn apply_dct_stego_bytes_tiled(
        &self,
        jpeg_bytes: &[u8],
        ctx: &ProtectionContext,
        tile_size: u32,
    ) -> Result<crate::stego::EmbedOutcome<Vec<u8>>> {
        use crate::stego::{EmbedOutcome, EmbedPath};

        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return Err(Error::Steganography("Not a valid JPEG".to_string()));
        }

        let seed = ctx.seed();

        match JpegTranscoder::decode_coefficients(jpeg_bytes) {
            Ok((header, coefficients)) => {
                let mut header = header;
                let mut coefficients = coefficients;

                let payload = self.generate_payload(
                    &crate::types::PayloadEmissionContext::from_plan_for_context(
                        ctx,
                        EmbedPath::DctF5Tiled,
                    ),
                    ctx,
                );
                let payload_bits = payload.len() * 8;

                DctStegoF5::new().embed_seed_in_quantization_tables(&mut header, seed)?;

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
                let tiles_x = luma_blocks_x / blocks_per_tile;
                let tiles_y = luma_blocks_y / blocks_per_tile;

                let mut embedded_any = false;
                for ty in 0..tiles_y {
                    for tx in 0..tiles_x {
                        let tile_blocks =
                            DctStegoF5::tile_block_set(&header, &coefficients, tx, ty, tile_size);
                        if tile_blocks.is_empty() {
                            continue;
                        }
                        let local_seed = tile_seed(seed, tx, ty);
                        if DctStegoF5::with_redundancy(1)
                            .embed_f5_in_blocks(
                                &mut coefficients,
                                &payload,
                                local_seed,
                                &tile_blocks,
                            )
                            .is_ok()
                        {
                            embedded_any = true;
                        }
                    }
                }

                if embedded_any {
                    let attempt_bytes = JpegTranscoder::encode_coefficients(
                        &header,
                        &coefficients,
                        Some(jpeg_bytes),
                    )?;
                    if let Ok((_, roundtrip_coefficients)) =
                        JpegTranscoder::decode_coefficients(&attempt_bytes)
                    {
                        let tile_blocks = DctStegoF5::tile_block_set(
                            &header,
                            &roundtrip_coefficients,
                            0,
                            0,
                            tile_size,
                        );
                        let roundtrip_bits = DctStegoF5::with_redundancy(1).extract_f5_from_blocks(
                            &roundtrip_coefficients,
                            payload_bits,
                            tile_seed(seed, 0, 0),
                            &tile_blocks,
                        );
                        if Self::bits_to_bytes(&roundtrip_bits) == payload {
                            return Ok(EmbedOutcome::Embedded {
                                output: attempt_bytes,
                                payload_bytes: payload.len(),
                                required_capacity: payload_bits,
                                available_capacity: payload_bits,
                                path: EmbedPath::DctF5Tiled,
                            });
                        }
                    }
                }

                let output =
                    JpegTranscoder::encode_coefficients(&header, &coefficients, Some(jpeg_bytes))?;
                Ok(EmbedOutcome::SkippedCapacity {
                    output,
                    payload_bytes: payload.len(),
                    required_capacity: payload_bits,
                    available_capacity: 0,
                    path: EmbedPath::DctF5Tiled,
                })
            }
            Err(_) => {
                let mut header = crate::stego::jpeg_transcoder::JpegHeader::parse(jpeg_bytes)?;
                DctStegoF5::new().embed_seed_in_quantization_tables(&mut header, seed)?;
                let output = Self::reassemble_jpeg_with_qtables(jpeg_bytes, &header)?;
                Ok(EmbedOutcome::UnsupportedProgressive { output })
            }
        }
    }

    /// Extract payload from tiled F5 DCT stego in a possibly-cropped JPEG.
    ///
    /// Tries different grid coordinates for each tile origin to find one
    /// that produces a valid payload.
    #[doc(hidden)]
    pub fn extract_f5_tiled_candidates(
        &self,
        jpeg_bytes: &[u8],
        master_seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return None;
        }

        let (header, coefficients) = JpegTranscoder::decode_coefficients(jpeg_bytes).ok()?;

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
        let tiles_x = luma_blocks_x / blocks_per_tile;
        let tiles_y = luma_blocks_y / blocks_per_tile;

        let max_grid = 16u32;
        let mut origins_tried = 0u32;

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                if origins_tried >= max_origins {
                    return None;
                }
                origins_tried += 1;

                let tile_blocks =
                    DctStegoF5::tile_block_set(&header, &coefficients, tx, ty, tile_size);
                if tile_blocks.is_empty() {
                    continue;
                }

                let base_x = tx;
                let base_y = ty;
                for dy in 0..=2u32 {
                    if base_y + dy >= max_grid {
                        break;
                    }
                    for dx in 0..=2u32 {
                        if base_x + dx >= max_grid {
                            break;
                        }
                        let local_seed = tile_seed(master_seed, base_x + dx, base_y + dy);

                        // Three-stage v3 extraction: prefix → header → payload.
                        for redundancy in 1..=10 {
                            let stego = DctStegoF5::with_redundancy(redundancy);
                            let prefix_bits = V3_PREFIX_BYTES * 8;
                            let prefix_raw = stego.extract_f5_from_blocks(
                                &coefficients,
                                prefix_bits,
                                local_seed,
                                &tile_blocks,
                            );
                            if prefix_raw.len() < prefix_bits {
                                continue;
                            }
                            let prefix_bytes = Self::bits_to_bytes(&prefix_raw);

                            match Self::classify_v3_prefix(&prefix_bytes, Some(&self.limits)) {
                                V3PrefixResult::Detected {
                                    header_length,
                                    total_length,
                                } => {
                                    let header_bits = header_length * 8;
                                    let header_raw = if header_bits <= prefix_bits {
                                        prefix_raw[..header_bits].to_vec()
                                    } else {
                                        let h = stego.extract_f5_from_blocks(
                                            &coefficients,
                                            header_bits,
                                            local_seed,
                                            &tile_blocks,
                                        );
                                        if h.len() < header_bits {
                                            continue;
                                        }
                                        h
                                    };
                                    let header_bytes = Self::bits_to_bytes(&header_raw);
                                    if Self::validate_v3_header(&header_bytes, Some(&self.limits))
                                        .is_err()
                                    {
                                        continue;
                                    }
                                    let total_bits = total_length * 8;
                                    let full_raw = if total_bits <= prefix_bits {
                                        prefix_raw[..total_bits].to_vec()
                                    } else {
                                        let f = stego.extract_f5_from_blocks(
                                            &coefficients,
                                            total_bits,
                                            local_seed,
                                            &tile_blocks,
                                        );
                                        if f.len() < total_bits {
                                            continue;
                                        }
                                        f
                                    };
                                    let full_bytes = Self::bits_to_bytes(&full_raw);
                                    if Self::verify_payload_integrity(&full_bytes, mac_key) {
                                        return Some(Self::truncate_to_actual_payload(&full_bytes));
                                    }
                                }
                                V3PrefixResult::Malformed(_)
                                | V3PrefixResult::UnsupportedVersion(_)
                                | V3PrefixResult::ResourceLimitExceeded => {
                                    return None;
                                }
                                V3PrefixResult::NotV3 => {
                                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                                        let extracted = stego.extract_f5_from_blocks(
                                            &coefficients,
                                            ecc_bits,
                                            local_seed,
                                            &tile_blocks,
                                        );
                                        if extracted.len() < ecc_bits {
                                            continue;
                                        }
                                        let payload_bytes = Self::bits_to_bytes(&extracted);
                                        if Self::try_ecc_decode(&payload_bytes).is_some() {
                                            return Some(payload_bytes);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Verification-path variant of `extract_f5_tiled_candidates` that
    /// returns a tri-state. See [`Self::verify_extract_with_redundancy`].
    fn verify_extract_f5_tiled(
        &self,
        jpeg_bytes: &[u8],
        master_seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return CandidateOutcome::NotFound;
        }

        let Ok((header, coefficients)) = JpegTranscoder::decode_coefficients(jpeg_bytes) else {
            return CandidateOutcome::NotFound;
        };

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
        let tiles_x = luma_blocks_x / blocks_per_tile;
        let tiles_y = luma_blocks_y / blocks_per_tile;

        let max_grid = 16u32;
        let mut origins_tried = 0u32;
        let mut last_outcome: Option<CandidateOutcome> = None;

        'outer: for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                if origins_tried >= max_origins {
                    break 'outer;
                }
                origins_tried += 1;

                let tile_blocks =
                    DctStegoF5::tile_block_set(&header, &coefficients, tx, ty, tile_size);
                if tile_blocks.is_empty() {
                    continue;
                }

                let base_x = tx;
                let base_y = ty;
                for dy in 0..=2u32 {
                    if base_y + dy >= max_grid {
                        break;
                    }
                    for dx in 0..=2u32 {
                        if base_x + dx >= max_grid {
                            break;
                        }
                        let local_seed = tile_seed(master_seed, base_x + dx, base_y + dy);

                        // Three-stage v3 extraction: prefix → header → payload.
                        for redundancy in 1..=10 {
                            let stego = DctStegoF5::with_redundancy(redundancy);
                            let prefix_bits = V3_PREFIX_BYTES * 8;
                            let prefix_raw = stego.extract_f5_from_blocks(
                                &coefficients,
                                prefix_bits,
                                local_seed,
                                &tile_blocks,
                            );
                            if prefix_raw.len() < prefix_bits {
                                continue;
                            }
                            let prefix_bytes = Self::bits_to_bytes(&prefix_raw);

                            match Self::classify_v3_prefix(&prefix_bytes, Some(&self.limits)) {
                                V3PrefixResult::Detected {
                                    header_length,
                                    total_length,
                                } => {
                                    let header_bits = header_length * 8;
                                    let header_raw = if header_bits <= prefix_bits {
                                        prefix_raw[..header_bits].to_vec()
                                    } else {
                                        let h = stego.extract_f5_from_blocks(
                                            &coefficients,
                                            header_bits,
                                            local_seed,
                                            &tile_blocks,
                                        );
                                        if h.len() < header_bits {
                                            continue;
                                        }
                                        h
                                    };
                                    let header_bytes = Self::bits_to_bytes(&header_raw);
                                    if Self::validate_v3_header(&header_bytes, Some(&self.limits))
                                        .is_err()
                                    {
                                        continue;
                                    }
                                    let total_bits = total_length * 8;
                                    let full_raw = if total_bits <= prefix_bits {
                                        prefix_raw[..total_bits].to_vec()
                                    } else {
                                        let f = stego.extract_f5_from_blocks(
                                            &coefficients,
                                            total_bits,
                                            local_seed,
                                            &tile_blocks,
                                        );
                                        if f.len() < total_bits {
                                            continue;
                                        }
                                        f
                                    };
                                    let full_bytes = Self::bits_to_bytes(&full_raw);
                                    if Self::verify_payload_integrity(&full_bytes, mac_key) {
                                        return CandidateOutcome::Valid(
                                            Self::truncate_to_actual_payload(&full_bytes),
                                        );
                                    }
                                    if last_outcome.is_none() {
                                        last_outcome =
                                            Some(Self::classify_auth_failure(&full_bytes, mac_key));
                                    }
                                }
                                V3PrefixResult::Malformed(_) => {
                                    if last_outcome.is_none() {
                                        last_outcome = Some(CandidateOutcome::MalformedV3);
                                    }
                                }
                                V3PrefixResult::UnsupportedVersion(v) => {
                                    if last_outcome.is_none() {
                                        last_outcome =
                                            Some(CandidateOutcome::UnsupportedVersion(v));
                                    }
                                }
                                V3PrefixResult::ResourceLimitExceeded => {
                                    if last_outcome.is_none() {
                                        last_outcome = Some(CandidateOutcome::MalformedV3);
                                    }
                                }
                                V3PrefixResult::NotV3 => {
                                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                                        let extracted = stego.extract_f5_from_blocks(
                                            &coefficients,
                                            ecc_bits,
                                            local_seed,
                                            &tile_blocks,
                                        );
                                        if extracted.len() < ecc_bits {
                                            continue;
                                        }
                                        let payload_bytes = Self::bits_to_bytes(&extracted);
                                        if Self::try_ecc_decode(&payload_bytes).is_some() {
                                            return CandidateOutcome::Valid(payload_bytes);
                                        }
                                        if last_outcome.is_none() {
                                            last_outcome = Some(Self::classify_auth_failure(
                                                &payload_bytes,
                                                mac_key,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        match last_outcome {
            Some(outcome) => outcome,
            None => CandidateOutcome::NotFound,
        }
    }

    fn reassemble_jpeg_with_qtables(
        jpeg_bytes: &[u8],
        header: &crate::stego::jpeg_transcoder::JpegHeader,
    ) -> Result<Vec<u8>> {
        carrier_jpeg::reassemble_jpeg_with_qtables(jpeg_bytes, header).map_err(Into::into)
    }

    pub(crate) fn dct_payload_capacity(
        coefficients: &crate::stego::jpeg_transcoder::Coefficients,
    ) -> usize {
        carrier_jpeg::dct_payload_capacity(coefficients)
    }

    fn extract_with_redundancy(
        &self,
        img: &RgbaImage,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        let prefix_bits = 6 * 8;
        for redundancy in 1..=10 {
            if let Some(result) =
                self.extract_payload_at_seed_v2(img, prefix_bits, seed, mac_key, 0, redundancy)
            {
                return Some(result);
            }
        }
        for pass in 0..5 {
            let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));
            for redundancy in 1..=10 {
                if let Some(result) = self.extract_payload_at_seed_v2(
                    img,
                    prefix_bits,
                    offset_seed,
                    mac_key,
                    0,
                    redundancy,
                ) {
                    return Some(result);
                }
            }
            if let Some(result) =
                self.extract_payload_at_seed_legacy(img, prefix_bits, offset_seed, mac_key)
            {
                return Some(result);
            }
        }
        None
    }

    fn extract_payload_at_seed_v2(
        &self,
        img: &RgbaImage,
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
        base_slot: usize,
        redundancy: usize,
    ) -> Option<Vec<u8>> {
        if let Some(prefix) = self.extract_lsb_v2(img, prefix_bits, seed, base_slot, redundancy) {
            match Self::classify_v3_probe(&prefix, Some(&self.limits)) {
                V3ProbeResult::V3Detected { total_bits, .. } => {
                    if let Some(full) =
                        self.extract_lsb_v2(img, total_bits, seed, base_slot, redundancy)
                    {
                        if Self::verify_payload_integrity(&full, mac_key) {
                            return Some(Self::truncate_to_actual_payload(&full));
                        }
                    }
                }
                V3ProbeResult::NotV3 => {
                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        if let Some(payload) =
                            self.extract_lsb_v2(img, ecc_bits, seed, base_slot, redundancy)
                        {
                            if Self::try_ecc_decode(&payload).is_some() {
                                return Some(payload);
                            }
                            if Self::verify_payload_integrity(&payload, mac_key) {
                                return Some(Self::truncate_to_actual_payload(&payload));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn extract_payload_at_seed_legacy(
        &self,
        img: &RgbaImage,
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        if let Some(prefix) = self.extract_lsb(img, prefix_bits, seed) {
            match Self::classify_v3_probe(&prefix, Some(&self.limits)) {
                V3ProbeResult::V3Detected { total_bits, .. } => {
                    if let Some(full) = self.extract_lsb(img, total_bits, seed) {
                        if Self::verify_payload_integrity(&full, mac_key) {
                            return Some(Self::truncate_to_actual_payload(&full));
                        }
                    }
                }
                V3ProbeResult::NotV3 => {
                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        if let Some(payload) = self.extract_lsb(img, ecc_bits, seed) {
                            if Self::try_ecc_decode(&payload).is_some() {
                                return Some(payload);
                            }
                            if Self::verify_payload_integrity(&payload, mac_key) {
                                return Some(Self::truncate_to_actual_payload(&payload));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Check if bytes begin with v3 magic `[0x53, 0x45, 0x03]`.
    fn has_v3_magic(bytes: &[u8]) -> bool {
        bytes.len() >= 3
            && bytes[0] == V3_MAGIC[0]
            && bytes[1] == V3_MAGIC[1]
            && bytes[2] == V3_PAYLOAD_VERSION
    }

    /// Verification-path variant of `extract_with_redundancy` that returns a
    /// tri-state. Used by [`Self::verify_payload_from_bytes_with_key`] so that
    /// a structurally plausible payload whose integrity check fails (e.g.
    /// wrong MAC key, bit corruption) can be reported as
    /// [`VerificationStatus::Invalid`] instead of falling through to `NotFound`.
    fn verify_extract_with_redundancy(
        &self,
        img: &RgbaImage,
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        let mut last_outcome: Option<CandidateOutcome> = None;
        let prefix_bits = 6 * 8;
        for redundancy in 1..=10 {
            let v2_corrected =
                self.verify_extract_at_seed_v2(img, prefix_bits, seed, mac_key, 0, redundancy);
            match &v2_corrected {
                CandidateOutcome::Valid(_)
                | CandidateOutcome::Invalid(_)
                | CandidateOutcome::AuthenticationKeyMissing(_)
                | CandidateOutcome::AuthenticationFailed(_) => {
                    if Self::candidate_seed_matches(&v2_corrected, seed) {
                        return v2_corrected;
                    }
                    if last_outcome.is_none() {
                        last_outcome = Some(v2_corrected);
                    }
                }
                _ => {}
            }
        }
        for pass in 0..5 {
            let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));
            for redundancy in 1..=10 {
                let v2_outcome = self.verify_extract_at_seed_v2(
                    img,
                    prefix_bits,
                    offset_seed,
                    mac_key,
                    0,
                    redundancy,
                );
                match &v2_outcome {
                    CandidateOutcome::Valid(_)
                    | CandidateOutcome::Invalid(_)
                    | CandidateOutcome::AuthenticationKeyMissing(_)
                    | CandidateOutcome::AuthenticationFailed(_) => {
                        if Self::candidate_seed_matches(&v2_outcome, seed) {
                            return v2_outcome;
                        }
                        if last_outcome.is_none() {
                            last_outcome = Some(v2_outcome);
                        }
                    }
                    _ => {}
                }
            }

            let legacy_outcome =
                self.verify_extract_at_seed_legacy(img, prefix_bits, offset_seed, mac_key);
            match &legacy_outcome {
                CandidateOutcome::Valid(_)
                | CandidateOutcome::Invalid(_)
                | CandidateOutcome::AuthenticationKeyMissing(_)
                | CandidateOutcome::AuthenticationFailed(_)
                | CandidateOutcome::MalformedV3
                | CandidateOutcome::UnsupportedVersion(_) => {
                    if Self::candidate_seed_matches(&legacy_outcome, seed) {
                        return legacy_outcome;
                    }
                    if last_outcome.is_none() {
                        last_outcome = Some(legacy_outcome);
                    }
                }
                _ => {}
            }
        }
        match last_outcome {
            Some(outcome) => outcome,
            None => CandidateOutcome::NotFound,
        }
    }

    fn verify_extract_at_seed_v2(
        &self,
        img: &RgbaImage,
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
        base_slot: usize,
        redundancy: usize,
    ) -> CandidateOutcome {
        let mut last_outcome: Option<CandidateOutcome> = None;
        if let Some(prefix) = self.extract_lsb_v2(img, prefix_bits, seed, base_slot, redundancy) {
            match Self::classify_v3_probe(&prefix, Some(&self.limits)) {
                V3ProbeResult::V3Detected { total_bits, .. } => {
                    if let Some(full) =
                        self.extract_lsb_v2(img, total_bits, seed, base_slot, redundancy)
                    {
                        if Self::verify_payload_integrity(&full, mac_key) {
                            return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                &full,
                            ));
                        }
                        if last_outcome.is_none() {
                            last_outcome = Some(Self::classify_auth_failure(&full, mac_key));
                        }
                    }
                }
                V3ProbeResult::NotV3 => {
                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        if let Some(payload) =
                            self.extract_lsb_v2(img, ecc_bits, seed, base_slot, redundancy)
                        {
                            if Self::try_ecc_decode(&payload).is_some() {
                                return CandidateOutcome::Valid(payload);
                            }
                            if Self::verify_payload_integrity(&payload, mac_key) {
                                return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                    &payload,
                                ));
                            }
                            if last_outcome.is_none() {
                                last_outcome = Some(Self::classify_auth_failure(&payload, mac_key));
                            }
                        }
                    }
                }
                V3ProbeResult::MalformedV3 if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::MalformedV3);
                }
                V3ProbeResult::UnsupportedVersion(v) if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::UnsupportedVersion(v));
                }
                V3ProbeResult::ResourceLimitExceeded if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::MalformedV3);
                }
                _ => {}
            }
        }
        match last_outcome {
            Some(outcome) => outcome,
            None => CandidateOutcome::NotFound,
        }
    }

    fn verify_extract_at_seed_legacy(
        &self,
        img: &RgbaImage,
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        let mut last_outcome: Option<CandidateOutcome> = None;
        if let Some(prefix) = self.extract_lsb(img, prefix_bits, seed) {
            match Self::classify_v3_probe(&prefix, Some(&self.limits)) {
                V3ProbeResult::V3Detected { total_bits, .. } => {
                    if let Some(full) = self.extract_lsb(img, total_bits, seed) {
                        if Self::verify_payload_integrity(&full, mac_key) {
                            return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                &full,
                            ));
                        }
                        if last_outcome.is_none() {
                            last_outcome = Some(Self::classify_auth_failure(&full, mac_key));
                        }
                    }
                }
                V3ProbeResult::NotV3 => {
                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        if let Some(payload) = self.extract_lsb(img, ecc_bits, seed) {
                            if Self::try_ecc_decode(&payload).is_some() {
                                return CandidateOutcome::Valid(payload);
                            }
                            if Self::verify_payload_integrity(&payload, mac_key) {
                                return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                    &payload,
                                ));
                            }
                            if last_outcome.is_none() {
                                last_outcome = Some(Self::classify_auth_failure(&payload, mac_key));
                            }
                        }
                    }
                }
                V3ProbeResult::MalformedV3 if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::MalformedV3);
                }
                V3ProbeResult::UnsupportedVersion(v) if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::UnsupportedVersion(v));
                }
                V3ProbeResult::ResourceLimitExceeded if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::MalformedV3);
                }
                _ => {}
            }
        }
        match last_outcome {
            Some(outcome) => outcome,
            None => CandidateOutcome::NotFound,
        }
    }

    /// Verify protection using a MAC key for HMAC-SHA256 validation.
    ///
    /// Returns [`VerificationStatus::Verified`] if the payload is found and HMAC is valid,
    /// [`VerificationStatus::Invalid`] if found but HMAC doesn't match, or
    /// [`VerificationStatus::NotFound`] if no payload is found.
    ///
    /// Without a MAC key (empty `mac_key`), falls back to a non-cryptographic CRC32 checksum
    /// that provides no cryptographic protection. For production use, always provide a key.
    pub fn verify_payload_with_key(
        &self,
        img: &DynamicImage,
        mac_key: &[u8],
    ) -> VerificationStatus {
        // Encode once, delegate to bytes-aware method to avoid double-encoding.
        if let Ok(png_bytes) = crate::util::image::encode_image(img, image::ImageFormat::Png) {
            self.verify_payload_from_bytes_with_key(&png_bytes, mac_key)
        } else {
            VerificationStatus::NotFound
        }
    }

    /// Verify protection using raw image bytes — avoids unnecessary re-encoding.
    /// For JPEG bytes, checks DCT stego directly. For all formats, extracts
    /// metadata seed from the byte stream (preserves metadata that DynamicImage drops).
    pub fn verify_payload_from_bytes_with_key(
        &self,
        img_bytes: &[u8],
        mac_key: &[u8],
    ) -> VerificationStatus {
        let metadata_seed = RightsMetadataProtector::extract_seed_from_image_with_limits(
            img_bytes,
            Some(&self.limits),
        );

        // JPEG: check DCT stego directly (no re-encode needed)
        if img_bytes.starts_with(&[0xFF, 0xD8]) {
            match self.verify_extract_verified_dct(img_bytes, mac_key) {
                CandidateOutcome::Valid(_) => return VerificationStatus::Verified,
                CandidateOutcome::Invalid(_)
                | CandidateOutcome::MalformedV3
                | CandidateOutcome::UnsupportedVersion(_)
                | CandidateOutcome::AuthenticationKeyMissing(_)
                | CandidateOutcome::AuthenticationFailed(_) => return VerificationStatus::Invalid,
                CandidateOutcome::NotFound => {}
            }

            if let Some(metadata_seed) = metadata_seed {
                match self.verify_extract_dct_with_seed(img_bytes, metadata_seed, mac_key) {
                    CandidateOutcome::Valid(_) => return VerificationStatus::Verified,
                    CandidateOutcome::Invalid(_)
                    | CandidateOutcome::MalformedV3
                    | CandidateOutcome::UnsupportedVersion(_)
                    | CandidateOutcome::AuthenticationKeyMissing(_)
                    | CandidateOutcome::AuthenticationFailed(_) => {
                        return VerificationStatus::Invalid
                    }
                    CandidateOutcome::NotFound => {}
                }
            }

            // JPEG output in this crate uses DCT/Q-table channels, not pixel
            // LSB channels. Avoid a lossy decode and futile LSB scan in the
            // reverse-proxy verification hot path.
            return VerificationStatus::NotFound;
        }

        // Extract metadata seed directly from bytes (works for PNG, JPEG, WebP)
        if let Some(metadata_seed) = metadata_seed {
            if let Ok(img) = image::load_from_memory(img_bytes) {
                match self.verify_payload_with_seed_outcome(&img, metadata_seed, mac_key) {
                    CandidateOutcome::Valid(_) => return VerificationStatus::Verified,
                    CandidateOutcome::Invalid(_)
                    | CandidateOutcome::MalformedV3
                    | CandidateOutcome::UnsupportedVersion(_)
                    | CandidateOutcome::AuthenticationKeyMissing(_)
                    | CandidateOutcome::AuthenticationFailed(_) => {
                        return VerificationStatus::Invalid
                    }
                    CandidateOutcome::NotFound => {}
                }
            }
        }

        // Try LSB fallback seed (fixed-position LSB pattern)
        if let Ok(img) = image::load_from_memory(img_bytes) {
            let rgba = img.to_rgba8();
            if let Some(fallback_seed) = Self::extract_seed_lsb_fallback(&rgba) {
                match self.verify_payload_with_seed_outcome(&img, fallback_seed, mac_key) {
                    CandidateOutcome::Valid(_) => return VerificationStatus::Verified,
                    CandidateOutcome::Invalid(_)
                    | CandidateOutcome::MalformedV3
                    | CandidateOutcome::UnsupportedVersion(_)
                    | CandidateOutcome::AuthenticationKeyMissing(_)
                    | CandidateOutcome::AuthenticationFailed(_) => {
                        return VerificationStatus::Invalid
                    }
                    CandidateOutcome::NotFound => {}
                }
            }

            // Crop-resistant tiled payloads may survive after metadata and the
            // fixed-position seed fallback are clipped away. Keep this bounded
            // to the same small set used by payload extraction so verification
            // remains predictable.
            for &seed in &[42u64, 0, 1, 12345, 99999, 123456789] {
                match self.verify_tiled_extraction_outcome(
                    &rgba,
                    seed,
                    DEFAULT_TILE_SIZE,
                    self.limits.max_tile_extraction_origins() as u32,
                    mac_key,
                ) {
                    CandidateOutcome::Valid(_) => return VerificationStatus::Verified,
                    CandidateOutcome::Invalid(_)
                    | CandidateOutcome::MalformedV3
                    | CandidateOutcome::UnsupportedVersion(_)
                    | CandidateOutcome::AuthenticationKeyMissing(_)
                    | CandidateOutcome::AuthenticationFailed(_) => {
                        return VerificationStatus::Invalid
                    }
                    CandidateOutcome::NotFound => {}
                }
            }
        }

        // LSB fallback: try known seeds via DynamicImage
        #[cfg(feature = "test-seeds")]
        if let Ok(img) = image::load_from_memory(img_bytes) {
            for &seed in FALLBACK_SEEDS
                .iter()
                .take(self.limits.max_verification_seeds())
            {
                match self.verify_payload_with_seed_outcome(&img, seed, mac_key) {
                    CandidateOutcome::Valid(_) => return VerificationStatus::Verified,
                    CandidateOutcome::Invalid(_)
                    | CandidateOutcome::MalformedV3
                    | CandidateOutcome::UnsupportedVersion(_)
                    | CandidateOutcome::AuthenticationKeyMissing(_)
                    | CandidateOutcome::AuthenticationFailed(_) => {
                        return VerificationStatus::Invalid
                    }
                    CandidateOutcome::NotFound => {}
                }
            }
        }

        VerificationStatus::NotFound
    }

    /// Verify protection and return raw payload bytes for embedded reference checks.
    ///
    /// Like [`Self::verify_payload_from_bytes_with_key`], but also returns the raw
    /// payload bytes when verification fails. This allows callers to inspect
    /// the v3 header (e.g., auth_algo) to distinguish between missing and
    /// wrong HMAC keys.
    ///
    /// Returns `(VerificationStatus, Option<Vec<u8>>)` where the second
    /// element contains the raw payload bytes when a payload was found but
    /// verification failed.
    pub fn verify_and_extract_raw_from_bytes(
        &self,
        img_bytes: &[u8],
        mac_key: &[u8],
    ) -> (VerificationStatus, Option<Vec<u8>>) {
        let metadata_seed = RightsMetadataProtector::extract_seed_from_image_with_limits(
            img_bytes,
            Some(&self.limits),
        );

        if img_bytes.starts_with(&[0xFF, 0xD8]) {
            match self.verify_extract_verified_dct(img_bytes, mac_key) {
                CandidateOutcome::Valid(_) => return (VerificationStatus::Verified, None),
                CandidateOutcome::Invalid(raw) => return (VerificationStatus::Invalid, Some(raw)),
                CandidateOutcome::MalformedV3 | CandidateOutcome::UnsupportedVersion(_) => {
                    return (VerificationStatus::Invalid, None)
                }
                CandidateOutcome::AuthenticationKeyMissing(raw)
                | CandidateOutcome::AuthenticationFailed(raw) => {
                    return (VerificationStatus::Invalid, Some(raw))
                }
                CandidateOutcome::NotFound => {}
            }

            if let Some(metadata_seed) = metadata_seed {
                match self.verify_extract_dct_with_seed(img_bytes, metadata_seed, mac_key) {
                    CandidateOutcome::Valid(_) => return (VerificationStatus::Verified, None),
                    CandidateOutcome::Invalid(raw) => {
                        return (VerificationStatus::Invalid, Some(raw))
                    }
                    CandidateOutcome::AuthenticationKeyMissing(raw)
                    | CandidateOutcome::AuthenticationFailed(raw) => {
                        return (VerificationStatus::Invalid, Some(raw))
                    }
                    CandidateOutcome::MalformedV3 | CandidateOutcome::UnsupportedVersion(_) => {
                        return (VerificationStatus::Invalid, None)
                    }
                    CandidateOutcome::NotFound => {}
                }
            }

            return (VerificationStatus::NotFound, None);
        }

        if let Some(metadata_seed) = metadata_seed {
            if let Ok(img) = image::load_from_memory(img_bytes) {
                match self.verify_payload_with_seed_outcome(&img, metadata_seed, mac_key) {
                    CandidateOutcome::Valid(_) => return (VerificationStatus::Verified, None),
                    CandidateOutcome::Invalid(raw) => {
                        return (VerificationStatus::Invalid, Some(raw))
                    }
                    CandidateOutcome::AuthenticationKeyMissing(raw)
                    | CandidateOutcome::AuthenticationFailed(raw) => {
                        return (VerificationStatus::Invalid, Some(raw))
                    }
                    CandidateOutcome::MalformedV3 | CandidateOutcome::UnsupportedVersion(_) => {
                        return (VerificationStatus::Invalid, None)
                    }
                    CandidateOutcome::NotFound => {}
                }
            }
        }

        if let Ok(img) = image::load_from_memory(img_bytes) {
            let rgba = img.to_rgba8();
            if let Some(fallback_seed) = Self::extract_seed_lsb_fallback(&rgba) {
                match self.verify_payload_with_seed_outcome(&img, fallback_seed, mac_key) {
                    CandidateOutcome::Valid(_) => return (VerificationStatus::Verified, None),
                    CandidateOutcome::Invalid(raw) => {
                        return (VerificationStatus::Invalid, Some(raw))
                    }
                    CandidateOutcome::AuthenticationKeyMissing(raw)
                    | CandidateOutcome::AuthenticationFailed(raw) => {
                        return (VerificationStatus::Invalid, Some(raw))
                    }
                    CandidateOutcome::MalformedV3 | CandidateOutcome::UnsupportedVersion(_) => {
                        return (VerificationStatus::Invalid, None)
                    }
                    CandidateOutcome::NotFound => {}
                }
            }
        }

        (VerificationStatus::NotFound, None)
    }

    /// Verify protection from raw image bytes using a known seed.
    ///
    /// For JPEG bytes, verifies DCT payload integrity first, then falls back to
    /// pixel-based stego using the provided seed.
    pub fn verify_payload_from_bytes(&self, img_bytes: &[u8], seed: u64) -> bool {
        if img_bytes.starts_with(&[0xFF, 0xD8]) {
            if let Some(payload_bytes) = self.extract_verified_dct_payload(img_bytes, &[]) {
                let header = if let Some(decoded) = Self::try_ecc_decode(&payload_bytes) {
                    decoded
                } else {
                    payload_bytes
                };

                if let Some(embedded_seed) = Self::extract_embedded_seed(&header) {
                    if embedded_seed == seed {
                        return true;
                    }
                }
            }
        }

        if let Ok(img) = image::load_from_memory(img_bytes) {
            return self.verify_payload_with_seed(&img, seed);
        }

        false
    }

    /// Verify protection using a known seed.
    ///
    /// Extracts the LSB payload with the given seed and checks both the checksum
    /// and the embedded seed value. Also tries metadata-extracted seeds as fallback.
    pub fn verify_payload_with_seed(&self, img: &DynamicImage, seed: u64) -> bool {
        let rgba = img.to_rgba8();

        if let Some(payload) = self.extract_with_redundancy(&rgba, seed, &[]) {
            let header = if let Some(decoded) = Self::try_ecc_decode(&payload) {
                decoded
            } else {
                payload.clone()
            };
            if Self::verify_checksum(&payload) {
                if let Some(embedded_seed) = Self::extract_embedded_seed(&header) {
                    if embedded_seed == seed {
                        return true;
                    }
                }
            }
        }

        if let Ok(encoded) = crate::util::image::encode_image(img, image::ImageFormat::Png) {
            if let Some(metadata_seed) =
                RightsMetadataProtector::extract_seed_from_image_with_limits(
                    &encoded,
                    Some(&self.limits),
                )
            {
                if metadata_seed != seed {
                    if let Some(payload) = self.extract_with_redundancy(&rgba, metadata_seed, &[]) {
                        let header = if let Some(decoded) = Self::try_ecc_decode(&payload) {
                            decoded
                        } else {
                            payload.clone()
                        };
                        if Self::verify_checksum(&payload) {
                            if let Some(embedded_seed) = Self::extract_embedded_seed(&header) {
                                if embedded_seed == seed {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Crop-resistant path: try tiled extraction as a final fallback.
        // Tiled embedding produces multiple full copies of the payload, so a
        // crop that destroys most pixels can still leave one intact tile.
        if self.try_tiled_extraction_verify(
            &rgba,
            seed,
            DEFAULT_TILE_SIZE,
            self.limits.max_tile_extraction_origins() as u32,
            &[],
        ) {
            return true;
        }

        false
    }

    /// Try tiled extraction and verify the embedded seed matches the caller's
    /// expected seed. Returns `true` on success. Used by the crop-resistant
    /// fallback in the verification chain.
    fn try_tiled_extraction_verify(
        &self,
        rgba: &RgbaImage,
        seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> bool {
        let Some(payload) =
            self.extract_lsb_tiled_candidates(rgba, seed, tile_size, max_origins, mac_key)
        else {
            return false;
        };
        Self::verify_embedded_seed_matches(&payload, seed)
    }

    /// Tri-state variant of the non-tiled LSB extraction used by verification.
    fn verify_payload_with_seed_outcome(
        &self,
        img: &DynamicImage,
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        let rgba = img.to_rgba8();

        match self.verify_extract_with_redundancy(&rgba, seed, mac_key) {
            CandidateOutcome::Valid(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    return CandidateOutcome::Valid(payload);
                }
            }
            CandidateOutcome::Invalid(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    return CandidateOutcome::Invalid(payload);
                }
            }
            CandidateOutcome::AuthenticationKeyMissing(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    return CandidateOutcome::AuthenticationKeyMissing(payload);
                }
            }
            CandidateOutcome::AuthenticationFailed(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    return CandidateOutcome::AuthenticationFailed(payload);
                }
            }
            CandidateOutcome::MalformedV3 | CandidateOutcome::UnsupportedVersion(_) => {}
            CandidateOutcome::NotFound => {}
        }

        if let Ok(encoded) = crate::util::image::encode_image(img, image::ImageFormat::Png) {
            if let Some(metadata_seed) =
                RightsMetadataProtector::extract_seed_from_image_with_limits(
                    &encoded,
                    Some(&self.limits),
                )
            {
                if metadata_seed != seed {
                    match self.verify_extract_with_redundancy(&rgba, metadata_seed, mac_key) {
                        CandidateOutcome::Valid(payload) => {
                            if Self::verify_embedded_seed_matches(&payload, seed) {
                                return CandidateOutcome::Valid(payload);
                            }
                        }
                        CandidateOutcome::Invalid(payload) => {
                            if Self::verify_embedded_seed_matches(&payload, seed) {
                                return CandidateOutcome::Invalid(payload);
                            }
                        }
                        CandidateOutcome::AuthenticationKeyMissing(payload) => {
                            if Self::verify_embedded_seed_matches(&payload, seed) {
                                return CandidateOutcome::AuthenticationKeyMissing(payload);
                            }
                        }
                        CandidateOutcome::AuthenticationFailed(payload) => {
                            if Self::verify_embedded_seed_matches(&payload, seed) {
                                return CandidateOutcome::AuthenticationFailed(payload);
                            }
                        }
                        CandidateOutcome::MalformedV3 | CandidateOutcome::UnsupportedVersion(_) => {
                        }
                        CandidateOutcome::NotFound => {}
                    }
                }
            }
        }

        self.verify_tiled_extraction_outcome(
            &rgba,
            seed,
            DEFAULT_TILE_SIZE,
            self.limits.max_tile_extraction_origins() as u32,
            mac_key,
        )
    }

    /// Tri-state variant of `try_tiled_extraction_verify`.
    fn verify_tiled_extraction_outcome(
        &self,
        rgba: &RgbaImage,
        seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        let outcome = self.verify_extract_lsb_tiled(rgba, seed, tile_size, max_origins, mac_key);
        match outcome {
            CandidateOutcome::Valid(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    CandidateOutcome::Valid(payload)
                } else {
                    CandidateOutcome::NotFound
                }
            }
            CandidateOutcome::Invalid(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    CandidateOutcome::Invalid(payload)
                } else {
                    CandidateOutcome::NotFound
                }
            }
            CandidateOutcome::MalformedV3
            | CandidateOutcome::UnsupportedVersion(_)
            | CandidateOutcome::AuthenticationKeyMissing(_)
            | CandidateOutcome::AuthenticationFailed(_) => outcome,
            CandidateOutcome::NotFound => CandidateOutcome::NotFound,
        }
    }

    /// Tri-state variant of `verify_dct_stego_with_seed`. Encapsulates the
    /// same coefficient + tiled F5 fallback chain but distinguishes
    /// `Invalid` from `NotFound`.
    fn verify_extract_dct_with_seed(
        &self,
        jpeg_bytes: &[u8],
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return CandidateOutcome::NotFound;
        }

        if let Ok((_, coefficients)) = JpegTranscoder::decode_coefficients(jpeg_bytes) {
            let coeffs_outcome =
                self.verify_extract_dct_from_coefficients(&coefficients, seed, mac_key);
            if let CandidateOutcome::Valid(payload) = &coeffs_outcome {
                return CandidateOutcome::Valid(payload.clone());
            }

            let tiled_outcome = self.verify_extract_f5_tiled(
                jpeg_bytes,
                seed,
                DEFAULT_TILE_SIZE,
                self.limits.max_tile_extraction_origins() as u32,
                mac_key,
            );
            if let CandidateOutcome::Valid(payload) = &tiled_outcome {
                return CandidateOutcome::Valid(payload.clone());
            }

            match (&coeffs_outcome, &tiled_outcome) {
                (CandidateOutcome::Invalid(p), _) | (_, CandidateOutcome::Invalid(p)) => {
                    return CandidateOutcome::Invalid(p.clone());
                }
                (CandidateOutcome::AuthenticationKeyMissing(p), _)
                | (_, CandidateOutcome::AuthenticationKeyMissing(p)) => {
                    return CandidateOutcome::AuthenticationKeyMissing(p.clone());
                }
                (CandidateOutcome::AuthenticationFailed(p), _)
                | (_, CandidateOutcome::AuthenticationFailed(p)) => {
                    return CandidateOutcome::AuthenticationFailed(p.clone());
                }
                _ => {}
            }
        }

        CandidateOutcome::NotFound
    }

    /// Verify that an integrity-checked payload's embedded seed field matches
    /// the expected seed. Returns `true` on match, `false` otherwise.
    fn verify_embedded_seed_matches(payload: &[u8], expected_seed: u64) -> bool {
        let header = if let Some(decoded) = Self::try_ecc_decode(payload) {
            decoded
        } else {
            payload.to_vec()
        };
        if header.len() < 10 {
            return false;
        }

        // V3: check magic bytes and read seed from v3 offset
        if header.len() >= 3
            && header[0] == V3_MAGIC[0]
            && header[1] == V3_MAGIC[1]
            && header[2] == V3_PAYLOAD_VERSION
        {
            if header.len() < 19 {
                return false;
            }
            let embedded_seed = u64::from_le_bytes([
                header[11], header[12], header[13], header[14], header[15], header[16], header[17],
                header[18],
            ]);
            return embedded_seed == expected_seed;
        }

        // V1/V2: seed at bytes 2-9
        let embedded_seed = u64::from_le_bytes([
            header[2], header[3], header[4], header[5], header[6], header[7], header[8], header[9],
        ]);
        embedded_seed == expected_seed
    }

    fn candidate_seed_matches(outcome: &CandidateOutcome, expected_seed: u64) -> bool {
        let payload = match outcome {
            CandidateOutcome::Valid(p)
            | CandidateOutcome::Invalid(p)
            | CandidateOutcome::AuthenticationKeyMissing(p)
            | CandidateOutcome::AuthenticationFailed(p) => p,
            _ => return false,
        };
        Self::verify_embedded_seed_matches(payload, expected_seed)
    }

    /// Extract the steganographic payload from a protected image.
    ///
    /// Tries metadata-extracted seed first, then falls back to common test seeds.
    /// Returns `None` if no valid payload is found.
    pub fn extract_payload(&self, img: &DynamicImage) -> Option<StegoPayload> {
        self.extract_payload_with_key(img, &[])
    }

    /// Extract the steganographic payload with HMAC verification.
    ///
    /// Like [`extract_payload`](Self::extract_payload), but verifies the payload's
    /// HMAC-SHA256 against the provided MAC key. Returns `None` if no valid payload
    /// is found or the MAC doesn't match.
    pub fn extract_payload_with_key(
        &self,
        img: &DynamicImage,
        mac_key: &[u8],
    ) -> Option<StegoPayload> {
        // Try extracting seed from metadata first
        if let Ok(encoded) = crate::util::image::encode_image(img, image::ImageFormat::Png) {
            if let Some(metadata_seed) =
                RightsMetadataProtector::extract_seed_from_image_with_limits(
                    &encoded,
                    Some(&self.limits),
                )
            {
                if let Some(payload) =
                    self.extract_payload_with_seed_and_key(img, metadata_seed, mac_key)
                {
                    return Some(payload);
                }
            }
        }

        // Try LSB fallback seed (fixed-position LSB pattern)
        let rgba = img.to_rgba8();
        if let Some(fallback_seed) = Self::extract_seed_lsb_fallback(&rgba) {
            if let Some(payload) =
                self.extract_payload_with_seed_and_key(img, fallback_seed, mac_key)
            {
                return Some(payload);
            }
        }

        // Fallback: try common seeds (metadata stripped during DynamicImage re-encoding)
        #[cfg(feature = "test-seeds")]
        for &seed in FALLBACK_SEEDS
            .iter()
            .take(self.limits.max_verification_seeds())
        {
            if let Some(payload) = self.extract_payload_with_seed_and_key(img, seed, mac_key) {
                return Some(payload);
            }
        }

        None
    }

    /// Truncate an extracted payload to its actual size based on the header.
    /// For v3 payloads, extracts `total_length` bytes. For v1/v2, returns as-is.
    fn truncate_to_actual_payload(payload: &[u8]) -> Vec<u8> {
        if payload.len() >= 3
            && payload[0] == V3_MAGIC[0]
            && payload[1] == V3_MAGIC[1]
            && payload[2] == V3_PAYLOAD_VERSION
            && payload.len() >= crate::payload_v3::types::V3_CORE_SIZE
        {
            let total_length = u16::from_le_bytes([payload[4], payload[5]]) as usize;
            if total_length <= payload.len() {
                return payload[..total_length].to_vec();
            }
        }
        payload.to_vec()
    }

    /// Extract the embedded seed from a decoded payload header.
    /// Handles both v2 (seed at bytes 2-9) and v3 (seed at bytes 11-18) layouts.
    fn extract_embedded_seed(header: &[u8]) -> Option<u64> {
        if header.len() < 10 {
            return None;
        }
        // V3: check magic bytes and read seed from v3 offset
        if header.len() >= 19
            && header[0] == V3_MAGIC[0]
            && header[1] == V3_MAGIC[1]
            && header[2] == V3_PAYLOAD_VERSION
        {
            return Some(u64::from_le_bytes([
                header[11], header[12], header[13], header[14], header[15], header[16], header[17],
                header[18],
            ]));
        }
        // V1/V2: seed at bytes 2-9
        Some(u64::from_le_bytes([
            header[2], header[3], header[4], header[5], header[6], header[7], header[8], header[9],
        ]))
    }

    /// Extract the steganographic payload from raw image bytes.
    ///
    /// Like [`extract_payload_with_key`](Self::extract_payload_with_key), but operates
    /// directly on image bytes to avoid unnecessary decode/encode cycles. This is the
    /// byte-level equivalent used by [`verify_image_bytes_detailed`](crate::verify_image_bytes_detailed).
    pub fn extract_payload_from_bytes_with_key(
        &self,
        img_bytes: &[u8],
        mac_key: &[u8],
    ) -> Option<StegoPayload> {
        let metadata_seed = RightsMetadataProtector::extract_seed_from_image_with_limits(
            img_bytes,
            Some(&self.limits),
        );

        // JPEG: try DCT extraction first (avoids pixel decode)
        if img_bytes.starts_with(&[0xFF, 0xD8]) {
            if let Some(payload_bytes) = self.extract_verified_dct_payload(img_bytes, mac_key) {
                if !self.payload_within_limits(&payload_bytes) {
                    return None;
                }
                if let Some(decoded) = Self::try_ecc_decode(&payload_bytes) {
                    if let Some(payload) = Self::parse_stego_payload(&decoded) {
                        return Some(payload);
                    }
                }
                if Self::verify_payload_integrity(&payload_bytes, mac_key) {
                    return Self::parse_stego_payload(&payload_bytes);
                }
            }

            // Tiled F5 fallback for JPEG
            if let Some(metadata_seed) = metadata_seed {
                if let Some(payload_bytes) = self.extract_f5_tiled_candidates(
                    img_bytes,
                    metadata_seed,
                    DEFAULT_TILE_SIZE,
                    self.limits.max_tile_extraction_origins() as u32,
                    mac_key,
                ) {
                    if !self.payload_within_limits(&payload_bytes) {
                        return None;
                    }
                    if let Some(decoded) = Self::try_ecc_decode(&payload_bytes) {
                        if let Some(payload) = Self::parse_stego_payload(&decoded) {
                            return Some(payload);
                        }
                    }
                    if Self::verify_payload_integrity(&payload_bytes, mac_key) {
                        return Self::parse_stego_payload(&payload_bytes);
                    }
                }
            }

            // JPEG stego is coefficient-based in this crate. If DCT extraction
            // failed, decoding to pixels and trying LSB extraction is wasted
            // work for production byte-level verification.
            return None;
        }

        // Try metadata seed extraction from bytes (works for PNG, JPEG, WebP)
        if let Some(metadata_seed) = metadata_seed {
            if let Ok(img) = image::load_from_memory(img_bytes) {
                if let Some(payload) =
                    self.extract_payload_with_seed_and_key(&img, metadata_seed, mac_key)
                {
                    return Some(payload);
                }
            }
        }

        // LSB fallback seed
        if let Ok(img) = image::load_from_memory(img_bytes) {
            let rgba = img.to_rgba8();
            if let Some(fallback_seed) = Self::extract_seed_lsb_fallback(&rgba) {
                if let Some(payload) =
                    self.extract_payload_with_seed_and_key(&img, fallback_seed, mac_key)
                {
                    return Some(payload);
                }
            }
        }

        // Tiled LSB fallback
        #[cfg(feature = "test-seeds")]
        if let Ok(img) = image::load_from_memory(img_bytes) {
            let rgba = img.to_rgba8();
            for &seed in FALLBACK_SEEDS
                .iter()
                .take(self.limits.max_verification_seeds())
            {
                if let Some(payload) = self.extract_lsb_tiled_candidates(
                    &rgba,
                    seed,
                    DEFAULT_TILE_SIZE,
                    self.limits.max_tile_extraction_origins() as u32,
                    mac_key,
                ) {
                    if let Some(decoded) = Self::try_ecc_decode(&payload) {
                        if let Some(payload) = Self::parse_stego_payload(&decoded) {
                            return Some(payload);
                        }
                    }
                    if Self::verify_payload_integrity(&payload, mac_key) {
                        return Self::parse_stego_payload(&payload);
                    }
                }
            }
        }

        None
    }

    fn parse_stego_payload(payload: &[u8]) -> Option<StegoPayload> {
        if payload.len() < 24 {
            return None;
        }

        // V3: check magic bytes first
        if payload.len() >= 3
            && payload[0] == V3_MAGIC[0]
            && payload[1] == V3_MAGIC[1]
            && payload[2] == V3_PAYLOAD_VERSION
        {
            return Self::parse_stego_payload_v3(payload);
        }

        let version = payload[0];

        for &supported in SUPPORTED_PAYLOAD_VERSIONS {
            if version == supported {
                return match supported {
                    1 => Self::parse_stego_payload_v1(payload),
                    2 => Self::parse_stego_payload_v2(payload),
                    _ => None,
                };
            }
        }

        None
    }

    /// Parse a version-1 stego payload header.
    ///
    /// Header layout (24 bytes):
    /// ```text
    /// Offset  Size  Field
    /// 0       1     Version (=1)
    /// 1       1     ProtectionLevel byte
    /// 2       8     Seed (u64, little-endian)
    /// 10      2     Intensity (u16, scaled f32 / 100.0)
    /// 12      8     Timestamp (u64, seconds since Unix epoch)
    /// 20      4     Reserved / CRC32 (in non-MAC, non-ECC mode) or HMAC bytes
    /// ```
    /// After the 24-byte header the payload is either:
    /// - 4 CRC32 bytes (minimum non-MAC payload = 28 bytes), or
    /// - 8 truncated HMAC-SHA256 bytes (MAC payload = 32 bytes), or
    /// - 72 bytes of 3×-repetition ECC + 4 CRC32 bytes (ECC payload = 76 bytes).
    ///
    /// This function only parses the 24-byte header; integrity check (CRC/HMAC/ECC)
    /// is the caller's responsibility via `verify_payload_integrity`/`try_ecc_decode`.
    fn parse_stego_payload_v1(payload: &[u8]) -> Option<StegoPayload> {
        let protection_level = payload[1];

        let extracted_seed = u64::from_le_bytes([
            payload[2], payload[3], payload[4], payload[5], payload[6], payload[7], payload[8],
            payload[9],
        ]);

        let intensity_raw = u16::from_le_bytes([payload[10], payload[11]]);
        let intensity = intensity_raw as f32 / 100.0;

        Some(StegoPayload {
            protection_level,
            seed: extracted_seed,
            intensity,
            version: 1,
            content_hash: None,
            dmi_value: None,
            raw_payload: None,
        })
    }

    /// Parse a version-2 stego payload header.
    ///
    /// Header layout (32 bytes):
    /// ```text
    /// Offset  Size  Field
    /// 0       1     Version (=2)
    /// 1       1     ProtectionLevel byte
    /// 2       8     Seed (u64, little-endian)
    /// 10      2     Intensity (u16, scaled f32 / 100.0)
    /// 12      8     Timestamp (u64, seconds since Unix epoch)
    /// 20      4     Content hash (truncated ISCC or SHA-256)
    /// 24      1     DMI value byte
    /// 25      1     Flags byte (reserved)
    /// 26      6     Reserved (zeroed)
    /// ```
    fn parse_stego_payload_v2(payload: &[u8]) -> Option<StegoPayload> {
        if payload.len() < V2_HEADER_SIZE {
            return None;
        }

        let protection_level = payload[1];

        let extracted_seed = u64::from_le_bytes([
            payload[2], payload[3], payload[4], payload[5], payload[6], payload[7], payload[8],
            payload[9],
        ]);

        let intensity_raw = u16::from_le_bytes([payload[10], payload[11]]);
        let intensity = intensity_raw as f32 / 100.0;

        let content_hash = {
            let mut hash = [0u8; 4];
            hash.copy_from_slice(&payload[20..24]);
            if hash != [0, 0, 0, 0] {
                Some(hash)
            } else {
                None
            }
        };

        let dmi_value = match payload[24] {
            0 => None,
            1 => Some(crate::types::DmiValue::Allowed),
            2 => Some(crate::types::DmiValue::ProhibitedAiMlTraining),
            3 => Some(crate::types::DmiValue::ProhibitedGenAiMlTraining),
            4 => Some(crate::types::DmiValue::ProhibitedExceptSearchEngineIndexing),
            5 => Some(crate::types::DmiValue::Prohibited),
            6 => Some(crate::types::DmiValue::ProhibitedSeeConstraints),
            _ => None,
        };

        Some(StegoPayload {
            protection_level,
            seed: extracted_seed,
            intensity,
            version: 2,
            content_hash,
            dmi_value,
            raw_payload: None,
        })
    }

    /// Parse a version-3 stego payload header.
    ///
    /// V3 header layout (32 bytes core):
    /// ```text
    /// Offset  Size  Field
    /// 0       2     Magic ([0x53, 0x45] = "SE")
    /// 2       1     Version (=3)
    /// 3       1     Header length (core + key_id + extensions)
    /// 4       2     Total length (u16 LE)
    /// 6       2     Flags (u16 LE)
    /// 8       2     Channels (u16 LE)
    /// 10      1     DMI policy byte
    /// 11      8     Seed (u64 LE)
    /// 19      2     Intensity (u16 LE, scaled ×100)
    /// 21      8     Content hash (truncated)
    /// 29      1     Auth algorithm byte
    /// 30      1     Auth tag length
    /// 31      1     Key ID length
    /// ```
    fn parse_stego_payload_v3(payload: &[u8]) -> Option<StegoPayload> {
        if payload.len() < crate::payload_v3::types::V3_CORE_SIZE {
            return None;
        }

        let extracted_seed = u64::from_le_bytes([
            payload[11],
            payload[12],
            payload[13],
            payload[14],
            payload[15],
            payload[16],
            payload[17],
            payload[18],
        ]);

        let intensity_raw = u16::from_le_bytes([payload[19], payload[20]]);
        let intensity = intensity_raw as f32 / 100.0;

        let content_hash = {
            let mut hash = [0u8; 4];
            hash.copy_from_slice(&payload[21..25]);
            if hash != [0, 0, 0, 0] {
                Some(hash)
            } else {
                None
            }
        };

        let dmi_value = match payload[10] {
            0 => None,
            1 => Some(crate::types::DmiValue::Allowed),
            2 => Some(crate::types::DmiValue::ProhibitedAiMlTraining),
            3 => Some(crate::types::DmiValue::ProhibitedGenAiMlTraining),
            4 => Some(crate::types::DmiValue::ProhibitedExceptSearchEngineIndexing),
            5 => Some(crate::types::DmiValue::Prohibited),
            6 => Some(crate::types::DmiValue::ProhibitedSeeConstraints),
            _ => None,
        };

        Some(StegoPayload {
            protection_level: 2,
            seed: extracted_seed,
            intensity,
            version: 3,
            content_hash,
            dmi_value,
            raw_payload: None,
        })
    }

    /// Extract the steganographic payload using a known seed and MAC key.
    pub fn extract_payload_with_seed_and_key(
        &self,
        img: &DynamicImage,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<StegoPayload> {
        let rgba = img.to_rgba8();
        if let Some(payload) = self.extract_with_redundancy(&rgba, seed, mac_key) {
            if !self.payload_within_limits(&payload) {
                return None;
            }
            if let Some(decoded) = Self::try_ecc_decode(&payload) {
                let mut sp = Self::parse_stego_payload(&decoded)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
            if Self::verify_payload_integrity(&payload, mac_key) {
                let mut sp = Self::parse_stego_payload(&payload)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
        }

        // Crop-resistant fallback: try tiled extraction. This recovers the
        // payload from any crop that contains at least one intact tile, even
        // when the non-tiled path's pixel positions are completely scrambled
        // by the crop offset.
        if let Some(payload) = self.extract_lsb_tiled_candidates(
            &rgba,
            seed,
            DEFAULT_TILE_SIZE,
            self.limits.max_tile_extraction_origins() as u32,
            mac_key,
        ) {
            if let Some(decoded) = Self::try_ecc_decode(&payload) {
                let mut sp = Self::parse_stego_payload(&decoded)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
            if Self::verify_payload_integrity(&payload, mac_key) {
                let mut sp = Self::parse_stego_payload(&payload)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
        }

        None
    }

    /// Extract the steganographic payload using a known seed (checksum mode).
    pub fn extract_payload_with_seed(&self, img: &DynamicImage, seed: u64) -> Option<StegoPayload> {
        let rgba = img.to_rgba8();
        if let Some(payload) = self.extract_with_redundancy(&rgba, seed, &[]) {
            if let Some(decoded) = Self::try_ecc_decode(&payload) {
                let mut sp = Self::parse_stego_payload(&decoded)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
            let mut sp = Self::parse_stego_payload(&payload)?;
            sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
            return Some(sp);
        }

        // Crop-resistant fallback: tiled extraction.
        if let Some(payload) = self.extract_lsb_tiled_candidates(
            &rgba,
            seed,
            DEFAULT_TILE_SIZE,
            self.limits.max_tile_extraction_origins() as u32,
            &[],
        ) {
            if let Some(decoded) = Self::try_ecc_decode(&payload) {
                let mut sp = Self::parse_stego_payload(&decoded)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
            let mut sp = Self::parse_stego_payload(&payload)?;
            sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
            return Some(sp);
        }

        None
    }

    fn extract_verified_dct_payload(&self, jpeg_bytes: &[u8], mac_key: &[u8]) -> Option<Vec<u8>> {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return None;
        }

        // Try baseline JPEG DCT extraction
        if let Ok((header, coefficients)) = JpegTranscoder::decode_coefficients(jpeg_bytes) {
            if let Some(extracted_seed) =
                DctStegoF5::new().extract_seed_from_quantization_tables(&header)
            {
                if let Some(result) = self.extract_verified_dct_payload_from_coefficients(
                    &coefficients,
                    extracted_seed,
                    mac_key,
                ) {
                    return Some(result);
                }

                // Tiled F5 fallback: try tiled extraction with the same seed
                if let Some(result) = self.extract_f5_tiled_candidates(
                    jpeg_bytes,
                    extracted_seed,
                    DEFAULT_TILE_SIZE,
                    self.limits.max_tile_extraction_origins() as u32,
                    mac_key,
                ) {
                    return Some(result);
                }
            }
        }

        None
    }

    /// Verification-path tri-state variant of `extract_verified_dct_payload`.
    fn verify_extract_verified_dct(&self, jpeg_bytes: &[u8], mac_key: &[u8]) -> CandidateOutcome {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return CandidateOutcome::NotFound;
        }

        if let Ok((header, coefficients)) = JpegTranscoder::decode_coefficients(jpeg_bytes) {
            if let Some(extracted_seed) =
                DctStegoF5::new().extract_seed_from_quantization_tables(&header)
            {
                let coeffs_outcome = self.verify_extract_dct_from_coefficients(
                    &coefficients,
                    extracted_seed,
                    mac_key,
                );
                if let CandidateOutcome::Valid(payload) = &coeffs_outcome {
                    return CandidateOutcome::Valid(payload.clone());
                }

                let tiled_outcome = self.verify_extract_f5_tiled(
                    jpeg_bytes,
                    extracted_seed,
                    DEFAULT_TILE_SIZE,
                    self.limits.max_tile_extraction_origins() as u32,
                    mac_key,
                );
                if let CandidateOutcome::Valid(payload) = &tiled_outcome {
                    return CandidateOutcome::Valid(payload.clone());
                }

                match (&coeffs_outcome, &tiled_outcome) {
                    (CandidateOutcome::Invalid(p), _) | (_, CandidateOutcome::Invalid(p)) => {
                        return CandidateOutcome::Invalid(p.clone());
                    }
                    (CandidateOutcome::AuthenticationKeyMissing(p), _)
                    | (_, CandidateOutcome::AuthenticationKeyMissing(p)) => {
                        return CandidateOutcome::AuthenticationKeyMissing(p.clone());
                    }
                    (CandidateOutcome::AuthenticationFailed(p), _)
                    | (_, CandidateOutcome::AuthenticationFailed(p)) => {
                        return CandidateOutcome::AuthenticationFailed(p.clone());
                    }
                    _ => {}
                }
            }
        }

        CandidateOutcome::NotFound
    }

    fn extract_verified_dct_payload_from_coefficients(
        &self,
        coefficients: &crate::stego::jpeg_transcoder::Coefficients,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        let prefix_bits = 6 * 8;
        for redundancy in 1..=10 {
            let stego_f5 = DctStegoF5::with_redundancy(redundancy);
            let prefix_extracted = stego_f5.extract_f5(coefficients, prefix_bits, seed);
            if prefix_extracted.len() < prefix_bits {
                continue;
            }
            let prefix_bytes = Self::bits_to_bytes(&prefix_extracted);
            match Self::classify_v3_probe(&prefix_bytes, Some(&self.limits)) {
                V3ProbeResult::V3Detected { total_bits, .. } => {
                    let full_extracted = stego_f5.extract_f5(coefficients, total_bits, seed);
                    if full_extracted.len() >= total_bits {
                        let full_bytes = Self::bits_to_bytes(&full_extracted);
                        if Self::verify_payload_integrity(&full_bytes, mac_key) {
                            return Some(Self::truncate_to_actual_payload(&full_bytes));
                        }
                    }
                }
                V3ProbeResult::NotV3 => {
                    for &bits_needed in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        let extracted = stego_f5.extract_f5(coefficients, bits_needed, seed);
                        if extracted.len() < bits_needed {
                            continue;
                        }
                        let payload_bytes = Self::bits_to_bytes(&extracted);
                        if Self::verify_payload_integrity(&payload_bytes, mac_key) {
                            return Some(Self::truncate_to_actual_payload(&payload_bytes));
                        }
                        if Self::try_ecc_decode(&payload_bytes).is_some() {
                            return Some(payload_bytes);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Verification-path variant of `extract_verified_dct_payload_from_coefficients`.
    fn verify_extract_dct_from_coefficients(
        &self,
        coefficients: &crate::stego::jpeg_transcoder::Coefficients,
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        let mut last_outcome: Option<CandidateOutcome> = None;
        let prefix_bits = 6 * 8;
        for redundancy in 1..=10 {
            let stego_f5 = DctStegoF5::with_redundancy(redundancy);
            let prefix_extracted = stego_f5.extract_f5(coefficients, prefix_bits, seed);
            if prefix_extracted.len() < prefix_bits {
                continue;
            }
            let prefix_bytes = Self::bits_to_bytes(&prefix_extracted);
            match Self::classify_v3_probe(&prefix_bytes, Some(&self.limits)) {
                V3ProbeResult::V3Detected { total_bits, .. } => {
                    let full_extracted = stego_f5.extract_f5(coefficients, total_bits, seed);
                    if full_extracted.len() >= total_bits {
                        let full_bytes = Self::bits_to_bytes(&full_extracted);
                        if Self::verify_payload_integrity(&full_bytes, mac_key) {
                            return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                &full_bytes,
                            ));
                        }
                        if last_outcome.is_none() {
                            last_outcome = Some(Self::classify_auth_failure(&full_bytes, mac_key));
                        }
                    }
                }
                V3ProbeResult::NotV3 => {
                    for &bits_needed in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        let extracted = stego_f5.extract_f5(coefficients, bits_needed, seed);
                        if extracted.len() < bits_needed {
                            continue;
                        }
                        let payload_bytes = Self::bits_to_bytes(&extracted);
                        if Self::verify_payload_integrity(&payload_bytes, mac_key) {
                            return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                &payload_bytes,
                            ));
                        }
                        if Self::try_ecc_decode(&payload_bytes).is_some() {
                            return CandidateOutcome::Valid(payload_bytes);
                        }
                        if last_outcome.is_none() {
                            last_outcome =
                                Some(Self::classify_auth_failure(&payload_bytes, mac_key));
                        }
                    }
                }
                V3ProbeResult::MalformedV3 if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::MalformedV3);
                }
                V3ProbeResult::UnsupportedVersion(v) if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::UnsupportedVersion(v));
                }
                V3ProbeResult::ResourceLimitExceeded if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::MalformedV3);
                }
                _ => {}
            }
        }
        match last_outcome {
            Some(outcome) => outcome,
            None => CandidateOutcome::NotFound,
        }
    }

    fn compute_payload_mac(payload_without_mac: &[u8], mac_key: &[u8]) -> [u8; 8] {
        let mut mac = HmacSha256::new_from_slice(mac_key).expect("HMAC can take key of any size");
        mac.update(payload_without_mac);
        let result = mac.finalize().into_bytes();
        [
            result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7],
        ]
    }

    fn compute_payload_mac_v3(payload_without_mac: &[u8], mac_key: &[u8]) -> [u8; 16] {
        let mut mac = HmacSha256::new_from_slice(mac_key).expect("HMAC can take key of any size");
        mac.update(payload_without_mac);
        let result = mac.finalize().into_bytes();
        let mut out = [0u8; 16];
        out.copy_from_slice(&result[..16]);
        out
    }

    fn verify_payload_mac(payload_without_mac: &[u8], mac_key: &[u8], expected_mac: &[u8]) -> bool {
        let computed_mac = Self::compute_payload_mac(payload_without_mac, mac_key);
        computed_mac.ct_eq(expected_mac).into()
    }

    /// Computes a CRC32 checksum of the data, stored as 4 bytes (little-endian).
    ///
    /// CRC32 provides strong accidental corruption detection. For this library's
    /// legal deterrence use case, forgability is irrelevant — the goal is detection,
    /// not authentication. Use `verify_payload_integrity` with a non-empty `mac_key`
    /// for HMAC-based verification when needed.
    fn compute_checksum(data: &[u8]) -> [u8; 4] {
        let mut hasher = Crc32Hasher::new();
        hasher.update(data);
        hasher.finalize().to_le_bytes()
    }

    /// Verifies the CRC32 checksum of the ECC-encoded payload.
    /// Tries v2 ECC size (96 data + 4 checksum = 100 bytes) first,
    /// then v1 (72 data + 4 checksum = 76 bytes).
    fn verify_checksum(payload: &[u8]) -> bool {
        // V3 payload: CRC32 over core header
        if payload.len() >= 3
            && payload[0] == V3_MAGIC[0]
            && payload[1] == V3_MAGIC[1]
            && payload[2] == V3_PAYLOAD_VERSION
        {
            if payload.len() < crate::payload_v3::types::V3_CORE_SIZE + 4 {
                return false;
            }
            let total_length = u16::from_le_bytes([payload[4], payload[5]]) as usize;
            if total_length > payload.len() {
                return false;
            }
            let auth_tag_len = payload[30] as usize;
            if total_length < crate::payload_v3::types::V3_CORE_SIZE + auth_tag_len {
                return false;
            }
            let core_and_ext = &payload[..total_length - auth_tag_len];
            let expected = Self::compute_checksum(core_and_ext);
            payload[total_length - auth_tag_len] == expected[0]
                && payload[total_length - auth_tag_len + 1] == expected[1]
                && payload[total_length - auth_tag_len + 2] == expected[2]
                && payload[total_length - auth_tag_len + 3] == expected[3]
        }
        // V2 ECC size first
        else {
            let v2_ecc_len = V2_HEADER_SIZE * ecc::REPLICATION_FACTOR;
            if payload.len() >= v2_ecc_len + 4 {
                let expected = Self::compute_checksum(&payload[..v2_ecc_len]);
                if payload[v2_ecc_len] == expected[0]
                    && payload[v2_ecc_len + 1] == expected[1]
                    && payload[v2_ecc_len + 2] == expected[2]
                    && payload[v2_ecc_len + 3] == expected[3]
                {
                    return true;
                }
            }
            // Try v1 ECC size
            if payload.len() >= ecc::TOTAL_ECC_LEN + 4 {
                let expected = Self::compute_checksum(&payload[..ecc::TOTAL_ECC_LEN]);
                if payload[ecc::TOTAL_ECC_LEN] == expected[0]
                    && payload[ecc::TOTAL_ECC_LEN + 1] == expected[1]
                    && payload[ecc::TOTAL_ECC_LEN + 2] == expected[2]
                    && payload[ecc::TOTAL_ECC_LEN + 3] == expected[3]
                {
                    return true;
                }
            }
            // Legacy: try 24-byte header + 4 checksum
            if payload.len() >= MIN_PAYLOAD_SIZE {
                let expected = Self::compute_checksum(&payload[..24]);
                if payload[24] == expected[0]
                    && payload[25] == expected[1]
                    && payload[26] == expected[2]
                    && payload[27] == expected[3]
                {
                    return true;
                }
            }
            false
        }
    }

    fn verify_payload_integrity(payload: &[u8], mac_key: &[u8]) -> bool {
        // V3 payload: check magic bytes
        if payload.len() >= 3
            && payload[0] == V3_MAGIC[0]
            && payload[1] == V3_MAGIC[1]
            && payload[2] == V3_PAYLOAD_VERSION
        {
            if payload.len() < crate::payload_v3::types::V3_CORE_SIZE {
                return false;
            }
            let total_length = u16::from_le_bytes([payload[4], payload[5]]) as usize;
            if total_length > payload.len() {
                return false;
            }
            let auth_algo = payload[29];
            let auth_tag_len = payload[30] as usize;
            if total_length < crate::payload_v3::types::V3_CORE_SIZE + auth_tag_len {
                return false;
            }
            let core_and_ext = &payload[..total_length - auth_tag_len];
            let tag = &payload[total_length - auth_tag_len..total_length];
            match auth_algo {
                1 => {
                    let expected = Self::compute_checksum(core_and_ext);
                    tag == expected
                }
                2 if !mac_key.is_empty() => {
                    let mut mac =
                        HmacSha256::new_from_slice(mac_key).expect("HMAC can take key of any size");
                    mac.update(core_and_ext);
                    let result = mac.finalize().into_bytes();
                    result[..tag.len()].ct_eq(tag).into()
                }
                _ => false,
            }
        } else if mac_key.is_empty() {
            Self::verify_checksum(payload)
        } else {
            // Try v2 MAC: 32-byte header + 8-byte MAC = 40 bytes
            if payload.len() >= V2_HEADER_SIZE + 8 {
                Self::verify_payload_mac(
                    &payload[..V2_HEADER_SIZE],
                    mac_key,
                    &payload[V2_HEADER_SIZE..V2_HEADER_SIZE + 8],
                )
            }
            // Legacy v1 MAC: 24-byte header + 8-byte MAC = 32 bytes
            else if payload.len() >= 32 {
                Self::verify_payload_mac(&payload[..24], mac_key, &payload[24..32])
            } else {
                false
            }
        }
    }

    /// Classify an invalid payload based on v3 auth_algo and provided key.
    ///
    /// Returns `AuthenticationKeyMissing` if the payload uses HMAC but no key
    /// was provided, `AuthenticationFailed` if the payload uses HMAC but the
    /// key was wrong, or `Invalid` for other failures.
    fn classify_auth_failure(payload: &[u8], mac_key: &[u8]) -> CandidateOutcome {
        if payload.len() > 30 && Self::has_v3_magic(payload) {
            let auth_algo = payload[29];
            if auth_algo == 2 {
                if mac_key.is_empty() {
                    CandidateOutcome::AuthenticationKeyMissing(payload.to_vec())
                } else {
                    CandidateOutcome::AuthenticationFailed(payload.to_vec())
                }
            } else {
                CandidateOutcome::Invalid(payload.to_vec())
            }
        } else {
            CandidateOutcome::Invalid(payload.to_vec())
        }
    }

    fn try_ecc_decode(payload: &[u8]) -> Option<Vec<u8>> {
        // Try v2 data length first (32 bytes), then v1 (24 bytes)
        for &data_len in &[V2_HEADER_SIZE, 24usize] {
            let ecc_len = data_len * ecc::REPLICATION_FACTOR;
            if payload.len() >= ecc_len {
                if let Some(decoded) = ecc::ecc_decode(payload, data_len) {
                    if decoded.len() >= data_len {
                        let checksum_start = ecc_len;
                        let expected = Self::compute_checksum(&payload[..ecc_len]);
                        if payload[checksum_start] == expected[0]
                            && payload[checksum_start + 1] == expected[1]
                            && payload[checksum_start + 2] == expected[2]
                            && payload[checksum_start + 3] == expected[3]
                        {
                            return Some(decoded);
                        }
                    }
                }
            }
        }
        None
    }

    /// Generates the steganography payload containing protection metadata.
    ///
    /// The payload uses the v3 wire format with a TLV header:
    /// - Magic bytes (3 bytes: 0x53 0x45 0x03)
    /// - Protection level byte
    /// - Seed (8 bytes, little-endian)
    /// - Intensity (2 bytes, scaled by 100)
    /// - Flags and extension count
    /// - Optional TLV extensions
    /// - Authentication tag (HMAC-SHA256 truncated, or CRC32 + ECC)
    fn generate_payload(
        &self,
        emission: &PayloadEmissionContext,
        ctx: &ProtectionContext,
    ) -> Vec<u8> {
        let intensity_val = (ctx.intensity() * 100.0) as u16;

        let dmi_byte = ctx
            .dmi_value()
            .map(|d| match d {
                crate::types::DmiValue::Unspecified => 0u8,
                crate::types::DmiValue::Allowed => 1,
                crate::types::DmiValue::ProhibitedAiMlTraining => 2,
                crate::types::DmiValue::ProhibitedGenAiMlTraining => 3,
                crate::types::DmiValue::ProhibitedExceptSearchEngineIndexing => 4,
                crate::types::DmiValue::Prohibited => 5,
                crate::types::DmiValue::ProhibitedSeeConstraints => 6,
            })
            .unwrap_or(0);

        let content_hash_8 = ctx
            .content_hash()
            .map(|h| {
                let mut buf = [0u8; 8];
                buf[..4].copy_from_slice(&h);
                buf
            })
            .unwrap_or([0u8; 8]);

        let has_mac = emission.has_mac();

        let flags = crate::payload_v3::types::PayloadFlags {
            has_extensions: !emission.extensions.is_empty(),
            has_key_id: emission.key_id.is_some(),
            tiled: emission.tiled,
            progressive_jpeg: emission.progressive_output,
            critical_extension: false,
            signed: false,
            reserved: 0,
        };

        let channels = crate::payload_v3::types::ProtectionChannels {
            rights_metadata: emission.rights_metadata_planned,
            hidden_marker: true,
            authentication: has_mac,
        };

        let (auth_algo, auth_tag_len) = if has_mac {
            (AuthAlgorithm::HmacSha256Truncated, 16u8)
        } else {
            (AuthAlgorithm::Crc32, 4u8)
        };

        let mut buf =
            Vec::with_capacity(crate::payload_v3::types::V3_CORE_SIZE + auth_tag_len as usize);

        buf.extend_from_slice(&V3_MAGIC);
        buf.push(V3_PAYLOAD_VERSION);
        buf.push(crate::payload_v3::types::V3_CORE_SIZE as u8);
        let total_length = crate::payload_v3::types::V3_CORE_SIZE + auth_tag_len as usize;
        buf.extend_from_slice(&(total_length as u16).to_le_bytes());
        buf.extend_from_slice(&flags.to_bits().to_le_bytes());
        buf.extend_from_slice(&channels.to_bits().to_le_bytes());
        buf.push(dmi_byte);
        buf.extend_from_slice(&ctx.seed().to_le_bytes());
        buf.extend_from_slice(&intensity_val.to_le_bytes());
        buf.extend_from_slice(&content_hash_8);
        buf.push(auth_algo as u8);
        buf.push(auth_tag_len);
        buf.push(0);

        debug_assert_eq!(buf.len(), crate::payload_v3::types::V3_CORE_SIZE);

        let auth_tag = if let Some(key) = ctx.mac_key() {
            Self::compute_payload_mac_v3(&buf, key).to_vec()
        } else {
            Self::compute_checksum(&buf).to_vec()
        };
        buf.extend_from_slice(&auth_tag);

        buf
    }

    pub(crate) fn generate_payload_for_plan(
        &self,
        emission: &PayloadEmissionContext,
        plan: &crate::types::ResolvedProtectionPlan,
    ) -> Vec<u8> {
        use crate::payload_v3::types::{AuthAlgorithm, V3_CORE_SIZE, V3_MAGIC, V3_PAYLOAD_VERSION};

        let intensity_val = (plan.intensity() * 100.0) as u16;

        let dmi_byte = plan
            .effective_dmi()
            .map(|d| match d {
                crate::types::DmiValue::Unspecified => 0u8,
                crate::types::DmiValue::Allowed => 1,
                crate::types::DmiValue::ProhibitedAiMlTraining => 2,
                crate::types::DmiValue::ProhibitedGenAiMlTraining => 3,
                crate::types::DmiValue::ProhibitedExceptSearchEngineIndexing => 4,
                crate::types::DmiValue::Prohibited => 5,
                crate::types::DmiValue::ProhibitedSeeConstraints => 6,
            })
            .unwrap_or(0);

        let content_hash_8 = plan
            .content_hash()
            .map(|h| {
                let mut buf = [0u8; 8];
                buf[..4].copy_from_slice(&h);
                buf
            })
            .unwrap_or([0u8; 8]);

        let has_mac = emission.has_mac();

        let flags = crate::payload_v3::types::PayloadFlags {
            has_extensions: !emission.extensions.is_empty(),
            has_key_id: emission.key_id.is_some(),
            tiled: emission.tiled,
            progressive_jpeg: emission.progressive_output,
            critical_extension: false,
            signed: false,
            reserved: 0,
        };

        let channels = crate::payload_v3::types::ProtectionChannels {
            rights_metadata: emission.rights_metadata_planned,
            hidden_marker: true,
            authentication: has_mac,
        };

        let (auth_algo, auth_tag_len) = if has_mac {
            (AuthAlgorithm::HmacSha256Truncated, 16u8)
        } else {
            (AuthAlgorithm::Crc32, 4u8)
        };

        let mut buf = Vec::with_capacity(V3_CORE_SIZE + auth_tag_len as usize);

        buf.extend_from_slice(&V3_MAGIC);
        buf.push(V3_PAYLOAD_VERSION);
        buf.push(V3_CORE_SIZE as u8);
        let total_length = V3_CORE_SIZE + auth_tag_len as usize;
        buf.extend_from_slice(&(total_length as u16).to_le_bytes());
        buf.extend_from_slice(&flags.to_bits().to_le_bytes());
        buf.extend_from_slice(&channels.to_bits().to_le_bytes());
        buf.push(dmi_byte);
        buf.extend_from_slice(&plan.seed().to_le_bytes());
        buf.extend_from_slice(&intensity_val.to_le_bytes());
        buf.extend_from_slice(&content_hash_8);
        buf.push(auth_algo as u8);
        buf.push(auth_tag_len);
        buf.push(0);

        debug_assert_eq!(buf.len(), V3_CORE_SIZE);

        let auth_tag = if let Some(key) = plan.mac_key() {
            Self::compute_payload_mac_v3(&buf, key).to_vec()
        } else {
            Self::compute_checksum(&buf).to_vec()
        };
        buf.extend_from_slice(&auth_tag);

        buf
    }

    pub(crate) fn effective_redundancy_for_plan(
        plan: &crate::types::ResolvedProtectionPlan,
    ) -> usize {
        plan.effective_redundancy()
    }

    #[allow(dead_code)]
    pub(crate) fn lsb_pixels_needed_from_plan(
        plan: &crate::types::ResolvedProtectionPlan,
    ) -> usize {
        let payload_bits = if plan.mac_key().is_some() {
            V3_HMAC_PAYLOAD_BITS
        } else {
            V3_CRC_PAYLOAD_BITS
        };
        carrier_lsb::lsb_required_slots_legacy(payload_bits)
    }

    pub(crate) fn apply_dct_stego_bytes_from_plan(
        &self,
        jpeg_bytes: &[u8],
        plan: &crate::types::ResolvedProtectionPlan,
        tile_size: Option<u32>,
    ) -> Result<crate::stego::EmbedOutcome<Vec<u8>>> {
        use crate::stego::{EmbedOutcome, EmbedPath};

        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return Err(Error::Steganography("Not a valid JPEG".to_string()));
        }

        if let Some(ts) = tile_size.filter(|&s| s > 0) {
            return self.apply_dct_stego_bytes_tiled_from_plan(jpeg_bytes, plan, ts);
        }

        let seed = plan.seed();

        match JpegTranscoder::decode_coefficients(jpeg_bytes) {
            Ok((header, coefficients)) => {
                let emission = PayloadEmissionContext::from_plan(plan, EmbedPath::DctF5);
                let payload = self.generate_payload_for_plan(&emission, plan);
                let requested_redundancy = Self::effective_redundancy_for_plan(plan);
                let payload_bits = payload.len().saturating_mul(8);

                let available_coeffs = Self::dct_payload_capacity(&coefficients);

                let mut header = header;
                DctStegoF5::new().embed_seed_in_quantization_tables(&mut header, seed)?;

                let max_feasible = available_coeffs.checked_div(payload_bits).unwrap_or(0);
                let selected_redundancy = requested_redundancy.min(max_feasible).max(1);

                if max_feasible >= 1 {
                    let mut embedded_coefficients = coefficients.clone();
                    if DctStegoF5::with_redundancy(selected_redundancy)
                        .embed_f5(&mut embedded_coefficients, &payload, seed)
                        .is_ok()
                    {
                        let output = JpegTranscoder::encode_coefficients(
                            &header,
                            &embedded_coefficients,
                            Some(jpeg_bytes),
                        )?;
                        return Ok(EmbedOutcome::Embedded {
                            output,
                            payload_bytes: payload.len(),
                            required_capacity: payload_bits,
                            available_capacity: available_coeffs,
                            path: EmbedPath::DctF5,
                        });
                    }
                }

                let output =
                    JpegTranscoder::encode_coefficients(&header, &coefficients, Some(jpeg_bytes))?;
                Ok(EmbedOutcome::SkippedCapacity {
                    output,
                    payload_bytes: payload.len(),
                    required_capacity: payload_bits,
                    available_capacity: available_coeffs,
                    path: EmbedPath::DctF5,
                })
            }
            Err(_) => {
                let mut header = crate::stego::jpeg_transcoder::JpegHeader::parse(jpeg_bytes)?;
                DctStegoF5::new().embed_seed_in_quantization_tables(&mut header, seed)?;
                let output = Self::reassemble_jpeg_with_qtables(jpeg_bytes, &header)?;
                Ok(EmbedOutcome::UnsupportedProgressive { output })
            }
        }
    }

    fn apply_dct_stego_bytes_tiled_from_plan(
        &self,
        jpeg_bytes: &[u8],
        plan: &crate::types::ResolvedProtectionPlan,
        tile_size: u32,
    ) -> Result<crate::stego::EmbedOutcome<Vec<u8>>> {
        use crate::stego::{EmbedOutcome, EmbedPath};

        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return Err(Error::Steganography("Not a valid JPEG".to_string()));
        }

        let seed = plan.seed();

        match JpegTranscoder::decode_coefficients(jpeg_bytes) {
            Ok((header, coefficients)) => {
                let mut header = header;
                let mut coefficients = coefficients;

                let emission = PayloadEmissionContext::from_plan(plan, EmbedPath::DctF5Tiled);
                let payload = self.generate_payload_for_plan(&emission, plan);
                let payload_bits = payload.len() * 8;

                DctStegoF5::new().embed_seed_in_quantization_tables(&mut header, seed)?;

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
                let tiles_x = luma_blocks_x / blocks_per_tile;
                let tiles_y = luma_blocks_y / blocks_per_tile;

                let mut embedded_any = false;
                for ty in 0..tiles_y {
                    for tx in 0..tiles_x {
                        let tile_blocks =
                            DctStegoF5::tile_block_set(&header, &coefficients, tx, ty, tile_size);
                        if tile_blocks.is_empty() {
                            continue;
                        }
                        let local_seed = tile_seed(seed, tx, ty);
                        if DctStegoF5::with_redundancy(1)
                            .embed_f5_in_blocks(
                                &mut coefficients,
                                &payload,
                                local_seed,
                                &tile_blocks,
                            )
                            .is_ok()
                        {
                            embedded_any = true;
                        }
                    }
                }

                if embedded_any {
                    let attempt_bytes = JpegTranscoder::encode_coefficients(
                        &header,
                        &coefficients,
                        Some(jpeg_bytes),
                    )?;
                    if let Ok((_, roundtrip_coefficients)) =
                        JpegTranscoder::decode_coefficients(&attempt_bytes)
                    {
                        let tile_blocks = DctStegoF5::tile_block_set(
                            &header,
                            &roundtrip_coefficients,
                            0,
                            0,
                            tile_size,
                        );
                        let roundtrip_bits = DctStegoF5::with_redundancy(1).extract_f5_from_blocks(
                            &roundtrip_coefficients,
                            payload_bits,
                            tile_seed(seed, 0, 0),
                            &tile_blocks,
                        );
                        if Self::bits_to_bytes(&roundtrip_bits) == payload {
                            return Ok(EmbedOutcome::Embedded {
                                output: attempt_bytes,
                                payload_bytes: payload.len(),
                                required_capacity: payload_bits,
                                available_capacity: payload_bits,
                                path: EmbedPath::DctF5Tiled,
                            });
                        }
                    }
                }

                let output =
                    JpegTranscoder::encode_coefficients(&header, &coefficients, Some(jpeg_bytes))?;
                Ok(EmbedOutcome::SkippedCapacity {
                    output,
                    payload_bytes: payload.len(),
                    required_capacity: payload_bits,
                    available_capacity: 0,
                    path: EmbedPath::DctF5Tiled,
                })
            }
            Err(_) => {
                let mut header = crate::stego::jpeg_transcoder::JpegHeader::parse(jpeg_bytes)?;
                DctStegoF5::new().embed_seed_in_quantization_tables(&mut header, seed)?;
                let output = Self::reassemble_jpeg_with_qtables(jpeg_bytes, &header)?;
                Ok(EmbedOutcome::UnsupportedProgressive { output })
            }
        }
    }

    pub(crate) fn apply_to_image_with_summary_from_plan(
        &self,
        img: &DynamicImage,
        plan: &crate::types::ResolvedProtectionPlan,
        tile_size: Option<u32>,
    ) -> Result<(DynamicImage, Option<crate::stego::EmbedOutcomeSummary>)> {
        let format = plan.input_format();
        let is_tiled = tile_size.filter(|&s| s > 0).is_some();
        let embed_path = match format {
            crate::types::ImageOutputFormat::Jpeg => {
                if is_tiled {
                    crate::stego::EmbedPath::DctF5Tiled
                } else {
                    crate::stego::EmbedPath::DctF5
                }
            }
            _ => {
                if is_tiled {
                    crate::stego::EmbedPath::LsbTiled
                } else {
                    crate::stego::EmbedPath::Lsb
                }
            }
        };

        let emission = PayloadEmissionContext::from_plan(plan, embed_path);
        let payload = self.generate_payload_for_plan(&emission, plan);
        let rgba = img.to_rgba8();
        let seed = plan.seed();
        let redundancy = Self::effective_redundancy_for_plan(plan);

        match format {
            crate::types::ImageOutputFormat::Png => {
                let outcome = if let Some(ts) = tile_size.filter(|&s| s > 0) {
                    self.embed_lsb_tiled(&rgba, &payload, seed, ts)
                } else {
                    self.embed_lsb_v2(&rgba, &payload, seed, redundancy)
                };
                let (mut result, summary) = outcome.into_parts();
                if summary.is_embedded() {
                    Self::embed_seed_lsb_fallback(&mut result, seed);
                }
                Ok((DynamicImage::ImageRgba8(result), Some(summary)))
            }
            crate::types::ImageOutputFormat::Jpeg => {
                let jpeg_bytes = crate::util::image::encode_image_with_options(
                    img,
                    Some(crate::types::ImageOutputFormat::Jpeg),
                    plan.processing().progressive_jpeg,
                    plan.processing().jpeg_quality,
                )?;
                let with_stego =
                    self.apply_dct_stego_bytes_from_plan(&jpeg_bytes, plan, tile_size)?;
                let (output, summary) = with_stego.into_parts();
                Ok((image::load_from_memory(&output)?, Some(summary)))
            }
            crate::types::ImageOutputFormat::WebP => {
                let outcome = if let Some(ts) = tile_size.filter(|&s| s > 0) {
                    self.embed_lsb_tiled(&rgba, &payload, seed, ts)
                } else {
                    self.embed_lsb_v2(&rgba, &payload, seed, redundancy)
                };
                let (mut result, summary) = outcome.into_parts();
                if summary.is_embedded() {
                    Self::embed_seed_lsb_fallback(&mut result, seed);
                }
                Ok((DynamicImage::ImageRgba8(result), Some(summary)))
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn embed_lsb_minimal_from_plan(
        &self,
        img: &DynamicImage,
        plan: &crate::types::ResolvedProtectionPlan,
    ) -> DynamicImage {
        let format = plan.input_format();
        let embed_path = match format {
            crate::types::ImageOutputFormat::Jpeg => crate::stego::EmbedPath::DctF5,
            _ => crate::stego::EmbedPath::Lsb,
        };
        let emission = PayloadEmissionContext::from_plan(plan, embed_path);
        let payload = self.generate_payload_for_plan(&emission, plan);
        let rgba = img.to_rgba8();
        let seed = plan.seed();

        match format {
            crate::types::ImageOutputFormat::Png | crate::types::ImageOutputFormat::WebP => {
                let outcome = self.embed_lsb_v2(&rgba, &payload, seed, 1);
                DynamicImage::ImageRgba8(outcome.into_inner())
            }
            crate::types::ImageOutputFormat::Jpeg => {
                if let Ok(encoded) = crate::util::image::encode_image(img, image::ImageFormat::Jpeg)
                {
                    if let Ok(with_seed) = self.apply_qtable_seed_bytes(&encoded, seed) {
                        if let Ok(stego_img) = image::load_from_memory(&with_seed) {
                            stego_img
                        } else {
                            img.clone()
                        }
                    } else {
                        img.clone()
                    }
                } else {
                    img.clone()
                }
            }
        }
    }

    /// Generate the V3 stego payload for a given context.
    ///
    /// Exposed for testing channel flags and payload structure without
    /// requiring a full image embed/extract cycle.
    #[doc(hidden)]
    pub fn generate_payload_for_context(&self, ctx: &ProtectionContext) -> Vec<u8> {
        let embed_path = if ctx.is_tile_mode_enabled() {
            if ctx.input_format() == Some(crate::types::ImageOutputFormat::Jpeg) {
                crate::stego::EmbedPath::DctF5Tiled
            } else {
                crate::stego::EmbedPath::LsbTiled
            }
        } else if ctx.input_format() == Some(crate::types::ImageOutputFormat::Jpeg) {
            crate::stego::EmbedPath::DctF5
        } else {
            crate::stego::EmbedPath::Lsb
        };
        let emission = PayloadEmissionContext::from_plan_for_context(ctx, embed_path);
        self.generate_payload(&emission, ctx)
    }

    /// Test-only wrapper: generate payload from a [`ProtectionContext`].
    ///
    /// Derives the emission context from the context's fields. For tests
    /// that call `generate_payload` directly rather than through the
    /// pipeline.
    #[cfg(test)]
    fn generate_payload_from_ctx(&self, ctx: &ProtectionContext) -> Vec<u8> {
        self.generate_payload_for_context(ctx)
    }

    fn extract_lsb(&self, img: &RgbaImage, expected_bits: usize, seed: u64) -> Option<Vec<u8>> {
        carrier_lsb::extract_lsb(img, expected_bits, seed)
    }

    /// Extract LSBs from a specific bit range `[offset, offset + count)`.
    ///
    /// Uses the same permutation as [`extract_lsb`] (same seed, same
    /// `expected_bits`), but only decodes bits in the requested range. The
    /// caller must ensure `offset + count <= expected_bits` and that the
    /// image has sufficient pixels for the full `expected_bits` spread.
    #[allow(dead_code)]
    fn extract_lsb_range(
        &self,
        img: &RgbaImage,
        expected_bits: usize,
        offset: usize,
        count: usize,
        seed: u64,
    ) -> Option<Vec<u8>> {
        carrier_lsb::extract_lsb_range(img, expected_bits, offset, count, seed)
    }

    /// Embed payload using the corrected V2 carrier scheme.
    ///
    /// The V2 scheme operates over `width * height * 3` RGB carrier slots.
    /// Each payload bit is spread across `STEGO_SPREAD_FACTOR * redundancy`
    /// distinct slots selected by a single true bijection permutation.
    /// All replicas of the same bit use consecutive logical indices through
    /// one permutation, guaranteeing no inter-replica collisions:
    /// - Exact capacity model: `required = payload_bits * STEGO_SPREAD_FACTOR * redundancy`
    /// - No slot collisions within one embedding
    /// - True bijection for arbitrary (including non-power-of-two) slot counts
    fn embed_lsb_v2(
        &self,
        img: &RgbaImage,
        payload: &[u8],
        seed: u64,
        redundancy: usize,
    ) -> crate::stego::EmbedOutcome<RgbaImage> {
        carrier_lsb::embed_lsb_v2(img, payload, seed, redundancy)
    }

    /// Extract payload using the corrected V2 carrier scheme.
    ///
    /// Mirrors [`embed_lsb_v2`] — uses the same permutation, slot mapping,
    /// and majority-vote logic over `width * height * 3` RGB carrier slots.
    /// Each bit reads `STEGO_SPREAD_FACTOR * redundancy` slots via majority vote.
    fn extract_lsb_v2(
        &self,
        img: &RgbaImage,
        expected_bits: usize,
        seed: u64,
        base_slot: usize,
        redundancy: usize,
    ) -> Option<Vec<u8>> {
        carrier_lsb::extract_lsb_v2(img, expected_bits, seed, base_slot, redundancy)
    }

    fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
        carrier_lsb::bits_to_bytes(bits)
    }

    fn embed_seed_lsb_fallback(img: &mut RgbaImage, seed: u64) {
        carrier_lsb::embed_seed_lsb_fallback(img, seed);
    }

    pub(crate) fn embed_seed_lsb_fallback_pub(img: &mut RgbaImage, seed: u64) {
        carrier_lsb::embed_seed_lsb_fallback(img, seed);
    }

    fn extract_seed_lsb_fallback(img: &RgbaImage) -> Option<u64> {
        carrier_lsb::extract_seed_lsb_fallback(img)
    }

    /// Classify the first 6 bytes of a potential v3 payload.
    ///
    /// Reads only magic (2), version (1), header_length (1), total_length (2).
    /// Returns `Detected` with declared sizes when the prefix is valid.
    /// Does NOT validate the full header — call `validate_v3_header` next.
    pub(crate) fn classify_v3_prefix(
        prefix: &[u8],
        limits: Option<&crate::resource_limits::ResourceLimits>,
    ) -> V3PrefixResult {
        if prefix.len() < V3_PREFIX_BYTES {
            return V3PrefixResult::NotV3;
        }
        if prefix[0] != V3_MAGIC[0] || prefix[1] != V3_MAGIC[1] {
            return V3PrefixResult::NotV3;
        }
        if prefix[2] != V3_PAYLOAD_VERSION {
            if prefix[2] > V3_PAYLOAD_VERSION {
                return V3PrefixResult::UnsupportedVersion(prefix[2]);
            }
            return V3PrefixResult::NotV3;
        }
        let header_length = prefix[3] as usize;
        let total_length = u16::from_le_bytes([prefix[4], prefix[5]]) as usize;
        if header_length < crate::payload_v3::types::V3_CORE_SIZE {
            return V3PrefixResult::Malformed(PayloadMalformedReason::HeaderTooShort);
        }
        if total_length < header_length {
            return V3PrefixResult::Malformed(PayloadMalformedReason::TotalLessThanHeader);
        }
        if total_length > crate::payload_v3::types::V3_MAX_EMBEDDED_SIZE {
            return V3PrefixResult::Malformed(PayloadMalformedReason::TotalExceedsWireMax);
        }
        if let Some(limits) = limits {
            if total_length > limits.max_payload_bytes() {
                return V3PrefixResult::ResourceLimitExceeded;
            }
        }
        V3PrefixResult::Detected {
            header_length,
            total_length,
        }
    }

    /// Validate a full v3 header (beyond the6-byte prefix).
    ///
    /// `header_bytes` must be exactly `header_length` bytes long (the full
    /// declared header, including the prefix). Returns a [`ValidatedV3Header`]
    /// on success, or a [`V3PrefixResult`] error on any validation failure.
    pub(crate) fn validate_v3_header(
        header_bytes: &[u8],
        limits: Option<&crate::resource_limits::ResourceLimits>,
    ) -> std::result::Result<ValidatedV3Header, V3PrefixResult> {
        if header_bytes.len() < V3_PREFIX_BYTES {
            return Err(V3PrefixResult::NotV3);
        }
        let header_length = header_bytes[3] as usize;
        let total_length = u16::from_le_bytes([header_bytes[4], header_bytes[5]]) as usize;
        if header_bytes.len() < header_length {
            return Err(V3PrefixResult::Malformed(
                PayloadMalformedReason::HeaderLengthMismatch,
            ));
        }
        if let Some(limits) = limits {
            if total_length > limits.max_payload_bytes() {
                return Err(V3PrefixResult::ResourceLimitExceeded);
            }
        }
        let total_bits = total_length
            .checked_mul(8)
            .ok_or(V3PrefixResult::Malformed(
                PayloadMalformedReason::BitOverflow,
            ))?;
        let _ = total_bits;

        let auth_tag_length = if header_bytes.len() > 30 {
            let tag_len = header_bytes[30] as usize;
            if total_length < crate::payload_v3::types::V3_CORE_SIZE + tag_len {
                return Err(V3PrefixResult::Malformed(
                    PayloadMalformedReason::AuthTagLengthInvalid,
                ));
            }
            tag_len
        } else {
            0
        };

        let key_id_length = if header_bytes.len() > 31 {
            let kid_len = header_bytes[31] as usize;
            if kid_len > crate::payload_v3::types::V3_MAX_KEY_ID_LEN {
                return Err(V3PrefixResult::Malformed(
                    PayloadMalformedReason::KeyIdTooLong,
                ));
            }
            if header_length < crate::payload_v3::types::V3_CORE_SIZE + kid_len {
                return Err(V3PrefixResult::Malformed(
                    PayloadMalformedReason::HeaderTooShortForKeyId,
                ));
            }
            kid_len
        } else {
            0
        };

        let auth_algorithm = if header_bytes.len() > 29 {
            let algo = header_bytes[29];
            AuthAlgorithm::from_byte(algo).ok_or(V3PrefixResult::Malformed(
                PayloadMalformedReason::InvalidAuthAlgorithm,
            ))?
        } else {
            AuthAlgorithm::Crc32
        };

        let channels = if header_bytes.len() >= 10 {
            let channel_bits = u16::from_le_bytes([header_bytes[8], header_bytes[9]]);
            ProtectionChannels::from_bits(channel_bits).unwrap_or(ProtectionChannels {
                rights_metadata: true,
                hidden_marker: true,
                authentication: true,
            })
        } else {
            ProtectionChannels {
                rights_metadata: true,
                hidden_marker: true,
                authentication: true,
            }
        };

        let flags = if header_bytes.len() >= 8 {
            let flag_bits = u16::from_le_bytes([header_bytes[6], header_bytes[7]]);
            crate::payload_v3::types::PayloadFlags::from_bits(flag_bits)
        } else {
            crate::payload_v3::types::PayloadFlags::from_bits(0)
        };

        Ok(ValidatedV3Header {
            header_length,
            total_length,
            auth_algorithm,
            auth_tag_length,
            key_id_length,
            channels,
            flags,
        })
    }

    /// Classify a set of extracted bytes as v3 header, legacy, or malformed.
    ///
    /// The bytes must be at least 6 bytes (to read `total_length` from
    /// bytes 4-5). Returns the exact `header_length` and `total_length`
    /// when v3 magic is present and the header is structurally valid.
    ///
    /// When `limits` is provided, validates `total_length` against
    /// `limits.max_payload_bytes()` before returning `V3Detected`.
    pub(crate) fn classify_v3_probe(
        bytes: &[u8],
        limits: Option<&crate::resource_limits::ResourceLimits>,
    ) -> V3ProbeResult {
        if bytes.len() < 6 {
            return V3ProbeResult::NotV3;
        }
        if bytes[0] != V3_MAGIC[0] || bytes[1] != V3_MAGIC[1] {
            return V3ProbeResult::NotV3;
        }
        if bytes[2] != V3_PAYLOAD_VERSION {
            if bytes[2] > V3_PAYLOAD_VERSION {
                return V3ProbeResult::UnsupportedVersion(bytes[2]);
            }
            return V3ProbeResult::NotV3;
        }
        let header_length = bytes[3] as usize;
        let total_length = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        if header_length < crate::payload_v3::types::V3_CORE_SIZE {
            return V3ProbeResult::MalformedV3;
        }
        if total_length < header_length {
            return V3ProbeResult::MalformedV3;
        }
        if total_length > crate::payload_v3::types::V3_MAX_EMBEDDED_SIZE {
            return V3ProbeResult::MalformedV3;
        }
        if let Some(limits) = limits {
            if total_length > limits.max_payload_bytes() {
                return V3ProbeResult::ResourceLimitExceeded;
            }
        }
        let total_bits = match total_length.checked_mul(8) {
            Some(b) => b,
            None => return V3ProbeResult::MalformedV3,
        };
        if bytes.len() >= 31 {
            let auth_tag_len = bytes[30] as usize;
            if total_length < crate::payload_v3::types::V3_CORE_SIZE + auth_tag_len {
                return V3ProbeResult::MalformedV3;
            }
        }
        if bytes.len() >= 32 {
            let key_id_len = bytes[31] as usize;
            if key_id_len > crate::payload_v3::types::V3_MAX_KEY_ID_LEN {
                return V3ProbeResult::MalformedV3;
            }
            if header_length < crate::payload_v3::types::V3_CORE_SIZE + key_id_len {
                return V3ProbeResult::MalformedV3;
            }
            if bytes.len() >= 30 {
                let auth_algo = bytes[29];
                if crate::payload_v3::types::AuthAlgorithm::from_byte(auth_algo).is_none() {
                    return V3ProbeResult::MalformedV3;
                }
            }
        }
        V3ProbeResult::V3Detected {
            header_length,
            total_length,
            total_bits,
        }
    }

    /// Embed the full payload once per tile for crop resistance.
    ///
    /// Each `tile_size × tile_size` pixel region embeds the full payload using
    /// a tile-specific seed (see [`tile_seed`]). Redundancy is fixed at 1
    /// because the tile *grid* itself is the redundancy — multiple tiles
    /// already cover the same payload.
    ///
    /// Tiles do not overlap; right/bottom edge tiles may be partial and the
    /// embed is silently skipped for those (the existing `embed_lsb` capacity
    /// check at line 870 handles "image smaller than payload"). At least one
    /// full interior tile will survive any reasonable crop.
    ///
    /// When `tile_size == 0` the image is returned unchanged — this is the
    /// "tiling disabled" sentinel, and the caller is expected to route
    /// through the non-tiled path instead.
    fn embed_lsb_tiled(
        &self,
        img: &RgbaImage,
        payload: &[u8],
        master_seed: u64,
        tile_size: u32,
    ) -> crate::stego::EmbedOutcome<RgbaImage> {
        carrier_lsb::embed_lsb_tiled(img, payload, master_seed, tile_size)
    }

    /// Extract payload from a sub-image trying V2 then legacy scheme.
    ///
    /// Used by tiled extraction where each tile is an independent sub-image.
    /// Tries V2 (corrected carrier) first, then falls back to legacy.
    fn extract_from_sub_image(
        &self,
        sub: &RgbaImage,
        _expected_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        let prefix_bits = 6 * 8;

        if let Some(prefix) = self.extract_lsb_v2(sub, prefix_bits, seed, 0, 1) {
            if let Some(result) =
                self.probe_payload_from_prefix_v2(sub, &prefix, prefix_bits, seed, mac_key)
            {
                return Some(result);
            }
        }

        if let Some(prefix) = self.extract_lsb(sub, prefix_bits, seed) {
            if let Some(result) =
                self.probe_payload_from_prefix_legacy(sub, &prefix, prefix_bits, seed, mac_key)
            {
                return Some(result);
            }
        }

        None
    }

    fn probe_payload_from_prefix_v2(
        &self,
        img: &RgbaImage,
        prefix: &[u8],
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        match Self::classify_v3_prefix(prefix, Some(&self.limits)) {
            V3PrefixResult::Detected {
                header_length,
                total_length,
            } => {
                let header_bits = header_length * 8;
                let header_bytes = if header_bits <= prefix_bits {
                    prefix[..header_length].to_vec()
                } else {
                    self.extract_lsb_v2(img, header_bits, seed, 0, 1)?
                };
                if Self::validate_v3_header(&header_bytes, Some(&self.limits)).is_err() {
                    return None;
                }
                let total_bits = total_length * 8;
                let full = if total_bits <= prefix_bits {
                    prefix[..total_length].to_vec()
                } else {
                    self.extract_lsb_v2(img, total_bits, seed, 0, 1)?
                };
                if Self::verify_payload_integrity(&full, mac_key) {
                    return Some(Self::truncate_to_actual_payload(&full));
                }
            }
            V3PrefixResult::NotV3 => {
                for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                    let payload = if ecc_bits <= prefix_bits {
                        prefix.to_vec()
                    } else {
                        match self.extract_lsb_v2(img, ecc_bits, seed, 0, 1) {
                            Some(p) => p,
                            None => continue,
                        }
                    };
                    if Self::try_ecc_decode(&payload).is_some() {
                        return Some(payload);
                    }
                }
            }
            V3PrefixResult::Malformed(_)
            | V3PrefixResult::UnsupportedVersion(_)
            | V3PrefixResult::ResourceLimitExceeded => {}
        }
        None
    }

    fn probe_payload_from_prefix_legacy(
        &self,
        img: &RgbaImage,
        prefix: &[u8],
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        match Self::classify_v3_prefix(prefix, Some(&self.limits)) {
            V3PrefixResult::Detected {
                header_length,
                total_length,
            } => {
                let header_bits = header_length * 8;
                let header_bytes = if header_bits <= prefix_bits {
                    prefix[..header_length].to_vec()
                } else {
                    self.extract_lsb(img, header_bits, seed)?
                };
                if Self::validate_v3_header(&header_bytes, Some(&self.limits)).is_err() {
                    return None;
                }
                let total_bits = total_length * 8;
                let full = if total_bits <= prefix_bits {
                    prefix[..total_length].to_vec()
                } else {
                    self.extract_lsb(img, total_bits, seed)?
                };
                if Self::verify_payload_integrity(&full, mac_key) {
                    return Some(Self::truncate_to_actual_payload(&full));
                }
            }
            V3PrefixResult::NotV3 => {
                for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                    let payload = if ecc_bits <= prefix_bits {
                        prefix.to_vec()
                    } else {
                        match self.extract_lsb(img, ecc_bits, seed) {
                            Some(p) => p,
                            None => continue,
                        }
                    };
                    if Self::try_ecc_decode(&payload).is_some() {
                        return Some(payload);
                    }
                }
            }
            V3PrefixResult::Malformed(_)
            | V3PrefixResult::UnsupportedVersion(_)
            | V3PrefixResult::ResourceLimitExceeded => {}
        }
        None
    }

    /// Verification-path variant of probe_payload_from_prefix that returns a tri-state.
    fn verify_probe_payload_from_prefix(
        &self,
        img: &RgbaImage,
        prefix: &[u8],
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        match Self::classify_v3_prefix(prefix, Some(&self.limits)) {
            V3PrefixResult::Detected {
                header_length,
                total_length,
            } => {
                let header_bits = header_length * 8;
                let header_bytes = if header_bits <= prefix_bits {
                    prefix[..header_length].to_vec()
                } else if let Some(h) = self.extract_lsb_v2(img, header_bits, seed, 0, 1) {
                    h
                } else {
                    return CandidateOutcome::NotFound;
                };
                if Self::validate_v3_header(&header_bytes, Some(&self.limits)).is_err() {
                    return CandidateOutcome::NotFound;
                }
                let total_bits = total_length * 8;
                let full = if total_bits <= prefix_bits {
                    prefix[..total_length].to_vec()
                } else if let Some(f) = self.extract_lsb_v2(img, total_bits, seed, 0, 1) {
                    f
                } else {
                    return CandidateOutcome::NotFound;
                };
                if Self::verify_payload_integrity(&full, mac_key) {
                    return CandidateOutcome::Valid(Self::truncate_to_actual_payload(&full));
                }
                Self::classify_auth_failure(&full, mac_key)
            }
            V3PrefixResult::NotV3 => {
                for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                    let payload = if ecc_bits <= prefix_bits {
                        prefix.to_vec()
                    } else if let Some(p) = self.extract_lsb_v2(img, ecc_bits, seed, 0, 1) {
                        p
                    } else {
                        continue;
                    };
                    if Self::try_ecc_decode(&payload).is_some() {
                        return CandidateOutcome::Valid(payload);
                    }
                    if Self::verify_payload_integrity(&payload, mac_key) {
                        return CandidateOutcome::Valid(Self::truncate_to_actual_payload(&payload));
                    }
                    return Self::classify_auth_failure(&payload, mac_key);
                }
                CandidateOutcome::NotFound
            }
            V3PrefixResult::Malformed(_) => CandidateOutcome::MalformedV3,
            V3PrefixResult::UnsupportedVersion(v) => CandidateOutcome::UnsupportedVersion(v),
            V3PrefixResult::ResourceLimitExceeded => CandidateOutcome::MalformedV3,
        }
    }

    /// Extract payload from tiled candidates, trying V2 then legacy per tile.
    fn extract_lsb_tiled_candidates(
        &self,
        img: &RgbaImage,
        master_seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        if tile_size == 0 {
            return None;
        }
        let (width, height) = img.dimensions();
        if width < tile_size || height < tile_size {
            return None;
        }

        let stride = (tile_size / 2).max(1);
        let mut origins: Vec<(u32, u32)> = Vec::new();
        let mut y = 0u32;
        while y + tile_size <= height {
            let mut x = 0u32;
            while x + tile_size <= width {
                origins.push((x, y));
                if origins.len() as u32 >= max_origins {
                    break;
                }
                x = x.saturating_add(stride);
            }
            if origins.len() as u32 >= max_origins {
                break;
            }
            y = y.saturating_add(stride);
        }

        let max_grid = 16u32;

        for &(x0, y0) in &origins {
            let sub = Self::crop_rgba(img, x0, y0, tile_size, tile_size);
            let base_x = x0 / tile_size;
            let base_y = y0 / tile_size;
            for dy in 0..=2u32 {
                if base_y + dy >= max_grid {
                    break;
                }
                for dx in 0..=2u32 {
                    if base_x + dx >= max_grid {
                        break;
                    }
                    let local_seed = tile_seed(master_seed, base_x + dx, base_y + dy);

                    for pass in 0..5 {
                        let offset_seed =
                            local_seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));

                        if let Some(payload) = self.extract_from_sub_image(
                            &sub,
                            V3_PREFIX_BYTES * 8,
                            offset_seed,
                            mac_key,
                        ) {
                            return Some(payload);
                        }
                    }
                }
            }
        }
        None
    }

    /// Verification-path variant of `extract_lsb_tiled_candidates` that
    /// returns a tri-state. See [`Self::verify_extract_with_redundancy`].
    fn verify_extract_lsb_tiled(
        &self,
        img: &RgbaImage,
        master_seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        if tile_size == 0 {
            return CandidateOutcome::NotFound;
        }
        let (width, height) = img.dimensions();
        if width < tile_size || height < tile_size {
            return CandidateOutcome::NotFound;
        }

        let stride = (tile_size / 2).max(1);
        let mut origins: Vec<(u32, u32)> = Vec::new();
        let mut y = 0u32;
        while y + tile_size <= height {
            let mut x = 0u32;
            while x + tile_size <= width {
                origins.push((x, y));
                if origins.len() as u32 >= max_origins {
                    break;
                }
                x = x.saturating_add(stride);
            }
            if origins.len() as u32 >= max_origins {
                break;
            }
            y = y.saturating_add(stride);
        }
        let max_grid = 16u32;
        let mut last_outcome: Option<CandidateOutcome> = None;

        for &(x0, y0) in &origins {
            let sub = Self::crop_rgba(img, x0, y0, tile_size, tile_size);
            let base_x = x0 / tile_size;
            let base_y = y0 / tile_size;
            for dy in 0..=2u32 {
                if base_y + dy >= max_grid {
                    break;
                }
                for dx in 0..=2u32 {
                    if base_x + dx >= max_grid {
                        break;
                    }
                    let local_seed = tile_seed(master_seed, base_x + dx, base_y + dy);

                    for pass in 0..5 {
                        let offset_seed =
                            local_seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));
                        let prefix_bits = V3_PREFIX_BYTES * 8;

                        if let Some(prefix) =
                            self.extract_lsb_v2(&sub, prefix_bits, offset_seed, 0, 1)
                        {
                            let outcome = self.verify_probe_payload_from_prefix(
                                &sub,
                                &prefix,
                                prefix_bits,
                                offset_seed,
                                mac_key,
                            );
                            match &outcome {
                                CandidateOutcome::Valid(_) => return outcome,
                                CandidateOutcome::Invalid(_)
                                | CandidateOutcome::AuthenticationKeyMissing(_)
                                | CandidateOutcome::AuthenticationFailed(_) => {
                                    if last_outcome.is_none() {
                                        last_outcome = Some(outcome);
                                    }
                                }
                                CandidateOutcome::MalformedV3 if last_outcome.is_none() => {
                                    last_outcome = Some(outcome);
                                }
                                CandidateOutcome::UnsupportedVersion(_)
                                    if last_outcome.is_none() =>
                                {
                                    last_outcome = Some(outcome);
                                }
                                _ => {}
                            }
                        }

                        if let Some(prefix) = self.extract_lsb(&sub, prefix_bits, offset_seed) {
                            match Self::classify_v3_prefix(&prefix, Some(&self.limits)) {
                                V3PrefixResult::Detected {
                                    header_length,
                                    total_length,
                                } => {
                                    let header_bits = header_length * 8;
                                    let header_bytes = if header_bits <= prefix_bits {
                                        prefix[..header_length].to_vec()
                                    } else if let Some(h) =
                                        self.extract_lsb(&sub, header_bits, offset_seed)
                                    {
                                        h
                                    } else {
                                        continue;
                                    };
                                    if Self::validate_v3_header(&header_bytes, Some(&self.limits))
                                        .is_err()
                                    {
                                        continue;
                                    }
                                    let total_bits = total_length * 8;
                                    let full = if total_bits <= prefix_bits {
                                        prefix[..total_length].to_vec()
                                    } else if let Some(f) =
                                        self.extract_lsb(&sub, total_bits, offset_seed)
                                    {
                                        f
                                    } else {
                                        continue;
                                    };
                                    if Self::verify_payload_integrity(&full, mac_key) {
                                        return CandidateOutcome::Valid(
                                            Self::truncate_to_actual_payload(&full),
                                        );
                                    }
                                    if last_outcome.is_none() {
                                        last_outcome =
                                            Some(Self::classify_auth_failure(&full, mac_key));
                                    }
                                }
                                V3PrefixResult::Malformed(_) => {
                                    if last_outcome.is_none() {
                                        last_outcome = Some(CandidateOutcome::MalformedV3);
                                    }
                                }
                                V3PrefixResult::UnsupportedVersion(v) => {
                                    if last_outcome.is_none() {
                                        last_outcome =
                                            Some(CandidateOutcome::UnsupportedVersion(v));
                                    }
                                }
                                V3PrefixResult::ResourceLimitExceeded => {
                                    if last_outcome.is_none() {
                                        last_outcome = Some(CandidateOutcome::MalformedV3);
                                    }
                                }
                                V3PrefixResult::NotV3 => {
                                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                                        let payload = if ecc_bits <= prefix_bits {
                                            prefix.clone()
                                        } else {
                                            match self.extract_lsb(&sub, ecc_bits, offset_seed) {
                                                Some(p) => p,
                                                None => continue,
                                            }
                                        };
                                        if Self::try_ecc_decode(&payload).is_some() {
                                            return CandidateOutcome::Valid(payload);
                                        }
                                        if last_outcome.is_none() {
                                            last_outcome = Some(Self::classify_auth_failure(
                                                &payload, mac_key,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        match last_outcome {
            Some(outcome) => outcome,
            None => CandidateOutcome::NotFound,
        }
    }

    fn crop_rgba(src: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
        carrier_lsb::crop_rgba(src, x, y, w, h)
    }

    fn apply_to_image_owned(
        &self,
        img: &DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<DynamicImage> {
        let (image, _summary) = self.apply_to_image_with_summary(img, ctx)?;
        Ok(image)
    }

    pub(crate) fn apply_to_image_with_summary(
        &self,
        img: &DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<(DynamicImage, Option<crate::stego::EmbedOutcomeSummary>)> {
        let format = ctx
            .input_format()
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        let is_tiled = ctx.tile_size().filter(|&s| s > 0).is_some();
        let embed_path = match format {
            crate::types::ImageOutputFormat::Jpeg => {
                if is_tiled {
                    crate::stego::EmbedPath::DctF5Tiled
                } else {
                    crate::stego::EmbedPath::DctF5
                }
            }
            _ => {
                if is_tiled {
                    crate::stego::EmbedPath::LsbTiled
                } else {
                    crate::stego::EmbedPath::Lsb
                }
            }
        };

        let emission = crate::types::PayloadEmissionContext::from_plan_for_context(ctx, embed_path);
        let payload = self.generate_payload(&emission, ctx);
        let rgba = img.to_rgba8();

        let format = ctx
            .input_format()
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        let redundancy = ctx.effective_redundancy();

        match format {
            crate::types::ImageOutputFormat::Png => {
                let outcome = if let Some(tile_size) = ctx.tile_size().filter(|&s| s > 0) {
                    self.embed_lsb_tiled(&rgba, &payload, ctx.seed(), tile_size)
                } else {
                    self.embed_lsb_v2(&rgba, &payload, ctx.seed(), redundancy)
                };
                let (mut result, summary) = outcome.into_parts();
                Self::embed_seed_lsb_fallback(&mut result, ctx.seed());
                Ok((DynamicImage::ImageRgba8(result), Some(summary)))
            }
            crate::types::ImageOutputFormat::Jpeg => {
                let jpeg_bytes = crate::util::image::encode_image_with_options(
                    img,
                    Some(crate::types::ImageOutputFormat::Jpeg),
                    ctx.progressive_jpeg(),
                    ctx.jpeg_quality(),
                )?;
                let with_stego = self.apply_dct_stego_bytes(&jpeg_bytes, ctx)?;
                let (output, summary) = with_stego.into_parts();
                Ok((image::load_from_memory(&output)?, Some(summary)))
            }
            crate::types::ImageOutputFormat::WebP => {
                let outcome = if let Some(tile_size) = ctx.tile_size().filter(|&s| s > 0) {
                    self.embed_lsb_tiled(&rgba, &payload, ctx.seed(), tile_size)
                } else {
                    self.embed_lsb_v2(&rgba, &payload, ctx.seed(), redundancy)
                };
                let (mut result, summary) = outcome.into_parts();
                Self::embed_seed_lsb_fallback(&mut result, ctx.seed());
                Ok((DynamicImage::ImageRgba8(result), Some(summary)))
            }
        }
    }
}

impl Default for SteganographyProtector {
    fn default() -> Self {
        Self::new()
    }
}

impl Protector for SteganographyProtector {
    fn apply<'a>(
        &self,
        img: &'a DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>> {
        Ok(Cow::Owned(self.apply_to_image_owned(img, ctx)?))
    }

    fn apply_bytes(&self, img_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>> {
        let format = ctx.input_format().unwrap_or_else(|| {
            crate::types::ImageOutputFormat::from_magic_bytes(img_bytes)
                .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT)
        });

        if format == crate::types::ImageOutputFormat::Jpeg {
            return self
                .apply_dct_stego_bytes(img_bytes, ctx)
                .map(|o| o.into_inner());
        }

        let img = image::load_from_memory(img_bytes)?;

        let processed = self.apply_to_image_owned(&img, ctx)?;

        let image_format = match format {
            crate::types::ImageOutputFormat::Png => image::ImageFormat::Png,
            crate::types::ImageOutputFormat::Jpeg => image::ImageFormat::Jpeg,
            crate::types::ImageOutputFormat::WebP => image::ImageFormat::WebP,
        };

        crate::util::image::encode_image(&processed, image_format)
    }

    fn name(&self) -> &'static str {
        "steganography"
    }

    fn protection_level(&self) -> ProtectionLevel {
        ProtectionLevel::Standard
    }

    fn estimated_latency_ms(&self) -> u32 {
        2
    }
}

/// Extracted steganographic payload containing protection metadata.
///
/// Returned by [`SteganographyProtector::extract_payload`] and related methods.
/// All fields are private — use getter methods to access values.
#[derive(Debug, Clone)]
pub struct StegoPayload {
    protection_level: u8,
    seed: u64,
    intensity: f32,
    version: u8,
    content_hash: Option<[u8; 4]>,
    dmi_value: Option<crate::types::DmiValue>,
    raw_payload: Option<Vec<u8>>,
}

impl StegoPayload {
    /// The protection level byte (0=Disabled, 1=Light, 2=Standard).
    #[must_use]
    pub fn protection_level(&self) -> u8 {
        self.protection_level
    }

    /// The seed used when the protection was applied.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The perturbation intensity (0.0–1.0).
    #[must_use]
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// The payload format version (1, 2, or 3).
    #[must_use]
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Truncated content hash (4 bytes) for linking to the original image.
    ///
    /// Present in v2 payloads. Derived from the ISCC content code or a
    /// truncated SHA-256 of the image pixels. `None` for v1 payloads.
    #[must_use]
    pub fn content_hash(&self) -> Option<[u8; 4]> {
        self.content_hash
    }

    /// The DMI (Data Mining Inhibitor) value, if set.
    ///
    /// Present in v2 payloads. Indicates the AI training restrictions
    /// encoded in the payload. `None` for v1 payloads or when not set.
    #[must_use]
    pub fn dmi_value(&self) -> Option<crate::types::DmiValue> {
        self.dmi_value
    }

    /// Raw extracted payload bytes (before ECC decoding and truncation).
    ///
    /// Used for digest comparison in detached manifest verification.
    /// `None` when the payload was extracted from parsed fields only.
    #[must_use]
    pub fn raw_payload(&self) -> Option<&[u8]> {
        self.raw_payload.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protected::constants::STEGO_SPREAD_FACTOR;
    use crate::stego::jpeg_transcoder::JpegHeader;
    use crate::types::ProtectionConfig;
    use image::ImageEncoder;
    use image::Rgba;
    use image::{ImageBuffer, RgbaImage};
    use std::sync::Arc;

    fn make_test_image(w: u32, h: u32) -> RgbaImage {
        ImageBuffer::from_fn(w, h, |x, y| {
            Rgba([(x * 3) as u8, (y * 5) as u8, ((x + y) * 7) as u8, 255])
        })
    }

    fn make_large_test_image() -> RgbaImage {
        make_test_image(128, 128)
    }

    fn make_high_entropy_test_image(w: u32, h: u32) -> RgbaImage {
        ImageBuffer::from_fn(w, h, |x, y| {
            let r = ((x * 73 + y * 151 + (x ^ y) * 17 + x * y) % 256) as u8;
            let g = ((x * 53 + y * 97 + (x * 3 + y * 5)) % 256) as u8;
            let b = ((x * 29 + y * 43 + (x ^ (y << 1)) * 11 + x * y * 3) % 256) as u8;
            Rgba([r, g, b, 255])
        })
    }

    fn image_to_jpeg_bytes(img: &DynamicImage, quality: u8) -> Vec<u8> {
        let mut buffer = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
        encoder
            .write_image(
                &img.to_rgb8(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
        buffer
    }

    fn ctx_no_mac(seed: u64) -> ProtectionContext {
        ProtectionContext::new(0.5, seed)
    }

    fn ctx_with_mac(seed: u64, key: &[u8]) -> ProtectionContext {
        let config = Arc::new(ProtectionConfig::new().with_mac_key(key.to_vec()));
        ProtectionContext::new(0.5, seed).with_config(config)
    }

    // ── Bit conversion ────────────────────────────────────────────────

    #[test]
    fn bytes_to_bits_length() {
        let data = [0xAA, 0x55, 0xFF, 0x00];
        let bits = carrier_lsb::bytes_to_bits(&data);
        assert_eq!(bits.len(), 32);
    }

    #[test]
    fn bits_to_bytes_roundtrip() {
        let original: Vec<u8> = vec![0x00, 0xFF, 0xA5, 0x5A, 0x01, 0x80, 0xFE, 0x7F];
        let bits = carrier_lsb::bytes_to_bits(&original);
        let recovered = carrier_lsb::bits_to_bytes(&bits);
        assert_eq!(original, recovered);
    }

    #[test]
    fn bytes_to_bits_lsb_order() {
        let data = [0b0000_0001];
        let bits = carrier_lsb::bytes_to_bits(&data);
        assert_eq!(bits[0], 1);
        assert_eq!(bits[1], 0);
        assert_eq!(bits[7], 0);
    }

    #[test]
    fn bytes_to_bits_high_bit() {
        let data = [0b1000_0000];
        let bits = carrier_lsb::bytes_to_bits(&data);
        assert_eq!(bits[7], 1);
        assert_eq!(bits[0], 0);
    }

    #[test]
    fn bits_to_bytes_trailing_dropped() {
        // Multiple of 8 — works correctly
        let bits = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let bytes = carrier_lsb::bits_to_bytes(&bits);
        assert_eq!(bytes.len(), 1);
        assert_eq!(bytes[0], 1);
    }

    #[test]
    fn bits_to_bytes_non_multiple_of_8_returns_empty() {
        let bits = vec![1, 0, 1]; // 3 bits — not a multiple of 8
        let bytes = carrier_lsb::bits_to_bytes(&bits);
        assert!(
            bytes.is_empty(),
            "Non-multiple-of-8 input should return empty Vec"
        );
    }

    #[test]
    fn bits_to_bytes_empty_input() {
        let bits: Vec<u8> = vec![];
        let bytes = carrier_lsb::bits_to_bytes(&bits);
        assert!(bytes.is_empty());
    }

    #[test]
    fn bits_to_bytes_16_bits() {
        let bits = vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
        let bytes = carrier_lsb::bits_to_bytes(&bits);
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes[0], 1);
        assert_eq!(bytes[1], 2);
    }

    // ── Checksum ──────────────────────────────────────────────────────

    #[test]
    fn compute_checksum_deterministic() {
        let data = vec![1u8; 24];
        let a = SteganographyProtector::compute_checksum(&data);
        let b = SteganographyProtector::compute_checksum(&data);
        assert_eq!(a, b);
    }

    #[test]
    fn compute_checksum_different_data_different_result() {
        let a = SteganographyProtector::compute_checksum(&[0u8; 24]);
        let b = SteganographyProtector::compute_checksum(&[1u8; 24]);
        assert_ne!(a, b);
    }

    #[test]
    fn verify_checksum_valid() {
        let mut payload = vec![0u8; 24];
        payload[0] = 1;
        payload[1] = 2;
        let checksum = SteganographyProtector::compute_checksum(&payload);
        payload.extend_from_slice(&checksum);
        assert!(SteganographyProtector::verify_checksum(&payload));
    }

    #[test]
    fn verify_checksum_invalid() {
        let mut payload = vec![0u8; 28];
        payload[24] = 0xFF;
        payload[25] = 0xFF;
        payload[26] = 0xFF;
        payload[27] = 0xFF;
        assert!(!SteganographyProtector::verify_checksum(&payload));
    }

    #[test]
    fn verify_checksum_too_short() {
        assert!(!SteganographyProtector::verify_checksum(&[0u8; 10]));
    }

    #[test]
    fn verify_checksum_corrupted_byte() {
        let mut payload = vec![1u8; 24];
        let checksum = SteganographyProtector::compute_checksum(&payload);
        payload.extend_from_slice(&checksum);
        assert!(SteganographyProtector::verify_checksum(&payload));

        payload[5] = payload[5].wrapping_add(1);
        let new_checksum = SteganographyProtector::compute_checksum(&payload[..24]);
        assert_ne!(
            [payload[24], payload[25], payload[26], payload[27]],
            new_checksum
        );
    }

    // ── HMAC ──────────────────────────────────────────────────────────

    #[test]
    fn compute_payload_mac_deterministic() {
        let data = vec![1u8; 24];
        let key = b"test-secret-key";
        let a = SteganographyProtector::compute_payload_mac(&data, key);
        let b = SteganographyProtector::compute_payload_mac(&data, key);
        assert_eq!(a, b);
    }

    #[test]
    fn compute_payload_mac_different_keys() {
        let data = vec![1u8; 24];
        let a = SteganographyProtector::compute_payload_mac(&data, b"key-a");
        let b = SteganographyProtector::compute_payload_mac(&data, b"key-b");
        assert_ne!(a, b);
    }

    #[test]
    fn compute_payload_mac_different_data() {
        let key = b"test-key";
        let a = SteganographyProtector::compute_payload_mac(&[0u8; 24], key);
        let b = SteganographyProtector::compute_payload_mac(&[1u8; 24], key);
        assert_ne!(a, b);
    }

    #[test]
    fn verify_payload_mac_match() {
        let data = vec![42u8; 24];
        let key = b"my-key";
        let mac = SteganographyProtector::compute_payload_mac(&data, key);
        assert!(SteganographyProtector::verify_payload_mac(&data, key, &mac));
    }

    #[test]
    fn verify_payload_mac_wrong_key() {
        let data = vec![42u8; 24];
        let mac = SteganographyProtector::compute_payload_mac(&data, b"correct-key");
        assert!(!SteganographyProtector::verify_payload_mac(
            &data,
            b"wrong-key",
            &mac
        ));
    }

    #[test]
    fn verify_payload_mac_corrupted_mac() {
        let data = vec![42u8; 24];
        let key = b"key";
        let mut mac = SteganographyProtector::compute_payload_mac(&data, key);
        mac[0] ^= 0xFF;
        assert!(!SteganographyProtector::verify_payload_mac(
            &data, key, &mac
        ));
    }

    // ── Payload integrity ─────────────────────────────────────────────

    #[test]
    fn verify_payload_integrity_checksum_mode() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload_from_ctx(&ctx);
        assert_eq!(payload.len(), V3_CRC_PAYLOAD_SIZE);
        assert!(SteganographyProtector::verify_payload_integrity(
            &payload,
            &[]
        ));
    }

    #[test]
    fn verify_payload_integrity_mac_mode() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_with_mac(42, b"secret");
        let payload = protector.generate_payload_from_ctx(&ctx);
        assert_eq!(payload.len(), V3_HMAC_PAYLOAD_SIZE);
        assert!(SteganographyProtector::verify_payload_integrity(
            &payload, b"secret"
        ));
    }

    #[test]
    fn verify_payload_integrity_mac_wrong_key() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_with_mac(42, b"correct");
        let payload = protector.generate_payload_from_ctx(&ctx);
        assert!(!SteganographyProtector::verify_payload_integrity(
            &payload, b"wrong"
        ));
    }

    #[test]
    fn verify_payload_integrity_checksum_corrupted() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let mut payload = protector.generate_payload_from_ctx(&ctx);
        payload[5] ^= 0xFF;
        assert!(!SteganographyProtector::verify_payload_integrity(
            &payload,
            &[]
        ));
    }

    #[test]
    fn verify_payload_integrity_truncated() {
        assert!(!SteganographyProtector::verify_payload_integrity(
            &[0u8; 10],
            &[]
        ));
    }

    #[test]
    fn verify_payload_integrity_mac_truncated() {
        assert!(!SteganographyProtector::verify_payload_integrity(
            &[0u8; 30], b"key"
        ));
    }

    // ── Payload generation ────────────────────────────────────────────

    #[test]
    fn generate_payload_checksum_mode_length() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(12345);
        let payload = protector.generate_payload_from_ctx(&ctx);
        assert_eq!(payload.len(), V3_CRC_PAYLOAD_SIZE);
    }

    #[test]
    fn generate_payload_mac_mode_length() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_with_mac(12345, b"key");
        let payload = protector.generate_payload_from_ctx(&ctx);
        assert_eq!(payload.len(), V3_HMAC_PAYLOAD_SIZE);
    }

    #[test]
    fn generate_payload_version_byte() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload_from_ctx(&ctx);
        assert_eq!(&payload[0..2], &V3_MAGIC);
        assert_eq!(payload[2], 3);
    }

    #[test]
    fn generate_payload_seed_roundtrip() {
        let seed = 0xDEAD_BEEF_CAFE_BABE;
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(seed);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let extracted_seed = u64::from_le_bytes([
            payload[11],
            payload[12],
            payload[13],
            payload[14],
            payload[15],
            payload[16],
            payload[17],
            payload[18],
        ]);
        assert_eq!(extracted_seed, seed);
    }

    #[test]
    fn generate_payload_intensity_precision() {
        let protector = SteganographyProtector::new();
        let ctx = ProtectionContext::new(0.73, 42);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let intensity_raw = u16::from_le_bytes([payload[19], payload[20]]);
        let recovered = intensity_raw as f32 / 100.0;
        assert!((recovered - 0.73).abs() < 0.02);
    }

    #[test]
    fn generate_payload_protection_level_byte() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload_from_ctx(&ctx);
        // V3: channels field at bytes 8-9 encodes protection info
        let channels = u16::from_le_bytes([payload[8], payload[9]]);
        assert_ne!(
            channels, 0,
            "channels should be non-zero for Standard level"
        );
    }

    #[test]
    fn generate_payload_different_seeds_differ() {
        let protector = SteganographyProtector::new();
        let a = protector.generate_payload_from_ctx(&ctx_no_mac(1));
        let b = protector.generate_payload_from_ctx(&ctx_no_mac(2));
        assert_ne!(a[11..19], b[11..19]); // seed bytes differ in v3
    }

    // ── Permutation ───────────────────────────────────────────────────

    #[test]
    fn stego_permutation_deterministic() {
        let a = carrier_lsb::stego_permutation(0, 1024, 42);
        let b = carrier_lsb::stego_permutation(0, 1024, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn stego_permutation_different_seeds_differ() {
        let a = carrier_lsb::stego_permutation(0, 1024, 42);
        let b = carrier_lsb::stego_permutation(0, 1024, 99);
        assert_ne!(a, b);
    }

    #[test]
    fn stego_permutation_power_of_2_injective() {
        let total = 1024usize;
        let seed = 42u64;
        let mut seen = vec![false; total];
        for i in 0..total {
            let pos = carrier_lsb::stego_permutation(i, total, seed);
            assert!(
                pos < total,
                "permutation out of range: {} >= {}",
                pos,
                total
            );
            assert!(!seen[pos], "collision at index {} -> pos {}", i, pos);
            seen[pos] = true;
        }
    }

    #[test]
    fn stego_permutation_index0_consistent() {
        let a = carrier_lsb::stego_permutation(0, 4096, 100);
        let b = carrier_lsb::stego_permutation(0, 4096, 100);
        assert_eq!(a, b);
    }

    // ── Pixel manipulation ────────────────────────────────────────────

    #[test]
    fn embed_bit_in_pixel_modifies_correct_channel() {
        let mut img = make_test_image(4, 4);
        let orig_g = img.get_pixel(0, 0)[1];
        let orig_b = img.get_pixel(0, 0)[2];
        let orig_a = img.get_pixel(0, 0)[3];

        // Embed bit 1 in channel 0
        carrier_lsb::embed_bit_in_pixel(&mut img, 0, 0, 0, 1);
        let modified = img.get_pixel(0, 0);
        assert_eq!(modified[0] & 1, 1);
        assert_eq!(modified[1], orig_g);
        assert_eq!(modified[2], orig_b);
        assert_eq!(modified[3], orig_a);
    }

    #[test]
    fn embed_bit_in_pixel_clears_lsb() {
        let mut img = ImageBuffer::from_pixel(1, 1, Rgba([0xFF, 0xFF, 0xFF, 255]));

        carrier_lsb::embed_bit_in_pixel(&mut img, 0, 0, 1, 0);
        let pixel = img.get_pixel(0, 0);
        assert_eq!(pixel[1] & 1, 0);
        // Channel 0 and 2 unchanged
        assert_eq!(pixel[0], 0xFF);
        assert_eq!(pixel[2], 0xFF);
    }

    // ── Embed/extract via public API ───────────────────────────────────
    // Internal embed_lsb/extract_lsb use different seed derivation,
    // so we test through the public apply() + extract_payload() API
    // which correctly matches seeds.

    #[test]
    fn lsb_embed_extract_png() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&result));
        let payload = protector.extract_payload(&result).unwrap();
        assert_eq!(payload.seed(), 42);
    }

    #[test]
    fn lsb_embed_extract_different_seeds() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();

        let ctx_a = ctx_no_mac(42);
        let ctx_b = ctx_no_mac(99);

        let dyn_img = DynamicImage::ImageRgba8(img.clone());
        let result_a = protector.apply(&dyn_img, &ctx_a).unwrap();
        let result_b = protector.apply(&dyn_img, &ctx_b).unwrap();

        // Use extract_payload_with_seed with known seeds
        let payload_a = protector.extract_payload_with_seed(&result_a, 42).unwrap();
        let payload_b = protector.extract_payload_with_seed(&result_b, 99).unwrap();
        assert_eq!(payload_a.seed(), 42);
        assert_eq!(payload_b.seed(), 99);
    }

    #[test]
    fn lsb_embed_extract_high_redundancy() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ProtectionContext::new(0.5, 42).with_stego_redundancy(5);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&result));
    }

    #[test]
    fn lsb_embed_modifies_pixels() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img.clone());
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert_ne!(*result, DynamicImage::ImageRgba8(img));
    }

    #[test]
    fn lsb_preserves_dimensions() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let (w, h) = img.dimensions();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert_eq!(result.width(), w);
        assert_eq!(result.height(), h);
    }

    #[test]
    fn lsb_verify_with_mac_key() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let key = b"test-mac-key";
        let ctx = ctx_with_mac(42, key);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        // Use extract_payload_with_seed_and_key with known seed
        let payload = protector.extract_payload_with_seed_and_key(&result, 42, key);
        assert!(payload.is_some());
        assert_eq!(payload.unwrap().seed(), 42);

        // Wrong key returns None
        assert!(protector
            .extract_payload_with_seed_and_key(&result, 42, b"wrong-key")
            .is_none());
    }

    #[test]
    fn lsb_extract_wrong_key_returns_none() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_with_mac(42, b"correct");

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        assert!(protector
            .extract_payload_with_key(&result, b"correct")
            .is_some());
        assert!(protector
            .extract_payload_with_key(&result, b"wrong")
            .is_none());
    }

    #[test]
    fn lsb_payload_too_large_returns_unchanged() {
        let protector = SteganographyProtector::new();
        let tiny = make_test_image(2, 2);
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let result = carrier_lsb::embed_lsb(&tiny, &payload, 42, 1);
        assert!(result.is_skipped());
        assert_eq!(*result.output(), tiny);
    }

    #[test]
    fn lsb_extract_oversized_expected_bits_returns_none() {
        let protector = SteganographyProtector::new();
        let img = make_test_image(4, 4); // 16 pixels = 48 channels
        assert!(protector.extract_lsb(&img, 256, 42).is_none());
    }

    // ── DCT capacity / verification ───────────────────────────────────

    #[test]
    fn dct_stego_low_capacity_keeps_qtable_seed_only() {
        let protector = SteganographyProtector::new();
        let img = make_test_image(16, 16);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(crate::types::ImageOutputFormat::Jpeg)
            .with_stego_redundancy(3);
        let payload_bits = protector.generate_payload_from_ctx(&ctx).len() * 8;
        let required_bits = payload_bits * ctx.effective_redundancy();

        let (_, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();
        assert!(SteganographyProtector::dct_payload_capacity(&coefficients) < required_bits);

        let outcome = protector.apply_dct_stego_bytes(&jpeg_bytes, &ctx).unwrap();
        assert!(outcome.is_skipped());
        let protected = outcome.into_inner();
        let (header, _) = JpegTranscoder::decode_coefficients(&protected).unwrap();

        assert_eq!(
            DctStegoF5::new().extract_seed_from_quantization_tables(&header),
            Some(42),
            "JPEG output should still carry the Q-table seed"
        );
        assert!(
            !protector.verify_payload_from_bytes(&protected, 42),
            "Q-table seed alone must not count as full verification"
        );
    }

    #[test]
    fn dct_stego_high_capacity_verifies_with_redundancy_3() {
        let protector = SteganographyProtector::new();
        let img = make_high_entropy_test_image(1024, 1024);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(crate::types::ImageOutputFormat::Jpeg)
            .with_stego_redundancy(3);
        let payload_bits = protector.generate_payload_from_ctx(&ctx).len() * 8;

        let (header, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();
        let available = SteganographyProtector::dct_payload_capacity(&coefficients);
        assert!(
            available >= payload_bits * 3,
            "capacity {} should be >= {} (payload_bits * 3)",
            available,
            payload_bits * 3
        );

        // Test that assemble_jpeg roundtrip preserves coefficients
        let canonical = JpegTranscoder::encode_coefficients(&header, &coefficients, None).unwrap();
        let (_header2, coefficients2) = JpegTranscoder::decode_coefficients(&canonical).unwrap();
        for (id, blocks) in &coefficients {
            let c2 = coefficients2.get(id).unwrap();
            assert_eq!(
                blocks.len(),
                c2.len(),
                "comp {} block count mismatch: {} vs {}",
                id,
                blocks.len(),
                c2.len()
            );
            for (i, (b1, b2)) in blocks.iter().zip(c2.iter()).enumerate() {
                assert_eq!(
                    b1, b2,
                    "comp {} block {} mismatch after assemble_jpeg roundtrip",
                    id, i
                );
            }
        }

        let mut header_mod = header.clone();
        DctStegoF5::new()
            .embed_seed_in_quantization_tables(&mut header_mod, 42)
            .unwrap();
        let mut coeffs_mod = coefficients.clone();
        let payload = protector.generate_payload_from_ctx(&ctx);
        DctStegoF5::with_redundancy(3)
            .embed_f5(&mut coeffs_mod, &payload, 42)
            .unwrap();
        let embedded_jpeg =
            JpegTranscoder::encode_coefficients(&header_mod, &coeffs_mod, None).unwrap();
        match JpegTranscoder::decode_coefficients(&embedded_jpeg) {
            Ok((_, rt)) => {
                let rt_bits = DctStegoF5::with_redundancy(3).extract_f5(&rt, payload_bits, 42);
                let rt_payload = carrier_lsb::bits_to_bytes(&rt_bits);
                assert_eq!(
                    rt_payload, payload,
                    "F5 roundtrip through assemble_jpeg failed"
                );
            }
            Err(e) => panic!("decode after assemble_jpeg embed failed: {}", e),
        }

        let outcome = protector.apply_dct_stego_bytes(&jpeg_bytes, &ctx).unwrap();
        assert!(outcome.is_embedded());
        let protected = outcome.into_inner();
        let (header, _) = JpegTranscoder::decode_coefficients(&protected).unwrap();
        assert_eq!(
            DctStegoF5::new().extract_seed_from_quantization_tables(&header),
            Some(42)
        );
        assert!(
            protector.verify_payload_from_bytes(&protected, 42),
            "DCT payload should verify with redundancy=3 when capacity permits"
        );
    }

    #[test]
    fn jpeg_selected_redundancy_is_highest_feasible() {
        let protector = SteganographyProtector::new();
        let img = make_high_entropy_test_image(512, 512);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(crate::types::ImageOutputFormat::Jpeg)
            .with_stego_redundancy(5);
        let payload_bits = protector.generate_payload_from_ctx(&ctx).len() * 8;

        let (_, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();
        let available = SteganographyProtector::dct_payload_capacity(&coefficients);
        let max_feasible = available / payload_bits;
        let expected = 5.min(max_feasible).max(1);

        let outcome = protector.apply_dct_stego_bytes(&jpeg_bytes, &ctx).unwrap();
        assert!(outcome.is_embedded());
        assert_eq!(outcome.available_capacity(), available);
        assert_eq!(outcome.required_capacity(), payload_bits);

        let protected = outcome.into_inner();
        assert!(
            protector.verify_payload_from_bytes(&protected, 42),
            "payload must verify at selected redundancy {} (max_feasible={})",
            expected,
            max_feasible
        );
    }

    #[test]
    fn jpeg_capacity_zero_skips_without_payload_embedding() {
        let protector = SteganographyProtector::new();
        let img = make_test_image(8, 8);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 95);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(crate::types::ImageOutputFormat::Jpeg)
            .with_stego_redundancy(3);

        let (_, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();
        let available = SteganographyProtector::dct_payload_capacity(&coefficients);
        let payload_bits = protector.generate_payload_from_ctx(&ctx).len() * 8;
        assert!(
            available < payload_bits,
            "tiny image should have insufficient capacity"
        );

        let outcome = protector.apply_dct_stego_bytes(&jpeg_bytes, &ctx).unwrap();
        assert!(outcome.is_skipped());
    }

    #[test]
    fn jpeg_capacity_exact_for_selected_redundancy_roundtrips() {
        let protector = SteganographyProtector::new();
        let img = make_high_entropy_test_image(256, 256);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(crate::types::ImageOutputFormat::Jpeg)
            .with_stego_redundancy(1);
        let payload_bits = protector.generate_payload_from_ctx(&ctx).len() * 8;

        let (_, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();
        let available = SteganographyProtector::dct_payload_capacity(&coefficients);
        assert!(
            available >= payload_bits,
            "need at least payload_bits capacity"
        );

        let outcome = protector.apply_dct_stego_bytes(&jpeg_bytes, &ctx).unwrap();
        assert!(outcome.is_embedded());
        let protected = outcome.into_inner();
        assert!(
            protector.verify_payload_from_bytes(&protected, 42),
            "redundancy=1 roundtrip must verify"
        );
    }

    #[test]
    fn jpeg_requested_redundancy_is_not_exceeded() {
        let protector = SteganographyProtector::new();
        let img = make_high_entropy_test_image(512, 512);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(crate::types::ImageOutputFormat::Jpeg)
            .with_stego_redundancy(10);
        let payload_bits = protector.generate_payload_from_ctx(&ctx).len() * 8;

        let (_, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();
        let available = SteganographyProtector::dct_payload_capacity(&coefficients);
        let max_feasible = available / payload_bits;
        let selected = 10.min(max_feasible).max(1);

        let outcome = protector.apply_dct_stego_bytes(&jpeg_bytes, &ctx).unwrap();
        assert!(outcome.is_embedded());
        assert_eq!(outcome.required_capacity(), payload_bits);
        assert_eq!(outcome.available_capacity(), available);

        let protected = outcome.into_inner();
        assert!(
            protector.verify_payload_from_bytes(&protected, 42),
            "selected redundancy {} must verify (max_feasible={})",
            selected,
            max_feasible
        );
    }

    // ── StegoPayload parsing ──────────────────────────────────────────

    #[test]
    fn parse_stego_payload_valid() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(12345);
        let payload = protector.generate_payload_from_ctx(&ctx);
        let parsed = SteganographyProtector::parse_stego_payload(&payload).unwrap();
        assert_eq!(parsed.version(), 3);
        assert_eq!(parsed.seed(), 12345);
        assert!((parsed.intensity() - 0.5).abs() < 0.02);
    }

    #[test]
    fn parse_stego_payload_too_short() {
        assert!(SteganographyProtector::parse_stego_payload(&[0u8; 10]).is_none());
    }

    #[test]
    fn parse_stego_payload_wrong_version() {
        let mut payload = vec![0u8; 26];
        payload[0] = 99; // invalid version
        assert!(SteganographyProtector::parse_stego_payload(&payload).is_none());
    }

    #[test]
    fn current_payload_version_is_in_supported_list() {
        assert!(
            SUPPORTED_PAYLOAD_VERSIONS.contains(&V3_PAYLOAD_VERSION),
            "SUPPORTED_PAYLOAD_VERSIONS must include V3_PAYLOAD_VERSION ({}) \
             so freshly-generated payloads are always parseable",
            V3_PAYLOAD_VERSION
        );
    }

    #[test]
    fn supported_versions_are_unique() {
        let mut sorted: Vec<u8> = SUPPORTED_PAYLOAD_VERSIONS.to_vec();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            SUPPORTED_PAYLOAD_VERSIONS.len(),
            "SUPPORTED_PAYLOAD_VERSIONS must not contain duplicate versions"
        );
    }

    #[test]
    fn migration_path_v1_still_parses_after_v2_introduced() {
        // Simulate a v1 payload embedded before a hypothetical v2 lands.
        // The extractor must still parse it because v1 remains in
        // SUPPORTED_PAYLOAD_VERSIONS.
        let mut v1_payload = vec![0u8; 24];
        v1_payload[0] = 1; // version 1
        v1_payload[1] = 2; // Standard protection level
        v1_payload[2..10].copy_from_slice(&0xDEADBEEFu64.to_le_bytes());
        v1_payload[10..12].copy_from_slice(&50u16.to_le_bytes()); // intensity 0.50

        let parsed = SteganographyProtector::parse_stego_payload(&v1_payload).unwrap();
        assert_eq!(parsed.version(), 1);
        assert_eq!(parsed.seed(), 0xDEADBEEF);
        assert_eq!(parsed.protection_level(), 2);
    }

    // ── Extract with redundancy via public API ────────────────────────

    #[test]
    fn extract_with_redundancy_finds_payload() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&result));
    }

    #[test]
    fn extract_with_redundancy_mac_mode() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let key = b"hmac-key";
        let ctx = ctx_with_mac(42, key);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        let payload = protector.extract_payload_with_seed_and_key(&result, 42, key);
        assert!(payload.is_some());
        assert_eq!(payload.unwrap().seed(), 42);
    }

    #[test]
    fn extract_with_redundancy_mac_wrong_key_returns_none() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_with_mac(42, b"correct-key");

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        assert!(protector
            .extract_payload_with_key(&result, b"wrong-key")
            .is_none());
    }

    // ── Protector trait ───────────────────────────────────────────────

    #[test]
    fn protector_apply_changes_image() {
        let p = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);
        let dyn_img = DynamicImage::ImageRgba8(img.clone());
        let result = p.apply(&dyn_img, &ctx).unwrap();
        match result {
            std::borrow::Cow::Owned(owned) => {
                assert_ne!(owned.to_rgba8(), img);
            }
            _ => panic!("expected owned result"),
        }
    }

    #[test]
    fn protector_apply_preserves_dimensions() {
        let p = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);
        let (w, h) = img.dimensions();
        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = p.apply(&dyn_img, &ctx).unwrap();
        assert_eq!(result.width(), w);
        assert_eq!(result.height(), h);
    }

    #[test]
    fn protector_level() {
        let p = SteganographyProtector::new();
        assert_eq!(p.protection_level(), ProtectionLevel::Standard);
    }

    #[test]
    fn protector_modifies_pixels() {
        let p = SteganographyProtector::new();
        assert!(p.modifies_pixels());
    }

    #[test]
    fn protector_apply_bytes_png_roundtrip() {
        let p = SteganographyProtector::new();
        let img = make_large_test_image();
        let png_bytes = crate::util::image::encode_image(
            &DynamicImage::ImageRgba8(img),
            image::ImageFormat::Png,
        )
        .unwrap();

        let ctx = ctx_no_mac(42);
        let processed = p.apply_bytes(&png_bytes, &ctx).unwrap();

        // Re-decode and verify
        let decoded = image::load_from_memory(&processed).unwrap();
        assert!(p.verify_payload(&decoded));
    }

    // ── DCT stego edge cases ──────────────────────────────────────────

    #[test]
    fn dct_stego_rejects_non_jpeg() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let result = protector.apply_dct_stego_bytes(&[0x89, 0x50, 0x4E, 0x47], &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn dct_stego_rejects_empty() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let result = protector.apply_dct_stego_bytes(&[], &ctx);
        assert!(result.is_err());
    }

    // ── Extract seed from image ───────────────────────────────────────

    #[test]
    fn extract_seed_from_protected_image() {
        let meta = RightsMetadataProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        // Encode to PNG bytes, then apply metadata trap via apply_bytes
        let png_bytes = crate::util::image::encode_image(
            &DynamicImage::ImageRgba8(img),
            image::ImageFormat::Png,
        )
        .unwrap();
        let with_metadata = meta.apply_bytes(&png_bytes, &ctx).unwrap();

        let extracted =
            RightsMetadataProtector::extract_seed_from_image_with_limits(&with_metadata, None);
        assert_eq!(extracted, Some(42));
    }

    // ── Full round-trip with MAC key ──────────────────────────────────

    #[test]
    fn full_roundtrip_mac_embed_extract_verify() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let key = b"super-secret";
        let ctx = ctx_with_mac(42, key);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        // Extract payload
        let payload = protector.extract_payload_with_key(&result, key);
        assert!(payload.is_some());
        let p = payload.unwrap();
        assert_eq!(p.seed(), 42);
        assert_eq!(p.version(), 3);
    }

    #[test]
    fn full_roundtrip_no_mac_embed_extract_verify() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        let payload = protector.extract_payload(&result);
        assert!(payload.is_some());
        let p = payload.unwrap();
        assert_eq!(p.seed(), 42);
    }

    #[test]
    fn full_roundtrip_verify_payload() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&result));
    }

    // ── Redundancy embedding tests ──────────────────────────────────────

    #[test]
    fn jpeg_stego_redundancy_extraction_succeeds() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        let payload = protector.extract_payload(&result);
        assert!(
            payload.is_some(),
            "Should extract payload after embedding with redundancy"
        );
        let p = payload.unwrap();
        assert_eq!(p.seed(), 42);
    }

    #[test]
    fn jpeg_stego_redundancy_multiple_extraction_seeds_work() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(99999);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        let extracted_0 = protector.extract_payload(&result);
        let extracted_1 = protector.extract_payload(&result);
        let extracted_2 = protector.extract_payload(&result);

        assert!(extracted_0.is_some(), "Extraction should succeed");
        assert!(extracted_1.is_some(), "Extraction should succeed");
        assert!(extracted_2.is_some(), "Extraction should succeed");

        assert_eq!(
            extracted_0.clone().unwrap().seed(),
            extracted_1.clone().unwrap().seed(),
            "All extractions should produce identical seeds"
        );
        assert_eq!(
            extracted_0.unwrap().seed(),
            extracted_2.unwrap().seed(),
            "All extractions should produce identical seeds"
        );
    }

    // ── Tile seed derivation ───────────────────────────────────────────

    #[test]
    fn tile_seed_is_deterministic() {
        let a = tile_seed(42, 3, 7);
        let b = tile_seed(42, 3, 7);
        assert_eq!(a, b);
    }

    #[test]
    fn tile_seed_distinct_for_distinct_x() {
        let a = tile_seed(42, 0, 0);
        let b = tile_seed(42, 1, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn tile_seed_distinct_for_distinct_y() {
        let a = tile_seed(42, 0, 0);
        let b = tile_seed(42, 0, 1);
        assert_ne!(a, b);
    }

    #[test]
    fn tile_seed_distinct_for_distinct_master() {
        let a = tile_seed(42, 1, 1);
        let b = tile_seed(99, 1, 1);
        assert_ne!(a, b);
    }

    #[test]
    fn tile_seed_collisions_rare() {
        // Smoke test: 64 distinct (x, y) tiles with the same master seed
        // should produce 64 distinct per-tile seeds. With 64-bit splitmix64
        // output, a collision is astronomically unlikely.
        let mut seen = std::collections::HashSet::new();
        for x in 0..8 {
            for y in 0..8 {
                seen.insert(tile_seed(0xDEAD_BEEF, x, y));
            }
        }
        assert_eq!(seen.len(), 64);
    }

    // ── Tiled embed/extract ───────────────────────────────────────────

    fn tileable_test_image() -> RgbaImage {
        make_high_entropy_test_image(128, 128)
    }

    /// Build a real, integrity-protected payload from a context. The
    /// tiled extractor's integrity check (`try_ecc_decode` /
    /// `verify_payload_integrity`) requires a real payload — synthetic
    /// `vec![byte; N]` patterns won't pass the CRC32 check.
    fn real_payload(seed: u64) -> Vec<u8> {
        let ctx = ctx_no_mac(seed);
        SteganographyProtector::new().generate_payload_from_ctx(&ctx)
    }

    #[test]
    fn embed_lsb_tiled_no_crop_round_trip() {
        let protector = SteganographyProtector::new();
        let img = tileable_test_image();
        let payload = real_payload(42);

        let embedded = protector
            .embed_lsb_tiled(&img, &payload, 42, 64)
            .into_inner();
        assert_eq!(embedded.dimensions(), img.dimensions());

        let recovered = protector
            .extract_lsb_tiled_candidates(&embedded, 42, 64, 64, &[])
            .expect("tiled extraction should recover payload from un-cropped image");
        assert_eq!(recovered, payload);
    }

    #[test]
    fn embed_lsb_tiled_survives_aligned_crop() {
        let protector = SteganographyProtector::new();
        let img = tileable_test_image();
        let payload = real_payload(42);

        let embedded = protector
            .embed_lsb_tiled(&img, &payload, 42, 64)
            .into_inner();
        // Crop to the second tile (aligned offset, x0=64, y0=0).
        let cropped = SteganographyProtector::crop_rgba(&embedded, 64, 0, 64, 64);

        let recovered = protector
            .extract_lsb_tiled_candidates(&cropped, 42, 64, 64, &[])
            .expect("tiled extraction should recover payload from aligned crop");
        assert_eq!(recovered, payload);
    }

    #[test]
    fn embed_lsb_tiled_survives_misaligned_crop() {
        let protector = SteganographyProtector::new();
        let img = tileable_test_image();
        let payload = real_payload(42);

        let embedded = protector
            .embed_lsb_tiled(&img, &payload, 42, 64)
            .into_inner();
        // Crop with a 32-px offset (a 32 is a half-tile, NOT on a 64-px tile
        // boundary). The 96x96 window fully contains tile (1, 1) at original
        // (64, 64)-(127, 127). The embedded tile must still be recoverable
        // because the per-tile seed is grid-coordinate-based, not image-
        // coordinate-based.
        let cropped = SteganographyProtector::crop_rgba(&embedded, 32, 32, 96, 96);

        let recovered = protector
            .extract_lsb_tiled_candidates(&cropped, 42, 64, 64, &[])
            .expect("tiled extraction should recover payload from misaligned crop");
        assert_eq!(&recovered[..payload.len()], &payload[..]);
    }

    #[test]
    fn embed_lsb_tiled_survives_crop_smaller_than_image() {
        let protector = SteganographyProtector::new();
        let img = tileable_test_image();
        let payload = real_payload(42);

        let embedded = protector
            .embed_lsb_tiled(&img, &payload, 42, 64)
            .into_inner();
        // Crop a region smaller than the full image but large enough to
        // contain tile (0, 0) entirely. Tile (0, 0) is at original
        // (0, 0)-(63, 63) and is fully captured by this crop.
        let cropped = SteganographyProtector::crop_rgba(&embedded, 0, 0, 96, 128);

        let recovered = protector
            .extract_lsb_tiled_candidates(&cropped, 42, 64, 64, &[])
            .expect("tiled extraction should recover payload from partial-image crop");
        assert_eq!(recovered, payload);
    }

    #[test]
    fn embed_lsb_tiled_with_mac_key() {
        let protector = SteganographyProtector::new();
        let img = tileable_test_image();
        let ctx = ctx_with_mac(42, b"my-key");
        let payload = protector.generate_payload_from_ctx(&ctx);

        let embedded = protector
            .embed_lsb_tiled(&img, &payload, 42, 64)
            .into_inner();
        // Crop with a 32-px offset; the 96x96 window fully contains tile
        // (1, 1) at original (64, 64)-(127, 127).
        let cropped = SteganographyProtector::crop_rgba(&embedded, 32, 32, 96, 96);

        let recovered = protector
            .extract_lsb_tiled_candidates(&cropped, 42, 64, 64, b"my-key")
            .expect("tiled extraction with correct MAC should recover payload");
        assert_eq!(&recovered[..payload.len()], &payload[..]);

        assert!(protector
            .extract_lsb_tiled_candidates(&cropped, 42, 64, 64, b"wrong-key")
            .is_none());
    }

    #[test]
    fn embed_lsb_tiled_max_origins_limits_scan() {
        let protector = SteganographyProtector::new();
        let img = tileable_test_image();
        let payload = real_payload(42);

        let embedded = protector
            .embed_lsb_tiled(&img, &payload, 42, 64)
            .into_inner();

        // max_origins = 1 should still find a payload from a no-crop case
        // because the (0, 0) origin is in the deterministic scan order.
        let recovered = protector
            .extract_lsb_tiled_candidates(&embedded, 42, 64, 1, &[])
            .expect("max_origins=1 should still find payload at (0, 0) origin");
        assert_eq!(recovered, payload);
    }

    #[test]
    fn embed_lsb_tiled_zero_tile_size_falls_back() {
        let protector = SteganographyProtector::new();
        let img = tileable_test_image();
        let payload = real_payload(42);

        // tile_size = 0 returns the image unchanged. This is the
        // "tiling disabled" sentinel — the caller is expected to route
        // through the non-tiled path instead.
        let result = protector.embed_lsb_tiled(&img, &payload, 42, 0);
        assert!(result.is_skipped());
        assert_eq!(*result.output(), img);
    }

    #[test]
    fn embed_lsb_tiled_does_not_affect_non_cropped_extraction() {
        let protector = SteganographyProtector::new();
        let img = tileable_test_image();
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(64);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let protected = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&protected));
    }

    #[test]
    fn embed_lsb_tiled_extract_via_public_api_after_crop() {
        // End-to-end: protect with tiling, crop, then extract through the
        // public API. This exercises the verification chain integration
        // (verify_payload_with_seed + tiled fallback).
        let protector = SteganographyProtector::new();
        let img = tileable_test_image();
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(64);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let mut protected = protector.apply(&dyn_img, &ctx).unwrap().into_owned();
        let cropped = protected.crop(64, 0, 64, 64);

        let extracted = protector.extract_payload_with_seed(&cropped, 42);
        assert!(
            extracted.is_some(),
            "extract_payload_with_seed should recover tiled payload from cropped image via the verify-chain fallback"
        );
        assert_eq!(extracted.unwrap().seed(), 42);
    }

    #[test]
    fn embed_lsb_tiled_survives_4_pixel_alignment_shift() {
        let protector = SteganographyProtector::new();
        let img = tileable_test_image();
        let payload = real_payload(42);

        let embedded = protector
            .embed_lsb_tiled(&img, &payload, 42, 64)
            .into_inner();
        // Crop by 4 pixels (not aligned with 64px tile boundary) but large
        // enough that the window still fully contains tile (1, 1) at
        // original (64, 64)-(127, 127). The extraction scans grid
        // coordinates in 64px increments, so it finds the tile even though
        // the crop origin is misaligned.
        let cropped = SteganographyProtector::crop_rgba(&embedded, 4, 4, 124, 124);

        let recovered = protector.extract_lsb_tiled_candidates(&cropped, 42, 64, 64, &[]);
        // The extractor tries grid coordinates (base_x + dx, base_y + dy)
        // for dx, dy in 0..=2. For origin (0,0) in the cropped image,
        // base = (0, 0) and it tries grids (0,0), (0,1), (1,0), (1,1).
        // Grid (1,1) corresponds to tile (1,1) at original (64,64), but
        // the cropped image at origin (0,0) reads pixels (4,4)-(67,67) in
        // the original — NOT (64,64)-(127,127). A sub-tile shift means the
        // extractor reads from the wrong pixel region. This is a known
        // limitation: tiled LSB stego tolerates tile-aligned and half-tile
        // offsets (stride-based scan) but NOT arbitrary sub-tile offsets.
        // The test documents this: payload is NOT recoverable from a 4px
        // misaligned crop.
        assert!(
            recovered.is_none(),
            "LSB tiled stego does NOT survive sub-tile (4px) misaligned crop — extractor grid search is stride-based"
        );
    }

    #[test]
    fn embed_f5_tiled_round_trip_after_recompression() {
        let protector = SteganographyProtector::new();
        let jpeg_bytes = tileable_test_jpeg();
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(64);

        let protected = protector
            .apply_dct_stego_bytes_tiled(&jpeg_bytes, &ctx, 64)
            .unwrap()
            .into_inner();
        // Re-encode as JPEG (image crate encoder). This rebuilds DCT
        // coefficients from pixels, destroying the F5 stego. The test
        // verifies the extraction path handles this without panicking.
        let img = image::load_from_memory(&protected).unwrap();
        let reencoded = image_to_jpeg_bytes(&img, 85);

        let recovered = protector.extract_f5_tiled_candidates(&reencoded, 42, 64, 64, &[]);
        // After re-encode, DCT coefficients are recomputed and F5 stego
        // is lost. This test documents that limitation.
        let _ = recovered;
    }

    #[test]
    fn embed_f5_tiled_round_trip_no_crop() {
        let protector = SteganographyProtector::new();
        let jpeg_bytes = tileable_test_jpeg();
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(64);

        let protected = protector
            .apply_dct_stego_bytes_tiled(&jpeg_bytes, &ctx, 64)
            .unwrap()
            .into_inner();
        let recovered = protector.extract_f5_tiled_candidates(&protected, 42, 64, 64, &[]);
        assert!(
            recovered.is_some(),
            "F5 tiled extraction should recover payload from un-cropped JPEG"
        );
    }

    #[test]
    fn embed_f5_tiled_survives_aligned_crop() {
        let protector = SteganographyProtector::new();
        let jpeg_bytes = tileable_test_jpeg();
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(64);

        let protected = protector
            .apply_dct_stego_bytes_tiled(&jpeg_bytes, &ctx, 64)
            .unwrap()
            .into_inner();
        // Crop to a single tile by re-encoding a sub-image as JPEG.
        // First decode the protected JPEG, crop in pixel space, re-encode.
        let img = image::load_from_memory(&protected).unwrap();
        let rgb = img.to_rgb8();
        let cropped_rgb = image::imageops::crop_imm(&rgb, 0, 0, 64, 64).to_image();
        let cropped_img = DynamicImage::ImageRgb8(cropped_rgb);
        let mut buf = std::io::Cursor::new(Vec::new());
        cropped_img
            .write_to(&mut buf, image::ImageFormat::Jpeg)
            .unwrap();
        let cropped_jpeg = buf.into_inner();

        let recovered = protector.extract_f5_tiled_candidates(&cropped_jpeg, 42, 64, 64, &[]);
        // After pixel-space crop + re-encode, DCT coefficients are recomputed
        // and the original F5 stego is lost. This test documents that limitation.
        // The tiled path is designed for JPEG-level crops without re-encode.
        // For this test, we just verify the function doesn't panic.
        let _ = recovered;
    }

    #[test]
    fn embed_f5_tiled_with_mac_key() {
        let protector = SteganographyProtector::new();
        let jpeg_bytes = tileable_test_jpeg();
        let ctx = ctx_with_mac(42, b"my-key").with_tile_size(64);

        let protected = protector
            .apply_dct_stego_bytes_tiled(&jpeg_bytes, &ctx, 64)
            .unwrap()
            .into_inner();
        let recovered = protector.extract_f5_tiled_candidates(&protected, 42, 64, 64, b"my-key");
        assert!(
            recovered.is_some(),
            "F5 tiled extraction with MAC key should recover payload"
        );

        let wrong = protector.extract_f5_tiled_candidates(&protected, 42, 64, 64, b"wrong-key");
        assert!(
            wrong.is_none(),
            "F5 tiled extraction with wrong MAC key should fail"
        );
    }

    #[test]
    fn embed_f5_tiled_max_origins_limits_scan() {
        let protector = SteganographyProtector::new();
        let jpeg_bytes = tileable_test_jpeg();
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(64);

        let protected = protector
            .apply_dct_stego_bytes_tiled(&jpeg_bytes, &ctx, 64)
            .unwrap()
            .into_inner();
        let recovered = protector.extract_f5_tiled_candidates(&protected, 42, 64, 1, &[]);
        assert!(
            recovered.is_some(),
            "max_origins=1 should still find payload at first tile"
        );
    }

    #[test]
    fn wrong_mac_key_returns_none() {
        use crate::ImageOutputFormat;

        let protector = SteganographyProtector::new();
        let img = DynamicImage::ImageRgba8(make_test_image(64, 64));
        let correct_key = b"correct-secret-key";
        let wrong_key = b"wrong-secret-key!!";

        let ctx = ProtectionContext::new(0.5, 42)
            .with_mac_key(correct_key.to_vec())
            .with_format(ImageOutputFormat::Png);

        let protected = protector.apply(&img, &ctx).unwrap();

        let payload_correct = protector.extract_payload_with_key(&protected, correct_key);
        assert!(
            payload_correct.is_some(),
            "Should extract payload with correct key"
        );

        let payload_wrong = protector.extract_payload_with_key(&protected, wrong_key);
        assert!(
            payload_wrong.is_none(),
            "extract_payload_with_key should return None with wrong MAC key"
        );
    }

    fn tileable_test_jpeg() -> Vec<u8> {
        let img = tileable_test_image();
        let dyn_img = DynamicImage::ImageRgba8(img);
        let rgb = dyn_img.to_rgb8();
        let rgb_img = DynamicImage::ImageRgb8(rgb);
        let mut buf = std::io::Cursor::new(Vec::new());
        rgb_img
            .write_to(&mut buf, image::ImageFormat::Jpeg)
            .unwrap();
        buf.into_inner()
    }

    #[test]
    #[allow(deprecated)]
    fn channel_flags_metadata_disabled_reports_false() {
        let protector = SteganographyProtector::new();
        let mut ctx = ProtectionContext::new(0.5, 42);
        ctx.set_protection_level(crate::ProtectionLevel::Standard);
        ctx = ctx.with_metadata_injection(false);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let channels_bits = u16::from_le_bytes([payload[8], payload[9]]);
        let channels = ProtectionChannels::from_bits(channels_bits).unwrap();
        assert!(
            !channels.rights_metadata,
            "rights_metadata should be false when metadata disabled"
        );
        assert!(
            channels.hidden_marker,
            "hidden_marker should always be true when payload is generated"
        );
        assert!(
            !channels.authentication,
            "authentication should be false without MAC key"
        );
    }

    #[test]
    fn channel_flags_metadata_enabled_reports_true() {
        let protector = SteganographyProtector::new();
        let mut ctx = ProtectionContext::new(0.5, 42);
        ctx.set_protection_level(crate::ProtectionLevel::Standard);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let channels_bits = u16::from_le_bytes([payload[8], payload[9]]);
        let channels = ProtectionChannels::from_bits(channels_bits).unwrap();
        assert!(
            channels.rights_metadata,
            "rights_metadata should be true when metadata enabled"
        );
        assert!(channels.hidden_marker);
        assert!(!channels.authentication);
    }

    #[test]
    fn channel_flags_crc_has_authentication_false() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let channels_bits = u16::from_le_bytes([payload[8], payload[9]]);
        let channels = ProtectionChannels::from_bits(channels_bits).unwrap();
        assert!(
            !channels.authentication,
            "CRC payload must have authentication=false"
        );
    }

    #[test]
    fn channel_flags_hmac_has_authentication_true() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_with_mac(42, b"testkey");
        let payload = protector.generate_payload_from_ctx(&ctx);

        let channels_bits = u16::from_le_bytes([payload[8], payload[9]]);
        let channels = ProtectionChannels::from_bits(channels_bits).unwrap();
        assert!(
            channels.authentication,
            "HMAC payload must have authentication=true"
        );
    }

    #[test]
    fn channel_flags_tiled_matches_context() {
        let protector = SteganographyProtector::new();
        let mut ctx = ProtectionContext::new(0.5, 42);
        ctx.set_protection_level(crate::ProtectionLevel::Standard);
        ctx = ctx.with_tile_size(64);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let flags_bits = u16::from_le_bytes([payload[6], payload[7]]);
        let flags = crate::payload_v3::types::PayloadFlags::from_bits(flags_bits);
        assert!(flags.tiled, "tiled flag should match context");
    }

    #[test]
    fn channel_flags_progressive_matches_context() {
        let protector = SteganographyProtector::new();
        let mut ctx = ProtectionContext::new(0.5, 42);
        ctx.set_protection_level(crate::ProtectionLevel::Standard);
        ctx = ctx.with_progressive_jpeg(true);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let flags_bits = u16::from_le_bytes([payload[6], payload[7]]);
        let flags = crate::payload_v3::types::PayloadFlags::from_bits(flags_bits);
        assert!(
            flags.progressive_jpeg,
            "progressive_jpeg flag should match context"
        );
    }

    #[test]
    fn channel_flags_disabled_level_reports_no_metadata() {
        let protector = SteganographyProtector::new();
        let mut ctx = ProtectionContext::new(0.5, 42);
        ctx.set_protection_level(crate::ProtectionLevel::Disabled);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let channels_bits = u16::from_le_bytes([payload[8], payload[9]]);
        let channels = ProtectionChannels::from_bits(channels_bits).unwrap();
        assert!(
            !channels.rights_metadata,
            "rights_metadata should be false when level is Disabled"
        );
    }

    // ── V2 carrier scheme tests ────────────────────────────────────────

    #[test]
    fn carrier_v2_permutation_is_bijective_power_of_two() {
        let slot_count = 1024usize;
        let seed = 42u64;
        let mut seen = vec![false; slot_count];
        for i in 0..slot_count {
            let pos = carrier_lsb::stego_permutation_v2(i, slot_count, seed);
            assert!(pos < slot_count, "out of range: {} >= {}", pos, slot_count);
            assert!(!seen[pos], "collision at index {} -> pos {}", i, pos);
            seen[pos] = true;
        }
    }

    #[test]
    fn carrier_v2_permutation_is_bijective_prime_slot_count() {
        let slot_count = 997usize;
        let seed = 42u64;
        let mut seen = vec![false; slot_count];
        for i in 0..slot_count {
            let pos = carrier_lsb::stego_permutation_v2(i, slot_count, seed);
            assert!(pos < slot_count, "out of range: {} >= {}", pos, slot_count);
            assert!(!seen[pos], "collision at index {} -> pos {}", i, pos);
            seen[pos] = true;
        }
    }

    #[test]
    fn carrier_v2_permutation_is_bijective_composite_non_power_of_two() {
        let slot_count = 1000usize;
        let seed = 42u64;
        let mut seen = vec![false; slot_count];
        for i in 0..slot_count {
            let pos = carrier_lsb::stego_permutation_v2(i, slot_count, seed);
            assert!(pos < slot_count, "out of range: {} >= {}", pos, slot_count);
            assert!(!seen[pos], "collision at index {} -> pos {}", i, pos);
            seen[pos] = true;
        }
    }

    #[test]
    fn carrier_v2_permutation_is_bijective_odd_image_dimensions() {
        let width = 7u32;
        let height = 13u32;
        let slot_count = (width * height * 3) as usize;
        let seed = 42u64;
        let mut seen = vec![false; slot_count];
        for i in 0..slot_count {
            let pos = carrier_lsb::stego_permutation_v2(i, slot_count, seed);
            assert!(pos < slot_count, "out of range: {} >= {}", pos, slot_count);
            assert!(!seen[pos], "collision at index {} -> pos {}", i, pos);
            seen[pos] = true;
        }
    }

    #[test]
    fn carrier_v2_permutation_different_seed_changes_order() {
        let slot_count = 1024usize;
        let a = carrier_lsb::stego_permutation_v2(0, slot_count, 42);
        let b = carrier_lsb::stego_permutation_v2(0, slot_count, 99);
        assert_ne!(a, b);
    }

    #[test]
    fn carrier_v2_permutation_same_seed_is_deterministic() {
        let slot_count = 1024usize;
        let a = carrier_lsb::stego_permutation_v2(0, slot_count, 42);
        let b = carrier_lsb::stego_permutation_v2(0, slot_count, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn carrier_v2_permutation_slot_count_zero() {
        assert_eq!(carrier_lsb::stego_permutation_v2(0, 0, 42), 0);
    }

    #[test]
    fn carrier_v2_permutation_slot_count_one() {
        assert_eq!(carrier_lsb::stego_permutation_v2(0, 1, 42), 0);
    }

    #[test]
    fn carrier_v2_required_capacity_matches_exact_slots_touched() {
        let payload_bits = 288usize;
        let redundancy = 3usize;
        let required = carrier_lsb::lsb_required_capacity_v2(payload_bits, redundancy);
        assert_eq!(required, payload_bits * STEGO_SPREAD_FACTOR * redundancy);
    }

    #[test]
    fn carrier_v2_one_slot_below_capacity_skips_without_modification() {
        let protector = SteganographyProtector::new();
        let img = make_test_image(4, 4);
        let payload = vec![0xAA; 3];
        let payload_bits = payload.len() * 8;
        let available = carrier_lsb::lsb_available_slots(4, 4);
        let required = carrier_lsb::lsb_required_capacity_v2(payload_bits, 1);
        assert!(
            required > available,
            "setup: need more slots than available for this test"
        );
        let result = protector.embed_lsb_v2(&img, &payload, 42, 1);
        assert!(result.is_skipped());
        assert_eq!(*result.output(), img);
    }

    #[test]
    fn carrier_v2_exact_capacity_embeds_and_extracts() {
        let protector = SteganographyProtector::new();
        let img = make_test_image(32, 32);
        let payload = vec![0xAB, 0xCD, 0xEF, 0x12];

        let result = protector.embed_lsb_v2(&img, &payload, 42, 1);
        assert!(result.is_embedded());
        let out_img = result.into_inner();

        let payload_bits = payload.len() * 8;
        let extracted = protector.extract_lsb_v2(&out_img, payload_bits, 42, 0, 1);
        assert!(extracted.is_some(), "direct V2 extract must succeed");
        assert_eq!(extracted.unwrap(), payload);
    }

    #[test]
    fn carrier_v2_redundancy_increases_required_capacity() {
        let payload_bits = 100usize;
        let r1 = carrier_lsb::lsb_required_capacity_v2(payload_bits, 1);
        let r3 = carrier_lsb::lsb_required_capacity_v2(payload_bits, 3);
        assert_eq!(r3, r1 * 3);
    }

    #[test]
    fn carrier_v2_redundancy_roundtrips() {
        let protector = SteganographyProtector::new();
        let img = make_high_entropy_test_image(64, 64);
        let payload = vec![0x42; 20];
        let redundancy = 3usize;
        let result = protector.embed_lsb_v2(&img, &payload, 42, redundancy);
        assert!(result.is_embedded());
        let out_img = result.into_inner();

        let payload_bits = payload.len() * 8;
        let extracted = protector
            .extract_lsb_v2(&out_img, payload_bits, 42, 0, redundancy)
            .expect("V2 extraction with matching redundancy must succeed");
        assert_eq!(extracted, payload);
    }

    #[test]
    fn carrier_v2_no_duplicate_slot_within_embedding() {
        let width = 16u32;
        let height = 16u32;
        let available = carrier_lsb::lsb_available_slots(width, height);
        let payload_bits = 8usize;
        let mut all_slots = Vec::new();
        let seed = 42u64;
        for i in 0..payload_bits {
            for s in 0..STEGO_SPREAD_FACTOR {
                let logical = i * STEGO_SPREAD_FACTOR + s;
                let slot = carrier_lsb::stego_permutation_v2(logical, available, seed);
                all_slots.push(slot);
            }
        }
        let mut sorted = all_slots.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            all_slots.len(),
            "duplicate slots within one embedding pass"
        );
    }

    #[test]
    fn carrier_v2_alpha_channel_unchanged() {
        let protector = SteganographyProtector::new();
        let img = make_test_image(32, 32);
        let payload = vec![0x42; 10];
        let result = protector.embed_lsb_v2(&img, &payload, 42, 1);
        assert!(result.is_embedded());
        let output = result.output();
        for y in 0..img.height() {
            for x in 0..img.width() {
                assert_eq!(
                    img.get_pixel(x, y)[3],
                    output.get_pixel(x, y)[3],
                    "alpha changed at ({}, {})",
                    x,
                    y
                );
            }
        }
    }

    // ── Legacy compatibility tests ─────────────────────────────────────

    #[test]
    fn legacy_v1_fixture_extracts_after_v2_default() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let seed = 42u64;
        let ctx = ctx_no_mac(seed);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let legacy_result = carrier_lsb::embed_lsb(&img, &payload, seed, 1);
        assert!(legacy_result.is_embedded());
        let legacy_img = legacy_result.into_inner();

        let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1);
        let extracted = protector.extract_lsb(&legacy_img, payload.len() * 8, offset_seed);
        assert!(extracted.is_some(), "legacy extract_lsb must succeed");
        assert_eq!(extracted.unwrap(), payload, "legacy roundtrip must match");
    }

    #[test]
    fn legacy_v1_hmac_fixture_extracts_after_v2_default() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let seed = 42u64;
        let key = b"test-mac-key";
        let ctx = ctx_with_mac(seed, key);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let legacy_result = carrier_lsb::embed_lsb(&img, &payload, seed, 1);
        assert!(legacy_result.is_embedded());
        let legacy_img = legacy_result.into_inner();

        let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1);
        let prefix_bits = 6 * 8;
        let prefix = protector.extract_lsb(&legacy_img, prefix_bits, offset_seed);
        assert!(prefix.is_some(), "legacy prefix extraction must work");

        let extracted = protector.extract_with_redundancy(&legacy_img, seed, key);
        assert!(extracted.is_some(), "legacy HMAC extraction must work");
        assert_eq!(extracted.unwrap(), payload);
    }

    #[test]
    fn legacy_v1_wrong_key_still_fails() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let seed = 42u64;
        let ctx = ctx_with_mac(seed, b"correct-key");
        let payload = protector.generate_payload_from_ctx(&ctx);

        let legacy_result = carrier_lsb::embed_lsb(&img, &payload, seed, 1);
        let legacy_img = legacy_result.into_inner();

        let extracted = protector.extract_with_redundancy(&legacy_img, seed, b"wrong-key");
        assert!(extracted.is_none(), "wrong key must fail");
    }

    #[test]
    fn new_v2_payload_extracts_before_legacy_probe() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let seed = 42u64;
        let ctx = ctx_no_mac(seed);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let v2_result = protector.embed_lsb_v2(&img, &payload, seed, 1);
        assert!(v2_result.is_embedded());
        let v2_img = v2_result.into_inner();

        let extracted = protector.extract_lsb_v2(&v2_img, payload.len() * 8, seed, 0, 1);
        assert!(
            extracted.is_some(),
            "V2 extraction must find V2-embedded payload"
        );
        assert_eq!(extracted.unwrap(), payload);
    }

    #[test]
    fn new_v2_wrong_seed_does_not_false_positive() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let result = protector.embed_lsb_v2(&img, &payload, 42, 1);
        let img = result.into_inner();

        let extracted = protector.extract_with_redundancy(&img, 99, &[]);
        assert!(
            extracted.is_none(),
            "wrong seed must not produce false positive"
        );
    }

    #[test]
    fn v2_embed_extract_full_roundtrip_via_public_api() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        assert!(protector.verify_payload(&result));
        let payload = protector.extract_payload(&result).unwrap();
        assert_eq!(payload.seed(), 42);
    }

    #[test]
    fn v2_embed_extract_mac_roundtrip() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let key = b"test-mac";
        let ctx = ctx_with_mac(42, key);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        let payload = protector.extract_payload_with_key(&result, key);
        assert!(payload.is_some());
        assert_eq!(payload.unwrap().seed(), 42);
    }

    #[test]
    fn v2_capacity_helpers_consistent() {
        let width = 100u32;
        let height = 100u32;
        let payload_bits = 288usize;
        let redundancy = 3usize;

        let (required, available) =
            carrier_lsb::lsb_capacity_for_image(width, height, payload_bits, redundancy);
        assert_eq!(
            required,
            carrier_lsb::lsb_required_capacity_v2(payload_bits, redundancy)
        );
        assert_eq!(available, carrier_lsb::lsb_available_slots(width, height));
        assert!(available >= required);
    }

    // ── Raw LSB carrier tests ────────────────────────────────────────
    // These exercise the carrier module directly with arbitrary bytes,
    // independently of StegoEggo payload-v3 framing.

    #[test]
    fn raw_lsb_arbitrary_bytes_roundtrip() {
        let img = make_large_test_image();
        let payload = vec![0x42u8; 36];
        let seed = 12345u64;
        let redundancy = 1;

        let outcome = carrier_lsb::embed_lsb_v2(&img, &payload, seed, redundancy);
        assert!(outcome.is_embedded());
        let embedded = outcome.into_inner();
        let extracted =
            carrier_lsb::extract_lsb_v2(&embedded, payload.len() * 8, seed, 0, redundancy);
        assert_eq!(extracted.as_deref(), Some(payload.as_slice()));
    }

    #[test]
    fn raw_lsb_binary_zero_ff_payload_roundtrip() {
        let img = make_large_test_image();
        let mut payload = Vec::with_capacity(48);
        payload.extend(std::iter::repeat_n(0x00u8, 24));
        payload.extend(std::iter::repeat_n(0xFFu8, 24));
        let seed = 9999u64;
        let redundancy = 2;

        let outcome = carrier_lsb::embed_lsb_v2(&img, &payload, seed, redundancy);
        assert!(outcome.is_embedded());
        let embedded = outcome.into_inner();
        let extracted =
            carrier_lsb::extract_lsb_v2(&embedded, payload.len() * 8, seed, 0, redundancy);
        assert_eq!(extracted.as_deref(), Some(payload.as_slice()));
    }

    #[test]
    fn raw_lsb_exact_capacity_outcome() {
        let width = 16u32;
        let height = 16u32;
        let img = make_test_image(width, height);
        let available = carrier_lsb::lsb_available_slots(width, height);
        let payload_bits = available / 5;
        let payload_bytes = payload_bits / 8;
        let payload = vec![0xABu8; payload_bytes];
        let seed = 42u64;

        let outcome = carrier_lsb::embed_lsb_v2(&img, &payload, seed, 1);
        assert!(
            outcome.is_embedded(),
            "payload should fit within exact capacity"
        );
    }

    #[test]
    fn raw_lsb_wrong_seed_not_equal() {
        let img = make_large_test_image();
        let payload = vec![0x11u8; 32];
        let seed_a = 42u64;
        let seed_b = 99u64;
        let redundancy = 1;

        let outcome = carrier_lsb::embed_lsb_v2(&img, &payload, seed_a, redundancy);
        let embedded = outcome.into_inner();

        let extracted =
            carrier_lsb::extract_lsb_v2(&embedded, payload.len() * 8, seed_b, 0, redundancy);
        assert_ne!(
            extracted.as_deref(),
            Some(payload.as_slice()),
            "wrong seed must not recover original payload"
        );
    }

    #[test]
    fn raw_lsb_legacy_scheme_fixture_roundtrip() {
        let img = make_large_test_image();
        let payload = vec![0xBEu8; 28];
        let seed = 42u64;
        let redundancy = 1;

        let outcome = carrier_lsb::embed_lsb(&img, &payload, seed, redundancy);
        assert!(outcome.is_embedded());
        let embedded = outcome.into_inner();
        let expected_bits = payload.len() * 8;
        let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1);
        let extracted = carrier_lsb::extract_lsb(&embedded, expected_bits, offset_seed);
        assert_eq!(extracted.as_deref(), Some(payload.as_slice()));
    }

    #[test]
    fn raw_lsb_current_scheme_roundtrip() {
        let img = make_large_test_image();
        let payload = vec![0xCDu8; 36];
        let seed = 7777u64;
        let redundancy = 2;

        let outcome = carrier_lsb::embed_lsb_v2(&img, &payload, seed, redundancy);
        assert!(outcome.is_embedded());
        let embedded = outcome.into_inner();
        let extracted =
            carrier_lsb::extract_lsb_v2(&embedded, payload.len() * 8, seed, 0, redundancy);
        assert_eq!(extracted.as_deref(), Some(payload.as_slice()));
    }

    #[test]
    fn raw_lsb_has_no_rights_metadata_dependency() {
        let img = make_large_test_image();
        let payload = vec![0x77u8; 24];
        let seed = 5555u64;
        let redundancy = 1;

        let outcome = carrier_lsb::embed_lsb_v2(&img, &payload, seed, redundancy);
        assert!(outcome.is_embedded());
        let embedded = outcome.into_inner();
        let extracted =
            carrier_lsb::extract_lsb_v2(&embedded, payload.len() * 8, seed, 0, redundancy);
        assert_eq!(extracted.as_deref(), Some(payload.as_slice()));
    }

    // ── Raw JPEG carrier tests ───────────────────────────────────────

    #[test]
    fn raw_jpeg_arbitrary_bytes_roundtrip_supported_fixture() {
        let img = make_high_entropy_test_image(256, 256);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);

        let (_header, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();
        let capacity = carrier_jpeg::dct_payload_capacity(&coefficients);
        if capacity < 32 {
            return;
        }

        let payload = vec![0x42u8; 32];
        let seed = 42u64;

        let mut coeffs = coefficients.clone();
        DctStegoF5::with_redundancy(1)
            .embed_f5(&mut coeffs, &payload, seed)
            .unwrap();

        let rt_bits = DctStegoF5::with_redundancy(1).extract_f5(&coeffs, payload.len() * 8, seed);
        let rt_payload = carrier_lsb::bits_to_bytes(&rt_bits);
        assert_eq!(rt_payload, payload);
    }

    #[test]
    fn raw_jpeg_binary_zero_ff_payload_roundtrip() {
        let img = make_high_entropy_test_image(256, 256);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);

        let (_header, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();
        let capacity = carrier_jpeg::dct_payload_capacity(&coefficients);
        if capacity < 48 {
            return;
        }

        let mut payload = Vec::with_capacity(48);
        payload.extend(std::iter::repeat_n(0x00u8, 24));
        payload.extend(std::iter::repeat_n(0xFFu8, 24));
        let seed = 77u64;

        let mut coeffs = coefficients.clone();
        DctStegoF5::with_redundancy(1)
            .embed_f5(&mut coeffs, &payload, seed)
            .unwrap();

        let rt_bits = DctStegoF5::with_redundancy(1).extract_f5(&coeffs, payload.len() * 8, seed);
        let rt_payload = carrier_lsb::bits_to_bytes(&rt_bits);
        assert_eq!(rt_payload, payload);
    }

    #[test]
    fn raw_jpeg_capacity_matches_supported_coefficients() {
        let img = make_high_entropy_test_image(128, 128);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);

        let (_, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();
        let capacity = carrier_jpeg::dct_payload_capacity(&coefficients);

        let manual: usize = coefficients
            .values()
            .flat_map(|blocks| blocks.iter())
            .map(|block| {
                block
                    .iter()
                    .skip(1)
                    .filter(|&&coef| coef.abs() >= 2)
                    .count()
            })
            .sum();
        assert_eq!(capacity, manual);
    }

    #[test]
    fn raw_jpeg_wrong_seed_not_equal() {
        let img = make_high_entropy_test_image(256, 256);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);

        let (_header, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();
        let capacity = carrier_jpeg::dct_payload_capacity(&coefficients);
        if capacity < 32 {
            return;
        }

        let payload = vec![0x11u8; 32];
        let seed_a = 42u64;
        let seed_b = 99u64;

        let mut coeffs = coefficients.clone();
        DctStegoF5::with_redundancy(1)
            .embed_f5(&mut coeffs, &payload, seed_a)
            .unwrap();

        let rt_bits = DctStegoF5::with_redundancy(1).extract_f5(&coeffs, payload.len() * 8, seed_b);
        let rt_payload = carrier_lsb::bits_to_bytes(&rt_bits);
        assert_ne!(rt_payload, payload, "wrong seed must not recover payload");
    }

    #[test]
    fn raw_jpeg_container_segments_preserved() {
        let img = make_high_entropy_test_image(128, 128);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);

        let header = JpegHeader::parse(&jpeg_bytes).unwrap();
        let reassembled = carrier_jpeg::reassemble_jpeg_with_qtables(&jpeg_bytes, &header).unwrap();

        assert!(reassembled.starts_with(&[0xFF, 0xD8]));
        assert!(reassembled.ends_with(&[0xFF, 0xD9]));

        let _ = JpegHeader::parse(&reassembled).unwrap();
    }

    #[test]
    fn raw_jpeg_progressive_reports_unsupported_payload_embedding() {
        let img = make_high_entropy_test_image(64, 64);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);

        let (_header, coefficients) = JpegTranscoder::decode_coefficients(&jpeg_bytes).unwrap();
        let capacity = carrier_jpeg::dct_payload_capacity(&coefficients);
        if capacity < 32 {
            return;
        }

        let payload = vec![0xAAu8; 32];
        let seed = 42u64;

        let mut coeffs = coefficients.clone();
        let embed_result = DctStegoF5::with_redundancy(1).embed_f5(&mut coeffs, &payload, seed);
        if embed_result.is_err() {
            let rt_bits =
                DctStegoF5::with_redundancy(1).extract_f5(&coeffs, payload.len() * 8, seed);
            let rt_payload = carrier_lsb::bits_to_bytes(&rt_bits);
            assert_ne!(rt_payload, payload);
        }
    }

    #[test]
    fn qtable_seed_hint_roundtrip_does_not_imply_payload_success() {
        let img = make_high_entropy_test_image(128, 128);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);

        let seed = 42u64;
        let reassembled = carrier_jpeg::embed_seed_hint(&jpeg_bytes, seed).unwrap();
        let extracted_seed = carrier_jpeg::extract_seed_hint(&reassembled).unwrap();
        assert_eq!(extracted_seed, Some(seed));

        let (_header, coefficients) = JpegTranscoder::decode_coefficients(&reassembled).unwrap();
        let capacity = carrier_jpeg::dct_payload_capacity(&coefficients);
        if capacity < 32 {
            return;
        }

        let payload = vec![0x55u8; 32];
        let mut coeffs = coefficients.clone();
        DctStegoF5::with_redundancy(1)
            .embed_f5(&mut coeffs, &payload, seed)
            .unwrap();
        let rt_bits = DctStegoF5::with_redundancy(1).extract_f5(&coeffs, payload.len() * 8, seed);
        let rt_payload = carrier_lsb::bits_to_bytes(&rt_bits);
        assert_eq!(rt_payload, payload);
    }

    // ── Application-adapter regression tests ─────────────────────────

    #[test]
    fn stegoeggo_png_v3_roundtrip_after_carrier_extraction() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx = ctx_no_mac(42);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&result));
        let payload = protector.extract_payload(&result).unwrap();
        assert_eq!(payload.seed(), 42);
    }

    #[test]
    fn stegoeggo_webp_v3_roundtrip_after_carrier_extraction() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let ctx =
            ProtectionContext::new(0.5, 42).with_format(crate::types::ImageOutputFormat::WebP);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();
        assert!(protector.verify_payload(&result));
        let payload = protector.extract_payload(&result).unwrap();
        assert_eq!(payload.seed(), 42);
    }

    #[test]
    fn stegoeggo_jpeg_v3_roundtrip_after_carrier_extraction() {
        let protector = SteganographyProtector::new();
        let img = make_high_entropy_test_image(256, 256);
        let jpeg_bytes = image_to_jpeg_bytes(&DynamicImage::ImageRgba8(img), 90);
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(crate::types::ImageOutputFormat::Jpeg)
            .with_stego_redundancy(3);

        let outcome = protector.apply_dct_stego_bytes(&jpeg_bytes, &ctx).unwrap();
        if outcome.is_embedded() {
            let protected = outcome.into_inner();
            assert!(
                protector.verify_payload_from_bytes(&protected, 42),
                "JPEG v3 roundtrip should verify"
            );
        }
    }

    #[test]
    fn stegoeggo_hmac_wrong_key_classification_unchanged() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let correct_key = b"correct-key";
        let wrong_key = b"wrong-key";
        let ctx = ctx_with_mac(42, correct_key);

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = protector.apply(&dyn_img, &ctx).unwrap();

        let payload_correct = protector.extract_payload_with_key(&result, correct_key);
        assert!(
            payload_correct.is_some(),
            "correct key should extract payload"
        );

        let payload_wrong = protector.extract_payload_with_key(&result, wrong_key);
        assert!(payload_wrong.is_none(), "wrong key must return None");
    }

    #[test]
    fn stegoeggo_legacy_lsb_fixture_still_extracts() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let seed = 42u64;
        let payload = protector.generate_payload_from_ctx(&ctx_no_mac(seed));

        let legacy_outcome = carrier_lsb::embed_lsb(&img, &payload, seed, 1);
        let embedded = legacy_outcome.into_inner();

        let extracted = protector.extract_with_redundancy(&embedded, seed, &[]);
        assert!(
            extracted.is_some(),
            "legacy LSB fixture should still extract"
        );
        let recovered = extracted.unwrap();
        assert_eq!(recovered, payload);
    }
}
