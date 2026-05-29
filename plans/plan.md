# Cloakrs Bug Fix & Improvement Plan

## Status: Complete

Architecture documentation fixes are **complete** (2026-05-29). This plan tracks the remaining code-level bugs and improvements identified during the documentation review.

---

## Wave 1: Independent Bug Fixes (Parallelizable)

All five items below touch different files with no interdependencies. They can be implemented in parallel by separate agents.

**Status: All 5 tasks completed (2026-05-29)**

---

### Task 1: Add Dimension Validation to `process_bytes`

**File**: `src/lib.rs`
**Bug**: `process()` calls `validate_dimensions()` (line 217) but `process_bytes()` (lines 318-344) does not. Users who set `max_dimension` on `ProtectionContext` only get enforcement on the `DynamicImage` API path, not the byte path.

**Implementation Steps**:

1. In `apply_multi_protector_bytes()` (line 346), after format detection but before processing:
   - **Non-JPEG path** (line 372): After `load_image_from_bytes(img_bytes)?`, add `Self::validate_dimensions(&img, ctx.max_dimension())?;`
   - **JPEG fast path** (lines 364-369): Use the existing `jpeg_transcoder::header::parse()` to extract dimensions from the JPEG header without full decode. Validate those dimensions against `ctx.max_dimension()`.
2. Alternatively, add the validation in `process_bytes()` itself after format detection (around line 325), before dispatching to the match arms. This covers both paths but requires knowing the format first.
3. Add a test that creates a `ProtectionContext` with `max_dimension` set to a small value (e.g., 100), processes a larger image via `process_image_bytes()`, and asserts that `Error::ImageDecode` is returned.

**Key detail**: The `validate_dimensions` method (line 179) takes a `&DynamicImage`. For the JPEG fast path, either parse the header for dimensions or accept the tradeoff and document it. The header parser at `jpeg_transcoder::header::parse()` returns `JpegHeader` which contains width/height.

**Verification**: `cargo test` passes. New test confirms dimension validation on byte path. Existing tests unaffected.

**✅ Completed**: Commit `321f825`. Added `validate_jpeg_dimensions_from_bytes()` for JPEG fast path and `validate_dimensions()` on non-JPEG path. Test `test_max_dimension_validation_process_bytes` verifies 1000×1000 PNG with max_dimension=512.

---

### Task 2: Add LRU Eviction to PrecomputedProtector Cache

**File**: `src/protected/precomputed.rs`
**Bug**: The `variants: RwLock<HashMap<String, ProtectedVariant>>` (line 37) has no eviction, size limit, or TTL. Each cached entry stores `width * height * 4` bytes of perturbation data. A 4000x3000 image = ~48MB per entry. Under sustained load with diverse images, memory grows without bound.

**Implementation Steps**:

1. Add `lru` crate to `Cargo.toml` dependencies (or use `mini_moka` for concurrent cache).
2. Replace `RwLock<HashMap<String, ProtectedVariant>>` with an LRU cache. The `lru` crate's `LruCache` is not `Sync`, so either:
   - Use `RwLock<LruCache<String, ProtectedVariant>>` (simple, matches existing pattern), or
   - Use `mini_moka::sync::Cache` for better concurrent performance.
3. Set a reasonable default capacity. Consider making it configurable via a new field on `PrecomputedProtector` or a constant in `constants.rs`. Suggested default: 100 entries or 1GB (whichever is reached first).
4. Update `register_variant()`, `register_variants()`, and `get_cached_variant()` to work with the new cache type.
5. Add a test that inserts more entries than the capacity and verifies old entries are evicted.

**Key detail**: The cache key format is `{hash}_{level}_{intensity}` (from `ProtectedVariant::cache_key()`). The `register_variant` method (line 77) does a two-phase insert (persist without lock, then insert with write lock). This pattern needs to work with the LRU cache.

**Verification**: `cargo test` passes. New test confirms eviction behavior. Memory usage bounded under load.

**✅ Completed**: Commit `14121aa`. Added `lru` 0.12 crate, `PRECOMPUTED_CACHE_CAPACITY = 100` constant, `with_capacity()` constructor. Uses `peek()` for reads (no LRU promotion on reads). Test `lru_eviction_removes_old_entries` inserts 5 entries into capacity-3 cache.

---

### Task 3: Fix Seed Embedding Silent Failure for Unit Quantization Values

**File**: `src/jpeg_transcoder/stego_f5.rs`
**Bug**: When embedding seed data into JPEG quantization tables, `&= 0xFE` on a value of 1 produces 0, which is then clamped back to 1. This means 0-bits can never be embedded when the quantization value is 1. The embed function returns `Ok(())` even when it has failed, and extraction silently returns wrong data.

**Implementation Steps**:

1. In `embed_seed_into_quantization_tables()` (around line 94), when the target bit is 0 and the current value is 1:
   - Option A (preferred): Return an error when the quantization table contains values of 1 and the target bit is 0. Use a custom error variant or `Error::ImageDecode` with a descriptive message.
   - Option B: Log a warning (if logging is available) and skip that bit. The seed will be partially corrupted but at least the caller knows something went wrong.
   - Option C: Document the limitation clearly and add a precondition check at the start of the function that fails fast if any quantization value is < 2.
2. Add a test that uses a quantization table with all-1 values, attempts seed embedding, and verifies the correct behavior (error returned or warning logged).
3. Update the doc comment on the function to document the constraint: quantization values must be >= 2 for reliable seed embedding.

**Key detail**: The existing test `test_seed_embed_with_unit_quant_values` (line 592) uses `[2; 64]` — it deliberately avoids the edge case. The AGENTS.md already notes "Use quantization values >= 2 for reliable seed embedding" but the code doesn't enforce this.

**Verification**: `cargo test` passes. New test confirms error/warning on all-1 tables. No regressions on existing tests.

**✅ Completed**: Commit `ef5c249`. Added precondition check in `embed_seed_in_quantization_tables()` that returns `TranscoderError::EmbeddingFailed` if any quantization value < 2. Removed unreachable clamping code. Test `test_seed_embed_all_ones_quant_returns_error`.

---

### Task 4: Document `Option<bool>` Three-State Semantics

**File**: `src/types.rs`
**Not a bug**: The `inject_metadata: Option<bool>` and `inject_legal_claims: Option<bool>` fields (lines 304-305) are a deliberate three-state design:
- `None` = use level-based defaults (inject metadata unless `Disabled`; never inject legal claims)
- `Some(true)` = force-enable (override level default)
- `Some(false)` = force-disable (override level default)

The behavior is correct but underdocumented. Callers may not realize that omitting `with_metadata_injection()` differs from `.with_metadata_injection(false)` for non-Disabled levels.

**Implementation Steps**:

1. Add doc comments to the `inject_metadata` and `inject_legal_claims` fields in `types.rs` explaining the three-state semantics.
2. Update the doc comments on `with_metadata_injection()` (line 437) and `with_legal_claims()` (line 443) to explain:
   - What `None` (not calling the method) means
   - What `Some(true)` / explicit `true` means
   - What `Some(false)` / explicit `false` means
   - The behavioral difference for non-Disabled protection levels
3. Add a doc comment or note on the getter methods explaining the same.
4. Consider adding a code example in the doc comment showing the three states.

**Verification**: `cargo doc` generates clean documentation. `cargo test` passes (no code changes, only doc comments).

**✅ Completed**: Commit `c296213`. Doc comments on fields, builders (`with_metadata_injection`, `with_legal_claims`), and getters (`inject_metadata`, `inject_legal_claims`) in `types.rs`. Three-state semantics clearly explained.

---

### Task 5: Fix CLI Batch Mode Filename Collisions

**File**: `cloakrs-cli/src/main.rs`
**Bug**: In `process_single_file()` (lines 231-247), the output filename is derived from `file_stem()` + `_protected.` + output extension. Two files with the same stem (e.g., `photo.jpg` and `photo.jpeg`) both produce `photo_protected.jpg`, and the second silently overwrites the first.

**Implementation Steps**:

1. In the batch processing loop (lines 451-484), track output paths in a `HashSet<PathBuf>` or `HashMap<String, usize>` (stem → count).
2. Before writing, check if the output path already exists in the set. If collision detected:
   - Option A (preferred): Append a numeric suffix: `photo_protected_1.jpg`, `photo_protected_2.jpg`, etc.
   - Option B: Use the original subdirectory path to preserve uniqueness: `dir1/photo_protected.jpg` vs `dir2/photo_protected.jpg`.
   - Option C: Include part of the original filename/hash in the output name.
3. For the single-file path (not batch), collision detection is not needed since there's only one file.
4. Add a test or manual verification that processes two files with the same stem and confirms both are written with distinct names.

**Key detail**: The `process_single_file` function is called per-file in the batch loop. The collision state (the set of seen output paths) needs to be threaded through the loop, not per-function. The loop at line 451 should maintain the set and pass it to `process_single_file`, or `process_single_file` should return the output path and the loop handles collision.

**Verification**: `cargo test` passes (CLI tests if any). Manual test: create two JPEGs with same stem, run CLI batch, confirm both outputs exist.

**✅ Completed**: Commit `237fe23`. Added `override_output: Option<PathBuf>` parameter to `process_single_file()`. Both serial and parallel batch paths maintain `HashMap<PathBuf, usize>` for collision detection. Duplicate stems get `_protected_1`, `_protected_2`, etc.

---

## Execution Notes

### Parallelization

All 5 tasks are independent — they touch different files with no shared state. They can be assigned to 5 parallel sub-agents. Each agent should:
1. Read the relevant source file(s) referenced above
2. Implement the fix
3. Run `cargo test` and `cargo clippy --all-targets -- -D warnings`
4. Verify the fix works per the verification criteria

### Post-Wave 1

After all 5 tasks complete, run the full test suite:
```bash
cargo test --all-features
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Then update AGENTS.md to reflect any new conventions or gotchas discovered during implementation.

### Results

All 5 tasks completed via parallel sub-agents in separate worktrees. Merged to master on 2026-05-29.

- 264 tests pass (168 unit + 20 basic + 63 integration + 9 async)
- Clippy clean
- Format clean
- AGENTS.md updated with new conventions
- AGENTS.override.md updated with implementation notes
