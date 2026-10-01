# Language Bindings Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/000-long-term-specification.md#5-release-invariants`
- `plans/001-terminology-and-domain-model.md#core-request-model`
- `plans/001-terminology-and-domain-model.md#verification-model`
- `plans/002-long-term-roadmap.md#phase-5--language-bindings-active`

Related ADRs:

- `plans/adrs/ADR-0001-canonical-protection-request.md`
- `plans/adrs/ADR-0003-byte-vs-pixel-paths.md`
- `plans/adrs/ADR-0005-foreign-language-bindings.md`

## 1. Purpose and ownership boundary

Owns supported non-Rust library frontends under `bindings/`: initially
Python, then Node.js, then a C ABI. The subsystem owns language-facing API
projection, runtime integration, package metadata, wheel/npm/native artifact
qualification, binding-specific tests, and binding documentation.

It does not own protection algorithms, metadata/container codecs, stego
carrier behavior, canonical verification semantics, CLI behavior, or the
existing carrier -> library -> CLI crates.io release chain. Those remain in
their existing subsystems.

## 2. Work classification

### Invariants

- Every protection operation delegates to `ProtectionRequest` +
  `process_request_bytes*`; verification delegates to
  `verify_image_bytes_report`.
- Encoded bytes are the canonical binding boundary when metadata matters.
- Deprecated Rust compatibility adapters are not promoted into new binding
  APIs.
- Root and carrier crates retain `#![forbid(unsafe_code)]`.
- Binding-specific dependencies do not enter core/carrier/CLI dependency
  graphs.
- Required Rust CI is not expanded merely by adding a binding.
- Foreign output must remain interoperable with Rust output and vice versa.

### Capabilities

- Python users can protect and verify encoded PNG/JPEG/WebP bytes with
  structured requests, reports, warnings, and exceptions.
- Node.js users can perform the same canonical operations with Buffer/
  Uint8Array and typed TypeScript declarations.
- C callers can eventually invoke a small versioned byte-oriented ABI with
  explicit ownership and error contracts.

### Infrastructure

- Binding package layout and isolated Cargo manifests.
- DTO/enum/error projections.
- Wheel/npm/native artifact matrices and smoke tests.
- Cross-language fixture/parity harnesses.

### Polish

- Python/Node file helpers, type stubs/declarations, examples, performance
  measurements, packaging ergonomics, optional batch APIs after correctness
  closure.

## 3. Non-goals

- Reimplementing StegoEggo algorithms in Python, JavaScript, or C.
- Using the C ABI as the implementation substrate for Python or Node.
- Exposing `DynamicImage` or Rust lifetime/borrow abstractions directly.
- Exposing deprecated `ProtectionContext`/`ProtectionLevel`/
  `EvidenceProfile` APIs.
- Generic `stegoeggo-stego` bindings in the first Python milestone.
- Browser/WASM, PyPy, Swift/Kotlin, .NET, Ruby, or JVM bindings in the
  initial roadmap.
- Automatic PyPI/npm publication while the project release contract remains
  manual-only.

## 4. Current state

At baseline `fe86d7361ff9b4ac725e1b05ef915ac391803868`, there are no
foreign-language binding packages. The canonical Rust surface is already
binding-friendly:

- `process_request_bytes`, `process_request_bytes_with_warnings`, and
  `process_request_bytes_with_report` are byte-in/byte-out canonical paths.
- `verify_image_bytes_report` is the canonical rich verification path.
- `RightsPolicy`, `ProtectionPreset`, `ProtectionRequest`,
  `RightsNotice`, `ExecutionReport`, `ProtectionWarning`, and
  `VerificationReport` provide the application model.
- Structured root errors preserve important data such as capacity and
  resource-limit counts.
- The CLI and generic carrier are already separate crates.
- Existing fixtures, known-answer vectors, cross-format tests, container
  preservation tests, and deterministic seed/timestamp support provide a
  strong parity oracle.

The root release profile uses `panic = "abort"`, which is appropriate for
the standalone CLI but must not be inherited as the host-process failure
policy of an embedded extension.

## 5. Target architecture

```text
Python package       Node package          C ABI library
PyO3 + maturin       napi-rs               explicit extern C
      |                  |                       |
      +------------------+-----------------------+
                         |
                    stegoeggo
                         |
                 stegoeggo-stego
```

Each frontend owns language-native constructors, typed result objects,
exceptions/errors, and convenience file helpers. The Rust core remains the
single semantic implementation.

Binding Cargo packages remain outside the root `--workspace` set until a
maintainer explicitly chooses to change required CI prerequisites. They depend
on the root library by path during development and by the matching source
release when packaged.

Binding package versions identify the StegoEggo source version they wrap.
Registry publication remains an explicit manual maintainer action.

## 6. Dependency graph

Closed canonical API/verification/container/carrier contracts
-> M001 Python binding foundation
-> M002 Python packaging and cross-platform qualification
-> M003 Python corrective qualification
-> M004 M003 closure corrective pass
-> M005 Node binding foundation and qualification
-> M006 post-core compatibility corrective and requalification
-> M007 C ABI contract design
-> M008 C ABI implementation and qualification

M001/M002 do not depend on the blocked release-distribution A->B updater
evidence milestone. M003 consumes the concrete findings from M001/M002.
PR #1 contains the M003 implementation and successful five-platform
release qualification. M003 was accepted, and M004 — the corrective pass
for M003's closure/evidence defects — closed, on PR #1: the lightweight
Python compatibility CI now builds and installs a wheel and is green
(run `36456938690`), and the release qualification identity is the
GitHub-recorded SHA `d40f1b37cf031b05e4f9c76af1cdfcf789d739b5` for run
`36337194059`. M005 consumes the final accepted Python contract rather than inventing a
competing semantic surface. M005 closed with five-target Node qualification,
but later canonical-core correctness changes made both Python and Node
compatibility workflows red and added a new Rust resource-limit variant not
explicitly projected by either binding. M006 owns that post-core compatibility
corrective and fresh native requalification. It has an interface dependency on
verification-conformance M6's explicit absence-versus-corruption contract and
cannot close before that core corrective closes. M007 C ABI design starts only
after language-bindings M006 closes so it is informed by two currently-green
foreign-runtime clients.

## 7. Milestones

### M001 — Python binding foundation

Class: capability/infrastructure. Add an isolated PyO3/maturin package with
canonical request/protect/verify APIs, typed reports/warnings, structured
exceptions, interpreter-detached CPU work, deterministic parity tests, and
pure-Python file helpers. No registry publication.

Implementation:
`plans/implementation/language-bindings/001-python-binding-foundation.md`.

### M002 — Python packaging and cross-platform qualification

Class: capability/infrastructure. Produce installable CPython abi3 wheels for
the supported platform matrix plus a source-build path, prove clean-environment
installation, document support, and add a manual artifact-build/rehearsal path.
No automatic PyPI publication.

Implementation:
`plans/implementation/language-bindings/002-python-packaging-qualification.md`.

Hard dependency: M001 closure.

### M003 — Python corrective qualification

Class: capability/infrastructure. Correct the post-M001/M002 findings before
the Python surface is treated as fully qualified: structured exception fields,
failure-safe file helpers, lightweight binding compatibility CI, native
five-platform wheel build/smoke mechanics, direct sdist `pip install`
evidence, and documentation/status reconciliation.

Implementation:
`plans/implementation/language-bindings/003-python-corrective-qualification.md`.

Hard dependency: M001 closed and M002 closure findings recorded. M002 may
remain conditionally closed while this corrective milestone executes.

### M004 — M003 closure corrective pass

Class: invariant. Correct the remaining closure/evidence defects observed in
PR #1 after the substantive M003 implementation: make the lightweight Python
compatibility CI green, correct the recorded successful-dispatch SHA, promote
qualified wheel rows in SUPPORT.md, and only then accept M003 closure.

Implementation:
`plans/implementation/language-bindings/004-m003-closure-corrective.md`.

Hard dependency: M003 implementation evidence exists in PR #1. M003 closure
itself remained pending this corrective pass and was accepted when it
closed.

### M005 — Node.js binding

Class: capability/infrastructure. Use napi-rs v3 directly over the canonical
Rust byte API, map encoded bytes to Buffer/Uint8Array, preserve u64 seeds as
JavaScript bigint, run CPU-heavy protection/verification through an off-thread
Promise boundary, generate and compile-check TypeScript declarations, preserve
structured error codes/fields, reuse canonical parity fixtures, and qualify
five native platform packages without npm publication.

Implementation:
`plans/implementation/language-bindings/005-node-binding-foundation-qualification.md`.

Hard dependency: M003 and M004 closure. M005 closed with accepted evidence in
`plans/closure/language-bindings/005-status.md`.

### M006 — Post-core compatibility corrective and requalification

Class: invariant. Reconcile Python and Node with post-M005 canonical-core
changes: consume verification-conformance M6's corrected `NotFound` versus
`Invalid` contract without wrapper reinterpretation, explicitly map the new
`Error::ResourceLimitExceeded(String)` into each language's existing
resource-limit family, stop freezing descriptive metadata-limit `kind`
wording as an enum-like contract, and requalify both native binding matrices.

Implementation:
`plans/implementation/language-bindings/006-post-core-compatibility-corrective.md`.

Interface dependency:
`plans/implementation/verification-conformance/006-unstructured-candidate-false-positive-corrective.md`.
The interface is contracted, so implementation may proceed in parallel, but
language-bindings M006 cannot close until verification-conformance M6 closes.

### M007 — C ABI contract design

Class: invariant/infrastructure. Define ownership, allocation/free functions,
opaque handles versus serialized DTOs, error codes/details, ABI version
negotiation, panic containment, thread-safety, symbol visibility, and
header-generation strategy. No stable symbols ship before the contract is
reviewed. Hard dependency: language-bindings M006 closure.

### M008 — C ABI implementation and qualification

Class: capability/infrastructure. Implement the accepted C boundary in a leaf
crate, confine/audit required unsafe there, add C smoke/integration tests,
cross-language parity, and native artifact/header packaging. Hard dependency:
M007 accepted contract.

## 8. Cross-cutting requirements

- Preserve ADR-0003 byte-vs-pixel semantics.
- Preserve output-domain routing and all format/container behavior.
- Keep resource-limit enforcement available to foreign callers.
- Never expose secret MAC/private-key bytes through repr/debug/JSON by
  accident.
- Foreign enum/error projections must tolerate future non-exhaustive Rust
  additions without undefined behavior or silent misclassification.
- Avoid unnecessary input copies, but correctness and safe host-runtime
  detachment take priority over zero-copy complexity.
- Python/Node CPU work must not unnecessarily monopolize interpreter/event
  loop execution.
- Core Rust MSRV remains 1.89 unless independently changed.
- Binding toolchain minimums are documented separately and may be newer than
  the Rust library MSRV.
- Existing `./scripts/check.sh` remains the required Rust CI gate unless a
  separate maintainer decision changes that policy.

## 9. Verification strategy

For every implemented language:

1. Run `./scripts/check.sh` for core regression evidence.
2. Run binding-native lint/type/unit/integration suites.
3. Protect via Rust and verify via the binding.
4. Protect via the binding and verify via Rust.
5. Compare deterministic bytes when seed + timestamp override make output
   deterministic.
6. Exercise PNG, JPEG, and WebP metadata-only plus hidden-marker paths.
7. Exercise malformed/truncated inputs, unsupported format, capacity failure,
   resource limits, and HMAC missing/wrong/correct-key cases in scope.
8. Install/package in clean environments rather than treating compilation as
   release evidence.

## 10. Risks and decision points

- PyPI/npm package names must be confirmed before publication; do not assume
  registry namespace availability from search results.
- CPython free-threaded ABI support is evolving; initial commitment is normal
  CPython abi3, with free-threaded qualification deferred.
- Workspace-local path dependencies can make sdists incomplete; source
  install must be tested from the built artifact in a clean environment.
- Root `panic = "abort"` must not terminate embedding runtimes; binding
  builds require unwind semantics.
- C ABI is the highest compatibility-cost surface and must not be designed
  from Rust struct layout.
- Language wrappers can accidentally drift from canonical terminology;
  cross-language fixtures and source-versioned docs are the guardrail.

## 11. Completion definition

The subsystem is complete when Python and Node packages are independently
installable and cross-language-equivalent to the canonical Rust API, and a
versioned C ABI with explicit ownership/error/panic contracts is implemented
and qualified, without regressions to core Rust, CLI, carrier, or release
invariants.

## 12. Milestone status table

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 Python foundation | closed | `implementation/language-bindings/001-python-binding-foundation.md` | `closure/language-bindings/001-status.md` | none |
| M002 Python packaging | closed (four-platform condition satisfied by M003 evidence, accepted in M004) | `implementation/language-bindings/002-python-packaging-qualification.md` | `closure/language-bindings/002-status.md` + `closure/language-bindings/004-status.md` §11 | none |
| M003 Python corrective qualification | closed | `implementation/language-bindings/003-python-corrective-qualification.md` | `closure/language-bindings/003-status.md` (corrected; §13 lists the M004 corrections) | none; M001 closed and M002 findings recorded |
| M004 M003 closure corrective | closed | `implementation/language-bindings/004-m003-closure-corrective.md` | `closure/language-bindings/004-status.md` | none; bounded defects corrected in PR #1 |
| M005 Node binding | closed | `implementation/language-bindings/005-node-binding-foundation-qualification.md` | `closure/language-bindings/005-status.md` | historical qualification valid for its recorded SHA; later core drift is owned by M006 |
| M006 post-core compatibility corrective | active | `implementation/language-bindings/006-post-core-compatibility-corrective.md` | pending | verification-conformance M6 closed; M006 closure needs binding-side corrective evidence |
| M007 C ABI design | proposed | not yet written | pending | language-bindings M006 closure |
| M008 C ABI implementation | proposed | not yet written | pending | M007 accepted contract |
