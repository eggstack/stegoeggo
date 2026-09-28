# Language Bindings Milestone 004 — Closure Status

Status: closed

Source implementation plan: `plans/implementation/language-bindings/004-m003-closure-corrective.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#m004--m003-closure-corrective-pass`
Repository baseline reviewed: `25ca9dd` (main at the corrective-handoff
merge into PR #1's branch; the plan's own registration baseline was
`26da599f699ceff0a9881a8c7a24297bbecf468a`)
Implementation commits (on branch `feat/m003-python-corrective-qualification`,
PR #1):
`f9c55e4` — `ci: build and install a wheel in the python-binding check`
(the workflow fix; the sole executable-behaviour change of this pass)

Final PR #1 head SHA: the tip commit that carries this record. That SHA
does not exist until the record is committed, so it is recorded in the
PR #1 body (see §12) rather than inline here; every acceptance run cited
below names its exact head SHA.

## 1. Executive finding

The bounded corrective pass succeeded with the smallest possible code
change: the lightweight `python-binding` CI now builds a real wheel and
installs that wheel instead of invoking `maturin develop` without an
environment. The corrected run `36456938690` is green and executes the
full 71-test Python suite. The release qualification identity is exact —
run `36337194059`'s GitHub-recorded head SHA
`d40f1b37cf031b05e4f9c76af1cdfcf789d739b5` — replacing a SHA that does
not exist in this repository. All five SUPPORT wheel rows are
`Qualified`, the stale "workflow cannot run before merge" claim is
removed, and no binding production code, packaging manifest, or
`release-python.yml` behaviour changed after the qualified dispatch, so
the five-platform qualification evidence remains valid for the merged
state. M003 is accepted as closed, M002's conditional platform
qualification is satisfied by that evidence, this corrective pass closes,
and Node binding work (M005) is unblocked.

No new Python API capability was added.

## 2. Requirement-to-evidence matrix

| Plan requirement (M004) | Evidence |
| --- | --- |
| WP1 Repair lightweight Python CI (self-contained install path, Rust 1.89 / CPython 3.11, build the extension, install it, run the full suite) | `.github/workflows/python-binding.yml` (`f9c55e4`): `maturin build --release --out dist-ci`, then `python -m pip install --force-reinstall bindings/python/dist-ci/stegoeggo-*.whl`, then `python -m pytest -v`. Run `36456938690` on PR #1: `success`; job `python-binding-check` built `stegoeggo-0.4.2-cp311-abi3-manylinux_2_34_x86_64.whl` (release profile in 32.24s) and installed it before running tests. |
| WP1 build/install succeeds, all tests execute and pass, run conclusion `success` | Run `36456938690`: 71 collected, `70 passed, 1 skipped in 0.26s`, conclusion `success`. The single skip is the pre-existing M001 `@pytest.mark.skipif` guard on `test_rust_cli_round_trip` (Rust CLI not installed in the lightweight runner); no test scope was weakened (§4). |
| WP2 replace the invalid SHA with `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5` where it identifies run `36337194059` | `gh run view 36337194059 --json headSha` returns exactly that value, and `git cat-file -t d40f1b37cf031b05e4f9c76af1cdfcf789d739b5` resolves (the old value `d40f1b3c8dbe92e70ed1f2da9b6764be6e5a5a84` does not). Corrected in `plans/closure/language-bindings/003-status.md` §3 (table + prose), §13 (correction history), and the PR #1 body (§12). Full-repo grep finds the invalid SHA only in the implementation plan's historical observation record. |
| WP2 no closure claim points at the invalid SHA | Grep over `plans/closure/`, `plans/registry.md`, `plans/subsystems/`, `SUPPORT.md`, `RELEASING.md` finds no occurrence of the invalid SHA outside the plan's own observation section (see §3). |
| WP3 update M003 closure with the successful lightweight CI run | `003-status.md` §4 gains "Remote lightweight compatibility run (corrected during M004)" (run `36456938690`, 71 collected) and a local wheel-path rehearsal section (71/71). |
| WP3 remove the stale "cannot run until main" statement | `003-status.md` §10: the claim is struck and replaced with the factual record that GitHub executed the workflow twice on the PR (`36338087686` red, `36456938690` green). §11/§13 record the correction. |
| WP3 promote all five SUPPORT rows to `Qualified` | `SUPPORT.md`: all five rows read `Qualified`, backed by run `36337194059` (head `d40f1b3`); the placeholder "Configured (qualified when smoke evidence recorded)" wording is retired, and the intro no longer conditions qualification on future work. |
| WP3 no PyPI/publication claim introduced | `release-python.yml` still has no credentials, no `twine upload` / `pypi-publish` step; SUPPORT/closures still describe manual-only artifacts. Verified by grep. |
| WP4 required final PR checks are green | CI run `36456939020` (`Check`, `success`) and python-binding run `36456938690` (`success`), both observed on the workflow-fix head `f9c55e4`; both re-observed green on the final evidence tip `430599e` (python-binding `36459733771`, CI `36459734181`; see §4). PR #1 remains open and mergeable. |
| WP4 no executable change after the qualified release SHA invalidates the release evidence | `git diff --name-only d40f1b3..HEAD -- bindings/python/src bindings/python/python bindings/python/pyproject.toml bindings/python/Cargo.toml .github/workflows/release-python.yml src stegoeggo-stego` is empty (§3). The only behavioural change since `d40f1b3` is the lightweight `python-binding.yml` fix. No release rerun required. |
| Plan invariant: `./scripts/check.sh` stays Python-free | `check.sh` untouched; `grep -ri python scripts/check.sh` empty. |
| Plan invariant: standard Rust `ci.yml` contract unchanged | `.github/workflows/ci.yml` untouched by this branch (not in `git diff main...HEAD`). |
| Plan invariant: `release-python.yml` manual-only and artifact-only | Trigger is `workflow_dispatch` only; no publish step (see §3). |

## 3. Production implementation evidence

### Changed files (branch `feat/m003-python-corrective-qualification`, relative to main)

The substantive M003 production changes (structured exception
attributes, pure-Python file helpers, 71 Python tests, corrected
`release-python.yml`) were already on the branch; this pass adds:

- `.github/workflows/python-binding.yml` — header comment explains why
  the develop path cannot work; the `maturin develop --release` step is
  replaced by `maturin build --release --out dist-ci` plus
  `python -m pip install --force-reinstall
  bindings/python/dist-ci/stegoeggo-*.whl`. Triggers, toolchain pins
  (Rust 1.89, CPython 3.11), and the final `python -m pytest -v` step are
  unchanged.
- `SUPPORT.md` — five wheel rows `Configured ...` → `Qualified`; intro
  and matrix preamble rewritten to cite run `36337194059` and its
  GitHub-recorded head SHA; a paragraph documents the lightweight
  `python-binding.yml` suite.
- `plans/closure/language-bindings/003-status.md` — factual corrections
  only, listed in its §13 (SHA identity, workflow schedulability, SUPPORT
  status, M005 numbering) plus the corrected-CI evidence in §4.
- `plans/registry.md`, `plans/subsystems/language-bindings-roadmap.md` —
  M003 closed, M002's condition satisfied, M004 closed, M005 ready to
  plan (see §11/§12).
- This record.

No change to `bindings/python/src/**`, `bindings/python/python/**`,
packaging manifests, `release-python.yml`, `src/**`,
`stegoeggo-stego/**`, `Cargo.toml`/`Cargo.lock`, `ci.yml`, or
`scripts/check.sh`. Verified:

```bash
$ git diff --name-only d40f1b3..HEAD -- bindings/python/src bindings/python/python \
    bindings/python/pyproject.toml bindings/python/Cargo.toml \
    .github/workflows/release-python.yml src stegoeggo-stego
(empty)
```

### Remote evidence identities

| Run | Workflow | Head SHA | Event | Conclusion |
| --- | --- | --- | --- | --- |
| `36337194059` | release-python | `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5` | workflow_dispatch | success (all 12 jobs) |
| `36338087686` | python-binding | `ccdd05fd35b28c48da907e8415fe51c0a5c3c4f4` | pull_request | failure (before tests: no virtualenv for `maturin develop`) |
| `36456938690` | python-binding | `f9c55e44d95183d9a360b7a3fc1307bef366f852` | pull_request | success (71 collected; 70 passed, 1 skipped) |
| `36456939020` | CI | `f9c55e44d95183d9a360b7a3fc1307bef366f852` | pull_request | success (Check) |

The twelve jobs of `36337194059` (five wheel builds, one sdist build,
five native smokes, one sdist direct-pip-install smoke) are all
`success`; wheel filenames and smoke log lines are recorded in
`003-status.md` §3 and are unchanged by this pass.

The invalid identity `d40f1b3c8dbe92e70ed1f2da9b6764be6e5a5a84` does not
resolve in this repository and is documented in the implementation plan
only as the observed planning-time defect.

## 4. Verification executed (exact commands + results)

### Local wheel-path rehearsal for the corrected CI mechanics

```bash
$ python3 -m venv /tmp/m004-venv
$ /tmp/m004-venv/bin/python -m pip install -U pip "maturin>=1.5,<2.0" pytest
$ cd bindings/python && /tmp/m004-venv/bin/python -m maturin build --release --out dist-ci
    Finished `release` profile [optimized] target(s) in 22.03s
    📦 Built wheel for abi3 Python ≥ 3.11 to dist-ci/stegoeggo-0.4.2-cp311-abi3-manylinux_2_34_x86_64.whl
$ /tmp/m004-venv/bin/python -m pip install --force-reinstall bindings/python/dist-ci/stegoeggo-*.whl
Successfully installed stegoeggo-0.4.2
$ cd bindings/python && /tmp/m004-venv/bin/python -m pytest -v
=============================== 71 passed in 0.25s ==============================
```

`dist-ci/` was removed after the rehearsal; wheels are already ignored by
`bindings/python/.gitignore` (`*.whl`).

### Required root gate

```bash
$ ./scripts/check.sh
```

Passed locally on the feature branch: `cargo fmt --check`, clippy with
`-D warnings`, `cargo check -p stegoeggo --no-default-features`, the full
workspace test suite (44 `test result: ok` summaries, zero failures,
ignored-only suites unchanged), and `check-docs-contract.sh`
(`Documentation contracts valid: 5 release targets`). Log retained at
`/tmp/opencode/m004-checksh.log` for the session.

### Remote evidence

- Corrected `python-binding` on PR #1: run `36456938690`, conclusion
  `success` on head `f9c55e44d95183d9a360b7a3fc1307bef366f852`.
  The job built the wheel (`Finished release profile [optimized] in
  32.24s` → `manylinux_2_34_x86_64`), installed it, and ran the suite:
  `70 passed, 1 skipped in 0.26s` out of 71 collected. The skip is the
  M001-era guard
  (`@pytest.mark.skipif(shutil.which("stegoeggo") is None, ...)` on
  `test_rust_cli_round_trip`); the lightweight runner has no Rust CLI on
  PATH. The test passes locally (71/71) and CLI behaviour is covered by
  the required Rust gate. No scope was removed to obtain green.
- Standard Rust CI on the same head: run `36456939020`, conclusion
  `success` (job `Check`).
- Existing five-platform qualification: run `36337194059`, conclusion
  `success`, reused because no executable binding/release change after
  the qualified commit `d40f1b3` (§3).
- Existing direct sdist install smoke: same run, `success`.

### Remote evidence on the final evidence tip

Head `430599e` (`430599ea0624f5238a2f40a4507f1621c56d9a1e`) carries the
complete evidence state: workflow fix, SUPPORT qualification, corrected
`003-status.md`, this record, and the final registry/roadmap states. Both
required workflows ran green on it:

- python-binding run `36459733771`: `success` — wheel
  `stegoeggo-0.4.2-cp311-abi3-manylinux_2_34_x86_64.whl` built and
  installed, `70 passed, 1 skipped in 0.19s` (the same pre-existing M001
  CLI-guard skip).
- CI run `36459734181`: `success` (job `Check`).

The commit carrying this paragraph changes only this planning record
relative to `430599e`; per this plan's documentation-only-commit
semantics it does not invalidate the evidence above. The final tip SHA
and its own check runs are recorded in the PR #1 body (§12).

## 5. Invariant review

1. M003 Python API and Rust semantic integration unchanged. — The only
   production-touching file edited is `python-binding.yml`; the binding
   source, request surface, and error projections are byte-identical to
   the qualified state (§3 diff).
2. `bindings/python` stays outside the root Rust workspace. — Untouched.
3. `./scripts/check.sh` stays Python-free. — Untouched; the python-binding
   job is a separate workflow.
4. Standard Rust `ci.yml` contract unchanged. — `.github/workflows/ci.yml`
   is not in this branch's diff against main.
5. `python-binding.yml` stays a lightweight single-platform compatibility
   signal. — One Linux x86_64 job; no matrix, no artifacts, no publish.
6. `release-python.yml` stays manual-only and artifact-only. — Trigger and
   step inventory unchanged; rerun was not required and was not run.
7. No rerun of five-platform qualification for bookkeeping changes. —
   None run; reuse justified by the empty executable diff (§3).
8. Exact SHA/run/artifact identity in evidence. — §3 table; the invalid
   SHA is retired everywhere it identified run `36337194059`.
9. No "unschedulable" claim. — The stale claim is removed from
   `003-status.md`; both PR executions are recorded.
10. No green claim while a gate is red. — Registry/roadmap final states
    were written only after runs `36456938690` and `36456939020` were both
    `success`.
11. Node work stayed out of this pass. — No `bindings/node`, no C ABI, no
    dependency changes.

## 6. Failure and recovery review

- **Red `python-binding` run `36338087686`** (head `ccdd05f`):
  `maturin develop --release` exited with `Couldn't find a virtualenv or
  conda environment` before any test ran. Root cause is CI setup, not the
  binding: `actions/setup-python` provides a system-style interpreter,
  not a virtualenv, and the workflow created none. Recovery: replace the
  step with a wheel build + install, which needs no virtualenv and
  additionally validates the artifact users install.
- **Invalid qualification SHA in PR body and draft closure**: the recorded
  `d40f1b3c8dbe92e70ed1f2da9b6764be6e5a5a84` matched no commit; GitHub
  records `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5` as the run's
  `head_sha`. Recovery: corrected at every identifying site; the history
  of the correction is in `003-status.md` §13.
- **Merge conflict** (`main` into the feature branch, `plans/registry.md`
  and roadmap §12): the branch had recorded M003 as closed while main
  (M004 registration) recorded it as closing. Resolved to the true state
  (M003 closing, M004 ready) until the green gates existed, then moved to
  the final state in the same push as this record.
- **One CI skip, not a failure**: run `36456938690` reports `1 skipped`.
  Investigated: it is `test_rust_cli_round_trip` behind the pre-existing
  M001 `skipif` (no `stegoeggo` on the runner PATH), present since commit
  `1810366`. Not a product defect, not a scope change.

## 7. Migration and compatibility review

No user-facing compatibility change. The pass alters CI mechanics,
evidence/status text, and support qualification wording. The M003 Python
public API and wheel target matrix are unchanged. `bindings/python/README.md`
local-install docs (explicit `python3 -m venv` + `maturin develop`) remain
accurate for developers and were not part of this pass.

## 8. Security review

- No secret material is involved: the diff touches CI steps, SUPPORT
  wording, and planning records. The structured-exception no-leak
  guarantees from M003 are untouched and their tests still pass (local
  71/71 rehearsal plus green CI run).
- `python-binding.yml` builds and installs a local wheel in the CI
  workspace and publishes nothing; `release-python.yml` still carries no
  PyPI credentials and was not modified.

## 9. Documentation and operations

- `SUPPORT.md` — five wheel rows `Qualified` with the exact qualifying
  run and SHA; the lightweight suite is documented.
- `RELEASING.md` — unchanged and still accurate (no mechanics changed).
- `bindings/python/README.md` — unchanged; no claim in it contradicts the
  wheel-build CI path.
- `plans/closure/language-bindings/003-status.md` — corrected (see its
  §13); `001-status.md` and `002-status.md` untouched.
- PR #1 body — reconciled (see §12).

## 10. Unresolved findings

None at the level required by the plan acceptance criteria.

Carried forward:

- `test_rust_cli_round_trip` does not execute in the lightweight CI
  runner (documented skip; M001 guard). If CI coverage of the
  Rust↔Python round trip is wanted, installing the CLI in that job is a
  separate, explicitly scoped decision — not a corrective-pass
  requirement.
- `VerificationBudgetExceeded` end-to-end Python coverage remains
  deferred per M003 WP1 (documented in `003-status.md` §10); unchanged by
  this pass.
- Free-threaded CPython and PyPy qualification remain deferred per the
  subsystem roadmap.

## 11. Roadmap disposition

- M004 is **closed** by this record.
- M003 is **closed**, accepted with the corrected lightweight-CI evidence
  (`36456938690`) and the corrected qualification identity
  (`d40f1b37cf031b05e4f9c76af1cdfcf789d739b5` / `36337194059`). Reference:
  `plans/closure/language-bindings/003-status.md` (+ §13).
- M002's qualification condition is **satisfied**: the outstanding
  four-platform native-smoke evidence is M003's, and this record accepts
  that evidence. M002 moves from "conditionally closed" to "closed". The
  `002-status.md` record itself is unchanged; the audit trail for the
  lift is this section plus `003-status.md` §11.
- M005 (Node binding) is **unblocked and ready to plan** against the
  final accepted Python contract. No Node plan is written by this pass;
  the next subsystem-local implementation number is free for it.
- M006/M007 (C ABI) remain proposed with unchanged dependencies.
- `release-distribution` M001 remains blocked (no stable B newer than
  0.4.2); unrelated to this pass.

## 12. Registry updates

- `plans/registry.md`:
  - language-bindings subsystem row: `M001–M004 closed; M005 Node ready
    to plan`; blockers `none`.
  - "Dependency-ready implementation plans": empty — M004 is closed and
    no successor plan is written.
  - "Active closure work": empty.
  - "Blocked work": the language-bindings M005 row is removed (its
    blocker, accepted M003/M004 closure, is satisfied); the
    release-distribution row is unchanged.
  - "Recently closed work": adds M004, M003, and M002 entries citing this
    record; the interim "closing/ready" rows are retired.
- `plans/subsystems/language-bindings-roadmap.md`:
  - §6 dependency prose: M003 accepted and M004 closed on PR #1; M005
    must consume the final Python contract.
  - M004 section: closure accepted.
  - M005 section: ready to plan (both M003 and M004 closed).
  - §12 milestone table: M002 `closed`, M003 `closed`, M004 `closed`,
    M005 `proposed (dependency-ready)`, M006/M007 unchanged.
- PR #1 body reconciliation (no commit; applied with `gh pr edit` after
  the final push):
  - "SHA dispatched" corrected to
    `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5` (GitHub-recorded
    `head_sha` of run `36337194059`);
  - lightweight CI status corrected: run `36338087686` (red before
    tests) superseded by run `36456938690` (success, 71 collected);
  - final PR #1 head SHA recorded (the tip carrying this record).
