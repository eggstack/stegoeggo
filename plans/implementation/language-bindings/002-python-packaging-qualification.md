# Language Bindings Milestone 002 — Python Packaging and Qualification

Status: blocked
Repository baseline: `fe86d7361ff9b4ac725e1b05ef915ac391803868`
Source roadmap: `plans/subsystems/language-bindings-roadmap.md#7-milestones`
Long-term requirements: `plans/000-long-term-specification.md#3-execution-invariants`, `plans/000-long-term-specification.md#5-release-invariants`
Applicable ADRs: `plans/adrs/ADR-0003-byte-vs-pixel-paths.md`, `plans/adrs/ADR-0005-foreign-language-bindings.md`
Primary class: capability

Blocker: M001 Python binding foundation must close with accepted parity,
error-model, panic-profile, and local-install evidence.

## 1. Objective

Turn the locally functional Python binding into a reproducibly buildable,
installable, cross-platform Python distribution without changing StegoEggo's
manual-release policy.

The milestone closes when the package produces and smoke-tests normal-CPython
abi3 wheels for the intended platform matrix, a clean source-build path is
proven from the actual built sdist, support/version contracts are documented,
and a manual artifact build/rehearsal workflow exists. PyPI publication itself
is a separate maintainer action and is not required for closure.

## 2. Why this milestone is blocked

Packaging must not freeze a Python compatibility contract before the M001 API,
exception hierarchy, secret-redaction behavior, interpreter-detachment model,
and Rust/Python parity tests are accepted.

Unblock only after
`plans/closure/language-bindings/001-status.md` exists and the registry marks
M001 closed.

## 3. Current implementation evidence

Baseline repository distribution supports five native CLI targets:

- Linux x86_64;
- Linux aarch64;
- macOS x86_64;
- macOS aarch64;
- Windows x86_64.

The binary workflow is manually dispatched and never publishes crates. The
project requires manual release ownership and prohibits automated registry
publication. Root package releases use one source version across carrier,
library, and CLI.

M001 is expected to leave `bindings/python` outside the root Rust workspace,
with its own lock/build profile and local maturin build.

## 4. Invariants that must not regress

1. No tag-triggered or automatic PyPI publication.
2. No PyPI credential/token added to required CI.
3. Existing crates.io carrier -> library -> CLI publication order is unchanged.
4. Python artifacts identify the exact StegoEggo source version they wrap.
5. The extension uses unwind panic semantics.
6. Wheel installation must not require a Rust toolchain.
7. The sdist path, if shipped, must include enough workspace source to build in
   isolation; success from a checkout is not sufficient evidence.
8. Required Rust `./scripts/check.sh` remains independent of Python package
   tooling unless a separate maintainer decision changes policy.
9. Supported-platform claims require native install/import/protect/verify
   smoke evidence.
10. Do not claim PyPy or free-threaded CPython support without explicit
    qualification.

## 5. Scope

### In

- Finalize CPython minimum-version and abi3 baseline from M001 evidence.
- Build wheels for the five platform/architecture families matching the CLI
  support matrix where PyPI wheel standards permit.
- Linux manylinux policy selection and audit.
- macOS universal/source minimum-version metadata as appropriate; separate
  x86_64/arm64 wheels are acceptable.
- Windows x86_64 wheel.
- Build an sdist if it can be made self-contained and reproducible.
- Clean-environment wheel install + import + protect + verify smoke tests.
- Clean-environment sdist build/install smoke test.
- Type-stub inclusion verification.
- Version-matching checks.
- Manual GitHub Actions artifact-build/rehearsal workflow or scripts that
  produce downloadable wheel/sdist artifacts without publishing them.
- Root docs/SUPPORT/RELEASING updates for manual Python distribution.
- Package-registry namespace preflight before any later publication.

### Explicitly out

- Automatic PyPI publication.
- Tag-triggered publication.
- Free-threaded CPython support commitment.
- PyPy support commitment.
- Node.js/C ABI work.
- Generic stego carrier bindings.
- Changing the existing CLI release target list or updater authority.
- Making Python wheel success a required merge gate without a maintainer
  decision.

## 6. Required production changes

### Stable ABI and Python range

Use PyO3/maturin's normal-CPython abi3 support so one wheel per
OS/architecture can support the documented Python range. The initial planning
baseline is Python >=3.11; M002 may raise but must not lower that floor without
explicit support justification and tests.

Do not advertise free-threaded compatibility merely because the package can
compile on one interpreter. Treat free-threaded CPython as a later
qualification milestone.

### Artifact versioning

The Python distribution version must equal the StegoEggo source version
embedded/used by the binding. Add a mechanical check that catches version
drift before artifact creation.

The binding Rust crate remains `publish = false` on crates.io. Python
artifact publication is independent of the three-crate crates.io dependency
sequence.

### Wheel matrix

Target at least:

```text
manylinux x86_64
manylinux aarch64
macOS x86_64
macOS arm64
Windows x86_64
```

Prefer a Linux compatibility floor no newer than necessary; align with the
project's existing broad Linux distribution intent where toolchain/dependency
constraints allow. Record the final manylinux/glibc contract in SUPPORT docs
rather than inferring it from a runner image.

### Source distribution

Because `bindings/python` depends on the repository root and
`stegoeggo-stego` through workspace/path relationships, build the sdist and
then test that tarball from outside the checkout with no access to repository
parent paths.

If maturin's normal sdist assembly omits required root/carrier sources, use its
documented include/source-generation facilities or a minimal packaging script.
Do not publish an sdist that only works when built from inside the monorepo.

If a correct self-contained sdist materially bloats or complicates the
distribution, stop and present wheel-only versus sdist tradeoffs rather than
shipping a broken source artifact.

### Manual artifact workflow

A new workflow may be manually dispatched to build/test artifacts and upload
them as GitHub Actions artifacts. It MUST:

- have no PyPI publishing credentials;
- not run on tag push as a publication action;
- fail if version checks or smoke tests fail;
- build from an explicitly selected source ref/tag;
- retain per-platform artifacts for maintainer inspection.

Registry publication remains a documented local/manual maintainer step.

## 7. Ordered work packages

### WP1 — Freeze post-M001 package contract

Intent: turn the accepted M001 API into an explicit 0.x Python support
contract.

Changes:

- record minimum Python;
- record abi3 baseline;
- record supported/unsupported interpreter families;
- audit package contents, import surface, typing, exceptions, secret repr;
- add package version synchronization check.

Acceptance evidence:

- package metadata and docs agree;
- M001 tests remain green without API drift.

### WP2 — Linux wheels

Intent: produce installable x86_64/aarch64 manylinux wheels.

Changes:

- maturin manylinux build configuration;
- native/cross runners as justified;
- audit tags and external shared-library dependencies;
- clean-container/runner installation tests.

Acceptance evidence:

- wheel installs without Rust;
- import, metadata-only protect, hidden-marker protect, and verify smoke pass
  on both architectures;
- wheel tags match the documented compatibility floor.

### WP3 — macOS and Windows wheels

Intent: complete the primary platform matrix.

Changes:

- x86_64 + arm64 macOS wheel builds;
- Windows x86_64 wheel build;
- native smoke tests;
- verify no accidental dependency on CLI executable presence.

Acceptance evidence:

- install/import/protect/verify smoke on every target;
- package includes type metadata/stubs on every platform.

### WP4 — Isolated sdist

Intent: prove source users can build without monorepo parent access.

Changes:

- include all required root/carrier source;
- exclude unrelated release artifacts/secrets;
- document Rust 1.89+ and native build prerequisites;
- isolated pip build/install test.

Acceptance evidence:

- build sdist;
- copy only sdist to a clean temp directory/container;
- `pip install <sdist>` succeeds with supported Rust/Python;
- import/protect/verify smoke passes.

### WP5 — Manual artifact workflow/rehearsal

Intent: make artifact production repeatable without changing release policy.

Changes:

- manually dispatched `.github/workflows/release-python.yml` or equivalent;
- matrix build;
- per-target smoke;
- sdist test;
- artifact upload;
- no registry publication step.

Acceptance evidence:

- one manual rehearsal produces the expected complete artifact set;
- no PyPI/npm/crates credentials are present;
- workflow cannot publish merely from tag creation.

### WP6 — Distribution docs and publication runbook

Intent: make future maintainer publication explicit and auditable.

Changes:

- `bindings/python/README.md`;
- root README installation snippet after artifacts are actually qualified;
- `SUPPORT.md` Python matrix;
- `RELEASING.md` manual Python artifact/publication section;
- package-name ownership preflight instructions;
- rollback/yank/new-version rules consistent with immutable registry artifacts.

Acceptance evidence:

- docs do not claim an unpublished package is available;
- manual commands distinguish build/rehearsal from actual PyPI publication.

## 8. Failure, cancellation, restart, contention semantics

Artifact builds are immutable outputs of a source ref. Re-running a failed
build may replace Actions artifacts for rehearsal, but must not imply a
published registry version can be overwritten.

If any wheel target fails native smoke, the release set is incomplete; do not
document that target as supported.

If a Python registry version is ever published with defective bytes, correct
with a new unused version. Do not assume deletion/yanking makes a version
reusable.

If the target PyPI project name is unavailable or owned by an unrelated party,
stop before publication and request a naming decision. Do not silently publish
under an ad hoc alternate name while keeping `import stegoeggo` ambiguous.

## 9. Compatibility and migration

M002 establishes the first distribution-level Python compatibility statement.
Keep it 0.x and additive where possible.

The Python distribution version tracks StegoEggo source version. Installation
through wheels does not affect or replace the Rust CLI binary or crates.io
packages; users may install both independently.

## 10. Required tests

In addition to all M001 tests:

- built wheel contents include `py.typed` and stubs;
- wheel install without Cargo/Rust;
- import/version on each platform;
- one metadata-only protect+verify smoke per platform;
- one hidden-marker protect+verify smoke per platform;
- malformed input exception smoke per platform;
- sdist isolated build/install;
- source-version mismatch negative test;
- artifact-set completeness test;
- workflow no-publish/credential policy review;
- package metadata validation (for example `twine check` or maturin
  equivalent) on wheel + sdist.

## 11. Required verification commands

Exact commands depend on the implemented workflow, but closure MUST record at
least:

```bash
./scripts/check.sh

cd bindings/python
maturin build --release
maturin sdist

# clean wheel environment
python -m venv <tmp>
python -m pip install <built-wheel>
python -c "import stegoeggo; print(stegoeggo.__version__)"

# clean sdist environment, outside repository tree
python -m pip install <built-sdist>
python -c "import stegoeggo; print(stegoeggo.__version__)"
```

Record native platform matrix workflow run identifiers/artifact names for the
manual rehearsal. Do not claim a platform from cross-compilation alone when
the plan requires native smoke evidence.

## 12. Documentation updates

Update:

- `bindings/python/README.md`;
- root `README.md`;
- `SUPPORT.md`;
- `RELEASING.md`;
- architecture/binding documentation as needed;
- `CHANGELOG.md` only when the implementation is actually scheduled for a
  release.

Clearly separate:

- local/source installation;
- qualified wheel support;
- manual artifact build;
- actual PyPI publication.

## 13. Acceptance criteria

A maintainer can select a StegoEggo source ref and produce a complete,
version-matched Python artifact set. Each supported wheel installs without Rust
and passes native protect/verify smoke. The sdist, if shipped, builds from
itself outside the monorepo. Documentation states the exact interpreter and
platform support.

No automatic registry publication exists, and the existing Rust release/CI
contracts are unchanged.

## 14. Stop conditions

Stop and report if:

- M001 closure is missing or records unresolved semantic/parity defects;
- abi3 cannot support a required Python API used by the implementation;
- a wheel needs an undocumented system shared library not acceptable for the
  target;
- the sdist cannot be made self-contained without a significant packaging
  architecture change;
- the package namespace is unavailable;
- adding artifact qualification would require making Python tooling a required
  Rust CI prerequisite;
- the only available workflow design violates manual-only publication policy.

## 15. Closure evidence required

Create
`plans/closure/language-bindings/002-status.md` with:

- final Python/interpreter/abi3 support matrix;
- wheel filenames/tags and native smoke results;
- sdist filename + isolated build evidence, or explicit accepted wheel-only
  disposition;
- artifact workflow rehearsal evidence;
- version-synchronization evidence;
- package-content/type-stub checks;
- docs updated;
- remaining free-threaded/PyPy limitations;
- commit(s) implementing the milestone.

Then update the roadmap and registry: M002 -> closed and M003 Node binding ->
ready for planning/implementation-plan derivation.

## 16. Handoff notes

Do not broaden this milestone into Node, C ABI, generic carrier Python
bindings, or automatic publishing. Treat packaging as capability evidence:
the artifact must install and run on the target, not merely compile.

If M001 discovers a materially different public surface or packaging
constraint, write a corrective M001 plan/closure first and then revise this
blocked plan before marking it ready.
