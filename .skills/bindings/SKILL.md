---
name: bindings
description: Use when writing, modifying, or reviewing code under bindings/ (C, Node, Python FFI leaves) or the canonical byte API they wrap. Triggers on "add a binding export", "change the C ABI", "napi", "PyO3", "cdylib", "ABI-V1", "handle", "panic containment", "export manifest", or when a change to src/ could break a binding surface.
---

# FFI Bindings Conventions

`bindings/{c,node,python}/` are three **leaf** crates, each its own nested
`[workspace]`, `publish = false`. They are **excluded from the root Cargo
workspace** (`exclude = ["bindings"]` in root `Cargo.toml`), so
`./scripts/check.sh` — which runs `cargo test --workspace` — never compiles,
tests, or lints them. A binding can be fully broken while `check.sh` is green.

## Non-negotiable invariants

1. **`panic = "unwind"` in both `dev` and `release`.** The root crate builds with
   `panic = "abort"`. Every binding overrides it so a panic crossing the FFI
   boundary is caught instead of aborting the host process. Changing a binding
   profile to `abort` silently removes panic containment — a host-process crash
   bug, not a build error.
2. **`stegoeggo = { version = "=0.4.2", default-features = false }`.** All three
   pin the exact version and disable default features, so the FFI surface is the
   base library only. No root feature (`signatures`, `detached-manifest`,
   `iscc`, `parallel`, `async`, `conformance`, `webp`) is compiled in. If a
   binding needs one, add it to that binding's manifest — and remember the
   path-filtered binding workflows are the only CI that will notice.
3. **`#![forbid(unsafe_code)]` does not apply.** Bindings are inherently
   `unsafe`; the root and carrier crates forbid it, the bindings do not. Do not
   "fix" this by adding the attribute.
4. **Never publish.** No workflow pushes to crates.io, npm, or PyPI.
   `release-c.yml`, `release-node.yml`, and `release-python.yml` are
   manual-dispatch and produce artifacts only.

## CI: independent signals, but they run on PRs

`c-binding.yml`, `node-binding.yml`, and `python-binding.yml` are **outside**
required CI, but they are *not* scheduled-only — each triggers on push **and**
pull_request to `main`, path-filtered on `bindings/<lang>/**`, `src/**`,
`stegoeggo-stego/**`, `Cargo.toml`, `Cargo.lock`, and its own fixtures.

**Consequence for library work:** a change under `src/` that breaks a binding
surface is caught by the binding workflow even though `check.sh` cannot see it.
Before claiming a `src/` change is safe, ask whether it alters any function a
binding calls.

## C binding (`bindings/c/`)

- `cdylib` crate `stegoeggo_c`; cbindgen generates `include/stegoeggo.h`, which
  is **committed** and drift-checked in CI.
- **ABI-V1 is a stability promise.** `bindings/c/ABI-V1.md` is normative; see
  `STABILITY.md`. Changing a signature, removing a symbol, or changing error
  codes is an ABI break requiring a new version, not an edit.
- **90 exported functions.** `bindings/c/abi-v1-symbols.txt` is the checked
  manifest; `scripts/check-exports.sh` compares it against the built cdylib.
  The count is arithmetic-checked — if you add a symbol, update the manifest,
  the header, and the count arithmetic together.
- **Opaque handle model:** callers never see Rust layout. Handle create/destroy
  is explicit; buffer returns are caller-freed with a documented error on
  failure.
- Error mapping lives in `src/error.rs` + `src/codes.rs`; every
  `Error` variant reachable from a binding must map to a stable code.
- Generated-code rule: never hand-edit `include/stegoeggo.h` or
  `abi-v1-symbols.txt`; regenerate via `scripts/generate-header.sh`.

## Node binding (`bindings/node/`)

- napi-rs 3.13 (`napi6`, `dyn-symbols`) + `napi-derive`; `cdylib` named
  `stegoeggo`.
- **`index.d.ts` and `stegoeggo.d.ts` are generated and drift-checked.** After
  changing any `#[napi]` signature, regenerate and commit them; CI fails on
  drift.
- **Threadpool model:** blocking library calls go through `src/tasks.rs` so the
  JS event loop is not blocked.
- **Marshalling is lossy — flatten deliberately.** Rust enums
  (`RightsPolicy`, `ProtectionPreset`, `DmiValue`) and numeric/reporting types
  are converted to JS-safe shapes in `src/enums.rs` / `src/numeric.rs` /
  `src/report.rs`. `VerificationReport` is flattened to a plain object, not
  passed as a class instance.
- `panic = "abort"` in the root release profile is overridden here; see
  invariant 1.

## Python binding (`bindings/python/`)

- PyO3 0.27 with `abi3-py311` + `extension-module`; `cdylib` named
  `_stegoeggo_native`.
- **The GIL must be released** around every library call that does not touch
  Python objects: PyO3 0.27 spells this `py.detach(|| process_request_bytes(...))`
  (not the older `allow_threads`). Holding the GIL across a large image
  operation serializes the whole interpreter. Every `#[pyfunction]` wrapping a
  library call must detach; a missing `py.detach` is a real concurrency
  regression, not a style issue.
- `src/lib.rs` is a thin `#[pyfunction]` surface; richer ergonomics live in the
  **pure-Python** layer at `bindings/python/python/stegoeggo/`.
- **Exceptions are structured.** Every resource-limit and capacity error has a
  typed Python exception with attributes; file helpers are failure-safe
  (`output_path` optional, no partial writes).
- The `extension-module` feature means the module cannot be imported from a
  plain `cargo test` context; build a real wheel (`maturin`) as CI does.
  `maturin develop` does not work under `actions/setup-python` — CI builds and
  installs a wheel instead.

## Cross-binding parity

The three bindings must agree on **semantics**, not on identical APIs:

- Same canonical byte operations, same error taxonomy, same rights vocabulary.
- `bindings/*/tests/` (or `test/`) each contain a parity suite against the
  library. A new canonical operation or a changed error mapping is incomplete
  until all three bindings cover it.
- Abort-safe assumption: every binding is a thin marshalling layer. If a
  binding needs domain logic, that logic belongs in the library.

## Where things live

| Concern | Location |
|---|---|
| Deep dives | `architecture/bindings-{c,node,python}.md` |
| Normative C ABI | `bindings/c/ABI-V1.md` |
| C export manifest / header | `bindings/c/abi-v1-symbols.txt`, `bindings/c/include/stegoeggo.h` |
| C contract scripts | `bindings/c/scripts/` (`check-contract.py`, `check-exports.sh`, `parity.sh`, `run-c-tests.sh`, `generate-header.sh`) |
| CI + release workflows | `.github/workflows/{c,node,python}-binding.yml`, `release-{c,node,python}.yml` |
| Platform/support matrix | `SUPPORT.md` |
| Binding decisions | `plans/adrs/ADR-0005-foreign-language-bindings.md`, `ADR-0006-versioned-c-abi.md` |
| Subsystem history | `plans/subsystems/language-bindings-roadmap.md`, `plans/closure/language-bindings/` |

## Verification

`./scripts/check.sh` covers **none** of this. Binding changes are verified by:

1. `./scripts/check.sh` — still required for shared library changes.
2. The binding's own path-filtered CI workflow (C, Node, or Python).
3. The manual qualification workflow (`release-c|node|python.yml`) for
   cross-platform claims — never required CI, cite only when actually run.

Locally, a binding crate builds with `cargo build --manifest-path
bindings/<lang>/Cargo.toml` from inside that directory (it is its own
workspace). Do not add binding steps to `check.sh` without a maintainer
decision.
