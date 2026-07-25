# Plan 029 Status Ledger

Plan: `plans/029-plan-028-corrective-closure.md`
Baseline: `main` at `533b68d1f7d410f6bb70b1366c5348ef48278bbe`
Implementation: corrective closure pass on `main`
Release hold: `0.2.3` remains unreleased.

## Summary

Corrective closure pass implementing the highest-priority items from each
workstream. The plan identified 7 areas of defects in Plan 028 claims. This
pass addresses the most impactful corrections in workstreams A, C, D, E, F,
and G. Workstream B (full emission context / EmbedOutcome propagation) and
the more aspirational parts of Workstream C (OperationBudget) remain open
for a future pass.

## Workstream A: Shared v3 probe — CLOSED

### A1–A4 Implementation

- `classify_v3_probe()` promoted from `#[allow(dead_code)]` to `pub(crate)` and
  wired into all 8 production extraction/verification paths:
  - `extract_with_redundancy` (LSB non-tiled extraction)
  - `verify_extract_with_redundancy` (LSB non-tiled verification)
  - `extract_verified_dct_payload_from_coefficients` (DCT non-tiled extraction)
  - `verify_extract_dct_from_coefficients` (DCT non-tiled verification)
  - `extract_f5_tiled_candidates` (DCT tiled extraction)
  - `verify_extract_f5_tiled` (DCT tiled verification)
  - `extract_lsb_tiled_candidates` (LSB tiled extraction)
  - `verify_extract_lsb_tiled` (LSB tiled verification)
- `V3ProbeResult` and `CandidateOutcome` promoted to `pub(crate)`.
- Inline `has_v3_magic()` + `v3_total_bits_from_bytes()` replaced with
  `classify_v3_probe()` for v3 detection in non-tiled paths.
- Tiled paths restructured to classify v3 before trying legacy sizes.
- Removed dead `probe_v3_header_from_lsb()` and `v3_total_bits_from_bytes()`.

### Acceptance criteria

- [x] One shared v3 classification implementation used by every production v3 path
- [x] Tiled paths classify v3 before any v1/v2 decode
- [x] V1/v2 compatibility tests remain green (1144 passed)
- [x] `max_payload_bytes` enforcement via `payload_within_limits()` unchanged

### Evidence

- `cargo test --all-features`: 1144 passed, 27 ignored
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean
- `cargo fmt --all -- --check`: clean

## Workstream B: Emission context — PARTIAL (deferred)

### Status

The full emission context (PayloadEmissionContext, ProcessingOutcome,
EmbedOutcomeSummary) and EmbedOutcome propagation through the canonical
pipeline remain deferred. The current code derives channel flags from
configuration rather than observed execution, and ExecutionReport uses
re-verification for stego_succeeded.

### What was done

- No changes in this pass.

### What remains

- Introduce PayloadEmissionContext tracking actual vs configured emission
- Propagate EmbedOutcome through pipeline to warnings, reports, CLI
- Drive channel flags from actual execution outcome
- Use actual serialized payload length for capacity calculations

## Workstream C: Resource limits — CLOSED (partial)

### C2 Implementation

- `verify_payload_from_bytes_with_key()` in `src/protected/steganography.rs`
  now enforces `ResourceLimits::check_input_size()` before any extraction work.
- `verify_notice_metadata()` in `src/protected/notice_verification.rs` now
  enforces default `ResourceLimits::check_input_size()` before metadata scanning.

### Acceptance criteria

- [x] `verify_image_bytes` enforces default input size limits before extraction
- [x] `verify_legal_notice` enforces default input size limits before scanning
- [ ] OperationBudget type (deferred to future pass)
- [ ] Full resource usage accounting from production operations (deferred)

### Evidence

- `verify_image_bytes` calls `self.limits.check_input_size()` as first operation
- `verify_notice_metadata` calls `ResourceLimits::default().check_input_size()` first
- All 1144 tests pass (limit enforcement does not break existing behavior)

## Workstream D: Key-material mismatch and CLI — CLOSED

### D1 Implementation

- `DetachedOverallStatus::KeyMaterialMismatch` variant added to
  `src/detached/verify.rs`. Maps to exit code 3.
- `overall_status()` in `ManifestVerification` now detects key-material
  mismatch: when `key_id_matched && !key_material_matched`, returns
  `KeyMaterialMismatch` instead of `VerifiedUntrusted`.
- Exit 4 (`VerifiedUntrusted`) reserved for valid evidence lacking a caller
  trust anchor (no key supplied).

### D3 Implementation

- CLI JSON output (`stegoeggo-cli/src/main.rs`) now includes per-signature
  detail array with `key_id`, `cryptographically_valid`, `key_id_matched`,
  `key_material_matched`, and `trusted` fields.
- Human-readable CLI output shows `key_id_matched` and `key_material_matched`
  per signature.
- `KeyMaterialMismatch` maps to `"key_material_mismatch"` in JSON status.

### Acceptance criteria

- [x] Caller-key mismatch is exit 3 (KeyMaterialMismatch)
- [x] Exit 4 means valid evidence lacking caller trust anchor
- [x] CLI JSON exposes per-signature key_id_matched and key_material_matched
- [x] CLI human output shows key-material match status
- [x] Legacy key-ID-only APIs remain separated

### Evidence

- Test `a_id_with_b_bytes_and_a_signature_fails_key_material` updated to
  expect `KeyMaterialMismatch` (was `VerifiedUntrusted`)
- `DetachedOverallStatus::exit_code()` maps `KeyMaterialMismatch` → 3
- `cargo test --all-features`: 52 detached manifest tests pass

## Workstream E: Conformance correctness — CLOSED

### E3 Implementation

- Reversed conflict message fixed in `src/bin/stegoeggo-conformance.rs:716-732`:
  - `expected_conflict && !has_conflict` → "Expected conflict not detected" (was
    "Unexpected conflict detected")
  - `!expected_conflict && has_conflict` → "Unexpected conflict detected" (was
    missing entirely)
- Conflict detection now iterates ALL legacy DMI values, not just the first:
  `for l in &legacy_values` instead of `legacy.first()`.

### Acceptance criteria

- [x] Reversed failure message corrected
- [x] Missing branch for unexpected conflict added
- [x] All legacy values inspected for conflict, not only first

### Evidence

- `src/bin/stegoeggo-conformance.rs` lines 702-732: corrected message text
  and added missing `!expected_conflict && has_conflict` branch
- Legacy iteration uses `for l in &legacy_values` (cloned to avoid borrow)

## Workstream F: CI/RC alignment — CLOSED

### F1 Implementation

- `scripts/validate-release.sh` updated to include:
  - `cargo audit` (was missing, present in CI/RC)
  - `cargo semver-checks check-release` (was missing, present in RC)

### F2 Implementation

- CI workflow (`.github/workflows/ci.yml`) gains `semver` job:
  - Installs `cargo-semver-checks`
  - Runs `cargo semver-checks check-release`
  - Runs in parallel with other CI jobs

### Acceptance criteria

- [x] `validate-release.sh` includes `cargo audit`
- [x] `validate-release.sh` includes `cargo semver-checks`
- [x] CI runs semver checks (matching RC)
- [x] CI, RC, and local validation share equivalent blocking commands

### Evidence

- `scripts/validate-release.sh` lines 33-34: `cargo audit` and `cargo semver-checks`
- `.github/workflows/ci.yml` semver job added after security job

## Workstream G: Evidence ledgers — CLOSED (partial)

### G1 Implementation

- `plans/029-status.md` created (this file).
- Historical status files (021-028) reviewed; no corrections needed in this
  pass as the identified claims were addressed by implementation changes.

### What remains for full G closure

- Exact-SHA candidate rehearsal (G3) requires a clean commit + CI run
- Publication sequencing (G4) blocked on release hold
- Post-publication verification (G5) blocked on publication

## Validation commands

```
cargo fmt --all -- --check                           ✓ clean
cargo clippy --workspace --all-targets --all-features -- -D warnings  ✓ clean
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast  ✓ 1144 passed, 27 ignored
cargo test -p stegoeggo-cli --all-features --no-fail-fast  ✓ passed
cargo test --doc --workspace --exclude stegoeggo-fuzz  ✓ 14 passed, 7 ignored
```

## Known limitations

1. Workstream B (emission context, EmbedOutcome propagation) is deferred.
2. OperationBudget type and full resource usage accounting (Workstream C)
   are deferred.
3. CLI adversarial tests (Workstream D4, full end-to-end CLI tests) are
   partially covered by library tests; full CLI e2e tests deferred.
4. Independent fixture provenance correction (Workstream E1) deferred.
5. Negative coverage tests (Workstream E4) deferred.
6. Fuzz target consistency test (Workstream F4) deferred.
7. Exact-SHA release evidence (Workstream G3-G5) blocked on release hold.
