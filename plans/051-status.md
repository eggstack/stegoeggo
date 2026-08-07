# Plan 051 Status Ledger

Plan baseline SHA: `b3a08587861a17e9b290ba34fd82ca5e65575a92`

Disposition: **PARTIAL — final residual closure delegated to Plans 054 and 055**

Post-Plan-053 audit head: `e683c87785d7e4ec60b17fa8ec961d983e4b6fac`

This ledger was previously marked complete after Plan 053. A later source audit found bounded XMP, animated-WebP, JPEG structural, and final-evidence criteria still open. Those remaining contracts are now split between Plans 054 and 055.

No version bump, publication, tag, release, or release automation is authorized by this status.

---

## Correctly closed Plan 051 work

| item | retained result | disposition |
|---|---|---|
| Legacy Light policy mapping | one compatibility mapping; Light maps to Unspecified | CLOSED |
| Standard legacy default | Standard maps to ProhibitedAiMlTraining | CLOSED |
| Explicit Unspecified precedence | explicit policy is not replaced by level fallback | CLOSED |
| Missing constraints reporting | structured warning exists with strict-error severity | CLOSED |
| Stale TDM emission documentation | current output no longer claims `tdm:reserve_tdm` emission | CLOSED |
| Basic Huffman table validation | count/value equality, empty table, duplicate symbols, oversubscription checks | CLOSED |
| Exact SOS table references | table-0 fallback removed | CLOSED |
| JPEG post-scan containment | post-scan marker segments excluded from DCT-success classification | CLOSED |
| Duplicate top-level VP8X | parser rejects a second VP8X | CLOSED |
| RIFF extent/final cursor improvements | later Plan 052 work materially landed | CLOSED |
| CI/release scope | one required CI job and manual release policy retained | CLOSED |

---

## Later improvements retained

The following work from Plans 052-053 substantially satisfies delegated Plan 051 criteria and is retained:

- exact normal-path JPEG entropy-only slicing;
- actual decoded-block counting;
- unread entropy byte and final pad-bit rejection;
- DHT class rejection outside DC/AC;
- decoder lookup derived from shared canonical Huffman entries;
- restart-bearing scan rejection at the full DCT probe;
- strict RIFF declared-size, padded-end, and final-cursor checks;
- VP8X structural length and reserved-field checks;
- corrected VP8L bit layout/version helper;
- duplicate ANIM/ICCP/EXIF checks;
- declared-vs-derived WebP feature separation;
- crate-private namespace-aware XMP parser foundation.

---

## Remaining delegated work

Plan 054 owns the residual XMP and animated-WebP semantic contracts:

- RDF-qualified preserved descriptions;
- exact owned-element subtree removal;
- event-based XMP merge;
- real description deduplication and semantic idempotence;
- standards-correct ANMF header/frame bounds;
- order-independent frame ALPH/VP8L rules;
- malformed VP8L propagation;
- animated rewrite proof.

Plan 055 owns the residual JPEG/evidence contracts:

- checked structural analysis instead of partial best-effort state;
- exact repeated-marker-fill boundary handling;
- checked probe/decode routing;
- final exact-head workspace/CI evidence and planning reconciliation.

---

## Current planning reconciliation

| plan | disposition |
|---|---|
| Roadmap 045 | PARTIAL — final closure requires Plans 054 and 055 |
| Plan 048 | substantially closed; checked JPEG structural residuals in Plan 055 |
| Plan 049 | substantially closed; XMP/animation residuals in Plan 054 |
| Plan 050 | Superseded |
| Plan 051 | PARTIAL |
| Plan 052 | PARTIAL — final closure requires Plans 054 and 055 |
| Plan 053 | PARTIAL — residual closure delegated |
| Plan 054 | OPEN |
| Plan 055 | OPEN |

---

## Closure rule

Plan 051 may return to `COMPLETE` only after Plans 054 and 055 are complete and their focused evidence supports all remaining delegated criteria.

A local test count or CI pass for an earlier SHA does not supersede an open source-level contract.

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plans 054-055. Release remains manual and separate.
