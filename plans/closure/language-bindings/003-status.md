# Language Bindings Milestone 003 — Closure Status

Status: closed

Source implementation plan: `plans/implementation/language-bindings/003-python-corrective-qualification.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#m003--python-corrective-qualification`
Repository baseline reviewed: `90c58d4f0df037319cc554a84c00245338491879`
Implementation commits (single dispatch SHA = `d40f1b3`, plus workflow-defect follow-ups): see "Working dispatch" below for the SHA actually exercised by the manually-dispatched workflow.

```
1f23a3c  bindings/python: M003 corrective qualification (WP1-WP6)
495a57f  release-python: use dtolnay toolchain @rev tag (not toolchain input)
2298322  release-python: pass CIBW_BUILD via env var (not --build cli flag)
d269513  release-python: expose cargo PATH in cibuildwheel container via CIBW_ENVIRONMENT
619c0e3  release-python: pin bash shell + MACOSX_DEPLOYMENT_TARGET=10.12
601ffd0  release-python: pin each row to a single cibuildwheel identifier
d40f1b3  release-python: Linux-only Rust provisioning and a valid PNG smoke
```

## 1. Executive finding

The Python binding is now a fully-qualified foreign-language frontend.
Plan 003 closes the structured-error, file-helper, lightweight CI,
native five-platform wheel matrix, single-sdist, and documentation
gaps that M001/M002 left open. The manually-dispatched
`release-python.yml` workflow produced every documented wheel on its
native arch (Linux x86_64 `ubuntu-24.04`, Linux aarch64
`ubuntu-24.04-arm`, macOS x86_64 `macos-15-intel`, macOS arm64
`macos-14`, Windows x86_64 `windows-2022`), ran a per-platform
import + protect + verify smoke against the same-native-arch wheel,
and ran a single dedicated sdist build whose
`pip install <tarball>` smoke succeeded in a clean venv that has
no path access to the source checkout.

M002's outstanding "other four wheel platforms" condition is
satisfied: every Linux/macOS/Windows platform row now has recorded
native smoke evidence in this closure record.

M004 (Node binding) is now dependency-ready.

## 2. Requirement-to-evidence matrix

| Plan requirement (M003) | Evidence |
| --- | --- |
| WP1 Structured `InsufficientCapacityError` with `required`/`available` | `test_insufficient_capacity_structured_attributes` and `test_input_too_large_*` exercises the new path; `bindings/python/src/lib.rs` `InsufficientCapacity { .. }` arm of `map_error` sets `required` and `available` (`map_error` block at `bindings/python/src/lib.rs`). `_native.pyi` documents both attributes. |
| WP1 Structured `ResourceLimitError` for every variant: `InputTooLarge` (resource=input_bytes, size, limit); `DimensionsExceeded` (resource=dimensions, width, height, max_width, max_height); `ContainerLimitExceeded` (resource=container, kind, count, limit); `MetadataLimitExceeded` (resource=metadata, kind, size, limit); `VerificationBudgetExceeded` (resource=verification_budget, kind, count, limit) | All five variants are projected in the new `map_error` (`bindings/python/src/lib.rs`). Four of the five (InputTooLarge, DimensionsExceeded, ContainerLimitExceeded, MetadataLimitExceeded) are exercised end-to-end via `tests/test_errors.py` (`test_input_too_large_structured_attributes`, `test_dimensions_exceeded_structured_attributes`, `test_container_limit_exceeded_structured_attributes`, `test_metadata_limit_exceeded_structured_attributes`). `VerificationBudgetExceeded` is a defined-but-not-yet-raised variant; per WP1 it is intentionally left without an end-to-end Python test and is documented in `_native.pyi`. |
| WP1 `ImageTruncated` maps to a documented existing category | `tests/test_errors.py::test_truncated_input_is_encoded_decode_error` asserts `EncodeDecodeError` for a payload whose truncation triggers `Error::ImageTruncated`. The mapping lives in the new `map_error` match arm. |
| WP1 Unknown future variants fall back to base safely | The `_ => PyErr::new::<StegoEggoError, _>` arm of `map_error` is still hit when no other variant matches; covered by the existing `test_invalid_format_bytes_raises` plus the existing `test_invalid_format_raises_invalid_format_error` and the no-leak tests. |
| WP1 No secret key leakage in any structured exception | `test_resource_limit_error_does_not_leak_secret_keys`, `test_invalid_format_does_not_leak_secret_keys`, and the pre-existing `test_protect_with_report_does_not_leak_mac_key` / `test_verify_does_not_leak_mac_key` all assert `secret_key` text is absent from the exception repr / __dict__. |
| WP2 `protect_file(path, request)` two-argument compatibility form | `test_protect_file_in_place` exercises the existing 2-arg call. |
| WP2 Optional `output_path` leaves source unchanged | `test_protect_file_explicit_output_leaves_source_unchanged` builds a fresh `in.png`, processes with `output_path=out.png`, asserts `in.png` bytes are unchanged and `out.png` decodes as a protected image. |
| WP2 Processing failure leaves source unchanged; no partial output | `test_protect_file_processing_failure_leaves_source_unchanged` invokes the helper on an invalid payload (`b"definitely not an image"`), asserts a `StegoEggoError` is raised and the source bytes are identical. |
| WP2 Forced write failure leaves source unchanged | `test_protect_file_forced_write_failure_leaves_source_unchanged` monkey-patches `_atomic_write_bytes` to raise `OSError("simulated write failure")`; the source file is unchanged afterwards. |
| WP2 Temp files are cleaned up on every tested failure path | `test_protect_file_forced_failure_cleans_temp_files` and `test_atomic_write_failure_cleans_temp` iterate the destination directory and assert no leftover `.stegoeggo-*.tmp` files. |
| WP2 `verify_file` is a thin byte-API wrapper | `test_verify_file_equivalent_to_byte_verify` asserts `verify_file(path)` and `verify(path.read_bytes())` agree on `rights_found` and `copyright_holder`. Native helpers (`protect_file`/`verify_file`) were removed from the PyO3 module (`bindings/python/src/lib.rs`) and re-exported from the pure-Python `_files.py`. |
| WP3 Lightweight binding CI on Linux x86_64/CPython 3.11/Rust 1.89, path-filtered | `.github/workflows/python-binding.yml` triggers on push/PR when `bindings/python/**`, `src/**`, `stegoeggo-stego/**`, `Cargo.toml`, `Cargo.lock`, `bindings/python/tests/fixtures/**`, or the workflow file itself changes; single job `python-binding-check`. `./scripts/check.sh` and `ci.yml` are unchanged. |
| WP4 Native runner per wheel row (Linux x86_64 `ubuntu-24.04`, Linux aarch64 `ubuntu-24.04-arm`, macOS x86_64 `macos-15-intel`, macOS arm64 `macos-14`, Windows x86_64 `windows-2022`) | `.github/workflows/release-python.yml` matrix uses exactly those five runners; `build_wheels` job confirms `Image:` for each row matched its native arch in run 36337194059 (see "Working dispatch" below). |
| WP4 Rust 1.89 provisioned inside the cibuildwheel Linux build container | `CIBW_BEFORE_ALL_LINUX` installs `rustup` with `1.89` for the host target (`x86_64-unknown-linux-gnu` / `aarch64-unknown-linux-gnu`); `CIBW_ENVIRONMENT_LINUX: PATH="$HOME/.cargo/bin:$PATH"` exposes it. Build-wheel logs for `linux-x86_64` and `linux-aarch64` show `1.89.0-x86_64-unknown-linux-gnu installed - rustc 1.89.0` / `1.89.0-aarch64-unknown-linux-gnu installed - rustc 1.89.0` *inside* cibuildwheel's container. |
| WP4 Each wheel's smoke runs on a runner that matches the wheel's native arch | The `verify_smoke` matrix uses the same five runners. Run 36337194059 logs show `smoke linux-x86_64` on `Image: ubuntu-24.04`, `smoke linux-aarch64` on `Image: ubuntu-24.04-arm`, `smoke macos-x86_64` on `Image: macos-15`, `smoke macos-arm64` on `Image: macos-14-arm64`, and `smoke windows-x86_64` on `Image: windows-2022`. |
| WP4 Explicit manylinux floor: manylinux_2_28 | `CIBW_MANYLINUX_X86_64_IMAGE: manylinux_2_28` and `CIBW_MANYLINUX_AARCH64_IMAGE: manylinux_2_28`. The macOS deployment target is pinned at 10.12 via `CIBW_ENVIRONMENT_MACOS: MACOSX_DEPLOYMENT_TARGET=10.12`. |
| WP4 One sdist per dispatch (not per matrix row) | The `build_sdist` job runs once on `ubuntu-24.04` and uploads `python-sdist`; downstream `sdist_install_smoke` and the matrix smoke downloads consume it exactly once. |
| WP5 A literal `pip install <tarball>` from a clean venv | `sdist_install_smoke` step executes `python -m venv /tmp/stegoeggo-sdist-venv` and then `/tmp/stegoeggo-sdist-venv/bin/python -m pip install --no-cache-dir stegoeggo-0.4.2.tar.gz`; no checkout on `sys.path`. Run log shows the install succeeded and `sdist direct-pip-install smoke ok`. |
| WP5 Import + protect + verify against the installed package | sdist smoke script imports `stegoeggo`, asserts `__version__ == "0.4.2"`, generates a 16x16 RGB PNG in-script, calls `protect` and `verify`, and asserts `rights_found` and `copyright_holder`. |
| WP6 README enum examples use the actual exported PyO3 names | `bindings/python/README.md` example now uses `stegoeggo.RightsPolicy.ProhibitedAiMlTraining` and `stegoeggo.ImageOutputFormat.Png`. |
| WP6 SUPPORT.md distinguishes configured/pending from qualified | The Python Binding platform matrix now uses "Configured (qualified when smoke evidence recorded)" for every row until that row's run is recorded. |
| WP6 RELEASING.md reflects corrected five-platform mechanics | RELEASING.md `## Python Wheel Artifacts` describes the corrected runners, the in-container Rust install, the `manylinux_2_28` floor, the single sdist, and the literal `pip install <tarball>` proof step. |
| WP6 plans/registry.md no duplicate heading | Consolidated the duplicated `## Recently closed work` heading under M002; preserved the rest of the section. |
| WP6 M002 condition reconciliation | M002's outstanding "other four wheel platforms" condition is satisfied by the M003 evidence recorded below; M002 status moves from "conditionally closed" to "closed" at the closure of M003 (see Registry updates). |
| WP7 Five native wheel installs + protect/verify smokes | Five matrix jobs in run 36337194059 produced install + protect + verify smoke = `smoke ok`. See "Working dispatch". |
| WP7 One sdist direct pip install smoke | `sdist direct pip install smoke` job in run 36337194059 produced `sdist direct-pip-install smoke ok`. See "Working dispatch". |
| WP7 No PyPI publication | `release-python.yml` has no PyPI credentials, no `twine upload` / `pypi-publish` step; workflow comment block documents the manual-only policy. |

## 3. Production implementation evidence

### Changed files

- `bindings/python/src/lib.rs` — `map_error` now takes `py` and projects
  structured attributes for `InsufficientCapacity`, `InputTooLarge`,
  `DimensionsExceeded`, `ContainerLimitExceeded`,
  `MetadataLimitExceeded`, `VerificationBudgetExceeded`; `ImageTruncated`
  mapped to `EncodeDecodeError`; native `protect_file`/`verify_file`
  removed in favor of the pure-Python layer.
- `bindings/python/python/stegoeggo/_files.py` — new module
  implementing failure-safe `protect_file` (and optional
  `output_path`) and `verify_file` using same-directory tempfile +
  `os.fsync` + `os.replace`.
- `bindings/python/python/stegoeggo/__init__.py` — re-exports the
  file helpers from `_files`.
- `bindings/python/python/stegoeggo/_native.pyi` — adds `required`/
  `available` to `InsufficientCapacityError` and the documented
  structured fields to `ResourceLimitError`.
- `bindings/python/tests/test_errors.py` — extended with structured
  attribute tests for `InsufficientCapacityError`,
  `InputTooLarge`, `DimensionsExceeded`, `ContainerLimitExceeded`,
  `MetadataLimitExceeded`, and no-leak tests.
- `bindings/python/tests/test_files.py` — new: in-place,
  explicit-output, processing-failure, forced-write-failure, temp
  cleanup, `verify_file` equivalence, and atomic-write primitives.
- `bindings/python/.gitignore` — new: ignores build/cache artifacts.
- `bindings/python/README.md` — corrected enum names; documents
  optional `output_path`, failure-safe in-place semantics, and the
  structured-exception contract.
- `.github/workflows/python-binding.yml` — new lightweight push/PR
  CI; Linux x86_64 / CPython 3.11 / Rust 1.89 single job; path-filtered
  on binding-relevant changes only.
- `.github/workflows/release-python.yml` — corrected platform
  matrix, explicit `manylinux_2_28` floor, `CIBW_BEFORE_ALL_LINUX`
  installing Rust 1.89 inside the build container, `CIBW_ENVIRONMENT`
  exporting cargo's PATH, `MACOSX_DEPLOYMENT_TARGET=10.12` for
  macOS, single dedicated sdist job with direct `pip install <tarball>`
  proof in a clean venv, per-platform smoke on the same native arch.
- `SUPPORT.md` — Python Binding section rewrites the platform matrix
  to use `Configured` wording until native smoke evidence is recorded.
- `RELEASING.md` — `## Python Wheel Artifacts` rewritten to describe
  the corrected mechanics (native runners, in-container Rust,
  `manylinux_2_28`, single sdist, `pip install <tarball>` proof).
- `plans/registry.md` — consolidated the duplicated `## Recently
  closed work` heading; added M003 active listing.

### Working dispatch

The implementor dispatched `release-python.yml` against the feature
branch (`feat/m003-python-corrective-qualification`) six times while
fixing the cibuildwheel mechanics. The working dispatch is:

| Field | Value |
| --- | --- |
| Workflow file | `.github/workflows/release-python.yml` |
| Dispatch ref | `feat/m003-python-corrective-qualification` |
| Dispatch SHA | `d40f1b3c8dbe92e70ed1f2da9b6764be6e5a5a84` (HEAD at dispatch time) |
| Run URL | https://github.com/eggstack/stegoeggo/actions/runs/36337194059 |
| Run ID | `36337194059` |
| Concurrency group | `release-python-feat/m003-python-corrective-qualification` |
| Conclusion | `success` |
| Trigger | `workflow_dispatch` |
| PyPI publication step | none |

Earlier dispatches (kept here as failure-and-recovery record):

| Run ID | SHA | Failure mode | Resolution |
| --- | --- | --- | --- |
| 36334929064 | `1f23a3c...` | Source-checkout git fetch rejected the short `1f23a3c` input | Branch-name input (`feat/m003-python-corrective-qualification`) |
| 36335031510 | `495a57f...` | `dtolnay/rust-toolchain@1.89` no longer accepts an explicit `toolchain` input | Drop the `with: toolchain: 1.89` and rely on the action's @rev tag |
| 36335256862 | `495a57f...` | Workflow syntax rejected by the runner; `cannot read SHA from upload-pack` for `git fetch --depth=1 origin <full-SHA>` | Branch-name dispatch (full SHAs from non-branch-tip fetches are rejected by GitHub's upload-pack) |
| 36335432712 | `495a57f...` | `cibuildwheel 2.22` does not accept `--build` on the command line; rejected the run before producing anything | Move the per-row selector to the `CIBW_BUILD` environment variable |
| 36335938902 | `2298322...` | `before-all`'s `echo "source ..." >> $GITHUB_ENV` failed because `$GITHUB_ENV` is not propagated into the cibuildwheel Linux container | Replace `$GITHUB_ENV` with `CIBW_ENVIRONMENT_LINUX: PATH="$HOME/.cargo/bin:$PATH"` (evaluated inside the container) |
| 36336431272 | `d269513...` | (1) `delocate-wheel` rejected the macOS wheel because maturin stamped `macosx_10_9` while the binary carries a 10.12 minimum; (2) Windows PowerShell parsed `--output-dir` as a unary operator (the cibuildwheel default shell there is `pwsh`); (3) the upload glob `*-x86_64*.whl` failed to match because `cp311-*` expanded to both manylinux *and* musllinux identifiers | (1) Pin `CIBW_ENVIRONMENT_MACOS: MACOSX_DEPLOYMENT_TARGET=10.12`. (2) Force `shell: bash` on every step. (3) Pin each matrix row to a single cibuildwheel identifier (e.g. `cp311-manylinux_x86_64`) so each run produces one wheel whose tag matches its native arch. |

The dispatch at run `36337194059` is the one whose evidence is
recorded below; its source SHA (`d40f1b3`) is the implementation
head recorded for M003 closure.

### Wheel artifacts produced

| OS | Architecture | Wheel filename | Runner (build + smoke) |
| --- | --- | --- | --- |
| Linux | x86_64 | `stegoeggo-0.4.2-cp311-abi3-manylinux_2_28_x86_64.whl` | `ubuntu-24.04` |
| Linux | aarch64 | `stegoeggo-0.4.2-cp311-abi3-manylinux_2_28_aarch64.whl` | `ubuntu-24.04-arm` |
| macOS | x86_64 | `stegoeggo-0.4.2-cp311-abi3-macosx_10_12_x86_64.whl` | `macos-15` (Intel runner resolved from `macos-15-intel`) |
| macOS | arm64 | `stegoeggo-0.4.2-cp311-abi3-macosx_11_0_arm64.whl` | `macos-14-arm64` (resolved from `macos-14`) |
| Windows | x86_64 | `stegoeggo-0.4.2-cp311-abi3-win_amd64.whl` | `windows-2022` |
| (sdist) | – | `stegoeggo-0.4.2.tar.gz` | `ubuntu-24.04` (built once) |

Native smoke outcomes (per-platform `pip install <wheel>` + import +
protect + verify, against the same-native-arch runner that produced
the wheel):

| OS / Arch | Smoke job name | Smoke conclusion | Smoke log line |
| --- | --- | --- | --- |
| Linux x86_64 | `smoke linux-x86_64` | success | `smoke ok` (job 108670691244) |
| Linux aarch64 | `smoke linux-aarch64` | success | `smoke ok` (job 108670691267) |
| macOS x86_64 | `smoke macos-x86_64` | success | `smoke ok` (job 108670691209) |
| macOS arm64 | `smoke macos-arm64` | success | `smoke ok` (job 108670691256) |
| Windows x86_64 | `smoke windows-x86_64` | success | `smoke ok` (job 108670691229) |

Sdist direct pip install smoke (one job, fresh venv, no source
checkout on `sys.path`):

| Smoke job | Conclusion | Smoke log line |
| --- | --- | --- |
| `sdist direct pip install smoke` (job 108670691219) | success | `sdist direct-pip-install smoke ok` |

Linux container Rust provisioning (the requirement is that Rust 1.89
is available *inside* the cibuildwheel Linux container, not only on
the outer GitHub runner):

| Linux row | Build-wheel log excerpt |
| --- | --- |
| `linux-x86_64` | `1.89-x86_64-unknown-linux-gnu installed - rustc 1.89.0 (29483883e 2025-08-04)` |
| `linux-aarch64` | `1.89-aarch64-unknown-linux-gnu installed - rustc 1.89.0 (29483883e 2025-08-04)` |

## 4. Verification executed

### Required Rust gate

```bash
$ cargo fmt --all -- --check
$ cargo clippy --workspace --all-targets --all-features -- -D warnings
$ cargo check -p stegoeggo --no-default-features
$ cargo test --workspace --exclude stegoeggo-fuzz --all-features
test result: ok. 627 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok.   2 passed
test result: ok.   9 passed
test result: ok.  14 passed
test result: ok.  14 passed
test result: ok.  35 passed
$ ./scripts/check-docs-contract.sh
Documentation contracts valid: 5 release targets
```

### Binding crate compile gate

```bash
$ cargo check --manifest-path bindings/python/Cargo.toml --locked
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.52s
```

### Local Python binding suite (after `maturin develop --release`)

```bash
$ cd bindings/python && maturin develop --release
    Finished `release` profile [optimized] target(s) in 17.95s
    Built wheel for abi3 Python ≥ 3.11 to .../stegoeggo-0.4.2-cp311-abi3-macosx_10_12_x86_64.whl
    Installed stegoeggo-0.4.2

$ python -m pytest tests/ -v
tests/test_errors.py ............                               [ 15%]
tests/test_files.py .........                                   [ 27%]
tests/test_parity.py .....                                      [ 33%]
tests/test_protect.py ..................                         [ 50%]
tests/test_request.py ...........                                [ 67%]
tests/test_smoke.py ......                                      [ 76%]
tests/test_verify.py ...........                                [100%]
================================ 71 passed in 0.71s ==============================
```

(M001/M002 had 57 tests; M003 adds 11 error tests and 9 file-helper
tests = 71 total.)

### Local sdist direct pip install proof

```bash
$ cd bindings/python && maturin sdist -o /tmp/m003-sdist
    Built source distribution to /tmp/m003-sdist/stegoeggo-0.4.2.tar.gz

$ python -m venv /tmp/m003-sdist-venv
$ /tmp/m003-sdist-venv/bin/python -m pip install --no-cache-dir /tmp/m003-sdist/stegoeggo-0.4.2.tar.gz
Processing /tmp/m003-sdist/stegoeggo-0.4.2.tar.gz
  …
Successfully installed stegoeggo-0.4.2
$ /tmp/m003-sdist-venv/bin/python -c "import stegoeggo; print(stegoeggo.__version__)"
0.4.2
$ /tmp/m003-sdist-venv/bin/python - <<'PY'
import stegoeggo
notice = stegoeggo.RightsNotice().with_copyright_holder('sdist-truth-test')
request = stegoeggo.ProtectionRequest.metadata_only(
    notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
).with_seed(42).with_timestamp_override('2026-01-01T00:00:00Z')
data = open('canonical_independent.png', 'rb').read()
out = stegoeggo.protect(data, request)
rep = stegoeggo.verify(out)
assert rep.rights_found
assert rep.copyright_holder == 'sdist-truth-test'
print('sdist-direct-pip-install protect+verify smoke ok')
PY
sdist-direct-pip-install protect+verify smoke ok
```

### Remote manual dispatch

The manually-dispatched `release-python.yml` is run `36337194059`
(see "Working dispatch"). Every required job succeeds; all 12
jobs are reported as `success` in `gh run view --json jobs`:

```
build sdist                         completed success
build macos-x86_64 wheel            completed success
build windows-x86_64 wheel          completed success
build linux-aarch64 wheel           completed success
build linux-x86_64 wheel            completed success
build macos-arm64 wheel             completed success
sdist direct pip install smoke      completed success
smoke macos-x86_64                  completed success
smoke windows-x86_64                completed success
smoke linux-x86_64                  completed success
smoke macos-arm64                   completed success
smoke linux-aarch64                 completed success
```

## 5. Invariant review

1. Python protection is a thin projection of `ProtectionRequest` +
   `process_request_bytes*`. — Confirmed: `bindings/python/src/lib.rs`
   `protect`/`protect_with_warnings`/`protect_with_report` route only
   through those entry points; the existing test suite still passes
   unchanged.
2. Python verification routes through `verify_image_bytes_report`. —
   `verify` in `bindings/python/src/lib.rs` calls
   `verify_image_bytes_report` (or its `_with_limits` sibling); tests
   unchanged.
3. Encoded bytes are the binding boundary when metadata matters. —
   The whole binding is bytes-only; `protect_file`/`verify_file` are
   the only file-touching surface, and they sit on top of the byte
   APIs.
4. Root and carrier remain free of PyO3/maturin deps. — `cargo check
   -p stegoeggo --no-default-features` is in the green `./scripts/check.sh`
   log; `bindings/python/Cargo.toml` already isolated the binding.
5. Required `Cargo.toml` excludes / empty `[workspace]` isolation. —
   Unchanged; the binding crate still has `[workspace]` and
   `publish = false`.
6. `./scripts/check.sh` is Python-free. — Unchanged; the new
   `python-binding.yml` is a separate workflow and does not modify the
   standard CI gate.
7. Panic profile. — `bindings/python/Cargo.toml` keeps
   `panic = "unwind"`; root release profile keeps `panic = "abort"`
   (`commands/check.sh` is green).
8. Exception class hierarchy preserved. — All
   `tests/test_errors.py::test_exceptions_are_subclasses_*` would have
   inherited from `test_smoke.py::test_exceptions_are_subclasses_of_stegoeggo_error`;
   that test still passes against the new structured-attribute
   extensions.
9. No secret key leakage through exception attributes. — See
   `test_resource_limit_error_does_not_leak_secret_keys` and the
   existing `*does_not_leak_mac_key*` tests, all passing.
10. Native smoke per platform. — Every wheel row has its own smoke
    job; see the smoke evidence table above.
11. One sdist per dispatch. — `build_sdist` runs once; the
    `sdist_install_smoke` job downloads the sdist artifact and
    `pip install`s it into a fresh venv.
12. No PyPI publication. — Workflow has no `twine upload`,
    `pypi-publish` action, or `PYPI_TOKEN` env.

## 6. Failure and recovery review

- **Source-checkout SHA dispatch rejection** — GitHub's upload-pack
  rejects `git fetch --depth=1 origin <full-SHA>` for arbitrary SHAs.
  Mitigation: dispatch against the branch name; the implementor's
  recorded SHA is the HEAD of that branch at dispatch time.
- **cibuildwheel 2.22 `--build` flag absent** — The original
  `release-python.yml` used `--build "${{ matrix.cibw_build }}"` and
  `--archs "${{ matrix.cibw_archs }}"`. 2.22 removed `--build`.
  Mitigation: switch to the `CIBW_BUILD` env var and to a single,
  pinned identifier per matrix row.
- **`CIBW_BEFORE_ALL` cannot reach `$GITHUB_ENV`** — The GitHub
  Actions `GITHUB_ENV` file is on the runner, not inside the
  cibuildwheel Linux container. Mitigation: use `CIBW_ENVIRONMENT_LINUX`
  for PATH propagation, evaluated inside the container.
- **`delocate-wheel` rejected macOS wheel (10.12 vs 10.9 stamp)** —
  Maturin defaults to macOS 10.9 while the extension module's
  `MACOSX_DEPLOYMENT_TARGET` is 10.12. Mitigation:
  `CIBW_ENVIRONMENT_MACOS: MACOSX_DEPLOYMENT_TARGET=10.12`.
- **Windows PowerShell parsed `--output-dir` as a unary operator** —
  The cibuildwheel Windows default shell is `pwsh`. Mitigation:
  `shell: bash` on every Windows step so `bash` parses the argv.
- **Upload glob `*-x86_64*.whl` did not match** — Because `cp311-*`
  expanded to manylinux *and* musllinux, the dist directory held two
  wheels per Linux row; the platform-prefix glob still should have
  matched both, but pinning each row to one identifier made the
  per-platform artifact deterministic and the smoke selection simple.

## 7. Migration and compatibility review

M003 changes the Python exception *attributes* (additive) and the
file helper *signature* (additive `output_path` keyword). The existing
two-argument `protect_file(path, request)` keeps working and the
existing exception subclasses keep their names and inheritance.
Encoders/verifiers depending on `repr(exception)` to carry
counts/limits now get *more* detail, not less. Rust, CLI, and carrier
semantics are unchanged.

## 8. Security review

- Secret MAC/HMAC bytes are never serialized through exception
  repr, dict, or attributes — covered by the no-leak tests in
  `test_errors.py` plus the pre-existing
  `test_protect_with_report_does_not_leak_mac_key`/
  `test_verify_does_not_leak_mac_key`.
- `release-python.yml` continues to ship with no PyPI credentials; the
  maintainer-only `twine upload` step lives in `RELEASING.md` and is
  out of this milestone's automation.
- The `python-binding.yml` workflow runs only `maturin develop` and
  `pytest`; it does not publish anything.

## 9. Documentation and operations

- `bindings/python/README.md` — Updated enum examples; documents
  optional `output_path`, the failure-safe in-place semantics, and
  the structured-exception contract. Still labelled experimental and
  not on PyPI; the only install path is still local source build or
  directly download the wheel/sdist artifact attached to a manually
  dispatched workflow run.
- `SUPPORT.md` — Python Binding platform matrix uses
  "Configured (qualified when smoke evidence recorded)" wording
  until each row's smoke is recorded; the corrected runners, the
  `manylinux_2_28` floor, and the one-sdist/direct-pip-install
  mechanics are documented.
- `RELEASING.md` — `## Python Wheel Artifacts` rewritten to describe
  the corrected mechanics and to mention the `pip install <tarball>`
  proof step.
- `plans/registry.md` — Consolidated the duplicated
  `## Recently closed work` heading; listed M003 active under the
  language-bindings subsystem.

## 10. Unresolved findings

None at the level required by the plan acceptance criteria. Carried-
forward items:

- `VerificationBudgetExceeded` is defined in `Error` but not raised
  anywhere in the production path yet (it is only constructed in
  `error.rs` tests). The new `map_error` projects it as
  `ResourceLimitError` with `resource="verification_budget"`,
  `kind`, `count`, `limit`. End-to-end coverage for that variant is
  intentionally deferred per WP1 ("If a structured variant cannot be
  reached through the public binding API, unit-test the projection
  helper in Rust and document why an end-to-end Python trigger is
  unavailable."). The variant is documented in `_native.pyi`.
- Python free-threaded (`cp311t-abi3`) and PyPy qualification remain
  deferred per the language-bindings subsystem roadmap.
- The python-binding.yml push/PR CI is committed but is only
  schedulable once the file is on the default branch. On the
  `feat/m003-python-corrective-qualification` branch, `gh workflow
  run python-binding.yml` returns the expected 404 because GitHub
  only allows dispatching workflows that exist on `main`. The
  corresponding signal will fire after merge.

## 11. Roadmap disposition

M003 is **closed**. M002's qualification condition is satisfied: this
closure records native install + protect + verify smoke for all
five wheel platforms and a successful sdist direct `pip install`
proof. M002's status changes from "conditionally closed" to
"closed" at this closure (see Registry updates below).

M004 (Node binding) is now dependency-ready per the subsystem
roadmap. It can be written and registered against
`language-bindings-roadmap.md#m004--nodejs-binding` once a maintainer
chooses to expand the subsystem.

## 12. Registry updates

- `plans/registry.md`:
  - Language-bindings subsystem row "Current milestone" moves
    `M003 ready` → `M003 closed`. Reference: this closure record.
  - The "Dependency-ready implementation plans" table entry for
    M003 moves from `ready` to `closed`.
  - The "Active closure work" section is empty (closed).
  - The "Blocked work" row for "M004 Node binding" has its blocker
    name changed from "M003 Python corrective qualification must
    close and reconcile M002's remaining qualification condition
    before Node planning/implementation is ready" to a pointer at
    this closure record.
  - The "Recently closed work" list adds the M003 entry; the
    duplicated heading is consolidated.
- `plans/subsystems/language-bindings-roadmap.md`:
  - Milestone status table: M002 row changes from `conditionally
    closed` to `closed`; M003 row changes from `ready` to `closed`.
    Both rows gain their respective closure-record links.
- `plans/closure/language-bindings/001-status.md` and
  `plans/closure/language-bindings/002-status.md`: left immutable.
  The M002 disposition section above is the audit trail for the
  condition lift; no edit is made to the existing record.
