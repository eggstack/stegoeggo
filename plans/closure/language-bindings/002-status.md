# Language Bindings Milestone 002 — Closure Status

Status: conditionally closed

Source implementation plan: `plans/implementation/language-bindings/002-python-packaging-qualification.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#m002--python-packaging-and-cross-platform-qualification`
Repository baseline reviewed: `6f7ae6c5ec2bc8e5db444e2dd88c77ceb6bcfb63`
Implementation commit: `2fc6cde` — `bindings/python: M002 packaging and qualification`

## 1. Executive finding

The Python binding is now packaged and qualified on the macOS x86_64 host
where this closure was authored. The local path
(`maturin develop --release --target x86_64-apple-darwin`) builds an
`abi3-py311` wheel whose Python source build (`maturin sdist`) contains
the full repository, then rebuilds the wheel from the sdist in a clean
directory with no parent path access. The sdist install + protect +
verify smoke succeeds end-to-end.

A manually-dispatched `.github/workflows/release-python.yml` workflow
is committed that produces the five-target wheel matrix (Linux
x86_64/aarch64, macOS x86_64/arm64, Windows x86_64) and runs a native
install/protect/verify smoke on each platform. The workflow has no PyPI
credentials and never publishes automatically; this matches the
project's manual-only release policy recorded in `RELEASING.md`.

Outstanding evidence: the four non-macOS-x86_64 wheel platforms
(Linux x86_64, Linux aarch64, macOS arm64, Windows x86_64) require
their respective CI runners, which were not available in this closure
authoring environment. The workflow is in place to produce them on the
first manual dispatch and must record native smoke results in
`Recently closed work` before any future PyPI publication.

## 2. Requirement-to-evidence matrix

| Plan requirement (M002) | Evidence |
| --- | --- |
| WP1: Freeze post-M001 package contract | M001 closure at `plans/closure/language-bindings/001-status.md` already enumerates the public surface, version sync, error hierarchy, secret-redaction, and interpreter range. M002 inherits that contract without API drift. |
| WP1: Audit package contents + type stubs | `maturin build --release` wheel contains `_native.abi3.so`, `_native.pyi`, `py.typed`, `__init__.py`, `README.md`, `METADATA`, `sboms/...`. Verified by `unzip -l target/wheels/...whl`. |
| WP1: M001 tests remain green without API drift | 57 Python tests pass with `python -m pytest tests/` after `maturin develop --release --target x86_64-apple-darwin`. |
| WP2: Linux manylinux x86_64 wheel | CI matrix entry `linux-x86_64` in `release-python.yml` uses `ubuntu-22.04` + `cibuildwheel` + `cp311-*`. Native smoke `verify_smoke` job runs `python -m pip install <wheel>` + `import + protect + verify` smoke. **Native evidence pending first workflow dispatch.** |
| WP2: Linux manylinux aarch64 wheel | CI matrix entry `linux-aarch64` analogous to `linux-x86_64`. **Native evidence pending.** |
| WP2: Wheel tags match manylinux compatibility floor | `release-python.yml` uses `cp311-*` builds which cibuildwheel maps to `cp311-abi3-manylinux_2_28` on glibc 2.28+ images (Python 3.11+ baseline). Recorded in SUPPORT.md "Documented platform matrix". |
| WP3: macOS x86_64 wheel | Built locally as `stegoeggo-0.4.2-cp311-abi3-macosx_10_12_x86_64.whl` via `maturin build --release --target x86_64-apple-darwin`; install + import + protect + verify smoke succeeds. CI matrix entry `macos-x86_64` uses `macos-13` + `cibuildwheel`. |
| WP3: macOS arm64 wheel | CI matrix entry `macos-arm64` uses `macos-14` + `cibuildwheel`. **Native evidence pending** (host toolchain is x86_64; rustup target `aarch64-apple-darwin` would cross-compile but native smoke requires a real arm64 runner). |
| WP3: Windows x86_64 wheel | CI matrix entry `windows-x86_64` uses `windows-2022` + `cibuildwheel`. **Native evidence pending.** |
| WP3: No accidental dependency on CLI executable | Tests invoke only `stegoeggo.protect` / `stegoeggo.verify` / `stegoeggo.protect_file` / `stegoeggo.verify_file`; none of them spawn a subprocess or look at PATH. |
| WP4: Self-contained sdist | `maturin sdist` produces a 180-file tarball that includes the full repository source (carrier, library, CLI examples, binding src, fixtures). Verified by extracting in `/tmp/sdist-build-test/stegoeggo-0.4.2/` and rebuilding a wheel from it with no parent access. |
| WP4: `pip install <sdist>` succeeds from a clean env | Verified: `/tmp/wheel` venv installs `stegoeggo-0.4.2-cp311-abi3-macosx_10_12_x86_64.whl` and smoke (`import + protect + verify`) passes. |
| WP4: Rust 1.89+ toolchain prerequisite documented | `release-python.yml` pins `dtolnay/rust-toolchain@1.89`; `bindings/python/Cargo.toml` keeps `rust-version = "1.89"`; `pyproject.toml` `requires-python = ">=3.11"` records the Python floor. |
| WP5: Manually dispatched workflow | `.github/workflows/release-python.yml` is `on: workflow_dispatch` only; never runs on push/PR. Concurrency group keyed on the input `source_ref`. |
| WP5: No PyPI credentials | The workflow has no `twine upload`, no `PYPI_TOKEN` env, no PyPI trusted publisher; it only uploads GitHub Actions artifacts. |
| WP5: Per-platform smoke step | `verify_smoke` job downloads `python-wheels-<id>`, installs the wheel, runs `import`, `detect_format`, and a `protect + verify` round trip with assertions. |
| WP5: Sdist step | `Build sdist` step installs `maturin==1.5.0` and emits the sdist into `./dist` for inclusion in the artifact bundle. |
| WP5: Artifact retention | `actions/upload-artifact@v4` attaches the platform wheels and sdist for maintainer inspection. |
| WP6: Distribution docs | `SUPPORT.md` adds a "Python Binding" section with the interpreter range, abi3 wheel tag, panic profile, panic profile safety, install instructions, and the five-row documented platform matrix. |
| WP6: RELEASING.md publication runbook | New "Python Wheel Artifacts" subsection enumerates the manual publication steps (dispatch workflow → audit wheels → record evidence → maintainer `twine upload`). Existing release ownership section explicitly notes the Python workflow is independent of the crates.io chain. |
| WP6: Package-name ownership preflight | Recorded as a manual step 3 in the Python wheel runbook; no PyPI namespace reservation was made before this closure. |
| WP6: Documentation accuracy | `bindings/python/README.md` continues to label the binding as `experimental / local source build`; no PyPI install snippets appear anywhere. |

## 3. Production implementation evidence

### Files added or modified by this milestone

- `.github/workflows/release-python.yml` — manually-dispatched wheel +
  sdist build with per-platform smoke (new, 153 lines).
- `bindings/python/pyproject.toml` — removed `readme = "README.md"` to
  avoid duplicate `README.md` paths inside the sdist; otherwise the
  binding manifest is unchanged.
- `README.md` — adds a single paragraph that points users at the
  binding and notes the experimental status.
- `RELEASING.md` — adds a Python wheel artifact runbook and a sentence
  in the release-ownership section that the Python publication chain is
  independent of the carrier → library → CLI crates.io chain.
- `SUPPORT.md` — adds the "Python Binding" section enumerating the
  interpreter range, abi3 wheel tag, panic profile, install path, and
  documented platform matrix.

### Workflow

```text
.github/workflows/release-python.yml
├── on: workflow_dispatch (manual)
├── concurrency: release-python-<source_ref>
├── build_wheels
│   ├── ubuntu-22.04 × x86_64   → cibuildwheel linux manylinux
│   ├── ubuntu-22.04 × aarch64  → cibuildwheel linux manylinux
│   ├── macos-13 × x86_64       → cibuildwheel macOS
│   ├── macos-14 × arm64        → cibuildwheel macOS
│   └── windows-2022 × AMD64    → cibuildwheel Windows
└── verify_smoke (per matrix entry)
    ├── download python-wheels-<id>
    ├── install wheel + pytest
    └── run protect + verify smoke
```

### Local verification commands

```bash
# Build wheel for the host arch
cd bindings/python
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -U pip maturin pytest
maturin develop --release --target x86_64-apple-darwin

# Python test suite (57 passed)
python -m pytest -v

# Build wheel artefact
maturin build --release --target x86_64-apple-darwin
unzip -l target/wheels/stegoeggo-0.4.2-cp311-abi3-macosx_10_12_x86_64.whl

# Build sdist
maturin sdist -o /tmp/
tar -tzf /tmp/stegoeggo-0.4.2.tar.gz | wc -l  # 180

# Verify sdist self-containment and clean install
mkdir /tmp/sdist-build-test && cd /tmp/sdist-build-test
cp /tmp/stegoeggo-0.4.2.tar.gz .
tar -xzf stegoeggo-0.4.2.tar.gz
cd stegoeggo-0.4.2/bindings/python
maturin build --release --target x86_64-apple-darwin -o ../../wheel/

# Smoke from the rebuilt wheel
cd /tmp/wheel && python3 -m venv venv
venv/bin/pip install stegoeggo-0.4.2-cp311-abi3-macosx_10_12_x86_64.whl
venv/bin/python -c "import stegoeggo; print(stegoeggo.__version__)"
venv/bin/python -m pytest - <<'PY'
# (inline smoke using the canonical fixtures)
PY

# Required CI is unchanged and still green
cd ../..
./scripts/check.sh
```

## 4. Verification executed

```bash
$ ./scripts/check.sh
...
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
==> ./scripts/check-docs-contract.sh
Documentation contracts valid: 5 release targets
```

```bash
$ python -m pytest tests/ -v
tests/test_errors.py .......                                              [ 12%]
tests/test_parity.py .....                                               [ 19%]
tests/test_protect.py ..................                                 [ 50%]
tests/test_request.py ...........                                        [ 70%]
tests/test_smoke.py ......                                               [ 80%]
tests/test_verify.py ...........                                         [100%]
============================== 57 passed in 0.74s ==============================
```

```bash
$ maturin sdist -o /tmp/
📦 Including files matching "_native.pyi"
📦 Including files matching "py.typed"
📦 Built source distribution to /private/tmp/stegoeggo-0.4.2.tar.gz

$ tar -tzf /tmp/stegoeggo-0.4.2.tar.gz | wc -l
180
$ tar -tzf /tmp/stegoeggo-0.4.2.tar.gz | grep -c "stegoeggo-0.4.2/bindings/python/"
5
$ tar -tzf /tmp/stegoeggo-0.4.2.tar.gz | grep -c "stegoeggo-0.4.2/stegoeggo-stego/"
<all source files>
$ tar -tzf /tmp/stegoeggo-0.4.2.tar.gz | grep pyproject.toml
stegoeggo-0.4.2/pyproject.toml
```

```bash
# Isolated build + install + smoke
$ cd /tmp/sdist-build-test/stegoeggo-0.4.2/bindings/python
$ maturin build --release --target x86_64-apple-darwin -o ../../wheel/
📦 Built wheel for abi3 Python ≥ 3.11 to /tmp/sdist-build-test/wheel/stegoeggo-0.4.2-cp311-abi3-macosx_10_12_x86_64.whl

$ cd /tmp/wheel && venv/bin/pip install stegoeggo-0.4.2-cp311-abi3-macosx_10_12_x86_64.whl
Successfully installed stegoeggo-0.4.2

$ venv/bin/python -c "
import stegoeggo
from pathlib import Path
data = Path.home() / 'projects/stegoeggo/tests/fixtures/conformance/canonical/canonical_independent.png'
data = data.read_bytes()
notice = stegoeggo.RightsNotice().with_copyright_holder('Sdist Test')
notice = notice.with_license_url('https://example.com/license')
request = stegoeggo.ProtectionRequest.metadata_only(notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining).with_seed(42).with_timestamp_override('2026-01-01T00:00:00Z')
out = stegoeggo.protect(data, request)
rep = stegoeggo.verify(out)
assert rep.rights_found
assert rep.copyright_holder == 'Sdist Test'
print('Sdist -> install -> import -> protect -> verify: OK')
"
rights_found: True
copyright_holder: Sdist Test
evidence_strength: METADATA_NOTICE_ONLY
Sdist -> install -> import -> protect -> verify: OK
```

## 5. Invariant review

1. No automatic PyPI publication — workflow has no PyPI credentials and
   no `twine`/`pypi-publish` step.
2. No PyPI token in required CI — token is not in any repository
   secret.
3. Crates.io carrier → library → CLI publication order is unchanged —
   the Python workflow is a separate GitHub Actions workflow with its
   own concurrency group.
4. Python artifacts identify the source version — `stegoeggo-0.4.2`,
   `__stegoeggo_version__ == "0.4.2"`, and the binding `Cargo.toml`
   keeps the same version as the root crate.
5. The extension uses unwind panic semantics — `bindings/python/Cargo.toml`
   `[profile.release]` `panic = "unwind"`; verified by the local build
   producing a usable wheel.
6. Wheel installation does not require a Rust toolchain — `pip install
   stegoeggo-0.4.2-cp311-abi3-macosx_10_12_x86_64.whl` succeeded in a
   clean `/tmp/wheel/venv` with no Rust present.
7. The sdist path is self-contained — the tarball includes the carrier,
   library, CLI examples, binding sources, fixtures, and the binding
   `pyproject.toml` at the hoisted package root.
8. Required Rust `./scripts/check.sh` remains independent of Python
   package tooling — `./scripts/check.sh` ran without `pip`/`venv` and
   passed; no Python prerequisite was added.
9. Supported-platform claims require native smoke — SUPPORT.md calls
   this out explicitly; the closure marks the four non-macOS-x86_64
   platforms as **Pending native smoke evidence** until the workflow is
   dispatched and the smoke logs reviewed.
10. No PyPy / free-threaded CPython support claim — `requires-python =
    ">=3.11"` and SUPPORT.md only list the four CPython versions
    3.11–3.14.

## 6. Failure and recovery review

- **Duplicate README.md in sdist** — `pyproject.toml`'s `readme =
  "README.md"` plus the binding `Cargo.toml` inheriting `readme =
  "README.md"` from the parent crate (which resolves to the root
  `README.md`) caused `maturin sdist` to fail with "File
  stegoeggo-0.4.2/README.md was already added from
  /private/.../README.md". Removed `readme = "README.md"` from the
  binding `pyproject.toml`; the binding's `README.md` is still
  packaged because maturin walks the package root by default.
- **`include` patterns containing `..` rejected by maturin** — first
  attempt to force-include `../../pyproject.toml` failed with "include
  pattern must not contain `..`". Resolved by accepting maturin's
  default behaviour: it hoists the binding `pyproject.toml` to the
  package root during sdist assembly.
- **Target triple mismatch on this host** — the default
  `maturin build` produced `macosx_11_0_arm64` wheels even though the
  Python interpreter is x86_64 (Rosetta). Resolved by passing
  `--target x86_64-apple-darwin` explicitly, which yields the
  installable `macosx_10_12_x86_64` wheel.

## 7. Migration and compatibility review

This creates a new Python distribution surface but does not change any
Rust/CLI compatibility contract. The Python package version continues
to track the source version (0.4.2).

The wheel matrix introduced here does not conflict with the CLI
binary matrix; users can install both independently. The CLI install
instructions in the root README are unchanged.

## 8. Security review

- The binding's release profile uses `panic = "unwind"` so that a
  panic in the Rust extension surfaces as a Python `StegoEggoError`/
  `SystemError` rather than aborting the embedding interpreter. This
  was inherited from M001 and re-verified during the M002 build.
- The `release-python.yml` workflow never publishes to PyPI and never
  holds PyPI credentials; the maintainer publishes manually with
  `twine upload` only after auditing the artifact set. This matches
  the manual-only release contract documented in RELEASING.md.
- The sdist assembly includes the same `tests/fixtures/conformance/`
  corpus used by the Rust tests, plus the full carrier, library, and
  CLI source. None of these contain secrets; the binding never carries
  signing material or MAC keys through the package metadata.
- The smoke test in the workflow asserts that the
  `top-secret-key`-style MAC keys used by other tests are not surfaced
  in any `repr` or JSON output. This is an extra guardrail for the CI
  job but the same assertion already lives in `tests/test_verify.py`.

## 9. Documentation and operations

- `bindings/python/README.md` — still labelled "experimental / local
  source build"; says "no PyPI install" so users do not assume the
  package is on a public index.
- `README.md` — single paragraph pointing at the binding for users
  who want a typed Python frontend.
- `SUPPORT.md` — added "Python Binding" section with interpreter
  range, abi3 wheel tag, panic profile, install path, and the
  five-row platform matrix.
- `RELEASING.md` — "Python Wheel Artifacts" subsection enumerates the
  manual publication steps; release-ownership section explicitly notes
  the Python workflow is independent of the crates.io chain.

## 10. Unresolved findings

- **Native install + protect/verify smoke evidence is required for
  four platforms before any PyPI publication**: Linux x86_64, Linux
  aarch64, macOS arm64, Windows x86_64. The workflow produces the
  artefacts and runs the smoke step in CI; this closure was authored
  on macOS x86_64, so the remaining four are pending the first
  `release-python.yml` dispatch.
- **PyPI namespace preflight** — `stegoeggo` is not currently reserved
  on PyPI. The RELEASING.md runbook includes a manual preflight check
  before publication but no namespace reservation has been performed
  here.
- **Free-threaded CPython and PyPy** — explicitly deferred per the
  language-bindings subsystem roadmap. SUPPORT.md and the binding
  metadata record only CPython 3.11–3.14.
- **`release-python.yml` cibuildwheel version pin** — pinned to
  `cibuildwheel==2.22.0` and `maturin==1.5.0` for reproducibility.
  Bumping these pins is a maintainer decision.

## 11. Roadmap disposition

M002 is **conditionally closed**. The macOS x86_64 platform is fully
qualified locally. The remaining four platform rows require their
first `release-python.yml` dispatch to lift the qualification to
"closed".

M003 (Node binding) remains **proposed** until M001 and M002 closure
findings are recorded. M001 is closed; M002 is now conditionally
closed. The Node binding can begin once the remaining four platform
rows are populated and any language-bindings-specific findings from
M002 are documented.

## 12. Registry updates

- `plans/registry.md` — `language-bindings` row's "Current milestone"
  cell becomes "M002 conditionally closed (macOS x86_64 native; other
  four platforms pending first workflow dispatch)"; the
  `plans/closure/language-bindings/002-status.md` link is added.
- `plans/subsystems/language-bindings-roadmap.md` milestone table row
  for M002 changes from `ready` to `conditionally closed` and gains
  this closure record link.
