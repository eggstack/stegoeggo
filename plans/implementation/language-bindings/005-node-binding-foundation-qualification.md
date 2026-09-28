# Language Bindings Milestone 005 — Node.js Binding Foundation and Qualification

Status: ready for handoff

Repository baseline: `32f4f096d07849bba7b14cb72aff115239c7eb8f`

Source roadmap:
`plans/subsystems/language-bindings-roadmap.md#m005--nodejs-binding`

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

Primary class: capability

## 1. Objective

Add the first Node.js frontend for StegoEggo as an isolated napi-rs v3 leaf
binding under `bindings/node/`, directly over the canonical Rust byte API.

The milestone must deliver one coherent, typed Node package surface that:

- accepts encoded image bytes as `Buffer` / `Uint8Array`;
- exposes canonical `ProtectionRequest` / `RightsPolicy` semantics rather
  than legacy protection adapters;
- performs CPU-heavy protection and verification off the JavaScript event-loop
  thread;
- preserves structured Rust errors as stable JavaScript errors;
- preserves full-width Rust `u64` seed values without JavaScript number
  truncation;
- produces generated TypeScript declarations that are tested as part of the
  contract;
- proves Rust/Node interoperability and deterministic parity;
- builds and smoke-tests native binaries for the same five host targets already
  qualified by the Python binding;
- assembles the napi-rs root/per-platform package layout without publishing to
  npm.

No npm publication occurs in M005.

## 2. Why this milestone is ready

The hard dependencies are closed:

- M001 Python foundation — closed;
- M002 Python packaging — closed after M003/M004 qualification evidence;
- M003 Python corrective qualification — closed;
- M004 closure corrective pass — closed;
- ADR-0005 already fixes the architecture: Node uses napi-rs directly against
  `stegoeggo`, not the future C ABI and not a new shared bindings-core crate.

The current Rust surface is binding-ready:

- `process_request_bytes`;
- `process_request_bytes_with_warnings`;
- `process_request_bytes_with_report`;
- `verify_image_bytes_report`;
- cloneable `ProtectionRequest`, `ProcessingOptions`, `RightsNotice`,
  and `ResourceLimits`;
- deterministic seed/timestamp overrides;
- existing canonical conformance fixtures;
- structured root error variants already projected successfully into Python.

Current napi-rs v3 is compatible with the repository MSRV: upstream documents
Rust 1.88+ while StegoEggo requires Rust 1.89. The current napi-rs build CLI
requires a modern Node runtime and the upstream source matrix exercises Node
22, 24, and 26. Node 22 and 24 are current LTS lines and Node 26 is current at
planning time.

No new architecture ADR is required.

## 3. Current implementation evidence

At baseline there is no Node package:

- no `bindings/node/`;
- no root `package.json`;
- no pnpm/yarn/npm workspace;
- no TypeScript configuration;
- no Node-specific GitHub Actions workflow.

The root workspace already has the correct isolation boundary:

```toml
[workspace]
members = [".", "stegoeggo-stego", "stegoeggo-cli", "fuzz"]
exclude = ["bindings"]
```

The Python binding establishes useful foreign-runtime precedent without being
an implementation dependency:

- leaf Cargo package with its own `[workspace]`;
- exact `stegoeggo = "=0.4.2"` source-version alignment;
- binding release profile `panic = "unwind"`;
- no binding dependencies in core/carrier/CLI;
- canonical request/protect/verify API only;
- structured capacity/resource-limit error fields;
- no secret-key exposure;
- separate lightweight binding CI;
- separate manual artifact-only release qualification.

The authoritative repository conformance fixture set remains
`tests/fixtures/conformance/`. Node tests should consume that source rather
than creating another binding-specific copy. Package-install smoke tests that
run independently of the checkout should create their own tiny valid input in
the smoke script.

## 4. Invariants that must not regress

- The Rust application crate and `stegoeggo-stego` remain
  `#![forbid(unsafe_code)]`.
- Node/napi dependencies never enter root, carrier, or CLI dependency graphs.
- `bindings/node` remains outside the root Cargo workspace.
- `./scripts/check.sh` remains the required Rust gate and gains no Node,
  pnpm, napi-rs, or TypeScript prerequisite.
- Root/CLI release builds retain `panic = "abort"`; the Node addon uses
  `panic = "unwind"`.
- Every protection operation delegates to `ProtectionRequest` +
  `process_request_bytes*`.
- Verification delegates to `verify_image_bytes_report`.
- Metadata-capable operations use encoded bytes, never a pixel-only FFI path.
- No deprecated `ProtectionLevel`, `ProtectionContext`, or
  `EvidenceProfile` surface is promoted into Node.
- Resource-limit behavior is preserved for untrusted inputs.
- MAC/HMAC key material never appears in error messages, error properties,
  object inspection, JSON, generated declarations, or logs.
- CPU-heavy protection/verification must not run synchronously on the
  JavaScript event-loop thread.
- JavaScript-owned Buffer/TypedArray backing storage must not be read from a
  worker while JavaScript can mutate it; input bytes are copied into Rust-owned
  storage before worker dispatch.
- `u64` seed values are never coerced through JavaScript `number`; the Node
  contract uses `bigint`.
- No M005 workflow invokes `npm publish`, `pnpm publish`,
  `napi pre-publish`, or any command with registry side effects.
- C ABI work does not begin until M005 closes.

## 5. Scope

### In scope

- New isolated `bindings/node/` Rust + Node package.
- napi-rs v3, modern v3 API only; no `compat-mode`.
- Maintained pnpm-template conventions adapted to this existing monorepo.
- Generated napi loader and TypeScript declarations.
- Node-API level 6 or higher only if concrete used APIs require it; initial
  contract is `napi6` because full-width `u64` seeds are represented as
  JavaScript BigInt.
- Runtime support contract beginning at Node 22.13.0.
- Blocking runtime tests on Node 22, 24, and 26.
- Canonical request builders and canonical result/report projections.
- Async protect/protect-with-warnings/protect-with-report/verify operations.
- Synchronous, bounded `detectFormat`.
- Stable structured JavaScript error codes and properties.
- Rust-owned input copies before worker dispatch.
- Buffer output.
- Type declaration drift/compile tests.
- Rust↔Node parity and deterministic-output tests.
- Lightweight Node binding CI separate from root CI.
- Manual artifact-only five-target native build/qualification workflow.
- napi-rs per-platform package directory/artifact assembly.
- Package tarball inspection / clean local package smoke with no registry
  publication.
- Node support/release documentation.

### Explicitly out of scope

- npm publication, npm credentials, provenance publication, or namespace
  reservation as a release action.
- Automatic registry publication.
- Bun, Deno, Electron-specific qualification, browser/WASM, or WASI fallback.
- Linux musl packages.
- Windows x86 or arm64.
- macOS universal binaries.
- Android, FreeBSD, OpenHarmony, or other accepted napi target triples.
- JavaScript file helpers such as `protectFile` / `verifyFile`; byte APIs
  close first and file helpers remain later polish.
- Streaming APIs.
- Batch/parallel Node APIs.
- Callbacks; the public CPU-heavy API is Promise-based.
- Cancellation/AbortSignal in the first milestone.
- A shared `bindings-core` crate.
- C ABI work.
- Root workspace membership for the Node crate.
- Changes to canonical Rust semantics made only for FFI convenience.

## 6. Required production changes

### A. Package/crate layout

Create `bindings/node/` with at least:

```text
bindings/node/
  Cargo.toml
  Cargo.lock
  build.rs
  package.json
  pnpm-lock.yaml
  src/
    lib.rs
    ... bounded projection modules as useful
  test/
    *.test.mjs
    types/
      contract.ts
  index.js
  index.d.ts
  README.md
  .gitignore
```

Use a local `[workspace]` in the binding Cargo manifest so release profiles
and lock state are binding-owned.

Minimum Rust metadata:

- crate name: `stegoeggo-node`;
- version: exact root source version (`0.4.2` at this baseline);
- edition 2021;
- `rust-version = "1.89"`;
- `publish = false` for the Rust crate;
- `crate-type = ["cdylib"]`;
- exact path/version dependency on root `stegoeggo`, default features off;
- napi-rs v3 + napi-derive v3;
- napi-build v3 as build dependency;
- no napi `compat-mode`;
- Node-API feature floor `napi6` unless implementation evidence proves a
  higher floor is required.

Release and dev profiles must use `panic = "unwind"`. Release optimization
may mirror the Python binding (`lto`, one codegen unit, stripped symbols,
size optimization) unless native-addon debugging evidence justifies a
different tradeoff.

Use the maintained napi-rs pnpm template as a configuration reference, not as
an excuse to import unrelated template features (WASI, benchmark harnesses,
publish automation, alternate targets).

Use a scoped provisional JavaScript package identity
`@eggstack/stegoeggo` and binary name `stegoeggo`. This is an unpublished
development identity in M005; registry ownership/availability is a release
decision, not an M005 acceptance condition. If package tooling requires a
different unpublished identity, stop and record the reason rather than
silently creating a public naming commitment.

Commit the pnpm lockfile and the exact `packageManager` version emitted or
selected at implementation time.

### B. Node-API/runtime contract

Set the package runtime floor to Node `>=22.13.0`.

Blocking compatibility evidence must cover:

- Node 22 LTS;
- Node 24 LTS;
- Node 26 current.

The addon uses Node-API 6 initially because the public request/report contract
needs exact `u64` conversion to JavaScript BigInt and Node-API BigInt APIs
are available at level 6. Do not opt into Node-API 9/10 merely because current
Node versions provide them. Raise the selected Node-API level only when a
concrete napi-rs API used by this binding requires it; record that change in
closure evidence.

Distinguish in docs:

- package `engines.node` floor;
- Node-API ABI level;
- Node versions actually tested;
- napi-rs CLI build-time Node requirement.

Do not claim “all Node versions”.

### C. Public semantic surface

Match the final Python binding's canonical capability, but use JavaScript
naming conventions.

Required request/configuration types:

- `RightsPolicy`;
- `DmiValue`;
- `ImageOutputFormat`;
- `MetadataUpdatePolicy`;
- `ProtectionPreset`;
- `AuthenticationMode`;
- `HiddenMarkerMode`;
- `RightsNotice`;
- `ProcessingOptions`;
- `ResourceLimits` / builder;
- `ProtectionRequest`.

Required result/status types:

- `ProtectionWarning`;
- `ExecutionReport`;
- `ResourceUsage`;
- `VerificationStatus`;
- `EvidenceStrength`;
- `VerificationReport`.

For request/configuration state with nontrivial validation or secret material,
prefer native classes/builders that own canonical Rust values rather than
duplicating request semantics in a large JavaScript object translator.
Read-only report/result values may be projected as typed JavaScript objects
where that makes the API simpler.

Use camelCase JavaScript/TypeScript method/property names. Preserve canonical
domain terminology and explicit policy values.

Enum-like values must have explicit stable JavaScript representations. Prefer
string-valued exports/unions where napi-rs can generate them without runtime
ambiguity. Generated TypeScript text is not itself proof of runtime behavior;
tests must assert actual exported values.

### D. Numeric boundary

JavaScript `number` cannot exactly represent all Rust `u64` values.

Therefore:

- `ProtectionRequest.withSeed` accepts `bigint`;
- any exposed hidden-marker/resolved seed is `bigint | null`;
- deterministic parity tests include values above
  `Number.MAX_SAFE_INTEGER`;
- no seed path converts through `f64` / JS number.

Normal bounded counts/sizes/dimensions may use JavaScript `number` only when
the conversion validates non-negative safe integers and the underlying Rust
range. Reject NaN, infinity, fractions, negatives, and unsafe integers before
Rust narrowing.

### E. Async/event-loop boundary

Expose these CPU-heavy public operations as Promises:

```ts
protect(data, request): Promise<Buffer>
protectWithWarnings(data, request): Promise<{ data: Buffer; warnings: ProtectionWarning[] }>
protectWithReport(data, request): Promise<{ data: Buffer; report: ExecutionReport }>
verify(data, options?): Promise<VerificationReport>
```

Exact declaration syntax may differ based on generated napi types, but these
semantics are required.

Use napi-rs `AsyncTask` / `Task` (libuv worker pool) or an equivalently
bounded napi-rs worker mechanism. Do not add a Tokio runtime merely to wrap
synchronous CPU work.

At the synchronous JS→Rust entry boundary:

1. validate input type;
2. copy Buffer/Uint8Array bytes to Rust-owned `Vec<u8>`;
3. clone/own the canonical request/options required by the worker;
4. return the async task.

The worker may only touch Rust-owned state. Conversion back to Buffer/report
objects happens on the JavaScript thread through the napi task resolution
path.

`detectFormat` may remain synchronous because it only examines bounded
header bytes and does not run image decoding/protection/verification.

No public synchronous `protectSync` or `verifySync` is added in M005.

### F. Error contract

JavaScript callers need stable machine-readable errors, not message parsing.

Required stable `error.code` values:

- `ERR_STEGOEGGO_INVALID_CONFIG`;
- `ERR_STEGOEGGO_INVALID_FORMAT`;
- `ERR_STEGOEGGO_ENCODE_DECODE`;
- `ERR_STEGOEGGO_METADATA`;
- `ERR_STEGOEGGO_STEGANOGRAPHY`;
- `ERR_STEGOEGGO_INSUFFICIENT_CAPACITY`;
- `ERR_STEGOEGGO_VERIFICATION`;
- `ERR_STEGOEGGO_RESOURCE_LIMIT`;
- `ERR_STEGOEGGO_INTERNAL` for safe non-exhaustive/internal fallback.

Structured properties must match the final Python contract where applicable:

- insufficient capacity: `required`, `available`;
- input limit: `resource="input_bytes"`, `size`, `limit`;
- dimensions: `resource="dimensions"`, `width`, `height`,
  `maxWidth`, `maxHeight`;
- container limit: `resource="container"`, `kind`, `count`, `limit`;
- metadata limit: `resource="metadata"`, `kind`, `size`, `limit`;
- verification budget: `resource="verification_budget"`, `kind`,
  `count`, `limit`.

`ImageTruncated` maps to the encode/decode category.

The human-readable `message` remains useful but is not the machine contract.

napi-rs custom status types may be used to set `error.code`. If arbitrary
structured properties cannot be preserved reliably through
`AsyncTask::reject`, introduce a small internal native error DTO and a thin
JavaScript wrapper that constructs/rejects a normal `Error` with those
properties. Do not serialize fields into the message and parse them back.

The public Promise must reject on failure; an internal discriminated result is
acceptable only below the public wrapper.

Unknown future non-exhaustive Rust errors fall back safely to
`ERR_STEGOEGGO_INTERNAL`.

### G. Panic boundary

The Node binding must not inherit root `panic = "abort"`.

Keep `panic = "unwind"` in dev/release. Expected failures remain
`Result`-based.

Use napi-rs' supported panic-catching boundary where it actually covers the
executed Rust callback. For worker `Task::compute`, confirm during
implementation whether the selected napi-rs version already contains worker
panics; if it does not, wrap the canonical task call in a narrow
`catch_unwind` and map an unwind to `ERR_STEGOEGGO_INTERNAL`. Do not use
panic catching to hide ordinary application errors.

A process abort during malformed/untrusted-input tests is a stop condition.

### H. TypeScript declarations

Generate TypeScript declarations through napi-rs v3.

The generated declaration contract must include all public classes, values,
objects, Promise return types, Buffer types, bigint seed types, structured
report properties, and nullable/optional distinctions.

Commit the generated root loader/declaration files required by the napi-rs
package layout. Add a deterministic declaration drift check: after the normal
generation command, the tracked generated files must have no unexpected diff.

Add a small TypeScript compile-only contract fixture that imports the package
and exercises:

- metadata-only request construction;
- hidden-marker request construction;
- bigint seed;
- resource limits;
- `await protect`;
- `await protectWithReport`;
- `await verify`;
- structured report access;
- structured-error narrowing by `code`/properties.

Do not hand-edit generated napi declaration output to conceal a mismatched
runtime type.

### I. Fixtures and parity

Use `tests/fixtures/conformance/canonical/` as the repository-authoritative
fixture source for Node tests. Do not add a third copied canonical fixture set
under `bindings/node`.

Required parity directions:

1. Node protects → Rust verifies.
2. Rust/CLI protects → Node verifies.
3. Node and Rust produce byte-identical deterministic output when explicit
   seed + timestamp make the operation deterministic.
4. Where useful, Python and Node projections over the same Rust-produced bytes
   agree on canonical report facts, but M005 must not depend on Python at
   runtime.

Cross-language tests may invoke the locally built Rust CLI as a test oracle.
If lightweight Node CI intentionally omits the CLI, the complete local/manual
qualification path must still execute those parity tests before closure.

### J. Lightweight Node CI

Add `.github/workflows/node-binding.yml`, separate from `ci.yml`.

Trigger on changes to at least:

- `bindings/node/**`;
- root `src/**`;
- `stegoeggo-stego/**`;
- root `Cargo.toml` / `Cargo.lock`;
- authoritative conformance fixtures used by Node tests;
- the workflow itself.

Preferred shape:

1. Linux x86_64 build job with Rust 1.89 + Node 24, pnpm lockfile install,
   addon build, generated-file drift check, Node unit/integration tests, and
   TypeScript contract check.
2. Reuse the built addon/package artifact for runtime smoke under Node 22, 24,
   and 26 rather than recompiling Rust three times unless artifact reuse proves
   materially more complex than the saved work.

The workflow must remain a binding-specific signal. Do not add Node steps to
`./scripts/check.sh`.

### K. Five-target manual qualification workflow

Add `.github/workflows/release-node.yml`.

It is `workflow_dispatch` only, has read-only repository permissions unless
artifact upload requires the normal token, and performs no registry or GitHub
Release publication.

Initial native package matrix:

| Target | Runner | Build rule |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `ubuntu-24.04` | napi build with `--use-napi-cross` |
| `aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` | napi build with `--use-napi-cross` |
| `x86_64-apple-darwin` | `macos-15-intel` | native |
| `aarch64-apple-darwin` | `macos-14` or current native arm64 equivalent | native |
| `x86_64-pc-windows-msvc` | `windows-2022` | native |

Linux `--use-napi-cross` is required so the addon does not inherit the
runner's newer glibc floor; napi-rs v3's cross toolchain targets glibc 2.17.
Document that this is the addon's libc build floor, not a promise that every
Node binary itself runs on glibc 2.17.

Every target must:

- build one correctly suffixed `.node` artifact;
- load it on a native runner of matching OS/architecture;
- execute import + protect + verify smoke;
- assert source/binding version;
- assert Buffer output;
- assert at least one Promise-based operation;
- emit no registry publication.

Use Node 22.13+ for the per-platform oldest-supported-line smoke. Linux x86_64
also records runtime compatibility on Node 22, 24, and 26.

### L. Package assembly without publication

After downloading all five target artifacts into one collection job:

1. run `napi create-npm-dirs`;
2. run `napi artifacts --output-dir artifacts --npm-dir npm`;
3. assert exactly five expected platform package directories/artifacts exist;
4. inspect each generated platform `package.json` for correct
   `os`/`cpu`/`libc` constraints;
5. inspect the root generated loader and exact-version optional-dependency
   mapping produced for packaging;
6. create local tarball(s) or use package-manager pack/dry-run commands that
   have no registry side effects;
7. perform a clean local install/load smoke for the current collector platform
   from the assembled local package material when the generated optional
   dependency layout permits a faithful local install.

Do not call `napi pre-publish` in M005. Its normal behavior has registry and
release side effects.

If the tooling cannot faithfully test root-package optional dependency
selection without a real registry, record that boundary accurately: qualify
the native platform packages, loader, metadata, and local collector-platform
assembly, but do not claim registry-install qualification.

## 7. Ordered work packages

### WP1 — Scaffold the isolated Node leaf package

Intent: establish a reproducible napi-rs v3 package without contaminating the
root workspace.

Changes:

1. create `bindings/node` Cargo/package layout;
2. use current maintained pnpm-template conventions;
3. add exact root source-version dependency;
4. configure Node-API 6, five target triples, binary name, generated TS;
5. set unwind profiles;
6. commit Cargo/pnpm lockfiles;
7. add local build/test scripts.

Acceptance evidence:

- `cargo check --manifest-path bindings/node/Cargo.toml --locked`;
- frozen pnpm install succeeds;
- local napi build creates a loadable addon;
- root `cargo metadata --workspace` does not include the Node crate.

### WP2 — Canonical request/type projection

Intent: expose the canonical application model without inventing Node-only
semantics.

Changes:

1. map policy/preset/format/status/warning enums;
2. implement `RightsNotice`, `ProcessingOptions`, `ResourceLimits`,
   `ProtectionRequest` builders/classes;
3. use bigint for u64 seeds;
4. validate JS numeric inputs before narrowing;
5. project reports/resource usage/verification facts;
6. generate/test declarations.

Acceptance evidence:

- TypeScript fixture compiles;
- request builder semantics match Rust/Python test vectors;
- no deprecated API appears in runtime exports or declarations;
- bigint seed > MAX_SAFE_INTEGER round-trips exactly.

### WP3 — Async operations and safe byte ownership

Intent: make the Node API usable in servers without event-loop blocking or JS
backing-store races.

Changes:

1. implement synchronous `detectFormat`;
2. implement Promise-based protect variants with `AsyncTask`;
3. implement Promise-based verify;
4. copy incoming Buffer/Uint8Array before task scheduling;
5. return Buffer/result objects only during JS-thread resolution;
6. preserve request/options ownership per task.

Acceptance evidence:

- Buffer and plain Uint8Array inputs both work;
- mutating the original JS array immediately after the native call cannot alter
  the bytes the worker processes;
- a representative large operation allows queued event-loop work to run before
  the Promise settles;
- concurrent operations complete without data races or cross-request state.

### WP4 — Structured error projection

Intent: preserve machine-readable failure semantics across Promise rejection.

Changes:

1. implement stable error-code mapping;
2. preserve all structured fields listed in §6F;
3. map truncation correctly;
4. add safe non-exhaustive fallback;
5. verify async rejection retains properties;
6. audit repr/inspection/logging for secret leakage.

Acceptance evidence:

- JS tests assert `instanceof Error`, `code`, `message`, and structured
  fields;
- no test parses the human message to recover data;
- secret key bytes never appear in `String(error)`, inspected properties, or
  reports.

### WP5 — Node parity and declaration contract

Intent: prove semantic equivalence rather than merely successful FFI calls.

Changes:

1. reuse root conformance fixtures;
2. Node-protect/Rust-verify;
3. Rust-protect/Node-verify;
4. deterministic byte equality;
5. PNG/JPEG/WebP metadata-only coverage;
6. hidden-marker + HMAC paths;
7. generated declaration drift check;
8. TypeScript compile fixture.

Acceptance evidence:

- all parity directions pass;
- deterministic hashes/bytes match canonical Rust output;
- generated declarations match runtime exports.

### WP6 — Lightweight binding CI

Intent: catch Rust API or napi projection drift before merge.

Changes:

1. add path-filtered `node-binding.yml`;
2. build once on Linux x86_64 with Rust 1.89;
3. run full Node tests + TS contract;
4. smoke the built addon on Node 22/24/26;
5. preserve existing Rust CI contract.

Acceptance evidence:

- binding/root-API change triggers the workflow;
- all three supported runtime lines load the same Node-API addon;
- `./scripts/check.sh` remains unchanged.

### WP7 — Five-target build and native smoke

Intent: qualify actual distributable binaries.

Changes:

1. add manual `release-node.yml`;
2. build the five-target matrix;
3. use napi-cross for GNU Linux;
4. upload target artifacts;
5. same-architecture native smoke each artifact;
6. record target suffixes/runner identities.

Acceptance evidence:

- all five build jobs succeed;
- all five native import/protect/verify smokes succeed;
- Linux artifacts show the intended gnu target and napi-cross build path;
- no target is called qualified from compilation alone.

### WP8 — Package assembly and clean-install evidence

Intent: verify napi-rs distribution structure without publishing.

Changes:

1. create platform package dirs;
2. collect artifacts;
3. validate generated package metadata;
4. create local pack artifacts;
5. clean-install/load current collector-platform package;
6. audit root optional-dependency mapping;
7. confirm no registry side effects.

Acceptance evidence:

- exactly five intended package targets;
- loader selects the expected local/platform binary in smoke;
- package archives contain no source secrets/build junk;
- npm registry remains untouched.

### WP9 — Documentation and closure

Intent: make support claims equal actual evidence.

Changes:

1. add Node section to `SUPPORT.md`;
2. add Node local-build/artifact procedure to `RELEASING.md`;
3. document API and runtime contract in `bindings/node/README.md`;
4. write `plans/closure/language-bindings/005-status.md`;
5. reconcile registry/roadmap;
6. unblock M006 only after closure acceptance.

Acceptance evidence:

- support docs name Node-API level, Node runtime lines, five native targets,
  glibc build floor, unpublished status, and alternate-runtime exclusions;
- closure cites actual workflow run IDs/artifacts.

## 8. Failure, cancellation, restart, and contention semantics

- A failed/cancelled Node binding or release-node workflow is not
  qualification evidence.
- Release matrix uses fail-fast false so one target failure does not hide other
  evidence, but every required target must pass before closure.
- Reruns after executable source/package/workflow changes must qualify the new
  exact SHA.
- Documentation-only commits after a fully qualified executable SHA do not
  invalidate native artifacts if closure proves the executable diff is empty.
- Async worker errors reject the public Promise; they do not return partial
  protected bytes.
- Input bytes are fully copied before worker scheduling; mutation of caller
  memory after the call cannot change the task input.
- A task owns its request/options and output until resolve/reject.
- No global mutable request/key state is introduced.
- If the libuv worker pool is saturated, work may queue; correctness must not
  depend on start order.
- No automatic retry of protection/verification occurs in the wrapper.
- Package artifact collection fails closed on missing/mismatched configured
  targets.
- A partial local pack/assembly run never triggers registry publication.
- If future publication is attempted in another milestone, immutable npm
  versions and partial platform publication require an explicit release
  failure policy; M005 does not consume versions.

## 9. Compatibility and migration

This is a new unpublished 0.x frontend, so no existing Node consumer contract
must be preserved. Still, the surface should begin conservatively because the
eventual first npm publication will create compatibility cost.

- Node names use canonical StegoEggo terminology.
- Node syntax may be idiomatic while semantics match Python/Rust.
- String enum values, error codes, structured fields, and Promise-vs-sync
  behavior are treated as compatibility-sensitive once M005 closes.
- `bigint` for seed is intentional and not later silently changed to
  `number`.
- Binding/package version tracks the wrapped root StegoEggo version.
- No migration impact to Rust, CLI, carrier, or Python consumers.
- The future C ABI must learn from both Python and Node closure evidence; Node
  does not pre-design C ownership/layout rules.

## 10. Required tests

At minimum:

1. module loads from generated loader;
2. module version equals wrapped StegoEggo version;
3. `detectFormat` for PNG/JPEG/WebP and unknown input;
4. RightsPolicy and other enum/string representations are stable;
5. RightsNotice builders/getters;
6. ProtectionRequest metadata-only;
7. ProtectionRequest hidden-marker;
8. preset construction;
9. ProcessingOptions projection;
10. ResourceLimits defaults/builder;
11. bigint seed at 0, normal values, `u64::MAX`, and above JS safe-number
    range;
12. invalid numeric JS inputs rejected before narrowing;
13. Buffer protect;
14. Uint8Array protect;
15. caller mutation after invocation does not race/change task input;
16. metadata-only PNG/JPEG/WebP protect;
17. hidden-marker protect;
18. protect-with-warnings;
19. protect-with-report;
20. verify unprotected;
21. verify metadata-only;
22. verify correct HMAC key;
23. verify wrong/missing HMAC key;
24. malformed/truncated input;
25. InsufficientCapacity structured error;
26. input resource-limit structured error;
27. dimension resource-limit structured error;
28. container resource-limit structured error where reachable;
29. metadata resource-limit structured error where reachable;
30. verification-budget structured projection via helper/unit test if not
    reachable through public API;
31. truncation error category;
32. unknown/internal fallback cannot panic the host;
33. secret key absent from error/report inspection;
34. concurrent Promise operations;
35. event-loop liveness during representative CPU work;
36. Node-produced → Rust verification;
37. Rust-produced → Node verification;
38. deterministic byte equality Rust↔Node;
39. generated `index.d.ts` drift check;
40. TypeScript consumer compile contract;
41. CommonJS `require` load if provided by generated package;
42. ESM `import` load;
43. Node 22 runtime smoke;
44. Node 24 runtime smoke;
45. Node 26 runtime smoke;
46. five same-native-platform artifact smokes;
47. package assembly metadata checks;
48. clean local pack/install smoke on collector platform.

Do not manufacture production Rust error paths merely to trigger an otherwise
unreachable variant. A focused binding-side projection unit test is acceptable
when the canonical Rust path does not currently emit the variant.

## 11. Required verification commands

Required root gate:

```bash
./scripts/check.sh
```

Binding Rust gate:

```bash
cargo check --manifest-path bindings/node/Cargo.toml --locked
cargo clippy --manifest-path bindings/node/Cargo.toml --locked --all-targets -- -D warnings
```

Node dependency/build/test gate (use the package's committed package-manager
version):

```bash
cd bindings/node
pnpm install --frozen-lockfile
pnpm build
pnpm test
pnpm typecheck
```

Generated contract drift:

```bash
cd bindings/node
pnpm build
git diff --exit-code -- index.js index.d.ts
```

At least one direct Rust↔Node parity command/script must build the local Rust
CLI first:

```bash
cargo build -p stegoeggo-cli
cd bindings/node
STEGOEGGO_CLI=../../target/debug/stegoeggo pnpm test:parity
```

Exact script naming may differ, but closure must record the final commands.

Package assembly rehearsal after target artifacts are present:

```bash
cd bindings/node
pnpm napi create-npm-dirs
pnpm napi artifacts --output-dir artifacts --npm-dir npm
pnpm pack
```

Do not execute `napi pre-publish` or any publish command.

Required remote qualification:

- `node-binding.yml` green on the implementation SHA;
- Node 22, 24, 26 runtime smokes green;
- manual `release-node.yml` dispatch on the exact implementation SHA;
- all five native build jobs green;
- all five native runtime smokes green;
- package assembly/metadata job green;
- collector-platform clean local pack/install smoke green;
- record run URL/ID, artifact names, target suffixes, and exact SHA.

## 12. Documentation updates

Update:

- `bindings/node/README.md`
  - install/build from source;
  - Promise-based byte API;
  - bigint seed contract;
  - structured error codes;
  - supported/tested Node versions;
  - unpublished/experimental status;
  - unsupported Bun/Deno/WASI/browser scope.
- `SUPPORT.md`
  - Node binding section;
  - Node-API level;
  - Node engine floor;
  - tested Node 22/24/26 lines;
  - five-target matrix;
  - GNU Linux addon glibc build floor;
  - distinction between configured and qualified rows until release-node
    evidence exists.
- `RELEASING.md`
  - manual Node artifact-build runbook;
  - package assembly commands;
  - explicit prohibition on registry publication in M005;
  - future publication warning that napi multi-platform publication is
    non-atomic and immutable.
- `plans/registry.md` and subsystem roadmap through closure.

Do not edit canonical `plans/000-002`; the Python → Node → C sequence is
unchanged.

## 13. Acceptance criteria

M005 closes only when all of the following are externally observable:

- `bindings/node` is isolated from the root Cargo workspace;
- Rust MSRV remains 1.89 and root `./scripts/check.sh` is green;
- napi-rs v3 is used without `compat-mode`;
- Node-API level is documented and justified by used APIs;
- package runtime floor is documented;
- Node 22, 24, and 26 load/test the addon;
- public protection/verification work is Promise-based and runs off-thread;
- JS input bytes are copied before worker execution;
- Buffer and Uint8Array are accepted;
- outputs are Buffer/report values;
- `u64` seed values use bigint with >MAX_SAFE_INTEGER parity evidence;
- structured error codes/properties are stable and tested;
- no secret key material leaks;
- canonical Rust/Node parity passes in both directions;
- deterministic output matches Rust for fixed seed/timestamp;
- generated TypeScript declarations compile against a consumer fixture;
- five target binaries build and pass same-native-architecture smoke;
- GNU Linux uses napi-cross rather than inheriting Ubuntu 24's glibc floor;
- napi per-platform package directories and root loader metadata are assembled
  and audited;
- a clean local package smoke succeeds where local optional-package selection
  can be faithfully exercised;
- no npm version is published or consumed;
- docs distinguish tested support from ABI possibility;
- closure evidence is accepted;
- only then does M006 C ABI design become dependency-ready.

## 14. Stop conditions

Stop and report rather than improvise if:

- napi-rs v3 requires a Rust version above the repository's declared MSRV 1.89;
- the needed async or BigInt API forces a Node-API floor above the planned
  value and changes the runtime contract materially;
- structured async errors cannot preserve machine-readable fields without a
  public API redesign;
- safe worker execution would require sharing mutable JS backing memory;
- the generated loader cannot support the five configured targets without
  manual unsafe/native loader code;
- a five-target artifact requires a new root/core dependency;
- Linux GNU cannot be built without raising the intended addon libc floor;
- Node packaging would require adding Node prerequisites to
  `./scripts/check.sh`;
- a clean local package assembly test would require publishing temporary npm
  versions;
- package naming/namespace decisions would become irreversible in M005;
- a requested feature requires WASI/browser/Bun/Deno or C ABI work;
- core Rust semantics must change to make the binding work;
- panic containment cannot be established without a wider safety design.

## 15. Closure evidence required

Create `plans/closure/language-bindings/005-status.md` with:

- implementation commit(s);
- exact napi/napi-derive/napi-build versions resolved in Cargo.lock;
- exact pnpm and `@napi-rs/cli` versions resolved in the Node lockfile;
- selected Node-API level and rationale;
- package `engines.node` value;
- public API/declaration inventory;
- structured error-code/property matrix;
- proof that seed uses bigint and a >MAX_SAFE_INTEGER parity vector;
- input-copy/thread-safety evidence;
- exact local Rust/Node test commands/results;
- `./scripts/check.sh` result;
- lightweight node-binding workflow run ID/SHA and Node 22/24/26 results;
- release-node run ID/SHA;
- five native artifact names and runners;
- Linux napi-cross/glibc-floor evidence;
- five native smoke outcomes;
- package assembly directory/artifact inventory;
- clean local pack/install result and any honest boundary that could not be
  tested without a registry;
- confirmation `napi pre-publish` / registry publication did not occur;
- support/release documentation updates;
- security/no-secret review;
- unresolved/deferred findings;
- roadmap/registry disposition making M006 ready only if all required evidence
  is accepted.

## 16. Handoff notes

Implement M005 as one dependency-ordered branch/PR, but do not start with the
cross-platform workflow.

Recommended execution order:

1. scaffold/package isolation;
2. request/report/type projection;
3. async byte operations;
4. structured errors;
5. local parity and TypeScript contract;
6. lightweight Node CI;
7. five-target release-node workflow;
8. native qualification;
9. package assembly;
10. closure.

The current Python binding is a semantic oracle, not a source-code template.
Reuse canonical terminology, error facts, fixture expectations, and release
lessons; do not make Node depend on Python.

Prefer small Rust projection modules over one very large `lib.rs` if doing so
keeps request, report, error, and task boundaries reviewable.

Do not add a shared bindings crate during this milestone merely because Python
and Node duplicate some projection code. ADR-0005 explicitly defers that
decision until real duplication exists and can be evaluated after Node closure.
