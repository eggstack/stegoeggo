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
| release-distribution | active | `plans/subsystems/release-distribution-roadmap.md` | M001 blocked operationally; M002 conditionally closed; M003 closed | M003 closed the consumer-owned nested-runtime panic; M001/M002 still wait for the same ordinary stable B > 0.4.2 |
| language-bindings | active | `plans/subsystems/language-bindings-roadmap.md` | M006 post-core compatibility corrective ready | Python + Node current compatibility CI red; verification-conformance M006 interface contracted; C ABI shifted to M007/M008 |

## Dependency-ready implementation plans

| Subsystem | Milestone | Status | Plan | Notes |
|---|---|---|---|---|
| verification-conformance | M006 unstructured-candidate false-positive corrective | closed | `plans/implementation/verification-conformance/006-unstructured-candidate-false-positive-corrective.md` | Closure at `plans/closure/verification-conformance/006-status.md`; absence-vs-corruption restored with Rust guards |
| language-bindings | M006 post-core compatibility corrective and requalification | ready | `plans/implementation/language-bindings/006-post-core-compatibility-corrective.md` | Verification M006 closed, releasing the closure gate; map new carrier resource-limit variant in Python/Node, reconcile descriptive metadata kind and rights-source label, then requalify both five-target matrices |
| release-distribution | M002 Eggpack producer adoption and second-consumer qualification | conditionally closed | `plans/implementation/release-distribution/002-eggpack-producer-adoption-and-second-consumer-qualification.md` | cutover landed at `3b96fae` (Eggpack pin `56ed7e7`); closure at `plans/closure/release-distribution/002-status.md`; live B > 0.4.2 evidence outstanding, shared with M001; updater-runtime code blocker resolved by closed M003 |
| release-distribution | M003 synchronous Eggup updater bridge corrective | closed | `plans/implementation/release-distribution/003-synchronous-eggup-updater-bridge-corrective.md` | outer Tokio runtime removed at `e611c91`; closure at `plans/closure/release-distribution/003-status.md`; updater rehearsal green |

## Active closure work

None.

## Blocked work

| Subsystem | Milestone | Blocker |
|---|---|---|
| release-distribution | M001 real eggfetch-to-eggfetch A→B transition (flat `plans/106-real-eggfetch-to-eggfetch-self-update-closure.md`) | No stable B newer than 0.4.2 published; plan forbids throwaway versions. Evidence recorded in `plans/106-status.md`. |
| release-distribution | M002 live second-consumer proof (WP9 first Eggpack-produced B + WP10 shared A→B) | Missing ordinary stable B > 0.4.2. The updater-runtime code blocker is resolved by closed M003; cutover/closure remains `plans/closure/release-distribution/002-status.md`. |
| language-bindings | M007 C ABI contract design | Language-bindings M006 compatibility corrective/requalification must close against a green current Python + Node contract first. Verification M006 is closed; only language M006 remains. |

## Recently closed work

- Language Bindings M005 Node binding foundation and qualification: historical
  closure remains valid at `plans/closure/language-bindings/005-status.md`
  with release-node run `36488540671` on `abc0354`. Later canonical-core
  changes made current Python/Node compatibility CI red; that post-closure
  drift is owned by verification-conformance M006 + language-bindings M006 and
  does not rewrite the original M005 evidence.
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
