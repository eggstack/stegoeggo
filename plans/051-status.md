# Plan 051 Status Ledger

Plan baseline SHA: `b3a08587861a17e9b290ba34fd82ca5e65575a92`

Current disposition: **PARTIAL — residual closure delegated to Plan 053**

Authoritative remaining work:

- `plans/053-xmp-animated-webp-and-jpeg-exactness-closure.md`
- `plans/053-status.md`

This ledger was originally marked complete after the Plan 051 implementation. A follow-up audit reopened bounded source contracts under Plan 052, and the post-Plan-052 audit found the final residual items now specified by Plan 053.

No version bump, publication, tag, release, or release automation is authorized by this status.

---

## Correctly closed Plan 051 work

| item | retained result | disposition |
|---|---|---|
| Legacy Light policy mapping | CLI and library use one compatibility mapping; Light maps to Unspecified | CLOSED |
| Standard legacy default | Standard maps to ProhibitedAiMlTraining | CLOSED |
| Explicit Unspecified precedence | Explicit policy is not replaced by level fallback | CLOSED |
| Missing constraints reporting | Structured warning exists with strict-error severity | CLOSED |
| Stale TDM emission documentation | Current output no longer claims to emit `tdm:reserve_tdm` | CLOSED |
| Basic Huffman table validation | Count/value equality, empty table, duplicate symbols, and oversubscription checks exist | CLOSED |
| Exact SOS table references | Table-0 fallback was removed | CLOSED |
| JPEG post-scan containment | Post-scan marker segments are excluded from DCT-success classification | CLOSED |
| Duplicate top-level VP8X | Parser rejects a second VP8X chunk | CLOSED |
| Status-file existence | Retrospective ledgers exist and are labeled historical | CLOSED |
| CI/release scope | One required CI job and manual release policy retained | CLOSED |

---

## Plan 052 improvements retained

The following later work substantially closes Plan 051 criteria and is retained:

- exact normal-path JPEG entropy-only slicing;
- actual decoded-block counting;
- extra entropy-byte and invalid pad-bit rejection;
- DHT class rejection outside DC/AC;
- RIFF declared-size equality;
- top-level RIFF padded-end and final-cursor validation;
- basic VP8X structural validation;
- duplicate/conflicting top-level VP8 and VP8L validation;
- production invocation of final WebP validation;
- adoption of `quick-xml` as the bounded XML parser dependency.

---

## Residual items delegated to Plan 053

| item | reason still open | disposition |
|---|---|---|
| XMP whole-packet parsing | Current production flow still mixes substring extraction with `quick-xml` | OPEN — Plan 053 |
| Expanded-name recognition | RDF Description and owned fields are not reliably matched by URI plus local name | OPEN — Plan 053 |
| Namespace scope | Outer namespace declarations are not safely available to isolated description filtering | OPEN — Plan 053 |
| Mixed-field preservation | Owned and unrelated fields in one description are not reliably separated | OPEN — Plan 053 |
| Malformed-XMP behavior | Partial `break`/`None` paths can omit malformed metadata without failing the rewrite | OPEN — Plan 053 |
| XMP idempotence | One current semantic owned claim after repeated rewrites is not proven | OPEN — Plan 053 |
| XMP API surface | Internal helper module is publicly exposed for tests | OPEN — Plan 053 |
| VP8L semantics | Header dimensions and version bits are parsed incorrectly or incompletely | OPEN — Plan 053 |
| One-pixel VP8X | Valid stored zero dimension-minus-one fields are rejected | OPEN — Plan 053 |
| Animated WebP coherence | ANIM/ANMF and nested frame rules remain incomplete | OPEN — Plan 053 |
| Frame alpha propagation | Parsed ANMF alpha does not reach writer flag derivation | OPEN — Plan 053 |
| Non-circular validation | Declared VP8X bits can influence the feature state used to validate those bits | OPEN — Plan 053 |
| JPEG canonical decoder | Decoder lookup still repeats canonical construction instead of consuming returned entries | OPEN — Plan 053 |
| JPEG restart/fill exactness | Restart classification and marker-fill boundaries remain incomplete | OPEN — Plan 053 |
| Final evidence | Current-head CI and planning state require exact reconciliation | OPEN — Plan 053 |

---

## Planning reconciliation

| plan | disposition |
|---|---|
| Roadmap 045 | PARTIAL — residual closure delegated to Plan 053 |
| Plan 048 | substantially implemented; bounded JPEG exactness remains |
| Plan 049 | substantially implemented; XMP and animated-WebP semantics remain |
| Plan 050 | Superseded |
| Plan 051 | PARTIAL |
| Plan 052 | PARTIAL — residual closure delegated to Plan 053 |
| Plan 053 | OPEN |

---

## Closure rule

Plan 051 may return to `COMPLETE` only after every Plan 053 row is closed with focused evidence and all affected planning documents agree.

A local test count or CI pass for an earlier SHA does not supersede an open source-level contract.

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plan 053. Release remains manual and separate.
