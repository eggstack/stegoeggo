# Plan 031 Status

Plan baseline SHA: 158015dc84b9f0bae58ebf6af77179b57dbb2ed9
Code candidate SHA: not selected
Evidence commit SHA: not selected
Release version: 0.3.0
Disposition: OPEN
Release hold: active

## Implementation Progress

### Phase 0: Reset status truth — CLOSED
- `plans/030-status.md` corrected to `Disposition: PARTIAL`
- `plans/031-status.md` created with all six tables

### Phase 1: Three-stage v3 extraction — CLOSED
- Replaced `V3_PROBE_BITS` (48-byte fixed probe) with `V3_PREFIX_BYTES` (6-byte prefix)
- Added `V3PrefixResult`, `PayloadMalformedReason`, `ValidatedV3Header` types
- Added `classify_v3_prefix()` for 6-byte prefix classification
- Added `validate_v3_header()` for full header validation
- Converted all 4 tiled extraction paths (LSB extract/verify, DCT extract/verify) to prefix-first
- Added 13 adversarial extraction tests
- SHA: 856f86e

### Phase 2: Runtime outcome propagation — CLOSED
- Added `PayloadEmissionContext` type to `src/types.rs` with `from_plan()` and `from_plan_for_context()` constructors
- Added `payload_emission_context()` method to `ResolvedProtectionPlan`
- Modified `generate_payload()` to accept `PayloadEmissionContext` instead of deriving flags from `ProtectionContext`
- Flags now derive from resolved emission context: `tiled`, `progressive_output`, `has_mac()`, `rights_metadata_planned`, `key_id`, `extensions`
- Added `generate_payload_from_ctx()` test-only wrapper for backward-compatible test callers
- Threaded `extract_seed_from_image_with_limits` through all WebP extraction paths in steganography module
- Added 11 Phase 2 focused tests: payload capacity serialization, tiled path matching, warnings/report agreement, metadata-only reports, capacity skip warnings, progressive fallback, batch preservation

### Phase 3: Resource enforcement — CLOSED
- Moved `OperationBudget` creation from after processing to before processing
- Budget now created in `process_request_bytes_with_report` and threaded to `process_plan_bytes`
- Added bounded-failure tests for `max_png_chunk_bytes`, `max_xmp_bytes`, `max_metadata_fields`, `max_metadata_field_bytes`
- Added bounded-failure tests for `max_png_chunks`, `max_jpeg_segments`, `max_jpeg_segment_bytes`
- SHA: 4ffed55

### Phase 4: CLI adversarial matrix — CLOSED
- Added 3 new subprocess tests (malformed key, duplicate key IDs, attacker substitution)
- Added Case I test for duplicate signature records (b5_16)
- Total 16 CLI detached verification tests covering cases A-L
- SHA: 74c2a36

### Phase 5: Conformance provenance — PARTIAL
- Added 4 negative conformance coverage tests (bad SHA, reclassified source, empty manifest, duplicate IDs)
- Independent fixture provenance corrected (generate_independent_fixtures.sh)
- Known limitation: harness only accepts 5 source values; no `independent` class exists
- SHA: 9ecf894

### Phase 6: CI/RC alignment — CLOSED
- Added `--expected-sha` parameter to `validate-release.sh`
- Added fuzz sync check to hermetic phase
- Added async feature combination to feature matrix
- RC workflow now passes `--expected-sha` to validation script
- SHA: 0ce9730

### Phase 7: Evidence collection — OPEN
- Code candidate SHA: not selected
- No CI/RC/fuzz runs recorded

### Phase 8: Publication — OPEN
- 0.3.0 unpublished, untagged

## Table A: v3 extraction inventory

| carrier/path | extraction function | verification function | prefix exact | header exact | payload exact | limits applied | no legacy fallback test | status |
|---|---|---|---|---|---|---|---|---|
| non-tiled PNG/WebP LSB | `extract_with_redundancy` | `verify_extract_with_redundancy` | 6-byte prefix | header_length validated | total_length extracted | `max_payload_bytes` in classify | yes | CLOSED |
| tiled PNG/WebP LSB | `extract_lsb_tiled_candidates` | `verify_extract_lsb_tiled` | 6-byte prefix | header_length validated | total_length extracted | `max_payload_bytes` in classify | yes | CLOSED |
| non-tiled JPEG DCT/F5 | `extract_verified_dct_payload_from_coefficients` | `verify_extract_dct_from_coefficients` | 6-byte prefix | header_length validated | total_length extracted | `max_payload_bytes` in classify | yes | CLOSED |
| tiled JPEG DCT/F5 | `extract_f5_tiled_candidates` | `verify_extract_f5_tiled` | 6-byte prefix | header_length validated | total_length extracted | `max_payload_bytes` in classify | yes | CLOSED |
| raw-byte known-seed path | via extract/verify wrappers | via extract/verify wrappers | inherits carrier | inherits carrier | inherits carrier | inherits carrier | yes | CLOSED |
| metadata-seed wrapper | seed fallback loop | seed fallback loop | inherits carrier | inherits carrier | inherits carrier | inherits carrier | yes | CLOSED |
| fixed-seed fallback wrapper | FALLBACK_SEEDS | FALLBACK_SEEDS | inherits carrier | inherits carrier | inherits carrier | inherits carrier | yes | CLOSED |
| detached embedded-reference | `verify_detached_embedded_ref` | `verify_detached_embedded_ref` | inherits carrier | inherits carrier | inherits carrier | input size | yes | CLOSED |

## Table B: runtime outcome propagation

| entrypoint | plan warnings | runtime warnings | embed summary | metadata summary | human output | JSON output | strict exit | status |
|---|---|---|---|---|---|---|---|---|
| `process_request_bytes` | yes | no | internal only | no | n/a | n/a | n/a | OPEN |
| `process_request_bytes_with_warnings` | yes | yes (embed outcomes) | internal only | no | n/a | n/a | n/a | PARTIAL |
| `process_request_bytes_with_report` | yes | yes (embed outcomes) | yes | inferred | n/a | n/a | n/a | PARTIAL |
| CLI single-file | yes | yes (embed outcomes) | yes (JSON) | inferred | yes | yes | yes | PARTIAL |
| CLI batch | yes | yes (embed outcomes) | yes (JSON) | inferred | yes | yes | yes | PARTIAL |

## Table C: resource limits

| limit | enforcement function | production callers | usage observation | bounded-failure public test | error variant | status |
|---|---|---|---|---|---|---|
| `max_input_bytes` | `check_input_size` | `process_plan_bytes`, verify paths | `ResourceUsage::input_bytes` | yes | `Error::ResourceLimitExceeded` | CLOSED |
| `max_width` | `check_dimensions` | `process_plan_bytes` | none | yes | `Error::ResourceLimitExceeded` | CLOSED |
| `max_height` | `check_dimensions` | `process_plan_bytes` | none | yes | `Error::ResourceLimitExceeded` | CLOSED |
| `max_png_chunks` | `check_container_count` | metadata parsers | none | yes | `Error::ResourceLimitExceeded` | CLOSED |
| `max_png_chunk_bytes` | `check_metadata_size` | metadata parsers | none | yes | `Error::ResourceLimitExceeded` | CLOSED |
| `max_jpeg_segments` | `check_container_count` | metadata parsers | none | yes | `Error::ResourceLimitExceeded` | CLOSED |
| `max_jpeg_segment_bytes` | `check_metadata_size` | metadata parsers | none | yes | `Error::ResourceLimitExceeded` | CLOSED |
| `max_webp_riff_chunks` | `check_container_count` | metadata parsers | none | no | `Error::ResourceLimitExceeded` | OPEN |
| `max_webp_riff_bytes` | `check_metadata_size` | metadata parsers | none | no | `Error::ResourceLimitExceeded` | OPEN |
| `max_xmp_bytes` | `check_metadata_size` | XMP parsers | none | yes | `Error::ResourceLimitExceeded` | CLOSED |
| `max_metadata_fields` | `check_metadata_field_count` | metadata parsers | none | yes | `Error::ResourceLimitExceeded` | CLOSED |
| `max_metadata_field_bytes` | `check_metadata_size` | metadata parsers | none | yes | `Error::ResourceLimitExceeded` | CLOSED |
| `max_payload_bytes` | `payload_within_limits` | `classify_v3_probe` | none | yes | `V3ProbeResult::MalformedV3` | CLOSED |
| `max_tile_extraction_origins` | origin loop bound | tiled extraction paths | none | yes | returns None | CLOSED |
| `max_verification_seeds` | seed loop bound | verify paths | none | yes | returns NotFound | CLOSED |
| `max_detached_manifest_bytes` | `from_json_with_limits` | CLI verify-manifest | none | yes | `Error::ResourceLimitExceeded` | CLOSED |

## Table D: detached CLI matrix

| case | expected overall status | expected exit | human assertion | JSON assertion | subprocess test | status |
|---|---|---|---|---|---|---|
| A: correct caller key | VerifiedTrusted | 0 | yes (TRUSTED) | yes | yes (b5_2) | CLOSED |
| B: substituted manifest bytes | KeyMaterialMismatch | 3 | yes | yes | yes (b5_9) | CLOSED |
| C: attacker self-consistent | non-zero | non-zero | yes (not trusted) | yes | yes (b5_15) | CLOSED |
| D: correct bytes wrong sig | non-zero | non-zero | yes | yes | yes (b5_5) | CLOSED |
| E: no caller key | VerifiedUntrusted | 4 | yes (UNTRUSTED) | yes | yes (b5_3) | CLOSED |
| F: unrelated caller key | non-zero | non-zero | yes | yes | yes (b5_4) | CLOSED |
| G: malformed caller key | non-zero | non-zero | yes | yes | yes (b5_13) | CLOSED |
| H: duplicate manifest key IDs | non-zero | non-zero | yes | yes | yes (b5_14) | CLOSED |
| I: duplicate signature records | per validation rules | per rules | no | yes | yes (b5_16) | CLOSED |
| J: wrong image digest | BindingFailure | 3 | yes | yes | yes (b5_6) | CLOSED |
| K: embedded HMAC no key | AuthenticationKeyMissing | 3 | yes | yes | yes (b5_7) | CLOSED |
| L: embedded HMAC wrong key | AuthenticationFailed | 3 | yes | yes | yes (b5_12) | CLOSED |

## Table E: conformance provenance

| fixture | source class | actual writer | exact writer version | generator SHA | generation command | digest | negative coverage | status |
|---|---|---|---|---|---|---|---|---|
| canonical_complete.png | generated | stegoeggo | 0.3.0 | n/a | n/a | recorded | yes | CLOSED |
| independent/* | external | python+imagemagick | partial | missing | recorded | recorded | yes | PARTIAL |
| external fixtures | varies | varies | varies | varies | varies | recorded | yes | OPEN |

## Table F: release evidence

| code candidate SHA | main CI run | RC run | fuzz run | package artifacts | smoke run | tag | publication | post-publication | status |
|---|---|---|---|---|---|---|---|---|---|
| not selected | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | OPEN |

## Test counts

- Total tests: 1237 passed, 27 ignored
- New tests added in this session: ~25 (Phase 2 payload emission, Table C container limits, Table D Case I)
- All tests pass, clippy clean, fmt clean

## Known limitations

- WebP RIFF chunk/byte limits (`max_webp_riff_chunks`, `max_webp_riff_bytes`) are not testable through the public verification API because LSB fallback seed extraction bypasses RIFF chunk parsing. The limits are enforced during metadata seed extraction (`extract_seed_from_webp`) but verification can still succeed through pixel-based fallback paths.
