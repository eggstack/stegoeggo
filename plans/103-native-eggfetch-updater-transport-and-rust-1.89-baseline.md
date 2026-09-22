# Plan 103: Native Eggfetch Updater Transport and Rust 1.89 Baseline

## Goal

Replace the `stegoeggo update` command's external `curl` transport with
`eggfetch-core 0.2.x` while preserving the updater's existing release,
integrity, fallback, replacement, and user-facing contracts.

This is a narrow CLI transport migration. It must not add HTTP behavior to the
`stegoeggo` library or `stegoeggo-stego` carrier crate, and it must not
change how image processing, release asset naming, checksum verification, or
self-replacement work.

The migration also raises the repository MSRV from Rust 1.87 to Rust 1.89 so
the published `eggfetch-core 0.2.0` dependency is supported explicitly rather
than compiling only on newer undeclared toolchains.

## Why This Plan Exists

The current updater in `stegoeggo-cli/src/update.rs` shells out to `curl`
for three network operations:

1. query crates.io for the latest stable `stegoeggo-cli` version;
2. download the matching GitHub Release executable;
3. download the executable's SHA-256 sidecar.

The rest of the updater is already appropriately conservative:

- crates.io is the stable version authority;
- GitHub Release `vX.Y.Z` is the binary source;
- unsupported targets and the exact executable asset HTTP 404 are the only
  Cargo fallback cases;
- checksum, network, TLS, identity, version, and replacement failures are hard
  failures;
- destination writability is checked before download;
- candidate identity/version are checked before replacement;
- the updater never invokes `sudo`.

`eggfetch-core 0.2.0` now has a supported updater-oriented feature profile
and exposes the timeout, redirect, body-limit, proxy-environment, TLS, and
typed status/error behavior needed to retain those invariants in-process.

Reference release:

- crate: `eggfetch-core 0.2.0`;
- tag: `eggfetch v0.2.0`;
- release commit: `8959ca890ee34f4cf456aed648315322f1e83ef7`;
- eggfetch MSRV: Rust 1.89.

Do not depend on an unpublished git revision.

## Scope

In scope:

- add `eggfetch-core 0.2.x` to `stegoeggo-cli` only;
- add the minimal direct Tokio runtime dependency needed by the synchronous CLI
  wrapper;
- replace curl-backed updater downloads with one configured native eggfetch
  client;
- preserve proxy-environment behavior deliberately;
- preserve connect and whole-request deadlines;
- add explicit response-body bounds;
- use strict HTTPS redirect downgrade handling;
- remove curl-specific updater code and errors;
- preserve Cargo and candidate subprocess bounds;
- raise repository MSRV to Rust 1.89;
- update MSRV CI/scripts/docs;
- add deterministic loopback updater transport tests;
- update updater architecture/security/installation documentation where the
  transport implementation is described.

Out of scope:

- using eggfetch in the root library or carrier crate;
- adding general URL fetching commands;
- background update checks;
- HTTP/2 or HTTP/3;
- response decompression;
- cookies, multipart, JSON convenience features, or tracing;
- changing crates.io as version authority;
- changing release target names or supported targets;
- changing the SHA-256 trust model;
- code signing/notarization;
- changing self-replacement semantics;
- changing installer transport in `packaging/install.sh` or
  `packaging/install.ps1`;
- adopting eggfetch's CLI or Python API.

## Hard Compatibility Invariants

Implementation must preserve all of the following.

### Version authority

`https://crates.io/api/v1/crates/stegoeggo-cli` remains authoritative for
the latest stable version. Continue ignoring yanked and prerelease versions
according to the existing stable-version parser.

### Binary source

For version `X.Y.Z`, use the existing exact GitHub Release URL contract:

```text
https://github.com/eggstack/stegoeggo/releases/download/vX.Y.Z/stegoeggo-<target>[.exe]
https://github.com/eggstack/stegoeggo/releases/download/vX.Y.Z/stegoeggo-<target>[.exe].sha256
```

Do not introduce the GitHub Releases API as a second version authority.

### Fallback policy

Cargo fallback remains allowed only when:

- the host target is intentionally unsupported by the binary matrix; or
- the exact executable asset returns HTTP 404.

The checksum sidecar 404 is still a hard failure.

The following must never fall back to Cargo:

- DNS failure;
- connection refusal;
- TLS/certificate failure;
- timeout;
- proxy configuration failure;
- redirect policy failure;
- HTTP 401/403/429/5xx;
- malformed registry response;
- missing/malformed checksum;
- checksum mismatch;
- candidate execution failure;
- candidate identity mismatch;
- candidate version mismatch;
- replacement failure.

### Trust and replacement

Retain the current sequence:

```text
resolve stable version
  -> verify destination is replaceable
  -> select target
  -> download exact asset + checksum
  -> verify SHA-256
  -> mark candidate executable where needed
  -> execute candidate "version"
  -> require exact stegoeggo identity and version
  -> self_replace
```

The current executable must remain untouched on every failure before the final
replacement step.

## Workstream A: Dependency and Feature Profile

Modify `stegoeggo-cli/Cargo.toml` only for the HTTP/runtime dependencies.

Preferred eggfetch declaration:

```toml
eggfetch-core = {
    version = "0.2.0",
    default-features = false,
    features = [
        "http1",
        "tls-rustls",
        "tls-native-roots",
        "proxy",
    ],
}
```

This deliberately excludes:

- HTTP/2;
- HTTP/3;
- compression;
- cookies;
- multipart;
- JSON;
- tracing.

A direct Tokio dependency is required because `eggfetch-core` is async-first
while the StegoEggo CLI remains synchronous. Enable only the runtime features
needed to construct and drive a current-thread runtime; rely on normal Cargo
feature unification rather than enabling Tokio's `full` feature.

Do not add `eggfetch-core` to the root or carrier manifests.

After dependency changes:

```bash
cargo update -p eggfetch-core
cargo tree -p stegoeggo-cli
cargo tree -p stegoeggo-cli -e features
```

Inspect the graph to ensure the excluded eggfetch capabilities have not been
accidentally activated.

## Workstream B: Rust 1.89 MSRV Migration

Raise the repository's supported Rust floor from 1.87 to 1.89 as one explicit
support-contract change.

Update every authoritative declaration/reference, including at minimum:

- root `Cargo.toml` `rust-version`;
- `stegoeggo-stego/Cargo.toml` `rust-version`;
- `stegoeggo-cli/Cargo.toml` `rust-version`;
- `.github/workflows/assurance.yml`;
- `scripts/validate-msrv-package.sh`;
- `SUPPORT.md`;
- `AGENTS.md`;
- other docs/scripts found by a repository-wide search for `1.87`.

Use exact Rust 1.89.0 for MSRV evidence, not merely a current stable toolchain.

Required search:

```bash
rg -n '1\.87|MSRV' .
```

Do not leave the carrier/library documented at 1.87 while the workspace
lockfile contains a CLI dependency requiring 1.89. The project uses one
repository-level support floor.

## Workstream C: Native HTTP Client Construction

Refactor the updater to create one eggfetch client per `run_update`
invocation and reuse it for registry, executable, and checksum requests.

The client must configure:

- user agent:
  `stegoeggo-cli/<CARGO_PKG_VERSION>`;
- connect deadline: 10 seconds;
- absolute total request deadline: 60 seconds;
- redirect following with HTTPS -> HTTP downgrade denied;
- explicit proxy environment snapshot;
- native/root TLS verification.

Do not use `Timeout::from_secs(60)` as a substitute for the current
`curl --max-time 60` contract because that helper does not set the absolute
total deadline. Construct the timeout explicitly, for example:

```rust
let timeout = eggfetch_core::Timeout::builder()
    .connect(Duration::from_secs(10))
    .total(Duration::from_secs(60))
    .build();
```

Use a strict redirect policy, e.g. `RedirectPolicy::strict(...)`, with a
finite redirect count.

### Proxy compatibility

Curl historically honored conventional proxy variables automatically.
Eggfetch does not do so unless requested.

Snapshot and apply:

- `HTTP_PROXY` / `http_proxy`;
- `HTTPS_PROXY` / `https_proxy`;
- `ALL_PROXY` / `all_proxy`;
- `NO_PROXY` / `no_proxy`.

Use `ProxyEnvironment::from_env()` and
`ClientBuilder::proxy_environment(...)`.

Invalid proxy configuration must fail closed with an updater error; do not
silently retry direct.

## Workstream D: Synchronous CLI / Async Transport Boundary

Keep `main`, command dispatch, and `run_update()` synchronous.

Introduce a small private async implementation such as:

```text
run_update()
  -> construct current-thread Tokio runtime
  -> block_on(run_update_async())
```

or an equivalent narrow helper.

Requirements:

- one runtime per updater invocation is acceptable;
- no global runtime;
- no nested-runtime assumptions elsewhere in the CLI;
- runtime construction failure maps to an actionable updater error;
- no async surface leaks into public CLI/library APIs.

Do not convert unrelated CLI modules to async.

## Workstream E: Bounded Download Primitive

Replace `curl_download()` and `download_required()` with one native helper
that:

1. issues GET;
2. returns the exact HTTP status separately from transport failures;
3. rejects unexpected non-2xx status according to the caller's fallback
   rules;
4. consumes the body under a caller-supplied maximum size;
5. writes the body to the requested private temporary path;
6. propagates typed eggfetch failures without converting them into fallback.

Use eggfetch's `max_decoded_body_size` request/client facility rather than an
unbounded caller-side `Vec` accumulation.

Because compression features are intentionally disabled, this limit applies
to the identity response body without pulling the decompression stack into the
binary.

Recommended initial bounds:

```text
crates.io registry JSON:    4 MiB
SHA-256 sidecar:            8 KiB
release executable:        64 MiB
```

These values should be named constants and covered by tests. If existing
release artifacts justify a different executable ceiling, record the evidence
in Plan 104 rather than making the limit effectively unbounded.

Preserve temporary-directory staging and the existing file paths so checksum
and candidate verification code does not need to change.

## Workstream F: Error Model Cleanup

Remove curl-specific state from `UpdateError`, including concepts such as:

- `CurlUnavailable`;
- `CurlFailed`;
- parsing curl's `%{http_code}` stdout.

Introduce transport errors that preserve the distinction between:

- request construction/configuration;
- proxy configuration;
- transport/TLS/DNS/timeout failure;
- HTTP status;
- response body too large;
- local I/O.

Do not include secrets from proxy URLs or credentials in diagnostics.
Eggfetch's redaction guarantees should not be bypassed by formatting internal
structures manually.

Keep the existing `HttpStatus { url, status }`-style semantic distinction if
it remains useful for the 404-only fallback rule.

## Workstream G: Preserve Process Helpers

Do not delete `run_bounded()` merely because curl is gone.

It remains required for:

- `cargo --version`;
- `cargo install stegoeggo-cli ...`;
- staged candidate `stegoeggo version`.

Retain the current bounded stdout/stderr capture and process timeout semantics
unless a test reveals a separate defect.

After migration, no updater code should execute `curl`.

Required repository check:

```bash
rg -n 'curl|CURL_' stegoeggo-cli/src/update.rs
```

Expected result: no runtime curl dependency remains in that module.

Installer shell scripts may still use curl and are outside this plan.

## Workstream H: Deterministic Transport Tests

Expand updater tests so ordinary CI never depends on crates.io or GitHub.

Use a loopback HTTP server or an injected/local endpoint through the existing
`STEGOEGGO_CRATES_API_URL` and `STEGOEGGO_RELEASES_URL` test hooks.

Cover at minimum:

```text
native_registry_200_resolves_latest
native_asset_200_is_written
native_sidecar_200_is_written
native_asset_404_allows_cargo_fallback
native_sidecar_404_is_hard_failure
native_500_is_hard_failure
native_timeout_is_hard_failure
native_oversized_registry_body_is_rejected
native_oversized_sidecar_is_rejected
native_oversized_asset_is_rejected
native_redirect_is_followed
https_to_http_redirect_policy_is_strict
proxy_environment_is_applied_explicitly
network_failure_does_not_allow_fallback
```

Where testing strict HTTPS downgrade would require disproportionate fixture
TLS machinery, test the configured policy directly plus an eggfetch-level
loopback case where practical; do not weaken the production policy merely to
simplify tests.

Keep existing tests for:

- checksum mismatch;
- wrong candidate program;
- wrong candidate version;
- target mapping;
- asset naming;
- unwritable destination;
- stable version filtering.

## Workstream I: Documentation and Maintainer Contract

Update only documentation whose assertions change.

At minimum review:

- `docs/installation.md`;
- `docs/cli-usage.md`;
- `architecture/cli.md`;
- `SECURITY.md`;
- `SUPPORT.md`;
- `AGENTS.md`;
- `RELEASING.md`.

Required changes:

- remove the claim that `stegoeggo update` requires/uses external curl;
- state that the updater uses the embedded eggfetch Rust transport;
- retain crates.io authority and GitHub binary source;
- retain the narrow Cargo fallback rule;
- retain checksum limitations: SHA-256 sidecars are integrity checks, not
  independent publisher signatures;
- document proxy-environment behavior if user-facing docs mention updater
  networking;
- update MSRV to 1.89;
- leave bootstrap installer curl/PowerShell behavior unchanged.

Do not describe eggfetch as part of the public `stegoeggo` Rust API.

## Workstream J: Lockfile, License, and Advisory Review

Regenerate `Cargo.lock` through Cargo rather than hand editing.

Run:

```bash
cargo tree -p stegoeggo-cli -e features
cargo deny check licenses
cargo deny check advisories
```

If eggfetch introduces a license not currently permitted by `deny.toml`,
verify the actual dependency license before altering policy. Do not add a broad
license exception merely to make the check pass.

Review duplicate major/minor dependency lines with:

```bash
cargo tree -p stegoeggo-cli -d
```

Duplicate versions are not automatically a blocker, but unexpected heavy
duplicates should be recorded for Plan 104 footprint qualification.

## Validation Commands

Required before marking implementation complete:

```bash
./scripts/check.sh
cargo +1.89.0 check -p stegoeggo-stego --locked
cargo +1.89.0 check -p stegoeggo --locked
cargo +1.89.0 check -p stegoeggo --locked --no-default-features
cargo +1.89.0 check -p stegoeggo --locked --all-features
cargo +1.89.0 check -p stegoeggo-cli --locked
cargo +1.89.0 check -p stegoeggo-cli --locked --all-features
cargo +1.89.0 test -p stegoeggo-stego --locked
cargo +1.89.0 test -p stegoeggo --lib --locked --all-features
cargo test -p stegoeggo-cli --all-features
cargo deny check licenses
cargo deny check advisories
./scripts/test-release-updater.sh
./scripts/check-docs-contract.sh
```

Also verify:

```bash
cargo tree -p stegoeggo-cli -e features
rg -n '1\.87' .
rg -n 'curl|CURL_' stegoeggo-cli/src/update.rs
```

The first search should show no stale authoritative MSRV claim. The second
should show no updater runtime dependency on curl.

## Acceptance Criteria

Plan 103 is complete when:

- `stegoeggo-cli` uses published `eggfetch-core 0.2.x` for updater HTTP;
- root and carrier crates do not depend on eggfetch;
- the updater no longer invokes curl;
- the CLI remains synchronously structured outside the private updater runtime;
- connect timeout remains 10 seconds and total request deadline remains 60
  seconds;
- conventional proxy environment variables are explicitly supported;
- redirects are followed but HTTPS -> HTTP downgrade is denied;
- registry, checksum, and executable responses have explicit finite body
  limits;
- exact asset HTTP 404 remains the only network-status Cargo fallback;
- sidecar 404 and all other network/status failures remain hard failures;
- checksum, candidate identity/version, and self-replacement logic remain
  intact;
- the repository's documented and tested MSRV is Rust 1.89;
- updater transport behavior is covered by local deterministic tests;
- workspace checks, exact-MSRV checks, cargo-deny checks, updater rehearsal,
  and docs-contract checks pass.

## Failure Policy

Do not make the migration appear successful by weakening existing updater
security or fallback rules.

If eggfetch cannot reproduce an existing required behavior:

- stop and record the exact incompatibility in `plans/103-status.md`;
- do not silently reintroduce a second HTTP stack;
- do not keep curl as an undocumented fallback;
- do not allow generic transport failures to trigger Cargo;
- do not disable TLS verification;
- do not permit HTTPS downgrade redirects;
- do not remove checksum/candidate validation.

If the embedded transport materially breaks a supported release target,
record it and proceed to Plan 104 qualification rather than dropping the
target ad hoc.

## Ordering

Plan 103 follows the completed CLI updater/release work in Plans 098-102.

Plan 104 is the required qualification/closeout pass after this implementation
lands. Plan 103 should not claim binary-footprint or full release-target
qualification that has not actually been measured.
