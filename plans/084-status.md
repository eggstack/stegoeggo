# Plan 084 Status

Status: COMPLETE

Baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`
Execution base: `e6a4f85` (plan 083) on `main`

## Required evidence

- [x] baseline focused test counts recorded
- [x] `types.rs` decomposed with stable re-exports
- [x] metadata facade decomposed by cohesive responsibility
- [x] stego plan/context adapter duplication reduced
- [x] repeated report/outcome/raster branch conversions centralized
- [x] root pipeline internals decomposed where justified
- [x] XMP/WebP decomposition disposition recorded
- [x] no public path/wire behavior regressions
- [x] `./scripts/check.sh` passes

## Baseline focused test counts (before)

Workspace `-- --list`: 1860. Lib unit: 603. webp|xmp filtered: 223.

| Suite | Count |
|---|---|
| request_api | 80 |
| cross_format_semantics | 34 |
| output_domain_routing | 17 |
| merge_policy (metadata update policy) | 11 |
| verification_report_tests | 19 |
| verification_convergence | 3 |
| public_stego_api | 52 |
| semantic_correctness | 29 |
| plan065_legacy_compat | 10 |
| metadata_only_idempotence | 3 |
| canonical_rights | 14 |

## After (final)

Workspace `-- --list`: 1863 (+3 new `embed_helper_tests`). Lib unit: 606. All
focused suites match baseline exactly. `./scripts/check.sh` exit 0, 42 suites,
0 failures. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
clean, `cargo fmt --check` clean, `cargo check -p stegoeggo --no-default-features`
clean.

## Before/after file ownership

Wave 1 — `src/types.rs` (5411) is now a facade (945 lines, module declarations
plus the original test suite) with stable `stegoeggo::types::*` re-exports over
`src/types/`: `rights.rs` (175, DMI vocabulary), `compat.rs` (291, level/format
compat), `legal.rs` (1092, notice + legal metadata), `context.rs` (888, config +
context), `verification.rs` (1016, verification compat + emission context),
`warnings.rs` (194, warnings), `request.rs` (835, policy/request/plan/report).
One visibility widening, no public change: `RightsNotice` fields are now
`pub(crate)` so `context.rs` can construct the notice (previously same-file
private access).

Wave 2 — `src/protected/metadata_trap.rs` (4256) is now a facade (1937 lines,
struct + dispatch + orchestration + original tests) over
`src/protected/metadata_trap/`: `common.rs` (86, date helpers),
`notice.rs` (627, shared notice rendering + markers + checksums),
`png.rs` (406), `jpeg.rs` (765), `webp.rs` (477). Moved inherent methods are
`pub(super)`; `current_timestamp_iso8601` keeps its
`crate::protected::metadata_trap::` path via re-export. No Plan 085 traversal
code was moved.

Wave 3 — `src/protected/steganography/embed.rs` (493 → 515 incl. 3 new tests):
new shared private helpers `outcome_from_report`, `progressive_fallback`,
`embed_dct_payload`, `embed_dct_tiled_payload`, `inplace_summary`,
`embed_raster_with_seed_fallback`. Context-based DCT functions and the
plan-based DCT function delegate to the same DCT helpers; both LSB entry
points share the raster helper; identical PNG/WebP arms in
`apply_to_image_with_summary` merged into one path; `lsb_in_place_summary` /
`lsb_tiled_in_place_summary` replaced by the single `inplace_summary`;
`embed_seed_lsb_fallback_pub` delegates to `embed_seed_lsb_fallback`.
Retained: `embed_lsb_v2` (test-used, keeps its best-effort error-swallowing
semantics), `lsb_pixels_needed_from_plan` (dead-code legacy capacity estimate
with extension-hazard docs). Plan-based execution remains the source of truth.

Wave 4 — `src/lib.rs` (2417 → 2152) with new private `src/pipeline.rs` (276):
moved `PipelineResult`, `warnings_from_embed_outcome`, `process_plan_bytes`,
`execute_metadata_only`, `execute_full_marker_and_metadata`,
`execute_seed_only_and_metadata`. The container-accounting walker
`observe_metadata_work` stays in `src/lib.rs` for Plan 085. All public root
exports and function paths stable.

Deliberately retained: `src/xmp.rs` (1792: 1052 impl + 740 tests, cohesive
quick-xml filter/merge around one `NsStack`) and `src/webp_container.rs`
(2092: 821 impl + 1271 tests, cohesive RIFF parser). Splitting either would be
gratuitous file explosion per plan section 2.5; test code dominates both
counts.

## Docs updated

`AGENTS.md` (pipeline/types/metadata architecture bullets),
`architecture/pipeline.md`, `architecture/types.md`,
`architecture/protected-metadata-trap.md`,
`architecture/protected-steganography.md`, `architecture/overview.md`
(repository layout + root module map), `.skills/stegoeggo-conventions/SKILL.md`
(pitfalls 13/17, verification-type locations),
`.skills/architecture-review/SKILL.md` (key source files, line-number pruning).
`README.md` needs no change (no file-layout references).

## Notes

Behavior-preserving structural refactor only: no public API removal or rename,
no payload/frame/carrier byte changes, no metadata serialization changes, no
warning severity changes, no new dependencies. CLI untouched (Plan 087 owns
it). Status ledger is the authoritative record; final commit SHA recorded at
merge time.

Final commit: `6b469f8` on `main`, pushed to `stegoeggo/main`, CI green
(3m29s).
