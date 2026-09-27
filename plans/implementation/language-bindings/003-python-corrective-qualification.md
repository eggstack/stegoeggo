# Language Bindings Milestone 003 — Python Corrective Qualification

Status: ready for handoff

Repository baseline: `90c58d4f0df037319cc554a84c00245338491879`

Source roadmap: `plans/subsystems/language-bindings-roadmap.md#m003--python-corrective-qualification`

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

Corrects:

- `plans/implementation/language-bindings/001-python-binding-foundation.md`
- `plans/closure/language-bindings/001-status.md`
- `plans/implementation/language-bindings/002-python-packaging-qualification.md`
- `plans/closure/language-bindings/002-status.md`

Primary class: capability

## 1. Objective

Close the concrete correctness and qualification gaps found after Python M001
and M002 without changing the canonical Rust protection/verification semantics
or weakening the manual-release policy.

This milestone makes the existing Python binding safe to treat as a qualified
foreign-language frontend by:

1. preserving structured Rust error data as Python exception attributes;
2. making file helpers failure-safe and non-destructive while retaining the
   current two-argument in-place call as a compatibility form;
3. adding a small binding-specific push/PR compatibility check without adding
   Python tooling to `./scripts/check.sh`;
4. correcting the five-platform wheel workflow so every wheel is built and
   smoke-tested on a matching native architecture and Linux builds provision
   Rust inside cibuildwheel's build container;
5. proving a literal clean `pip install <sdist>` path, rather than only
   rebuilding a wheel manually from an extracted sdist;
6. reconciling binding documentation and planning status with actual evidence;
7. running the corrected five-platform manual qualification workflow and
   recording the results before Node.js work begins.

No PyPI publication is part of this milestone.

## 2. Why this milestone is ready

M001 is closed and already provides a working PyO3/maturin binding over the
canonical byte API. M002 is conditionally closed and has a concrete packaging
implementation plus local macOS x86_64 evidence. The remaining issues are
bounded corrective findings against code that already exists; no unresolved
architecture decision blocks them.

The repository already uses the native GitHub-hosted
`ubuntu-24.04-arm` runner in its assurance workflow, so Linux aarch64
qualification does not require QEMU or a new runner strategy.

Current cibuildwheel documentation confirms that Linux wheel builds execute
inside isolated build containers and that `before-all` is the mechanism for
installing non-Python build prerequisites inside those containers. Current
GitHub Actions documentation lists `ubuntu-24.04-arm` as an arm64
GitHub-hosted runner. Maturin's source-distribution contract is PEP 517 based,
so the closure test can and should exercise `pip install` directly on the
built tarball.

Research references:

- https://cibuildwheel.pypa.io/en/stable/options/#before-all
- https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job
- https://www.maturin.rs/distribution.html

## 3. Current implementation evidence

At baseline `90c58d4f0df037319cc554a84c00245338491879`:

- `bindings/python/` is isolated from the root Rust workspace and uses PyO3
  `abi3-py311`, maturin, Rust 1.89, and `panic = "unwind"`.
- M001 records 57 passing Python tests plus a green `./scripts/check.sh`.
- `map_error` maps structured Rust variants to Python exception classes but
  discards their fields by constructing exceptions from only
  `err.to_string()`.
- `Error::InsufficientCapacity` exposes `required` and `available`.
- Resource-limit variants expose structured fields:
  - `InputTooLarge { size, limit }`;
  - `DimensionsExceeded { width, height, max_width, max_height }`;
  - `ContainerLimitExceeded { kind, count, limit }`;
  - `MetadataLimitExceeded { kind, size, limit }`;
  - `VerificationBudgetExceeded { kind, count, limit }`.
- `protect_file(path, request)` reads and then directly rewrites `path`
  through `std::fs::write`; a write failure can leave the source truncated
  or partially replaced.
- Public file helpers are implemented in the private native extension rather
  than as the pure-Python convenience layer envisioned by M001.
- `.github/workflows/release-python.yml` currently:
  - uses x86_64 `ubuntu-22.04` for both Linux x86_64 and aarch64 rows;
  - tries to smoke an aarch64 wheel on an x86_64 runner;
  - installs Rust on the host and then invokes `rustup` from inside
    cibuildwheel's isolated Linux build container without provisioning it
    there;
  - builds the same sdist once per wheel-matrix row.
- `ci.yml` checks the Rust workspace only; a Rust API change can break the
  excluded Python crate without a normal push/PR signal.
- M002's recorded sdist evidence manually rebuilds a wheel from an extracted
  sdist, but does not prove `pip install stegoeggo-<version>.tar.gz` in an
  isolated environment.
- `SUPPORT.md` says all five Python wheel targets are "Produced in CI" while
  four targets still lack native smoke evidence.
- `bindings/python/README.md` uses an all-caps `RightsPolicy` example name
  even though the exported PyO3 attribute is `ProhibitedAiMlTraining`.
- `plans/registry.md` contains duplicate "Recently closed work" headings.

These findings do not require changes to root protection semantics, carrier
semantics, the CLI, or the Rust release profile.

## 4. Invariants that must not regress

- Python protection remains a thin projection of
  `ProtectionRequest + process_request_bytes*`.
- Python verification remains a projection of
  `verify_image_bytes_report` / its resource-limited equivalent.
- Encoded bytes remain the metadata-preserving boundary.
- Root `stegoeggo` and `stegoeggo-stego` remain free of PyO3/maturin
  dependencies and retain their existing unsafe-code policy.
- `bindings/python` remains outside the root Rust workspace.
- Root/CLI release builds retain `panic = "abort"`; the Python extension
  retains `panic = "unwind"`.
- `./scripts/check.sh` gains no Python, maturin, or Node prerequisite.
- The existing Python exception class hierarchy remains compatible; structured
  attributes are additive.
- Existing `protect_file(path, request)` callers remain valid.
- MAC/signing key material must never appear in exception attributes, repr,
  report JSON, logs, or file-helper diagnostics.
- No workflow in this milestone publishes to PyPI.
- The crates.io carrier -> library -> CLI publication order is unchanged.
- Python output remains interoperable with Rust and Rust output remains
  verifiable from Python.
- M004 Node work does not begin until this corrective milestone closes.

## 5. Scope

### In scope

- `bindings/python/src/lib.rs` error projection changes needed to preserve
  structured exception data.
- `bindings/python/python/stegoeggo/` pure-Python file helper implementation
  and typing changes.
- Python tests for exception attributes and file-helper failure semantics.
- A lightweight, binding-specific push/PR GitHub Actions workflow.
- Corrective changes to `.github/workflows/release-python.yml`.
- Native Linux aarch64 build/smoke on `ubuntu-24.04-arm`.
- Native wheel smoke for every row of the documented five-target matrix.
- A single, isolated sdist build plus direct clean-environment
  `pip install <sdist>` verification.
- Documentation corrections in the binding README, `SUPPORT.md`, and
  `RELEASING.md` if command/runbook text changes.
- Planning/registry reconciliation and M002 condition-lift bookkeeping.
- Closure evidence for the corrected manual workflow.

### Explicitly out of scope

- PyPI publication or credentials.
- Reserving the PyPI package name as an automated action.
- Free-threaded CPython or PyPy support.
- New Python application features, async APIs, generic carrier bindings, or
  broader format support.
- Node.js implementation.
- C ABI design or implementation.
- Moving binding crates into the root Rust workspace.
- Adding Python to `./scripts/check.sh`.
- Changing canonical Rust error variants solely for FFI convenience.
- Broad dependency upgrades unrelated to making the current qualification path
  correct.

## 6. Required production changes

### A. Preserve structured error data

Keep the existing public exception classes but construct instances with
machine-readable attributes derived from the matching Rust variant.

Minimum required attributes:

| Python exception | Rust source | Required attributes |
|---|---|---|
| `InsufficientCapacityError` | `InsufficientCapacity` | `required`, `available` |
| `ResourceLimitError` | `InputTooLarge` | `resource="input_bytes"`, `size`, `limit` |
| `ResourceLimitError` | `DimensionsExceeded` | `resource="dimensions"`, `width`, `height`, `max_width`, `max_height` |
| `ResourceLimitError` | `ContainerLimitExceeded` | `resource="container"`, `kind`, `count`, `limit` |
| `ResourceLimitError` | `MetadataLimitExceeded` | `resource="metadata"`, `kind`, `size`, `limit` |
| `ResourceLimitError` | `VerificationBudgetExceeded` | `resource="verification_budget"`, `kind`, `count`, `limit` |

The human-readable exception message remains the Rust error display string.
Unknown future non-exhaustive variants must still map safely to
`StegoEggoError` rather than panicking or fabricating fields.

Audit the existing mapping while touching it. `ImageTruncated` must map to a
documented existing Python category rather than silently falling through to the
generic base class. Do not create a broad new hierarchy unless a concrete Rust
variant requires it.

Update `_native.pyi` so typed callers can discover the guaranteed structured
attributes.

### B. Make file helpers safe frontend conveniences

Move the public file-helper behavior into the pure-Python package layer so the
native extension remains focused on bytes, requests, reports, and exceptions.

Retain this compatibility form:

`protect_file(path, request)`

It remains an in-place operation, but implementation must be failure-safe:
process the bytes first, write the completed output to a temporary sibling in
the destination directory, flush/close it, and replace the destination with
`os.replace` only after the complete output exists. A processing or write
failure must leave the original file untouched. Temporary files must be
cleaned up on failure.

Add an optional explicit destination without breaking the current form:

`protect_file(path, request, output_path=None)`

When `output_path` is supplied, atomically replace that destination from a
same-directory temporary file and never mutate the input path. Keep the return
type `None` for compatibility.

Move/retain `verify_file` as a thin Python read -> `verify(bytes, ...)`
helper.

The private native module may remove its file helper exports once the
pure-Python layer and stubs no longer depend on them.

### C. Add lightweight Python compatibility CI

Add a separate binding workflow, preferably
`.github/workflows/python-binding.yml`, that runs on push/PR when files that
can affect the binding change. Include at least:

- `bindings/python/**`;
- root `src/**`;
- `stegoeggo-stego/**`;
- root `Cargo.toml` / `Cargo.lock`;
- fixtures used by Python parity tests;
- the workflow itself.

Use one Linux x86_64 job, CPython 3.11, and Rust 1.89. It should build/install
the extension and run the Python test suite. Do not add a multi-platform matrix
here; release qualification remains the job of `release-python.yml`.

This workflow is a binding compatibility signal. It must not add Python
commands to `./scripts/check.sh` or silently change the existing Rust
`Check` job contract.

### D. Correct the manual wheel workflow

Keep `release-python.yml` manual-only and artifact-only.

Required runner matrix:

| Target | Native runner |
|---|---|
| Linux x86_64 | `ubuntu-24.04` (or documented equivalent x86_64 runner) |
| Linux aarch64 | `ubuntu-24.04-arm` |
| macOS x86_64 | `macos-15-intel` or another explicitly Intel GitHub-hosted runner |
| macOS arm64 | `macos-14` / current arm64 equivalent |
| Windows x86_64 | `windows-2022` / current x86_64 equivalent |

Do not emulate Linux aarch64 on x86_64 for this milestone when a native runner
is already available.

For Linux cibuildwheel jobs, install Rust 1.89 inside the cibuildwheel build
container via Linux `before-all` (or an equivalently documented container
hook), and make `$HOME/.cargo/bin` available to the build environment.
Installing Rust only on the outer GitHub runner is not sufficient evidence.

Pin/declare the Linux manylinux compatibility image explicitly so the wheel
tag and documented glibc floor are deterministic. The initial target remains
manylinux_2_28 unless actual dependency/tool evidence requires a different
floor; such a change is a stop condition requiring documentation review.

Every wheel must be installed and smoke-tested on a runner of the same native
architecture before it is called qualified. The smoke remains import +
protect + verify and should additionally assert the reported source version.

Do not build the sdist once per matrix row.

### E. Build and verify the sdist once

Create one sdist artifact in a dedicated job/environment with Rust 1.89 and
CPython 3.11.

Then, outside the source checkout in a new virtual environment, execute a
literal PEP 517 install of the built tarball, for example:

`python -m pip install --no-cache-dir /path/to/stegoeggo-0.4.2.tar.gz`

The test must not manually extract the archive or call `maturin build`
against the extracted tree. After installation, run import + source-version +
protect + verify smoke.

Upload only one sdist artifact per workflow run.

### F. Reconcile documentation and status claims

- Fix Python README examples to use the actual exported PyO3 names.
- In `SUPPORT.md`, distinguish "configured/pending qualification" from
  "qualified" until native smoke evidence exists.
- After the corrected workflow succeeds, change only the rows backed by
  recorded evidence to qualified/produced wording.
- Keep the package labelled experimental/not-on-PyPI until an explicit later
  publication decision.
- Update `RELEASING.md` if runner, sdist, or qualification mechanics change.
- Consolidate the duplicate "Recently closed work" heading in
  `plans/registry.md`.
- Do not rewrite the historical M002 closure analysis. At M003 closure, record
  that M002's outstanding qualification condition was satisfied by M003
  evidence, and reconcile roadmap/registry status accordingly.

## 7. Ordered work packages

### WP1 — Structured Python errors

Intent: make the Python error contract match the structured Rust contract.

Changes:

1. Add a small error-construction helper that can set typed attributes on the
   existing exception instances.
2. Project all fields listed in section 6A.
3. Correct `ImageTruncated` categorization.
4. Update `_native.pyi`.
5. Add focused tests that assert both exception type and exact attributes.

Acceptance evidence:

- capacity tests observe `required` and `available`;
- every resource-limit variant covered by a reachable fixture/test exposes its
  structured fields;
- generic fallback remains safe for non-exhaustive errors;
- key material is absent from exception repr/attributes.

### WP2 — Failure-safe file helpers

Intent: eliminate destructive direct writes from the embedded Rust extension.

Changes:

1. Implement public helpers in the Python layer.
2. Preserve two-argument in-place compatibility.
3. Add optional `output_path`.
4. Use same-directory temporary file + `os.replace`.
5. Clean temporary files on all tested failure paths.
6. Remove private native file helper exposure if no longer needed.

Acceptance evidence:

- successful in-place protection remains usable;
- explicit-output protection leaves input unchanged;
- injected/forced write failure leaves the original unchanged;
- processing failure creates no partial output;
- verify helper remains equivalent to `verify(path.read_bytes())`.

### WP3 — Binding compatibility CI

Intent: catch root API drift before merge without inflating the standard Rust
gate.

Changes:

1. Add path-filtered one-platform Python workflow.
2. Pin Rust 1.89 and Python 3.11.
3. install maturin/pytest;
4. build/install binding;
5. run complete Python tests.

Acceptance evidence:

- workflow runs for a binding/root API change;
- green run records the full Python test count;
- `ci.yml` and `./scripts/check.sh` remain Rust-only.

### WP4 — Wheel workflow correction

Intent: make every platform row executable and architecture-valid.

Changes:

1. split Linux x86_64 and Linux arm64 onto matching native runners;
2. use an explicitly Intel macOS runner for x86_64;
3. provision Rust inside Linux build containers;
4. make manylinux floor explicit;
5. retain native macOS/Windows Rust setup;
6. pair each wheel with a same-architecture smoke job;
7. retain manual dispatch and artifact-only behavior.

Acceptance evidence:

- workflow syntax is accepted by GitHub Actions;
- each wheel filename/tag matches its target;
- no smoke job installs a foreign-architecture wheel;
- Linux build logs prove `rustc 1.89.x` is available inside the wheel build
  container.

### WP5 — Sdist truth test

Intent: prove the artifact works through the user-facing PEP 517 path.

Changes:

1. build one sdist;
2. create a clean venv outside the checkout;
3. direct-`pip install` the tarball;
4. run import/version/protect/verify smoke;
5. upload the sdist once.

Acceptance evidence:

- command transcript shows direct tarball installation;
- no parent checkout is used as a path dependency;
- installed package works after source tree is no longer on `sys.path`.

### WP6 — Documentation and planning reconciliation

Intent: remove claims that exceed evidence.

Changes:

1. fix binding README enum examples;
2. correct SUPPORT qualification wording before/after workflow evidence;
3. update RELEASING mechanics as necessary;
4. remove duplicate registry heading;
5. keep M004 Node blocked until M003 closure.

Acceptance evidence:

- docs match actual public names and artifact evidence;
- no PyPI install claim exists;
- registry and subsystem roadmap agree on the active milestone.

### WP7 — Five-platform qualification and closure

Intent: turn the corrected workflow into evidence rather than leaving it
untested infrastructure.

Changes:

1. dispatch `release-python.yml` at the exact implementation SHA;
2. inspect all wheel build and smoke jobs;
3. inspect the single sdist job and direct-install smoke;
4. record run URL/ID, artifact names, wheel tags, and per-platform results;
5. rerun only after correcting failures; do not record a failed attempt as
   qualification;
6. write `plans/closure/language-bindings/003-status.md`;
7. mark M002's outstanding platform condition satisfied only if all required
   rows pass;
8. make M004 Node ready only after closure is accepted.

Acceptance evidence:

- five wheel targets have native build + native install/protect/verify smoke;
- direct sdist install smoke passes;
- local/core checks pass;
- closure record captures exact evidence.

## 8. Failure, cancellation, restart, and contention semantics

- Wheel matrix uses fail-fast false so one platform failure does not hide
  evidence from other rows; any required row failure still prevents closure.
- A failed/cancelled workflow run is not qualification evidence.
- Reruns must build the same implementation SHA; if source changes, dispatch a
  new run and record the new SHA.
- Manual release workflow concurrency may cancel an older run for the same
  source ref, but cancellation never counts as success.
- File helpers must not replace the destination until all Rust processing and
  complete temporary-file writing have succeeded.
- On Python helper failure, best-effort cleanup removes the temporary sibling;
  cleanup failure must not hide the original exception.
- Existing destination files are replaced only at the final atomic replace
  boundary supported by the host filesystem.
- Structured-error projection failure must fall back to a safe
  `StegoEggoError`, never panic the Python host.
- No implementation step may upload to PyPI as a recovery mechanism.

## 9. Compatibility and migration

The binding is still an experimental 0.x surface and unpublished, but this
milestone should avoid gratuitous breakage.

- Existing exception subclasses keep their names and inheritance.
- Added exception attributes are backward-compatible.
- Existing `protect_file(path, request)` remains valid.
- Optional `output_path` is additive.
- `verify_file` remains available.
- Byte-oriented public operations and result/report objects are unchanged.
- Python package/source version remains aligned with the root StegoEggo
  version.
- Rust, CLI, and carrier APIs do not change.
- Wheel platform support does not expand beyond the already documented five
  targets.

## 10. Required tests

At minimum add/retain tests for:

1. `InsufficientCapacityError.required` and `.available`.
2. Input byte-limit fields `resource/size/limit`.
3. Dimension-limit fields.
4. Container-limit fields where a deterministic fixture can reach the variant.
5. Metadata-limit fields where a deterministic fixture can reach the variant.
6. Verification-budget fields where a deterministic fixture can reach the
   variant.
7. `ImageTruncated` mapping.
8. Unknown/fallback error mapping does not panic.
9. Secret key text absent from all structured exception projections.
10. `protect_file(path, request)` successful in-place compatibility.
11. explicit `output_path` leaves input unchanged.
12. invalid input leaves source/destination unchanged.
13. forced destination write/replace failure leaves existing destination
    unchanged and cleans temporary files.
14. `verify_file` equivalence to byte verification.
15. existing deterministic Python/Rust parity tests.
16. existing PNG/JPEG/WebP protection/verification tests.
17. lightweight Python CI executes the full binding suite.
18. direct sdist `pip install` smoke.
19. five native wheel install/protect/verify smokes.

Do not manufacture artificial core error variants solely to make every field
test reachable. If a structured variant cannot be reached through the public
binding API, unit-test the projection helper in Rust and document why an
end-to-end Python trigger is unavailable.

## 11. Required verification commands

Required local/core gate:

```bash
./scripts/check.sh
```

Binding crate compile:

```bash
cargo check --manifest-path bindings/python/Cargo.toml --locked
```

Local Python binding suite:

```bash
cd bindings/python
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -U pip
python -m pip install "maturin>=1.5,<2.0" pytest
maturin develop --release
python -m pytest -v
```

Local sdist construction:

```bash
cd bindings/python
maturin sdist -o ../../dist-python
```

Direct isolated sdist installation must be run from outside the repository
checkout. Substitute the produced versioned tarball path exactly:

```bash
python3 -m venv /tmp/stegoeggo-sdist-venv
/tmp/stegoeggo-sdist-venv/bin/python -m pip install -U pip
/tmp/stegoeggo-sdist-venv/bin/python -m pip install --no-cache-dir \
  /absolute/path/to/dist-python/stegoeggo-0.4.2.tar.gz
/tmp/stegoeggo-sdist-venv/bin/python -c \
  "import stegoeggo; print(stegoeggo.__version__)"
```

Then run an import + protect + verify smoke using only the installed package
and test-owned bytes.

Required remote qualification:

- manually dispatch `release-python.yml` with `source_ref` set to the exact
  implementation commit SHA;
- require success for Linux x86_64, Linux aarch64, macOS x86_64, macOS arm64,
  Windows x86_64 build and smoke jobs;
- require success for the single sdist direct-install smoke job;
- record the workflow run URL/ID and artifact names in closure.

If implementation changes after the workflow run, repeat the dispatch against
the new SHA before closure.

## 12. Documentation updates

Update as evidence requires:

- `bindings/python/README.md`
  - correct enum names;
  - document optional output-path file helper;
  - describe failure-safe in-place behavior;
  - keep experimental/not-on-PyPI statement.
- `SUPPORT.md`
  - distinguish configured from qualified wheel rows until the successful run;
  - after qualification, record the actual five-platform evidence;
  - retain CPython 3.11-3.14 / `cp311-abi3` scope unless separately changed.
- `RELEASING.md`
  - record native runner expectations;
  - record one-sdist/direct-install requirement;
  - keep manual publication gate.
- `plans/registry.md` and
  `plans/subsystems/language-bindings-roadmap.md`
  - track M003 through closure and unblock M004 only afterward.

Do not edit canonical `plans/000-002`; the high-level Python -> Node -> C
sequence is unchanged.

## 13. Acceptance criteria

This milestone is accepted only when all of the following are observable:

- structured Python exceptions expose the documented Rust counts/limits;
- no key material is exposed;
- file-helper failures cannot truncate the original protected input path;
- the existing two-argument file-helper call still works;
- a separate lightweight Python push/PR workflow catches binding drift;
- `./scripts/check.sh` remains Python-free and green;
- Linux x86_64 and aarch64 wheels build with Rust provisioned inside the
  cibuildwheel Linux container;
- Linux aarch64 uses a native arm64 runner and native smoke;
- macOS x86_64/arm64 and Windows x86_64 wheels pass native smoke;
- one sdist installs directly through `pip` in a clean external venv;
- all five wheel rows have recorded workflow evidence;
- no PyPI publication occurred;
- docs no longer claim qualification ahead of evidence;
- M003 closure is recorded and M002's outstanding qualification condition is
  explicitly reconciled;
- M004 Node becomes the next dependency-ready language-bindings milestone.

## 14. Stop conditions

Stop and report rather than improvising if:

- native `ubuntu-24.04-arm` cannot run the required cibuildwheel container
  stack;
- the required manylinux floor differs from the documented support contract;
- PyO3/abi3 prevents structured exception attributes without changing the
  public exception hierarchy;
- safe file-helper semantics require a breaking public signature rather than
  the additive optional destination described here;
- a direct sdist `pip install` cannot resolve the monorepo path dependencies
  without materially changing package architecture;
- fixing wheel builds requires adding Python/PyO3 dependencies to root/core
  crates;
- any correction requires changing canonical Rust protection or verification
  semantics;
- workflow qualification requires PyPI credentials or automatic publication;
- a supported platform passes only under emulation when native evidence was
  required;
- binding CI would have to be added to `./scripts/check.sh` to function.

Document the blocker and write a new corrective/architecture plan if scope must
change.

## 15. Closure evidence required

Create `plans/closure/language-bindings/003-status.md` using the closure
template and record:

- implementation commit SHA(s);
- changed production/workflow/docs files;
- final Python test count and transcript summary;
- `./scripts/check.sh` result;
- binding `cargo check --locked` result;
- structured-exception attribute examples;
- file-helper success/failure evidence;
- lightweight Python CI run URL/ID;
- manual `release-python.yml` run URL/ID and exact source SHA;
- exact wheel artifact filenames/tags for all five targets;
- per-platform native smoke outcomes;
- Linux logs proving Rust 1.89 availability inside the build container;
- direct sdist `pip install` command and smoke output;
- confirmation that no PyPI publication occurred;
- final SUPPORT/README/RELEASING claims;
- disposition of M002's conditional closure;
- registry/roadmap change that makes M004 Node ready if all gates pass.

Do not copy unexecuted commands into the closure as if they were evidence.

## 16. Handoff notes

Start with WP1/WP2 because they are local correctness changes and expand the
Python test suite before packaging work. Land the lightweight CI and corrected
release workflow only after the local binding remains green.

Keep the release workflow manual. Its first successful dispatch is part of
this milestone's closure evidence, not merely a future operational step.

Prefer the repository's already-used native Linux arm64 runner over introducing
QEMU. Keep the five-target support matrix unchanged.

Do not begin Node binding implementation during this milestone. M004 should be
planned from the final Python error/file/package contract after M003 closes.
