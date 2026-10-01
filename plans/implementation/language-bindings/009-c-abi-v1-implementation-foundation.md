# Language Bindings Milestone 009 — C ABI v1 Implementation Foundation

Status: blocked

Repository baseline: `9cac8fcda81ecc3e9e491d28e166470a9e8a7d8c`

Source roadmap:
`plans/subsystems/language-bindings-roadmap.md#m009--c-abi-v1-implementation-foundation`

Hard dependency:

- language-bindings M008 symbol-inventory corrective closure.

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
- `plans/adrs/ADR-0006-versioned-c-abi.md`

Normative ABI contract:

- `bindings/c/ABI-V1.md`
- `bindings/c/abi-v1-symbols.txt` after M008 closure

Primary class: capability

## 1. Objective

Implement the accepted C ABI v1 contract as an isolated Rust `cdylib` leaf
crate and prove the complete boundary locally/on lightweight Linux x86_64 CI
before cross-platform release qualification.

M009 owns:

- the `bindings/c` Rust crate;
- all corrected 90 normative exported symbols;
- opaque handle implementations;
- request/notice/resource-limit projection;
- canonical protect/verify delegation;
- owned buffer/report/error lifecycles;
- panic containment;
- generated committed C header;
- deterministic header/manifest checks;
- Rust unit tests;
- Linux x86_64 C11 integration/parity tests;
- C++17 header inclusion/compile;
- a separate path-filtered `c-binding.yml` compatibility workflow.

M009 does NOT declare ABI v1 stable/shipped. M010 performs the five-platform
native artifact/export qualification and activates the stability promise.

## 2. Why this milestone is blocked but fully planned

The architecture is closed in ADR-0006 and M007. The implementation mechanics
are sufficiently researched.

The only blocker is M008: implementation must consume a corrected, automated
90-symbol source of truth instead of the stale 89-symbol count.

Current ecosystem/tooling evidence:

- Rust `cdylib` is the system dynamic-library crate type intended for loading
  from non-Rust languages.
- Non-`repr(C)` Rust structs have no guaranteed C layout; current cbindgen
  intentionally emits forward declarations for such types, which matches the
  opaque-handle design.
- cbindgen 0.29.4 supports C output, C++ compatibility wrappers,
  `usize_is_size_t`, include guards, explicit export configuration, and
  standalone generation.
- Rust non-unwinding `extern "C"` boundaries may not allow a Rust panic to
  escape; `catch_unwind` must contain recoverable Rust panics before return.
- The repo already proves isolated leaf bindings with local `[workspace]`,
  exact path/version dependency on root `stegoeggo`, and
  `panic = "unwind"`.

No further public architecture decision should be made by M009.

## 3. Current implementation evidence

Before M009:

- `bindings/c` contains contract documentation only;
- no C crate/header/library exists;
- root/carrier forbid unsafe;
- Python/Node bindings demonstrate canonical semantic projection;
- M006 keeps resource-limit and verification semantics current;
- root MSRV is Rust 1.89;
- standard required CI is `./scripts/check.sh` and must not acquire C/cbindgen
  prerequisites.

The C implementation therefore starts as a clean leaf rather than extracting
existing unsafe code.

## 4. Invariants that must not regress

- Export exactly the 90 M008-manifest symbols and no other
  `stegoeggo_*` symbol.
- No Rust-native layout crosses C.
- Opaque handle Rust structs remain non-`repr(C)` so cbindgen forward-declares
  them rather than emitting fields.
- All raw pointer dereference/slice/string construction is confined to
  `bindings/c`.
- Root and carrier `#![forbid(unsafe_code)]` remain untouched.
- C input pointers are borrowed only for the call.
- Rust output memory is freed only through matching ABI free functions.
- Full-width seeds are `uint64_t`; seed 0 and `u64::MAX` are valid.
- MAC key has no getter and is not serialized/logged.
- Canonical Rust `ProtectionRequest` and `verify_image_bytes_report` remain
  semantic authorities.
- Verification status is not reinterpreted.
- All current Rust `Error` variants map exactly to the frozen ABI categories.
- Unknown future non-exhaustive Rust errors map to internal.
- No Rust panic unwinds into C.
- No global/TLS last-error state.
- Same-handle concurrency remains unsupported.
- Header is generated, not handwritten.
- cbindgen stays outside `build.rs` and root build dependencies.
- Python/Node bindings do not move onto C.
- M009 does not mark C ABI stable in `STABILITY.md`.

## 5. Scope

### In scope

- `bindings/c/Cargo.toml` + lockfile.
- `bindings/c/src/**`.
- `bindings/c/cbindgen.toml`.
- committed `bindings/c/include/stegoeggo.h`.
- header-generation/check scripts.
- manifest/export checker reuse.
- C/C++ consumer tests/examples.
- parity scripts.
- `.github/workflows/c-binding.yml`.
- binding README implementation/build section.
- M009 closure evidence.

### Explicitly out of scope

- Five-platform release workflow.
- ARM/macOS/Windows native qualification.
- GitHub Release publication.
- staticlib.
- installer integration.
- package managers.
- async/callback/batch/file APIs.
- C++ wrapper classes.
- ABI v2.
- Python/Node implementation changes.
- root required CI expansion.
- release-distribution milestone changes.

## 6. Required production changes

### A. Create isolated leaf crate

Create `bindings/c/Cargo.toml`:

- package: `stegoeggo-c`;
- source version exactly matches root;
- edition 2021;
- rust-version 1.89;
- `publish = false`;
- local empty `[workspace]`;
- `[lib] name = "stegoeggo_c"`, `crate-type = ["cdylib"]`;
- exact path/version dependency:
  `stegoeggo = { path = "../..", version = "=<root>", default-features = false }`;
- direct `serde_json = "1.0"` only if needed for details/report serialization;
- no FFI framework dependency.

Profiles:

- dev/release `panic = "unwind"`;
- do not enable symbol stripping until export qualification proves it harmless;
- LTO/codegen choices may mirror other bindings after correctness.

Do not add the crate to the root workspace.

### B. Organize the unsafe boundary

Use a small module structure such as:

- `codes.rs` — frozen scalar aliases/constants and Rust conversion;
- `handles.rs` — opaque Rust-owned handle structs;
- `input.rs` — pointer/length/UTF-8 conversion helpers;
- `error.rs` — ABI error DTO/handle and Rust Error mapping;
- `panic.rs` — common `catch_unwind` wrappers;
- `notice.rs`, `request.rs`, `limits.rs` — builders;
- `operations.rs` — detect/protect/verify;
- `report.rs` — execution/verification getters + JSON;
- `buffer.rs` — owned bytes;
- `lib.rs` — public export assembly.

Exact files may differ, but raw-pointer mechanics must not be duplicated
through dozens of exports.

At crate level enable `#![deny(unsafe_op_in_unsafe_fn)]`.

### C. Implement opaque handles

Rust handle structs own canonical/native values, for example conceptually:

- notice -> `RightsNotice`;
- request -> `ProtectionRequest`;
- limits -> `ResourceLimits`;
- buffer -> `Vec<u8>`;
- execution report -> `ExecutionReport`;
- verification report -> `VerificationReport`;
- error -> ABI-owned error DTO.

Do NOT mark these handle structs `repr(C)`; cbindgen must emit forward
declarations only.

Use `Box::into_raw` for owned handle transfer and recover ownership only in
the matching free function.

Free(NULL) is a no-op.

### D. Centralize pointer validation

Provide helpers for:

- required `*const T` / `*mut T` handles;
- optional handles;
- byte pointer + length;
- mutable out-pointer initialization;
- UTF-8 pointer + length;
- exact 4-byte content hash;
- optional MAC key with empty-key rules from ABI-V1.

Rules:

- validate NULL/length combinations before creating slices;
- check pointer arithmetic/length conditions before conversion;
- use `from_raw_parts` only after validation;
- copy borrowed caller bytes into canonical owned state whenever the request
  must outlive the call;
- arbitrary invalid non-NULL pointers remain caller UB per contract.

### E. Implement fixed code conversions

Never transmute numeric input to Rust enums.

Use explicit `match` conversions for every input code:

- policy;
- DMI;
- image format;
- metadata update policy;
- preset;
- authentication mode;
- hidden marker mode;
- presence flags/tile constraints.

Unknown values return invalid argument/config.

Output enum-like values map explicitly from Rust variants. Future non-exhaustive
Rust variants must hit a defined fallback or block compilation/test, never leak
an accidental discriminant.

### F. Implement notice/request/limit builders

Implement every normative setter from the corrected manifest.

Semantics:

- setters mutate the leaf handle then return status;
- request constructors clone the notice canonical value;
- applying resource limits snapshots/clones them;
- MAC key copied into canonical request and never returned;
- request free relies on canonical `ProtectionRequest::drop` zeroization;
- seed presence flag distinguishes absent from zero;
- output format presence flag distinguishes same-as-input from explicit format;
- tiled mode validates tile-size pairing;
- all 18 resource-limit setters preserve `usize` range safely across
  32/64-bit C callers (first qualified targets are 64-bit, but conversion
  must still be explicit).

### G. Implement common fallible-export wrapper

All fallible exports use one pattern:

1. validate required output pointers;
2. NULL/zero-initialize owned outputs immediately;
3. clear `*out_error` to NULL when supplied;
4. validate input pointers/scalars;
5. execute canonical work inside
   `catch_unwind(AssertUnwindSafe(...))`;
6. map canonical errors to frozen code/resource/details;
7. map panic to internal with fixed non-secret message;
8. return without unwinding.

Do not expose panic payload/debug text.

Free/accessor functions also must not unwind. Use internal containment/sentinel
behavior appropriate to their fixed return type; do not add non-normative error
symbols.

Provide an internal test seam that deliberately panics inside the common
wrapper without exporting an extra `stegoeggo_*` function.

### H. Implement operations exactly

Implement:

- detect format;
- protect;
- protect with report;
- verify.

Protect routes through canonical byte request APIs.

Verification routes through `verify_image_bytes_report`.

No direct carrier API calls are allowed.

All returned bytes/reports/errors are newly owned ABI handles.

### I. Implement error and JSON surfaces

Map every current root `Error` variant to ABI-V1 exactly.

For error details JSON:

- `schema_version = 1`;
- base code/resource/message always present as contracted;
- structured fields only when Rust has them;
- carrier resource limit has message/category only;
- unknown future/internal/panic has internal category and no invented facts.

Verification report JSON serializes the canonical `VerificationReport`
directly using its stable serde schema.

No execution-report JSON in v1.

### J. Generate the header

Pin standalone cbindgen 0.29.4.

Create `cbindgen.toml`:

- `language = "C"`;
- `cpp_compat = true`;
- explicit include guard;
- `usize_is_size_t = true`;
- deterministic sorting;
- only ABI public items;
- C11-compatible scalar definitions/constants;
- no struct bodies for opaque handles.

Important upstream constraint: cbindgen emits forward declarations for Rust
types without guaranteed layout. Verify this output; do not use zero-sized
`repr(C)` tricks.

Create a generation script that:

1. invokes exact cbindgen;
2. writes to a temp file;
3. validates manifest/count;
4. atomically replaces `include/stegoeggo.h` only on success.

Header drift check regenerates to temp and `diff`s against committed header.

If cbindgen 0.29.4 cannot parse the selected rustc export-attribute syntax,
stop. Do not hand-maintain the header. Use the edition-2021-compatible export
form accepted by Rust 1.89 and the pinned generator, or write a contract review
before changing generator version.

### K. Add Linux export audit

After release build on Linux x86_64:

- extract defined dynamic `stegoeggo_*` symbols with `nm -D --defined-only`;
- normalize/sort;
- compare byte-for-byte with `abi-v1-symbols.txt`;
- assert 90 total.

Any extra/missing symbol blocks M009 closure.

### L. Add C consumer tests

Commit C11 consumers that exercise:

- bootstrap versions;
- notice/request/limits lifecycle;
- metadata-only protect;
- hidden marker protect;
- HMAC protect/verify;
- verification JSON;
- resource-limit error + details JSON;
- malformed input;
- invalid UTF-8;
- invalid numeric constants;
- null required outputs;
- NULL-free no-op;
- buffer data/len/free;
- execution-report getters/warnings;
- verification-report getters;
- cleanup after mid-operation failure;
- seed 0;
- seed `UINT64_MAX`;
- secret non-disclosure.

Do not intentionally double-free/dereference bogus pointers in normal tests;
those are documented caller UB, not valid inputs.

### M. Add Rust tests

At minimum:

- every input constant conversion;
- every output enum projection;
- every root Error variant mapping;
- details JSON fields;
- carrier limit message-only behavior;
- unknown/future fallback helper;
- pointer/null/length helper cases that can be tested safely;
- panic wrapper maps deliberate panic to internal;
- handle free NULL no-op;
- version constants;
- manifest/function-count consistency.

### N. Add Rust↔C parity

On Linux x86_64:

- C protects deterministic fixture with explicit seed + timestamp; Rust/CLI
  verifies it;
- Rust/CLI protects equivalent fixture; C verifies it;
- compare deterministic bytes where the existing canonical parity oracle says
  equality is valid;
- compare verification status/evidence facts;
- test HMAC correct/wrong/missing-key cases.

Reuse canonical fixtures; do not fork them into a C-only authoritative copy.

### O. Add lightweight `c-binding.yml`

Separate path-filtered workflow, not part of root required CI.

Trigger on changes to:

- `bindings/c/**`;
- `src/**`;
- `stegoeggo-stego/**`;
- root manifests;
- canonical fixture paths;
- workflow itself.

Ubuntu x86_64 job:

- Rust 1.89;
- install exact cbindgen 0.29.4;
- Cargo check/clippy/test for C manifest;
- header generation drift;
- release cdylib build;
- 90-symbol audit;
- C11 compile/link/run suite;
- C++17 header compile;
- Rust↔C parity.

Upload the Linux test library/header as CI evidence only, not release
distribution.

## 7. Ordered work packages

### WP1 — Crate + types/constants

Scaffold isolated crate and implement fixed code tables/opaque handles.

Acceptance:

- no root workspace change;
- header generator sees only forward-declared handles;
- all numeric constants match ABI-V1.

### WP2 — Pointer/panic/error foundation

Implement centralized conversions, output initialization, panic wrapper, and
error mapping before business operations.

Acceptance:

- Rust unit matrix green;
- deliberate internal panic becomes internal error;
- no panic payload exposed.

### WP3 — Builders

Implement notice/request/resource-limit symbols.

Acceptance:

- every builder symbol in manifest implemented;
- max-width seed/resource values preserved;
- invalid constants/ranges fail deterministically;
- secrets never readable.

### WP4 — Operations/buffer/report

Implement all remaining operations and handle accessors.

Acceptance:

- all 90 symbols link;
- canonical byte APIs are the only operation path;
- verification JSON matches Rust serde output.

### WP5 — Header + manifest gate

Generate committed header and compare dynamic exports on Linux.

Acceptance:

- no header drift;
- C11/C++17 compile;
- exports exactly equal 90-line manifest.

### WP6 — C integration + parity

Implement native C lifecycle/error/parity suite.

Acceptance:

- deterministic two-way parity green;
- cleanup paths leak no owned handles in test logic;
- HMAC/resource/malformed cases green.

### WP7 — Lightweight CI + docs

Add `c-binding.yml`, build docs, and closure evidence.

Acceptance:

- root CI remains unchanged;
- c-binding workflow green;
- STABILITY still says planned/not stable/not shipped.

## 8. Failure, cancellation, restart, and contention semantics

- On any fallible call, output handles are initialized NULL before work.
- A failed call never transfers a partially initialized owned output handle.
- If an error handle cannot be produced, numeric status still carries failure.
- Free(NULL) is safe; double-free remains caller UB.
- Same mutable handle concurrent access remains unsupported.
- Independent handles can execute concurrently; no global mutable state.
- Panic is contained to internal status when recoverable via unwind.
- OOM/abort/process-fatal conditions remain outside the recoverable guarantee.
- Retrying a failed operation is caller-owned; ABI maintains no hidden
  transactional/global state.
- If implementation requires a 91st symbol, stop and return to contract
  planning rather than exporting it.

## 9. Compatibility and migration

No existing C consumers.

M009 implements but does not activate ABI stability. The committed header and
library remain pre-qualified until M010 closure.

Python/Node/Rust contracts remain unchanged.

## 10. Required tests

At minimum:

1. ABI major/minor/source version;
2. 90-symbol manifest equality;
3. header drift;
4. C11 syntax;
5. C++17 syntax;
6. create/free every handle;
7. free NULL every handle;
8. notice all text setters;
9. notice DMI/seed including zero/max;
10. all three request constructors;
11. all 14 request setters;
12. all 18 limit setters;
13. unknown numeric constants rejected;
14. invalid UTF-8 rejected;
15. NULL/nonzero-length rejected;
16. content hash !=4 rejected;
17. tile-mode/range mismatch rejected;
18. protect returns buffer;
19. protect-with-report getters;
20. warning indexing + sentinel;
21. verify unprotected => NotFound;
22. metadata-only rights + marker NotFound;
23. hidden marker => Verified;
24. HMAC correct;
25. HMAC wrong;
26. HMAC missing;
27. malformed/truncated input;
28. insufficient capacity structured details;
29. input resource limit details;
30. dimensions/container/metadata/verification-budget projections;
31. carrier resource limit => carrier/message only;
32. verification JSON exact serde equivalence;
33. error details schema_version 1;
34. error message data/len lifetime;
35. buffer data/len lifetime;
36. secret non-disclosure;
37. internal panic seam;
38. C protect -> Rust/CLI verify;
39. Rust/CLI protect -> C verify;
40. deterministic byte parity.

## 11. Required verification commands

Root:

```bash
./scripts/check.sh
```

C binding Rust:

```bash
cargo check --manifest-path bindings/c/Cargo.toml --locked
cargo clippy --manifest-path bindings/c/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path bindings/c/Cargo.toml --locked
cargo build --manifest-path bindings/c/Cargo.toml --locked --release
```

Header:

```bash
cargo install cbindgen --version 0.29.4 --locked
bindings/c/scripts/generate-header.sh --check
python3 bindings/c/scripts/check-contract.py
```

Linux symbols:

```bash
nm -D --defined-only bindings/c/target/release/libstegoeggo_c.so \
  | awk '{print $3}' | grep '^stegoeggo_' | LC_ALL=C sort > /tmp/c-exports.txt
diff -u bindings/c/abi-v1-symbols.txt /tmp/c-exports.txt
```

C/C++:

```bash
cc -std=c11 -Wall -Wextra -Werror <consumer sources> -Ibindings/c/include \
  -Lbindings/c/target/release -lstegoeggo_c -o /tmp/stegoeggo-c-smoke
LD_LIBRARY_PATH=bindings/c/target/release /tmp/stegoeggo-c-smoke
c++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only <header smoke>
```

Parity command names may be wrapped in scripts; closure records the exact final
commands.

## 12. Documentation updates

Update/create:

- `bindings/c/README.md`;
- `bindings/c/include/stegoeggo.h`;
- `bindings/c/cbindgen.toml`;
- C examples/tests docs;
- language-bindings roadmap/registry;
- `plans/closure/language-bindings/009-status.md`.

Keep `STABILITY.md` explicitly pre-stable until M010.

## 13. Acceptance criteria

M009 closes only when:

- M008 is closed;
- isolated C crate builds on Rust 1.89;
- all 90 manifest functions are implemented;
- Linux dynamic exports exactly equal manifest;
- generated header is drift-free and opaque;
- C11 and C++17 checks pass;
- Rust unit/error/panic tests pass;
- C lifecycle/operation/error tests pass;
- Rust↔C parity passes;
- path-filtered c-binding workflow is green;
- root `./scripts/check.sh` is green;
- no stable/shipped ABI claim is made.

M010 then becomes dependency-ready.

## 14. Stop conditions

Stop and report if:

- pinned cbindgen cannot generate the exact accepted header without manual
  declarations;
- a handle body leaks into the generated header;
- implementing a signature requires exposing Rust layout;
- panic containment requires `extern "C-unwind"`;
- any contract operation requires a new canonical Rust API;
- dynamic exports cannot be limited to the manifest without changing ABI
  design;
- implementation needs a new public symbol;
- root/core unsafe policy would need relaxation;
- deterministic parity reveals semantic drift from Python/Node/Rust.

## 15. Closure evidence required

Record:

- M008 closure reference;
- implementation commits;
- resolved dependency/tool versions;
- exact crate manifest/profile;
- unsafe-boundary module inventory;
- count of unsafe blocks/functions and audit disposition;
- all 90 export evidence;
- header generation cbindgen version/result;
- header C11/C++17 evidence;
- Rust test counts;
- C consumer test counts;
- parity results;
- panic test evidence;
- resource/error matrix;
- root check result;
- c-binding workflow run ID/SHA;
- confirmation no stable ABI/publication;
- unresolved findings;
- transition making M010 ready.

## 16. Handoff notes

Implement the contract mechanically. Do not improve it while implementing it.

The most important engineering property is that there is one small set of
raw-pointer/panic/error helpers and 90 thin exports over that foundation.
Duplicated unsafe pointer logic across exports is a corrective-worthy defect.

Use current cbindgen behavior intentionally: non-guaranteed Rust handle layout
should produce C forward declarations. Never add `repr(C)` to make cbindgen
happy for an opaque object.

Research references:

- Rust system-library linkage:
  https://doc.rust-lang.org/reference/linkage.html
- Rust FFI/panic rules:
  https://doc.rust-lang.org/reference/panic.html
  https://doc.rust-lang.org/reference/items/functions.html
- cbindgen current documentation/config:
  https://github.com/mozilla/cbindgen/blob/main/docs.md
  https://github.com/mozilla/cbindgen/blob/main/template.toml
