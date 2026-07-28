# Plan 035: Collapse Required CI and Demote Specialist Verification

Status: Ready for implementation

Baseline: `main` after Plan 034 plan commit `37482fca571001540a18a673d9ceb5222760c834`

Depends on:

- `plans/033-ci-verification-and-manual-release-simplification-roadmap.md`
- completion of `plans/034-remove-automated-release-and-decouple-tags.md`

Must be completed before:

- `plans/036-local-verification-policy-and-documentation-closure.md`

---

## Purpose

Reduce ordinary GitHub Actions verification to one stable-toolchain job while retaining specialist verification as deliberate, manually invoked capability.

The target is fast and comprehensible feedback for normal development. This plan removes automatic fan-out, duplicated test execution, cargo-plugin installation, operating-system package installation, artifact collection, nightly fuzz matrices, and release-grade checks from push and pull-request CI.

This plan changes execution policy, not product correctness policy. Existing deterministic tests, conformance fixtures, fuzz targets, and diagnostic scripts should generally remain available.

---

## Baseline workflow inventory

At the reviewed baseline, `.github/workflows/ci.yml` contains these logical jobs:

```text
MSRV Check
Docs.rs Build
MSRV Package Consumer
Package Check
Test
Lint
Security Audit
Semver Check
License & Advisory Check
External Integration
External Conformance
Feature Matrix: no-default
Feature Matrix: async
Feature Matrix: signatures
Feature Matrix: detached-manifest
Feature Matrix: all-features
Benchmark (manual only)
```

The current test job also runs CLI tests separately even though the CLI is a workspace member and the workspace command already includes it.

`.github/workflows/fuzz.yml` launches one job per fuzz target on every push and pull request. At the baseline there are twelve targets, producing twelve additional nightly-toolchain jobs for ordinary changes.

The resulting normal change fan-out is approximately twenty-eight jobs.

---

## Required end state

### Required workflow

Exactly one workflow runs automatically for pushes and pull requests to `main`.

Recommended path:

```text
.github/workflows/ci.yml
```

Recommended structure:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

permissions:
  contents: read

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      - run: cargo check -p stegoeggo --no-default-features
      - run: cargo test --workspace --exclude stegoeggo-fuzz --all-features
```

A separate doctest command may be retained only after verifying that it adds real coverage not already exercised by the workspace command:

```bash
cargo test --doc -p stegoeggo --all-features
```

The implementation agent may later have CI invoke `./scripts/check.sh` after Plan 036 creates it. During this plan, direct commands are acceptable and preferable to retaining the current large validator.

### Required properties

The required job must have:

- one operating system: Ubuntu;
- one Rust toolchain: stable;
- no matrix;
- no `cargo install`;
- no `apt-get`;
- no artifact upload;
- no release-mode build unless a specific existing test requires it;
- no semver comparison;
- no network advisory scan beyond ordinary Cargo dependency resolution;
- no external command-line image tools;
- no docs.rs simulation;
- no fresh packaged MSRV consumer generation;
- no fuzz execution;
- no benchmark execution;
- no package publication or release behavior.

---

## Phase 0: Create the status ledger and measure the baseline

Create `plans/035-status.md` before editing workflows.

Initialize:

```text
Plan baseline SHA: <actual implementation baseline SHA>
Disposition: OPEN
Automatic push/PR workflows: 2
Required push/PR jobs: approximately 28
Required matrices: feature matrix + fuzz target matrix
Required toolchains: stable + MSRV + nightly
Required cargo-installed tools: cargo-audit + cargo-semver-checks + cargo-deny + cargo-fuzz
Required OS packages: ExifTool + xmllint + ImageMagick + libvips
```

Record exact counts from the actual baseline rather than copying the approximation blindly.

Add these tables.

### Table A: automatic workflow triggers

```text
workflow | push | pull_request | schedule | workflow_dispatch | tag | automatic job count
```

### Table B: required job purpose and disposition

```text
job | unique correctness purpose | duplicates another job | external dependency | target disposition | reason
```

### Table C: command coverage map

```text
correctness concern | old command/job | new required command | optional/manual command | coverage retained
```

Required concerns:

- formatting;
- clippy;
- workspace tests;
- CLI tests;
- minimal/default feature compilation;
- maximal/all-feature behavior;
- doctests;
- packaging;
- MSRV;
- docs.rs;
- semver;
- licenses;
- advisories;
- external integration;
- conformance;
- fuzzing;
- benchmarks.

### Phase 0 acceptance criteria

- Exact pre-change job and trigger counts are recorded.
- Every existing job has a disposition.
- The coverage map shows that specialist capabilities are being moved rather than silently forgotten.
- Required CI target name is selected, preferably `Check`.
- Branch-protection migration implications are recorded.

Suggested commit:

```text
plans: inventory CI fan-out and coverage disposition
```

---

## Phase 1: Replace the primary CI graph with one job

Rewrite `.github/workflows/ci.yml` instead of incrementally disabling jobs.

### 1.1 Preserve appropriate triggers

Retain only:

```yaml
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
```

`workflow_dispatch` may be omitted from the required workflow because manually rerunning normal CI is already supported through GitHub’s rerun interface. Retain it only if there is a demonstrated maintainer need and it adds no extra jobs.

Do not add tags or schedules.

### 1.2 Retain concurrency cancellation

Use one concurrency group per workflow/ref and cancel superseded runs. Rapid iteration should not queue stale checks.

Recommended:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

### 1.3 Use read-only permissions

Add:

```yaml
permissions:
  contents: read
```

No write permission is needed.

### 1.4 Install one stable toolchain

Use stable Rust with `rustfmt` and `clippy` components. Do not install MSRV or nightly in the required workflow.

### 1.5 Run the minimum complete command set

Required commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
```

Rationale:

- formatting catches mechanical drift;
- clippy checks all workspace targets and all features with warnings denied;
- minimal library compilation catches accidental unconditional references to optional dependencies;
- one all-feature workspace test pass covers library, CLI, integration tests, examples compiled as targets where applicable, and feature-gated behavior;
- excluding the fuzz crate avoids nightly-only workspace behavior.

### 1.6 Remove duplicate CLI execution

Do not retain:

```bash
cargo test -p stegoeggo-cli --all-features
```

unless empirical command listing proves the workspace test command does not execute the CLI crate’s tests. Verify with one of:

```bash
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- --list
cargo test -p stegoeggo-cli --all-features -- --list
```

Compare the CLI test names. Record the result in `plans/035-status.md`.

If workspace tests do include all CLI tests, separate execution is prohibited as duplication.

If they do not, document why and add only the missing coverage, preferably by correcting workspace membership or test configuration rather than permanently duplicating the entire crate test pass.

### 1.7 Decide doctest treatment empirically

Run:

```bash
cargo test --workspace --exclude stegoeggo-fuzz --all-features -- --list
cargo test --doc -p stegoeggo --all-features -- --list
```

If doctests are not executed by the workspace test command and meaningful doctests exist, retain one focused doctest command.

Do not run doc tests for every workspace package separately unless necessary.

### Phase 1 acceptance criteria

- `ci.yml` contains exactly one job.
- The job uses stable Rust only.
- No matrix exists.
- No `cargo install` occurs.
- No OS packages are installed.
- No artifacts are uploaded.
- The four required commands run.
- Duplicate CLI testing is removed or explicitly justified with test-list evidence.
- Doctest treatment is evidence-based.
- The workflow is understandable without reading another large orchestration script.

Suggested commit:

```text
ci: collapse required verification to one stable job
```

---

## Phase 2: Remove specialist checks from required CI

This phase is mostly accomplished by the rewrite in Phase 1, but the status ledger must explicitly classify each removed job.

### 2.1 MSRV checks

Remove from automatic CI:

- `MSRV Check`;
- `MSRV Package Consumer`.

Do not change `rust-version = "1.87"` in this plan.

MSRV remains a targeted local check for dependency or MSRV changes and before release. Plan 036 will define the command.

### 2.2 docs.rs checks

Remove `Docs.rs Build` from automatic CI.

Retain `scripts/validate-docs-rs.sh` temporarily unless Plan 036 determines it should be removed or simplified. It becomes a targeted diagnostic for changes to:

- crate-level documentation attributes;
- `cfg(docsrs)`;
- public feature-gated API documentation;
- `[package.metadata.docs.rs]`;
- package include/exclude behavior.

### 2.3 package checks

Remove `Package Check` from automatic CI.

Packaging remains part of local pre-release verification.

### 2.4 security and license jobs

Remove automatic jobs for:

- `cargo audit`;
- `cargo semver-checks`;
- `cargo deny check licenses`;
- `cargo deny check advisories`.

Do not delete `deny.toml` solely because the job is removed. Plan 036 will decide the retained local policy.

Avoid retaining both `cargo audit` and `cargo deny check advisories` as mandatory release checks without explicit justification; they overlap through RustSec advisory data.

### 2.5 feature matrix

Remove the full feature matrix.

Retain only:

```bash
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
```

This is sufficient for the normal additive feature model unless a known interaction requires an isolated combination.

If a specific feature combination has a historical regression not reproduced by minimal or all-feature configurations, convert that regression into a deterministic test gated appropriately rather than retaining a five-run matrix.

### 2.6 benchmarks

Remove the benchmark job from `ci.yml`.

Benchmarks remain local:

```bash
cargo bench
```

Do not create a separate benchmark workflow in this plan unless one already exists outside `ci.yml` and has a demonstrated use. The roadmap prefers local benchmarks.

### Phase 2 acceptance criteria

- No specialist job remains in required CI.
- The status ledger assigns each removed capability a local/manual cadence.
- MSRV declaration is unchanged.
- Feature correctness retains minimal and maximal coverage.
- No benchmark runs automatically.
- No replacement matrix is introduced.

---

## Phase 3: Consolidate external interoperability into one manual path

External interoperability is important to the product but should not run automatically on every change.

Choose one of these two acceptable outcomes.

### Preferred outcome: one manual workflow

Create or rewrite a workflow such as:

```text
.github/workflows/external-verification.yml
```

Trigger:

```yaml
on:
  workflow_dispatch:
```

One job only. It may:

1. checkout the repository;
2. install stable Rust;
3. restore Cargo cache;
4. install ExifTool, xmllint, ImageMagick, and libvips once;
5. run ignored external integration tests;
6. build and execute the conformance harness.

Illustrative commands:

```bash
cargo test --test external_tools -- --ignored
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict
```

Do not create separate external-integration and conformance jobs. They share setup and should run sequentially.

Artifact policy:

- do not upload tool-version manifests routinely;
- do not upload conformance reports on every success;
- an artifact on failure is optional if it materially aids diagnosis;
- console output is sufficient for normal manual execution.

### Acceptable outcome: local-only external verification

Delete automatic external jobs and document the same local commands without retaining a workflow.

Choose this outcome if the project does not need remote external-tool validation.

### Trigger prohibition

Whether manual workflow or local-only:

- no push trigger;
- no pull-request trigger;
- no schedule;
- no tag trigger.

### Phase 3 acceptance criteria

- External integration and conformance do not run automatically.
- They remain runnable through one deliberate path.
- External tools are installed at most once per invocation.
- No duplicate jobs exist.
- No routine evidence artifact bureaucracy is retained.
- The conformance fixtures and harness remain intact.

Suggested commit:

```text
ci: make external interoperability verification manual
```

---

## Phase 4: Convert fuzzing to local or single-target manual execution

The fuzz targets are valuable, but launching every target on every push and pull request is disproportionate.

Choose one of these outcomes.

### Preferred outcome: single-target manual workflow

Rewrite `.github/workflows/fuzz.yml` with `workflow_dispatch` only and an input selecting one target.

Example design:

```yaml
name: Fuzz

on:
  workflow_dispatch:
    inputs:
      target:
        description: Fuzz target to run
        required: true
        type: choice
        options:
          - pipeline_bytes
          - jpeg_parser
          - payload_v3_parser
          - tiled_round_trip
          - png_metadata
          - webp_riff_parser
          - xmp_extract
          - metadata_merge
          - detached_manifest_parse
          - detached_manifest_verify
          - provenance_canonicalize
          - verification_report
      seconds:
        description: Maximum run duration
        required: false
        default: "60"
```

One job runs the selected target. Validate the input before passing it to the shell. For a choice input, compare against an allowlist or rely on the declared choices while still quoting values.

Illustrative command:

```bash
cargo fuzz run "${{ inputs.target }}" -- \
  -max_total_time="${{ inputs.seconds }}" \
  -max_len=4096 \
  -timeout=10
```

Validate that `seconds` is a bounded positive integer before use. A reasonable upper bound for the workflow is 3600 seconds.

Do not retain a matrix.

Do not retain a weekly schedule.

Upload crash artifacts only when the fuzz run fails and a corpus/crash file exists.

### Acceptable outcome: local-only fuzzing

Delete `.github/workflows/fuzz.yml` and document:

```bash
cargo +nightly fuzz run <target> -- -max_total_time=60 -max_len=4096 -timeout=10
```

The fuzz crate and all targets remain.

### Fuzz-target synchronization

If `scripts/check_fuzz_sync.sh` exists, keep it as a local maintenance diagnostic. It should not be part of required push/PR CI unless it executes nearly instantly and catches a concrete manifest mismatch. The default disposition in this plan is to remove it from required CI.

### Phase 4 acceptance criteria

- Fuzzing has no push, pull-request, schedule, or tag trigger.
- No fuzz matrix exists.
- At most one target runs per manual workflow invocation.
- All existing fuzz targets remain available unless a target is independently proven obsolete.
- Crash artifacts are conditional rather than routine.
- Local invocation is documented for Plan 036.

Suggested commit:

```text
ci: make fuzzing single-target and manual
```

---

## Phase 5: Verify workflow count and command coverage

Run static checks:

```bash
find .github/workflows -maxdepth 1 -type f -print | sort
rg -n "pull_request:|push:|schedule:|workflow_dispatch:|matrix:|cargo install|apt-get|upload-artifact" .github/workflows
```

Expected result:

- only `ci.yml` has push and pull-request triggers;
- `ci.yml` has one job and no matrix;
- optional workflows use only `workflow_dispatch`;
- `ci.yml` contains no cargo installs, apt installs, or artifact uploads;
- fuzzing is not automatic;
- external verification is not automatic.

Run the final required command set locally:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
```

Run focused doctests if retained.

Run workflow YAML validation through GitHub Actions or an already available parser.

Exercise each optional workflow path once if practical. If external tools are unavailable locally, dispatch the manual external workflow and record the run. If neither is possible, record the exact blocker rather than claiming success.

### Coverage audit

Verify that removal of the feature matrix did not remove compilation of feature-gated modules from the all-feature test pass.

Verify that the minimal check compiles without optional dependencies.

Verify that CLI tests are present in the workspace run.

Verify that ordinary integration tests remain selected.

### Phase 5 acceptance criteria

- Required local commands pass.
- Required GitHub CI passes as one job.
- Optional workflows are syntactically valid.
- CLI coverage is demonstrated in the workspace command.
- Minimal and maximal feature configurations are both covered.
- Before-and-after automatic job counts are recorded.

---

## Phase 6: Branch-protection migration handoff

The source change may rename or remove required status checks. Record:

```text
old required check names | new required check name | source commit | maintainer setting action | verified
```

Likely old checks include names such as:

```text
MSRV Check
Docs.rs Build
MSRV Package Consumer
Package Check
Test
Lint
Security Audit
Semver Check
License & Advisory Check
External Integration
External Conformance
Feature Matrix (...)
Fuzz Smoke Test (...)
```

The target new required check is:

```text
Check
```

Do not restore obsolete jobs merely because branch protection still expects them. Update repository settings instead.

If the connector or implementation environment can inspect branch protection, record the exact configured checks. If not, leave a precise maintainer checklist.

### Phase 6 acceptance criteria

- Old and new check names are documented.
- No compatibility shim jobs exist solely to satisfy stale settings.
- Maintainer action is explicit.
- Status ledger does not claim settings were changed without verification.

---

## Phase 7: Close the status ledger

Update `plans/035-status.md` with exact before-and-after metrics:

```text
Disposition: CLOSED
Automatic push/PR workflows: 1
Required push/PR jobs: 1
Required matrices: 0
Required Rust toolchains: stable only
Required cargo-installed tools: 0
Required OS packages: 0
Required artifact uploads: 0
Automatic fuzz jobs: 0
Automatic external-tool jobs: 0
```

Record:

- final required workflow path;
- final required check name;
- optional workflow paths;
- required command output summaries;
- GitHub Actions run ID for the simplified CI;
- optional workflow exercise evidence;
- branch-protection follow-up status;
- implementation commit SHAs.

---

## Non-goals

Do not:

- delete deterministic product tests to shorten CI;
- remove conformance fixtures;
- remove fuzz targets solely because automatic execution is removed;
- change the crate MSRV;
- publish crates;
- change crate versions;
- add coverage-reporting services;
- add code-scanning suites;
- add another operating-system matrix;
- add Windows or macOS CI as part of this simplification;
- add dependency bots or scheduled security automation;
- create a new release pipeline;
- add a single mandatory script that internally runs every removed specialist check.

---

## Failure handling

### If the single CI job is still slow

Measure individual command duration. Do not immediately split it back into parallel jobs. Optimize test selection, caching, or test internals only when supported by evidence.

### If all-feature tests exceed practical limits

Identify the slow test modules and determine whether they are deterministic regressions, ignored external tests, or accidental stress tests. Do not remove coverage globally. Move genuinely specialist or long-running cases behind explicit invocation only when their purpose supports that classification.

### If an individual feature combination fails but all-features passes

Add a focused compile check or regression test only for that demonstrated incompatibility. Do not restore the entire generic matrix.

### If docs.rs previously failed

Retain the targeted docs.rs script and run it before releases or when documentation configuration changes. Do not restore a permanent nightly CI job by default.

### If advisory scanning finds a vulnerability between releases

Address it when intentionally running the local advisory check. This roadmap accepts that continuous advisory monitoring is not a required GitHub CI responsibility for this project.

### If manual external verification fails because of tool-version drift

Classify whether the product or external tool changed. Keep the failure local to the manual workflow unless it exposes a deterministic product defect. Do not make all ordinary iteration depend on external package repositories.

---

## Definition of done

Plan 035 is complete when:

- exactly one workflow runs automatically on pushes and pull requests;
- that workflow has one job named `Check` or another explicitly documented single check;
- it uses stable Rust only;
- it has no matrix;
- it installs no cargo subcommands;
- it installs no operating-system packages;
- it uploads no artifacts;
- it runs formatting, strict clippy, minimal-feature compilation, and all-feature workspace tests;
- duplicate CLI test execution is removed;
- doctest treatment is evidence-based;
- external interoperability remains available but is manual only;
- fuzzing remains available but is local or single-target manual only;
- package, MSRV, docs.rs, semver, advisory, license, and benchmark checks are absent from required CI;
- branch-protection migration is documented;
- simplified CI passes;
- no product release occurs during implementation.
