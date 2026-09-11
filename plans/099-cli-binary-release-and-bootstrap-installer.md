# Plan 099: CLI Binary Release and Bootstrap Installer

## Goal

Make prebuilt `stegoeggo` CLI binaries the preferred installation path and provide a one-command bootstrap installer modeled on `eggstack/gregg`, while preserving the repository's manual crates.io publication policy.

Preferred Unix installation:

```bash
curl -fsSL https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh | bash
```

Cargo remains a supported fallback:

```bash
cargo install stegoeggo-cli --locked
```

The binary release pipeline must publish GitHub Release assets only. It must not publish crates, change crates.io release ownership, or make ordinary CI a publication trigger.

## Reference Implementation

Gregg already contains a mature pattern worth adapting:

- `.github/workflows/release-binaries.yml`
- `packaging/install.sh`
- `packaging/install.ps1`
- `docs/installation.md`
- `scripts/release-preflight.sh`
- `scripts/release-check-assets.sh`

Important Gregg behaviors to retain where applicable:

- stable target-to-asset naming;
- SHA-256 sidecars;
- temp-directory download and cleanup;
- checksum verification before execution/install;
- candidate `version` validation before replacement;
- `/usr/local/bin` for root Unix installs and `$HOME/.local/bin` for user installs;
- no silent `sudo`;
- Cargo fallback only for unsupported/missing binary targets, never for checksum or identity failure;
- pinned-version support;
- release-time smoke of `version` and `--help`.

Do not copy daemon/service registration logic from Gregg.

## Scope

In scope:

- Prebuilt CLI binaries attached to GitHub Releases.
- Unix bootstrap installer (`install.sh`).
- Windows PowerShell installer (`install.ps1`) if the supported target matrix includes Windows binary distribution.
- Checksums for every distributed executable.
- Release asset validation.
- Documentation making bootstrap installation primary.
- Local/manual release preflight integration.

Out of scope:

- Automated crates.io publication.
- Package-manager repositories (Homebrew, apt, winget, etc.).
- Code signing/notarization unless separately approved.
- Generic library distribution.
- `stegoeggo-conformance` distribution unless explicitly added later.

## Target Matrix

Start from platforms already exercised by StegoEggo assurance and from Gregg's proven release matrix:

```text
Linux   x86_64   x86_64-unknown-linux-gnu
Linux   aarch64  aarch64-unknown-linux-gnu
macOS   x86_64   x86_64-apple-darwin
macOS   arm64    aarch64-apple-darwin
Windows x86_64   x86_64-pc-windows-msvc
```

Before implementation, confirm each target builds the full distributed CLI feature set and that its image codec dependencies support that target.

For Linux, prefer the same portability strategy as Gregg: build GNU binaries against a conservative glibc floor using `cargo-zigbuild` if StegoEggo's dependency graph supports it cleanly. Reuse Gregg's glibc 2.17 target only after a real build/runtime smoke; do not assume compatibility merely because Gregg supports it.

If a target cannot meet the binary portability requirement without disproportionate complexity, mark it source-install-only and let the installer fall back to Cargo.

## Distributed Feature Set

Decide one canonical feature set for release binaries and keep it consistent across platforms.

Recommended disposition: distribute the CLI with the `signatures` feature enabled so a downloaded binary exposes the complete intended user-facing command set (`keygen`, `sign`, `verify-manifest`) rather than a reduced surface compared with source builds.

Required checks before adopting this:

- measure binary size impact;
- verify all signature dependencies build on the target matrix;
- ensure README/help/docs identify the distributed feature set;
- ensure `cargo install stegoeggo-cli --locked` documentation explains whether the default crates.io build differs.

If consistency is preferred, consider making `signatures` a default feature of the CLI package in the same release that introduces binary distribution. Do not silently ship two materially different CLIs under the same version without documentation.

## Asset Contract

Use version namespace from the GitHub tag and versionless asset filenames, matching Gregg's simple contract:

```text
stegoeggo-x86_64-unknown-linux-gnu
stegoeggo-aarch64-unknown-linux-gnu
stegoeggo-x86_64-apple-darwin
stegoeggo-aarch64-apple-darwin
stegoeggo-x86_64-pc-windows-msvc.exe
```

For each executable publish:

```text
<asset>
<asset>.sha256
```

Also attach:

```text
install.sh
install.ps1   # if Windows binary install is supported
```

Keep this naming contract shared by installer, updater, workflow, documentation, and tests. Do not duplicate target mappings independently in multiple places without tests enforcing equivalence.

## Workstream A: Release Workflow

Add `.github/workflows/release-binaries.yml` based on Gregg's release-only pipeline.

Recommended trigger contract:

- manual `workflow_dispatch` for an existing tag initially;
- optionally tag-triggered build after the workflow has proven stable;
- never publish crates.io packages;
- `contents: write` only where required to attach release artifacts.

Because StegoEggo currently says tags do not create releases, implementation must explicitly update `RELEASING.md` and `AGENTS.md` when this behavior changes. A conservative first implementation is manual dispatch against a maintainer-created `vX.Y.Z` tag/release, preserving maintainer control while automating artifact construction.

Each target job must:

1. check out the exact requested tag;
2. verify workspace/CLI version equals the tag;
3. build release `stegoeggo` with the canonical distributed feature set;
4. stage the stable public asset name;
5. run `<asset> version` and `<asset> --help` natively when possible;
6. run at least one minimal protect/inspect smoke using a small fixture;
7. generate SHA-256 sidecar;
8. upload workflow artifact;
9. aggregate and attach assets only after all required jobs succeed.

## Workstream B: Release Preflight

Adapt Gregg's preflight pattern rather than duplicating shell fragments in YAML.

Add focused scripts such as:

```text
scripts/release-binary-preflight.sh
scripts/release-check-assets.sh
scripts/release-targets.txt
```

The preflight should validate:

- `vX.Y.Z` tag syntax;
- root, carrier, and CLI lockstep versions;
- CLI exact root dependency;
- tag points to checked-out commit;
- working tree/tag assumptions for local use;
- expected release assets/target matrix;
- canonical distributed feature set.

Keep existing `scripts/release-check.sh` responsible for crate publication readiness. Binary artifact checks may call shared version helpers but should not turn that script into an Actions publisher.

## Workstream C: Unix Installer

Add `packaging/install.sh` or `install.sh` source that is attached to each GitHub Release.

Contract:

- require Bash and fail clearly when invoked with `sh`/dash;
- support `--version X.Y.Z` for pinned installs;
- map `uname -s`/`uname -m` to the canonical target list;
- use a private `mktemp -d` and cleanup trap;
- download executable + `.sha256` from `releases/latest/download` or exact version tag;
- verify checksum before any execution;
- run candidate `version` and require `stegoeggo X.Y.Z` identity;
- install to `/usr/local/bin/stegoeggo` when root, otherwise `$HOME/.local/bin/stegoeggo`;
- warn if install directory is absent from `PATH`;
- never edit shell rc files;
- never invoke `sudo` internally;
- if target is unsupported or exact binary asset returns 404, use `cargo install stegoeggo-cli --locked` if Cargo exists;
- checksum retrieval failure, checksum mismatch, candidate identity mismatch, or network failure must be hard failures rather than Cargo fallbacks.

The one-command README path should be exactly one curl pipeline and should not require Rust.

## Workstream D: Windows Installer

If Windows remains a distributed binary target, adapt Gregg's PowerShell installer with StegoEggo-specific simplification:

- detect AMD64 target;
- download `.exe` and checksum;
- verify with `Get-FileHash -Algorithm SHA256`;
- validate `version` output;
- install user-local by default unless an explicitly documented elevated destination is chosen;
- no service/startup logic;
- Cargo fallback only for unsupported/missing assets.

Document PowerShell execution clearly; avoid examples that imply a downloaded script exists locally when it is actually piped directly.

## Workstream E: Installer Tests

Add shell-level tests for target mapping, URL construction, pinned versions, checksum mismatch, unsupported platform fallback, and install destination selection.

Where practical, use local fixture assets/server indirection rather than downloading from live GitHub in ordinary CI.

Suggested tests:

```text
installer_maps_linux_x86_64
installer_maps_linux_aarch64
installer_maps_macos_arm64
installer_pinned_url_uses_exact_tag
installer_checksum_failure_is_fatal
installer_identity_failure_is_fatal
installer_404_can_fallback_to_cargo
installer_network_failure_does_not_fallback
installer_user_destination_is_local_bin
```

Run ShellCheck on installer scripts if available, but do not expand required CI beyond the repository policy without an explicit maintainer decision.

## Documentation Changes

Update:

- `README.md`: make one-curl install the first installation method.
- new `docs/installation.md`: targets, direct download, checksums, pinned versions, Cargo fallback, PATH notes.
- `RELEASING.md`: GitHub binary release procedure and separation from manual crates.io publication.
- `SUPPORT.md`: distributed binary matrix versus compile/test assurance matrix.
- `SECURITY.md`: explain checksum trust boundary and unsigned binaries if applicable.
- `AGENTS.md`: release artifact contract and local verification commands.

## Required Validation

```bash
./scripts/check.sh
bash -n packaging/install.sh
shellcheck packaging/install.sh   # when available
./scripts/release-binary-preflight.sh --help
```

For each supported release target, release workflow must demonstrate:

```text
stegoeggo version
stegoeggo --help
minimal protect smoke
minimal inspect smoke
checksum verification
```

Perform an end-to-end install from a test/prerelease GitHub Release before calling the installer canonical.

## Acceptance Criteria

- A user on a supported platform can install `stegoeggo` without Rust using one curl command.
- Every binary has a verified SHA-256 sidecar.
- Installer validates binary identity/version before installation.
- User-local install works without sudo.
- Unsupported/missing binary targets can fall back to Cargo when available.
- Corrupt or unverifiable assets never trigger an unsafe fallback.
- GitHub binary publication does not publish crates.io packages.
- Release/docs/support files accurately distinguish crate publication from binary release artifacts.
- Asset naming is stable and shared with Plan 100 self-update.

## Dependency / Ordering

Implement after or alongside Plan 098. Complete the release asset contract before Plan 100 self-update, because update correctness depends on deterministic asset names and checksums.
