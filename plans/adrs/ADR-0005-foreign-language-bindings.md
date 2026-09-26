# ADR-0005: Foreign-Language Bindings Are Leaf Frontends over the Canonical Byte API

Status: accepted

Date: 2026-09-26

Decision owners: project maintainers

Related specification sections:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/000-long-term-specification.md#5-release-invariants`
- `plans/001-terminology-and-domain-model.md#core-request-model`
- `plans/001-terminology-and-domain-model.md#verification-model`

Affected subsystem roadmaps:

- `plans/subsystems/language-bindings-roadmap.md`
- `plans/subsystems/api-cli-contract-roadmap.md`
- `plans/subsystems/release-distribution-roadmap.md`

## Context

The canonical application API is already byte-oriented:
`process_request_bytes*` consumes encoded image bytes plus a
`ProtectionRequest`, and `verify_image_bytes_report` returns the canonical
structured verification model. The CLI is a separate consumer and generic
carrier functionality is isolated in `stegoeggo-stego`.

This makes foreign-language bindings useful without moving language-runtime
concerns into the Rust core. The project direction is Python first, then
Node.js, then a stable C ABI. A shared C ABI is not required as an
implementation substrate for Python or Node.js.

## Decision drivers

- Preserve one canonical protection and verification implementation.
- Keep rights metadata on encoded-byte paths per ADR-0003.
- Avoid making the C ABI the lowest common denominator for higher-level
  language APIs.
- Keep PyO3, napi-rs, Python, Node.js, and C-ABI dependencies out of
  `stegoeggo` and `stegoeggo-stego`.
- Preserve `#![forbid(unsafe_code)]` in the library and carrier crates.
- Allow language-idiomatic errors, DTOs, typing, and file helpers without
  expanding the canonical Rust surface.
- Do not expand required Rust CI merely by adding a language binding.
- Delay irreversible C ABI layout/version decisions until the higher-level
  bindings have exercised the canonical integration boundary.

## Considered options

### Option A — Native leaf bindings per language over the Rust API (chosen)

Python uses PyO3/maturin directly against `stegoeggo`; Node.js uses napi-rs
directly against `stegoeggo`; the C ABI is a later explicit leaf crate.

Benefits: idiomatic APIs, direct structured-error translation, no extra ABI
layer or copying, and no premature C ABI freeze. Costs: some projection code is
language-specific. Failure mode: bindings drift semantically; mitigated by
cross-language known-answer and verification parity tests.

### Option B — Implement C ABI first and build every language binding over it

Benefits: one FFI substrate. Costs: commits the project to allocator,
ownership, error, handle, struct-layout, and ABI-version rules before Python or
Node requirements are known; higher-level bindings become less idiomatic and
gain an unnecessary translation layer. Rejected for the initial sequence.

### Option C — Reimplement protection logic in each language

Benefits: no native extension packaging. Costs: duplicated protocol and image
logic, conformance drift, larger security/correctness surface. Rejected.

## Decision

Foreign-language bindings are leaf frontends that delegate to the canonical
Rust byte APIs and canonical request/verification models.

Sequence:

1. Python binding and packaging/qualification.
2. Node.js binding using the same semantic contract, informed by Python
   closure evidence.
3. C ABI design and implementation after Python and Node have demonstrated
   the minimal stable cross-language boundary.

The initial Python and Node implementations MUST NOT depend on the future C
ABI. They may share concepts, tests, and fixtures, but no common
`bindings-core` crate is introduced until concrete duplication demonstrates
a need.

The public binding surfaces expose canonical APIs, not deprecated
`ProtectionLevel`, `EvidenceProfile`, or `ProtectionContext` adapters.

Encoded bytes are the canonical FFI data path when metadata matters. Language
file helpers remain frontend conveniences and delegate to byte operations.

Binding packages live under `bindings/` and are kept outside the root Rust
workspace initially so `cargo ... --workspace` and `./scripts/check.sh`
do not silently acquire Python/Node/native-toolchain prerequisites. Each
binding has explicit language-specific qualification.

Binding artifact versions track the StegoEggo source version they wrap, but
they do not join the carrier -> library -> CLI crates.io publication chain.
Automated registry publication remains out of scope under the current release
invariants.

For the future C ABI, any required `unsafe` is confined to the leaf C-ABI
crate and audited there. The root `stegoeggo` and `stegoeggo-stego` crates
remain `#![forbid(unsafe_code)]`.

## Consequences

### Positive

- Python and Node can present idiomatic typed APIs without capability forks.
- Core Rust MSRV, dependency, safety, and CLI contracts remain isolated.
- Python/Node experience informs the eventual C ABI instead of being
  constrained by a speculative ABI.
- Existing fixtures and known-answer tests can prove cross-language parity.

### Negative

- Binding wrappers duplicate some enum/error/report projection logic.
- Release qualification becomes multi-ecosystem work.
- Binding compatibility promises must be documented independently from Rust
  semver promises.

### Neutral or deferred

- Generic `stegoeggo-stego` language exposure is deferred until the
  application binding is qualified.
- Free-threaded CPython, PyPy, browser/WASM, Swift/Kotlin, and .NET are not
  initial support commitments.
- A shared internal projection crate may be reconsidered only after Python
  and Node expose concrete duplicated logic.

## Compatibility and migration

No existing Rust or CLI API changes. Binding packages start as new 0.x
surfaces. Python and Node public names should model canonical terminology from
`plans/001-terminology-and-domain-model.md`; compatibility guarantees are
recorded before first registry publication.

## Security and reliability implications

- Bindings MUST preserve `ResourceLimits` semantics for untrusted inputs.
- Secret key material must not be accidentally serialized or included in
  Python/Node repr/debug projections.
- CPU-heavy native work should run without unnecessarily holding the host
  language interpreter lock/event-loop thread.
- Panics must not be configured to abort the embedding host process; binding
  release profiles use unwind semantics even though the standalone CLI uses
  `panic = "abort"`.
- C ABI ownership/freeing rules and panic containment require an explicit
  later milestone before any ABI is declared stable.

## Verification

Each binding milestone requires:

- canonical Rust `./scripts/check.sh` remains green;
- language-specific unit/integration tests;
- Rust-produced -> foreign verify parity;
- foreign-produced -> Rust verify parity;
- deterministic byte equality where explicit seed and timestamp override
  make output deterministic;
- malformed input, resource-limit, and structured-error tests.

## Supersession

None.
