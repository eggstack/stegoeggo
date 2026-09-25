# StegoEggo Planning System

This directory separates durable direction from temporary execution planning.
Plan 107 (`plans/107-planning-convention-migration-to-codegg-hierarchy.md`)
adopted this hierarchy; it is the last flat-numbered plan.

## Canonical long-term documents

The following files define what StegoEggo is becoming and what must remain
true. They MUST NOT be edited as part of ordinary implementation work:

- `000-long-term-specification.md` — normative end-state and invariants.
- `001-terminology-and-domain-model.md` — normative language and identity model.
- `002-long-term-roadmap.md` — dependency-ordered capability roadmap.
- `003-planning-process.md` — rules for deriving and managing interim plans.

Grounding contracts live alongside them: `STABILITY.md` (stable API/CLI),
`architecture/overview.md` (39 deep-dives), `docs/` (user guides),
`DEPRECATIONS.md` (0.x removal policy). Interim plans MUST reference these
rather than copying or silently revising their requirements.

## Planning hierarchy

```text
Long-term specification and terminology
        |
        v
Architecture decision records
        |
        v
Long-term roadmap
        |
        v
Subsystem roadmaps
        |
        v
Milestone implementation plans
        |
        v
Implementation and verification
        |
        v
Closure records and archive
```

## Directory roles

- `adrs/` — durable architecture decisions. Accepted decisions are
  superseded, not rewritten.
- `subsystems/` — subsystem specifications and dependency-ordered roadmaps.
- `implementation/` — focused milestone plans handed to implementation
  agents (number local to subsystem).
- `closure/` — verification, evidence, and completion records.
- `archive/` — completed or superseded interim planning retained for
  traceability.
- `registry.md` — compact index of active roadmaps, plans, closure work,
  and dependencies.
- Flat `001`-`106` (+ `107`) files — immutable predecessor history. They
  stay in place; subsystem roadmaps index them as evidence. Never renumber
  or rewrite them; only their `-status.md` companions record facts.

## Core rule

Long-term documents state **what StegoEggo is becoming and what must remain
true**. Interim documents state **what an agent should implement next
against a specific repository baseline**.

## Planning lifecycle

1. Identify the relevant long-term specification sections and invariants.
2. Record any unresolved architectural decision in `adrs/`.
3. Create or update a subsystem roadmap in `subsystems/`.
4. Select one dependency-ready milestone.
5. Write a bounded handoff plan under `implementation/<subsystem>/`.
6. Implement and verify the milestone (`./scripts/check.sh` minimum).
7. Write a closure record under `closure/<subsystem>/`.
8. Update `registry.md` and the subsystem roadmap status.
9. Move completed or superseded interim documents to `archive/` when they
   no longer represent active work.

No milestone is complete merely because code landed. Completion requires
the closure evidence defined by its implementation plan and subsystem
roadmap.

## Required classification

Every subsystem roadmap and implementation plan MUST distinguish:

- **Invariant** — a property that must always remain true.
- **Capability** — user- or operator-visible behavior.
- **Infrastructure** — internal machinery required by capabilities.
- **Polish** — ergonomics, diagnostics, performance tuning, cleanup, docs.

Infrastructure and polish MUST NOT be presented as completed user
capability unless the user-visible acceptance criteria are satisfied.

## Naming conventions

- ADR: `adrs/ADR-NNNN-short-title.md`
- Subsystem roadmap: `subsystems/<subsystem>-roadmap.md`
- Milestone implementation plan: `implementation/<subsystem>/NNN-short-title.md`
- Closure record: `closure/<subsystem>/NNN-status.md`
- Archived document: retain its original relative structure beneath `archive/`

Flat `NNN-*.md` numbering retired after Plan 107. New milestone numbers
are local to their subsystem.

## Starting a new workstream

Begin with `subsystems/README.md`, then use the templates and rules in:

- `adrs/README.md`
- `implementation/README.md`
- `closure/README.md`

Register active work in `registry.md` before handing implementation plans
to agents. Load `.skills/planning/SKILL.md` for governance and
`.skills/plan-execution/SKILL.md` for worktree execution mechanics.
