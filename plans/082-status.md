# Plan 082 Status: Canonical Request Auxiliary API Convergence

Status: IMPLEMENTED

Baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`.
Head: `60cc150 plans: add pre-v1 consolidation and maintainability roadmap` plus working tree below.

## Exact API names

### New request-based async (`async` feature, `src/async_api.rs`)

- `process_request_bytes_async(Vec<u8>, ProtectionRequest) -> Result<Vec<u8>>`
- `process_request_bytes_with_warnings_async(Vec<u8>, ProtectionRequest) -> Result<(Vec<u8>, Vec<ProtectionWarning>)>`
- `process_request_bytes_with_report_async(Vec<u8>, ProtectionRequest) -> Result<(Vec<u8>, ExecutionReport)>`
- `process_request_bytes_parallel_async(Vec<Vec<u8>>, ProtectionRequest) -> Result<Vec<Vec<u8>>>` (`async` + `parallel`)
- `process_request_bytes_with_warnings_parallel_async(Vec<Vec<u8>>, ProtectionRequest) -> Result<Vec<(Vec<u8>, Vec<ProtectionWarning>)>>` (`async` + `parallel`)
- `process_request_bytes_with_report_parallel_async(Vec<Vec<u8>>, ProtectionRequest) -> Result<Vec<(Vec<u8>, ExecutionReport)>>` (`async` + `parallel`)

Each calls its synchronous canonical counterpart inside one `spawn_blocking` closure. No policy, routing, validation, or warning duplication in `async_api.rs`.

### New request-based batch (`parallel` feature, `src/lib.rs`)

- `process_request_bytes_parallel(&[Vec<u8>], &ProtectionRequest) -> Result<Vec<Vec<u8>>>`
- `process_request_bytes_with_warnings_parallel(&[Vec<u8>], &ProtectionRequest) -> Result<Vec<(Vec<u8>, Vec<ProtectionWarning>)>>`
- `process_request_bytes_with_report_parallel(&[Vec<u8>], &ProtectionRequest) -> Result<Vec<(Vec<u8>, ExecutionReport)>>`

Minimal useful surface: one shared `ProtectionRequest` applied to all inputs. No per-item `(bytes, request)` form — no caller need demonstrated (CLI and library callers use one request per batch). Each reuses `process_request_bytes*` through Rayon, preserves input order, no second batch executor.

### Tightened wrappers (`src/lib.rs`)

- `process_image_bytes` — already `request_from_legacy()` → `process_request_bytes`; unchanged.
- `process_image_bytes_with_warnings` — now `request_from_legacy()` → `process_request_bytes_with_warnings`, then prepends only compatibility presentation warnings. Removed duplicated `MetadataInjectionDisabled` push (covered by `plan.warnings()`), removed LSB capacity preflight (covered by runtime `warnings_from_embed_outcome`), removed manual `resolve_request` / `OperationObserver` / `process_plan_bytes` handling.
- `process_image_bytes_with_info` — unchanged (delegates to tightened `with_warnings`).
- `process_image_ref` (backs `process_image` and `ProtectionPipeline::process`) — now `request_from_legacy()` → `resolve_request()` → dispatch on `plan.channels().hidden_marker` using `plan.seed()` / `plan.processing()` / `*_from_plan` stego helpers. No independent Light/Standard marker selection.
- `ProtectionPipeline::process_bytes` — already delegates to `process_image_bytes`; unchanged.

## Tests

New `tests/request_aux_convergence.rs` (24 tests, all pass with `--all-features`):

- `sync_variants_agree_{png,jpeg,webp}` — plain / warnings / report bytes and warnings agree for metadata-only, seed-only, best-effort, tiled, authenticated requests.
- `parallel_matches_sync_per_item`, `parallel_warnings_match_sync`, `parallel_report_matches_sync`, `parallel_preserves_order`, `parallel_error_matches_sync_error`, `parallel_tiled_matches_sync` (`parallel` gate).
- `async_matches_sync_png`, `async_matches_sync_jpeg_webp`, `async_warnings_and_report_match_sync`, `async_errors_match_sync`, `async_parallel_matches_sync_parallel` (`async` gate, Tokio).
- `legacy_standard_matches_explicit_request_png`, `legacy_light_matches_explicit_request_png` — legacy bytes == explicitly built equivalent request bytes (deterministic seed + `timestamp_override`).
- `legacy_warnings_are_canonical_plus_compat_jpeg` — canonical warnings ⊆ legacy warnings; `JpegReencodeFragile` present as compat extra.
- `legacy_missing_mac_key_is_compat_only`, `legacy_contradictory_claims_is_compat_only` — profile-driven / contradictory warnings only on legacy path.
- `invalid_format_errors_match`, `hmac_without_key_errors_match`, `tiny_image_capacity_warning_in_both_paths`, `resource_limit_errors_match`, `metadata_disabled_warning_matches`.

Determinism: all equivalence inputs use explicit `.with_seed(42)` and `.with_timestamp_override("2025-01-01T00:00:00Z")`.

## Intentionally retained compatibility exceptions

- `MissingMacKey` — emitted only by legacy `process_image_bytes_with_warnings` when `EvidenceProfile::{AuthenticatedProvenance, Maximal}` was selected without a key. Canonical HMAC-without-key is `Error::Config`, not a warning.
- `ContradictoryLegalClaims` — emitted only by legacy wrapper for `with_legal_claims(false)` + non-empty legal metadata. Canonical requests have no disable flag; presence means include.
- `JpegReencodeFragile` — emitted only by legacy wrapper for non-`Disabled` JPEG output. Canonical path reports only resolution + runtime warnings.
- Ordering: compat warnings are prepended before canonical warnings to preserve 0.x `process_image_bytes_with_info` first-warning behavior (verified by `tests/integration.rs::progressive_jpeg_warning`).
- Pixel-only `DynamicImage` APIs (`process_image`, `process_images_parallel`, `process_image_async`, `process_images_parallel_async`) remain legacy-only; no request-based `DynamicImage` form because file metadata cannot survive the pixel roundtrip.
- Removed dead legacy helpers `SteganographyProtector::lsb_pixels_needed(ctx)` and `payload_bits_for_context(ctx)` after preflight removal; `lsb_pixels_needed_from_plan` retained.

No deprecated public API removed.

## Docs

- `src/lib.rs` crate docs: feature table, async/parallel/warnings examples now request-based; no-new-legacy-features invariant stated.
- `src/async_api.rs` module docs: canonical vs compatibility split.
- `docs/rust-api.md`: canonical async + parallel sections; compatibility section for legacy wrappers.
- `architecture/async-api.md`: canonical + compatibility function lists, single-`spawn_blocking` batch rationale.
- `architecture/pipeline.md`: canonical vs compatibility function lists, no-new-legacy-features invariant, reverse-proxy guidance now request-based.
- `architecture/resolve.md`: corrected warning inventory (HMAC-without-key etc. are errors, not warnings); documented legacy-only warnings; added invariant and new entry points.
- `architecture/types.md`: warning source note + invariant.
- `architecture/overview.md`: public API layer diagram now canonical-first.
- `STABILITY.md`: stable `process_request_bytes_with_warnings` / `..._with_report`; experimental `async` row (canonical async) and updated `parallel` row (canonical batch).
- `DEPRECATIONS.md`: legacy auxiliary API mapping table (Plan 082).
- `README.md`, `AGENTS.md`: feature tables now name canonical async/parallel APIs; AGENTS policy-first bullet states invariant.
- `.skills/stegoeggo-conventions/SKILL.md`: canonical parallel + async signatures, invariant, legacy `with_warnings` note.
- `architecture/resolve.md` discrepancy fixed as pruning: prior doc claimed resolution emits `MissingMacKey`; source returns `Error::Config`.

## Verification

- `cargo fmt --all -- --check`: pass (after `cargo fmt --all`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass via `./scripts/check.sh`.
- `cargo check -p stegoeggo --no-default-features`: pass via `./scripts/check.sh`.
- `cargo test --workspace --exclude stegoeggo-fuzz --all-features`: pass via `./scripts/check.sh` (includes 24 new convergence tests; 2 pre-existing `progressive_jpeg_warning` with_info ordering tests initially failed after canonical-first reorder, fixed by compat-first prepend preserving 0.x first-warning order).
