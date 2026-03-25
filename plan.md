# cloakrs Fix Plan

Priority tiers: **P0** (bugs/correctness), **P1** (performance/robustness), **P2** (cleanup/docs).

---

## Phase 1: Correctness Bugs (P0)

### 1.1 Fix `embed_jpeg_stego` redundancy — `src/protected/steganography.rs:729-768`

**Problem:** `embedded` is not reset between passes. When `embedded >= bits_per_pass`, the function returns inside the inner loop, breaking the outer `for pass in 0..redundancy` after just one pass. `redundancy > 1` is a no-op for JPEG pixel-based stego.

**Fix:** Reset `embedded` to 0 at the start of each pass. Remove the early `return` — instead `break` out of the inner loops when the pass is complete, then continue to the next pass.

```rust
for pass in 0..redundancy {
    let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_mul(pass as u64));
    let mut rng = XorShiftRng::new(offset_seed);
    let mut embedded = 0;
    // ... existing loops ...
    // Replace `return output` with:
    break; // break out of inner loops, continue to next pass
}
output
```

Also verify that `extract_jpeg_stego` (line ~795) correctly handles multi-pass extraction — it already iterates `0..5` passes with majority voting, so it should work once embedding is fixed.

**Tests:** Add a test that embeds with `redundancy=3`, then extracts and verifies all 3 passes contribute to the result.

---

### 1.2 Add empty-input guard in JPEG header parser — `src/jpeg_transcoder/header.rs:118-130`

**Problem:** `data.len() - 1` underflows to `usize::MAX` when `data` is empty, causing a panic or massive iteration.

**Fix:** Add at the top of `parse()`:

```rust
if data.len() < 2 {
    return Err(TranscoderError::InvalidFormat("Input too short".into()));
}
```

**Tests:** Add a test that passes an empty slice and a single-byte slice, asserting `Err(InvalidFormat)`.

---

### 1.3 Fix tiny-JPEG underflow in SOF search — `src/jpeg_transcoder/header.rs:161`

**Problem:** `while search_pos < end_pos - 10` underflows when `end_pos < 10`.

**Fix:** The `data.len() < 2` guard from 1.2 partially covers this, but also add:

```rust
if end_pos < 10 {
    return Err(TranscoderError::InvalidFormat("JPEG too short for SOF".into()));
}
```

---

### 1.4 Fix `precomputed.rs` double `to_rgba8()` — `src/protected/precomputed.rs:122,206`

**Problem:** `generate_perturbation_data` calls `img.to_rgba8()` to get dimensions. Then `apply()` calls `to_rgba8()` again on the same image. Two allocations where one suffices.

**Fix:** In `apply()`, convert to RGBA once, get dimensions, pass those dimensions to `generate_perturbation_data` instead of the full image. Change `generate_perturbation_data` signature to take `(width, height)` directly:

```rust
pub fn generate_perturbation_data(
    &self,
    width: u32,
    height: u32,
    ctx: &ProtectionContext,
) -> Result<Vec<u8>> {
```

Then in `apply()`, do a single `img.to_rgba8()` call and reuse it for both generation and application.

---

## Phase 2: Performance (P1)

### 2.1 Cache HuffmanDecoder per table — `src/jpeg_transcoder/entropy.rs:285-286`

**Problem:** `HuffmanDecoder::from_table()` is called per-component per-MCU. For a 1024x768 JPEG with 4:2:0, that's ~98K table constructions. The tables are constant.

**Fix:** Build decoders once before the MCU loop. Store in a `HashMap` or fixed array keyed by table ID. The `CoefficientDecoder` or `decode` method should construct them up-front:

```rust
// Before the MCU loop:
let mut dc_decoders: [Option<HuffmanDecoder>; 4] = [None, None, None, None];
let mut ac_decoders: [Option<HuffmanDecoder>; 4] = [None, None, None, None];
for comp in &self.header.components {
    if dc_decoders[comp.dc_table_id as usize].is_none() {
        let table = self.header.get_dc_huffman_table(comp.dc_table_id)
            .or_else(|| self.header.get_dc_huffman_table(0))
            .ok_or_else(|| ...)?;
        dc_decoders[comp.dc_table_id as usize] =
            Some(HuffmanDecoder::from_table(&table.counts, &table.values));
    }
    // same for AC
}
```

Then index into the pre-built decoders inside the MCU loop.

---

### 2.2 Pre-compute Huffman encode lookup — `src/jpeg_transcoder/entropy.rs:691-721`

**Problem:** `write_huffman_code` does a linear scan of the Huffman table values for every symbol during encoding. O(n) per symbol.

**Fix:** Before encoding, build a `HashMap<u8, (u16, u8)>` mapping symbol → (code, code_length). Then `write_huffman_code` becomes a single hash lookup. Alternatively, use a `[Option<(u16, u8)>; 256]` array for O(1) with no hashing overhead.

---

### 2.3 Add constant-time HMAC comparison — `src/protected/steganography.rs:449-451`

**Problem:** `computed_mac == expected_mac` is timing-attackable.

**Fix:** Add `subtle = "2"` to `Cargo.toml` `[dependencies]`. Replace:

```rust
use subtle::ConstantTimeEq;
computed_mac.ct_eq(expected_mac).into()
```

This is a single-line change with one new dependency.

---

### 2.4 Cache generated perturbations in `PrecomputedProtector` — `src/protected/precomputed.rs`

**Problem:** When no variant is registered, every `apply()` regenerates perturbation data from scratch.

**Fix:** After `generate_perturbation_data` succeeds in `apply()`, call `self.register_variant(hash, ctx, perturbation_data)` to populate the cache. This way subsequent calls for the same image/seed/intensity will hit the cache.

---

## Phase 3: Cleanup (P2)

### 3.1 Fix README inaccuracies

- Remove `--target` CLI flag reference (line ~288)
- Change `ctx.with_output_format()` to `ctx.with_format()` (line ~255)
- Change `cloakrs::util::image::compute_image_hash` to `cloakrs::compute_image_hash` (line ~449)
- Change `DynamicImage::open` to `image::open` (line ~584)

### 3.2 Unify `XorShiftRng` implementations

Two separate implementations exist in `src/util/image.rs` and `src/jpeg_transcoder/stego_f5.rs`. If intentional (to prevent cross-context seed reuse), keep both but add a `/// WARNING` doc comment. Otherwise, extract a shared `xorshift` module under `src/util/`.

### 3.3 Add `generate_random_seed()` unit test

Verify: non-zero return, determinism with known SystemTime, distribution quality over a sample.

### 3.4 Add unit tests for unprotected modules

- `src/protected/noise.rs` — test that noise is applied, intensity=0 returns borrowed, pixel values stay in range
- `src/protected/enhanced.rs` — test that enhanced produces different output than standard
- `src/protected/precomputed.rs` — test dimension mismatch, cache hit, cache miss generation
- `src/types.rs` — test builder chain, intensity clamping, seed round-trip through serde

### 3.5 Consider `apply()` metadata loss for `MetadataTrapProtector`

The current behavior (`apply()` strips injected metadata on re-decode) is documented in AGENTS.md but violates trait expectations. Options:
- **A)** Add a doc comment on `apply()` warning users to use `apply_bytes()` for metadata survival
- **B)** Change `apply()` to return the re-encoded bytes as an error variant or a wrapper type

Recommend option A — it's the least disruptive and the current behavior is by design for the `DynamicImage` API.

### 3.6 Extract `SteganographyProtector` into submodules

**Rationale:** `steganography.rs` is 1821 lines covering LSB, JPEG pixel stego, DCT F5 stego, payload generation, and verification. Hard to navigate and review.

**Approach:**
- `src/protected/stego/mod.rs` — module aggregation, re-exports
- `src/protected/stego/lsb.rs` — `embed_lsb`, `extract_lsb`, `extract_with_redundancy`
- `src/protected/stego/dct.rs` — `embed_jpeg_stego`, `extract_jpeg_stego`, `apply_dct_stego_bytes`
- `src/protected/stego/verification.rs` — payload generation, MAC, checksum, `verify_*` methods
- Update imports in `lib.rs` and other consumers

### 3.7 Add edge-case integration tests

**Tests to add in `tests/integration.rs`:**
- Corrupted/invalid image bytes (truncated PNG, truncated JPEG)
- Unsupported image formats (BMP, GIF)
- Very small images (1x1, 2x2)
- Malformed JPEG handling (missing markers, invalid Huffman tables)
- WebP with unusual configurations

### 3.8 Document `generate_random_seed()` security properties

`src/util/seed.rs` has a doc comment warning about non-cryptographic security, but `ProtectionContext::default()` (which calls it) has no such warning. Add doc comment on the `Default` impl for `ProtectionContext` warning that the seed is predictable.

Consider adding a `generate_secure_seed()` function using `getrandom` for users who need cryptographic seeds.

### 3.9 Document zero-intensity pipeline behavior

**Not a bug, but confusing.** When `intensity == 0.0`, `NoiseProtector` returns `Cow::Borrowed` (no-op), but steganography and metadata injection still run. This is by design — intensity controls noise only; stego and metadata are orthogonal layers. Add a doc comment on `ProtectionPipeline::process` clarifying that intensity only affects the perturbation stage.

### 3.10 Improve error context in `error.rs`

Use `thiserror` source chaining to add context to error variants. Currently `ImageDecode(String)` and `Image(ImageError)` overlap — consider merging or documenting when each is used.

### 3.11 Add JPEG fast-path benchmark

`benches/bench.rs` has no benchmark for the JPEG-in/JPEG-out fast path. Add one to quantify the performance gain over pixel decode/encode.

### 3.12 Add async API usage examples

`src/async_api.rs` has no doc comments with usage examples. Add examples showing typical async CDN/WAF integration patterns.

---

## Verification

After each phase, run:

```bash
cargo check
cargo test --all-features
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

After all phases, run:

```bash
cargo bench
```

to verify no regressions in the JPEG entropy codec (Phase 2.1/2.2 should improve it).

---

## File Change Summary

| File | Phase | Change |
|------|-------|--------|
| `src/protected/steganography.rs` | 1.1 | Fix redundancy loop |
| `src/jpeg_transcoder/header.rs` | 1.2, 1.3 | Add length guards |
| `src/protected/precomputed.rs` | 1.4, 2.4 | Fix double RGBA, add auto-caching |
| `src/jpeg_transcoder/entropy.rs` | 2.1, 2.2 | Cache decoders, pre-compute encode table |
| `src/protected/steganography.rs` | 2.3 | Constant-time HMAC |
| `Cargo.toml` | 2.3 | Add `subtle` dependency |
| `README.md` | 3.1 | Fix 4 doc bugs |
| `src/util/image.rs` / `stego_f5.rs` | 3.2 | Unify or document PRNGs |
| `src/util/seed.rs` | 3.3, 3.8 | Add unit test + security docs |
| `src/protected/noise.rs` | 3.4 | Add unit tests |
| `src/protected/enhanced.rs` | 3.4 | Add unit tests |
| `src/protected/precomputed.rs` | 3.4 | Add unit tests |
| `src/types.rs` | 3.4 | Add unit tests |
| `src/protected/steganography.rs` → `stego/` | 3.6 | Extract into submodules |
| `tests/integration.rs` | 3.7 | Add edge-case tests |
| `src/lib.rs` | 3.9 | Document zero-intensity behavior |
| `src/error.rs` | 3.10 | Improve error context |
| `benches/bench.rs` | 3.11 | Add JPEG fast-path benchmark |
| `src/async_api.rs` | 3.12 | Add usage examples |
