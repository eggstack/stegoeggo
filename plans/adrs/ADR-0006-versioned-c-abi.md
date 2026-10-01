# ADR-0006: Versioned C ABI Uses Opaque Rust-Owned Handles and Explicit Ownership

Status: accepted

Date: 2026-10-01

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

ADR-0005 delayed the C ABI until Python and Node had exercised the canonical
foreign-runtime boundary. Those bindings are now closed and requalified
through language-bindings M006.

The proven common boundary is smaller than either language-native API:

- encoded image bytes in/out;
- canonical `ProtectionRequest` construction;
- parser/resource limits;
- canonical protect/verify operations;
- stable error categories plus structured details;
- rich verification/report data;
- deterministic seed/timestamp controls;
- explicit ownership and secret-key non-disclosure.

A C ABI has a higher compatibility cost than Python/Node because struct layout,
enum representation, allocator ownership, symbol names, panic behavior, and
calling convention all become external contracts. Rust's native type layout
and allocator must therefore remain behind the boundary.

## Decision drivers

- Preserve the canonical byte API and request/verification semantics.
- Never expose Rust-owned `Vec`, `String`, enum layout, trait objects, or
  non-`repr(C)` structs as C layout.
- Keep all required `unsafe` confined to the leaf C binding crate.
- Make allocation/free ownership obvious and allocator-correct.
- Ensure Rust panics never unwind across a non-unwinding C ABI boundary.
- Make the ABI additive within a major version and allow future ABI majors to
  coexist.
- Avoid thread-local last-error state and stale-error ambiguity.
- Keep the public header reviewable, generated from the actual FFI surface,
  and independent from normal Rust builds.
- Keep Python and Node as direct Rust frontends; C remains a sibling leaf
  binding, not their substrate.

## Considered options

### Option A — Opaque handles + fixed-width codes + explicit Rust frees (chosen)

Expose incomplete C handle types for requests, notices, limits, buffers,
reports, and errors. Constructors return owned handles; explicit destructor
functions return ownership to Rust. Functions accept/return fixed-width scalar
codes and raw borrowed byte/string views only for the duration of a call or
handle lifetime.

Benefits: Rust layout remains private; additive getters/setters do not change
existing handle layout; allocator ownership is unambiguous; the same model is
widely proven by long-lived C libraries.

Costs: more function calls and explicit lifecycle management.

### Option B — Public `repr(C)` request/report structs

Benefits: fewer calls and easy stack allocation.

Costs: freezes field order, size, padding, enum representation, optional-value
encoding, and extension strategy; would force future Rust evolution through a
C layout. Rejected for canonical request/report objects.

### Option C — JSON-only request and result ABI

Benefits: tiny symbol surface and no C object lifecycle beyond strings.

Costs: would create a new stable request JSON schema, require every C consumer
to parse/construct JSON, weaken typed input validation, and make binary image
output awkward. Rejected as the primary ABI. JSON remains an optional complete
report serialization.

### Option D — Thread-local last-error API

Benefits: familiar from libraries such as libgit2 and small call signatures.

Costs: hidden mutable state, stale-error semantics, extra thread-local
contract, and awkward nested/multi-error handling. Rejected. Errors are
explicit owned handles returned by the failing call.

### Option E — A higher-level FFI framework as the ABI authority

Examples include macro/framework approaches such as `safer-ffi`.

Benefits: can reduce handwritten unsafe code and automate header generation.

Costs: adds a framework-owned compatibility layer, makes the exact exported
contract less direct to audit, and is unnecessary for the intentionally small
surface. Rejected for v1. The leaf crate may still use small internal helpers
that do not define the public ABI.

## Decision

### 1. Leaf crate and artifact

The C binding lives under `bindings/c/`, outside the root Cargo workspace,
and depends directly on the canonical `stegoeggo` library.

The first qualified distributable artifact is a system dynamic library
(`cdylib`) plus a C header. A stable `staticlib` distribution is deferred
until its transitive native-symbol/export contract is separately qualified;
Rust's reference notes that static libraries include upstream dependencies and
can require explicit export/link management.

The binding's dev/release profile uses `panic = "unwind"`.

### 2. Symbol and ABI versioning

There is one unversioned bootstrap/version surface and versioned operational
symbols.

Bootstrap symbols:

- `stegoeggo_abi_version_major()`;
- `stegoeggo_abi_version_minor()`;
- `stegoeggo_source_version()`.

Operational v1 symbols use the `stegoeggo_v1_` prefix.

ABI major 1 is append-only: existing symbol signatures, numeric code values,
and documented ownership semantics do not change. New functions/codes may be
added compatibly. A breaking ABI requires a new major symbol namespace
(`stegoeggo_v2_*`) that may coexist with v1.

The source package version and C ABI version are independent.

### 3. Public C types

Canonical Rust objects are represented by distinct opaque handles, including
at minimum:

- notice;
- protection request;
- resource limits;
- owned byte buffer;
- execution report;
- verification report;
- error.

Opaque handle internals are never emitted in the public header.

C-visible scalar categories use fixed-width integer typedefs/constants rather
than exporting Rust enums. Unknown scalar input values are rejected as invalid
arguments/configuration; they are never transmuted to Rust enums.

Use:

- `uint64_t` for full-width seeds;
- `uint32_t`/other explicit widths for enum-like codes and dimensions;
- `size_t` for native buffer lengths/counts;
- `uint8_t` 0/1 values where a stable one-byte boolean representation is
  needed.

### 4. Request construction

C v1 constructs canonical requests through opaque mutable builder handles.
The API exposes the semantic equivalents of the finalized Python/Node common
surface:

- metadata-only / hidden-marker / preset request constructors;
- rights notice fields and DMI/policy;
- seed, intensity, output format, JPEG quality, progressive mode;
- max dimension, metadata update policy, redundancy, content hash,
  timestamp override;
- MAC key;
- resource limits;
- hidden-marker mode/tile size;
- authentication mode.

C mutability is a frontend convenience only. Each operation delegates to a
canonical Rust `ProtectionRequest`; deprecated `ProtectionContext`,
`ProtectionLevel`, and `EvidenceProfile` are not exported.

Secret key bytes have no getter and are zeroized by the canonical Rust object
when the request handle is destroyed.

### 5. Borrowed input and owned output memory

Input image/key/string data is borrowed only for the duration of the
synchronous call. Pointer + length is the canonical input form; text is
validated as UTF-8 where required. A null pointer is accepted only where the
API explicitly documents absence or zero length.

Output image bytes, serialized report bytes, and errors are Rust-owned opaque
handles. Callers inspect borrowed data/length views from those handles and
must release the handle with the matching `stegoeggo_v1_*_free` function.

C callers never call `free()` on Rust-owned memory and never construct or
free opaque handles themselves. `*_free(NULL)` is a documented no-op.
Double-free, use-after-free, wrong-handle-type use, or arbitrary invalid
non-null pointers remain caller contract violations.

### 6. Operations and reports

The initial ABI is synchronous and reentrant. It includes:

- format detection;
- protect;
- protect with warnings/report;
- verify with optional MAC key/resource limits.

No callback, async runtime, worker thread, or cancellation contract is part of
v1. C callers may execute independent operations on their own threads.

Execution and verification reports remain opaque. Stable top-level getters are
provided for common scalar facts. Complete verification JSON uses the existing
stable `VerificationReport` JSON schema. If execution-report JSON is exposed,
it is explicitly schema-versioned by the C binding and additive within ABI v1.

Opaque report handles allow future getters to be added without changing
existing layout.

### 7. Error model

Every fallible operational/builder function returns a fixed-width status/error
code and accepts an explicit out-error handle. Success is zero. Failure is
nonzero and, when an error out-parameter is supplied, returns one owned error
handle describing that call.

The v1 error categories include at least:

- invalid argument / ABI misuse that can be detected safely;
- invalid configuration;
- invalid format;
- encode/decode;
- metadata;
- steganography;
- insufficient capacity;
- verification;
- resource limit;
- internal/panic.

The error handle exposes a message plus stable machine category/resource
information. Existing structured capacity/resource-limit fields are preserved
where Rust provides them. The carrier resource-limit category remains
`resource = carrier`; unavailable fields are not invented.

No thread-local "last error" is authoritative.

### 8. Panic boundary

All operational exported functions use the non-unwinding `extern "C"` ABI.
No Rust panic may cross that boundary.

The leaf crate is compiled with `panic = "unwind"`, and each exported
fallible entry point catches Rust panics around the complete internal
operation and maps them to the internal/panic error category.

This is containment for unexpected Rust panics, not ordinary control flow.
Expected invalid input and canonical operation failures remain `Result`
paths.

Allocator-level aborts and process-fatal conditions that do not unwind cannot
be converted into C errors; resource limits remain the primary defense against
hostile oversized inputs.

### 9. Pointer, lifetime, and thread contract

Every non-null pointer supplied by C must be valid/aligned for the documented
type and lifetime. The binding validates null/length combinations and scalar
ranges before dereferencing/conversion where possible, but cannot make
arbitrary dangling pointers safe.

Independent handles and operations may be used concurrently. A single mutable
handle must not be mutated, freed, or used concurrently by multiple threads
unless a future function explicitly documents such support. No global mutable
request/error state is introduced.

### 10. Header generation

The public C header is generated with a pinned standalone cbindgen tool and
committed to the repository. cbindgen is a development/qualification tool, not
a `build.rs` dependency and not a prerequisite of ordinary root Rust builds.

At decision time cbindgen 0.29.4 is current and declares Rust 1.74, below the
project's Rust 1.89 MSRV.

Header generation is configured for C, C++ compatibility guards, explicit
include guards, `size_t` for Rust `usize`, opaque handle emission, and only
the intended FFI items. CI/qualification regenerates the header and rejects
drift.

The generated header must compile as C11 and C++17 in qualification.

### 11. Export surface

Only the documented bootstrap and `stegoeggo_v1_*` C symbols are part of the
stable export contract.

M008 must inspect dynamic-library exports on Linux, macOS, and Windows and
record a canonical symbol manifest. Accidental Rust/internal exports are a
release blocker.

## Consequences

### Positive

- Rust request/report layout remains evolvable.
- Ownership and allocator rules are explicit.
- Existing v1 consumers can coexist with a future v2 ABI.
- Error handling is deterministic per call and naturally thread-safe.
- The unsafe surface is small, leaf-local, and auditable.
- C can consume the same canonical byte operations without constraining
  Python/Node.

### Negative

- Opaque handles require more boilerplate and explicit lifecycle calls.
- C consumers must manage handles carefully.
- Complete rich reports may require JSON parsing for fields without direct
  getters.
- Dynamic-library distribution adds loader/search-path concerns.

### Neutral or deferred

- Static-library distribution is deferred pending export/link qualification.
- Async/callback APIs are deferred.
- WASM/browser and language-specific wrappers over the C ABI are not part of
  v1.
- A future ABI major may add a different object/result strategy while retaining
  v1 symbols.

## Compatibility and migration

The C ABI is new. ABI v1 compatibility begins only when M008 implementation
and qualification close; M007 defines the contract but does not claim shipped
stable symbols.

Within ABI v1:

- existing symbol signatures and numeric code values are frozen;
- opaque layouts remain private;
- new functions/constants may be added;
- existing report JSON follows its documented schema compatibility rules.

Breaking changes require a new ABI-major namespace.

Python and Node remain unchanged and continue to bind directly to Rust.

## Security and reliability implications

- Raw pointers are inherently unsafe; all raw-pointer dereference/conversion
  is confined to the leaf crate.
- Pointer/length overflow and null combinations are validated before slice/text
  creation.
- Resource limits remain configurable from C and apply to untrusted input.
- MAC keys are write-only from the C perspective and never serialized.
- Rust allocations are always returned to Rust for destruction.
- Panics are caught before the C boundary; no Rust unwind crosses
  `extern "C"`.
- No hidden global/thread-local error state is required.

## Verification

M007 must validate the contract design with header/API review and a compile-only
consumer sketch. M008 implementation must add:

- Rust unit tests for pointer/scalar/error conversion helpers;
- C lifecycle/operation/error tests;
- C++ header-compatibility compile smoke;
- Rust ↔ C protect/verify parity;
- malformed/null/boundary input tests;
- panic-containment test seam;
- allocator ownership/free tests;
- cross-platform dynamic-library/header qualification;
- exported-symbol manifest checks;
- canonical root `./scripts/check.sh`.

## Research basis

- Rust Reference panic/unwinding and FFI boundary rules:
  https://doc.rust-lang.org/reference/panic.html
- Rust Nomicon FFI/opaque types/ownership:
  https://doc.rust-lang.org/nomicon/ffi.html
- Rust Reference linkage and `cdylib`/`staticlib` behavior:
  https://doc.rust-lang.org/reference/linkage.html
- cbindgen documentation and current release:
  https://github.com/mozilla/cbindgen/blob/main/docs.md
  https://github.com/mozilla/cbindgen/releases
- SQLite opaque-handle lifecycle precedent:
  https://sqlite.org/cintro.html
- libgit2 return-code/error precedent and limitations of last-error state:
  https://libgit2.org/docs/reference/main/errors/index.html

## Supersession

None.
