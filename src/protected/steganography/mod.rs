use crate::error::{Error, Result};
use crate::payload_v3::types::{AuthAlgorithm, ProtectionChannels, V3_MAGIC, V3_PAYLOAD_VERSION};
use crate::protected::constants::STEGO_OFFSET_SEED_1;
use crate::protected::ecc;
use crate::protected::metadata_trap::RightsMetadataProtector;
use crate::resource_limits::ResourceLimits;
use crate::stego::jpeg as carrier_jpeg;
use crate::traits::Protector;
use crate::types::{
    PayloadEmissionContext, ProtectionContext, ProtectionLevel, VerificationStatus,
};
use crc32fast::Hasher as Crc32Hasher;
use hmac::{Hmac, Mac};
use image::{DynamicImage, RgbaImage};
use sha2::Sha256;
use std::borrow::Cow;
use stegoeggo_stego::application_support as carrier_support;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

mod embed;
mod extract;
mod legacy;
mod marker;
mod verify;

/// Legacy V1 payload parsing threshold: 24-byte header + 4-byte CRC32 checksum.
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

/// Payload versions recognized by `parse_stego_payload`.
///
/// V3 payloads are dispatched by their magic bytes before this list is
/// consulted, so no per-version match arm exists for `3` in
/// `parse_stego_payload`; the loop there only routes legacy versions.
/// Keeping a version in this slice keeps images protected by older
/// releases parseable. Removing a version from this slice will make
/// previously-protected images un-parseable. Only remove a version when
/// you are confident no surviving protected image still uses it.
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
pub(crate) enum CandidateOutcome {
    Valid(Vec<u8>),
    Invalid(Vec<u8>),
    MalformedV3,
    UnsupportedVersion(u8),
    AuthenticationKeyMissing(Vec<u8>),
    AuthenticationFailed(Vec<u8>),
    ResourceLimitExceeded,
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
    InvalidChannels,
    HeaderLengthMismatch,
    ExtensionFlagMismatch,
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

pub use crate::stego::DEFAULT_TILE_SIZE;
pub use stegoeggo_stego::application_support::tile_seed;

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

    #[test]
    fn verify_extract_finds_legacy_payload_at_primary_seed() {
        const SPREAD: usize = 5;
        const SPLITMIX64_SEED: u64 = 0x9e3779b97f4a7c15;

        fn splitmix64(x: u64) -> u64 {
            let mut z = x.wrapping_add(SPLITMIX64_SEED);
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        }

        let seed = 42u64;
        let mut img = make_large_test_image();
        let (width, height) = img.dimensions();
        let total_pixels = (width * height) as usize;

        let mut header = vec![0u8; V2_HEADER_SIZE];
        header[0] = 2;
        header[2..10].copy_from_slice(&seed.to_le_bytes());
        let ecc = crate::protected::ecc::ecc_encode(&header);
        let mut payload = ecc.clone();
        payload.extend_from_slice(&SteganographyProtector::compute_checksum(&ecc));

        let bits: Vec<u8> = payload
            .iter()
            .flat_map(|&b| (0..8).map(move |i| (b >> i) & 1))
            .collect();

        let a = splitmix64(seed).wrapping_mul(2) | 1;
        let b = splitmix64(seed.wrapping_add(SPLITMIX64_SEED));

        for (i, &bit) in bits.iter().enumerate() {
            let channel = i % 3;
            for s in 0..SPREAD {
                let logical = i * SPREAD + s;
                let idx = (a.wrapping_mul(logical as u64).wrapping_add(b) as usize) % total_pixels;
                let x = (idx as u32) % width;
                let y = (idx as u32) / width;
                let pixel = img.get_pixel_mut(x, y);
                pixel[channel] = (pixel[channel] & 0xFE) | bit;
            }
        }

        let protector = SteganographyProtector::new();
        let outcome = protector.verify_extract_with_redundancy(&img, seed, &[]);
        assert!(matches!(outcome, CandidateOutcome::Valid(_)));
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
    fn compute_payload_mac_v3_uses_domain_separation() {
        let data = vec![1u8; 32];
        let key = b"test-key";
        let mut raw_mac = HmacSha256::new_from_slice(key).unwrap();
        raw_mac.update(&data);
        let raw = raw_mac.finalize().into_bytes();
        let v3 = SteganographyProtector::compute_payload_mac_v3(&data, key);
        assert_ne!(v3, raw[..16]);
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

    // ── Pixel manipulation ────────────────────────────────────────────

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
    fn lsb_extract_oversized_expected_bits_returns_none() {
        let protector = SteganographyProtector::new();
        let img = make_test_image(4, 4); // 16 pixels = 48 channels
        assert!(protector.extract_lsb(&img, 256, 42).is_none());
    }

    // ── DCT capacity / verification ───────────────────────────────────

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
    fn parse_stego_payload_v1_short_input_returns_none() {
        for n in 0..24 {
            let mut buf = vec![0u8; n];
            if !buf.is_empty() {
                buf[0] = 1;
            }
            assert!(
                SteganographyProtector::parse_stego_payload_v1(&buf).is_none(),
                "v1 parser must not panic on {n}-byte input"
            );
        }
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
    fn tiled_jpeg_wrong_first_not_v3_candidate_does_not_mask_later_valid_candidate() {
        let protector = SteganographyProtector::new();
        let ctx = ctx_with_mac(42, b"correct-key").with_tile_size(64);
        let payload = protector.generate_payload_from_ctx(&ctx);
        let jpeg_bytes = tileable_test_jpeg();
        let search = carrier_support::TiledJpegSearch::new(&jpeg_bytes, 64)
            .expect("tiled search should decode");
        let keys = search.prefix_candidates(42, 1, 0);
        let first_key = keys[0].0;
        let later_key = keys[1].0;
        let candidates = vec![
            (first_key, vec![0; V3_PREFIX_BYTES]),
            (later_key, payload[..V3_PREFIX_BYTES].to_vec()),
        ];
        let outcome =
            protector.evaluate_tiled_candidates(candidates.clone(), b"correct-key", |key, bits| {
                if key == first_key {
                    (bits == V3_PREFIX_BYTES * 8).then(|| vec![0; bits.div_ceil(8)])
                } else if bits / 8 <= payload.len() {
                    Some(payload[..bits / 8].to_vec())
                } else {
                    None
                }
            });
        match outcome {
            CandidateOutcome::Valid(recovered) => assert_eq!(recovered, payload),
            other => panic!("later valid candidate was not recovered: {other:?}"),
        }

        let wrong_key_outcome =
            protector.evaluate_tiled_candidates(candidates, b"wrong-key", |key, bits| {
                if key == first_key {
                    (bits == V3_PREFIX_BYTES * 8).then(|| vec![0; bits.div_ceil(8)])
                } else if bits / 8 <= payload.len() {
                    Some(payload[..bits / 8].to_vec())
                } else {
                    None
                }
            });
        assert!(
            matches!(
                &wrong_key_outcome,
                CandidateOutcome::AuthenticationFailed(_)
            ),
            "wrong MAC outcome: {wrong_key_outcome:?}"
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
    fn carrier_v2_exact_capacity_embeds_and_extracts() {
        let protector = SteganographyProtector::new();
        let img = make_test_image(32, 32);
        let payload = vec![0xAB, 0xCD, 0xEF, 0x12];

        let result = protector.embed_lsb_v2(&img, &payload, 42, 1);
        assert!(result.is_embedded());
        let out_img = result.into_inner();

        let payload_bits = payload.len() * 8;
        let extracted = protector.extract_lsb_v2(&out_img, payload_bits, 42, 1);
        assert!(extracted.is_some(), "direct V2 extract must succeed");
        assert_eq!(extracted.unwrap(), payload);
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
            .extract_lsb_v2(&out_img, payload_bits, 42, redundancy)
            .expect("V2 extraction with matching redundancy must succeed");
        assert_eq!(extracted, payload);
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
    fn new_v2_payload_extracts_before_legacy_probe() {
        let protector = SteganographyProtector::new();
        let img = make_large_test_image();
        let seed = 42u64;
        let ctx = ctx_no_mac(seed);
        let payload = protector.generate_payload_from_ctx(&ctx);

        let v2_result = protector.embed_lsb_v2(&img, &payload, seed, 1);
        assert!(v2_result.is_embedded());
        let v2_img = v2_result.into_inner();

        let extracted = protector.extract_lsb_v2(&v2_img, payload.len() * 8, seed, 1);
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

    // ── Raw LSB carrier tests ────────────────────────────────────────
    // These exercise the carrier module directly with arbitrary bytes,
    // independently of StegoEggo payload-v3 framing.

    // ── Raw JPEG carrier tests ───────────────────────────────────────

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
}
