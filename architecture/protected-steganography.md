# Steganography Protector

**Source:** `src/protected/steganography/` (application adapter) + `stegoeggo-stego/src/{frame,lsb,jpeg}.rs` (public carrier API) + `stegoeggo-stego/src/lsb_internal.rs` (carrier mechanics) + `stegoeggo-stego/src/jpeg_transcoder/` (private JPEG mechanics)

The rights-aware hidden-marker adapter is split into five responsibility modules behind the `SteganographyProtector` facade:

- `marker.rs` constructs current V3 application payload bytes from the resolved plan or compatibility context.
- `embed.rs` selects the LSB, tiled LSB, JPEG DCT/F5, or seed-only carrier operation and maps carrier outcomes.
- `extract.rs` discovers seeds, performs bounded non-tiled/tiled searches, and reuses one carrier-owned tiled JPEG search per operation.
- `verify.rs` parses payloads and classifies CRC/HMAC/signature, malformed, unsupported, and authentication failures.
- `legacy.rs` contains compatibility-only V1/V2 decoding and ECC adapters.

Generic carrier mechanics (permutations, bit embedding/extraction, capacity calculation) remain delegated to `stegoeggo-stego`.

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
fn embed_lsb_v2_in_place(
    img: &mut RgbaImage,
    payload: &[u8],
    seed: u64,
    redundancy: usize,
) -> InPlaceEmbedReport
fn embed_lsb_v2(
    img: &RgbaImage,
    payload: &[u8],
    seed: u64,
    redundancy: usize,
) -> EmbedOutcome<RgbaImage>
fn extract_lsb_v2(
    img: &RgbaImage,
    expected_bits: usize,
    seed: u64,
    base_slot: usize,
    redundancy: usize,
) -> Option<Vec<u8>>
```

**Corrected V2 carrier model (current default):**

- Carrier domain: `width * height * 3` RGB slots (alpha never a carrier)
- Permutation: cycle-walking bijective LCG (`stego_permutation_v2`) over `[0, slot_count)`
- Each payload bit occupies `STEGO_SPREAD_FACTOR * redundancy` consecutive logical
  indices through one permutation — no inter-replica collisions
- Capacity formula: `payload_bits * STEGO_SPREAD_FACTOR * redundancy` slots exact
- Embed uses raw seed directly (no `STEGO_OFFSET_SEED_1` offset) for non-tiled V2.
  Tiled V2 embed uses `local_seed * STEGO_OFFSET_SEED_1` to match extraction probing.

**Legacy carrier (backward-compatible extraction only):**

- Pixel-index carrier: `total_pixels` = `width * height`
- Channel derived from `bit_index % 3`
- Redundancy via multiple passes with offset seeds
- `extract_lsb` preserves the historical mapping exactly
- The cloning V2 API clones once and delegates to the same in-place mutation
  core used by `embed_lsb_v2_in_place`.
- Corrected V2 embedding reads payload bits directly and extraction writes the
  majority-voted bits into its final byte buffer; no bit-per-byte intermediate
  vector is used.

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
pub(crate) fn apply_dct_stego_bytes_from_plan(
    jpeg_bytes: &[u8],
    plan: &ResolvedProtectionPlan,
    tile_size: Option<u32>,
) -> Result<EmbedOutcome<Vec<u8>>>
```

The plan-driven entry point is canonical. The legacy `apply_dct_stego_bytes(jpeg_bytes, ctx)`
helper remains as a compatibility adapter and produces the same application payload before
calling the carrier operation.

- For baseline JPEG: F5 coefficient embedding + seed in quantization tables when those tables are preserved
- For progressive JPEG: Seed-in-Q-tables only (F5 not supported for progressive)
- Calls the carrier crate's encoded-byte JPEG operations; the transcoder and F5 coefficient types remain private to `stegoeggo-stego`
- `probe_dct_support_full()` gates DCT entry: rejects progressive, restart-bearing, non-8-bit, multi-scan, and sampling >4 inputs; unsupported inputs fall back to metadata-only processing
- **One-pass embed**: Computes `max_feasible = available / payload_bits`, selects `min(requested, max_feasible)`, embeds+encodes once. No retry loop, no roundtrip decode/extract self-test. The DCT success path always uses `encode_coefficients_preserving` (the original-JPEG preserving path), so APP/COM/unknown segments survive byte-for-byte.
- **Tiled JPEG success verification**: Tiled embedding records the first successful tile and its local seed, performs the normal single encode, then decodes the encoded output and extracts that exact tile. It reports `Embedded` only when the extracted payload matches the original; a failed post-encode check is reported as a non-success outcome.
- **Tiled JPEG extraction search**: Each extraction/verification call creates one operation-local carrier-owned search context. The coefficient container is decoded once; prefix enumeration, exact-key V3 header/full extraction, and V1/V2 fallback all reuse the retained private state. Candidate identity, origin bounds, nearby seed range (`0..=2`), and redundancy range (`1..=10`) are unchanged.

## Extraction & Verification

```rust
pub fn extract_payload(&self, img: &DynamicImage) -> Option<StegoPayload>
pub fn verify_payload(&self, img: &DynamicImage) -> bool
pub fn verify_payload_with_key(&self, img: &DynamicImage, mac_key: &[u8]) -> VerificationStatus
pub fn verify_payload_from_bytes(&self, img_bytes: &[u8], seed: u64) -> bool
pub fn verify_payload_from_bytes_with_key(&self, img_bytes: &[u8], mac_key: &[u8]) -> VerificationStatus
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
- Non-tiled DCT: capacity-selected redundancy = `min(requested, available / payload_bits)`, single embed+encode
- Tiled LSB: redundancy=1 per tile (tile grid provides the redundancy)
- Extraction always runs 5 passes regardless of redundancy setting

## Fallback Seeds

When metadata is stripped (seed unavailable), extraction tries `FALLBACK_SEEDS` — common test/dev seeds.

## Module Interactions

- **lib.rs**: Applied in Standard pipeline
- **stegoeggo-stego/src/lsb.rs**: Public LSB API surface (`embed`, `embed_in_place`, `extract`, `embed_framed`, `extract_framed`, `capacity`, `LsbConfig`, `InPlaceEmbedReport`, `DEFAULT_TILE_SIZE`). Raw operations are backed by `lsb_internal`; framed operations compose the public frame module with those raw calls. `LsbConfig` exposes fallible `try_new` and `try_with_redundancy` for untrusted input; the panicking `with_redundancy` is retained for compile-time-constant values.
- **stegoeggo-stego/src/application_support.rs**: Narrow parent-crate operations for payload-aware LSB and JPEG calls; hidden behind the optional `application-support` feature. Its opaque tiled-JPEG search context owns decoded state for one operation and exposes no parser, coefficient, or F5 types
- **stegoeggo-stego/src/lsb_internal.rs**: Generic LSB carrier mechanics (permutations, embed/extract, crop, seed fallback). Private; no application-type imports.
- **stegoeggo-stego/src/jpeg.rs**: Generic encoded-byte JPEG carrier facade (DCT capacity, raw/framed embed/extract, Q-table reassembly, seed hint). No application-type imports
- **stegoeggo-stego/src/jpeg_transcoder/**: Private JPEG fast-path implementation used behind `jpeg.rs` and `application_support.rs`
- **stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs**: Private F5-style DCT manipulation
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

When `tile_size > 0`, Standard/canonical hidden-marker paths use tiled embedding. Legacy
`Light` remains `SeedOnly` even when a tile size is present; the tile setting never promotes
Light into full-payload tiled steganography. The tile grid is fixed; no state is shared
between tiles.

### Per-Tile Seed Derivation

Each tile uses `tile_seed(master_seed, tile_x, tile_y)` — a splitmix64 hash
of the master seed mixed with the tile grid coordinate. The same tile coordinate
in any cropped image produces the same seed, so extraction is self-coordinating.

### LSB Tiled Path

- `embed_lsb_tiled`: embeds payload directly into image regions using V2 carrier
  with sub-image slot coordinates, avoiding per-tile RgbaImage allocations. Each
  tile computes carrier slots using the tile's dimensions and maps them to full-image
  coordinates via `(x0 + lx, y0 + ly)`.
- `extract_lsb_tiled_candidates`: scans candidate tile origins in the cropped
  image (stride = `tile_size / 2`, up to `max_origins`), crops sub-images, tries
  grid coordinates around each origin, extracts and verifies integrity.

### F5 Tiled Path

- `apply_dct_stego_bytes_tiled`: iterates tile grid in DCT block space,
  embeds payload in each tile's blocks using `embed_f5_in_blocks` with per-tile
  seed.
- `extract_f5_tiled_candidates`: scans tile positions in the cropped JPEG's
  coefficient container through the feature-gated carrier support layer.
  Prefix enumeration returns an opaque candidate key containing the tile
  origin, nearby grid-seed coordinate, and redundancy. The root reuses that
  exact key for V3 header/full extraction and legacy lengths; a wrong,
  malformed, unsupported, or failed-integrity candidate cannot terminate the
  bounded search before a later candidate is tried. `max_origins` bounds tile
  origins while the nearby seed and redundancy ranges remain bounded by the
  carrier contract.

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

## Public Generic Carrier API

The `stegoeggo::stego` module exposes application-neutral carrier operations
for arbitrary payload bytes, independent of the rights-protection pipeline.

### Module Structure

```
stegoeggo::stego
├── error       — StegoError, JpegUnsupportedReason
├── lsb         — LsbConfig, capacity, embed, embed_in_place, extract, embed_framed, extract_framed
├── jpeg        — JpegConfig, JpegSupport, probe_support, capacity, embed, extract, embed_framed, extract_framed, embed_seed_hint, extract_seed_hint
└── frame       — FrameHeader, encode, decode, decode_prefix
```

### Types

- `StegoError` — Structured error for generic carrier ops (InsufficientCapacity, UnsupportedJpeg, FrameNotFound, MalformedFrame, FrameChecksumMismatch, etc.)
- `CapacityReport` — `{ required, available }` in carrier units (RGB slots for LSB, non-zero AC coefficients for DCT)
- `EmbedReport` — `{ embedded, output, payload_bytes, required_capacity, available_capacity, actual_redundancy }`
- `InPlaceEmbedReport` — `{ embedded, payload_bytes, required_capacity, available_capacity, actual_redundancy }`; returned by `lsb::embed_in_place` without an output image
- `LsbConfig` — seed + redundancy (1–10, default 2)
- `JpegConfig` — seed + redundancy (1–10, default 3)
- `JpegSupport` — Supported or Unsupported(JpegUnsupportedReason)
- `FrameHeader` — version + payload_len (for frame decode)

### Design Decisions

1. **Legacy scheme not exposed** — Only V2 corrected carrier exposed publicly
2. **JPEG seed hint is public** — `embed_seed_hint`/`extract_seed_hint` with fragility warnings
3. **`intensity` not exposed** — Redundancy is the explicit capacity/cost control
4. **Error model** — `StegoError` converts to crate `Error` via `From`
5. **Frame is optional** — Raw APIs don't require framing
6. **Raw versus framed recovery** — Raw extraction requires caller-known length and, for JPEG, `actual_redundancy`. Framed extraction reads the fixed header first, validates the declared length against frame and carrier bounds, and for JPEG probes only the configured redundancy down to 1.
7. **Frame composition** — Framed operations call the existing `frame::encode`, `frame::decode_prefix`, and `frame::decode`; they do not create a second carrier format or import application rights state.
8. **CRC limitation** — The frame CRC32 detects accidental corruption but is not adversarial authentication.
9. **Fallible configuration** — Both `LsbConfig` and `JpegConfig` expose `try_new(seed, redundancy)` and `try_with_redundancy(value)` returning `StegoError::InvalidConfig` for out-of-range values. The original `with_redundancy` is retained for compatibility with callers that pass validated constants; it still panics on invalid values.
10. **Capacity units are documented per carrier** — `CapacityReport` and `EmbedReport` explicitly state that LSB uses RGB carrier slots and JPEG uses non-zero AC coefficients. `InPlaceEmbedReport` is RGB carrier slots only.
11. **CRC vs authentication scope** — The `frame` module-level docs lead with "CRC32 is corruption detection, not authentication". Report units are spelled out in `CapacityReport`/`EmbedReport`/`InPlaceEmbedReport` field docs.

### Frame Wire Format

```
Offset  Size  Field
0       2     Magic [0x53, 0x47]
2       1     Version (=1)
3       4     Payload length (u32 LE, max 16 MiB)
7       4     CRC32 of payload bytes
11..    N     Payload bytes
```

Overhead: 11 bytes. Max payload: 16 MiB. CRC32 covers payload bytes only. The
framed carrier helpers count this overhead in the carrier payload length.

### Tests

`tests/public_stego_api.rs` covers LSB/JPEG/frame raw and framed roundtrips,
frame-overhead capacity, bounded JPEG redundancy downgrade, error conditions,
and rights-metadata absence. Framed tests intentionally recover without the
original payload length or JPEG embed report.

## Carrier Crate Layout (Plan 063)

The `stegoeggo-stego` workspace member is the standalone generic carrier crate.
It contains:

```
stegoeggo-stego/src/
├── lib.rs                 Re-exports + carrier-level reports
├── constants.rs           STEGO_OFFSET_SEED_1, STEGO_SPREAD_FACTOR, SPLITMIX64_SEED
├── error.rs               StegoError, JpegUnsupportedReason, StegoResult
├── frame.rs               Generic framed payload (magic, version, length, CRC32)
├── lsb.rs                 V2 LSB facade (raw, in-place, and framed operations)
├── jpeg.rs                Encoded-JPEG facade (raw/framed operations, seed hint)
├── application_support.rs Narrow parent-crate operation layer (optional feature)
├── jpeg_transcoder/       Private JPEG DCT decode/encode/Huffman/F5 primitives
└── types.rs               EmbedOutcome, EmbedPath, EmbedStatus, EmbedOutcomeSummary, InPlaceEmbedReport
```

The crate has no rights-policy/legal/provenance type dependencies. The root
crate re-exports an explicit allowlist under `stegoeggo::stego`, so the Plan 062
public API (`stegoeggo::stego::lsb`, `stegoeggo::stego::jpeg`,
`stegoeggo::stego::frame`) remains stable while private codec types stay hidden.
The parent crate enables the narrow, unstable `application-support` feature for
its application adapter; that feature exposes operations, not parser/coefficient
types. Plan 063's split decision rationale and dependency measurements are
recorded in `plans/063-status.md`.
