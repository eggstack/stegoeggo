# Plan 105: First Eggfetch-Enabled Release and Five-Target Qualification

## Goal

Ship the first public StegoEggo CLI release that contains the Plan 103 native
`eggfetch-core` updater, and use that release to close the remaining
distribution evidence gap from Plan 104.

This plan is release/qualification work, not a redesign of the updater.

The next release should use the next unused workspace version. If no other
release lands first, that is expected to be `0.4.2`; do not hard-code
`0.4.2` into scripts or documentation if the repository version advances
before execution.

## Why This Plan Exists

Plans 103-104 established that the native updater:

- uses `eggfetch-core 0.2.0` in `stegoeggo-cli` only;
- preserves crates.io as stable-version authority;
- preserves exact GitHub Release asset naming and SHA-256 verification;
- preserves the unsupported-target / exact executable-404 Cargo fallback rule;
- uses Rust 1.89 as the repository MSRV;
- passes local deterministic updater tests and the normal CI gate;
- builds locally for Linux x86_64/aarch64 and macOS x86_64/aarch64;
- preserves the Linux glibc 2.17 floor.

The remaining gap is public release evidence for the new dependency graph,
especially the native Windows build and the exact five-target release workflow.

The currently published `v0.4.1` binaries predate the eggfetch migration and
still contain the curl-based updater, so they cannot serve as release evidence
for Plan 103.

## Scope

In scope:

- select and bump to the next unused synchronized workspace version;
- update changelog/release notes for the eggfetch migration and Rust 1.89 MSRV;
- run the full release checks and staged crates.io publication sequence;
- create the exact matching Git tag and GitHub Release;
- dispatch the real five-target `release-binaries.yml` workflow;
- require successful native Windows x86_64 build/smoke;
- verify both Linux release artifacts still target glibc 2.17;
- audit the attached five binaries, five checksum sidecars, and two installers;
- install at least one Unix artifact through the public bootstrap URL;
- verify the released CLI contains the native updater and no runtime curl
  requirement;
- verify `stegoeggo update` on the new release reports current cleanly while
  it is the latest stable version;
- correct stale wording that says destination replaceability is checked before
  *any* download;
- record complete release evidence in `plans/105-status.md`.

Out of scope:

- producing an artificial extra release solely to test updater replacement;
- changing release authority or asset naming;
- changing updater transport/features;
- changing installer implementation;
- adding release targets;
- changing the SHA-256 trust model;
- code signing/notarization;
- automatic crates.io publication.

The first real eggfetch-to-eggfetch replacement is Plan 106.

## Workstream A: Correct the Preflight Contract Wording

The implementation sequence is intentionally:

```text
fetch crates.io stable version
  -> compare current vs latest
  -> if newer: verify current executable destination is replaceable
  -> resolve target
  -> download release executable and sidecar
```

The registry lookup must happen before replacement preflight because the CLI
should not reject an unwritable installation merely to report that it is
already current.

Review current documentation and change wording such as:

```text
check the current executable's destination before any download
```

to the accurate contract:

```text
check the current executable's destination before downloading any update
artifact
```

or equivalent.

Review at minimum:

- `AGENTS.md`;
- `architecture/cli.md`;
- `docs/installation.md`;
- `SECURITY.md`;
- `STABILITY.md`;
- `SUPPORT.md`.

Do not change the code merely to satisfy stale wording. The current
registry-first / asset-preflight sequence is the intended behavior.

Add or adjust a focused test if needed to make the distinction explicit:

- current == latest may succeed without destination replaceability;
- current < latest must fail destination preflight before the release
  executable or checksum is requested.

## Workstream B: Version Selection and Release Preparation

Determine the next unused version from:

- workspace manifests;
- crates.io;
- Git tags;
- GitHub Releases.

Expected next patch if nothing else intervenes:

```text
0.4.2
```

All three published crates remain lockstep:

```text
stegoeggo-stego
stegoeggo
stegoeggo-cli
```

Update:

- root `Cargo.toml`;
- `stegoeggo-stego/Cargo.toml`;
- `stegoeggo-cli/Cargo.toml`;
- exact intra-workspace `=X.Y.Z` dependencies;
- `Cargo.lock`;
- `CHANGELOG.md`;
- any release-facing version examples that are intended to track the current
  release.

The release notes should explicitly mention:

- native self-update transport now uses `eggfetch-core`;
- external curl is no longer required by `stegoeggo update`;
- bootstrap shell installer may still use curl;
- repository MSRV is Rust 1.89;
- updater now has strict HTTPS downgrade denial and bounded response bodies.

Do not overstate SHA-256 sidecars as publisher signatures.

## Workstream C: Pre-Publication Verification

Run the full normal and release-specific verification before consuming a
version:

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
./scripts/release-check.sh --stage=pre
```

Also verify the updater dependency graph:

```bash
cargo tree -p stegoeggo-cli -e features
```

Confirm no unintended HTTP/2, HTTP/3, compression, cookie, multipart, JSON, or
tracing feature was introduced since Plan 104.

## Workstream D: Manual Crates.io Publication

Preserve the repository's manual staged publication contract.

Publish in exact order:

```text
stegoeggo-stego -> stegoeggo -> stegoeggo-cli
```

Use the existing `RELEASING.md` staged checks and dry-runs.

Required sequence:

1. run pre-stage verification;
2. publish carrier;
3. confirm the exact carrier version is visible from crates.io;
4. run root-stage verification;
5. publish root library;
6. confirm the exact root version is visible;
7. run CLI-stage verification;
8. publish CLI;
9. confirm the exact CLI stable version is visible through the crates.io API.

Do not use a fixed sleep in place of checking registry propagation.

Do not tag the release until all intended crates are successfully published.

## Workstream E: Tag and GitHub Release

After crates.io publication succeeds:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

Create the GitHub Release from that immutable tag.

Run:

```bash
./scripts/release-binary-preflight.sh --tag=vX.Y.Z
```

The tag must point exactly to the source commit containing:

- the eggfetch updater;
- Rust 1.89 declarations;
- the release version bump;
- the finalized release documentation.

Do not force-move the tag.

## Workstream F: Real Five-Target Release Workflow

Dispatch `.github/workflows/release-binaries.yml` for the exact new tag and
require every matrix entry to pass:

```text
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
x86_64-apple-darwin
aarch64-apple-darwin
x86_64-pc-windows-msvc
```

This workflow run is the missing native Windows qualification for the
eggfetch/Rustls/ring dependency graph.

For each target capture:

- workflow job URL;
- runner;
- successful release build;
- `stegoeggo X.Y.Z` smoke;
- `--help` smoke;
- protect/inspect smoke;
- generated checksum sidecar.

Do not treat the old v0.4.1 Windows artifact as evidence for the new
dependency graph.

If Windows fails, diagnose the actual dependency/toolchain problem and keep
Plan 105 open. Do not drop the Windows target or change the support matrix
without a separate plan.

## Workstream G: Linux ABI Verification

Download the newly produced Linux artifacts and verify the glibc floor remains
2.17.

Check both:

```text
stegoeggo-x86_64-unknown-linux-gnu
stegoeggo-aarch64-unknown-linux-gnu
```

Use the existing release/ABI inspection method and record the maximum referenced
GLIBC symbol version in `plans/105-status.md`.

Expected maximum:

```text
GLIBC_2.17
```

Any higher runtime floor is a release blocker until understood.

## Workstream H: Public Asset Audit

Download the attached release assets and run:

```bash
./scripts/release-check-assets.sh --dir=<downloaded-assets> --version=X.Y.Z
```

Require exactly:

- five executable assets;
- five matching `.sha256` files;
- `install.sh`;
- `install.ps1`.

Verify the public assets' first-line version identity matches the tag.

Where practical, record release binary sizes to confirm the observed Plan 104
eggfetch footprint is reflected in production artifacts rather than a local
build anomaly.

## Workstream I: Public Installer Smoke

On at least one supported Unix host, exercise the actual public latest installer
after the release is complete:

```bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh | bash
```

Prefer an isolated temporary HOME/PATH or disposable environment rather than
overwriting the maintainer's primary installation.

Verify:

```bash
stegoeggo version
stegoeggo --help
stegoeggo protect <fixture> ...
stegoeggo inspect <output>
stegoeggo verify <output>
```

Confirm the installed version is the new release.

If a Windows environment is available, also exercise `install.ps1`; the
release workflow's native Windows binary build is mandatory even if installer
smoke must be recorded separately.

## Workstream J: Released Native-Updater Smoke

From the newly released binary, run:

```bash
stegoeggo update
```

while that release is the current crates.io stable version.

Expected behavior:

- crates.io query succeeds through embedded eggfetch;
- the command reports the installation is current;
- no release asset is downloaded;
- no replacement is attempted;
- no external curl executable is required.

Where practical, run with curl absent from PATH to prove the installed update
command is independent of curl. Do not break the bootstrap installer test to do
this; this check applies to the already-installed CLI.

Also verify a malformed proxy environment fails closed without exposing proxy
credentials.

This does not replace Plan 106's real public A->B updater test.

## Workstream K: Record Release Evidence

Populate `plans/105-status.md` with:

- release version;
- source commit SHA;
- crates.io URLs/confirmation for all three crates;
- Git tag and GitHub Release URL;
- five-target workflow run URL;
- per-target job conclusions;
- Windows native build result;
- Linux glibc floor result;
- downloaded asset audit result;
- public installer smoke environment/result;
- installed `stegoeggo update` current-version result;
- any release-specific correction made;
- explicit statement that real eggfetch-to-eggfetch A->B replacement remains
  Plan 106 if a second native-updater release does not yet exist.

## Acceptance Criteria

Plan 105 is complete when:

- the next stable lockstep version is published for carrier/library/CLI;
- that version contains the eggfetch updater and Rust 1.89 baseline;
- the exact Git tag and GitHub Release exist;
- the real five-target release workflow succeeds for the new source, including
  Windows x86_64;
- both Linux artifacts retain the glibc 2.17 floor;
- all required release assets and sidecars pass the asset audit;
- the public Unix installer installs the new version successfully;
- the released binary's `update` command reaches crates.io through eggfetch
  and cleanly reports current when no newer version exists;
- updater operation does not require curl after installation;
- documentation accurately says destination preflight occurs before update
  asset download, not before the crates.io version lookup;
- `plans/105-status.md` contains reproducible release evidence.

## Failure Policy

Do not hide release failures with local-only evidence.

- A Windows matrix failure keeps this plan open.
- A raised Linux glibc floor keeps this plan open.
- Missing/misnamed release assets keep this plan open.
- Failure of the published CLI's eggfetch updater keeps this plan open.
- Do not force-move a release tag.
- Do not republish an already consumed crates.io version.
- Do not weaken TLS, proxy, redirect, body-limit, checksum, candidate, or
  fallback behavior to make the release pass.

## Ordering

Plan 105 follows Plans 103-104.

Plan 106 follows after there are two stable public releases whose binaries both
contain the native eggfetch updater.
