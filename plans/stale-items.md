# Stale Architecture Items Report

Generated from systematic review of all architecture documents.

---

## Stale Documentation Items

### 1. `protected-precomputed.md` — Outdated Cache Warning
**Issue**: Documentation warns about "unbounded cache growth" but code uses `LruCache` with bounded capacity (100) and LRU eviction.
**Evidence**: `precomputed.rs:326-348` test `lru_eviction_removes_old_entries` confirms bounded behavior.
**Action**: Update documentation to reflect `LruCache` implementation.

### 2. `overview.md:257` — Incorrect Parallel Threshold
**Issue**: Claims 4 cores = 65536 pixel threshold, but actual formula `cores * 64 * 64` gives 16384.
**Evidence**: Code uses `parallel_threshold()` which scales as `current_num_threads() * 64 * 64`.
**Action**: Correct threshold calculation in documentation.

### 3. `pipeline.md:60` — DCT Stego Claim
**Issue**: Claims "no DCT stego" in JPEG fast path but code does apply DCT stego.
**Evidence**: `apply_multi_protector_bytes` applies stego for Standard/Enhanced/Strong levels.
**Action**: Remove erroneous "no DCT stego" claim.

### 4. `pipeline.md:24` — Dimension Validation Claim
**Issue**: Claims "no dimension validation" but validation does occur in `process_bytes`.
**Evidence**: `process_bytes` validates `max_dimension` for both JPEG (header parse) and non-JPEG paths.
**Action**: Update claim to reflect validation exists.

### 5. `protected-noise.md` — XorShiftRng Algorithm Description
**Issue**: Documents claim `XorShiftRng` uses XOR operations but code uses `wrapping_add`.
**Evidence**: `util/image.rs:new()` uses `wrapping_add` mixing, not XOR.
**Action**: Correct algorithm description in documentation.

---

## Architecture Documents Without Source Files

None identified. All architecture documents have corresponding source files.

---

## Source Files Without Architecture Documents

None identified. All source modules have corresponding architecture documents.

---

## Verified Accurate Documents

- `error.md` — No stale items found (only dead code identified)
- `async-api.md` — No stale items found
- `cli.md` — Minor line count discrepancy only (545 vs 628 lines)
- `traits.md` — Minor stale references (field/method name mismatches)
- `types.md` — Minor stale references (StegoPayload location)
- `jpeg-header.md`, `jpeg-entropy.md`, `jpeg-stego-f5.md`, `jpeg-transcoder.md` — Highly accurate
- `util-seed.md`, `util-iscc.md`, `constants.md` — No stale references
- `protected-metadata-trap.md`, `protected-steganography.md`, `protected-passthrough.md`, `protected-enhanced.md` — No stale references identified

---

## Summary

| Document | Status | Issues |
|----------|--------|--------|
| `protected-precomputed.md` | Stale | Cache description outdated |
| `overview.md` | Stale | Parallel threshold calculation wrong |
| `pipeline.md` | Stale | DCT stego and validation claims incorrect |
| `protected-noise.md` | Stale | XorShiftRng algorithm description wrong |
| All other docs | Current | Minor discrepancies only |
