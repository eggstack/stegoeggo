# Maintenance Quality Milestone 003 — Agent Guidance and Documentation Currency

Status: implemented
Repository baseline: `de69068a5d5c5da44671a97f1605fa970709a817`
Source roadmap: `plans/subsystems/maintenance-quality-roadmap.md#7-milestones`
Long-term requirements: `plans/000-long-term-specification.md#5-release-invariants`
Applicable ADRs: `ADR-0005`, `ADR-0006`
Primary class: infrastructure

## 1. Objective

Restore the accuracy and coverage of the agent-guidance surface — `AGENTS.md`,
`.skills/`, `plans/registry.md`, and the `architecture/` component index — after
the language-bindings and carrier-surface subsystems grew past what those
documents described.

## 2. Why this milestone is ready

Pure documentation and agent-guidance maintenance. It changes no product code,
no public API, no release authority, and no CI placement, so it is independent of
the blocked release-distribution stable-B evidence and of the proposed M002 lint.
It is a semantic prerequisite for future agents: the guidance currently
misdescribes CI topology and omits an entire subsystem.

## 3. Current implementation evidence

Baseline `de69068` carried these verified defects:

- `AGENTS.md` stated "Everything else (`assurance.yml`, `external-verification.yml`,
  `fuzz.yml`) is scheduled/manual". Four further workflows
  (`c-binding.yml`, `node-binding.yml`, `python-binding.yml`, `release-drift.yml`)
  actually trigger on push **and** PR to `main`. Because `bindings/` is excluded
  from the Cargo workspace, `check.sh` is blind to bindings entirely, so this
  omission directly hides a real failure mode for library work.
- `AGENTS.md` claimed `architecture/` holds 39 files; it holds 42.
- No agent guidance mentioned the FFI bindings subsystem at all, despite three
  crates, a normative ABI-V1 promise, a 90-symbol export manifest, and three
  PR-triggering CI workflows.
- `AGENTS.md` carried no index from a change area to its `architecture/` deep dive.
- `.skills/stegoeggo-conventions/SKILL.md` listed 35 test files (actual 36), 39
  architecture files (actual 42), and omitted the carrier's public `limits` and
  `webp` modules plus `metadata_trap/spec.rs`.
- `.skills/architecture-review/SKILL.md` omitted `metadata_trap/spec.rs` and did
  not flag the CI-topology or crate-count discrepancy classes.
- `.skills/plan-execution/SKILL.md` referenced a non-existent
  `subagent_type: general` and did not mention that `bindings/` sits outside the
  workspace.

## 4. Invariants that must not regress

No product-code change; no public API change; no change to required CI or to
`check.sh` contents; no publication behavior change; no rewrite of flat
`001`–`107` plan history; no change to `architecture/` claims that are already
correct; version, MSRV, and support-matrix facts unchanged.

## 5. Scope

In: `AGENTS.md`; `.skills/{bindings,stegoeggo-conventions,planning,plan-execution,architecture-review}/SKILL.md`;
`architecture/overview.md`; `plans/registry.md`; the maintenance-quality roadmap
and this plan plus its closure record.

Out: any `src/`, `stegoeggo-stego/`, `stegoeggo-cli/`, or `bindings/` code change;
`docs/` user-guide rewrites (verified accurate, see §7); `architecture/` deep-dive
rewrites (verified accurate); new scripts or CI checks; M002 drift linting.

## 6. Required production changes

1. Add `.skills/bindings/SKILL.md` covering the four binding invariants
   (`panic = "unwind"`, exact pin + `default-features = false`, no
   `forbid(unsafe_code)`, never publish), the PR-triggering-but-non-required CI
   topology, the C ABI-V1/90-symbol/generated-artifact contract, the Node
   threadpool and generated `.d.ts` rules, the Python `py.detach` GIL rule, and
   cross-binding parity.
2. Correct `AGENTS.md`: CI topology, 39 → 42 architecture files, bindings
   subsection, `webp` feature, `forbid(unsafe_code)` scoping, `limits.rs`/`webp.rs`
   carrier facts, and a change-area → architecture deep-dive index table.
3. Correct the three remaining skills and add a re-verification table so a future
   agent can re-derive every count from the tree instead of copying it.
4. Add the discovered discrepancy classes to `architecture-review/SKILL.md`.
5. Reconcile `plans/registry.md` and the maintenance-quality roadmap.

## 7. Ordered work packages

**WP1 — Bindings skill.** Write `.skills/bindings/SKILL.md`. Verify each claim
against `bindings/*/Cargo.toml`, `ABI-V1.md`, `abi-v1-symbols.txt`,
`src/{panic,error,codes}.rs`, `src/tasks.rs`, and the three binding workflows.

**WP2 — Conventions skill.** Fix counts; add `limits`/`webp` to the carrier public
list; add `metadata_trap/spec.rs`; add the bindings crate map; add a
re-verification table and a CI-reality-check paragraph; add pitfalls 23 (carrier
`limits`/`webp`) and 24 (bindings outside the workspace).

**WP3 — Remaining skills.** `architecture-review`: file count, `spec.rs`, bindings
cross-load, four new discrepancy rows. `planning`: cross-load the bindings skill;
disambiguate 12 `##` sections vs 16/12 *numbered* template sections. `plan-execution`:
replace `subagent_type: general` with the real agent roles; note the excluded crates.

**WP4 — AGENTS.md.** Rewrite per §6.2, preserving every still-correct operational
statement (commands, canonical API, carrier, CLI, features, releases).

**WP5 — Architecture + plans.** Add the bindings skill to the `overview.md` agent
guidance line; update the registry and roadmap; write the closure record.

## 8. Failure, cancellation, restart, contention semantics

Documentation-only: no runtime, no partial state, safe to abandon at any point.
Concurrent edits to `plans/registry.md` are the only realistic conflict; re-read
before writing.

## 9. Compatibility and migration

No consumer impact. Guidance text is not a compatibility contract. The `webp`
feature was already documented in `architecture/overview.md`; `AGENTS.md` now
matches it rather than changing the product.

## 10. Required tests

None. No code changed. `./scripts/check-docs-contract.sh` runs as part of
`check.sh` and covers installer/command/target wording.

## 11. Required verification commands

```bash
ls -1 architecture | wc -l                 # 42
ls -1 tests/*.rs | wc -l                   # 36
ls -1 .github/workflows/*.yml | wc -l      # 12
ls -1 fuzz/fuzz_targets/*.rs | wc -l       # 12
ls -1 docs/*.md | wc -l                    # 7
grep -c '^[a-zA-Z_]' bindings/c/abi-v1-symbols.txt   # 90
sed -n '/pub enum Error/,/^}/p' src/error.rs
./scripts/check.sh
```

## 12. Documentation updates

This plan, its closure record, `AGENTS.md`, all five skills, and the single
`architecture/overview.md` agent-guidance line.

## 13. Acceptance criteria (externally observable)

- A reader of `AGENTS.md` alone can determine which workflows run on a pull
  request and that `check.sh` cannot see `bindings/`.
- `AGENTS.md` states 42 architecture files and links every change area to a
  deep dive.
- `.skills/bindings/SKILL.md` exists and every claim in it is checkable.
- No stale count remains in any skill or in `AGENTS.md`.
- `./scripts/check.sh` exits 0; no product code changed.

## 14. Stop conditions (report rather than improvise)

Any finding that would require a product-code change, a `check.sh` change, or a
CI-placement decision is out of scope — report it instead. A count that cannot be
reproduced by its documented command is reported, not silently "corrected".

## 15. Closure evidence required

`./scripts/check.sh` green; `git diff --stat` showing no `src/`, `stego*/`, or
`bindings/` paths; the §11 command output.

## 16. Handoff notes

Closure record: `plans/closure/maintenance-quality/003-status.md`.
