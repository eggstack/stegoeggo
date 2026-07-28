# AGENTS.md

## Project Overview

`stegoeggo` is a Rust library and CLI for protecting images from unauthorized AI use through rights-reservation metadata and steganographic markers.

## Workspace Structure

Three workspace members:
- `.` — Main library crate (`stegoeggo`) + conformance harness binary (`stegoeggo-conformance`)
- `stegoeggo-cli/` — CLI binary (`stegoeggo` binary name), entry point at `stegoeggo-cli/src/main.rs`
- `fuzz/` — Fuzz harnesses (12 targets, requires `cargo-fuzz` + nightly)

## Build & Test Commands

```bash
cargo check                              # Compilation
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
cargo test -p stegoeggo-cli --all-features --no-fail-fast
cargo test --doc --workspace --exclude stegoeggo-fuzz
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check               # Format check (4-space indent, max width 100)
cargo package --workspace                # Package dry-run
cargo test --test external_tools -- --ignored  # External tool tests (requires exiftool/xmllint)
```

**Single test:** `cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast -- <test_name>`

**Full release validation:**
```bash
./scripts/validate-release.sh                     # hermetic + external + feature phases
./scripts/validate-release.sh --skip-external     # hermetric + feature only
./scripts/validate-release.sh --phase hermetic     # just fmt, clippy, tests, package, deny
./scripts/validate-release.sh --phase feature      # feature combination matrix
```

The script also runs `cargo semver-checks check-release`, `cargo audit`, `scripts/validate-docs-rs.sh` (docs.rs-equivalent rustdoc), and `scripts/validate-msrv-package.sh` (fresh MSRV consumer resolution) in the hermetic phase.

**Docs.rs-equivalent validation (nightly, DOCS_RS=1, cfg(docsrs)):**
```bash
scripts/validate-docs-rs.sh
```

**MSRV package validation (fresh resolution, no lockfile):**
```bash
scripts/validate-msrv-package.sh
```

**Conformance (requires external tools: exiftool, xmllint, imagemagick, libvips):**
```bash
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict --json conformance-report.json
```

Conformance exit codes: 0=pass, 1=fail, 2=config, 3=digest mismatch, 4=coverage violation, 5=internal.

## CI Pipeline

GitHub Actions (`.github/workflows/ci.yml`) runs these jobs in parallel:
1. MSRV check (`cargo check --all-features` with Rust 1.87)
2. Tests + doc tests
3. Format + clippy lint
4. Security audit (`cargo audit`)
5. Semver check (`cargo semver-checks check-release`)
6. License/advisory check (`cargo deny`)
7. Package dry-run
8. External integration tests (installs exiftool/xmllint/imagemagick/libvips)
9. External Conformance (builds + runs `stegoeggo-conformance --strict`)
10. Feature combination matrix (no-default, async, signatures, detached-manifest, all-features)
11. Docs.rs Build (nightly rustdoc with DOCS_RS=1, cfg(docsrs), all features)
12. MSRV Package Consumer (fresh-resolution compile on declared MSRV)
13. Benchmarks (manual dispatch only)

## Code Conventions

- Rustfmt: 4-space indentation, max width 100 (`rustfmt.toml`)
- `#![forbid(unsafe_code)]` throughout — no unsafe blocks in the library crate
- No comments in code unless explicitly asked
- `#[must_use]` on builder methods
- `pub(crate)` for internal modules (e.g., `jpeg_transcoder`)
- Private fields with getter methods on `ProtectionContext`, `StegoPayload`, `LegalMetadata`

## Features

| Feature | Description | Default |
|---------|-------------|---------|
| `async` | Tokio-based async API wrappers | No |
| `signatures` | Ed25519 signing via `ed25519-dalek` | No |
| `detached-manifest` | Detached signed manifest sidecar | No |

Feature-gated tests: `tests/async_integration.rs` requires `async`.

## Deprecated API Surfaces

These still work but will be removed in the next major version. See `DEPRECATIONS.md` for full inventory.

- `EvidenceProfile` — use `ProtectionPreset` instead
- `with_dmi()` — use `RightsPolicy` in `ProtectionRequest`
- `with_metadata_injection()` — use `ProtectionChannels`
- `with_inject_legal_claims()` — use `ProtectionChannels`
- `compute_iscc()` — use `compute_content_identifiers()`
- `NoticeVerification::new()` positional constructor — use `NoticeVerification::builder()`

**Policy-first architecture (Release 4+):** `ProtectionRequest` and `RightsPolicy` are the canonical API. `ProtectionLevel` and `EvidenceProfile` are deprecated compatibility adapters.

## Things to Watch Out For

**Gotchas that will bite you:**

- **`MIN_PAYLOAD_SIZE` is 28, not the output size** — Non-MAC payloads are 76 bytes (ECC-encoded), MAC payloads are 32 bytes. The constant is a parsing threshold
- **Two separate XorShiftRng implementations** — `PixelSelectionRng` in `src/util/image.rs` and `DctCoefficientRng` in `src/jpeg_transcoder/stego_f5.rs`. Different algorithms, different sequences for same seed. Do NOT interchange them
- **`ProtectionContext::default()` uses CSPRNG seed** — For reproducible results, use `ProtectionContext::new(intensity, seed)` with an explicit seed
- **Pipeline flow order** — JPEG output: encode → DCT stego → metadata. Non-JPEG: pixel stego → encode → metadata. JPEG→JPEG fast path bypasses pixel decode entirely
- **`MetadataTrapProtector::apply()` returns `Cow::Borrowed(img)` unchanged** — Metadata injection is byte-level. The pipeline routes `Light` through `apply_light_bytes()` which encodes → injects → decodes
- **`#[serde(skip)]` on `config` field** — MAC keys and legal metadata are lost in serde roundtrips
- **CLI `--verify` always exits 0** — Use output text to determine protection state, not exit code
- **F5 seed Q-table edge case** — `embed_seed_in_quantization_tables()` fails if any quantization value in the first 2 tables is < 2
- **`--tdm-reserved` is deprecated** — TDMRep deployment deferred; sets DMI to `ProhibitedSeeConstraints`
- **CLI binary location** — `stegoeggo-cli/src/main.rs`, not `src/bin/`
- **`--require-complete` is removed** — `--strict` is the single complete-validation mode

**Metadata and API traps:**

- **No synthetic defaults** — When no `LegalMetadata` is provided, no "All Rights Reserved" copyright text, no default usage terms, no `DateCreated` are emitted. Each field is emitted only when explicitly provided
- **`inject_metadata` / `inject_legal_claims` are `Option<bool>`** — Default `None` (use level default) vs explicit `false` (disable). `with_metadata_injection(false)` ≠ not calling it at all
- **`inject_legal_claims` auto-enables when `LegalMetadata` present** — No need to call `with_legal_claims(true)`. Explicitly setting `with_legal_claims(false)` still disables injection
- **Contact not written to `photoshop:Credit`** — Contact remains in PNG tEXt and JPEG COM markers only
- **`photoshop:Credit` maps to `credit_line`** — In WebP XMP, not `contact`. Previous mapping was semantically incorrect
- **`has_notice()` includes DMI** — Returns true when any legal field OR `dmi.is_some()` is found. `DmiValue::Allowed` and `DmiValue::Unspecified` make `has_notice()` true — this means "legal metadata was found" not "restrictions were imposed"
- **`LegalMetadata::MAX_FIELD_LEN`** — 8192 bytes. `validate()` checks all 8 fields, returns `Error::Config` on violation
- **Verification returns `VerificationStatus`** — Not `Option<bool>`. Use `== VerificationStatus::Verified` in assertions
- **Metadata overflow checks** — PNG chunk lengths use `u32::try_from()`, JPEG marker lengths use `u16::try_from()`. Overflow returns `Error::Metadata`

**Steganography details:**

- **Three seed storage locations** — (1) Q-table LSBs in JPEG, (2) metadata markers (strippable), (3) fixed-position LSB in first 64 pixel channels. Extraction chain: metadata → LSB fallback → `FALLBACK_SEEDS`
- **ECC on stego payload** — Non-MAC payloads use 3× repetition with majority voting before CRC32. MAC payloads use 8-byte HMAC instead
- **Spread spectrum LSB** — Each payload bit embedded across `STEGO_SPREAD_FACTOR` (=5) adjacent pixels via majority voting
- **`stego_redundancy` is `Option<usize>`** — Default `None` derives from intensity via `effective_redundancy()` (<0.3→1, 0.3-0.7→2, >=0.7→3). Valid range 1-10
- **F5 redundancy cap** — Max redundancy is 10. Extraction tries all 10 values
- **Payload version 3 is current** — V1/V2 still supported for extraction. V3 adds TLV extensions with domain-separated authentication. V3 extraction paths (magic bytes `[0x53, 0x45]`) tried first before V2/V1 fallback
- **Tiled steganography** (`with_tile_size(n)`) — Crop-resistant mode, full payload per tile. `tile_seed(master_seed, tile_x, tile_y)` uses splitmix64
- **F5 tiled block set** — MCU-interleaved: `block_idx = (mcu_y * mcus_per_row + mcu_x) * h * v + sub_y * h + sub_x`. Do NOT assume row-major ordering

**Canonical metadata format:**

- XMP writer emits `plus:DataMining` with PLUS LDF vocabulary keys (e.g., `DMI-PROHIBITED-AIMLTRAINING`). Legacy `Iptc4xmpExt:DMI-*` properties are parsed for backward compatibility but not emitted
- `tdm:reserve_tdm` is no longer emitted (TDMRep is a web-distribution mechanism, not an image-metadata signal)
- WebP legal fields live in XMP inside `<rdf:Description>`. `dc:rights` and `xmpRights:UsageTerms` use `<rdf:Alt><rdf:li>` containers. `dc:creator` uses `<rdf:Seq>`
- WebP exiftool: `exiftool -Copyright` does not resolve `dc:rights` — use `exiftool -XMP-dc:Rights`

**Warning system:**

- `ProtectionWarning` has 6 variants: `MissingMacKey`, `MetadataInjectionDisabled`, `ProgressiveJpegFallback`, `JpegReencodeFragile`, `LsbCapacitySkipped`, `DctCapacityInsufficient`
- `MissingMacKey` is profile-dependent: only emitted for `AuthenticatedProvenance` and `Maximal` profiles
- `severity_for_profile(profile)` classifies warnings as `Info`, `Warning`, or `Error`

## Architecture

- **Strategy pattern** via `Protector` trait (`src/traits.rs`) with three levels: Disabled, Light, Standard
- **Pipeline** (`src/lib.rs`): `ProtectionPipeline` orchestrates protectors
- **JPEG fast path**: When input/output are both JPEG, operates directly on DCT coefficients via `src/jpeg_transcoder/`, bypassing pixel decode/encode
- **`#![forbid(unsafe_code)]`** throughout the library crate

## Validation Scripts

- `scripts/validate-release.sh` — Phases: hermetic (fmt, clippy, tests, package, deny, audit, semver-checks, MSRV), feature (feature combination matrix), external (external integration + conformance). Supports `--phase` and `--expected-sha`
- `scripts/validate-docs-rs.sh` — Docs.rs-equivalent rustdoc validation (nightly, DOCS_RS=1, cfg(docsrs), workspace + packaged crate)
- `scripts/validate-msrv-package.sh` — Fresh MSRV consumer resolution (packages crate, creates clean consumers, tests all feature combos on declared MSRV)
- `scripts/verify_metadata_conformance.sh` — Shell wrapper for conformance checks
- `scripts/check_fuzz_sync.sh` — Verifies fuzz harness parity

## Conformance Suite

The conformance harness (`src/bin/stegoeggo-conformance.rs`) validates metadata interoperability against ExifTool and xmllint. It produces JSON reports and is a mandatory CI gate.

Key files: `src/conformance.rs` (report types), `tests/fixtures/conformance/manifest.toml` (fixture manifest with SHA-256 digests).

External integration tests in `tests/external_tools.rs` are `#[ignore]` — run with `--ignored`.

## Fuzzing

12 targets in `fuzz/fuzz_targets/`. Run with: `cargo +nightly fuzz run <target> -- -max_total_time=60`. Add regression tests in `tests/robustness.rs` for findings.

## Release Policy

Releases are manual. GitHub Actions must not publish crates or create releases.
Use direct Cargo/crates.io publication after local validation.
Do not push a version tag as a publication mechanism.
Published crates.io versions are immutable and cannot be reused.

## Other Reference Files

- `DEPRECATIONS.md` — Deprecated API inventory
- `SUPPORT.md` — Support matrix
- `STABILITY.md` — Stability tiers
- `architecture/` — Architecture documentation (24 files, verified against source)
