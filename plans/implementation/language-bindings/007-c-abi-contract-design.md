# Language Bindings Milestone 007 — C ABI Contract Design

Status: ready for handoff

Repository baseline: `a5b3dff84a3b83eecbca0ac3d44fe5ebf4951673`

Source roadmap:
`plans/subsystems/language-bindings-roadmap.md#m007--c-abi-contract-design`

Long-term requirements:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/000-long-term-specification.md#5-release-invariants`
- `plans/001-terminology-and-domain-model.md#core-request-model`
- `plans/001-terminology-and-domain-model.md#verification-model`
- `plans/002-long-term-roadmap.md#phase-5--language-bindings-active`

Applicable ADRs:

- `plans/adrs/ADR-0001-canonical-protection-request.md`
- `plans/adrs/ADR-0003-byte-vs-pixel-paths.md`
- `plans/adrs/ADR-0005-foreign-language-bindings.md`
- `plans/adrs/ADR-0006-versioned-c-abi.md`

Primary class: invariant

## 1. Objective

Turn ADR-0006 into an exact, implementation-ready C ABI v1 contract before
any stable C symbols are shipped.

M007 defines:

- the complete v1 symbol inventory and exact C signatures;
- fixed numeric values for ABI-visible status/category constants;
- opaque handle lifecycle and ownership;
- pointer/null/length/precondition rules;
- request/notice/resource-limit builder semantics;
- synchronous protect/verify operation semantics;
- error/report access and JSON rules;
- panic containment requirements;
- thread/reentrancy rules;
- header-generation and symbol-manifest policy;
- the five-target M008 qualification matrix.

M007 is a design/contract milestone. It MUST NOT ship a working C shared
library or claim ABI v1 availability. M008 implements and qualifies the
accepted contract.

## 2. Why this milestone is ready

The hard dependency is closed:

- Python foundation/qualification/correctives: closed;
- Node foundation/qualification/corrective: closed;
- language-bindings M006 requalified both frontends against the same canonical
  Rust source, including the new carrier resource-limit category;
- verification-conformance M006 restored and guarded the canonical
  `NotFound` versus `Invalid` boundary.

Those two mature frontends show which concepts genuinely cross language
boundaries and which are language ergonomics.

ADR-0006 now fixes the durable architecture:

- leaf C binding directly over `stegoeggo`;
- opaque handles rather than public Rust/C struct layouts;
- Rust-owned output allocation and explicit Rust frees;
- fixed-width scalar/category codes;
- versioned `stegoeggo_v1_*` symbols;
- explicit per-call error handles;
- synchronous v1 operations;
- `extern "C"` + catch-unwind panic containment;
- cbindgen as a pinned standalone development/qualification tool;
- `cdylib` as the first qualified artifact;
- staticlib distribution deferred.

No remaining architecture decision should be invented by the M008
implementation agent.

## 3. Current implementation evidence

There is no C binding package today.

The common Python/Node capability already proves:

- canonical encoded-byte protection and verification;
- canonical request construction;
- full-width `u64` seeds;
- resource-limit projection including
  `ResourceLimitExceeded(String) -> resource="carrier"`;
- structured errors without secret leakage;
- deterministic Rust↔foreign parity;
- five-target native qualification;
- panic-unwind profiles in embedded runtimes.

Relevant stability evidence:

- `STABILITY.md` marks `ProtectionRequest`,
  `verify_image_bytes_report`, `VerificationReport`,
  `ExecutionReport`, `ProtectionWarning`, and canonical operations stable;
- `VerificationReport` JSON is already a stable machine-readable schema;
- request/report Rust structs are evolvable Rust APIs and are NOT C-layout
  contracts.

External research confirms:

- Rust only guarantees C-compatible struct layout when explicitly
  `#[repr(C)]`; opaque handles avoid unnecessarily freezing layout;
- Rust panics must not cross a non-unwinding `extern "C"` boundary;
  `catch_unwind` is the supported containment mechanism for unwind panics;
- `cdylib` is intended for dynamic libraries loaded by other languages;
  `staticlib` includes upstream dependencies and needs additional symbol/link
  care;
- cbindgen 0.29.4 is current at planning time, supports opaque exports and
  `usize -> size_t`, and declares Rust 1.74, below StegoEggo's Rust 1.89
  MSRV;
- long-lived C APIs such as SQLite use typed opaque handles with explicit
  constructors/destructors;
- libgit2 demonstrates return-code/error-category conventions but also the
  stale/TLS semantics ADR-0006 avoids with explicit error handles.

## 4. Invariants that must not regress

- C delegates to canonical `ProtectionRequest` and
  `process_request_bytes*` / `verify_image_bytes_report`.
- No deprecated `ProtectionLevel`, `ProtectionContext`, or
  `EvidenceProfile` appears in C v1.
- Encoded bytes remain the metadata-capable boundary.
- Root and carrier crates retain `#![forbid(unsafe_code)]`.
- All FFI `unsafe` remains in the future `bindings/c` leaf.
- Rust enum discriminants/layout are never the C contract.
- Rust `Vec`/`String`/report/request layouts are never exposed directly.
- C never frees Rust memory with `free()`.
- No Rust panic unwinds through `extern "C"`.
- The C API preserves full-width `uint64_t` seeds.
- MAC key bytes are write-only and never returned in errors/reports/getters.
- Resource limits stay available to C callers.
- Unknown future Rust variants map safely to a stable internal/unknown C
  category; no unchecked transmute from arbitrary C numeric values.
- ABI v1 existing symbols/signatures/code values become immutable once M008
  closes.
- Python and Node remain direct Rust bindings and do not move onto C.
- Normal root Rust builds/checks gain no C compiler/cbindgen prerequisite.

## 5. Scope

### In scope

- Exact ABI v1 C signature inventory.
- Exact numeric code tables.
- Exact opaque handle inventory.
- Constructor/setter/free semantics.
- Input pointer/string/key semantics.
- Buffer/result/error ownership.
- Error code/resource/detail contract.
- Execution report and verification report accessor contract.
- Verification JSON contract reuse.
- Decision whether execution report JSON is in v1; if yes, exact schema/version
  must be specified.
- Header generation contract.
- Exported-symbol manifest contract.
- C11 and C++17 header compatibility target.
- Thread/reentrancy contract.
- M008 five-target qualification requirements.
- `bindings/c/ABI-V1.md` normative design document.
- `bindings/c/README.md` pre-implementation status/documentation.
- Closure record for design acceptance.

### Explicitly out of scope

- Rust `extern "C"` implementations.
- `bindings/c/Cargo.toml` production crate unless needed only for a
  non-exporting compile experiment; no stable symbols may ship.
- Generated shipping `stegoeggo.h`.
- Dynamic/shared library release artifacts.
- Staticlib distribution.
- registry/package-manager publication.
- callback/async/cancellation APIs.
- file-path APIs.
- batch APIs.
- detached manifest/signature experimental features.
- generic carrier/stego-only C APIs.
- Python/Node refactors.
- C++ wrapper classes.
- WASM/Swift/JNI/.NET wrappers.

## 6. Required production/design changes

M007 is documentation/contract work. No stable C production binary is created.

### A. Create the normative ABI document

Create `bindings/c/ABI-V1.md` with a "not yet shipped" banner and the exact
contract below.

It becomes the implementation authority for M008 beneath ADR-0006.

### B. Bootstrap/version symbols

Freeze these unversioned bootstrap symbols:

```c
uint32_t stegoeggo_abi_version_major(void);
uint32_t stegoeggo_abi_version_minor(void);
const char *stegoeggo_source_version(void);
```

Contract:

- ABI major initially `1`;
- ABI minor initially `0`;
- source version is a library-owned, NUL-terminated static ASCII string valid
  for process lifetime;
- caller never frees the source-version pointer;
- future ABI majors retain the bootstrap functions.

All operational symbols are `stegoeggo_v1_*`.

### C. Opaque handles

Freeze distinct incomplete C handle types for:

```c
stegoeggo_v1_notice_t
stegoeggo_v1_request_t
stegoeggo_v1_resource_limits_t
stegoeggo_v1_buffer_t
stegoeggo_v1_execution_report_t
stegoeggo_v1_verification_report_t
stegoeggo_v1_error_t
```

Every owned handle has exactly one matching free function. Freeing NULL is a
no-op. Handle internals are absent from the public header.

M007 must state which constructors return each handle and which functions
borrow each handle.

### D. Fixed-width code tables

Do not generate C enums whose ABI size depends on the C compiler.

Define fixed-width typedefs such as:

```c
typedef uint32_t stegoeggo_v1_status_t;
typedef uint32_t stegoeggo_v1_error_code_t;
typedef uint32_t stegoeggo_v1_resource_code_t;
typedef uint32_t stegoeggo_v1_rights_policy_t;
typedef uint32_t stegoeggo_v1_dmi_value_t;
typedef uint32_t stegoeggo_v1_image_format_t;
typedef uint32_t stegoeggo_v1_metadata_update_policy_t;
typedef uint32_t stegoeggo_v1_preset_t;
typedef uint32_t stegoeggo_v1_authentication_mode_t;
typedef uint32_t stegoeggo_v1_hidden_marker_mode_t;
typedef uint32_t stegoeggo_v1_verification_status_t;
typedef uint32_t stegoeggo_v1_evidence_strength_t;
typedef uint32_t stegoeggo_v1_warning_t;
```

M007 MUST assign and document explicit numeric values for every v1 constant.
Do not derive values from Rust enum order/discriminants.

Reserve unused numeric ranges for additive future constants.

At minimum freeze error/status codes:

- 0 success;
- invalid argument;
- invalid configuration;
- invalid format;
- encode/decode;
- metadata;
- steganography;
- insufficient capacity;
- verification;
- resource limit;
- internal/panic.

At minimum freeze resource categories:

- none;
- input bytes;
- dimensions;
- container;
- metadata;
- verification budget;
- carrier.

Unknown input constants return invalid-argument/configuration errors, never UB.

### E. Notice/request/resource-limit lifecycle

The exact v1 symbol list must cover the common finalized Python/Node contract.

Notice:

- create/free;
- setters for every currently exposed `RightsNotice` field;
- DMI;
- optional seed.

Request constructors:

- metadata-only;
- with-hidden-marker;
- from-preset.

Request setters:

- seed;
- intensity;
- output format;
- JPEG quality;
- progressive JPEG;
- max dimension;
- metadata update policy;
- stego redundancy;
- 4-byte content hash;
- timestamp override;
- MAC key;
- resource limits;
- hidden-marker mode + tile size;
- authentication mode.

Resource limits:

- defaults/create/free;
- setter for every limit exposed by Python/Node M006.

The C handles may be mutable as frontend builders, but operation execution must
materialize/delegate to canonical Rust values.

Do not expose MAC-key getters.

### F. Text and byte input contract

Use pointer + length for caller-owned variable data:

- encoded image bytes;
- MAC key;
- content hash;
- UTF-8 notice/timestamp strings.

Freeze these rules:

- input memory is borrowed only for the synchronous function call;
- nonzero length requires non-NULL pointer;
- zero length may use NULL where the parameter permits empty input;
- text is UTF-8 and invalid UTF-8 returns invalid argument/configuration;
- sizes are checked before Rust slice creation where arithmetic can overflow;
- content hash length must be exactly 4;
- arbitrary invalid/dangling non-NULL pointers remain caller UB.

No C input API depends on NUL termination.

### G. Core operation signatures

The exact v1 contract must include synchronous equivalents of:

- detect format;
- protect;
- protect with report;
- verify.

`protect_with_warnings` is intentionally NOT required as a separate v1
operation if the execution-report handle exposes warning count/value access;
avoid redundant ABI surface.

Proposed semantic shape to freeze in `ABI-V1.md`:

```c
stegoeggo_v1_status_t stegoeggo_v1_detect_format(
    const uint8_t *data,
    size_t data_len,
    stegoeggo_v1_image_format_t *out_format,
    stegoeggo_v1_error_t **out_error);

stegoeggo_v1_status_t stegoeggo_v1_protect(
    const uint8_t *data,
    size_t data_len,
    const stegoeggo_v1_request_t *request,
    stegoeggo_v1_buffer_t **out_data,
    stegoeggo_v1_error_t **out_error);

stegoeggo_v1_status_t stegoeggo_v1_protect_with_report(
    const uint8_t *data,
    size_t data_len,
    const stegoeggo_v1_request_t *request,
    stegoeggo_v1_buffer_t **out_data,
    stegoeggo_v1_execution_report_t **out_report,
    stegoeggo_v1_error_t **out_error);

stegoeggo_v1_status_t stegoeggo_v1_verify(
    const uint8_t *data,
    size_t data_len,
    const uint8_t *mac_key,
    size_t mac_key_len,
    const stegoeggo_v1_resource_limits_t *limits,
    stegoeggo_v1_verification_report_t **out_report,
    stegoeggo_v1_error_t **out_error);
```

M007 may adjust argument ordering/naming only if it documents a concrete ABI
reason. It must not change the semantics.

Detect-format unknown input should be represented explicitly (prefer a stable
UNKNOWN format code on successful inspection) rather than treating an
unrecognized header as a Rust panic.

### H. Output and buffer ownership

Freeze an opaque owned buffer interface:

```c
const uint8_t *stegoeggo_v1_buffer_data(const stegoeggo_v1_buffer_t *buffer);
size_t stegoeggo_v1_buffer_len(const stegoeggo_v1_buffer_t *buffer);
void stegoeggo_v1_buffer_free(stegoeggo_v1_buffer_t *buffer);
```

The returned data view is valid until buffer free and must not be freed or
reallocated by C.

Do not expose Rust allocation capacity.

### I. Error contract

Freeze per-call explicit errors; no last-error TLS.

Every fallible call:

- returns status 0 on success;
- nonzero on failure;
- initializes owned output handles to NULL before work;
- if `out_error != NULL`, returns NULL on success and an owned error handle
  on failure;
- never requires an error out-pointer to receive the numeric return code.

Freeze direct accessors for:

- error code;
- resource category;
- message data + length.

For detailed structured fields, choose ONE before M007 closure:

1. additive typed getters for each existing structured field; or
2. `error_details_json` returning a versioned JSON object containing
   `required`, `available`, `size`, `limit`, dimensions, `kind`,
   and `count` where available.

Preferred design: direct stable category/resource/message accessors plus a
versioned details-JSON buffer. This keeps common branching C-native while
avoiding a large nullable getter surface.

The details schema must never invent unavailable values. Carrier resource
limits report category `carrier` and message only unless Rust later provides
structured facts.

### J. Report contract

Execution report handle must expose at least:

- effective policy;
- effective DMI / presence;
- metadata injected;
- stego attempted;
- stego succeeded;
- format transcoded;
- warning count;
- warning at index.

If complete execution-report JSON is included, M007 must define
`schema_version = 1` and an additive-only schema before closure.

Verification report handle must expose at least:

- marker/canonical verification status;
- evidence strength;
- rights found;
- authenticated.

It must also expose complete JSON using the already-stable
`VerificationReport` JSON schema documented in `STABILITY.md`.

Serialized JSON is returned as a Rust-owned buffer handle.

### K. Panic/FFI entry contract

Every future exported fallible function must use a common internal wrapper
that:

1. validates out-pointer presence required by the function;
2. initializes outputs to NULL/zero;
3. validates null/length/range rules before unsafe conversion;
4. executes inside `catch_unwind(AssertUnwindSafe(...))`;
5. maps canonical Rust `Error` to stable C error code/resource/details;
6. maps panic to internal/panic;
7. never unwinds across `extern "C"`.

M007 must specify free/version/data accessor behavior separately: trivial
accessors/destructors still MUST NOT unwind across C, but need not allocate an
error object.

OOM/process-abort behavior is explicitly outside catch-unwind guarantees.

### L. Thread/reentrancy contract

Freeze:

- no global mutable request/error state;
- independent handles can be used concurrently;
- same mutable handle cannot be mutated/freed/used concurrently;
- v1 operations are synchronous;
- caller owns external thread scheduling;
- no callbacks into C.

If same-handle concurrent read-only operation support is not explicitly
qualified in M008, document it unsupported rather than assuming Rust
`Sync` leaks into the C contract.

### M. Header-generation contract

M008 will create a leaf `bindings/c` Rust crate and generate
`bindings/c/include/stegoeggo.h` with pinned cbindgen 0.29.4 (or a newer
version only after M007 contract review).

M007 must freeze:

- cbindgen is invoked as a standalone dev/CI tool, not `build.rs`;
- generated header is committed;
- generation configured for C with C++ compatibility;
- include guard;
- C11 compatible;
- C++17 include smoke;
- only intended opaque/scalar/function items exported;
- `usize_is_size_t = true`;
- generated-header drift is a qualification failure.

### N. Symbol manifest and platform contract

Create a normative planned symbol inventory in
`bindings/c/ABI-V1.md`. M008 will turn it into a checked
`bindings/c/abi-v1-symbols.txt`.

First M008 matrix:

| Target | Native smoke |
|---|---|
| Linux x86_64 GNU | GCC/Clang C11 load/link/run |
| Linux aarch64 GNU | native ARM64 C11 load/link/run |
| macOS x86_64 | clang C11 load/link/run |
| macOS arm64 | clang C11 load/link/run |
| Windows x86_64 MSVC | cl.exe C compile/link/run |

Every target also builds the header as C++; Linux x86_64 performs the full
C++17 compatibility compile.

M008 inspects exported dynamic symbols using platform tooling and fails on
unexpected public `stegoeggo_*` symbols or missing expected v1 symbols.

## 7. Ordered work packages

### WP1 — Materialize ADR-0006 as an exact ABI inventory

Create `bindings/c/ABI-V1.md` and assign exact signatures and numeric values.

Acceptance:

- no "TBD" for v1-required symbols/codes;
- every function has ownership, nullability, success/failure, and thread notes;
- deprecated Rust APIs absent.

### WP2 — Close handle and ownership semantics

For every handle, document:

- creator;
- borrower functions;
- destructor;
- whether NULL is accepted;
- whether borrowed views are invalidated by mutation/free;
- concurrency restriction.

Acceptance:

- no memory is described as C-freeable;
- no cross-allocator ambiguity remains.

### WP3 — Close request/limits semantic coverage

Cross-check Rust, Python M006, and Node M006 common semantics.

Acceptance:

- every common stable request option is representable;
- full-width seed preserved;
- no secret getter;
- every current `ResourceLimits` field representable;
- no legacy adapter leaks into C.

### WP4 — Close operation/report/error semantics

Acceptance:

- exact protect/verify signatures fixed;
- error code/resource values fixed;
- error-detail strategy fixed;
- execution-report getter list fixed;
- verification-report JSON explicitly reuses stable schema;
- no wrapper-side reinterpretation of canonical verification state.

### WP5 — Header/export/tooling contract

Acceptance:

- cbindgen pin and invocation strategy fixed;
- C11/C++17 target fixed;
- first dynamic-library target matrix fixed;
- symbol inventory/export audit rule fixed;
- staticlib explicitly deferred.

### WP6 — Design-review fixtures

Create non-shipping consumer examples in the design document for:

1. metadata-only protect lifecycle;
2. hidden-marker + HMAC protect lifecycle;
3. verification + JSON extraction;
4. resource-limit error handling;
5. cleanup on mid-operation failure.

These are pseudocode/contract examples only in M007.

Acceptance:

- every allocated handle in each example has an obvious single destructor;
- error paths do not leak successful earlier handles;
- no C `free()` touches Rust memory.

### WP7 — Planning/stability reconciliation

Update:

- language-bindings roadmap;
- registry;
- `STABILITY.md` with a clearly marked "C ABI v1 planned/not shipped"
  section only if useful to prevent accidental claims;
- closure `plans/closure/language-bindings/007-status.md`.

Do not mark C ABI stable/shipped.

## 8. Failure, cancellation, restart, and contention semantics

- M007 cannot close with unresolved ABI-level TODOs.
- If a required semantic cannot be represented without exposing Rust layout,
  revise the opaque-handle function surface rather than weakening ADR-0006.
- If a report field is not required in direct C getters, keep it behind JSON;
  do not freeze a struct just for convenience.
- A design review may remove proposed redundant symbols before M007 closure;
  after M008 closes ABI v1, removals/signature changes require a new ABI major.
- No implementation experiment from M007 becomes stable merely because it
  compiles.
- Parallel M008 coding should not begin until M007 closure is accepted.
- If the repository core changes materially during M007, re-audit the exact
  common Python/Node contract before closure.

## 9. Compatibility and migration

There are no existing C ABI consumers.

M007 establishes the rules M008 will freeze:

- v1 operational prefix;
- numeric values;
- lifecycle rules;
- report/error semantics.

Before M008 closure these are design contracts, not distributed ABI promises.

After M008 closure, v1 is append-only. Breaking changes use
`stegoeggo_v2_*`.

Rust/Python/Node APIs are unchanged.

## 10. Required tests/reviews

M007 is a design milestone; it must not fabricate runtime evidence.

Required review checks:

1. symbol inventory contains no Rust-native layout;
2. all numeric categories have explicit values;
3. all pointer parameters have nullability/lifetime rules;
4. all owned handles have free functions;
5. all outputs have defined initialization on failure;
6. every request field used by Python/Node common stable contract is covered;
7. every current structured Rust error maps to a C category/resource/detail;
8. carrier resource limit maps to resource `carrier`;
9. unknown future Rust errors map to internal;
10. verification `NotFound`/`Invalid` projected verbatim;
11. full-width seed uses `uint64_t`;
12. no secret getters;
13. report JSON ownership defined;
14. panic wrapper contract covers every future operational export;
15. thread rules do not accidentally promise same-handle concurrency;
16. staticlib remains out of the first qualification contract;
17. M008 matrix covers all five existing native distribution architectures.

Optional but useful: compile a manually extracted signature sketch as C11/C++17
without linking. Do not commit a handwritten shipping header.

## 11. Required verification commands

Minimum repository gate:

```bash
./scripts/check.sh
```

Planning/design consistency:

```bash
grep -R "M007 C ABI" plans/subsystems/language-bindings-roadmap.md plans/registry.md
grep -R "ADR-0006" plans/subsystems/language-bindings-roadmap.md plans/implementation/language-bindings/007-c-abi-contract-design.md
```

If a temporary header sketch is created for review, compile it without
shipping it:

```bash
cc -std=c11 -Wall -Wextra -Werror -fsyntax-only <consumer.c>
c++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only <consumer.cpp>
```

Record actual compilers/versions if this optional evidence is used.

No cbindgen drift command is required until M008 has an actual Rust FFI source
and generated header.

## 12. Documentation updates

Create/update:

- `bindings/c/ABI-V1.md` — normative exact v1 design;
- `bindings/c/README.md` — explicitly "design only; no shipped library yet";
- `plans/subsystems/language-bindings-roadmap.md`;
- `plans/registry.md`;
- `STABILITY.md` only to prevent premature stability claims if needed;
- `plans/closure/language-bindings/007-status.md`.

ADR-0006 is already accepted and MUST NOT be rewritten to absorb local
implementation details discovered during M007. If M007 discovers a durable
contradiction, create a superseding ADR.

## 13. Acceptance criteria

M007 closes only when:

- ADR-0006 remains satisfied;
- `ABI-V1.md` contains an exact complete symbol inventory;
- every ABI-visible constant has an explicit fixed numeric value;
- request/notice/limits coverage matches the stable common foreign-binding
  semantics;
- exact ownership/free/null/lifetime rules exist for every handle/pointer;
- error model is exact and covers all current Rust error variants;
- report accessor/JSON contract is exact;
- panic boundary is exact;
- thread/reentrancy contract is exact;
- cbindgen/header-generation contract is exact;
- dynamic-library/export-symbol contract is exact;
- M008 five-target matrix is exact;
- no stable C symbols/artifacts are shipped by M007;
- `./scripts/check.sh` is green;
- closure record is accepted.

Only then does M008 become dependency-ready.

## 14. Stop conditions

Stop and report if:

- exact C semantics require exposing a Rust-native struct/enum layout;
- a stable C request requires a new canonical Rust API;
- VerificationReport JSON can no longer be treated as stable according to
  `STABILITY.md`;
- panic containment would require allowing unwind across C;
- header generation requires making cbindgen a root build dependency;
- a required v1 operation would need callbacks/async ownership;
- staticlib becomes required for M008 despite its unresolved symbol/link
  contract;
- the exact symbol surface cannot be kept meaningfully smaller than a
  mechanical projection of the entire Python/Node APIs.

## 15. Closure evidence required

Create `plans/closure/language-bindings/007-status.md` with:

- baseline and design commits;
- ADR-0006 reference;
- final handle inventory;
- final bootstrap + v1 symbol count/list;
- numeric code tables;
- lifecycle/ownership matrix;
- pointer/string rules;
- request/limits coverage matrix against Python/Node;
- error mapping matrix against every current Rust `Error` variant;
- report getter/JSON strategy;
- panic containment contract;
- threading contract;
- cbindgen version/config decision;
- M008 target matrix/export-audit decision;
- `./scripts/check.sh` result;
- optional C11/C++17 syntax-smoke evidence if run;
- unresolved/deferred items;
- explicit statement that no ABI binary/header was shipped;
- roadmap/registry transition making M008 ready.

## 16. Handoff notes

Keep M007 boring and exact.

The goal is not to make C look like Rust, Python, or Node. The goal is a small
stable systems boundary that can survive Rust implementation evolution.

Prefer another function over another public struct field. Prefer an opaque
handle over exporting layout. Prefer a fixed code over a Rust enum
discriminant. Prefer Rust-owned destruction over cross-allocator assumptions.

Do not begin M008 implementation merely because most of the contract is
obvious; close every ABI-visible number, signature, and ownership rule first.

### Research references

- Rust FFI/opaque type guidance:
  https://doc.rust-lang.org/nomicon/ffi.html
- Rust panic/unwind FFI rules:
  https://doc.rust-lang.org/reference/panic.html
- Rust system-library linkage:
  https://doc.rust-lang.org/reference/linkage.html
- cbindgen documentation/current release:
  https://github.com/mozilla/cbindgen/blob/main/docs.md
  https://github.com/mozilla/cbindgen/releases
- SQLite opaque-handle lifecycle:
  https://sqlite.org/cintro.html
- libgit2 error model:
  https://libgit2.org/docs/reference/main/errors/index.html
