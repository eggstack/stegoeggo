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

Embeds 12 bytes in quantization table LSBs:
- Magic bytes: `"SEED"` (4 bytes)
- Seed: 8 bytes (u64, little-endian)

### Q-Table Edge Case

Clears quantization table LSBs with `&= 0xFE`. A quantization value of 1 becomes 0 (invalid in JPEG). A post-clear clamp (`if val == 0 { val = 1 }`) prevents this. However, seed embedding may fail silently if too many values are 1 (0-bits can't stick).

**Recommendation:** Use quantization values >= 2 for reliable seed embedding.

## F5 Embedding

```rust
pub fn embed_f5(coefficients: &mut Coefficients, payload: &[u8], seed: u64) -> Result<usize>
pub fn extract_f5(coefficients: &Coefficients, expected_bits: usize, seed: u64) -> Vec<u8>
```

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
- Extraction reads all repeated bits and takes the majority value per bit position
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
