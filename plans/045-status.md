# Roadmap 045 Status Ledger

This ledger was created retrospectively after the original source changes. The roadmap's required pre-edit Phase 0 ledger did not exist at implementation time.

Plan baseline: `main` at `4d2e849f40049bef6416cfdc4970ba576d269869`

Current disposition: **PARTIAL — reopened by Plan 052**

The final remaining work is specified in:

- `plans/052-container-boundary-and-metadata-preservation-closure.md`

---

## Original implementation commits

Roadmap 045 delegated to Plans 046-050. The original implementation sequence was:

| SHA | Description |
|---|---|
| `b7e0d13` | Metadata canonical classification |
| `a3ae07e` | CLI default-policy correction |
| `3cc4300` | JPEG preservation and entropy correction |
| `76343cd` | WebP XMP/VP8X correction |
| `b3a0858` | Attempted evidence closure |

---

## Plan 051 corrective sequence

| SHA | Description | Current disposition |
|---|---|---|
| `a3860e1` | Unify legacy policy mapping and report missing constraints | Closed |
| `80d0a91` | Add JPEG table validation, exact references, and trailing-segment rejection | Partially closed |
| `93fe670` | Add declared-bound WebP iteration and initial XMP merge | Partially closed |
| `fb44e3e` | Derive several VP8X bits from output chunks | Partially closed |
| `508361e` | Formatting and clippy correction | Retained |
| `70016f8` | Documentation update | Requires final Plan 052 correction |
| `bff0258` | Retrospective evidence ledgers | Retained as historical evidence |
| `14e81fb` | Rustfmt compatibility correction | Retained |
| `b414939` | Attempt to close remaining Plan 051 gaps | Partially closed |

---

## Correctly closed roadmap items

The following work is complete and is not reopened by Plan 052:

- canonical and legacy rights-policy classification corrections;
- one legacy protection-level policy mapping;
- Standard default policy restoration;
- explicit Unspecified precedence;
- structured missing-constraints warning and strict severity;
- current-output TDM documentation correction;
- basic JPEG Huffman count/value, empty-table, duplicate-symbol, and oversubscription validation;
- exact SOS Huffman table references;
- rejection of JPEG post-scan marker segments from DCT-success classification;
- duplicate top-level WebP VP8X rejection;
- basic output-derived ICC, EXIF, XMP, animation, and top-level ALPH flag handling;
- manual release policy and one-job CI scope.

---

## Remaining roadmap closure items

A follow-up audit of `main` at `b414939b0b14083d5c56ae09ae87cade53736776` found these criteria still open:

1. JPEG structural analysis does not provide the decoder an exact entropy-only span.
2. JPEG finalization does not reject extra complete entropy bytes and receives fabricated expected/observed MCU equality.
3. DHT classes outside DC/AC are not rejected.
4. Encoder and decoder canonical Huffman state is still constructed independently.
5. WebP rewrite parsing does not require declared RIFF extent to equal physical input length.
6. WebP parsing does not prove exact final cursor and odd-chunk pad containment.
7. VP8X-only, duplicate/conflicting primary payload, malformed VP8X, and incoherent animation structures are not fully rejected.
8. Existing XMP packets containing any owned property are skipped in full, so unrelated fields in mixed packets can be lost.
9. Malformed owned XMP can be skipped before checked parsing, and the string scanner can return partial success.
10. Intrinsic VP8L alpha and ANMF frame alpha are not reflected in final VP8X state.
11. No production final-output validator proves emitted WebP structure and feature-bit consistency.
12. Plan 051 evidence and current-head CI claims require truthful reconciliation.

These are the bounded scope of Plan 052.

---

## Closure rule

Roadmap 045 may return to `COMPLETE` only after Plan 052 proves every definition-of-done item with focused tests, exact local command results, and exact current-head CI evidence or an honest unavailable disposition.

If any item remains open, this roadmap stays `PARTIAL`.

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plan 052. Release remains manual and separate from this roadmap.
