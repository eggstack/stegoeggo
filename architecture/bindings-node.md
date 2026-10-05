# Node Bindings (napi-rs)

**Sources:** `bindings/node/src/` — `enums.rs` (723) · `error.rs` (497) · `limits.rs` (428) · `notice.rs` (505) · `request.rs` (402) · `report.rs` (309) · `tasks.rs` (330) · `numeric.rs` (167) · `lib.rs` (101) · `src/bin/oracle.rs` (58) · `build.rs` (3); plus `bindings/node/index.d.ts` (809, generated) · `stegoeggo.d.ts` (142, hand-written) · `stegoeggo.js` (171, hand-written). Crate `stegoeggo-node` v0.4.2, `crate-type = ["cdylib"]`, `publish = false`, not published to npm.

A leaf napi-rs v3 addon that projects the canonical byte API to Node. It owns no protection logic: `src/tasks.rs` is the only file that calls the library, and it calls exactly four entry points. The crate is excluded from the root workspace (`Cargo.toml` `exclude = ["bindings"]`) and opens its own `[workspace]` table, so it is versioned and feature-set independently of the Rust workspace.

## Position and dependency direction

`bindings/node` (cdylib) → `stegoeggo` (lib) → `stegoeggo-stego` (carrier). The binding is a path dependency pinned to the exact workspace version:

```toml
[workspace]

[package]
name = "stegoeggo-node"
version = "0.4.2"
rust-version = "1.89"
publish = false

[lib]
name = "stegoeggo"
crate-type = ["cdylib"]

[dependencies]
stegoeggo = { path = "../..", version = "=0.4.2", default-features = false }
napi = { version = "3.13.0", default-features = false, features = ["napi6", "dyn-symbols"] }
napi-derive = "3.6.9"

[build-dependencies]
napi-build = "2.5.0"
```

Consequences of `default-features = false` and version `0.4.2`:

- No `async` (tokio), no `parallel` (rayon), no `signatures`, no `detached-manifest`, no `iscc`, no `conformance`. The binding layers its own `Task` types instead of the library's async API ([async-api.md](async-api.md)).
- No `webp` feature. This is *not* a capability gap: the root `webp` feature only gates the `pub use stegoeggo_stego::webp` re-export (`src/lib.rs:251`). The library's WebP hidden-marker path is the `image`-crate LSB path (`src/protected/steganography/mod.rs:188`), which is compiled in regardless. The carrier's optional WebP *byte facade* is simply not reachable from here.
- Not a workspace member, so `./scripts/check.sh`, `cargo test --workspace`, and `cargo deny` do not cover this crate. It is gated by its own workflow instead.

Profile divergence from the root release profile is deliberate. The root sets `panic = "abort"`; the addon sets `panic = "unwind"` in both profiles, which is what makes the `catch_unwind` guard in `tasks.rs` possible:

```toml
[profile.release]
lto = true
codegen-units = 1
panic = "unwind"
opt-level = "s"
strip = "symbols"
```

`src/lib.rs` carries `#![deny(missing_docs)]` but not `#![forbid(unsafe_code)]`, which the library and carrier both carry.

## Exported JS surface

Grouped by concern. Every entry is a `#[napi]` item; the owning file is the Rust source of truth.

| JS export | Kind | Owner | Reaches |
|-----------|------|-------|---------|
| `stegoeggoVersion()` | fn | `lib.rs:47` | `env!("CARGO_PKG_VERSION")` — no library call |
| `detectFormat(data)` | fn (sync) | `lib.rs:57` | `ImageOutputFormat::from_magic_bytes` only |
| `ProtectionRequest` (+ 3 static factories, 15 `with*`, 8 getters) | class | `request.rs` | none; pure builder projection |
| `RightsNotice` (constructor, 17 `with*`, 18 getters) | class | `notice.rs` | none |
| `ProcessingOptions` (constructor, 8 `with*`, 8 getters) | class | `notice.rs` | none |
| `HiddenMarkerMode` (4 static factories, 2 getters) | class | `enums.rs:323` | none; validates `tileSize` in `32..=1024` |
| `ResourceLimits` (1 factory, `toBuilder`, 18 getters) | class | `limits.rs` | none |
| `ResourceLimitsBuilder` (1 factory, 18 `with*`, `build`) | class | `limits.rs` | none |
| `protectInternal(data, request)` | fn → `Promise` | `tasks.rs:224` | `process_request_bytes` |
| `protectWithWarningsInternal(data, request)` | fn → `Promise` | `tasks.rs:233` | `process_request_bytes_with_warnings` |
| `protectWithReportInternal(data, request)` | fn → `Promise` | `tasks.rs:245` | `process_request_bytes_with_report` |
| `verifyInternal(data, macKey?, resourceLimits?)` | fn → `Promise` | `tasks.rs:257` | `verify_image_bytes_report` / `verify_image_bytes_report_with_limits` |
| `ProtectOutcome`, `ProtectWithWarningsOutcome`, `ProtectWithReportOutcome`, `VerifyOutcome` | interface | `tasks.rs:28-65` | — |
| `ExecutionReport`, `ResourceUsage`, `VerificationReport`, `RightsVerification`, `HiddenMarkerVerification`, `AuthenticationVerification`, `BindingVerification`, `TrustEvaluation`, `Diagnostic` | interface | `report.rs` | — |
| `NativeError` | interface | `error.rs:86` | — |
| `ErrorCode` + 13 `#[napi(string_enum)]` types | enum | `error.rs`, `enums.rs` | — |
| `protect`, `protectWithWarnings`, `protectWithReport`, `verify`, `toPublicError` | fn | `stegoeggo.js` (JS) | wraps the four `*Internal` promises |

The four `*Internal` names are the real ABI. The public names exist only in the hand-written `stegoeggo.js` wrapper, because `AsyncTask::reject` cannot attach structured properties to a rejection. `stegoeggo.js` re-exports the whole native module (`...native`) and overrides the operations.

Only `detectFormat` is synchronous. It is a bounded magic-byte inspection, deliberately kept on the JS thread; everything that decodes or embeds is a `Promise`.

## Type marshalling

### String enums

Every domain enum is a `#[napi(string_enum)]` with explicit wire values, so the JS enum is a runtime string union with a `string` backing — not a numeric const enum (the build passes `--no-const-enum --runtime-string-enum`):

```rust
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightsPolicy {
    #[napi(value = "UNSPECIFIED")]
    Unspecified,
    #[napi(value = "ALLOWED")]
    Allowed,
    #[napi(value = "PROHIBITED_AI_ML_TRAINING")]
    ProhibitedAiMlTraining,
```

Which become in `index.d.ts`:

```ts
export declare enum RightsPolicy {
  /** No restriction expressed. */
  Unspecified = 'UNSPECIFIED',
  /** Use is allowed. */
  Allowed = 'ALLOWED',
  /** Prohibited for AI/ML training. */
  ProhibitedAiMlTraining = 'PROHIBITED_AI_ML_TRAINING',
```

There are 14 string enums in total: 13 in `enums.rs` — `RightsPolicy`, `DmiValue`, `ImageOutputFormat`, `MetadataUpdatePolicy`, `ProtectionPreset`, `AuthenticationMode`, `HiddenMarkerKind`, `ProtectionWarning`, `VerificationStatus`, `EvidenceStrength`, `FieldSource`, `EvidenceChannel`, `DiagnosticLevel` — plus `ErrorCode` in `error.rs`.

### Numbers: `bigint` vs `number`

The `u64` seed never passes through a JS `number`. `numeric.rs` is the only narrowing authority, and it splits the two widths deliberately.

Seeds cross as `BigInt` in both directions, and `get_u64()` is checked for exactness:

```rust
/// Extracts a full-width `u64` from a JavaScript `bigint` without ever routing
/// the value through a JavaScript `number`.
pub fn to_u64(field: &str, value: &BigInt) -> Result<u64> {
    let (sign_bit, magnitude, lossless) = value.get_u64();
    if sign_bit || !lossless {
        return Err(reject(
            field,
            "must be a non-negative BigInt that fits in an unsigned 64-bit integer",
        ));
    }
    Ok(magnitude)
}
```

Bounded counts, sizes, and dimensions cross as `f64` and are validated *before* the Rust cast, so a bad JS value becomes `ERR_STEGOEGGO_INVALID_CONFIG` rather than a silent truncation:

```rust
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

fn check_count(field: &str, value: f64) -> Result<()> {
    if !value.is_finite() {
        return Err(reject(field, "must be a finite number"));
    }
    if value.fract() != 0.0 {
        return Err(reject(field, "must be an integer"));
    }
    if value < 0.0 {
        return Err(reject(field, "must not be negative"));
    }
    if value > MAX_SAFE_INTEGER {
        return Err(reject(field, "exceeds Number.MAX_SAFE_INTEGER"));
    }
    Ok(())
}
```

`to_u32`, `to_u8`, and `to_usize` call `check_count` then re-check the target width; `to_unit_f32` bounds intensity to the closed unit interval. `from_usize` projects the other way and is total.

The consequence is that `usize` counts and `f64` reports share the `MAX_SAFE_INTEGER` ceiling: a value beyond `2^53 - 1` can never be *set* through JS and would be *reported* imprecisely. That is unreachable for byte counts under any `ResourceLimits` default (`max_input_bytes` is 100 MiB), so the ceiling is a guard, not a live limitation.

The `napi6` feature exists for one reason: exact `u64` ↔ `BigInt` conversion. It sets the Node-API floor; `package.json` requires Node `>= 22.13.0`.

### Byte payloads

`&[u8]` parameters accept a `Buffer`/`Uint8Array` and are typed `Uint8Array` in the declarations. Output uses napi's `Buffer`, so `ProtectOutcome.data` is `Buffer | null` — a copy out of Rust, not a view.

### Requests and notices

`request.rs` and `notice.rs` are pure projection: each napi class owns a canonical Rust value and every `with*` clones and returns a new instance, matching the library's immutable builder style. `Request::with_hidden_marker_mode` and `with_authentication` cannot mutate a field directly — they clone `channels()`, edit one field, and rebuild:

```rust
/// Rebuilds a canonical request with a different channel configuration while
/// preserving every other canonical field.
fn with_channels(request: &RustRequest, channels: ProtectionChannels) -> RustRequest {
    let mut next = RustRequest::new(request.notice().clone(), request.policy(), channels)
        .with_processing(request.processing().clone())
        .with_intensity(request.intensity());
    if let Some(seed) = request.seed() {
        next = next.with_seed(seed);
    }
    if let Some(legal) = request.legal_metadata() {
        next = next.with_legal_metadata(legal.clone());
    }
    if let Some(key) = request.mac_key() {
        next = next.with_mac_key(key.to_vec());
    }
    if let Some(limits) = request.resource_limits() {
        next = next.with_resource_limits(limits.clone());
    }
    next
}
```

This is an allowlist, not a merge. Any canonical field added later must be added to this function or it is silently dropped by these two methods.

### Reports

`report.rs` flattens the canonical verification tree into nine `#[napi(object, object_from_js = false)]` structs. Five of the nine additionally set `use_nullable = true`, which maps Rust `Option<T>` to `T | null` rather than `undefined` so JS can distinguish "not applicable" from "missing field"; the other four (`ResourceUsage`, `BindingVerification`, `TrustEvaluation`, `Diagnostic`) omit it and emit optional/undefined fields. The projection is a hand-written `From` per type, e.g. the whole `HiddenMarkerVerification`:

```rust
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct HiddenMarkerVerification {
    /// Status of the hidden marker.
    pub status: VerificationStatus,
    /// Detected payload version, or `null`.
    pub payload_version: Option<u8>,
    /// Extracted seed as a full-width `bigint`, or `null`.
    pub seed: Option<u64>,
    /// Extracted embedding intensity, or `null`.
    pub intensity: Option<f64>,
    /// Whether tiled steganography was detected.
    pub tiled: bool,
    /// Where the marker was read from.
    pub source: FieldSource,
}
```

`VerificationReport` reproduces the canonical nesting one-for-one (`rights`, `hiddenMarker`, `authentication`, `bindings`, `trust`, `diagnostics`) and flattens the canonical private fields `evidence_strength` and `has_errors` to public accessors `evidenceStrength` and `hasErrors`. Depth is 3 (`report.hiddenMarker.seed`); nothing is stringified or JSON-encoded.

## Error handling

Two distinct representations, and it matters which one a caller sees.

**Domain failures are values, not throws.** Every napi task returns an outcome object with a nullable `error` field, so a canonical `stegoeggo::Error` never unwinds into a raw Node-API status:

```rust
/// Result of a `protect` call: the protected bytes, or a structured failure.
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct ProtectOutcome {
    /// Protected encoded bytes, or `null` on failure.
    pub data: Option<Buffer>,
    /// Structured failure, or `null` on success.
    pub error: Option<NativeError>,
}
```

`NativeError::from_rust` maps the canonical `stegoeggo::Error` onto a stable code plus structured fields, with a catch-all that never invents facts:

```rust
pub type BindingError = napi::Error<ErrorCode>;
pub type Result<T> = std::result::Result<T, BindingError>;

pub fn binding_error(code: ErrorCode, message: impl Into<String>) -> BindingError {
    napi::Error::new(code, message.into())
}
```

The nine public codes are `ERR_STEGOEGGO_INVALID_CONFIG`, `ERR_STEGOEGGO_INVALID_FORMAT`, `ERR_STEGOEGGO_ENCODE_DECODE`, `ERR_STEGOEGGO_METADATA`, `ERR_STEGOEGGO_STEGANOGRAPHY`, `ERR_STEGOEGGO_INSUFFICIENT_CAPACITY`, `ERR_STEGOEGGO_VERIFICATION`, `ERR_STEGOEGGO_RESOURCE_LIMIT`, `ERR_STEGOEGGO_INTERNAL`. Configuration validation (bad numbers, wrong content-hash width) throws through `napi::Error<ErrorCode>` directly; that path is synchronous and happens before any Promise exists.

**The public rejection is a real `Error` built in JS.** `stegoeggo.js` turns the outcome DTO into a thrown `Error`, because `AsyncTask::reject` cannot carry properties:

```js
function toPublicError (dto) {
  const error = new Error(
    typeof dto?.message === 'string' ? dto.message : 'stegoeggo operation failed'
  )
  error.name = 'StegoEggoError'
  error.code = normalizeNativeCode(dto?.code)
  for (const field of STRUCTURED_FIELDS) {
    if (dto && dto[field] !== null && dto[field] !== undefined) {
      error[field] = dto[field]
    }
  }
  return error
}
```

So: the four public operations are `Promise`-returning and **reject** with an `Error` whose `error.name` is `StegoEggoError` and whose `error.code` is one of the nine `ERR_STEGOEGGO_*` strings. It is not a result object at the public boundary, and it is not a rejected promise from napi itself — the rejection is constructed by the wrapper from a resolved-with-`error` native outcome. Structured fields (`resource`, `required`, `available`, `size`, `limit`, `width`, `height`, `maxWidth`, `maxHeight`, `kind`, `count`) are copied on only when non-null, which is why the JS type marks them optional (`StegoEggoErrorDetails`) while the Rust DTO marks them nullable.

`normalizeNativeCode` re-homes napi-rs's own statuses (`InvalidArg`, `ERR_INVALID_ARG_TYPE` → `ERR_STEGOEGGO_INVALID_CONFIG`; anything else → `ERR_STEGOEGGO_INTERNAL`) so a raw Node-API string can never escape.

`verify` is the one operation that does not fail on missing or invalid evidence — read `report.status` and `report.evidenceStrength` instead. The carrier-level `resource: "carrier"` case keeps the canonical message and no numeric fields; branch on `resource`, not on `kind` wording.

Panics are contained. `panic = "unwind"` plus this guard means a malformed-input bug surfaces as `ERR_STEGOEGGO_INTERNAL`, never an aborted host process:

```rust
fn guarded<T>(operation: impl FnOnce() -> T) -> std::result::Result<T, NativeError> {
    catch_unwind(AssertUnwindSafe(operation))
        .map_err(|_| NativeError::internal("native operation panicked"))
}
```

## Async and the threadpool

The binding does **not** use the library's `async` feature. It layers napi's `AsyncTask` + `Task`, which moves `compute()` onto a libuv worker thread. That is the whole concurrency mechanism: no tokio, no `spawn`, no rayon, no thread pool owned by this crate. `dyn-symbols` means symbols resolve from the host process at load time.

Lifetime and ownership across the boundary are the correctness-critical part. The JS `Buffer` is *not* referenced by the worker. It is copied into Rust-owned storage synchronously, on the calling thread, before `AsyncTask::new` schedules anything:

```rust
/// Copies JavaScript-owned bytes into Rust-owned storage before the worker is
/// scheduled. Mutating the caller's `Buffer`/`Uint8Array` after this call
/// cannot change the bytes the worker processes.
#[must_use]
pub fn owned_bytes(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}
```

```rust
#[napi(js_name = "protectInternal")]
pub fn protect_internal(data: &[u8], request: &ProtectionRequest) -> AsyncTask<ProtectTask> {
    AsyncTask::new(ProtectTask {
        data: owned_bytes(data),
        request: RustRequest::from(request),
    })
}
```

Each `Task` struct owns a `Vec<u8>` and an owned `RustRequest`; no reference to the JS heap or to the napi `Env` crosses into `compute()`. `compute()` then `std::mem::take`s the buffers, which both moves the bytes into the closure and leaves the task empty so a retry cannot observe stale data. `resolve()` runs back on the JS thread and does no conversion beyond the napi object marshalling — the outcome is already a plain owned Rust value.

`VerifyTask` is the one task with three owned fields, and its `mac_key` is copied the same way:

```rust
pub struct VerifyTask {
    data: Vec<u8>,
    mac_key: Vec<u8>,
    resource_limits: Option<RustLimits>,
}
```

```rust
let outcome = guarded(move || match &limits {
    Some(limits) => verify_image_bytes_report_with_limits(&data, &mac_key, limits),
    None => verify_image_bytes_report(&data, &mac_key),
});
```

There is no `Env`, `Object`, or `Reference` held across `compute()`, so no napi handle lifetime is at risk. The cost of this design is one full copy of the input and one of the output per call; nothing is borrowed or shared, which is also why mutation-after-call is not observable.

## Limits

`limits.rs` is a straight projection of `stegoeggo::ResourceLimits` ([resource-limits.md](resource-limits.md)) — 18 limits, one `with*` per limit, no defaults redefined here. Defaults come from the canonical `Default` impl (100 MiB input, 16384×16384, 500 PNG chunks, 256 JPEG segments, 500 WebP RIFF chunks, 8192-byte metadata field, 256-byte payload, 64 tile origins, 32 verification seeds).

Two behaviours worth knowing:

- `ResourceLimits.toBuilder()` starts from the **canonical defaults**, not from the receiver's values. The doc comment says so explicitly, and the implementation matches:
  ```rust
  #[must_use]
  #[napi]
  pub fn to_builder(&self) -> ResourceLimitsBuilder {
      ResourceLimitsBuilder::new()
  }
  ```
  `ResourceLimitsBuilder::build()` starts from `RustLimits::builder()` (also defaults) and applies only set overrides, so an unset limit is a default, not a copy of the source limits.
- Width/height getters return `u32` directly while every other limit goes through `from_usize` to `f64`. That is a width decision only, not a semantic one.

The carrier's `CarrierLimits` ([carrier-surface.md](carrier-surface.md)) is a **separate** budget the binding never exposes or forwards. Carrier limit failures reach JS as `ERR_STEGOEGGO_RESOURCE_LIMIT` with `resource: "carrier"` and no numeric fields, because the canonical error variant `ResourceLimitExceeded` carries only a message. The `webp` Cargo feature discussion above applies here too: the addon does not enable it and does not need to.

## `src/bin/oracle.rs`

A 58-line binding-local Rust binary, not a library surface. It is a **parity oracle**: a minimal, napi-free way to produce reference bytes from the same canonical call the addon makes, so Node output can be compared byte-for-byte.

```rust
use stegoeggo::{process_request_bytes, ProtectionRequest, RightsNotice, RightsPolicy};
```

```rust
fn run(args: &[String]) -> Result<(), String> {
    if args.len() < 7 {
        return Err(format!(
            "usage: {} <input> <output> <metadata|marker> <seed-u64> <timestamp> <holder> [mac-hex]",
            args.first().map(String::as_str).unwrap_or("oracle")
        ));
    }
```

It is deliberately tiny: positional argv only, `metadata` or `marker` mode, a `u64` seed parsed as text, an argv-supplied timestamp, an argv-supplied copyright holder, a fixed policy (`ProhibitedAiMlTraining`), and an optional hex MAC key. That narrowness is the point — it is a fixture generator with no surface for the bugs it is meant to detect. `test/parity.test.mjs` invokes it as `target/debug/oracle` and asserts `sha256` and `Buffer.equals` equality against the Node output for metadata-only, HMAC-marker, and above-`MAX_SAFE_INTEGER` seeds, plus two-way cross-checks against the real Rust CLI. It is not built by `pnpm build`; `pnpm test:parity` builds it explicitly and the test hard-fails if either the CLI or the oracle binary is missing.

## TypeScript declarations and contract tests

Three files, two provenance classes.

`index.d.ts` (809 lines) and `index.js` are **generated** by `@napi-rs/cli` and committed. `stegoeggo.d.ts` (142 lines) and `stegoeggo.js` (171 lines) are **hand-maintained**. The split is deliberate: everything mechanically derivable from Rust is generated; the four public Promise operations and the structured error contract are hand-written because they exist only in the wrapper.

`stegoeggo.d.ts` re-exports 28 names from `./index.js` and adds what the generator cannot express:

```ts
/** The public rejection type of every Promise-based operation. */
export type StegoEggoError = Error & StegoEggoErrorDetails & { code: ErrorCode }
```

`test/types/contract.ts` (110 lines) is compile-only. `tsconfig.json` includes exactly the two declaration files and that consumer file, with `strict: true`, `noEmit: true`, and `skipLibCheck: false` — so the generated types are fully checked, not skipped. It exercises every entry point: all three request factories, `ProcessingOptions`, `ResourceLimitsBuilder`, `HiddenMarkerMode`, `verify` with a MAC key, a `u64::MAX` seed, report field reads, and a narrowed `catch` on `StegoEggoError`.

Drift is caught in CI, not by convention. `.github/workflows/node-binding.yml` builds the addon and then:

```yaml
      - name: Check generated-file drift
        working-directory: bindings/node
        run: git diff --exit-code -- index.js index.d.ts
```

That is a hard gate: regenerating the addon after a Rust change and failing to commit `index.d.ts` fails the job. `pnpm typecheck` is a second gate on the consumer contract. The workflow is path-filtered on `bindings/node/**`, `src/**`, `stegoeggo-stego/**`, and the root manifests, and is explicitly independent of the `ci.yml` Rust gate — it adds no Node tooling to `scripts/check.sh`. `test/errors.test.mjs` adds one more declaration assertion: the generated `index.d.ts` must not contain secret key material and must still declare `withMacKey(key: Uint8Array)`.

## Release contract

The npm package name is `@eggstack/stegoeggo` (`package.json` `name` and `napi.packageName`), version `0.4.2`, binary name `stegoeggo`, `publish = false` in Cargo, and **not published** — the README states registry ownership is a release decision that was not taken.

`release/eggpack/build-bindings.toml` and `release/eggpack/qualification-bindings.toml` do **not** build or qualify this addon. Despite the filenames, both target the CLI binary:

```toml
"aarch64-apple-darwin" = [{ selector = { kind = "direct" }, package = "stegoeggo-cli", binary = "stegoeggo" }]
```

```toml
[targets."aarch64-apple-darwin".smoke]
selector = { kind = "direct" }
argv = ["version"]
timeout_ms = 10000
```

They are consumed by `scripts/check-release-contract.py`, which only asserts the five canonical triples are present in both files. The addon has a separate, entirely manual path: `.github/workflows/release-node.yml` (`workflow_dispatch` only, `source_ref` input, never publishes). It builds five `.node` artifacts — `stegoeggo.linux-x64-gnu`, `linux-arm64-gnu`, `darwin-x64`, `darwin-arm64`, `win32-x64-msvc` — matching the `napi.targets` list in `package.json`, runs `scripts/smoke.mjs` against each just-built addon on a matching runner (Node 22 floor, plus 24 and 26 on Linux x86_64), then assembles with `napi create-npm-dirs` + `napi artifacts`, audits the five platform `package.json` files, `pnpm pack`s, and does a clean local install smoke. `napi pre-publish` is never invoked. `scripts/check-release-workflow-contract.py` drift-guards the matrix, the five artifact names, the pnpm `12.6.0` pin, and the three Node smoke lines.

## Review risks

Each is grounded in a specific line above.

- **`licenseUrl` is a duplicate of `rightsUrl`, and the real license URL is dropped.** `report.rs:133` reads `license_url: value.rights_url().map(ToString::to_string)`. The canonical `RightsVerification` (`src/verification/report.rs:30-40`) has `rights_url` and no `license_url` field at all, so there is nothing else to read — but `canonical.rs` does carry a separate `license_url` that this projection never surfaces. JS callers reading `report.rights.licenseUrl` get a second copy of the rights URL, and the actual license URL is never reported. This is a live data-correctness bug, not a theoretical one.
- **Silent enum downgrade on unknown core variants.** Every `From<Rust…>` in `enums.rs` ends in a catch-all that maps an unrecognised canonical variant to a *plausible* value rather than an error: `_ => Self::Unspecified` (RightsPolicy, DmiValue), `_ => Self::Png` (ImageOutputFormat), `_ => Self::QTableSeed` (EvidenceChannel), `_ => Self::Xmp` (FieldSource), `_ => Self::BestEffort` (HiddenMarkerKind), `_ => Self::MissingMacKey` (ProtectionWarning), and so on. If the core adds a variant, the addon reports the wrong fact confidently instead of failing. The `DmiValue` and `FieldSource` cases are the dangerous direction, because `UNSPECIFIED`/`xmp` is a weaker claim, not a louder one.
- **Full input copy per call, twice.** `owned_bytes` copies the entire input on the JS thread before the worker starts, and the output crosses back as a fresh napi `Buffer`. For a 100 MiB default-limit input that is a 200 MiB transient cost with no zero-copy path. This is the price of the lifetime guarantee and is the correct trade, but it is a real memory ceiling worth knowing before calling `protect` in a loop.
- **`with_channels` is an allowlist.** `with_hidden_marker_mode` and `with_authentication` rebuild the request through `RustRequest::new(...)` plus an explicit re-list. A canonical field added to `ProtectionRequest` later is silently dropped by those two methods unless `with_channels` is updated in the same change. There is no test that compares a channel-mutated request against an unmutated one field-by-field.
- **`with_stego_redundancy` does not enforce the `1..=10` its doc comment claims.** `request.rs:134` only calls `numeric::to_usize`; the canonical `with_stego_redundancy` is infallible (`src/types/request.rs:358`). Out-of-range values therefore surface as a downstream steganography error, not `ERR_STEGOEGGO_INVALID_CONFIG`, and the binding is the only place a caller could have caught it cheaply. By contrast `HiddenMarkerMode::tiled` *does* validate `32..=1024` in the binding.
- **`stegoeggoVersion` is not the library version.** It returns `CARGO_PKG_VERSION` of the addon, not `stegoeggo`'s. They agree only because the addon pins `=0.4.2`; nothing enforces that the two stay in lockstep, and the pinning is outside this crate's own build.
- **Verification report drops the signature channel.** The binding's `VerificationReport` has no `signatures` field, while the canonical struct has `signatures: Vec<SignatureVerification>` (`src/verification/report.rs:1009`). This is correct today because the `signatures` feature is off in the addon, but it means enabling the feature would silently produce an incomplete report rather than a compile error.
- **Not covered by the standard gates.** `exclude = ["bindings"]` means `cargo fmt`, `clippy`, and `cargo test --workspace` skip this crate, and it has no `#![forbid(unsafe_code)]`. Its only Rust-level gates are `node-binding.yml` (Linux x86_64) and `release-node.yml` (manual, five targets) — neither runs `clippy` or `fmt --check`.
- **Stale local `.node` binaries are committed in the working tree.** `stegoeggo.darwin-arm64.node` and `stegoeggo.darwin-x64.node` sit alongside `node_modules/` and `target/`. `.gitignore` covers them, but a developer testing on macOS can exercise a hand-built addon that does not match the current Rust source; the CI drift check only covers `index.js`/`index.d.ts`, not the binary.

## Discrepancies

The generated `index.d.ts` agrees with the Rust source on every name, arity, casing, and wire value I checked, and CI enforces that with `git diff --exit-code -- index.js index.d.ts`. One substantive gap is in the hand-written layer and one in the generated layer's provenance:

- `stegoeggo.d.ts` documents `NativeError` (from the generated `index.d.ts`) with nullable fields, but its own `StegoEggoErrorDetails` declares the same fields as optional (`resource?: string`). Both are correct — the wrapper omits null fields rather than copying them — but the two types are not structurally identical, and a consumer comparing them must account for the optionality. This is intended, not drift; `stegoeggo.d.ts` says the generated file "must never be used to hide a mismatched runtime type".
- `index.d.ts` exports no public `protect`/`verify`. Consumers importing from the package root get `stegoeggo.d.ts` (per `package.json` `types`), so this is invisible in normal use; it only matters when a consumer deep-imports the `./native` subpath, which is declared to point at `index.d.ts`.

Not verified by execution: everything above was read, not run. I did not execute `pnpm build`, `pnpm test`, `pnpm test:parity`, or `cargo test` for this crate, and the drift/typecheck/parity assertions are reported as *read from configuration*, not as observed results.

## Relationship to other modules

- [pipeline.md](pipeline.md) — owns `process_request_bytes*` and the plan executors that `tasks.rs` calls; this binding adds no execution logic of its own.
- [verification.md](verification.md) — owns `VerificationReport` and the canonical facts; `report.rs` is a one-way projection of that tree into napi objects.
- [resource-limits.md](resource-limits.md) — owns `ResourceLimits`; `limits.rs` mirrors its 18 fields and the carrier's separate `CarrierLimits` is not exposed here.
- [error.md](error.md) — owns `stegoeggo::Error`; `error.rs` is the only place the addon maps it, into nine `ERR_STEGOEGGO_*` codes.
- [carrier-surface.md](carrier-surface.md) — the carrier underneath. The addon never touches the `stego` re-export, so carrier API changes surface only through library behavior, not through this surface.
- [types.md](types.md) — `RightsNotice`, `ProcessingOptions`, `ProtectionRequest`, and the 14 enums are projections of the types documented there.
- [async-api.md](async-api.md) — the library's tokio async path, which the addon deliberately does not use in favor of napi `AsyncTask`.
- [container-walk.md](container-walk.md) — the accounting walk whose counts appear in the projected `ResourceUsage`; the same untrusted-input budget, reached from a different caller.
- [cli.md](cli.md) — the sibling frontend. `test/parity.test.mjs` asserts byte-for-byte agreement between the two, which is what keeps the JS projection honest.
- [tooling.md](tooling.md) — owns the two binding workflows (`node-binding.yml`, an independent push+PR signal path-filtered on `bindings/node/**` and never a required gate, and the manual `release-node.yml`) plus the contract checks.
