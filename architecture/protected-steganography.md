# Steganography Protector

**Source:** `src/protected/steganography.rs` (application adapter) + `src/stego/lsb.rs` + `src/stego/jpeg.rs` (generic carrier core)

The most complex module. Handles LSB and DCT-based steganographic embedding for payload storage and verification. `SteganographyProtector` generates StegoEggo payloads (v1/v2/v3) and manages seed discovery/verification; generic carrier mechanics (permutations, bit embedding/extraction, capacity calculation) are delegated to `src/stego/`.

## Payload Format

### V3 Header (32 bytes, current default)

```
Offset  Size  Field
0       2     Magic bytes ('S', 'E')
2       1     Version (=3)
3       1     Header length (includes extensions and key ID)
4       2     Total payload length
6       8     Seed (little-endian)
14      2     Intensity (0–10000, little-endian)
16      1     DMI policy byte
17      8     Content hash (truncated)
25      1     Key ID length (0–32)
26      1     Auth algorithm (0=CRC32, 1=HMAC-SHA256, 2=Ed25519)
27      1     Auth tag length
28      2     Flags
30      2     Reserved
```

V3 supports TLV extensions for additional metadata and optionally carries an Ed25519 signature or HMAC-SHA256 authentication tag.

### V2 Header (32 bytes, legacy, extraction only)

```
Offset  Size  Field
0       1     Version (=2)
1       1     ProtectionLevel byte (0/1/2)
2       8     Seed (u64, little-endian)
10      2     Intensity (u16, scaled f32 * 100.0)
12      8     Timestamp (u64, seconds since Unix epoch)
20      4     Content hash (truncated ISCC or SHA-256)
24      1     DMI value byte
25      1     Flags byte (reserved)
26      6     Reserved (zeroed)
```

### V1 Header (24 bytes, legacy, extraction only)

```
Offset  Size  Field
0       1     Version (=1)
1       1     ProtectionLevel byte
2       8     Seed (u64, little-endian)
10      2     Intensity (u16, scaled f32)
12      8     Timestamp (u64, seconds since epoch)
20      4     CRC32 checksum (or 8-byte HMAC with MAC key)
```

### Payload Sizes

- **V3 CRC (no MAC)**: 32-byte core + 4-byte CRC32 = 36 bytes total
- **V3 HMAC**: 32-byte core + 16-byte HMAC-SHA256 = 48 bytes total
- **V2 ECC (legacy)**: 32-byte header × 3 (ECC replication) + 4 CRC32 = 100 bytes
- **V1 ECC (legacy)**: 24-byte header × 3 + 4 CRC32 = 76 bytes
- **`MIN_PAYLOAD_SIZE = 28`**: Parsing threshold (24-byte V1 header + 4-byte CRC32), not the output size

## StegoPayload (Extracted)

```rust
pub struct StegoPayload { /* private fields */ }
```

Getter methods: `protection_level()`, `seed()`, `intensity()`, `version()`.

## Embedding Methods

### LSB Embedding (PNG/WebP)

```rust
fn embed_lsb_v2(img: &mut RgbaImage, payload: &[u8], seed: u64, redundancy: usize)
fn extract_lsb_v2(img: &RgbaImage, seed: u64, redundancy: usize) -> Vec<u8>
```

**Corrected V2 carrier model (current default):**

- Carrier domain: `width * height * 3` RGB slots (alpha never a carrier)
- Permutation: cycle-walking bijective LCG (`stego_permutation_v2`) over `[0, slot_count)`
- Each payload bit occupies `STEGO_SPREAD_FACTOR * redundancy` consecutive logical
  indices through one permutation — no inter-replica collisions
- Capacity formula: `payload_bits * STEGO_SPREAD_FACTOR * redundancy` slots exact
- Embed uses raw seed directly (no `STEGO_OFFSET_SEED_1` offset)

**Legacy carrier (backward-compatible extraction only):**

- Pixel-index carrier: `total_pixels` = `width * height`
- Channel derived from `bit_index % 3`
- Redundancy via multiple passes with offset seeds
- `extract_lsb` preserves the historical mapping exactly

**Extraction probing order:**

1. V2 corrected: raw seed, redundancy 1..=10
2. V2 legacy: offset seeds (`seed * (STEGO_OFFSET_SEED_1 + pass)`), redundancy 1..=10
3. Legacy carrier: offset seeds, legacy extraction

**WebP caveat:** LSB embedding survives **lossless** WebP round-trips (which is what `stegoeggo` produces via the `image` crate's `WebPEncoder::new_lossless`). Lossy WebP re-encoding (the common web delivery path) destroys the LSB payload. If WebP is the chosen delivery format, configure the CDN to deliver lossless WebP, or accept metadata-only protection.

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
- `probe_dct_support()` gates DCT entry: rejects progressive, restart-bearing, non-8-bit, multi-scan, and sampling >4 inputs; unsupported inputs fall back to metadata-only processing

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
- **src/stego/lsb.rs**: Generic LSB carrier mechanics (permutations, embed/extract, crop, seed fallback). No application-type imports
- **src/stego/jpeg.rs**: Generic JPEG carrier facade (DCT capacity, Q-table reassembly, seed hint). No application-type imports
- **jpeg_transcoder/**: Used for JPEG fast path (`apply_dct_stego_bytes`)
- **stego_f5.rs**: `DctStegoF5` for F5-style DCT manipulation
- **util/image.rs**: `XorShiftRng` for LSB pixel selection
- **protected/constants.rs**: `STEGO_OFFSET_SEED_1`, `STEGO_SPREAD_FACTOR`, etc.
- **types.rs**: Uses `ProtectionLevel`, `StegoPayload`

## Tiled Embedding (Crop Resistance)

Tiled mode embeds the full payload in each `tile_size × tile_size` pixel region
independently, so the payload survives arbitrary crops that leave at least one
intact tile.

### Configuration

```rust
let ctx = ProtectionContext::new(0.5, seed)
    .with_tile_size(64)        // 0 = disabled (default), 32..=1024
    .with_tile_extraction_max_origins(64);  // max candidate origins for extraction
```

When `tile_size > 0`, both LSB (PNG/WebP) and DCT (JPEG) paths use tiled
embedding. The tile grid is fixed; no state is shared between tiles.

### Per-Tile Seed Derivation

Each tile uses `tile_seed(master_seed, tile_x, tile_y)` — a splitmix64 hash
of the master seed mixed with the tile grid coordinate. The same tile coordinate
in any cropped image produces the same seed, so extraction is self-coordinating.

### LSB Tiled Path

- `embed_lsb_tiled`: clones the image, iterates tiles, embeds payload in each
  tile's pixel sub-region using `embed_lsb` with per-tile seed and redundancy 1.
- `extract_lsb_tiled_candidates`: scans candidate tile origins in the cropped
  image (stride = `tile_size / 2`, up to `max_origins`), tries grid coordinates
  around each origin, extracts and verifies integrity.

### F5 Tiled Path

- `apply_dct_stego_bytes_tiled`: iterates tile grid in DCT block space,
  embeds payload in each tile's blocks using `embed_f5_in_blocks` with per-tile
  seed.
- `extract_f5_tiled_candidates`: scans tile positions in the cropped JPEG's
  coefficient container, tries grid coordinates, extracts and verifies.

### Verification Chain Integration

Both tiled paths are wired as fallbacks in the existing verification chain:
- `extract_verified_dct_payload`: tries non-tiled first, then tiled F5 fallback
- `verify_dct_stego_with_seed`: tries non-tiled first, then tiled F5 fallback
- `verify_payload_with_seed`: tries non-tiled first, then tiled LSB fallback
- `extract_payload_with_seed` / `extract_payload_with_seed_and_key`: tiled LSB fallback

### Limitations

- **Crop + re-encode destroys DCT stego.** Tiled F5 only survives JPEG crops
  that preserve DCT coefficients (no re-encode). For re-encoded crops, the LSB
  tiled path (if output is PNG/WebP) is the recovery channel.
- **Capacity cost.** Each tile embeds the full payload (64× for a 64×64 grid).
  Tiled mode is opt-in via `with_tile_size(n)` because of this cost.
- **Extraction cost is O(K²).** For a 1024×1024 cropped image with
  `tile_min = 32`, up to ~1024 origins × 9 grid coords × 10 redundancies =
  ~92,160 extraction attempts. Early exit on first success keeps this practical.
