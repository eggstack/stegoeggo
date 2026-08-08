# Plan 054 Status Ledger

Plan baseline SHA: `e683c87785d7e4ec60b17fa8ec961d983e4b6fac`

Disposition: **COMPLETE**

Plan 054 implementation head: `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15`

Post-closure audit baseline: `81c934d02dd43578482e01a15ea645a62ec0209b`

Final XMP closure implementation head: `96926b761275e70c83c6def2be0f667154799037`

Final XMP residuals are closed in `plans/056-status.md`.

Plan 054 owns the residual XMP semantic-preservation and animated-WebP correctness defects identified after the Plan 053 audit. Its animated-WebP product work remains closed. A later source audit found a narrower XMP reference/serialization gap outside the fixtures used for the original completion claim.

No release, version, tag, publication, or CI expansion is authorized.

---
## Workstream state

```text
XMP qualified RDF description serialization: CLOSED
XMP owned-subtree suppression for originally tested element/text cases: CLOSED
XMP expanded-name description close handling: CLOSED for ordinary preserved descriptions
XMP deterministic namespace serialization: CLOSED
XMP structural merge without substring parsing: CLOSED
XMP exact filtered-description deduplication: CLOSED
XMP semantic three-round idempotence for originally tested values: CLOSED
XMP safe scoped-prefix behavior: CLOSED
ANMF uint24/header decoding: CLOSED
ANMF frame flag validation: CLOSED
ANMF rectangle/canvas bounds: CLOSED
ANMF nested cursor/pad exactness: CLOSED
ANMF ALPH/VP8L order independence: CLOSED
Animated top-level ALPH rejection: CLOSED
Malformed top-level VP8L propagation: CLOSED
Malformed nested VP8L propagation: CLOSED
Animated alpha/feature derivation: CLOSED
Valid animated metadata rewrite: CLOSED
XML predefined/numeric reference handling: CLOSED by Plan 056
Merge attribute decode-before-escape: CLOSED by Plan 056
Owned-subtree event-complete suppression: CLOSED by Plan 056
Reference-aware three-round semantic proof: CLOSED by Plan 056
Focused verification: CLOSED at `96926b761275e70c83c6def2be0f667154799037`
Workspace verification: CLOSED at `96926b761275e70c83c6def2be0f667154799037`
Publication hold: RETAINED
```

---
## Defect ledger

| item | baseline behavior | exact closure contract | implementation SHA | focused evidence | disposition |
|---|---|---|---|---|---|
| RDF description qualification | preserved description serialized as bare `Description` | serialize an RDF-qualified description that reparses with RDF expanded name | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `preserved_description_remains_rdf_qualified`, `preserved_description_reparses_as_rdf_description` | CLOSED |
| Owned subtree filtering | nested events can escape an owned element skip | suppress the entire owned element subtree until matching end | `96926b761275e70c83c6def2be0f667154799037` | `owned_nested_rdf_description_does_not_close_outer_description`, owned comment/PI regression | CLOSED |
| Description close identity | local-name-only close handling | require RDF namespace + local `Description` | `96926b761275e70c83c6def2be0f667154799037` | `preserved_description_remains_rdf_qualified`, nested owned RDF regression | CLOSED |
| Namespace self-containment | inherited bindings are partially reconstructed | every serialized preserved description reparses standalone | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `preserved_description_outer_namespace_becomes_self_contained` | CLOSED |
| Structural XMP merge | `metadata_trap.rs` uses `find/rfind` on `rdf:RDF` | insert preserved descriptions through XML events | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `merge_preserved_descriptions` + conformance rewrite tests; reference serialization in Plan 056 | CLOSED |
| Description dedup | filtered descriptions are appended without exact dedup | deduplicate byte-identical filtered serialization, preserving first order | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `webp_three_round_rewrite_xmp_is_parser_idempotent` | CLOSED |
| Semantic idempotence | current test checks one chunk + substring presence | parser-based 3-round uniqueness/preservation proof | `96926b761275e70c83c6def2be0f667154799037` | `webp_three_round_rewrite_xmp_is_parser_idempotent`, `webp_xmp_reference_values_are_semantically_stable_three_rounds` | CLOSED |
| Scoped prefix reuse | global prefix conflict can over-reject | sibling self-contained scopes may reuse textual prefix for different URI | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `deduplicate_descriptions` scoped-safe behavior | CLOSED |
| ANMF header | first four bytes treated as reserved | decode X/Y/W/H/duration/flags from all 16 bytes | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `anmf_decodes_x_y_width_height_duration_and_flags` | CLOSED |
| ANMF flags | real reserved/blend/dispose layout not validated | require `(flags & 0xFC) == 0`, preserve bits 1/0 semantics | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `anmf_reserved_flag_bits_rejected` | CLOSED |
| Frame rectangle | not validated against canvas | checked `x + width <= canvas_width`, `y + height <= canvas_height` | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `anmf_frame_right_edge_exactly_on_canvas_accepted`, `anmf_frame_right_outside_canvas_rejected` | CLOSED |
| Nested exactness | basic nested bounds exist | exact header/payload/pad/final cursor and one image payload | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `anmf_nested_chunk_overrun_rejected`, `anmf_missing_pad_rejected`, `anmf_final_cursor_mismatch_rejected` | CLOSED |
| ALPH + VP8L ordering | rejection depends on encounter order | reject combination regardless of order | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `anmf_vp8l_then_alph_rejected`, `anmf_alph_with_vp8l_rejected` | CLOSED |
| Animated top-level ALPH | not explicitly forbidden | reject top-level ALPH whenever coherent animation path is used | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `animated_top_level_alph_rejected` | CLOSED |
| Top-level malformed VP8L | parse error can be ignored during feature derivation | malformed classified VP8L fails | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `malformed_top_level_vp8l_header_rejected` | CLOSED |
| Nested malformed VP8L | parse error can be ignored during frame feature derivation | malformed nested VP8L fails | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `malformed_nested_vp8l_header_rejected` | CLOSED |
| Animation feature derivation | partial frame alpha support | derive alpha/animation only from validated payload semantics | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `anmf_with_alph_detects_alpha`, `valid_alpha_vp8l_frame_accepted` | CLOSED |
| Animated rewrite | parser tests do not fully prove writer behavior | valid animation rewrites with unchanged ANMF payloads and correct flags | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | `webp_animated_opaque_vp8_rewrite_succeeds`, `webp_animated_multiple_frame_rewrite_byte_identical`, `webp_animated_with_xmp_rewrite_succeeds`, `webp_animated_unknown_top_level_chunk_preserved` | CLOSED |
| Predefined/numeric XML references | all `Event::GeneralRef` values rejected | accept valid predefined/numeric references while retaining bounded entity policy | `96926b761275e70c83c6def2be0f667154799037` | `filter_accepts_xml_references_in_unrelated_text`, `webp_xmp_reference_values_are_semantically_stable_three_rounds` | CLOSED |
| Merge attribute escaping | raw `Attribute::value` bytes are escaped again | decode/normalize semantic value before exactly one escaping pass | `96926b761275e70c83c6def2be0f667154799037` | `merge_attribute_references_are_decoded_before_escaping` | CLOSED |

---
## Verification ledger

The following evidence remains valid for the Plan 054 implementation scope, but it does not close Plan 056 reference fixtures.

| command | observed result | exact SHA | status |
|---|---|---|---|
| `cargo test -p stegoeggo xmp --all-features` | 81 passed, 5 ignored, 1310 filtered out | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | HISTORICAL PASS |
| `cargo test -p stegoeggo webp --all-features` | 129 passed, 10 ignored, 1257 filtered out | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | HISTORICAL PASS |
| `cargo test -p stegoeggo --test conformance_container_tests --all-features` | 34 passed | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | HISTORICAL PASS |
| `cargo fmt --all -- --check` | no diff | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | HISTORICAL PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | no issues found | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | HISTORICAL PASS |
| `cargo check -p stegoeggo --no-default-features` | clean | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | HISTORICAL PASS |
| `cargo test --workspace --exclude stegoeggo-fuzz --all-features` | 1503 passed, 32 ignored | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | HISTORICAL PASS |
| `./scripts/check.sh` | clean | `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15` | HISTORICAL PASS |

---
## Closure rule

Plan 054's animated-WebP and XMP work are COMPLETE. Plan 056 closed predefined/numeric reference handling, exact attribute serialization, event-complete owned-subtree suppression, and reference-aware multi-round public rewrite evidence.

A broad historical test pass cannot override an open source-level Plan 056 contract.
