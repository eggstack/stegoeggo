# Releasing

This document describes the manual release procedure for stegoeggo crates and
the separate manual workflow for attaching CLI binaries to GitHub Releases.

## Release Cadence

All three crates share one version and are released in workspace lockstep
(Plan 097 disposition, retained): atomic parent/carrier compatibility and
manual-release simplicity outweigh unrelated version bumps. The root depends
on the carrier via an exact `=X.Y.Z` specifier plus hidden
`application-support`, whose surface only makes sense in lockstep. No
independent carrier releases are made for rights/CLI-only changes.

## Release Ownership

- Release cadence is a maintainer decision.
- Releases are performed manually using direct Cargo/crates.io commands.
- GitHub Actions do not publish crates or create releases automatically, and
  Actions has no crates.io publication credentials.
- Version tags do not publish crates.
- GitHub binary releases are manual but are a supported CLI distribution
  contract. The manually dispatched binary workflow attaches the complete
  target matrix, checksums, and installers to an existing release.
- CI success is useful development evidence but not a publication trigger.

The CLI updater uses the stable `stegoeggo-cli` crates.io version as its sole
version authority. A release intended for updates must therefore publish the
carrier, library, and CLI crates in order before its matching `vX.Y.Z` GitHub
Release is made available. The binary workflow must attach every target asset
and its `.sha256` sidecar before that release is considered updater-ready.

Python wheels follow the same manual publication policy. The
`.github/workflows/release-python.yml` workflow is **manually dispatched**;
it never publishes to PyPI, has no PyPI credentials in CI, and only uploads
the built wheels as GitHub Actions artifacts for maintainer inspection.
PyPI publication (if ever performed) remains an explicit local maintainer
step performed after the artifact rehearsal is audited. The Python package
version tracks the StegoEggo source version but lives outside the
carrier → library → CLI crates.io publication chain.

## Immutable Crates.io Versions

Once crates.io accepts a package version, its bytes cannot be replaced. Key implications:

- Yanking does not make a version reusable.
- Deleting or moving a Git tag does not make a version reusable.
- Documentation-only defects in a published crate require a new version.
- If a release attempt partially succeeds, do not republish the already accepted crate version.
- Select the first unused version greater than every published version for the package.

## Build Configuration

The workspace release profile is size-optimized:

```toml
[profile.release]
lto = true
strip = "symbols"
codegen-units = 1
panic = "abort"
opt-level = "s"
```

The CLI binary enables its `signatures` package feature by default (no direct
`image` dependency). This adds `stegoeggo/signatures` +
`stegoeggo/detached-manifest` for `keygen`/`sign`/`verify-manifest`:

```toml
stegoeggo = { path = "..", version = "=X.Y.Z" }
```

The conformance binary requires the `conformance` feature:

```bash
cargo build --release --bin stegoeggo-conformance --features conformance
```

## CI Evidence: Blocking vs Scheduled

Only one CI signal blocks development: the standard `Check` job in
`.github/workflows/ci.yml` (stable Rust, Linux x86_64, `./scripts/check.sh`).
Everything else is non-blocking signal for the maintainer's judgment:

- `Assurance` (weekly + manual dispatch): MSRV 1.89 matrix and stable
  compile+tests on Linux aarch64, macOS aarch64, and Windows x86_64.
- `External Verification` (monthly + manual dispatch): ExifTool/xmllint
  conformance signal.
- `Fuzz` smoke (weekly rotating subset + manual single-target dispatch):
  parser robustness signal with crash-artifact upload.

Scheduled failures are investigated as signal, never as publication triggers.
Release readiness is established by the local checks in Pre-Release
Preparation below, not by green scheduled runs. See `SUPPORT.md` for the exact
evidence matrix. No workflow publishes crates or reacts automatically to tags.

## Pre-Release Preparation

1. Confirm a clean working tree (`git status` shows no uncommitted changes).
2. Select an unused version greater than every published version on crates.io.
3. Update the carrier crate version in `stegoeggo-stego/Cargo.toml`.
4. Update the library version in the workspace root `Cargo.toml`.
5. Update the CLI version in `stegoeggo-cli/Cargo.toml`.
6. Update the CLI exact library dependency in `stegoeggo-cli/Cargo.toml` (`stegoeggo = { path = "..", version = "=X.Y.Z" }`).
7. Update `CHANGELOG.md` with the new version and release date.
8. Update `SECURITY.md` supported versions table if the release line changes.
9. Run `./scripts/release-check.sh --stage=pre --allow-dirty` before publishing.
10. Run targeted specialist checks appropriate to the changes (see table below).
11. Inspect package contents with `cargo package -p stegoeggo --list` and `cargo package -p stegoeggo-cli --list`.
12. Verify no publication command is being run by automation.

## Targeted Specialist Checks

Run checks applicable to the release contents. This is not a universal checklist.

| Change Type | Additional Check |
|---|---|
| Dependency or MSRV change | `./scripts/validate-msrv-package.sh` |
| docs.rs or public docs configuration change | `./scripts/validate-docs-rs.sh` |
| Metadata serialization, parser, or fixture change | `./scripts/verify_metadata_conformance.sh --strict` |
| Public API compatibility-sensitive release | `cargo semver-checks check-release` |
| Dependency or release preparation | `cargo deny check licenses` and `cargo deny check advisories` |
| Parser or untrusted-input change | `cargo +nightly fuzz run <target> -- -max_total_time=60` |
| Performance-sensitive hot path | `cargo bench` |

## Publication Sequence

Publication must follow dependency order: carrier first, then library, then CLI. Each crate's exact dependency must resolve from crates.io before the next crate is published.

```bash
# Before publication, the release check runs:
./scripts/release-check.sh --stage=pre

# 1. Publish the carrier crate
cargo publish -p stegoeggo-stego --dry-run
cargo publish -p stegoeggo-stego

# 2. Confirm the carrier version is available on crates.io
cargo search stegoeggo-stego

# 3. Verify the library against the now-published carrier, then publish it
./scripts/release-check.sh --stage=root
cargo publish -p stegoeggo --dry-run
cargo publish -p stegoeggo

# 4. Confirm the library version is available on crates.io
cargo search stegoeggo

# 5. Verify the CLI against the now-published library, then publish it
./scripts/release-check.sh --stage=cli
cargo publish -p stegoeggo-cli --dry-run
cargo publish -p stegoeggo-cli
```

Do not prescribe a fixed sleep between publications. Registry propagation should be confirmed, not guessed with a timed delay.

**Package verification stages:** `./scripts/release-check.sh --stage=pre` requires full carrier verification and structurally lists the unpublished root and CLI packages (their exact crates.io dependencies cannot be resolved locally before publication). After the carrier is published, `--stage=root` performs full root verification. After the root is published, `--stage=cli` performs full CLI verification. The script never publishes crates.

## GitHub Release and binary assets

After successful crates.io publication:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

- A binary release requires an exact `vX.Y.Z` tag pointing to the published
  source commit.
- Do not force-move the tag after publication.
- Dispatch the Eggpack-generated `.github/workflows/release-binaries.yml`
  with the exact `vX.Y.Z` tag. The workflow checks out that tag, resolves the
  release identity from `release/eggpack/distribution.toml`, builds the five
  targets, qualifies each exact candidate natively, runs the StegoEggo
  product validator (`scripts/smoke-release-binary.py`, including the Linux
  GLIBC_2.17 ceiling), and stages a draft GitHub Release. The draft is
  created or reconciled, never clobbered: reruns reuse exact matching assets
  and fail closed on same-name/different-digest.
- A maintainer inspects the complete draft (including the staging receipt)
  and explicitly publishes it. Neither the workflow nor Eggpack publishes the
  draft.
- Binary staging does not publish any crate and does not replace the
  carrier → library → CLI crates.io sequence.
- Run `./scripts/release-binary-preflight.sh --tag=vX.Y.Z` before dispatch and
  `./scripts/release-check-assets.sh --dir=<downloaded-assets>` when auditing
  a completed release.
- The asset audit requires exactly the 15-file Eggpack inventory: the five
  versionless executables, one correctly named `.sha256` sidecar per
  executable, `install.sh`, `install.ps1`, the generated `install-exact.sh` /
  `install-exact.ps1`, and `release-manifest.json`; it rejects extra or
  misnamed files.
- Verify that `stegoeggo X.Y.Z` from every attached executable matches the tag;
  the updater performs the same candidate identity/version check.
- A CLI release is not updater-ready until all five executable assets, their
  `.sha256` sidecars, `install.sh`, and `install.ps1` are attached. This is the
  contract behind the stable `releases/latest/download/install.sh` URL.
- A crates-only/library release may omit binary assets, but it must not be
  presented as a binary CLI release or as an updater target.

## Python Wheel Artifacts

The Python binding (`bindings/python/`) is governed by the same manual-only
policy as the rest of the release surface. It is not part of the
carrier → library → CLI crates.io chain and is published independently
(if at all). The end-to-end process is:

1. Confirm the binding version in `bindings/python/pyproject.toml`,
   `bindings/python/Cargo.toml`, and `bindings/python/Cargo.lock` matches
   the StegoEggo source release version (0.4.2 by default).
2. Dispatch `.github/workflows/release-python.yml` manually with the
   source ref. The workflow:
   - Builds `cp311-abi3` wheels on native runners for the documented
     five-target matrix using `cibuildwheel` (Linux x86_64 and aarch64
     on `ubuntu-24.04` / `ubuntu-24.04-arm` with Rust 1.89 provisioned
     inside the build container via `before-all`, macOS x86_64 on
     `macos-15-intel`, macOS arm64 on `macos-14`, Windows x86_64 on
     `windows-2022`). Linux wheels use the explicit `manylinux_2_28`
     compatibility floor.
   - Runs a per-platform install + import + protect + verify smoke
     step on a runner whose native architecture matches the wheel tag.
   - Builds one sdist in a dedicated job via `maturin sdist`, then
     installs the tarball through a literal `pip install <tarball>` in
     a fresh venv that does not have the source checkout on `sys.path`,
     and runs an import + protect + verify smoke against the installed
     package.
3. Download the artifacts, audit the wheel filenames and sdist against
   `plans/subsystems/language-bindings-roadmap.md#m003--python-corrective-qualification`,
   and confirm each smoke step succeeded on its native architecture. No
   wheel is treated as qualified by the build job alone — the matching
   native smoke run must also pass.
4. Record the artifact identifiers, the workflow run URL/ID, the exact
   implementation SHA, and the per-platform native smoke outcomes in
   `plans/closure/language-bindings/003-status.md`. Update `SUPPORT.md`
   only when the platform's native smoke evidence is recorded (the
   default wording is "Configured"; promote individual rows to
   "Qualified" as the evidence arrives).
5. Maintainer-only PyPI publication (separate from the GitHub workflow)
   is performed locally with `twine upload <dist/*>` after the wheel
   set is validated. Add the new version with `--skip-existing` if a
   prior attempt partially landed.
6. If a published PyPI version is defective, publish a new unused version
   with the fix; yanking does not make a version reusable.

The Python distribution's `requires-python = ">=3.11"`, free-threaded
CPython support, and PyPy qualification are explicitly deferred per the
language-bindings subsystem roadmap. Every wheel platform must clear its
native smoke run before any maintainer-published PyPI release.

## Node Native Artifacts

The Node binding (`bindings/node/`) is governed by the same manual-only
policy as the rest of the release surface. No npm publication occurs in
M005: there are no npm credentials in Actions, no registry publish step in
any workflow, and `napi pre-publish` is never invoked (its normal behavior
has registry and release side effects). The end-to-end process is:

1. Confirm the binding version in `bindings/node/package.json`,
   `bindings/node/Cargo.toml`, and `bindings/node/Cargo.lock` matches
   the StegoEggo source release version (0.4.2 by default).
2. Dispatch `.github/workflows/release-node.yml` manually with the
   source ref. The workflow:
   - Builds release addons on native runners for the documented
     five-target matrix with the napi-rs CLI (Linux x86_64 and aarch64
     on `ubuntu-24.04` / `ubuntu-24.04-arm` with `--use-napi-cross` so
     the addon targets the glibc 2.17 build floor instead of the
     runner's newer glibc, macOS x86_64 on `macos-15-intel`, macOS
     arm64 on `macos-14`, Windows x86_64 on `windows-2022`).
   - Runs a per-platform import + protect + verify smoke
     (`scripts/smoke.mjs`) on a runner whose native OS/architecture
     matches the addon before uploading it. Linux x86_64 additionally
     smokes Node 22, 24, and 26.
   - Collects all five artifacts in an assembly job with
     `napi create-npm-dirs` / `napi artifacts`, audits the
     per-platform `os`/`cpu`/`libc` metadata and the root loader
     mapping, creates a local pack tarball, and performs a clean local
     install/load smoke on the collector platform (standing in the
     registry-provided platform package explicitly, since
     registry-install selection cannot be exercised without a real
     registry).
3. Download the artifacts, audit the `.node` filenames and platform
   package metadata against
   `plans/subsystems/language-bindings-roadmap.md`, and confirm each
   smoke step succeeded on its native architecture. No addon is treated
   as qualified by the build job alone — the matching native smoke run
   must also pass.
4. Record the artifact identifiers, the workflow run URL/ID, the exact
   implementation SHA, and the per-platform native smoke outcomes in
   `plans/closure/language-bindings/005-status.md`. Update `SUPPORT.md`
   only when the platform's native smoke evidence is recorded (the
   default wording is "Configured"; promote individual rows to
   "Qualified" as the evidence arrives).
5. Do not run `napi artifacts` locally without all five target
   artifacts present: it fails closed on a partial set and deletes an
   already-collected local `.node`. Rebuild locally with `pnpm build`
   if the local addon goes missing.

A future first npm publication (outside M005) must account for napi
multi-platform publication being non-atomic over immutable versions:
a partial platform set cannot be repaired by republishing the same
version, so the release failure policy must be explicit before any
registry action is attempted.

## Partial Failure Handling

### Carrier publishes, library fails before acceptance

- The carrier version is consumed on crates.io.
- Determine whether the library can be corrected and published under the intended version without changing the already published carrier contract.
- If source changes require a different exact dependency or synchronized version policy, increment both versions appropriately.
- Never attempt to overwrite the carrier.

### Library publishes, CLI fails before acceptance

- The library version is consumed on crates.io.
- Determine whether the CLI can be corrected and published under the intended version without changing the already published library contract.
- If source changes require a different exact dependency or synchronized version policy, increment both versions appropriately.
- Never attempt to overwrite the library.

### All publish, docs.rs fails

- Fix the source or docs configuration.
- Select a new unused patch version.
- Republish in dependency order (carrier → library → CLI).
- Optionally yank the defective version, understanding that it remains consumed.

### Dry-run fails

- Fix locally. No version is consumed until crates.io accepts publication.
- Rerun the dry-run and release check.

### Tag created before publication failure

- Do not treat the tag as proof of publication.
- Correct repository history carefully before public reliance, but never use tag movement to imply a published crate was replaced.
- Prefer publishing first, tagging second.
