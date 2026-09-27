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
| release-distribution | active | `plans/subsystems/release-distribution-roadmap.md` | M001 blocked | stable release B newer than 0.4.2 (flat Plan 106 failure policy: no throwaway version) |
| language-bindings | active | `plans/subsystems/language-bindings-roadmap.md` | M003 closed (M004 ready to plan) | M001 closed; M002 closed (condition satisfied by M003); M003 closed; M004 ready to plan from the final Python contract |

## Dependency-ready implementation plans

None. (M003 is closed and M004 is ready to be planned under
`plans/implementation/language-bindings/`.)

## Active closure work

None.

## Blocked work

| Subsystem | Milestone | Blocker |
|---|---|---|
| release-distribution | M001 real eggfetch-to-eggfetch A→B transition (flat `plans/106-real-eggfetch-to-eggfetch-self-update-closure.md`) | No stable B newer than 0.4.2 published; plan forbids throwaway versions. Evidence recorded in `plans/106-status.md`. |

## Recently closed work

- Language Bindings M003 Python corrective qualification: closure
  recorded at `plans/closure/language-bindings/003-status.md`.
  Structured Python exception attributes (InsufficientCapacity +
  every ResourceLimit variant; ImageTruncated mapped to
  EncodeDecodeError), failure-safe pure-Python file helpers with
  optional `output_path`, lightweight path-filtered python-binding
  workflow, corrected five-platform manual wheel workflow with
  Rust provisioned inside the cibuildwheel Linux container and
  native-arch smokes, single-sdist/direct-pip-install proof,
  documentation/registry reconciliation. Five matrix wheels
  qualified natively; M002's outstanding qualification condition
  satisfied.
- Language Bindings M002 Python packaging and qualification: closure
  recorded at `plans/closure/language-bindings/002-status.md`.
  Conditionally closed at M002 publication; M003's evidence
  satisfies the outstanding four-platform native-smoke condition,
  closing M002 as part of the M003 closure per the M003 disposition.
- Language Bindings M001 Python binding foundation: closure recorded at
  `plans/closure/language-bindings/001-status.md`. PyO3/maturin
  leaf binding over canonical byte APIs, 57 Python tests (now 71
  in M003+), panic profile `unwind`, required `./scripts/check.sh`
  unchanged.
- Plan 107 planning-convention migration (this registry, `000`–`003`,
  ADRs, six original roadmaps, planning skill): `plans/107-status.md`.
- First eggfetch-enabled release 0.4.2 five-target qualification:
  `plans/105-status.md`.
- Carrier v1 packaging/documentation/evidence closure:
  `plans/097-status.md`.
- Roadmap-081 corrective closure: `plans/088-status.md`.
- Fuzz assurance corrective closure: `plans/089-status.md`.
