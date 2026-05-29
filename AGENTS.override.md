# AGENTS.override.md

Session-specific learnings and corrections for future agents.

## Plan Review Session (2026-05-29)

### Bug Verification Results

Verified all 5 bugs in `plans/plan.md` against the codebase:

| Bug | Verified | Notes |
|-----|----------|-------|
| `process_bytes` dimension validation | **Real** | `validate_dimensions()` only called in `process()`, not `process_bytes()`. JPEG fast path needs header parse for dimensions. |
| PrecomputedProtector unbounded cache | **Real** | `RwLock<HashMap>` with zero eviction mechanisms. Each entry = `width * height * 4` bytes. |
| Seed embedding silent failure | **Real** | `&= 0xFE` on value 1 → 0 → clamped to 1. 0-bits can never embed. `Ok(())` returned even on failure. |
| `Option<bool>` ambiguity | **Not a bug** | Deliberate three-state pattern (`None` = level default, `Some(true/false)` = override). Real issue is documentation gap. |
| CLI filename collision | **Real** | `file_stem()` + `_protected.` + ext. Same-stem files silently overwrite. |

### Implementation Notes

- **Bug 1 fix location**: `apply_multi_protector_bytes()` at `src/lib.rs:346`. Non-JPEG path can use `load_image_from_bytes` + `validate_dimensions`. JPEG fast path can use `jpeg_transcoder::header::parse()` for dimensions without full decode.
- **Bug 2 fix**: Add `lru` crate dependency. Replace `HashMap` with `LruCache` behind `RwLock`. Consider configurable capacity.
- **Bug 3 fix**: Fail fast if any quantization value < 2, or return error when embedding fails. Existing test at line 592 uses `[2; 64]` to avoid this edge case.
- **Bug 4 fix**: Doc comments only. Add three-state explanation to `with_metadata_injection()` and `with_legal_claims()` builder methods.
- **Bug 5 fix**: Thread a `HashSet<PathBuf>` through the batch loop in `cloakrs-cli/src/main.rs:451`. Append numeric suffix on collision.

### Parallelization

All 5 tasks are fully independent (different files, no shared state). Assign to 5 parallel sub-agents. Each runs `cargo test` + `cargo clippy` independently.

### Architecture Docs

All 21 files in `architecture/` have been verified against source code. No cross-references to plan files exist in architecture docs. The `architecture/protected-precomputed.md` file already documents the unbounded cache warning.
