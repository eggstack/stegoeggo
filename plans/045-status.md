# Roadmap 045 Status Ledger

This ledger was created retrospectively after the original source changes. The roadmap's required pre-edit Phase 0 ledger did not exist at implementation time.

Roadmap baseline: `main` at `4d2e849f40049bef6416cfdc4970ba576d269869`

Disposition: **COMPLETE**

Post-Plan-053 audit head: `e683c87785d7e4ec60b17fa8ec961d983e4b6fac`

Plan 054/055 corrective implementation head: `0df12ede57bdbcc74194cbb8a8cb5a406f4d9a15`

Post-closure audit baseline: `81c934d02dd43578482e01a15ea645a62ec0209b`

Plan 056 implementation head: `96926b761275e70c83c6def2be0f667154799037`

Final XMP reference/serialization closure is recorded in `plans/056-status.md`.

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
| `d507d96` | strict XMP parser foundation | Substantially closed; serializer/subtree/merge residuals remained |
| `7262c78` | XMP merge/privacy and conformance tests | Partial; dedup/idempotence proof remained |
| `f00b993` | VP8L and animation validation | Partial; exact ANMF semantics remained |
| `e765e07` | JPEG canonical decoder and marker work | Substantially closed; checked structure/fill exactness remained |
| `7810d41` | attempted final evidence closure | Historical only; completion claim withdrawn |
| `e683c87` | empty CI-trigger commit | Not correctness evidence |

Plans 054 and 055 then materially closed the previously identified XMP/animation/JPEG residuals. A later source audit found a narrower final XMP reference/serialization defect set now assigned to Plan 056.

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
- checked JPEG structure and first-fill-byte entropy boundaries;
- restart-bearing scan rejection at the full support probe;
- strict WebP RIFF extent, chunk padding, and final cursor checks;
- duplicate top-level VP8X/VP8/VP8L/ALPH/ANIM/ICCP/EXIF checks;
- corrected VP8L bit layout and version helper;
- one-pixel VP8X canvas acceptance;
- declared/derived VP8X feature separation;
- exact ANMF frame header, bounds, flags, nested payload, and alpha semantics;
- animated WebP metadata rewrite with unchanged frame payloads;
- RDF-qualified XMP descriptions;
- namespace-expanded owned-field matching;
- event-based XMP merge architecture;
- byte-identical preserved-description deduplication;
- crate-private XMP module;
- one required CI job and manual release policy.

---

## Historical residual work closed by Plans 054 and 055

Plan 054 materially closed:

1. RDF-qualified preserved XMP descriptions.
2. Owned-element subtree suppression for the originally covered nested element/text cases.
3. Event-based XMP merge without substring RDF boundary searches.
4. Exact filtered-description deduplication.
5. Parser-based multi-round semantic idempotence for the originally covered values.
6. Correct scope handling for namespace prefix reuse.
7. Exact ANMF X/Y/W/H/duration/flag decoding.
8. Frame rectangle/canvas bounds.
9. Order-independent ALPH/VP8L validation.
10. Animated top-level ALPH rejection.
11. Propagation of malformed top-level/nested VP8L errors.
12. End-to-end valid animated metadata rewrite evidence.

Plan 055 materially closed:

13. Checked JPEG structural analysis.
14. Malformed marker/segment boundary errors instead of partial state.
15. First-fill-byte entropy termination for repeated FF marker runs.
16. Checked support/decode routing through the exact structure.
17. Exact-head workspace/CI evidence for the Plan 054/055 implementation head.

---

## Final XMP closure — Plan 056

Plan 056 closed the final bounded XMP residuals:

1. Accept XML predefined references (`amp`, `lt`, `gt`, `apos`, `quot`) without adding DTD/entity-expansion machinery.
2. Accept valid decimal and hexadecimal numeric character references and reject invalid references.
3. Keep unresolved custom named entities and DOCTYPE expansion unsupported.
4. Decode/normalize attribute semantic values before exactly one escaping pass in structural merge.
5. Apply the attribute rule to both Start and Empty events.
6. Make `owned_depth` End handling take precedence over nested RDF-description close detection.
7. Suppress comments and processing instructions inside owned subtrees.
8. Ensure valid reference events inside owned subtrees cannot leak into preserved output.
9. Prove reference-bearing XMP semantics across the public WebP rewrite path and three rewrite rounds.
10. Run exact final-head verification and reconcile the historical status chain truthfully.

---

## Closure rule

Roadmap 045 is COMPLETE because `plans/056-status.md` satisfies every required Plan 056 definition-of-done item and final verification is recorded against implementation head `96926b761275e70c83c6def2be0f667154799037`.

A broad historical test pass cannot override an open source-level contract. A CI result from an earlier SHA cannot serve as final-head evidence for Plan 056.

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plan 056. Release remains manual and separate from this roadmap.
