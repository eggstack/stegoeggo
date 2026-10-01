# Verification and Conformance Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/001-terminology-and-domain-model.md#verification-model`
- `plans/002-long-term-roadmap.md#phase-0--foundation`

Related ADRs:

- `plans/adrs/ADR-0001-canonical-protection-request.md`

## 1. Purpose and ownership boundary

Owns trust evidence: `verify_image_bytes*` family, `VerificationReport`
builder, evidence-strength computation, conformance harness
(`src/bin/stegoeggo-conformance.rs`, feature `conformance`), and
external-tool qualification. Owns `src/verification/`,
`src/protected/notice_verification.rs`, `src/conformance.rs`. Does not
own carriers or distribution.

## 2. Work classification

### Invariants

- `verify_image_bytes_report` canonical; other types are projections of
  the same single-parse/single-search facts; `VerificationStatus` live.
- `authenticated` requires found + key-matched + tag-verified; CRC-only
  markers never claim authentication.

### Capabilities

- Rich embedded reports, detached trust via caller `TrustPolicy`,
  machine-readable conformance JSON (exit 0–5).

### Infrastructure

- Report builder, conformance manifest parsing.

### Polish

- Verification polish/readiness closures.

## 3. Non-goals

Marker embedding, container shaping, CLI update mechanics.

## 4. Current state

The 0.4.2 verification/conformance line was accepted as closed, but the
subsystem is reactivated for M6 after post-closure core commit `8c89e8cb`
introduced an absence-versus-corruption regression. Standard Rust CI stayed
green while both Python and Node compatibility suites independently observed
valid unprotected images reporting hidden-marker `Invalid` rather than
`NotFound`.

M6 owns the canonical correction and Rust-level regression guards. Prior
closure evidence remains historical and is not rewritten. Existing convergence
evidence remains in `tests/verification_convergence.rs`,
`tests/verification_report_tests.rs`, `tests/conformance_*`, and
`tests/plan026_gate1_2_3_tests.rs`.

## 5. Target architecture

No change through 0.x.

## 6. Dependency graph

Rights-metadata M1 (interface: notice contract) + stego-carrier M1–M3
(interface: extract contract) → verification milestones (hard).

M1–M5 remain historically closed. Post-closure regression evidence from the
foreign-language bindings → M6 unstructured-candidate false-positive
corrective. Language-bindings M006 consumes M6's written interface contract
and cannot close until verification M6 closes.

## 7. Milestones

- M1 profiles/warnings/reporting/polish/closeout — flat `003`, `004`,
  `005`, `006`, `007`. Class: capability/polish.
- M2 external conformance + notice-verification audit — flat `009`,
  `010`. Class: invariant.
- M3 release-3 conformance + releases-1-3 closure chain — flat `018`,
  `019`, `020`, `021`, `023`. Class: capability.
- M4 payload/trust verification + correctives — flat `026`, `027`,
  `028`, `029`, `030`, `031`. Class: invariant.
- M5 verification-model convergence — flat `083`. Class: invariant.
- M6 unstructured-candidate false-positive corrective —
  `implementation/verification-conformance/006-unstructured-candidate-false-positive-corrective.md`.
  Class: invariant.

### M6 — Unstructured candidate false-positive corrective

Restore the canonical absence-versus-corruption boundary after post-closure
candidate-classification drift and add Rust-level regression guards. Ordinary
valid carriers with no credible StegoEggo marker evidence must report
`NotFound`; structured/provenance-bearing corrupted candidates remain
`Invalid`.

Implementation:
`plans/implementation/verification-conformance/006-unstructured-candidate-false-positive-corrective.md`.

The written classification contract is an interface dependency for
language-bindings M006. Verification M6 itself has no open hard dependency.

## 8. Cross-cutting requirements

Compat: report JSON schema backward-compatible per `STABILITY.md`.
Security: bounded verification budgets
(`VerificationBudgetExceeded`); constant-time HMAC compare. Docs:
`architecture/verification.md`.

## 9. Verification strategy

Convergence/report/conformance suites; specialist external-tools suite
(`#[ignore]`, run with `--ignored`) for ExifTool/xmllint claims.

## 10. Risks and decision points

M6 must distinguish unstructured fallback probes from provenance-bearing
StegoEggo candidates without hiding real structured corruption, malformed V3,
authentication failures, seed 0, or resource-limit outcomes. Blindly restoring
the deleted seed-specific suppression is not acceptable.

Without-key forgeability remains a documented limitation, not this defect.

## 11. Completion definition

The original completion definition was met for the accepted 0.4.2 state.
The subsystem returns to closed only after M6 restores the
absence-versus-corruption invariant, adds Rust-level regression guards, and
records accepted closure evidence.

## 12. Milestone status table

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M1 reporting/polish | closed | flat `005`/`006`/`007` | `019-status.md` chain | — |
| M2 external/audit | closed | flat `009`/`010` | `019-status.md` chain | — |
| M3 release-3 closure | closed | flat `018`–`021`, `023` | `019-status.md`–`024-status.md` (as present) | — |
| M4 trust correctives | closed | flat `026`–`031` | `029-status.md`–`031-status.md` (as present) | — |
| M5 model convergence | closed | flat `083` | `083-status.md` | — |
| M6 unstructured-candidate false-positive corrective | ready | `implementation/verification-conformance/006-unstructured-candidate-false-positive-corrective.md` | pending | none; regression reproduced by Python + Node compatibility CI |
