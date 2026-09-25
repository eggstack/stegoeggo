# Milestone Implementation Plans

Bounded plans handed directly to implementation agents. Operational
documents tied to the current repository state; may be corrected,
superseded, or archived without touching `plans/000`–`002`.

## Layout and naming

```text
implementation/<subsystem>/NNN-short-title.md
```

Numbering is local to the subsystem. Flat `NNN-*.md` numbering retired
after Plan 107.

## Required template

```markdown
# <Subsystem> Milestone NNN — <Title>

Status: ready for handoff | active | blocked | implemented | superseded
Repository baseline: `<commit SHA or branch state>`
Source roadmap: `plans/subsystems/<subsystem>-roadmap.md#...`
Long-term requirements: `plans/000-long-term-specification.md#...`
Applicable ADRs: `plans/adrs/ADR-NNNN-...md`
Primary class: invariant | capability | infrastructure | polish

## 1. Objective
## 2. Why this milestone is ready
## 3. Current implementation evidence
## 4. Invariants that must not regress
## 5. Scope (in / explicitly out)
## 6. Required production changes
## 7. Ordered work packages (intent, changes, acceptance evidence)
## 8. Failure, cancellation, restart, contention semantics
## 9. Compatibility and migration
## 10. Required tests
## 11. Required verification commands (exact; `./scripts/check.sh` minimum)
## 12. Documentation updates
## 13. Acceptance criteria (externally observable)
## 14. Stop conditions (report rather than improvise)
## 15. Closure evidence required
## 16. Handoff notes
```

## Handoff rules

Confirm baseline currency, closed hard deps, ADRs-or-out-of-scope,
one-pass size, specific tests/evidence — then register in
`plans/registry.md`. Agents may adjust file mechanics but MUST NOT
weaken invariants or enlarge scope. Corrective work takes a new local
number referencing the original plan + closure.
