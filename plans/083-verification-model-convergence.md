# Plan 083: Verification Model Convergence

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/081-pre-v1-consolidation-maintainability-and-portability-roadmap.md`

Audited baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`.

## 1. Objective

Make the structured `VerificationReport` model the canonical semantic result of image verification, with simpler 0.x compatibility results derived from it wherever practical. Eliminate duplicated decision logic among `VerificationStatus`, `VerificationResult`, `NoticeVerification`, and report construction.

This is a convergence plan, not a public-API removal plan.

## 2. Audit first

Before product edits, record in `plans/083-status.md` a call graph for:

- `verify_image_bytes`
- `verify_image_bytes_with_limits`
- `verify_image_bytes_detailed`
- legal-notice verification entry points
- `VerificationReportBuilder`
- CLI verification/JSON rendering
- detached-manifest verification interfaces where they consume embedded-image verification

For each, identify the code that decides: rights present/absent, hidden marker present/valid/invalid, authentication key missing/failed/valid, binding status, malformed/unsupported payload diagnostics, resource-limit failure, and final coarse status.

## 3. Canonical semantic core

Introduce or identify one private/internal verification operation that computes the richest available facts once and returns `VerificationReport` (or a private richer intermediate that is converted to `VerificationReport` exactly once if necessary for compatibility).

The canonical operation must preserve:

- existing bounded JPEG/LSB search behavior;
- legacy payload v1/v2/v3 read compatibility;
- distinction among not found, corruption, malformed v3, unsupported version, missing auth key, auth failure, and resource-limit exhaustion;
- metadata-only evidence and rights-policy interpretation;
- current HMAC/CRC semantics;
- trust/signature fields when feature-gated functionality is present.

Do not simplify by collapsing meaningful diagnostics.

## 4. Compatibility projections

Implement explicit, tested projections from the canonical facts/report to:

- `VerificationStatus`
- `VerificationResult`
- `NoticeVerification` or its builder output

If any public type contains information that cannot be faithfully reconstructed from `VerificationReport`, document the missing field and either extend the report additively or retain a narrowly scoped shared intermediate. Do not retain two independent verification searches merely to satisfy old result types.

Projection rules must be centralized and named, not open-coded at multiple call sites.

## 5. CLI/reporting alignment

Ensure CLI human-readable and JSON verification output consume the canonical structured result or a documented compatibility projection. The CLI must not re-decide verification semantics from partial fields.

If stable JSON schemas require old fields, preserve them while deriving values from canonical facts.

## 6. Tests

Build a table-driven verification matrix covering at least:

- clean protected CRC image;
- clean protected HMAC image with correct key;
- HMAC image with missing key;
- HMAC image with wrong key;
- corrupted payload;
- metadata-only image;
- no protection found;
- malformed v3 prefix/header;
- unsupported payload version;
- legacy v1/v2 fixtures;
- tiled LSB/JPEG recovery;
- resource-limit exhaustion;
- metadata stripped but surviving marker where fixtures permit.

For every case assert consistency across canonical report, `VerificationStatus`, `VerificationResult`, `NoticeVerification`, and CLI JSON fields that represent the same fact.

## 7. Documentation

Update `STABILITY.md`, `docs/rust-api.md`, verification architecture docs, and crate docs to explain:

- which verification API is canonical for rich integrations;
- which types are convenience/compatibility projections;
- that `VerificationStatus` remains stable and is not deprecated unless a separate semver decision is made;
- exact meanings of authentication, binding, trust, and marker validity.

Correct any stale module-path references discovered during the audit.

## 8. Acceptance criteria

- one verification semantic core performs the expensive extraction/classification work;
- compatibility outputs are centralized projections or a documented unavoidable exception;
- no reduction in diagnostic granularity or legacy recovery coverage;
- CLI verification semantics come from canonical facts;
- matrix tests prove cross-API consistency;
- `./scripts/check.sh` passes;
- `plans/083-status.md` records the final semantic ownership map and exceptions.

## 9. Non-goals

No C2PA, certificate trust infrastructure, new cryptographic algorithm, payload format change, or removal of stable 0.x verification APIs.
