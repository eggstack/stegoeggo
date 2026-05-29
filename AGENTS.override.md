# AGENTS.override.md

Session-specific learnings and corrections for future agents.

## Plan Implementation Complete (2026-05-29)

All 5 tasks from `plans/plan.md` Wave 1 implemented and merged to master.

| Task | Commit | Notes |
|------|--------|-------|
| Dimension validation in `process_bytes` | `321f825` | `validate_jpeg_dimensions_from_bytes()` + validation on non-JPEG path |
| LRU eviction for PrecomputedProtector | `14121aa` | `lru` 0.12, default capacity 100, `peek()` for reads |
| Seed embedding unit quant error | `ef5c249` | Precondition check fails if any value < 2 |
| `Option<bool>` documentation | `c296213` | Three-state semantics on fields, builders, getters |
| CLI batch filename collisions | `237fe23` | `HashMap<PathBuf, usize>` collision tracking |

Full test suite: 264 tests pass. Clippy clean. Format clean.

## Architecture Review Session (2026-05-29)

Reviewed all 7 plan files in `plans/` using subagents to avoid context window exhaustion:
- `stale-items.md`, `core-pipeline-review.md`, `async-cli-review.md`, `types-traits-review.md`, `utilities-review.md` (batch 1)
- `jpeg-transcoder-review.md`, `error-review.md`, `protected-review.md` (batch 2)

### Critical Bugs Found (not yet implemented)

1. **`embed_jpeg_stego` redundancy embedding broken** (`steganography.rs:788-841`): Only first pass embeds. `embedded` counter not reset between passes. Add `embedded = 0;` after line 840.

2. **JPEG segment truncation silent data loss** (`metadata_trap.rs:326-329`): Returns partial data + breaks instead of `Error::ImageTruncated` when parsing truncated JPEG.

### Other Issues Identified

3. **Division by zero potential** (`util/image.rs:560`): `apply_perturbation` divides by `divisor` with no validation. Add `debug_assert!(divisor != 0)`.

4. **JPEG segment length bounds** (`steganography.rs:162-165`): Malformed segment can cause slice panic. Add validation before slicing.

5. **`From<TranscoderError> for Error` not implemented**: Error semantics lost when transcoder errors bubble up.

6. **Dead error variants**: `Dimensions` and `JpegTranscode` defined but never constructed.

7. **`bits_to_bytes` debug_assert panics in release** (`steganography.rs:699-714`): Non-8-multiple input causes release panic.

### Documentation Issues Fixed (per stale-items.md)

- Parallel threshold: 4 cores = 16384, not 65536
- XorShiftRng uses `wrapping_add`, not XOR
- Protected/ diagram grouping doesn't exist (flat struct)
- verify_image_bytes DOES DCT stego verification
- CLI batch does NOT preserve directory structure

### Items Consolidated into plan.md

14 actionable items across 6 waves. See `plans/plan.md` for full implementation order.
