# Language Bindings Milestone 007 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/language-bindings/007-c-abi-contract-design.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#m007--c-abi-contract-design`
Repository baseline reviewed: `89b2198f1d8b7072ec6720a16130bb440a718604` (work-start HEAD; plan baseline `a5b3dff84a3b83eecbca0ac3d44fe5ebf4951673`)
Design commit: `3e3e4637f131cfe667d9f110d999742358f16277` — docs: add C ABI v1 normative contract and binding README (M007 WP1-WP6)
Closing commit: this record plus roadmap/registry/`STABILITY.md` reconciliation (see git log for `plans: close language-bindings M007 C ABI contract design`).

## 1. Executive finding

ADR-0006 is materialized into an exact, implementation-ready C ABI v1
contract with no remaining TBDs. `bindings/c/ABI-V1.md` freezes the full
v1 symbol inventory (**89 symbols**: 3 unversioned bootstrap + 86
`stegoeggo_v1_*`), every ABI-visible numeric value, opaque-handle
lifecycles, pointer/null/length rules, request/notice/limits coverage,
synchronous operation semantics, the error/report/JSON contract, panic
containment, thread rules, the cbindgen/header-generation contract, the
planned symbol manifest, and the five-target M008 qualification matrix.
No stable C symbol, header, library artifact, or Rust FFI source is
shipped; ABI v1 promises begin only at M008 closure. No acceptance
criterion remains outstanding; M007 closes and M008 becomes
dependency-ready (its implementation plan is not yet written).

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Exact complete symbol inventory, ADR-0006 satisfied | `ABI-V1.md` §§1–2, 6–12: 89 symbols enumerated with C signatures; no Rust layout anywhere (§3) |
| Every ABI-visible constant has an explicit fixed value | `ABI-V1.md` §4: 12 code tables with frozen values + reserved ranges, not derived from Rust discriminants (§4) |
| Ownership/free/null/lifetime rules for every handle/pointer | `ABI-V1.md` §§2, 5–12: one free per handle, NULL-free no-op, NULL/length/UTF-8/overflow rules, borrow-vs-copy stated per function (§5) |
| Request/notice/limits coverage matches common foreign semantics | §7 of this record: every Python∩Node-stable option representable; full-width seeds; no secret getter; all 18 `ResourceLimits` fields; no legacy adapters (§6) |
| Error model exact, covers all current Rust variants | §8 of this record: 20 always-available variants + async-only `Task` mapped; carrier → resource `carrier` with message only; unknown futures → internal (§8) |
| Report accessor/JSON contract exact | `ABI-V1.md` §12: execution getters fixed + JSON explicitly OUT of v1; verification getters + stable-schema JSON reuse decided and documented (§9) |
| Panic boundary exact | `ABI-V1.md` §13: 7-step wrapper contract for every future operational export; accessors/destructors must not unwind; OOM/abort out of scope (§9) |
| Thread/reentrancy exact, no over-promised concurrency | `ABI-V1.md` §14: no global state; same-handle concurrent read-only use explicitly UNSUPPORTED unless M008 qualifies it (§9) |
| cbindgen/header-generation exact | `ABI-V1.md` §15: pin `0.29.4`, standalone-tool (never `build.rs`/root prerequisite), committed header, drift-is-failure, C11 + C++17 (§10) |
| Dynamic-library/export-symbol contract exact | `ABI-V1.md` §16: 89-symbol manifest rule + platform inspection tooling; staticlib deferred (§10) |
| M008 five-target matrix exact | `ABI-V1.md` §16 table: the five native distribution architectures (§10) |
| No stable C symbols/artifacts shipped | §11 of this record: only two markdown files added; no crate/header/library (§11) |
| `./scripts/check.sh` green | §4 of this record (§4) |
| Design-review fixtures (5 lifecycles) | `ABI-V1.md` §17: metadata-only, HMAC, verify+JSON, resource-limit errors, mid-failure cleanup (§6) |

## 3. Production/design evidence

Design commit `3e3e463` (documentation only; no Rust source touched):

- `bindings/c/ABI-V1.md` (+~870 lines): normative v1 contract, §§0–18,
  "NOT YET SHIPPED" banner, frozen numbers/signatures/ownership rules.
- `bindings/c/README.md`: pre-implementation status marker ("design only;
  no shipped library yet") with the M008 work list and binding rules.

No `bindings/c/Cargo.toml`, no generated header, no `cdylib`/`staticlib`
artifact, no Rust `extern "C"` source. `git log
a5b3dff..89b2198 -- src/ Cargo.toml Cargo.lock STABILITY.md` is empty, so
no core change during M007 required re-auditing the Python/Node contract.

## 4. Verification executed (exact commands + results)

Repository gate (repository root, clean tree plus the M007 documents):

```bash
./scripts/check.sh   # exit 0
```

Planning/design consistency:

```bash
grep -R "M007 C ABI" plans/subsystems/language-bindings-roadmap.md plans/registry.md
grep -R "ADR-0006" plans/subsystems/language-bindings-roadmap.md plans/implementation/language-bindings/007-c-abi-contract-design.md
```

Optional C11/C++17 syntax smoke (review-only sketch in `/tmp`, never
committed): all 89 contract signatures transcribed to a sketch header plus
a lifecycle consumer exercising bootstrap/notice/request/operations/
buffer/error/report symbols:

```bash
cc -std=c11 -Wall -Wextra -Werror -fsyntax-only m007_consumer.c    # OK
c++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only -x c++ m007_consumer.c  # OK
# Apple clang 21.0.0 (clang-2100.1.1.101), both compilers
```

No cbindgen drift command applies: M008 has not yet produced FFI source or
a generated header.

## 5. Final handle inventory

| Handle | Creator | Borrowers | Destructor |
|---|---|---|---|
| `stegoeggo_v1_notice_t` | `notice_create` | request constructors (cloned) | `notice_free` |
| `stegoeggo_v1_request_t` | `request_metadata_only`, `request_with_hidden_marker`, `request_from_preset` | `protect`, `protect_with_report` (call-only borrow) | `request_free` (zeroizes MAC key) |
| `stegoeggo_v1_resource_limits_t` | `resource_limits_create` (= canonical defaults) | `request_set_resource_limits` (snapshot copy), `verify` (call-only borrow) | `resource_limits_free` |
| `stegoeggo_v1_buffer_t` | `protect`, `protect_with_report`, `error_details_json`, `verification_report_json` | `buffer_data`, `buffer_len` (valid until free) | `buffer_free` |
| `stegoeggo_v1_execution_report_t` | `protect_with_report` | scalar/warning getters | `execution_report_free` |
| `stegoeggo_v1_verification_report_t` | `verify` | scalar getters, `verification_report_json` | `verification_report_free` |
| `stegoeggo_v1_error_t` | any failing fallible call (when `out_error != NULL`) | `error_code`, `error_resource`, message/data/len, `error_details_json` | `error_free` |

Freeing NULL is a no-op for all seven. No memory is C-freeable; no
cross-allocator ambiguity remains.

## 6. Bootstrap + v1 symbol count and coverage

Bootstrap (3, unversioned, retained by future majors):
`stegoeggo_abi_version_major`, `stegoeggo_abi_version_minor`,
`stegoeggo_source_version`.

Operational v1 (86): notice 20 (create/free + 18 setters), request 18
(3 constructors/free + 14 setters), limits 20 (create/free + 18 setters),
operations 4 (`detect_format`, `protect`, `protect_with_report`,
`verify`), buffer 3, error 6, execution report 9, verification report 6.
Total normative inventory: **89 symbols** (`ABI-V1.md` §§1, 6–12).

Request/limits coverage against the Python∩Node M006 common contract:

- Constructors: metadata-only / with-hidden-marker / from-preset — all
  three covered, notice cloned, policy/preset by frozen code.
- Request setters cover every common stable option: seed (full-width
  `uint64_t` + presence flag; `0` valid), intensity, output format
  (presence flag; `None` = same as input), JPEG quality, progressive flag,
  max dimension, metadata update policy, stego redundancy (1..=10),
  4-byte content hash (exact length enforced), timestamp override, MAC key
  (write-only, no getter, zeroized on destroy), resource limits (snapshot
  copy), hidden-marker mode + tile size (strict pairing: nonzero size only
  with `TILED` 32..=1024), authentication mode.
- Notice setters cover all 16 canonical text fields — a strict superset of
  the common contract (neither Python nor Node M006 exposes
  `usage_terms_lang`; C v1 does) — plus DMI and optional seed.
- Resource-limits setters cover all 18 current `ResourceLimits` fields —
  a strict superset of the Python M006 builder (12 setters); Node M006
  covers all 18.
- No `ProtectionLevel`, `ProtectionContext`, or `EvidenceProfile` appears.
- `protect_with_warnings` is intentionally not a separate operation:
  execution-report warning count/value access covers it.

## 7. Numeric code tables (frozen)

Status/error codes share one space: `0` OK; `1` invalid argument;
`2` invalid configuration; `3` invalid format; `4` encode/decode;
`5` metadata; `6` steganography; `7` insufficient capacity;
`8` verification; `9` resource limit; `10` internal/panic; 11..99
reserved. Resources: `0` none; `1` input_bytes; `2` dimensions;
`3` container; `4` metadata; `5` verification_budget; `6` carrier; 7..99
reserved. Policy 0..6 (7..99 reserved); DMI 0..6; format 0..3
(`0` = `UNKNOWN` detect result); update policy 0..2; preset 0..3; auth
0..1; marker mode 0..3; verification status 0..2 (data codes, verbatim
`Verified`/`Invalid`/`NotFound`); evidence strength 0..3; warnings 0..7
with `UINT32_MAX` as the out-of-range sentinel. Full tables: `ABI-V1.md`
§4.

## 8. Error mapping matrix (every current Rust `Error` variant)

| Rust `Error` | C code | C resource | Details JSON beyond base four |
|---|---|---|---|
| `Config` | 2 invalid configuration | 0 none | — |
| `InvalidFormat` | 3 | 0 | — |
| `ImageDecode`, `ImageEncode`, `Image`, `ImageTruncated`, `Io` (in image work) | 4 encode/decode | 0 | — |
| `Metadata` | 5 | 0 | — |
| `Steganography` | 6 | 0 | — |
| `InsufficientCapacity { required, available }` | 7 | 0 | `required`, `available` |
| `PayloadVerification`, `Crypto` | 8 verification | 0 | — |
| `InputTooLarge { size, limit }` | 9 | 1 input_bytes | `size`, `limit` |
| `DimensionsExceeded { width, height, max_width, max_height }` | 9 | 2 dimensions | `width`, `height`, `max_width`, `max_height` |
| `ContainerLimitExceeded { kind, count, limit }` | 9 | 3 container | `kind`, `count`, `limit` |
| `MetadataLimitExceeded { kind, size, limit }` | 9 | 4 metadata | `kind`, `size`, `limit` |
| `VerificationBudgetExceeded { kind, count, limit }` | 9 | 5 verification_budget | `kind`, `count`, `limit` |
| `ResourceLimitExceeded(String)` | 9 | 6 carrier | message only, nothing invented |
| `Serialization`, `Iscc` (unreachable: no `iscc`/report-JSON-instantiation path in the leaf), async-only `Task` (unreachable in sync v1), any future non-exhaustive variant, any caught panic | 10 internal | 0 | base four only |

ABI-misuse detectable before canonical work (NULL/length/UTF-8/range/
unknown-constant/tile-pairing/content-hash-length/empty-key violations)
returns code `1` without reaching Rust. Strategy: stable
category/resource/message accessors plus versioned (`schema_version = 1`,
additive-only) details-JSON buffer; no per-field nullable getter surface.

## 9. Report, panic, and threading contracts

- Execution report: direct getters for effective policy, DMI + presence,
  metadata-injected, stego-attempted, stego-succeeded,
  format-transcoded, warning count, warning-at-index. Complete
  execution-report JSON is explicitly NOT in v1 (all operational facts
  have getters; `resource_usage`/`embed_summary` depth stays behind Rust
  until an additive v1 getter is justified).
- Verification report: direct getters for marker status (verbatim),
  evidence strength, rights-found, authenticated (attempted + HMAC-verified
  + key-matched), plus complete JSON reusing the already-stable
  `VerificationReport` schema (`STABILITY.md`). JSON travels as Rust-owned
  buffer handles.
- Panic: one common wrapper per fallible export — validate out-pointers,
  NULL-initialize outputs, validate null/length/range before unsafe
  conversion, `catch_unwind(AssertUnwindSafe(..))`, map `Error` → frozen
  code/resource/details, map panic → internal with a fixed message (no
  payload text), never unwind across `extern "C"`. Accessors/destructors
  must not unwind; OOM/abort outside the guarantee.
- Threads: no global mutable state; independent handles/operations may run
  concurrently; same-handle concurrent mutation/use/free is a violation;
  same-handle concurrent read-only use is explicitly UNSUPPORTED unless
  M008 qualifies it; v1 is synchronous with no callbacks.

## 10. cbindgen, header, symbol-manifest, and M008 matrix decisions

- cbindgen pinned `0.29.4`; standalone dev/qualification tool, never
  `build.rs`, never a root-build prerequisite. Newer versions need a
  contract review note first.
- Generated `bindings/c/include/stegoeggo.h` committed; drift is failure.
  Normative generation: C with C++ guards, include guard, `usize_is_size_t`,
  opaque handles, only the 89 symbols + scalar typedefs/constants,
  C11-clean and C++17-clean on every matrix target.
- Planned manifest: the 89 symbols become a checked sorted
  `bindings/c/abi-v1-symbols.txt` in M008; missing expected or unexpected
  `stegoeggo_*` exports fail qualification (`nm -D`, `nm -gU`,
  `dumpbin /EXPORTS`). `cdylib` first; staticlib deferred.
- M008 matrix: Linux x86_64 GNU, Linux aarch64 GNU, macOS x86_64, macOS
  arm64 (C11 load/link/run), Windows x86_64 MSVC (C compile/link/run);
  C++ header inclusion everywhere, full C++17 compile on Linux x86_64.

## 11. What was NOT shipped

No Rust `extern "C"` source, no `bindings/c/Cargo.toml`, no generated
`stegoeggo.h`, no `cdylib`/`staticlib` artifact, no registry publication,
no Python/Node refactor, no new required-CI prerequisite. The M007 diff is
exactly two markdown files (`ABI-V1.md`, `README.md`) plus this closure
record and roadmap/registry/`STABILITY.md` reconciliation. Normal root
Rust builds gain no C compiler/cbindgen prerequisite.

## 12. Roadmap disposition

Language-bindings M007 is closed with the accepted exact ABI v1 contract.
M008 (C ABI implementation and qualification) becomes dependency-ready:
its blocking dependency on the M007 contract is satisfied. Its
implementation plan is not yet written — writing it is the next planning
step, not part of this closure. No new subsystem-local milestone is
required by this closure. Python and Node remain direct Rust bindings.

## 13. Registry updates

- `plans/registry.md`: M007 implementation plan `ready` → `closed` with
  this closure record linked; subsystem current milestone notes M007
  closed and M008 proposed; M008 blocker row removed (dependency
  satisfied); recent-closure entry added.
- `plans/subsystems/language-bindings-roadmap.md`: §7 M007 text closed;
  §12 M007 row `ready` → `closed` with this closure record linked; M008
  row blocker cleared to "M007 contract accepted (plan not yet written)".
- `STABILITY.md`: added a "C ABI v1 (planned — not stable, not shipped)"
  section pointing at `bindings/c/ABI-V1.md` solely to prevent premature
  stability claims; no C surface marked stable or shipped.
