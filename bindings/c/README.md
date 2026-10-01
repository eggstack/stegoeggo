# C language binding — design only, nothing shipped yet

This directory will host the versioned C ABI leaf binding over the
canonical `stegoeggo` Rust library.

Status: **design contract accepted, no implementation**. The normative
contract is `ABI-V1.md` (ABI major `1`, minor `0`; 89 planned symbols),
materialized from accepted `plans/adrs/ADR-0006-versioned-c-abi.md` by
language-bindings M007. M007 ships no Crate, no header, no shared library,
and no stable symbols.

What happens next (language-bindings M008, blocked until M007 closure):

- a leaf `bindings/c` Rust crate outside the root Cargo workspace,
  depending directly on `stegoeggo` with `panic = "unwind"`;
- all required `unsafe` confined to that leaf (root and carrier crates
  keep `#![forbid(unsafe_code)]`);
- a committed cbindgen-generated `include/stegoeggo.h` (cbindgen `0.29.4`
  standalone tool, never a `build.rs` or root-build prerequisite);
- a checked `abi-v1-symbols.txt` manifest and the five-target
  load/link/run qualification in `ABI-V1.md` §16.

Design rules that already bind M008:

- opaque handles with explicit Rust frees; C never calls `free()` on
  Rust memory; freeing NULL is a no-op;
- fixed-width scalar codes with frozen numeric values (never Rust enum
  discriminants or layout);
- per-call explicit error handles (no last-error thread-local state);
- versioned `stegoeggo_v1_*` operational symbols plus three unversioned
  bootstrap symbols; ABI v1 is append-only after M008 closes;
- full-width `uint64_t` seeds; write-only MAC keys, zeroized on request
  destruction, never in errors/reports/getters/JSON;
- `NotFound` vs `Invalid` verification evidence projected verbatim;
- panics contained by `catch_unwind` at every `extern "C"` export;
- Python and Node stay direct Rust bindings and are not moved onto C.

Do not treat anything under `bindings/c/` as a stable or usable C API
until M008 closes and `STABILITY.md` says so.
