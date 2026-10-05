# Maintenance Quality Roadmap

Status: closed

Repository baseline reviewed: `0a8b356a056bdab2f9d1996be78578cb3f6bc724`

Long-term references:

- `plans/000-long-term-specification.md#5-release-invariants`
- `plans/003-planning-process.md#10-verification-bar`

Related ADRs:

- `plans/adrs/ADR-0004-verified-self-update.md`
- `plans/adrs/ADR-0005-foreign-language-bindings.md`
- `plans/adrs/ADR-0006-versioned-c-abi.md`

## 1. Purpose and ownership boundary

Owns cross-cutting repository maintenance that is not product capability: workflow
duplication, build/bootstrap consistency, immutable third-party action references,
drift guards, and agent guidance (`AGENTS.md`, `.skills/`, `architecture/` index)
currency. It does not change release authority, supported platforms, public API,
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

M001 is closed. The affected Python/Node/C release workflows now pin third-party
actions to full commit SHAs, and verified Zig 0.14.1 plus cargo-zigbuild 0.23.3 setup
used by the C release flow is centralized in local composite actions. The qualified
`release-c` run recorded 11/11 green jobs, and no publication behavior changed.

`release-binaries.yml` remains intentionally large and was not refactored: its
Eggpack-produced byte/workflow shape is owned by the release-distribution contract, so
M001 stopped rather than obscuring or duplicating that ownership.

The only remaining item in this workstream is optional M002 workflow-contract/drift
linting. It is proposed, not required for closure, and requires a maintainer decision
before any new check is placed in required CI.

M003 is closed. It restored agent-guidance accuracy after the language-bindings and
carrier-surface work outgrew `AGENTS.md` and `.skills/`: it added a `bindings` skill for
the previously undocumented FFI subsystem, corrected `AGENTS.md`'s CI topology (four more
workflows run on push/PR than the old text claimed), corrected drifted counts, and added a
change-area → deep-dive index. No product code, public API, or CI placement changed.

M004 is closed. Executing the README quickstart — rather than only reading it —
exposed a user-facing CLI bug: every bare relative `-o` output path failed because
`Path::parent()` returns `Some("")` for a bare name, defeating the `unwrap_or_else`
fallback in `check_input_output_disjoint`. The CLI test suite only used absolute
`tempfile` paths, so CI could not see it. Fixed with three regression tests (one
confirmed to fail against the pre-fix code), and the README was restructured
quickstart-first with detail delegated to `docs/`.

M005 is closed. It removed the release-contract temp-state flake that M003 first
reported and M004 re-deferred. The test asserted on the shared system temp
directory, so a concurrently running sibling validator's live `stegoeggo-release-smoke-*`
directory was counted as this test's residue; the racer was proven by per-sibling
pairing, and the pre-fix failure rate was 8/60 at default harness parallelism.
The test now observes a private temp root through `TMPDIR` redirection, which
needed no change to the production validator and left the count assertion intact.
Re-verifying the M004 README by execution produced one example fix and confirmed
the rest, including the preset table.

## 5. Target architecture

Keep top-level workflows separate by evidence purpose, but factor repeated step-level
bootstrap into local composite actions/scripts and repeated whole-job mechanics into
same-repository reusable workflows only where the logging/runner model remains clear.

Every external action reference in release/binding workflows is pinned to a verified
full commit SHA with an adjacent human-readable version comment or update record.

## 6. Dependency graph

M001 is closed and remained independent of release-distribution's operational stable-B
blocker; its implementation did not alter release-distribution evidence semantics.

M002 has its M001 precondition satisfied but remains proposed optional polish. No
implementation plan exists, and no workflow-contract lint may be added to required CI
without a maintainer decision.

## 7. Milestones

- **M001 — Release/binding workflow consolidation and immutable action pinning.**
  Class: infrastructure.
- **M002 — Workflow contract/drift linting.**
  Class: polish, future; only after M001 closure and maintainer review of whether any
  lint belongs in required CI.
- **M003 — Agent guidance and documentation currency.**
  Class: infrastructure; closed. Keeps `AGENTS.md`, `.skills/`, and the
  `architecture/` index accurate as subsystems are added.
- **M004 — CLI relative-path fix and README quickstart verification.**
  Class: bug fix + documentation; closed. Running the documented quickstart
  exposed a bare-relative-output-path failure in the CLI; fixed with regression
  tests, and the README was restructured quickstart-first.
- **M005 — Release-contract test isolation and README re-verification.**
  Class: bug fix (test isolation) + documentation; closed. The
  `validator_temp_state_is_cleaned` flake on the required gate was diagnosed to
  a shared-temp-directory assertion, fixed by observing a private temp root, and
  proven both race-free and still sensitive to a real leak.

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
| M001 workflow consolidation/pinning | closed | `plans/implementation/maintenance-quality/001-release-workflow-consolidation.md` | `plans/closure/maintenance-quality/001-status.md` | none |
| M002 drift linting | proposed | none | none | maintainer decision on CI placement (M001 precondition satisfied) |
| M003 agent guidance and doc currency | closed | `plans/implementation/maintenance-quality/003-agent-guidance-and-doc-currency.md` | `plans/closure/maintenance-quality/003-status.md` | none |
| M004 CLI relative-path fix + README verification | closed | `plans/implementation/maintenance-quality/004-cli-relative-path-and-readme-verification.md` | `plans/closure/maintenance-quality/004-status.md` | none |
| M005 release-contract test isolation + README re-verification | closed | `plans/implementation/maintenance-quality/005-release-contract-test-isolation-and-readme-reverification.md` | `plans/closure/maintenance-quality/005-status.md` | none |
