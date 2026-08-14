# Releasing

This document describes the manual release procedure for stegoeggo crates.

## Release Ownership

- Release cadence is a maintainer decision.
- Releases are performed manually using direct Cargo/crates.io commands.
- GitHub Actions do not publish crates or create releases.
- Version tags do not publish crates.
- GitHub releases are optional and manual.
- CI success is useful development evidence but not a publication trigger.

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

The CLI binary enables three optional library features via its `stegoeggo` dependency:

```toml
stegoeggo = { path = "..", version = "=X.Y.Z", features = ["iscc", "conformance", "parallel"] }
```

The conformance binary requires the `conformance` feature:

```bash
cargo build --release --bin stegoeggo-conformance --features conformance
```

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

## Optional Tag and GitHub Release

After successful crates.io publication:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

- Tagging is optional but recommended for repository history.
- The tag must point to the published source commit.
- Do not force-move the tag after publication.
- A GitHub release may be created manually from the tag.
- Do not attach a single Linux binary unless the maintainer intentionally supports it as a distributed artifact.
- Source releases and crates.io publication do not require an automated binary artifact.

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
