# F5 DCT Steganography

**Source:** `src/jpeg_transcoder/stego_f5.rs` (~658 lines)

F5-style steganographic embedding in JPEG DCT coefficients. The most sophisticated protection layer.

## DctStegoF5

```rust
pub struct DctStegoF5;
```

Static methods for F5 coefficient manipulation.

## Seed Embedding in Quantization Tables

```rust
pub fn embed_seed_in_quantization_tables(&self, header: &mut JpegHeader, seed: u64) -> Result<()>
pub fn extract_seed_from_quantization_tables(&self, header: &JpegHeader) -> Option<u64>
```

Embeds 12 bytes in quantization table LSBs when the tables are preserved:
- Magic bytes: `"SEED"` (4 bytes)
- Seed: 8 bytes (u64, little-endian)

### Q-Table Edge Case

Clears quantization table LSBs with `&= 0xFE`. A quantization value of 1 would become 0 (invalid in JPEG), so those positions are skipped instead. Seed embedding can still fail to recover every bit if too many values are 1 and there are not enough usable positions.

**Recommendation:** Use quantization values >= 2 for reliable seed embedding.

## F5 Embedding

```rust
pub fn embed_f5(coefficients: &mut Coefficients, payload: &[u8], seed: u64) -> Result<usize>
pub fn extract_f5(coefficients: &Coefficients, expected_bits: usize, seed: u64) -> Vec<u8>
```

`expected_bits` is the original payload bit count. Redundancy is handled internally by reading `expected_bits * redundancy` bits before majority voting.

### F5 Algorithm

1. Use `F5XorShiftRng` to generate a permutation of coefficient positions
2. For each payload bit:
   - Find next non-zero coefficient
   - Modify LSB to match payload bit
   - Handle shrinkage (when modification creates zero)

### No-Zero Variant

When |coef|==1 and LSB mismatches:
- **Standard F5:** Would decrement to 0 (shrinkage — detectable pattern)
- **No-zero variant:** Increments absolute value (+1→+2, -1→-2)

This avoids detectable zero creation. The embed/extract position alignment is preserved because no coefficient is ever zeroed out.

### Redundancy and Majority Voting

F5 extraction handles redundancy-based majority voting in a single pass (not multiple passes):
- Embedding repeats bits `redundancy` times before writing to coefficients
- Extraction reads `expected_bits * redundancy` bits, then votes per original bit position
- Robust against noise and perturbation

Note: The 5-pass extraction logic with multiple seed derivations is in `steganography.rs` (`extract_with_redundancy`), not in F5 extraction.

## F5XorShiftRng

```rust
pub struct F5XorShiftRng { state: u64 }
```

F5-specific PRNG for DCT coefficient shuffling. **Different algorithm from `XorShiftRng`** in `util/image.rs`. Do NOT interchange — each is paired with their respective embed/extract code paths.

## Module Interactions

- **steganography.rs**: `apply_dct_stego_bytes` calls `DctStegoF5` methods
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
pub fn embed_f5_in_blocks(coefficients, payload, seed, tile_blocks) -> Result<usize>
pub fn extract_f5_from_blocks(coefficients, expected_bits, seed, tile_blocks) -> Vec<u8>
```

Same F5 algorithm as the global variants but the carrier set is restricted to
the specified `(comp_id, block_idx)` pairs. Redundancy is fixed at 1 because
the tile grid itself is the redundancy.

### Integration

- `apply_dct_stego_bytes_tiled`: iterates the tile grid, calls
  `embed_f5_in_blocks` for each tile with `tile_seed(master, tx, ty)`.
- `extract_f5_tiled_candidates`: scans tile positions in the cropped JPEG,
  tries grid coordinates, calls `extract_f5_from_blocks`, verifies integrity.
