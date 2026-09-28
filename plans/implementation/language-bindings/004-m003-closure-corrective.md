# Language Bindings Milestone 004 — M003 Closure Corrective Pass

Status: ready for handoff

Repository baseline: `26da599f699ceff0a9881a8c7a24297bbecf468a`

Observed implementation branch: `feat/m003-python-corrective-qualification`

Observed PR: `#1` — `bindings/python: M003 corrective qualification + close M003 + lift M002`

Observed PR head at planning time:
`ccdd05fd35b28c48da907e8415fe51c0a5c3c4f4`

Source roadmap:
`plans/subsystems/language-bindings-roadmap.md#m004--m003-closure-corrective-pass`

Corrects:

- `plans/implementation/language-bindings/003-python-corrective-qualification.md`
- proposed `plans/closure/language-bindings/003-status.md` in PR #1
- PR #1's Python compatibility workflow and qualification bookkeeping

Long-term requirements:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/000-long-term-specification.md#5-release-invariants`
- `plans/001-terminology-and-domain-model.md#core-request-model`
- `plans/001-terminology-and-domain-model.md#verification-model`

Applicable ADRs:

- `plans/adrs/ADR-0001-canonical-protection-request.md`
- `plans/adrs/ADR-0003-byte-vs-pixel-paths.md`
- `plans/adrs/ADR-0005-foreign-language-bindings.md`

Primary class: invariant

## 1. Objective

Correct the remaining closure/evidence defects in PR #1 before M003 is accepted
and before Node.js work becomes dependency-ready.

The substantive M003 implementation and five-platform packaging qualification
already exist on the feature branch. This corrective pass is deliberately
narrow: make the lightweight Python compatibility CI actually pass, correct
the recorded qualification SHA to the commit that GitHub Actions truly
executed, reconcile SUPPORT/closure status with the successful qualification
evidence, and ensure M003 is only marked closed after all acceptance gates are
green.

No new Python API capability is part of this milestone.

## 2. Why this milestone is ready

PR #1 is open and mergeable, and its current evidence is concrete:

- standard Rust CI run `36338087653` is green;
- manually-dispatched `release-python` run `36337194059` is green;
- the successful release run produced five native wheel artifacts plus one
  sdist and all native smoke jobs succeeded;
- the lightweight `python-binding` PR run `36338087686` is red;
- that red run fails before tests because `maturin develop --release` is
  invoked without a virtualenv;
- the successful release run records actual head SHA
  `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5`;
- the PR description and draft M003 closure instead record the non-resolving
  SHA `d40f1b3c8dbe92e70ed1f2da9b6764be6e5a5a84`;
- `SUPPORT.md` still calls all five wheel rows merely "Configured" even
  though native qualification evidence now exists.

These are bounded correctness/evidence defects. No architecture decision is
open.

## 3. Current implementation evidence

At planning time, PR #1 contains the intended M003 production changes:

- structured `InsufficientCapacityError` and `ResourceLimitError`
  attributes;
- `ImageTruncated` mapped to `EncodeDecodeError`;
- pure-Python failure-safe file helpers using same-directory temporary files,
  `fsync`, and `os.replace`;
- additive `protect_file(..., output_path=None)`;
- Python tests expanded from 57 to 71;
- `.github/workflows/python-binding.yml`;
- corrected native runner matrix in `release-python.yml`;
- Linux Rust 1.89 provisioning inside manylinux containers;
- explicit `manylinux_2_28` repair/tagging;
- one sdist plus literal direct-`pip install` smoke.

Release run `36337194059` proves:

- Linux x86_64 wheel:
  `stegoeggo-0.4.2-cp311-abi3-manylinux_2_28_x86_64.whl`;
- Linux aarch64 wheel:
  `stegoeggo-0.4.2-cp311-abi3-manylinux_2_28_aarch64.whl`;
- matching native Linux smoke jobs succeeded;
- macOS x86_64, macOS arm64, and Windows x86_64 build/smoke jobs succeeded;
- direct sdist install smoke succeeded;
- no PyPI publication occurred.

The failing Python compatibility run `36338087686` shows:

```text
maturin develop --release
...
Couldn't find a virtualenv or conda environment
```

Therefore WP3 of M003 is not yet satisfied and M003 closure cannot be accepted
as written.

## 4. Invariants that must not regress

- Do not change the M003 Python API or Rust semantic integration unless a
  regression test proves a real defect.
- Keep `bindings/python` outside the root Rust workspace.
- Keep `./scripts/check.sh` Python-free.
- Keep the standard Rust `ci.yml` contract unchanged.
- Keep `python-binding.yml` a lightweight single-platform compatibility
  signal, not a release matrix.
- Keep `release-python.yml` manual-only and artifact-only.
- Do not rerun or rebuild five-platform qualification solely for bookkeeping
  changes when the exact qualified production/workflow commit is unchanged.
- If executable release workflow or binding production code changes, the
  five-platform qualification must be rerun against the new executable SHA.
- Evidence must reference the exact GitHub-recorded commit SHA, run ID, and
  artifact/run outcome.
- Do not claim a workflow is unschedulable when GitHub has already executed it.
- Do not mark M003 closed while any required acceptance workflow is red.
- Node.js implementation remains blocked until this corrective pass closes and
  M003 closure is accepted.

## 5. Scope

### In scope

- Fix `.github/workflows/python-binding.yml` so its PR/push job installs the
  binding correctly and runs the complete Python suite.
- Prefer a wheel-build + wheel-install CI path over `maturin develop` unless
  an explicitly created virtualenv is retained for a documented reason.
- Run the corrected `python-binding` workflow on PR #1 and require green
  evidence.
- Correct the successful release qualification SHA everywhere it is recorded.
- Correct stale M003 closure statements about the Python workflow not being
  runnable before merge.
- Update `SUPPORT.md` five-platform Python rows to qualified status backed by
  run `36337194059`.
- Reconcile PR #1 description if it repeats the bad SHA or stale status.
- Reconcile M003 closure/registry/roadmap status only after the corrected
  lightweight CI passes.
- Preserve the existing successful release qualification evidence when no
  executable release/binding behavior changes.

### Explicitly out of scope

- New Python public API.
- New tests unrelated to the lightweight CI defect.
- New wheel targets.
- PyPI publication.
- Node.js implementation.
- C ABI design/implementation.
- Dependency upgrades unrelated to the CI failure.
- Rerunning five-platform qualification merely because documentation/planning
  text changed.

## 6. Required production changes

### A. Make `python-binding.yml` self-contained

The current workflow uses `actions/setup-python` and then
`maturin develop --release`, but no virtualenv exists.

Preferred correction:

1. keep Python 3.11 and Rust 1.89;
2. install pinned/bounded maturin + pytest;
3. run `maturin build --release --out dist-ci` from
   `bindings/python`;
4. install the produced wheel with `python -m pip install
   dist-ci/stegoeggo-*.whl`;
5. run `python -m pytest -v`.

This avoids a CI-only editable-development environment and validates an
installable wheel while remaining much lighter than the five-platform release
workflow.

An explicitly created/activated venv plus `maturin develop` is acceptable
only if the implementor documents why it is preferable. Do not rely on an
implicit environment.

### B. Correct release evidence identity

Replace every occurrence of the incorrect successful-dispatch SHA

`d40f1b3c8dbe92e70ed1f2da9b6764be6e5a5a84`

with the GitHub-recorded run head SHA

`d40f1b37cf031b05e4f9c76af1cdfcf789d739b5`

where the text is intended to identify run `36337194059`.

At minimum audit:

- PR #1 body;
- `plans/closure/language-bindings/003-status.md`;
- any registry/roadmap or release-note reference added by PR #1.

Do not rewrite unrelated historical short-SHA references.

### C. Correct closure claims

The M003 closure must no longer state that `python-binding.yml` cannot run
until merge. GitHub executed it on PR #1 as run `36338087686`.

After the workflow fix, record the new successful PR/push run ID and outcome,
including the final Python test count.

M003 may be marked `closed` only after:

- standard Rust CI is green;
- corrected Python binding compatibility CI is green;
- existing five-platform release qualification remains valid;
- closure evidence is internally consistent.

### D. Promote support rows backed by evidence

The five Python wheel rows in `SUPPORT.md` should move from
"Configured (qualified when smoke evidence recorded)" to `Qualified` once
the M003 closure cites successful run `36337194059`.

Preserve the exact runner and compatibility details already recorded.

### E. Reconcile planning state

Until the corrected Python compatibility run is green:

- M003 is `closing`, not `closed`;
- M004 corrective is `ready`;
- Node is not dependency-ready.

After this corrective pass closes:

- M003 may be accepted as closed;
- M002's conditional platform qualification may be treated as satisfied by
  M003 evidence;
- this M004 corrective closes;
- Node becomes M005 and may be planned next.

## 7. Ordered work packages

### WP1 — Repair lightweight Python CI

Intent: satisfy the missing M003 acceptance gate.

Changes:

1. replace the implicit-`maturin develop` path with a self-contained install
   path;
2. keep Rust 1.89 / CPython 3.11;
3. build the extension;
4. install it;
5. execute the full Python suite.

Acceptance evidence:

- PR workflow runs on the changed workflow file;
- build/install succeeds;
- all Python tests execute and pass;
- run conclusion is `success`.

### WP2 — Evidence identity correction

Intent: make the release qualification audit trail exact.

Changes:

1. replace the invalid SHA with
   `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5`;
2. confirm run `36337194059` reports that same `head_sha`;
3. retain run ID, artifact names, and per-platform outcomes;
4. update PR body if necessary.

Acceptance evidence:

- every successful-dispatch reference resolves to the same real commit;
- no closure claim points at the invalid SHA.

### WP3 — Closure/support reconciliation

Intent: make status follow evidence.

Changes:

1. update M003 closure with the successful lightweight CI run;
2. remove the stale "cannot run until main" statement;
3. promote all five SUPPORT rows to `Qualified`;
4. verify no PyPI/publication claim was introduced;
5. ensure registry/roadmap only close M003 after WP1 evidence exists.

Acceptance evidence:

- SUPPORT, closure, registry, roadmap, and PR body agree;
- M003's acceptance matrix has no known red gate.

### WP4 — Final PR qualification

Intent: make PR #1 merge-ready.

Changes:

1. rerun/observe standard Rust CI on the final PR head;
2. observe corrected Python CI on the final PR head;
3. confirm PR remains mergeable;
4. confirm no executable release workflow or binding production code changed
   after the five-platform qualification commit except the lightweight CI
   workflow.

If executable binding/release code did change after
`d40f1b37cf031b05e4f9c76af1cdfcf789d739b5`, rerun
`release-python.yml` at the new executable SHA and replace qualification
evidence accordingly.

Acceptance evidence:

- required final PR checks are green;
- release evidence corresponds to the actual executable code being merged.

## 8. Failure, cancellation, restart, and contention semantics

- A red or cancelled `python-binding` run blocks M003/M004 closure.
- A rerun after a workflow-only correction must record the new run ID.
- A documentation-only commit does not invalidate prior wheel artifacts.
- A change to `bindings/python/src/**`,
  `bindings/python/python/**`, packaging manifests, or
  `release-python.yml` after the qualified SHA invalidates the assumption
  that run `36337194059` covers the final executable state; rerun release
  qualification.
- A force-push/rebase that changes commit identity but not file content still
  requires the closure to reference the actual final GitHub commit identities.
- Do not hide a red workflow by removing its trigger/path filter.
- Do not weaken test scope to obtain green status.

## 9. Compatibility and migration

No user-facing compatibility change is expected.

The corrective pass should only alter:

- CI mechanics;
- evidence/status text;
- support qualification wording;
- planning metadata.

The M003 Python public API and wheel target matrix remain unchanged.

## 10. Required tests

No new product tests are required unless the workflow fix exposes a real
binding defect.

Required executed tests/evidence:

1. full Python binding suite under corrected `python-binding.yml`;
2. standard Rust `./scripts/check.sh` through existing CI;
3. existing successful five-platform native wheel smokes remain referenced;
4. existing direct sdist install smoke remains referenced.

If the final branch changes binding production behavior, rerun the affected
product tests and full release qualification.

## 11. Required verification commands

Local sanity for the preferred lightweight-CI mechanics:

```bash
cd bindings/python
python3 -m pip install -U pip "maturin>=1.5,<2.0" pytest
maturin build --release --out dist-ci
python3 -m pip install --force-reinstall dist-ci/stegoeggo-*.whl
python3 -m pytest -v
```

Required root gate:

```bash
./scripts/check.sh
```

Required remote evidence:

- corrected `python-binding` workflow on PR #1: `success`;
- standard `CI` workflow on final PR head: `success`;
- existing release qualification run `36337194059` may be reused only if
  executable binding/release code remains covered by its qualified commit.

Verify release identity from GitHub:

- run ID: `36337194059`;
- actual head SHA:
  `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5`.

## 12. Documentation updates

Update:

- `plans/closure/language-bindings/003-status.md`;
- `SUPPORT.md`;
- PR #1 body if it still contains the incorrect SHA;
- `plans/registry.md`;
- `plans/subsystems/language-bindings-roadmap.md`.

Do not change canonical `plans/000-002`.

## 13. Acceptance criteria

This corrective pass is accepted only when:

- `python-binding.yml` succeeds on PR #1;
- the full Python suite runs rather than being skipped;
- the final Python test count is recorded;
- standard Rust CI is green on the final PR head;
- successful release run `36337194059` is referenced with the actual SHA
  `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5`;
- the stale "workflow cannot run before merge" claim is removed;
- all five SUPPORT wheel rows are marked qualified;
- no executable change after the qualified release SHA invalidates the
  release evidence, or a replacement release run is recorded;
- M003 closure accurately reflects all required green gates;
- M002 condition lift is evidence-backed;
- M004 corrective closes;
- Node is unblocked only afterward as M005.

## 14. Stop conditions

Stop and report rather than improvise if:

- the lightweight workflow cannot build/install the binding without changing
  package architecture;
- the 71-test suite fails for a real product reason rather than CI setup;
- PR #1's final executable state is no longer covered by run
  `36337194059`;
- a five-platform rerun exposes a platform regression;
- closure reconciliation would require rewriting immutable historical M001 or
  M002 closure facts rather than adding/correcting current factual evidence;
- fixing CI would require adding Python to `./scripts/check.sh`.

## 15. Closure evidence required

Create `plans/closure/language-bindings/004-status.md` and record:

- implementation/fix commit SHA;
- final PR #1 head SHA;
- corrected `python-binding` run ID and conclusion;
- final Python test count;
- final standard Rust CI run ID and conclusion;
- release qualification run `36337194059`;
- its actual head SHA
  `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5`;
- confirmation that all five wheel smoke jobs and sdist smoke remain successful;
- confirmation that SUPPORT rows are qualified;
- confirmation that the invalid SHA is no longer used as qualification
  identity;
- confirmation that no PyPI publication occurred;
- disposition of M003 and M002;
- registry/roadmap transition that makes M005 Node ready.

## 16. Handoff notes

Implement this corrective work on PR #1's feature branch so the failing PR
workflow can validate the fix before merge.

Do not create a separate production feature branch unless PR #1 cannot be
updated safely.

The smallest expected code change is the lightweight workflow install method.
Most remaining work is evidence and planning reconciliation.

Do not merge PR #1 with `python-binding` red. Do not begin Node implementation
until this corrective closure is accepted.
