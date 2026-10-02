# Language Bindings Milestone 009 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/language-bindings/009-c-abi-v1-implementation-foundation.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#m009--c-abi-v1-implementation-foundation`
Repository baseline reviewed: `593d05b` (M008 closure; plan baseline `9cac8fcda81ecc3e9e491d28e166470a9e8a7d8c`)
Implementation commit: `ee0f99e` — feat: implement C ABI v1 leaf crate with 90 exports, header, tests (language M009 implementation)
Closing commit: this record plus roadmap/registry reconciliation (see git log for `plans: close language-bindings M009 implementation foundation`).

## 1. Executive finding

The corrected 90-symbol C ABI v1 contract is implemented as an
isolated Rust `cdylib` leaf crate and proven locally and on
lightweight Linux x86_64 CI. All 90 manifest functions are exported
with opaque handles, centralized pointer/panic/error handling,
canonical byte-API delegation, a committed cbindgen-generated header,
Rust unit tests, a native C11 integration suite, C++17 inclusion
proof, two-way Rust↔C parity, and a path-filtered `c-binding.yml`
compatibility workflow that is green. No acceptance criterion remains
outstanding; M009 closes without declaring ABI v1 stable or shipped.
M010 becomes dependency-ready.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Isolated leaf crate, exact root dep, `panic = "unwind"` | `bindings/c/Cargo.toml`: `stegoeggo-c 0.4.2`, edition 2021, `rust-version 1.89`, `publish = false`, own `[workspace]` + lockfile, `crate-type = ["cdylib"]`, `stegoeggo = { path = "../..", version = "=0.4.2", default-features = false }`, `serde_json 1.0` (details/verification JSON), `zeroize 1` (MAC scrub); dev+release `panic = "unwind"`; not in the root workspace (§3) |
| Small unsafe-boundary module set | `codes/handles/input/error/panic/notice/request/limits/operations/report/buffer/lib` — raw-pointer mechanics centralized in `input.rs` + `panic.rs`, never duplicated across the 90 exports (§3) |
| Opaque handles, no Rust layout across C | Handle structs are plain (non-`repr(C)`) Rust types named exactly `stegoeggo_v1_*_t`; the generated header contains only forward declarations — verified by the generator's struct-body gate (§3) |
| Root/carrier `forbid(unsafe_code)` untouched | No file outside `bindings/c/`, `.github/workflows/c-binding.yml`, and planning docs changed; 46 `unsafe` blocks and 0 `unsafe fn`, all inside `bindings/c/src` (§5) |
| Pointer validation before conversion | `borrow_bytes`/UTF-8/flag/optional helpers validate NULL/length/range first; `from_raw_parts` only after validation; borrowed caller bytes copied into owned staged state (§4) |
| Full-width seeds, write-only MAC key | `uint64_t` seeds with presence flags (`0` and `u64::MAX` tested); MAC key copied into the staged request, never readable, scrubbed on request drop via `zeroize` (§4) |
| Canonical delegation only | `protect` → `process_request_bytes`, `protect_with_report` → `process_request_bytes_with_report`, `verify` → `verify_image_bytes_report[_with_limits]`; no carrier calls (§4) |
| Exact frozen error mapping + details JSON | Every current `Error` variant mapped per ABI-V1 §8 (`Serialization`/`Iscc`/futures → internal); `schema_version = 1` details JSON with base four plus variant-provided fields only; carrier case message-only (§4) |
| Verification JSON reuses the stable schema | `serde_json::to_vec` over the canonical `VerificationReport`; no second schema; no execution-report JSON (§4) |
| Generated (never handwritten) header | cbindgen 0.29.4 standalone; `cbindgen.toml` (C, C++ guards, include guard, `usize_is_size_t`, sorted); `generate-header.sh` regenerates to temp, gates 90-symbol manifest equality + opaque handles, replaces atomically; `--check` drift mode (§3) |
| Linux export audit | `check-exports.sh` (`nm -D --defined-only` on Linux, `nm -gU` on macOS): dynamic `stegoeggo_*` exports byte-equal the manifest, exactly 90, zero extra (§4) |
| C consumer tests | `tests/test_c_abi.c` (171 checks green): bootstrap, all-handle create/free + NULL-free, detect, metadata-only + report getters, hidden-marker round trip, HMAC correct/wrong/missing, maximal-without-key config error, resource-limit error + details JSON, malformed/truncated input, invalid UTF-8/NUL, every invalid-constant class, NULL outputs, all-18 limit setters, presets, progressive/MAC-key clearing rules, secret non-disclosure sweep (§4) |
| Rust unit tests | 26 green: conversions (known + unknown codes), every `Error` variant mapping + details schema, carrier message-only, pointer/flag/UTF-8 helpers, NULL sentinels, panic seam (payload never leaks), version constants, staged materialization, in-process C↔Rust byte parity (§4) |
| Rust↔C parity | `scripts/parity.sh`: C protects deterministically (seed + timestamp, byte-identical reruns), C verifies its own output, Rust CLI verifies C output, C verifies Rust CLI output — all green (§4) |
| Lightweight `c-binding.yml`, root CI unchanged | Path-filtered push/PR workflow, Ubuntu x86_64 Rust 1.89: check/clippy/test, header drift, release build, export audit, C11/C++17 suite, parity, artifact upload; `./scripts/check.sh` untouched (§4) |
| No stable/shipped ABI claim | `STABILITY.md` still says planned/not stable/not shipped; `bindings/c/README.md` says implemented, pre-stable (§9) |

## 3. Production implementation evidence

Commit `ee0f99e` (27 files, +5206/−21; no root/carrier/CLI source touched):

- `bindings/c/src/` (12 modules): 90 `#[no_mangle] pub extern "C"` exports
  over one `invoke()` fallible wrapper (validate out-pointers,
  NULL-initialize outputs, validate before conversion, `catch_unwind`,
  frozen error mapping, fixed-message panic mapping, never unwinds).
  Notice/request handles stage C-set values in Rust-native options and
  materialize canonical `RightsNotice`/`ProtectionRequest` at operation
  time, which is what makes presence-flag clearing, tile-size pairing,
  and write-only MAC semantics exact without mutating private
  canonical fields.
- `bindings/c/cbindgen.toml`, `bindings/c/include/stegoeggo.h`
  (committed generated header: 90 declarations, 13 scalar typedefs, all
  frozen constants, 7 opaque forward declarations, C++ guards),
  `bindings/c/scripts/` (header generation/drift, export audit,
  C-test runner, parity), `bindings/c/tests/` (C suite, parity tools,
  C++ smoke, dependency-free PNG synthesizer), `bindings/c/examples/`
  (M010 artifact smokes, added in `95796be`).
- `.github/workflows/c-binding.yml` (new lightweight workflow).

M008 checker reuse: `generate-header.sh` asserts the generated
header's 90 exports equal `abi-v1-symbols.txt`, and
`check-contract.py` still gates document↔manifest equality.

## 4. Verification executed (exact commands + results)

C crate (repository root):

```bash
cargo check --manifest-path bindings/c/Cargo.toml --locked   # clean
cargo clippy --manifest-path bindings/c/Cargo.toml --locked --all-targets -- -D warnings   # clean
cargo test --manifest-path bindings/c/Cargo.toml --locked   # 26 passed, 0 failed
cargo build --manifest-path bindings/c/Cargo.toml --locked --release   # clean (arm64 + x86_64-apple-darwin)
```

Header and exports:

```bash
cargo install cbindgen --version 0.29.4 --locked
bindings/c/scripts/generate-header.sh --check   # drift-free + contract OK (90 total, 3 bootstrap + 87 v1)
python3 bindings/c/scripts/check-contract.py   # ABI v1 contract OK
bindings/c/scripts/check-exports.sh   # exact 90-symbol manifest on arm64 and x86_64 macOS dylibs
```

Native consumers and parity:

```bash
bindings/c/scripts/run-c-tests.sh   # cc C11 + c++ C++17 compile clean; c-abi checks=171 failures=0; C tests OK
bindings/c/scripts/parity.sh   # C deterministic reruns byte-identical; C<->CLI both directions Verified; parity OK
```

Root gate (repository root, clean tree):

```bash
./scripts/check.sh   # exit 0; closes with `./scripts/check-docs-contract.sh`: "Documentation contracts valid: 5 release targets"
```

Remote: `c-binding` run `36945460318` on `ee0f99e` — **success**
(`c-binding-check`: Rust gates + header drift + release build +
90-symbol audit + C11/C++17 suite + parity).
`https://github.com/eggstack/stegoeggo/actions/runs/36945460318`
`c-binding` run `36946341300` on `95796be` (M009 content unchanged;
only the M010 workflow/examples added) — **success**.
Required `CI` Check runs `36945460405` (on `ee0f99e`) and `36946341303`
(on `95796be`) — **success**. (The one earlier `CI` failure on the
pre-M009 base `839c4c8`, `update::tests::candidate_wrong_version_is_fatal`,
did not recur on either M009 tree and is unrelated to this subsystem.)

## 5. Invariant review

- Exactly the 90 M008-manifest symbols exported; zero extra
  `stegoeggo_*` on both locally built architectures and Linux CI.
- No Rust-native layout crosses C; cbindgen emits forward declarations
  only (generator fails closed on any struct body).
- `unsafe` confined to the leaf (46 blocks: handle lifecycle
  `into_raw`/`from_raw`, validated slice construction, NULL-checked
  reads); zero `unsafe fn`; `deny(unsafe_op_in_unsafe_fn)` at crate
  level; no `extern "C-unwind"`.
- C inputs borrowed per call; Rust outputs freed only via matching ABI
  frees; `free(NULL)` is a no-op everywhere.
- Unknown input codes → invalid argument/config, never transmuted;
  unknown future Rust errors → internal with base-four details only.
- No panic payload text, no MAC key bytes, in any message or JSON
  (swept by unit + C tests).
- Same-handle concurrency remains unsupported per contract; no global
  or TLS error state.
- Python/Node untouched and still direct Rust bindings.

## 6. Failure and recovery review

- Every fallible export NULL-initializes owned outputs before work; a
  failed call never transfers a partial handle (covered by the
  maximal-without-key and NULL-output C cases).
- Panic containment proven by the internal `cfg(test)` seam (no extra
  exported symbol): deliberate panic → code 10, fixed message, error
  handle well-formed.
- The first `release-c` dispatch (M010 work, run `36946360923`)
  exposed two qualification defects in CI-authored scaffolding, not in
  the implementation: cargo's default macOS install-id embeds the
  build worktree path (fixed with `@rpath` link args), and the MSVC
  import-library path assumption needs verification (owned by M010).
  No M009 implementation change resulted.

## 7. Migration and compatibility review

No C consumers exist yet; M009 implements but does not activate
stability. The header and library are pre-qualified until M010
closes. Python/Node/Rust/CLI contracts are unchanged (no shared file
touched).

## 8. Security review

MAC key handling: write-only setter, staged copy scrubbed on request
drop, never in messages/reports/getters/JSON — asserted by the C
secret sweep over error message + details JSON. Bounded parsing via
centralized length/UTF-8/range validation before any unsafe
conversion. No registry or release publication in any workflow touched
here (`c-binding.yml` uploads CIevidence artifacts only).

## 9. Documentation and operations

- `bindings/c/README.md`: implementation/build/test section with the
  exact command set; still marks ABI v1 pre-stable.
- `bindings/c/include/stegoeggo.h`: committed generated header.
- `STABILITY.md`: unchanged discharge — still planned/not stable/not
  shipped (activation is M010's job after five-target qualification).
- New `plans/closure/language-bindings/009-status.md` (this record).

## 10. Unresolved findings (critical/high/medium/low)

None blocking. Two lows recorded for M010 hygiene:

- The `notice_set_usage_terms_lang` materialization routes through
  `LegalMetadata::with_usage_terms_localized`, so setting a language
  tag while `usage_terms` is absent materializes terms as an empty
  string (covered by `notice_lang_without_terms_materializes_lang`;
  the mainstream terms+lang path is exact). The ABI contract does not
  pin this corner down.
- Error-details JSON for `container`/`verification_budget` carries
  exactly `kind` + `count` per ABI-V1 §11.1 (the available `limit` is
  omitted to match the normative key set).

## 11. Roadmap disposition

Language-bindings M009 is closed with all required evidence accepted.
The M010 cross-platform qualification hard dependency (closed M009
implementation foundation) is now satisfied: M010 becomes
dependency-ready and its already-dispatched qualification run may be
used as evidence once green on the final SHA. No new subsystem-local
milestone is required by this closure.

## 12. Registry updates

- `plans/registry.md`: language-bindings M009 implementation plan
  `ready` → `closed` with this closure record linked; subsystem
  current milestone notes M009 closed and M010 dependency-ready; M010
  blocker row removed; recent-closure entry added.
- `plans/subsystems/language-bindings-roadmap.md`: §7 M009 text closed;
  §12 M009 row `ready` → `closed` with this closure record linked;
  M010 row blocker cleared to "M009 implementation accepted".
- `plans/implementation/language-bindings/010-c-abi-v1-cross-platform-qualification.md`:
  `blocked` → `ready for handoff` (its hard M008/M009 prerequisites
  are both closed).
