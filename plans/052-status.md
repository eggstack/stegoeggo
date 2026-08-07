# Plan 052 Status Ledger

Plan baseline SHA: `c9d4d4f1edf6d43557c85c1d5c121d0071eeeaa1`

Disposition: **COMPLETE — final residual closure completed by Plans 054 and 055**

Initial implementation SHA: `c092fe0ca58d8e01679924017e4ee5b57c80d576`

Follow-up corrective SHA: `40cdea8dbc9e110ed6e6bb3d325a10d25903b0b2`

Plan 053 implementation SHAs: `d507d96`, `7262c78`, `f00b993`, `e765e07`

Post-Plan-053 audit head: `e683c87785d7e4ec60b17fa8ec961d983e4b6fac`

Final corrective implementation head: `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15`

The previous `COMPLETE` disposition is withdrawn. Plan 053 closed much of the Plan 052 residual scope, but source review found remaining semantic and structural gaps now assigned to Plans 054 and 055.

No version bump, publication, tag, GitHub release, or release automation is authorized by this status correction.

---

## Correctly retained Plan 052 work

The following items are materially implemented and are not reopened without a focused failing fixture:

- exact normal-path JPEG entropy-only slice;
- actual decoded JPEG block counting;
- rejection of extra complete entropy bytes and invalid final pad bits;
- DHT class rejection outside DC `0` and AC `1`;
- exact referenced SOS Huffman tables;
- RIFF declared extent equality with physical input;
- top-level RIFF padded-end and final-cursor checks;
- duplicate top-level VP8X rejection;
- duplicate/conflicting top-level VP8 and VP8L rejection;
- duplicate top-level ALPH rejection and static ALPH plus VP8L rejection;
- exact ten-byte VP8X payload requirement;
- VP8X reserved-bit and reserved-byte validation;
- VP8X-only container rejection;
- production final-WebP validation call;
- `quick-xml` as the bounded XMP parser dependency;
- one-job CI and manual release policy.

Plan 053 additionally retained:

- decoder lookup derived from shared canonical Huffman entries;
- restart-bearing JPEG rejection;
- corrected VP8L header bit layout/version helper;
- one-pixel VP8X canvas support;
- duplicate ANIM/ICCP/EXIF checks;
- declared/derived WebP feature separation;
- crate-private namespace-aware XMP parser foundation.

---

## Historical residual work closed by Plan 054

The following Plan 052/053 WebP-XMP closure claims remain incomplete:

1. Preserved RDF descriptions must remain RDF-qualified after serialization.
2. Owned XMP elements must be removed as complete nested subtrees.
3. XMP merge must stop using substring RDF boundary searches.
4. Filtered descriptions require real deterministic exact deduplication.
5. Three-round semantic idempotence requires expanded-name/value proof.
6. ANMF requires exact X/Y/W/H/duration/flag decoding.
7. Frame rectangles must be checked against the VP8X canvas.
8. Nested ALPH + VP8L rejection must be order-independent.
9. Animated top-level ALPH must be rejected.
10. Malformed classified VP8L payloads must propagate errors.
11. Valid opaque and alpha animation metadata rewrites must be proven end to end.

See `plans/054-status.md` for the authoritative CLOSED evidence.

---

## Historical residual work closed by Plan 055

The following JPEG/evidence closure claims remain incomplete:

1. Structural scan analysis must be checked rather than best-effort.
2. Truncated/short/overflow marker segments must error instead of returning partial state.
3. Repeated marker-fill runs must set entropy end at the first `0xFF`.
4. The full support probe and decoder must consume the checked structure.
5. Exact final-head verification/CI evidence must be reconciled truthfully.
6. Historical Plans 045/051/052/053 may close only after Plans 054 and 055 both close.

See `plans/055-status.md` for the authoritative CLOSED evidence.

---

## Planning reconciliation

| plan | current disposition |
|---|---|
| Roadmap 045 | COMPLETE — final closure completed by Plans 054 and 055 |
| Plan 048 | substantially closed; checked JPEG structure delegated to Plan 055 |
| Plan 049 | substantially closed; XMP/animation semantics delegated to Plan 054 |
| Plan 050 | Superseded |
| Plan 051 | COMPLETE |
| Plan 052 | COMPLETE |
| Plan 053 | COMPLETE |
| Plan 054 | COMPLETE |
| Plan 055 | COMPLETE |

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plans 052-055. Release remains manual and separate from correctness closure.
