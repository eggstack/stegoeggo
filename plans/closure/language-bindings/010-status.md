# Language Bindings Milestone 010 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/language-bindings/010-c-abi-v1-cross-platform-qualification.md`
Source subsystem roadmap: `plans/subsystems/language-bindings-roadmap.md#m010--c-abi-v1-cross-platform-qualification-and-stability-activation`
Repository baseline reviewed: `49dbb4a` (M009 closure; plan baseline `cfce18b3b7ab24832e7988201c981e49af96a9c7`)
Implementation commits: `95796be` (release-c workflow + artifact smokes), `aebfc9f` (macOS @rpath install-id), `e0407e1` (MSVC `stegoeggo_c.dll.lib` identity), `cb53ef7` (MSVC setup before dumpbin audit), `8563084` (Windows smoke DLL PATH), `a8395a5` (bundle LF normalization), `5f74b95` (Rust 1.89 pin + bundle source version), `c6b8c38` (exact 1.89.0 toolchain name), `0e52feb` (clippy component for 1.89)
Closing commit: stability/support/releasing activation, this record, and roadmap/registry reconciliation (see git log for `plans: close language-bindings M010 cross-platform qualification`).
Final qualification: `release-c` run `36952787634` on immutable SHA `0e52febef43096b6a8610cc5050631fab47feb4a` — 11/11 jobs green.
`https://github.com/eggstack/stegoeggo/actions/runs/36952787634`

## 1. Executive finding

The M009 C ABI v1 implementation is qualified as a portable system
library on all five native distribution architectures, the exact
90-symbol export contract is proven on every target, the Linux glibc
2.17 floor is preserved, artifact bundles are consumable from the
downloaded artifact alone, and the collector audit proves one
consistent five-bundle set from a single source SHA. No acceptance
criterion remains outstanding; M010 closes and the ABI v1 stability
promise in `STABILITY.md` is activated. Nothing was published to any
registry or release. The initial Python → Node → C language-bindings
sequence is complete.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Manual `release-c.yml`, dispatch-only, exact-ref checkout, no publication | `workflow_dispatch` with required `source_ref` only; every build job checks out the exact ref and records `git rev-parse HEAD`; `contents: read`; no publish/upload-release step anywhere (§3) |
| Five-target matrix, exact Rust 1.89 | ubuntu-24.04 / ubuntu-24.04-arm / macos-15-intel / macos-14 / windows-2022; `dtolnay/rust-toolchain` + `RUSTUP_TOOLCHAIN=1.89.0`; BUILD-INFO on all five bundles records `rustc 1.89.0 (29483883e 2025-08-04)` (§4) |
| cargo-zigbuild 0.23.3 for Linux `.2.17` | Exact install mechanics reused from `release-binaries.yml` (isolated `CARGO_INSTALL_ROOT`, verified Zig 0.14.1 SHA per arch); targets `x86_64/aarch64-unknown-linux-gnu.2.17`; BUILD-INFO records `cargo_zigbuild=0.23.3` (§4) |
| cbindgen 0.29.4 drift gate | Installed + `generate-header.sh --check` on the authoritative Linux x86_64 job, green; M008 checker rerun in the same step (§4) |
| Exact cdylib build, exact paths | One manifest build per row; top-level `libstegoeggo_c.so` / `.dylib` / `stegoeggo_c.dll` asserted with `test -f` (Linux rows tolerate only the two exact zigbuild target-dir spellings, never a broad find) (§4) |
| Per-platform 90-symbol audit | Linux `nm -D --defined-only`, macOS `nm -gU` (underscore-normalized), Windows `dumpbin /EXPORTS` (PowerShell name extraction): 90/90/zero-extra on all five; each compared byte-for-byte with `abi-v1-symbols.txt` (§4) |
| Linux glibc ≤ 2.17 proof | `readelf --version-info` semantic gate, not the target string: Linux x86_64 max required version is exactly `GLIBC_2.17`; `readelf -d` recorded (§4) |
| macOS install-name/deps proof | `otool -L/-D` recorded; install-id is `@rpath/libstegoeggo_c.dylib` (set at link time, not patched after audit); gate fails on any `/Users/` worktree path; runtime deps are libSystem + libiconv only (§4) |
| Windows import-lib/deps proof | `stegoeggo_c.dll.lib` asserted beside the DLL and bundled; `dumpbin /DEPENDENTS` recorded; C/C++ smokes link the import library (§4) |
| Native C11 functional smoke ×5 | `examples/smoke.c` (self-contained PNG synthesis, metadata-only protect, buffer inspect, verify, rights assertion, verification JSON, one invalid-argument error, full cleanup) compiled `-Wall -Wextra -Werror` (MSVC `/W3 /WX`) and run green on all five (§4) |
| Native C++17 smoke ×5 | `examples/smoke.cpp` includes the same header and links/calls bootstrap + notice/request/detect: compiled and run green against every built library; include re-proven from every downloaded bundle (§4) |
| Bundle + clean install/load proof ×5 | `stegoeggo-c/{include,lib,share,examples,README.md,LICENSE,BUILD-INFO.txt}` (+ import lib on Windows); text files LF-normalized so contract hashes are identical; clean jobs compile/link/run from the downloaded bundle only with the loader path pointed at the bundle `lib/` (§4) |
| Collector audit | Exactly five bundles; one dynamic library each (+ Windows import lib); all five headers and manifests byte-identical (`4a281fe1b5e2…` / `faa8b671429ce…`); ABI 1.0 and one source SHA (`0e52feb…`) everywhere (§4) |
| Docs/stability activation | `SUPPORT.md` (C ABI status, five targets, glibc floor, dynamic-only, C11/C++17, run/SHA), `RELEASING.md` (manual procedure, no-publication, inspection guidance), `STABILITY.md` (append-only v1 contract, v2 rule, source-vs-distribution distinction, no staticlib) (§9) |
| Root `./scripts/check.sh` green | Exit 0 at the closing tree (§4) |

## 3. Production implementation evidence

M010 implementation (workflow + examples + qualification fixes; no
Rust/header/manifest/implementation change — `git diff
0e52feb...` for the closing commit touches only `.md` planning and
user/maintainer docs):

- `.github/workflows/release-c.yml` (new): `build_c` matrix (5 rows),
  `smoke_artifact` matrix (5 rows, no source checkout for
  library/header contents), `collect_audit` (fails closed on any
  partial set). Concurrency scoped to the source ref without
  cancelling qualification reruns.
- `bindings/c/examples/smoke.c` / `smoke.cpp` (new, self-contained;
  also shipped inside every bundle under `examples/`).
- Three genuine cross-platform defects found and fixed during
  qualification (all in M010 scaffolding, none in the M009
  implementation): cargo's default macOS install-id embeds the build
  worktree (now `@rpath` at link time); the MSVC import library is
  emitted as `stegoeggo_c.dll.lib` (asserted/bundled/linked under its
  real name); Windows checkouts carry CRLF (bundle text files
  LF-normalized so contract hashes match).
- One toolchain-correctness fix with subsystem-wide effect: the
  repo `rust-toolchain.toml` (`channel = "stable"`) silently routed
  every binding workflow through stable despite the installed 1.89.
  `RUSTUP_TOOLCHAIN=1.89.0` is now enforced in `c-binding.yml` and
  `release-c.yml`; the final run builds everything on genuine
  `rustc 1.89.0`. The older Python/Node workflows were left
  untouched as out of scope (§10).

## 4. Verification executed (exact commands + results)

Local pre-dispatch (repository root, clean tree):

```bash
./scripts/check.sh   # exit 0; closes with `./scripts/check-docs-contract.sh`: "Documentation contracts valid: 5 release targets"
cargo test --manifest-path bindings/c/Cargo.toml --locked   # 26 passed (unchanged implementation)
bindings/c/scripts/generate-header.sh --check   # drift-free
python3 bindings/c/scripts/check-contract.py   # 90 total (3 bootstrap + 87 v1)
```

Final remote qualification (`release-c` run `36952787634`, source SHA
`0e52febef43096b6a8610cc5050631fab47feb4a`):

- `build linux-x86_64 library` — success (cbindgen drift gate,
  zigbuild `.2.17`, 90/90 exports, max `GLIBC_2.17`, C11 + C++17
  smokes, bundle `c-abi-linux-x86_64`).
- `build linux-aarch64 library` — success (same gates on ARM64,
  bundle `c-abi-linux-aarch64`).
- `build macos-x86_64 library` — success (`@rpath` id, otool
  evidence, 90/90 exports, smokes, bundle `c-abi-macos-x86_64`).
- `build macos-arm64 library` — success (same gates on ARM64,
  bundle `c-abi-macos-arm64`).
- `build windows-x86_64 library` — success (DLL + `stegoeggo_c.dll.lib`,
  `dumpbin /EXPORTS` 90/90, dependents recorded, `cl` C11 + C++17
  smokes, bundle `c-abi-windows-x86_64`).
- `clean smoke <all five>` — success (each compiles/links/runs
  `smoke.c` from the downloaded bundle only; C++ include re-proven).
- `collect audit` — success (five bundles, one source SHA, identical
  header/manifest hashes, ABI 1.0, Windows import lib present).

Superseded dispatch attempts on earlier SHAs (`36946360923`,
`36946945824`, `36947515153`, `36948190448`, `36948727199`,
`36949363572`, `36950544632`) are not cited as evidence; each failed
only on the scaffolding defects listed in §3, and the final run above
is the single complete green run on the immutable final SHA.

Per-target library evidence (downloaded bundles at the final SHA):

| Target | Library | Size | Header SHA-16 | Manifest SHA-16 |
|---|---|---|---|---|
| linux-x86_64 | `libstegoeggo_c.so` | 2243608 | `4a281fe1b5e2` | `faa8b671429ce` |
| linux-aarch64 | `libstegoeggo_c.so` | — | `4a281fe1b5e2` | `faa8b671429ce` |
| macos-x86_64 | `libstegoeggo_c.dylib` | — | `4a281fe1b5e2` | `faa8b671429ce` |
| macos-arm64 | `libstegoeggo_c.dylib` | — | `4a281fe1b5e2` | `faa8b671429ce` |
| windows-x86_64 | `stegoeggo_c.dll` + `stegoeggo_c.dll.lib` | 2055168 + 30534 | `4a281fe1b5e2` | `faa8b671429ce` |

All five `BUILD-INFO.txt` agree: `source_sha=0e52feb…`,
`source_version=0.4.2`, `abi_major=1`, `abi_minor=0`,
`rustc 1.89.0`, `cbindgen=0.29.4` (plus `cargo_zigbuild=0.23.3` on
Linux). No GitHub Release publication and no registry publication
occurred in any job (workflow holds `contents: read` only).

## 5. Invariant review

- Exact 90-symbol manifest on every target; zero extra `stegoeggo_*`
  exports anywhere; header identical across platforms.
- Fixed numeric values identical (single committed header consumed by
  all rows); opaque handle layout never in the header.
- No Rust panic crosses C on any target (same audited leaf binary
  family; M009 panic-seam evidence stands).
- Native callers allocate/use/free all returned handles (C11 smoke);
  C++ includes/links without name mangling (C++17 smoke).
- Linux artifacts need no glibc newer than 2.17 (proven by symbol
  versions, not by the target string).
- Root required CI unchanged; Python/Node workflows untouched.

## 6. Failure and recovery review

- Any failed/cancelled platform row invalidates qualification: the six
  superseded runs above were discarded for exactly this reason; only
  the complete 11/11 run counts.
- Partial-matrix success was never accepted as evidence at any point.
- No executable/header/manifest change exists after the qualified
  SHA: the closing commit is documentation-only (`STABILITY.md`,
  `SUPPORT.md`, `RELEASING.md`, this record, roadmap, registry) —
  `git diff --stat` against `0e52feb` proves it.
- Retries always reused the same source SHA; the final closure cites
  one complete successful run.

## 7. Migration and compatibility review

M010 is the point at which C ABI v1 becomes an external compatibility
promise, and it activates exactly as designed in ADR-0006: existing v1
symbols/signatures/numeric codes/ownership rules are frozen and
append-only within major 1; compatible additions may extend v1;
breaking changes require `stegoeggo_v2_*` symbols; the source version
may advance independently of ABI major/minor; Python/Node remain
direct bindings. There are no prior stable C consumers to migrate.

## 8. Security review

No new attack surface: the same M009 leaf boundary runs on every
target; MAC keys remain write-only and scrubbed; error/report paths
unchanged. The qualification workflow needs no write permissions and
performs no publication; artifact contents are inspection bundles,
not installers, and never execute with privilege. No `sudo` is invoked
anywhere.

## 9. Documentation and operations

- `STABILITY.md`: C ABI v1 section activated (append-only promise,
  v2 rule, source-vs-distribution distinction, cdylib-only scope).
- `SUPPORT.md`: new C ABI section (status, five qualified targets,
  glibc 2.17 floor, `@rpath`/import-lib notes, dynamic-only,
  C11/C++17 support, run `36952787634` on `0e52feb…`).
- `RELEASING.md`: new C ABI Native Artifacts section (manual
  `release-c.yml` procedure, exact pins, no-publication behavior,
  bundle inspection guidance).
- `bindings/c/README.md`: already describes the pre-stable state;
  updated status wording now points at the qualified ABI (stable per
  `STABILITY.md`).
- ADR-0006 untouched; no durable contradiction discovered.

## 10. Unresolved findings (critical/high/medium/low)

None blocking. Two lows, both owned outside this milestone:

- The `rust-toolchain.toml` (`channel = "stable"`) override affects
  every binding workflow that relies on the dtolnay action default
  (including the older Python/Node workflows, which still claim Rust
  1.89 without `RUSTUP_TOOLCHAIN`). Fixed for the two C workflows
  here; the older workflows are out of scope for this subsystem
  milestone and need a maintainer decision before anyone touches
  their behavior.
- `dtolnay/rust-toolchain@1.89` no longer accepts the `toolchain:`
  input without an "unexpected input" warning (it still installs the
  resolved `1.89.0`, and the pin makes the build hermetic anyway).
  A future action-version bump can remove the noise; it changes no
  behavior.

## 11. Roadmap disposition

Language-bindings M010 is closed with all required evidence accepted.
The initial Python → Node → C language-bindings sequence is complete:
Python and Node packages are independently installable and
cross-language-equivalent to the canonical Rust API, and the versioned
C ABI with explicit ownership/error/panic contracts is implemented and
qualified on all five native targets. No new subsystem-local milestone
is required by this closure.

## 12. Registry updates

- `plans/registry.md`: language-bindings M010 implementation plan
  `ready` → `closed` with this closure record linked; subsystem marked
  complete (all milestones closed); recent-closure entry added.
- `plans/subsystems/language-bindings-roadmap.md`: §7 M010 text closed;
  §11 completion definition satisfied; §12 M010 row `ready` →
  `closed` with this closure record linked.
