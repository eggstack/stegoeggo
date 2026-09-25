# Plan 107: Adopt CodeGG-Style Planning Hierarchy

## Goal

Transition `stegoeggo` planning from the flat sequential convention
(`plans/NNN-name.md` + `plans/NNN-status.md`, highest so far 106) to the
CodeGG hierarchical convention reviewed at `/Users/davidbowman/projects/codegg`
(`plans/000-003` canonical docs, `plans/registry.md` control surface,
`plans/adrs/`, `plans/subsystems/`, `plans/implementation/<subsystem>/`,
`plans/closure/<subsystem>/`, `plans/archive/`, plus a planning skill),
without rewriting history. Plan 107 is the last flat-numbered plan; all new
work after it uses the new hierarchy.

## Review findings (codegg vs stegoeggo)

CodeGG (`plans/README.md`, `plans/003-planning-process.md`,
`.skills/planning/SKILL.md`):

- Four canonical long-term docs (`000` specification, `001` terminology,
  `002` roadmap, `003` process) that are stable and never edited for
  implementation convenience.
- Interim hierarchy: ADRs (immutable once accepted) -> subsystem roadmaps
  (stable per workstream) -> milestone implementation plans (bounded handoff,
  number local to subsystem) -> closure records (evidence gate) -> archive.
- `plans/registry.md` is the compact control surface with a fixed status
  vocabulary (`proposed`, `ready`, `active`, `blocked`, `closing`, `closed`,
  `conditionally closed`, `superseded`, `archived`).
- Every item carries one work class (`invariant`, `capability`,
  `infrastructure`, `polish`); infrastructure/polish must not be presented
  as completed capability.
- Corrective passes are new plans, never amendments pretending the original
  succeeded. Closure requires requirement-to-evidence matrices, not commit
  messages.

Stegoeggo today:

- 180 files flat in `plans/`: `001`–`106` plus 73 `-status.md` companions;
  historical plans immutable except their status file (AGENTS.md).
- No canonical long-term docs, no registry, no ADRs, no subsystem grouping,
  no work classification, no closure-evidence template.
- `.skills/plan-execution/SKILL.md` covers worktree-parallel execution only;
  there is no planning-governance skill.
- Strengths to preserve: immutability rule, status companions as
  authoritative record, `architecture/` deep-dives, `STABILITY.md` contracts.

## Decision

Adapter, not transplant: keep CodeGG's document classes, status vocabulary,
lifecycle, and closure bar, but ground canonical content in stegoeggo's
existing contracts (`STABILITY.md`, `architecture/overview.md`, `docs/`,
`DEPRECATIONS.md`). Flat `001`–`106` remain in place as immutable
predecessor evidence; each new subsystem roadmap indexes the flat plans it
supersedes. Nothing is moved or rewritten.

## Scope

In scope:

- `plans/README.md`, `plans/000-003`, `plans/registry.md`.
- `plans/adrs/README.md` + ADR-0001–ADR-0004 (canonical request API,
  output-domain routing, byte-vs-pixel paths, verified self-update).
- `plans/subsystems/README.md` + six roadmaps: `rights-metadata`,
  `container-correctness`, `stego-carrier`, `verification-conformance`,
  `api-cli-contract`, `release-distribution`.
- `plans/implementation/README.md`, `plans/closure/README.md`,
  `plans/archive/README.md` (empty active queues; 106 stays flat until it
  closes).
- `.skills/planning/SKILL.md` (governance) alongside the existing
  `plan-execution` skill (execution mechanics).
- AGENTS.md plans/skills pointers.
- `plans/107-status.md` closeout evidence.

Out of scope:

- Moving, renaming, or editing flat `001`–`106` content.
- Re-closing already-closed milestones under the new templates.
- Migrating Plan 106 itself (it stays flat and blocked until release B
  exists, per its failure policy).
- Expanding required CI (`scripts/check.sh` unchanged).

## Workstream A: Canonical docs (000-003) + README

Write concise stegoeggo-grounded docs (not CodeGG-length rewrites):

- `000`: end-state invariants (metadata primary / stego best-effort
  redundant, not DRM; byte paths carry metadata; output-domain routing;
  verification projections; CLI exit codes; manual releases).
- `001`: terminology map (notice, policy, request/plan, marker modes,
  V3/V2/V1, reports, DMI/PLUS, seed hint, tiled/framed/prepared).
- `002`: dependency-ordered roadmap; all subsystems closed except
  `release-distribution` (blocked on 106/release B) and v1 deprecation
  removal tracked in `DEPRECATIONS.md`.
- `003`: governance adapted from CodeGG (classes, statuses, lifecycle,
  closure bar) plus stegoeggo specifics (`check.sh`, `forbid(unsafe_code)`,
  no-comments, byte/pixel footgun, flat-history rule).

## Workstream B: Registry

`plans/registry.md` with active roadmaps table, dependency-ready plans
(106 blocked entry pointing at the flat file), blocked work, and recently
closed work. Links only, no duplicated requirements.

## Workstream C: ADRs

Four accepted ADRs recording already-implemented durable decisions with
code evidence pointers. Numbers `ADR-0001`–`ADR-0004`, never reused.

## Workstream D: Subsystem roadmaps

Six roadmaps following the 12-section template. Each states purpose,
classification, non-goals, current state, target, dependency graph,
milestones (all marked closed with predecessor flat-plan links except
release-distribution M001 blocked), verification strategy, risks,
completion definition, and status table.

## Workstream E: Implementation / closure / archive READMEs

Templates copied from CodeGG structure with stegoeggo verification
commands (`./scripts/check.sh`, specialist scripts explicitly out).
No active milestone entries yet; next new work takes
`implementation/<subsystem>/001-*` (numbers local to subsystem; the flat
107 sequence is retired after this plan).

## Workstream F: Planning skill + AGENTS.md

New `.skills/planning/SKILL.md` (when to load, hierarchy, lifecycle,
templates, anti-patterns, stegoeggo specifics). AGENTS.md: point plans
to `plans/registry.md` + `plans/README.md`, list both skills, note that
107 is the last flat plan and historical flat files stay immutable.

## Workstream G: Closeout

Populate `plans/107-status.md`, run `./scripts/check.sh`, verify new
markdown links resolve to files that exist.

## Acceptance criteria

- `plans/README.md`, `000`, `001`, `002`, `003`, `registry.md` exist and
  cross-link without dangling references.
- `adrs/` has README + 4 accepted ADRs; `subsystems/` has README + 6
  roadmaps each with 12 sections + predecessor flat-plan index.
- `implementation/`, `closure/`, `archive/` each have a README.
- `.skills/planning/SKILL.md` exists; AGENTS.md references it and the
  registry; next-plan numbering rule names subsystem-local numbers.
- Flat `001`–`106` untouched (except new `107` files).
- `./scripts/check.sh` passes (docs-only change; no Rust touched).
- `plans/107-status.md` records evidence.

## Failure policy

If verification finds dangling links or misclassified milestones, fix the
new scaffolding in place (it is not yet historical). Do not edit flat
history to accommodate the new hierarchy. If a subsystem boundary proves
wrong, revise the roadmap with a recorded reason rather than silently
re-sorting predecessor links.
