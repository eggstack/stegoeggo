# Maintenance Quality Milestone 001 — Release Workflow Consolidation

Status: ready for handoff  
Repository baseline: `d5425d3c73e634fd2276d44507fa94728a18784f`  
Source roadmap: `plans/subsystems/maintenance-quality-roadmap.md#7-milestones`  
Long-term requirements: `plans/000-long-term-specification.md#5-release-invariants`  
Applicable ADRs: `ADR-0004`, `ADR-0005`, `ADR-0006`  
Primary class: infrastructure

## 1. Objective

Reduce GitHub Actions maintenance and supply-chain drift by factoring repeated
release/binding bootstrap logic and pinning third-party actions immutably, while
preserving every current runner, target, artifact, smoke, and manual-publication
contract.

## 2. Why this milestone is ready

It is independent of the blocked real A→B updater proof. Current workflows are already
qualified; this is a semantics-preserving maintenance pass.

## 3. Current implementation evidence

- `.github/workflows/release-binaries.yml` contains repeated checkout, Rust,
  cargo-zigbuild, verified Zig, Eggpack, artifact staging, and target setup.
- `release-python.yml`, `release-node.yml`, and `release-c.yml` repeat platform
  setup and use several moving action tags.
- Binary workflow already pins multiple actions by SHA, proving the repo accepts that
  maintenance style.
- GitHub recommends reusable workflows/composite actions for duplication and full SHA
  pins for immutable action references.

## 4. Invariants that must not regress

Manual dispatch/publication only; same target matrices; same glibc floors; same Rust
MSRV/build toolchains; same C symbol audit; same Python wheel/sdist smokes; same Node
native-addon smokes; same Eggpack artifact identity; no secrets expansion.

## 5. Scope

In: local composite actions and/or reusable workflows for genuinely repeated setup;
shared scripts for verified Zig/cargo-zigbuild where shell reuse is clearer; full-SHA
pinning; comments/update documentation; matrix-equivalence guard.

Out: changing package versions, targets, publication policy, release authority,
artifact names, or required CI policy.

## 6. Required production changes

1. Inventory every external `uses:` reference in release/binding workflows and resolve
   the intended upstream release to a verified full commit SHA.
2. Factor repeated step sequences with the smallest abstraction:
   - composite action for repeated steps inside jobs;
   - reusable workflow only for repeated whole-job structure;
   - checked shell script where cross-workflow command logic is the real duplicate.
3. Keep target-specific build commands visible in caller workflows when hiding them
   would reduce diagnosability.
4. Centralize verified Zig 0.14.1 provisioning and exact cargo-zigbuild setup used by
   Linux release/C workflows, preserving architecture-specific checksums.
5. Add a non-required contract checker or fixture that compares expected target IDs,
   artifact names, and Rust/tool versions across workflows. Do not wire it into required
   CI without explicit maintainer approval.
6. Update workflow comments so pin update procedure is obvious.

## 7. Ordered work packages

WP1 inventory/pin table.  
WP2 shared bootstrap extraction.  
WP3 migrate binary/C workflows, then Python/Node where applicable.  
WP4 matrix/artifact equivalence audit.  
WP5 manual-dispatch smoke/qualification evidence and docs.

## 8. Failure, cancellation, restart, contention semantics

Existing workflow concurrency settings remain. Shared setup must fail closed on checksum,
tool version, missing artifact, or source-ref mismatch. No partial publication is added.

## 9. Compatibility and migration

No user-facing migration. Maintainers gain fewer duplicated update sites. Pin updates
become explicit reviewed changes.

## 10. Required tests

YAML parse/actionlint if available; local script unit checks; exact matrix diff against
baseline; exercise representative Linux/macOS/Windows paths through existing manually
dispatchable workflows without publishing.

## 11. Required verification commands

```bash
./scripts/check.sh
git diff -- .github/workflows .github/actions scripts
```

Run `actionlint` if available and record version/result. Run the affected manual
workflows far enough to prove setup/build/smoke equivalence; closure must record run IDs.

## 12. Documentation updates

`RELEASING.md`, `SUPPORT.md` only where workflow maintenance instructions change,
`architecture/tooling.md`, roadmap/registry status.

## 13. Acceptance criteria

Every third-party action in affected release/binding workflows is full-SHA pinned;
duplicated verified-tool bootstrap has one maintained source; target/artifact/smoke
contracts are unchanged; representative workflow runs are green; no publication occurs.

## 14. Stop conditions

Stop if factoring changes runner architecture, artifact identity, publication
permissions, or obscures target-specific behavior enough that failures cannot be
diagnosed from logs.

## 15. Closure evidence required

Before/after workflow line/duplication inventory, pin table with upstream versions,
matrix/artifact equivalence, actionlint result if available, workflow run IDs,
`check.sh`.

## 16. Handoff notes

Prefer a modest reduction in duplication with transparent logs over a highly abstract
workflow framework.
