# Plan 104: Eggfetch Updater Release-Target Qualification and Closeout

## Goal

Qualify the Plan 103 native `eggfetch-core` updater migration against
StegoEggo's real distribution contract and close the line of work only after
the dependency, binary-size, target-build, updater, installer, and
documentation effects are measured rather than assumed.

Plan 104 is not a second implementation of updater networking. It is a
qualification and corrective-closeout pass for the code landed under Plan 103.

## Preconditions

Do not begin Plan 104 implementation work until Plan 103 has landed far enough
that:

- `stegoeggo-cli` resolves published `eggfetch-core 0.2.x`;
- the updater no longer invokes external curl;
- Rust 1.89 is the declared repository MSRV;
- ordinary workspace tests pass locally.

If Plan 103 is blocked before those conditions are true, record the blocker in
`plans/103-status.md` instead of using Plan 104 to bypass it.

## Scope

In scope:

- verify the exact resolved eggfetch dependency/features;
- measure dependency-graph growth attributable to the native updater;
- measure release binary size before/after the migration;
- compile/smoke all five supported prebuilt targets;
- preserve Linux glibc 2.17 release compatibility;
- exercise the updater's native transport through deterministic fixtures;
- exercise the current installer/updater release scripts together;
- perform a controlled live-network smoke against crates.io/GitHub without
  replacing a maintainer's primary binary;
- verify proxy, timeout, redirect, body-limit, 404 fallback, checksum, and
  candidate-validation behavior;
- reconcile documentation and agent guidance with observed behavior;
- make only narrow corrective changes required by qualification;
- write final evidence into `plans/104-status.md`.

Out of scope:

- adding new release targets;
- changing supported asset names;
- changing crates.io publication order;
- replacing installer shell/PowerShell transport with eggfetch;
- adding code signing/notarization;
- broad CLI refactors;
- introducing a second HTTP client;
- enabling eggfetch HTTP/2/3/compression/cookies/multipart/JSON/tracing without
  a separate plan.

## Workstream A: Dependency Graph Qualification

Capture the post-migration graph:

```bash
cargo tree -p stegoeggo-cli
cargo tree -p stegoeggo-cli -e features
cargo tree -p stegoeggo-cli -d
```

Record in `plans/104-status.md`:

- exact `eggfetch-core` version;
- exact enabled eggfetch features;
- whether `eggfetch-http-connect` is present because the selected proxy
  profile requires it;
- Tokio feature union;
- Rustls/native-root stack;
- duplicate dependency versions worth noting;
- confirmation that HTTP/2, HTTP/3, compression, cookies, multipart, JSON, and
  tracing are absent unless technically forced by the selected profile.

Do not optimize away expected dependencies simply to reduce a count. Correct
behavior and release portability come first.

If a feature is unexpectedly enabled, trace the activating edge with:

```bash
cargo tree -p stegoeggo-cli -e features -i <crate>
```

and correct the manifest if possible without reducing required updater
behavior.

## Workstream B: Baseline and Binary-Size Evidence

Measure the footprint impact of embedding the native updater.

Use comparable release builds:

```bash
cargo build --locked --release --package stegoeggo-cli --bin stegoeggo
```

Where a pre-Plan-103 baseline commit is available, build the same target and
feature set from that commit/worktree. Record:

- stripped executable byte size before Plan 103;
- stripped executable byte size after Plan 103;
- absolute delta;
- percentage delta;
- target and toolchain used;
- whether LTO/strip/codegen settings match the release profile.

At minimum measure native development-host output and Linux x86_64 release
output. Prefer measurements for all five public targets when artifacts are
available.

The plan does not impose an arbitrary maximum percentage increase. Instead,
treat unexpectedly large growth as an investigation trigger. If the native
updater adds substantial size, use `cargo bloat` or equivalent local tooling
only as a diagnostic aid; do not add such tooling to required CI.

Do not remove TLS validation, native roots, proxy support, body bounds, or
strict redirects to win a size benchmark.

## Workstream C: Five-Target Build Qualification

Re-run the current release matrix with the Plan 103 dependency graph:

```text
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
x86_64-apple-darwin
aarch64-apple-darwin
x86_64-pc-windows-msvc
```

Use the existing `.github/workflows/release-binaries.yml` contract rather
than creating a parallel matrix.

For Linux targets, preserve the existing cargo-zigbuild glibc 2.17
compatibility target.

Validate that eggfetch/Rustls/native-certs do not accidentally make the
existing release targets non-buildable or raise the effective Linux runtime
floor.

Required local/CI evidence where available:

- exact build command;
- successful link;
- `stegoeggo version`;
- `stegoeggo --help`;
- protect/inspect/verify smoke already required by the release workflow.

If a target fails due to eggfetch or a transitive native dependency, diagnose
the target-specific dependency before altering the support matrix.

## Workstream D: MSRV Qualification

Run the complete updated Rust 1.89 matrix, including fresh package resolution:

```bash
rustc +1.89.0 --version
cargo +1.89.0 check -p stegoeggo-stego --locked
cargo +1.89.0 check -p stegoeggo --locked
cargo +1.89.0 check -p stegoeggo --locked --no-default-features
cargo +1.89.0 check -p stegoeggo --locked --all-features
cargo +1.89.0 check -p stegoeggo-cli --locked
cargo +1.89.0 check -p stegoeggo-cli --locked --all-features
cargo +1.89.0 test -p stegoeggo-stego --locked
cargo +1.89.0 test -p stegoeggo --lib --locked --all-features
MSRV=1.89 ./scripts/validate-msrv-package.sh
```

Also verify no authoritative 1.87 claim remains:

```bash
rg -n '1\.87' Cargo.toml stegoeggo-stego stegoeggo-cli .github scripts docs   README.md SUPPORT.md STABILITY.md RELEASING.md AGENTS.md
```

Historical plan/status files may legitimately retain old evidence and should
not be rewritten.

## Workstream E: Native Updater Contract Tests

Exercise the new in-process transport at the behavioral boundaries that
matter to update safety.

Required deterministic cases:

### Registry

- 200 valid JSON resolves newest stable non-yanked version;
- prereleases ignored;
- yanked releases ignored;
- malformed JSON is fatal;
- oversized body is fatal;
- timeout is fatal;
- connection failure is fatal;
- non-2xx is fatal.

### Release executable

- 200 downloads exact bytes;
- allowed HTTPS redirect is followed;
- HTTPS -> HTTP downgrade is denied;
- exact executable 404 may enter Cargo fallback;
- 401/403/429/5xx do not enter Cargo fallback;
- oversized executable is fatal;
- timeout is fatal;
- partial/truncated response is fatal.

### Sidecar

- 200 downloads and parses valid digest;
- 404 is fatal;
- malformed digest is fatal;
- oversized body is fatal;
- network failure is fatal.

### Proxy

Using deterministic environment or injected settings:

- HTTP(S)_PROXY is explicitly applied;
- lowercase/uppercase precedence follows eggfetch's documented
  `ProxyEnvironment` behavior;
- NO_PROXY bypass works;
- invalid proxy URL fails closed;
- diagnostics do not expose embedded proxy credentials.

### Candidate/replacement

Retain the existing checks:

- checksum mismatch leaves current executable untouched;
- candidate wrong program leaves current executable untouched;
- candidate wrong version leaves current executable untouched;
- unwritable destination fails before network work;
- replacement is attempted only after all verification succeeds;
- updater never invokes `sudo`.

## Workstream F: Release Rehearsal Scripts

Run the existing distribution validation against the migrated updater:

```bash
./scripts/check.sh
./scripts/check-docs-contract.sh
./scripts/test-release-installers.sh
./scripts/test-release-updater.sh
```

Review `scripts/test-release-updater.sh` to ensure it actually reaches the
new native transport path rather than only asserting old curl subprocess
behavior.

If the script currently mocks curl or depends on executable substitution,
rewrite only the updater-network fixture portion so it drives the eggfetch
path through loopback URLs. Retain the existing candidate/Cargo subprocess
fixtures where those subprocesses still exist.

Do not weaken failure cases simply because the transport implementation
changed.

## Workstream G: Controlled Live-Network Smoke

After deterministic tests pass, perform a non-destructive live smoke against
the real public endpoints.

Required checks:

1. query the real crates.io `stegoeggo-cli` endpoint using the same native
   client configuration used by `update`;
2. fetch a small known checksum sidecar from an existing StegoEggo release;
3. optionally fetch a release executable to a temporary directory if needed
   to validate redirect/body-limit behavior;
4. verify no self-replacement occurs during this diagnostic path.

Prefer an existing test harness/helper or temporary test binary over invoking
`stegoeggo update` against a maintainer's primary installation when a newer
release exists.

Record:

- date;
- tested release/version;
- host/target;
- success/failure;
- proxy environment if relevant, without secrets.

A transient external outage is not a reason to weaken deterministic tests.
Record it and retry during qualification.

## Workstream H: Installer/Updater Boundary

The bootstrap installers remain external-tool scripts and are not migrated to
eggfetch. Verify that documentation still makes this distinction clear:

```text
bootstrap install:
    shell/PowerShell downloads release binary

installed stegoeggo update:
    embedded eggfetch-core transport
```

Run installer tests to prove Plan 103 did not accidentally alter asset naming,
version output, checksum formatting, or binary feature set.

The installer and updater must still agree on:

- target table;
- versionless asset names;
- SHA-256 sidecar names;
- candidate `stegoeggo X.Y.Z` first-line identity;
- unsupported-target behavior;
- exact executable 404 fallback semantics.

## Workstream I: Security and Documentation Reconciliation

Re-read observed implementation rather than mechanically replacing the word
`curl`.

Review at minimum:

- `README.md`;
- `docs/installation.md`;
- `docs/cli-usage.md`;
- `architecture/cli.md`;
- `SECURITY.md`;
- `SUPPORT.md`;
- `STABILITY.md`;
- `RELEASING.md`;
- `AGENTS.md`.

Required final truth:

- updater transport is eggfetch-core;
- bootstrap installers may still use curl/PowerShell;
- updater honors explicit conventional proxy environment handling;
- TLS verification remains enabled;
- HTTPS downgrade redirects are denied;
- downloads are bounded;
- checksum sidecars are integrity evidence, not independent signing;
- crates.io remains stable-version authority;
- GitHub Release is the binary source;
- only unsupported target / exact executable 404 permit Cargo fallback;
- MSRV is 1.89.

Update the plan-execution or conventions skill only if a genuinely reusable
new repository rule emerged. Do not add eggfetch-specific detail to generic
skills unnecessarily.

## Workstream J: Corrective Threshold

Narrow corrective changes are allowed in Plan 104 when qualification exposes:

- a feature-activation mistake;
- missing response bound;
- wrong timeout semantics;
- proxy behavior regression;
- redirect downgrade regression;
- fixture script still tied to curl;
- target-specific build break;
- stale MSRV/documentation claim;
- unexpectedly large dependency accidentally enabled.

Do not use this plan for unrelated updater redesign.

If correction requires changing release authority, target support, trust model,
or fallback policy, stop and write a new numbered plan rather than silently
broadening Plan 104.

## Required Validation

At final closeout run:

```bash
./scripts/check.sh
cargo test -p stegoeggo-cli --all-features
cargo +1.89.0 check -p stegoeggo-stego --locked
cargo +1.89.0 check -p stegoeggo --locked --all-features
cargo +1.89.0 check -p stegoeggo-cli --locked --all-features
MSRV=1.89 ./scripts/validate-msrv-package.sh
cargo deny check licenses
cargo deny check advisories
./scripts/check-docs-contract.sh
./scripts/test-release-installers.sh
./scripts/test-release-updater.sh
./scripts/release-binary-preflight.sh --tag=<controlled-tag>
```

Where a controlled tag is inappropriate on the working tree, use the same
temporary local-tag discipline established by Plan 102 and record that it was
not pushed.

## Acceptance Criteria

Plan 104 is complete when:

- the exact eggfetch dependency/features are documented;
- no unintended eggfetch protocol/codec features are active;
- native and release-target builds succeed for the supported binary matrix, or
  any genuine blocker is explicitly recorded rather than hidden;
- Linux release compatibility remains at the documented glibc 2.17 floor;
- Rust 1.89.0 MSRV checks pass with fresh package resolution;
- deterministic updater transport tests cover registry, asset, sidecar,
  proxy, redirect, timeout, body-limit, status, checksum, candidate, and
  replacement boundaries;
- installer and updater rehearsal scripts pass through the new architecture;
- at least one controlled live public-endpoint smoke succeeds;
- before/after dependency and binary-size evidence is recorded;
- documentation accurately distinguishes installer transport from updater
  transport;
- no security/fallback invariant from Plans 100-103 was weakened;
- `plans/103-status.md` records implementation completion;
- `plans/104-status.md` records the final qualification evidence and any
  deliberately deferred real-release item.

## Closeout Rule

Do not mark this line of work complete merely because the code compiles.

The native updater migration is closed only after target portability,
dependency/size impact, updater failure behavior, release scripts, and
documentation have all been qualified and recorded.
