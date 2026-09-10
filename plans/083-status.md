# Plan 083 Status

Status: COMPLETE

Baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`
Implementation commit: `e6a4f85` — verification model convergence.

## 1. Audit: verification call graph (pre-change)

### `verify_image_bytes(img_bytes, mac_key) -> VerificationStatus` (`src/lib.rs:1507`)
- Creates `SteganographyProtector::new()` (default limits).
- Calls `verify_payload_from_bytes_with_key` (`src/protected/steganography/verify.rs:52`).
- That calls `verify_payload_from_bytes_outcome(img, key, false)` (un-suppressed).
- Outcome mapping (single place): `Valid -> Verified`; `Invalid|MalformedV3|UnsupportedVersion|AuthKeyMissing|AuthFailed|ResourceLimitExceeded -> Invalid`; `NotFound -> NotFound`.
- Decides: hidden marker present/valid/invalid via `CandidateOutcome`; auth key missing/failed vs corruption collapsed to `Invalid`; resource-limit exhaustion collapsed to `Invalid`; no rights/metadata interpretation; no binding/trust; no diagnostics.

### `verify_image_bytes_with_limits(img, key, limits)` (`src/lib.rs:1517`)
- Same as above with `SteganographyProtector::with_resource_limits`.
- Resource limits gate tile-origin scan, fallback seeds, payload bytes via `verify_payload_from_bytes_outcome`.

### `verify_image_bytes_detailed(img, key) -> VerificationResult` (`src/lib.rs:1548`)
### `verify_image_bytes_detailed_with_limits(img, key, limits)` (`src/lib.rs:1578`)
- Creates protector (default or limited).
- Calls `verify_and_extract_raw_for_detailed` (`verify.rs:203`) which calls `verify_payload_from_bytes_outcome(img, key, true)` (suppressed unstructured candidates).
- Maps `(Verified, Some(raw))` + `parse_verified_payload` -> `Verified{payload}`; `(Invalid, Some(raw))` + parse -> `Corrupted{payload}`; else falls through.
- Falls back to `RightsMetadataProtector::extract_seed_from_image` (metadata seed, no limits in non-limits variant) -> `MetadataOnly{seed}`; else `NotFound`.
- Decides: payload version/seed/intensity via `parse_stego_payload` (V3 magic first, then V1/V2); corruption vs not-found; metadata-only evidence; resource-limit/malformed/unsupported with `None` raw collapse to metadata-or-notfound (granularity lost here).
- Diverges from `verify_image_bytes` in suppress flag (only matters with `test-seeds`) and in second metadata-seed lookup.

### Legal-notice verification (`verify_legal_notice`, `verify_legal_notice_with_limits`) (`src/lib.rs:1636,1645` -> `src/protected/notice_verification.rs:26,197`)
- Parses rights metadata per format: `extract_png_notice` / `extract_jpeg_notice` / `extract_webp_notice` + `extract_xmp_dmi_from_{png,jpeg,webp}` (and `_with_limits` variants). Produces 15 text fields, `dmi`, `tdm_reserved`, `canonical_dmi`, `legacy_dmi`, `rights_signal_kind`, metadata `seed`, `channels` (PngText/PngXmp/JpegComment/JpegIptc/JpegXmp/WebPXmp/WebPExif).
- Decides rights present via `has_notice` (any of 16 fields incl. DMI).
- Then does its own stego search: `verify_payload_from_bytes_with_key` + (if Verified with key) `extract_payload_from_bytes_with_key` (second search) to get `StegoPayload`; pushes `DctPayload`/`LsbPayload` channel on Verified; sets `authenticated = (Verified with non-empty key)`.
- Computes `evidence_strength` from `(has_notice, authenticated, channels has stego)`.
- Builds `NoticeVerification` via builder. This is a second independent stego search path (metadata parse + stego verify + stego extract = up to 2 searches) separate from the two paths above.

### `VerificationReportBuilder` (`src/verification/builder.rs:5`, `src/verification/report.rs:1003`)
- Pure data aggregation; computes `evidence_strength` via `compute_evidence_strength()` (rights.found + hidden_marker Verified + auth attempted/verified/matched) and `summary_status()` (hidden Verified->Verified, Invalid->Invalid, NotFound + rights.found->Verified else NotFound).
- `from_notice_verification` bridges legacy `NoticeVerification` -> `Report` with defaults (empty signatures/bindings, untrusted caller-owned trust, no diagnostics). Only current embedded->report bridge; detached builds its own report separately.
- No extraction; not a semantic owner pre-change.

### CLI verification/JSON (`stegoeggo-cli/src/main.rs:1660-1801`, `JsonVerifyOutput:1594`)
- Calls `verify_legal_notice(bytes, mac_key)` once.
- Human output reads `has_notice`, `copyright_holder`, `creator`, `contact`, `rights_url`, `dmi`, `canonical_dmi`, `legacy_dmi`, `has_dmi_conflict`, `tdm_reserved`, `usage_terms`, `credit_line`, `copyright_owner`, `licensor_*`, `metadata_date`, `notice_applied_at`, `protection_seed`, `stego_status`, `authenticated`, `evidence_strength`, `stego_payload`.
- JSON (`schema_version:1`) emits `copyright_holder`, `rights_url`, `ai_constraints`, `stego_status (Debug)`, `evidence_strength (Display)` from `NoticeVerification`.
- Re-decides auth presentation from `args.key.is_some()` instead of canonical auth facts: `authenticated -> Verified`, `key present but not authenticated -> Not verified`, else `Not configured`. Semantics partially re-decided from CLI args, not from verification facts alone.

### Detached-manifest embedded reference (`src/detached/verify.rs:604-677`)
- Consumes embedded-image verification via `verify_and_extract_raw_from_bytes` (suppressed outcome, single search) to get `(status, raw)`, then calls `extract_payload_from_bytes_with_key` (second search) to get parsed `StegoPayload` for version/digest checks.
- Maps to `EmbeddedReferenceStatus`: `NotProvided|Stripped|VersionMismatch|DigestMismatch|Malformed|PresentValid|AuthenticationKeyMissing|AuthenticationFailed|UnsupportedVersion`. Inspects v3 `auth_algo==2` + empty key to distinguish missing vs failed.
- Builds its own `VerificationReport` for signatures/binding/trust (instance digest, format/dimensions/file-size, Ed25519 sigs, caller trust), not reusing embedded rights/stego report. Two searches for embedded reference (raw + parsed) is the duplication to fix.

### Decision ownership (pre-change)
- Rights present/absent: `notice_verification::has_notice` (16 fields).
- Hidden marker present/valid/invalid: `verify_payload_from_bytes_outcome` -> `CandidateOutcome` -> `VerificationStatus` (three call sites, two suppress modes).
- Auth key missing/failed/valid: `classify_auth_failure` (V3 HMAC only) + `verify_payload_integrity`; collapsed to `Invalid` in coarse paths, preserved as `AuthenticationKeyMissing|AuthenticationFailed` only inside `CandidateOutcome` and detached reference mapping.
- Binding: detached `instance_digest/format/dimensions/file_size`; embedded has none (bindings default).
- Malformed/unsupported diagnostics: `classify_v3_prefix`/`validate_v3_header`/`classify_v3_probe` -> `MalformedV3|UnsupportedVersion`, collapsed to `Invalid` (coarse) or `Malformed`/`Stripped` (detached).
- Resource-limit failure: `ResourceLimitExceeded` -> `Invalid` (coarse) or `manifest_valid:false` (detached input-size gate).
- Final coarse status: `verify_payload_from_bytes_with_key` (stego-only, no rights fallback) vs `VerificationReport::summary_status` (rights fallback) — two different "final" semantics.

## 2. Plan (post-audit decision)

- Canonical owner: new `src/verification/canonical.rs` with `verify_canonical_with_limits` performing one rights parse + one `CandidateOutcome` search, building `CanonicalFacts` + `VerificationReport` exactly once.
- Projections (centralized, named): `project_status_from_canonical`, `project_result_from_canonical`, `project_notice_from_canonical`, `project_report_from_canonical`.
- `verify_image_bytes*` -> status projection; `verify_image_bytes_detailed*` -> result projection; `verify_legal_notice*` -> notice projection; new public `verify_image_bytes_report[_with_limits]` -> report clone for rich integrations.
- Detached embedded reference: reuse single-outcome raw+parse (no second full search); full detached report remains separate (documented exception: manifest-aware version/digest/binding/trust classification needs manifest context).
- CLI `--verify`: keep single `verify_legal_notice` call (now canonical-backed projection) for full legal fields; derive auth presentation from `notice.authenticated/stego_status` without re-deciding from `args.key`; JSON fields derived from same notice (documented compatibility projection over canonical facts).
- `NoticeVerification` retains fields not in `Report` (license_url, web_statement, credit_line, copyright_owner, licensor_*, metadata_date, notice_applied_at, tdm_reserved, rights_signal_kind, canonical/legacy DMI, protection_seed, full StegoPayload); documented as narrowly-scoped shared-intermediate exception: both projections come from same `CanonicalFacts`, not two searches.

## Required evidence

- [x] verification entry-point/call graph recorded
- [x] canonical rich verification semantic core identified/implemented
- [x] `VerificationStatus` projection centralized
- [x] `VerificationResult` projection centralized
- [x] `NoticeVerification` projection centralized or exception documented
- [x] CLI verification consumes canonical facts
- [x] cross-API verification matrix green
- [x] docs/stability paths corrected
- [x] `./scripts/check.sh` passes

## Final semantic ownership map

- Canonical owner: `src/verification/canonical.rs`
  - `verify_canonical` / `verify_canonical_with_limits`: one rights parse
    (`notice_verification` format helpers + `*_with_limits` XMP/DMI) plus one
    `SteganographyProtector::verify_payload_from_bytes_outcome(_, _, true)` search.
  - Builds `VerificationReport` exactly once, with diagnostics for malformed v3,
    unsupported version, missing/failed HMAC auth, corruption, resource exhaustion,
    and metadata-only cases.
  - Preserves bounded JPEG/LSB search, V1/V2/V3 read compat, HMAC/CRC semantics,
    metadata-only evidence, and rights-policy interpretation. Trust/signatures and
    bindings stay default-empty for embedded verification (detached owns those).
- Projections (centralized, named):
  - `project_status_from_canonical` -> `VerificationStatus` (stego-only; `summary_status`
    rights fallback is separate and documented)
  - `project_result_from_canonical` -> `VerificationResult`
  - `project_notice_from_canonical` -> `NoticeVerification`
  - `project_report_from_canonical` -> `VerificationReport` clone
- Public API:
  - `verify_image_bytes{,_with_limits}` -> status projection
  - `verify_image_bytes_detailed{,_with_limits}` -> result projection
  - `verify_legal_notice{,_with_limits}` -> notice projection
  - New canonical `verify_image_bytes_report{,_with_limits}` -> report for rich integrations
- CLI `--verify` (`stegoeggo-cli/src/main.rs`): single `verify_legal_notice` call
  (now canonical-backed); auth presentation derived from
  `notice.authenticated`/`stego_status`, not `args.key`; JSON `schema_version:1`
  fields derived from same notice (documented compatibility projection).
- Detached manifest (`src/detached/verify.rs`): embedded reference now uses
  `verify_and_extract_raw_for_detailed` plus `parse_verified_payload` (single search,
  no second full extraction). Full detached signature/binding/trust report remains
  manifest-aware by necessity (documented unavoidable exception).

## Exceptions (documented, no second search)

- `NoticeVerification` carries fields absent from `VerificationReport`
  (license_url, web_statement, credit_line, copyright_owner, licensor_*,
  metadata_date, notice_applied_at, tdm_reserved, rights_signal_kind,
  canonical/legacy DMI, protection_seed, full `StegoPayload`). Both projections
  come from the same `CanonicalFacts`; this is a narrowly-scoped shared-intermediate
  exception, not two searches.
- Detached verification needs manifest context (expected version/digest, instance
  digest, caller trust) and therefore builds its own report section; embedded
  stego facts still come from the single-outcome path.
- `VerificationStatus::Invalid` with no raw payload and no metadata seed projects to
  `VerificationResult::NotFound` (granularity lives in report diagnostics by design).

## Regression cases and test counts

- New `tests/verification_convergence.rs`: 13-case table
  (clean CRC, HMAC correct/missing/wrong key, corrupted, metadata-only,
  no-protection, malformed-truncated, unsupported-garbage, seed-only,
  tiled LSB, resource-limit exhaustion, stripped-marker) asserting
  report/Status/Result/Notice/CLI-JSON consistency, plus tiled-crop and JPEG-tiled
  recovery tests. Green locally (3 tests).
- Existing suites green via `./scripts/check.sh` (fmt, clippy `-D warnings`,
  no-default-features check, workspace all-features tests).

## Current-main revalidation

Revalidated after Plans 084-089 on current `main`:

- `src/verification/canonical.rs` remains the semantic owner. The public
  status, detailed result, legal-notice, and rich-report APIs all call the
  named canonical projections from one `CanonicalFacts` result.
- `stegoeggo-cli/src/verify.rs` consumes the canonical-backed
  `verify_legal_notice` projection and presents authentication from
  `NoticeVerification::authenticated()` / `stego_status()`; it does not
  independently verify or reclassify the marker.
- `src/detached/verify.rs` uses
  `verify_and_extract_raw_for_detailed` followed by
  `parse_verified_payload` for the embedded reference, preserving the
  single-search path. Its separate manifest-aware report remains required
  for digest, binding, signature, and trust decisions.
- `cargo test --all-features --test verification_convergence` passes all 3
  table-driven integration tests. The compatibility and convergence matrix
  remains green in the integrated gate recorded by Plan 088.

No discrepancy was found that would require reopening this plan.
