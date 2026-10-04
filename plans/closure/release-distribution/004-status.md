# Release and Distribution Milestone 004 — Blocked Status

Status: blocked — no release publication or formal completion
Source implementation plan: `plans/implementation/release-distribution/004-0.4.3-release-and-planning-reconciliation.md`
Source subsystem roadmap: `plans/subsystems/release-distribution-roadmap.md#milestone-4--043-release-and-planning-reconciliation`
Repository baseline reviewed: `a01a0222c6a5022a9eb3561f153b1e72b6e6c42a`
Implementation commits: `b87c61f71d1e3ceee6b19927385063f3ab73eed1` — candidate source-version preparation only; no release artifact was published

## 1. Executive finding

The 0.4.3 release was stopped before publication because the required
`cargo semver-checks check-release -p stegoeggo` gate found two public API
breaks relative to crates.io 0.4.2. `STABILITY.md` marks both affected enum
surfaces stable, and no plan or accepted decision authorizes those breaks in a
patch release. M004 §14 explicitly requires a stop in this case.

No crates.io package, Git tag, Eggpack draft, or GitHub release was created.
The source-version changes and release notes in the release-prep branch are
candidate preparation only; they are not a qualified or publishable 0.4.3.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence / disposition |
|---|---|
| 0.4.3 unused before mutation | Confirmed before source edits: all three `cargo info <crate>@0.4.3` queries reported version not found; GitHub had no `v0.4.3` release/tag. |
| Lockstep source-version preparation | Root, carrier, CLI, Python, Node, and C source package metadata was updated to 0.4.3 in the isolated release-prep branch; Cargo regenerated Rust lockfiles. No binding registry was published. |
| Required fast check | `./scripts/check.sh` passed. |
| Pre-release package qualification | `./scripts/release-check.sh --stage=pre --allow-dirty` passed and verified package contents. |
| Updater / installer local rehearsal | `./scripts/test-release-updater.sh` passed. `bash -x scripts/test-release-installers.sh` passed after stopping an earlier silent run that had stalled in its local-server setup. |
| Release workflow contract | `python3 scripts/check-release-contract.py`, `python3 scripts/check-release-workflow-contract.py`, and Eggpack drift check passed (`ci check: match (55963 bytes)`). |
| Carrier crate semver | `cargo semver-checks check-release -p stegoeggo-stego`: 196 checks passed, 58 skipped; no update required. |
| Root crate semver | `cargo semver-checks check-release -p stegoeggo`: 194 passed, 2 failed, 58 skipped; release stopped. |
| Crates.io / GitHub publication | Not performed. |
| Public A→B updater evidence | Not available; Plan 106 and M002 retain their existing live-release blocker. |

## 3. Production implementation evidence

The release-prep branch contains candidate version metadata, regenerated Cargo
locks, version-neutral binding-release instructions, and an unreleased
changelog section. The source tree itself had no functional Rust implementation
changes for M004. Binding CI and native release qualification were not used to
override the root-crate semver failure.

## 4. Verification executed (exact commands + results)

- `./scripts/check.sh` — pass.
- `./scripts/release-check.sh --stage=pre --allow-dirty` — pass.
- `./scripts/test-release-updater.sh` — pass.
- `bash -x scripts/test-release-installers.sh` — pass (`Release installer tests passed`).
- `python3 scripts/check-release-contract.py` — pass.
- `python3 scripts/check-release-workflow-contract.py` — pass.
- `eggpack ci check --workflow-shape release/eggpack/workflow-shape.json --contract release/eggpack/distribution.toml --github-policy release/eggpack/github-policy.json --workflow .github/workflows/release-binaries.yml` — pass; generated workflow matches at 55,963 bytes.
- `cargo semver-checks check-release -p stegoeggo-stego` — 196 pass, 58 skip.
- `cargo semver-checks check-release -p stegoeggo` — 194 pass, 2 fail, 58 skip. Failures: `enum_no_repr_variant_discriminant_changed` and `enum_variant_added`.

The affected additions are `FieldSource::Unavailable` in
`src/verification/report.rs` and `VerificationResult::Invalid` in
`src/types/verification.rs`. Both enums are listed as stable in `STABILITY.md`.
The lint also identifies shifted implicit discriminants for the subsequent
`FieldSource` and `VerificationResult` variants. No explicit decision accepting
these breaks for a 0.4.x patch release was found.

The remaining specialist gates (MSRV package, docs.rs, strict external metadata
conformance, cargo-deny, parser fuzzing), host binding workflows, hosted CI,
crates.io publication, tag, Eggpack native jobs/draft, and real updater proof
were not run after the mandatory semver stop. They remain unclaimed.

## 5. Invariant review

The source candidate retains exact dependency lockstep. The public 0.4.2 source
and release remain untouched. No C ABI major/minor change or binding registry
publication was performed. The 15-file release asset contract and generated
workflow were unchanged. The candidate is not labeled updater-ready.

## 6. Failure and recovery review

This is a pre-publication stop. No immutable external state needs recovery.
The candidate version must not be published or tagged. Maintainers must either
restore compatibility with the published 0.4.2 API and re-qualify, or authorize
a separately planned release version and reconcile all version, updater, and
planning evidence. An accepted decision is required before resuming.

## 7. Migration and compatibility review

M008's JPEG compatibility behavior is unchanged. The blocker concerns unrelated
public enum additions made after the 0.4.2 release. Shipping those additions in
0.4.3 would contradict the stable API and patch-release compatibility promises.

## 8. Security review

No security finding was introduced by this release-preparation work. No artifact
was published or installed from a public release endpoint.

## 9. Documentation and operations

The binding instructions no longer default to the predecessor version. The
installation guide continues to identify the actually published v0.4.2 assets.
The candidate changelog remains under `Unreleased` and has no release date.

## 10. Unresolved findings (critical/high/medium/low)

- **Release blocker:** two unaccepted stable public API breaks prevent the
  planned 0.4.3 patch release. Maintainer disposition is required.
- **Live evidence outstanding:** Plan 106 / M001 and M002 still require the
  first qualified stable release newer than 0.4.2 and a real public updater
  transition.
- No new critical/high/medium/low security or carrier finding was identified.

## 11. Roadmap disposition

M004 is blocked before publication. M001 and M002 remain blocked on a qualified
ordinary stable B. The release-distribution subsystem remains active. Stego
Library M006 remains blocked only on ADR-0007 acceptance; M007 remains proposed.

## 12. Registry updates

`plans/registry.md` removes M004 from dependency-ready work and records its
semver blocker. The release-distribution roadmap records M004 as blocked. Plan
106 and the M002 conditional closure remain unchanged because no public B or
updater evidence exists.
