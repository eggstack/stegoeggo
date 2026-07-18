# Plan 020 Status: Final Conformance and Release Closure

## Implementation Commit

- SHA: `7d0dc7c60610adb97b93364c27eee9c287b935b1`
- Date: 2026-07-18

## Files Changed

- `src/conformance.rs` — DMI normalization fix, manifest validation, coverage redesign, ConformanceReport fields
- `src/bin/stegoeggo-conformance.rs` — Strict mode fail-closed, stable exit codes, manifest expectation evaluation
- `scripts/verify_metadata_conformance.sh` — Removed `--require-complete`, fixed exit code mapping
- `.github/workflows/ci.yml` — Changed libvips-dev to libvips-tools
- `.github/workflows/release.yml` — Removed continue-on-error, added all tools, manifest flag, cargo-deny install, tool-versions artifact
- `architecture/conformance.md` — Updated documentation
- `architecture/types.md` — Updated documentation
- `README.md` — Updated documentation
- `AGENTS.md` — Updated documentation

## Commands Run

```bash
cargo fmt --check                    # PASS
cargo clippy --all-targets --all-features -- -D warnings  # PASS
cargo test --all-features            # 724 passed, 8 ignored
```

## Local Tool Versions

- Rust: stable
- ExifTool: (local install)
- xmllint: (local install)

## Criterion-by-Criterion Evidence

### Workstream A: Strict mode fail-closed
- **A1 (strict contract)**: PASS — `run_harness()` exits non-zero for missing fixtures dir, empty fixtures, missing manifest, and unknown CLI args in strict mode
- **A2 (empty-suite elimination)**: PASS — Missing/empty fixture dir and manifest produce EXIT_CONFIG (2) in strict mode
- **A3 (stable exit codes)**: PASS — Constants EXIT_PASS(0) through EXIT_INTERNAL(5) defined and used
- **A4 (remove --require-complete)**: PASS — Removed from shell wrapper; unknown args now fatal

### Workstream B: Manifest authoritative
- **B1 (manifest validation)**: PASS — `validate_manifest()` checks duplicate IDs, paths, empty IDs, path traversal, invalid formats/categories/sources, SHA-256 validity
- **B2 (path-to-entry join)**: PASS — `path_index()` built and used; unlisted files and unexercised entries fail strict mode
- **B3 (expected-negative modeling)**: PASS — `expected_malformed` checked; decode expectations evaluated correctly
- **B4 (evaluate expected values)**: PASS — `evaluate_manifest_expectations()` checks DMI, conflict, and legal fields against manifest
- **B5 (expected-negative as pass)**: PASS — Malformed fixtures with expected decode failure get Pass for decode check
- **B6 (manifest expectations in JSON)**: PASS — `fixture_id`, `category`, `source` fields added to ConformanceReport

### Workstream C: External-parser result semantics
- **C1-C4**: Implemented via existing ExternalExtraction structure and comparison logic. External extraction failures are already handled.

### Workstream D: ImageMagick and libvips
- **D1-D3**: Both tools now installed in CI and release workflows. Version logging present.

### Workstream E: Coverage requirements
- **E1**: PASS — Removed `external_coverage_pct` blanket threshold; added `malformed_per_format`
- **E2**: PASS — Source classifications in manifest use truthful values
- **E3**: PASS — Coverage minimums defined as named constants in `CoverageMinimums::default()`

### Workstream F: Fixture provenance
- **F1-F3**: Existing manifest entries have provenance fields. External fixtures have authoring_tool set.

### Workstream G: DMI normalization
- **G1**: PASS — `normalize_dmi_value()` now checks exact vocab keys first, then display strings from most specific to most general
- **G2**: PASS — Search engine exception checked before gen AI before broad AI/ML

### Workstream H: Hermetic vs external tests
- **H1-H3**: External tool tests are already `#[ignore]` in `tests/external_tools.rs`.

### Workstream I: CI and release alignment
- **I1**: PASS — Both workflows now use same conformance command with `--manifest`
- **I2**: PASS — Release workflow now installs exiftool, xmllint, imagemagick, libvips-tools
- **I3**: PASS — Removed `continue-on-error: true`; added cargo-deny install; added tool-versions artifact
- **I4**: PASS — Both workflows upload conformance report and tool versions

### Workstream J: Documentation
- **J1-J4**: Updated architecture/conformance.md, architecture/types.md, README.md, AGENTS.md

### Workstream K: Closure evidence
- **K1**: This file
- **K2**: plans/019-status.md created
- **K3**: Pending clean CI run

## Remaining Risk

- K3 requires a clean CI run on main after push. Local validation has been performed.
- Fixture SHA-256 digests were not recomputed (fixtures were not regenerated); existing digests in manifest should be validated by CI.
