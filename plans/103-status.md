# Plan 103 Status: Native Eggfetch Updater Transport and Rust 1.89 Baseline

Status: complete. Implementation landed; qualification continues in Plan 104.

## Objective

Migrate only the `stegoeggo-cli` self-updater from external `curl` to
published `eggfetch-core 0.2.x`, while preserving the current crates.io
version authority, GitHub Release asset contract, checksum/candidate
verification, narrow Cargo fallback policy, and self-replacement semantics.

The same implementation raises the repository MSRV from Rust 1.87 to Rust
1.89 because `eggfetch-core 0.2.0` requires Rust 1.89.

## Implementation evidence

- Resolved dependency: `eggfetch-core 0.2.0` (plus `eggfetch-http-connect
  0.2.0` via the `proxy` feature) in `Cargo.lock`; `tokio 1.52.3` unified
  through eggfetch's `rt, net, time, sync, macros, io-util` requirements
  with the CLI's direct `tokio { version = "1", features = ["rt"] }`.
- Enabled eggfetch features: `http1`, `tls-rustls`, `tls-native-roots`,
  `proxy` (and their implied `high-level-url`, `redirects`,
  `logical-retry`, `basic-auth`, `native-http1`, `standard-route`,
  `transport-http1`). Absent unless forced: HTTP/2, HTTP/3, compression,
  cookies, multipart, JSON, tracing.
- Root `stegoeggo` and `stegoeggo-stego` do not depend on eggfetch
  (`cargo tree -p stegoeggo` shows no eggfetch edge; the new dependency is
  declared only in `stegoeggo-cli/Cargo.toml`).
- MSRV declarations: `rust-version = "1.89"` in root, carrier, and CLI
  manifests; `assurance.yml` MSRV job pins `1.89.0`;
  `scripts/validate-msrv-package.sh` defaults `MSRV=1.89`; `SUPPORT.md`,
  `README.md` badge/text, `RELEASING.md`, `AGENTS.md`,
  `architecture/overview.md`, and `architecture/tooling.md` state 1.89.
  `cargo +1.89.0 check` passes for carrier, library (default, minimal,
  all-features), and CLI (default, all-features); `cargo +1.89.0 test`
  passes for carrier and library; `MSRV=1.89
  ./scripts/validate-msrv-package.sh` passes.
- Updater loopback tests: 14 native transport tests in
  `stegoeggo-cli/src/update.rs` cover registry 200 resolution, asset and
  sidecar 200 writes, asset 404 fallback, sidecar 404 hard failure, 500
  hard failure, timeout hard failure, oversized registry/sidecar/asset
  rejection, redirect following, strict HTTPS-downgrade policy,
  explicit proxy environment (precedence, NO_PROXY, fail-closed invalid,
  credential redaction), and transport-error hard failure. All pass under
  `cargo test -p stegoeggo-cli --all-features`.
- `./scripts/test-release-updater.sh` passes through the native transport
  (fixture rewritten to use a newer-than-current version so the download
  path is actually exercised, and to stage all five target assets so the
  rehearsal is robust to uname/Rust target-name skew).
- `cargo deny check licenses` and `cargo deny check advisories` pass with
  no policy change.
- `rg -n 'curl|CURL_' stegoeggo-cli/src/update.rs` returns no matches;
  installer scripts still use curl/PowerShell by design.

## Behavior delta from curl

- Timeouts: explicit `Timeout::builder().connect(10s).total(60s)` replaces
  `curl --connect-timeout 10 --max-time 60`; no per-chunk read timeout in
  either implementation.
- Redirects: `RedirectPolicy::strict(10)` follows up to 10 hops and denies
  HTTPS-to-HTTP downgrade before second-hop I/O; curl followed redirects
  without downgrade denial.
- Proxy: `ProxyEnvironment::from_env()` snapshot applied explicitly with
  fail-closed invalid configuration; curl honored proxy variables
  implicitly.
- Body limits: per-request `max_decoded_body_size` bounds (4 MiB registry,
  8 KiB sidecar, 64 MiB executable) plus a 64 MiB client ceiling replace
  unbounded curl output files.
- Errors: curl-specific `CurlUnavailable`/`CurlFailed`/`InvalidHttpStatus`
  variants replaced by `RequestBuild`/`Proxy`/`Transport`/`BodyTooLarge`
  while `HttpStatus { url, status }` and the 404-only fallback rule are
  unchanged.
- `run_bounded()` retained for `cargo` and candidate `version`
  subprocesses with identical bounds.

## Closeout checks

- No updater runtime path invokes curl.
- Proxy environment handling is explicit.
- Connect (10s) and total (60s) deadlines are finite.
- HTTPS downgrade redirects are denied.
- Registry, sidecar, and executable responses are bounded.
- Only exact executable-asset HTTP 404 can trigger network-path Cargo
  fallback; sidecar 404 and all other statuses and transport failures are
  hard failures.
- Checksum, candidate identity/version, and replacement validation are
  unchanged in strength.
- The current executable is left untouched on every pre-replacement
  failure (destination preflight runs before download; replacement is the
  final step).

## Handed to Plan 104

- Before/after binary-size measurement.
- Five-target build qualification and glibc floor evidence.
- Controlled live-network smoke against crates.io/GitHub.
- Installer/updater rehearsal through the new architecture.
- Final documentation reconciliation.
