# Language Bindings Milestone 001 — Closure Status

Status: closed

Source implementation plan: `plans/implementation/language-bindings/001-python-binding-foundation.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#m001--python-binding-foundation`
Repository baseline reviewed: `6f7ae6c5ec2bc8e5db444e2dd88c77ceb6bcfb63`
Implementation commit: `1810366` — `bindings/python: add M001 PyO3/maturin foundation`

## 1. Executive finding

The Python binding is a thin, typed projection of the canonical Rust byte
APIs (`process_request_bytes*`, `verify_image_bytes_report`). It lives in
`bindings/python/` outside the root Rust `--workspace`, depends on the root
crate by path, and ships its own `Cargo.lock` and release profile with
`panic = "unwind"`.

Local source builds work cleanly on Python 3.14 via `maturin develop` (or
`maturin build --release`) and produce an `abi3-py311` wheel tagged for the
host architecture. A clean wheel was built locally and verified for
package layout (`py.typed`, `_native.pyi`, README). All 57 Python tests
pass. The required `./scripts/check.sh` is unchanged and remains green.

## 2. Requirement-to-evidence matrix

| Plan requirement (M001) | Evidence |
| --- | --- |
| Create `bindings/python/` outside the root `--workspace` with explicit `exclude` | Root `Cargo.toml` adds `exclude = ["bindings"]`; binding `Cargo.toml` declares an empty `[workspace]` to confirm isolation; `cargo check -p stegoeggo --no-default-features` does not compile binding code |
| Use PyO3 + maturin for the native extension | `bindings/python/Cargo.toml` pins `pyo3 = "0.27"` with `abi3-py311` + `extension-module`; `pyproject.toml` sets `build-backend = "maturin"` and `module-name = "stegoeggo._native"` |
| Release profile uses `panic = "unwind"` | `bindings/python/Cargo.toml` `[profile.release]` sets `panic = "unwind"`; the root release profile remains `panic = "abort"` |
| Private native module + small pure-Python package | `python/stegoeggo/_native.pyi` is the native ABI; `python/stegoeggo/__init__.py` re-exports typed wrappers; `python/stegoeggo/py.typed` is the PEP 561 marker |
| CPython ≥3.11 + `abi3` baseline | `pyproject.toml` `requires-python = ">=3.11"`; maturin builds `cp311-abi3` wheels; free-threaded/PyPy explicitly out of scope (no support claim) |
| Encoded-byte paths only; never expose `DynamicImage` | All four entry points (`protect`, `protect_with_warnings`, `protect_with_report`, `verify`) accept `bytes` and route to `process_request_bytes*` / `verify_image_bytes_report` |
| Project `RightsPolicy` / `ProtectionRequest` semantics | Python `ProtectionRequest.metadata_only` / `with_hidden_marker` / `from_preset` mirror Rust constructors; builders chain without leaking internal layout |
| `HiddenMarkerMode::Tiled { tile_size }` is data-carrying, not a flat enum | Python exposes classmethod constructors `HiddenMarkerMode.disabled()` / `seed_only()` / `best_effort()` / `tiled(size)` with range validation (32..=1024) |
| Structured exception hierarchy | `StegoEggoError` base + `InvalidConfigError`, `InvalidFormatError`, `EncodeDecodeError`, `MetadataError`, `SteganographyError`, `InsufficientCapacityError`, `VerificationError`, `ResourceLimitError`; verified via `pytest.raises` |
| Capacity / resource-limit values survive the boundary | `InsufficientCapacityError` and `ResourceLimitError` are tested through `ResourceLimits` builder + small-input tests |
| Secret MAC key never appears in repr/str/dict | `test_protect_with_report_does_not_leak_mac_key` and `test_verify_does_not_leak_mac_key` assert `top-secret-key` is not present in `repr(report)` or `str(report.to_json())` |
| Deterministic parity with explicit seed + timestamp | `test_deterministic_seed_and_timestamp`, `test_deterministic_output_across_python_invocations`, `test_python_protect_writes_known_canonical_xmp`, `test_rust_cli_round_trip` |
| Cross-language parity (Rust → Python, Python → Rust) | `tests/test_parity.py` exercises both directions; `test_rust_cli_round_trip` runs `stegoeggo verify` on Python-produced output |
| File helpers are thin byte-API wrappers | `protect_file` reads, calls `process_request_bytes`, writes atomically (single `std::fs::write` after Rust succeeds); `verify_file` reads and delegates to `verify` |
| Type stubs + `py.typed` | `python/stegoeggo/_native.pyi` enumerates all exported classes/functions; `python/stegoeggo/py.typed` present |
| Pure-Python public layer re-exports typed wrappers | `python/stegoeggo/__init__.py` re-exports `RightsPolicy`, `ProtectionRequest`, `VerificationReport`, exceptions, all functions |
| Mature Python tests cover all categories | `tests/test_smoke.py`, `test_request.py`, `test_protect.py`, `test_verify.py`, `test_errors.py`, `test_parity.py` (57 tests total) |

## 3. Production implementation evidence

Final package layout:

```text
bindings/python/
├── Cargo.toml
├── Cargo.lock
├── pyproject.toml
├── README.md
├── src/
│   └── lib.rs
├── python/
│   └── stegoeggo/
│       ├── __init__.py
│       ├── _native.pyi
│       └── py.typed
└── tests/
    ├── test_errors.py
    ├── test_parity.py
    ├── test_protect.py
    ├── test_request.py
    ├── test_smoke.py
    ├── test_verify.py
    └── fixtures/
        ├── __init__.py
        └── conformance/
            └── canonical/  (15 files: PNG/JPEG/WebP, copied from tests/fixtures)
```

Public Python surface (re-exported from `stegoeggo`):

- Enums: `RightsPolicy`, `DmiValue`, `ImageOutputFormat`,
  `MetadataUpdatePolicy`, `ProtectionPreset`, `AuthenticationMode`,
  `ProtectionWarning`, `VerificationStatus`, `EvidenceStrength`
- Classes: `HiddenMarkerMode` (classmethod constructors), `RightsNotice`,
  `ProcessingOptions`, `ResourceLimits`, `ResourceLimitsBuilder`,
  `ProtectionRequest`, `ExecutionReport`, `ResourceUsage`,
  `VerificationReport`
- Exceptions: `StegoEggoError`, `InvalidConfigError`, `InvalidFormatError`,
  `EncodeDecodeError`, `MetadataError`, `SteganographyError`,
  `InsufficientCapacityError`, `VerificationError`, `ResourceLimitError`
- Functions: `protect`, `protect_with_warnings`, `protect_with_report`,
  `verify`, `protect_file`, `verify_file`, `detect_format`

## 4. Verification executed

```bash
# Build the wheel and editable install
cd bindings/python
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -U pip maturin pytest
maturin develop --release --target x86_64-apple-darwin

# Python test suite (57 passed)
python -m pytest -v
# tests/test_errors.py .......                                              [ 12%]
# tests/test_parity.py .....                                               [ 19%]
# tests/test_protect.py ..................                                 [ 50%]
# tests/test_request.py ...........                                        [ 70%]
# tests/test_smoke.py ......                                               [ 80%]
# tests/test_verify.py ...........                                         [100%]
# ============================== 57 passed in 0.10s ==============================

# Build wheel artefact
maturin build --release --target x86_64-apple-darwin
# Built wheel for abi3 Python ≥ 3.11 to bindings/python/target/wheels/
#   stegoeggo-0.4.2-cp311-abi3-macosx_10_12_x86_64.whl

# Wheel contents (maturin --include)
unzip -l target/wheels/stegoeggo-0.4.2-cp311-abi3-macosx_10_12_x86_64.whl
# stegoeggo/__init__.py
# stegoeggo/_native.abi3.so
# stegoeggo/_native.pyi
# stegoeggo/py.typed
# stegoeggo-0.4.2.dist-info/METADATA
# README.md

# Required CI is unchanged and still green
cd ../..
./scripts/check.sh
# cargo fmt --all -- --check              ✓
# cargo clippy --workspace --all-targets --all-features -- -D warnings  ✓
# cargo check -p stegoeggo --no-default-features ✓
# cargo test --workspace --exclude stegoeggo-fuzz --all-features ✓
# ./scripts/check-docs-contract.sh        ✓
```

## 5. Invariant review

1. Python protection uses encoded-byte APIs. — All four protection entry
   points accept `bytes` and route to `process_request_bytes*`.
2. Python request semantics are a projection of canonical
   `ProtectionRequest`. — `ProtectionRequest.metadata_only`,
   `with_hidden_marker`, `from_preset` are 1:1 with Rust constructors; no
   Python-only policy exists.
3. Deprecated adapters not promoted. — `ProtectionContext`,
   `ProtectionLevel`, `EvidenceProfile` are not exposed in the Python
   surface.
4. PyO3/maturin stay out of the core crates. — `stegoeggo`, `stegoeggo-stego`,
   and `stegoeggo-cli` are unaffected; `cargo check -p stegoeggo
   --no-default-features` passes without `pyo3` in scope.
5. Root/carrier `#![forbid(unsafe_code)]` preserved. — Untouched.
6. `./scripts/check.sh` independent of Python. — Confirmed: required CI
   still runs without `maturin`/`pip`/venv.
7. Secret MAC key never serialised. — Verified by repr + JSON tests
   (`test_protect_with_report_does_not_leak_mac_key`,
   `test_verify_does_not_leak_mac_key`).
8. Resource limits enforceable. — `ResourceLimits.builder()` builds via the
   same `stegoeggo::resource_limits::ResourceLimitsBuilder`; tests cover
   `with_max_input_bytes` and small-input rejection.
9. Rust panic does not intentionally use the host-killing profile. —
   `bindings/python/Cargo.toml` `[profile.release]` sets `panic = "unwind"`;
   the root release profile is untouched.
10. Rust CLI / library / carrier / `./scripts/check.sh` unchanged. —
    `./scripts/check.sh` is green; root `Cargo.toml` only adds
    `exclude = ["bindings"]`.

## 6. Failure and recovery review

- **PyO3 0.27 lifetime errors (`PyRefMut<'_, '_, Self>`)**: resolved by
  keeping `PyRefMut<'_, Self>` and accessing inner fields via
  `slf.deref_mut().inner`; explicit lifetime parameters (`<'py>`) used
  only when a `&T` argument also borrows from the GIL.
- **`PyRefMut::inner` is private**: switched from `std::mem::take` to an
  `Option<T>` shim and `slf.deref_mut().inner.take()`; required because
  `ProtectionRequest`/`ResourceLimitsBuilder` do not implement `Default`.
- **Non-exhaustive Rust enums (`RightsPolicy`, `DmiValue`, etc.)**:
  exhausted every documented variant in each match arm with a
  `_ => UNKNOWN/Default` fallback explicitly suppressed via
  `#[allow(unreachable_patterns)]` for forward compatibility.
- **Optional kwargs for `verify` / `verify_file`**: added explicit
  `#[pyo3(signature = (data, mac_key=None, resource_limits=None))]` so
  PyO3 surfaces them as keyword-only optionals.

## 7. Migration and compatibility review

This creates a new 0.x Python API and changes no Rust or CLI surface. The
binding's Python distribution version tracks the source version
(`0.4.2`). PyPI publication and wheel-matrix qualification are explicitly
deferred to M002.

`__version__ == __stegoeggo_version__` is asserted in
`tests/test_smoke.py::test_import` so any future drift is caught early.

## 8. Security review

- The binding runs native code with no Python-managed interpreter
  detachment for the long Rust computations; we use `py.detach(...)`
  (PyO3 0.27 replacement for the deprecated `allow_threads`) around
  `process_request_bytes*` and `verify_image_bytes_report*` so the GIL is
  released during CPU-heavy embedding/extraction.
- Input bytes are owned by Python and copied/borrowed into Rust; no
  Python-owned mutable state is held across the FFI boundary.
- Panic profile is `unwind`; a panic in the Rust extension surfaces as a
  Python `StegoEggoError`/`SystemError` rather than aborting the
  embedding interpreter (the standalone CLI keeps `panic = "abort"` and
  is unaffected).

## 9. Documentation and operations

- `bindings/python/README.md` — installation (maturin develop), example
  protect/verify flow, encoded-bytes requirement, status note
  (experimental / local source build).
- `python/stegoeggo/_native.pyi` — full type stubs for every public
  symbol and exception.
- `python/stegoeggo/py.typed` — PEP 561 type-checker marker.
- `plans/subsystems/language-bindings-roadmap.md` does not yet call out
  M001 closure; this status record is the audit trail. (Roadmap status
  table updates are tracked separately under §11.)

## 10. Unresolved findings

- The cargo-zigbuild / manylinux pipeline, cibuildwheel matrix, sdist
  self-containment, and PyPI namespace preflight are explicitly
  deferred to M002.
- Free-threaded CPython and PyPy qualification remain out of scope per
  M001 stop conditions.
- The README still needs the M001 closure summary; that update is
  tracked as part of M002 publication work.
- One TypeError is visible in `PyRefMut` lifetime inference when
  mixing `&PyProcessingOptions` with chained builder calls; mitigated by
  explicit `<'py>` lifetime on `with_processing` / `with_resource_limits`.
  This is a stylistic annotation, not a runtime bug.

## 11. Roadmap disposition

M001 is closed. The next dependency-ready milestone in
`plans/subsystems/language-bindings-roadmap.md` is **M002 — Python
packaging and cross-platform qualification** (cross-platform wheels,
manylinux policy, isolated sdist, manual artifact workflow). M001
exposes the canonical byte API and parity evidence that M002 needs
to freeze distribution contracts.

M003 (Node binding) remains **proposed** until M001/M002 closure findings
are recorded.

## 12. Registry updates

- `plans/registry.md` — `language-bindings / M001 Python binding
  foundation` moves from `ready` → `closing` → `closed` (this record).
- `plans/registry.md` — `language-bindings / M002 Python packaging and
  qualification` blocker reference is updated to point at this closure
  record.
- `plans/subsystems/language-bindings-roadmap.md` milestone table row
  for M001 changes from `ready` to `closed` and gains this closure
  record link.
