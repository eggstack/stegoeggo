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
| language-bindings | active | `plans/subsystems/language-bindings-roadmap.md` | M008 M007 symbol-inventory corrective ready | M009 blocked on M008 closure; M010 blocked on M009 closure |

## Dependency-ready implementation plans

| Subsystem | Milestone | Status | Plan | Notes |
|---|---|---|---|---|
| language-bindings | M008 M007 symbol-inventory corrective | ready | `plans/implementation/language-bindings/008-m007-symbol-inventory-corrective.md` | Correct 89→90/86→87 bookkeeping without changing signatures; materialize checked 90-symbol manifest before implementation |
| release-distribution | M002 Eggpack producer adoption and second-consumer qualification | conditionally closed | `plans/implementation/release-distribution/002-eggpack-producer-adoption-and-second-consumer-qualification.md` | cutover landed at `3b96fae` (Eggpack pin `56ed7e7`); live B > 0.4.2 evidence remains outstanding, shared with M001 |

## Active closure work

None.

## Blocked work

| Subsystem | Milestone | Blocker |
|---|---|---|
| release-distribution | M001 real eggfetch-to-eggfetch A→B transition (flat `plans/106-real-eggfetch-to-eggfetch-self-update-closure.md`) | No stable B newer than 0.4.2 published; plan forbids throwaway versions. Evidence recorded in `plans/106-status.md`. |
| release-distribution | M002 live second-consumer proof (WP9 first Eggpack-produced B + WP10 shared A→B) | Missing ordinary stable B > 0.4.2. The updater-runtime code blocker is resolved by closed M003; cutover/closure remains `plans/closure/release-distribution/002-status.md`. |
| language-bindings | M009 C ABI v1 implementation foundation | M008 must close with corrected 90-symbol manifest/evidence before implementation begins. Plan: `plans/implementation/language-bindings/009-c-abi-v1-implementation-foundation.md`. |
| language-bindings | M010 C ABI v1 cross-platform qualification | M009 implementation foundation must close before five-target qualification/stability activation. Plan: `plans/implementation/language-bindings/010-c-abi-v1-cross-platform-qualification.md`. |

## Recently closed work

- Language Bindings M007 C ABI contract design: closure recorded at
  `plans/closure/language-bindings/007-status.md` (normative
  `bindings/c/ABI-V1.md` plus `bindings/c/README.md`; design commit
  `3e3e463`; `./scripts/check.sh` green; C11/C++17 signature sketch
  smoke on Apple clang 21.0.0; no C symbols, header, or library shipped).
  A post-closure audit found a bounded count defect: the document declares
  90 callable functions while its arithmetic says 89. M008 is registered to
  correct the evidence and add a checked 90-symbol manifest before M009
  implementation.
- Language Bindings M006 post-core compatibility corrective and
  requalification: closure recorded at
  `plans/closure/language-bindings/006-status.md` against final SHA
  `d24b377` (implementation `e6c1e2b` + test-only `d24b377`; CI
  `36922556633`, `python-binding` `36922556590`, fresh `release-python`
  `36922890804` and `release-node` `36922895760` all green; no
  registry publication). M007 C ABI design is now dependency-ready.
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
