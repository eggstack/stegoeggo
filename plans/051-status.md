# Plan 051 Status Ledger

Plan baseline SHA: `b3a08587861a17e9b290ba34fd82ca5e65575a92`

Disposition: **PARTIAL — residual closure delegated to Plan 052**

This ledger was originally marked complete after the Plan 051 implementation. A follow-up audit of `main` at `b414939b0b14083d5c56ae09ae87cade53736776` found that several source-level acceptance criteria remained open despite the completion claim.

The corrective implementation is specified in:

- `plans/052-container-boundary-and-metadata-preservation-closure.md`

No version bump, publication, tag, release, or release automation is authorized by this status change.

---

## Implementation commits retained from Plan 051

| SHA | Description | Current disposition |
|---|---|---|
| `e8f7d11` | Establish Plan 051 closure ledger | Retained |
| `a3860e1` | Unify legacy rights defaults and report missing constraints | Closed |
| `80d0a91` | Add JPEG table validation, exact references, and trailing-segment reason | Partially closed; remaining JPEG work moved to Plan 052 |
| `93fe670` | Add declared-bound WebP iteration and initial XMP merge changes | Partially closed; remaining WebP work moved to Plan 052 |
| `fb44e3e` | Derive several VP8X bits from output inventory | Partially closed; VP8L/frame alpha and final validation remain |
| `508361e` | Formatting and clippy compliance | Retained |
| `70016f8` | Documentation updates | Requires final correction after Plan 052 |
| `bff0258` | Backfill retrospective ledgers | Retained as retrospective evidence only |
| `14e81fb` | Rustfmt compatibility correction | Retained |
| `b414939` | Attempt to close remaining Plan 051 gaps | Partially closed; audit residuals listed below |

---

## Correctly closed Plan 051 items

| Item | Result | Disposition |
|---|---|---|
| Legacy Light policy mapping | CLI and library use `ProtectionLevel::default_policy()`; Light maps to Unspecified | CLOSED |
| Standard legacy default | Standard maps to ProhibitedAiMlTraining | CLOSED |
| Explicit Unspecified precedence | Explicit policy is not replaced by level fallback | CLOSED |
| Missing constraints reporting | `MissingRightsConstraints` warning exists and is strict-error severity | CLOSED |
| Stale TDM emission documentation | Current output no longer claims to emit `tdm:reserve_tdm` | CLOSED |
| Basic Huffman count/value validation | Count/value equality, empty table, duplicate symbol, and oversubscription checks exist | CLOSED |
| Exact SOS Huffman references | Table-0 fallback was removed | CLOSED |
| JPEG post-scan containment | Post-scan marker segments are excluded from DCT-success classification | CLOSED |
| Duplicate top-level VP8X | Parser rejects a second VP8X chunk | CLOSED |
| Basic ICC/EXIF/XMP/animation flag derivation | Several VP8X bits now derive from copied output chunks | CLOSED, subject to Plan 052 final validator |
| Required status-file existence | Retrospective ledgers now exist | CLOSED as historical record |

---

## Residual defects found after the completion claim

| Item | Audited behavior at `b414939` | Required correction | Disposition |
|---|---|---|---|
| JPEG entropy span | Decoder receives the JPEG remainder beginning at scan data, including EOI/trailing bytes | Return exact entropy start/end offsets and decode only that slice | OPEN — Plan 052 |
| JPEG scan exhaustion | `finish_scan()` checks only partial-byte padding and does not reject unread complete entropy bytes | Prove exact byte/bit exhaustion after expected blocks | OPEN — Plan 052 |
| JPEG progress evidence | Expected MCU count is passed as both expected and observed | Track actual progress or eliminate fabricated comparison | OPEN — Plan 052 |
| Invalid DHT classes | Any nonzero table class is treated as AC | Reject classes other than 0 and 1 | OPEN — Plan 052 |
| Shared canonical Huffman representation | Encoder and decoder still construct canonical codes independently | Derive both from one checked table representation | OPEN — Plan 052 |
| RIFF declared size smaller than physical input | Only `declared_end > data.len()` is rejected | Require `declared_end == data.len()` for rewrite | OPEN — Plan 052 |
| RIFF final cursor/pad | Parser does not prove final cursor equality or final odd-chunk pad containment | Validate padded end and exact final cursor | OPEN — Plan 052 |
| VP8X-only container | VP8X itself satisfies `image_kind` and can pass without VP8/VP8L/ANMF payload | Require one valid primary payload or coherent animation | OPEN — Plan 052 |
| Duplicate/conflicting primary payloads | VP8 and VP8L indices are inventoried but multiplicity/conflicts are not rejected | Enforce bounded primary-payload rules | OPEN — Plan 052 |
| VP8X structural fields | Exact payload length and reserved bits/bytes are not validated | Validate before rewrite | OPEN — Plan 052 |
| Mixed owned/unrelated XMP | Entire packets containing any owned property are skipped | Parse every packet and filter owned fields at expanded-name field granularity | OPEN — Plan 052 |
| Malformed owned XMP | Owned packets can be skipped before parsing; string parser can return partial success | Fail the entire rewrite before output | OPEN — Plan 052 |
| VP8L intrinsic alpha | Alpha is derived only from ALPH chunks | Parse the VP8L alpha-used bit | OPEN — Plan 052 |
| ANMF frame alpha | Nested frame payloads are not inspected for ALPH or VP8L alpha | Add bounded frame alpha inspection | OPEN — Plan 052 |
| Final WebP validator | Output is not reparsed and checked for exact flags/structure before return | Add production final-container validation | OPEN — Plan 052 |
| Current-head CI evidence | Recorded pass was for an earlier SHA and current-head connector evidence was not available | Record exact final SHA evidence or mark unavailable | OPEN — Plan 052 |

---

## Evidence correction

The former ledger statement that `declared_end == data.len()` was enforced was inaccurate. At the audited head the source rejects only declarations larger than physical input.

The former entropy-exhaustion closure statement was also incomplete. The current finalizer validates some partial-byte padding but does not prove that no complete entropy bytes remain.

The former mixed-XMP closure statement was inaccurate. The current path skips entire XMP packets containing StegoEggo-owned properties, so unrelated fields in the same packet can be lost.

The former alpha closure statement covered top-level ALPH chunks only and did not cover intrinsic VP8L or ANMF frame alpha.

A local test or CI pass proves only the tested implementation. It does not supersede these source-level contract findings.

---

## Current planning disposition

| Plan | Disposition after follow-up audit |
|---|---|
| 045 | Reopened for final container closure by Plan 052 |
| 048 | JPEG preservation substantially implemented; exact entropy and shared Huffman work remain in Plan 052 |
| 049 | WebP replacement substantially implemented; strict structure, field-level XMP, and alpha work remain in Plan 052 |
| 050 | Superseded by later evidence passes |
| 051 | PARTIAL |
| 052 | Ready for implementation |

---

## Publication hold

No version bump, crates.io publication, tag, GitHub release, or release automation is part of Plan 052. Release remains manual and separate from correctness closure.
