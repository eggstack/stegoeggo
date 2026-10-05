# Python Bindings (PyO3)

**Source:** `bindings/python/src/lib.rs` (~1895 lines, `cdylib` `_stegoeggo_native`, not a workspace member) · `bindings/python/python/stegoeggo/{__init__.py,_files.py,_native.pyi}` (40 / 74 / 408 lines) · `bindings/python/tests/*.py` (7 files, 976 lines).

PyO3 0.27 projection of the canonical Rust **byte** API. It wraps `process_request_bytes*` and `verify_image_bytes_report*` and nothing else — no pixel/`DynamicImage` path, no reimplementation of protection logic. The Rust crate is the single semantic source of truth; this crate is a transport and marshalling layer.

## Position and dependency direction

```
stegoeggo-python (bindings/python, cdylib)  →  stegoeggo  →  stegoeggo-stego
```

`bindings/python/Cargo.toml` declares its own `[workspace]` (it is a separate workspace root) and depends on the library by path with an exact pin:

```toml
stegoeggo = { path = "../..", version = "=0.4.2", default-features = false }
```

`default-features = false` means the binding ships the library with **every** optional feature off: no `async`, `parallel`, `signatures`, `detached-manifest`, `iscc`, `conformance`, or `webp`. Three consequences visible in the surface: there is no async or parallel entry point; there is no `keygen`/`sign`/`verify_manifest` equivalent of the CLI's `signatures`-gated commands; and JPEG/WebP byte carriers are reached through the root crate's own re-encoders rather than the carrier's `webp` feature facade. See [carrier-surface.md](carrier-surface.md) for the generic carrier surface that the library delegates to.

The root `Cargo.toml` opts the whole tree out of the workspace:

```toml
exclude = ["bindings"]
```

That single line covers all three binding crates (`c`, `node`, `python`) — each has its own `Cargo.lock`, its own MSRV check, and its own release workflow. The binding version `0.4.2` currently matches the library `0.4.2` and `stegoeggo-stego` `0.4.2`, but nothing mechanically enforces that: the pin is a `=0.4.2` **path+version** dependency, so Cargo resolves it from the local tree and the numeric agreement is maintained by hand across three manifests plus `pyproject.toml`.

## Exported Python surface

All native symbols are registered by one `#[pymodule]` function. There are 27 `m.add(..., py.get_type::<T>())` registrations, 2 module constants, and 5 `#[pyfunction]` wrappers.

**Functions** (`bindings/python/src/lib.rs`)

| Python | Rust entry point | Notes |
|--------|------------------|-------|
| `protect(data, request) -> bytes` | `process_request_bytes` | line 1747 |
| `protect_with_warnings(data, request) -> (bytes, list[ProtectionWarning])` | `process_request_bytes_with_warnings` | line 1760 |
| `protect_with_report(data, request) -> (bytes, ExecutionReport)` | `process_request_bytes_with_report` | line 1779 |
| `verify(data, mac_key=None, resource_limits=None) -> VerificationReport` | `verify_image_bytes_report` / `verify_image_bytes_report_with_limits` | line 1795, `#[pyo3(signature = (data, mac_key=None, resource_limits=None))]` |
| `detect_format(data) -> str \| None` | `ImageOutputFormat::from_magic_bytes` → `.extension()` | line 1742, no GIL detach (no allocation, no I/O) |

**Request/notice DTOs**

| Class | Rust wrapper | Constructor path |
|-------|--------------|------------------|
| `RightsNotice` | `PyRightsNotice { inner: RustRightsNotice }` | `new()` → `RustRightsNotice::new()`; 16 `with_*` builders, all `std::mem::take`-based |
| `ProtectionRequest` | `PyProtectionRequest { inner: Option<RustRequest> }` | static `metadata_only` / `with_hidden_marker` / `from_preset`; 13 instance `with_*` builders (14 `with_` fns total, counting the static `with_hidden_marker`) |
| `ProcessingOptions` | `PyProcessingOptions { inner: RustProcessingOptions }` | `new()` → `RustProcessingOptions::default()`; 8 `with_*` |
| `ResourceLimits` | `PyResourceLimits { inner: RustLimits }` | `defaults()` / `builder()` only — no `#[new]` |
| `ResourceLimitsBuilder` | `PyResourceLimitsBuilder { inner: Option<RustLimitsBuilder> }` | 12 `with_max_*` + `build()` |

**Enums** — all are `#[pyclass(name = "...", eq, eq_int)]` on a fieldless Rust enum. They are real Python classes holding class-level int constants, **not** `enum.IntEnum`; they compare equal to their int discriminant because of `eq_int`.

| Class | Variants | Also exposes |
|-------|----------|--------------|
| `RightsPolicy` | `Unspecified`…`ProhibitedSeeConstraints` (0–6) | `name`, `__str__`, `__repr__` |
| `DmiValue` | `Unspecified`, `Allowed`, `ProhibitedAiMlTraining`, `ProhibitedGenAiMlTraining`, `ProhibitedExceptSearchEngineIndexing`, `Prohibited`, `ProhibitedSeeConstraints` (0–6) | `name`, `__str__` |
| `ImageOutputFormat` | `Png`, `Jpeg`, `WebP` (0–2) | `from_extension(ext)` static → `RustFormat::from_extension` |
| `MetadataUpdatePolicy` | `ReplaceStegoOwned`, `FailOnConflict`, `PreserveExisting` | `name`, `__str__` |
| `ProtectionPreset` | `LegalNotice`, `LegalNoticeWithStego`, `AuthenticatedProvenance`, `Maximal` | `name`, `__str__` |
| `AuthenticationMode` | `None`, `Hmac` | `name`, `__str__` |
| `ProtectionWarning` | `MissingMacKey`…`MissingRightsConstraints` (0–7) | `name`, `__str__` |
| `VerificationStatus` | `Verified`, `Invalid`, `NotFound` (0–2) | `name`, `__str__` |
| `EvidenceStrength` | `NoNoticeFound`…`MetadataNoticeAndAuthenticatedProvenance` (0–3) | `name`, `__str__` |

`HiddenMarkerMode` is the one non-enum class: `disabled()`, `seed_only()`, `best_effort()`, `tiled(tile_size)` static constructors around `RustHidden`, plus `is_tiled` / `tile_size` getters and `__eq__`. `tiled` range-checks `32..=1024` and raises `PyValueError` (line 573).

**Report classes**

| Class | Wraps | Getters |
|-------|-------|---------|
| `ExecutionReport` | `stegoeggo::ExecutionReport` (`src/types/request.rs:744`) | `effective_policy`, `effective_dmi`, `metadata_injected`, `stego_attempted`, `stego_succeeded`, `format_transcoded`, `warnings` (list), `has_degradation`, `any_succeeded`, `resource_usage` |
| `ResourceUsage` | `stegoeggo::ResourceUsage` (`src/resource_limits.rs:357`) | 10 read-only counters: `input_bytes`, `png_chunks_scanned`, `jpeg_segments_scanned`, `webp_riff_chunks_scanned`, `xmp_bytes_parsed`, `metadata_fields_extracted`, `metadata_bytes_copied`, `tile_origins_checked`, `verification_seeds_tried`, `peak_allocations_bytes` |
| `VerificationReport` | `stegoeggo::verification::VerificationReport` | 22 getters + `to_dict()` / `to_json()` |

**Module constants** — `__version__` and `__stegoeggo_version__` are both `env!("CARGO_PKG_VERSION")` of the *binding* crate (line 1815-1816). There is no separate value for the linked library version; `__stegoeggo_version__` is an alias, not an independent probe. `test_smoke.py:7-8` pins both to `"0.4.2"` as literal strings.

## The GIL and threadpool

PyO3 0.27 renamed the historical `py.allow_threads` to `py.detach`. All four blocking calls use it, and it is scoped to exactly the library call — no Python object is touched inside the closure:

```rust
#[pyfunction]
fn protect<'py>(
    py: Python<'py>,
    data: &[u8],
    request: &PyProtectionRequest,
) -> PyResult<Bound<'py, PyBytes>> {
    let req = request.inner.as_ref().expect("request initialized").clone();
    let bytes = py
        .detach(|| process_request_bytes(data, &req))
        .map_err(|e| map_error(py, e))?;
    Ok(PyBytes::new(py, &bytes))
}
```

`protect_with_warnings` (line 1766), `protect_with_report` (line 1785), and `verify` (line 1803) use the identical pattern. The ordering is deliberate and correct: the `&[u8]` borrow and the `RustRequest` clone are taken **before** `detach`, so no Python API is entered while the GIL is released, and `map_error(py, …)` runs after the GIL is re-acquired (it must — it constructs Python exception objects and calls `setattr`).

**The binding is sync-only.** There is no `#[pyfunction] async fn`, no `pyo3-async-runtimes`, and no coroutine wrapper. The `async` feature of the library is not enabled, so `process_request_bytes_async` is not compiled in and `process_request_bytes_parallel` is not either.

For a threaded web server this is the load-bearing property: `protect`/`verify` release the GIL for the entire duration of the image decode, DCT/LSB embed or extract, and re-encode, so N worker threads running `protect` in parallel do execute the Rust work concurrently. Two caveats follow from the source rather than from PyO3: the work is *not* moved to a separate threadpool (it runs on the calling thread with the GIL dropped, so a single call still occupies one thread for its full duration), and `verify` drops the GIL for a path that is frequently microseconds-to-milliseconds on a cache hit, so the `detach`/`attach` pair is pure overhead for the common no-marker case.

## Type conversion

| Direction | Mechanism | Copy cost |
|-----------|-----------|-----------|
| `data: &[u8]` | PyO3 extracts a borrowed slice from an immutable `bytes` object | **Zero** — no copy into Rust |
| image return | `PyBytes::new(py, &bytes)` | **One full copy** of the output `Vec<u8>` |
| `mac_key: Option<Vec<u8>>` | owned extraction, then `mac_key.as_deref().unwrap_or(&[])` | One copy of the key, taken under the GIL |
| `request: &PyProtectionRequest` | borrowed `#[pyclass]` | `request.inner.clone()` deep-copies the whole `RustRequest` (notice strings, MAC key bytes) under the GIL |
| enums | `#[pyclass(eq, eq_int)]` fieldless enums + explicit `From` impls in both directions | `Copy`, no allocation |
| `HiddenMarkerMode` | newtype over `RustHidden` with a `From` impl | `Clone`, no allocation |
| `VerificationReport` | `#[pyclass]` wrapper, **not** a dict | getters allocate per call |

`data: &[u8]` is a borrowed slice of the caller's `bytes`. The binding never accepts `bytearray` or `memoryview`, and never copies the input up front; the library owns all intermediate buffers. The output, by contrast, is always a fresh `bytes` — the `Vec<u8>` the library returns is copied into a new Python object and then dropped, so peak memory is roughly library-output + Python-bytes at the moment of return.

The nested Rust `VerificationReport` is converted by **flattening**, not by projecting the struct shape. `PyVerificationReport` re-exposes sub-results as prefixed flat getters, so `hidden_marker.status()` becomes `hidden_marker_status`, `authentication.key_matched()` becomes `authentication_key_matched`, and `trust.reason()` becomes `trust_reason`. There is no `VerificationReport.hidden_marker` sub-object on the Python side.

Full nested structure is available only through serde, and only on demand:

```rust
fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
    let json = serde_json::to_string(&self.inner).map_err(|e| {
        PyErr::new::<StegoEggoError, _>(format!("serialization failed: {e}"))
    })?;
    let parsed: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
        PyErr::new::<StegoEggoError, _>(format!("json parse failed: {e}"))
    })?;
    json_to_pydict(py, &parsed)
}
```

`json_to_pydict`/`json_to_pyobject` (line 1692) walk the `serde_json::Value` tree and rebuild `dict`/`list`/`str`/`int`/`float`/`None`. `serde_json::Value::Bool` is handled by importing `builtins` and fetching the `True`/`False` singleton rather than by `into_pyobject`. The round-trip string→`Value`→dict is a double pass over the report. This is the same JSON the CLI emits, so `to_dict()` is the only path that surfaces the `signatures`, `bindings`, and `diagnostics` sub-reports documented in [verification.md](verification.md) — the `#[pyclass]` getters expose none of them.

`Path`/`PathBuf` never cross the boundary. The native module deals only in `bytes`, DTOs, and exceptions; all path handling is pure-Python (below).

## Error handling

`stegoeggo::Error` is mapped by one exhaustive-match-with-catch-all function, `map_error` (line 75). Nine exception classes are created with `create_exception!` on `stegoeggo._native`, all deriving from a common `StegoEggoError` base, which itself derives from `PyException` — not from `Exception` directly, and never from a builtin like `ValueError` or `OSError`.

| `stegoeggo::Error` | Python exception | Structured attributes | C ABI v1 code |
|---------------------|-------------------|-----------------------|---------------|
| `Config(_)` | `InvalidConfigError` | — | 2 `ERR_INVALID_CONFIGURATION` |
| `InvalidFormat(_)` | `InvalidFormatError` | — | 3 `ERR_INVALID_FORMAT` |
| `ImageDecode` / `ImageEncode` / `Image` / `ImageTruncated` | `EncodeDecodeError` | — | 4 `ERR_ENCODE_DECODE` |
| `Metadata(_)` | `MetadataError` | — | 5 `ERR_METADATA` |
| `Steganography(_)` | `SteganographyError` | — | 6 `ERR_STEGANOGRAPHY` |
| `InsufficientCapacity { required, available }` | `InsufficientCapacityError` | `required`, `available` | 7 `ERR_INSUFFICIENT_CAPACITY` |
| `PayloadVerification` / `Crypto` | `VerificationError` | — | 8 `ERR_VERIFICATION` |
| `InputTooLarge { size, limit }` | `ResourceLimitError` | `resource="input_bytes"`, `size`, `limit` | 9 `ERR_RESOURCE_LIMIT` |
| `DimensionsExceeded { .. }` | `ResourceLimitError` | `resource="dimensions"`, `width`, `height`, `max_width`, `max_height` | 9 |
| `ContainerLimitExceeded { kind, count, limit }` | `ResourceLimitError` | `resource="container"`, `kind`, `count`, `limit` | 9 |
| `MetadataLimitExceeded { kind, size, limit }` | `ResourceLimitError` | `resource="metadata"`, `kind`, `size`, `limit` | 9 |
| `VerificationBudgetExceeded { kind, count, limit }` | `ResourceLimitError` | `resource="verification_budget"`, `kind`, `count`, `limit` | 9 |
| `ResourceLimitExceeded(_)` | `ResourceLimitError` | `resource="carrier"` only | 9 |
| `Io`, `Serialization`, `Iscc`, `Task`, anything new | `StegoEggoError` (catch-all `_`) | — | 1 / 10 |

**Codes are not preserved.** Python has no integer code: the numeric channel the C ABI uses is replaced entirely by exception class identity plus `setattr`-injected attributes. The *category set* is 1:1 with the ten C ABI categories in `bindings/c/include/stegoeggo.h` (`STEGOEGGO_V1_ERR_*` 1–10), and the Python side is strictly more informative per limit error, because the C ABI flattens all six limit variants into code 9 plus a message while Python keeps the `resource` discriminator and the numbers. It is strictly *less* discriminating in one respect: the C ABI's `ERR_INVALID_ARGUMENT` (1) and `ERR_INTERNAL` (10) are separate codes, whereas Python merges them and every future variant into the single base `StegoEggoError` arm, which is also the fallback for `Io` and `Serialization` — an I/O failure and a genuinely unexpected internal error are indistinguishable to a `except stegoeggo.StegoEggoError` caller.

Two security properties are asserted in-source and in-test. The message is taken once via `err.to_string()` before the match, so no secret is re-derived per arm. `test_errors.py:170-199` and the crate's own `#[cfg(test)]` case `carrier_resource_limit_projects_to_resource_limit_error` (line 1859) both check that a `super-secret-mac-key` never appears in the exception text, its `repr`, or its `__dict__`, and the Rust test additionally asserts the carrier arm sets **no** `size`/`limit` attributes at all (they raise `AttributeError`).

Note that `pyo3::exceptions::PyValueError` is the one non-`StegoEggoError` exception raised anywhere in the crate: `HiddenMarkerMode.tiled()` raises it for an out-of-range tile size (line 575). Python callers must therefore catch both hierarchies.

## Pure-Python layer

`python/stegoeggo/__init__.py` (40 lines) is a re-export shim and nothing else. It imports `protect_file`/`verify_file` from `._files` first, then 31 names from `._native`, then aliases `__version__ = __binding_version__`. It defines **no `__all__`**, so the public surface is whatever the import statements bind — including every name the shim itself was written to re-export, and no marker distinguishing deliberate API from convenience. `_files` imports `from . import _native` while `__init__` is still executing; the binding resolves because `_files` only ever dereferences `_native` at call time.

`python/stegoeggo/_files.py` (74 lines) adds the only I/O. It exists so the Rust layer never sees a path:

```python
def protect_file(
    path: str,
    request: _native.ProtectionRequest,
    output_path: Optional[str] = None,
) -> None:
    src = Path(path)
    dst = Path(output_path) if output_path is not None else src
    data = src.read_bytes()
    protected = _native.protect(data, request)
    _atomic_write_bytes(dst, protected)
```

`_atomic_write_bytes` is the durability contract: `mkdir(parents=True, exist_ok=True)` on the parent, `tempfile.mkstemp(prefix=".stegoeggo-", suffix=".tmp", dir=str(target_dir))` in the **same directory** as the target (so `os.replace` stays on one filesystem and is therefore atomic), `flush()` + `os.fsync()` before the swap, then `os.replace`. The `except BaseException` handler unlinks the temp file and re-raises. `verify_file` is three lines: read, delegate, return.

The same-directory rule means a partially written file can never be observed at `dst`, and the source file is untouched if the native `protect` raises — the failure happens before any write is attempted.

`_native.pyi` (408 lines) is a hand-maintained type stub shipped via `pyproject.toml`'s `include` alongside an empty `py.typed` marker, so type checkers see the surface without importing the extension.

## Testing strategy

7 test modules, 976 lines, run under `pytest`. **I read these tests; I did not execute them** — per the task's constraint no `pytest`, `maturin`, or `cargo build` was run against this crate, so the assertions below are claims about what the code checks, not observed pass/fail results. The crate also carries 2 Rust `#[cfg(test)]` cases inside `lib.rs` (line 1854) that call `Python::initialize()` / `Python::attach` directly to test `map_error` without building a wheel.

| File | Lines | What it asserts |
|------|-------|-----------------|
| `test_smoke.py` | 53 | Import succeeds; `__version__` and `__stegoeggo_version__` are both the literal `"0.4.2"`; all 8 concrete exceptions subclass `StegoEggoError`; the 7 `RightsPolicy` and 3 `ImageOutputFormat` constants exist; `detect_format` on PNG/JPEG/WebP magic, on `b"not an image"`, and on `b""`; `ImageOutputFormat.from_extension` case-insensitivity and `None` for `gif` |
| `test_request.py` | 155 | DTO construction only, no images. The full 16-call `RightsNotice` builder chain and read-back of every field; empty-notice defaults (`has_legal_content is False`, all `None`); the three `ProtectionRequest` static constructors including all 4 presets; the 11-call request builder chain; `repr` does not leak the MAC key; `ResourceLimits.builder()` repr contains `8192x8192`; `HiddenMarkerMode` equality/`is_tiled`/`tile_size` and `ValueError` on `tiled(8)`/`tiled(2048)`; `ProcessingOptions` repr mentions the format |
| `test_protect.py` | 164 | 5 PNG + 2 JPEG + 1 WebP fixtures protected to a PNG signature; hidden-marker protect changes bytes; return type is `bytes`; same seed+timestamp → byte-identical output; different seeds → different stego output; `protect_with_warnings` returns a `list`; `protect_with_report` field-by-field (`metadata_injected`, `stego_attempted`, `stego_succeeded`, `effective_policy`, `effective_dmi`, `format_transcoded`, `resource_usage.input_bytes == len(data)`); stego report succeeds; `repr(report)` has no MAC key; `protect_file` + `verify_file` round trip |
| `test_verify.py` | 150 | Unprotected image → `rights_found False` / `NotFound` / `NoNoticeFound`; metadata-only → holder + `license_url` + `MetadataNoticeOnly`; correct MAC key → `key_matched`, `Verified`, `hidden_marker_seed == 42`, `hidden_marker_payload_version == 3`, `MetadataNoticeAndAuthenticatedProvenance`; wrong key and missing key both → `Invalid`; no MAC key in `repr` or `to_json()`; malformed / truncated-to-128-bytes / empty inputs return a report rather than raising; `to_dict()` shape (`d["rights"]["found"] is True`) and `to_json()` keys; explicit `ResourceLimits.defaults()` accepted |
| `test_errors.py` | 199 | The exception matrix. `InvalidFormatError` MRO parent is `StegoEggoError`; truncated PNG → `EncodeDecodeError` with "truncated" in the message; `MISSING_RIGHTS_CONSTRAINTS` warning name; then one test per structured limit arm asserting `resource`, `size`/`limit`, `width`/`height`/`max_width`/`max_height`, `kind`/`count`/`limit`, and the exact `kind == "PNG chunks"` string; two secret-leak tests (limits error and invalid-format error) |
| `test_files.py` | 143 | The atomic-write contract. In-place replacement; explicit `output_path` leaves the source byte-identical; native failure leaves the source unchanged; monkeypatched writer failure leaves the source unchanged; no `.stegoeggo-` temp leftovers; `verify_file` agrees with byte-level `verify`; `_atomic_write_bytes` replaces an existing file; a monkeypatched `os.replace` failure cleans the temp and leaves the target **non-existent** |
| `test_parity.py` | 112 | Cross-language and determinism. See below |

### What "parity" means in `test_parity.py`

The module docstring states two goals, and only one of them is actually cross-language:

1. **Python reads Rust output** — the docstring claims this, but no test does it. There is no fixture of CLI-produced bytes in the tree and no test that shells out to `protect` and verifies the result with Python. The only sub-process test goes the other direction.
2. **Rust CLI reads Python output** — `test_rust_cli_round_trip` (line 70) is the sole cross-language check. It protects a fixture in Python, writes it to a temp file, and runs `stegoeggo verify <file> -o <dir>`, asserting only `result.returncode in (0, 3)`.
3. **Within-Python consistency** — the other four tests: self-consistency (protect→verify reads back the holder), literal XMP content presence, byte-identical output across two invocations with a fixed seed+timestamp, and a `protect_with_report` field summary serialized to sorted JSON.

So the real cross-binding guarantee is one direction, one fixture, and it is **skipped whenever `shutil.which("stegoeggo")` is `None`** (line 66). It is also weak in two specific ways a reviewer should know: the accepted return-code set includes `3`, which is the CLI's missing/invalid-evidence code, so the assertion passes whether or not the CLI actually found the notice; and the `-o` flag it passes does not exist on the CLI's `verify` command — `VerifyArgs` in `stegoeggo-cli/src/args.rs:391` accepts only `--key`, `--json`, and `-v/--verbose`. A CLI on `PATH` makes this test fail on an argument-parsing error, not on a parity defect. `test_protection_summary_json_round_trip`'s `assert "METADATA_INJECTED" not in raw` is a negative assertion against a name that is never produced by any code path; it guards nothing.

There is no parity test against the C or Node bindings, and no shared golden-hash fixture across bindings — the deterministic-output test compares two calls in the *same* process, so it would not catch a change to the library's serialization that both calls agree on. See the `Bindings` section of [verification.md](verification.md) for the report-side binding record, which is unrelated to language bindings.

**Fixtures.** `tests/fixtures/__init__.py` (6 lines, docstring only) states that `conformance/canonical/` is a *copy* of the Rust crate's `tests/fixtures/conformance/canonical/`. I verified both directories: 17 files with identical names, including `canonical_{complete,copyright_only,independent,policy_only,unicode,multi_creator,alt_prefix}` in `.png`/`.jpg`/`.webp` variants. The copy is unmanaged — nothing in the Rust build or in the Python workflows re-syncs it, so it is a drift risk independent of any code change. `python-binding.yml` triggers on `bindings/python/tests/fixtures/**` precisely because those files are binding-local.

## Packaging and release contract

Build backend is maturin (`pyproject.toml`):

```toml
[build-system]
requires = ["maturin>=1.5,<2.0"]
build-backend = "maturin"
```

`[project]` declares distribution name `stegoeggo`, version `0.4.2`, `requires-python = ">=3.11"`, MIT, and an optional `test` extra pinning `pytest>=7`. `[tool.maturin]` sets `python-source = "python"`, `module-name = "stegoeggo._native"`, and `include = ["_native.pyi", "py.typed"]` so the stub and marker land in the wheel.

`bindings/python/Cargo.toml` names the crate `stegoeggo-python` (distinct from the distribution name `stegoeggo`), sets `[lib] name = "_stegoeggo_native"`, `crate-type = ["cdylib"]`, `publish = false`, and `rust-version = "1.89"` — same MSRV as the library. Release profile is `lto = true`, `codegen-units = 1`, `opt-level = "s"`, `strip = "symbols"`, and `panic = "unwind"` (deliberately **not** `abort`, so a panic inside `detach` unwinds back into PyO3's panic→exception conversion rather than killing the process).

**ABI tags.** PyO3 is built with `features = ["abi3-py311"]`, and `[tool.maturin] features = ["pyo3/abi3-py311"]` restates it. This yields the stable ABI3 tag `cp311-abi3-<platform>`, so one wheel per platform covers CPython 3.11 through 3.14 — which is exactly the set enumerated in the `pyproject.toml` classifiers.

**Eggpack does not build this.** `release/eggpack/build-bindings.toml` and `qualification-bindings.toml` are named "bindings" in the eggpack sense — the *binding* from a target triple to a Cargo output and to a smoke command — and both describe the CLI binary only:

```toml
"aarch64-apple-darwin" = [{ selector = { kind = "direct" }, package = "stegoeggo-cli", binary = "stegoeggo" }]
```

and `argv = ["version"]` for the smoke. Neither file mentions `stegoeggo-python`, `pyo3`, or `maturin`; `release/eggpack/distribution.toml` carries the CLI asset matrix. There is no `build-python.toml`, no `.sha256` sidecar contract for wheels, and no eggpack qualification of any Python artifact. The Python binding's release path is entirely separate and entirely manual:

| Workflow | Trigger | Scope |
|----------|---------|-------|
| `python-binding.yml` | push/PR to `main` on `bindings/python/**`, `src/**`, `stegoeggo-stego/**`, `Cargo.toml`, `Cargo.lock`, fixtures | Ubuntu, CPython 3.11, Rust 1.89. `maturin build --release --out dist-ci`, `pip install` the real wheel (deliberately not `maturin develop`, which needs a venv), then `python -m pytest -v`. Never publishes. **Not** part of `scripts/check.sh` or the required `ci.yml` gate |
| `release-python.yml` | `workflow_dispatch` only, requires `source_ref` | cibuildwheel 2.22.0 across 5 targets (manylinux x86_64/aarch64 pinned to `manylinux_2_28`, macOS x86_64/arm64 with `MACOSX_DEPLOYMENT_TARGET=10.12`, win_amd64), each `CIBW_BUILD=cp311-<tag>`; a separate `maturin==1.5.0 sdist` job; then two smoke matrices that `pip install` the artifact in a clean venv and protect+verify a synthetic 16×16 PNG built with `struct`/`zlib`. Rust 1.89 is installed *inside* the Linux build container via `CIBW_BEFORE_ALL_LINUX`. Never publishes to PyPI |

Both workflows consume artifacts only; PyPI publication is a manual maintainer step outside the repository, consistent with the manual-only release policy in `RELEASING.md`. There is no version-drift guard: the workflows build whatever `source_ref` is checked out, so a tag whose three manifests disagree would produce a wheel that reports the binding's version, not the library's.

## Review risks

Each of these points at specific source.

- **Output is copied twice.** `PyBytes::new(py, &bytes)` copies the library's `Vec<u8>` into a new Python object. For a 100 MiB input (the documented `max_input_bytes` default is 100 MiB, per [resource-limits.md](resource-limits.md)) peak memory at return is roughly 2× the output. There is no `memoryview`/buffer-protocol export.
- **The whole request is deep-cloned under the GIL.** Every one of `protect`, `protect_with_warnings`, and `protect_with_report` does `request.inner.as_ref().expect(…).clone()` *before* `detach`. A request carrying a 16-field `RightsNotice` and a MAC key is fully copied on the GIL before any work is released. For large notices this is measurable GIL-held time that `detach` does not cover.
- **`ResourceLimitsBuilder.build()` is one-shot and panics on reuse.** `build()` does `self.inner.take().expect("builder initialized")` and never restores it, unlike the `with_*` methods which `take()` then reassign. Calling `.build()` twice on the same builder object hits `expect("builder initialized")` on `None` — a Rust panic across the FFI boundary, not a Python exception. `PyProtectionRequest` uses the same `Option`+`expect` idiom and restores it correctly, so the pattern is deliberate; the builder is the one place it is not.
- **`ResourceLimits` are opt-in and default to the library's own defaults.** `verify`'s `resource_limits` parameter is `None` by default, in which case the binding calls the unbounded `verify_image_bytes_report`. A caller who never touches `ResourceLimits` gets whatever `RustLimits::default()` the library applies internally, with no binding-level ceiling, and `protect`/`protect_with_report` have **no** `resource_limits` parameter at all — limits reach protect only via `ProtectionRequest.with_resource_limits`. The binding adds no limit of its own in any path.
- **Error code lossiness is real and asymmetric.** Every `Error` variant without a `map_error` arm — `Io`, `Serialization`, `Iscc`, `Task`, and any variant added later — collapses into the base `StegoEggoError`. The C ABI's separate `ERR_INVALID_ARGUMENT` (1) and `ERR_INTERNAL` (10) codes have no Python equivalent, so a caller cannot distinguish an I/O failure from an internal bug.
- **`verify` drops the GIL for a path that is usually cheap.** Unlike `protect`, the overwhelmingly common `verify` outcome is "no marker found", which returns quickly. The unconditional `py.detach` costs a release/acquire pair for no concurrency benefit in that case.
- **`VerificationReport.to_dict()` is a double parse.** `to_string` then `from_str` into `serde_json::Value` then a recursive Python object build, under the GIL. It is also the only way to see `notice`, `signatures`, `bindings`, and `diagnostics` — `NoticeVerification` is not projected to a `#[pyclass]` at all (`src/types/verification.rs:308`) — and the `#[pyclass]` getters expose 22 of the report's fields and stop there. `ExecutionReport`, `ResourceUsage`, and `VerificationReport` are the DTOs that also carry a `__repr__`.
- **The unmanaged fixture copy can drift.** `bindings/python/tests/fixtures/conformance/canonical/` is a byte copy of the Rust fixture directory with no sync step and no hash manifest. Nothing detects divergence; `test_parity.py` and `test_protect.py` both key on filenames only.
- **The version constants are not a real cross-crate probe.** `__stegoeggo_version__` is `env!("CARGO_PKG_VERSION")` of the binding crate, identical to `__version__` by construction. It reports intent, not the version of the library actually linked into the `.so`.
- **An installed `.venv` of this version is present on disk** (`stegoeggo-0.4.2.dist-info` plus a `stegoeggo.pth`, i.e. a `maturin develop` editable install). It agrees with the current source version, but nothing in the test suite asserts that the importable extension matches the checkout — `python-binding.yml` gets this for free by installing a freshly built wheel, while a local `pytest` run against a stale `maturin develop` install would not.

## Discrepancies

Verified against source; listed rather than normalized.

- **`VerificationReport.license_url` duplicates `rights_url`.** Both getters (line 1614 and 1619) return `self.inner.rights().rights_url()`, and `RightsVerification` in `src/verification/report.rs` has no `license_url` accessor at all. `test_verify.py:67` asserts `report.license_url == "https://example.com/license"` and passes, because the notice's `with_license_url` value is stored as the rights URL — so the test is green while the name is misleading. `_native.pyi` declares both.
- **`test_parity.py::test_rust_cli_round_trip` passes a flag the CLI does not have.** It invokes `["stegoeggo", "verify", <file>, "-o", <dir>]`; `VerifyArgs` (`stegoeggo-cli/src/args.rs:391`) accepts only `--key`, `--json`, `-v/--verbose`. Reachable whenever a `stegoeggo` binary is on `PATH`. The test is currently silent in CI, which has no CLI installed.
- **The same test's assertion cannot fail on a parity defect.** `returncode in (0, 3)` accepts the CLI's missing/invalid-evidence code, so it passes if the CLI reports no notice at all. The module docstring's claim that these tests verify "the Python binding can read back output produced by the Rust CLI/library" is not implemented by any test in the file.
- **`_native.pyi` documents a member that does not exist at runtime.** It declares `AuthenticationMode.None_` with the comment "name 'None' is reserved in Python; use None_ as alias". PyO3 registers the variant as `None` (line 492); no `None_` is registered in the `#[pymodule]`, so `stegoeggo.AuthenticationMode.None_` raises `AttributeError`.
- **`AuthenticationMode` and `HiddenMarkerMode` are unreachable from any entry point.** Both are registered and constructible, but no `#[pyfunction]` or `#[pymethods]` signature accepts either one — `ProtectionRequest`'s constructors take only `(notice, policy)`, and authentication is reachable solely through `with_mac_key(bytes)`. `HiddenMarkerMode.tiled()` and `seed_only()` therefore have no effect on any protect call the binding can make. `test_request.py` exercises both classes standalone but never threads one into a request.
- **`ProtectionRequest` is not constructible without a static constructor.** `PyProtectionRequest` has no `#[new]`; only `metadata_only`, `with_hidden_marker`, and `from_preset` exist. Callers cannot build an unset request, which is consistent with `inner: Option<RustRequest>` and the `expect("request initialized")` invariant.
- **`ResourceLimits` is also constructor-less** — `defaults()` and `builder()` only. `ResourceLimitsBuilder` has no `#[new]` either; it is obtained solely from `ResourceLimits.builder()`.
- **`__init__.py` defines no `__all__`.** The public surface is therefore whatever the import statements happen to bind, with no explicit contract and no marker for deliberate-vs-convenient exports. The README's claim that `py.typed` is shipped holds (the file exists, empty), but the `include` list in `pyproject.toml` is unenforced for anything a future maturin rename would drop.
- **Naming collision across the release machinery.** `release/eggpack/{build,qualification}-bindings.toml` have nothing to do with language bindings; they map target triples to the `stegoeggo-cli` binary. The Python binding has no eggpack contract at all. A reader following the file names will reach the wrong component.

## Relationship to other modules

- [overview.md](overview.md) — workspace layout: 4 members plus 3 excluded binding crates.
- [types.md](types.md) — `ProtectionRequest`, `RightsPolicy`, `RightsNotice`, `ProcessingOptions`, `HiddenMarkerMode`; the DTOs wrapped here by `#[pyclass]`.
- [pipeline.md](pipeline.md) — what `process_request_bytes*` executes.
- [verification.md](verification.md) — `VerificationReport`, its `rights`/`hidden_marker`/`authentication`/`trust`/`signatures`/`bindings`/`diagnostics` sub-reports, and the `to_dict()`/`to_json()` JSON shape.
- [resource-limits.md](resource-limits.md) — `ResourceLimits`, `ResourceLimitsBuilder`, and the `ResourceUsage` counters; owner of the six structured limit error variants.
- [error.md](error.md) — the `stegoeggo::Error` variants consumed by `map_error`.
- [cli.md](cli.md) — the CLI surface that `test_parity.py` shells out to, and the `-o` flag it gets wrong.
- [container-walk.md](container-walk.md) — the per-format work accounting whose counts surface as `ResourceUsage` counters.
- [carrier-surface.md](carrier-surface.md) — the generic carrier crate reached transitively; `InsufficientCapacityError.required`/`.available` are its `CapacityReport` units.
- [testing.md](testing.md) — Rust-side test organisation; the Python suite is a separate, separately-gated layer.
