# Plan 051 Status Ledger

Plan baseline SHA: `b3a08587861a17e9b290ba34fd82ca5e65575a92`

Disposition: **PARTIAL — final XMP reference/serialization closure delegated to Plan 056**

Post-Plan-053 audit head: `e683c87785d7e4ec60b17fa8ec961d983e4b6fac`

Plan 054/055 corrective implementation head: `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15`

Post-closure audit baseline: `81c934d02dd43578482e01a15ea645a62ec0209b`

Authoritative remaining work:

- `plans/056-xmp-reference-and-serialization-final-closure.md`
- `plans/056-status.md`

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

The following later corrective work remains accepted and is not reopened without a focused failing fixture:

- exact JPEG entropy-only slicing, block counting, unread-byte and pad-bit rejection;
- checked JPEG structural analysis and exact repeated-marker-fill boundaries;
- DHT class rejection and shared canonical Huffman-derived decoder lookup;
- restart-bearing JPEG rejection;
- strict RIFF declared-size, padded-end, and final-cursor checks;
- VP8X structural and feature validation;
- corrected VP8L header layout/version handling;
- ANMF exact frame semantics, bounds, flags, nested payload validation, and alpha derivation;
- animated WebP metadata rewrite preserving frame payloads;
- crate-private namespace-aware XMP parser foundation;
- RDF-qualified preserved descriptions;
- event-based XMP merge architecture;
- preserved-description deduplication.

---

## Remaining delegated work — Plan 056

A post-Plan-054/055 source audit found a narrower XMP semantic gap not exercised by the prior completion fixtures:

- predefined XML references are rejected unconditionally as `Event::GeneralRef`;
- valid numeric character references are rejected likewise;
- structural merge can double-escape raw attribute values;
- nested owned `rdf:Description` End handling can outrank `owned_depth`;
- comments and processing instructions can leak from owned subtrees;
- reference-aware public rewrite/idempotence evidence is missing.

Plan 056 owns only those residuals. It does not reopen the closed JPEG or animated-WebP work.

---

## Current planning reconciliation

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

Plan 051 may return to `COMPLETE` only after Plan 056 closes every remaining XMP reference/serialization criterion and records exact implementation-head verification.

A local test count or CI pass for an earlier SHA does not supersede an open source-level contract.

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plan 056. Release remains manual and separate.
