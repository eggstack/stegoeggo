# Plan 052 Status Ledger

Plan baseline SHA: `c9d4d4f1edf6d43557c85c1d5c121d0071eeeaa1`

Disposition: **COMPLETE — residual closure completed by Plan 053**

Initial implementation SHA: `c092fe0ca58d8e01679924017e4ee5b57c80d576`

Follow-up corrective SHA: `40cdea8dbc9e110ed6e6bb3d325a10d25903b0b2`

Plan 053 residual closure SHAs: `d507d96`, `7262c78`, `f00b993`, `e765e07`

Final head SHA: `e765e074efeefc83093fb9d92817955b00a87d90`

The earlier `COMPLETE` disposition was not supported by the final source audit. After Plan 053 closed the residual defects, the entire Plan 052 contract is materially implemented.

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

## Plan 053 residual closure evidence

All Plan 053 rows that delegated to this plan are closed. The focused tests and evidence are recorded in `plans/053-status.md`. This ledger retains the previous residual table as historical context only.

---

## Evidence correction

The previous ledger statements that mixed XMP preservation, malformed-XMP fail-closed behavior, complete animation handling, namespace-URI ownership, and current-head CI were closed are withdrawn. The withdrawn claims are now backed by focused tests in `plans/053-status.md`.

The `quick-xml` dependency remains justified and retained. The defect was in how it was used; the new pipeline uses `NsReader` with namespace-aware events.

---

## Planning reconciliation

| plan | current disposition |
|---|---|
| Roadmap 045 | COMPLETE — residual closure completed by Plan 053 |
| Plan 048 | COMPLETE — canonical decoder and restart/fill exactness landed in `e765e07` |
| Plan 049 | COMPLETE — XMP and animated-WebP semantics landed in `d507d96`, `7262c78`, `f00b993` |
| Plan 050 | Superseded |
| Plan 051 | COMPLETE — residual closure completed by Plan 053 |
| Plan 052 | COMPLETE — residual closure completed by Plan 053 |
| Plan 053 | COMPLETE |

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plans 052 or 053. Release remains manual and separate from correctness closure.
