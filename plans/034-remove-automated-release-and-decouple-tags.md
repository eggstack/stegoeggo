# Plan 034: Remove Automated Release and Decouple Tags

Status: Ready for implementation

Baseline: `main` after Plan 033 roadmap commit `d2f6dfc7021a3286949df96e6e7c97460149189f`

Depends on:

- `plans/033-ci-verification-and-manual-release-simplification-roadmap.md`

Must be completed before:

- `plans/035-collapse-required-ci-and-demote-specialist-verification.md`
- `plans/036-local-verification-policy-and-documentation-closure.md`

---

## Purpose

Remove all GitHub Actions paths that can publish crates, create releases, or require release-candidate evidence. This is the first implementation phase because the current tag-triggered publish workflow is a mutating control path and directly contradicts the required manual release model.

After this plan, pushing a version tag must have no publication or release effect. The maintainer will publish directly through Cargo and crates.io outside GitHub Actions.

This plan does not yet collapse ordinary CI. It removes release automation cleanly and establishes a safe baseline for the later CI reduction.

---

## Current baseline facts

At the baseline, the following workflows exist and are relevant:

```text
.github/workflows/release-candidate.yml
.github/workflows/release.yml
.github/workflows/publish.yml
```

Their current responsibilities include:

- manually validating an exact 40-character commit SHA;
- creating evidence artifacts for release candidates;
- requiring a successful prior RC run for the publication SHA;
- validating version-tag equality against library and CLI manifests;
- installing cargo-deny, cargo-audit, and cargo-semver-checks;
- installing ExifTool, xmllint, ImageMagick, and libvips;
- running the complete release validator;
- publishing `stegoeggo` and `stegoeggo-cli` to crates.io;
- waiting for crates.io index propagation;
- building a Linux release binary;
- creating a GitHub release;
- generating checksums and dependency inventories.

The publication workflow is triggered by a `v*` tag and references `CARGO_REGISTRY_TOKEN`.

---

## Required outcome

The following files are deleted:

```text
.github/workflows/release-candidate.yml
.github/workflows/release.yml
.github/workflows/publish.yml
```

No replacement GitHub publication workflow is created.

No `workflow_dispatch` publication workflow is created.

No tag-triggered validation workflow is retained.

No GitHub release creation workflow is retained.

The repository source contains no executable GitHub Actions reference to:

```text
CARGO_REGISTRY_TOKEN
cargo publish
softprops/action-gh-release
gh run list --workflow release-candidate.yml
```

Human-facing documentation may contain `cargo publish` examples because manual release documentation is added in Plan 036.

---

## Phase 0: Establish a truthful pre-change ledger

Create `plans/034-status.md` before deleting workflows.

Initialize it with:

```text
Plan baseline SHA: d2f6dfc7021a3286949df96e6e7c97460149189f
Disposition: OPEN
Automated crates.io publication: present
Tag-triggered release mutation: present
Release-candidate workflow: present
Tag-validation workflow: present
GitHub release automation: present
```

Add a workflow inventory table:

```text
workflow path | trigger | mutates crates.io | mutates GitHub releases | requires secrets | depends on other run | disposition
```

Populate one row for each of the three workflows before deletion.

Add a repository-settings follow-up table:

```text
setting | current source reference | source reference after plan | maintainer action required | status
```

Required settings rows:

- `CARGO_REGISTRY_TOKEN` Actions secret;
- `crates-io` GitHub environment;
- environment protection rules or reviewers tied to publication;
- branch-protection checks named after release workflows, if any;
- repository rulesets that require tag workflows, if any.

The implementation agent may not be able to change all repository settings through source control. In that case, record the exact manual action and do not falsely mark it complete.

### Phase 0 acceptance criteria

- `plans/034-status.md` exists.
- Every release workflow and trigger is inventoried.
- Every source-controlled secret reference is identified.
- Non-source repository settings are separated from source changes.
- No workflow has yet been deleted in the same evidence commit unless the agent cannot make separate commits; if combined, clearly preserve the before-state in the ledger.

Suggested commit:

```text
plans: inventory automated release control paths
```

---

## Phase 1: Delete release-candidate validation

Delete:

```text
.github/workflows/release-candidate.yml
```

Do not replace it with another exact-SHA certification workflow.

Remove or update source-controlled references to:

- “Release Candidate workflow”;
- successful RC run requirements;
- RC run IDs;
- exact-SHA release evidence;
- RC conformance artifacts;
- RC package inventories;
- RC tool-version manifests;
- release-candidate concurrency groups.

At this phase, documentation changes should be limited to preventing broken references. Full replacement policy belongs in Plan 036.

### Required search

Run:

```bash
rg -n "release-candidate|Release Candidate|RC run|rc_sha|rc-conformance|rc-package|rc-tool" .
```

Classify every result as one of:

- obsolete operational reference to remove now;
- historical plan evidence that may remain;
- documentation to rewrite in Plan 036;
- unrelated text.

Historical planning documents should not be rewritten merely to pretend the old workflow never existed. Current operational documentation must not instruct agents to use it.

### Phase 1 acceptance criteria

- The RC workflow file is absent.
- No current workflow searches for or depends on an RC run.
- No current operational document requires RC evidence.
- Historical plans remain truthful.
- No substitute certification workflow is introduced.

Suggested commit:

```text
ci: remove release-candidate workflow
```

---

## Phase 2: Delete tag-validation workflow

Delete:

```text
.github/workflows/release.yml
```

This workflow is not a publication safety gate because it executes independently on the same tag event. Its removal is intentional.

Do not retain a smaller tag-version verification job. Version consistency will be checked locally before publication in Plan 036.

Do not add a tag-triggered package dry-run.

Do not add a tag-triggered docs.rs check.

Do not add a tag-triggered crates.io existence check.

### Required trigger audit

Search all workflows:

```bash
rg -n "tags:|refs/tags|github\.ref_name|GITHUB_REF_NAME|on:\s*push" .github/workflows
```

After this phase, no workflow should contain a tag-specific trigger. A generic `push` trigger in ordinary CI is acceptable only if it is branch-scoped or otherwise does not create tag-specific release behavior. Plan 035 will finalize CI triggers.

### Phase 2 acceptance criteria

- `.github/workflows/release.yml` is absent.
- No workflow validates or acts on `v*` tags.
- Tag/version equality logic is removed from GitHub Actions.
- No tag-triggered replacement workflow exists.
- Current operational docs do not claim that tag validation is required.

Suggested commit:

```text
ci: remove tag-triggered release validation
```

---

## Phase 3: Delete automated publication and GitHub release creation

Delete:

```text
.github/workflows/publish.yml
```

This deletion must remove all of the following automation:

- lookup of successful RC runs;
- exact-SHA checkout transfer;
- pre-publish full validation;
- crates.io token use;
- library publication;
- fixed sleep for registry indexing;
- CLI publication;
- GitHub environment use;
- release-binary compilation;
- checksum generation;
- GitHub release creation;
- dependency-inventory artifact upload.

Do not preserve the workflow under another name.

Do not convert it to `workflow_dispatch`.

Do not keep a disabled copy in `.github/workflows/` because disabled-looking YAML can be accidentally re-enabled and remains misleading operational documentation.

If historical preservation is desired, Git history already preserves the file.

### Required source audit

Run:

```bash
rg -n "CARGO_REGISTRY_TOKEN|cargo publish|action-gh-release|crates-io|publish-\$\{\{|Wait for crates.io|dependency-inventory" .github scripts AGENTS.md README.md plans
```

Interpret results carefully:

- no current workflow may reference any publication token or publication command;
- current scripts must not publish;
- historical plans may describe the removed mechanism;
- Plan 036 will introduce manual `cargo publish` commands in `RELEASING.md`;
- `crates.io` links and dependency documentation are not publication automation and should remain.

### Phase 3 acceptance criteria

- `.github/workflows/publish.yml` is absent.
- No workflow has `contents: write` solely for release creation.
- No workflow references `CARGO_REGISTRY_TOKEN`.
- No workflow invokes `cargo publish`.
- No workflow invokes a GitHub release action.
- No workflow waits for crates.io index propagation.
- No workflow uses the `crates-io` environment.
- No release publication path remains in GitHub Actions.

Suggested commit:

```text
release: remove GitHub Actions publication path
```

---

## Phase 4: Add temporary manual-release ownership note

Plan 036 will add the complete release procedure. Until then, add a concise current-policy note to the most appropriate existing operational document, normally `AGENTS.md`, stating:

```text
Releases are manual. GitHub Actions must not publish crates or create releases.
Use direct Cargo/crates.io publication after local validation.
Do not push a version tag as a publication mechanism.
Published crates.io versions are immutable and cannot be reused.
```

Keep this temporary edit short. Do not attempt the full documentation rewrite in this plan.

If `RELEASING.md` already exists, place the temporary note there instead and let Plan 036 expand it.

### Phase 4 acceptance criteria

- A current operational document explicitly says release publication is manual.
- The note does not claim the full local release process is already implemented.
- The immutable-version rule is retained.
- The library-before-CLI ordering is not contradicted.

Suggested commit:

```text
docs: declare manual release ownership
```

---

## Phase 5: Validate inert tag behavior and repository source state

This phase must not push a real release tag.

Perform static verification:

```bash
find .github/workflows -maxdepth 1 -type f -print | sort
rg -n "tags:|refs/tags|GITHUB_REF_NAME|github\.ref_name" .github/workflows || true
rg -n "CARGO_REGISTRY_TOKEN|cargo publish|action-gh-release|release-candidate" .github scripts AGENTS.md || true
```

Expected conclusions:

- release-candidate, release, and publish workflow files are absent;
- no workflow has a tag trigger;
- no workflow can publish;
- no workflow can create a GitHub release;
- manual release policy is visible.

Also run YAML syntax validation through an available lightweight method. Do not install a new CI framework merely for this check. Acceptable approaches include:

- parse remaining YAML with an already available YAML parser;
- inspect through GitHub Actions after the commit;
- use an existing repository validation command if one already exists.

Do not create or push a synthetic `v0.0.0-test` tag to prove inertness.

### Phase 5 acceptance criteria

- Static searches show no executable release automation.
- Remaining workflows are syntactically valid.
- Ordinary source checks still pass or any pre-existing failure is recorded.
- No crates were published.
- No tags were created, moved, or deleted.
- No GitHub release was created.

---

## Phase 6: Close the status ledger

Update `plans/034-status.md` with:

```text
Disposition: CLOSED
Automated crates.io publication: absent
Tag-triggered release mutation: absent
Release-candidate workflow: absent
Tag-validation workflow: absent
GitHub release automation: absent
```

Add a deletion table:

```text
path | prior trigger | prior mutation | deletion commit | verified absent
```

Add static search evidence and the final commit SHA.

Record manual repository-setting work separately. A plan can be source-complete while settings cleanup remains a maintainer follow-up, but the ledger must not mark those settings as changed unless verified.

### Required manual repository-setting follow-up

The final handoff must tell the maintainer to inspect and, if obsolete, remove:

1. Actions secret `CARGO_REGISTRY_TOKEN`;
2. GitHub environment `crates-io`;
3. environment reviewers or protection rules used only by automated publication;
4. branch-protection or ruleset checks referring to deleted workflows;
5. tag rules that assume the deleted validation workflow.

These items are not excuses to retain source-controlled release automation.

---

## Non-goals

Do not perform the following in this plan:

- collapse ordinary CI jobs;
- change fuzz triggers;
- redesign `scripts/validate-release.sh`;
- delete docs.rs or MSRV scripts;
- delete external conformance capability;
- publish a release manually;
- bump crate versions;
- change changelog release entries;
- create a GitHub release manually;
- alter product code;
- add signing, provenance, SBOM, attestation, or artifact frameworks;
- replace GitHub publication with another CI provider.

Those actions are either handled by later plans or outside this roadmap.

---

## Failure handling

### If a deleted workflow is required by branch protection

Do not restore the workflow. Record the exact required-check name and instruct the maintainer to remove or replace it in repository settings.

### If a tag push is needed for unrelated testing

Do not use a release-like tag. This plan does not require tag testing; static trigger inspection is sufficient.

### If documentation contains extensive historical RC references

Preserve historical plan records. Remove only current operational instructions. Plan 036 will perform the comprehensive documentation pass.

### If a maintainer still wants GitHub release notes

Document that GitHub releases may be created manually after crates.io publication. Do not automate them in this plan.

### If the library and CLI release order is questioned

Preserve library-first publication because `stegoeggo-cli` declares an exact dependency on the matching `stegoeggo` version. Plan 036 will codify the sequence.

---

## Definition of done

Plan 034 is complete when all of the following are true:

- `release-candidate.yml`, `release.yml`, and `publish.yml` are deleted;
- no GitHub Actions workflow publishes crates;
- no GitHub Actions workflow creates GitHub releases;
- no GitHub Actions workflow is triggered specifically by version tags;
- no workflow references `CARGO_REGISTRY_TOKEN`;
- no workflow depends on a previous RC run;
- current operational documentation states that release publication is manual;
- immutable crates.io versions remain documented;
- source validation confirms the deleted control paths are absent;
- the status ledger records source completion and separate repository-setting follow-ups;
- no release, tag, or publication mutation occurred while implementing the plan.
