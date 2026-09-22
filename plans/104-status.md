# Plan 104 Status: Eggfetch Updater Release-Target Qualification and Closeout

Status: complete. Evidence below; no security or fallback invariant weakened.

## Dependency graph

- Exact resolution: `eggfetch-core 0.2.0`, `eggfetch-http-connect 0.2.0`
  (proxy-transport primitive owned by eggfetch), `tokio 1.52.3` in
  `Cargo.lock`, all from crates.io.
- Enabled eggfetch features: `http1`, `tls-rustls`, `tls-native-roots`,
  `proxy` plus implied `high-level-url`, `redirects`, `logical-retry`,
  `basic-auth`, `native-http1`, `standard-route`, `transport-http1`.
  `eggfetch-http-connect` is present because the `proxy` profile requires
  it for CONNECT tunneling.
- Absent: HTTP/2, HTTP/3, compression, cookies, multipart, JSON, tracing
  (no such `eggfetch-core feature` edge in
  `cargo tree -p stegoeggo-cli -e features`).
- Tokio feature union comes from eggfetch (`rt, net, time, sync, macros,
  io-util`); the CLI declares only `tokio { version = "1", features =
  ["rt"] }` for its narrow current-thread runtime.
- TLS stack: rustls with native roots plus packaged WebPKI fallback; no
  verification toggle in updater code.
- Duplicate versions are pre-existing major-split pairs (base64 0.22 vs
  0.23, block-buffer/digest/sha2 0.10 vs 0.11 line via ed25519, getrandom
  0.2 vs 0.4 via tempfile); no unexpected heavy duplication from
  eggfetch.
- Root and carrier crates have no eggfetch edge.

## Binary footprint

Comparable release builds (`cargo build --locked --release --package
stegoeggo-cli --bin stegoeggo`, profile `lto=true, strip=symbols,
codegen-units=1, panic=abort, opt-level=s`, stable 1.98.1):

| Build | Target | Bytes |
|---|---|---:|
| Pre-103 baseline (curl updater) | aarch64-apple-darwin | 1,598,288 |
| Post-103 (eggfetch updater) | aarch64-apple-darwin | 3,438,640 |
| Post-103 | x86_64-apple-darwin | 3,960,240 |
| Post-103 (zigbuild glibc 2.17) | x86_64-unknown-linux-gnu | 4,316,432 |
| Post-103 (zigbuild glibc 2.17) | aarch64-unknown-linux-gnu | 3,783,232 |
| Published v0.4.1 asset (curl) | aarch64-apple-darwin | 1,598,288 |

Delta on the host target: +1,840,352 bytes (+115.1%). The growth is the
expected cost of embedding a TLS HTTP stack (rustls/ring/hyper) where
previously the OS curl binary was reused; the published-asset size match
confirms the baseline measurement. No size-driven capability was removed:
TLS verification, native roots, proxy support, body bounds, and strict
redirects are intact.

## Target qualification

- `x86_64-unknown-linux-gnu`: release builds with
  `cargo zigbuild --target 'x86_64-unknown-linux-gnu.2.17'`; max
  referenced GLIBC is 2.17, floor preserved.
- `aarch64-unknown-linux-gnu`: release builds with
  `cargo zigbuild --target 'aarch64-unknown-linux-gnu.2.17'`; max
  referenced GLIBC is 2.17, floor preserved.
- `x86_64-apple-darwin`: release builds locally and executes
  (`stegoeggo 0.4.1`); `cargo check --target` passes.
- `aarch64-apple-darwin`: release builds locally; `version`, `--help`,
  and protect/inspect/verify smoke pass per the release-workflow
  procedure; `cargo check --target` passes.
- `x86_64-pc-windows-msvc`: local `cargo check --target` cannot link
  ring without an MSVC/mingw C toolchain (same environmental limit
  applies to any C-building dependency; direct `cargo check` for the
  linux targets fails locally for the same reason absent zigbuild).
  Windows buildability is qualified by the manual `release-binaries.yml`
  job on `windows-latest`, which builds natively; no code-level Windows
  exclusion was found (no unix-only paths in the new transport code
  beyond the pre-existing executable-permission block).

## MSRV 1.89

- `cargo +1.89.0 check` passes for carrier, library (default, minimal,
  all-features), and CLI (default, all-features), all `--locked`.
- `cargo +1.89.0 test -p stegoeggo-stego --locked` and
  `cargo +1.89.0 test -p stegoeggo --lib --locked --all-features` pass.
- `MSRV=1.89 ./scripts/validate-msrv-package.sh` passes with fresh
  package resolution.
- `rg -n '1\.87'` over manifests, workflows, scripts, and current docs
  shows no remaining authoritative claim (only CHANGELOG history and
  historical plan evidence retain 1.87).

## Deterministic transport tests

`cargo test -p stegoeggo-cli --all-features`: 25 updater tests pass,
including the 14 native cases (registry/asset/sidecar 200, asset 404
fallback, sidecar 404 hard failure, 500 hard failure, timeout hard
failure, three oversized rejections, redirect following, strict
downgrade policy, proxy precedence/bypass/fail-closed/redaction, and
transport-error hard failure) plus retained checksum, candidate,
target-mapping, asset-naming, fallback-rule, and preflight tests.

## Rehearsal scripts

- `./scripts/check.sh` passes (fmt, clippy, no-default-features check,
  workspace tests, docs contract).
- `./scripts/check-docs-contract.sh` passes (5 release targets).
- `./scripts/test-release-installers.sh` passes.
- `./scripts/test-release-updater.sh` passes through the native
  transport. The fixture was corrected: it now updates to a
  newer-than-current version so the download path is actually exercised
  (previously current==latest made every case a no-op), stages all five
  target assets (robust to uname/Rust architecture-name skew found on
  this host), and derives expected versions from the built binary.
- `release-binary-preflight.sh` updater-mapping checks (5 targets, Unix
  and Windows asset naming) pass. Full preflight needs a real `vX.Y.Z`
  tag pointing at HEAD and is deferred to release time; no temporary tag
  was pushed.

## Controlled live smoke (2026-09-22)

A temporary harness using the production client configuration
(UA `stegoeggo-cli/0.4.1`, connect 10s, total 60s, strict redirects,
proxy environment, no decompression) against public endpoints, with no
self-replacement:

- `crates.io/api/v1/crates/stegoeggo-cli`: 200, 13,913 bytes, latest
  stable 0.4.1.
- `releases/download/v0.4.1/...aarch64-apple-darwin.sha256`: 200 (via
  HTTPS redirect), 112 bytes, valid 64-hex digest.
- `releases/download/v0.4.1/...aarch64-apple-darwin`: 200, 1,598,288
  bytes staged to temp only.

Host: aarch64-apple-darwin. No proxy in environment. Nothing installed
or replaced.

## Installer/updater boundary

Documentation states the split in `docs/installation.md`,
`architecture/cli.md`, `architecture/tooling.md`, `SECURITY.md`,
`SUPPORT.md`, `STABILITY.md`, `README.md`, and `docs/cli-usage.md`:
bootstrap installers use curl/PowerShell; installed `stegoeggo update`
uses embedded eggfetch-core. Asset naming, sidecar format, candidate
`stegoeggo X.Y.Z` identity, unsupported-target behavior, and exact-asset
404 fallback semantics are unchanged on both sides.

## Corrective changes made during qualification

- Fixed new-clippy `chunks_exact_to_as_chunks` findings in
  `stegoeggo-stego` (`lsb_internal.rs`, `jpeg.rs`, `pixels.rs`) via
  `as_chunks` (allowed by the 1.89 floor); required for `check.sh`.
- Corrected `test-release-updater.sh` as described above; the old
  fixture could not exercise any download on version 0.4.1.
- No release-authority, target-support, trust-model, or fallback-policy
  change was needed or made.

## Deferred

- Full five-target `release-binaries.yml` run and real-tag preflight:
  manual at next release; local evidence above covers four targets plus
  the documented Windows path.
- `cargo bloat` diagnostics: unnecessary; the delta is attributable to
  the TLS/HTTP stack, not investigated further.


## Follow-on release closure

The implementation/qualification work in Plan 104 is complete, but two
release-dependent proofs are intentionally tracked separately:

- Plan 105: first public eggfetch-enabled stable release, real five-target
  workflow including native Windows, public asset audit, installer smoke, and
  released updater current-version smoke.
- Plan 106: first real public eggfetch-to-eggfetch stable self-update A -> B.

These follow-on plans do not reopen the Plan 103 transport migration.
