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
Eggpack is now the producer authority for the same five native CLI targets,
and the next ordinary stable release serves as both M002's live
second-consumer proof and, after publication, M001's A→B evidence. No
throwaway version may be cut for either. The updater rehearsal panic is
resolved by closed M003, the synchronous Eggup updater bridge corrective
(`plans/closure/release-distribution/003-status.md`); the shared live A→B
proof still waits for the ordinary stable B event.

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
                        +--> first eggfetch release 0.4.2 (105, closed, hard dep)
                                  |
                                  `--> M001 real A->B transition (106, BLOCKED on stable B)

Eggpack Ecosystem M001/M003h [CLOSED]
    |
    `--> M002 Eggpack producer adoption [CONDITIONALLY CLOSED: CUTOVER LANDED]
             |
             `--> next ordinary stable B live Eggpack release
                        |
                        `--> same public B supplies M001 A->B evidence
```

M001's blocker is operational (external release event), not a code dep.

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

The subsystem closes only when both capability milestones are fully closed and the M003 updater-runtime corrective is closed:
M001 holds reproducible public A→B evidence in `plans/106-status.md`, and M002
records the Eggpack producer-adoption/live-release evidence in
`plans/closure/release-distribution/002-status.md`. The same ordinary stable
B release may satisfy both operational evidence sets, but the closure records
remain separate.

## 12. Milestone status table

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 real A→B update | blocked | flat `plans/106-real-eggfetch-to-eggfetch-self-update-closure.md` | flat `plans/106-status.md` (pending B) | stable B > 0.4.2 not yet published |
| M002 Eggpack producer adoption | conditionally closed | `plans/implementation/release-distribution/002-eggpack-producer-adoption-and-second-consumer-qualification.md` | `plans/closure/release-distribution/002-status.md` | cutover landed; live B > 0.4.2 evidence outstanding on the same ordinary release as M001 |
| M003 synchronous Eggup updater bridge corrective | closed | `plans/implementation/release-distribution/003-synchronous-eggup-updater-bridge-corrective.md` | `plans/closure/release-distribution/003-status.md` | nested Tokio runtime removed; updater rehearsal green; no release/fallback policy change |
| prior gates/binary/eggfetch/0.4.2 | closed | flat `008`, `024`–`025`, `032`–`037`, `086`, `089`, `099`–`105` | `*.status.md` companions as present | — |
