# Plan 053 Status Ledger

Plan baseline SHA: `40cdea8dbc9e110ed6e6bb3d325a10d25903b0b2`

Disposition: **PARTIAL — final XMP reference/serialization closure delegated to Plan 056**

Last attempted implementation/planning head: `7810d41960d79e06e910ed0fccb5026339c2b7eb`

Plan 054/055 corrective implementation head: `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15`

Post-implementation audit head: `e683c87785d7e4ec60b17fa8ec961d983e4b6fac`

Post-closure audit baseline: `81c934d02dd43578482e01a15ea645a62ec0209b`

Authoritative remaining work:

- `plans/056-xmp-reference-and-serialization-final-closure.md`
- `plans/056-status.md`

No release, version, tag, publication, or CI expansion is authorized.

---

## Correctly retained Plan 053 work

The following work is accepted and is not reopened without a focused regression:

- `quick-xml` retained as the single bounded XML parser dependency;
- `xmp` restored to crate-private visibility;
- invalid XMP UTF-8 rejected;
- namespace-URI plus local-name ownership matching introduced;
- alternate XMP prefixes recognized;
- corrected VP8L width/height/alpha bit extraction and version checking;
- valid one-pixel VP8X dimensions accepted;
- duplicate top-level ANIM, ICCP, and EXIF checks added;
- declared VP8X bits separated from derived `WebPFeatures`;
- final WebP validator compares declared bits against derived features;
- JPEG decoder lookup derives from shared canonical Huffman entries;
- restart-bearing JPEG scans are rejected by the full DCT support probe;
- checked JPEG structural analysis and exact fill boundaries landed in Plan 055;
- ANMF exact field/bounds/flag/payload semantics landed in Plan 054;
- RDF-qualified preserved descriptions, event-based merge, and description deduplication landed in Plan 054.

---

## Historical residual defects closed by Plans 054 and 055

| item | original required correction | owner | current disposition |
|---|---|---|---|
| Preserved RDF identity | serialize RDF-qualified descriptions by expanded name | Plan 054 | CLOSED for original scope |
| Owned XMP subtree removal | suppress owned nested element/text subtrees | Plan 054 | PARTIAL only for later-discovered event-class edge cases; Plan 056 |
| XMP structural merge | replace substring RDF insertion with XML events | Plan 054 | CLOSED architecture |
| Filtered-description dedup | deterministic first-seen exact dedup | Plan 054 | CLOSED |
| XMP three-round proof | parser-based semantic uniqueness/preservation | Plan 054 | PARTIAL only for later-discovered reference-bearing values; Plan 056 |
| Scoped namespace conflicts | accept safe self-contained sibling prefix reuse | Plan 054 | CLOSED |
| ANMF header semantics | parse exact 16-byte frame header | Plan 054 | CLOSED |
| Frame rectangle bounds | checked frame/canvas containment | Plan 054 | CLOSED |
| ANMF flags | reject reserved bits; preserve blend/dispose semantics | Plan 054 | CLOSED |
| ALPH + VP8L ordering | validate combination after inventory | Plan 054 | CLOSED |
| Animated top-level ALPH | reject in coherent animation path | Plan 054 | CLOSED |
| Malformed VP8L propagation | propagate top-level/nested parse errors | Plan 054 | CLOSED |
| JPEG checked structure | checked `Result<JpegStructure>` supported path | Plan 055 | CLOSED |
| JPEG repeated marker fill | entropy end uses first `FF` in marker run | Plan 055 | CLOSED |
| JPEG malformed boundaries | malformed marker/segment boundaries error | Plan 055 | CLOSED |
| Plan 054/055 implementation-head evidence | exact local verification and recorded CI for `0df12ede` | Plan 055 | HISTORICALLY CLOSED |

---

## Remaining residual defects — Plan 056

The Plan 054/055 completion claim did not exercise the following source-level cases:

- valid predefined XML entity references represented as `Event::GeneralRef`;
- valid decimal/hex numeric character references;
- raw `Attribute::value` being escaped a second time in structural merge;
- owned-depth End precedence when an owned subtree contains nested `rdf:Description`;
- comment/PI leakage from owned subtrees;
- reference-aware public WebP/XMP three-round semantic stability.

These are the only currently identified residuals in this line. Plan 056 explicitly does not reopen JPEG or animated-WebP implementation.

---

## Evidence correction

The earlier focused tests remain evidence for the behavior they actually exercise, but they do not prove Plan 056's reference-bearing cases.

In particular:

- ordinary text fixtures do not prove `Event::GeneralRef` handling;
- a valid XML parse without escaped attributes does not prove absence of double escaping;
- nested owned element tests without comments/PIs/nested RDF descriptions do not prove event-complete subtree suppression;
- the historical exact-head CI result for `0df12ede` cannot serve as final Plan 056 implementation-head CI evidence after source changes.

---

## Current planning state

| plan | disposition |
|---|---|
| Roadmap 045 | PARTIAL — final closure requires Plan 056 |
| Plan 048 | substantially closed |
| Plan 049 | substantially closed; final XMP residuals now narrowed to Plan 056 |
| Plan 050 | Superseded |
| Plan 051 | PARTIAL — final closure requires Plan 056 |
| Plan 052 | PARTIAL — final closure requires Plan 056 |
| Plan 053 | PARTIAL — final closure requires Plan 056 |
| Plan 054 | PARTIAL — animated-WebP closed; final XMP residuals in Plan 056 |
| Plan 055 | PARTIAL only for final cross-plan reconciliation; JPEG closed |
| Plan 056 | OPEN |

---

## Closure rule

Plan 053 may return to `COMPLETE` only after:

1. every required Plan 056 source/test row is closed;
2. final workspace verification is recorded against the actual Plan 056 implementation head;
3. exact-head CI is recorded as PASS/FAIL only if independently available, otherwise honestly `UNAVAILABLE`;
4. no required Plan 056 status row remains OPEN/PENDING.

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plan 056. Release remains manual and separate.
