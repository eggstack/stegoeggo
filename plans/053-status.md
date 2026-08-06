# Plan 053 Status Ledger

Plan baseline SHA: `40cdea8dbc9e110ed6e6bb3d325a10d25903b0b2`

Disposition: **OPEN**

Implementation head: not started

Planning head: pending final planning reconciliation commit

This ledger is created before any Plan 053 product-source edits.

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
XMP whole-packet strict parse: OPEN
XMP RDF/Description expanded-name recognition: OPEN
XMP namespace-scope preservation: OPEN
XMP alternate-prefix owned-field removal: OPEN
XMP mixed attribute preservation: OPEN
XMP mixed child-element preservation: OPEN
XMP malformed fail-closed behavior: OPEN
XMP multi-packet deterministic merge: OPEN
XMP three-round idempotence: OPEN
XMP internal API privacy: OPEN
VP8L header layout: OPEN
VP8L version validation: OPEN
VP8X one-pixel dimensions: OPEN
WebP structural multiplicity: OPEN
WebP coherent animation: OPEN
ANMF nested payload validation: OPEN
ANMF alpha propagation: OPEN
Non-circular feature derivation: OPEN
Strict final WebP validation: OPEN
JPEG canonical decoder derivation: OPEN
JPEG restart-marker classification: OPEN
JPEG SOS/span offsets: OPEN
JPEG fill-byte handling: OPEN
Focused tests: OPEN
Workspace verification: OPEN
Current-head CI evidence: OPEN
Publication hold: RETAINED
```

---

## Table A: defect closure

| item | audited source path | exact contract | implementation commit | focused fixture/test | observed result | disposition |
|---|---|---|---|---|---|---|
| Whole-packet XMP parse | `src/xmp.rs`, `src/protected/metadata_trap.rs` | One strict `quick-xml` event pipeline parses the complete packet | — | `rejects_unclosed_description`, `rejects_mismatched_end_tag` | not run | OPEN |
| RDF Description recognition | `src/xmp.rs` | Match RDF namespace URI plus local `Description` | — | standard and alternate RDF prefix tests | not run | OPEN |
| Outer namespace scope | `src/xmp.rs` | Namespace declarations inherited from outer elements are available and serialized self-contained | — | outer-xmpmeta and RDF namespace tests | not run | OPEN |
| Alternate-prefix owned fields | `src/xmp.rs` | Owned fields are removed by URI plus local name, never literal prefix | — | alternate PLUS and StegoEggo prefix tests | not run | OPEN |
| Same local wrong namespace | `src/xmp.rs` | Same local name under unrelated namespace survives | — | wrong-namespace test | not run | OPEN |
| Unprefixed same local | `src/xmp.rs` | Unprefixed attribute has no namespace and survives | — | unprefixed test | not run | OPEN |
| Mixed XMP attributes | `src/xmp.rs`, `metadata_trap.rs` | Remove owned attributes while preserving unrelated attributes and values | — | mixed-attribute unit and WebP tests | not run | OPEN |
| Mixed XMP child elements | `src/xmp.rs`, `metadata_trap.rs` | Remove exact owned subtree while preserving unrelated children and order | — | mixed-child unit and WebP tests | not run | OPEN |
| Malformed XMP fail closed | `src/xmp.rs`, `metadata_trap.rs` | Any malformed packet fails complete rewrite before output | — | malformed plus valid packet test | not run | OPEN |
| Multi-packet merge | `metadata_trap.rs` | Parse all packets, preserve order, emit one XMP chunk | — | compatible/differing packet tests | not run | OPEN |
| XMP idempotence | `metadata_trap.rs` | Three rounds leave one current owned claim and stable unrelated semantics | — | three-round test | not run | OPEN |
| XMP module privacy | `src/lib.rs` | No public `xmp` module or replacement public test helper | — | static search and compile | not run | OPEN |
| VP8L bit layout | `src/webp_container.rs` | Width bits 0..13, height 14..27, alpha 28 | — | VP8L dimension/alpha tests | not run | OPEN |
| VP8L version | `src/webp_container.rs` | Bits 29..31 must be zero | — | nonzero-version test | not run | OPEN |
| VP8X one-pixel canvas | `src/webp_container.rs` | Raw stored zero decodes to actual dimension one | — | 1x1, 1xN, Nx1 tests | not run | OPEN |
| Duplicate structural chunks | `src/webp_container.rs` | Duplicate ANIM, ICCP, and EXIF fail closed | — | duplicate tests | not run | OPEN |
| Animation coherence | `src/webp_container.rs` | Exactly one ANIM, at least one ANMF, no top-level primary | — | ANIM/ANMF coherence tests | not run | OPEN |
| ANMF boundaries | `src/webp_container.rs` | Header, nested chunks, pads, and final cursor are exact | — | short/overrun/missing-pad tests | not run | OPEN |
| ANMF image payload | `src/webp_container.rs` | Exactly one VP8 or VP8L; ALPH only with VP8 | — | frame payload conflict tests | not run | OPEN |
| ANMF alpha propagation | `webp_container.rs`, `metadata_trap.rs` | Parsed frame alpha reaches emitted VP8X alpha bit | — | alpha-animation rewrite test | not run | OPEN |
| Non-circular features | `src/webp_container.rs` | Derived features never initialize from declared feature bits | — | stale-bit normalization tests | not run | OPEN |
| Final WebP validation | `src/webp_container.rs` | Reparse output and compare independently derived features to declared flags | — | mismatch-rejection test | not run | OPEN |
| JPEG canonical decoder | `src/jpeg_transcoder/entropy.rs` | Decoder lookup derives directly from shared canonical entries | — | canonical lookup test | not run | OPEN |
| JPEG restart markers | `header.rs`, `mod.rs` | Any detected restart marker is unsupported | — | restart with/without DRI tests | not run | OPEN |
| JPEG exact span fields | `src/jpeg_transcoder/header.rs` | SOS and entropy offsets have exact documented meanings | — | offset tests | not run | OPEN |
| JPEG marker fill | `src/jpeg_transcoder/header.rs` | Fill bytes excluded; FF00 stuffing included; malformed runs rejected | — | fill/stuffing tests | not run | OPEN |
| Focused suites | listed source/tests | All required named fixtures pass | — | focused cargo test commands | not run | OPEN |
| Workspace verification | workspace | fmt, clippy, no-default, workspace tests, check.sh pass | — | command table below | not run | OPEN |
| Current-head CI | GitHub Actions | Exact final SHA is PASS, FAIL, or honestly UNAVAILABLE | — | exact status/run query | unavailable at baseline | OPEN |

---

## Table B: commands and evidence

| command/tool | environment/version | expected result | observed result | exact SHA or evidence location | status |
|---|---|---|---|---|---|
| `cargo test -p stegoeggo xmp --all-features` | stable Rust | pass | not run | — | OPEN |
| `cargo test -p stegoeggo webp --all-features` | stable Rust | pass | not run | — | OPEN |
| `cargo test -p stegoeggo jpeg --all-features` | stable Rust | pass | not run | — | OPEN |
| `cargo test -p stegoeggo --test conformance_container_tests --all-features` | stable Rust | pass | not run | — | OPEN |
| `cargo fmt --all -- --check` | stable rustfmt | pass | not run | — | OPEN |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | stable clippy | pass | not run | — | OPEN |
| `cargo check -p stegoeggo --no-default-features` | stable Rust | pass | not run | — | OPEN |
| `cargo test --workspace --exclude stegoeggo-fuzz --all-features` | stable Rust | pass | not run | — | OPEN |
| `./scripts/check.sh` | bash | pass | not run | — | OPEN |
| GitHub Actions status/run query | GitHub | exact final-head result | no final implementation head yet | — | OPEN |

---

## Table C: planning reconciliation

| plan | pre-053 claim | open criteria at Plan 053 baseline | corrective commit(s) | final disposition |
|---|---|---|---|---|
| 045 | PARTIAL | delegated container/XMP closure remains | — | PARTIAL |
| 048 | substantially closed | canonical decoder and restart/fill exactness | — | PARTIAL residual |
| 049 | substantially closed | XMP and animated-WebP semantics | — | PARTIAL residual |
| 050 | superseded | none | N/A | Superseded |
| 051 | PARTIAL | delegated residual closure remains | — | PARTIAL |
| 052 | incorrectly claimed COMPLETE | 16 audited residual defects | — | PARTIAL — delegated to 053 |
| 053 | OPEN | all rows in this ledger | — | OPEN |

---

## Closure rule

Plan 053, Plan 052, Plan 051, and Roadmap 045 may be marked complete only when every Plan 053 defect row is backed by a named test or exact command result and final-head CI evidence is exact or honestly recorded as unavailable.

A passing broad test suite does not override an open focused contract row.
