# Plan 037: CI and Manual Release Simplification Corrective Closure

Status: Ready for implementation

Baseline: `main` at `ce1b3ef2661a348a494c8fe3c3389eea5a5ec9da`

Depends on:

- `plans/033-ci-verification-and-manual-release-simplification-roadmap.md`
- `plans/034-remove-automated-release-and-decouple-tags.md`
- `plans/035-collapse-required-ci-and-demote-specialist-verification.md`
- `plans/036-local-verification-policy-and-documentation-closure.md`

This plan is the narrow corrective and evidence-closure pass for Plans 033 through 036.

---

## Purpose

The principal simplification work landed in the intended direction:

- automated crates.io publication was removed from GitHub Actions;
- tag-triggered validation and release mutation were removed;
- ordinary push and pull-request CI was reduced to one stable Ubuntu job;
- the required CI job invokes a small deterministic `scripts/check.sh` interface;
- external interoperability and fuzzing were retained as deliberate manual workflows;
- release cadence and publication were assigned to maintainers using direct Cargo/crates.io commands;
- `scripts/validate-release.sh` was removed;
- `RELEASING.md` and CI-complexity guardrails were added.

The line of work is not yet fully closed because several residual correctness and evidence gaps remain:

1. `scripts/release-check.sh` converts every CLI packaging failure into an expected warning, so an unrelated packaging defect can be reported as a successful release check.
2. `scripts/check_fuzz_sync.sh` still parses the former fuzz matrix layout and does not correctly parse the current `workflow_dispatch.inputs.target.options` list.
3. `scripts/validate-msrv-package.sh` exposes an `MSRV` variable but invokes the hard-coded `cargo +1.87` toolchain and does not explicitly disable default features for the minimal consumer.
4. `scripts/validate-docs-rs.sh` labels an unqualified `cargo doc` invocation as a stable-toolchain check and both specialist scripts select the first Cargo metadata package rather than the package named `stegoeggo`.
5. `AGENTS.md` overstates which specialist checks are available through manual-dispatch workflows; several are local-only commands.
6. The required `plans/034-status.md`, `plans/035-status.md`, and `plans/036-status.md` evidence ledgers were never created.
7. A green run of the final one-job CI workflow and smoke evidence for the two manual workflows have not been recorded in the planning system.
8. Repository-level cleanup remains unverified: obsolete Actions secrets, environments, required status checks, tag rules, and publication-related rulesets may still exist outside source control.

This plan corrects only those residuals. It must not redesign the simplified CI architecture or restore any removed release machinery.

---

# Target end state

At completion, the repository must satisfy all of the following.

## Required CI

```text
Automatic push/PR workflows: 1
Automatic push/PR jobs: 1
Required check name: Check
Required runner OS: Ubuntu
Required Rust toolchains: stable only
Required matrices: 0
Required cargo-installed tools: 0
Required OS packages: 0
Required artifact uploads: 0
```

The required job continues to invoke only:

```bash
./scripts/check.sh
```

and `scripts/check.sh` continues to contain only:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
```

## Release model

```text
Automated crates.io publication paths: 0
Tag-triggered release workflows: 0
GitHub Actions crates.io token references: 0
GitHub Actions GitHub-release creation paths: 0
Release-candidate workflow dependencies: 0
Publication owner: maintainer
Publication mechanism: direct cargo publish commands
```

## Local verification model

```text
Fast deterministic verification: scripts/check.sh
Bounded pre-release readiness: scripts/release-check.sh
Specialist diagnostics: targeted local scripts/commands or explicit manual workflows
Publication side effects from verification scripts: 0
```

## Evidence model

```text
Plan 034 retrospective status ledger: present
Plan 035 retrospective status ledger: present
Plan 036 retrospective status ledger: present
Plan 037 status ledger: present and closed
Final one-job CI run evidence: recorded
Manual external workflow smoke evidence: recorded or exact blocker recorded
Manual fuzz workflow smoke evidence: recorded or exact blocker recorded
Repository settings state: verified or explicitly maintainer-pending
```

---

# Non-goals

Do not expand this corrective pass into unrelated work.

Specifically, do not:

- add or restore any automated publication workflow;
- add a `workflow_dispatch` publication action;
- add tag-triggered validation;
- add release-candidate certification;
- restore a multi-phase authoritative release validator;
- add another required CI job;
- add a required feature matrix;
- add scheduled fuzzing, advisory scans, benchmarks, or dependency checks;
- move docs.rs, MSRV, semver, conformance, fuzzing, or advisory checks into required CI;
- add Windows or macOS CI;
- delete deterministic product tests;
- delete conformance fixtures;
- delete fuzz targets merely because they are manual;
- change image, metadata, steganography, signing, provenance, or public API behavior;
- change the crate MSRV;
- bump crate versions;
- publish either crate;
- create, move, or delete a release tag;
- create a GitHub release;
- add a new command runner such as Make, Just, Task, or xtask;
- add a shell-testing framework solely for this pass;
- add SBOM, attestation, signing, inventory, or release-evidence frameworks;
- rewrite historical plans to pretend the former CI/release system did not exist.

The desired result is a small correctness and evidence correction, not a second CI redesign.

---

# Mandatory execution rules

1. Work phases in order unless an earlier phase explicitly permits parallel work.
2. Create `plans/037-status.md` before modifying scripts or current documentation.
3. Create the missing retrospective ledgers without inventing evidence.
4. Use full 40-character commit SHAs in ledgers.
5. Preserve the one-workflow/one-job required CI architecture.
6. Do not add specialist commands to `scripts/check.sh`.
7. Do not make `scripts/release-check.sh` publish, tag, push, create releases, install tools, or dispatch workflows.
8. Do not tolerate arbitrary command failures based on broad warning text.
9. Do not parse Cargo error wording as the primary correctness mechanism when a deterministic package-assembly path is available.
10. Keep specialist scripts single-purpose and local/manual.
11. Do not claim GitHub repository settings were changed unless directly verified.
12. Do not restore obsolete checks merely because branch protection or a ruleset still references them; update the setting instead.
13. Do not use `continue-on-error` to manufacture green evidence.
14. Do not publish or create a test tag while validating this plan.
15. Every negative test must run in a temporary copy or temporary worktree and must not leave the primary working tree dirty.
16. A phase may not be marked `CLOSED` without source changes, command evidence, and the implementing commit SHA.
17. If a required external prerequisite is unavailable, record the exact command and blocker; do not falsely record success.
18. Source completion and repository-settings completion must be tracked separately.

---

# Verified baseline facts

The implementation agent must begin from these repository facts and revalidate them against the actual baseline.

1. Current implementation head is `ce1b3ef2661a348a494c8fe3c3389eea5a5ec9da`.
2. `.github/workflows/ci.yml` has one automatic job named `Check`.
3. The required job invokes `./scripts/check.sh`.
4. `scripts/check.sh` contains four deterministic Cargo commands and no specialist or publication actions.
5. `.github/workflows/external-verification.yml` is `workflow_dispatch` only.
6. `.github/workflows/fuzz.yml` is `workflow_dispatch` only and selects one target through an input choice.
7. `.github/workflows/release-candidate.yml` is absent.
8. `.github/workflows/release.yml` is absent.
9. `.github/workflows/publish.yml` is absent.
10. `scripts/validate-release.sh` is absent.
11. `scripts/release-check.sh` currently swallows every non-zero CLI package command and prints an expected-warning message.
12. `scripts/check_fuzz_sync.sh` still searches for the old matrix target layout.
13. `scripts/validate-msrv-package.sh` defines `MSRV` but invokes `cargo +1.87` directly.
14. The minimal MSRV consumer does not set `default-features = false`.
15. `scripts/validate-docs-rs.sh` invokes unqualified `cargo doc` in a phase labeled stable.
16. The docs.rs and MSRV scripts currently derive the version from `metadata["packages"][0]`.
17. `AGENTS.md` states that specialist checks including MSRV, docs.rs, packaging, semver, and benchmarks are available as manual-dispatch workflows even though only external verification and fuzzing have workflows.
18. `plans/034-status.md`, `plans/035-status.md`, and `plans/036-status.md` are absent.
19. The current source does not itself prove whether obsolete GitHub secrets, environments, branch protection checks, or rulesets remain.

If any fact has changed before implementation begins, record the new baseline and adapt narrowly rather than applying stale edits blindly.

---

# Phase 0: Establish the corrective ledger and retrospective truth

## 0.1 Create `plans/037-status.md`

Create the file before production edits.

Initialize it with:

```text
Plan baseline SHA: ce1b3ef2661a348a494c8fe3c3389eea5a5ec9da
Disposition: OPEN
Source architecture: simplified and retained
Release-check correctness: OPEN
Fuzz synchronization correctness: OPEN
MSRV diagnostic correctness: OPEN
Docs.rs diagnostic correctness: OPEN
Operational documentation truth: OPEN
Plans 034-036 retrospective ledgers: OPEN
Final CI evidence: OPEN
Manual workflow smoke evidence: OPEN
Repository settings: UNVERIFIED
Publication hold: no publication is part of this plan
```

Add the following tables.

### Table A: corrective issue inventory

```text
issue | path | observed behavior | required behavior | implementation commit | verification | status
```

Required rows:

- arbitrary CLI package failure is treated as expected;
- CLI package contents are not inspected;
- fuzz sync parser targets old matrix layout;
- MSRV variable is ignored by the command;
- minimal MSRV consumer does not disable default features;
- docs stable phase does not force stable;
- Cargo metadata package selection is order-dependent;
- AGENTS specialist-workflow wording is inaccurate;
- Plans 034-036 status ledgers are missing;
- final CI and manual workflow evidence are missing;
- repository settings are unverified.

### Table B: verification command evidence

```text
command | purpose | expected result | actual result | date | commit SHA | status
```

### Table C: GitHub Actions evidence

```text
workflow | event | run ID | head SHA | jobs | conclusion | URL/evidence | status
```

Required workflow rows:

- `CI`;
- `External Verification`;
- `Fuzz`.

### Table D: repository settings

```text
setting | expected simplified value | observed value | action | directly verified | status
```

Required rows:

- Actions secret `CARGO_REGISTRY_TOKEN`;
- environment `crates-io`;
- environment reviewers/protection rules used only for publication;
- branch-protection required check names;
- repository rulesets requiring deleted workflows;
- tag rules requiring release validation;
- Actions default token permission;
- directly referenced reusable workflows capable of publication.

### Table E: final complexity metrics

```text
metric | target | observed | evidence | status
```

Required metrics:

- automatic push/PR workflows;
- automatic push/PR jobs;
- required matrices;
- required toolchains;
- required cargo-installed tools;
- required OS packages;
- required artifacts;
- automatic fuzz jobs;
- automatic external-tool jobs;
- tag-triggered workflows;
- automated publication paths;
- workflow token references.

## 0.2 Create retrospective `plans/034-status.md`

This ledger must truthfully record the already-landed Plan 034 implementation.

Minimum content:

```text
Plan baseline SHA: d2f6dfc7021a3286949df96e6e7c97460149189f
Implementation commit: 0a0b3112f5adfb700ab80b88ee2509c714c66831
Source disposition: CLOSED
Repository settings disposition: UNVERIFIED or PARTIAL
```

Record the three deleted paths and their prior responsibilities:

```text
.github/workflows/release-candidate.yml
.github/workflows/release.yml
.github/workflows/publish.yml
```

Record static source conclusions:

- no current workflow publishes crates;
- no current workflow creates GitHub releases;
- no current workflow is triggered specifically by version tags;
- no current workflow depends on RC evidence;
- no current workflow references `CARGO_REGISTRY_TOKEN`;
- manual release policy is present.

Do not mark repository secrets, environments, branch protection, or rulesets closed until Phase 6 verifies them.

## 0.3 Create retrospective `plans/035-status.md`

Minimum content:

```text
Implementation commit: c04ee0dec10b03e58891cba914748175307a3777
Source disposition: CLOSED
CI run evidence disposition: OPEN until Phase 5
Branch-protection disposition: UNVERIFIED until Phase 6
```

Record before-and-after metrics:

```text
automatic push/PR workflows: approximately 2 -> 1
required push/PR jobs: approximately 28 -> 1
required matrices: feature + fuzz -> 0
required toolchains: stable + MSRV + nightly -> stable only
required cargo-installed tools: multiple -> 0
required OS packages: external image tools -> 0
required artifacts: multiple -> 0
```

Record the retained manual paths:

```text
.github/workflows/external-verification.yml
.github/workflows/fuzz.yml
```

Do not record a green CI run until one exact run is observed.

## 0.4 Create retrospective `plans/036-status.md`

Minimum content:

```text
Implementation commit: ce1b3ef2661a348a494c8fe3c3389eea5a5ec9da
Source disposition: PARTIAL pending Plan 037 corrections
Repository settings disposition: UNVERIFIED
```

Record completed work:

- `scripts/check.sh` added;
- `scripts/release-check.sh` added;
- `scripts/validate-release.sh` removed;
- `RELEASING.md` added;
- `AGENTS.md` simplified;
- specialist scripts made dynamic in part;
- current CI invokes `scripts/check.sh`.

Record residuals explicitly rather than claiming unconditional closure.

## Phase 0 acceptance criteria

- `plans/037-status.md` exists before corrective source edits.
- `plans/034-status.md`, `plans/035-status.md`, and `plans/036-status.md` exist.
- Retrospective ledgers distinguish source facts from unverified repository settings.
- No run IDs, setting values, command results, or conclusions are invented.
- Plan 036 is marked partial until this corrective pass closes its residuals.

Suggested commit:

```text
plans: add retrospective CI and release simplification ledgers
```

---

# Phase 1: Make `release-check.sh` fail safely and inspect both packages

## Problem

Current behavior is unsafe:

```bash
if cargo package -p stegoeggo-cli ...; then
    echo "CLI package dry-run: OK"
else
    echo "WARNING: CLI dry-run failed. This is expected before library publication."
fi
```

This treats every failure as the expected unpublished-library ordering constraint. A malformed CLI package, missing file, bad manifest, packaging regression, or unrelated dependency failure can therefore be followed by:

```text
=== Release check passed ===
```

The correction must avoid brittle parsing of Cargo error messages and must not introduce mandatory network queries merely to classify failures.

## 1.1 Adopt a deterministic two-package contract

Use this preferred contract.

### Library

Run a fully verified package dry-run:

```bash
cargo package -p stegoeggo [--allow-dirty]
```

Any non-zero exit is fatal.

### CLI before library publication

Run deterministic package assembly without registry-resolved verification:

```bash
cargo package -p stegoeggo-cli --no-verify [--allow-dirty]
```

Any non-zero exit is fatal.

This command must not be wrapped in a blanket warning or ignored failure branch.

Rationale:

- normal workspace CI already compiles and tests the CLI against the local library;
- the CLI package can be assembled and inspected before the matching library version is available on crates.io;
- the extracted package manifest can be checked to prove that the path dependency was normalized to the exact registry version;
- registry-resolved CLI verification remains a required manual step after the library version is published;
- no Cargo error-text classifier is needed.

Do not call the CLI `--no-verify` result a complete registry-resolved dry-run. Label it accurately, for example:

```text
CLI package assembly: OK
CLI registry-resolved verification: deferred until matching library version is published
```

## 1.2 Preserve the narrow script interface

Retain only the existing optional flags unless a demonstrated defect requires another:

```text
--allow-dirty
--skip-check
```

Do not add:

- phase selectors;
- expected-SHA arguments;
- RC run IDs;
- publication flags;
- tag flags;
- workflow dispatch flags;
- crates.io token handling.

## 1.3 Select packages by name

Use Cargo metadata and explicitly select:

```text
stegoeggo
stegoeggo-cli
```

Do not depend on package ordering.

Illustrative Python pattern:

```python
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--no-deps", "--format-version", "1"
]))
packages = {package["name"]: package for package in metadata["packages"]}
lib = packages.get("stegoeggo")
cli = packages.get("stegoeggo-cli")
if lib is None or cli is None:
    raise SystemExit("required workspace packages not found")
```

Continue to verify:

- library and CLI versions match;
- CLI dependency on `stegoeggo` exists;
- the dependency requirement equals `=<library version>`.

## 1.4 Discover package artifacts dynamically

After packaging, derive the expected artifact paths from metadata:

```text
target/package/stegoeggo-<version>.crate
target/package/stegoeggo-cli-<version>.crate
```

Require each path to exist and be a regular non-empty file.

Do not use a hard-coded release number.

Before packaging, remove or identify stale same-package artifacts so a previous package cannot satisfy the existence check. Acceptable approaches:

- remove only the expected artifact path before invoking Cargo; or
- capture modification time and require it to change; or
- package into a clean target directory using `CARGO_TARGET_DIR`.

Do not broadly delete unrelated build outputs without need.

## 1.5 Inspect both package inventories

Run:

```bash
cargo package -p stegoeggo --list
cargo package -p stegoeggo-cli --list
```

Require non-empty output and print it under clearly labeled sections.

At minimum, verify the library package contains:

```text
Cargo.toml
Cargo.toml.orig
LICENSE
README.md
src/lib.rs
```

At minimum, verify the CLI package contains:

```text
Cargo.toml
Cargo.toml.orig
LICENSE
README.md
src/main.rs
```

Use package-relative path matching that reflects the actual generated package. If Cargo names a license or readme differently, inspect the package and encode the truthful expected names.

Do not hard-code the complete file inventory; only require files essential to package usability.

## 1.6 Inspect the normalized CLI manifest

Extract the generated CLI `.crate` to a temporary directory and inspect the generated `Cargo.toml`.

Required assertions:

- package name is `stegoeggo-cli`;
- package version equals the workspace library version;
- dependency `stegoeggo` exists;
- dependency version equals `=<workspace version>`;
- no local `path = ".."` survives in the normalized packaged dependency;
- binary target `stegoeggo` points to the packaged entry point.

Use Python standard-library TOML support only if available (`tomllib` on supported Python). A focused text check is acceptable if it is precise and documented. Do not add a package-manager dependency solely for TOML parsing.

## 1.7 Keep registry-resolved CLI verification in `RELEASING.md`

After the library is published and resolvable, the maintainer must run:

```bash
cargo publish -p stegoeggo-cli --dry-run
cargo publish -p stegoeggo-cli
```

`release-check.sh` must print a clear reminder that CLI registry-resolved verification is deferred until that point.

The script must not publish the library or CLI itself.

## 1.8 Required negative tests

Perform these tests in temporary repository copies or temporary worktrees.

### Test A: normal repository

```bash
./scripts/release-check.sh --skip-check
```

Expected:

- exit 0;
- library verified package succeeds;
- CLI `--no-verify` package assembly succeeds;
- both inventories are printed or checked;
- normalized CLI manifest check succeeds;
- no publication, tag, push, or GitHub release occurs.

### Test B: unpublished synthetic version

In a temporary copy only:

1. change library version to an obviously unused version such as `99.99.99`;
2. change CLI package version to the same value;
3. change the CLI exact dependency to `=99.99.99`;
4. run:

```bash
./scripts/release-check.sh --allow-dirty --skip-check
```

Expected:

- exit 0;
- library package verification succeeds;
- CLI package assembly succeeds with `--no-verify`;
- output explicitly states that registry-resolved CLI verification is deferred;
- the generated CLI package manifest contains `=99.99.99` and no path dependency;
- no network-error text is parsed to manufacture success.

Do not publish or query ownership for the synthetic version.

### Test C: unrelated CLI package defect

In a temporary copy only, introduce a deterministic package-assembly defect after metadata parsing, for example:

- set the CLI `readme` path to a missing file; or
- set the CLI `license-file` to a missing file; or
- exclude the binary entry point from package contents.

Run:

```bash
./scripts/release-check.sh --allow-dirty --skip-check
```

Expected:

- non-zero exit;
- no `Release check passed` message;
- the defect is not described as expected unpublished-library behavior;
- the temporary copy is removed afterward.

Choose a mutation that demonstrably fails `cargo package --no-verify`; verify the mutation before using it as evidence.

## Phase 1 acceptance criteria

- Every library packaging failure is fatal.
- Every CLI package-assembly failure is fatal.
- No generic `else` branch converts arbitrary CLI failures to success.
- The CLI is packaged with `--no-verify` before library publication.
- Both package artifacts are discovered dynamically.
- Both package inventories are checked.
- The normalized CLI manifest proves exact registry dependency normalization.
- The script accurately distinguishes package assembly from deferred registry-resolved verification.
- Normal, synthetic-unpublished, and unrelated-defect tests produce the expected exit codes.
- The script contains no `cargo publish`, `git tag`, `git push`, `gh release`, workflow dispatch, token, or RC logic.

Suggested commit:

```text
release: make local package readiness checks fail safely
```

---

# Phase 2: Repair fuzz-target synchronization for the manual workflow

## Problem

The current synchronization script was written for the former matrix layout and attempts to extract targets after a matrix-style `target:` key. The current workflow stores targets at:

```text
on.workflow_dispatch.inputs.target.options
```

The script therefore cannot reliably compare the workflow choices with `fuzz/Cargo.toml`.

## 2.1 Update the script contract

Keep `scripts/check_fuzz_sync.sh` as a targeted local maintenance diagnostic.

It must compare:

- every `[[bin]] name` in `fuzz/Cargo.toml`;
- every choice under `workflow_dispatch.inputs.target.options` in `.github/workflows/fuzz.yml`.

It must exit:

```text
0 when the sets are identical
1 when either side has missing or extra targets
```

It must report separate lists:

```text
In Cargo.toml but not in workflow
In workflow but not in Cargo.toml
```

## 2.2 Parse the current YAML structure deliberately

Do not use the former matrix indentation assumptions.

Acceptable implementation approaches:

### Preferred: small Python standard-library indentation parser

Use Python only to locate the controlled YAML block by indentation:

1. locate `workflow_dispatch:`;
2. locate its `inputs:` child;
3. locate the `target:` input;
4. locate its `options:` child;
5. collect subsequent list entries at the expected deeper indentation;
6. stop when indentation returns to the `target:` or `inputs:` level.

The workflow structure is controlled by the repository, so a focused parser is acceptable. Do not claim it is a general YAML parser.

### Acceptable: corrected AWK state machine

An AWK implementation is acceptable if it explicitly follows `target -> options -> list` and has positive and negative tests.

Do not install PyYAML or another dependency solely for this script.

## 2.3 Make paths testable without adding a test framework

Allow path overrides through environment variables:

```bash
CARGO_TOML="${CARGO_TOML:-fuzz/Cargo.toml}"
FUZZ_YML="${FUZZ_YML:-.github/workflows/fuzz.yml}"
```

This permits deterministic negative testing against temporary copies.

The default invocation remains:

```bash
./scripts/check_fuzz_sync.sh
```

## 2.4 Update comments and documentation

Replace references to a matrix with accurate wording such as:

```text
Verify fuzz targets in fuzz/Cargo.toml match the manual workflow target choices.
```

Update `AGENTS.md` if needed, but do not add the script to required CI.

## 2.5 Required tests

### Positive test

```bash
./scripts/check_fuzz_sync.sh
```

Expected:

- exit 0;
- reports exactly 12 synchronized targets unless the actual target count changed;
- target names are identical on both sides.

### Negative test: missing workflow target

```bash
cp .github/workflows/fuzz.yml "$tmp/fuzz.yml"
# Remove one known target choice from the temporary workflow.
FUZZ_YML="$tmp/fuzz.yml" ./scripts/check_fuzz_sync.sh
```

Expected:

- exit 1;
- removed target appears under `In Cargo.toml but not in workflow`.

### Negative test: extra workflow target

Add a fake target such as `not_a_real_target` to the temporary workflow choices.

Expected:

- exit 1;
- fake target appears under `In workflow but not in Cargo.toml`.

### Negative test: malformed target block

Remove or rename the `options:` block in the temporary workflow.

Expected:

- exit 1 with a clear parse/configuration error;
- not a misleading empty-set success.

## Phase 2 acceptance criteria

- The script parses the current workflow-dispatch choice layout.
- The script no longer mentions or depends on the old matrix layout.
- Default positive invocation passes.
- Missing, extra, and malformed negative tests fail correctly.
- The script remains local/manual and is not added to `scripts/check.sh` or required CI.
- No new runtime dependency is introduced.

Suggested commit:

```text
fuzz: repair manual workflow target synchronization check
```

---

# Phase 3: Correct MSRV and docs.rs specialist diagnostics

These scripts remain targeted diagnostics. The objective is to make them truthful and internally consistent, not to broaden them or add them back to CI.

## 3.1 Use named Cargo metadata selection in both scripts

Replace package-order-dependent extraction such as:

```python
metadata["packages"][0]["version"]
```

with explicit package-name selection.

Required behavior:

- find package named `stegoeggo`;
- fail clearly if absent;
- print the selected version;
- do not assume workspace package ordering.

A shared helper file is not required. Duplicating a small, clear Python snippet in two single-purpose scripts is preferable to creating another abstraction layer.

## 3.2 Correct the MSRV toolchain override

Current behavior defines:

```bash
MSRV="${MSRV:-1.87}"
```

but invokes:

```bash
cargo +1.87 check
```

Change every MSRV Cargo/Rust invocation to use the configured value:

```bash
cargo +"${MSRV}" check
rustc +"${MSRV}" --version
```

The script must print the exact selected toolchain and fail clearly if it is not installed.

Do not silently fall back to the default toolchain.

## 3.3 Make minimal consumer semantics explicit

For the minimal consumer dependency, generate:

```toml
stegoeggo = { path = "<package path>", default-features = false }
```

For the maximal public-feature consumer, generate:

```toml
stegoeggo = {
  path = "<package path>",
  default-features = false,
  features = ["async", "signatures", "detached-manifest"]
}
```

Use valid single-line TOML if the script emits inline tables.

Do not rely on the current `default = []` feature set to make the minimal case accidentally correct. The script should remain meaningful if defaults are introduced later.

Do not automatically include internal/test-only features unless the public compatibility policy explicitly requires them.

## 3.4 Keep the MSRV scope narrow

The script should continue to test only:

- minimal public configuration;
- maximal intended public configuration.

Do not restore six feature combinations.

Do not add it to required CI or `release-check.sh`.

## 3.5 Correct the docs.rs stable phase

The phase labeled stable must invoke stable explicitly:

```bash
cargo +stable doc -p stegoeggo --all-features --no-deps
```

or equivalent.

Required properties:

- `DOCS_RS` is unset;
- docs.rs-specific `RUSTDOCFLAGS` are unset;
- stable is selected explicitly;
- the command targets the package named `stegoeggo`;
- the phase label matches the actual toolchain.

If an explicitly stable phase provides no distinct value, it may be removed instead. If retained, it must be truthful.

## 3.6 Keep docs.rs reproduction semantic rather than bureaucratic

Retain only the useful conditions:

- nightly rustdoc;
- `DOCS_RS=1`;
- `--cfg docsrs`;
- check-cfg for `docsrs`;
- warnings denied;
- relevant features;
- workspace build;
- packaged crate build;
- crate-root `feature(doc_cfg)` placement check.

Do not add workflow run IDs, RC semantics, expected SHA, evidence artifacts, or publication logic.

## 3.7 Required MSRV tests

Run:

```bash
MSRV=1.87 ./scripts/validate-msrv-package.sh
```

Expected:

- output identifies Rust/Cargo 1.87;
- both minimal and maximal consumers run with `cargo +1.87` through the variable;
- minimal generated manifest includes `default-features = false`;
- maximal generated manifest includes `default-features = false` and the intended feature list;
- both checks pass with fresh consumer lockfiles;
- no current-version literal is embedded in script source.

Static verification:

```bash
rg -n 'cargo \+1\.87|rustc \+1\.87|packages\]\[0\]|packages"\]\[0\]' scripts/validate-msrv-package.sh
```

Expected: no hard-coded toolchain invocation and no order-dependent package selection.

If only Rust 1.87 is supported operationally, the variable may remain for clarity, but it still must control the command it labels.

## 3.8 Required docs.rs tests

Run:

```bash
./scripts/validate-docs-rs.sh
```

Expected:

- nightly docs.rs-equivalent workspace build passes;
- module-level `feature(doc_cfg)` rejection passes;
- packaged crate docs build passes;
- explicitly stable ordinary docs build passes;
- output toolchain labels match actual toolchains;
- package version is selected by name.

Static verification:

```bash
rg -n 'packages\]\[0\]|packages"\]\[0\]|^cargo doc|Stable docs' scripts/validate-docs-rs.sh
```

Interpret carefully: a `Stable docs` label is acceptable only when the command uses `+stable`.

If nightly or stable toolchains are unavailable, install only the required Rust toolchain through rustup or record the exact prerequisite blocker. Do not add this installation to required CI.

## Phase 3 acceptance criteria

- Both scripts select `stegoeggo` by package name.
- The MSRV variable controls every selected-toolchain command.
- Minimal MSRV consumer explicitly disables default features.
- Maximal consumer explicitly states intended public features.
- The MSRV script remains two-combination and targeted.
- The docs stable phase either uses `+stable` or is removed/renamed truthfully.
- Both scripts retain dynamic version handling.
- Neither script is invoked by required CI or `release-check.sh`.
- Neither script publishes, tags, pushes, or dispatches workflows.

Suggested commit:

```text
scripts: correct targeted docs and MSRV diagnostics
```

---

# Phase 4: Correct current operational documentation

## 4.1 Fix `AGENTS.md` specialist-path wording

Current wording implies all specialist verification is available through manual-dispatch workflows.

Replace it with a precise distinction.

Recommended structure:

```text
Specialist verification is available through manual workflows or targeted local commands.

Manual workflows:
- external-verification.yml
- fuzz.yml

Local targeted commands:
- validate-docs-rs.sh
- validate-msrv-package.sh
- check_fuzz_sync.sh
- cargo semver-checks check-release
- cargo deny check licenses
- cargo deny check advisories
- cargo bench
- cargo package / release-check.sh as documented
```

Do not characterize local-only scripts as GitHub workflows.

## 4.2 Update release-check documentation

Update `AGENTS.md`, `RELEASING.md`, script comments, and the unreleased changelog entry so they accurately state:

- library package is fully verified locally;
- CLI package is assembled and inspected locally with `--no-verify` before library publication;
- CLI registry-resolved dry-run occurs after the matching library version is published and resolvable;
- arbitrary CLI package-assembly failures are fatal;
- `release-check.sh` never publishes.

Do not imply that a failed verified CLI package command is automatically expected.

## 4.3 Correct fuzz synchronization wording

Replace matrix-era comments with workflow-choice wording in:

- `scripts/check_fuzz_sync.sh`;
- `AGENTS.md` if needed;
- any current architecture/developer documentation.

## 4.4 Audit current operational references

Run:

```bash
rg -n \
  "manual-dispatch workflows|matrix target|fuzz matrix|CLI dry-run failed|expected before library publication|packages\]\[0\]|cargo \+1\.87|Stable docs" \
  AGENTS.md README.md RELEASING.md CHANGELOG.md architecture scripts .github
```

Classify every match.

Run the broader release-safety audit:

```bash
rg -n \
  "CARGO_REGISTRY_TOKEN|action-gh-release|release-candidate|expected-sha|gh release create|git push.*tag|cargo publish" \
  .github scripts AGENTS.md README.md architecture
```

Expected:

- no executable workflow or script publication path;
- manual `cargo publish` examples may appear in `RELEASING.md`;
- historical plans may retain old terms;
- current operational documents must not instruct use of deleted workflows.

## 4.5 Keep README user-focused

Do not expand README into a process document.

It should continue to provide only:

- `./scripts/check.sh` for contributors;
- a concise pointer to `RELEASING.md` for maintainers.

## Phase 4 acceptance criteria

- `AGENTS.md` distinguishes manual workflows from local targeted commands.
- Release-check behavior is documented accurately.
- Fuzz synchronization documentation describes workflow choices, not a matrix.
- No current operational document describes deleted RC, tag-validation, or publish workflows as active.
- No current script or workflow contains executable publication behavior.
- README remains concise and user-focused.
- Historical plans remain unchanged except for newly added status ledgers.

Suggested commit:

```text
docs: correct verification and release closure guidance
```

---

# Phase 5: Execute source verification and GitHub Actions smoke evidence

This phase proves that the corrected simplified system works. It must not turn the evidence run into a permanent gate.

## 5.1 Run the fast local check

From a clean tree at the intended source candidate:

```bash
./scripts/check.sh
```

Record:

- start/end time or duration;
- exit code;
- command summary;
- exact candidate SHA.

All four commands must pass.

## 5.2 Run the bounded release check

From a clean tree:

```bash
./scripts/release-check.sh
```

Record:

- exit code;
- library package artifact path;
- CLI package artifact path;
- inventory checks;
- normalized CLI dependency result;
- confirmation that no publication occurred.

The command must pass under the corrected contract.

## 5.3 Run specialist smoke commands

Run:

```bash
./scripts/check_fuzz_sync.sh
MSRV=1.87 ./scripts/validate-msrv-package.sh
./scripts/validate-docs-rs.sh
```

Run external conformance locally if prerequisites are available:

```bash
./scripts/verify_metadata_conformance.sh --strict
```

These commands remain targeted and are run once for closure evidence only. They are not added to required CI or `release-check.sh`.

If external tools are unavailable locally, use the manual external workflow in Phase 5.5 and record that as the evidence path.

## 5.4 Obtain one green final CI run

The final candidate commit must produce one successful `CI` workflow run with one `Check` job.

Use GitHub CLI or Actions UI evidence. Illustrative commands:

```bash
gh run list \
  --repo eggstack/stegoeggo \
  --workflow ci.yml \
  --branch main \
  --limit 10 \
  --json databaseId,headSha,event,status,conclusion,url
```

Select the run whose `headSha` equals the exact final candidate SHA.

Then inspect:

```bash
gh run view <run-id> \
  --repo eggstack/stegoeggo \
  --json headSha,conclusion,jobs,url
```

Required evidence:

- event is push or pull request for the final candidate;
- head SHA exactly matches candidate;
- workflow conclusion is success;
- exactly one required job named `Check` ran;
- job conclusion is success.

A cancelled older run is not evidence of failure. A successful run for a different SHA is not closure evidence.

## 5.5 Exercise the manual external workflow once

Dispatch against the final candidate or final merged SHA:

```bash
gh workflow run external-verification.yml \
  --repo eggstack/stegoeggo \
  --ref <candidate-ref-or-sha>
```

Record the run ID and conclusion.

Required result:

- one job runs;
- external tools are installed once;
- external integration tests pass;
- conformance harness passes;
- no publication or release action occurs.

If the connector/API cannot dispatch by SHA, use a branch pointing exactly to the candidate and record the resolved head SHA.

If GitHub permissions prevent dispatch, record the exact permission error and run the equivalent local command with external tools. Mark workflow execution as maintainer-pending rather than falsely complete.

## 5.6 Exercise the manual fuzz workflow once

Dispatch one short target, preferably a broad parser/pipeline target:

```bash
gh workflow run fuzz.yml \
  --repo eggstack/stegoeggo \
  --ref <candidate-ref-or-sha> \
  -f target=pipeline_bytes \
  -f seconds=10
```

Required result:

- one fuzz job runs;
- no matrix fan-out occurs;
- selected target and duration are honored;
- workflow succeeds without a crash;
- no routine artifact upload occurs on success.

A ten-second smoke is sufficient for workflow validation. Do not interpret it as deep fuzz assurance.

If permissions prevent dispatch, record the exact blocker and run the equivalent local command:

```bash
cargo +nightly fuzz run pipeline_bytes -- \
  -max_total_time=10 \
  -max_len=4096 \
  -timeout=10
```

## 5.7 Validate workflow trigger and complexity state

Run:

```bash
find .github/workflows -maxdepth 1 -type f -print | sort
rg -n "push:|pull_request:|schedule:|workflow_dispatch:|tags:|matrix:|cargo install|apt-get|upload-artifact" .github/workflows
```

Required conclusions:

- only `ci.yml` has push/pull-request triggers;
- no workflow has a schedule;
- no workflow has a tag trigger;
- `ci.yml` has one job and no matrix;
- `ci.yml` has no `cargo install`, `apt-get`, or artifact upload;
- external verification is manual only;
- fuzzing is manual only and single-target;
- `cargo install cargo-fuzz` exists only inside the manual fuzz workflow;
- `apt-get` exists only inside the manual external workflow;
- crash artifact upload exists only conditionally in the manual fuzz workflow.

## Phase 5 acceptance criteria

- `scripts/check.sh` passes at the final candidate SHA.
- `scripts/release-check.sh` passes at the final candidate SHA.
- Corrected fuzz sync, MSRV, and docs.rs scripts pass or have exact prerequisite blockers recorded.
- One green final `CI` run is recorded for the exact candidate SHA.
- The `CI` run has exactly one `Check` job.
- External workflow smoke passes or an exact permissions blocker plus equivalent local evidence is recorded.
- Fuzz workflow smoke passes or an exact permissions blocker plus equivalent local evidence is recorded.
- Workflow static metrics match the complexity budget.
- No crate, tag, or GitHub release is created.

Suggested evidence commit:

```text
plans: record corrective verification and workflow smoke evidence
```

Do not mix new source corrections into the evidence-only commit unless a command exposes a defect that requires another source commit and rerun.

---

# Phase 6: Verify and clean repository-level settings

Source control cannot prove these settings. Inspect them directly where permissions allow.

## 6.1 Actions secret

Inspect secret names:

```bash
gh api repos/eggstack/stegoeggo/actions/secrets
```

Required simplified state:

```text
CARGO_REGISTRY_TOKEN absent
```

If present and used only by deleted automation, remove it:

```bash
gh secret delete CARGO_REGISTRY_TOKEN --repo eggstack/stegoeggo
```

Do not print or retrieve secret values.

If permissions do not allow inspection/deletion, record:

```text
Repository settings disposition: PARTIAL — maintainer must remove CARGO_REGISTRY_TOKEN
```

## 6.2 `crates-io` environment

Inspect environments:

```bash
gh api repos/eggstack/stegoeggo/environments
```

If an environment named `crates-io` exists solely for the deleted publication workflow, remove it or explicitly repurpose it outside this plan.

Before deletion, confirm it is not used by another current workflow.

Illustrative deletion API:

```bash
gh api \
  --method DELETE \
  repos/eggstack/stegoeggo/environments/crates-io
```

Do not delete a shared or intentionally retained environment without evidence.

## 6.3 Branch protection required checks

Inspect branch protection or rulesets for `main`.

Possible commands, depending on repository configuration:

```bash
gh api repos/eggstack/stegoeggo/branches/main/protection

gh api repos/eggstack/stegoeggo/rulesets
```

Required state if status checks are enforced:

```text
required check: Check
obsolete deleted job names: absent
```

Obsolete names may include:

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
Validate Release
Validate Release Candidate
Publish
```

Do not restore compatibility jobs. Update settings to require only `Check`, or record that the repository intentionally has no required checks.

## 6.4 Tag and release rules

Inspect repository rulesets for:

- required release workflows on `v*` tags;
- tag protection that assumes deleted validation jobs;
- rules requiring release-candidate evidence;
- rules invoking a reusable publication workflow.

Remove only stale rules associated with the deleted apparatus.

Tag protection unrelated to automation may remain.

## 6.5 Actions token permissions

Inspect repository Actions permissions where available:

```bash
gh api repos/eggstack/stegoeggo/actions/permissions/workflow
```

Preferred default:

```text
read-only contents permission unless another current workflow has a justified need
```

Current workflows declare read-only contents permissions. Do not broaden defaults.

## 6.6 Reusable workflow audit

Inspect only directly referenced reusable workflows from the repository’s current `.github/workflows/*.yml` files.

Required conclusion:

- no current workflow calls an organization or repository reusable workflow that can publish stegoeggo;
- no hidden publication path is directly wired through `uses:`.

Do not expand this into an organization-wide audit.

## Phase 6 acceptance criteria

- `CARGO_REGISTRY_TOKEN` is verified absent or explicitly maintainer-pending.
- `crates-io` environment is removed if obsolete, retained with a documented purpose, or explicitly maintainer-pending.
- branch protection/rulesets no longer require deleted job names.
- only `Check` is required if required checks are enabled.
- no tag rule requires deleted release workflows.
- Actions permissions are not unnecessarily write-enabled.
- no directly referenced reusable publication workflow exists.
- every setting row in the status ledgers is marked verified, not applicable, or maintainer-pending.
- no source workflow is restored to satisfy stale settings.

Suggested settings/evidence commit if only ledgers change:

```text
plans: record repository settings closure state
```

Repository-setting API changes do not create a source commit by themselves; record their date and evidence in the ledgers.

---

# Phase 7: Close Plans 034 through 037 and the roadmap

## 7.1 Close `plans/034-status.md`

Required final source fields:

```text
Source disposition: CLOSED
Automated crates.io publication: absent
Tag-triggered release mutation: absent
Release-candidate workflow: absent
Tag-validation workflow: absent
GitHub release automation: absent
```

Repository setting disposition must be one of:

```text
VERIFIED-CLOSED
PARTIAL — maintainer action required
NOT-APPLICABLE
```

Do not use unconditional `CLOSED` for settings that were not inspected.

## 7.2 Close `plans/035-status.md`

Required final metrics:

```text
Automatic push/PR workflows: 1
Required push/PR jobs: 1
Required matrices: 0
Required Rust toolchains: stable only
Required cargo-installed tools: 0
Required OS packages: 0
Required artifact uploads: 0
Automatic fuzz jobs: 0
Automatic external-tool jobs: 0
Required check name: Check
```

Record the final green CI run ID and exact head SHA.

## 7.3 Close `plans/036-status.md`

Required final fields:

```text
Source disposition: CLOSED
Fast local check: verified
Bounded release check: verified
Oversized validator: removed
Specialist scripts: corrected and targeted
Manual release documentation: verified
Automated publication references: absent from executable paths
Version-hard-coded package paths: absent
```

Record the package-check negative-test evidence and corrected specialist-script evidence.

## 7.4 Close `plans/037-status.md`

Required final disposition:

```text
Disposition: CLOSED
```

or, if only settings remain inaccessible:

```text
Source disposition: CLOSED
Repository settings disposition: PARTIAL — explicit maintainer checklist remains
Overall disposition: SOURCE-CLOSED / SETTINGS-PARTIAL
```

Required closure summary:

```text
release-check arbitrary failure suppression: corrected
CLI package assembly and inventory: verified
fuzz synchronization parser: corrected
MSRV toolchain override: corrected
minimal default-features behavior: corrected
docs stable toolchain claim: corrected
Cargo metadata package selection: corrected
AGENTS specialist-path wording: corrected
Plans 034-036 ledgers: present
final one-job CI: green and recorded
manual external workflow: exercised or blocker recorded
manual fuzz workflow: exercised or blocker recorded
repository settings: verified or explicitly pending
```

## 7.5 Record final complexity budget

The closing ledger must include:

```text
required automatic workflows: 1
required push/PR jobs: 1
required matrices: 0
required stable toolchains: 1
required cargo-installed tools: 0
required OS packages: 0
required artifact uploads: 0
tag-triggered workflows: 0
automated publication paths: 0
workflow crates.io token references: 0
manual specialist paths retained: yes
manual release procedure: documented
```

## 7.6 Final static audit

Run:

```bash
rg -n \
  "CARGO_REGISTRY_TOKEN|action-gh-release|release-candidate|expected-sha|gh release create|git push.*tag" \
  .github scripts AGENTS.md README.md architecture
```

Run:

```bash
rg -n "tags:|refs/tags|GITHUB_REF_NAME|github\.ref_name" .github/workflows
```

Run:

```bash
rg -n "cargo publish" .github scripts
```

Expected result:

- no executable automated publication path;
- no tag-triggered workflow behavior;
- manual publication commands remain only in `RELEASING.md` or historical plans;
- verification scripts do not publish.

## Phase 7 acceptance criteria

- All four status ledgers exist and are truthful.
- Plans 034 and 035 are source-closed.
- Plan 036 is source-closed after the corrective fixes.
- Plan 037 is closed or source-closed/settings-partial with an exact maintainer checklist.
- Final metrics match the simplification roadmap.
- Every command/run cited has an exact SHA or run ID.
- No release, tag, crate publication, or GitHub release occurred during implementation.

Suggested final commit:

```text
plans: close CI and manual release simplification corrective pass
```

---

# Recommended commit sequence

Use small commits so a reviewer can distinguish code corrections from evidence.

Recommended sequence:

```text
1. plans: add retrospective CI and release simplification ledgers
2. release: make local package readiness checks fail safely
3. fuzz: repair manual workflow target synchronization check
4. scripts: correct targeted docs and MSRV diagnostics
5. docs: correct verification and release closure guidance
6. plans: record corrective verification and workflow smoke evidence
7. plans: record repository settings closure state
8. plans: close CI and manual release simplification corrective pass
```

Combining adjacent source commits is acceptable if the final diff remains reviewable. Do not combine source correction and fabricated after-the-fact evidence.

---

# Detailed final acceptance checklist

The implementation is complete only when every applicable item is checked.

## Required CI architecture

- [ ] `.github/workflows/ci.yml` is the only push/PR workflow.
- [ ] `ci.yml` has exactly one job named `Check`.
- [ ] The job uses stable Rust only.
- [ ] The job invokes `./scripts/check.sh`.
- [ ] No required matrix exists.
- [ ] No required `cargo install` exists.
- [ ] No required `apt-get` exists.
- [ ] No required artifact upload exists.
- [ ] No required specialist diagnostic exists.
- [ ] A green run for the exact final candidate SHA is recorded.

## Release automation removal

- [ ] `release-candidate.yml` is absent.
- [ ] `release.yml` is absent.
- [ ] `publish.yml` is absent.
- [ ] No workflow has a version-tag trigger.
- [ ] No workflow invokes `cargo publish`.
- [ ] No workflow creates a GitHub release.
- [ ] No workflow references `CARGO_REGISTRY_TOKEN`.
- [ ] No workflow depends on RC evidence.

## `release-check.sh`

- [ ] Fails on dirty tree by default.
- [ ] Runs `scripts/check.sh` by default.
- [ ] Verifies version lockstep.
- [ ] Verifies exact CLI dependency version.
- [ ] Fully verifies the library package.
- [ ] Requires CLI package assembly with `--no-verify` to succeed.
- [ ] Does not suppress arbitrary CLI packaging failures.
- [ ] Inspects both package inventories.
- [ ] Inspects normalized CLI manifest.
- [ ] Discovers artifacts dynamically.
- [ ] Does not hard-code the current version.
- [ ] Clearly labels deferred registry-resolved CLI verification.
- [ ] Contains no publication, tag, push, release, RC, or workflow logic.
- [ ] Normal positive test passes.
- [ ] Synthetic unpublished-version test passes.
- [ ] Unrelated CLI package-defect test fails.

## Fuzz synchronization

- [ ] Parses `workflow_dispatch.inputs.target.options`.
- [ ] Compares exactly with `fuzz/Cargo.toml` bin targets.
- [ ] Positive test passes.
- [ ] Missing-target test fails with correct classification.
- [ ] Extra-target test fails with correct classification.
- [ ] Malformed-block test fails clearly.
- [ ] Script is not part of required CI.

## MSRV diagnostic

- [ ] Selects package `stegoeggo` by name.
- [ ] `MSRV` variable controls the selected toolchain command.
- [ ] No hard-coded `cargo +1.87` remains unless the variable itself is intentionally removed and documentation is updated.
- [ ] Minimal consumer sets `default-features = false`.
- [ ] Maximal consumer states intended public features explicitly.
- [ ] Only minimal and maximal configurations run.
- [ ] Fresh-resolution MSRV command passes.
- [ ] Script remains targeted/manual.

## Docs.rs diagnostic

- [ ] Selects package `stegoeggo` by name.
- [ ] Nightly docs.rs-equivalent workspace build passes.
- [ ] Packaged crate docs build passes.
- [ ] Module-level `feature(doc_cfg)` rejection passes.
- [ ] Stable phase uses explicit `+stable` or is renamed/removed truthfully.
- [ ] No RC/workflow/publication coupling exists.
- [ ] Script remains targeted/manual.

## Documentation

- [ ] `AGENTS.md` distinguishes manual workflows from local commands.
- [ ] `RELEASING.md` accurately describes CLI package assembly and deferred registry verification.
- [ ] Manual library-first publication remains explicit.
- [ ] Immutable-version rules remain explicit.
- [ ] No fixed registry sleep is reintroduced.
- [ ] Fuzz sync wording no longer refers to the old matrix.
- [ ] README remains concise.
- [ ] Historical plans remain truthful.

## Evidence and settings

- [ ] `plans/034-status.md` exists.
- [ ] `plans/035-status.md` exists.
- [ ] `plans/036-status.md` exists.
- [ ] `plans/037-status.md` exists.
- [ ] Final CI run ID and SHA are recorded.
- [ ] External workflow smoke is recorded or exact blocker is recorded.
- [ ] Fuzz workflow smoke is recorded or exact blocker is recorded.
- [ ] Secret state is verified or maintainer-pending.
- [ ] Environment state is verified or maintainer-pending.
- [ ] Branch protection/rulesets are verified or maintainer-pending.
- [ ] No obsolete check name is restored through a shim job.
- [ ] Required final complexity metrics are recorded.

## Safety

- [ ] No crate was published.
- [ ] No version was bumped.
- [ ] No tag was created or moved.
- [ ] No GitHub release was created.
- [ ] No product behavior changed.
- [ ] No required CI surface was expanded.

---

# Failure handling

## If CLI `cargo package --no-verify` still requires the unpublished library

Do not return to blanket failure suppression.

Investigate the exact Cargo behavior and select the smallest deterministic alternative that still:

- assembles the CLI package;
- inspects its normalized manifest and contents;
- fails on unrelated package defects;
- does not publish;
- does not require error-message parsing.

Possible alternatives include packaging in an isolated workspace copy or using an explicit packaged-library patch only for local verification. Document the exact mechanism and keep it out of required CI.

## If normalized manifest parsing is difficult

Do not add a large TOML dependency or parser framework.

Use one of:

- Python `tomllib` when available;
- a focused text check over Cargo’s normalized manifest;
- `cargo metadata` against an extracted temporary consumer if it can read the normalized manifest without registry publication.

The check must still prove the local path is absent and exact version is retained.

## If fuzz YAML parsing remains brittle

Do not install a general YAML parser merely for one controlled list.

An acceptable simplification is to establish a tiny canonical target-list text file and generate/check both surfaces from it, but only if this reduces rather than increases maintenance. Do not add code generation to required CI.

## If Rust 1.87 is not installed

Install it locally through rustup for this targeted verification:

```bash
rustup toolchain install 1.87
```

Do not add MSRV installation to required CI.

## If nightly is not installed

Install it locally through rustup for the targeted docs/fuzz smoke:

```bash
rustup toolchain install nightly
```

Do not add nightly to required CI.

## If external tools are unavailable

Prefer the manual external workflow. If workflow permissions also block execution, record the exact missing tools and permission error. Do not mark external workflow smoke successful.

## If GitHub Actions run evidence is inaccessible

Use GitHub CLI, repository Actions UI, or an authorized connector. Record the exact limitation. Do not substitute a local command for proof that the final workflow YAML ran successfully, though local success may be recorded separately.

## If branch protection requires deleted checks

Do not restore deleted jobs. Update branch protection or rulesets to require only `Check`, or record a maintainer action.

## If `CARGO_REGISTRY_TOKEN` cannot be inspected

Do not claim it is absent. Record the exact maintainer command:

```bash
gh secret delete CARGO_REGISTRY_TOKEN --repo eggstack/stegoeggo
```

and keep repository-settings disposition partial.

## If a source correction changes the final candidate SHA

Rerun all candidate-specific local checks and obtain new final CI evidence. Do not cite a green run for the superseded SHA.

---

# Definition of done

Plan 037 and the Plans 033-036 simplification program are complete when:

- the one-workflow/one-job required CI architecture remains intact;
- automated publication, tag validation, release-candidate certification, and GitHub release automation remain absent;
- `scripts/release-check.sh` can no longer convert arbitrary CLI package failures into success;
- library package verification and CLI package assembly/content verification both work deterministically;
- registry-resolved CLI verification is deferred explicitly and safely until after library publication;
- fuzz-target synchronization matches the current manual workflow choice layout and has positive/negative evidence;
- the MSRV diagnostic honors its configured toolchain and explicitly tests no-default-features;
- the docs.rs diagnostic uses truthful toolchain selection;
- both specialist scripts select the library package by name;
- current documentation accurately distinguishes required CI, manual workflows, local diagnostics, pre-release readiness, and publication;
- Plans 034, 035, 036, and 037 have status ledgers with exact commit/run evidence;
- one green final `Check` run is recorded for the exact final source candidate;
- manual external and fuzz workflow paths are exercised once or have exact permission blockers recorded;
- obsolete GitHub secrets, environments, checks, tag rules, and publication rules are removed or explicitly maintainer-pending;
- final complexity metrics match the roadmap budget;
- no release, version bump, tag mutation, GitHub release, or product behavior change occurs during implementation.

The intended final disposition is:

```text
CI/release simplification source: CLOSED
Corrective script behavior: CLOSED
Planning/evidence closure: CLOSED
Repository settings: VERIFIED-CLOSED
```

If repository settings cannot be inspected or changed through the implementation environment, the only acceptable partial result is:

```text
CI/release simplification source: CLOSED
Corrective script behavior: CLOSED
Planning/evidence closure: CLOSED
Repository settings: PARTIAL — exact maintainer actions recorded
```

No other residual should remain open.