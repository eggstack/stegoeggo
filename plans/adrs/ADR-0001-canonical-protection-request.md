# ADR-0001: ProtectionRequest and RightsPolicy as Canonical API

Status: accepted

Date: 2026-09-25

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/001-terminology-and-domain-model.md#core-request-model`

Affected subsystem roadmaps:

- `plans/subsystems/api-cli-contract-roadmap.md`
- `plans/subsystems/rights-metadata-roadmap.md`

## Context

Before Release 4, processing features accumulated on `ProtectionLevel`,
`EvidenceProfile`, and `ProtectionContext` builders, coupling policy,
routing, and warning behavior to legacy level knobs. Predecessor evidence:
flat plans `016`, `017`, `022`, `082`.

## Decision drivers

- Policy must be explicit, never inferred from intensity/format/channels.
- One validation point (`resolve_request()`) producing an immutable plan.
- 0.x compatibility for existing callers without forking behavior.

## Considered options

### Option A — Canonical request with translating adapters

New features land on `ProtectionRequest`/`ProcessingOptions`/
`ProtectionChannels`; legacy builders translate via
`request_from_legacy()`. Benefits: single execution path, stable compat.
Costs: adapter maintenance through 0.x. Failure mode: adapter drift —
mitigated by the no-independent-legacy-features invariant.

### Option B — Parallel modern/legacy executors

Benefits: no translation layer. Costs: duplicated routing/warnings,
divergent behavior. Failure mode: silent semantic fork. Rejected.

## Decision

Option A. `ProtectionRequest` + `RightsPolicy` are canonical; legacy
builders are translation-only compatibility adapters removed at v1.0.0
at the earliest.

## Consequences

### Positive

- One pipeline (`src/pipeline.rs`) for all callers, including the CLI.
- Explicit policy eliminates intensity-inferred semantics.

### Negative

- Adapter surface must be maintained and tested through 0.x.

### Neutral or deferred

- v1 removal tracked in `DEPRECATIONS.md`; never in 0.x.

## Compatibility and migration

`ProtectionLevel`/`EvidenceProfile`/`ProtectionContext` remain
functional; `docs/migration-v0.3.md` covers the v0.3 path. No new legacy
builder behavior is permitted.

## Security and reliability implications

Policy misclassification risk moves to one audited translation point;
`ContradictoryLegalClaims` guards explicit `false` against legal metadata.

## Verification

`tests/request_api.rs`, `tests/request_aux_convergence.rs`,
`tests/plan065_legacy_compat.rs`, plus `./scripts/check.sh`.

## Supersession

None.
