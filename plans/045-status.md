# Roadmap 045 Status Ledger

This ledger was created retrospectively after the original source changes.
The plan's required pre-edit Phase 0 ledger did not exist at implementation time.

Plan baseline: `main` at `4d2e849f40049bef6416cfdc4970ba576d269869`
Original claimed disposition: COMPLETE

## Original Implementation Commits

Roadmap 045 delegated to plans 046-050. Those plans were implemented via:

| SHA | Description |
|-----|-------------|
| b7e0d13 | metadata canonical classification |
| a3ae07e | CLI default-policy correction |
| 3cc4300 | JPEG preservation and entropy correction |
| 76343cd | WebP XMP/VP8X correction |
| b3a0858 | attempted evidence closure |

## Audit Findings After Original Commits

The Plan 051 audit found remaining defects:
1. Light policy inconsistency between CLI and library
2. ProhibitedSeeConstraints without constraints silently treated as complete
3. Stale tdm:reserve_tdm documentation claims
4. JPEG Huffman construction lacks complete table validation
5. JPEG entropy decoding does not prove exact exhaustion
6. WebP XMP replacement can silently discard unrelated metadata
7. VP8X alpha flag derived from input, not final payload
8. Duplicate VP8X not rejected

## Plan 051 Corrective Commits

| SHA | Description |
|-----|-------------|
| a3860e1 | policy: unify legacy level rights defaults and report incomplete see-constraints |
| 80d0a91 | jpeg: validate canonical Huffman tables and exact references |
| 93fe670 | webp: enforce declared RIFF bounds and improve XMP merge |
| fb44e3e | webp: derive and validate VP8X flags from final container state |
| 508361e | fix: formatting and clippy compliance |
| 70016f8 | docs: update AGENTS.md for correctness pass changes |

## Final Disposition

CLOSED by Plan 051. All correctness defects identified by the roadmap have been addressed.
The plan header was updated to `COMPLETE — closed by Plan 051`.

## Publication Hold

No version bump, publication, tag, release, or release automation was introduced.
