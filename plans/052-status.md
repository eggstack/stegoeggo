# Plan 052 Status Ledger

Plan baseline SHA: `c9d4d4f1edf6d43557c85c1d5c121d0071eeeaa1`

Disposition: **PARTIAL — final XMP reference/serialization closure delegated to Plan 056**

Initial implementation SHA: `c092fe0ca58d8e01679924017e4ee5b57c80d576`

Follow-up corrective SHA: `40cdea8dbc9e110ed6e6bb3d325a10d25903b0b2`

Plan 053 implementation SHAs: `d507d96`, `7262c78`, `f00b993`, `e765e07`

Post-Plan-053 audit head: `e683c87785d7e4ec60b17fa8ec961d983e4b6fac`

Plan 054/055 corrective implementation head: `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15`

Post-closure audit baseline: `81c934d02dd43578482e01a15ea645a62ec0209b`

Authoritative remaining work:

- `plans/056-xmp-reference-and-serialization-final-closure.md`
- `plans/056-status.md`

No version bump, publication, tag, GitHub release, or release automation is authorized.

---

## Correctly retained Plan 052 work

The following is materially implemented and is not reopened without a focused failing fixture:

- exact normal-path JPEG entropy-only slice;
- actual decoded JPEG block counting;
- rejection of extra complete entropy bytes and invalid final pad bits;
- DHT class rejection outside DC `0` and AC `1`;
- exact referenced SOS Huffman tables;
- checked JPEG structure and exact repeated-marker-fill entropy boundaries;
- RIFF declared extent equality with physical input;
- top-level RIFF padded-end and final-cursor checks;
- duplicate/conflicting top-level WebP chunk validation;
- exact ten-byte VP8X payload requirement and reserved-field validation;
- VP8X-only container rejection;
- corrected VP8L layout/version handling;
- exact ANMF header/bounds/flags/nested-payload/alpha semantics;
- production final-WebP validation call;
- `quick-xml` as the bounded XMP parser dependency;
- RDF-qualified preserved descriptions and event-based merge architecture;
- one-job CI and manual release policy.

---

## Historical residual work closed by Plans 054 and 055

Plans 054 and 055 materially closed the previously identified XMP/animation/JPEG gaps, including RDF qualification, event-based merge, description deduplication, ANMF semantics, checked JPEG structure, and exact entropy fill handling.

Those closures remain accepted for the behavior they actually exercised.

---

## Remaining delegated work — Plan 056

A later source audit found a smaller XMP reference/serialization gap:

1. predefined XML references are rejected as `Event::GeneralRef`;
2. valid decimal/hex numeric character references are rejected;
3. merge Start/Empty attributes can be double-escaped from raw `Attribute::value` bytes;
4. owned-depth End processing does not take precedence over nested RDF-description close detection;
5. comments and processing instructions can leak from owned subtrees;
6. reference-bearing semantic idempotence is not proven through the public WebP rewrite path.

Plan 056 owns these items exclusively and retains the bounded no-DTD/no-custom-entity design.

---

## Planning reconciliation

| plan | current disposition |
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

Plan 052 may return to `COMPLETE` only after Plan 056 closes the remaining XMP reference/serialization contracts and exact implementation-head verification is recorded.

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plan 056. Release remains manual and separate from correctness closure.
