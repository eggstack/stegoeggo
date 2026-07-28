# Plan 032: Release Pipeline and Immutable-Version Closure

Status: Ready for implementation

Baseline: `main` at `3ee3a32cac51554d81cab2366ea81c11f3694efb`

Depends on:

- `plans/030-plan-029-residual-corrective-handoff.md`
- `plans/031-plan-030-correctness-and-release-evidence-closure.md`
- `plans/030-status.md`
- `plans/031-status.md`

Current workspace version at the baseline: `0.3.1`

Expected next release version: `0.3.2`, **provided `0.3.1` is already present on crates.io**.

Release hold: active until this plan is completed against one exact code candidate SHA.

---

## Purpose

The `0.3.0` release was published and then found to have a docs.rs-specific build failure that was not reproduced by the repository's CI, release-candidate workflow, or local release validation script. A crate-root `doc_cfg` correction and workspace bump to `0.3.1` followed, after which further CI failures were observed.

This plan closes the release-engineering gaps that allowed those failures to escape and establishes a safe next-release procedure.

The central release rule is non-negotiable:

> A version published to crates.io is immutable. Its package bytes cannot be replaced. If a published release is defective, fix the repository and publish a new, previously unused version.

Therefore:

- never attempt to overwrite `0.3.0`;
- if `0.3.1` is already published, never attempt to overwrite `0.3.1`;
- the next release must use `0.3.2` or a later unused version;
- if `0.3.2` is published and then found defective, the correction must be released as `0.3.3` or later;
- yanking a release does not make its version reusable;
- deleting or moving a Git tag does not make a crates.io version reusable;
- a documentation-only defect still requires a new crate version once the defective package was published.

This plan is written for reliable execution by a smaller implementation model. It provides exact work phases, examples, failure classifications, test requirements, commit boundaries, and completion criteria.

---

# Verified baseline facts

The implementation agent must begin from these repository facts.

1. `main` is at `3ee3a32cac51554d81cab2366ea81c11f3694efb`.
2. The library and CLI manifests both declare version `0.3.1`.
3. The original docs.rs repair moved `#![cfg_attr(docsrs, feature(doc_cfg))]` from module files to the crate root.
4. The repository declares `rust-version = "1.87"`.
5. The published dependency requirement for `image` is still broad enough to permit later `0.25.x` releases, even though the lockfile currently selects `0.25.6`.
6. Main CI runs doc tests but does not reproduce a docs.rs rustdoc build.
7. `scripts/validate-release.sh` runs doc tests but does not reproduce a docs.rs rustdoc build.
8. The tag validation workflow runs doc tests but does not reproduce a docs.rs rustdoc build.
9. A locked workspace MSRV check does not prove that a fresh downstream dependency resolution supports the declared MSRV.
10. `plans/031-status.md` is marked `CLOSED` despite containing `OPEN` and `PARTIAL` rows.
11. `CHANGELOG.md` still labels `0.3.0` as unreleased and does not accurately describe the current release sequence.
12. README dependency examples still reference the `0.2` line.
13. `SECURITY.md` still lists only `0.2.x` as supported.
14. Current push-run CI details must be captured directly from GitHub Actions because cancelled runs and actual failures may be mixed together.
15. The next publish operation must not reuse any version already accepted by crates.io.

---

# Non-goals

Do not expand this plan into unrelated product work.

Specifically, do not:

- redesign steganography;
- add payload v4;
- change detached-manifest semantics unless a current CI failure proves it necessary;
- add new image formats;
- perform broad performance refactors;
- redesign the public rights-policy API;
- change cryptographic algorithms;
- rewrite conformance fixtures unrelated to an observed release failure;
- remove the declared MSRV without an explicit version-policy decision;
- disable, ignore, or make failing checks non-blocking;
- use yanking as a substitute for publishing a corrected version;
- rewrite the history of `v0.3.0` or any published release tag.

The goal is a deterministic, truthful, reproducible release pipeline.

---

# Mandatory execution rules

1. Work phases in order.
2. Create `plans/032-status.md` before production edits.
3. Record all observed GitHub Actions run IDs and job IDs before changing CI.
4. Separate cancelled jobs from failed jobs.
5. Do not call a run failed merely because it was cancelled by concurrency.
6. Do not rerun a failed workflow until its first failure logs have been recorded.
7. Do not lower test coverage to make CI green.
8. Do not add `continue-on-error` to release-blocking jobs.
9. Do not suppress rustdoc errors with broad `allow` attributes.
10. Do not change `rust-version` silently.
11. Do not rely on `Cargo.lock` to prove library-consumer MSRV compatibility.
12. Test the packaged `.crate` contents, not only the workspace checkout.
13. Use the same authoritative validation commands in main CI and the release-candidate workflow.
14. Use full 40-character SHAs in evidence.
15. A documentation or ledger commit after validation is not the code candidate SHA.
16. Do not publish until the candidate SHA has passed every pre-publication gate.
17. Before publishing, query crates.io and prove the selected version is unused.
18. If the selected version already exists, increment again before publishing.
19. If publication succeeds for either crate, that version is consumed permanently.
20. If the library publishes but CLI publication fails, do not republish the library; fix the CLI and publish a new CLI version as required by the chosen versioning policy.
21. If docs.rs fails after publication, fix the source and bump to another unused patch version.
22. Never force-move a release tag after publication.
23. Status values are only `OPEN`, `PARTIAL`, `CLOSED`, or `SUPERSEDED`.
24. A phase cannot be `CLOSED` without production changes, tests, command evidence, and a commit SHA.

---

# Phase 0: Establish truthful release state

## 0.1 Confirm registry state

Before selecting a release number, inspect crates.io for both packages:

- `stegoeggo`;
- `stegoeggo-cli`.

Record whether these versions exist and whether they are active or yanked:

- `0.3.0`;
- `0.3.1`.

Use the registry API or `cargo search`/`cargo info`; do not infer publication solely from repository commits.

Required status table:

```text
package | version | exists | yanked | publish timestamp | checksum/source evidence
```

Version selection rule:

```text
next_version = first unused patch version greater than every published 0.3.x version
```

Expected normal result:

- if `0.3.1` exists for the package being released, select `0.3.2`;
- if `0.3.2` already exists, select `0.3.3`;
- never select a version that exists but is yanked.

## 0.2 Correct Plan 031 status truth

Update `plans/031-status.md` so it distinguishes:

- historical publication of `0.3.0`;
- successful code-candidate validation performed before that publication;
- the subsequent docs.rs failure;
- the incomplete rows already present in the ledger;
- supersession of release-pipeline closure by Plan 032.

Recommended disposition:

```text
Disposition: PARTIAL
Release pipeline closure: SUPERSEDED by Plan 032
```

Do not erase valid historical evidence. Correct the conclusion, not the history.

## 0.3 Create `plans/032-status.md`

Initialize with:

```text
Plan baseline SHA: 3ee3a32cac51554d81cab2366ea81c11f3694efb
Selected next version: not confirmed
Code candidate SHA: not selected
Evidence commit SHA: not selected
Disposition: OPEN
Release hold: active
```

Add these tables.

### Table A: published versions

```text
package | version | exists | yanked | immutable | action
```

### Table B: current CI failures

```text
workflow | run ID | triggering SHA | job ID | job name | conclusion | cancellation reason | first failing step | root cause | status
```

### Table C: release gates

```text
gate | workspace test | packaged-crate test | fresh resolution | main CI | RC | tag workflow | status
```

Required gates:

- formatting;
- clippy;
- workspace tests;
- CLI tests;
- doc tests;
- docs.rs-equivalent rustdoc;
- package creation;
- package-content verification;
- fresh MSRV consumer resolution;
- current stable consumer resolution;
- semver;
- audit;
- deny licenses;
- deny advisories;
- external integration;
- conformance;
- fuzz-target synchronization;
- required fuzz smoke/full run.

### Table D: release artifacts

```text
artifact | candidate SHA | package version | checksum | produced by run | inspected | status
```

### Table E: post-publication verification

```text
package/version | crates.io install | clean consumer build | docs.rs build | CLI smoke | security smoke | status
```

## Phase 0 acceptance criteria

- Registry state is confirmed independently of repository claims.
- The next version is selected by the first-unused-version rule.
- The plan explicitly states that published versions cannot be overwritten.
- Plan 031 no longer claims unconditional release-pipeline closure.
- `plans/032-status.md` exists with all five tables.
- Release hold remains active.

## Suggested commit

```text
plans: reopen release closure and record immutable version state
```

---

# Phase 1: Capture and classify every current CI failure

Do not begin by guessing from commit messages.

## 1.1 Capture workflow runs

For the current head and the commits immediately preceding it, record:

- workflow name;
- run ID;
- attempt number;
- triggering event;
- triggering SHA;
- start and completion timestamps;
- conclusion;
- every job name and job ID.

At minimum inspect:

- main CI;
- release candidate;
- tag/release validation;
- fuzz workflow;
- any publish workflow.

## 1.2 Separate cancellation from failure

The main CI workflow uses cancellation for superseded runs. Classify each job as:

- `success`;
- `failure`;
- `cancelled_by_newer_run`;
- `cancelled_manually`;
- `skipped_due_to_dependency`;
- `infrastructure_failure`;
- `deterministic_repository_failure`;
- `transient_external_failure`.

A cancelled run is not evidence that the candidate failed.

## 1.3 Preserve first-failure logs

For every actual failure, save the final relevant log section and identify:

- command invoked;
- toolchain version;
- dependency resolution;
- exact compiler/test error;
- whether the same command fails locally or in an isolated container;
- whether failure depends on a lockfile;
- whether failure depends on all features;
- whether failure occurs only in packaged source.

Do not rerun until this evidence is recorded.

## 1.4 Build a root-cause map

Expected categories include, but are not limited to:

- docs.rs/rustdoc configuration;
- fresh MSRV resolution;
- semver baseline selection after publishing `0.3.0`/`0.3.1`;
- package workspace ordering;
- CLI dependency version mismatch;
- conformance fixture drift;
- transient package-index or network failure;
- cancellation due to rapid pushes;
- toolchain-version drift.

Each failure needs one root-cause row. Do not group unrelated failures under “CI issue.”

## Phase 1 acceptance criteria

- Every current non-success run is classified.
- Every actual failure has a saved command and error summary.
- Cancelled jobs are not counted as defects.
- Deterministic failures reproduce with an explicit command.
- Transient failures have evidence supporting that classification.
- No check has been disabled.

## Suggested commit

```text
plans: record post-publish CI failure evidence
```

This phase may be evidence-only. Do not change production or workflow code in the same commit.

---

# Phase 2: Add a true docs.rs-equivalent validation gate

The crate-root `doc_cfg` relocation is necessary but not sufficient. The release pipeline must reproduce the environment that activates it.

## 2.1 Add `scripts/validate-docs-rs.sh`

Create one authoritative script used by local validation, main CI, and RC.

Required behavior:

1. fail on a dirty tree when invoked for release validation;
2. install or require nightly Rust;
3. build library documentation with all features;
4. set `DOCS_RS=1`;
5. activate `cfg(docsrs)`;
6. use `--check-cfg=cfg(docsrs)` to prevent accidental unknown-cfg warnings;
7. deny rustdoc warnings;
8. build documentation from the workspace;
9. build documentation from the extracted packaged crate;
10. verify expected public modules appear;
11. fail if module-level `#![feature(...)]` attributes reappear.

Illustrative script core:

```bash
#!/usr/bin/env bash
set -euo pipefail

export DOCS_RS=1
export RUSTDOCFLAGS="--cfg docsrs --check-cfg=cfg(docsrs) -D warnings"

cargo +nightly rustdoc \
  -p stegoeggo \
  --all-features \
  --no-deps
```

Do not blindly copy docs.rs internals. The goal is to reproduce the important semantic conditions:

- nightly;
- `DOCS_RS=1`;
- `cfg(docsrs)`;
- all features;
- rustdoc rather than only doc tests.

## 2.2 Validate the packaged crate

After `cargo package -p stegoeggo`, extract the generated `.crate` into a temporary directory and run the same rustdoc command from that package.

Required checks:

- package contains `src/lib.rs`;
- package contains every source module required by all features;
- package metadata includes the docs.rs settings;
- rustdoc succeeds without workspace-only files;
- no path dependency is required to build the library package docs.

Illustrative flow:

```bash
cargo package -p stegoeggo --allow-dirty
crate_file=$(find target/package -maxdepth 1 -name 'stegoeggo-*.crate' | sort | tail -1)
mkdir -p "$tmp/pkg"
tar -xzf "$crate_file" -C "$tmp/pkg"
cd "$tmp/pkg"/stegoeggo-*
DOCS_RS=1 RUSTDOCFLAGS="--cfg docsrs --check-cfg=cfg(docsrs) -D warnings" \
  cargo +nightly rustdoc --all-features --no-deps
```

Use a clean candidate for final validation; `--allow-dirty` is only illustrative for development.

## 2.3 Add compile-fail regression protection

Add a repository check that rejects `feature(doc_cfg)` outside crate roots.

Allowed location:

```text
src/lib.rs
```

Potentially allowed binary roots only if actually required:

```text
src/bin/*.rs
```

Disallowed:

```text
src/**/mod.rs
```

A simple script/grep check is acceptable if it has a regression test.

## 2.4 Add CI job

Add a blocking `Docs.rs Build` job to main CI.

It must:

- check out the exact triggering SHA;
- install nightly;
- invoke `scripts/validate-docs-rs.sh`;
- upload rustdoc diagnostics on failure when practical;
- remain blocking.

## 2.5 Add to authoritative release validation

`scripts/validate-release.sh` must invoke the docs.rs script during the hermetic phase.

The RC workflow must continue to call `validate-release.sh`, not duplicate the docs commands independently.

## Required tests

- docs.rs-equivalent build succeeds with default features;
- docs.rs-equivalent build succeeds with all features;
- packaged library docs succeed;
- a test fixture placing `#![feature(doc_cfg)]` in a module is rejected by the policy check;
- ordinary stable `cargo test --doc` still succeeds;
- `cargo doc --no-deps` still succeeds under stable without `cfg(docsrs)`.

## Phase 2 acceptance criteria

- The actual docs.rs failure mode is reproduced by a regression test or documented pre-fix command.
- `scripts/validate-docs-rs.sh` exists and is executable.
- Main CI has a blocking docs.rs-equivalent job.
- RC invokes the same docs validation through the authoritative script.
- Workspace and packaged-crate docs both pass.
- The pipeline would have rejected the original `0.3.0` package.

## Suggested commits

```text
docs: add docs.rs-equivalent workspace and package validation
ci: make docs.rs-equivalent build release-blocking
```

---

# Phase 3: Make the declared MSRV true under fresh resolution

A library lockfile does not constrain downstream consumers. The published dependency requirements must themselves permit the declared MSRV.

## 3.1 Make an explicit policy decision

Preferred corrective path for this release:

```text
Preserve MSRV 1.87 and exact-pin image to a release compatible with 1.87.
```

At the current baseline, that means using an exact dependency requirement where needed:

```toml
image = { version = "=0.25.6", default-features = false, features = ["png", "jpeg", "webp"] }
```

Apply consistently to:

- root `[dependencies]`;
- root dev dependencies if separately declared;
- CLI dependencies;
- CLI dev dependencies;
- fuzz dependencies if applicable;
- test/support crates in the workspace.

Alternative path:

- raise all `rust-version` declarations to the minimum required by the selected current dependency set;
- update README, SECURITY/STABILITY documentation, CI toolchains, and release metadata;
- treat that as an intentional support-policy change.

Do not mix the paths. Record the decision in `plans/032-status.md`.

## 3.2 Add fresh-resolution MSRV consumer test

Create a script such as `scripts/validate-msrv-package.sh`.

It must:

1. package the library;
2. extract the `.crate` or use a local package source;
3. create a fresh temporary consumer crate;
4. remove any lockfile;
5. depend on the packaged candidate;
6. run dependency resolution using Rust 1.87;
7. compile with representative feature combinations;
8. prove the resolved graph contains no crate requiring a newer compiler.

Required combinations:

- default/no features;
- `async`;
- `signatures`;
- `detached-manifest`;
- all features.

Representative temporary consumer manifest:

```toml
[package]
name = "stegoeggo-msrv-consumer"
version = "0.0.0"
edition = "2021"
rust-version = "1.87"

[dependencies]
stegoeggo = { path = "/path/to/extracted/package", features = ["signatures", "detached-manifest"] }
```

The final registry smoke uses the published version instead of a path.

## 3.3 Add current-stable fresh-resolution test

A second clean consumer should resolve and build on stable without the repository lockfile.

This catches:

- missing package files;
- feature dependency mistakes;
- accidental workspace-only assumptions;
- CLI/library version divergence.

## 3.4 Validate CLI package resolution

Package the CLI and inspect its normalized Cargo.toml.

Required conditions:

- dependency on `stegoeggo` declares the intended release line/version;
- the CLI package does not accidentally resolve to an older incompatible library during smoke testing;
- the library is published before the CLI;
- the CLI install test uses the registry package, not the workspace path.

For a paired patch release, update the CLI dependency from the broad baseline form to the selected release version, for example:

```toml
stegoeggo = { path = "..", version = "0.3.2", features = ["test-seeds"] }
```

Use the actual selected version. Do not hard-code `0.3.2` if registry inspection selected a later patch.

## Phase 3 acceptance criteria

- The MSRV policy is explicit.
- Published dependency requirements, not merely the lockfile, support that policy.
- A no-lockfile consumer resolves and compiles on the declared MSRV.
- All required feature combinations compile under fresh resolution.
- A fresh stable consumer compiles.
- Packaged CLI resolution uses the intended library release.
- Main CI and RC run these tests as blocking gates.

## Suggested commits

```text
deps: make image requirement compatible with declared MSRV
ci: validate packaged crates under fresh MSRV resolution
```

---

# Phase 4: Unify main CI, RC, tag validation, and publish prerequisites

## 4.1 Define one authoritative contract

`scripts/validate-release.sh` remains the authoritative command contract.

It must include, directly or through dedicated scripts:

```text
format
clippy
workspace tests
CLI tests
doc tests
docs.rs-equivalent rustdoc
workspace package
package inventory
packaged library build
packaged CLI build/fresh install
fresh MSRV consumer
fresh stable consumer
semver
audit
deny licenses
deny advisories
fuzz target sync
external integration
strict conformance
```

No workflow may substitute a smaller duplicate set and still call itself release validation.

## 4.2 Main CI

Main CI may retain parallel jobs, but every authoritative gate must be represented.

Add or verify blocking jobs for:

- docs.rs build;
- fresh MSRV package consumer;
- package install/smoke;
- all existing correctness and security gates.

## 4.3 Release-candidate workflow

The RC workflow must:

- accept a full 40-character SHA;
- check out that exact SHA;
- prove a clean tree;
- invoke `scripts/validate-release.sh --phase all --expected-sha=<sha>`;
- upload package inventories;
- upload `.crate` checksums;
- upload docs validation summary;
- upload conformance report;
- upload tool versions;
- upload commit information.

## 4.4 Tag validation workflow

The tag workflow must not use a weaker hand-maintained command list.

Preferred implementation:

```yaml
- name: Run authoritative validation
  run: scripts/validate-release.sh --phase all --expected-sha="$(git rev-parse HEAD)"
```

It must also verify:

- tag version matches the library manifest;
- tag version matches the CLI manifest;
- tag commit equals the selected code candidate;
- selected version was unused at candidate-selection time;
- package checksums match RC artifacts where feasible.

## 4.5 Publish workflow or manual publish procedure

If publication is automated, it must depend on successful tag validation and use trusted publishing or narrowly scoped credentials.

If publication is manual, document an exact command sequence and require a clean detached checkout of the validated candidate/tag.

Never publish from a working directory with uncommitted changes.

## 4.6 Concurrency policy

Keep cancellation for ordinary branch CI if desired, but make status interpretation explicit.

Recommended:

- main CI: cancellation allowed for superseded pushes;
- RC: no cancellation by unrelated branch pushes;
- tag validation: no cancellation by main pushes;
- publish: no cancellation once publication begins.

Use distinct concurrency groups based on candidate SHA or tag.

## Phase 4 acceptance criteria

- Main CI, RC, and tag validation enforce equivalent release gates.
- No release workflow uses a weaker duplicate command list.
- Docs.rs and fresh-resolution MSRV checks are blocking everywhere required.
- RC artifacts identify the exact candidate SHA.
- Cancellation cannot interrupt an active publish due to an unrelated push.
- A cancelled main run is distinguishable from a failed release gate.

## Suggested commits

```text
ci: unify main RC and tag release validation
ci: isolate release concurrency from branch pushes
```

---

# Phase 5: Correct release documentation and support metadata

This phase is required before choosing the final candidate SHA because package metadata and included documentation are part of the published artifact.

## 5.1 Changelog

Correct `CHANGELOG.md` so it reflects reality.

Required structure should distinguish:

- `0.3.0`: published release, with actual publication date;
- `0.3.1`: docs.rs correction release if it was published;
- selected next version: unreleased until publication, then dated.

Remove or correct obsolete claims, including fixed-size v3 probing descriptions that no longer match production behavior.

For the next release, include at minimum:

- docs.rs-equivalent validation added;
- crate-root `doc_cfg` correction;
- MSRV dependency-resolution correction;
- release workflow unification;
- release metadata corrections;
- any actual deterministic CI bug fixed during Phase 1.

Do not describe cancelled runs as bugs.

## 5.2 README

Update installation examples to the current supported release line.

Example after the next release:

```toml
[dependencies]
stegoeggo = "0.3"
```

Feature examples must use the same line.

Do not point new users at `0.2` unless documenting legacy compatibility.

## 5.3 Security policy

Update supported versions truthfully.

Example:

```text
0.3.x: supported
0.2.x: security fixes only or unsupported, according to maintainer policy
<0.2: unsupported
```

Make one explicit policy choice. Do not leave stale `0.2.x`-only support text after releasing `0.3.x`.

## 5.4 Stability and deprecation documents

Verify that:

- stable API promises match `0.3.x`;
- experimental feature language remains accurate;
- MSRV statement matches the chosen policy;
- CLI exit codes match implementation;
- package version examples are current.

## 5.5 Plan ledgers

Update Plan 031 to historical/partial truth and Plan 032 to current execution status.

Do not mark Plan 032 closed before post-publication docs.rs verification.

## Phase 5 acceptance criteria

- Changelog contains accurate entries and dates.
- README points to the current release line.
- Security support table is current.
- Stability/MSRV documentation matches manifests and CI.
- Package-included documentation contains no stale release claims.
- Plan ledgers do not contradict their own tables.

## Suggested commit

```text
docs: correct 0.3 release history and support policy
```

---

# Phase 6: Select and apply the immutable next version

Do this only after Phases 0-5 are complete.

## 6.1 Reconfirm availability immediately before bump

Registry state may have changed during implementation.

Requery both packages immediately before selecting the version.

Selection algorithm:

```text
candidate version = lowest patch version greater than all existing published versions for both packages
```

If both `0.3.0` and `0.3.1` exist, the normal candidate is `0.3.2`.

If either package already has `0.3.2`, use `0.3.3` or later so the paired release version remains aligned.

## 6.2 Bump all version surfaces atomically

Update in one commit:

- root `Cargo.toml` package version;
- CLI `Cargo.toml` package version;
- CLI dependency version on `stegoeggo`;
- `Cargo.lock` workspace package versions;
- changelog unreleased heading;
- any generated version fixtures or expected output;
- release documentation referring to the pending version.

Search for stale versions:

```bash
rg -n '0\.3\.[0-9]+' \
  Cargo.toml Cargo.lock stegoeggo-cli README.md CHANGELOG.md SECURITY.md STABILITY.md plans scripts .github
```

Every match must be classified as:

- intentional historical reference;
- current selected version;
- stale and corrected.

## 6.3 Add a version-consistency check

Create a script or test that verifies:

- root package version equals CLI package version;
- CLI dependency requirement includes the selected library version;
- tag version, when present, equals both manifests;
- changelog contains the selected version;
- selected version is not already published before publish begins.

Registry availability checks may be separated from hermetic CI, but they are mandatory in RC/publish preparation.

## Phase 6 acceptance criteria

- The selected version was unused when chosen.
- All version surfaces agree.
- No already-published version is reused.
- CLI resolves against the intended library version.
- Version consistency is automatically checked.
- Version-bump commit precedes final candidate selection.

## Suggested commit

```text
release: prepare stegoeggo <selected-version>
```

For the expected state, this will normally be:

```text
release: prepare stegoeggo 0.3.2
```

---

# Phase 7: Freeze and validate one exact release candidate

## 7.1 Freeze candidate

After all code, workflow, dependency, and package-document changes are committed:

```bash
git rev-parse HEAD
```

Record the full SHA as `Code candidate SHA`.

After freezing:

- no code changes;
- no workflow changes;
- no manifest changes;
- no package-included documentation changes;
- no fixture changes.

Any such change invalidates the candidate and requires selecting a new SHA and rerunning all gates.

## 7.2 Required successful gates against the same SHA

The candidate must have successful evidence for:

1. main CI;
2. release-candidate workflow;
3. docs.rs-equivalent workspace build;
4. docs.rs-equivalent packaged-crate build;
5. fresh MSRV consumer;
6. fresh stable consumer;
7. workspace package;
8. packaged library tests/build;
9. packaged CLI install/smoke;
10. semver check;
11. cargo audit;
12. cargo deny licenses;
13. cargo deny advisories;
14. external integration;
15. strict conformance;
16. fuzz target synchronization;
17. required fuzz run;
18. version availability check.

## 7.3 Package artifacts and checksums

Produce and record:

- library `.crate` filename;
- CLI `.crate` filename;
- SHA-256 of each artifact;
- package inventory of each;
- normalized Cargo.toml inspection;
- candidate SHA;
- workflow run ID that produced them.

## 7.4 Packaged smoke matrix

From extracted packages or registry-like local source, test:

### Library

- clean consumer default features;
- async;
- signatures;
- detached manifest;
- all features;
- representative PNG/JPEG/WebP protect and verify;
- HMAC payload protect and verify;
- detached caller-key trusted verification;
- attacker key-material mismatch rejection.

### CLI

- `--help`;
- protect PNG;
- protect JPEG;
- protect WebP;
- verify protected image;
- keygen;
- sign manifest;
- trusted verify-manifest exit `0`;
- valid-untrusted exit `4`;
- integrity failure exit `3`;
- malformed configuration exit `2`.

## Phase 7 acceptance criteria

- Every gate passed against one full candidate SHA.
- No evidence comes from a different code SHA.
- Both package artifacts have recorded checksums and inventories.
- Packaged smoke tests pass independently of workspace paths.
- Candidate version remains unused immediately before publication.
- Plan status remains `PARTIAL` until post-publication verification.

## Suggested evidence commit

Evidence may be committed after candidate validation only if it changes files excluded from package contents and is clearly labeled as an evidence commit.

```text
plans: record <selected-version> release candidate evidence
```

Record both:

- code candidate SHA;
- evidence commit SHA.

Do not tag the evidence commit as the release if it differs from the validated code candidate.

---

# Phase 8: Publish without attempting version reuse

## 8.1 Final immutability guard

Immediately before each `cargo publish`, query crates.io for the exact package/version.

Required behavior:

```text
if version exists:
    stop
    bump to a new unused version
    rerun candidate validation
```

Do not continue because the existing version is yanked or has failed docs.

## 8.2 Publish order

Publish library first:

```bash
cargo publish -p stegoeggo --dry-run
cargo publish -p stegoeggo
```

Wait until the registry index can resolve the exact new library version from a clean consumer.

Then publish CLI:

```bash
cargo publish -p stegoeggo-cli --dry-run
cargo publish -p stegoeggo-cli
```

Do not publish the CLI while its library dependency is unavailable from the registry.

## 8.3 Failure handling

### Library publish rejected before acceptance

If crates.io rejects the upload and the version does not appear in the registry, correct the cause. Revalidate whether package bytes changed. If package bytes or manifest changed, choose a new candidate SHA; retain or bump the version according to registry state.

### Library publish accepted, CLI publish fails

The library version is consumed permanently.

Fix the CLI. If republishing only the CLI under the same package version is impossible because its version was already accepted, increment the CLI version. Prefer restoring aligned versions in the next paired release rather than falsifying history.

### Docs.rs fails after publication

The published version remains immutable.

Required response:

1. record the docs.rs failure log;
2. do not attempt to replace the package;
3. fix the repository;
4. select the next unused version;
5. rerun all release gates;
6. publish the corrected version.

Example:

```text
0.3.2 published -> docs.rs fails -> fix -> publish 0.3.3
```

Never attempt:

```text
0.3.2 published -> modify source -> cargo publish 0.3.2 again
```

### Severe defective release

Yank only if warranted and document the reason. Yanking prevents new normal selection but does not erase the release and does not free the version for reuse.

## 8.4 Tag and GitHub release

Create the release tag at the exact validated code candidate SHA.

Rules:

- tag name equals manifest version, e.g. `v0.3.2`;
- do not move the tag after publishing;
- GitHub release links to the exact tag;
- release notes disclose superseded defective versions where relevant;
- do not claim docs.rs success until verified.

## Phase 8 acceptance criteria

- Exact version availability was checked immediately before publish.
- No published/yanked version was reused.
- Library and CLI were published from the validated candidate.
- Registry can resolve the new library before CLI publication.
- Tag points to the candidate SHA.
- Publish logs and package checksums are recorded.
- Any failure followed the immutable-version escalation rule.

---

# Phase 9: Post-publication verification and closure

Publication alone is not closure.

## 9.1 Registry verification

From a clean temporary directory with no workspace path and no inherited lockfile:

```bash
cargo new --lib consumer
```

Depend on the exact published version and test:

- declared MSRV resolution;
- stable resolution;
- all required feature combinations;
- representative API usage.

## 9.2 CLI install verification

Install the exact version:

```bash
cargo install stegoeggo-cli --version <selected-version> --locked
```

Also test without `--locked` if that is a supported install mode, because it exercises fresh dependency resolution.

Run the packaged CLI smoke matrix from Phase 7.

## 9.3 Docs.rs verification

Wait for docs.rs to finish the exact published version build.

Record:

- docs.rs build ID or URL;
- build conclusion;
- rustc/rustdoc version;
- feature set;
- target;
- confirmation that major public modules render.

Required pages/modules include, as applicable:

- crate root;
- request API;
- verification reports;
- signatures module under feature documentation;
- detached module under feature documentation;
- payload/conformance public items intended for documentation.

Do not close the plan while docs.rs is queued, building, or failed.

## 9.4 Crates.io metadata verification

Confirm:

- README renders correctly;
- repository/documentation links are correct;
- selected version is latest unless intentionally yanked;
- dependencies show the intended MSRV-compatible requirements;
- license and categories are correct.

## 9.5 Historical release handling

If `0.3.0` or `0.3.1` remains published with known defects:

- document the issue in release notes/changelog;
- optionally yank only if the defect justifies it;
- state the minimum corrected version;
- do not delete or rewrite historical evidence.

## 9.6 Close ledgers

Only after all post-publication checks pass:

- mark Plan 032 `CLOSED`;
- record exact code candidate SHA;
- record evidence commit SHA;
- record main CI run ID;
- record RC run ID;
- record tag validation run ID;
- record fuzz run ID;
- record crate checksums;
- record crates.io versions;
- record docs.rs success;
- record clean consumer and CLI smoke results.

## Phase 9 acceptance criteria

- Exact registry version installs in a clean environment.
- Fresh dependency resolution supports the declared MSRV.
- CLI install and security smoke pass from crates.io.
- Docs.rs succeeds for the exact version.
- Metadata and README render correctly.
- Known defective historical versions are documented truthfully.
- Plan ledgers contain no open rows while marked closed.

---

# Required regression tests

The implementation must add or preserve tests for all of the following.

## Documentation tests

1. Crate-root `doc_cfg` compiles under nightly with `cfg(docsrs)`.
2. All-feature rustdoc succeeds.
3. Packaged-crate all-feature rustdoc succeeds.
4. Stable ordinary docs succeed without `cfg(docsrs)`.
5. Module-level `feature(doc_cfg)` placement is rejected by repository policy.

## MSRV tests

6. Fresh no-lockfile default-feature consumer compiles on declared MSRV.
7. Fresh no-lockfile async consumer compiles on declared MSRV.
8. Fresh no-lockfile signatures consumer compiles on declared MSRV.
9. Fresh no-lockfile detached-manifest consumer compiles on declared MSRV.
10. Fresh no-lockfile all-feature consumer compiles on declared MSRV.
11. CLI package resolves the intended library version.

## Version tests

12. Library and CLI package versions match.
13. CLI library dependency accepts/includes the selected exact release.
14. Tag version matches both manifests.
15. Changelog contains the selected release version.
16. Registry guard refuses a version that already exists.
17. Registry guard refuses a yanked existing version.

## Release tests

18. Authoritative validation includes docs.rs simulation.
19. Authoritative validation includes fresh MSRV resolution.
20. RC invokes authoritative validation.
21. Tag workflow invokes authoritative validation.
22. Packaged library builds without workspace files.
23. Packaged CLI installs without workspace paths.
24. Release candidate artifacts record SHA-256 checksums.

---

# Mandatory validation commands

Adjust exact flags only when the repository requires it; do not weaken semantics.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude stegoeggo-fuzz --all-features --no-fail-fast
cargo test -p stegoeggo-cli --all-features --no-fail-fast
cargo test --doc --workspace --exclude stegoeggo-fuzz

scripts/validate-docs-rs.sh
scripts/validate-msrv-package.sh
scripts/check_fuzz_sync.sh

cargo package -p stegoeggo
cargo package -p stegoeggo-cli
cargo package --workspace

cargo semver-checks check-release
cargo audit
cargo deny check licenses
cargo deny check advisories

cargo test --test external_tools -- --ignored
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --json conformance-report.json

scripts/validate-release.sh \
  --phase all \
  --expected-sha="$(git rev-parse HEAD)"
```

Docs.rs-equivalent core:

```bash
DOCS_RS=1 \
RUSTDOCFLAGS="--cfg docsrs --check-cfg=cfg(docsrs) -D warnings" \
  cargo +nightly rustdoc \
    -p stegoeggo \
    --all-features \
    --no-deps
```

Version inventory search:

```bash
rg -n '0\.3\.[0-9]+' \
  Cargo.toml Cargo.lock stegoeggo-cli README.md CHANGELOG.md \
  SECURITY.md STABILITY.md plans scripts .github
```

Feature-gate placement search:

```bash
rg -n 'feature\(doc_cfg\)' src
```

Expected final result: only valid crate-root placement.

---

# Suggested commit sequence

Use small, reviewable commits.

1. `plans: reopen release closure and record immutable version state`
2. `plans: record post-publish CI failure evidence`
3. `docs: add docs.rs-equivalent workspace and package validation`
4. `ci: make docs.rs-equivalent build release-blocking`
5. `deps: make image requirement compatible with declared MSRV`
6. `ci: validate packaged crates under fresh MSRV resolution`
7. `ci: unify main RC and tag release validation`
8. `ci: isolate release concurrency from branch pushes`
9. `docs: correct 0.3 release history and support policy`
10. `release: prepare stegoeggo <selected-version>`
11. `plans: record <selected-version> release candidate evidence`
12. `plans: record <selected-version> post-publication closure`

Do not combine the version bump with untested production fixes.

Do not combine publication evidence with the code candidate commit.

---

# Final definition of done

Plan 032 is complete only when every item below is true.

1. Registry state for `0.3.0` and `0.3.1` is recorded for both packages.
2. The next release uses a previously unused version.
3. Published or yanked versions are never reused.
4. Plan 031 no longer overclaims release-pipeline closure.
5. Every current CI run is classified as success, failure, cancellation, or infrastructure/transient failure.
6. Every actual failure has a recorded root cause.
7. The docs.rs failure is covered by a regression gate.
8. Docs.rs-equivalent rustdoc runs on nightly with `DOCS_RS=1` and `cfg(docsrs)`.
9. Packaged-crate docs.rs-equivalent rustdoc passes.
10. Module-level `feature(doc_cfg)` cannot regress unnoticed.
11. The declared MSRV is supported by published dependency requirements.
12. A fresh no-lockfile consumer compiles on the declared MSRV.
13. All required feature combinations compile under fresh MSRV resolution.
14. A fresh stable consumer compiles.
15. CLI package resolution uses the intended library release.
16. Main CI contains every release-blocking gate.
17. RC invokes the authoritative validation script against an exact SHA.
18. Tag validation invokes the same authoritative contract.
19. Release/publish concurrency cannot be cancelled by unrelated main pushes.
20. Changelog accurately records `0.3.0`, `0.3.1` if published, and the next release.
21. README installation examples use the current release line.
22. Security support policy includes the maintained `0.3.x` line.
23. All package versions and dependency versions agree.
24. Candidate version availability is checked immediately before publication.
25. One exact code candidate passes main CI, RC, docs, MSRV, package, security, conformance, and fuzz gates.
26. Library and CLI `.crate` artifacts have recorded SHA-256 checksums.
27. Packaged library and CLI smoke tests pass independently of workspace paths.
28. Library publishes before CLI and is registry-resolvable first.
29. The release tag points to the validated code candidate SHA.
30. The release tag is not moved after publication.
31. Clean crates.io consumer tests pass for the exact version.
32. Clean CLI installation and exit-code/security smoke tests pass.
33. Docs.rs succeeds for the exact published version.
34. Crates.io metadata and README render correctly.
35. Defective historical releases are documented truthfully and optionally yanked only with justification.
36. `plans/032-status.md` contains full run IDs, SHAs, checksums, versions, and post-publication evidence.
37. No table contains `OPEN` or `PARTIAL` rows when disposition is `CLOSED`.
38. The final ledger explicitly reiterates that the next correction after any published failure requires another new version.

---

# Reviewer rejection conditions

Reject implementation and keep the release hold active if any of the following is true:

- the agent attempts to republish `0.3.0` or any existing `0.3.1`;
- the agent assumes a yanked version can be reused;
- docs validation consists only of `cargo test --doc`;
- docs validation runs only from the workspace and not the packaged crate;
- MSRV validation relies only on the repository lockfile;
- `image = "0.25.6"` is described as an exact pin without the leading `=`;
- the declared MSRV changes without documentation and CI changes;
- current CI cancellations are misreported as deterministic failures;
- a release check becomes non-blocking;
- main CI, RC, and tag validation enforce materially different contracts;
- the CLI package can resolve an unintended older library version during release smoke;
- package-included documentation remains stale;
- a candidate is tagged before all pre-publication gates pass;
- an evidence-only commit is tagged instead of the validated code candidate;
- docs.rs is still pending when the plan is marked closed;
- any Plan 032 table contains open work while the plan says `CLOSED`.

---

# Handoff summary

The next implementer must treat this as a release-pipeline correction, not as an ordinary retry of the prior publication.

The expected sequence is:

```text
confirm published versions
-> choose first unused version (normally 0.3.2)
-> capture current CI failure evidence
-> add docs.rs-equivalent gate
-> fix fresh-resolution MSRV correctness
-> unify main/RC/tag validation
-> correct release metadata
-> bump all package/version surfaces
-> freeze exact candidate SHA
-> pass every pre-publication gate
-> confirm version still unused
-> publish library
-> verify registry resolution
-> publish CLI
-> verify crates.io installs and docs.rs
-> close ledger
```

At no point may a published package version be overwritten. Every post-publication correction consumes a new version number.