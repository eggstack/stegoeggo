# AGENTS.md

`stegoeggo` is a Rust library + CLI that writes rights-reservation metadata (primary channel) and optional steganographic markers (best-effort redundant channel) into PNG, JPEG, and WebP images. It is not DRM or proof of training.

## Commands

Fast check (mirrors required CI — run this before committing):
```bash
./scripts/check.sh
```
Individual steps: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo check -p stegoeggo --no-default-features`, `cargo test --workspace --exclude stegoeggo-fuzz --all-features`.

- Single test: `cargo test --workspace --exclude stegoeggo-fuzz --all-features -- <name>`
- Pre-release (local only, never publishes): `./scripts/release-check.sh [--allow-dirty] [--stage=pre|root|cli]`
- Specialist checks are manual, never part of `check.sh`: `scripts/verify_metadata_conformance.sh --strict` (needs exiftool, xmllint, imagemagick, libvips), `scripts/validate-docs-rs.sh` (nightly), `scripts/validate-msrv-package.sh` (Rust 1.87), `scripts/check_fuzz_sync.sh` (after adding/removing fuzz targets), `cargo deny check licenses|advisories`, `cargo semver-checks check-release`.
- Fuzz: `RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run <target> -- -max_total_time=60` (12 targets in `fuzz/fuzz_targets/`; release-profile LTO must be off or sanitizer linking fails). Add regression tests to `tests/robustness.rs`.

CI (`.github/workflows/ci.yml`): one required job on push/PR to `main` that runs `scripts/check.sh`. Everything else (`assurance.yml` MSRV/platform matrix, `external-verification.yml`, `fuzz.yml`) is scheduled/manual, non-blocking, never publishes. Do not add specialist checks to `check.sh` or expand required CI without a maintainer decision.

## Workspace

- `.` — library crate `stegoeggo`; canonical entry points are `process_request_bytes*` in `src/lib.rs`, plan executors in `src/pipeline.rs`. Conformance binary `stegoeggo-conformance` (`src/bin/`, needs `conformance` feature).
- `stegoeggo-stego/` — generic carrier crate for arbitrary-payload LSB/JPEG-DCT stego. Depend on it directly for generic use; `stegoeggo::stego` is a convenience re-export of the same surface.
- `stegoeggo-cli/` — binary `stegoeggo` at `stegoeggo-cli/src/main.rs` (modules `args`, `request`, `protect`, `verify`, `output`, `keys`, `manifest`). Its default package features include `signatures`, enabling `keygen`, `sign`, and `verify-manifest` through `stegoeggo/signatures` + `stegoeggo/detached-manifest`; the CLI still does not enable the library's unrelated `iscc`, `conformance`, or `parallel` features.
- `fuzz/` — 12 harnesses, `cargo-fuzz` + nightly only, excluded from workspace tests.

Toolchain is stable, MSRV 1.87 (`rust-toolchain.toml`, `rust-version` in root + carrier manifests). Rustfmt: 4-space indent, max width 100. `#![forbid(unsafe_code)]` in both crates. No code comments unless asked. `#[must_use]` on builders.

## Canonical API

`ProtectionRequest` + `RightsPolicy` are canonical (Release 4+). `ProtectionLevel`/`EvidenceProfile`/`ProtectionContext` are deprecated compatibility adapters that translate via `request_from_legacy()` — see `DEPRECATIONS.md` (removal at v1.0.0, never in 0.x). New processing features go in `ProtectionRequest`/`ProcessingOptions`/`ProtectionChannels` first; never extend legacy builders independently. `VerificationStatus` is **not** deprecated: `verify_image_bytes` returns it directly (not `Result`, not `bool`); `verify_image_bytes_report` is the canonical rich operation, other verification types are projections of the same facts (`src/verification/canonical.rs`).

Byte paths vs pixel paths (top footgun): `process_image`/`process_images_parallel` (`DynamicImage` in/out) embed stego only — PNG tEXt, JPEG COM/XMP, WebP XMP do not survive. Use byte APIs (`process_request_bytes`, `process_image_bytes`) when metadata matters. Reproducible output needs both an explicit seed (`ProtectionContext::new(intensity, seed)` — `default()` uses CSPRNG) and `with_timestamp_override(...)`.

Other traps: `inject_metadata`/`inject_legal_claims` are `Option<bool>` (`None` = level default; explicit `false` ≠ unset, and `false` with legal metadata emits `ContradictoryLegalClaims`); `#[serde(skip)]` on config drops MAC keys/legal metadata in roundtrips; `has_notice()` is true for any DMI value including `Allowed`/`Unspecified`; `MIN_PAYLOAD_SIZE` (28) is a parsing threshold, not an output size (V3 = 36 non-MAC / 48 MAC bytes); `LegalMetadata::MAX_FIELD_LEN` is 8192 over 16 validated fields.

## Carrier essentials

- Public `stego` module uses `StegoError`, not crate `Error` (convert via `From`).
- Carrier follows the **output** format (`JPEG ? DCT : LSB`); JPEG→JPEG uses a byte-only fast path (no pixel decode). JPEG→PNG/WebP is one pixel decode + raster LSB, never transient DCT.
- `jpeg::embed`/`embed_framed` are best-effort (auto-downgrade redundancy, seed-only fallback — application policy); `*_strict` variants embed at exactly the requested redundancy or return `InsufficientCapacity`. Capacity units are AC coefficients with `|coef| >= 2`. Pass `report.actual_redundancy` to `extract`.
- Use validated `Redundancy` + `from_redundancy`/`with_redundancy_value` for runtime values; legacy `with_redundancy` is constants-only (debug-assert vs release-clamp). Zero seeds are valid. Max redundancy 10. V3 payloads are written; V1/V2 extract-only.
- Two unrelated XorShiftRngs: `PixelSelectionRng` (`src/util/image.rs`) vs `DctCoefficientRng` (`stegoeggo-stego/.../stego_f5.rs`) — do not interchange.
- Container/format edge cases (Q-table hints, VP8X flags, XMP shape, preserving JPEG encoding) live in `architecture/jpeg-*.md` and `architecture/protected-*.md` — read those before touching those paths.

## CLI essentials

Canonical commands are `protect <INPUT>...`, `inspect <IMAGE>`, `verify <IMAGE>`,
`version`, and the reserved `update` command. The old root protection syntax
and `--verify` remain accepted during 0.x; exact command-name paths need `./`
or `--` to disambiguate. All protection routes through
`request::build_protection_request_with_explicit_options` and then the byte
processing APIs. Canonical flags are `--rights-policy`, `--preset`,
`--hidden-marker`, `--authentication`, and `--key`; `--level`/`--profile`/`--dmi`
and shorthand policy flags are translation-only compatibility syntax.
`--dry-run` prints the plan. `inspect` is read-only and normally exits 0 even
for an unprotected image; `verify` exits 3 for missing or invalid protection
evidence. Exit codes are 0 ok, 1 error, 2 config, 3 integrity, 4
verified-but-untrusted manifest (`verify-manifest` only), and 5 internal.
`--verify` always exits 0 — read the output text. Full contract:
`docs/cli-usage.md` and `architecture/cli.md`.

## Features

All default-off: `async` (canonical `process_request_bytes_async*`), `parallel` (canonical `process_request_bytes_parallel*`, one shared request, order-preserving), `signatures`, `detached-manifest`, `iscc`, `conformance`. `test-seeds`/`fuzz` are test-only, never in production binaries. `tests/async_integration.rs` needs `async`; `tests/external_tools.rs` is `#[ignore]` (run with `--ignored`); conformance harness exit codes are 0 pass / 1 fail / 2 config / 3 digest / 4 coverage / 5 internal.

## Releases

Manual only: no crates.io publication, no crates.io token in Actions, and no tag-triggered publication. A manually dispatched `release-binaries.yml` attaches CLI assets to an existing GitHub Release; it never publishes crates. All three crates share one version with exact `=X.Y.Z` deps; publish crates in order carrier → library → CLI. See `RELEASING.md` and `docs/installation.md`.

Binary release contract: assets use the versionless names in
`scripts/release-targets.txt` and every executable has a `.sha256` sidecar.
The release binary feature set is the CLI package default (`signatures`). Run
`./scripts/release-binary-preflight.sh --tag=vX.Y.Z` and
`./scripts/release-check-assets.sh --dir=<asset-directory>` before attaching
assets. `packaging/install.sh` and `packaging/install.ps1` verify checksums and
candidate identity before installation; Cargo fallback is allowed only for an
unsupported target or a missing (404) binary asset, never for checksum,
identity, or network failure.

## Where things live

- Skills (load before working): `.skills/stegoeggo-conventions/SKILL.md` (signatures, constants, pitfalls) for any Rust change; `.skills/plan-execution/SKILL.md` when executing a numbered plan in `plans/`; `.skills/architecture-review/SKILL.md` when verifying/editing `architecture/` docs. Architecture index: `architecture/overview.md` (39 deep-dives). User guides: `docs/` (`cli-usage.md`, `rust-api.md`, `carrier-crate.md`, `formats.md`, `legal_notice_model.md`, `migration-v0.3.md`). Examples (`protect_and_verify.rs`, `verify_saved.rs`, `legal_metadata.rs`, `generic_stego.rs`) must keep compiling. Plans: `plans/` (highest so far 098; next is 099+; historical plans are immutable except their `-status.md`).
