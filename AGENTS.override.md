# AGENTS.override.md

Session-specific learnings and corrections for future agents.

## Plan Consolidation Session (2026-05-29)

Consolidated all architecture review plan files into single `plans/plan.md`.
Reviewed and corrected plan claims against current source code.

### Corrections Applied to plan.md

1. **Removed stale-item items as completed**: All items in `stale-items.md` were already fixed in prior sessions (LRU cache, parallel threshold, XorShiftRng docs, etc.)

2. **Removed documentation-only items**: Items 1.2 (JPEG segment bounds - already fixed), 4.2 (LRU cache doc - already fixed), 5.1 (verify_image_bytes doc - already fixed), 5.2 (protection layers doc - already fixed) were removed as they are no longer issues

3. **Removed Wave 5 (Documentation)**: All documentation discrepancies identified in review were already fixed in prior sessions

4. **Corrected Redundancy Bug Description**: The bug is that `embedded` counter is not reset between passes, causing only first pass to embed. Fix is `embedded = 0;` after line 840

5. **Removed duplicate item**: The JPEG segment length bounds issue appears in both steganography.rs (reassembly) and metadata_trap.rs. Consolidated into single item 1A.2

6. **ImageTruncated error variant**: Does not exist in `error.rs` yet — must be added before item 1B.1 can be implemented

7. **Item count corrected**: 11 actionable items across 5 waves (previously claimed 14 across 6 waves)

### New Parallelization Strategy

Wave 1 splits into 3 parallel tracks (different files, no conflicts):
- **Track A** (`steganography.rs`): Items 1A.1, 1A.2
- **Track B** (`metadata_trap.rs`): Item 1B.1 (also needs `error.rs` for ImageTruncated variant)
- **Track C** (`image.rs`): Item 1C.1

### Session Summary

| Action | Status |
|--------|--------|
| plan.md consolidated and updated | ✅ |
| AGENTS.md pruned (removed planning-specific items) | ✅ |
| AGENTS.override.md updated | ✅ |
| Original plan files removed | ⏳ Pending |
| Commit | ⏳ Pending |

### Key Findings from Code Review

1. **`reassemble_jpeg` at steganography.rs:207-209**: Bounds check missing before slice operation `&jpeg_bytes[pos..pos + 2 + segment_len]` — item 1A.2 addresses this

2. **Division by zero**: `apply_perturbation` at image.rs:560 divides by `divisor` without validation — item 1C.1 addresses this with `debug_assert`

3. **Error module**: `Dimensions` and `JpegTranscode` are dead code (never constructed) — items 2.1 and 2.2 address error improvements

4. **`bits_to_bytes` at steganography.rs:699**: Uses `chunks_exact` which panics in release if not multiple of 8 — item 3.1 adds runtime check

### Files Reference

| File | Items |
|------|-------|
| `src/protected/steganography.rs` | 1A.1 (redundancy bug), 1A.2 (segment bounds), 3.1 (bits_to_bytes), 5.1 (redundancy test) |
| `src/protected/metadata_trap.rs` | 1B.1 (truncation error) |
| `src/util/image.rs` | 1C.1 (division by zero) |
| `src/error.rs` | 1B.1 (add ImageTruncated), 2.1 (From<TranscoderError>), 2.2 (remove dead variants), 5.2 (error tests) |
| `cloakrs-cli/src/main.rs` | 4.1 (dedupe helper), 4.2 (better error messages) |

## Prior Sessions

### Plan Implementation Complete (2026-05-29)

All 5 tasks from `plans/plan.md` Wave 1 implemented and merged to master.

| Task | Commit | Notes |
|------|--------|-------|
| Dimension validation in `process_bytes` | `321f825` | `validate_jpeg_dimensions_from_bytes()` + validation on non-JPEG path |
| LRU eviction for PrecomputedProtector | `14121aa` | `lru` 0.12, default capacity 100, `peek()` for reads |
| Seed embedding unit quant error | `ef5c249` | Precondition check fails if any value < 2 |
| `Option<bool>` documentation | `c296213` | Three-state semantics on fields, builders, getters |
| CLI batch filename collisions | `237fe23` | `HashMap<PathBuf, usize>` collision tracking |

Full test suite: 264 tests pass. Clippy clean. Format clean.

### Architecture Review Session (2026-05-29)

Reviewed all 7 plan files in `plans/` using subagents to avoid context window exhaustion.

All findings consolidated into `plans/plan.md`.