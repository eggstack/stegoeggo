# Roadmap 045 Status Ledger

This ledger was created retrospectively after the original source changes. The roadmap's required pre-edit Phase 0 ledger did not exist at implementation time.

Roadmap baseline: `main` at `4d2e849f40049bef6416cfdc4970ba576d269869`

Disposition: **COMPLETE — residual closure completed by Plan 053**

Final head SHA: `7810d41960d79e06e910ed0fccb5026339c2b7eb`

Plan 053 SHAs closing the residual items: `d507d96`, `7262c78`, `f00b993`, `e765e07`

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

Plan 053 sequence:

| SHA | description | retained disposition |
|---|---|---|
| `d507d96` | strict XMP filter via `quick-xml::NsReader` | Closed |
| `7262c78` | private XMP merge and conformance tests | Closed |
| `f00b993` | VP8L and animation validation | Closed |
| `e765e07` | JPEG canonical decoder and scan marker exactness | Closed |

---

## Correctly closed roadmap items

The following roadmap work is complete and is not reopened without a focused regression:

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

## Plan 053 closure items

All sixteen Plan 053 closure items are closed. The focused tests and evidence are recorded in `plans/053-status.md`.

---

## Closure rule

Roadmap 045 is `COMPLETE`. Every row in `plans/053-status.md` is closed with named focused evidence. Final-head CI requires a post-push GitHub Actions run; local `scripts/check.sh` runs green at `e765e07`.

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plan 053. Release remains manual and separate from this roadmap.
