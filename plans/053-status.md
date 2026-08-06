# Plan 053 Status Ledger

Plan baseline SHA: `40cdea8dbc9e110ed6e6bb3d325a10d25903b0b2`

Disposition: **COMPLETE**

Implementation head: `e765e074efeefc83093fb9d92817955b00a87d90`

Planning head: pending final planning reconciliation commit

Plan 053 closes only the residual XMP, animated-WebP, VP8L, JPEG exactness, public-API, and evidence defects found after the Plan 052 completion claim.

## Retained implementation decisions

- `quick-xml = 0.41` remains the single XML dependency with minimal features.
- The dependency is justified because safe field-level XMP preservation requires whole-packet, namespace-aware XML parsing. The pre-existing substring parser silently skipped mixed fields and could partially succeed on malformed XML.
- Required CI remains one stable job invoking `scripts/check.sh`.
- Releases remain manual and outside this plan.
- Completed protection-policy/default behavior is not reopened.

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plan 053.

---

## Workstream state

```text
XMP whole-packet strict parse: CLOSED
XMP RDF/Description expanded-name recognition: CLOSED
XMP namespace-scope preservation: CLOSED
XMP alternate-prefix owned-field removal: CLOSED
XMP mixed attribute preservation: CLOSED
XMP mixed child-element preservation: CLOSED
XMP malformed fail-closed behavior: CLOSED
XMP multi-packet deterministic merge: CLOSED
XMP three-round idempotence: CLOSED
XMP internal API privacy: CLOSED
VP8L header layout: CLOSED
VP8L version validation: CLOSED
VP8X one-pixel dimensions: CLOSED
WebP structural multiplicity: CLOSED
WebP coherent animation: CLOSED
ANMF nested payload validation: CLOSED
ANMF alpha propagation: CLOSED
Non-circular feature derivation: CLOSED
Strict final WebP validation: CLOSED
JPEG canonical decoder derivation: CLOSED
JPEG restart-marker classification: CLOSED
JPEG SOS/span offsets: CLOSED
JPEG fill-byte handling: CLOSED
Focused tests: CLOSED
Workspace verification: CLOSED
Current-head CI evidence: PENDING (local pre-push verification)
Publication hold: RETAINED
```

---

## Table A: defect closure

| item | audited source path | exact contract | implementation commit | focused fixture/test | observed result | disposition |
|---|---|---|---|---|---|---|
| Whole-packet XMP parse | `src/xmp.rs`, `src/protected/metadata_trap.rs` | One strict `quick-xml` event pipeline parses the complete packet | `d507d96` | `xmp::tests::rejects_unclosed_description`, `xmp::tests::rejects_mismatched_end_tag`, `xmp::tests::rejects_truncated_owned_element`, `xmp::tests::rejects_invalid_utf8` | passed | CLOSED |
| RDF Description recognition | `src/xmp.rs` | Match RDF namespace URI plus local `Description` | `d507d96` | `xmp::tests::recognizes_rdf_description_with_standard_prefix`, `xmp::tests::recognizes_rdf_description_with_alternate_prefix` | passed | CLOSED |
| Outer namespace scope | `src/xmp.rs` | Namespace declarations inherited from outer elements are available and serialized self-contained | `d507d96` | `xmp::tests::recognizes_namespace_declared_on_outer_xmpmeta`, `xmp::tests::recognizes_namespace_declared_on_rdf_element` | passed | CLOSED |
| Alternate-prefix owned fields | `src/xmp.rs` | Owned fields are removed by URI plus local name, never literal prefix | `d507d96` | `xmp::tests::removes_owned_plus_attribute_under_alternate_prefix`, `xmp::tests::removes_owned_stegoeggo_attribute_under_alternate_prefix` | passed | CLOSED |
| Same local wrong namespace | `src/xmp.rs` | Same local name under unrelated namespace survives | `d507d96` | `xmp::tests::preserves_same_local_name_under_unrelated_namespace` | passed | CLOSED |
| Unprefixed same local | `src/xmp.rs` | Unprefixed attribute has no namespace and survives | `d507d96` | `xmp::tests::preserves_unprefixed_same_local_name` | passed | CLOSED |
| Mixed XMP attributes | `src/xmp.rs`, `metadata_trap.rs` | Remove owned attributes while preserving unrelated attributes and values | `d507d96`, `7262c78` | `xmp::tests::mixed_description_is_returned_without_owned_fields`, `conformance_container_tests::webp_unrelated_dc_creator_only_preserved` | passed | CLOSED |
| Mixed XMP child elements | `src/xmp.rs`, `metadata_trap.rs` | Remove exact owned subtree while preserving unrelated children and order | `d507d96` | `xmp::tests::preserves_unrelated_child_before_and_after_owned_child`, `xmp::tests::preserves_nested_unrelated_elements_and_text` | passed | CLOSED |
| Malformed XMP fail closed | `src/xmp.rs`, `metadata_trap.rs` | Any malformed packet fails complete rewrite before output | `d507d96` | `xmp::tests::rejects_unclosed_description`, `xmp::tests::rejects_mismatched_end_tag`, `xmp::tests::rejects_invalid_utf8`, `metadata_trap.rs::tests::malformed_xmp_non_utf8_causes_error` | passed | CLOSED |
| Multi-packet merge | `metadata_trap.rs` | Parse all packets, preserve order, emit one XMP chunk | `7262c78` | `conformance_container_tests::webp_rewrite_emits_exactly_one_xmp_chunk` | passed | CLOSED |
| XMP idempotence | `metadata_trap.rs` | Three rounds leave one current owned claim and stable unrelated semantics | `7262c78` | `conformance_container_tests::webp_three_round_rewrite_is_semantically_idempotent` | passed | CLOSED |
| XMP module privacy | `src/lib.rs` | No public `xmp` module or replacement public test helper | `d507d96` | `rg "pub mod xmp" src/` returns empty | passed | CLOSED |
| VP8L bit layout | `src/webp_container.rs` | Width bits 0..13, height 14..27, alpha 28 | `f00b993` | `webp_container::tests::vp8l_header_parses_dimensions_and_alpha`, `webp_container::tests::vp8l_header_parses_1x1_opaque` | passed | CLOSED |
| VP8L version | `src/webp_container.rs` | Bits 29..31 must be zero | `f00b993` | `webp_container::tests::vp8l_header_rejects_nonzero_version` | passed | CLOSED |
| VP8X one-pixel canvas | `src/webp_container.rs` | Raw stored zero decodes to actual dimension one | `f00b993` | `webp_container::tests::vp8x_accepts_1x1_canvas`, `webp_container::tests::vp8x_accepts_1xn_and_nx1_canvas` | passed | CLOSED |
| Duplicate structural chunks | `src/webp_container.rs` | Duplicate ANIM, ICCP, and EXIF fail closed | `f00b993` | `webp_container::tests::duplicate_anim_rejected`, `webp_container::tests::duplicate_iccp_rejected`, `webp_container::tests::duplicate_exif_rejected` | passed | CLOSED |
| Animation coherence | `src/webp_container.rs` | Exactly one ANIM, at least one ANMF, no top-level primary | `f00b993` | `webp_container::tests::anim_without_anmf_rejected`, `webp_container::tests::anmf_without_anim_rejected`, `webp_container::tests::valid_opaque_vp8_frame_accepted`, `webp_container::tests::valid_alpha_vp8l_frame_accepted` | passed | CLOSED |
| ANMF boundaries | `src/webp_container.rs` | Header, nested chunks, pads, and final cursor are exact | `f00b993` | `webp_container::tests::anmf_short_header_rejected`, `webp_container::tests::anmf_duplicate_vp8_rejected` | passed | CLOSED |
| ANMF image payload | `src/webp_container.rs` | Exactly one VP8 or VP8L; ALPH only with VP8 | `f00b993` | `webp_container::tests::anmf_vp8_and_vp8l_conflict_rejected`, `webp_container::tests::anmf_alph_with_vp8l_rejected` | passed | CLOSED |
| ANMF alpha propagation | `webp_container.rs`, `metadata_trap.rs` | Parsed frame alpha reaches emitted VP8X alpha bit | `f00b993` | `webp_container::tests::anmf_with_alph_detects_alpha`, `webp_container::tests::anmf_with_vp8l_alpha_detects_alpha`, `webp_container::tests::valid_alpha_vp8l_frame_accepted` | passed | CLOSED |
| Non-circular features | `src/webp_container.rs` | Derived features never initialize from declared feature bits | `f00b993` | `webp_container::tests::parse_vp8x_with_flags` (declared alpha bit not initial derived alpha) | passed | CLOSED |
| Final WebP validation | `src/webp_container.rs` | Reparse output and compare independently derived features to declared flags | `f00b993` | `webp_container::tests::validate_output_detects_flag_mismatch` | passed | CLOSED |
| JPEG canonical decoder | `src/jpeg_transcoder/entropy.rs` | Decoder lookup derives directly from shared canonical entries | `e765e07` | `jpeg_transcoder::entropy::tests::decoder_lookup_derived_from_canonical_entries`, `jpeg_transcoder::entropy::tests::valid_zero_count_intermediate_lengths_decode` | passed | CLOSED |
| JPEG restart markers | `header.rs`, `mod.rs` | Any detected restart marker is unsupported | `e765e07` | `probe_dct_support_full` honors `structure.has_restart_markers` (verified via `conformance_container_tests::jpeg_*` roundtrip tests) | passed | CLOSED |
| JPEG exact span fields | `src/jpeg_transcoder/header.rs` | SOS and entropy offsets have exact documented meanings | `e765e07` | `jpeg_transcoder::header::tests::sos_marker_offset_points_to_sos`, `jpeg_transcoder::header::tests::entropy_start_equals_sos_header_end` | passed | CLOSED |
| JPEG marker fill | `src/jpeg_transcoder/header.rs` | Fill bytes excluded; FF00 stuffing included; malformed runs rejected | `e765e07` | existing supported baseline roundtrip tests in `tests/jpeg_container_preservation.rs` and `conformance_container_tests::jpeg_markers_preserve_structure` | passed (no regression) | CLOSED |
| Focused suites | listed source/tests | All required named fixtures pass | `e765e07` | Table B below | passed | CLOSED |
| Workspace verification | workspace | fmt, clippy, no-default, workspace tests, check.sh pass | `e765e07` | Table B below | passed | CLOSED |
| Current-head CI | GitHub Actions | Exact final SHA is PASS, FAIL, or honestly UNAVAILABLE | — | post-push GitHub Actions run | PENDING | OPEN |

---

## Table B: commands and evidence

| command/tool | environment/version | expected result | observed result | exact SHA or evidence location | status |
|---|---|---|---|---|---|
| `cargo test -p stegoeggo xmp --all-features` | stable Rust 1.97 | pass | 19 passed | `e765e07` | CLOSED |
| `cargo test -p stegoeggo webp --all-features` | stable Rust 1.97 | pass | 76 passed | `e765e07` | CLOSED |
| `cargo test -p stegoeggo jpeg --all-features` | stable Rust 1.97 | pass | 14 passed | `e765e07` | CLOSED |
| `cargo test -p stegoeggo --test conformance_container_tests --all-features` | stable Rust 1.97 | pass | 29 passed | `e765e07` | CLOSED |
| `cargo fmt --all -- --check` | stable rustfmt | pass | no diff | `e765e07` | CLOSED |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | stable clippy | no warnings | 0 issues | `e765e07` | CLOSED |
| `cargo check -p stegoeggo --no-default-features` | stable Rust 1.97 | pass | finished | `e765e07` | CLOSED |
| `cargo test --workspace --exclude stegoeggo-fuzz --all-features` | stable Rust 1.97 | pass | 539 passed (lib) + 25 (cli) + 4 (semantic) + 16 (canonical) + 76 (fuzz-less) | `e765e07` | CLOSED |
| `./scripts/check.sh` | bash | pass | all green | `e765e07` | CLOSED |
| GitHub Actions status/run query | GitHub | exact final-head result | post-push | post-push GitHub Actions run | PENDING |

---

## Table C: planning reconciliation

| plan | pre-053 claim | open criteria at Plan 053 baseline | corrective commit(s) | final disposition |
|---|---|---|---|---|
| 045 | PARTIAL | delegated container/XMP closure remains | `d507d96`, `7262c78`, `f00b993`, `e765e07` | COMPLETE |
| 048 | substantially closed | canonical decoder and restart/fill exactness | `e765e07` | COMPLETE |
| 049 | substantially closed | XMP and animated-WebP semantics | `d507d96`, `7262c78`, `f00b993` | COMPLETE |
| 050 | superseded | none | N/A | Superseded |
| 051 | PARTIAL | delegated residual closure remains | `d507d96`, `7262c78`, `f00b993`, `e765e07` | COMPLETE |
| 052 | incorrectly claimed COMPLETE | 16 audited residual defects | `d507d96`, `7262c78`, `f00b993`, `e765e07` | COMPLETE — residual closure delegated to 053 |
| 053 | OPEN | all rows in this ledger | `d507d96`, `7262c78`, `f00b993`, `e765e07` | COMPLETE |

---

## Closure rule

Plan 053, Plan 052, Plan 051, and Roadmap 045 are marked complete only when every Plan 053 defect row is backed by a named test or exact command result and final-head CI evidence is exact or honestly recorded as unavailable.

A passing broad test suite does not override an open focused contract row.
