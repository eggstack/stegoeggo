# Maintenance Quality Milestone 005 — Release-contract test isolation and README re-verification

Status: implemented
Repository baseline: `f19985e58a609ae8714cda93ed17b1e08cd7c236`
Source roadmap: `plans/subsystems/maintenance-quality-roadmap.md#7-milestones`
Long-term requirements: `plans/000-long-term-specification.md#5-release-invariants`
Applicable ADRs: none; this changes no release, publication, or authority behavior
Primary class: bug fix (test isolation) + documentation

## 1. Objective

Remove the pre-existing required-CI flake in
`tests/release_eggpack.rs::validator_temp_state_is_cleaned` that M003 first
reported and M004 re-deferred, and re-verify the user-facing documentation by
executing it after the M004 rewrite.

## 2. Why this milestone is ready

The defect is fully diagnosed and the fix is proven against the pre-fix code, so
nothing here depends on an open decision. It is independent of the blocked
release-distribution work and of proposed M002 linting, and it needs no new CI
check: the flaky test already runs inside required `check.sh`.

## 3. Current implementation evidence

Baseline `f19985e`, established by running the test binary in a loop rather than
a single pass:

- Full binary, 60 runs, default 14 threads: **8 failures**, always
  `assert_eq!(before.len(), after.len())` with `left: 0, right: 1`.
- The single test in isolation, 40 runs: **0 failures** — it passes alone and
  fails in the suite, which is the signature of cross-test interference.
- Pairing the test with each sibling for 30 runs at 8 threads:

| Sibling | Failures |
|---|---|
| `validator_happy_path_with_fake_candidate` | **4/30** |
| `validator_wrong_version_fails` | 0/30 |
| `validator_nonzero_help_fails` | 0/30 |
| alone | 0/30 |

Root cause: `validator_temp_state_is_cleaned` counted
`stegoeggo-release-smoke-*` entries in the *shared* `std::env::temp_dir()`.
`scripts/smoke-release-binary.py:105` creates one such directory for the lifetime
of its subprocess. `validator_happy_path_with_fake_candidate` runs the same
happy-mode validator, and Cargo's harness runs tests concurrently, so the
sibling's live directory was counted as this test's residue. The two non-racing
siblings fail at `smoke-release-binary.py:94` and `:98`, before `mkdtemp`, so
they never create a prefixed directory.

Pre-fix rate by harness parallelism: 2 threads 0/40, 4 threads 5/40, 8 threads
3/40, 14 threads 8/60. Because `scripts/check.sh:16` runs
`cargo test --workspace --all-features`, this is a flake in the single required
gate, and 4-vCPU CI runners sit inside the observed failure band.

A second, sharper consequence: the count-only assertion could not distinguish
"sibling left a directory" from "this run leaked one". Both produced `left + 1`.

## 4. Invariants that must not regress

- The temp-state test must still fail when the validator genuinely fails to clean
  up. Isolating the observation must not turn the test into a no-op.
- `scripts/smoke-release-binary.py` must remain byte-identical: this is a
  production release-path validator, and the fix is not allowed to change it to
  suit a test.
- The three sibling validator tests must keep their existing behavior and
  must not acquire a private temp root they do not need.
- The `assert_eq!(before.len(), after.len())` assertion shape is retained.
- No public API, version, publication, or CI placement change.

## 5. Scope

In: `tests/release_eggpack.rs` (temp-root redirection and a shared entry
helper), `architecture/testing.md` (name the isolation property), `README.md`
(one example fix found by re-verification), plan/closure/registry/roadmap.

Out: `scripts/smoke-release-binary.py` (deliberately unchanged), the other
`docs/` guides, required-CI composition, the `ignore_errors=True` finding below.

## 6. Required production changes

None. `tests/release_eggpack.rs` is a test target, not production code, and the
validator script is untouched. The only functional change in the repository is
the test-isolation fix itself.

## 7. Ordered work packages

**WP1 — Reproduce and isolate the racer.** Run the binary in a loop, then pair
the test with each sibling to prove which one perturbs the shared count.

**WP2 — Fix by observation scoping.** Give `run_validator` an optional temp
root; when present, set `TMPDIR`/`TEMP`/`TMP` on the child so Python's
`tempfile` creates its directory under a directory only this test can write.
Count entries in that private root instead of the shared system temp. Extract
the duplicated `read_dir` filter into `smoke_temp_entries`.

**WP3 — Prove the fix.** Re-run the same 60-iteration loop that previously
failed 8 times, and confirm the count is zero. Then confirm the test still has
teeth by temporarily disabling the validator's `shutil.rmtree` and checking the
test fails, restoring the script afterward and verifying it is byte-identical to
HEAD.

**WP4 — Re-verify the user-facing documentation.** Execute every README command
against a clean, unmarked PNG; check the preset table against observed evidence
strength; extract the Rust example verbatim into a scratch crate and run it;
confirm documented exit codes against observed behavior and against the source
mapping; confirm every internal link resolves.

**WP5 — Documentation touch-up.** Name the temp-state isolation property in
`architecture/testing.md` so the next agent does not re-introduce a
shared-directory assertion, and apply the README fix WP4 found.

## 8. Failure, cancellation, restart, contention semantics

The test change is self-contained: a private directory under the test's own
`tempfile::tempdir()`, so the fixture and the candidate are removed together
when the test ends. There is no persistent state and no cross-test lock, which
is the point: a lock would hide the shared-global assumption rather than remove
it. The validator's own `finally` cleanup is unchanged, so an aborted validator
still leaves its directory inside the private root where the test, not the
system temp, absorbs it.

## 9. Compatibility and migration

None. The other three validator tests call `run_validator` with no temp root
and behave exactly as before; the validator script's interface is unchanged;
`TMPDIR`/`TEMP`/`TMP` are set only on a child process that this test spawns.

## 10. Required tests

The existing `tests/release_eggpack.rs` suite. No new test is required: the
defect is in the test itself, and WP3's two-sided proof (clean under
concurrency, still failing on a simulated leak) is the evidence that the fixed
test is both stable and sensitive.

## 11. Required verification commands

```bash
# defect: full binary, looped at default parallelism
BIN=target/debug/deps/release_eggpack-*
for i in $(seq 1 60); do "$BIN" || echo "FAIL $i"; done

# racer isolation: 30 runs per sibling pairing
"$BIN" --test-threads=8 validator_temp_state_is_cleaned \
                        validator_happy_path_with_fake_candidate

# teeth: disable the validator's rmtree, confirm the test still fails, restore
# README examples, verbatim, from a scratch directory
./scripts/check.sh
```

## 12. Documentation updates

`architecture/testing.md` (release-contract row names the temp-state isolation
property). `README.md` only if re-verification finds a defect — recorded in the
closure record either way.

## 13. Acceptance criteria (externally observable)

- The 60-iteration loop that previously failed 8/60 now fails 0/60.
- Pairing with `validator_happy_path_with_fake_candidate` at 8 threads fails
  0/30, where the baseline was 4/30.
- With the validator's `rmtree` disabled the test still fails; the script is
  byte-identical to HEAD afterward.
- `./scripts/check.sh` exits 0.
- Every README command runs verbatim with the documented exit codes, and the
  Rust example compiles warning-free and produces a verifiably protected file.

## 14. Stop conditions (report rather than improvise)

If making the test pass requires editing `scripts/smoke-release-binary.py`, or
requires a new required-CI check, or requires weakening the count assertion,
report instead. Any documented claim that cannot be reproduced by running the
code is reported as unverified rather than edited.

## 15. Closure evidence required

`./scripts/check.sh` green; the pre-fix and post-fix loop rates; the
per-sibling isolation table; the teeth check with the script restored to
byte-identical; the executed README transcript; `check-docs-contract.sh` green.

## 16. Handoff notes

Closure record: `plans/closure/maintenance-quality/005-status.md`.
