# Plan 074 Status: Stego Ergonomics Evidence and Documentation Closure

## Baseline

- Starting HEAD: `8fd0153` (`Harden generic stego public API with fallible config and documentation`)
- Working tree: clean on `main`, tracking `origin/main`
- Workspace versions: root `stegoeggo` 0.3.2, carrier `stegoeggo-stego` 0.3.2, CLI 0.3.2
- Root carrier dependency: `stegoeggo-stego = 0.3.2` with the `application-support` feature
- Carrier direct dependency list: `image = 0.25.6 (png, jpeg)`, `jpeg-encoder = 0.7`, `crc32fast = 1.4`, `thiserror = 1.0` — unchanged across Plans 070-073
- Roadmap 069 current status: PARTIAL — implementation complete across 070-073 but closure audit deferred to Plan 074

## Implementation commit chain

| Plan | Commit | Subject |
|---|---|---|
| 070 | `ffdab7a` | refactor: decompose application stego adapter |
| 071 | `bb5189e` | feat: add framed carrier convenience APIs |
| 072 | `9125e0f` | Optimize LSB embedding and extraction allocations |
| 073 | `8fd0153` | Harden generic stego public API with fallible config and documentation |

## Source inventory at audit baseline

### Root application stego adapter

`src/protected/steganography/` is the decomposed adapter:

| File | Lines | Responsibility |
|---|---|---|
| `mod.rs` | 1835 | Constants, shared internal types (`CandidateOutcome`, `V3PrefixResult`, `PayloadMalformedReason`, `ValidatedV3Header`, `V3ProbeResult`, `ExtractionTrace`), `SteganographyProtector` facade, `StegoPayload` accessor type, and unit tests |
| `marker.rs` | 221 | V3 marker construction (`generate_payload`, `generate_payload_for_plan`, `generate_payload_from_ctx`, `payload_bits_for_context`) |
| `embed.rs` | 455 | Carrier selection and embedding dispatch (`apply_dct_stego_bytes`, `apply_dct_stego_bytes_tiled`, `apply_dct_stego_bytes_from_plan`, `embed_lsb_v2`, `embed_lsb_v2_in_place`, `embed_lsb_tiled`, `apply_to_image_owned`) |
| `extract.rs` | 1494 | Seed discovery, bounded search, extraction orchestration (`extract_f5_tiled_candidates`, `evaluate_tiled_candidates`, `extract_lsb_v2`, `extract_lsb_tiled_candidates`, `extract_lsb`, `extract_with_redundancy`, `extract_payload`, `try_tiled_extraction_verify`) |
| `verify.rs` | 893 | Payload parsing, integrity, verification classification (`verify_payload`, `verify_payload_with_key`, `verify_payload_from_bytes_with_key`, `parse_stego_payload`, `parse_stego_payload_v3`, `compute_checksum`, `compute_payload_mac`, `verify_payload_integrity`, `classify_v3_prefix`, `validate_v3_header`) |
| `legacy.rs` | 97 | V1/V2 compatibility-only (`parse_stego_payload_v1`, `parse_stego_payload_v2`, `try_ecc_decode`) |

Total: 4,995 lines. The decomposition is real: `mod.rs` is a thin facade containing types, the `SteganographyProtector` constructors, the `Protector` trait routing, and unit tests; all embedding, extraction, and verification implementation is in the named submodules.

### Carrier crate (`stegoeggo-stego/src/`)

```
lib.rs                 Re-exports + carrier-level reports
constants.rs           STEGO_OFFSET_SEED_1, STEGO_SPREAD_FACTOR, SPLITMIX64_SEED, MIN/MAX_REDUNDANCY, validate_redundancy
error.rs               StegoError, JpegUnsupportedReason, StegoResult
frame.rs               Self-describing frame (magic, version, length, CRC32)
lsb.rs                 V2 LSB facade (raw, in-place, framed, capacity, embed_in_place)
jpeg.rs                Encoded-JPEG facade (raw/framed, capacity, probe_support, seed hint)
types.rs               EmbedOutcome, EmbedPath, EmbedStatus, EmbedOutcomeSummary, InPlaceEmbedReport
application_support.rs Narrow parent-crate operation layer (feature: application-support)
lsb_internal.rs        Private V2 LSB mechanics (permutations, embed/extract, tile mapping)
jpeg_transcoder/       Private JPEG DCT internals (header.rs, entropy.rs, stego_f5.rs, mod.rs)
```

`jpeg_transcoder/` and `lsb_internal` remain crate-private. `compile_fail` doctests in `stegoeggo-stego/src/lib.rs:27-33` guard `JpegTranscoder` and `stego_permutation_v2`.

## Status rows

| ID | Status | Evidence |
|---|---|---|
| R01 | CLOSED | Plan 070 R01-R14 independently verified: `src/protected/steganography/{mod,marker,embed,extract,verify,legacy}.rs` exist with the claimed responsibilities; 78 unit tests for `stego`, 123 for `payload`, 34 for `legacy`, 2 for `tiled_jpeg` |
| R02 | CLOSED | Plan 071 R01-R14 independently verified: `lsb::embed_framed`/`extract_framed` at `stegoeggo-stego/src/lsb.rs:73` and `:88`; `jpeg::embed_framed`/`extract_framed` at `stegoeggo-stego/src/jpeg.rs:543` and `:639`; JPEG framed extractor probes `(1..=config.redundancy()).rev()` at `jpeg.rs:655` |
| R03 | CLOSED | Plan 072 R01-R14 independently verified: `lsb::embed_in_place` at `stegoeggo-stego/src/lsb.rs:46`; `embed_lsb_v2_in_place` at `lsb_internal.rs:328`; cloning `embed` calls `embed_lsb_v2_in_place(&mut output, ...)` at `lsb_internal.rs:308`; known-answer vector at `tests/public_stego_api.rs:40` |
| R04 | CLOSED | Plan 073 R01-R14 independently verified: `LsbConfig::try_new`/`try_with_redundancy` at `lsb_internal.rs:662`/`:701`; `JpegConfig::try_new`/`try_with_redundancy` at `jpeg.rs:237`/`:276`; both call `crate::constants::validate_redundancy`; `validate_redundancy` at `stegoeggo-stego/src/constants.rs:11`; `compile_fail` doctests at `stegoeggo-stego/src/lib.rs:27-33` |
| R05 | CLOSED | Root stego application responsibilities are genuinely decomposed: `marker.rs` owns V3 marker construction, `embed.rs` owns embedding dispatch, `extract.rs` owns seed discovery and search, `verify.rs` owns payload parsing and classification, `legacy.rs` owns V1/V2 compatibility. `mod.rs` contains only shared types, the facade, the `StegoPayload` accessor struct, and unit tests |
| R06 | CLOSED | Carrier/private boundary audit passes: 0 matches for `use stegoeggo_stego::jpeg_transcoder` and `use stegoeggo_stego::lsb_internal` outside the carrier crate. Root imports only `application_support as carrier_support` (`mod.rs:17`) and the generic public facade (`src/lib.rs:223-231`) |
| R07 | CLOSED | Raw generic API compatibility passes: `lsb::embed`/`extract`, `jpeg::embed`/`extract`, `CapacityReport`, `EmbedReport`, `InPlaceEmbedReport`, `LsbConfig`, `JpegConfig`, `JpegSupport`, `probe_support` all unchanged at the original signatures. `tests/public_stego_api.rs::public_lsb_raw_roundtrip_arbitrary_bytes`, `public_jpeg_raw_roundtrip_arbitrary_bytes` |
| R08 | CLOSED | Framed LSB external-consumer round-trip passes without caller-known payload length: `tests/public_stego_api.rs::public_lsb_framed_roundtrip` at `:359` and `public_lsb_framed_capacity_includes_frame_overhead` at `:388` |
| R09 | CLOSED | Framed JPEG external-consumer auto-redundancy recovery passes: `tests/public_stego_api.rs::public_jpeg_framed_roundtrip` at `:416` and `public_jpeg_framed_extracts_after_capacity_downgrade` at `:433` (asserts `actual_redundancy < requested.redundancy()` and successful framed recovery) |
| R10 | CLOSED | Malformed framed input boundedness tests pass: `public_frame_checksum_detects_corruption`, `public_frame_malformed_length_fails_before_large_allocation`, `public_frame_trailing_bytes_rejected`, `public_lsb_framed_rejects_oversized_declared_length_before_full_extract`, `public_lsb_framed_rejects_declared_frame_beyond_carrier_capacity`, `public_frame_rejects_over_max_payload` |
| R11 | CLOSED | In-place LSB no-clone contract evidenced: `lsb::embed_in_place` (`stegoeggo-stego/src/lsb.rs:46`) returns `InPlaceEmbedReport` without an output image; `tests/public_stego_api.rs::public_lsb_in_place_matches_clone_and_preserves_alpha` at `:62` proves pixel-identical output; `public_lsb_in_place_capacity_failure_is_atomic` at `:94` proves atomicity on insufficient capacity |
| R12 | CLOSED | Corrected LSB known-answer output preserved: `tests/public_stego_api.rs::public_lsb_known_answer_vector` at `:40` locks the mixed-bit 5×3 vector byte-for-byte |
| R13 | CLOSED | Legacy V1/V2 and tiled compatibility pass: workspace tests pass, including legacy V1/V2 payload parsing in `src/protected/steganography/legacy.rs`, tiled LSB and F5 round-trips in `src/protected/steganography/embed.rs` and `extract.rs` |
| R14 | CLOSED | JPEG container/progressive/unsupported regressions pass: `tests/public_stego_api.rs::public_jpeg_supported_container_preservation` (`:252`), `public_jpeg_unsupported_progressive_is_explicit` (`:264`), plus the full JPEG structural probe suite in `stegoeggo-stego/src/jpeg_transcoder/` |
| R15 | CLOSED | Carrier docs/doctests/package structural check pass: 21 carrier doctests pass (`stegoeggo-stego/src/{lsb,jpeg,lib,lsb_internal}.rs`), `cargo package -p stegoeggo-stego --allow-dirty` packages successfully; carrier README distinguishes raw/framed/in-place with consistent terminology |
| R16 | CLOSED | Architecture docs updated to final state: see Phase 5 diff for `architecture/overview.md`, `architecture/protected-steganography.md`, `architecture/jpeg-stego-f5.md`, `architecture/jpeg-transcoder.md`, `architecture/util-image.md`, `architecture/jpeg-header.md`, `architecture/jpeg-entropy.md`, `architecture/constants.md`, `architecture/protected-metadata-trap.md`, and `README.md` |
| R17 | CLOSED | README and example terminology is accurate: README describes raw/in-place/framed API style names, links `examples/generic_stego.rs`, and notes fallible config (`LsbConfig::try_new`, `try_with_redundancy`, `JpegConfig::try_new`, `try_with_redundancy`) with `StegoError::InvalidConfig` semantics |
| R18 | CLOSED | `./scripts/check.sh` passes locally: 1642 tests passed, 25 ignored (feature-gated), 0 failed; fmt, strict clippy, no-default-features, all-feature workspace tests |
| R19 | CLOSED | Staged pre-release structural check passes: `cargo package -p stegoeggo-stego --allow-dirty`, `cargo package -p stegoeggo --list --allow-dirty`, `cargo package -p stegoeggo-cli --list --allow-dirty` all succeed without publishing |
| R20 | CLOSED | No version bump, no publish, no tag, no GitHub Release, no CI expansion occurred. Workspace versions stay at 0.3.2; `.github/workflows/ci.yml` and `.github/workflows/external-verification.yml` are unchanged; no new required CI jobs |
| R21 | CLOSED | Roadmap 069 final disposition is truthful: `Status: COMPLETE` with closure note referencing Plan 074 evidence (this ledger); Roadmap 057 history is not rewritten; no publication/release claims are made |

## Change log

- Phase 5: Reconciliation of architecture docs, README, and skills to the final
  decomposed + framed + in-place + fallible-config state. No behavior changes.
- No new public APIs, no new dependencies, no new tests added in this closure plan.
- Plan 075 later identified a chronology defect in the reconstructed Plan-073
  ledger and residual JPEG framed-extraction, numeric-validation, benchmark,
  and cadence-wording issues. Plan 074's broad architecture evidence remains
  historical and is not rewritten as failed work.

## Verification log

- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
- `cargo check -p stegoeggo --no-default-features` — PASS
- `cargo test -p stegoeggo-stego --all-features` — PASS (123 passed; 90 lib + 33 doc including 21 doctests)
- `cargo test -p stegoeggo --test public_stego_api --all-features` — PASS (43 passed)
- `cargo test -p stegoeggo --all-features stego` — PASS
- `cargo test -p stegoeggo --all-features tiled_jpeg` — PASS
- `cargo test -p stegoeggo --all-features legacy` — PASS
- `cargo test -p stegoeggo --all-features payload` — PASS
- `cargo test --workspace --exclude stegoeggo-fuzz --all-features` — PASS (1642 passed, 25 ignored, 38 suites)
- `./scripts/check.sh` — PASS
- `cargo package -p stegoeggo-stego --allow-dirty` — PASS (packages successfully, no publish)
- `cargo package -p stegoeggo --list --allow-dirty` — PASS
- `cargo package -p stegoeggo-cli --list --allow-dirty` — PASS
- `./scripts/release-check.sh --allow-dirty --skip-check --stage=pre` — PASS (version lockstep verified; structural package list verified; no publication)
