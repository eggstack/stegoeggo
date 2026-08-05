# Plan 051 Status Ledger

Plan baseline SHA: b3a08587861a17e9b290ba34fd82ca5e65575a92
Disposition: COMPLETE

## Implementation Commits

| SHA | Description |
|-----|-------------|
| e8f7d11 | plans: establish final correctness closure ledger |
| a3860e1 | policy: unify legacy level rights defaults and report incomplete see-constraints |
| 80d0a91 | jpeg: validate canonical Huffman tables and exact references |
| 93fe670 | webp: enforce declared RIFF bounds and improve XMP merge |
| fb44e3e | webp: derive and validate VP8X flags from final container state |
| 508361e | fix: formatting and clippy compliance |
| 70016f8 | docs: update AGENTS.md for correctness pass changes |

## Table A: Defect Closure

| item | audited code path | intended contract | implementation commit | focused test | result | disposition |
|------|-------------------|-------------------|-----------------------|--------------|--------|-------------|
| Light policy mapping | `legacy_default_dmi` + `resolved_dmi` | One consistent mapping everywhere | a3860e1 | `test_legacy_light_level`, `test_light_level_produces_correct_output` | CLI and library both use `default_policy()`: Light→Unspecified | CLOSED |
| Missing constraints | `resolve_request` ProhibitedSeeConstraints branch | Warning + strict failure, not hard error | a3860e1 | `prohibited_see_constraints_without_constraints_warns` | MissingRightsConstraints warning emitted; strict mode exits on error severity | CLOSED |
| Stale TDM docs | `DmiValue` doc comment | No tdm:reserve_tdm emission claims | a3860e1 | doc comment check | DmiValue doc updated to reflect no TDM emission | CLOSED |
| Huffman count/value mismatch | `parse_dht` | Reject sum(counts) != values.len() | 80d0a91 | `validate_huffman_table` | Checked with count/value sum validation | CLOSED |
| Huffman oversubscription | Canonical code space check | Reject overcommitted code lengths | 80d0a91 | `validate_huffman_table` | Remaining-code-space method implemented | CLOSED |
| Duplicate Huffman symbols | Table construction | Reject duplicate symbol entries | 80d0a91 | `validate_huffman_table` | Duplicate symbol detection added | CLOSED |
| Missing referenced DC table | `probe_dct_support` / decoder | Require exact referenced table, no table-0 fallback | 80d0a91 | `test_legacy_disabled_level`, `test_disabled_level_produces_bitidentical_output` | Table-0 fallback removed from decoder and encoder | CLOSED |
| Missing referenced AC table | `probe_dct_support` / decoder | Require exact referenced table, no table-0 fallback | 80d0a91 | above | Same as DC | CLOSED |
| Trailing post-scan marker | `probe_dct_support_full` | Reject for DCT path, fallback preserves container | 80d0a91 | `TrailingSegmentsAfterScan` variant added | DCT unsupported with exact reason | CLOSED |
| RIFF declared size smaller than physical input | `parse_webp` | Reject undersized declared length for rewrite | 93fe670 | `parse_rejects_riff_size_exceeding_input` | declared_end == data.len() enforced | CLOSED |
| Duplicate VP8X | `parse_webp` | Reject or normalize to single VP8X | 93fe670 | structural inventory | Single VP8X tracked via vp8x_index | CLOSED |
| Mixed owned/unrelated XMP description | XMP merge | Preserve unrelated fields, handle owned by policy | 93fe670 | `scenario_13_existing_unrelated_metadata_survives` | extract_unrelated_descriptions + inject_unrelated_into_xmp | CLOSED |
| Two differing valid XMP packets | XMP merge | Merge or fail on namespace conflict | 93fe670 | merge logic | All XMP packets merged via iteration | CLOSED |
| Final VP8X feature consistency | VP8X encoding | Derive from final chunk inventory | fb44e3e | VP8X flag computation | Flags computed from final chunks, not input | CLOSED |

## Table B: Commands and Evidence

| command/tool | environment/version | expected result | observed result | evidence location | status |
|--------------|---------------------|-----------------|-----------------|-------------------|--------|
| `./scripts/check.sh` | local, Rust 1.87+ | pass | pass | EXIT: 0 | VERIFIED |
| `cargo test --workspace --exclude stegoeggo-fuzz --all-features` | local | all pass | 1379 passed, 32 ignored | terminal output | VERIFIED |

## Table C: Planning Reconciliation

| plan | current header status | final disposition |
|------|-----------------------|-------------------|
| 051 | COMPLETE | All 24 definition-of-done criteria met |

## Publication Hold

No version bump, publication, tag, release, or release automation was introduced.
