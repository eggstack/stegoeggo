# cloakrs Issue Correction Plan

## Overview

This plan addresses issues identified during a comprehensive code review of the cloakrs library.
Items from three review passes were merged and deduplicated into this single plan.
Ordered by implementation priority.

**Status:** All Critical, Moderate, Minor, and Test issues resolved (24 of 24). All deferred items now implemented.

---

## Critical Issues

### 1. Fix double zigzag in JPEG entropy decoder — ✅ DONE

**File:** `src/jpeg_transcoder/entropy.rs`
**Lines:** 388-403

**Problem:** The decoder stores coefficients at natural positions via `block[ZIGZAG[k]]` (line 388), then applies a second zigzag permutation at lines 401-403. Output blocks end up in zigzag order, not natural 8x8 order. The encoder (line 660) reads these zigzag-ordered blocks in linear order, producing scrambled AC coefficients in re-encoded JPEGs.

**Does NOT affect:** F5 steganography path (operates on in-memory coefficients, never passes through encoder).

**Fix:**
- Remove the redundant reorder loop at lines 401-403
- The block is already in natural order after `block[ZIGZAG[k]] = magnitude`
- The encoder already iterates `k=1..63` in natural order, which is correct for naturally-ordered blocks
- Add an integration test that decodes a JPEG, re-encodes it, and verifies visual fidelity (PSNR or byte-level comparison for lossless path)

---

### 2. Add bounds check in JPEG segment length handling — ✅ DONE

**File:** `src/jpeg_transcoder/mod.rs`
**Lines:** 266-267

**Problem:** `pos += 2 + seg_len` can advance past `data.len()` without an explicit bounds check. If `seg_len` is 0 or 1 (malformed), the while loop re-checks `pos + 4 <= data.len()` and exits, but there's no validation that the segment length is sane. A `seg_len` value causing integer overflow in the addition is theoretically possible.

**Fix:**
- After computing `seg_len`, validate `pos + 2 + seg_len <= data.len()`
- Use checked addition: `pos.checked_add(2).and_then(|p| p.checked_add(seg_len))`
- Return `None` on overflow or out-of-bounds instead of panicking
- Add test with malformed JPEG containing invalid segment lengths

---

## Moderate Issues

### 3. Guard against zero quantization values in F5 seed embedding — ✅ DONE

**File:** `src/jpeg_transcoder/stego_f5.rs`
**Lines:** 103-104

**Problem:** `quant.values[pos] &= 0xFE` on a value of 1 produces 0, which is invalid in JPEG (division by zero during quantization).

**Fix:**
- After clearing the LSB, check if the result is 0 and set it to 1
- Add a test that exercises seed embedding with quantization table values of 1

---

### 4. Align format detection strictness — ✅ DONE

**File:** `src/lib.rs`

**Problem:** `process_image_bytes()` (line 424-425) rejects unrecognized formats with `Error::InvalidFormat`, while `apply_multi_protector_bytes()` (line 331) silently defaults to PNG via `unwrap_or(DEFAULT_OUTPUT_FORMAT)`. Confusing for callers.

**Fix:**
- Make `apply_multi_protector_bytes` return an error when `input_format` is `None` instead of defaulting
- Or make both paths lenient (default to PNG) with a warning
- Recommend strict: unrecognized format should be an error

---

### 5. Remove unnecessary context clone in `apply_multi_protector_bytes` — ✅ DONE

**File:** `src/lib.rs`
**Line:** 326

**Problem:** `let ctx_with_level = ctx.clone()` clones context even though the caller (`process_bytes`) already set the protection level. The comment at line 325 acknowledges this.

**Fix:**
- Remove the redundant clone and use `ctx` directly
- Verify no code path between `process_bytes` and `apply_multi_protector_bytes` relies on the original `ctx` being unmodified

---

### 6. Document MetadataTrapProtector::apply() metadata stripping — ✅ DONE

**File:** `src/protected/metadata_trap.rs`
**Method:** `apply()`

**Problem:** `apply()` re-encodes then re-decodes via `image::load_from_memory()`, which strips injected metadata. Users who call `apply()` expecting metadata to survive will be surprised.

**Fix:**
- Enhance the doc comment on `apply()` with a `# Warning` section
- State that callers MUST use `apply_bytes()` if metadata must survive
- Add a test demonstrating `apply()` returns a `DynamicImage` without metadata, while `apply_bytes()` preserves it

---

### 7. Fix async double-pooling documentation — ✅ DONE

**File:** `src/async_api.rs`
**Lines:** 103-104, 128-129 (doc comments)

**Problem:** Doc comments claim "avoids double-pooling by not using rayon internally," but the synchronous functions called inside `spawn_blocking` DO use rayon (for perturbation and stego). There IS double-pooling: tokio blocking pool threads each run rayon's thread pool.

**Fix:**
- Update doc comments to accurately describe the thread pool situation
- Note that per-image processing uses rayon while batch parallelism uses tokio's blocking pool
- Add note about potential thread pool contention under heavy load

---

### 8. Document PrecomputedProtector error suppression — ✅ DONE

**File:** `src/protected/precomputed.rs`
**Line:** 214

**Problem:** `let _ = self.register_variant(variant)` silently discards persistence failures from `VariantLoader`.

**Fix:**
- Add a doc comment on `apply()` explaining that registration failures are silently ignored by design (best-effort caching)
- This is consistent with the library's design philosophy; no behavioral change needed

---

### 9. PrecomputedProtector clone optimization — ✅ DONE

**File:** `src/protected/precomputed.rs`
**Line:** 209

**Problem:** `perturbation.clone()` clones the data for `ProtectedVariant::new()`, then the original `perturbation` is used again at line 218. Unnecessary allocation.

**Fix:**
- Store perturbation in the variant, then borrow it back via `variant.perturbation_data()` for application
- Or use `Arc<Vec<u8>>` for shared ownership (breaking change to `ProtectedVariant`)

---

### 10. PrecomputedProtector two-phase design documentation — ✅ DONE

**File:** `src/protected/precomputed.rs`
**Lines:** 34-48

**Observation:** `register_variant` intentionally performs loader I/O (line 38-39) without the write lock held, then acquires the write lock (lines 42-46) for the in-memory insert. This is correct — holding the lock during I/O would block readers unnecessarily.

**Fix:**
- Add a doc comment on `register_variant` explaining the two-phase design: persist first (no lock), then insert into cache (with write lock)
- This is a documentation-only change; no behavioral modification needed

---

### 11. JPEG assembly quantization value truncation — ✅ DONE

**File:** `src/jpeg_transcoder/mod.rs`
**Lines:** 130-131

**Problem:** `output.push(val as u8)` silently truncates 16-bit quantization values to 8-bit. When `precision == 8`, values should fit in 8 bits, but there's no validation.

**Fix:**
- Add a debug assertion: `debug_assert!(val <= 255, "Quantization value exceeds 8-bit range")`
- Or validate before truncation and return an error for out-of-range values

---

### 12. Add missing error context in JPEG transcoding — ✅ DONE

**File:** `src/jpeg_transcoder/mod.rs`

**Problem:** `TranscoderError` variants don't include byte position or context for debugging.

**Fix:**
- Add `position: Option<usize>` field to relevant `TranscoderError` variants
- Pass byte position when creating errors during parsing

---

## Minor Issues

### 13. Deduplicate noise generation in serial/parallel paths — ✅ DONE

**File:** `src/util/image.rs`

**Problem:** `apply_perturbation_single_pass_keyed_serial` and `_par` independently construct `NoiseGenerator`, derive `spatial_seed`, and compute `y_variations`.

**Fix:**
- Extract shared setup into a common helper function
- Have both paths call the helper before their respective processing loops

**Deferred reason:** Low priority — the duplication is small and the shared setup is already encapsulated in `PerturbationParams`. Refactoring risks subtle regressions in the noise generation path for marginal code quality gain.

---

### 14. Extract hardcoded fallback seeds as a constant — ✅ DONE

**File:** `src/protected/steganography.rs`
**Lines:** 239, 328

**Problem:** Magic number seed arrays `[42u64, 0, 1, 12345, 99999, 123456789]` duplicated in two locations.

**Fix:**
- Extract to a module-level constant: `const FALLBACK_SEEDS: &[u64] = &[42, 0, 1, 12345, 99999, 123456789];`
- Add doc comment explaining these are common test/dev seeds used when metadata is stripped

---

### 15. Test assertion weakness — ✅ DONE

**File:** `src/lib.rs`
**Line:** 609

**Problem:** `assert!(protected_bytes.len() != input_bytes.len() || ctx.intensity() == 0.0)` passes if output size differs OR intensity is 0. Very weak test — could pass even if output differs by a single byte.

**Fix:**
- Replace with a stronger assertion that validates actual pixel/content differences
- Or at minimum add a comment explaining the edge case

---

### 16. Parallel threshold auto-tuned — ✅ DONE

**File:** `src/util/image.rs`
**Line:** 507

**Problem:** `PARALLEL_THRESHOLD_PIXELS = 256 * 256` is a const. Optimal threshold varies by hardware.

**Fix (optional):**
- Make configurable via `ProtectionContext` or auto-tune based on `rayon::current_num_threads()`
- Low priority — current value is reasonable for most hardware

**Deferred reason:** Low priority — 65536 pixels is a reasonable default for most hardware. Making it configurable adds API surface for marginal benefit. Monitor under production load first.

---

## Test Gaps

### 17. Add entropy codec round-trip test — ✅ DONE

**File:** `src/jpeg_transcoder/entropy.rs` (test module)

**Fix:**
- Add a test that decodes a JPEG to coefficients, re-encodes them, and verifies the output
- This test would have caught the double-zigzag bug

---

### 18. Add `verify_image_bytes()` synchronous test — ✅ DONE

**File:** `tests/integration.rs`

**Fix:**
- Process an image with a MAC key, then verify with `verify_image_bytes()`
- Assert result is `Some(true)`

---

### 19. Test `ProtectionContext` serde behavior with skipped config — ✅ DONE

**File:** `src/types.rs` (test module)

**Fix:**
- Create a `ProtectionContext` with config (MAC key + legal metadata), serialize, deserialize
- Assert the config is `None` after roundtrip (documents `#[serde(skip)]` behavior)

---

### 20. Add WebP end-to-end pipeline test — ✅ DONE

**File:** `tests/integration.rs`

**Fix:**
- Process a PNG image to WebP output, verify the output is valid WebP

---

### 21. Add error variant assertions to existing tests — ✅ DONE

**Files:** `tests/integration.rs`, `tests/basic.rs`

**Fix:**
- Update `is_err()` assertions to check specific error variants using `matches!()`
- Focus on: `InvalidFormat`, `ImageTooLarge`, `DimensionsMismatch`

---

### 22. Add inline tests for PassthroughProtector — ✅ DONE

**File:** `src/protected/passthrough.rs`

**Fix:**
- Add `#[cfg(test)]` module testing: `name()`, `protection_level()`, `estimated_latency_ms()`, `modifies_pixels()`, `apply()` returns `Cow::Borrowed`

---

### 23. Test `NoOpLoader` and `VariantLoader` trait — ✅ DONE

**File:** `src/traits.rs` (test module)

**Fix:**
- Test `NoOpLoader::load_variant()` returns `None`
- Test `NoOpLoader::store_variant()` returns `Ok(())`

---

### 24. Test `is_progressive_jpeg()` public function — ✅ DONE

**File:** `src/jpeg_transcoder/mod.rs` (test module)

**Fix:**
- Add tests with baseline and progressive JPEG inputs

---

## Implementation Order

| Phase | Items | Description |
|-------|-------|-------------|
| 1 | #1, #2 | Critical correctness fixes (zigzag bug, bounds check) |
| 2 | #3, #4, #5 | Moderate correctness fixes (F5 zero, format strictness, clone) |
| 3 | #6, #7, #8 | Documentation improvements (metadata stripping, async, precomputed) |
| 4 | #9, #10, #11, #12 | Performance and error handling improvements |
| 5 | #14, #15 | Minor code quality improvements (#13, #16 deferred) |
| 6 | #17-24 | Test coverage improvements |

---

## Success Criteria

- [x] All Critical and Moderate issues resolved
- [x] `cargo clippy --all-targets -- -D warnings` passes
- [x] `cargo test --all-features` passes (259 tests, up from ~245)
- [ ] `cargo bench` shows no regression (not yet run — recommended before release)
- [x] New tests cover each fix to prevent regression

---

## Completion Summary

**Completed:** 24 of 24 items
**Deferred:** None — items #13 and #16 implemented in final pass

| Category | Status |
|----------|--------|
| Critical (#1-2) | ✅ Done |
| Moderate (#3-12) | ✅ Done |
| Minor (#13-16) | ✅ Done |
| Tests (#17-24) | ✅ Done |

**Test growth:** ~245 → 261 tests (+16 new tests)

**Key learnings documented in AGENTS.md:**
- Entropy decoder reorder loop was identity operation on AC coefficients (ZIGZAG maps zigzag→natural)
- F5 seed embedding with all-1 quantization values can't embed 0-bits (get clamped back to 1)
- `apply_multi_protector_bytes` had silent PNG default and unnecessary context clone
- Async API docs were inaccurate about rayon double-pooling
- `#[serde(skip)]` on config field is now documented by a test

---

## Deferred Item Detailed Plans

### Deferred #13: Deduplicate noise generation in serial/parallel paths

**File:** `src/util/image.rs`

#### Current State

Both `apply_perturbation_single_pass_keyed_serial` (lines 307-362) and
`apply_perturbation_single_pass_keyed_par` (lines 202-265) independently
perform the same setup:

```rust
// Repeated in both functions (approx lines 220-234 and 324-333)
let noise_gen = if mac_key.is_empty() {
    NoiseGenerator::new(seed)
} else {
    NoiseGenerator::with_mac_key(seed, mac_key)
};
let spatial_seed = noise_gen.derive_keyed_seed(0x12345678);
let mut spatial_rng = XorShiftRng::new(spatial_seed);

let variation_min = 0.98f32;
let variation_range_size = 0.04f32;
```

The parallel path collects `y_variations` into a `Vec<f32>` (lines 230-234)
so rayon chunks can index it. The serial path computes each `y_variation`
on the fly (line 336-337).

#### Proposed Plan

**Step 1: Create a shared `PerturbationRuntime` struct**

```rust
/// Pre-computed runtime state shared between serial and parallel paths.
/// Holds per-row variation factors and the shared perturbation parameters.
struct PerturbationRuntime {
    params: PerturbationParams,
    y_variations: Vec<f32>,
}

impl PerturbationRuntime {
    fn new(
        seed: u64,
        intensity: f32,
        intensity_multiplier: f32,
        mac_key: &[u8],
        width: u32,
        height: usize,
    ) -> Self {
        let params = PerturbationParams::new(seed, intensity, intensity_multiplier, mac_key, width);

        let noise_gen = if mac_key.is_empty() {
            NoiseGenerator::new(seed)
        } else {
            NoiseGenerator::with_mac_key(seed, mac_key)
        };
        let spatial_seed = noise_gen.derive_keyed_seed(0x12345678);
        let mut spatial_rng = XorShiftRng::new(spatial_seed);

        let variation_min = 0.98f32;
        let variation_range_size = 0.04f32;
        let y_variations: Vec<f32> = (0..height)
            .map(|_| {
                variation_min
                    + (spatial_rng.next_u64() as f32 / u64::MAX as f32)
                        * variation_range_size
            })
            .collect();

        Self { params, y_variations }
    }

    fn y_variation(&self, y: usize) -> f32 {
        self.y_variations[y]
    }
}
```

**Step 2: Refactor both paths to use `PerturbationRuntime`**

Both `_serial` and `_par` functions call `PerturbationRuntime::new()` once,
then use `runtime.params.perturb_pixel(...)` and `runtime.y_variation(y)`
inside the loop.

**Step 3: Inline constants**

The magic numbers `0.98` and `0.04` should become named constants:

```rust
const VARIATION_MIN: f32 = 0.98;
const VARIATION_RANGE: f32 = 0.04;
const SPATIAL_SEED_TAG: u64 = 0x12345678;
```

**Step 4: Add regression test**

Add a test that calls both serial and parallel paths on the same image with
the same seed and asserts identical output (when pixel count triggers parallel
path, mock or temporarily lower the threshold for the test, or test with a
large enough image).

#### Risk Assessment

- **Risk:** Low. The refactor only changes internal setup; the loop logic is
  untouched. `PerturbationParams` already handles the core parameters.
- **Verification:** Run full test suite. Compare serial vs parallel output on
  the same image/seed/intensity — they must be bit-identical since both use
  the same `y_variations` vector.
- **Estimate:** ~1 hour including tests.

---

### Deferred #16: Make parallel threshold configurable / auto-tuned

**File:** `src/util/image.rs`
**Line:** 507

#### Current State

```rust
pub const PARALLEL_THRESHOLD_PIXELS: usize = 256 * 256;
```

Used in 6 locations across `image.rs` and `precomputed.rs`:
- `apply_perturbation_single_pass_keyed` (line 288): choose serial vs par
- `apply_perturbation_par` (line 521): fall back to serial below threshold
- `apply_perturbation_single_pass_keyed_par` (called via `apply_perturbation_single_pass_keyed`)
- `PrecomputedProtector::apply` (line 180, 214): choose serial vs par

#### Proposed Plan

**Option A: Auto-tune via `rayon::current_num_threads()` (recommended)**

Replace the const with a function that scales the threshold by core count:

```rust
/// Returns the pixel count threshold at which parallelism is worthwhile.
/// Scales with rayon's thread pool size to avoid unnecessary parallel
/// overhead on few-core machines or over-parallelization on many cores.
///
/// Heuristic: base threshold for 4 cores, linearly scaled.
pub fn parallel_threshold() -> usize {
    let cores = rayon::current_num_threads().max(1);
    // Base: 64*64 = 4096 per core. At 4 cores = 64*64*4 = 256*256 = 65536.
    // At 1 core: 64*64 = 4096. At 16 cores: 64*64*16 = 256*512 = 131072.
    cores * 64 * 64
}
```

Keep the old `PARALLEL_THRESHOLD_PIXELS` as a deprecated constant that calls
the function (or remove it and update all call sites).

**Changes required:**
1. Add `parallel_threshold()` function in `image.rs`
2. Replace all 6 usages of `PARALLEL_THRESHOLD_PIXELS` with `parallel_threshold()`
3. Remove or deprecate the const
4. Add a test verifying the function returns a sane value (non-zero, reasonable range)

**Option B: Configurable via `ProtectionContext`**

Add a field to `ProtectionContext`:

```rust
// In types.rs
parallel_threshold: Option<usize>,  // None = auto-tune
```

With builder method:

```rust
#[must_use]
pub fn with_parallel_threshold(mut self, threshold: usize) -> Self {
    self.parallel_threshold = Some(threshold);
    self
}
```

Then pass `ctx.parallel_threshold.unwrap_or_else(parallel_threshold)` at
each usage site.

**Trade-offs:**
- Option A is zero-API-surface, backward-compatible, and covers 95% of cases.
- Option B adds builder surface for the remaining 5% (custom hardware tuning).
- Recommend A first; add B only if production profiling shows the heuristic
  is wrong for a specific deployment.

#### Risk Assessment

- **Risk:** Medium — changes the parallelization behavior for all images.
  On single-core machines, more images will use serial path (correct).
  On many-core machines, more images will use parallel (correct, but
  rayon overhead on small images could be a regression).
- **Mitigation:** The function should have a minimum floor (e.g., `max(cores, 1) * 64 * 64`)
  to ensure at least 4096 pixels before parallelism kicks in.
- **Verification:** Benchmark with `cargo bench` before/after. Test with
  `rayon::ThreadPoolBuilder::new().num_threads(1)` to verify single-core path.
- **Estimate:** ~2 hours including benchmarks and edge-case testing.
