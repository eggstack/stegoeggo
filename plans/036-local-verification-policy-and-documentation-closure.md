# Plan 036: Local Verification Policy and Documentation Closure

Status: Ready for implementation

Baseline: `main` after Plan 035 plan commit `e790be603216d98ee038dfd360559ee619728eb4`

Depends on:

- `plans/033-ci-verification-and-manual-release-simplification-roadmap.md`
- completion of `plans/034-remove-automated-release-and-decouple-tags.md`
- completion of `plans/035-collapse-required-ci-and-demote-specialist-verification.md`

This is the closure plan for the CI/release simplification line of work.

---

## Purpose

Codify the simplified operating model so future agents do not rebuild the removed release-certification apparatus.

This plan creates a small local verification interface, defines a bounded pre-release procedure, documents direct manual publication through crates.io, removes obsolete workflow-era instructions, eliminates version-hard-coded validation behavior, and records repository-setting cleanup.

The central design requirement is separation of concerns:

```text
fast development correctness
!=
targeted specialist diagnostics
!=
manual pre-release preparation
!=
publication
```

No script may blur those categories by silently running every available check or publishing as a side effect.

---

## Required end state

The repository should expose these clear paths.

### Fast development check

```bash
./scripts/check.sh
```

This mirrors required CI and contains only fast deterministic checks.

### Manual pre-release check

```bash
./scripts/release-check.sh
```

This performs bounded local package/readiness verification and never publishes.

### Targeted specialist checks

Explicit commands or scripts remain available for:

- external interoperability and conformance;
- docs.rs reproduction;
- MSRV validation;
- semver analysis;
- license/advisory analysis;
- fuzzing;
- benchmarks.

These are invoked intentionally according to the change type or maintainer judgment.

### Manual publication

Publication is performed with direct Cargo commands, documented in `RELEASING.md`. GitHub Actions are not part of this sequence.

---

## Phase 0: Create the closure ledger

Create `plans/036-status.md` before modifying scripts or documentation.

Initialize:

```text
Plan baseline SHA: <actual implementation baseline SHA>
Disposition: OPEN
Required CI architecture: expected simplified state from Plan 035
Manual release documentation: incomplete or absent
Fast local check: incomplete or absent
Bounded local release check: incomplete or oversized
Automated publication references: audit pending
Version-hard-coded package paths: audit pending
```

Add these tables.

### Table A: validation interface inventory

```text
path/command | current purpose | automatic or manual | publishes | duplicates | target disposition
```

At minimum inspect:

```text
scripts/validate-release.sh
scripts/validate-docs-rs.sh
scripts/validate-msrv-package.sh
scripts/verify_metadata_conformance.sh
scripts/check_fuzz_sync.sh
AGENTS.md
README.md
CHANGELOG.md
SECURITY.md
SUPPORT.md
STABILITY.md
DEPRECATIONS.md
.github/workflows/*
```

### Table B: targeted-check policy

```text
check | normal trigger | command | blocking for ordinary CI | blocking before release | notes
```

### Table C: documentation truth audit

```text
document | obsolete claim | replacement claim | commit | verified
```

### Table D: repository settings cleanup

```text
setting | expected obsolete value | required action | source-verifiable | maintainer-verifiable | status
```

### Phase 0 acceptance criteria

- Every validation script has a declared target disposition.
- Every current document mentioning CI or release is inventoried.
- Version-hard-coded package paths are identified.
- The status ledger distinguishes source completion from repository-setting completion.

Suggested commit:

```text
plans: inventory local verification and release policy
```

---

## Phase 1: Add a fast local check matching required CI

Create:

```text
scripts/check.sh
```

Required implementation:

```bash
#!/usr/bin/env bash
set -euo pipefail

run() {
    printf '\n==> %q' "$1"
    shift
    printf ' %q' "$@"
    printf '\n'
    "$@"
}

run cargo fmt --all -- --check
run cargo clippy --workspace --all-targets --all-features -- -D warnings
run cargo check -p stegoeggo --no-default-features
run cargo test --workspace --exclude stegoeggo-fuzz --all-features
```

The exact logging helper may be simpler. Correctness requirements are:

- fail fast;
- preserve command exit codes;
- no publication;
- no external tools;
- no cargo plugin installation;
- no toolchain installation;
- no package generation;
- no artifacts;
- no hidden specialist phases;
- no argument parser unless a concrete need exists.

If Plan 035 retained a focused doctest command, include it here so local and CI checks remain identical.

### CI integration

Update `.github/workflows/ci.yml` to invoke:

```bash
./scripts/check.sh
```

rather than maintaining a second copy of the command list.

This is acceptable because the script is intentionally small. Do not use `scripts/validate-release.sh` as the required CI entry point.

### Script contract documentation

At the top of the script, include a short comment:

```text
Fast deterministic checks used by local development and required CI.
Do not add external, release, network-audit, fuzz, benchmark, or publication steps here.
```

This comment is a maintenance boundary.

### Phase 1 acceptance criteria

- `scripts/check.sh` exists and is executable.
- It contains only the required fast commands.
- `ci.yml` invokes it.
- Running it locally reproduces required CI behavior.
- It does not invoke another large validator.
- Its contract prohibits specialist and publication steps.

Suggested commit:

```text
scripts: add fast check shared with CI
```

---

## Phase 2: Replace the oversized release validator with a bounded local release check

The current `scripts/validate-release.sh` is not retained as the authoritative path. It combines too many unrelated checks and encodes former RC/exact-SHA workflow requirements.

Preferred disposition:

1. create `scripts/release-check.sh`;
2. remove `scripts/validate-release.sh` after migrating any genuinely useful local behavior;
3. update all current documentation and scripts to reference the new interface.

A compatibility wrapper is discouraged. If temporarily required, it must print a deprecation warning and delegate directly to `release-check.sh`, with a follow-up deletion recorded. The preferred closure state has no wrapper.

### Required `release-check.sh` behavior

The script must:

1. fail on uncommitted changes by default;
2. run `scripts/check.sh`;
3. verify library and CLI versions are equal;
4. verify the CLI dependency requirement exactly matches the library version;
5. run package dry-runs for the library and CLI;
6. inspect package contents sufficiently to catch missing source/readme/license files;
7. never invoke `cargo publish`;
8. never create or push tags;
9. never create a GitHub release;
10. never require a GitHub Actions run ID;
11. never require an RC workflow;
12. never require an expected commit SHA argument;
13. derive versions dynamically from Cargo metadata or manifests;
14. avoid hard-coded package filenames such as `stegoeggo-0.3.2.crate`.

Recommended base commands:

```bash
./scripts/check.sh
cargo package -p stegoeggo
cargo package -p stegoeggo-cli
```

Because `stegoeggo-cli` depends on the workspace library by path and exact version, determine the correct package-dry-run sequence empirically. Acceptable approaches include:

- package the library first, then package the CLI with the workspace dependency configuration known to Cargo;
- use `cargo package --workspace` if it behaves correctly for the current workspace and does not rely on already published versions;
- use `cargo package -p stegoeggo-cli --allow-dirty` only if required during development and not for release readiness.

Do not hide a failing package relationship. Record the actual command behavior in the status ledger.

### Version extraction

Prefer:

```bash
cargo metadata --no-deps --format-version 1
```

Use an available JSON parser only if already part of the environment. A small Python standard-library parser is acceptable because Python is commonly available, but avoid adding a new dependency solely to read metadata.

A robust approach:

```bash
python3 - <<'PY'
import json
import subprocess

metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--no-deps", "--format-version", "1"
]))
packages = {pkg["name"]: pkg for pkg in metadata["packages"]}
lib = packages["stegoeggo"]
cli = packages["stegoeggo-cli"]
if lib["version"] != cli["version"]:
    raise SystemExit(
        f"version mismatch: stegoeggo={lib['version']} stegoeggo-cli={cli['version']}"
    )
print(lib["version"])
PY
```

Also verify that the CLI dependency requirement is exactly compatible with the selected publication policy. Since the repository currently uses an exact requirement, the check should prove it equals `=<workspace version>`.

### Package filename discovery

Use metadata or globbing tied to the derived version:

```bash
crate_file="target/package/stegoeggo-${version}.crate"
```

or inspect `target/package/` after packaging and require exactly one matching artifact.

Never embed a literal current version in script source.

### Optional flags

Keep the interface narrow. At most, support:

```text
--allow-dirty
--skip-check
```

Only add a flag if it serves a concrete local use. Default release readiness must require a clean tree and run the fast check.

Do not recreate phase selectors such as `--phase hermetic|feature|external|all`.

### Phase 2 acceptance criteria

- `scripts/release-check.sh` exists and is executable.
- It is local-only and non-publishing.
- It runs the fast check and package dry-runs.
- It verifies version lockstep and the CLI dependency version.
- It derives versions dynamically.
- It has no RC or expected-SHA semantics.
- `scripts/validate-release.sh` is deleted or explicitly deprecated with a recorded deletion path.
- No workflow invokes the release-check script.

Suggested commit:

```text
release: replace certification script with bounded local check
```

---

## Phase 3: Simplify or retain targeted specialist scripts

Review each existing specialist script independently. The goal is not to force all specialist checks into `release-check.sh`.

### 3.1 docs.rs reproduction

Current path:

```text
scripts/validate-docs-rs.sh
```

Retain if it provides useful reproduction of the previous docs.rs failure, but simplify it where possible.

Required corrections if retained:

- derive the package version dynamically;
- do not assume a specific `.crate` filename;
- clearly state that nightly Rust is a prerequisite;
- do not label an unqualified command as stable when the active toolchain is nightly;
- avoid redundant workspace and packaged builds unless both catch distinct failure classes;
- no dirty-tree requirement unless invoked from release readiness;
- no coupling to GitHub Actions.

Target invocation policy:

```text
Run when changing cfg(docsrs), documentation attributes, docs.rs metadata,
feature-gated public API docs, or package inclusion; also run before a release
when documentation packaging changed.
```

### 3.2 MSRV packaged consumer

Current path:

```text
scripts/validate-msrv-package.sh
```

Retain only if it remains useful as a targeted dependency/MSRV diagnostic.

Required corrections if retained:

- derive version dynamically;
- avoid six feature combinations by default;
- test minimal and all-feature consumer configurations unless a known regression requires another combination;
- make the toolchain prerequisite explicit;
- do not run from required CI;
- do not run automatically from `release-check.sh` unless dependencies, features, or MSRV changed and the maintainer chooses it.

Target invocation policy:

```text
Run when changing rust-version, dependencies, default features, optional
features, or before a release that includes dependency-resolution changes.
```

If the script’s maintenance cost exceeds its value, replace it with documented direct commands and delete it.

### 3.3 external conformance wrapper

Review:

```text
scripts/verify_metadata_conformance.sh
```

Retain if it provides a concise entry point. It should:

- check required external tools with clear errors;
- run external integration and/or conformance once;
- avoid artifact bureaucracy;
- avoid publication or package-version logic;
- match the manual workflow from Plan 035.

### 3.4 fuzz synchronization

Review:

```text
scripts/check_fuzz_sync.sh
```

Retain as a targeted maintenance check if it quickly verifies manifest/target parity. Do not include it in required CI or the default fast script.

Document when to use it: after adding, removing, or renaming fuzz targets.

### 3.5 semver, advisory, and license checks

Do not create wrapper scripts unless they materially improve clarity.

Document direct commands, selecting one advisory mechanism as the normal choice.

Recommended policy:

```bash
cargo semver-checks check-release
cargo deny check licenses
cargo deny check advisories
```

`cargo audit` may remain an alternative, but do not require both `cargo audit` and `cargo deny check advisories` for every release without a documented distinct purpose.

### Phase 3 acceptance criteria

- Every retained specialist script has one purpose.
- No retained script hard-codes the current crate version.
- No specialist script is invoked by required CI.
- No specialist script publishes.
- Invocation policy is documented by change category.
- Redundant or misleading scripts are removed.

Suggested commit:

```text
scripts: narrow specialist verification interfaces
```

---

## Phase 4: Create comprehensive manual release documentation

Create or replace:

```text
RELEASING.md
```

The document must be operationally complete but not evidence-heavy.

### Required section 1: release ownership

State explicitly:

- release cadence is a maintainer decision;
- releases are performed manually;
- GitHub Actions do not publish;
- version tags do not publish;
- GitHub releases are optional and manual;
- CI success is useful development evidence but not a publication trigger.

### Required section 2: immutable crates.io versions

State explicitly:

- once crates.io accepts a package version, its bytes cannot be replaced;
- yanking does not make a version reusable;
- deleting or moving a Git tag does not make a version reusable;
- documentation-only defects in a published crate require a new version;
- if a release attempt partially succeeds, do not republish the already accepted crate version;
- select the first unused version greater than every published version for the package.

### Required section 3: pre-release preparation

Document:

1. confirm a clean working tree;
2. select an unused version;
3. update library version;
4. update CLI version;
5. update the CLI exact library dependency;
6. update changelog and supported-version documentation;
7. run `./scripts/release-check.sh`;
8. run targeted specialist checks appropriate to the changes;
9. inspect package contents;
10. verify no publication command is being run by automation.

### Required section 4: targeted specialist decision table

Include a concise table:

```text
change type | additional check
```

Required rows:

- dependency/MSRV change → packaged MSRV consumer check;
- docs.rs/public docs configuration change → docs.rs reproduction;
- metadata serialization/parser/fixture change → external conformance;
- public API compatibility-sensitive release → semver check;
- dependency or release preparation → license/advisory check;
- parser or untrusted-input change → relevant fuzz target;
- performance-sensitive hot path → benchmark.

Clarify that this table is not a universal checklist. Run checks applicable to the release contents.

### Required section 5: publication sequence

Document the exact manual order.

Illustrative sequence:

```bash
cargo publish -p stegoeggo --dry-run
cargo publish -p stegoeggo

# Confirm the matching version is resolvable from crates.io.
cargo search stegoeggo
# or use cargo info / crates.io directly.

cargo publish -p stegoeggo-cli --dry-run
cargo publish -p stegoeggo-cli
```

Explain why the library is first: the CLI’s exact dependency must resolve from crates.io.

Do not prescribe a fixed sleep. Registry propagation should be confirmed, not guessed with `sleep 30`.

### Required section 6: optional tag and GitHub release

After successful crates.io publication:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

State:

- tagging is optional but recommended for repository history;
- the tag must point to the published source commit;
- do not force-move the tag after publication;
- a GitHub release may be created manually from the tag;
- do not attach a single Linux binary unless the maintainer intentionally supports it as a distributed artifact;
- source releases and crates.io publication do not require an automated binary artifact.

### Required section 7: partial failure handling

Cover at least:

#### Library publishes, CLI fails before acceptance

- library version is consumed;
- determine whether CLI can be corrected and published under the intended version without changing the already published library contract;
- if source changes require a different exact dependency or synchronized version policy, increment appropriately;
- never attempt to overwrite the library.

#### Both publish, docs.rs fails

- fix source/docs configuration;
- select a new unused patch version;
- republish in library-first order;
- optionally yank the defective version, understanding that it remains consumed.

#### Dry-run fails

- fix locally;
- no version is consumed until crates.io accepts publication;
- rerun the dry-run and release check.

#### Tag created before publication failure

- do not treat the tag as proof of publication;
- correct repository history carefully before public reliance, but never use tag movement to imply a published crate was replaced;
- prefer publishing first, tagging second.

### Phase 4 acceptance criteria

- `RELEASING.md` exists.
- It describes manual Cargo/crates.io publication.
- It contains no GitHub Actions publication path.
- It documents immutable versions and partial failures.
- It documents library-first publication.
- It uses registry confirmation instead of fixed sleeps.
- It makes tag and GitHub release creation optional and manual.

Suggested commit:

```text
docs: codify manual crates.io release procedure
```

---

## Phase 5: Rewrite `AGENTS.md` around the simplified model

The current `AGENTS.md` describes the former thirteen-job CI pipeline and large release validator. Rewrite the affected sections so future implementation agents receive the correct operating model.

### Required build and test section

Lead with:

```bash
./scripts/check.sh
```

Then list focused commands for single tests and specialist diagnostics.

Do not present every specialist check as part of normal development.

### Required CI section

State:

```text
Required GitHub CI is one stable Ubuntu job invoking scripts/check.sh.
It runs formatting, strict clippy, minimal-feature compilation, and all-feature
workspace tests. Specialist checks are manual and are not required on every push.
```

Do not enumerate deleted job names except in historical plans.

### Required release section

State:

```text
Releases are manual through Cargo/crates.io. GitHub Actions must not publish,
create releases, or use crates.io credentials. See RELEASING.md.
```

### Required complexity guardrail section

Add explicit invariants:

```text
- Required push/PR workflows: one.
- Required jobs per push/PR: one.
- No required job matrix.
- No tag-triggered release workflows.
- No CI publication.
- No crates.io token in GitHub Actions.
- Do not add specialist checks to scripts/check.sh.
- Preserve specialist tests, but invoke them deliberately.
- Any increase to required CI surface requires an explicit maintainer decision.
```

### Validation script inventory

Update the script list to accurately describe:

- `scripts/check.sh`;
- `scripts/release-check.sh`;
- retained targeted scripts;
- local/manual status;
- prerequisites.

Remove references to:

- release candidates;
- expected SHA validation;
- authoritative all-phase release scripts;
- mandatory external conformance in CI;
- automatic fuzz matrices;
- automated publication.

### Preserve product-specific gotchas

Do not use the CI rewrite as a reason to delete valuable architecture and correctness notes unrelated to process complexity.

### Phase 5 acceptance criteria

- `AGENTS.md` accurately describes one-job CI.
- Manual release policy is explicit.
- Complexity invariants are explicit.
- Specialist checks are clearly optional/targeted.
- Deleted workflows and script interfaces are no longer presented as current.
- Product-specific technical guidance remains intact.

Suggested commit:

```text
docs: align agent guidance with minimal CI policy
```

---

## Phase 6: Audit all current documentation and source references

Run broad searches:

```bash
rg -n "release-candidate|Release Candidate|expected-sha|authoritative validation|phase hermetic|phase feature|phase external|13 jobs|Docs.rs Build|MSRV Package Consumer|CARGO_REGISTRY_TOKEN|action-gh-release|cargo publish|mandatory CI gate|fuzz smoke" .
```

Classify each result.

### Historical plan documents

Historical plans may retain descriptions of the former system. Do not rewrite history. Current plans should mark the simplification roadmap as the active policy.

### Current operational documents

Rewrite or remove obsolete references in:

- `AGENTS.md`;
- `README.md` if release/CI claims exist;
- `SUPPORT.md`;
- `STABILITY.md`;
- `SECURITY.md`;
- developer documentation;
- script comments;
- workflow comments.

### README scope

The README should not grow into a release manual. At most, include a concise contributor command:

```bash
./scripts/check.sh
```

and a link to `RELEASING.md` for maintainers.

### Changelog

Add an unreleased entry describing process changes only if repository convention includes CI/release process changes in the changelog. Do not bump a version or publish as part of this plan.

### Phase 6 acceptance criteria

- No current operational document describes deleted workflows as active.
- `cargo publish` appears only in `RELEASING.md`, historical plans, or clearly non-executing examples.
- No current workflow or script publishes.
- No current script accepts RC/expected-SHA arguments.
- README remains user-focused.
- Historical plans remain truthful.

Suggested commit:

```text
docs: remove obsolete CI and release apparatus references
```

---

## Phase 7: Verify the complete operating model

### 7.1 Fast check

Run:

```bash
./scripts/check.sh
```

Record duration and result.

### 7.2 Release check

Run:

```bash
./scripts/release-check.sh
```

This must package successfully and must not publish.

Inspect shell tracing or source to confirm no `cargo publish`, `git tag`, `git push`, or GitHub CLI release command occurs.

### 7.3 Package contents

Inspect generated `.crate` files dynamically:

```bash
cargo package -p stegoeggo --list
cargo package -p stegoeggo-cli --list
```

Verify expected files and absence of obvious development-only artifacts.

### 7.4 Targeted specialist smoke

Exercise retained scripts once where practical:

- docs.rs script;
- MSRV script;
- external conformance wrapper or manual workflow;
- fuzz-target listing/single-target invocation;
- semver command;
- license/advisory command.

These exercises validate that demotion did not leave dead tooling. They are not added to required CI.

If a prerequisite is unavailable, record the exact prerequisite and command rather than falsely claiming success.

### 7.5 Static release-safety audit

Run:

```bash
rg -n "CARGO_REGISTRY_TOKEN|action-gh-release|cargo publish|git push.*tag|gh release create" .github scripts
```

Expected result: no executable matches.

Run:

```bash
rg -n "tags:|refs/tags|GITHUB_REF_NAME|github\.ref_name" .github/workflows
```

Expected result: no release-tag behavior.

### 7.6 Workflow complexity audit

Run:

```bash
find .github/workflows -maxdepth 1 -type f -print | sort
rg -n "matrix:|cargo install|apt-get|upload-artifact|schedule:" .github/workflows
```

Confirm:

- one automatic workflow;
- one required job;
- optional workflows are manual only;
- no required matrix;
- no required cargo/OS installs;
- no automatic schedules added by this work.

### Phase 7 acceptance criteria

- Fast check passes.
- Release check passes without publication.
- Package inventories are valid.
- Retained specialist tooling is runnable or precisely documented.
- Static searches show no automated release path.
- Workflow complexity matches the roadmap budget.

---

## Phase 8: Repository settings and branch protection closure

Source control cannot prove all repository settings. Record maintainer actions clearly.

### Required settings checklist

1. Remove obsolete Actions secret `CARGO_REGISTRY_TOKEN`.
2. Remove or repurpose the `crates-io` environment if it exists only for automated publishing.
3. Remove obsolete required status checks from branch protection/rulesets.
4. Require only the new single CI check, if branch protection is used.
5. Remove tag rules that require deleted release workflows.
6. Confirm Actions default token permissions are read-only unless another workflow needs more.
7. Confirm no organization-level reusable workflow can still publish this repository unexpectedly.

The last item is an inspection requirement, not an invitation to audit the entire organization. Check only directly applicable repository rules or reusable workflows.

### Status semantics

Use:

```text
source-complete
maintainer-setting-pending
verified-complete
```

Do not mark a setting verified without direct evidence.

---

## Phase 9: Close all ledgers and roadmap

Update:

```text
plans/034-status.md
plans/035-status.md
plans/036-status.md
```

Then add a closure section to `plans/036-status.md` summarizing Plan 033 program acceptance criteria.

Required final metrics:

```text
required automatic workflows: 1
required push/PR jobs: 1
required matrices: 0
required Rust toolchains: stable only
required cargo-installed tools: 0
required OS packages: 0
required artifact uploads: 0
tag-triggered workflows: 0
automated publication paths: 0
workflow crates.io token references: 0
manual specialist paths retained: documented
manual release procedure: documented
```

Record final commit SHAs and CI run evidence.

If repository settings remain pending, disposition may be:

```text
Source disposition: CLOSED
Repository settings disposition: PARTIAL — maintainer action required
```

Do not hold source closure hostage to settings that cannot be changed through the implementation environment, but do not conceal them.

---

## Non-goals

Do not:

- publish a release;
- bump crate versions merely to test release scripts;
- create a test tag;
- create a GitHub release;
- delete product tests to make `check.sh` fast;
- remove conformance fixtures;
- remove fuzz targets without independent justification;
- change metadata or steganography behavior;
- change MSRV;
- add provenance attestations, signing, SBOMs, or artifact inventories;
- add a release manager tool;
- add Make, Just, Task, or another command runner solely to wrap two shell scripts;
- create a new CI matrix;
- add scheduled dependency or advisory jobs;
- turn manual release into a GitHub `workflow_dispatch` action.

---

## Failure handling

### If `cargo package -p stegoeggo-cli` cannot package before the library is published

Investigate Cargo’s workspace packaging behavior. The release check may validate the CLI package after temporarily patching the dependency to the local packaged crate or by using an appropriate Cargo package workspace command, but it must not publish and must not conceal the dependency-order constraint.

Document the exact limitation in `RELEASING.md` if a perfect CLI dry-run is impossible before library publication.

### If stable and MSRV dependency resolution diverge

Keep MSRV as a targeted check. Do not add it back to required CI automatically. Pin or constrain dependencies only when this is the intended compatibility policy, and record the reason.

### If docs.rs simulation remains fragile

Narrow the script to the minimum semantic reproduction: nightly rustdoc, `DOCS_RS=1`, `cfg(docsrs)`, all relevant features. Remove pseudo-equivalence claims that cannot be guaranteed.

### If advisory tooling is unavailable locally

Document installation prerequisites. Do not have `release-check.sh` install cargo plugins on every run.

### If users depend on GitHub binary releases

Treat that as a separate distribution decision. This plan does not automate binaries. A later explicit plan may add intentional multi-platform artifacts, but it must not be smuggled into CI simplification.

### If future agents propose adding a required check

They must identify:

- the concrete failure class not covered by existing required commands;
- why a deterministic regression test cannot cover it;
- expected runtime and external dependencies;
- whether it can remain targeted/manual;
- maintainer approval for increasing the complexity budget.

---

## Definition of done

Plan 036 and the overall roadmap are complete when:

- `scripts/check.sh` is the fast local and CI entry point;
- `scripts/release-check.sh` performs bounded, local, non-publishing readiness checks;
- the oversized all-phase release validator is removed or fully retired;
- retained specialist scripts have one purpose and dynamic version handling;
- no script hard-codes `stegoeggo-0.3.2.crate` or any current release version;
- `RELEASING.md` documents direct manual crates.io publication;
- immutable version rules and partial-failure handling are explicit;
- library-first CLI-second publication is explicit;
- registry availability is confirmed rather than handled by a fixed sleep;
- tags and GitHub releases are optional manual post-publication actions;
- `AGENTS.md` describes one-job CI and the complexity budget;
- current documentation contains no operational references to deleted RC, tag-validation, or publish workflows;
- fast CI passes;
- release-check packaging passes without publication;
- specialist verification remains usable through deliberate commands;
- repository-setting follow-ups are explicit and truthfully marked;
- all status ledgers contain exact before-and-after metrics and commit evidence;
- no crate, tag, or GitHub release is created during implementation.
