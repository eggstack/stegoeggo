# Release and Distribution Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#4-cli-invariants`
- `plans/000-long-term-specification.md#5-release-invariants`
- `plans/001-terminology-and-domain-model.md#distribution`
- `plans/002-long-term-roadmap.md#phase-4--release-and-distribution`

Related ADRs:

- `plans/adrs/ADR-0004-verified-self-update.md`

## 1. Purpose and ownership boundary

Owns everything from a green `main` to an installed binary: required CI
gating, pre-release checks, binary asset matrix + sidecars, bootstrap
installers, `version`/`update` behavior, crates.io + GitHub release
mechanics. Owns `scripts/release-*.sh`, `scripts/test-release-*.sh`,
`packaging/`, `stegoeggo-cli/src/update.rs`, release workflows. Does not
own protection semantics.

## 2. Work classification

### Invariants

- Manual releases only; crates.io stable authority; checksum + identity
  + version gates before self-replacement; current executable preserved
  on failure; Cargo fallback limited to unsupported targets / exact-404.
- Required CI is exactly `scripts/check.sh`; specialist checks never
  gate without maintainer decision.

### Capabilities

- `version` (offline), `update` (verified), bootstrap installers,
  five-target asset matrix.

### Infrastructure

- Release workflows, preflight/asset-check scripts, fixture rehearsal
  servers.

### Polish

- Installation docs, release closeouts.

## 3. Non-goals

Protection algorithms, verification semantics, automated publication.

## 4. Current state

Active. 0.4.2 is the first eggfetch-enabled release, five-target
qualified (`105-status.md`). M001 remains operationally blocked on a newer
ordinary stable B for the real public A→B self-update proof. M002 cutover is
landed and conditionally closed (`plans/closure/release-distribution/002-status.md`):
Eggpack is now the producer authority for the same five native CLI targets.

M004 selected 0.4.3 but stopped before publication when root-crate semver checks
found additions to stable enums relative to crates.io 0.4.2. No 0.4.3 package,
tag, draft, or release was created. Per the maintainer decision, M004 is
superseded by M005, which selects 0.5.0 as the next 0.x minor compatibility
boundary and records the enum-match migration. M001 and M002 remain blocked
until that ordinary stable B is publicly qualified.

## 5. Target architecture

No change: crates.io authority → GitHub asset fulfillment → verified
self-replacement (ADR-0004).

## 6. Dependency graph

```text
CI/release gates (008, 032-037, closed)
    |
    +--> binary/installer/version (099, 100, 101, closed)
              |
              +--> eggfetch transport + target qualification (103, 104, closed)
                        |
                        +--> first eggfetch release 0.4.2 (105, closed)
                                  |
                                  +--> M001 real A->B transition
                                  |       (blocked until public stable B > 0.4.2)
                                  |
                                  `--> M004 0.4.3 release preparation [SUPERSEDED before publication]
                                           |
                                           `--> M005 0.5.0 release + reconciliation [ACTIVE]
                                           |
                                           +--> first live Eggpack stable B
                                           |       closes M002 condition
                                           |
                                           `--> public 0.4.2 -> 0.5.0 proof
                                                   closes M001 / Plan 106

Eggpack Ecosystem M001/M003h [CLOSED]
    |
    `--> M002 Eggpack producer adoption [CONDITIONALLY CLOSED]
             |
             `--> M005 public 0.5.0 event supplies live evidence
```

M001's live-evidence blocker remains the missing public stable B. M005 owns the
maintainer-selected 0.5.0 release event; M004 remains superseded with its stop
evidence at `plans/closure/release-distribution/004-status.md`.

## 7. Milestones

### Milestone 1 — Real eggfetch-to-eggfetch self-update closure

Class: capability

Objective: prove one real public stable A→B transition where both
binaries contain the native updater.

Dependencies: hard on 0.4.2 (closed); operational on stable B > 0.4.2
published via an ordinary release.

Deliverable boundary: isolated A install → real-endpoint `update` → B
verified by checksum/identity/version → functional smoke → already-current
no-op → deterministic regression re-run. No updater architecture change.

User value: users can trust `stegoeggo update` across real releases.

Exit conditions: all Plan 106 acceptance criteria; evidence in
`plans/106-status.md`.

Deferred work: none.

### Milestone 2 — Eggpack producer adoption and second-consumer qualification

Class: capability

Objective: replace the handwritten native CLI release matrix with Eggpack
producer configuration/generated CI, preserving StegoEggo crates.io,
installer, updater, and human-publication policy.

Dependencies: hard/interface on Eggpack Ecosystem M001/M003h (closed) and
StegoEggo 0.4.2 five-target qualification (closed). M001's missing stable B is
not a hard dependency; M002 can implement before B exists.

Deliverable boundary: checked-in `release/eggpack/` authority, deterministic
five-target generated workflow, bounded product/GLIBC validators,
installer/updater parity guards, drift checking, then one real ordinary stable
release with draft staging and human publication.

Shared operational event: the first ordinary B > 0.4.2 after cutover may also
supply flat Plan 106's real A→B updater evidence after B is public.

Implementation plan:
`plans/implementation/release-distribution/002-eggpack-producer-adoption-and-second-consumer-qualification.md`.

Exit conditions: M002 plan acceptance criteria and
`plans/closure/release-distribution/002-status.md`.

### Milestone 3 — Synchronous Eggup updater bridge corrective

Class: corrective

Objective: remove StegoEggo's unnecessary outer Tokio runtime around the
synchronous `eggup-eggfetch` acquisition seam. Closed implementation
`e611c91` removed the nested runtime; the deterministic updater rehearsal is
green and the shared M001/M002 path is now blocked only on the ordinary stable
B release event.

Dependencies: M002 cutover/closure evidence (landed). No Eggup implementation
dependency was required; the reviewed Eggup adapter documents the synchronous
seam and private runtime ownership. M003 is closed at
`plans/closure/release-distribution/003-status.md`.

Deliverable boundary: synchronous production updater call graph, nested-runtime
regression coverage, green deterministic updater rehearsal, and no release or
fallback policy change.

Implementation plan:
`plans/implementation/release-distribution/003-synchronous-eggup-updater-bridge-corrective.md`.

Exit conditions:
`plans/closure/release-distribution/003-status.md`.

M003 is closed. The next ordinary stable B release may now be used for the
shared M001/M002 operational evidence without an updater-runtime code blocker.

### Milestone 4 — 0.4.3 release preparation (superseded)

Class: capability

Objective: prepare a patch release, then use its public 0.4.2→0.4.3 transition to
complete M001/M002 live evidence. Root semver checks found stable enum additions
incompatible with a 0.4.x patch release before any external publication. The
stop and evidence are recorded at `plans/closure/release-distribution/004-status.md`.

Status: superseded before publication by M005 under the maintainer decision to
release the stable enum additions in 0.5.0. The 0.4.3 candidate was never tagged
or published.

Implementation plan: `plans/implementation/release-distribution/004-0.4.3-release-and-planning-reconciliation.md`.

### Milestone 5 — 0.5.0 release and planning reconciliation

Class: capability

Objective: qualify and manually publish 0.5.0 as the next ordinary stable release,
then use the public 0.4.2→0.5.0 transition to complete M001/M002 live evidence.
The 0.x minor boundary permits the already-landed stable enum additions; exhaustive
Rust matches on the affected enums require migration.

Dependencies: current carrier/release correctness, M008 and Release M003 closure,
M004 stop evidence, maintainer selection of 0.5.0, and confirmation that 0.5.0 is
unused. ADR-0007 remains outside scope.

Deliverable boundary: lockstep 0.5.0 source versions → full release qualification
including passing root/carrier semver checks → ordered crates.io publication →
immutable v0.5.0 tag → Eggpack five-target draft and manual GitHub publication →
real public 0.4.2→0.5.0 updater proof → M001/M002 closure reconciliation.

Implementation plan: `plans/implementation/release-distribution/005-0.5.0-release-and-planning-reconciliation.md`.

Exit conditions: M005 acceptance criteria and closure record at
`plans/closure/release-distribution/005-status.md`, plus successful live evidence
promoting M001 and M002 to closed. A named environmental blocker may justify
conditional closure; published evidence must never be invented.


## 8. Cross-cutting requirements

Storage: none. Protocol: crates.io API + GitHub Release assets over
bounded eggfetch transport, explicit proxy handling, HTTPS-downgrade
denial. Security: sidecar/identity/version gates, no `sudo`, secrets
never logged, oversized responses rejected. Docs: `docs/installation.md`,
`RELEASING.md` updated only on observed-behavior drift.

## 9. Verification strategy

Real-transition evidence (production endpoints, curl-absent where
practical) + deterministic rehearsals (`test-release-updater.sh`,
CLI feature tests). Fixture evidence never substitutes for the public
transition.

## 10. Risks and decision points

- B timing is release-driven; no throwaway stable solely for this test
  (failure policy).
- Windows real-transition is preferred but observational unless it
  regresses; minimum is Plan 105 Windows build/smoke + review.

## 11. Completion definition

The subsystem closes when M001, M002, M003, and M005 are closed. M001 must
hold reproducible public 0.4.2→0.5.0 evidence in `plans/106-status.md`; M002
must record the first live Eggpack-produced stable-release evidence in
`plans/closure/release-distribution/002-status.md`; M003 remains the closed
updater-runtime corrective; and M005 records the coordinated 0.5.0 release
evidence. The same 0.5.0 event may satisfy the M001/M002 operational evidence,
but their closure records remain separate.

## 12. Milestone status table

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 real A→B update | blocked | flat `plans/106-real-eggfetch-to-eggfetch-self-update-closure.md` | flat `plans/106-status.md` (pending B) | no qualified public B newer than 0.4.2; active M005 prepares 0.5.0 |
| M002 Eggpack producer adoption | conditionally closed | `plans/implementation/release-distribution/002-eggpack-producer-adoption-and-second-consumer-qualification.md` | `plans/closure/release-distribution/002-status.md` | live Eggpack-produced B evidence outstanding; active M005 prepares 0.5.0 |
| M003 synchronous Eggup updater bridge corrective | closed | `plans/implementation/release-distribution/003-synchronous-eggup-updater-bridge-corrective.md` | `plans/closure/release-distribution/003-status.md` | none |
| M004 0.4.3 release preparation | superseded | `plans/implementation/release-distribution/004-0.4.3-release-and-planning-reconciliation.md` | `plans/closure/release-distribution/004-status.md` | stopped before publication; superseded by maintainer-selected M005 0.5.0 |
| M005 0.5.0 release and planning reconciliation | active | `plans/implementation/release-distribution/005-0.5.0-release-and-planning-reconciliation.md` | pending | crates.io/GitHub state is unused; qualification in progress |
| prior gates/binary/eggfetch/0.4.2 | closed | flat `008`, `024`–`025`, `032`–`037`, `086`, `089`, `099`–`105` | `*.status.md` companions as present | — |
