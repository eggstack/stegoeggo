# AGENTS.md

`stegoeggo` is a Rust library + CLI that writes rights-reservation metadata (primary channel) and optional steganographic markers (best-effort redundant channel) into PNG, JPEG, and WebP images. It is not DRM or proof of training.

## Commands

Fast check (mirrors required CI — run this before committing):
```bash
./scripts/check.sh
```
Individual steps: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo check -p stegoeggo --no-default-features`, `cargo test --workspace --exclude stegoeggo-fuzz --all-features`, `./scripts/check-docs-contract.sh` (also run by `check.sh`; keeps installer URL, canonical commands, and release-target wording aligned).

- Single test: `cargo test --workspace --exclude stegoeggo-fuzz --all-features -- <name>`
- Pre-release (local only, never publishes): `./scripts/release-check.sh [--allow-dirty] [--stage=pre|root|cli]`
- Specialist checks are manual, never part of `check.sh`: `scripts/verify_metadata_conformance.sh --strict` (needs exiftool, xmllint, imagemagick, libvips), `scripts/validate-docs-rs.sh` (nightly), `scripts/validate-msrv-package.sh` (Rust 1.89), `scripts/check_fuzz_sync.sh` (after adding/removing fuzz targets), `cargo deny check licenses|advisories`, `cargo semver-checks check-release`.
- Fuzz: `RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run <target> -- -max_total_time=60` (12 targets in `fuzz/fuzz_targets/`; release-profile LTO must be off or sanitizer linking fails). Add regression tests to `tests/robustness.rs`.

### CI (12 workflows — only one is *required*)

`ci.yml` is the single required gate: one `Check` job on push/PR to `main` running `scripts/check.sh`. Do not add specialist checks to `check.sh` or expand required CI without a maintainer decision.

`ci.yml` is **not** the only workflow that runs on pull requests. `c-binding.yml`, `node-binding.yml`, `python-binding.yml`, and `release-drift.yml` also trigger on push **and** PR to `main` (path-filtered) and are non-blocking but *will* go red. Because `bindings/` is excluded from the Cargo workspace, `check.sh` cannot see a binding at all — a `src/` change that breaks a binding export fails in those workflows, not in required CI.

`assurance.yml` (MSRV 1.89 + Linux aarch64 / macOS aarch64 / Windows x86_64), `external-verification.yml`, and `fuzz.yml` are genuinely scheduled/manual and informational. `release-binaries.yml`, `release-c.yml`, `release-node.yml`, and `release-python.yml` are manual-dispatch and never publish. Full matrix: `architecture/tooling.md`.

## Workspace

- `.` — library crate `stegoeggo`; canonical entry points are `process_request_bytes*` in `src/lib.rs`, plan executors in `src/pipeline.rs`. Conformance binary `stegoeggo-conformance` (`src/bin/`, needs `conformance` feature).
- `stegoeggo-stego/` — generic carrier crate for arbitrary-payload LSB/JPEG-DCT stego. Depend on it directly for generic use; `stegoeggo::stego` is a convenience re-export of the same surface.
- `stegoeggo-cli/` — binary `stegoeggo` at `stegoeggo-cli/src/main.rs` (modules `args`, `request`, `protect`, `verify`, `update`, `output`, `keys`, `manifest`). Its default package features include `signatures`, enabling `keygen`, `sign`, and `verify-manifest` through `stegoeggo/signatures` + `stegoeggo/detached-manifest`; the CLI still does not enable the library's unrelated `iscc`, `conformance`, or `parallel` features.
- `fuzz/` — 12 harnesses, `cargo-fuzz` + nightly only, excluded from workspace tests.
- `bindings/{c,node,python}/` — three FFI leaf crates, each its own nested `[workspace]`, `publish = false`, **excluded from the root workspace** (`exclude = ["bindings"]`). All pin `stegoeggo = "=0.4.2"` with `default-features = false` and force `panic = "unwind"` in dev and release. `check.sh` never builds them. See the Bindings section.

Toolchain is stable, MSRV 1.89 (`rust-toolchain.toml`, `rust-version` in root + carrier + CLI manifests). Rustfmt: 4-space indent, max width 100. `#![forbid(unsafe_code)]` in the root and carrier crates only — the FFI leaves are inherently `unsafe` and must not inherit the attribute. No code comments unless asked. `#[must_use]` on builders.

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
- `limits.rs` (`CarrierLimits`) is the carrier's own bounded-input contract, independent of the root `ResourceLimits`. `webp.rs` is a still-lossless facade behind feature `webp`; it rejects lossy/animated input and preserves no container metadata. The root byte paths never route to it implicitly.
- Container/format edge cases (Q-table hints, VP8X flags, XMP shape, preserving JPEG encoding) live in `architecture/jpeg-*.md` and `architecture/protected-*.md` — read those before touching those paths.

## Bindings essentials

- All three pin `stegoeggo = "=0.4.2"` with `default-features = false` — no root feature is compiled in, so an FFI-facing gap needs an explicit feature in that binding's manifest.
- `panic = "unwind"` in both dev and release is mandatory (root uses `abort`); it is what makes panic containment at the FFI boundary possible.
- **C ABI v1 is a stability promise** (`bindings/c/ABI-V1.md`, `STABILITY.md`). 90 exports; `abi-v1-symbols.txt` and the cbindgen header are generated and drift-checked — regenerate, never hand-edit.
- Node `.d.ts` files are generated and drift-checked; library calls run on a threadpool (`src/tasks.rs`).
- Python releases the GIL with `py.detach(...)` around every library call; richer ergonomics live in the pure-Python layer.
- Before changing any function a binding calls, load `.skills/bindings/SKILL.md` and check the corresponding path-filtered workflow.

## CLI essentials

Canonical commands are `protect <INPUT>...`, `inspect <IMAGE>`, `verify <IMAGE>`,
`version`, and `update`. `version` prints exactly `stegoeggo X.Y.Z` on its first
line without network/config access. `update` uses the stable crates.io
`stegoeggo-cli` version as authority, then the matching GitHub Release asset;
it verifies checksum, candidate identity, and candidate version before
self-replacement. The old root protection syntax
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

All default-off: `async` (canonical `process_request_bytes_async*`), `parallel` (canonical `process_request_bytes_parallel*`, one shared request, order-preserving), `signatures`, `detached-manifest`, `iscc`, `conformance`, `webp` (carrier still-lossless WebP facade). `test-seeds`/`fuzz` are test-only, never in production binaries. `tests/async_integration.rs` needs `async`; `tests/external_tools.rs` is `#[ignore]` (run with `--ignored`); conformance harness exit codes are 0 pass / 1 fail / 2 config / 3 digest / 4 coverage / 5 internal.

## Releases

Manual only: no crates.io publication, no crates.io token in Actions, and no tag-triggered publication. The preferred Unix installer source is `https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh`. A manually dispatched Eggpack-generated `release-binaries.yml` stages a draft with the complete CLI asset matrix for maintainer publication; it never publishes crates or the GitHub draft. All three crates share one version with exact `=X.Y.Z` deps; publish crates in order carrier → library → CLI. See `RELEASING.md` and `docs/installation.md`.

Binary release contract: assets use the versionless names in
`release/eggpack/distribution.toml` and every executable has a `.sha256` sidecar.
The release binary feature set is the CLI package default (`signatures`). Run
`./scripts/release-binary-preflight.sh --tag=vX.Y.Z` and
`./scripts/release-check-assets.sh --dir=<asset-directory>` before staging
assets. The workflow captures the canonical Cargo output per target through
the Eggpack handoff explicitly; it must not discover candidates with
`find`. `packaging/install.sh` and `packaging/install.ps1` verify checksums and
candidate identity before installation; Cargo fallback is allowed only for an
unsupported target or a missing (404) binary asset, never for checksum,
identity, or network failure.

Updater invariants: query the crates.io stable version first, then check the
current executable's destination before downloading any update artifact (an
already-current installation reports current without requiring destination
replaceability); use the embedded eggup acquisition over eggfetch transport with bounded response limits plus
bounded Cargo/candidate subprocesses with argument arrays; honor conventional
proxy environment variables explicitly; deny HTTPS-downgrade redirects; ignore
prereleases; allow Cargo fallback only for unsupported targets or the exact
asset HTTP 404; never invoke `sudo`; and leave the current executable untouched
when staging or validation fails. A Cargo-installed path becomes binary-managed
after successful self-replacement until Cargo installs it again.

## Where things live

**Architecture index — read the deep dive before changing the area.** `architecture/overview.md` (42 files total: this overview + `review_plan.md`, historical + 40 component deep-dives) is the top-level index with the full Component Index table. Load `.skills/architecture-review/SKILL.md` before editing anything in `architecture/`.

| If you are changing | Read first |
|---|---|
| Request resolution, executors, format routing | `architecture/pipeline.md`, `architecture/resolve.md`, `architecture/types.md` |
| Metadata injection (PNG tEXt, JPEG COM/XMP, WebP) | `architecture/protected-metadata-trap.md`, `architecture/xmp.md`, `architecture/webp-container.md` |
| Hidden marker, payloads, redundancy, tiling | `architecture/protected-steganography.md`, `architecture/payload-v3.md` |
| Carrier LSB / JPEG DCT / generic surface | `architecture/carrier-lsb.md`, `architecture/carrier-jpeg.md`, `architecture/carrier-surface.md` |
| JPEG parsing, entropy coding, F5 | `architecture/jpeg-header.md`, `architecture/jpeg-entropy.md`, `architecture/jpeg-stego-f5.md`, `architecture/jpeg-transcoder.md` |
| Verification, evidence strength, trust | `architecture/verification.md`, `architecture/error.md` |
| Signing, detached manifests, provenance | `architecture/signing.md`, `architecture/detached.md`, `architecture/detached-manifest.md`, `architecture/provenance.md`, `architecture/provenance-claim.md` |
| C / Node / Python FFI | `architecture/bindings-c.md`, `architecture/bindings-node.md`, `architecture/bindings-python.md` |
| CLI surface, flags, exit codes | `architecture/cli.md` |
| Limits, accounting, async | `architecture/resource-limits.md`, `architecture/container-walk.md`, `architecture/async-api.md` |
| Tests, fuzz, scripts, CI, benches | `architecture/testing.md`, `architecture/tooling.md` |
| Constants and shared invariants | `architecture/constants.md`, `architecture/traits.md`, `architecture/util-image.md`, `architecture/util-seed.md`, `architecture/util-iscc.md` |

**Skills** (load before working): `.skills/stegoeggo-conventions/SKILL.md` (signatures, constants, pitfalls) for any Rust change; `.skills/bindings/SKILL.md` for anything touching `bindings/` or a function a binding calls; `.skills/planning/SKILL.md` when creating or managing plans, roadmaps, ADRs, closure records, or `registry.md`; `.skills/plan-execution/SKILL.md` when executing a milestone plan (worktree mechanics); `.skills/architecture-review/SKILL.md` when verifying/editing `architecture/` docs.

**Other locations**: User guides in `docs/` (`cli-usage.md`, `installation.md`, `rust-api.md`, `carrier-crate.md`, `formats.md`, `legal_notice_model.md`, `migration-v0.3.md`). Examples in `examples/` (`protect_and_verify.rs`, `verify_saved.rs`, `legal_metadata.rs`, `generic_stego.rs`) — all 4 are Cargo targets and must keep compiling. Plans in `plans/` (`README.md` system guide, `registry.md` control surface, `000`-`003` canonical docs). Flat `001`-`107` are immutable predecessor history (only `-status.md` companions record facts); 107 is the last flat plan — new milestones take subsystem-local numbers under `plans/implementation/<subsystem>/`.

Durable session learnings that are not yet in `AGENTS.md`, skills, or `architecture/` go in `AGENTS.override.md`; promote them to those canonical locations when they stabilize, then remove them there.

**Keeping docs honest.** Counts in these files go stale. Before repeating a number (architecture files, test files, workflows, enum variants, exported symbols), re-verify it against the tree — the re-verification commands are tabulated in `.skills/stegoeggo-conventions/SKILL.md`. `scripts/check-docs-contract.sh` enforces the installer/command/target wording contract on the required CI path, but it does **not** check counts or API claims; those are on you.
