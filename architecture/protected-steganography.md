# Steganography Protector

**Source:** `src/protected/steganography.rs` (~1915 lines)

The most complex module. Handles LSB and DCT-based steganographic embedding for payload storage and verification.

## Payload Format

```
Offset  Size  Field
0       1     Version (currently 1)
1       1     ProtectionLevel byte
2       8     Seed (u64, little-endian)
10      2     Intensity (u16, scaled f32)
12      8     Timestamp (u64, seconds since epoch)
20      4     CRC32 checksum (without MAC key, no ECC)
20      8     HMAC-SHA256 first 8 bytes (with MAC key)
```

Without a MAC key, the payload uses a 4-byte CRC32 checksum. With a MAC key, the 8 trailing bytes are a truncated HMAC-SHA256. `MIN_PAYLOAD_SIZE = 28` (24-byte header + 4-byte CRC32), `MIN_PAYLOAD_BITS = 224`. In non-MAC mode, `generate_payload()` produces an ECC-encoded payload of 76 bytes (24 bytes × 3 replication + 4 CRC32). In MAC mode, the payload is 32 bytes (24 header + 8 HMAC).

## StegoPayload (Extracted)

```rust
pub struct StegoPayload { /* private fields */ }
```

Getter methods: `protection_level()`, `seed()`, `intensity()`, `version()`.

## Embedding Methods

### LSB Embedding (PNG/WebP)

```rust
fn embed_lsb(img: &mut RgbaImage, payload: &[u8], seed: u64, redundancy: u8)
fn extract_lsb(img: &RgbaImage, seed: u64, redundancy: u8) -> Vec<u8>
```

- Uses collision-free LCG permutation (`stego_permutation`) for pixel selection
- Seed derivation: `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)` per pass
- Embeds payload bits into LSBs of selected pixels
- Redundancy 1–10: multiple passes for reliability

**WebP caveat:** LSB embedding survives **lossless** WebP round-trips (which is what `cloakrs` produces via the `image` crate's `WebPEncoder::new_lossless`). Lossy WebP re-encoding (the common web delivery path) destroys the LSB payload. If WebP is the chosen delivery format, configure the CDN to deliver lossless WebP, or accept metadata-only protection.

### JPEG Pixel Stego

Removed from the public pipeline. JPEG output now uses the DCT fast path and
quantization-table seed storage; there is no exposed pixel-domain JPEG fallback.

### DCT Stego (JPEG Fast Path)

```rust
pub fn apply_dct_stego_bytes(jpeg_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>>
```

- For baseline JPEG: F5 coefficient embedding + seed in quantization tables when those tables are preserved
- For progressive JPEG: Seed-in-Q-tables only (F5 not supported for progressive)
- Uses `JpegTranscoder` to decode/encode DCT coefficients
- Uses `DctStegoF5` for coefficient manipulation

## Extraction & Verification

```rust
pub fn extract_payload(&self, img: &DynamicImage) -> Option<StegoPayload>
pub fn verify_payload(&self, img: &DynamicImage) -> bool
pub fn verify_payload_with_key(&self, img: &DynamicImage, mac_key: &[u8]) -> Option<bool>
pub fn verify_payload_from_bytes(&self, img_bytes: &[u8], seed: u64) -> bool
pub fn verify_payload_from_bytes_with_key(&self, img_bytes: &[u8], mac_key: &[u8]) -> Option<bool>
```

### Verification Flow

1. Detect image format
2. For JPEG: detect the seed in quantization tables, then verify DCT payload integrity from coefficients when available
3. For PNG/WebP: extract from pixel LSBs
4. Verify integrity: HMAC-SHA256 (with key) or CRC32 checksum (without)
5. HMAC uses `subtle::ConstantTimeEq::ct_eq()` to prevent timing attacks

Seed detection is not the same as payload verification: a JPEG can expose its seed in quantization tables without a verifiable payload.

### Majority Voting

Extraction always runs 5 passes. Each pass uses different seed derivation. Results are combined via majority voting for robustness against noise.

## Redundancy

- Configurable 1–10 via `ProtectionContext::stego_redundancy` (clamped via `.with_stego_redundancy(n)`)
- Embedding loops with `break` to exit inner loops after each pass
- Extraction always runs 5 passes regardless of redundancy setting

## Fallback Seeds

When metadata is stripped (seed unavailable), extraction tries `FALLBACK_SEEDS` — common test/dev seeds.

## Module Interactions

- **lib.rs**: Applied in Standard pipeline
- **jpeg_transcoder/**: Used for JPEG fast path (`apply_dct_stego_bytes`)
- **stego_f5.rs**: `DctStegoF5` for F5-style DCT manipulation
- **util/image.rs**: `XorShiftRng` for LSB pixel selection
- **protected/constants.rs**: `STEGO_OFFSET_SEED_1`, `STEGO_SPREAD_FACTOR`, etc.
- **types.rs**: Uses `ProtectionLevel`, `StegoPayload`
