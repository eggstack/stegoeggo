# Plan 084: Large-Module Decomposition and Stego Adapter Deduplication

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/081-pre-v1-consolidation-maintainability-and-portability-roadmap.md`

Audited baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`.

## 1. Objective

Reduce review/maintenance risk in the largest modules and remove residual duplicated adapter code without changing public behavior, public paths, wire formats, or the carrier crate boundary.

This is a structural refactor plan. It must proceed in small, behavior-preserving moves with focused tests after each phase.

## 2. Target modules and decomposition intent

### 2.1 `src/types.rs`

Split by stable conceptual domains while preserving existing `stegoeggo::types::*` paths through `pub use`:

- rights/DMI/policy vocabulary;
- request/channel/preset types;
- processing options and compatibility `ProtectionContext`/`ProtectionLevel` types;
- warnings/execution reports;
- legal metadata and localized text;
- verification compatibility types if they remain housed under `types` after Plan 083.

Do not move public types to new externally visible module paths unless re-exported from `types` exactly as before.

### 2.2 `src/protected/metadata_trap.rs`

Decompose by format/responsibility, e.g. common ownership/update policy, PNG metadata operations, JPEG metadata operations, WebP metadata operations, and shared notice rendering. Preserve `RightsMetadataProtector` as the application facade.

Coordinate with Plan 085: shared low-level container walking belongs to the container/parser layer rather than being duplicated inside each metadata module.

### 2.3 `src/protected/steganography`

Finish adapter simplification. Current plan-based functions should be the implementation source of truth. Context-based compatibility functions should translate to plan/request state or delegate to common private helpers.

Specifically audit and deduplicate:

- `apply_dct_stego_bytes` vs `apply_dct_stego_bytes_from_plan`;
- `apply_to_image_with_summary` vs `apply_lsb_to_image_with_summary_from_plan`;
- repeated `EmbedReport -> EmbedOutcome` conversion;
- repeated tiled/non-tiled status-to-summary conversion;
- identical PNG/WebP raster branches;
- legacy helpers that remain `#[allow(dead_code)]` or compatibility-only.

Create small private conversion helpers rather than adding new public abstractions.

### 2.4 `src/lib.rs`

Extract canonical pipeline execution helpers into a private pipeline module if this materially reduces root-module size and clarifies ownership. Keep public root exports and function paths stable.

Do not move the container-accounting walker under this plan if Plan 085 is implementing it concurrently.

### 2.5 `src/webp_container.rs` and `src/xmp.rs`

Decompose only where cohesive parser/writer/merge/extraction responsibilities are already separable. Avoid a gratuitous file explosion. Preserve private/public visibility and exact metadata semantics.

### 2.6 CLI

CLI physical decomposition is owned by Plan 087; do not edit `stegoeggo-cli/src/main.rs` here except where compilation requires import adjustments caused by internal root refactors.

## 3. Refactor invariants

- No public API removal or rename.
- No payload/frame/carrier byte changes.
- No metadata serialization changes unless tests prove the old/new bytes are semantically and structurally equivalent and the change is intentionally documented.
- No warning severity/category changes.
- No new dependencies.
- Keep module visibility as narrow as possible.
- Prefer mechanical moves first, then local deduplication after tests are green.

## 4. Tests and evidence

Before moving code, record baseline focused test counts in `plans/084-status.md` for request API, cross-format semantics, output-domain routing, metadata update policy, verification, public stego API, WebP/XMP, and legacy compatibility suites.

After each decomposition phase:

- run the directly affected integration tests;
- run `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- at final integration run `./scripts/check.sh`.

Where practical, add equivalence tests around newly centralized helpers rather than snapshotting implementation details.

## 5. Suggested execution waves

Wave 1: split `types.rs` mechanically with re-exports.

Wave 2: split metadata facade by format, avoiding Plan-085-owned traversal code.

Wave 3: deduplicate stego plan/context adapters and report conversions.

Wave 4: extract root pipeline internals and optionally decompose XMP/WebP modules where evidence supports it.

Each wave should be independently mergeable.

## 6. Acceptance criteria

- large modules have cohesive private submodules with stable public re-exports;
- plan-based stego execution is clearly the implementation source of truth;
- repeated carrier report/outcome and raster branch code is materially reduced;
- no new public abstraction was introduced solely for refactoring;
- baseline behavior suites remain green;
- `./scripts/check.sh` passes;
- status ledger records before/after file ownership and any deliberately retained large module with rationale.

## 7. Non-goals

No algorithm optimization, carrier API change, dependency cleanup, CLI contract redesign, verification semantic redesign, or v1 removal work.
