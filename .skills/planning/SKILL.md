---
name: planning
description: StegoEggo's plans/ planning process — creating subsystem roadmaps, milestone implementation plans, closure records, ADRs, and registering them in registry.md
---

# Planning Process Skill

StegoEggo planning separates **canonical long-term direction** (stable)
from **interim planning** (operational, evolves with the codebase).
Implementation agents MUST NOT silently rewrite `000`–`002`.

Flat `plans/001`–`107` are immutable predecessor history (only `-status.md`
companions record facts). All new work uses the hierarchy below.

## When to Load

Load this skill when creating a subsystem roadmap, writing a milestone
implementation plan, writing a closure record, updating `registry.md`,
writing an ADR, or archiving interim docs.

For worktree-parallel execution mechanics, also load
`.skills/plan-execution/SKILL.md`. For Rust conventions, load
`.skills/stegoeggo-conventions/SKILL.md`.

## Document Hierarchy

```text
Long-term specification and terminology (000, 001 — canonical, stable)
        |
        v
Architecture decision records (adrs/ — durable, supersede don't rewrite)
        |
        v
Long-term roadmap (002 — canonical sequencing)
        |
        v
Subsystem roadmaps (subsystems/ — stable per workstream)
        |
        v
Milestone implementation plans (implementation/<subsystem>/ — bounded handoff)
        |
        v
Implementation and verification (./scripts/check.sh minimum)
        |
        v
Closure records (closure/<subsystem>/ — evidence gate)
        |
        v
Archive (archive/ — historical traceability)
```

Authority order on conflict: `000`/`001` > accepted ADRs > subsystem
roadmap > milestone plan > repository evidence. Preserve invariants,
record the discrepancy, make the smallest coherent adjustment.

## Document Classes

| Class | Location | Stability |
|---|---|---|
| Canonical long-term | `plans/000-`, `001-`, `002-`, `003-` | Very stable; ADR or user direction to amend |
| ADR | `plans/adrs/ADR-NNNN-*.md` | Immutable once accepted |
| Subsystem roadmap | `plans/subsystems/<name>-roadmap.md` | Stable per workstream |
| Milestone plan | `plans/implementation/<subsystem>/NNN-*.md` (local numbers) | Operational |
| Closure record | `plans/closure/<subsystem>/NNN-status.md` (same number) | Immutable except facts |
| Archive | `plans/archive/` (preserve structure, `git mv`) | Historical only |

## Status Vocabulary

`proposed` | `ready` | `active` | `blocked` | `closing` | `closed` |
`conditionally closed` | `superseded` | `archived`. See `registry.md`.

## Work Classification

One primary class per item: **invariant** (guards/property tests),
**capability** (end-to-end acceptance), **infrastructure** (never
presented as completed capability without a consumer path), **polish**
(follows correctness closure).

## Lifecycle

1. Identify canonical sections in `000`/`001`/`002`.
2. Record open architecture decisions as ADRs.
3. Create/update the subsystem roadmap (12 sections; index flat
   predecessors, don't duplicate them).
4. Select one dependency-ready milestone (hard deps closed, interface
   deps contracted).
5. Write a bounded handoff plan (16 sections per
   `implementation/README.md`).
6. Register it in `registry.md` (`ready` → `active` on start).
7. Implement + verify (`./scripts/check.sh` minimum; exact commands
   recorded).
8. Write the closure record (12 sections per `closure/README.md`).
9. Update `registry.md` + roadmap status; audit blocked work for
   newly-satisfied blockers in the same commit.
10. Archive superseded interim docs when inactive.

A milestone is complete only when its closure evidence is accepted. A
"closed" commit message is not evidence. Corrective passes are NEW plans
in the same subsystem, never amendments.

## Stegoeggo-Specific Rules

- Byte APIs when metadata matters (ADR-0003); output-domain routing
  (ADR-0002); canonical `ProtectionRequest` first, never extend legacy
  builders (ADR-0001).
- `#![forbid(unsafe_code)]`, no code comments unless asked, 4-space
  indent, max width 100, `#[must_use]` on builders.
- Specialist checks (external conformance, docs-rs, MSRV package, fuzz,
  deny, semver) are manual — cite only when actually run.
- Never edit flat `001`–`107` content; never expand required CI without
  a maintainer decision.

## Anti-Patterns (prohibited)

Transient TODOs in `000`–`002`; unbounded handoffs; compilation-as-closure;
infrastructure-as-capability; per-subsystem terminology drift; plans
overriding ADRs; stale active plans; success-only evidence; polish before
correctness; throwaway stable releases to satisfy evidence gates.
