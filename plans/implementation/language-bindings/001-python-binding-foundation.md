# Language Bindings Milestone 001 — Python Binding Foundation

Status: ready for handoff
Repository baseline: `fe86d7361ff9b4ac725e1b05ef915ac391803868`
Source roadmap: `plans/subsystems/language-bindings-roadmap.md#7-milestones`
Long-term requirements: `plans/000-long-term-specification.md#2-canonical-api-invariants`, `plans/000-long-term-specification.md#3-execution-invariants`, `plans/000-long-term-specification.md#5-release-invariants`
Applicable ADRs: `plans/adrs/ADR-0001-canonical-protection-request.md`, `plans/adrs/ADR-0003-byte-vs-pixel-paths.md`, `plans/adrs/ADR-0005-foreign-language-bindings.md`
Primary class: capability

## 1. Objective

Add the first foreign-language frontend: a Python package backed directly by
the canonical Rust `stegoeggo` library using PyO3 and maturin.

The milestone closes when a local Python installation can protect and verify
PNG/JPEG/WebP encoded bytes through a typed, Pythonic API; its outputs and
reports are demonstrably interoperable with the Rust API; structured failure
information survives the boundary; and the existing Rust workspace/CLI
contracts remain unchanged.

This milestone deliberately stops before cross-platform wheel release
qualification or PyPI publication. Those belong to M002.

## 2. Why this milestone is ready

The hard dependencies are already closed:

- canonical `ProtectionRequest` + `RightsPolicy`;
- canonical byte protection functions;
- canonical `VerificationReport`;
- stable PNG/JPEG/WebP container behavior;
- generic carrier split;
- known-answer and cross-format test fixtures;
- stable error/resource-limit models.

No algorithmic or container refactor is required to begin. The binding is a
leaf consumer.

## 3. Current implementation evidence

At the baseline:

- root `Cargo.toml` is StegoEggo 0.4.2, Rust 1.89;
- `process_request_bytes`,
  `process_request_bytes_with_warnings`, and
  `process_request_bytes_with_report` are canonical encoded-byte entry
  points;
- `verify_image_bytes_report` returns the canonical structured report;
- `RightsNotice` and `ProtectionRequest` provide getters/builders without
  requiring direct field access;
- `VerificationReport` and child report types provide getters and serde
  support;
- root `Error` preserves structured capacity/resource-limit values where
  those distinctions matter;
- the root release profile sets `panic = "abort"`;
- required CI invokes Rust `--workspace` commands through
  `./scripts/check.sh`;
- there is no `bindings/` tree today.

## 4. Invariants that must not regress

1. Python protection uses encoded-byte APIs. Do not route rights metadata
   through `DynamicImage`.
2. Python request semantics are a projection of canonical
   `ProtectionRequest`; do not add Python-only protection semantics.
3. Do not expose deprecated `ProtectionContext`, `ProtectionLevel`, or
   `EvidenceProfile` as new Python API.
4. Do not move PyO3/maturin dependencies into `stegoeggo`,
   `stegoeggo-stego`, or `stegoeggo-cli`.
5. Keep root/carrier `#![forbid(unsafe_code)]`.
6. Do not silently add Python prerequisites to the required Rust
   `./scripts/check.sh` path.
7. Do not serialize/repr secret MAC key material.
8. Resource limits must remain enforceable from Python.
9. A Rust panic in the extension must not intentionally use the root
   `panic = "abort"` host-killing profile.
10. Existing Rust tests/examples/CLI behavior must remain unchanged.

## 5. Scope

### In

- Create `bindings/python/` as an isolated Cargo/Python package.
- Use PyO3 for the native extension and maturin for local builds.
- Keep the package outside the root Rust `--workspace` membership, with an
  explicit root workspace exclusion if Cargo requires it.
- Give the binding package a release profile with `panic = "unwind"`.
- Use a private native module, preferably `stegoeggo._native`, with a small
  pure-Python public package layer.
- Initial normal-CPython support baseline: Python >=3.11.
- Initial stable-ABI target: normal CPython `abi3`; free-threaded CPython and
  PyPy are explicitly deferred.
- Expose canonical rights/protection/verification concepts.
- Typed Python stubs and `py.typed`.
- Structured Python exception hierarchy.
- Pure-Python path/file convenience helpers that call byte APIs.
- Python-native tests plus Rust/Python interoperability fixtures.
- Documentation sufficient for local editable/source installation and basic
  use.

### Explicitly out

- PyPI publication.
- Cross-platform wheel matrix/release workflow.
- Free-threaded CPython qualification.
- PyPy.
- Asyncio-native API.
- Rayon batch API.
- Signatures/detached-manifest binding.
- ISCC/conformance binding.
- Generic `stegoeggo-stego` API.
- Node.js or C ABI implementation.
- Required-CI expansion.
- Refactoring root APIs solely to make Python wrappers shorter, unless a
  genuinely missing stable getter prevents correct projection.

## 6. Required production changes

### Package layout

Create a layout equivalent to:

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
    ├── test_protect.py
    ├── test_verify.py
    ├── test_errors.py
    └── test_parity.py
```

Exact Python module splitting may differ if it improves maintainability.

The Rust package name should be unambiguous (for example
`stegoeggo-python`) and `publish = false` for crates.io. The Python
distribution/import name is `stegoeggo`, subject to package-registry name
confirmation before M002 publication work.

### Core Python surface

Provide a small public API centered on:

- `RightsPolicy`;
- `ProtectionPreset`;
- `RightsNotice`;
- `ProtectionRequest`;
- `ProtectionWarning`;
- `ExecutionReport`;
- `VerificationReport`;
- `protect(data, request) -> bytes`;
- `protect_with_warnings(data, request)`;
- `protect_with_report(data, request)`;
- `verify(data, *, mac_key=None, resource_limits=None) -> VerificationReport`;
- pure-Python `protect_file(...)` and `verify_file(...)` convenience
  functions.

The implementation may expose additional canonical enums/value objects needed
to construct `ProcessingOptions`, `ProtectionChannels`,
`HiddenMarkerMode`, `AuthenticationMode`, `MetadataUpdatePolicy`,
`ImageOutputFormat`, and `ResourceLimits`. Prefer Python constructors and
properties over mechanically mirroring every fluent Rust builder.

`HiddenMarkerMode::Tiled { tile_size }` is data-carrying and should be
represented by constructors/factories or a value object, not forced into a
flat Python Enum.

### Runtime boundary

- Accept Python bytes and buffer-compatible input where PyO3 can do so safely.
- Copy/own input before detaching from Python if required for soundness.
- Release/detach from the Python interpreter around CPU-heavy Rust
  processing/verification.
- Convert Rust output into Python `bytes`.
- Do not optimize to borrowed zero-copy buffers at the cost of lifetime or
  interpreter-safety complexity in M001.

### Reports and warnings

Expose typed read-only properties for report fields. Do not make Rust private
field layout part of Python compatibility.

`VerificationReport.to_dict()`/`to_json()` may use the existing stable
serialized report model. Do not add broad serialization derives to
`ProtectionRequest`, `RightsNotice`, or secret-bearing types merely for
binding convenience.

### Exceptions

Create a stable root exception such as `StegoEggoError` and structured
subclasses sufficient to distinguish at least:

- configuration errors;
- decode/encode/format errors;
- metadata errors;
- steganography errors;
- insufficient capacity, with `required` and `available` attributes;
- verification/integrity errors;
- cryptographic errors;
- resource-limit errors, preserving available structured counts/limits.

Unknown future non-exhaustive Rust error variants must degrade to a safe
generic StegoEggo exception, never panic in the translator.

### Secret handling

`ProtectionRequest` wrappers containing MAC key material must not include key
bytes in `repr`, conversion-to-dict helpers, tracebacks produced by explicit
wrapper formatting, or default debug presentation.

## 7. Ordered work packages

### WP1 — Package isolation and build skeleton

Intent: prove PyO3 can consume the current root crate without changing core
semantics or required CI.

Changes:

- establish `bindings/python`;
- configure path dependency to `stegoeggo` with minimal/default-off core
  features;
- ensure root workspace commands do not automatically include the Python
  package;
- configure Python metadata, maturin backend, Python >=3.11, private native
  module, type marker;
- configure an unwind release profile for the extension;
- add minimal import/version smoke test.

Acceptance evidence:

- `maturin develop` (or `uv run maturin develop`) succeeds in a clean local
  venv;
- `python -c "import stegoeggo"` succeeds;
- `./scripts/check.sh` still requires no Python binding setup and stays green.

### WP2 — Canonical request/value projection

Intent: make Python construct the same semantic request Rust constructs.

Changes:

- wrappers/enums for policy/preset/notice/request and needed option types;
- explicit validation through Rust request resolution where possible;
- Pythonic constructors/properties;
- resource-limit builder/value projection;
- repr/redaction policy.

Acceptance evidence:

- metadata-only, hidden-marker, authenticated, output-format, deterministic
  seed/timestamp, tiled-mode, and invalid-config request tests;
- canonical values round-trip through properties without semantic loss.

### WP3 — Protection operations

Intent: expose byte protection without CLI/filesystem coupling.

Changes:

- `protect`, `protect_with_warnings`, `protect_with_report`;
- interpreter detachment for native work;
- output bytes conversion;
- warning/report projections.

Acceptance evidence:

- PNG/JPEG/WebP metadata-only protection succeeds;
- hidden-marker protection succeeds on representative fixtures;
- deterministic calls with fixed seed + timestamp match Rust bytes exactly;
- warning/report fields match Rust expectations.

### WP4 — Verification operation

Intent: expose the canonical rich verification model.

Changes:

- `verify` delegates to `verify_image_bytes_report` or its limits-aware
  canonical variant;
- typed child report views/projections;
- HMAC key and resource-limit input;
- `to_dict`/`to_json` only where grounded in the existing report schema.

Acceptance evidence:

- protected and unprotected images;
- metadata-only summary behavior;
- valid/wrong/missing HMAC key cases;
- malformed/truncated/resource-limited inputs;
- Python report matches Rust canonical report for shared fixtures.

### WP5 — Structured errors

Intent: preserve actionable failure information.

Changes:

- exception hierarchy;
- conversion from root `Error`;
- structured capacity/resource fields;
- generic fallback for future non-exhaustive variants.

Acceptance evidence:

- each mapped error family has a targeted test;
- capacity and resource-limit attributes are programmatically assertable;
- no test relies only on matching error strings.

### WP6 — Pure-Python ergonomics and typing

Intent: keep filesystem and Python ergonomics outside Rust FFI.

Changes:

- `protect_file`/`verify_file` using `pathlib.Path`-compatible values;
- stubs/typing;
- basic package README examples.

Acceptance evidence:

- file helpers are thin wrappers around byte functions;
- type checker/stub validation selected by the implementation passes;
- examples perform a protect -> write -> verify flow.

### WP7 — Cross-language parity harness

Intent: prove this is a binding, not a divergent implementation.

Changes:

- reuse repository fixtures or copy only the minimal stable test corpus into a
  binding test-fixture path with provenance;
- add deterministic expected vectors where appropriate;
- add Rust-generated/Python-verified and Python-generated/Rust-verified tests
  or a shared harness invoking existing Rust fixture expectations.

Acceptance evidence:

- both interoperability directions pass;
- PNG/JPEG/WebP included;
- metadata-only + stego cases included;
- explicit seed + timestamp gives byte-identical result where Rust currently
  guarantees reproducibility.

## 8. Failure, cancellation, restart, contention semantics

The binding has no persistent mutable service state. Individual operations are
all-or-error and return owned bytes/results.

Python exceptions must be raised only after Rust has returned an error or a
panic has been safely translated by the binding runtime. Do not leave partially
written files in pure-Python file helpers: write to a temporary sibling and
replace only after successful protection if an output helper owns the write.

If interpreter detachment cannot be made sound with the selected input
borrowing strategy, fall back to an owned input copy rather than retaining a
borrowed Python view across detached execution.

If a future PyO3 version requires a Rust MSRV above the project binding policy,
stop and report rather than changing the root MSRV.

## 9. Compatibility and migration

This creates a new 0.x Python API and changes no Rust/CLI compatibility
surface. Python terminology follows canonical Rust concepts but need not match
Rust method names mechanically.

The binding package source version must be visible at runtime
(`stegoeggo.__version__` or equivalent) and match the StegoEggo source
version it wraps.

Do not claim PyPI package stability or supported wheel targets until M002
closes.

## 10. Required tests

At minimum:

- import/version smoke;
- enum/value construction;
- notice/request construction;
- metadata-only protect PNG/JPEG/WebP;
- hidden-marker protect representative PNG/JPEG/WebP;
- deterministic seed + timestamp parity;
- protection warnings and execution report;
- unprotected verify;
- protected verify;
- metadata-only verify summary;
- HMAC correct/wrong/missing key in scope;
- malformed/truncated/unsupported input;
- insufficient capacity structured exception;
- resource-limit structured exception;
- file-helper success and failure/no-partial-output behavior;
- repr/secret-redaction test;
- Rust/Python bidirectional interoperability.

Tests must be deterministic and must not require network access.

## 11. Required verification commands

Record exact successful commands in the closure record. Minimum expected set:

```bash
./scripts/check.sh

cd bindings/python
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -U pip maturin
maturin develop
python -m pytest
python -m pip install .
python -c "import stegoeggo; print(stegoeggo.__version__)"
```

Using `uv` is acceptable if the package documents an equivalent standard
pip/maturin path and closure records the exact commands actually run.

Also run a release-profile local build to prove `panic = "unwind"` is the
extension profile rather than the root CLI abort profile.

Do not add these commands to required `./scripts/check.sh` in this
milestone.

## 12. Documentation updates

Add:

- `bindings/python/README.md`;
- a concise Python section/link in root `README.md` only after the local
  API is functional;
- support status in `SUPPORT.md` labeled experimental/local until M002;
- architecture documentation for the foreign binding boundary, or update the
  architecture index to point to the binding README/ADR.

Document prominently that encoded bytes are required to preserve/inject
container metadata.

## 13. Acceptance criteria

A clean CPython >=3.11 environment can build/install the package locally and:

1. construct a canonical rights request;
2. protect encoded PNG/JPEG/WebP bytes;
3. receive warnings/execution reports;
4. verify the output through a typed canonical report;
5. catch structured exceptions;
6. use file convenience helpers;
7. interoperate with Rust-produced fixtures in both directions.

The Rust CLI, root library, carrier, `./scripts/check.sh`, and existing
release pipeline remain behaviorally unchanged.

## 14. Stop conditions

Stop and report rather than improvise if:

- correct binding requires changing a canonical protection/verification
  semantic;
- a needed core value has no safe public getter/constructor and exposing one
  would create a new Rust stability promise not justified by the binding;
- PyO3/maturin requires raising the root/core MSRV;
- the only proposed solution requires weakening `forbid(unsafe_code)` in the
  root or carrier;
- interpreter detachment appears unsound with the chosen buffer ownership;
- package naming collides with an existing unrelated installed module in a way
  that requires a public naming decision;
- tests reveal Rust/Python output divergence.

## 15. Closure evidence required

Create
`plans/closure/language-bindings/001-status.md` with:

- final package/API tree;
- versions of Python, PyO3, maturin, Rust used;
- exact verification commands and results;
- proof `./scripts/check.sh` did not gain Python prerequisites;
- parity-test summary by format/path;
- structured-error test summary;
- panic-profile evidence;
- known limitations/deferred items;
- commit(s) implementing the milestone.

Then update the roadmap and registry: M001 -> closed, M002 -> ready if no new
blocker emerged.

## 16. Handoff notes

Implement the smallest Python-native projection that covers the canonical
application workflow. Avoid binding every Rust symbol. Prefer owned/simple DTO
projections over exposing Rust internal layout. Reuse existing fixtures and
semantics rather than creating a parallel Python protocol vocabulary.

M002 owns wheel/sdist cross-platform qualification. Do not spend M001 on
release automation beyond what is needed to prove local packaging mechanics.
