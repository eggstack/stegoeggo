# Plan 052 Status Ledger

Plan baseline SHA: `c9d4d4f1edf6d43557c85c1d5c121d0071eeeaa1`

Current disposition: **PARTIAL — residual closure delegated to Plan 053**

Initial implementation SHA: `c092fe0ca58d8e01679924017e4ee5b57c80d576`

Follow-up corrective SHA: `40cdea8dbc9e110ed6e6bb3d325a10d25903b0b2`

Final residual plan:

- `plans/053-xmp-animated-webp-and-jpeg-exactness-closure.md`
- `plans/053-status.md`

The earlier `COMPLETE` disposition was not supported by the final source audit. This ledger now separates work that landed correctly from the remaining source contracts.

No version bump, publication, tag, GitHub release, or release automation is authorized by this status correction.

---

## Correctly retained Plan 052 work

The following items are materially implemented and are not reopened without a focused failing fixture:

- exact JPEG entropy-only slice for the normal supported baseline path;
- actual decoded JPEG block counting;
- rejection of extra complete entropy bytes and invalid final pad bits;
- DHT class rejection outside DC `0` and AC `1`;
- exact referenced SOS Huffman tables;
- RIFF declared extent equality with physical input;
- top-level RIFF padded-end and final-cursor checks;
- duplicate top-level VP8X rejection;
- basic duplicate/conflicting VP8 and VP8L rejection;
- duplicate top-level ALPH rejection and ALPH plus VP8L rejection;
- exact ten-byte VP8X payload requirement;
- VP8X reserved-bit and reserved-byte validation;
- VP8X-only container rejection;
- presence of a production final-WebP validation call;
- retention of one-job CI and manual release policy.

---

## Residual defects found after the completion claim

| item | audited behavior at `40cdea8` | required correction | disposition |
|---|---|---|---|
| RDF Description recognition | QName is split, then local name is compared to `rdf:Description`; the condition cannot match | Match RDF namespace URI plus local `Description` | OPEN — Plan 053 |
| Alternate XMP prefixes | Filtering is gated by literal `plus:DataMining` and `stegoeggo:` strings | Remove owned fields by expanded name regardless of prefix | OPEN — Plan 053 |
| XMP namespace scope | Isolated descriptions do not inherit declarations from outer XMP/RDF elements | Whole-packet namespace-aware parsing and self-contained serialization | OPEN — Plan 053 |
| Mixed XMP preservation | Current filter can return the original description with owned fields still present | Remove only owned attributes/elements and preserve unrelated fields | OPEN — Plan 053 |
| Malformed XMP | Substring extraction can `break` and return partial success; parser `None` can mean silent omission | Any malformed packet fails the complete rewrite before output | OPEN — Plan 053 |
| XMP architecture | Substring structural parsing remains alongside `quick-xml` | One authoritative whole-packet XML event pipeline | OPEN — Plan 053 |
| XMP public API | `pub mod xmp` exposes internal helpers for tests | Restore crate-private visibility and move helper tests internally | OPEN — Plan 053 |
| VP8L header layout | Width and height shifts are incorrect; version bits are unchecked | Parse exact VP8L bit layout and reject nonzero version | OPEN — Plan 053 |
| One-pixel VP8X | Raw width-minus-one/height-minus-one zero is rejected | Decode raw zero as actual dimension one | OPEN — Plan 053 |
| Structural multiplicity | Duplicate ANIM, ICCP, and EXIF are accepted | Reject selected duplicate structural chunks before rewrite | OPEN — Plan 053 |
| Animation coherence | ANIM/ANMF multiplicity, top-level conflicts, frame headers, nested chunks, and image payload count are not validated | Add bounded coherent-animation parser | OPEN — Plan 053 |
| ANMF alpha propagation | Parser detects frame alpha, writer recomputes alpha from top-level chunks only | Use structurally derived frame alpha for emitted VP8X flags | OPEN — Plan 053 |
| Circular feature validation | Derived alpha can be initialized from the declared VP8X alpha bit | Separate declared flags from payload-derived features | OPEN — Plan 053 |
| JPEG canonical decoder | Shared entries are validated but decoder rebuilds canonical ranges independently | Derive decoder lookup directly from returned canonical entries | OPEN — Plan 053 |
| JPEG restart markers | Structural restart detection is not checked by full support probe in all cases | Any restart-bearing scan is unsupported | OPEN — Plan 053 |
| JPEG span exactness | SOS marker offset and marker-fill boundaries are not exact | Correct offsets and checked fill/stuffing handling | OPEN — Plan 053 |
| Current-head CI evidence | Recorded CI evidence names earlier SHA `c092fe0`, not final head `40cdea8` | Record exact final implementation-head evidence or `UNAVAILABLE` | OPEN — Plan 053 |

---

## Evidence correction

The previous ledger statements that mixed XMP preservation, malformed-XMP fail-closed behavior, complete animation handling, namespace-URI ownership, and current-head CI were closed are withdrawn.

The test count reported by commit messages is retained as historical local evidence only. It does not close the focused source contracts listed above.

The `quick-xml` dependency remains justified and retained. The defect is in how it is currently used, not in the decision to use a bounded XML event parser.

---

## Planning reconciliation

| plan | current disposition |
|---|---|
| Roadmap 045 | PARTIAL — residual closure delegated to Plan 053 |
| Plan 048 | substantially implemented; bounded JPEG exactness remains in Plan 053 |
| Plan 049 | substantially implemented; XMP and animated-WebP semantics remain in Plan 053 |
| Plan 050 | Superseded |
| Plan 051 | PARTIAL — residual closure delegated to Plan 053 |
| Plan 052 | PARTIAL |
| Plan 053 | OPEN |

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plans 052 or 053. Release remains manual and separate from correctness closure.
