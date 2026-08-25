# AGENTS.md

## Project Overview

`stegoeggo` is a Rust library and CLI for protecting images from unauthorized AI use through rights-reservation metadata and steganographic markers.

## Workspace Structure

Four workspace members:
- `.` — Main library crate (`stegoeggo`) + conformance harness binary (`stegoeggo-conformance`)
- `stegoeggo-stego/` — Generic carrier crate (`stegoeggo-stego`) with application-neutral LSB and JPEG DCT mechanics
- `stegoeggo-cli/` — CLI binary (`stegoeggo` binary name), entry point at `stegoeggo-cli/src/main.rs`
- `fuzz/` — Fuzz harnesses (12 targets, requires `cargo-fuzz` + nightly)

## Build & Test Commands

**Fast local check (mirrors required CI):**
```bash
./scripts/check.sh
```

This runs formatting, strict clippy, minimal-feature compilation, and all-feature workspace tests. It contains only fast deterministic checks and does not publish, require external tools, or generate artifacts.

**Individual commands:**
```bash
cargo fmt --all -- --check               # Format check (4-space indent, max width 100)
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
```

**Single test:** `cargo test --workspace --exclude stegoeggo-fuzz --all-features -- <test_name>`

**Pre-release check (local only, never publishes):**
```bash
./scripts/release-check.sh
# staged package checks when the carrier/root/CLI release order matters:
./scripts/release-check.sh --allow-dirty --stage=pre
./scripts/release-check.sh --allow-dirty --stage=root
./scripts/release-check.sh --allow-dirty --stage=cli
```

**Specialist verification (manual, targeted, not run on every push):**
```bash
scripts/validate-docs-rs.sh              # nightly Rust required
scripts/verify_metadata_conformance.sh --strict  # exiftool, xmllint, imagemagick, libvips required
scripts/validate-msrv-package.sh         # Rust 1.87+ required
scripts/check_fuzz_sync.sh               # after adding/removing fuzz targets
cargo +nightly fuzz run <target> -- -max_total_time=60
cargo semver-checks check-release
cargo deny check licenses
cargo deny check advisories
```

Conformance exit codes: 0=pass, 1=fail, 2=config, 3=digest mismatch, 4=coverage violation, 5=internal.

## CI Pipeline

GitHub Actions (`.github/workflows/ci.yml`) runs one job on pushes and pull requests to `main`:
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo check -p stegoeggo --no-default-features`
4. `cargo test --workspace --exclude stegoeggo-fuzz --all-features`

Specialist verification (external tools, conformance, fuzzing, MSRV, docs.rs, packaging, semver, benchmarks) is available as manual-dispatch workflows:
- `.github/workflows/external-verification.yml` — external integration tests + conformance harness
- `.github/workflows/fuzz.yml` — single-target fuzz execution (workflow_dispatch with target/seconds inputs)

## Code Conventions

- Rustfmt: 4-space indentation, max width 100 (`rustfmt.toml`)
- `#![forbid(unsafe_code)]` throughout both crates — no unsafe blocks in the library or carrier crate
- No comments in code unless explicitly asked
- `#[must_use]` on builder methods
- `pub(crate)` for internal modules (e.g., `jpeg_transcoder` in `stegoeggo-stego`)
- Private fields with getter methods on `ProtectionContext`, `StegoPayload`, `LegalMetadata`

## MSRV

Rust **1.87** (declared in `Cargo.toml` and `stegoeggo-stego/Cargo.toml`). Toolchain is stable (`rust-toolchain.toml`).

## Features

| Feature | Description | Default |
|---------|-------------|---------|
| `async` | Tokio-based async API wrappers | No |
| `signatures` | Ed25519 signing via `ed25519-dalek` | No |
| `detached-manifest` | Detached signed manifest sidecar | No |
| `iscc` | ISCC content identifier computation | No |
| `conformance` | Conformance harness binary and manifest parsing (TOML) | No |
| `parallel` | Rayon-based parallel batch processing | No |
| `test-seeds` | Test infrastructure only — never in production binary | No |
| `fuzz` | Fuzzing support — never in production binary | No |

Feature-gated tests: `tests/async_integration.rs` requires `async`.
The conformance binary (`stegoeggo-conformance`) requires the `conformance` feature.
The CLI binary enables `iscc`, `conformance`, and `parallel` via its exact-version `stegoeggo` dependency, even though none are default features of the library.

## Deprecated API Surfaces

These still work but will be removed in the next major version. See `DEPRECATIONS.md` for full inventory.

- `EvidenceProfile` — use `ProtectionPreset` instead
- `with_dmi()` — use `RightsPolicy` in `ProtectionRequest`
- `with_legal_claims()` — auto-enabled when `LegalMetadata` present; explicit `false` emits `ContradictoryLegalClaims`
- `with_metadata_injection()` — use `ProtectionChannels`
- `with_inject_legal_claims()` — use `ProtectionChannels`
- `compute_iscc()` / `compute_iscc_with_metadata()` / `compute_iscc_from_bytes()` — use the `compute_content_identifiers*()` equivalents
- `NoticeVerification::new()` positional constructor — use `NoticeVerification::builder()`

Not deprecated (do not migrate away): `VerificationStatus` — still the return type of `verify_image_bytes`. The structured reports (`VerificationReport`, `VerificationResult`, `NoticeVerification`) are richer alternatives, not replacements for a removed API.

**Policy-first architecture (Release 4+):** `ProtectionRequest` and `RightsPolicy` are the canonical API. `ProtectionLevel` and `EvidenceProfile` are deprecated compatibility adapters.

## Gotchas

- **`MIN_PAYLOAD_SIZE` is 28, not the output size** — Non-MAC payloads are 76 bytes (ECC-encoded), MAC payloads are 32 bytes. The constant is a parsing threshold
- **Two separate XorShiftRng implementations** — `PixelSelectionRng` in `src/util/image.rs` and `DctCoefficientRng` in `stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs`. Different algorithms, different sequences for same seed. Do NOT interchange them
- **`ProtectionContext::default()` uses CSPRNG seed** — For reproducible results, use `ProtectionContext::new(intensity, seed)` with an explicit seed
- **Pipeline flow order** — JPEG output: encode → DCT stego → metadata. Non-JPEG: pixel stego → encode → metadata. JPEG→JPEG fast path bypasses pixel decode entirely
- **Generic stego API uses `StegoError`** — The public `stego` module uses its own `StegoError` type, not the crate root `Error`. Convert via `From<StegoError> for Error`
- **JPEG `extract` requires `actual_redundancy`** — The JPEG embed auto-downgrades redundancy when capacity is insufficient. Pass `report.actual_redundancy` to `extract` to match what was embedded
- **JPEG framed extraction reuses one decode** — `jpeg::extract_framed()` validates support, decodes the coefficient container once, and reuses private retained state across the configured redundancy search
- **`#[serde(skip)]` on `config` field** — MAC keys and legal metadata are lost in serde roundtrips
- **CLI unified path** — The CLI always routes through `ProtectionRequest`. `--dmi auto` and omitted `--dmi` are equivalent. Mixed conflicting policy options are configuration errors (exit code 2)
- **CLI exit codes** — `0`=ok, `1`=error, `2`=config, `3`=integrity, `5`=internal. `--verify` always exits 0; use output text to determine protection state, not exit code
- **CLI new-style flags** — `--rights-policy`, `--preset`, `--hidden-marker`, `--authentication` route through canonical `ProtectionRequest`. `--preset` cannot combine with `--level`/`--profile`. `--rights-policy` replaces `--dmi`. `--dry-run` prints the resolved plan without processing
- **CLI subcommands (feature: `signatures`)** — `keygen`, `sign`, `verify-manifest` are feature-gated. `verify-manifest` accepts `--payload-key` for HMAC verification. `--json` enables machine-readable output
- **No `test-seeds` in production CLI** — `test-seeds` is test infrastructure only, never in production binary
- **F5 seed Q-table edge case** — `embed_seed_in_quantization_tables()` fails if any quantization value in the first 2 tables is < 2
- **`--tdm-reserved` is deprecated** — TDMRep deployment deferred; sets DMI to `ProhibitedSeeConstraints`
- **CLI binary location** — `stegoeggo-cli/src/main.rs`, not `src/bin/`
- **`--require-complete` is removed** — `--strict` is the single complete-validation mode

**JPEG DCT and container correctness:**

- **JPEG DCT subset** — DCT embedding supports only: 8-bit precision, sequential Huffman DCT, single scan, 1-4 components with supported sampling factors, no restart intervals, valid terminal EOI. Unsupported inputs receive Q-table seed only via `probe_dct_support_full()` gating, with full metadata injection
- **`encode_coefficients` uses preserving encoding** — All successful DCT embedding attempts walk the original byte stream replacing only DQT and SOS scan data. APP2, APP13, APP14, COM, and other unrelated segments survive byte-for-byte
- **`probe_dct_support_full` checks scan structure** — Walks the complete JPEG byte stream to count scans (must be exactly 1), verify EOI, and reject trailing post-scan segments
- **`parse_sos` rejects malformed table IDs** — SOS table IDs > 3 return `InvalidFormat` error instead of clamping
- **Metadata-only JPEG is byte-safe** — `inject_text_chunks_jpeg` walks the raw byte stream, preserving all pre-SOS segments verbatim

**WebP container correctness:**

- **VP8X dimensions are 3-byte LE** — The `image-webp` decoder reads canvas width/height as 3-byte little-endian values, not 4-byte
- **VP8X flags bit positions** — ICC=0x20, Alpha=0x10, EXIF=0x08, XMP=0x04, Animation=0x02. The reserved mask is 0xC1. The EXIF bit (0x08) must NOT be set when no EXIF chunk is present, or `image-webp` returns `ChunkMissing`
- **EXIF seed emission retired** — No new EXIF seed chunks are emitted. Seed is stored in XMP via `stegoeggo:ProtectionSeed`. Historical EXIF seed data is still parsed for backward compatibility
- **One effective XMP chunk** — Output loop skips original XMP chunks; `merge_or_replace_webp_xmp` prepares the replacement
- **`xmp` module is `pub(crate)`** — The XMP parser and helpers are internal; integration tests exercise XMP behavior through the public injection API

**Metadata and API traps:**

- **No synthetic defaults** — When no `LegalMetadata` is provided, no copyright text, no usage terms, no `DateCreated` are emitted
- **Pixel-only paths drop file-level metadata** — `process_image`/`process_images_parallel` (`DynamicImage` in/out) embed stego markers only; PNG tEXt, JPEG COM/XMP, and WebP XMP do not survive. Use byte-path APIs (`process_request_bytes`, `process_image_bytes`) when metadata injection matters
- **`inject_metadata` / `inject_legal_claims` are `Option<bool>`** — Default `None` (use level default) vs explicit `false` (disable). `with_metadata_injection(false)` ≠ not calling it at all
- **`inject_legal_claims` auto-enables when `LegalMetadata` present** — No need to call `with_legal_claims(true)`
- **`has_notice()` includes DMI** — Returns true when any legal field OR `dmi.is_some()` is found. `DmiValue::Allowed` and `DmiValue::Unspecified` make `has_notice()` true — this means "legal metadata was found" not "restrictions were imposed"
- **`LegalMetadata::MAX_FIELD_LEN`** — 8192 bytes. `validate()` checks all 16 fields, returns `Error::Config` on violation
- **Verification returns `VerificationStatus`** — Not `Option<bool>`. Use `== VerificationStatus::Verified` in assertions

**Steganography details:**

- **Three seed storage locations** — (1) Q-table LSBs in JPEG, (2) metadata markers (strippable), (3) fixed-position LSB in first 64 pixel channels. Extraction chain: metadata → LSB fallback → `FALLBACK_SEEDS`
- **Spread spectrum LSB** — Each payload bit embedded across `STEGO_SPREAD_FACTOR * redundancy` (=5×r) RGB carrier slots via majority voting. The corrected V2 carrier uses a single bijective permutation over `width * height * 3` slots
- **Generic carrier core** — `stegoeggo-stego/src/lsb.rs`, `stegoeggo-stego/src/jpeg.rs`, and `stegoeggo-stego/src/frame.rs` are the stable generic carrier API. Low-level mechanics in `lsb_internal` and `jpeg_transcoder` are private implementation details
- **`stego_redundancy` is `Option<usize>`** — Default `None` derives from intensity via `effective_redundancy()` (<0.3→1, 0.3-0.7→2, >=0.7→3). Valid range 1-10
- **F5 redundancy cap** — Max redundancy is 10. Extraction tries all 10 values
- **Payload version 3 is current** — V1/V2 still supported for extraction only, never written. V3 adds TLV extensions with domain-separated authentication. V3 extraction paths tried first before V2/V1 fallback
- **Tiled steganography** (`with_tile_size(n)`) — Crop-resistant mode, full payload per tile. `tile_seed(master_seed, tile_x, tile_y)` uses splitmix64. Tiling is a Standard/canonical hidden-marker mode; legacy `Light` always resolves to `SeedOnly` regardless of tile size
- **F5 tiled block set** — MCU-interleaved: `block_idx = (mcu_y * mcus_per_row + mcu_x) * h * v + sub_y * h + sub_x`. Do NOT assume row-major ordering
- **JPEG DCT one-pass embed** — Supported DCT embedding computes max feasible redundancy from capacity, then embeds+encodes once. No retry loop, no roundtrip decode/extract self-test

**Canonical metadata format:**

- XMP writer emits `plus:DataMining` with full PLUS LDF URIs (e.g., `http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING`). Legacy bare keys are parsed for backward compatibility but not emitted
- Private `noai`/`noindex` and `DMI-PROHIBITED` tEXt/COM markers are no longer emitted in new output
- WebP legal fields live in XMP inside `<rdf:Description>`. `dc:rights` and `xmpRights:UsageTerms` use `<rdf:Alt><rdf:li>` containers. `dc:creator` uses `<rdf:Seq>`
- WebP exiftool: `exiftool -Copyright` does not resolve `dc:rights` — use `exiftool -XMP-dc:Rights`

**Warning system:**

- `ProtectionWarning` has 8 variants: `MissingMacKey`, `MetadataInjectionDisabled`, `ProgressiveJpegFallback`, `JpegReencodeFragile`, `LsbCapacitySkipped`, `DctCapacityInsufficient`, `ContradictoryLegalClaims`, `MissingRightsConstraints`
- `MissingRightsConstraints` is profile-dependent: only emitted for `ProhibitedSeeConstraints` without `ai_constraints` or `web_statement_of_rights`
- `severity_for_profile(profile)` classifies warnings as `Info`, `Warning`, or `Error`

## Architecture

- **Strategy pattern** via `Protector` trait (`src/traits.rs`) with three levels: Disabled, Light, Standard — see `architecture/traits.md`
- **Pipeline** (`src/lib.rs`): canonical `ProtectionRequest` execution is plan-driven; stateless `ProtectionPipeline` methods are compatibility adapters for legacy APIs — see `architecture/pipeline.md`
- **Direct plan executor** (`src/lib.rs`): `execute_metadata_only()`, `execute_stego_and_metadata()`, `execute_stego_and_metadata_tiled()`, `execute_seed_only_and_metadata()` — canonical execution from `ResolvedProtectionPlan`
- **Application stego adapter** (`src/protected/steganography/`): decomposed into five modules behind `SteganographyProtector` — `marker.rs` (V3 payload construction), `embed.rs` (carrier dispatch: LSB, tiled LSB, JPEG DCT/F5, seed-only), `extract.rs` (seed discovery and bounded search), `verify.rs` (integrity and authentication classification), `legacy.rs` (V1/V2 compatibility); `mod.rs` is the facade — see `architecture/protected-steganography.md`
- **Generic carrier core** (`stegoeggo-stego/src/`): Application-neutral LSB and JPEG DCT carrier mechanics — see `architecture/protected-steganography.md`
- **Public generic stego API** (`stegoeggo::stego`): Carrier-level embedding/extraction for arbitrary payload bytes, independent of the rights-protection pipeline — see `architecture/protected-steganography.md`
- **JPEG fast path**: When input/output are both JPEG, the application adapter calls the carrier's public encoded-byte operations, bypassing pixel decode/encode — see `architecture/jpeg-transcoder.md`
- **Policy-first API**: `ProtectionRequest` + `RightsPolicy` are the canonical API — see `architecture/types.md`
- **`#![forbid(unsafe_code)]`** throughout the library crate and `stegoeggo-stego`
- **Standalone carrier package wording** — `stegoeggo-stego` has its own package and public API surface, but current release checks enforce version lockstep across carrier/root/CLI

### Architecture Docs Index

Master index with repo layout, module maps, and data-flow diagrams: `architecture/overview.md`. Deep dives:

| Doc | Topic |
|-----|-------|
| `pipeline.md` | `ProtectionPipeline` orchestration, format routing, legacy adapters |
| `resolve.md` | `resolve_request()` single validation point → immutable `ResolvedProtectionPlan` |
| `types.md` | `RightsPolicy`, `ProtectionRequest`, `ProtectionPreset`, `ProtectionChannels`, `ExecutionReport` |
| `traits.md` | `Protector` trait contract + implementation table |
| `error.md` | `Error` enum (19 variants incl. structured resource-limit errors) |
| `constants.md` | Tuning constants in both crates (`STEGO_SPREAD_FACTOR`, seeds, payload sizes) |
| `protected-metadata-trap.md` | Metadata injection: canonical `plus:DataMining`, per-format writers, merge policies |
| `protected-steganography.md` | Application adapter (5 modules) + public generic carrier API (`stegoeggo::stego`) |
| `protected-passthrough.md` | No-op strategy for `Disabled` |
| `jpeg-header.md` | `JpegHeader` parser, checked scan-structure analysis |
| `jpeg-entropy.md` | Huffman codec (`CoefficientDecoder`/`CoefficientEncoder`) |
| `jpeg-stego-f5.md` | F5 DCT stego, Q-table seed, `DctCoefficientRng`, tiled F5 block order |
| `jpeg-transcoder.md` | Decode/encode flow, `DctSupport` probe, preserving encoding |
| `payload-v3.md` | V3 TLV wire format, domain-separated auth, ECC, multi-version parsing |
| `provenance.md` | `ProvenanceClaim` builder, canonical JSON, `TypedDigest` |
| `provenance-claim.md` | 15-field claim schema, binary encoding, test vectors |
| `signing.md` | Ed25519 signing (`signatures` feature), zeroize |
| `detached.md` | Detached manifest flow, `TrustPolicy` |
| `detached-manifest.md` | Manifest JSON schema and signing protocol spec |
| `verification.md` | `VerificationReport`, per-channel sub-results, `TrustEvaluation`, `EvidenceStrength` |
| `util-image.md` | `PixelSelectionRng`, encoding, format detection, hashing |
| `util-iscc.md` | ISCC via `iscc-lib` delegation (`iscc` feature) |
| `util-seed.md` | CSPRNG seed via `getrandom`, splitmix64 fallback |
| `async-api.md` | Tokio `spawn_blocking` wrappers (`async` feature) |
| `resource-limits.md` | Parser hardening, configurable limits, structured errors |
| `legal-metadata-field-mapping.md` | Legal field mapping across PNG/JPEG/WebP, round-trip caveats |
| `conformance.md` | Conformance harness, fixtures, strict mode, exit codes |
| `cli.md` | CLI flags, subcommands, exit codes, batch behavior |
| `adr-c2pa.md` | ADR: C2PA integration deferred |

## Validation Scripts

- `scripts/check.sh` — Fast deterministic checks used by local development and required CI (fmt, clippy, no-default-features, tests)
- `scripts/release-check.sh` — Bounded local pre-release readiness (runs check.sh + staged carrier/root/CLI package dry-runs + version lockstep verification). Use `--stage=pre|root|cli`; it never publishes
- `scripts/validate-docs-rs.sh` — Docs.rs-equivalent rustdoc validation (nightly, DOCS_RS=1, cfg(docsrs), workspace + packaged crate)
- `scripts/validate-msrv-package.sh` — Fresh MSRV consumer resolution (packages crate, creates clean consumers, tests minimal and all-feature combos on declared MSRV)
- `scripts/verify_metadata_conformance.sh` — Shell wrapper for conformance checks (delegates to Rust conformance harness)
- `scripts/check_fuzz_sync.sh` — Verifies fuzz harness parity between fuzz/Cargo.toml and fuzz.yml workflow
- `scripts/measure_binary_size.sh` — Measures compiled binary size for regression tracking

## Conformance Suite

The conformance harness (`src/bin/stegoeggo-conformance.rs`) validates metadata interoperability against ExifTool and xmllint. It produces JSON reports and is a mandatory pre-release check for metadata-affecting changes.

Key files: `src/conformance.rs` (report types), `tests/fixtures/conformance/manifest.toml` (fixture manifest with SHA-256 digests).

External integration tests in `tests/external_tools.rs` are `#[ignore]` — run with `--ignored`.

## Fuzzing

12 targets in `fuzz/fuzz_targets/`. Run with: `cargo +nightly fuzz run <target> -- -max_total_time=60`. Add regression tests in `tests/robustness.rs` for findings.

## Release Policy

Releases are manual. GitHub Actions must not publish crates or create releases.
Use direct Cargo/crates.io publication after local validation.
Do not push a version tag as a publication mechanism.
Published crates.io versions are immutable and cannot be reused.
Publication follows dependency order: carrier (`stegoeggo-stego`) first, then library, then CLI. All three crates share one version, wired with exact `=X.Y.Z` dependencies.
See `RELEASING.md` for the complete procedure.

## CI Complexity Guardrails

- Required push/PR workflows: one.
- Required jobs per push/PR: one.
- No required job matrix.
- No tag-triggered release workflows.
- No CI publication.
- No crates.io token in GitHub Actions.
- Do not add specialist checks to `scripts/check.sh`.
- Preserve specialist tests, but invoke them deliberately.
- Any increase to required CI surface requires an explicit maintainer decision.

## Repo-Local Agent Skills

`.skills/` contains agent skills with triggers (loaded on demand):
- `.skills/stegoeggo-conventions/SKILL.md` — code conventions, public API signatures, constants, common pitfalls
- `.skills/architecture-review/SKILL.md` — workflow for verifying `architecture/` docs against source, including known discrepancy patterns
- `.skills/plan-execution/SKILL.md` — multi-task execution from `plans/` via git worktrees + parallel agents

## Other Reference Files

- `CHANGELOG.md` — Keep-a-Changelog release notes (cut a version section when bumping versions)
- `DEPRECATIONS.md` — Deprecated API inventory (removal targeted at v1.0.0, never in 0.x)
- `SUPPORT.md` — Support matrix
- `STABILITY.md` — Stability tiers
- `RELEASING.md` — Manual publication procedure
- `plans/` — Numbered implementation plans (`NNN-name.md`) with `-status.md` companions; the authoritative record of what changed and why. Next plan number: 076+
- `examples/` — Four runnable examples (`protect_and_verify.rs`, `verify_saved.rs`, `legal_metadata.rs`, `generic_stego.rs`) referenced by `docs/rust-api.md`; keep them compiling when changing public APIs
- `docs/` — User-facing guides: `cli-usage.md`, `rust-api.md`, `carrier-crate.md`, `formats.md`, `legal_notice_model.md`, `migration-v0.3.md`
- `architecture/` — 30 architecture documents, verified against source; indexed in the table above and in `architecture/overview.md`
