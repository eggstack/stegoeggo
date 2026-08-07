# Roadmap 045 Status Ledger

This ledger was created retrospectively after the original source changes. The roadmap's required pre-edit Phase 0 ledger did not exist at implementation time.

Roadmap baseline: `main` at `4d2e849f40049bef6416cfdc4970ba576d269869`

Disposition: **COMPLETE — final residual closure completed by Plans 054 and 055**

Post-Plan-053 audit head: `e683c87785d7e4ec60b17fa8ec961d983e4b6fac`

Final corrective implementation head: `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15`

Historical corrective work, now closed:

- `plans/054-xmp-and-animated-webp-semantic-closure.md`
- `plans/054-status.md`
- `plans/055-jpeg-structure-and-final-evidence-closure.md`
- `plans/055-status.md`

No release action is authorized by this roadmap status.

---

## Historical implementation sequence

Roadmap 045 originally delegated to Plans 046 through 050.

| SHA | description |
|---|---|
| `b7e0d13` | metadata canonical classification |
| `a3ae07e` | CLI default-policy correction |
| `3cc4300` | JPEG preservation and entropy correction |
| `76343cd` | WebP XMP/VP8X correction |
| `b3a0858` | attempted evidence closure |

Plan 051/052 corrective work added policy consistency, Huffman/table validation, exact normal entropy slicing, RIFF extent validation, and early XMP/WebP fixes.

Plan 053 implementation sequence:

| SHA | description | post-audit disposition |
|---|---|---|
| `d507d96` | strict XMP parser foundation | Substantially closed; serializer/subtree/merge residuals remain |
| `7262c78` | XMP merge/privacy and conformance tests | Partial; dedup/idempotence proof remains |
| `f00b993` | VP8L and animation validation | Partial; exact ANMF semantics remain |
| `e765e07` | JPEG canonical decoder and marker work | Substantially closed; checked structure/fill exactness remains |
| `7810d41` | attempted final evidence closure | Historical only; completion claim withdrawn |
| `e683c87` | empty CI-trigger commit | Not correctness evidence |

---

## Correctly closed roadmap work

The following remains closed unless a focused regression demonstrates otherwise:

- canonical/legacy rights-policy behavior;
- Standard default and explicit Unspecified precedence;
- structured missing-constraints warning;
- current-output TDM documentation correction;
- core JPEG Huffman validation and exact SOS table references;
- exact normal-path entropy-only decoding and block-count/finalization checks;
- DHT class rejection;
- decoder lookup derived from shared canonical Huffman entries;
- restart-bearing scan rejection at the full support probe;
- strict WebP RIFF extent, chunk padding, and final cursor checks;
- duplicate top-level VP8X/VP8/VP8L/ALPH/ANIM/ICCP/EXIF checks already present;
- corrected VP8L bit layout and version helper;
- one-pixel VP8X canvas acceptance;
- declared/derived VP8X feature separation;
- crate-private XMP module;
- one required CI job and manual release policy.

---

## Remaining roadmap closure items

Plan 054 owns:

1. RDF-qualified preserved XMP descriptions.
2. Exact owned-element subtree suppression.
3. Event-based XMP merge without substring RDF boundary searches.
4. Exact filtered-description deduplication.
5. Parser-based multi-round semantic idempotence.
6. Correct scope handling for namespace prefix reuse.
7. Exact ANMF X/Y/W/H/duration/flag decoding.
8. Frame rectangle/canvas bounds.
9. Order-independent ALPH/VP8L validation.
10. Animated top-level ALPH rejection.
11. Propagation of malformed top-level/nested VP8L errors.
12. End-to-end valid animated metadata rewrite evidence.

Plan 055 owns:

13. Checked JPEG structural analysis.
14. Malformed marker/segment boundary errors instead of partial state.
15. First-fill-byte entropy termination for repeated FF marker runs.
16. Checked support/decode routing through the exact structure.
17. Final workspace and current-head CI evidence reconciliation.
18. Final agreement among Roadmap 045 and Plans 051-055.

---

## Final closure reconciliation

Plans 054 and 055 closed all residual XMP, animated-WebP, JPEG structural, and evidence
criteria. Local verification and exact implementation-head CI are recorded in
`plans/055-status.md`; CI run `31219089804` passed for `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15`.

## Closure rule

Roadmap 045 is `COMPLETE` because both `plans/054-status.md` and `plans/055-status.md`
satisfy every required definition-of-done item.

A broad test pass cannot override an open source-level contract. A CI result from an earlier SHA cannot serve as final-head evidence.

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plans 054 or 055. Release remains manual and separate from this roadmap.
