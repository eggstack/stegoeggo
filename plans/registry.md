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
| release-distribution | active | `plans/subsystems/release-distribution-roadmap.md` | M001 blocked operationally; M002 conditionally closed | M001 waits for ordinary stable B > 0.4.2; M002 cutover landed, live B evidence outstanding on that same future release |
| language-bindings | active | `plans/subsystems/language-bindings-roadmap.md` | M005 Node closed | M001–M005 closed; M006 C ABI design dependency-ready |

## Dependency-ready implementation plans

| Subsystem | Milestone | Status | Plan | Notes |
|---|---|---|---|---|
| language-bindings | M005 Node binding foundation and qualification | closed | `plans/implementation/language-bindings/005-node-binding-foundation-qualification.md` | closure at `plans/closure/language-bindings/005-status.md`; release-node run 36488540671 green; no npm publication |
| release-distribution | M002 Eggpack producer adoption and second-consumer qualification | conditionally closed | `plans/implementation/release-distribution/002-eggpack-producer-adoption-and-second-consumer-qualification.md` | cutover landed at `3b96fae` (Eggpack pin `56ed7e7`); closure at `plans/closure/release-distribution/002-status.md`; live B > 0.4.2 evidence outstanding, shared with M001; pre-existing updater rehearsal panic recorded there |

## Active closure work

None.

## Blocked work

| Subsystem | Milestone | Blocker |
|---|---|---|
| release-distribution | M001 real eggfetch-to-eggfetch A→B transition (flat `plans/106-real-eggfetch-to-eggfetch-self-update-closure.md`) | No stable B newer than 0.4.2 published; plan forbids throwaway versions. Evidence recorded in `plans/106-status.md`. |
| release-distribution | M002 live second-consumer proof (WP9 first Eggpack-produced B + WP10 shared A→B) | Same missing ordinary stable B > 0.4.2; pre-existing updater rehearsal panic (baseline-proven) must also be resolved before WP10 runs green. Cutover/closure in `plans/closure/release-distribution/002-status.md`. |

## Recently closed work

- Language Bindings M004 M003 closure corrective pass: closure recorded
  at `plans/closure/language-bindings/004-status.md`. Lightweight
  `python-binding` CI corrected to a self-contained wheel
  build + install path (run `36456938690`, success, 71 Python tests
  collected); release qualification identity corrected to the
  GitHub-recorded SHA `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5` for
  run `36337194059`; all five SUPPORT wheel rows promoted to
  `Qualified`; M003 closure accepted and its stale claims removed.
- Language Bindings M003 Python corrective qualification: closure
  recorded at `plans/closure/language-bindings/003-status.md`
  (corrected by M004; corrections listed in its §13). Structured Python
  exception attributes (InsufficientCapacity + every ResourceLimit
  variant; ImageTruncated mapped to EncodeDecodeError), failure-safe
  pure-Python file helpers with optional `output_path`, lightweight
  path-filtered python-binding workflow, corrected five-platform manual
  wheel workflow with Rust provisioned inside the cibuildwheel Linux
  container and native-arch smokes, single-sdist/direct-pip-install
  proof, documentation/registry reconciliation. Five matrix wheels
  qualified natively in release run `36337194059`.
- Language Bindings M002 Python packaging and qualification: closure
  recorded at `plans/closure/language-bindings/002-status.md`.
  Conditionally closed at M002 publication; the outstanding
  four-platform native-smoke condition is satisfied by M003's evidence,
  accepted in `plans/closure/language-bindings/004-status.md` §11. The
  M002 record itself is unchanged.
- Language Bindings M001 Python binding foundation: closure recorded at
  `plans/closure/language-bindings/001-status.md`. PyO3/maturin
  leaf binding over canonical byte APIs, 57 Python tests, panic
  profile `unwind`, required `./scripts/check.sh` unchanged.
- Plan 107 planning-convention migration (this registry, `000`–`003`,
  ADRs, six original roadmaps, planning skill): `plans/107-status.md`.
- First eggfetch-enabled release 0.4.2 five-target qualification:
  `plans/105-status.md`.
- Carrier v1 packaging/documentation/evidence closure:
  `plans/097-status.md`.
- Roadmap-081 corrective closure: `plans/088-status.md`.
- Fuzz assurance corrective closure: `plans/089-status.md`.
