# StegoEggo Active Planning Registry

Compact control surface for active interim planning. Detailed requirements
stay in roadmaps, implementation plans, `closure/`, and Git history.

Canonical direction: `plans/000-long-term-specification.md`,
`plans/001-terminology-and-domain-model.md`,
`plans/002-long-term-roadmap.md`, `plans/003-planning-process.md`.
System guide: `plans/README.md`. Flat `001`–`107` are immutable
predecessor history, indexed by the roadmaps below.

## Status vocabulary

- **proposed** — exists but not approved for execution.
- **ready** — dependencies satisfied; may be handed off.
- **active** — implementation or closure work in progress.
- **blocked** — a named dependency or evidence requirement prevents progress.
- **closing** — implementation landed; closure evidence being gathered.
- **closed** — closure record accepted.
- **conditionally closed** — substantial work landed, named evidence outstanding.
- **superseded** — replaced by another document.
- **archived** — inactive; retained for traceability.

## Active subsystem roadmaps

| Subsystem | Status | Roadmap | Current milestone | Dependencies or blockers |
|---|---|---|---|---|
| rights-metadata | closed | `plans/subsystems/rights-metadata-roadmap.md` | all milestones closed | none |
| container-correctness | closed | `plans/subsystems/container-correctness-roadmap.md` | all milestones closed | none |
| stego-carrier | closed | `plans/subsystems/stego-carrier-roadmap.md` | all milestones closed | none |
| verification-conformance | closed | `plans/subsystems/verification-conformance-roadmap.md` | all milestones closed | none |
| api-cli-contract | closed | `plans/subsystems/api-cli-contract-roadmap.md` | all milestones closed | none |
| release-distribution | active | `plans/subsystems/release-distribution-roadmap.md` | M004 0.4.3 release ready; M001 blocked pending public B; M002 conditionally closed; M003 closed | M004 is the ordinary stable B handoff intended to supply both M001 and M002 live evidence |
| language-bindings | active | `plans/subsystems/language-bindings-roadmap.md` | M001–M010 complete; M011 closed | All five Python wheel builds and smokes plus sdist build/install smoke passed in artifact run 44565805002; PR #4 remains open/unmerged to land macos-15 runner correction; independent of root-workspace M006 |
| stego-library-evolution | active | `plans/subsystems/stego-library-evolution-roadmap.md` | M001–M005 and M008 closed; M006 blocked; M007 proposed | M006 waits only for ADR-0007 acceptance |
| maintenance-quality | active | `plans/subsystems/maintenance-quality-roadmap.md` | M001, M003–M006 closed; M002 proposed optional polish | M006 required `.github/workflows/ci.yml` Check passed on candidate SHA `7f5f6a1`; M002 remains optional behind maintainer CI-placement decision |

## Dependency-ready implementation plans

| Subsystem | Milestone | Status | Plan | Notes |
|---|---|---|---|---|
| release-distribution | M002 Eggpack producer adoption and second-consumer qualification | conditionally closed | `plans/implementation/release-distribution/002-eggpack-producer-adoption-and-second-consumer-qualification.md` | cutover landed at `3b96fae` (Eggpack pin `56ed7e7`); live B > 0.4.2 evidence remains outstanding, shared with M001 |
| release-distribution | M004 0.4.3 release and planning reconciliation | ready | `plans/implementation/release-distribution/004-0.4.3-release-and-planning-reconciliation.md` | ordinary stable release; produces first live Eggpack B and public 0.4.2→0.4.3 updater evidence |
| maintenance-quality | M006 root workspace dependency security refresh | closed | `plans/implementation/maintenance-quality/006-root-workspace-dependency-security-refresh.md` | `plans/closure/maintenance-quality/006-status.md`; required Check passed on candidate SHA `7f5f6a1` |
| language-bindings | M011 binding dependency security refresh | closed | `plans/implementation/language-bindings/011-binding-dependency-security-refresh.md` | `plans/closure/language-bindings/011-status.md`; five-target artifact run 44565805002 and all wheel/sdist smokes passed; PR #4 open/unmerged; no package publication |

## Active closure work

No active closure work.

## Blocked work

| Subsystem | Milestone | Blocker |
|---|---|---|
| release-distribution | M001 real eggfetch-to-eggfetch A→B transition (flat `plans/106-real-eggfetch-to-eggfetch-self-update-closure.md`) | Public 0.4.3 not yet published; ready M004 is the authorized ordinary stable B release. Evidence remains in `plans/106-status.md` until the live transition runs. |
| release-distribution | M002 live second-consumer proof (WP9 first Eggpack-produced B + WP10 shared A→B) | Live 0.4.3 Eggpack publication/update evidence not yet recorded; ready M004 owns that shared release event. |
| stego-library-evolution | M006 keyed carrier placement | ADR-0007 acceptance only. |

## Recently closed work

- Maintenance Quality M003–M005: `plans/closure/maintenance-quality/003-status.md`–`005-status.md`.
- Stego Library Evolution M008: `plans/closure/stego-library-evolution/008-status.md`.
- Language Bindings M010: `plans/closure/language-bindings/010-status.md`.
- Language Bindings M011: `plans/closure/language-bindings/011-status.md`; five-target Python wheel and smoke qualification passed in run 44565805002.
- Release Distribution M003: `plans/closure/release-distribution/003-status.md`.

Older closure evidence remains in `plans/closure/`, subsystem roadmaps, and flat predecessor status records.
