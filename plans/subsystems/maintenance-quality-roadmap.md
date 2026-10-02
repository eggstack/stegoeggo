# Maintenance Quality Roadmap

Status: active

Repository baseline reviewed: `d5425d3c73e634fd2276d44507fa94728a18784f`

Long-term references:

- `plans/000-long-term-specification.md#5-release-invariants`
- `plans/003-planning-process.md#10-verification-bar`

Related ADRs:

- `plans/adrs/ADR-0004-verified-self-update.md`
- `plans/adrs/ADR-0005-foreign-language-bindings.md`
- `plans/adrs/ADR-0006-versioned-c-abi.md`

## 1. Purpose and ownership boundary

Owns cross-cutting repository maintenance that is not product capability: workflow
duplication, build/bootstrap consistency, immutable third-party action references, and
drift guards. It does not change release authority, supported platforms, public API,
or publication policy.

## 2. Work classification

### Invariants

- Manual-only publication remains unchanged.
- Existing five-target release/binding support matrices remain unchanged unless a
  separate support decision says otherwise.
- Required CI remains `scripts/check.sh`; this roadmap does not silently add a new
  required check.
- Release artifacts and binding smoke tests retain their current evidence meaning.

### Infrastructure

- Shared workflow/bootstrap components.
- Immutable third-party action references.
- Machine-checkable workflow contract inventory.

### Polish

- Smaller workflow files and one source of truth for repeated toolchain/platform setup.

## 3. Non-goals

Carrier/API changes, updater semantics, new release targets, automated publication,
organization-wide GitHub policy changes.

## 4. Current state

The repo has strong dedicated workflows for CI, assurance, fuzzing, external
verification, binary release staging, and Python/Node/C binding qualification.

Maintenance debt remains:

- `release-binaries.yml` is very large and carries reusable build/bootstrap logic.
- Python/Node/C release workflows repeat checkout/toolchain/artifact patterns and Linux
  platform setup.
- Some workflows use moving action tags such as `actions/checkout@v4`,
  `actions/setup-python@v5`, `actions/setup-node@v4`, and
  `actions/upload-artifact@v4`, while the Eggpack workflow already demonstrates
  immutable SHA pinning for several actions.
- The same five platform identities recur across binary, binding, and C ABI
  qualification but are expressed independently.

GitHub's current Actions documentation recommends reusable workflows/composite actions
to avoid duplication and states that full-length commit SHA pinning is the immutable
reference form.

Research reviewed 2026-10-02:
- https://docs.github.com/en/actions/concepts/workflows-and-actions/reusing-workflow-configurations
- https://docs.github.com/en/actions/reference/security/secure-use

## 5. Target architecture

Keep top-level workflows separate by evidence purpose, but factor repeated step-level
bootstrap into local composite actions/scripts and repeated whole-job mechanics into
same-repository reusable workflows only where the logging/runner model remains clear.

Every external action reference in release/binding workflows is pinned to a verified
full commit SHA with an adjacent human-readable version comment or update record.

## 6. Dependency graph

M001 is dependency-ready and independent of release-distribution's operational stable-B
blocker. It must not alter M001/M002 release-distribution evidence semantics.

Future M002 may add automated drift linting after M001 proves the shared contract, but
no implementation plan exists yet.

## 7. Milestones

- **M001 — Release/binding workflow consolidation and immutable action pinning.**
  Class: infrastructure.
- **M002 — Workflow contract/drift linting.**
  Class: polish, future; only after M001 closure and maintainer review of whether any
  lint belongs in required CI.

## 8. Cross-cutting requirements

No secrets are added to reusable components. Local composite actions receive explicit
inputs and do not perform publication. Pinned SHAs must be verified against the intended
upstream action release. Cross-platform shell assumptions remain explicit.

## 9. Verification strategy

Syntax validation, manual-dispatch dry/qualification paths where feasible, exact
before/after job matrix comparison, and `./scripts/check.sh`. No publication occurs.

## 10. Risks and decision points

Over-abstraction can make workflow logs harder to diagnose. Prefer repeated step
extraction over hiding target-specific build logic. Reusable workflows are appropriate
for whole jobs; composite actions for repeated steps.

## 11. Completion definition

M001 closed with equivalent matrices/smokes and immutable action references. M002 is
optional future polish and does not block roadmap closure.

## 12. Milestone status table

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 workflow consolidation/pinning | ready | `plans/implementation/maintenance-quality/001-release-workflow-consolidation.md` | pending | none |
| M002 drift linting | proposed future | none | none | M001 + maintainer decision on CI placement |
