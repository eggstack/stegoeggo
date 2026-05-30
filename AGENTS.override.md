# AGENTS.override.md

Session-specific learnings and corrections for future agents.

## Plan Implementation Session (2026-05-30)

All 11 items from `plans/plan.md` implemented and merged to master.

### Items Completed

| Wave | Item | Implementation |
|------|------|----------------|
| 1A | 1A.1 Redundancy Bug | Added `embedded = 0;` reset in `embed_jpeg_stego` |
| 1A | 1A.2 Segment Length Bounds | Added bounds check in `reassemble_jpeg` |
| 1B | 1B.1 JPEG Truncation | Added `ImageTruncated` error variant, fixed `inject_text_chunks_jpeg` and `extract_seed_from_jpeg` |
| 1C | 1C.1 Division by Zero | Added `debug_assert!(divisor != 0)` to `apply_perturbation` and `apply_perturbation_par` |
| 2 | 2.1 From<TranscoderError> | Implemented with proper variant mappings, updated callers to use `?` |
| 2 | 2.2 Remove Dead Variants | Removed `Dimensions` and `JpegTranscode`, updated `architecture/error.md` |
| 3 | 3.1 bits_to_bytes Runtime Check | Replaced `debug_assert!` with runtime check returning empty Vec |
| 4 | 4.1 Extract Batch Helper | Extracted `compute_output_path` helper in CLI, reduced ~50 lines duplication |
| 4 | 4.2 Batch Error Messages | Changed `failed_count` to `failed_files: Vec<PathBuf>` |
| 5 | 5.1 Redundancy Test | Added `jpeg_stego_redundancy_extraction_succeeds` and `jpeg_stego_redundancy_multiple_extraction_seeds_work` |
| 5 | 5.2 Error Variant Tests | Added 17 tests for error variant construction and Display/From implementations |

### Parallelization Notes

- Wave 1 used 3 worktrees (Track A: steganography.rs, Track B: metadata_trap.rs+error.rs, Track C: image.rs)
- Waves 2-5 used sequential worktrees after fixing 1A.2 conflict with main
- task3 (Wave 3) required re-commit on main due to merge conflict resolution

### Test Suite
- 264+ tests pass (168 unit + 9 async + 63 integration + 20 basic + 4+ doc-tests)
- Clippy clean, format clean

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

| File | Items (Updated) |
|------|----------------|
| `src/protected/steganography.rs` | 1A.1 (redundancy bug), 1A.2 (segment bounds), 3.1 (bits_to_bytes), 5.1 (redundancy test) |
| `src/protected/metadata_trap.rs` | 1B.1 (truncation error) |
| `src/util/image.rs` util/image.rs | 1C.1 (division by zero) |
| `src/error.rs` | 1B.1 (add ImageTruncated), 2.1 (From<TranscoderError>), 2.2 (removed Dimensions, JpegTranscode), 5.2 (error tests) |
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