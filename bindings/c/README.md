# C language binding — implemented, pre-stable qualification

This directory hosts the versioned C ABI leaf binding over the
canonical `stegoeggo` Rust library.

Status: **implemented on Linux; ABI v1 is not yet stable or shipped**.
The normative contract is `ABI-V1.md` (ABI major `1`, minor `0`;
90 symbols: 3 bootstrap + 87 `stegoeggo_v1_*`), materialized from
accepted `plans/adrs/ADR-0006-versioned-c-abi.md` by language-bindings
M007 and corrected by M008. M009 implements all 90 symbols as an
isolated Rust `cdylib` with a committed cbindgen-generated header.
ABI v1 stability promises begin only when M010 cross-platform
qualification closes.

## Layout

- `Cargo.toml` — isolated `stegoeggo-c` crate (own workspace, own
  lockfile; never added to the root workspace). Exact path/version
  dependency on `stegoeggo` with `panic = "unwind"`. All `unsafe`
  required by the boundary lives here; root and carrier crates keep
  `#![forbid(unsafe_code)]`.
- `src/` — `codes` (frozen scalar tables + conversions), `handles`
  (opaque staged builders), `input` (pointer/length/UTF-8 validation),
  `error` (ABI error DTO + details JSON), `panic` (common
  `catch_unwind` wrapper), `notice`/`request`/`limits` (builders),
  `operations` (detect/protect/verify), `report` (execution +
  verification getters + JSON), `buffer` (owned bytes).
- `cbindgen.toml` — pinned cbindgen `0.29.4` generation config.
- `include/stegoeggo.h` — committed generated header (drift is failure).
- `abi-v1-symbols.txt` — checked 90-line export manifest (M008 oracle).
- `scripts/` — `check-contract.py` (document↔manifest gate),
  `generate-header.sh` (regenerate or `--check` the header),
  `check-exports.sh` (dynamic export audit),
  `run-c-tests.sh` (build + C11/C++17 consumer suite),
  `parity.sh` (Rust↔C two-way parity via the CLI).
- `tests/` — `test_c_abi.c` (lifecycle/error/report suite over a
  dependency-free synthesized PNG), `c_parity_protect.c` /
  `c_parity_verify.c` (parity tools), `cxx_smoke.cpp` (C++17 include
  smoke), `minipng.h` (tiny PNG synthesizer, no third-party code).

## Build and test

```bash
cargo check --manifest-path bindings/c/Cargo.toml --locked
cargo clippy --manifest-path bindings/c/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path bindings/c/Cargo.toml --locked
cargo build --manifest-path bindings/c/Cargo.toml --locked --release
cargo install cbindgen --version 0.29.4 --locked
bindings/c/scripts/generate-header.sh --check
python3 bindings/c/scripts/check-contract.py
bindings/c/scripts/check-exports.sh bindings/c/target/release/libstegoeggo_c.so
bindings/c/scripts/run-c-tests.sh
bindings/c/scripts/parity.sh
```

Linux exports are audited with `nm -D --defined-only`; macOS uses
`nm -gU`; Windows uses `dumpbin /EXPORTS`. Every target must export
exactly the 90 manifest symbols and no other `stegoeggo_*` symbol.

## Design rules that bind consumers

- Opaque handles with explicit Rust frees; C never calls `free()` on
  Rust memory; freeing NULL is a no-op;
- fixed-width scalar codes with frozen numeric values (never Rust enum
  discriminants or layout);
- per-call explicit error handles (no last-error thread-local state);
- versioned `stegoeggo_v1_*` operational symbols plus three unversioned
  bootstrap symbols; ABI v1 is append-only after M010 closes;
- full-width `uint64_t` seeds; write-only MAC keys, zeroized on request
  destruction, never in errors/reports/getters/JSON;
- `NotFound` vs `Invalid` verification evidence projected verbatim;
- panics contained by `catch_unwind` at every `extern "C"` export;
- Python and Node stay direct Rust bindings and are not moved onto C.

Do not treat anything under `bindings/c/` as a stable API until M010
closes and `STABILITY.md` says so.
