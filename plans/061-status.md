# Plan 061 Status — COMPLETE

Baseline SHA: current HEAD

## Summary

Plan 061 is complete. The canonical request path (`ProtectionRequest -> ResolvedProtectionPlan -> execution`) no longer reconstructs `ProtectionContext` for metadata injection. The direct plan executor handles metadata-only, BestEffort, and Tiled paths using plan fields directly.

## Key Changes

### 1. `RightsMetadataProtector::inject_bytes_from_plan()` (src/protected/metadata_trap.rs)

New plan-aware entry point that takes `&ResolvedProtectionPlan` directly:
- Uses `plan.effective_notice()` for notice fields (copyright, license, etc.)
- Uses `plan.effective_dmi()` for DMI markers (EXIF, IPTC, XMP)
- Uses `plan.channels().rights_metadata` for injection decision
- Uses `plan.processing().metadata_update_policy` for conflict handling
- Uses `plan.seed()` for seed injection
- Uses `plan.resource_limits()` for container limit checks
- Creates a minimal `ProtectionContext` with only resource limits for JPEG format methods

### 2. Direct plan executor functions (src/lib.rs)

Three new crate-private functions replace the old `process_plan_bytes` → `plan_to_context` → pipeline path:

- `execute_metadata_only()` — Same-format and cross-format metadata injection using plan directly
- `execute_stego_and_metadata()` — Standard hidden marker: DCT/LSB stego + metadata injection
- `execute_stego_and_metadata_tiled()` — Tiled variant for crop-resistant mode

### 3. `process_plan_bytes()` rewritten (src/lib.rs)

Now routes through the direct executor functions instead of `plan_to_context()`:
- Metadata-only / Disabled → `execute_metadata_only()`
- BestEffort → `execute_stego_and_metadata()`
- Tiled → `execute_stego_and_metadata_tiled()`

### 4. Resolver now produces complete effective notice (src/protected/resolve.rs)

The resolver merges `request.legal_metadata()` fields into `effective_notice` via `RightsNotice::with_legal_metadata_fields()` and sets the seed, so the plan's effective notice is self-contained for metadata injection.

### 5. Dead code removed

- `ProtectionPipeline::process_metadata_only()` — replaced by `execute_metadata_only()`
- `ProtectionPipeline::observe_metadata_work()` — replaced by standalone `observe_metadata_work()`
- `ProtectionPipeline::process_bytes_pipeline()` — no longer called from canonical path

### 6. `plan_to_context()` retained for legacy compatibility

Still used by `execute_stego_and_metadata*` for steganography methods that require `&ProtectionContext`. Marked compatibility-only. Will be removed when steganography methods accept plan directly.

## plan_to_context() remaining call sites

1. `execute_stego_and_metadata()` — steganography DCT/LSB methods
2. `execute_stego_and_metadata_tiled()` — steganography DCT/LSB methods
3. `plan_to_context()` definition — retained for legacy + steganography

## Entry Point Inventory (post-change)

| Function | Input | Config type | Uses plan_to_context | Notes |
|----------|-------|-------------|---------------------|-------|
| `process_request_bytes*` | bytes | Canonical | NO (metadata) / YES (stego via plan) | Metadata path fully plan-aware |
| `process_image_bytes*` | bytes | Legacy | No (own pipeline) | Legacy path unchanged |
| `process_image` | DynamicImage | Legacy | No (own pipeline) | Legacy path unchanged |
| `ProtectionPipeline::process_bytes` | bytes | Legacy | No (own pipeline) | Legacy path unchanged |
| CLI `process_single_file` | bytes | Canonical | Same as `process_request_bytes_with_warnings` | Uses canonical path |

## Acceptance Criteria Met

1. ✅ `ResolvedProtectionPlan` consumed directly by canonical execution
2. ✅ Canonical paths no longer reconstruct `ProtectionContext` for metadata
3. ✅ Carrier selection and metadata-only decisions in direct plan executor
4. ✅ Legacy APIs retain their own independent pipeline (unchanged behavior)
5. ✅ Every execution-relevant field mapped: seed, intensity, DMI, legal metadata, quality, progressive, max_dimension, tile_size, MAC key, resource limits, metadata_update_policy
6. ✅ 1557 tests pass, 0 failures
7. ✅ `./scripts/check.sh` passes (fmt, clippy, no-default-features, tests)
8. ✅ No public API removal, no dependency changes, no CI changes
