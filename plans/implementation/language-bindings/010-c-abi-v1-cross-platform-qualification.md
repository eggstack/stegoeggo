# Language Bindings Milestone 010 — C ABI v1 Cross-Platform Qualification and Stability Activation

Status: ready for handoff

Repository baseline: `cfce18b3b7ab24832e7988201c981e49af96a9c7`

Source roadmap:
`plans/subsystems/language-bindings-roadmap.md#m010--c-abi-v1-cross-platform-qualification-and-stability-activation`

Hard dependencies:

- language-bindings M008 symbol-inventory corrective closed;
- language-bindings M009 C ABI implementation foundation closed.

Long-term requirements:

- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/000-long-term-specification.md#5-release-invariants`
- `plans/002-long-term-roadmap.md#phase-5--language-bindings-active`

Applicable ADRs:

- `plans/adrs/ADR-0005-foreign-language-bindings.md`
- `plans/adrs/ADR-0006-versioned-c-abi.md`

Normative ABI:

- `bindings/c/ABI-V1.md`
- `bindings/c/abi-v1-symbols.txt`
- generated `bindings/c/include/stegoeggo.h`

Primary class: infrastructure

## 1. Objective

Qualify the M009 C ABI implementation as a portable system library on the same
five native architectures already used by StegoEggo's binary/Python/Node
distribution work, prove the exact 90-symbol export contract on each platform,
package self-contained artifact bundles for inspection, and only then activate
the documented ABI v1 stability promise.

M010 owns:

- manual artifact-only `.github/workflows/release-c.yml`;
- Linux glibc-floor qualification;
- native C11 compile/link/run on all five targets;
- C++17 header inclusion on all five targets;
- per-platform export-manifest checks;
- clean artifact-only consumer smokes;
- package layout/content audit;
- SUPPORT/RELEASING/STABILITY updates;
- final C ABI closure evidence.

It does not publish to GitHub Releases or a package registry automatically.

## 2. Why this milestone is fully planned but blocked

The target environment is already established elsewhere in the repository.

Current GitHub-hosted runner availability supports:

- `ubuntu-24.04` x86_64;
- `ubuntu-24.04-arm` aarch64;
- `macos-15-intel` x86_64;
- `macos-14`/newer arm64;
- `windows-2022` x86_64.

The existing Python/Node release workflows already use the same architecture
matrix, so C should reuse those runner choices rather than introduce another
support topology.

For Linux, the repository's prebuilt CLI contract already targets glibc 2.17
using `cargo-zigbuild`. M010 should align with that compatibility floor.
The existing release workflow pins cargo-zigbuild 0.23.3; current upstream is
0.23.4, but this milestone should reuse the repo's established 0.23.3 release
toolchain unless a separate release-distribution update changes it first.

The only blockers are implementation prerequisites M008/M009.

## 3. Current implementation evidence expected from M009

M009 closure must provide:

- isolated `bindings/c` Rust 1.89 crate;
- all 90 exports;
- committed cbindgen-generated header;
- 90-line export manifest;
- Linux x86_64 C11/C++17 checks;
- Rust unit/panic/error tests;
- C integration tests;
- Rust↔C parity;
- lightweight `c-binding.yml` green.

M010 consumes that implementation unchanged except for qualification workflow
and defects found by cross-platform execution.

Any semantic/ABI correction discovered during qualification gets a new
corrective milestone; do not silently alter ABI v1 after qualification begins.

## 4. Invariants that must not regress

- Exact 90-symbol ABI manifest on every target.
- No extra `stegoeggo_*` exports.
- Header identical across platforms.
- Fixed numeric values identical across platforms.
- Opaque handle layout never appears in header.
- `sizeof`/Rust layout is not part of qualification.
- Rust panic does not cross C.
- Native C callers can allocate/use/free all returned handles.
- C++ can include/link the C header without name mangling.
- Linux artifacts require no glibc newer than 2.17.
- Windows package includes the import library required for MSVC linking.
- macOS libraries use the expected install-name/load behavior for artifact
  smoke; no developer-local absolute path is embedded.
- No registry/GitHub Release publication occurs automatically.
- Root required CI remains unchanged.
- Python/Node artifact workflows are unaffected.

## 5. Scope

### In scope

- `.github/workflows/release-c.yml`, manual dispatch only.
- Five native build rows.
- Exact Rust 1.89 toolchain.
- cargo-zigbuild 0.23.3 for Linux GNU `.2.17` builds, matching current CLI
  release tooling.
- cbindgen 0.29.4 drift verification.
- Native export inspection:
  - Linux `nm -D --defined-only`;
  - macOS `nm -gU`;
  - Windows `dumpbin /EXPORTS`.
- Native C11 consumer compile/link/run.
- Native C++17 header include/link smoke.
- Artifact bundle assembly/audit.
- Clean artifact-only smoke jobs.
- Linux version-needs/glibc-floor audit.
- platform dependency inspection.
- Actions artifact upload.
- SUPPORT/RELEASING/STABILITY and closure reconciliation.

### Explicitly out of scope

- Automatic GitHub Release attachment.
- Homebrew/vcpkg/Conan/pkg-config publication.
- staticlib.
- universal2 macOS library.
- Windows ARM64.
- musl.
- Android/iOS.
- ABI v2.
- installer integration.
- Python/Node-on-C refactors.
- release-binary installer/updater changes.

## 6. Required production/infrastructure changes

### A. Add manual release workflow

Create `.github/workflows/release-c.yml`:

- `workflow_dispatch` only;
- required `source_ref` input;
- concurrency scoped to source ref;
- `contents: read`;
- never publish;
- checkout the exact requested ref in every build job;
- record the resolved SHA in logs/artifact metadata.

Do not trigger on tags automatically.

### B. Five-target matrix

Use:

| ID | Runner | Rust target | Build |
|---|---|---|---|
| linux-x86_64 | ubuntu-24.04 | x86_64-unknown-linux-gnu.2.17 | cargo zigbuild |
| linux-aarch64 | ubuntu-24.04-arm | aarch64-unknown-linux-gnu.2.17 | cargo zigbuild |
| macos-x86_64 | macos-15-intel | x86_64-apple-darwin | cargo build |
| macos-arm64 | macos-14 or current pinned supported arm64 row | aarch64-apple-darwin | cargo build |
| windows-x86_64 | windows-2022 | x86_64-pc-windows-msvc | cargo build |

Keep the labels explicit; do not use `*-latest` for qualification identity.

Install Rust 1.89 everywhere.

For Linux, reuse the exact cargo-zigbuild 0.23.3 installation mechanics from
`release-binaries.yml` unless that workflow has already been intentionally
upgraded before implementation.

### C. Header drift before builds

On one authoritative Linux x86_64 job:

- install cbindgen 0.29.4;
- regenerate to temp;
- diff committed header;
- run M008 contract checker;
- require 90-line manifest.

Other rows consume the committed header. Header content must not be generated
differently per platform.

### D. Build exact cdylib

Build only the C binding manifest in release mode.

Expected library artifacts:

- Linux: `libstegoeggo_c.so`;
- macOS: `libstegoeggo_c.dylib`;
- Windows: `stegoeggo_c.dll` plus MSVC import library.

Do not package Rust rlibs/deps/target directories.

Assert exact expected paths rather than discovering candidates with broad
`find`.

### E. Per-platform export audit

Normalize only public `stegoeggo_*` exports and compare to
`abi-v1-symbols.txt`.

Linux:

```bash
nm -D --defined-only libstegoeggo_c.so
```

macOS:

```bash
nm -gU libstegoeggo_c.dylib
```

Normalize the Mach-O leading underscore before comparison if the tool output
includes it.

Windows:

```powershell
dumpbin /EXPORTS stegoeggo_c.dll
```

Parse the exported names only.

Acceptance for every row:

- 90 expected names;
- zero missing;
- zero extra `stegoeggo_*`.

Do not require absence of unrelated runtime/toolchain symbols unless they are
publicly named `stegoeggo_*`; record broader export observations separately.

### F. Linux glibc 2.17 proof

Build with target suffix `.2.17`.

Then inspect required symbol versions with `readelf --version-info` or
`objdump -T`.

Implement a semantic-version parser and fail if any required `GLIBC_X.Y`
version exceeds 2.17.

Also record `readelf -d` / `ldd` dependency information on the native
runner.

Do not claim 2.17 solely because cargo-zigbuild accepted the target string.

### G. macOS dependency/install-name proof

Record:

```bash
otool -L libstegoeggo_c.dylib
otool -D libstegoeggo_c.dylib
```

The library must not embed a developer-worktree absolute path that prevents
artifact use.

Compile/link/run the consumer against the actual artifact.

### H. Windows dependency/import-library proof

Record:

```powershell
dumpbin /DEPENDENTS stegoeggo_c.dll
dumpbin /EXPORTS stegoeggo_c.dll
```

Assert the MSVC import library exists and use it for the C/C++ link smoke.

Do not rely on PATH entries from the source-tree build when running the clean
artifact test.

### I. Native C11 functional smoke

On every target compile and run a dependency-free consumer distributed in the
artifact bundle.

It must:

- assert ABI major/minor and non-empty source version;
- synthesize a tiny valid PNG without project source fixtures;
- create notice/request;
- protect metadata-only with deterministic timestamp/seed;
- inspect returned buffer;
- verify protected result;
- assert rights found;
- exercise verification JSON buffer;
- free all handles;
- intentionally trigger one invalid-argument or resource-limit error and free
  the error.

Use platform-native compiler:

- GCC/Clang on Linux;
- clang on macOS;
- `cl.exe` on Windows.

### J. Native C++17 smoke

On every target compile a C++17 consumer that includes the same C header and
links/calls bootstrap + one lifecycle operation.

Purpose:

- verify `extern "C"` guards;
- verify fixed typedef/constants compile under C++;
- ensure no accidental C-only construct appears.

No C++ wrapper layer is created.

### K. Package artifact bundles

Each build row produces one inspection bundle with deterministic top-level
layout:

```text
stegoeggo-c/
  include/stegoeggo.h
  lib/<platform library>
  lib/<Windows import library when applicable>
  share/abi-v1-symbols.txt
  examples/smoke.c
  examples/smoke.cpp
  README.md
  LICENSE
  BUILD-INFO.txt
```

`BUILD-INFO.txt` records:

- source SHA;
- StegoEggo source version;
- ABI major/minor;
- Rust version;
- target;
- runner image;
- cbindgen version;
- cargo-zigbuild version when used.

Use tar.gz on Unix-like targets and zip on Windows, or let Actions preserve the
directory artifact without introducing a second archive if that is cleaner.
The contents contract matters more than compression format.

### L. Clean artifact-only consumer jobs

After build/upload, run native smoke from downloaded artifact only.

The smoke job must not depend on `bindings/c/src`, Cargo target output, or
the source checkout for library/header contents.

It may checkout nothing at all if the bundle contains the consumer examples.

Set loader path only to the downloaded artifact `lib/` directory:

- Linux `LD_LIBRARY_PATH`;
- macOS `DYLD_LIBRARY_PATH`;
- Windows prepend to `PATH`.

This proves the artifact bundle is actually consumable.

### M. Artifact assembly audit

Use one collector job after all five builds.

Download all artifacts and verify:

- exactly five target bundles;
- each contains header, manifest, README/license/build info;
- each contains one dynamic library;
- Windows also has import library;
- all five headers hash-identical;
- all five manifests hash-identical;
- ABI version in all BUILD-INFO files is 1.0;
- source SHA identical across all five.

No merged universal library is produced.

### N. Documentation/stability activation

Only after the complete final-SHA workflow is green:

Update `SUPPORT.md` with:

- C ABI status;
- five qualified targets;
- Linux glibc 2.17 floor;
- dynamic-library-only/staticlib deferred;
- C11/C++17 consumer support;
- qualification run ID/SHA.

Update `RELEASING.md` with:

- manual `release-c.yml` artifact qualification procedure;
- explicit no-publication behavior;
- artifact inspection guidance.

Update `STABILITY.md`:

- replace "planned — not stable, not shipped" with the actual ABI v1 stability
  contract;
- state existing v1 symbols/signatures/numeric codes/ownership rules are
  append-only within ABI major 1;
- breaking ABI requires `stegoeggo_v2_*`;
- distinguish source ABI stability from whether a given GitHub Release has C
  artifacts attached.

Do not claim staticlib stability.

### O. Release publication remains manual

The workflow ends with Actions artifacts.

If maintainers later attach C bundles to a GitHub Release, that is a separate
manual/release-distribution decision.

No token/write permission is needed here.

## 7. Ordered work packages

### WP1 — Manual five-target workflow

Create matrix and exact build/artifact paths.

Acceptance:

- workflow_dispatch only;
- exact SHA checkout;
- five native rows configured.

### WP2 — Export and platform audits

Add per-platform manifest/dependency/floor checks.

Acceptance:

- each target exports exactly 90 symbols;
- Linux <= glibc 2.17;
- macOS install-name/deps recorded;
- Windows import lib/deps recorded.

### WP3 — Native C/C++ consumers

Compile/link/run against each built artifact.

Acceptance:

- C functional lifecycle smoke green on five targets;
- C++17 include/link smoke green on five targets.

### WP4 — Bundle + clean install/load proof

Package minimal distribution bundle and test from downloaded artifact only.

Acceptance:

- no source-tree library/header dependency;
- five clean smokes green.

### WP5 — Collector audit

Verify five artifacts have identical contract files/source SHA and expected
platform libraries.

Acceptance:

- complete matrix;
- no partial success accepted.

### WP6 — Documentation/stability activation

Update SUPPORT/RELEASING/STABILITY and write closure only after final-SHA
workflow success.

Acceptance:

- no stale "planned" C ABI claim;
- no staticlib claim;
- exact run/SHA recorded.

## 8. Failure, cancellation, restart, and contention semantics

- Any failed/cancelled platform row invalidates qualification.
- Partial matrix success is not closure evidence.
- Any executable/header/manifest change after qualification SHA invalidates the
  run.
- Documentation-only closure commits after qualified SHA are allowed only when
  diff proves no executable/header/manifest change.
- If a platform exports extra/missing `stegoeggo_*` symbols, qualification
  stops; do not normalize them away.
- If Linux requires GLIBC >2.17, stop and correct build/tooling; do not raise
  the floor silently.
- If a clean artifact consumer only works with source-tree files, the package
  is not qualified.
- Retry failed workflow rows only on the same source SHA; final closure cites
  one complete successful run or clearly linked reruns with immutable SHA.
- No registry/release publication is used as a test mechanism.

## 9. Compatibility and migration

M010 is the point at which C ABI v1 becomes an external compatibility promise.

There are no prior stable C consumers to migrate.

After closure:

- existing v1 symbols/signatures/numeric codes/ownership rules are frozen;
- compatible additions may extend v1;
- breaking changes require v2 symbols;
- source version may advance independently of ABI major/minor;
- Python/Node remain direct bindings.

## 10. Required tests

Per platform:

1. cdylib builds;
2. expected library filename exists;
3. Windows import lib exists;
4. 90-symbol manifest exact;
5. header hash matches committed header;
6. C11 compile;
7. C11 link;
8. C11 runtime bootstrap;
9. metadata protect;
10. verify rights;
11. verification JSON;
12. error lifecycle;
13. all handle cleanup;
14. C++17 include;
15. C++17 link/call;
16. clean downloaded-artifact C smoke.

Linux additionally:

17. glibc max required <=2.17;
18. dynamic dependencies recorded.

macOS additionally:

19. install name recorded/usable;
20. dependencies recorded.

Windows additionally:

21. DLL dependencies recorded;
22. import library link proven.

Collector:

23. exactly five bundles;
24. same source SHA;
25. same header hash;
26. same manifest hash;
27. ABI 1.0 all rows.

## 11. Required verification commands

Local pre-dispatch:

```bash
./scripts/check.sh
cargo test --manifest-path bindings/c/Cargo.toml --locked
bindings/c/scripts/generate-header.sh --check
python3 bindings/c/scripts/check-contract.py
```

Linux release build shape:

```bash
cargo +1.89 zigbuild \
  --manifest-path bindings/c/Cargo.toml \
  --release --locked \
  --target x86_64-unknown-linux-gnu.2.17
```

Native platform builds use the exact target without glibc suffix.

Workflow dispatch must use an immutable SHA for final qualification.

Closure records the final:

- release-c run ID;
- source SHA;
- five job results;
- artifact names;
- export counts;
- Linux floor result;
- clean-smoke results.

## 12. Documentation updates

Update:

- `bindings/c/README.md`;
- `SUPPORT.md`;
- `RELEASING.md`;
- `STABILITY.md`;
- language-bindings roadmap;
- registry;
- new `plans/closure/language-bindings/010-status.md`.

Do not alter ADR-0006 unless a durable contradiction is discovered; a
contradiction requires a superseding ADR/corrective rather than silent edits.

## 13. Acceptance criteria

M010 closes only when:

- M008 and M009 closed;
- final immutable SHA builds all five targets;
- every target exports exactly the same 90-symbol manifest;
- Linux glibc requirement is <=2.17;
- C11 native lifecycle smoke green on all five;
- C++17 header/link smoke green on all five;
- clean artifact-only smoke green on all five;
- collector proves complete consistent artifact set;
- root `./scripts/check.sh` green;
- no automatic publication occurred;
- SUPPORT/RELEASING accurately document artifact process;
- STABILITY activates ABI v1 without claiming staticlib.

At closure the language-bindings roadmap can mark the initial
Python -> Node -> C sequence complete.

## 14. Stop conditions

Stop and write a corrective if:

- any platform needs a signature/code difference;
- cbindgen header is not portable across all five;
- Windows requires a contract-visible calling convention change;
- macOS loader correctness requires changing ABI signatures;
- Linux 2.17 floor cannot be met with established release tooling;
- symbol visibility cannot match the manifest on a platform;
- native consumer behavior differs by platform;
- safe packaging requires staticlib instead of the accepted cdylib contract.

## 15. Closure evidence required

Record:

- M008/M009 closure references;
- implementation SHA;
- final release-c run ID;
- exact runner labels/images;
- exact Rust/cbindgen/cargo-zigbuild versions;
- five library artifact names/hashes;
- 90-symbol comparison result per target;
- header/manifest hashes;
- Linux GLIBC version-needs result;
- macOS `otool` evidence;
- Windows `dumpbin` evidence;
- C11 and C++17 smoke result per target;
- clean artifact-only smoke result per target;
- collector audit;
- `./scripts/check.sh`;
- no-publication confirmation;
- STABILITY/SUPPORT/RELEASING transition;
- unresolved findings;
- roadmap/subsystem completion disposition.

## 16. Handoff notes

Treat qualification as ABI proof, not merely compilation.

A green `cargo build` is insufficient. The important evidence is that a
consumer with only the shipped header/library can compile, link, run, observe
the exact 90 exports, and manage ownership correctly on every supported
architecture.

Reuse existing StegoEggo release conventions where possible. In particular,
keep Linux at the existing glibc 2.17 floor rather than creating a C-specific
newer baseline.

Research references:

- current GitHub-hosted runner labels:
  https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- Rust cdylib/staticlib behavior:
  https://doc.rust-lang.org/reference/linkage.html
- cargo-zigbuild glibc-version targeting:
  https://github.com/rust-cross/cargo-zigbuild
