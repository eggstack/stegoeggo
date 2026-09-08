# Plan 082: Canonical Request Auxiliary API Convergence

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/081-pre-v1-consolidation-maintainability-and-portability-roadmap.md`

Audited baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`.

## 1. Objective

Make `ProtectionRequest -> ResolvedProtectionPlan -> canonical execution` the implementation source of truth for synchronous, asynchronous, and parallel/batch protection APIs while preserving all promised 0.x compatibility entry points.

Today the synchronous request API is canonical, but `async_api.rs` and the parallel convenience functions still expose `ProtectionLevel + ProtectionContext` as their primary shape. Those helpers eventually reach the canonical executor, but they encourage new integrations to depend on deprecated concepts and create opportunities for future feature drift.

## 2. Required changes

### 2.1 Add request-based async APIs

Add additive APIs behind `async` using owned input/request values suitable for `spawn_blocking`, including at minimum:

- `process_request_bytes_async(Vec<u8>, ProtectionRequest) -> Result<Vec<u8>>`
- `process_request_bytes_with_report_async(Vec<u8>, ProtectionRequest) -> Result<(Vec<u8>, ExecutionReport)>` or the exact existing synchronous report shape
- request-based warning/report equivalent if warnings remain a distinct compatibility surface

Implementation must call the canonical synchronous request function inside one `spawn_blocking` closure. Do not reimplement resolution, validation, warning policy, format routing, or metadata behavior in `async_api.rs`.

### 2.2 Add request-based parallel/batch APIs

Provide an additive request-based batch surface for callers processing multiple byte buffers. Choose the narrowest useful contract after source audit:

- one shared `ProtectionRequest` applied to all inputs; and/or
- a per-item `(bytes, request)` form only if there is a demonstrated caller need.

The implementation should reuse `process_request_bytes`/report variants through Rayon and preserve deterministic result ordering. Do not create a second batch executor.

### 2.3 Tighten compatibility wrappers

Audit `process_image`, `process_image_bytes`, `process_image_bytes_with_info`, `process_image_bytes_with_warnings`, `ProtectionPipeline::{process,process_bytes}`, and async/parallel legacy wrappers.

The allowed shape is:

```text
legacy arguments
   -> one translation function
   -> ProtectionRequest
   -> canonical request API
```

Any validation, warning construction, policy selection, hidden-marker selection, authentication selection, or routing duplicated outside the canonical request/plan path must either move into request resolution/execution or be explicitly justified as compatibility-only presentation logic.

Keep the 0.x deprecated APIs functional. Do not remove `ProtectionLevel`, `ProtectionContext`, or `EvidenceProfile`.

### 2.4 Establish a no-new-legacy-features invariant

Add architecture/developer documentation stating that new processing features must be expressed in `ProtectionRequest`/`ProcessingOptions`/`ProtectionChannels` first. Legacy context builders may only translate into those fields when compatibility requires it.

## 3. Tests

Add focused equivalence tests proving that, for representative PNG/JPEG/WebP inputs and metadata-only, seed-only, best-effort, tiled, and authenticated presets where applicable:

- synchronous request API == request-based async API output/report;
- synchronous request API == request-based parallel API per item;
- legacy API translation == explicitly constructed equivalent request;
- errors and warnings are equivalent, including missing MAC key, metadata-disabled contradictions, invalid format, capacity degradation, and resource-limit failures.

Do not assert nondeterministic timestamps/seeds without using existing deterministic overrides.

## 4. Documentation

Update `docs/rust-api.md`, `architecture/async-api.md`, `STABILITY.md`, `DEPRECATIONS.md`, and crate docs so request-based APIs are the recommended examples. Legacy forms should remain documented only in migration/compatibility sections.

## 5. Acceptance criteria

- request-based async API exists and contains no policy/routing duplication;
- request-based batch/parallel API exists at the minimal useful surface;
- legacy helpers translate once into the canonical request model;
- focused equivalence tests cover all supported output domains and warning/error behavior;
- no deprecated public API is removed;
- `cargo test --workspace --all-features` and `./scripts/check.sh` pass;
- `plans/082-status.md` records exact API names, tests, and any intentionally retained compatibility exception.

## 6. Non-goals

No async codecs, streaming image protocol, new executor abstraction, generic task runtime, carrier changes, wire-format changes, or v1 API removals.
