# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.3] - Unreleased

### Fixed
- V3-first extraction: all LSB/DCT/tiled extraction paths now try V3 CRC (288 bits) and V3 HMAC (384 bits) sizes first, falling back to V2/V1 for backward compatibility
- V3 channel flags: `authentication` flag now reflects actual HMAC usage (`ctx.mac_key().is_some()`) instead of hardcoded `true`
- DCT capacity warnings now use V3 payload sizes instead of legacy v2 constants
- Embedded reference verification now uses raw byte extraction (`extract_payload_from_bytes_with_key`) instead of DynamicImage decode, preserving JPEG quantization tables and metadata seeds
- Detached-manifest trust evaluation: `report.trust().trusted()` now correctly reflects per-signature trust (caller policy + cryptographic validity) instead of always returning `false`
- Resource limits: `max_width`/`max_height` from `ResourceLimits` are now enforced unconditionally on all processing paths, even when no explicit `max_dimension` is set
- CLI `verify-manifest` and `sign` use bounded parsing via `DetachedManifest::from_json_with_limits`

### Added
- 38 new tests covering V3 extraction (CRC/HMAC, PNG/JPEG/tiled), malformed v3, missing/wrong HMAC key, channel flags, detached verification (trust, embedded references, oversized manifests), and resource-limit enforcement (input size, dimensions, payload, tile origins, verification seeds)
- Resource closure table documenting all 18 `ResourceLimits` fields with enforcement sites and tests
- `SteganographyProtector::generate_payload_for_context` (test helper) for inspecting V3 payload structure
- `EXIT_TRUST` (exit code 4) for valid-but-untrusted detached-manifest verification
- Detached manifest structural validation: duplicate keys/signatures, hex digest validation, key length checks
- `DetachedOverallStatus` enum for overall verification verdict with structured exit codes
- `EmbeddedReferenceStatus` expanded with `PresentValid`, `AuthenticationKeyMissing`, `AuthenticationFailed`, `UnsupportedVersion`
- CLI `--payload-key` option for HMAC payload verification
- CLI public key parsing now preserves `key_id` from PEM files
- V3 extraction: `probe_v3_header` now returns `None` for non-v3 data (cleaner fallthrough)
- Detached manifest signing output is now atomic (write-to-temp + rename)
- Header-driven v3 extraction: all 8 extraction paths (LSB, DCT, tiled LSB, tiled DCT, and verify variants) now check v3 magic from the first extracted candidate and re-extract with the exact `total_bits` declared in the v3 header
- `CandidateOutcome` expanded with `MalformedV3`, `UnsupportedVersion`, `AuthenticationKeyMissing`, `AuthenticationFailed` for structured v3 failure reporting
- `V3ProbeResult` enum and `probe_v3_header_from_lsb`/`classify_v3_probe` functions for v3 header probing
- `extract_lsb_range` for bit-range LSB extraction (prepared for future use)
- Behavior tests for resource limit enforcement through public entrypoints (tile origins, verification seeds, request API input size, request API dimensions)

## [0.2.2] - Unreleased

### Added
- `LegalMetadata` fields: `credit_line`, `copyright_owner`, `licensor_name`, `licensor_email`, `licensor_url`, `metadata_date`, `notice_applied_at`
- `NoticeVerification` fields: `license_url`, `web_statement_of_rights`, `credit_line`, `copyright_owner`, `licensor_name`, `licensor_email`, `licensor_url`, `metadata_date`, `notice_applied_at`
- `MetadataUpdatePolicy` enum (`ReplaceStegoOwned`, `FailOnConflict`, `PreserveExisting`) for controlling behavior on repeated processing
- CLI flags: `--credit-line`, `--copyright-owner`, `--licensor-name`, `--licensor-email`, `--licensor-url`, `--content-created-at`
- Auto-computed `notice_applied_at` timestamp (RFC 3339) when not explicitly provided
- Cross-format semantic-equivalence test suite (19 scenarios)
- Merge policy tests (11 tests)
- Field-mapping audit architecture document
- Conformance harness: `DecodeExpectation`, `XmpExpectation`, `ExtractionExpectation` enums for per-fixture expected outcomes
- Conformance harness: `ExternalToolError` type preserving tool name, executable, exit status, stderr, and JSON parse failure
- Conformance harness: per-fixture ImageMagick (`identify`) and libvips (`vipsheader`) execution
- Conformance harness: `coverage_minimums` field in JSON report envelope
- Conformance harness: `required_external_fields` per-fixture enforcement
- Regression tests for manifest validation, DMI normalization, and harness configuration (15+ new tests)
- Legal-notice verification API: `verify_legal_notice()` and `NoticeVerification`.
- Evidence profiles (`EvidenceProfile`): `LegalNotice`, `LegalNoticeWithStego`, `AuthenticatedProvenance`, `Maximal`.
- CLI legal metadata flags: `--copyright-holder`, `--creator`, `--contact`, `--rights-url`, `--usage-terms`, `--ai-constraints`, `--no-ai-training`, `--no-genai-training`, `--tdm-reserved`.
- External metadata conformance script (`scripts/verify_metadata_conformance.sh`).
- Evidence channel reporting via `NoticeVerification::channels()`.
- `RightsSignalKind` enum for classifying rights-signal source (canonical, legacy, unknown).
- `PLUS_NAMESPACE` and `PLUS_DATA_MINING_PROPERTY` constants.
- Added `src/conformance.rs` module with `ConformanceReport`, `CheckSeverity`, `ExternalExtraction`, and `InternalExtraction` types for machine-readable conformance reporting
- Added `stegoeggo-conformance` binary — independent interoperability and conformance harness that validates metadata against ExifTool and xmllint
- Added conformance fixture taxonomy in `tests/fixtures/conformance/` (canonical, legacy, malformed, conflicting, preservation categories)
- Converted `scripts/verify_metadata_conformance.sh` to a thin wrapper delegating to the Rust conformance harness
- Added mandatory `External Conformance` CI job that installs exiftool + xmllint, runs the harness in strict mode, and uploads JSON reports as artifacts
- Added conformance gating to the release workflow — publication blocked unless conformance passes
- Cross-format semantic equivalence tests now cover 15 legal metadata scenarios across PNG, JPEG, and WebP
- External tool integration tests validate ExifTool extraction, XML validation, and ImageMagick smoke checks

### Fixed
- WebP extraction now reads `photoshop:Credit` as `credit_line` (was incorrectly mapped to `contact`)
- `creation_date` now extracts from WebP XMP `photoshop:DateCreated`
- `license_url` now emits to WebP XMP `xmpRights:WebStatement` (was missing)
- `rights_url` collision split: `license_url` and `web_statement_of_rights` are now distinct in `NoticeVerification`
- JPEG XMP namespace matching fix: `windows(28)` instead of `windows(29)` for `http://ns.adobe.com/xap/1.0/`
- Auto-enable legal claims when `LegalMetadata` is provided (no explicit `with_legal_claims(true)` needed)
- Conformance harness: external extraction errors now produce typed `ExternalToolError` instead of silent defaults
- Conformance harness: malformed fixtures with expected decode failure now correctly evaluate decode expectations
- Fixture provenance: 8 fixtures with empty `authoring_tool_version` now have pinned versions
- WebP outputs now carry legal metadata fields (copyright, creator, contact, rights URL, usage terms, AI constraints) in XMP, bringing WebP to parity with PNG/JPEG for external rights-reservation notice visibility.
- WebP XMP RDF structure: legal child elements (`dc:creator`, `dc:rights`, `xmpRights:UsageTerms`, `xmpRights:WebStatement`, `photoshop:Credit`, `stegoeggo:AIConstraints`) now appear inside the closed `<rdf:Description>` element, with `dc:rights` and `xmpRights:UsageTerms` wrapped in `<rdf:Alt>` containers. `exiftool` and other RDF-aware parsers now expose every legal field. WebP conformance script accepts `XMP-dc:Rights` as a parser-visible alias for copyright.
- Updated stegoeggo XMP namespace URL from `anomalyco/stegoeggo` to `eggstack/stegoeggo`.
- Panic on truncated PNG iTXt chunks in `verify_legal_notice()` extraction path.
- Single-file `--output` now correctly treats file paths as output files instead of directories.
- JPEG XMP evidence channel reporting.
- JPEG XMP namespace marker.
- Evidence channel misclassification for payload type.
- Semantic correctness: copyright notice, creator, contact, and credit-line concepts are no longer conflated across format writers.
- WebP XMP no longer maps contact to `photoshop:Credit` (wrong semantics).

### Changed
- Version bumped to 0.2.2 (new public API)
- `photoshop:Credit` in XMP now maps to `credit_line` (semantically correct), not `contact_email`
- Conformance manifest now includes explicit `expected_decode`, `expected_xmp`, `expected_internal`, `expected_external`, `required_external_fields` per fixture
- Conformance harness strict mode requires ImageMagick and libvips when available
- **Canonical rights metadata**: XMP writer now emits `plus:DataMining` with official PLUS LDF controlled-vocabulary URIs (`DMI-PROHIBITED-AIMLTRAINING`, etc.) instead of legacy `Iptc4xmpExt:DMI-*` properties. This is the canonical machine-readable rights signal per the PLUS License Data Format specification.
- TDM reservation (`tdm:reserve_tdm`) is no longer emitted in image metadata by default. TDMRep is a web-distribution mechanism, not an image-metadata signal. Legacy files containing `tdm:reserve_tdm` remain parseable.
- `DmiValue` now has `plus_vocab_key()` and `from_plus_vocab_key()` methods for canonical PLUS vocabulary mapping.
- `NoticeVerification` now reports `canonical_dmi()`, `legacy_dmi()`, `rights_signal_kind()`, and `has_dmi_conflict()` for detailed rights-signal diagnostics.
- Metadata conformance script checks canonical `plus:DataMining` by default, with legacy fallback.
- CI now includes package dry-run check and manual-dispatch benchmarks.
- `cargo-deny` configuration tightened (template comments removed).
- Default public framing is legal notice and rights-reservation metadata.
- Missing MAC warning is profile-aware and no longer emitted for legal-notice mode.
- CLI verification now reports legal-notice fields and evidence strength.
- `QTableSeed` evidence channel marked as reserved (not emitted by current verification path).
- CI clippy now runs with `--all-features`.
- Legal metadata semantic corrections: `LegalMetadata` now auto-enables claim injection when provided (no need for explicit `with_legal_claims(true)`).
- No synthetic defaults: when no `LegalMetadata` is provided, no "All Rights Reserved", default usage terms, or DateCreated are emitted. Only explicitly provided fields are written.
- Contact information is no longer written to `photoshop:Credit` in XMP — it remains in PNG tEXt and JPEG COM markers only.
- `DateCreated` is no longer populated from the current processing time. Only caller-supplied creation dates are emitted.
- Each legal metadata field is now emitted only when explicitly provided, eliminating silent synthesis.

## [0.2.1] - 2026-06-04

### Added
- **Crop-resistant tiled steganography** via `with_tile_size(n)` on `ProtectionContext`:
  - Embeds the full payload in each `tile_size × tile_size` tile independently
  - LSB tiled path for PNG/WebP; F5 tiled path for JPEG
  - Any crop containing ≥1 intact tile is recoverable
  - Configurable `tile_extraction_max_origins` to bound extraction time
- **Metadata injection** via `MetadataTrapProtector` — IPTC DMI tags, TDM reservation (ISO/IEC 21000-21), XMP packets, EXIF UserComment for AI-training opt-out markers
- **Steganographic watermarking** via `SteganographyProtector`:
  - LSB embedding for PNG/WebP with spread-spectrum, content-adaptive amplitude, and 3× repetition ECC
  - F5-style DCT coefficient embedding for JPEG with no-zero coefficient variant (avoids shrinkage detection)
  - Seed embedded in JPEG quantization tables (survives byte-preserving metadata insertion; not arbitrary recompression)
- **Three protection levels**: `Disabled`, `Light`, `Standard`
- **Verification API** (`verify_image_bytes`, `verify_payload`, `extract_payload`) with optional HMAC-SHA256 key verification
- **ISCC-like content identification** via `compute_iscc`
- **Async API** (behind `async` feature) for WAF/CDN edge integration
- **Parallel batch processing** via Rayon
- **JPEG fast path** (`JpegTranscoder`) operating on DCT coefficients without pixel decode/encode
- **Legal metadata injection** for copyright, contact, license, and usage terms
- **CLI tool** (`stegoeggo`) with batch processing and format conversion

### Changed
- MSRV: 1.87
- Byte APIs preserve detected input format by default; the CLI defaults to PNG output unless `--format` is set
- Stego redundancy now 1-10 (was 1-5)
- Default stego redundancy derived from intensity: 1 (<0.3), 2 (0.3-0.7), 3 (≥0.7)

### Fixed
- JPEG segment length bounds handling in header parser
- Runtime check for non-8-multiple bit lengths in `bits_to_bytes`
- JPEG redundancy embedding bug (embedded = 0 reset between passes)
- Dimension validation in `process_bytes` for both JPEG and non-JPEG paths

## [0.1.0] - 2026-03-13

Initial release.

[Unreleased]: https://github.com/eggstack/stegoeggo/compare/v0.2.3...HEAD
[0.2.3]: https://github.com/eggstack/stegoeggo/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/eggstack/stegoeggo/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/eggstack/stegoeggo/compare/v0.1.0...v0.2.1
[0.1.0]: https://github.com/eggstack/stegoeggo/releases/tag/v0.1.0
