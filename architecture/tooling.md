# Tooling (Scripts, CI, Benchmarks)

**Sources:** `scripts/` (15 scripts: 13 shell, 2 Python) · `packaging/` (2 installers) · `.github/workflows/` (12 workflows) · `.github/actions/` (2 local composite actions for shared Linux release bootstrap) · `benches/bench.rs` (592 lines, Criterion).

Specialist checks are manual and never part of `check.sh` or required CI without a maintainer decision.

## Scripts

| Script | Purpose | Needs |
|--------|---------|-------|
| `check.sh` | Fast check, mirrors required CI: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo check -p stegoeggo --no-default-features`, `cargo test --workspace --exclude stegoeggo-fuzz --all-features`, `./scripts/check-docs-contract.sh` | stable Rust |
| `release-check.sh [--allow-dirty] [--stage=pre\|root\|cli]` | Local pre-release readiness (staged pre/root/cli); never publishes, tags, or pushes | — |
| `release-binary-preflight.sh --tag=vX.Y.Z` | Validates binary-release tag/version lockstep, target manifest, updater mapping, and canonical CLI feature set; optional `--asset-dir` audits built assets | stable Rust, Python |
| `release-check-assets.sh --dir=DIR` | Verifies the exact five target executables, correctly named SHA-256 sidecars, installers, and absence of extra release files; optional version/native smoke checks | `sha256sum` or `shasum` |
| `smoke-release-binary.py PATH` | Bounded release-candidate validator: exact `stegoeggo X.Y.Z` version identity, `--help`, `protect`/`inspect`/`verify` semantics against the canonical fixture, and the Linux GLIBC 2.17 ceiling. Registered as the consumer validator for all five targets | Python 3; `readelf` for ELF candidates |
| `test-release-installers.sh` | Local fixture/server tests for installer mapping, URLs, checksums, identity, fallback, and destinations | Bash, Python, curl |
| `test-release-updater.sh` | Local fixture/server rehearsal for current/no-op, verified old→new update, and validation-failure preservation through the native eggfetch transport | Bash, Python, built CLI |
| `check-docs-contract.sh` | Fast deterministic check for installer URLs, canonical commands, target names, release-policy wording, and release-workflow matrix/source-path parity | Bash, Python |
| `check-release-contract.py` | Release-contract guard: expands the `release/eggpack/distribution.toml` target/asset set and asserts parity with `release-binaries.yml`, both installers, updater asset naming, `README.md`/`docs/installation.md` installer URLs, qualification and consumer-validator entries, and the cross-toolchain pins. Run by `release-drift.yml` on push/PR | Python 3.11+ (`tomllib`), stdlib only |
| `check-release-workflow-contract.py` | Manual-only guard for release/binding workflow action SHA pins plus target/artifact/toolchain equivalence (never required CI) | Python, PyYAML |
| `check_fuzz_sync.sh` | Verifies dispatch-target parity between `fuzz/Cargo.toml` and `fuzz.yml` (the scheduled smoke rotation derives its list at runtime from `cargo fuzz list`) | — |
| `verify_metadata_conformance.sh [--strict]` | External metadata conformance via the Rust harness + ExifTool/xmllint diff | exiftool, xmllint, imagemagick, libvips |
| `validate-docs-rs.sh` | Docs.rs-equivalent rustdoc validation | nightly |
| `validate-msrv-package.sh` | Fresh MSRV consumer resolution | Rust 1.89 |
| `measure_binary_size.sh` | Built binary size measurement | — |

Manual-only: `cargo deny check licenses|advisories`, `cargo semver-checks check-release`.

## CI

| Workflow | Trigger | Role |
|----------|---------|------|
| `ci.yml` | push/PR to `main` | **Required:** single `Check` job, stable Linux x86_64, `./scripts/check.sh` |
| `assurance.yml` | weekly + manual | MSRV 1.89 matrix + stable compile+tests on Linux aarch64, macOS aarch64, Windows x86_64. Informational only |
| `external-verification.yml` | monthly + manual | External-tool conformance. Informational only |
| `fuzz.yml` | weekly + manual single-target dispatch (+ `smoke=true` rotating 3-target subset, 120 s each, week-of-year rotation, full cycle 4 weeks) | Informational only; crash artifacts uploaded |
| `release-binaries.yml` | manual dispatch for an existing `vX.Y.Z` release tag | Attaches five CLI binaries, sidecars, and installers; never publishes crates |
| `c-binding.yml` | push/PR to `main`, path-filtered (`bindings/c/**`, `src/**`, `stegoeggo-stego/**`, `Cargo.toml`, `Cargo.lock`, `tests/fixtures/**`, itself) | Binding-specific C ABI signal, independent of `ci.yml`: Linux x86_64, cbindgen header drift check, 90-symbol export-manifest audit, native C11/C++17 consumer suite, Rust↔C parity. Never publishes |
| `node-binding.yml` | push/PR to `main`, path-filtered (`bindings/node/**`, `src/**`, `stegoeggo-stego/**`, `Cargo.toml`, `Cargo.lock`, `tests/fixtures/conformance/canonical/**`, itself) | Builds the napi-rs addon once (Rust 1.89 + Node 24), checks generated-declaration drift, the Node suite, and the TypeScript consumer contract, then reuses the addon for a dependency-free runtime smoke on Node 22/24/26. Never publishes to npm |
| `python-binding.yml` | push/PR to `main`, path-filtered (`bindings/python/**`, `src/**`, `stegoeggo-stego/**`, `Cargo.toml`, `Cargo.lock`, `bindings/python/tests/fixtures/**`, itself) | Single Linux x86_64 / Python 3.11 job; builds and installs a real wheel rather than `maturin develop` (which `actions/setup-python` cannot host), then pytest. Never publishes wheels or sdists |
| `release-c.yml` | manual dispatch (`source_ref`) | Five-platform native C qualification (Linux x86_64/aarch64, macOS x86_64/arm64, Windows MSVC); artifacts only, never publishes |
| `release-node.yml` | manual dispatch (`source_ref`) | Five-target napi-rs addon build, per-target native smoke, and package assembly; artifacts only, never publishes to npm |
| `release-python.yml` | manual dispatch (`source_ref`) | Five-platform cibuildwheel wheels plus a separate sdist build; artifacts only, never publishes to PyPI |
| `release-drift.yml` | push/PR to `main` + manual | `Drift` / "Eggpack drift + contract" on ubuntu-latest, 30-minute timeout: pinned `eggpack ci check` against `release/eggpack/` plus `scripts/check-release-contract.py` |

No workflow publishes crates or reacts automatically to tags. Releases remain
manual: carrier → library → CLI for crates.io, with the separate binary
workflow attaching GitHub Release assets (see `RELEASING.md`).

`bindings/` is excluded from the Cargo workspace (root `Cargo.toml`
`exclude = ["bindings"]`), so `check.sh` and its `cargo test --workspace` never
build or test a binding. The three binding CI workflows are independent signals
outside the required gate, and their C/Node/Python tooling is not added to
`check.sh`. Deep dives: [bindings-c.md](bindings-c.md),
[bindings-node.md](bindings-node.md), [bindings-python.md](bindings-python.md).

## Benchmarks

`benches/bench.rs` (Criterion, `harness = false`): pipeline sizes, protection levels, byte processing, large images, format preservation, allocations, memory usage, JPEG fast path, tiled embed/extract, LSB clone-vs-in-place, metadata-only, request-vs-legacy, JPEG verify, tiled LSB request. The `lsb_clone_vs_in_place` bench uses Criterion batching so each in-place iteration starts from a pristine source and the preparation clone stays outside the timed section.

Stego-library-evolution M004 (transactional tiled in-place, no rollback clone) measured no runtime regression on the affected path: `cargo bench -p stegoeggo --bench bench -- --noplot tiled_lsb_request` gives `png_tiled/256` 1.863 ms (before 1.866 ms) and `png_tiled/1024` 31.21 ms (before 31.50 ms). Peak auxiliary allocation fell from 1,053,952 bytes to 5,376 bytes on a 1 MiB image (counting-allocator regression test `stegoeggo-stego/tests/tiled_in_place_peak.rs`). One `tiled` filter run showed an outlying `png_tiled/1024` median with a 3x confidence-interval spread; an immediate re-run returned to the baseline band, so it was recorded as machine noise.
