# Tooling (Scripts, CI, Benchmarks)

**Sources:** `scripts/` (7 scripts) · `.github/workflows/` (4 workflows) · `benches/bench.rs` (592 lines, Criterion).

Specialist checks are manual and never part of `check.sh` or required CI without a maintainer decision.

## Scripts

| Script | Purpose | Needs |
|--------|---------|-------|
| `check.sh` | Fast check, mirrors required CI: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo check -p stegoeggo --no-default-features`, `cargo test --workspace --exclude stegoeggo-fuzz --all-features` | stable Rust |
| `release-check.sh [--allow-dirty] [--stage=pre\|root\|cli]` | Local pre-release readiness (staged pre/root/cli); never publishes, tags, or pushes | — |
| `check_fuzz_sync.sh` | Verifies dispatch-target parity between `fuzz/Cargo.toml` and `fuzz.yml` (the scheduled smoke rotation derives its list at runtime from `cargo fuzz list`) | — |
| `verify_metadata_conformance.sh [--strict]` | External metadata conformance via the Rust harness + ExifTool/xmllint diff | exiftool, xmllint, imagemagick, libvips |
| `validate-docs-rs.sh` | Docs.rs-equivalent rustdoc validation | nightly |
| `validate-msrv-package.sh` | Fresh MSRV consumer resolution | Rust 1.87 |
| `measure_binary_size.sh` | Built binary size measurement | — |

Manual-only: `cargo deny check licenses|advisories`, `cargo semver-checks check-release`.

## CI

| Workflow | Trigger | Role |
|----------|---------|------|
| `ci.yml` | push/PR to `main` | **Required:** single `Check` job, stable Linux x86_64, `./scripts/check.sh` |
| `assurance.yml` | weekly + manual | MSRV 1.87 matrix + stable compile+tests on Linux aarch64, macOS aarch64, Windows x86_64. Informational only |
| `external-verification.yml` | monthly + manual | External-tool conformance. Informational only |
| `fuzz.yml` | weekly + manual single-target dispatch (+ `smoke=true` rotating 3-target subset, 120 s each, week-of-year rotation, full cycle 4 weeks) | Informational only; crash artifacts uploaded |

No workflow publishes crates or reacts to tags. Releases are manual: carrier → library → CLI, exact `=X.Y.Z` deps (see `RELEASING.md`).

## Benchmarks

`benches/bench.rs` (Criterion, `harness = false`): pipeline sizes, protection levels, byte processing, large images, format preservation, allocations, memory usage, JPEG fast path, tiled embed/extract, LSB clone-vs-in-place, metadata-only, request-vs-legacy, JPEG verify, tiled LSB request. The `lsb_clone_vs_in_place` bench uses Criterion batching so each in-place iteration starts from a pristine source and the preparation clone stays outside the timed section.
