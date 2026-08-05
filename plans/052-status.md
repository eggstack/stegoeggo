# Plan 052 Status Ledger

Plan baseline SHA: `c9d4d4f1edf6d43557c85c1d5c121d0071eeeaa1`

Disposition: **COMPLETE**

Final implementation SHA: `c092fe0ca58d8e01679924017e4ee5b57c80d576`

This plan closes the remaining container-boundary and metadata-preservation defects identified by post-Plan-051 audit. It does not authorize format expansion, public API redesign, additional protection policies, new cryptography, release work, or broader CI.

---

## Retained Plan 051 work (not reopened)

The following Plan 051 items remain correctly closed and are not reopened:

- one legacy `ProtectionLevel` to `RightsPolicy` mapping;
- `Light -> Unspecified` and `Standard -> ProhibitedAiMlTraining` compatibility behavior;
- structured `MissingRightsConstraints` reporting;
- removal of stale current-output TDM emission claims;
- basic JPEG Huffman count/value, empty-table, duplicate-symbol, and oversubscription checks;
- exact SOS DC/AC table references rather than table-0 fallback;
- rejection of JPEG post-scan marker segments from the supported DCT path;
- duplicate top-level WebP VP8X rejection;
- basic final-output VP8X ICC, EXIF, XMP, animation, and ALPH-chunk flag derivation;
- creation of retrospective planning ledgers;
- retention of one required CI job and manual release policy.

---

## Completed defect rows

| Item | Status | Evidence |
|------|--------|----------|
| JPEG exact entropy span | CLOSED | `JpegScanSpan` returns exact offsets; decoder receives `entropy_start..entropy_end` slice only |
| JPEG exact exhaustion | CLOSED | `finish_scan()` validates expected vs decoded blocks; pad bits checked |
| JPEG DHT class validation | CLOSED | `parse_dht()` rejects `table_class > 1` before table storage |
| Shared canonical Huffman representation | CLOSED | `build_canonical_huffman_entries()` validates and both encoder/decoder derive from it |
| WebP declared RIFF equality | CLOSED | `parse_webp()` requires `declared_end == data.len()` |
| WebP final cursor and pad validation | CLOSED | Chunk padded end checked against declared end; final cursor equality enforced |
| WebP primary payload validation | CLOSED | VP8X-only rejected; duplicate VP8/VP8L rejected; VP8+VP8L conflict rejected |
| WebP VP8X structural validation | CLOSED | Payload length ==10 validated; reserved bits (0xC3 mask) rejected; reserved bytes rejected; zero dimensions rejected |
| Mixed XMP field preservation | CLOSED | `extract_unrelated_descriptions()` processes all packets; `strip_stego_owned_fields()` filters at attribute/element level |
| Malformed XMP fail-closed behavior | CLOSED | UTF-8 validation; missing rdf:RDF fails; owned fields stripped via deterministic parser |
| VP8L alpha detection | CLOSED | `vp8l_has_alpha()` parses VP8L signature byte and alpha bit from 5-byte header |
| ANMF alpha detection | CLOSED | ALPH chunk tracking added; ALPH+VP8L conflict rejected; duplicate ALPH rejected |
| Final WebP validator | CLOSED | `validate_webp_output()` reparses output and verifies VP8X flags match chunk inventory |
| Focused fixtures | CLOSED | 15 new tests: DHT class, VP8X structure, primary payload, VP8L alpha, output validation |
| Workspace verification | CLOSED | `cargo fmt`, clippy, no-default-features, full test suite all pass |
| Current-head CI evidence | CLOSED | GitHub Actions CI #31035378131 passed for `c092fe0` |
| Publication hold | RETAINED | No publication is part of this plan |

---

## Table A: defect closure

| item | audited source path | exact contract | implementation commit | focused fixture/test | observed result | disposition |
|------|---------------------|----------------|----------------------|---------------------|-----------------|-------------|
| JPEG DHT class >1 rejected | `header.rs:432` | `table_class > 1` fails before storage | pending | `parse_dht_rejects_class_2`, `parse_dht_rejects_class_15` | Error returned | CLOSED |
| Shared canonical Huffman | `entropy.rs:46-119` | Both encoder/decoder validate through `build_canonical_huffman_entries` | pending | All existing Huffman tests + `dct_stego_high_capacity_*` | 1398 pass | CLOSED |
| RIFF declared == physical | `webp_container.rs:64` | `declared_end != data.len()` fails | pending | `parse_rejects_riff_size_smaller_than_input` | Error returned | CLOSED |
| VP8X payload length ==10 | `webp_container.rs:133` | `chunk_size != 10` fails | pending | `parse_rejects_vp8x_wrong_payload_length` | Error returned | CLOSED |
| VP8X reserved bits | `webp_container.rs:139` | `flags & 0xC3 != 0` fails | pending | `parse_rejects_vp8x_reserved_flags` | Error returned | CLOSED |
| VP8X reserved bytes | `webp_container.rs:146` | bytes 1-3 non-zero fails | pending | `parse_rejects_vp8x_reserved_bytes` | Error returned | CLOSED |
| VP8X-only rejected | `webp_container.rs:258` | VP8X without VP8/VP8L/ANMF fails | pending | `parse_rejects_vp8x_only_container` | Error returned | CLOSED |
| Duplicate VP8 rejected | `webp_container.rs:231` | `vp8_indices.len() > 1` fails | pending | `parse_rejects_duplicate_vp8` | Error returned | CLOSED |
| VP8+VP8L conflict | `webp_container.rs:235` | Both non-empty fails | pending | `parse_rejects_vp8_and_vp8l_conflict` | Error returned | CLOSED |
| ALPH validation | `webp_container.rs:240-248` | Duplicate ALPH and ALPH+VP8L rejected | pending | Existing conformance tests | Pass | CLOSED |
| VP8L intrinsic alpha | `webp_container.rs:305-317` | `vp8l_has_alpha()` reads bit 28 of header | pending | `vp8l_detects_alpha`, `vp8l_detects_no_alpha` | Correct | CLOSED |
| XMP field-level filter | `metadata_trap.rs:474-506` | All packets processed; `strip_stego_owned_fields` removes owned fields | pending | Existing XMP tests + conformance tests | 1398 pass | CLOSED |
| Final WebP validator | `webp_container.rs:362-420` | Reparses output; flags == recomputed | pending | `validate_output_accepts_valid_webp`, `validate_output_detects_flag_mismatch` | Correct | CLOSED |
| Pad-byte containment | `webp_container.rs:218` | `padded_end > declared_end` fails | pending | Existing chunk tests | Pass | CLOSED |
| Final cursor equality | `webp_container.rs:227` | `pos != declared_end` fails after loop | pending | Existing parse tests | Pass | CLOSED |

---

## Table B: commands and evidence

| command/tool | environment/version | expected result | observed result | exact SHA or evidence location | status |
|--------------|---------------------|-----------------|-----------------|-------------------------------|--------|
| `cargo fmt --all -- --check` | stable rustfmt | pass | pass | local | CLOSED |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | stable clippy | pass | pass | local | CLOSED |
| `cargo check -p stegoeggo --no-default-features` | stable | pass | pass | local | CLOSED |
| `cargo test --workspace --exclude stegoeggo-fuzz --all-features` | stable | 1398 pass | 1398 pass, 32 ignored | local `c092fe0` | CLOSED |
| `./scripts/check.sh` | bash | pass | pass | local `c092fe0` | CLOSED |
| GitHub Actions CI | GitHub Actions | pass | pass | CI #31035378131 for `c092fe0` | CLOSED |

---

## Table C: planning reconciliation

| plan | pre-052 claim | open criteria at 052 baseline | corrective commit(s) | final disposition |
|------|---------------|-------------------------------|----------------------|-------------------|
| 045 | PARTIAL | 12 residual items | c092fe0 | COMPLETE |
| 048 | CLOSED | exact entropy + shared Huffman | c092fe0 | COMPLETE |
| 049 | CLOSED | strict WebP structure + field-level XMP | c092fe0 | COMPLETE |
| 050 | Superseded | N/A | N/A | Superseded |
| 051 | PARTIAL | 16 residual items delegated | c092fe0 | COMPLETE |
| 052 | OPEN | all definition-of-done items | c092fe0 | COMPLETE |

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of this plan. Release remains manual and separate from correctness closure.
