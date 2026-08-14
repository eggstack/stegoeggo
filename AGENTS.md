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
scripts/measure_binary_size.sh              # binary size tracking (no CI gate)
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

## Features

| Feature | Description | Default |
|---------|-------------|---------|
| `async` | Tokio-based async API wrappers | No |
| `signatures` | Ed25519 signing via `ed25519-dalek` | No |
| `detached-manifest` | Detached signed manifest sidecar | No |
| `iscc` | ISCC content identifier computation (`compute_content_identifiers`, etc.) | No |
| `conformance` | Conformance harness binary and manifest parsing (TOML) | No |
| `parallel` | Rayon-based parallel batch processing (`process_images_parallel`, etc.) | No |

Feature-gated tests: `tests/async_integration.rs` requires `async`.
The conformance binary (`stegoeggo-conformance`) requires the `conformance` feature.

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
- **Two separate XorShiftRng implementations** — `PixelSelectionRng` in `src/util/image.rs` and `DctCoefficientRng` in `stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs`. Different algorithms, different sequences for same seed. Do NOT interchange them
- **`ProtectionContext::default()` uses CSPRNG seed** — For reproducible results, use `ProtectionContext::new(intensity, seed)` with an explicit seed
- **Pipeline flow order** — JPEG output: encode → DCT stego → metadata. Non-JPEG: pixel stego → encode → metadata. JPEG→JPEG fast path bypasses pixel decode entirely
- **`MetadataTrapProtector::apply()` returns `Cow::Borrowed(img)` unchanged** — Metadata injection is byte-level. The pipeline routes `Light` through `apply_light_bytes()` which encodes → injects → decodes
- **Generic stego API uses `StegoError`** — The public `stego` module uses its own `StegoError` type, not the crate root `Error`. Convert via `From<StegoError> for Error`
- **JPEG `extract` requires `actual_redundancy`** — The JPEG embed auto-downgrades redundancy when capacity is insufficient. Pass `report.actual_redundancy` to `extract` to match what was embedded
- **`#[serde(skip)]` on `config` field** — MAC keys and legal metadata are lost in serde roundtrips
- **CLI unified path** — The CLI always routes through `ProtectionRequest`. There is no dual legacy/request code path. `ProtectionLevel::default_policy()` computes level defaults: `Standard`→`ProhibitedAiMlTraining`, `Light`/`Disabled`→`Unspecified`. `--dmi auto` and omitted `--dmi` are equivalent. Mixed conflicting policy options (e.g., `--rights-policy` contradicting `--no-ai-training`) are configuration errors (exit code 2)
- **CLI exit codes** — `0`=ok, `1`=error, `2`=config, `3`=integrity, `5`=internal. `--verify` always exits 0; use output text to determine protection state, not exit code
- **CLI new-style flags** — `--rights-policy`, `--preset`, `--hidden-marker`, `--authentication` route through canonical `ProtectionRequest`. `--preset` cannot combine with `--level`/`--profile`. `--rights-policy` replaces `--dmi`
- **CLI subcommands (feature: `signatures`)** — `keygen`, `sign`, `verify-manifest` are feature-gated. `verify-manifest` accepts `--payload-key` for HMAC verification. `--json` enables machine-readable output on verify and verify-manifest
- **No `test-seeds` in production CLI** — `test-seeds` is test infrastructure only, never in production binary
- **F5 seed Q-table edge case** — `embed_seed_in_quantization_tables()` fails if any quantization value in the first 2 tables is < 2
- **`--tdm-reserved` is deprecated** — TDMRep deployment deferred; sets DMI to `ProhibitedSeeConstraints`
- **CLI binary location** — `stegoeggo-cli/src/main.rs`, not `src/bin/`
- **`--require-complete` is removed** — `--strict` is the single complete-validation mode
- **CLI `--dry-run` is not new-style** — `--dry_run` is excluded from `has_new_style_flags()` so it does not force canonical path; dry-run uses the same `build_protection_request()` as execution

**JPEG DCT and container correctness:**

- **JPEG DCT subset** — DCT embedding supports only: 8-bit precision, sequential Huffman DCT, single scan, 1-4 components with supported sampling factors, no restart intervals, valid terminal EOI. Unsupported inputs (progressive, restart-bearing, multi-scan, CMYK) receive Q-table seed only (stego signal without payload) via `probe_dct_support_full()` gating, with full metadata injection.
- **`encode_coefficients` signature** — Takes `original_jpeg: Option<&[u8]>`. When `Some`, uses `encode_coefficients_preserving` which walks the original byte stream replacing only DQT and SOS scan data. When `None`, uses `assemble_jpeg` which rebuilds from parsed fields (drops unknown segments).
- **DCT success path uses preserving encoding** — All successful DCT embedding attempts encode via `encode_coefficients(header, coefficients, Some(original_jpeg))`. The `assemble_jpeg` path is never reachable from the normal original-JPEG DCT success path. This ensures APP2, APP13, APP14, COM, unknown APP, and other unrelated segments survive byte-for-byte.
- **`probe_dct_support_full` checks scan structure** — Beyond header properties, walks the complete JPEG byte stream to count scans (must be exactly 1), verify EOI presence, and reject trailing post-scan segments. Multi-scan sequential JPEGs are rejected as `Unsupported(MultipleScans)`. Trailing segments after scan are rejected as `Unsupported(TrailingSegmentsAfterScan)`.
- **Checked JPEG structure analysis** — `JpegHeader::analyze_structure_checked()` is the supported-path analyzer. It returns errors for truncated marker runs, missing/short/out-of-range segment lengths, malformed SOS extents, and unterminated entropy. Exact entropy spans retain `FF 00` stuffing but exclude the first `FF` of repeated marker-fill runs; `FF FF 00` is malformed. `analyze_structure()` is compatibility-only and must not be used for support or decode decisions.
- **`parse_sos` rejects malformed table IDs** — SOS table IDs > 3 return `InvalidFormat` error instead of clamping. Prevents OOB panics in the entropy decoder.
- **Truncation is a hard error** — `read_magnitude` returns `Err` on truncated data instead of producing partial blocks.
- **Malformed entropy fails closed** — Missing DC/AC symbols, AC run overflow, invalid zero-size symbols, and truncated magnitude data all return `HuffmanDecode` errors. Malformed entropy never produces partial successful coefficient maps.
- **Canonical Huffman construction advances unconditionally** — Both decoder and encoder advance the code through every bit-length slot including zero-count lengths. Empty slots get sentinel values. This ensures correct code assignments for tables with intermediate empty lengths.
- **Huffman table validation** — `validate_huffman_table()` checks count/value sum consistency, non-empty tables, unique symbols, and code-space oversubscription. Tables are rejected deterministically without panic. Only DHT class 0 (DC) and class 1 (AC) are accepted; classes 2-15 are rejected as malformed. Exact SOS DC/AC table references are required; table-0 fallback is removed from both decoder and encoder.
- **Shared canonical Huffman representation** — `build_canonical_huffman_entries()` is the single checked builder for both encoder and decoder Huffman state. It validates counts/values, uniqueness, and code-space before constructing entries.
- **Decoder derived from canonical entries** — `HuffmanDecoder::from_table` populates a per-length sorted vector of `(code, symbol)` pairs directly from the canonical entries. No second canonical-code algorithm remains. Empty intermediate bit-length buckets are allowed.
- **Restart-bearing scans unsupported** — `probe_dct_support_full` rejects any scan with detected restart markers, regardless of DRI marker presence. The supported path does not consume RST markers.
- **Scan-span offsets** — `JpegScanSpan.sos_marker_offset` points to the leading `0xff` of the SOS marker, `sos_header_end` is the first byte after the complete SOS segment, and `entropy_start` equals `sos_header_end`.
- **Metadata-only JPEG is byte-safe** — `inject_text_chunks_jpeg` walks the raw byte stream, preserving all pre-SOS segments verbatim. No coefficient decode/encode occurs.

**WebP container correctness:**

- **VP8X dimensions are 3-byte LE** — The `image-webp` decoder reads canvas width/height as 3-byte little-endian values. The 4-byte layout in the WebP spec diagram is misleading; use `encode_vp8x_chunk` which writes correct 3-byte encoding.
- **VP8X flags bit positions** — ICC=0x20, Alpha=0x10, EXIF=0x08, XMP=0x04, Animation=0x02. The reserved mask is 0xC1 (bits 0, 6, 7). The EXIF bit (0x08) must NOT be set when no EXIF chunk is present, or `image-webp` returns `ChunkMissing`.
- **VP8X flags derived from final output** — Flags are computed from the actual emitted chunk inventory, not from stale input booleans. Existing EXIF causes the EXIF bit to remain set; removed metadata clears the corresponding bit.
- **VP8X structural validation** — VP8X payload length must be exactly 10 bytes. Reserved flag bits (0xC3 mask) and reserved bytes 1-3 must be zero. Zero dimensions are rejected.
- **VP8X-only container rejected** — VP8X without a VP8, VP8L, or ANMF payload is rejected as invalid.
- **Primary payload rules** — Duplicate VP8, duplicate VP8L, and VP8+VP8L conflicts are rejected. ALPH paired with VP8L is rejected (VP8L has intrinsic alpha). Duplicate ALPH chunks are rejected. Duplicate ANIM, ICCP, and EXIF chunks are rejected.
- **Animation coherence** — Exactly one ANIM (six-byte payload), at least one ANMF, no top-level VP8/VP8L when ANIM/ANMF present, no top-level ALPH when ANIM/ANMF present, animated WebP requires VP8X. Frame ANMF requires 16-byte header with reserved bits zero, bounded nested chunks with final cursor matching the frame end, exactly one VP8 or VP8L image payload, and ALPH only paired with VP8.
- **ANMF frame header parsed** — `parse_anmf_frame` decodes bytes 0..2 as stored X (×2), bytes 3..5 as stored Y (×2), bytes 6..8 as frame width minus one, bytes 9..11 as frame height minus one, bytes 12..14 as duration ms, and byte 15 with reserved bits 0xFC rejected. Frame rectangle must satisfy `x + width ≤ canvas_width` and `y + height ≤ canvas_height` with checked arithmetic.
- **ANMF nested chunk order independence** — Nested ALPH+VP8L is rejected regardless of appearance order. The safest impl inventories chunk kinds first, then validates combinations after the loop.
- **Derived features** — `WebPFeatures` is computed from actual chunks and payload headers (VP8L intrinsic alpha, ANMF alpha, ALPH presence, ICCP/EXIF/XMP/ANIM chunk presence). The writer emits `WebPFeatures::as_vp8x_flags()` directly. `validate_webp_output` requires exact equality between declared and independently derived flags.
- **Simple → extended conversion** — Metadata insertion into VP8/VP8L creates VP8X with correct dimensions and flags. VP8X is emitted first, image payload preserved byte-identical, metadata appended after.
- **RIFF extent equality** — Rewrite parsing requires `declared_end == data.len()`. Both oversized and undersized RIFF declarations are rejected. Chunk padded ends are validated against declared extent.
- **Final cursor and pad-byte containment** — After chunk iteration, cursor must equal declared extent. Odd-sized chunks require a contained physical pad byte.
- **VP8L intrinsic alpha** — `parse_vp8l_header()` parses the VP8L signature byte, extracts width from bits 0..13, height from bits 14..27, alpha from bit 28, and rejects non-zero version in bits 29..31. VP8L dimensions and alpha are stored as width/height/alpha where raw stored zero decodes to actual dimension one.
- **ALPH chunk tracking** — `ParsedWebP.alph_indices` tracks ALPH chunks. ALPH without VP8 is structurally invalid.
- **Final output validator** — `validate_webp_output()` reparses output bytes, recomputes expected VP8X flags from chunk inventory, and verifies exact match.
- **EXIF seed emission retired** — No new EXIF seed chunks are emitted. Seed is stored in XMP via `stegoeggo:ProtectionSeed`. Historical EXIF seed data is still parsed for backward compatibility.
- **One effective XMP chunk** — Output loop skips original XMP chunks; `merge_or_replace_webp_xmp` prepares the replacement. Existing non-StegoEggo XMP properties are preserved under `ReplaceStegoOwned`. Identical duplicate XMP chunks collapse to one.
- **Strict XMP filter** — `crate::xmp::filter_xmp_packet` is the single whole-packet `quick-xml::NsReader` pipeline. It identifies `rdf:Description` by namespace URI plus local name, captures inherited prefix declarations, removes owned attributes and element subtrees by URI plus local name, accepts only XML 1.0 predefined/numeric references, rejects unresolved custom entities/DOCTYPEs and malformed packets, and fails the entire rewrite before output.
- **RDF-qualified preserved descriptions** — Preserved `rdf:Description` is serialized as `<rdf:Description ...>` (not bare `<Description>`), so namespace-aware reparsers recognize it as RDF. The element is closed with `</rdf:Description>` matched on namespace URI + local name. Self-contained preserved descriptions include inherited `xmlns:rdf` plus any other namespace declarations in scope at the moment of capture.
- **Owned subtree skip depth** — While `owned_depth > 0`, no nested element, text, reference, comment, or processing instruction is serialized as unrelated. Depth is incremented on `Start` inside owned scope, on `Empty` inside owned scope (no serialization), and decremented on matching `End` before any RDF-description close check. Empty elements remain depth-neutral.
- **Structural XMP merge** — `merge_preserved_descriptions` is the single merge site. It parses the canonical packet structurally via `quick-xml`, decodes normalized attribute semantics before one escaping pass, resolves only bounded XML 1.0 references, identifies the RDF container by expanded name, streams every event to the output, and inserts the deduped preserved descriptions immediately before the matching `</rdf:RDF>` End event. No substring parsing remains in the rewrite path.
- **Byte-identical description dedup** — `deduplicate_descriptions` keeps only the first occurrence of byte-identical preserved XML and excludes any preserved that is byte-identical to a canonical description. Safe sibling-local namespace prefix reuse is not treated as a global conflict.
- **Animation metadata rewrite** — `inject_text_chunks_webp_from_notice` copies existing ANIM and ANMF chunk bytes unchanged. It only adjusts VP8X flags, replaces the XMP chunk set, and recomputes RIFF size and padding. No WebP pixel or frame encoding is performed.
- **`xmp` module is `pub(crate)`** — The XMP parser and helpers are internal; integration tests exercise XMP behavior through the public injection API.

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
- **Spread spectrum LSB** — Each payload bit embedded across `STEGO_SPREAD_FACTOR * redundancy` (=5×r) RGB carrier slots via majority voting. The corrected V2 carrier uses a single bijective permutation over `width * height * 3` slots; all replicas of one bit use consecutive logical indices through the same permutation, guaranteeing no inter-replica collisions. Legacy extraction (pixel-index/channel-derived carrier) is preserved for backward compatibility
- **Generic carrier core** — `stegoeggo-stego/src/lsb.rs` (public API surface only: `embed`, `extract`, `capacity`, `LsbConfig`) and `stegoeggo-stego/src/lsb_internal.rs` (low-level carrier mechanics: permutations, embed/extract helpers, capacity, crop/blit) and `stegoeggo-stego/src/jpeg.rs` contain application-neutral carrier mechanics. `SteganographyProtector` delegates to these modules and adds StegoEggo-specific payload generation, parsing, and verification. The `jpeg_transcoder` module and `lsb_internal` module are `pub(crate)`; downstream consumers access internals only through `#[doc(hidden)] __internal_*_facade` re-exports. The carrier `EmbedReport` is generic over the output type (`RgbaImage` for LSB, `Vec<u8>` for JPEG) — pixel-domain no longer performs an implicit PNG encode
- **`stego_redundancy` is `Option<usize>`** — Default `None` derives from intensity via `effective_redundancy()` (<0.3→1, 0.3-0.7→2, >=0.7→3). Valid range 1-10
- **F5 redundancy cap** — Max redundancy is 10. Extraction tries all 10 values
- **Payload version 3 is current** — V1/V2 still supported for extraction only, never written. V3 adds TLV extensions with domain-separated authentication. V3 extraction paths (magic bytes `[0x53, 0x45]`) tried first before V2/V1 fallback. V3 core: 32 bytes. V3 CRC: 36 bytes. V3 HMAC: 48 bytes
- **Tiled steganography** (`with_tile_size(n)`) — Crop-resistant mode, full payload per tile. `tile_seed(master_seed, tile_x, tile_y)` uses splitmix64. Tiled LSB embeds directly into image regions using V2 carrier with sub-image slot coordinates, avoiding per-tile RgbaImage allocations
- **F5 tiled block set** — MCU-interleaved: `block_idx = (mcu_y * mcus_per_row + mcu_x) * h * v + sub_y * h + sub_x`. Do NOT assume row-major ordering
- **JPEG DCT one-pass embed** — Supported DCT embedding computes max feasible redundancy from capacity, then embeds+encodes once. No retry loop, no roundtrip decode/extract self-test. Capacity-selected redundancy = min(requested, available / payload_bits)
- **Tiled LSB direct region** — Tiled embed computes V2 carrier slots using sub-image dimensions and maps coordinates to full image, eliminating crop/blit allocations. Extraction crops sub-images and uses V2 carrier which matches the sub-image slot mapping

**Canonical metadata format:**

- XMP writer emits `plus:DataMining` with full PLUS LDF URIs (e.g., `http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING`). Legacy bare keys (`DMI-PROHIBITED-AIMLTRAINING`) and `Iptc4xmpExt:DMI-*` properties are parsed for backward compatibility but classified as `LegacyBarePlusVocabularyKey`, not `CanonicalPlusDataMining`. `Unspecified` emits no `plus:DataMining` property (the writer conditionally includes it only when `plus_vocab_uri()` returns `Some`)
- `ProhibitedSeeConstraints` emits `plus:OtherConstraints` alongside `plus:DataMining`
- Private `noai`/`noindex` and `DMI-PROHIBITED` tEXt/COM markers are no longer emitted in new output
- `tdm:reserve_tdm` is no longer emitted (TDMRep is a web-distribution mechanism, not an image-metadata signal)
- WebP legal fields live in XMP inside `<rdf:Description>`. `dc:rights` and `xmpRights:UsageTerms` use `<rdf:Alt><rdf:li>` containers. `dc:creator` uses `<rdf:Seq>`
- WebP exiftool: `exiftool -Copyright` does not resolve `dc:rights` — use `exiftool -XMP-dc:Rights`

**Warning system:**

- `ProtectionWarning` has 8 variants: `MissingMacKey`, `MetadataInjectionDisabled`, `ProgressiveJpegFallback`, `JpegReencodeFragile`, `LsbCapacitySkipped`, `DctCapacityInsufficient`, `ContradictoryLegalClaims`, `MissingRightsConstraints`
- `MissingRightsConstraints` is profile-dependent: only emitted for `ProhibitedSeeConstraints` without `ai_constraints` or `web_statement_of_rights`
- `severity_for_profile(profile)` classifies warnings as `Info`, `Warning`, or `Error`

## Architecture

- **Strategy pattern** via `Protector` trait (`src/traits.rs`) with three levels: Disabled, Light, Standard — see `architecture/traits.md`
- **Pipeline** (`src/lib.rs`): `ProtectionPipeline` orchestrates protectors for legacy APIs — see `architecture/pipeline.md`
- **Direct plan executor** (`src/lib.rs`): `execute_metadata_only()`, `execute_stego_and_metadata()`, `execute_stego_and_metadata_tiled()` — canonical execution from `ResolvedProtectionPlan` without `ProtectionContext` reconstruction for metadata
- **Generic carrier core** (`stegoeggo-stego/src/`): Application-neutral LSB and JPEG DCT carrier mechanics — see `architecture/protected-steganography.md`
- **Public generic stego API** (`stegoeggo::stego`): Carrier-level embedding/extraction for arbitrary payload bytes, independent of the rights-protection pipeline. Includes `stego::lsb` (pixel-domain), `stego::jpeg` (DCT-domain), and `stego::frame` (self-describing frame with CRC32). See `architecture/protected-steganography.md`
- **JPEG fast path**: When input/output are both JPEG, operates directly on DCT coefficients via `stegoeggo-stego/src/jpeg_transcoder/`, bypassing pixel decode/encode — see `architecture/jpeg-transcoder.md`
- **Policy-first API**: `ProtectionRequest` + `RightsPolicy` are the canonical API — see `architecture/types.md`
- **`#![forbid(unsafe_code)]`** throughout the library crate and `stegoeggo-stego`

## Validation Scripts

- `scripts/check.sh` — Fast deterministic checks used by local development and required CI (fmt, clippy, no-default-features, tests)
- `scripts/release-check.sh` — Bounded local pre-release readiness (runs check.sh + package dry-runs + version lockstep verification). Never publishes
- `scripts/validate-docs-rs.sh` — Docs.rs-equivalent rustdoc validation (nightly, DOCS_RS=1, cfg(docsrs), workspace + packaged crate)
- `scripts/validate-msrv-package.sh` — Fresh MSRV consumer resolution (packages crate, creates clean consumers, tests minimal and all-feature combos on declared MSRV)
- `scripts/verify_metadata_conformance.sh` — Shell wrapper for conformance checks (delegates to Rust conformance harness)
- `scripts/check_fuzz_sync.sh` — Verifies fuzz harness parity between fuzz/Cargo.toml and fuzz.yml workflow

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

## Other Reference Files

- `DEPRECATIONS.md` — Deprecated API inventory
- `SUPPORT.md` — Support matrix
- `STABILITY.md` — Stability tiers
- `architecture/` — Architecture documentation (30 files, verified against source)
  - `architecture/overview.md` — Module map, protection levels, pipeline flow
  - `architecture/pipeline.md` — Pipeline orchestration and public API (legacy + canonical paths)
  - `architecture/traits.md` — Protector trait definition
  - `architecture/types.md` — Core type definitions (ProtectionRequest, RightsPolicy, etc.)
  - `architecture/constants.md` — Tuning constants and their values (carrier + application copies)
  - `architecture/error.md` — Error variants (17 total: 16 always-available + 1 async-only)
  - `architecture/verification.md` — VerificationReport and sub-verification types
  - `architecture/cli.md` — CLI flags, subcommands, and exit codes
  - `architecture/jpeg-transcoder.md` — JPEG DCT coefficient processing
  - `architecture/jpeg-header.md` — JPEG header parsing and scan analysis
  - `architecture/jpeg-entropy.md` — Huffman codec (decoder/encoder)
  - `architecture/jpeg-stego-f5.md` — F5-style DCT steganography
  - `architecture/protected-steganography.md` — LSB and DCT embedding methods, including generic carrier API and crate layout
  - `architecture/protected-metadata-trap.md` — Metadata injection (RightsMetadataProtector)
  - `architecture/protected-passthrough.md` — Disabled-level no-op
  - `architecture/payload-v3.md` — V3 payload wire format and TLV extensions
  - `architecture/provenance.md` — Provenance claims and digest binding
  - `architecture/provenance-claim.md` — Provenance claim specification
  - `architecture/signing.md` — Ed25519 signing module
  - `architecture/detached.md` — Detached manifest sidecar
  - `architecture/detached-manifest.md` — Detached manifest JSON schema
  - `architecture/conformance.md` — Conformance harness and reporting
  - `architecture/resource-limits.md` — Input validation and resource bounds
  - `architecture/resolve.md` — ProtectionRequest → ResolvedProtectionPlan resolver
  - `architecture/legal-metadata-field-mapping.md` — Field mapping across PNG/JPEG/WebP
  - `architecture/util-image.md` — Image utilities (encoding, format detection)
  - `architecture/util-iscc.md` — ISCC content identifier computation
  - `architecture/util-seed.md` — CSPRNG seed generation
  - `architecture/async-api.md` — Tokio-based async API
  - `architecture/adr-c2pa.md` — C2PA integration deferred ADR
