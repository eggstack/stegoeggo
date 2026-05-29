# Stale Architecture Items Report

Generated from systematic review of all architecture documents.
Updated: Fixes have been applied to all identified stale items.

---

## Fixed Documentation Items

### 1. `protected-precomputed.md` — Outdated Cache Warning ✅ FIXED
**Issue**: Documentation warns about "unbounded cache growth" but code uses `LruCache` with bounded capacity (100) and LRU eviction.
**Evidence**: `precomputed.rs:326-348` test `lru_eviction_removes_old_entries` confirms bounded behavior.
**Action**: Updated documentation to reflect `LruCache` implementation.

### 2. `overview.md:257` — Incorrect Parallel Threshold ✅ FIXED
**Issue**: Claims 4 cores = 65536 pixel threshold, but actual formula `cores * 64 * 64` gives 16384.
**Evidence**: Code uses `parallel_threshold()` which scales as `current_num_threads() * 64 * 64`.
**Action**: Corrected threshold calculation in documentation to: 1 core: 4096, 4 cores: 16384, 16 cores: 65536.

### 3. `pipeline.md:60` — DCT Stego Claim ✅ FIXED
**Issue**: Claims "no DCT stego" in JPEG fast path but code does apply DCT stego.
**Evidence**: `apply_multi_protector_bytes` applies stego for Standard/Enhanced/Strong levels.
**Action**: Updated claim to reflect DCT stego verification is performed.

### 4. `pipeline.md:24` — Dimension Validation Claim ✅ FIXED
**Issue**: Claims "no dimension validation" but validation does occur in `process_bytes`.
**Evidence**: `process_bytes` validates `max_dimension` for both JPEG (header parse) and non-JPEG paths.
**Action**: Updated claim to reflect validation exists.

### 5. `protected-noise.md` — XorShiftRng Algorithm Description ✅ FIXED
**Issue**: Documents claim `XorShiftRng` uses XOR operations but code uses `wrapping_add`.
**Evidence**: `util/image.rs:new()` uses `wrapping_add` mixing, not XOR.
**Action**: Corrected algorithm description in documentation.

### 6. `overview.md:63` — Protection Layers Claim ✅ FIXED
**Issue**: Claims "all three protection layers" for all levels above Disabled, but JPEG fast path skips perturbation.
**Action**: Updated to clarify which layers apply at which levels.

### 7. `overview.md` — ASCII Diagram ✅ FIXED
**Issue**: Diagram showed `Protected/` grouping that doesn't exist in code (flat struct fields).
**Action**: Updated diagram to show flat structure.

### 8. `pipeline.md:26-28` — Protected Grouping ✅ FIXED
**Issue**: Diagram showed `Protected/` as a grouping.
**Action**: Updated to show flat struct fields.

### 9. `pipeline.md:71` — Output Format Error ✅ FIXED
**Issue**: Claims output format throws `InvalidFormat`, but code throws on input format detection.
**Action**: Corrected to reference input format.

### 10. `types.md` — Method Name Mismatch ✅ FIXED
**Issue**: `with_output_format()` vs actual `with_format()`.
**Action**: Updated doc to use `with_format()`.

### 11. `types.md` — ProtectedVariant Fields ✅ FIXED
**Issue**: Missing `protection_level` field from ProtectedVariant docs.
**Action**: Added field to documentation.

### 12. `types.md` — DmiValue Auto-mapping ✅ FIXED
**Issue**: Claims auto-mapped via `impl From` but no such impl exists.
**Action**: Updated to clarify mapping is via helper in metadata_trap.rs.

### 13. `types.md` — LegalMetadata Field Name ✅ FIXED
**Issue**: Field listed as `ai_training_constraints` but actual is `ai_constraints`.
**Action**: Corrected field name in documentation.

### 14. `util-image.md` — XorShiftRng Init ✅ FIXED
**Issue**: Claims XOR but code uses `wrapping_add`.
**Action**: Corrected to `wrapping_add`.

### 15. `util-image.md` — Parallel Threshold Operator ✅ FIXED
**Issue**: Claims `>` but code uses `>=`.
**Action**: Corrected to `>=`.

### 16. `cli.md` — Directory Structure Preservation ✅ FIXED
**Issue**: Claims "preserves directory structure" but batch outputs to flat directory.
**Action**: Corrected to clarify flat output behavior.
