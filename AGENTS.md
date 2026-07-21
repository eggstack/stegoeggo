# AGENTS.md

## Project Overview

`stegoeggo` is a Rust library and CLI for protecting images from unauthorized AI use through rights-reservation metadata and AI-training restriction notices.

## Tech Stack

- Rust (edition 2021, MSRV 1.87, stable channel)
- Key crates: `image` 0.25, `jpeg-encoder` 0.7, `rayon` 1.10, `sha2`/`hmac` for crypto, `serde`/`serde_json` for serialization, `subtle` 2 (constant-time comparisons), `iscc-lib` 0.4 (ISO 24138:2024 ISCC), `getrandom` 0.2 (CSPRNG), `tokio` (optional, for async), `ed25519-dalek` (optional, for signing)
- `unicode-normalization` 0.1: Used by conformance harness for NFC normalization during field comparison

## Design Philosophy

- **Legal deterrence, not anti-forensics**: The purpose is making protection detectable in legal proceedings, not preventing a determined adversary from removing it
- **Forgeability is irrelevant**: HMAC/checksum forgeability does not matter — the goal is detection, not authentication. A forger who can strip protection entirely has already lost the deterrence value
- **Robustness is the primary metric**: Steganographic payload survival against common image transformations (compression, resize, format conversion, social media re-encoding) is the most important engineering metric
- **Metadata stripping is a known limitation**: All image processing tools can strip metadata. This is why steganographic embedding is the primary detection channel — metadata is a bonus layer, not the core defense

## Workspace Structure

Three workspace members:
- `.` — Main library crate (`stegoeggo`) + conformance harness binary (`stegoeggo-conformance`)
- `stegoeggo-cli/` — CLI binary (`stegoeggo` binary name)
- `fuzz/` — Fuzz harnesses (requires `cargo-fuzz` + nightly)

## Architecture

- **Strategy pattern** via `Protector` trait (`src/traits.rs`) with three protection levels: Disabled, Light, Standard
- **Pipeline** (`src/lib.rs`): `ProtectionPipeline` orchestrates protectors based on `ProtectionLevel`
- **JPEG fast path**: When input/output are both JPEG, operates directly on DCT coefficients via custom transcoder (`src/jpeg_transcoder/`), bypassing pixel decode/encode
- **Cow returns**: `Protector::apply` returns `Result<Cow<'a, DynamicImage>>` to avoid unnecessary cloning

## Module Layout

```
src/
├── lib.rs                 # Pipeline, top-level functions, module exports
├── types.rs               # Core types (ProtectionLevel, ProtectionContext, etc.)
├── traits.rs              # Protector trait
├── error.rs               # Error enum (thiserror)
├── async_api.rs           # Async wrappers (spawn_blocking)
├── bin/
│   └── stegoeggo-conformance.rs  # Conformance harness binary
├── protected/             # Protection strategies
│   ├── constants.rs       # Tuning constants
│   ├── passthrough.rs     # No-op (Disabled)
│   ├── metadata_trap.rs   # Metadata injection (Light)
│   ├── steganography.rs   # LSB/DCT steganographic embedding
│   ├── ecc.rs             # 3x repetition ECC with majority voting
│   ├── notice_verification.rs # Legal notice verification (verify_legal_notice)
│   ├── resolve.rs         # Request resolution and validation
│   └── stego_cost.rs      # Pixel embedding cost computation (Laplacian-based)
├── jpeg_transcoder/       # JPEG-specific processing
│   ├── header.rs          # JPEG header parser
│   ├── entropy.rs         # Huffman entropy codec
│   └── stego_f5.rs        # F5-style DCT steganography
├── payload_v3/            # Payload v3 wire format (header, parser, types, errors)
├── provenance/            # Provenance claim model (claim, digest, canonical)
├── signing/               # Ed25519 signing (feature-gated: signatures)
├── detached/              # Detached signed manifests (feature-gated: detached-manifest)
├── verification/          # Structured verification report
└── util/
    ├── image.rs           # Encoding, hash
    ├── iscc.rs            # ISCC content identifiers
    └── seed.rs            # Random seed generation
```

## Key Types

- `ProtectionContext::new(intensity: f32, seed: u64)` — intensity clamped to [0.0, 1.0]
- `ProtectionConfig` — MAC key and legal metadata wrapped in `Arc`
- `StegoPayload` — extracted stego data with `protection_level()`, `seed()`, `intensity()`, `version()` getters
- `EvidenceProfile` — enum with 4 variants: `LegalNotice` (default), `LegalNoticeWithStego`, `AuthenticatedProvenance`, `Maximal`. Controls warning interpretation and evidence posture. Access via `ctx.evidence_profile()` (defaults to `LegalNotice` when not set). **Deprecated** — use `ProtectionPreset` instead
- `RightsPolicy` — Explicit data-mining policy enum. Maps 1:1 to `DmiValue`. Never inferred from processing intensity
- `ProtectionChannels` — Configuration of protection channels (rights_metadata, hidden_marker, authentication)
- `ProtectionRequest` — Request-based API entry point combining notice, policy, channels, and processing options
- `ResolvedProtectionPlan` — Immutable execution plan produced by `resolve_request()`. Consumed by pipeline stages
- `ProtectionPreset` — Executable presets (LegalNotice, LegalNoticeWithStego, AuthenticatedProvenance, Maximal) that expand into `ProtectionChannels`
- `ExecutionReport` — Tracks which channels executed and any degradation during processing
- `HiddenMarkerMode` — Controls steganographic embedding: Disabled, BestEffort, Tiled { tile_size }
- `AuthenticationMode` — Controls payload authentication: None, Hmac
- `ProcessingOptions` — Image processing options (output format, JPEG quality, etc.)
- `MetadataUpdatePolicy` — controls merge behavior on repeated processing: `ReplaceStegoOwned` (default), `FailOnConflict`, `PreserveExisting`
- `NoticeVerification` — use `NoticeVerification::builder()` for construction (26-field builder pattern). Deprecated `new()` positional constructor still available for backward compatibility
- `ProtectionContext` helper constructors: `::legal_notice()`, `::legal_notice_with_stego()`, `::authenticated_provenance()`, `::maximal()`
- All struct fields on `ProtectionContext` and `StegoPayload` are private — use getter methods (e.g., `ctx.intensity()`, `ctx.seed()`)
- `ProtectionContext` has `set_input_format()` (public) and `set_protection_level()` (crate-internal) for non-builder mutation
- `ProvenanceClaim` — Canonical provenance assertion
- `DetachedManifest` — Signed manifest sidecar
- `SigningKey` / `VerifyingKey` — Ed25519 key types (signatures feature)
- `VerificationReport` — Structured verification results
- `PayloadV3Header` — v3 payload header

## Features

| Feature | Description | Default |
|---------|-------------|---------|
| `async` | Tokio-based async API wrappers | No |
| `signatures` | Ed25519 signing and key management | No |
| `detached-manifest` | Detached signed manifest sidecar support | No |

## Build & Test Commands

```bash
cargo check                              # Compilation
cargo test                               # All tests (718 passed, 27 ignored)
cargo test --all-features                # Includes signing and detached manifest tests
cargo clippy --all-targets -- -D warnings # Lint check
cargo fmt --check                        # Format check
cargo package --workspace --allow-dirty  # Package dry-run
cargo bench                              # Criterion benchmarks
cargo test --test external_tools -- --ignored    # External tool integration tests
cargo build --release --bin stegoeggo-conformance  # Build conformance harness
./target/release/stegoeggo-conformance --fixtures tests/fixtures/conformance --manifest tests/fixtures/conformance/manifest.toml --strict  # Run conformance
./scripts/validate-release.sh                     # Full release validation
./scripts/validate-release.sh --skip-external     # Hermetic-only validation
```

## CI Pipeline

GitHub Actions (`.github/workflows/ci.yml`) runs:
- Concurrency control (cancel superseded runs)
- `workflow_dispatch` trigger for manual runs
1. MSRV check (`cargo check --all-features` with Rust 1.87)
2. Tests + doc tests (`cargo test --all-features`, `cargo test --doc`)
3. Format + clippy lint (`cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`)
4. Security audit (`cargo audit`)
5. License/advisory check (`cargo deny check licenses && cargo deny check advisories`)
6. Package dry-run (`cargo package --workspace --allow-dirty`)
7. External integration tests (`cargo test --test external_tools -- --ignored`, installs exiftool/xmllint/imagemagick/libvips)
8. External Conformance (`stegoeggo-conformance --strict`, uploads JSON report)
9. Benchmarks (manual dispatch only)

## Release Gate

Before any release, verify locally:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --doc
cargo package --workspace --allow-dirty
cargo deny check licenses
cargo deny check advisories
cargo test --test external_tools -- --ignored
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance --fixtures tests/fixtures/conformance --manifest tests/fixtures/conformance/manifest.toml --strict
```

Or use the centralized script:
```bash
./scripts/validate-release.sh                     # Full validation (hermetic + external)
./scripts/validate-release.sh --skip-external     # Hermetic-only validation
```

All checks must pass. MSRV is verified by CI (Rust 1.87). Benchmarks are run via manual workflow dispatch.

## Code Conventions

- Rustfmt: 4-space indentation, max width 100
- No comments in code unless explicitly asked
- `#[must_use]` on builder methods
- `pub(crate)` for internal modules (e.g., `jpeg_transcoder`)
- Private fields with getter methods on `ProtectionContext`, `StegoPayload`, `LegalMetadata`

## Architecture Documentation

Architecture docs live in `architecture/` (24 files). All docs have been verified against source code.

## Validation Scripts

- `scripts/validate-release.sh` — Centralized release validation. Runs hermetic phase (fmt, clippy, tests, package, deny) and optional external phase (external integration tests + conformance). Use `--skip-external` for environments without external tools.
- `scripts/verify_metadata_conformance.sh` — Shell wrapper for running conformance checks.

## Fuzzing

Three fuzz targets in `fuzz/`:
- `pipeline_bytes` — Full bytes-in/bytes-out path
- `tiled_round_trip` — Tiled steganography end-to-end
- `jpeg_parser` — JPEG header/entropy/DCT parsing

Run with: `cargo +nightly fuzz run <target> -- -max_total_time=60`

## Conformance Suite

The independent conformance suite (`src/bin/stegoeggo-conformance.rs`) validates
metadata interoperability against external parsers. It produces machine-readable
JSON reports and is a mandatory CI gate.

- `src/conformance.rs` — Report types (`ConformanceReport`, `ConformanceRunReport`, `CheckSeverity`, `FixtureManifest`, etc.)
- `src/bin/stegoeggo-conformance.rs` — Harness binary (CLI + internal/external extraction)
- `tests/conformance_harness_tests.rs` — Regression tests for report types and harness configuration
- `tests/external_tools.rs` — External tool integration tests (`#[ignore]`, run with `--ignored`)
- `tests/fixtures/conformance/` — Fixture taxonomy (canonical, legacy, malformed, conflicting, preservation)
- `tests/fixtures/conformance/manifest.toml` — Machine-readable manifest with SHA-256 digests and expected values
- `scripts/verify_metadata_conformance.sh` — Shell wrapper for operator convenience
- `scripts/validate-release.sh` — Full release validation (hermetic + external phases)

The harness performs:
1. Fixture manifest loading and SHA-256 digest verification
2. Manifest structural validation (duplicate IDs, duplicate paths, empty IDs, path traversal, invalid formats/categories, SHA-256 validity)
3. Format detection via magic bytes
4. Internal extraction via `verify_legal_notice()`
5. External extraction via ExifTool
6. XMP well-formedness validation via xmllint
7. Normalized field-by-field comparison (Unicode NFC, URL canonicalization, whitespace normalization, creator array ordering)
8. Coverage enforcement (explicit per-category and per-format minimums, including source-aware external minimums)
9. Versioned run report envelope (`ConformanceRunReport`) with `complete`/`passed` semantics
10. JSON and human-readable output

External integration tests in `tests/external_tools.rs` are `#[ignore]` and run explicitly with `--ignored`.

Required external tools: `exiftool`, `xmllint`, `imagemagick`, `libvips` (installed in both CI and release workflows)

Stable exit codes: 0=pass, 1=fail, 2=config error, 3=digest mismatch, 4=coverage violation, 5=internal error

## Things to Watch Out For

- **Stego payload format**: 24-byte header + 4-byte CRC32 checksum (or 8-byte HMAC), always padded to 32 bytes total. Use `MIN_PAYLOAD_SIZE` (=28) and `MIN_PAYLOAD_BITS` (=224) constants in `steganography.rs`. Note: `MIN_PAYLOAD_SIZE` is a parsing threshold, not the output size — non-MAC payloads are 76 bytes (ECC-encoded), MAC payloads are 32 bytes
- **`ProtectionContext::default()`**: Uses `getrandom` (CSPRNG) for an unpredictable seed. For reproducible results, use `ProtectionContext::new(intensity, seed)` with an explicit seed
- **XorShiftRng — two separate implementations**: `src/util/image.rs` has `PixelSelectionRng` (pixel selection) and `src/jpeg_transcoder/stego_f5.rs` has `DctCoefficientRng` (DCT shuffling). Different algorithms, different sequences for same seed. Do NOT interchange them
- **Canonical rights metadata**: XMP writer emits `plus:DataMining` with PLUS LDF vocabulary keys (e.g., `DMI-PROHIBITED-AIMLTRAINING`). Legacy `Iptc4xmpExt:DMI-*` properties are still parsed for backward compatibility but not emitted by default. `DmiValue::plus_vocab_key()` returns the canonical key; `DmiValue::from_plus_vocab_key()` parses it back.
- **TDM reservation removed from image output**: `tdm:reserve_tdm` is no longer emitted. TDMRep is a web-distribution mechanism (HTTP headers, `/.well-known/tdmrep.json`), not an image-metadata signal. Legacy files with `tdm:reserve_tdm` remain parseable.
- **RightsSignalKind**: `NoticeVerification::rights_signal_kind()` classifies the source of the DMI signal as `CanonicalPlusDataMining`, `LegacyStegoEggoDmi`, `LegacyTdmReservation`, or `Unknown`.
- **DMI conflict detection**: `NoticeVerification::has_dmi_conflict()` returns true when both canonical and legacy DMI values are found and disagree.
- **MetadataTrapProtector::apply()**: Returns `Cow::Borrowed(img)` unchanged — metadata injection is byte-level. The pipeline routes `Light` through `apply_light_bytes()` which encodes → injects → decodes. For byte-level output with metadata, use `apply_bytes()` or `process_bytes()`
- **`stego_redundancy` is `Option<usize>`**: Default `None` derives from intensity via `effective_redundancy()` (<0.3→1, 0.3-0.7→2, >=0.7→3). Explicit `.with_stego_redundancy(n)` overrides this. Valid range 1-10
- **Verification API**: `verify_payload_with_key()` and `verify_image_bytes()` return `VerificationStatus` (`Verified`, `Invalid`, `NotFound`), not `Option<bool>`. Use `== VerificationStatus::Verified` in assertions. For richer info, use `verify_image_bytes_detailed()` → `VerificationResult`. For comprehensive legal notice verification, use `verify_legal_notice()` → `NoticeVerification` with `EvidenceStrength` and `EvidenceChannel`
- **`inject_metadata` / `inject_legal_claims` are `Option<bool>`**: Default `None` (use level default) vs explicit `false` (disable). `with_metadata_injection(false)` ≠ not calling it at all. `inject_legal_claims` defaults to auto-injecting when `LegalMetadata` is present — no need to call `with_legal_claims(true)`. Explicitly setting `with_legal_claims(false)` still disables injection
- **No synthetic defaults**: When no `LegalMetadata` is provided, no "All Rights Reserved" copyright text, no default usage terms, and no `DateCreated` are emitted. Each field is emitted only when explicitly provided — no silent synthesis of missing fields. `DateCreated` is never populated from processing time; only caller-supplied creation dates are emitted
- **Contact not written to `photoshop:Credit`**: Contact remains in PNG tEXt and JPEG COM markers but is not mapped to the `photoshop:Credit` XMP property
- **Pipeline flow order**: JPEG output: encode → DCT stego → metadata. Non-JPEG: pixel stego → encode → metadata. JPEG→JPEG fast path bypasses pixel decode entirely
- **F5 seed embedding Q-table edge case**: `embed_seed_in_quantization_tables()` fails if any quantization value in the first 2 tables is < 2. Values of 1 cannot represent a 0-bit (`1 & 0xFE = 0`, clamped back to 1)
- **`#[serde(skip)]` on `config` field**: `ProtectionContext.config` is skipped during serialization. MAC keys and legal metadata are lost in serde roundtrips
- **Async batch processing**: `process_images_parallel_async` runs the entire batch inside a single `spawn_blocking`, delegating to rayon-based sync functions. Single-image async uses one `spawn_blocking` per image
- **Three seed storage locations**: (1) Q-table LSBs in JPEG (survives byte-preserving metadata insertion; not arbitrary recompression), (2) metadata markers (strippable by any tool), (3) fixed-position LSB in first 64 pixel channels. Extraction chain: metadata → LSB fallback → `FALLBACK_SEEDS`
- **ECC on stego payload**: Non-MAC payloads use 3× repetition with majority voting before CRC32. The `ecc` module provides `ecc_encode`/`ecc_decode`. MAC payloads use 8-byte HMAC instead
- **Spread spectrum LSB**: Each payload bit embedded across `STEGO_SPREAD_FACTOR` (=5) adjacent pixels via majority voting
- **Large-magnitude DCT coefficient preference**: F5 sorts non-zero AC coefficients by |magnitude| descending before shuffling
- **F5 redundancy cap**: Max redundancy is 10. Extraction tries all 10 values
- **Payload version migration**: Current version is 3. V1 (24-byte) and V2 (24-byte header + ECC/HMAC) still supported for extraction. V3 adds TLV extensions with domain-separated authentication
- **CLI file path**: CLI binary lives at `stegoeggo-cli/src/main.rs`, not `src/bin/`
- **CLI batch filename collisions**: Duplicate output stems get `_protected_1`, `_protected_2`, etc.
- **CLI legal metadata flags**: `--copyright-holder`, `--creator`, `--contact`, `--rights-url`, `--usage-terms`, `--ai-constraints`, `--credit-line`, `--copyright-owner`, `--licensor-name`, `--licensor-email`, `--licensor-url`, `--content-created-at` set `LegalMetadata` fields. `--no-ai-training`, `--no-genai-training`, `--tdm-reserved` are DMI presets that also set default `ai_constraints` text. Any legal flag auto-enables legal claims (no explicit `--legal-claims` needed). `--metadata false` + legal flags → error. `--strict` exits with error if any warnings have Error severity. `--tdm-reserved` is deprecated (TDMRep deployment deferred)
- **Tiled steganography** (`with_tile_size(n)`): Crop-resistant mode. Embeds full payload per tile. Tiled F5 limited to tile-aligned crops without re-encode. `tile_seed(master_seed, tile_x, tile_y)` uses splitmix64. Tiled paths are verification fallbacks
- **F5 tiled block set**: MCU-interleaved block ordering: `block_idx = (mcu_y * mcus_per_row + mcu_x) * h * v + sub_y * h + sub_x`. Do NOT assume row-major ordering
- **ProtectionWarning variants**: 6 variants: `MissingMacKey`, `MetadataInjectionDisabled`, `ProgressiveJpegFallback`, `JpegReencodeFragile`, `LsbCapacitySkipped`, `DctCapacityInsufficient`. Returned by `process_image_bytes_with_warnings`
- **WarningCategory / WarningSeverity**: `ProtectionWarning` has `category() -> WarningCategory` and `severity_for_profile(profile) -> WarningSeverity` methods. Categories: `LegalNotice`, `BestEffortStego`, `AuthenticatedProvenance`, `FormatFragility`. Severities: `Info`, `Warning`, `Error`
- **Fuzz harness**: 3 targets in `fuzz/`: `pipeline_bytes`, `tiled_round_trip`, `jpeg_parser`. Run with `cargo +nightly fuzz run <target> -- -max_total_time=60`. Add regression tests in `tests/robustness.rs` for findings
- **EvidenceProfile vs ProtectionLevel**: `ProtectionLevel` controls how much processing occurs (Disabled/Light/Standard). `EvidenceProfile` controls how warnings are interpreted and the default evidence posture. They are orthogonal — you can use any profile with any level
- **MissingMacKey warning is profile-dependent**: Only emitted for `AuthenticatedProvenance` and `Maximal` profiles. `LegalNotice` and `LegalNoticeWithStego` do not warn about missing MAC keys
- **`NoticeVerification`, `EvidenceStrength`, `EvidenceChannel`**: Types in `src/types.rs` for legal-notice conformance reporting. `verify_legal_notice()` in `src/protected/notice_verification.rs` extracts legal fields from PNG tEXt, JPEG COM, and WebP XMP markers, identifies evidence channels (JpegXmp, JpegIptc, PngXmp, etc.), verifies stego payload integrity (LsbPayload for non-JPEG, DctPayload for JPEG), and returns an evidence strength rating
- **Creator metadata injection**: `add_legal_metadata()` in `metadata_trap.rs` injects `Creator` as a tEXt chunk (PNG) or COM marker (JPEG). Ensure `LegalMetadata::creator()` is wired into both injection and extraction paths
- **`QTableSeed` is reserved**: The `EvidenceChannel::QTableSeed` variant is defined but not emitted by `verify_legal_notice()`. Q-table seeds are extracted during stego verification, not notice verification. Do not rely on this channel in consumer code
- **`has_notice()` includes DMI**: `has_notice()` returns true when any legal field OR `dmi.is_some()` is found. `DmiValue::Allowed` and `DmiValue::Unspecified` will make `has_notice()` true — this means "legal metadata was found" not "restrictions were imposed"
- **WebP legal fields in XMP**: WebP outputs carry legal metadata (copyright, creator, contact, rights URL, usage terms, AI constraints) in standard XMP properties alongside DMI/TDM. Legal child elements live inside `<rdf:Description>...</rdf:Description>` (not as attributes on its opening tag). `dc:rights` and `xmpRights:UsageTerms` are wrapped in `<rdf:Alt><rdf:li xml:lang="x-default">…</rdf:li></rdf:Alt>` containers. `dc:creator` uses `<rdf:Seq>`. External tool visibility depends on parser support for `dc:rights`, `dc:creator`, `xmpRights:*`, `photoshop:Credit`, and `stegoeggo:AIConstraints` namespaces. `extract_xmp_alt_property` in `notice_verification.rs` extracts the inner `<rdf:li>` value from such containers
- **WebP XMP exiftool field aliases**: `exiftool -Copyright` does not resolve `dc:rights` for WebP — use `exiftool -XMP-dc:Rights`. The conformance script accepts `XMP-dc:Rights` as a parser-visible alias for copyright so external viewers see the rights statement
- **`--verify` exits 0**: CLI verify mode always exits 0 regardless of stego status. Use output text to determine protection state, not exit code
- **`LegalMetadata::MAX_FIELD_LEN`**: 8192 bytes. `validate()` checks all 8 fields and returns `Error::Config` on violation. Called by library pipeline entry points (`process_image_bytes`, `process_image_bytes_with_warnings`). `Disabled` level skips validation
- **`NoticeVerification::new()` positional arguments**: `NoticeVerification::new()` now takes 26 positional arguments (was 18 in v0.2.0). The additional fields are: `license_url`, `web_statement_of_rights`, `credit_line`, `copyright_owner`, `licensor_name`, `licensor_email`, `licensor_url`, `metadata_date`, `notice_applied_at`. **Deprecated** in favor of `NoticeVerification::builder()` pattern for field-named construction
- **`photoshop:Credit` maps to `credit_line`**: In WebP XMP, `photoshop:Credit` now maps to `credit_line`, not `contact`. The previous mapping was semantically incorrect
- **Metadata overflow checks**: PNG chunk lengths use `u32::try_from()`, JPEG marker lengths use `u16::try_from()`. All 6 helper functions in `metadata_trap.rs` return `Result<Vec<u8>>` — `create_png_xmp_chunk`, `create_png_text_chunk`, `create_jpeg_xmp_marker`, `create_jpeg_exif_marker`, `create_jpeg_iptc_marker`, `create_jpeg_comment`. Overflow returns `Error::Metadata`
- **`--require-complete` is removed**: `--strict` is the single complete-validation mode. `--require-complete` no longer exists
- **`CoverageMinimums` no longer has `external_coverage_pct`**: Uses `malformed_per_format` (=1 per format) instead. Coverage is enforced via explicit per-category and per-format minimums, not a blanket percentage
- **`ConformanceReport` has `fixture_id`, `category`, `source` fields**: These `Option<String>` fields link reports to manifest entries. They are populated from the manifest and skipped in JSON serialization when `None`
- **`validate_manifest()` checks structure before processing**: Validates duplicate IDs, duplicate paths, empty IDs, path traversal, unsupported formats/categories/sources, and SHA-256 validity. Returns `Err(Vec<String>)` on violations. Called before any fixtures are processed
- **DMI normalization precedence**: `normalize_dmi_value()` matches in order: search engine indexing > gen AI/ML training > AI/ML training > see constraints > generic prohibited > allowed. This ordering ensures the most specific prohibition is returned
- **Policy-first architecture (Release 4)**: `ProtectionRequest` and `RightsPolicy` are the canonical API. `ProtectionLevel` and `EvidenceProfile` are deprecated compatibility adapters. New code should use `process_request_bytes()` and `process_request_bytes_with_report()`
- **Deprecated API surfaces**: `EvidenceProfile`, `with_dmi()`, `with_metadata_injection()`, `with_inject_legal_claims()` are deprecated. They still work but will be removed in the next major version
- **Metadata-only fast path**: `ProtectionRequest::metadata_only()` produces same-format output without pixel/stego processing. Use this for the simplest legal-notice workflow
- **Resolution runs once**: `resolve_request()` validates all input and produces an immutable `ResolvedProtectionPlan`. Pipeline stages consume the plan rather than re-querying mutable context
- **Payload v3 uses TLV extensions with domain-separated authentication**: v3 payloads support arbitrary key-value extensions with per-domain MAC keys. Domain separation ensures cross-domain forgery is infeasible
- **SigningKey does NOT implement Serialize — private keys are never serialized**: `ed25519_dalek::SigningKey` is kept out of serde. Use `to_bytes()` / `from_bytes()` for explicit key serialization
- **ISCC API names are deprecated — use compute_content_identifiers() instead**: `compute_iscc()` is renamed to `compute_content_identifiers()`. The old name is deprecated
- **VerificationReport replaces broad VerificationStatus with structured sub-results**: `VerificationReport` contains per-channel results (`StegoResult`, `MetadataResult`, `SigningResult`) instead of a single overall status
