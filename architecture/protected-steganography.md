# Steganography Protector

**Source:** `src/protected/steganography.rs` (~1915 lines)

The most complex module. Handles LSB and DCT-based steganographic embedding for payload storage and verification.

## Payload Format (32 bytes total)

```
Offset  Size  Field
0       1     Version (currently 1)
1       1     ProtectionLevel byte
2       8     Seed (u64, little-endian)
10      2     Intensity (u16, scaled f32)
12      8     Timestamp (u64, seconds since epoch)
20      2     Additive checksum (without MAC key)
20      8     HMAC-SHA256 first 8 bytes (with MAC key)
```

Checksum is 2 bytes (first 16-bit additive checksum). HMAC is 8 bytes (first 8 bytes of HMAC-SHA256). Always padded to 32 bytes. `MIN_PAYLOAD_SIZE = 26` (24-byte header + 2-byte checksum), `MIN_PAYLOAD_BITS = 208`.

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
- Redundancy 1–5: multiple passes for reliability

### JPEG Pixel Stego

```rust
fn embed_jpeg_stego(img: &mut RgbaImage, payload: &[u8], seed: u64, redundancy: u8)
fn extract_jpeg_stego(img: &RgbaImage, seed: u64, redundancy: u8) -> Vec<u8>
```

- Amplitude-based embedding with block stride
- Uses `STEGO_JPEG_AMPLITUDE` (40), `STEGO_JPEG_SPREAD` (5), `STEGO_JPEG_BLOCK_STRIDE` (15)
- For JPEG images that go through the pixel path (non-baseline or format conversion)

### DCT Stego (JPEG Fast Path)

```rust
pub fn apply_dct_stego_bytes(jpeg_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>>
```

- For baseline JPEG: Full F5 embedding + seed in quantization tables
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
2. For JPEG: extract from DCT coefficients (F5) or quantization tables
3. For PNG/WebP: extract from pixel LSBs
4. Verify integrity: HMAC-SHA256 (with key) or additive checksum (without)
5. HMAC uses `subtle::ConstantTimeEq::ct_eq()` to prevent timing attacks

### Majority Voting

Extraction always runs 5 passes. Each pass uses different seed derivation. Results are combined via majority voting for robustness against noise.

## Redundancy

- Configurable 1–5 via `ProtectionContext::stego_redundancy`
- Embedding loops with `break` to exit inner loops after each pass
- Extraction always runs 5 passes regardless of redundancy setting

## Fallback Seeds

When metadata is stripped (seed unavailable), extraction tries `FALLBACK_SEEDS` — common test/dev seeds.

## Module Interactions

- **lib.rs**: Applied after perturbation in Standard/Enhanced/Strong pipelines
- **jpeg_transcoder/**: Used for JPEG fast path (`apply_dct_stego_bytes`)
- **stego_f5.rs**: `DctStegoF5` for F5-style DCT manipulation
- **util/image.rs**: `XorShiftRng` for LSB pixel selection
- **protected/constants.rs**: `STEGO_OFFSET_SEED_1`, `STEGO_JPEG_AMPLITUDE`, etc.
- **types.rs**: Uses `ProtectionLevel`, `StegoPayload`
