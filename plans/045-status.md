# Roadmap 045 Status Ledger

This ledger was created retrospectively after the original source changes. The roadmap's required pre-edit Phase 0 ledger did not exist at implementation time.

Roadmap baseline: `main` at `4d2e849f40049bef6416cfdc4970ba576d269869`

Current disposition: **PARTIAL — residual closure delegated to Plan 053**

Authoritative remaining work:

- `plans/053-xmp-animated-webp-and-jpeg-exactness-closure.md`
- `plans/053-status.md`

No release action is authorized by this roadmap status.

---

## Historical implementation sequence

Roadmap 045 originally delegated to Plans 046 through 050:

| SHA | description |
|---|---|
| `b7e0d13` | metadata canonical classification |
| `a3ae07e` | CLI default-policy correction |
| `3cc4300` | JPEG preservation and entropy correction |
| `76343cd` | WebP XMP/VP8X correction |
| `b3a0858` | attempted evidence closure |

Plan 051 corrective sequence:

| SHA | description | retained disposition |
|---|---|---|
| `a3860e1` | one legacy policy mapping and missing-constraints reporting | Closed |
| `80d0a91` | JPEG table validation, exact references, and trailing-segment reason | Substantially closed |
| `93fe670` | declared-bound WebP iteration and initial XMP merge | Substantially closed |
| `fb44e3e` | initial output-derived VP8X feature handling | Substantially closed |
| `b414939` | attempted remaining Plan 051 closure | Partial |

Plan 052 sequence:

| SHA | description | retained disposition |
|---|---|---|
| `34a1052` | improve JPEG scan-span tracking and block counting | Substantially closed |
| `c092fe0` | JPEG/WebP container and metadata correction | Partial |
| `8a17e35` | attempted Plan 052 evidence closure | Historical only |
| `40cdea8` | follow-up Plan 052 definition-of-done correction | Partial |

---

## Correctly closed roadmap items

The following roadmap work is complete and is not reopened by Plan 053 without a focused regression:

- canonical and legacy rights-policy classification;
- one legacy protection-level policy mapping;
- Standard default restoration and explicit Unspecified precedence;
- structured missing-constraints warning and strict severity;
- current-output TDM documentation correction;
- basic JPEG Huffman count/value, empty-table, duplicate-symbol, and oversubscription validation;
- exact SOS Huffman table references;
- exact normal-path JPEG entropy slicing and decoded-block counting;
- rejection of extra entropy bytes and invalid pad bits;
- DHT class rejection outside DC/AC;
- rejection of JPEG post-scan marker segments from DCT-success classification;
- strict RIFF declared extent, chunk padding, and final cursor checks;
- duplicate VP8X and basic primary-payload conflict rejection;
- VP8X structural length and reserved-field checks;
- one required CI job and manual release policy.

---

## Remaining roadmap closure items

Plan 053 owns these final bounded items:

1. Replace mixed substring/`quick-xml` handling with one strict whole-packet namespace-aware XMP pipeline.
2. Preserve unrelated XMP attributes and child elements while removing only owned expanded names.
3. Fail the complete rewrite for malformed XMP rather than partially omitting it.
4. Merge all valid XMP packets deterministically and prove three-round semantic idempotence.
5. Restore internal XMP helpers to crate-private visibility.
6. Correct VP8L width, height, alpha, and version parsing.
7. Accept valid one-pixel VP8X canvas dimensions.
8. Reject duplicate ANIM, ICCP, and EXIF under the selected fail-closed policy.
9. Validate complete ANIM/ANMF coherence and exact nested frame payloads.
10. Propagate ANMF frame alpha into emitted VP8X flags.
11. Separate declared VP8X flags from independently derived payload features.
12. Make final WebP validation non-circular.
13. Derive JPEG decoder lookup directly from shared canonical entries.
14. Reject all restart-bearing entropy paths consistently.
15. Correct JPEG SOS offsets and marker-fill handling.
16. Reconcile final-head test and CI evidence exactly.

---

## Closure rule

Roadmap 045 may return to `COMPLETE` only when every row in `plans/053-status.md` is closed with named focused evidence and final-head CI is recorded as exact PASS, exact FAIL, or honestly UNAVAILABLE.

A passing broad test suite does not override an open focused contract row.

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plan 053. Release remains manual and separate from this roadmap.
