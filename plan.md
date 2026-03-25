# cloakrs Issue Correction Plan

## Overview

This plan addresses issues identified during a comprehensive code review of the cloakrs library.
Items from three review passes were merged and deduplicated into this single plan.
Ordered by implementation priority.

**Status:** All Critical, Moderate, and Minor issues resolved (except #13 and #16, deferred as low priority). All test gaps filled. See "Completion Summary" at end.

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

### 13. Deduplicate noise generation in serial/parallel paths — ⏸️ DEFERRED

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

### 16. Parallel threshold hardcoded — ⏸️ DEFERRED

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

**Completed:** 22 of 24 items
**Deferred:** #13 (noise dedup), #16 (parallel threshold) — both low priority

| Category | Status |
|----------|--------|
| Critical (#1-2) | ✅ Done |
| Moderate (#3-12) | ✅ Done |
| Minor (#13-16) | ✅ 2/4 done, 2 deferred |
| Tests (#17-24) | ✅ Done |

**Test growth:** ~245 → 259 tests (+14 new tests)

**Key learnings documented in AGENTS.md:**
- Entropy decoder reorder loop was identity operation on AC coefficients (ZIGZAG maps zigzag→natural)
- F5 seed embedding with all-1 quantization values can't embed 0-bits (get clamped back to 1)
- `apply_multi_protector_bytes` had silent PNG default and unnecessary context clone
- Async API docs were inaccurate about rayon double-pooling
- `#[serde(skip)]` on config field is now documented by a test
