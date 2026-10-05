# C Bindings (versioned C ABI v1)

**Source:** `bindings/c/src/` — `codes.rs` (379) · `error.rs` (424) · `request.rs` (361) · `limits.rs` (296) · `report.rs` (275) · `handles.rs` (269) · `notice.rs` (266) · `operations.rs` (234) · `input.rs` (160) · `panic.rs` (60) · `buffer.rs` (56) · `lib.rs` (14) = 2794 lines; plus `include/stegoeggo.h` (530, cbindgen-generated, committed) · `ABI-V1.md` (895, normative) · `abi-v1-symbols.txt` (90) · `cbindgen.toml` (10) · `scripts/{generate-header.sh,check-contract.py,parity.sh,run-c-tests.sh,check-exports.sh}` (73/88/83/52/48) · `tests/{test_c_abi.c,c_parity_protect.c,c_parity_verify.c,minipng.h,cxx_smoke.cpp}` (613/116/50/138/35) · `examples/{smoke.c,smoke.cpp}` (217/31). Crate `stegoeggo-c` v0.4.2, `crate-type = ["cdylib"]`, lib name `stegoeggo_c`, `publish = false`, not a workspace member.

The only **stable, frozen, cross-language** frontend in the repository. Python and Node are direct Rust bindings whose surfaces can move; this one is an ABI *contract* (`ABI-V1.md`) with 90 exported symbols, a committed generated header, a sorted export manifest, and a promise that names, signatures, and numeric values are append-only within ABI major 1. It owns no protection logic: `operations.rs` is the only file that calls the library, and it calls exactly four entry points.

## Position and dependency direction

```
bindings/c (stegoeggo_c, cdylib)  →  stegoeggo  →  stegoeggo-stego
```

```toml
[workspace]

[package]
name = "stegoeggo-c"
version = "0.4.2"
edition = "2021"
rust-version = "1.89"
publish = false

[lib]
name = "stegoeggo_c"
crate-type = ["cdylib"]

[dependencies]
stegoeggo = { path = "../..", version = "=0.4.2", default-features = false }
serde_json = "1.0"
zeroize = "1"

[dev-dependencies]
image = { version = "=0.25.6", default-features = false, features = ["png"] }

[profile.release]
panic = "unwind"

[profile.dev]
panic = "unwind"
```

The root workspace opts the whole tree out (`exclude = ["bindings"]`, root `Cargo.toml:4`), so this crate has its own `[workspace]` table, its own `Cargo.lock`, its own MSRV check, and its own workflow. `default-features = false` means no `async`, `parallel`, `signatures`, `detached-manifest`, `iscc`, `conformance`, or `webp` — every library optional feature is off. Visible consequences: there is no async/parallel entry point; the four operations are strictly synchronous; and the verification report's `signatures` sub-report is always empty.

`src/lib.rs` is 14 lines and carries no `forbid`:

```rust
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)]

mod buffer;
mod codes;
mod error;
mod handles;
mod input;
mod limits;
mod notice;
mod operations;
mod panic;
mod report;
mod request;
```

This is the one crate in the repository that *must* contain `unsafe`, and `README.md:20-21` states the rule: all `unsafe` required by the boundary lives here; root and carrier keep `#![forbid(unsafe_code)]`. The lint that is on is `unsafe_op_in_unsafe_fn` (edition-2024 hygiene), not `forbid(unsafe_code)`. Note there is **no** `#![deny(missing_docs)]` — unlike `bindings/node`, nothing forces the frozen constants to carry in-source documentation, so the normative prose lives only in `ABI-V1.md`.

### Library entry points

The binding is a pure transport and marshalling layer — it reimplements no protection logic. `src/operations.rs` holds the *only* library calls in the whole crate, and there are exactly five:

| Export | Library call |
|---|---|
| `stegoeggo_v1_detect_format` | `stegoeggo::ImageOutputFormat::from_magic_bytes(bytes)` (`operations.rs:24`) |
| `stegoeggo_v1_protect` | `stegoeggo::process_request_bytes(bytes, &canonical)` (`operations.rs:48`) |
| `stegoeggo_v1_protect_with_report` | `stegoeggo::process_request_bytes_with_report(bytes, &canonical)` (`operations.rs:73`) |
| `stegoeggo_v1_verify` | `stegoeggo::verify_image_bytes_report(bytes, key)`, or `…_with_limits(bytes, key, &handle.inner)` when a limits handle is passed (`operations.rs:100,103`) |
| `stegoeggo_v1_request_*` | `stegoeggo::ProtectionRequest::new(notice, policy, channels)` then a full re-list of setters (`handles.rs:156-195`) |
| `stegoeggo_v1_notice_*` | `stegoeggo::RightsNotice` builder methods, materialised in `to_notice()` (`handles.rs:48-108`) |
| `stegoeggo_v1_resource_limits_*` | `stegoeggo::ResourceLimits::builder()` with one generated `max_*` setter per field (`limits.rs:25-65`) |

Note the deliberate omission: there is **no** `DynamicImage`/pixel path. Per the byte-vs-pixel footgun, this binding only ever uses the byte APIs, so PNG `tEXt`, JPEG `COM`/XMP, and WebP XMP all survive — which is the whole point of routing through `process_request_bytes` rather than `process_image`. `stegoeggo_v1_request_from_preset` is the only convenience shortcut: it resolves a `ProtectionPreset` instead of taking a staged notice. Async and parallel entry points do not exist here, because `default-features = false` leaves both features off.

### Why `panic = "unwind"` overrides `panic = "abort"`

The root release profile sets `panic = "abort"` (`Cargo.toml:116`). A `cdylib` cannot use that: `extern "C"` functions have no unwind contract, and a panic that reaches one is instant UB in the host process. `panic = "abort"` would make `std::panic::catch_unwind` unable to return at all, so the entire panic-containment story in `src/panic.rs` would be dead code and every malformed-input bug would abort the embedding application instead of returning an error code.

Setting `panic = "unwind"` in *both* profiles is what makes `catch_unwind` real, in dev as well as release — so the containment behaviour a developer tests locally is the behaviour the qualified artifact has. Note this override is a property of the *build*, not of the source: nothing in `src/` asserts that the profile is right, and a consumer who links a differently-configured build loses the guarantee silently. `ABI-V1.md` §13 is explicit that OOM and process-abort conditions are outside the `catch_unwind` guarantee, and that resource limits are the primary defence against hostile oversized inputs.

## Exported surface — 90 symbols

Counted directly from `src/`: `grep -hoE '^pub extern "C" fn [a-z0-9_]+' src/*.rs` yields **90** names. That set is byte-identical to `abi-v1-symbols.txt` (90 lines) and to the 90 function declarations in `include/stegoeggo.h`. `ABI-V1.md` §0 declares "Total normative symbol count: **90** (3 bootstrap + 87 `stegoeggo_v1_*`)". **No drift between source, manifest, and committed header.** Every one is a `#[no_mangle] pub extern "C" fn`; there are no exported variables, no `#[no_mangle]` on types, and no `extern "C"` items that are not counted.

| Concern | Count | Owner | One-line purpose |
|---|---:|---|---|
| Bootstrap | 3 | `codes.rs:101,106,111` | `stegoeggo_abi_version_major/minor`, `stegoeggo_source_version` — infallible, allocation-free, never unwind |
| Byte buffer | 3 | `buffer.rs:4,16,25` | Borrow a data pointer + length from an owned output buffer; free it |
| Notice builder | 20 | `notice.rs:24-266` | `create`/`free` + 16 UTF-8 text setters + `set_dmi` + `set_seed` |
| Request builder | 18 | `request.rs:41-361` | 3 constructors + `free` + 14 typed setters |
| Resource limits | 20 | `limits.rs:112-296` | `create`/`free` + 18 one-per-limit setters |
| Operations | 4 | `operations.rs:13,36,59,86` | `detect_format`, `protect`, `protect_with_report`, `verify` |
| Error | 6 | `error.rs:194-253` | `error_code`, `error_resource`, `message_data`, `message_len`, `details_json`, `error_free` |
| Execution report | 10 | `report.rs:10-116` | 8 scalar getters, `warning_count`/`warning_at`, `free` |
| Verification report | 6 | `report.rs:119-190` | `status`, `evidence_strength`, `rights_found`, `authenticated`, `json`, `free` |

Category totals match `ABI-V1.md` §0 exactly: notice 20, request 18, limits 20, operations 4, buffer 3, error 6, execution report 10, verification report 6 = 87 versioned, + 3 bootstrap = 90.

### Exported types

Seven **opaque** handle types and thirteen fixed-width scalar typedefs. `cbindgen.toml` emits `typedef struct stegoeggo_v1_X_t stegoeggo_v1_X_t;` with no struct body, and `generate-header.sh:47-50` fails the build if `struct stegoeggo_v1_.*{` ever appears — opaque-ness is machine-enforced, not just documented.

| Opaque type | Rust definition | Contains |
|---|---|---|
| `stegoeggo_v1_notice_t` | `handles.rs:3` | 16 `Option<String>` + `Option<DmiValue>` + `Option<u64>` |
| `stegoeggo_v1_request_t` | `handles.rs:111` | owned canonical `RightsNotice`, `RightsPolicy`, `HiddenMarkerMode`, `AuthenticationMode`, seed, intensity, format, quality, flags, `Option<[u8;4]>` hash, timestamp, `Option<Vec<u8>>` mac key, `Option<ResourceLimits>` |
| `stegoeggo_v1_resource_limits_t` | `handles.rs:207` | one `stegoeggo::ResourceLimits` |
| `stegoeggo_v1_buffer_t` | `handles.rs:211` | one `Vec<u8>` |
| `stegoeggo_v1_execution_report_t` | `handles.rs:215` | one `stegoeggo::ExecutionReport` |
| `stegoeggo_v1_verification_report_t` | `handles.rs:219` | one `verification::VerificationReport` |
| `stegoeggo_v1_error_t` | `handles.rs:223` | one `ErrorDto` |

The 13 scalars are all `uint32_t` typedefs (`codes.rs:3-17`), never C enums, so ABI width never depends on the consumer compiler. `usize` fields cross as `size_t` (`usize_is_size_t = true`). Booleans are `uint8_t` with strict 0/1 validation.

The committed header also carries **67 `#define` constants**: 11 status/error codes, 7 resource categories, 7 policies, 7 DMI values, 4 formats, 3 update policies, 4 presets, 2 auth modes, 4 marker modes, 3 verification statuses, 4 evidence strengths, 8 warnings, and `STEGOEGGO_V1_ABI_MAJOR`/`_MINOR`. Values are assigned by hand in `codes.rs:19-96` and are explicitly **not** Rust enum discriminants — `ABI-V1.md` §4 says so and forbids changing them within v1.

## Handle and ownership model

Handles are **raw `Box::into_raw` pointers with no registry, no generation counter, and no tag**. There is no handle table: `require_out` just stores a pointer into the caller's out-slot, and the free functions are a bare `drop(Box::from_raw(x))`.

```rust
pub(crate) fn require_out<'a, T>(
    ptr: *mut *mut T,
    name: &'static str,
) -> Result<&'a mut *mut T, ErrorDto> {
    if ptr.is_null() {
        return Err(ErrorDto::invalid_argument(name));
    }
    unsafe {
        *ptr = core::ptr::null_mut();
    }
    Ok(unsafe { &mut *ptr })
}
```

`input.rs:20-31`. Three consequences:

1. **No use-after-free or double-free protection exists, by design.** There is no generation/version counter to detect a stale handle, so a stale pointer is indistinguishable from a live one and the read is UB. `ABI-V1.md` §2 states this explicitly: "double-free and use-after-free are caller contract violations (undefined behavior, never a Rust panic)". The protection is contractual, not mechanical — which is a real difference from a handle-table design and the single most important thing for a reviewer to internalise.
2. **Every free function is NULL-tolerant.** `stegoeggo_v1_buffer_free`, `stegoeggo_v1_error_free`, `stegoeggo_v1_notice_free`, `stegoeggo_v1_request_free`, `stegoeggo_v1_resource_limits_free`, and both report frees all begin `if x.is_null() { return; }`. Freeing NULL is a no-op, which is what makes the `ABI-V1.md` §17 cleanup sketches safe.
3. **`require_out` overwrites unconditionally.** It NULLs the caller's slot before any work, so passing a non-NULL pre-initialised out-pointer silently orphans whatever was there. There is no "slot must be empty" check.

### Thread safety

There is **no global mutable state** — no last-error TLS, no registry, no cache. The only process-wide object is `SOURCE_VERSION_BYTES` (`codes.rs:98`), a `const &[u8]` in `.rodata`:

```rust
const SOURCE_VERSION_BYTES: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
```

What backs the concurrency claim is therefore purely the absence of shared state: independent handles hold independent Rust values, and every operation reads its inputs and writes its own out-slots. `ABI-V1.md` §14 permits concurrent use of *independent* handles and forbids concurrent mutation/free of a *single* handle, and it explicitly declines to promise same-handle concurrent **read-only** sharing ("explicitly UNSUPPORTED in v1. M010 did not qualify that behavior, so Rust `Sync` implementation details do not become part of the C contract"). So the safe claim is narrower than "thread-safe": it is thread-safe *per handle*, with no qualified basis for sharing one request across threads even for parallel `protect`.

The one non-trivial `Drop` in the crate is the MAC-key zeroizer:

```rust
impl Drop for stegoeggo_v1_request_t {
    fn drop(&mut self) {
        use zeroize::Zeroize as _;
        if let Some(key) = self.mac_key.as_mut() {
            key.zeroize();
        }
    }
}
```

`handles.rs:198-205`. The canonical `ProtectionRequest` it builds has a matching `Drop` (`src/types/request.rs:667-673`), so the key is zeroized at both layers: once when the temporary canonical request is dropped at the end of the call, and once when the C handle is freed. `notice_t`, `resource_limits_t`, and both report types have **no** `Drop` — nothing in them is secret. There is no getter that can read the MAC key back; `ABI-V1.md` §17-B makes "never readable back through any getter, error, or report" a contract, and `panic.rs:24-40` is the test that keeps panic payloads out of error text.

## Buffer marshalling

Output crosses as an owned, Rust-allocated opaque buffer plus a borrowed `(pointer, length)` pair. Three symbols, all in `buffer.rs`:

```rust
#[no_mangle]
pub extern "C" fn stegoeggo_v1_buffer_data(buffer: *const stegoeggo_v1_buffer_t) -> *const u8 {
    if buffer.is_null() {
        return core::ptr::null();
    }
    let bytes = unsafe { &(*buffer).bytes };
    if bytes.is_empty() {
        return core::ptr::null();
    }
    bytes.as_ptr()
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_buffer_len(buffer: *const stegoeggo_v1_buffer_t) -> usize {
    if buffer.is_null() {
        return 0;
    }
    let bytes = unsafe { &(*buffer).bytes };
    bytes.len()
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_buffer_free(buffer: *mut stegoeggo_v1_buffer_t) {
    if buffer.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(buffer));
    }
}
```

The contract: **Rust allocates, C only borrows.** C never calls `free()` on the data pointer and never sees the `Vec` capacity; the data view is valid until `buffer_free`. The only producers are `stegoeggo_v1_protect` / `stegoeggo_v1_protect_with_report` (`out_data`), `stegoeggo_v1_error_details_json`, and `stegoeggo_v1_verification_report_json`. Note the empty-buffer sentinel: `buffer_data` returns NULL for a zero-length buffer while `buffer_len` returns 0, so "empty" is indistinguishable from "NULL handle" through the data pointer alone — callers must branch on the handle, not the pointer.

### Input direction

Every variable input is `(pointer, length)`, never NUL-terminated. `input.rs:33-51` is the single gate:

```rust
pub(crate) fn borrow_bytes<'a>(
    data: *const u8,
    len: usize,
    name: &'static str,
) -> Result<&'a [u8], ErrorDto> {
    if len == 0 {
        if data.is_null() {
            return Ok(&[]);
        }
        return Ok(unsafe { core::slice::from_raw_parts(data, 0) });
    }
    if data.is_null() {
        return Err(ErrorDto::invalid_argument(name));
    }
    if len == usize::MAX {
        return Err(ErrorDto::invalid_argument(name));
    }
    Ok(unsafe { core::slice::from_raw_parts(data, len) })
}
```

`len == usize::MAX` is rejected so `from_raw_parts` can never be handed an overflowing total size. Beyond that the binding validates NULL/length/range *combinations* only — `ABI-V1.md` §5 is candid that "arbitrary invalid or dangling non-NULL pointers remain caller undefined behavior".

Text goes through `read_text` (`input.rs:53-68`), which borrows the bytes, rejects any interior NUL, enforces `LegalMetadata::MAX_FIELD_LEN` (8192) as invalid-argument, and then requires valid UTF-8 — so bad text never reaches a Rust `String`. `read_optional_text` adds the presence convention: `(NULL, 0)` means "clear to None"; `(NULL, len>0)` and `(non-NULL, 0)` are both invalid argument. An **empty string is therefore not representable** in v1, and the only way to clear a notice field is to pass NULL/0.

`ABI-V1.md` §5 also records one deliberate narrowing: an empty non-NULL MAC key is invalid argument rather than a zero-length key, "narrower than Rust by design".

## Error mapping

Two stages. The library first collapses the carrier's `StegoError` into `stegoeggo::Error` (`src/error.rs:12-33`, the crate's `From` impl, not the binding's), and *then* the binding maps `stegoeggo::Error` into the frozen numeric space (`error.rs:39-156`). The binding never names `StegoError`.

Carrier stage (`src/error.rs:12-33`): `InvalidConfig → Config`, `InsufficientCapacity → InsufficientCapacity`, `MalformedInput`/`UnsupportedJpeg → InvalidFormat`, `FrameNotFound`/`MalformedFrame`/`EmptyCarrier → Steganography`, `FrameChecksumMismatch → PayloadVerification`, `ResourceLimitExceeded → ResourceLimitExceeded`, and a catch-all `_ → Steganography` that also swallows `UnsupportedWebP` and anything added later.

Binding stage, `stegoeggo::Error` is `#[non_exhaustive]` with 21 variants; `ErrorDto::from_rust` has 14 arms:

| `stegoeggo::Error` variant | Code | Resource | Extra JSON fields |
|---|---:|---:|---|
| `Config(_)` | 2 `ERR_INVALID_CONFIGURATION` | 0 `NONE` | — |
| `InvalidFormat(_)` | 3 `ERR_INVALID_FORMAT` | 0 | — |
| `ImageDecode` · `ImageEncode` · `Image(_)` · `ImageTruncated` · `Io(_)` | 4 `ERR_ENCODE_DECODE` | 0 | — |
| `Metadata(_)` | 5 `ERR_METADATA` | 0 | — |
| `Steganography(_)` | 6 `ERR_STEGANOGRAPHY` | 0 | — |
| `InsufficientCapacity { required, available }` | 7 `ERR_INSUFFICIENT_CAPACITY` | 0 | `required`, `available` |
| `PayloadVerification(_)` · `Crypto(_)` | 8 `ERR_VERIFICATION` | 0 | — |
| `InputTooLarge { size, limit }` | 9 `ERR_RESOURCE_LIMIT` | 1 `INPUT_BYTES` | `size`, `limit` |
| `DimensionsExceeded { width, height, max_width, max_height }` | 9 | 2 `DIMENSIONS` | all four |
| `ContainerLimitExceeded { kind, count, limit }` | 9 | 3 `CONTAINER` | `kind`, `count` |
| `MetadataLimitExceeded { kind, size, limit }` | 9 | 4 `METADATA` | `kind`, `size`, `limit` |
| `VerificationBudgetExceeded { kind, count, limit }` | 9 | 5 `VERIFICATION_BUDGET` | `kind`, `count` |
| `ResourceLimitExceeded(_)` | 9 | 6 `CARRIER` | — (base four only) |
| `Serialization` · `Iscc` · `Task` · any future variant | 10 `ERR_INTERNAL` | 0 | — |

The `resource` code is the discriminator that recovers most of what the single code 9 loses; that is exactly why six resource categories exist as a separate frozen table. The error message is taken once via `error.to_string()` before the match, so no secret is re-derived per arm.

### Lossy and ambiguous points

- **Five limit variants collapse onto code 9.** Only `resource` separates them, so a C consumer that branches on `status` alone cannot tell an input-size rejection from a carrier rejection. A consumer must read `stegoeggo_v1_error_resource`.
- **`ContainerLimitExceeded` and `VerificationBudgetExceeded` drop their `limit` field** — `error.rs:115` and `error.rs:132` destructure with `..`. Both the Rust variant and the underlying `ResourceLimits` value have the number; the C caller gets `count` but not `limit`. `ABI-V1.md` §11.1 documents this shape honestly, so it is a design decision, but it is an asymmetry with `MetadataLimitExceeded` and `InputTooLarge`, which do carry `limit`.
- **The carrier case is message-only.** `ResourceLimitExceeded(_)` carries a `String` and no numbers, so `error.rs:399-407` asserts the details object has exactly four keys. `details_json` for `resource = 6` is `{schema_version, code, resource, message}`.
- **`Serialization`, `Iscc`, and `Task` are indistinguishable** from each other and from a genuine internal bug — all code 10, resource 0. `Io` is *not* in this bucket: the C ABI puts it in `ERR_ENCODE_DECODE` (4), which is strictly more discriminating than the Python binding, where `Io` falls into the catch-all base exception.
- **`emit_error` overwrites without freeing.** `error.rs:174-183` stores a new `Box` into `*out_error` unconditionally. A caller who reuses one `stegoeggo_v1_error_t *err` variable across two failing calls without freeing in between leaks the first handle. `clear_error` on the success path has the same shape: it NULLs the slot, it does not free.

### Error-handle lifecycle

`ABI-V1.md` §11 freezes per-call explicit errors with no last-error TLS. `out_error` itself may be NULL — the caller then receives only the numeric status and no handle, which is what `panic.rs:53-59` and `operations.rs`'s unit tests exercise by passing `core::ptr::null_mut()` everywhere. `stegoeggo_v1_error_code(NULL)` returns `ERR_INVALID_ARGUMENT` (1) and `error_resource(NULL)` returns `RESOURCE_NONE` (0), so a NULL handle is never confused with a real code-0 code (there is no error code 0).

## Panic containment

One wrapper, 18 lines, used by every fallible export. `panic.rs:6-18`:

```rust
const PANIC_MESSAGE: &str = "Internal error: unexpected failure in C ABI call";

pub(crate) fn invoke(
    out_error: *mut *mut stegoeggo_v1_error_t,
    operation: impl FnOnce() -> Result<(), ErrorDto>,
) -> u32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(Ok(())) => {
            clear_error(out_error);
            crate::codes::STEGOEGGO_V1_OK
        }
        Ok(Err(dto)) => emit_error(out_error, dto),
        Err(_) => emit_error(out_error, ErrorDto::internal(PANIC_MESSAGE)),
    }
}
```

Mechanism: `catch_unwind` + `AssertUnwindSafe`, with the `Err(_)` arm **discarding the payload** and substituting the fixed `PANIC_MESSAGE`. The panic text never reaches C. `panic.rs:24-40` is the regression test for that: it panics with `"secret-payload-marker-xyz"` and asserts the marker appears in neither `dto.message` nor `dto.details_json`, and that `dto.resource == RESOURCE_NONE`. A caught panic becomes code 10, resource 0 — the same code as `Serialization`/`Iscc`/`Task`.

`AssertUnwindSafe` is required rather than lazy: the closures capture `&mut` borrows of caller handles (`require_handle_mut`), which are not `UnwindSafe`.

**Not every export is wrapped.** The trivial accessors and the seven `*_free` destructors are bare `extern "C"` with no `catch_unwind` — e.g. `stegoeggo_v1_notice_free` is just a null check and `drop(Box::from_raw(notice))`. `ABI-V1.md` §13 accepts this ("trivial accessors … and destructors MUST NOT unwind either, but they need not allocate an error object"), and today no reachable `Drop` impl in the crate can panic, so the claim holds. It is a latent rather than live gap: the guarantee depends on `stegoeggo::RightsNotice`/`ExecutionReport`/`VerificationReport` destructors never panicking, which the binding does not control.

## Limits

`limits.rs` is a mechanical 18-setter projection of `stegoeggo::ResourceLimits` ([resource-limits.md](resource-limits.md)); it redefines no defaults. `create` starts from `ResourceLimits::default()` (100 MiB input, 16384×16384, 500 PNG chunks, 16 MiB per PNG chunk, 256 JPEG segments, 500 WebP RIFF chunks, 8192-byte metadata field, 256-byte payload, 64 tile origins, 32 verification seeds). Two structural facts a reviewer needs:

**The setters are an allowlist, and rebuild through a full 18-field copy.** `limits.rs:19-65` reads every current getter into a fresh builder, then overrides the one targeted field:

```rust
fn rebuild(
    current: &stegoeggo::ResourceLimits,
    field: LimitField,
    usize_value: usize,
    u32_value: u32,
) -> stegoeggo::ResourceLimits {
    let builder = stegoeggo::ResourceLimits::builder()
        .max_input_bytes(current.max_input_bytes())
        .max_width(current.max_width())
        // … 16 more explicit copies …
        .max_verification_seeds(current.max_verification_seeds());
```

`ResourceLimitsBuilder::build()` (`src/resource_limits.rs:348-350`) is infallible and validates nothing, so a 19th limit added to the canonical struct would not be copied here and would silently revert to its default on **every** setter call. The 18 named copies are the whole reason there are 18 setters, and nothing tests that the list is complete.

**The binding enforces no ranges at all.** Unlike the request setters — which reject intensity outside `0.0..=1.0`, `jpeg_quality == 0`, `max_dimension == 0`, redundancy outside `1..=10`, and a content hash that is not exactly 4 bytes — the 18 limit setters accept any `usize`/`u32`, including 0. `max_input_bytes(0)` is a successful call that makes every subsequent `protect` fail with `InputTooLarge { size: N, limit: 0 }`. The only failure mode is a NULL handle. `ABI-V1.md` §8 states setters "store the value verbatim" and "no setter fails on a non-NULL handle", so this is contract-correct and just under-documented as a footgun.

`set_limit` (`limits.rs:6-17`) uses `core::mem::take` then reassigns inside the `invoke` closure, so a panic inside `rebuild` would leave the handle holding `ResourceLimits::default()` rather than its previous value — partial mutation on an unreachable path.

The carrier's `CarrierLimits` (`stegoeggo-stego/src/limits.rs:30`, 8 limits including `max_pixels` and `max_frame_bytes`) is a **separate budget the binding never exposes, never forwards, and never reads** — it is configured by the library internally. Its failures reach C only as `Error::ResourceLimitExceeded`, i.e. code 9 / resource 6 / message-only. The 18 library limits and the 8 carrier limits overlap by name (`max_input_bytes`, `max_width`, `max_height`, `max_jpeg_segments`, `max_jpeg_segment_bytes`) but are enforced at different layers with no linkage the caller can observe.

`verify` accepts an optional limits handle; `protect` and `protect_with_report` have **no** limits parameter, so limits reach protect only via `stegoeggo_v1_request_set_resource_limits`, which takes a **snapshot** (`request.rs:325`) — later mutation of the limits handle does not affect an already-configured request. When `limits == NULL` in `verify`, the library's own defaults apply.

## Request, notice, and report conversion

### Notice

`stegoeggo_v1_notice_t` is a flat staging struct of 16 `Option<String>` + `Option<DmiValue>` + `Option<u64>` (`handles.rs:3-22`). All 16 text setters funnel through one helper, `notice.rs:8-21`:

```rust
fn set_text_field(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
    assign: impl FnOnce(&mut stegoeggo_v1_notice_t, Option<String>),
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(notice, "notice must be non-NULL")?;
        let value = read_optional_text(text, text_len, "text")?;
        assign(handle, value);
        Ok(())
    })
}
```

Materialization is one `to_notice()` (`handles.rs:48-108`) of 16 `if let Some(v) = …` blocks calling the matching canonical builder. Two behaviours are worth knowing:

- **The `usage_terms_lang` path is the only legal-metadata path, and it is the one asymmetry.** It is handled last and separately (`handles.rs:101-106`): it builds a `LocalizedText` from `usage_terms.unwrap_or_default()` plus the lang, wraps it in a fresh `LegalMetadata`, and calls `with_legal_metadata_fields`. If only the language is set, `usage_terms` becomes `Some("")` — an empty usage-terms string is injected into the notice. `handles.rs:244-250` tests the lang-only case and asserts only `usage_terms_lang()`, not that `usage_terms` stayed `None`.
- **15 of the 16 canonical legal-metadata fields are unreachable.** Only usage terms (plain and localized) can be set from C. There is no `set_legal_metadata` symbol; see [legal-metadata-field-mapping.md](legal-metadata-field-mapping.md).

### Request

`stegoeggo_v1_request_t` (`handles.rs:111-128`) holds **already-canonical** Rust values — a real `RightsNotice`, a real `RightsPolicy`, a real `HiddenMarkerMode` — not more strings. The three constructors are thin (`request.rs:41-83`): `metadata_only` and `with_hidden_marker` differ only in the `ProtectionChannels` they pass, and `from_preset` resolves the preset first and rejects an unknown code as invalid-argument *before* touching any out-slot. Setters mutate one field each. `to_request()` (`handles.rs:156-195`) then rebuilds a canonical `ProtectionRequest` from scratch, re-listing every field — the same allowlist shape as the node binding's `with_channels`, and the same forward-compatibility caveat: a new canonical `ProtectionRequest` field silently disappears from every C protect call unless `to_request` is updated in the same change.

The one thing the handle does *not* round-trip is `rights_metadata: bool`. `to_request()` hardcodes it:

```rust
let channels = stegoeggo::ProtectionChannels {
    rights_metadata: true,
    hidden_marker: self.hidden_marker,
    authentication: self.authentication,
};
```

`handles.rs:157-161`. There is no C symbol to disable rights metadata, so the canonical `ContradictoryLegalClaims` warning and the `MetadataInjectionDisabled` path are unreachable from this ABI even though their codes are in the frozen warning table.

Validation that *is* performed, all in `request.rs`: intensity finite and in `0.0..=1.0` (`:118`), `jpeg_quality != 0` (`:157`), `max_dimension != 0` when presence is set (`:194`), redundancy in `1..=10` (`:233`) — which is stricter than the node binding's unenforced equivalent — content hash exactly 4 bytes (`:259`), timestamp ≤ 256 bytes (`:288`), `has_*` flags strictly 0/1 via `require_flag`, and `marker_from_code` rejecting a non-zero `tile_size` for non-tiled modes and a `tile_size` outside `32..=1024` for tiled (`codes.rs:229-262`).

### Execution report — 10 getters, and what is missing

Eight scalar getters plus `warning_count`/`warning_at`/`free`. Every one is a `if report.is_null() { return <sentinel>; }` guard, with `u32::MAX` for the out-of-range `warning_at`. `report.rs:153-164` is the one non-trivial projection:

```rust
let authenticated = inner.authentication().attempted()
    && inner.authentication().hmac_status() == Some(stegoeggo::VerificationStatus::Verified)
    && inner.authentication().key_matched();
```

**`ExecutionReport.resource_usage` (the 10 accounting counters) and `embed_summary` are completely unreachable from C.** There is no execution-report JSON accessor — only the verification report has one. `ABI-V1.md` §12.1 freezes this as a deliberate decision ("complete execution-report JSON is NOT in v1 … `resource_usage` and `embed_summary` depth stays behind Rust until a future additive v1 getter"), but it is still a real gap versus the CLI, whose JSON carries them ([container-walk.md](container-walk.md) documents the counts).

### Verification report — 6 symbols

`status`, `evidence_strength`, `rights_found`, `authenticated`, `json`, `free`. The flattening is severe and the doc is honest about it: `status` is `hidden_marker().status()`, not an overall report status, because the canonical `VerificationReport` (`src/verification/report.rs:1005-1014`) has **no top-level `status`** — it is `rights` + `hidden_marker` + `authentication` + `signatures` + `bindings` + `trust` + `evidence_strength` + `diagnostics`. So a metadata-only protected image legitimately reports `status = NOT_FOUND (2)` alongside `rights_found = 1`; the two must be read together, and `evidence_strength` is the intended single summary.

Everything the four scalar getters do not cover is recoverable through exactly one call:

```rust
#[no_mangle]
pub extern "C" fn stegoeggo_v1_verification_report_json(
    report: *const stegoeggo_v1_verification_report_t,
) -> *mut stegoeggo_v1_buffer_t {
    if report.is_null() {
        return core::ptr::null_mut();
    }
    let inner = unsafe { &(*report).inner };
    match serde_json::to_vec(inner) {
        Ok(bytes) => Box::into_raw(Box::new(stegoeggo_v1_buffer_t { bytes })),
        Err(_) => core::ptr::null_mut(),
    }
}
```

`report.rs:167-178`. This serializes the canonical `VerificationReport` **unchanged** — same schema the CLI emits, no second schema invented, and `signatures`/`bindings`/`trust`/`diagnostics` are visible here and nowhere else. The returned buffer is a *new* allocation the caller frees separately from the report. The `Err(_) => null` arm is the one place in the crate where a failure is signalled by NULL rather than an error code, and it is indistinguishable from a NULL input to the caller.

`stegoeggo_v1_error_details_json` clones the JSON string into a fresh buffer on **every** call, so calling it twice hands back two independent handles — the second does not invalidate the first, and both must be freed.

## ABI stability contract and qualification

`ABI-V1.md` (895 lines) is the normative document; its authority is `plans/adrs/ADR-0006-versioned-c-abi.md`, materialized by plans M007/M009/M010. The promise: within ABI major 1, existing symbol names, signatures, numeric values, and ownership rules are **append-only**; a breaking change requires a new `stegoeggo_v2_*` namespace. ABI major is 1, minor 0. Source package version is independent and only discoverable at runtime via `stegoeggo_source_version`.

Four independent gates enforce it, and none of them is `scripts/check.sh`:

| Gate | What it validates | Mechanism |
|---|---|---|
| `scripts/check-contract.py` (88) | `ABI-V1.md` ↔ `abi-v1-symbols.txt` agreement | Extracts every `stegoeggo*(` from the doc's ```` ```c ```` blocks, requires no duplicates, requires exact set equality with the manifest, requires the manifest be bytewise sorted and duplicate-free, and hardcodes `EXPECTED_TOTAL = 90`, `EXPECTED_BOOTSTRAP = 3`, `EXPECTED_V1 = 87` |
| `scripts/generate-header.sh --check` (73) | The committed header is drift-free | Requires cbindgen **exactly** `0.29.4` on PATH, regenerates to a temp file, asserts the regenerated symbol set equals the manifest and is 90, asserts no `struct stegoeggo_v1_.*{` body leaked, then `cmp`s against `include/stegoeggo.h` |
| `scripts/check-exports.sh` (48) | The built `cdylib` exports exactly the manifest | `nm -D --defined-only` (Linux), `nm -gU` (macOS, stripping the leading `_`), `dumpbin /EXPORTS` (Windows); hard-fails if the count is not 90 and `cmp`s the sorted list |
| `scripts/run-c-tests.sh` (52) | The header and library are consumable | Compiles `test_c_abi.c`, both parity tools with `-std=c11 -Wall -Wextra -Werror`, and `cxx_smoke.cpp` with `-std=c++17 -fsyntax-only`; runs the suite |

`cbindgen.toml` pins the generation contract: `language = "C"`, `cpp_compat = true`, `include_guard = "stegoeggo"`, `usize_is_size_t = true`, `sort_by = "Name"`, `style = "Both"`, `documentation = false`. `autogen_warning` stamps the header with `Generated by cbindgen 0.29.4. Do not edit.` `ABI-V1.md` §15 requires cbindgen to stay a standalone dev tool — never a `build.rs` dependency, never a prerequisite of an ordinary root build.

### CI and release

`.github/workflows/c-binding.yml` runs on **push and pull_request to `main`**, path-filtered to `bindings/c/**`, `src/**`, `stegoeggo-stego/**`, `Cargo.toml`, `Cargo.lock`, `tests/fixtures/**`, and itself. One job on `ubuntu-24.04`: Rust 1.89 pinned via `RUSTUP_TOOLCHAIN=1.89.0` (overriding `rust-toolchain.toml`), cbindgen 0.29.4 installed, then `cargo check` / `clippy -D warnings` / `test` on `bindings/c/Cargo.toml`, then the header drift check, a release build, the export audit, the C11/C++17 consumer suite, and `parity.sh`. It uploads `libstegoeggo_c.so`, the header, and the manifest as artifact `c-binding-linux-x86_64`. It never publishes. Its own header comment states it is "a binding-specific signal independent of the standard Rust Check gate in `ci.yml`; it does not add C tooling to `./scripts/check.sh`".

Two things are easy to get wrong here, so state them explicitly:

- **`release/eggpack/build-bindings.toml` and `qualification-bindings.toml` do not cover this crate.** "Bindings" in eggpack's schema means the *binding* from a target triple to a Cargo output and a smoke command, and both files map to `package = "stegoeggo-cli"`. Neither mentions `stegoeggo-c`. (The same trap is flagged in [bindings-node.md](bindings-node.md) and [bindings-python.md](bindings-python.md).)
- **The C release path is the manual `.github/workflows/release-c.yml`** (`workflow_dispatch`, never publishes). Five-target native qualification — Linux x86_64/aarch64 GNU, macOS x86_64/arm64, Windows x86_64 MSVC — is M010's responsibility and is *not* re-run by the push/PR workflow. `examples/smoke.c` is the dependency-free consumer shipped inside every release-c artifact bundle.

### Tests

26 Rust `#[cfg(test)]` cases across eight files (codes 4, error 4, input 5, handles 3, panic 3, operations 3, buffer 2, report 2; `limits.rs`, `notice.rs`, `request.rs` have none). `operations.rs:200-233` is a genuine byte-for-byte Rust↔C equivalence test on a synthesized 16×16 PNG. `tests/test_c_abi.c` (613) is 12 test functions with a `CHECK` counter: bootstrap, NULL-free no-ops, detect_format, metadata-only protect/verify, hidden-marker round trip, unprotected verify, HMAC, HMAC-missing-key, resource-limit error, malformed inputs, invalid arguments, presets-and-clearing. `tests/minipng.h` (138) is a dependency-free PNG synthesizer (CRC32 table + chunk writer) so the C suite needs no image library.

`parity.sh` (83) is two-way and uses the real CLI: C protects (twice, asserting determinism for seed 424242 and timestamp `2026-01-01T00:00:00Z`) → C verifies its own output and the Rust CLI verifies it; then the CLI protects with `--preset legal-notice-with-stego --seed 424242` → C verifies the CLI's output. Its default fixture is `tests/fixtures/conformance/canonical/canonical_complete.png`, which I confirmed exists. **I read all of the above; I executed none of it** — no `cargo`, `cc`, `c++`, `make`, or `check-contract.py` run was performed for this document, so every gate, parity assertion, and test description here is *read from configuration and source*, not an observed result.

## Review risks

Each points at specific source. None is speculative.

- **No use-after-free or double-free protection.** Raw `Box::into_raw` pointers with no handle table and no generation counter (`input.rs:20-31`; `ABI-V1.md` §2 makes it a caller contract). A stale handle is UB, and unlike a generation-tagged design the binding cannot detect it. This is the single largest structural difference from a managed FFI layer.
- **`to_request()` and `rebuild()` are allowlists that silently drop future fields.** `handles.rs:156-195` re-lists every canonical `ProtectionRequest` field; `limits.rs:19-65` re-lists all 18 `ResourceLimits` fields. A new canonical field is dropped from every C protect call, and a new limit is reset to its default on every limits setter — with no compile error and, for limits, no test asserting the list is complete.
- **`u32::MAX` is an overloaded sentinel.** `stegoeggo_v1_execution_report_warning_at` returns it for a NULL report, an out-of-range index, **and** (via `codes.rs:299`) for a `ProtectionWarning` variant added after v1. `ABI-V1.md` §12.1 claims it is "never a valid warning code", which is true for the eight codes frozen today but not by construction.
- **Every `*_to_code` silently downgrades unknown variants to the weakest value.** `policy_to_code`/`dmi_to_code` → `UNSPECIFIED`, `format_to_code` → `FORMAT_UNKNOWN`, `strength_to_code` → `EVIDENCE_NO_NOTICE_FOUND` (`codes.rs:143,175,195,284`). All the canonical enums are `#[non_exhaustive]`, so these arms are required — and `strength_to_code` in particular reports "no notice found" (a weaker claim) for a variant it does not recognise. The inbound `*_from_code` functions do the safe thing: they return `None` and become invalid-argument.
- **The 18 resource-limit setters validate nothing.** `ResourceLimitsBuilder::build()` is infallible, so `set_max_input_bytes(0)` succeeds and makes every later `protect` fail. Contract-correct per `ABI-V1.md` §8, undocumented as a hazard.
- **All 16 notice text setters report the same error message.** `set_text_field` hardcodes `name = "text"` (`notice.rs:17`), so a rejected `creator` and a rejected `licensor_url` both produce `Invalid argument: text`. With no per-call field context, a C caller cannot tell which setter was rejected except by its own bookkeeping.
- **`limits.rs` partial-mutation hazard.** `set_limit` `mem::take`s the inner limits and reassigns inside the closure; a panic in `rebuild` leaves the handle at `ResourceLimits::default()` rather than its prior value. Unreachable today, but the code has no rollback.
- **`emit_error`/`clear_error` overwrite the caller's error slot without freeing.** `error.rs:174-191`. Reusing one `stegoeggo_v1_error_t *err` across two failing calls without an intervening free leaks the first handle; on the success path the slot is NULLed rather than freed.
- **`out_format` is the one out-parameter not zeroed on entry.** `stegoeggo_v1_detect_format` writes `*out_format` only on success (`operations.rs:20-31`), so a failure leaves the caller's uninitialised value in place — a deviation from `ABI-V1.md` §13 step 2, and different from the handle out-slots, which `require_out` always NULLs.
- **`stegoeggo_v1_error_message_data` is not NUL-terminated.** It returns `message.as_ptr()` of a Rust `String` (`error.rs:214-222`), and the correct usage is always `data` + `len` — which is what every shipped C consumer does. But `ABI-V1.md` §1 explicitly promises NUL termination for `stegoeggo_source_version` (and `codes.rs:98` does append `"\0"`), so a reader who generalises from the bootstrap symbols to the message accessor and writes `printf("%s", …)` reads out of bounds. The header types it `const char *`, which invites exactly that mistake.
- **`usage_terms_lang` alone injects an empty usage-terms string.** `handles.rs:101-106` builds `LocalizedText::with_lang(usage_terms.unwrap_or_default(), lang)`, so a lang-only configuration sets `LegalMetadata::usage_terms = Some("")`. The in-crate test at `handles.rs:244-250` asserts only the language round-trips, so this is unguarded — a real data-correctness issue for a notice that sets a language and defers the text.
- **Two canonical report subtrees are unreachable.** `ExecutionReport.resource_usage` and `embed_summary` have no C accessor and no execution-report JSON (deliberate, `ABI-V1.md` §12.1); `VerificationReport.{signatures,bindings,trust,diagnostics}` survive only inside `verification_report_json`.
- **`rights_metadata: false` is unreachable.** `handles.rs:158` hardcodes it to `true` and there is no symbol to change it, so `ContradictoryLegalClaims`/`MetadataInjectionDisabled` are in the frozen warning table but cannot occur through this ABI.
- **Same-handle concurrent reads are explicitly unqualified.** `ABI-V1.md` §14 calls sharing one request across threads for parallel `protect` UNSUPPORTED. The absence of global state makes it *probably* fine, but it was not qualified and is not part of the contract — a caller who reads "thread-safe" from the README design rules and shares a request is outside what was tested.
- **The `panic = "unwind"` guarantee is a build property, not a source property.** Nothing in `src/` asserts it. A consumer linking a differently-configured build (or a future contributor adding `panic = "abort"` to this profile) loses panic containment entirely, and no compile error or test would catch it.
- **`details_json` and `verification_report_json` cannot report allocation failure.** `ABI-V1.md` §11.1/§12.2 promise NULL "on NULL input (or allocation failure)", but the implementation is `Box::into_raw(Box::new(…))` (`error.rs:242`, `report.rs:175`), and Rust's default allocator **aborts** on OOM rather than returning null. That half of the promise is unimplementable in this binding. `verification_report_json`'s `Err(_) => null` arm is likewise indistinguishable from a NULL input.
- **Destructors are not panic-guarded.** The seven `*_free` functions are bare `extern "C"` with no `catch_unwind`, unlike every fallible export. Acceptable today (no reachable `Drop` panics) but it is a latent divergence from `ABI-V1.md` §13.
- **`stegoeggo_source_version` reports the binding's version, not the library's.** It is `CARGO_PKG_VERSION` of `stegoeggo-c` (`codes.rs:98`), while `ABI-V1.md` §1 describes it as "the `stegoeggo` package version the library was built from" — a different crate, since the root package is also named `stegoeggo`. The two agree only because of the `=0.4.2` pin, and nothing enforces that they stay in lockstep. Same class of issue as the node binding's `stegoeggoVersion()`.
- **Not covered by the standard gates.** `exclude = ["bindings"]` means `scripts/check.sh` (fmt, clippy, `--workspace` test) skips this crate entirely. `c-binding.yml` is Linux-only and *does* run `clippy -D warnings` and `cargo test`, but it never runs `cargo fmt --check`, and the five-target qualification is manual-only.

## Relationship to other modules

- [bindings-node.md](bindings-node.md) — napi-rs sibling. Same canonical entry points and the same `with_channels`/`rebuild` allowlist shape, but no ABI contract, no code table, and nine `ERR_STEGOEGGO_*` string codes instead of the frozen integers.
- [bindings-python.md](bindings-python.md) — PyO3 sibling. Loses the numeric code space entirely (exception classes); its `Io` handling is *less* discriminating than this ABI's.
- [overview.md](overview.md) — workspace layout: 4 members plus 3 excluded binding crates.
- [pipeline.md](pipeline.md) — owns `process_request_bytes` and `process_request_bytes_with_report`, the two of four entry points `operations.rs` calls.
- [verification.md](verification.md) — owns `VerificationReport`; `report.rs` flattens five of its eight subtrees into four scalars and defers the rest to JSON.
- [resource-limits.md](resource-limits.md) — owns `ResourceLimits` and the six structured limit error variants that `error.rs` turns into code 9 plus a `resource` discriminator.
- [error.md](error.md) — owns the 21 `#[non_exhaustive]` `Error` variants; `error.rs` maps 14 arms of them and the rest to code 10.
- [types.md](types.md) — `RightsNotice`, `RightsPolicy`, `ProtectionRequest`, `ProcessingOptions`, and the enums whose discriminants are deliberately *not* used as ABI values.
- [carrier-surface.md](carrier-surface.md) — the carrier underneath. `StegoError` reaches this ABI only through the library's `From` impl, and `CarrierLimits` is never exposed.
- [legal-metadata-field-mapping.md](legal-metadata-field-mapping.md) — the 16 legal fields, of which C can set 1.
- [container-walk.md](container-walk.md) — the accounting walk whose counters populate `resource_usage`, the report subtree C cannot read.
- [cli.md](cli.md) — the sibling frontend that `parity.sh` cross-checks against in both directions.
- [constants.md](constants.md) — the canonical enum values; the ABI's 67 `#define`s are a hand-assigned, frozen restatement of them, not a derivation.
- [tooling.md](tooling.md) — owns the binding workflows, including `c-binding.yml` as an independent push+PR signal that is not part of the required `ci.yml` gate.
- [testing.md](testing.md) — Rust-side test organisation; the C suite, the parity tools, and the C++17 smoke are a separate, separately-gated layer.
