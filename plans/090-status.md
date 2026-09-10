# Roadmap 090 Status

Status: COMPLETE

Audited baseline: `27bdd3d429d663948021de43d3e6f818fa613319`

## Child plans

| Plan | Scope | Status |
|---|---|---|
| 091 | Transactional JPEG carrier semantics and seed-hint correctness | IMPLEMENTED |
| 092 | Validated carrier configuration and result contracts | IMPLEMENTED |
| 093 | LSB V2 permutation invariant proof and compatibility | IMPLEMENTED |
| 094 | Opaque prepared JPEG carrier API | IMPLEMENTED |
| 095 | Generic strided pixel-buffer LSB API | IMPLEMENTED |
| 096 | Parent/carrier boundary and v1 API cleanup | IMPLEMENTED |
| 097 | Carrier v1 packaging, documentation, and evidence closure | IMPLEMENTED |

## Closure ledger

- [x] Plan 091 complete with correctness/compatibility evidence
- [x] Plan 092 complete with public API/semver evidence
- [x] Plan 093 complete with permutation disposition and known-answer evidence
- [x] Plan 094 complete with single-decode reuse evidence
- [x] Plan 095 complete with strided-buffer equivalence evidence
- [x] Plan 096 complete with parent behavior and v1 API disposition
- [x] Plan 097 complete with package/external-consumer/MSRV/CI evidence
- [x] final integrated `./scripts/check.sh` passes (local full gate green, 2026-09-10)
- [x] no deferred algorithm/media/provenance work accidentally pulled into scope

Deferred items honoring roadmap constraints: progressive/multiscan JPEG
support, new carriers, watermarking, FPE/crypto adoption, C2PA, plugin
framework, and auto-dispatch facade untouched. Successor V2 mapping design
explicitly deferred (Plan 093 disposition C). Internal JPEG-context
signature threading deferred to the v1 boundary to protect canonical
metadata bytes (Plan 096). No branch-protection change made (docs made
truthful instead). No crates.io publish performed.

Update this file from child status ledgers and integrated source/test evidence. Do not mark the roadmap complete from plan text or agent reports alone.

Implementation commit SHA: `d42f8ba` (roadmap 090 implementation on `main`; this ledger closure is the follow-up commit).
Remote CI (`CI` / `Check`, run `34535528838`): success on `494397b` (4m20s, 2026-09-10).
