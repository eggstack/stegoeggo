# F5 DCT Steganography

**Source:** `stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs` (~1350 lines)

F5-style steganographic embedding in JPEG DCT coefficients. The most sophisticated protection layer.

This is an F5-inspired no-zero-coefficient StegoEggo variant, not an implementation of
conventional F5; no interoperability with other F5 implementations is claimed.
Carrier capacity is eligible AC coefficients with `|coef| >= 2` after canonicalization,
not all non-zero AC coefficients.

## DctStegoF5

```rust
pub struct DctStegoF5 { redundancy: usize }
```

Methods for F5 coefficient manipulation, parameterized by redundancy.

## Seed Embedding in Quantization Tables

```rust
pub fn embed_seed_in_quantization_tables(&self, header: &mut JpegHeader, seed: u64) -> Result<()>
pub fn extract_seed_from_quantization_tables(&self, header: &JpegHeader) -> Option<u64>
```

Embeds 12 bytes in quantization table LSBs when the tables are preserved:
- Magic bytes: `"SEED"` (4 bytes)
- Seed: 8 bytes (u64, little-endian)

### Q-Table Edge Case

Clears quantization table LSBs with `&= 0xFE`. Positions holding values below 2 are
skipped: setting the LSB of a value of 1 would change it to 0 (invalid in JPEG), and a
position holding 0 could flip to 1, both corrupting the Q-table.

The write is transactional. `qtable_hint_capacity` counts exactly the eligible positions
(values `>= 2` across tables 0..2, in the same order the extractor reads) before any
table is mutated, and fewer than `SEED_HINT_BITS` (96: 32 bits of `SEED` magic plus 64
seed bits) returns `TranscoderError::InsufficientHintCapacity { required: 96, available }`
without mutating the header. A successful return implies the paired extractor recovers
the seed from the mutated header. The public `jpeg::embed_seed_hint` maps the hint error
to `StegoError::InsufficientCapacity` in hint-bit units; payload embedding paths attempt
the hint on a best-effort basis via `try_write_seed_hint` so a short table never fails an
otherwise successful payload embed and no unrecoverable partial hint is written.

**Recommendation:** Use quantization values >= 2 for reliable seed embedding.

## F5 Embedding

```rust
pub fn embed_f5(&self, coefficients: &mut Coefficients, payload: &[u8], seed: u64) -> Result<usize>
pub fn extract_f5(&self, coefficients: &Coefficients, expected_bits: usize, seed: u64) -> Vec<u8>
```

`expected_bits` is the original payload bit count. Redundancy is handled internally by reading `expected_bits * redundancy` bits before majority voting.

### F5 Algorithm

1. Canonicalize AC coefficients into the encoder's representable range (±1023), skipping DC (position 0)
2. Collect eligible carrier positions — AC coefficients with `|coef| >= 2` — in deterministic component/block order, then shuffle with `DctCoefficientRng` (private tuple struct, distinct from `PixelSelectionRng` in the root crate)
3. For each payload bit (repeated `redundancy` times):
   - If LSB matches target, keep the coefficient
   - If LSB mismatches, flip it without creating zero (see no-zero variant below)

### No-Zero Variant

When |coef| == 2 and LSB mismatches (to avoid creating zeros or near-zero values):
- **Standard F5:** Would decrement to 0 or ±1 (shrinkage — detectable pattern)
- **No-zero variant:** For |coef| <= 2, sets to ±3; for larger values, decrements absolute value

This avoids detectable zero creation and keeps the carrier set stable after embedding,
since no selected coefficient drops below magnitude 2. The embed/extract position
alignment is preserved because no coefficient is ever zeroed out.

### Strict vs Best-Effort (Public Boundary)

The F5 mechanics above are shared by both public contracts in `stegoeggo-stego/src/jpeg.rs`:
- **Strict** (`embed_strict`/`embed_framed_strict`) — the requested redundancy must fit exactly; insufficient capacity returns `StegoError::InsufficientCapacity` with no carrier output, no redundancy reduction, and no seed-only degradation. Empty payloads are rejected with `InvalidConfig`.
- **Best-effort** (`embed`/`embed_framed`) — compatibility behavior that selects the largest feasible redundancy up to the configured value and emits a quantization-table seed-hint carrier with `embedded == false` when no payload fits. This is the established StegoEggo application policy, explicitly owned by the parent.

### Redundancy and Majority Voting

F5 extraction handles redundancy-based majority voting in a single pass (not multiple passes):
- Embedding repeats bits `redundancy` times before writing to coefficients
- Extraction reads `expected_bits * redundancy` bits, then votes per original bit position
- Robust against noise and perturbation

Note: The 5-pass extraction logic with multiple seed derivations is in `steganography/extract.rs` (`extract_with_redundancy`), not in F5 extraction.

## DctCoefficientRng (private)

```rust
struct DctCoefficientRng(u64);  // private tuple struct
```

F5-specific PRNG for DCT coefficient shuffling. **Different algorithm from `PixelSelectionRng`** in `util/image.rs`. Do NOT interchange — each is paired with their respective embed/extract code paths. This type is private to `stego_f5.rs` and not exposed publicly.

## Module Interactions

- **steganography/embed.rs**: `apply_dct_stego_bytes_from_plan` (canonical) and the legacy `apply_dct_stego_bytes` (legacy context path) call `DctStegoF5` methods through the carrier's narrow `application_support` operation layer
- **steganography/extract.rs**: `extract_f5_tiled_candidates`, `dct_candidates`, and `dct_outcome_with_seed` consume opaque candidate identity from the carrier-owned `TiledJpegSearch`; root never receives F5 objects directly
- **jpeg-transcoder.md**: Uses `JpegTranscoder` for coefficient decode/encode
- **jpeg-header.md**: Modifies quantization tables for seed embedding
- **jpeg-entropy.md**: Works with decoded `Coefficients` type

## Tiled F5 (Crop Resistance)

For crop-resistant JPEG protection, F5 embedding is scoped to tile-sized block
regions. Each tile embeds the full payload independently.

### Tile Block Set

```rust
pub fn tile_block_set(header, coefficients, tile_x, tile_y, tile_size) -> HashSet<(u8, usize)>
```

Computes the set of `(comp_id, block_idx)` pairs that fall within a tile. For
4:2:0 subsampling, a 64×64 luma tile maps to 8×8 luma blocks and 4×4 chroma
blocks per component. The function handles the MCU-interleaved block ordering
used by the coefficient container.

### Block-Scoped Embed/Extract

```rust
pub fn embed_f5_in_blocks(&self, coefficients, payload, seed, tile_blocks) -> Result<usize>
pub fn extract_f5_from_blocks(&self, coefficients, expected_bits, seed, tile_blocks) -> Vec<u8>
```

Same F5 algorithm as the global variants but the carrier set is restricted to
the specified `(comp_id, block_idx)` pairs. Redundancy is fixed at 1 because
the tile grid itself is the redundancy.

### Integration

- `apply_dct_stego_bytes_tiled`: iterates the tile grid, calls
  `embed_f5_in_blocks` for each tile with `tile_seed(master, tx, ty)`.
- `extract_f5_tiled_candidates`: scans tile positions in the cropped JPEG,
  returns an opaque candidate identity for each tile/grid-seed/redundancy
  combination, and reuses that identity for prefix/header/full or legacy
  extraction before verifying integrity. Distinct candidates are not
  deduplicated by equal prefix bytes. The carrier creates one operation-local
  `TiledJpegSearch` context, decodes coefficients once, and evaluates all
  bounded candidates against its retained private state; the root crate never
  receives JPEG headers, coefficient maps, or F5 state.
- Tiled embedding records a successful tile, encodes once, then self-checks that tile's
  payload against the already-mutated in-memory coefficients (one decode + one encode,
  never a re-decode of the output) before reporting `Embedded`. Tiled embedding is
  exact: fixed redundancy 1, no redundancy reduction, no seed-only fallback.
