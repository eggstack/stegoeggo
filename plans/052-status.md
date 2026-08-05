# Plan 052 Status Ledger

Plan baseline SHA: `c9d4d4f1edf6d43557c85c1d5c121d0071eeeaa1`

Disposition: OPEN

This plan closes the remaining container-boundary and metadata-preservation defects identified by post-Plan-051 audit. It does not authorize format expansion, public API redesign, additional protection policies, new cryptography, release work, or broader CI.

---

## Retained Plan 051 work (not reopened)

The following Plan 051 items remain correctly closed and are not reopened:

- one legacy `ProtectionLevel` to `RightsPolicy` mapping;
- `Light -> Unspecified` and `Standard -> ProhibitedAiMlTraining` compatibility behavior;
- structured `MissingRightsConstraints` reporting;
- removal of stale current-output TDM emission claims;
- basic JPEG Huffman count/value, empty-table, duplicate-symbol, and oversubscription checks;
- exact SOS DC/AC table references rather than table-0 fallback;
- rejection of JPEG post-scan marker segments from the supported DCT path;
- duplicate top-level WebP VP8X rejection;
- basic final-output VP8X ICC, EXIF, XMP, animation, and ALPH-chunk flag derivation;
- creation of retrospective planning ledgers;
- retention of one required CI job and manual release policy.

---

## Open defect rows

| Item | Status |
|------|--------|
| JPEG exact entropy span | OPEN |
| JPEG exact exhaustion | OPEN |
| JPEG DHT class validation | OPEN |
| Shared canonical Huffman representation | OPEN |
| WebP declared RIFF equality | OPEN |
| WebP final cursor and pad validation | OPEN |
| WebP primary payload validation | OPEN |
| WebP VP8X structural validation | OPEN |
| Mixed XMP field preservation | OPEN |
| Malformed XMP fail-closed behavior | OPEN |
| VP8L alpha detection | OPEN |
| ANMF alpha detection | OPEN |
| Final WebP validator | OPEN |
| Focused fixtures | OPEN |
| Workspace verification | OPEN |
| Current-head CI evidence | OPEN |
| Publication hold | no publication is part of this plan |

---

## Table A: defect closure

| item | audited source path | exact contract | implementation commit | focused fixture/test | observed result | disposition |
|------|---------------------|----------------|----------------------|---------------------|-----------------|-------------|
| | | | | | | |

Rows will be populated as each defect is closed.

---

## Table B: commands and evidence

| command/tool | environment/version | expected result | observed result | exact SHA or evidence location | status |
|--------------|---------------------|-----------------|-----------------|-------------------------------|--------|
| | | | | | |

---

## Table C: planning reconciliation

| plan | pre-052 claim | open criteria at 052 baseline | corrective commit(s) | final disposition |
|------|---------------|-------------------------------|----------------------|-------------------|
| 045 | COMPLETE | 12 residual items | | PARTIAL |
| 048 | CLOSED | exact entropy + shared Huffman | | Retained |
| 049 | CLOSED | strict WebP structure + field-level XMP | | Retained |
| 050 | Superseded | N/A | | Superseded |
| 051 | PARTIAL | 16 residual items delegated | | PARTIAL |
| 052 | OPEN | all definition-of-done items | | OPEN |

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of this plan. Release remains manual and separate from correctness closure.
