# Maintenance Quality Milestone 003 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/maintenance-quality/003-agent-guidance-and-doc-currency.md`
Source subsystem roadmap: `plans/subsystems/maintenance-quality-roadmap.md#7-milestones`
Repository baseline reviewed: `de69068a5d5c5da44671a97f1605fa970709a817`
Implementation commits: this record plus the guidance/documentation changes it closes (see git log for `docs: restore agent guidance and documentation currency`).
Final qualification: `./scripts/check.sh` exit 0 at the implementation tree (§4). No product code, public API, CI placement, or publication behavior changed; nothing was published.

## 1. Executive finding

The agent-guidance surface was materially behind the codebase. The most consequential
defect was a false CI claim: `AGENTS.md` asserted that every workflow other than `ci.yml`
was scheduled or manual, while `c-binding.yml`, `node-binding.yml`, `python-binding.yml`,
and `release-drift.yml` in fact trigger on push **and** PR to `main`. Because `bindings/`
is excluded from the Cargo workspace, `check.sh` cannot see a binding at all — so the old
text taught agents that a class of failures is invisible when in fact it is caught, and
taught them nothing about the FFI subsystem that those workflows cover. Alongside that,
`AGENTS.md` carried a wrong architecture file count (39 vs 42), no index from a change
area to its deep dive, and no mention of three FFI crates with a normative ABI-V1 promise
and a 90-symbol export manifest. Four skills had drifted counts or module lists, and
`plan-execution` referenced a subagent type that does not exist in this harness.

M003 closes. A reader of `AGENTS.md` alone can now determine which workflows run on a pull
request, that `check.sh` is blind to `bindings/`, and which architecture deep dive to read
before changing any area. `.skills/bindings/SKILL.md` documents the FFI subsystem's
non-negotiable invariants. Counts are no longer copied blindly: the conventions skill
tabulates the command that re-derives each one. M002 remains proposed and still needs a
maintainer decision on CI placement.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Add `.skills/bindings/SKILL.md` (§6.1) | New file; covers `panic = "unwind"`, `=0.4.2` + `default-features = false`, no `forbid(unsafe_code)`, never publish, PR-triggering CI, ABI-V1/90 symbols/generated header, Node threadpool + generated `.d.ts`, Python `py.detach`, cross-binding parity (§3) |
| Every binding claim is checkable (§6.1, WP1) | Verified against `bindings/*/Cargo.toml`, `ABI-V1.md`, `abi-v1-symbols.txt`, `src/{panic,codes,error}.rs`, `src/tasks.rs`, `pyproject.toml`, and the three binding workflows (§4) |
| Correct `AGENTS.md` CI topology (§6.2) | Rewritten CI section separating the one required gate from the four additional push/PR workflows and the genuinely scheduled three (§3) |
| Architecture count 39 → 42 (§6.2) | `ls -1 architecture \| wc -l` = 42; `AGENTS.md` and both skills now state 42 (§4) |
| Bindings subsection + deep-dive index (§6.2) | New `## Bindings essentials` section and a 12-row change-area → `architecture/*.md` index table (§3) |
| Carrier `limits.rs` / `webp.rs` facts (§6.3, WP2) | Added to the carrier module list and as new pitfalls 23/24; `webp` added to the features list in `AGENTS.md` and the skill (§3) |
| Correct the three remaining skills (§6.3, WP3) | `architecture-review`: count, `metadata_trap/spec.rs`, bindings cross-load, four new discrepancy rows. `planning`: bindings cross-load, 12 `##` vs 16/12 *numbered* section disambiguation. `plan-execution`: real agent roles replacing `subagent_type: general`, excluded-crate note (§3) |
| Re-verification table so counts are re-derived (§6.3) | Table in `.skills/stegoeggo-conventions/SKILL.md` mapping each count to its shell command, plus a CI-reality-check paragraph (§3) |
| Reconcile registry + roadmap (§6.5, WP5) | `registry.md` maintenance row and Recently-closed entry; roadmap §1 ownership, §4 state, §7 milestone, §12 table (§3) |
| No product code changed (§4, §13) | `git status --short` lists only `.skills/`, `AGENTS.md`, `architecture/overview.md`, and `plans/` — no `src/`, `stego*/`, or `bindings/` path (§4) |
| `check.sh` green (§11, §13) | Exit 0 (§4) |
| `docs/` and README left alone (§5, §7) | Verified accurate, so no change was made: `docs/cli-usage.md` documents the five canonical commands and all five exit codes; README's policy/preset/feature tables match the code (§4) |

## 3. Production implementation evidence

Guidance and documentation only. Concretely:

- `.skills/bindings/SKILL.md` (new) — the FFI subsystem's invariants, CI topology, and
  per-language contracts.
- `AGENTS.md` (rewritten) — corrected CI topology, corrected count, `## Bindings
  essentials`, carrier `limits.rs`/`webp.rs`, `webp` feature, corrected
  `forbid(unsafe_code)` scoping (it applies to the root and carrier crates only — the FFI
  leaves are inherently `unsafe`), change-area → deep-dive index, and a closing note that
  `check-docs-contract.sh` does not validate counts or API claims.
- `.skills/stegoeggo-conventions/SKILL.md` — test count 35 → 36, architecture 39 → 42,
  carrier `limits`/`webp` added, `metadata_trap/spec.rs` added, bindings crate map, the
  re-verification table, the CI reality check, and pitfalls 23–24.
- `.skills/architecture-review/SKILL.md` — bindings source-tree entry, bindings cross-load,
  `metadata_trap/spec.rs`, and four new discrepancy patterns (CI topology, crate count,
  `spec.rs`, Cargo-registered examples).
- `.skills/planning/SKILL.md`, `.skills/plan-execution/SKILL.md` — corrections above.
- `architecture/overview.md` — the agent-guidance line now lists the `bindings` skill.
- `plans/` — this plan, this record, `registry.md`, and the maintenance-quality roadmap.

`architecture/overview.md`, `architecture/testing.md`, `architecture/tooling.md`, the other
37 deep dives, `docs/`, and `README.md` were reviewed and left unchanged: their claims were
already accurate (the previous commit had reconciled `overview.md` against the tree, and
this milestone confirmed 42/36/12/12/7/90 independently).

## 4. Verification executed (exact commands + results)

```text
$ ls -1 architecture | wc -l                                   → 42
$ ls -1 tests/*.rs | wc -l                                     → 36
$ ls -1 .github/workflows/*.yml | wc -l                        → 12
$ ls -1 fuzz/fuzz_targets/*.rs | wc -l                         → 12
$ ls -1 docs/*.md | wc -l                                      → 7
$ grep -c '^[a-zA-Z_]' bindings/c/abi-v1-symbols.txt           → 90
$ ./scripts/check-docs-contract.sh
  Documentation contracts valid: 5 release targets              → exit 0
$ ./scripts/check.sh                                           → exit 0
$ git status --short
   M .skills/architecture-review/SKILL.md
   M .skills/plan-execution/SKILL.md
   M .skills/planning/SKILL.md
   M .skills/stegoeggo-conventions/SKILL.md
   M AGENTS.md
   M architecture/overview.md
  ?? .skills/bindings/
   (+ plans/ files)
```

**Pre-existing flake found while verifying (not introduced by M003).** The first
`check.sh` run failed `tests/release_eggpack.rs::validator_temp_state_is_cleaned`
(`assert_eq!(before.len(), after.len())` — left 0, right 1). The test counts
`stegoeggo-release-smoke-*` entries in the shared system temp directory before and
after running `scripts/smoke-release-binary.py`, so it races the sibling validator
tests in the same binary, which create the same prefix concurrently. It passes in
isolation (`--test release_eggpack validator_temp_state_is_cleaned` → ok) and
intermittently in the full file (15 passed on re-run). M003 changed no `.rs` file —
`git status --short -- src stegoeggo-stego stegoeggo-cli bindings tests` is empty —
so this is independent of this milestone. Recorded in §10 as a required-CI
reliability risk owned by a future corrective, not fixed here because the plan's
§14 stop condition puts product/test-code changes out of scope.

Binding-side claims were verified by reading `bindings/c/Cargo.toml`,
`bindings/node/Cargo.toml`, `bindings/python/Cargo.toml`, `bindings/c/ABI-V1.md`,
`bindings/c/abi-v1-symbols.txt`, `bindings/c/src/panic.rs`, `bindings/python/src/lib.rs`
(PyO3 0.27 spells GIL release `py.detach`, **not** the older `allow_threads`; the skill
records the correct spelling), `bindings/node/src/tasks.rs`, and the `on:` block of each
of the 12 workflows.

## 5. Invariant review

- **No product code change** — confirmed by `git status` (§4).
- **No public API, version, or MSRV change** — no manifest was edited.
- **No CI placement change** — no workflow was added, removed, or retargeted; `check.sh`
  is byte-identical.
- **No publication behavior change** — untouched by construction.
- **Flat `001`–`107` history preserved** — only `registry.md` and the subsystem roadmap
  were edited; no flat plan was modified.
- **Correct architecture claims left intact** — a single line in `overview.md` changed (the
  skills list). No corrected-elsewhere claim was "re-corrected".

## 6. Failure and recovery review

Documentation-only, so there is no partial runtime state. The one real risk was writing a
count that could not be reproduced; the plan's §14 stop condition required reporting such
a count rather than adjusting it, and all six counts reproduced exactly on the first run.
`py.detach` was initially written as `allow_threads` and was corrected against
`bindings/python/src/lib.rs` before the skill was finalized.

## 7. Migration and compatibility review

None required. Guidance text is not a compatibility contract, and no consumer compiles
against it. The only behavior-adjacent fact added — that `webp` is a default-off feature —
was already documented in `architecture/overview.md`; `AGENTS.md` was brought in line with
existing reality rather than the product being changed.

## 8. Security review

No security-relevant change. One guidance invariant worth recording: `panic = "unwind"` in
all three binding profiles is what makes panic containment at the FFI boundary possible,
and the root crate's `panic = "abort"` means silently adopting it in a binding would abort
a host process. That is now stated in `AGENTS.md` and `.skills/bindings/SKILL.md` so the
next agent does not "simplify" it away.

## 9. Documentation and operations

`.skills/bindings/SKILL.md`, `AGENTS.md`, the four existing skills,
`architecture/overview.md` (one line), this plan, this record, `registry.md`, and the
maintenance-quality roadmap. `check-docs-contract.sh` stays green, so the
installer/command/target wording contract is unaffected.

## 10. Unresolved findings

- **Medium — `tests/release_eggpack.rs::validator_temp_state_is_cleaned` is flaky in
  required CI.** It compares the count of `stegoeggo-release-smoke-*` entries in the
  shared system temp directory across a validator run, so it races the other validator
  tests in the same binary. Observed failing on the first `check.sh` run of this
  milestone and passing on re-run (§4). A green/red flip in the required gate is
  worth a corrective: give each validator test a unique temp prefix, or serialize them
  behind a mutex. Deliberately not fixed here — M003 changed no code, and §14 puts
  product/test changes out of scope. Owner: a future release-distribution or
  maintenance-quality corrective, not this milestone.
- **Medium — counts are still manually maintained.** `check-docs-contract.sh` enforces
  wording but not counts, so this class of drift can recur. M002 is the natural home but
  is proposed and needs a maintainer decision on CI placement; adding a count check to
  `check.sh` was explicitly out of scope here.
- **Low — no skill covers release/updater mechanics.** The updater invariants live in
  `AGENTS.md`, `RELEASING.md`, and `docs/installation.md` and are accurate, but unlike
  bindings they have no dedicated skill. Not worth one until someone changes that path.
- **Low — `.skills/` has no index file.** Each `SKILL.md` carries its own trigger
  description and `AGENTS.md` lists them, so this is cosmetic.

None of these block M003 closure.

## 11. Roadmap disposition

Maintenance-quality M003 is closed. The roadmap stays `closed`: M001 and M003 are done and
M002 remains optional polish requiring an explicit maintainer decision on CI placement,
exactly as before this milestone. The roadmap's §1 ownership boundary was extended to name
agent-guidance currency, since M003 established it as recurring work rather than a one-off.

## 12. Registry updates

`plans/registry.md`: the maintenance-quality row now reads "M001 and M003 closed; M002
proposed optional polish", and a Recently-closed entry records this closure with its
substantive findings. No other subsystem's status, blocker, or milestone changed — in
particular the release-distribution blockers and the stego-library-evolution M006 ADR-0007
blocker are untouched and remain accurate.
