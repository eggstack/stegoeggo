# Plan 107 Status: Planning-Convention Migration to CodeGG Hierarchy

Status: closed.

## Objective

Adopt the CodeGG hierarchical planning convention (`000`–`003`,
`registry.md`, `adrs/`, `subsystems/`, `implementation/`, `closure/`,
`archive/`, planning skill) without rewriting flat `001`–`106` history.
Plan 107 is the last flat-numbered plan.

## Evidence

New scaffolding (all present, links verified against existing files):

- `plans/README.md`, `plans/000-long-term-specification.md`,
  `plans/001-terminology-and-domain-model.md`,
  `plans/002-long-term-roadmap.md`, `plans/003-planning-process.md`,
  `plans/registry.md`.
- `plans/adrs/README.md` + `ADR-0001` (canonical request),
  `ADR-0002` (output-domain routing), `ADR-0003` (byte-vs-pixel),
  `ADR-0004` (verified self-update), all accepted.
- `plans/subsystems/README.md` + six 12-section roadmaps:
  `rights-metadata` (closed), `container-correctness` (closed),
  `stego-carrier` (closed), `verification-conformance` (closed),
  `api-cli-contract` (closed), `release-distribution` (active, M001
  blocked on stable B per flat Plan 106 failure policy).
- `plans/implementation/README.md`, `plans/closure/README.md`,
  `plans/archive/README.md` (queues empty; next work takes
  subsystem-local `001`).
- `.skills/planning/SKILL.md` (governance; `plan-execution` retained
  for worktree mechanics).
- AGENTS.md plans/skills pointers updated.

## Verification

- Flat `001`–`106` untouched (only new `107` files added).
- `./scripts/check.sh`: recorded below.
- Every new cross-link target confirmed to exist (canonical docs,
  ADRs, roadmaps, READMEs, skill, registry entries for 106/105/097).

## Registry updates

`plans/registry.md` created: six roadmaps registered (five closed, one
active), 106 listed under blocked work, 107 + 105 + 097 under recently
closed. Next-plan rule: subsystem-local numbers.

## Check result

`./scripts/check.sh` passed 2026-09-25: fmt, clippy `-D warnings`,
no-default-features check, full workspace `--all-features` test run
(all suites green, including soak/verification-convergence/CLI), and
`check-docs-contract.sh` (5 release targets valid).
