# Plan 051 Status Ledger

Plan baseline SHA: b3a08587861a17e9b290ba34fd82ca5e65575a92
Disposition: OPEN
Legacy Light-policy consistency: OPEN
Missing-constraints reporting: OPEN
TDM documentation correction: OPEN
JPEG canonical Huffman validation: OPEN
JPEG exact table references: OPEN
JPEG entropy exhaustion: OPEN
JPEG post-scan containment: OPEN
WebP declared-RIFF bounds: OPEN
WebP structural inventory: OPEN
WebP mixed/differing XMP preservation: OPEN
WebP malformed-XMP behavior: OPEN
VP8X final-inventory validation: OPEN
Targeted tests: OPEN
Workspace verification: OPEN
Retrospective ledgers: OPEN
CI evidence: OPEN
Publication hold: no publication is part of this plan

---

## Table A: Defect Closure

| item | audited code path | intended contract | implementation commit | focused test | result | disposition |
|------|-------------------|-------------------|-----------------------|--------------|--------|-------------|
| Light policy mapping | `legacy_default_dmi` + `resolved_dmi` | One consistent mapping everywhere | | | | OPEN |
| Missing constraints | `resolve_request` ProhibitedSeeConstraints branch | Warning + strict failure, not hard error for all | | | | OPEN |
| Stale TDM docs | `DmiValue` doc comment, README, AGENTS.md | No tdm:reserve_tdm emission claims | | | | OPEN |
| Huffman count/value mismatch | `parse_dht` / `HuffmanDecoder::from_table` | Reject sum(counts) != values.len() | | | | OPEN |
| Huffman oversubscription | Canonical code space check | Reject overcommitted code lengths | | | | OPEN |
| Duplicate Huffman symbols | Table construction | Reject duplicate symbol entries | | | | OPEN |
| Missing referenced DC table | `probe_dct_support` / decoder | Require exact referenced table, no table-0 fallback | | | | OPEN |
| Missing referenced AC table | `probe_dct_support` / decoder | Require exact referenced table, no table-0 fallback | | | | OPEN |
| Entropy trailing bytes | `BitReader` / scan exhaustion | Prove exact scan exhaustion, reject extra bytes | | | | OPEN |
| Invalid entropy pad bits | Finalization check | Require valid JPEG pad-bit values | | | | OPEN |
| Trailing post-scan marker | `probe_dct_support_full` | Reject for DCT path, fallback preserves container | | | | OPEN |
| RIFF declared size smaller than physical input | `parse_webp` | Reject undersized declared length for rewrite | | | | OPEN |
| Duplicate VP8X | `parse_webp` | Reject or normalize to single VP8X | | | | OPEN |
| VP8X without primary image/animation payload | `parse_webp` | Reject VP8X-only without VP8/VP8L/ANMF | | | | OPEN |
| Mixed owned/unrelated XMP description | `merge_xmp_preserve_unrelated` | Preserve unrelated fields, handle owned by policy | | | | OPEN |
| Two differing valid XMP packets | XMP merge | Merge or fail on namespace conflict | | | | OPEN |
| Malformed plus valid XMP packets | XMP merge | Fail before output, not silent discard | | | | OPEN |
| Final VP8X feature consistency | VP8X encoding | Derive from final chunk inventory | | | | OPEN |

---

## Table B: Commands and Evidence

| command/tool | environment/version | expected result | observed result | evidence location | status |
|--------------|---------------------|-----------------|-----------------|-------------------|--------|
| | | | | | |

---

## Table C: Planning Reconciliation

| plan | current header status | required missing ledger | source criteria still open | final disposition |
|------|-----------------------|-------------------------|---------------------------|-------------------|
| 039 | | plans/039-status.md | | |
| 040 | | plans/040-status.md | | |
| 041 | | plans/041-status.md | | |
| 042 | | plans/042-status.md | | |
| 043 | | plans/043-status.md | | |
| 044 | | plans/044-status.md | | |
| 045 | COMPLETE | | | Reopened by Plan 051 |
| 046 | COMPLETE | plans/046-status.md | | |
| 047 | COMPLETE | plans/047-status.md | | |
| 048 | COMPLETE | plans/048-status.md | | |
| 049 | COMPLETE | plans/049-status.md | | |
| 050 | COMPLETE | plans/050-status.md | | |
| 051 | OPEN | This file | | |
