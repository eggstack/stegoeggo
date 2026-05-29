# Utilities Module Review

## Verified Claims

### `src/util/image.rs` — XorShiftRng
- `gen_f32()` returns `[-1.0, 1.0)` (line 62-65) — **correct**
- `gen_range(range: Range<f32>)` — **correct** (line 68-72)
- `gen_range_usize(range: Range<usize>)` — **correct** (line 75-81)
- `next_u64()` algorithm (line 52-59): x ^= x >> 12; x ^= x << 25; x ^= x >> 27; x.wrapping_mul(...) — **correct**
- Warning at lines 34-38 about non-interchangeability with F5XorShiftRng — **correct** (two separate implementations exist)

### `src/util/image.rs` — NoiseGenerator
- `derive_keyed_seed()` returns `self.seed` when no MAC key is set (line 109-113) — **correct**
- HMAC-SHA256 key derivation uses `pixel_pos.to_le_bytes()` (line 117) — **correct**

### `src/util/image.rs` — PerturbationRuntime
- Pre-computes `NoiseGenerator` and spatial seed (line 235-236) — **correct**
- Generates per-row `y_variations: Vec<f32>` (line 238-242) — **correct**
- Eliminates duplicated code between serial and parallel paths — **correct**

### `src/util/image.rs` — PerturbationParams
- `derive_spatial_seed()` method exists at line 172-174 — **correct**
- Private struct with all documented fields (lines 132-140) — **correct**

### `src/util/image.rs` — Functions
- `parallel_threshold()` returns `cores * 64 * 64` (line 578-579) — **correct**
- `SIN_TABLE` has 1024 entries (line 15: `SIN_TABLE_SIZE = 1024`) — **correct**
- `compute_image_hash(img)` returns SHA-256 hex string (lines 410-418) — **correct**
- `detect_image_format(bytes)` detects PNG/JPEG/WebP (lines 424-431) — **correct**
- `encode_image(img, format)` uses quality 90 (line 438) — **correct**
- `encode_image_with_options(img, format, progressive, quality)` function exists (lines 499-525) — **correct**
- `load_image_from_bytes(bytes)` delegates to `image::load_from_memory` (line 531) — **correct**
- `apply_perturbation_single_pass_keyed_par` uses `par_chunks_mut` for row-level parallelism (line 278) — **correct**

### `src/util/seed.rs`
- Uses `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()` (line 17-19) — **correct**
- Guarantees non-zero output (returns 42 if mixing produces zero) (line 28-32) — **correct**
- Uses splitmix64-style bit mixing with golden-ratio constants (lines 22-27) — **correct**

### `src/util/iscc.rs` — Iscc struct
- All fields match: `meta`, `content`, `data`, `instance`, `full` (lines 10-20) — **correct**
- `compute_iscc(img)` and `compute_iscc_from_bytes(bytes)` functions exist (lines 178-188) — **correct**

### `src/util/iscc.rs` — Algorithm
- Normalizes to 32×32 grayscale (line 51) — **correct**
- Computes 2D DCT on normalized image (lines 69-114) — **correct**
- Extracts perceptual hash from DCT coefficients using median-based bit pattern (lines 116-155) — **correct**
- Data hash and instance hash both from SHA-256 of raw RGBA bytes (lines 157-166) — **correct**
- Encodes in base58 (line 169) — **correct**
- Non-standard compliance documented (architecture doc line 33) — **correct**

### `src/protected/constants.rs`
- `NOISE_INTENSITY_MULTIPLIER = 10.0` (line 2) — **correct**
- `STEGO_OFFSET_SEED_1 = 0x517cc1b727220a95` (line 6) — **correct**
- `STEGO_JPEG_AMPLITUDE = 40` (line 10) — **correct**
- `STEGO_JPEG_SPREAD = 5` (line 14) — **correct**
- `STEGO_JPEG_BLOCK_STRIDE = 15` (line 17) — **correct**
- `XORSHIFT_SEED_OFFSET = 0x123456789ABCDEF0` (line 21) — **correct**
- `SPLITMIX64_SEED = 0x9e3779b97f4a7c15` (line 24) — **correct**
- `PRECOMPUTED_CACHE_CAPACITY = 100` (line 27) — **correct** (documented in constants.md design notes)

---

## Discrepancies

### 1. XorShiftRng::new uses wrapping_add, not XOR
**Doc** (`util-image.md:15`): "Initializes with seed XOR'd with `XORSHIFT_SEED_OFFSET`"
**Actual** (`image.rs:45-49`):
```rust
pub fn new(seed: u64) -> Self {
    Self {
        state: seed.wrapping_add(XORSHIFT_SEED_OFFSET),
    }
}
```
Uses `wrapping_add`, not XOR. This is a documentation error.

### 2. XorShiftRng rotation amounts undocumented
**Doc** (`util-image.md`): Mentions `next_u64()` exists but doesn't document the algorithm
**Actual** (`image.rs:52-59`): Uses xorshift algorithm with rotations of 12, 25, 27 (not the standard 13, 17, 13 commonly known). The specific rotation amounts should be documented for reproducibility.

### 3. Parallel threshold comparison operator
**Doc** (`util-image.md:60`): "Uses `parallel_threshold()` to decide: if pixels > threshold, uses parallel path"
**Actual** (`image.rs:333`):
```rust
if total_pixels >= parallel_threshold()
```
Uses `>=`, not `>`. Minor but still a discrepancy.

### 4. Precomputed application `divisor` description
**Doc** (`util-image.md:80`): "`divisor` scales the perturbation intensity"
**Actual** (`image.rs:538`): The function signature and implementation are correct, but the doc comment at line 537 says "scales intensity" which is vague. More importantly, the function doesn't validate that `divisor != 0` — dividing by zero would panic.

### 5. DCT hash truncation not documented
**Doc** (`util-iscc.md:26`): "Produces a 256-bit perceptual hash, truncated to 8 bytes (64 bits) for the content component"
**Actual** (`iscc.rs:64`): The DCT hash is 32 bytes (256 bits), but only 8 bytes are used in `encode_iscc_component` via `&digest[..8]` at line 172. The truncation happens but isn't explained — why 8 bytes specifically?

### 6. ISCC component codes not in doc
**Doc** (`util-iscc.md:33`): Mentions custom component codes (`0x12` for content, `0x33` for data)
**Actual** (`iscc.rs:66, 165`): Uses `0x12` and `0x33` respectively — **matches**, but the architecture doc doesn't mention that `instance` reuses `data`'s code (0x33).

### 7. `instance` field documentation
**Doc** (`util-iscc.md:14`): "Instance hash (base58)"
**Actual** (`iscc.rs:37`): `instance: data_code` — same as data hash. The doc doesn't clarify that `instance == data`.

### 8. Stego redundancy default in extract
**Doc** (`util-image.md:89`): Mentions `parallel_threshold()` returns `cores * 64 * 64` for scaling
**Actual**: Correct, but the documentation doesn't mention that `steganography.rs` hardcodes `EXTRACT_REDUNDANCY = 5` (line 846) for extraction, while embedding uses `ctx.stego_redundancy()` which has a default.

---

## Bugs Found

### Bug 1: Division by zero in `apply_perturbation` and `apply_perturbation_par`
**File**: `src/util/image.rs:560-562` and `image.rs:616-618`
```rust
output_raw[i] = (px_r + perturbation_offset[0] / divisor).clamp(0, 255) as u8;
```
If `divisor == 0`, this will panic with "division by zero". The caller `protected/precomputed.rs` passes `divisor` from intensity calculations. While currently safe (intensity is clamped [0.0, 1.0]), there's no runtime check to prevent zero.

**Fix needed**: Add `debug_assert!(divisor != 0)` or return an error.

### Bug 2: DCT coefficient counting doesn't account for skipped DC coefficients
**File**: `src/protected/steganography.rs:98-102`
```rust
let available_coeffs: usize = coefficients
    .values()
    .flat_map(|v| v.iter())
    .map(|block| block.iter().skip(1).filter(|&&c| c != 0).count())
    .sum();
```
This counts non-zero AC coefficients (skipping index 0 = DC). But `DctStegoF5::embed_f5` may embed in DC coefficients depending on implementation. The available_coeffs check might be overly optimistic, causing premature failure when coefficients appear insufficient.

### Bug 3: Potential index out of bounds in `extract_jpeg_stego`
**File**: `src/protected/steganography.rs:906`
```rust
if bit_votes.iter().any(|v| v.is_empty()) {
    continue;
}
```
This check happens AFTER extracting votes for all `expected_bits`. If earlier logic skips some bits (due to image dimensions not covering all expected positions), some `bit_votes` entries could be empty vectors. The check is correct, but the continue means we silently skip that redundancy pass — no error, just fallback.

More critically: at line 920,
```rust
let bit = if avg > amplitude / 4.0 { 1 } else { 0 };
```
Uses `amplitude / 4.0` as threshold. But `STEGO_JPEG_AMPLITUDE = 40`, so threshold is 10. The embedded amplitude is also 40. This seems intentional (embed uses ±40, extract uses threshold 10 for majority voting), but it's not documented why 4.0 is the divisor.

### Bug 4: `Iscc::content_bytes()` creates subslice on every call
**File**: `src/util/iscc.rs:42-45`
```rust
pub fn content_bytes(&self) -> &[u8] {
    let bytes = self.content.as_bytes();
    &bytes[..bytes.len().min(8)]
}
```
Every call creates a new small allocation for the slice. If called frequently in hot paths, this is wasteful. Consider caching the first 8 bytes.

### Bug 5: JPEG reassembly may fail silently for malformed headers
**File**: `src/protected/steganography.rs:173-175`
```rust
let segment_len =
    u16::from_be_bytes([jpeg_bytes[pos + 2], jpeg_bytes[pos + 3]]) as usize;
pos += 2 + segment_len;
```
If `segment_len` is malformed (e.g., claims to be larger than remaining data), `pos` could exceed `jpeg_bytes.len()`. The loop condition `pos + 4 <= jpeg_bytes.len()` prevents immediate panic, but the segment copy at line 209 `&jpeg_bytes[pos..pos + 2 + segment_len]` could panic if `pos + 2 + segment_len > jpeg_bytes.len()`.

**Fix needed**: Add bounds check before slicing at line 209.

### Bug 6: `stego_permutation` bias for non-power-of-2 totals
**File**: `src/protected/steganography.rs:618-623`
```rust
a.wrapping_mul(index as u64).wrapping_add(b) as usize % total_pixels
```
Uses `% total_pixels` which introduces bias when `total_pixels` is not a power of 2. The docstring acknowledges this ("slight bias is negligible for steganography") but for small images with few pixels, this could matter. Consider using a bitmask if `total_pixels` is power-of-2.

### Bug 7: `fast_sin` modulo wrap could be slow
**File**: `src/util/image.rs:26-31`
```rust
fn fast_sin(angle: f32) -> f32 {
    let normalized = angle.rem_euclid(std::f32::consts::TAU);
    let index = ((normalized / std::f32::consts::TAU) * SIN_TABLE_SIZE as f32 + 0.5) as usize;
    let index = index % SIN_TABLE_SIZE;
    SIN_TABLE[index]
}
```
Uses `rem_euclid` which is a division. For angles already in [0, 2π), this is unnecessary. The `normalized` result could still be large if `angle` is extremely large, but for typical perturbation use (phase is `x * inv_pattern_scale + y_phase + phase_offset`), the angle grows linearly. Over a 10000-pixel width, phase could be large enough to warrant optimization.

### Bug 8: Missing validation in `extract_grayscale_pixels`
**File**: `src/util/iscc.rs:56-59`
```rust
fn extract_grayscale_pixels(img: &DynamicImage) -> Vec<u8> {
    let gray = img.to_luma8();
    gray.into_raw()
}
```
If `img` is an empty image (0×0), `to_luma8()` may panic or return an empty vector. The subsequent `compute_dct_2d` expects 1024 input pixels (32×32). No validation ensures the image has sufficient pixels before DCT computation.

---

## Improvement Opportunities

### Improvement 1: Document XOR vs wrapping_add discrepancy for XorShiftRng::new
The documentation claims XOR but implementation uses wrapping_add. Either fix the doc or fix the code to match the documented behavior. Given the existing warning about PRNG interchangeability, consistency matters.

### Improvement 2: Add debug_assert for divisor != 0 in perturbation functions
At `image.rs:560-562` and `image.rs:616-618`, adding `debug_assert!(divisor != 0)` would catch programming errors in release builds while allowing panic in debug. Better than silent UB or obscure failures.

### Improvement 3: Consider caching `Iscc::content_bytes`
The `content_bytes()` method allocates a new slice on every call (`image.rs:42-45`). Since `content` is a String and `content_bytes` always returns the first 8 bytes, this could be pre-computed and stored.

### Improvement 4: JPEG segment length validation
In `steganography.rs:209`, the slice `&jpeg_bytes[pos..pos + 2 + segment_len]` could panic if `segment_len` is malformed. Add a check:
```rust
if pos + 2 + segment_len > jpeg_bytes.len() {
    return Err(Error::Steganography("Malformed JPEG segment length".to_string()));
}
```

### Improvement 5: Document the 4.0 divisor in JPEG stego extraction threshold
At `steganography.rs:920`, `amplitude / 4.0` is used as threshold for majority voting. The choice of 4 (not 2, not 8) should be documented, as it affects bit extraction reliability.

### Improvement 6: Consider bitmask optimization for stego_permutation
When `total_pixels` is a power of 2, using `& (total_pixels - 1)` instead of `% total_pixels` would be faster and eliminate bias. The function already handles non-power-of-2 via modulo, but an optimization path for the common case could help.

### Improvement 7: DCT edge case validation
The ISCC `normalize_image` resizes to 32×32, but if the input image is smaller than 32×32, the resize might upscale. More importantly, if the image is extremely small (e.g., 1×1), the DCT would produce garbage. Consider adding a minimum dimension check in `compute_iscc`.

### Improvement 8: Consider SIMD/faster path for 2D DCT
The current 2D DCT (`iscc.rs:69-114`) uses triple-nested loops with no SIMD. For 32×32, this is fast enough, but if ISCC is used in hot paths, consider using a faster DCT implementation (like integer approximation or FFT-based).

### Improvement 9: fast_sin pre-normalization
The `fast_sin` function calls `rem_euclid` on every invocation. For the perturbation use case, `phase` accumulates via `x * inv_pattern_scale + y_phase + phase_offset`. If `x` is large, `phase` can be large, and `rem_euclid` will do a division each time. Consider checking if `phase` is already in [0, 2π) before calling `rem_euclid`.

### Improvement 10: Document JPEG stego amplitude relationship
The embedding uses `amplitude = 40` (STEGO_JPEG_AMPLITUDE). Extraction uses threshold `amplitude / 4.0 = 10`. These should be documented as a pair — the threshold should be some fraction of the embedding amplitude for majority voting to work.

---

## Stale References

No stale function or type names found in the reviewed documentation. The architecture docs correctly reference:
- `src/util/image.rs` — all functions and types match
- `src/util/seed.rs` — `generate_random_seed()` matches
- `src/util/iscc.rs` — `Iscc`, `compute_iscc`, `compute_iscc_from_bytes` all match
- `src/protected/constants.rs` — all constants match

---

## Summary

The documentation is generally accurate with minor discrepancies:
1. **XorShiftRng::new** uses `wrapping_add` not XOR (documentation error)
2. **Parallel threshold** uses `>=` not `>` (minor)
3. Several edge cases lack validation (division by zero, malformed JPEG segments, empty images for DCT)

The code quality is high with well-documented invariants. Most "bugs" are edge cases that are unlikely to trigger in production but should be hardened. The primary improvement opportunities are in defensive validation and documentation of magic constants.