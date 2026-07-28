# Plan 033: CI, Verification, and Manual Release Simplification Roadmap

Status: Ready for implementation

Baseline: `main` at `de143eeb58d07738e5ffefa5da734fa65b26fc08`

Depends on:

- `plans/032-release-pipeline-and-immutable-version-closure.md`

Supersedes, for future process design, the automated release and universal-gate portions of Plan 032. Historical release evidence and the crates.io immutable-version rule remain valid.

Implementation plans:

- `plans/034-remove-automated-release-and-decouple-tags.md`
- `plans/035-collapse-required-ci-and-demote-specialist-verification.md`
- `plans/036-local-verification-policy-and-documentation-closure.md`

---

## Purpose

The repository now has substantially more CI, release orchestration, and evidence-generation machinery than is justified by the size and operating model of the project. The current apparatus impedes iteration by executing overlapping checks across ordinary CI, release-candidate validation, tag validation, and publication. It also publishes to crates.io from GitHub Actions, contrary to the intended release model.

This roadmap reduces process complexity while preserving product correctness. It deliberately distinguishes between:

1. fast checks that should run on every pull request and push;
2. specialist checks that remain available but should run only when relevant or explicitly requested;
3. local pre-release checks controlled by the maintainer;
4. manual publication directly through Cargo and crates.io;
5. product-level tests and fixtures, which should generally remain intact.

The primary objective is not to obtain fewer tests by deleting useful correctness coverage. The objective is to stop executing the same expensive checks repeatedly and automatically at every lifecycle stage.

---

## Problem statement

At the baseline, the repository has the following process characteristics:

- the main CI workflow contains independent jobs for MSRV, docs.rs reproduction, fresh packaged MSRV resolution, packaging, tests, linting, audit, semver, cargo-deny, external integration, external conformance, a five-entry feature matrix, and manually dispatched benchmarks;
- the fuzz workflow launches twelve target-specific jobs on every push and pull request;
- the release-candidate workflow repeats the complete validation suite for an exact SHA;
- the tag-validation workflow repeats the complete validation suite on every `v*` tag;
- the publish workflow requires prior RC evidence, repeats complete validation, publishes two crates, waits for registry indexing, builds a Linux binary, creates a GitHub release, and uploads inventories;
- the shared release validator invokes formatting, linting, tests, packaging, two advisory mechanisms, semver checks, MSRV checks, fuzz synchronization, docs.rs simulation, packaged MSRV consumer checks, feature combinations, external integration, and conformance;
- release-support scripts contain hard-coded package-version filenames and therefore require maintenance whenever the crate version changes;
- ordinary changes can fan out into approximately twenty-eight GitHub Actions jobs before any release workflow is involved;
- a version tag can trigger mutating release automation, which conflicts with the requirement that publication cadence remain manually controlled.

This architecture creates multiple failure surfaces unrelated to the product itself: GitHub Actions orchestration, toolchain installation, cargo plugin installation, apt repository availability, external-tool versions, crates.io timing, workflow-run lookup, exact-SHA evidence transfer, artifact generation, and duplicated dependency resolution.

---

## Governing decisions

The implementation agent must treat the following as explicit project policy.

### Decision 1: crates.io publication is manual

GitHub Actions must not publish either crate. Publication is performed manually with `cargo publish` after local validation.

No GitHub workflow may require or reference `CARGO_REGISTRY_TOKEN`.

### Decision 2: tags do not publish

Pushing a `v*` tag must not publish crates, build release binaries, create a GitHub release, upload evidence, or invoke a release gate.

Tags and GitHub releases are optional maintainer actions after publication. They are not part of the publication control path.

### Decision 3: one required CI job

The target required CI surface is one Ubuntu job using stable Rust. It should run fast, deterministic, repository-local checks and should not install operating-system packages or cargo subcommands.

### Decision 4: specialist verification remains available

The following capabilities should remain available unless a specific implementation finding proves them obsolete:

- external metadata interoperability tests;
- the conformance harness and fixtures;
- fuzz targets and corpora;
- MSRV validation;
- docs.rs reproduction;
- semver analysis;
- license/advisory analysis;
- package dry-runs;
- benchmarks.

They are demoted from universal gates to targeted or manual checks.

### Decision 5: preserve regression coverage

Do not delete ordinary Rust unit, integration, CLI, doctest, or deterministic regression tests merely to reduce CI duration. First reduce duplicate execution, workflow fan-out, matrices, and external setup.

### Decision 6: local verification is authoritative for release readiness

The maintainer controls release timing and performs local pre-release checks. GitHub CI verifies ordinary development correctness; it does not certify or execute publication.

### Decision 7: immutable versions remain a release invariant

A version accepted by crates.io cannot be overwritten or reused, including when yanked. A failed or defective published version requires a new unused version. This policy survives the removal of automated release workflows.

---

## Target end state

### Required push and pull-request CI

One job, one stable toolchain, no matrix:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
```

A focused doctest command may remain in this job if it exercises real documentation examples and does not materially duplicate workspace execution:

```bash
cargo test --doc -p stegoeggo --all-features
```

The implementation agent must verify actual workspace behavior before deciding whether the separate doctest command is needed.

### Optional external verification

One manually dispatched workflow or documented local command installs the external tools once and runs both external integration and conformance. It must not run on pushes, pull requests, tags, or schedules.

### Optional fuzzing

Fuzzing remains local or manually dispatched. A manual workflow should accept a single target input and should not fan out over all targets automatically. Deep fuzzing may be run locally as needed.

### Manual release

The repository documents a local sequence similar to:

```bash
./scripts/check.sh
./scripts/release-check.sh

cargo publish -p stegoeggo --dry-run
cargo publish -p stegoeggo

# Wait until the matching library version is resolvable from crates.io.
cargo publish -p stegoeggo-cli --dry-run
cargo publish -p stegoeggo-cli

# Optional after publication:
git tag vX.Y.Z
git push origin vX.Y.Z
```

The exact script names and checks are finalized in Plan 036.

### CI complexity budget

The final repository must satisfy this budget:

```text
required push/PR workflows: 1
required jobs per push/PR: 1
required job matrices: 0
required Rust toolchains: stable only
required cargo-installed tools: 0
required apt-installed tools: 0
required artifact uploads: 0
tag-triggered mutating workflows: 0
GitHub-held crates.io credentials referenced by code: 0
```

---

## Roadmap sequence

### Milestone A: remove publication risk first

Implement Plan 034 before any other work.

Reason: the current `publish.yml` can mutate crates.io and GitHub release state when a version tag is pushed. This is the highest-risk mismatch with the desired policy. Removal of tag-triggered publication must not wait for CI refactoring.

Expected outcome:

- automated publication is impossible from repository workflows;
- RC and tag-validation workflows are removed;
- release secrets and environments are documented for manual cleanup;
- tags become inert from the perspective of GitHub Actions;
- manual release ownership is explicit.

### Milestone B: collapse required CI

Implement Plan 035 after Plan 034.

Expected outcome:

- one required stable CI job replaces the current job graph;
- feature and CLI duplication is removed;
- fuzzing no longer runs on pushes or pull requests;
- external conformance is manual only;
- MSRV, docs.rs, semver, audit, deny, package, and benchmark capabilities are removed from baseline CI and assigned a targeted cadence;
- branch protection migration is documented.

### Milestone C: codify local verification and prevent regression

Implement Plan 036 last.

Expected outcome:

- a small fast local check mirrors required CI;
- a bounded manual release-check script exists;
- version-hard-coded release scripts are removed or corrected;
- `AGENTS.md` no longer instructs agents to recreate the thirteen-job pipeline;
- `RELEASING.md` documents manual crates.io publication and immutable versions;
- a complexity policy prevents future agents from casually restoring matrices, RC evidence workflows, or automated publishing;
- obsolete documentation and references are removed.

---

## Scope boundaries

### In scope

- `.github/workflows/ci.yml`;
- `.github/workflows/fuzz.yml`;
- `.github/workflows/release.yml`;
- `.github/workflows/release-candidate.yml`;
- `.github/workflows/publish.yml`;
- validation scripts under `scripts/`;
- release and agent documentation;
- branch-protection migration notes;
- package dry-run and manual publication documentation;
- optional manual workflows for external tools or single-target fuzzing;
- status ledgers for Plans 033 through 036.

### Out of scope

Do not use this work as an opportunity to:

- redesign steganography or metadata semantics;
- reduce cryptographic correctness requirements;
- delete conformance fixtures because they are expensive;
- remove fuzz targets without proving they are duplicate or invalid;
- change the public API;
- change image format support;
- change the MSRV as a convenience;
- weaken `#![forbid(unsafe_code)]`;
- change dependency versions except where required to keep the simplified checks correct;
- publish a release;
- create, move, or delete release tags;
- alter already published crate versions;
- add another CI provider;
- replace GitHub Actions with an equally complex custom system.

---

## Required implementation discipline

1. Work plans in numeric order.
2. Create a status ledger for each implementation plan before changing workflows.
3. Capture the pre-change workflow inventory and event triggers.
4. Delete obsolete workflows rather than leaving disabled or misleading copies unless repository policy requires archival.
5. Do not retain tag-triggered validation “just in case.” Tag validation is part of the removed release apparatus.
6. Do not move all old checks into one enormous mandatory script and call that simplification.
7. Keep the fast check understandable by reading the script or workflow directly.
8. Every retained check must have one declared purpose and one normal execution location.
9. Avoid duplicated commands between the required CI workflow and local scripts by having CI invoke the fast local script, or by keeping both command lists small and mechanically identical.
10. Specialist scripts may fail independently without blocking ordinary development unless the maintainer intentionally invokes them.
11. Optional workflows must use `workflow_dispatch` only unless this roadmap explicitly says otherwise.
12. Do not add schedules during this line of work.
13. Do not add artifact uploads merely to replace deleted evidence workflows.
14. Do not make release publication a `workflow_dispatch` action. Manual means direct Cargo/crates.io operation outside GitHub Actions.
15. Do not store crates.io credentials in repository files.
16. Preserve the library-before-CLI publication order because the CLI depends on the exact matching library version.
17. Record any repository settings that cannot be changed through source control, especially branch protection required-check names, the `crates-io` environment, and the registry token secret.
18. If a currently required branch-protection check is deleted, state the exact old and new required-check names in the handoff.

---

## Verification strategy for the simplification itself

The implementation agent must prove that simplification did not silently remove basic correctness coverage.

At minimum, after Plan 035:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
```

If retained:

```bash
cargo test --doc -p stegoeggo --all-features
```

After Plan 036, manually exercise the optional paths once:

```bash
./scripts/check.sh
./scripts/release-check.sh --dry-run-only
```

The exact interface may differ, but the release check must not publish.

Also verify through static repository search:

```bash
rg -n "CARGO_REGISTRY_TOKEN|cargo publish|action-gh-release|release-candidate" .github scripts AGENTS.md RELEASING.md
```

Expected result:

- `cargo publish` appears only in human-facing manual release documentation or explicitly non-executing examples;
- no workflow references a registry token;
- no workflow invokes `cargo publish`;
- no workflow creates a GitHub release;
- no workflow searches for RC runs;
- no push or pull-request workflow invokes fuzzing, external tools, semver tools, advisory tools, docs.rs simulation, or fresh packaged MSRV consumers.

---

## Program-level acceptance criteria

This roadmap is complete only when all three implementation plans are closed and all of the following are true.

### Release control

- No GitHub Actions workflow publishes `stegoeggo` or `stegoeggo-cli`.
- No workflow references `CARGO_REGISTRY_TOKEN`.
- No `v*` tag trigger performs validation or mutation.
- No release-candidate workflow exists.
- No workflow creates a GitHub release or release artifact automatically.
- `RELEASING.md` describes direct manual publication through Cargo.
- The immutable-version rule is explicit.
- The library-before-CLI order is explicit.

### Required CI

- Exactly one required workflow runs on pushes and pull requests to `main`.
- That workflow has exactly one job.
- It uses stable Rust only.
- It has no matrix.
- It installs no cargo subcommands.
- It installs no operating-system packages.
- It uploads no artifacts.
- It runs formatting, clippy, minimal-feature compilation, and all-feature workspace tests.
- It does not repeat CLI tests already covered by workspace tests.

### Specialist verification

- External interoperability and conformance remain runnable.
- Fuzz targets remain runnable.
- Optional workflows use manual dispatch only.
- A manual fuzz workflow, if retained, runs one selected target rather than all targets.
- MSRV, docs.rs, semver, license, advisory, package, and benchmark checks have an explicit local or targeted invocation policy.
- None of these checks is silently presented as required CI.

### Documentation and maintenance

- `AGENTS.md` describes the simplified architecture accurately.
- Obsolete references to thirteen CI jobs, RC evidence, exact-SHA release candidates, and automated publishing are removed.
- Fast and release-local command sequences are concise and executable.
- No script hard-codes the current crate version in a package filename.
- Branch protection migration notes identify the new required check.
- Repository-setting cleanup steps identify obsolete crates.io secrets and environments.
- The status ledger contains before-and-after workflow counts and trigger counts.

### Product correctness

- Existing deterministic unit, integration, CLI, doctest, and conformance test sources are not removed without documented justification.
- The final fast command set passes.
- The optional external-conformance path is exercised successfully at least once after refactoring, or any environment-specific blocker is recorded with exact commands and output.
- The optional fuzz path is structurally valid after workflow changes.

---

## Expected repository-level reduction

The implementation should aim for approximately this result:

```text
Before:
  required/automatic workflows: CI + fuzz + tag validation + publish
  ordinary push/PR jobs: ~28
  release lifecycle validation repetitions: up to 3 full suites
  automated crates.io publication: yes
  tag-triggered mutation: yes

After:
  required/automatic workflows: CI only
  ordinary push/PR jobs: 1
  release lifecycle validation repetitions: 0 in GitHub Actions
  automated crates.io publication: no
  tag-triggered mutation: no
  optional specialist workflows: workflow_dispatch only
```

Do not treat line-count reduction alone as completion. The meaningful metrics are trigger count, job count, duplicated command count, external dependency count, and number of mutating release paths.

---

## Handoff notes for a smaller implementation model

The implementation agent should not reinterpret this roadmap as permission to remove all sophisticated testing. The desired transformation is:

```text
valuable test capability + deliberate local/manual use
instead of
valuable test capability + mandatory duplicated automation everywhere
```

When uncertain whether to delete a test or merely change its trigger, preserve the test and change the trigger.

When uncertain whether a release action should remain in GitHub Actions, remove it. Publication ownership is explicitly manual.

When uncertain whether a specialist check belongs in required CI, exclude it unless it is one of the four baseline commands listed in the target end state.

Do not add replacement evidence systems. The maintainer needs a fast correctness signal and a documented manual release procedure, not a new release-certification framework.
