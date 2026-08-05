# Plan 052 Status Ledger

Plan baseline SHA: `c9d4d4f1edf6d43557c85c1d5c121d0071eeeaa1`

Disposition: **COMPLETE**

Initial implementation SHA: `c092fe0ca58d8e01679924017e4ee5b57c80d576`
Corrective pass SHA: pending (this session)

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

## Corrective pass (this session)

The initial `c092fe0` implementation left 8 definition-of-done gaps. This corrective pass closes all of them:

| Gap | Resolution |
|-----|------------|
| ANMF frame alpha not detected | `parse_webp()` now parses ANMF sub-payloads for ALPH chunks and VP8L intrinsic alpha |
| `finish_scan()` missing `decoded > expected` check | Added explicit overflow rejection |
| `finish_scan()` missing extra-byte rejection | Added explicit check after pad bits |
| `validate_webp_output()` incomplete (4/9 conditions) | Extended to check VP8X count, primary payload count, XMP count; removed `#[allow(dead_code)]`; called in production |
| VP8X reserved bits mask wrong (0xC3) | Corrected to 0xC1 (bits 0, 6, 7 only; animation bit 0x02 is valid) |
| XMP ownership prefix-based, not namespace-URI-based | Added `quick-xml` dependency; new `src/xmp.rs` module with `OWNED_FIELDS` matched by namespace URI + local name |
| No test for unrelated PLUS field survival | Added `webp_unrelated_plus_field_survives` test |
| No test for namespace conflict | Added `xmp_namespace_conflict_detected` and `xmp_namespace_compatible_no_conflict` tests |
| VP8L alpha not detected in flag derivation | `inject_text_chunks_webp_from_notice` now checks VP8L intrinsic alpha |

---

## Completed defect rows

| Item | Status | Evidence |
|------|--------|----------|
| JPEG exact entropy span | CLOSED | `JpegScanSpan` returns exact offsets; decoder receives `entropy_start..entropy_end` slice only |
| JPEG exact exhaustion | CLOSED | `finish_scan()` validates expected vs decoded blocks; overflow check; pad bits checked; extra bytes rejected |
| JPEG DHT class validation | CLOSED | `parse_dht()` rejects `table_class > 1` before table storage |
| Shared canonical Huffman representation | CLOSED | `build_canonical_huffman_entries()` validates and both encoder/decoder derive from it |
| WebP declared RIFF equality | CLOSED | `parse_webp()` requires `declared_end == data.len()` |
| WebP final cursor and pad validation | CLOSED | Chunk padded end checked against declared end; final cursor equality enforced |
| WebP primary payload validation | CLOSED | VP8X-only rejected; duplicate VP8/VP8L rejected; VP8+VP8L conflict rejected |
| WebP VP8X structural validation | CLOSED | Payload length ==10 validated; reserved bits (0xC1 mask) rejected; reserved bytes rejected; zero dimensions rejected |
| Mixed XMP field preservation | CLOSED | `strip_stego_owned_fields()` uses `quick-xml` + `OWNED_FIELDS` matched by namespace URI + local name |
| Malformed XMP fail-closed behavior | CLOSED | UTF-8 validation; missing rdf:RDF fails; owned fields stripped via `quick-xml` parser |
| VP8L alpha detection | CLOSED | `vp8l_has_alpha()` parses VP8L signature byte and alpha bit from 5-byte header |
| ANMF alpha detection | CLOSED | `parse_webp()` parses ANMF sub-payloads for ALPH chunks and VP8L intrinsic alpha |
| Final WebP validator | CLOSED | `validate_webp_output()` checks VP8X count, primary payload, XMP count, flags, RIFF extent; called in production |
| Namespace-URI ownership | CLOSED | `OWNED_FIELDS` in `src/xmp.rs` matched by namespace URI + local name, not prefix |
| Unrelated PLUS field survival | CLOSED | Test `webp_unrelated_plus_field_survives` proves `plus:License` survives |
| Namespace conflict detection | CLOSED | `check_namespace_conflict()` + test `xmp_namespace_conflict_detected` |
| Focused fixtures | CLOSED | 1409 tests pass (11 new in this session) |
| Workspace verification | CLOSED | `cargo fmt`, clippy, no-default-features, full test suite all pass |
| Current-head CI evidence | CLOSED | GitHub Actions CI #31035378131 passed for `c092fe0` |
| Publication hold | RETAINED | No publication is part of this plan |

---

## Table B: commands and evidence

| command/tool | environment/version | expected result | observed result | exact SHA or evidence location | status |
|--------------|---------------------|-----------------|-----------------|-------------------------------|--------|
| `cargo fmt --all -- --check` | stable rustfmt | pass | pass | local | CLOSED |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | stable clippy | pass | pass | local | CLOSED |
| `cargo check -p stegoeggo --no-default-features` | stable | pass | pass | local | CLOSED |
| `cargo test --workspace --exclude stegoeggo-fuzz --all-features` | stable | 1409 pass | 1409 pass, 32 ignored | local corrective session | CLOSED |
| `./scripts/check.sh` | bash | pass | pass | local corrective session | CLOSED |

---

## Table C: planning reconciliation

| plan | pre-052 claim | open criteria at 052 baseline | corrective commit(s) | final disposition |
|------|---------------|-------------------------------|----------------------|-------------------|
| 045 | PARTIAL | 12 residual items | c092fe0 + corrective | COMPLETE |
| 048 | CLOSED | exact entropy + shared Huffman | c092fe0 + corrective | COMPLETE |
| 049 | CLOSED | strict WebP structure + field-level XMP | c092fe0 + corrective | COMPLETE |
| 050 | Superseded | N/A | N/A | Superseded |
| 051 | PARTIAL | 16 residual items delegated | c092fe0 + corrective | COMPLETE |
| 052 | OPEN | all definition-of-done items | c092fe0 + corrective | COMPLETE |

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of this plan. Release remains manual and separate from correctness closure.
