# Plan 100: CLI Version and Self-Update

## Goal

Add first-class `version` and `update` commands to `stegoeggo`, using the binary release contract from Plan 099 and adapting the proven Gregg self-update mechanics without importing Gregg-specific daemon abstractions.

Target UX:

```bash
stegoeggo version
stegoeggo update
```

`version` must be deterministic, offline, and suitable for installer/release validation. `update` must prefer verified prebuilt GitHub Release assets, fall back to Cargo only under narrowly defined conditions, and never invoke `sudo` internally.

## Reference Implementation

Gregg's internal `gregg-update` crate already separates the useful mechanics from service-manager policy:

- stable version discovery/comparison;
- target detection;
- deterministic asset naming/URL construction;
- bounded downloads;
- SHA-256 verification;
- candidate identity/version validation;
- staged executable replacement;
- Cargo fallback;
- no implicit privilege escalation.

StegoEggo has one updatable executable, so do not create a generic shared crate unless implementation experience proves the module is independently useful. Prefer a focused CLI module, likely:

```text
stegoeggo-cli/src/update.rs
```

with small supporting modules only if necessary.

## Scope

In scope:

- `stegoeggo version`.
- `stegoeggo update`.
- Stable version resolution.
- Host target detection shared with the release asset contract.
- Binary download and checksum validation.
- Candidate `version` validation before replacement.
- Self replacement on Unix/Windows where supported.
- Cargo fallback under explicit rules.
- Clear permission/network/integrity diagnostics.
- Unit/integration tests.

Out of scope:

- Automatic background update checks.
- Update telemetry.
- Privilege escalation.
- Package-manager-specific updates.
- Updating library crates independently.
- Daemon/service restart behavior.

## Workstream A: `version` Command

Add an explicit command that prints a stable, parseable first line:

```text
stegoeggo X.Y.Z
```

Optional additional build information may be added only after the first line and should not make installer validation fragile. Avoid timestamps or other non-reproducible values unless deliberately injected by the release build.

Support clap's standard `--version` as well if it is already free/appropriate, but keep the command because the installer/updater contract should not depend on clap formatting internals.

Tests:

```text
version_command_matches_package_version
version_first_line_is_stable
version_requires_no_network_or_config
```

## Workstream B: Version Authority

Choose one stable release authority and document it.

Recommended model, matching Gregg:

- crates.io `stegoeggo-cli` stable version is the release-version authority;
- the exact matching `vX.Y.Z` GitHub Release is the binary source;
- update only when the registry reports a newer stable CLI version;
- prereleases are ignored by default.

This fits the workspace's existing manual publication sequence: carrier -> library -> CLI. It also prevents a manually drafted GitHub Release from becoming an update source before the corresponding CLI crate is published.

If implementation instead uses the GitHub Releases API as the authority, document how prereleases/drafts/tags are filtered and ensure it cannot select an unpublished or mismatched crate release. Do not mix two authorities opportunistically.

## Workstream C: Target and Asset Resolution

Share the exact target/asset contract with Plan 099.

Supported target examples:

```text
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
x86_64-apple-darwin
aarch64-apple-darwin
x86_64-pc-windows-msvc
```

Asset function:

```text
stegoeggo-<target>[.exe]
```

Checksum function:

```text
stegoeggo-<target>[.exe].sha256
```

Do not maintain an updater-only target table that can drift from the release workflow/installer. Either generate/read a common source of truth or add tests that assert exact equivalence.

## Workstream D: Download and Verification

Adapt Gregg's conservative fallback rules:

1. Resolve latest stable version.
2. If current >= latest, print that the installation is current and exit 0.
3. Check the current executable can be replaced before downloading anything.
4. Detect target.
5. Download the exact tagged asset and checksum into a private temporary directory.
6. Verify SHA-256.
7. Make the staged candidate executable where required.
8. Run staged candidate `version`.
9. Require exact identity (`stegoeggo`) and exact expected version.
10. Replace the current executable.

Hard failures that must never silently fall back to Cargo:

- checksum file exists but is malformed;
- checksum mismatch;
- downloaded candidate fails to execute;
- identity mismatch;
- version mismatch;
- generic network/TLS/DNS failure;
- server errors other than the explicitly supported missing-asset case.

Cargo fallback is permitted only when:

- host target is intentionally source-only; or
- exact expected GitHub binary asset is absent (HTTP 404).

## Workstream E: HTTP/Process Strategy

Gregg deliberately uses external `curl` plus a small dependency set rather than embedding a full HTTP client. Evaluate the same choice here.

Preferred starting point: reuse the Gregg approach because the bootstrap installer already assumes common Unix download tooling and it avoids adding a large HTTP/TLS dependency tree to an image-processing CLI.

Requirements if using external `curl`:

- detect it explicitly and report a clear error;
- set connect/overall timeouts;
- fail on HTTP errors;
- distinguish 404 from other failures;
- never construct shell command strings from untrusted input; use `Command` arguments;
- bound subprocess execution;
- capture enough stderr for actionable diagnostics without dumping excessive output.

Cargo fallback similarly requires explicit Cargo discovery and bounded execution.

If a native HTTP client is chosen instead, justify binary-size/dependency/MSRV impact in closeout notes.

## Workstream F: Replacement Semantics

Adapt Gregg's `self-replace` strategy or equivalent proven mechanism.

Requirements:

- resolve the actual running executable path;
- reject unwritable destination before download;
- stage before replacement;
- preserve executable permissions on Unix;
- handle Windows running-image replacement correctly;
- never invoke `sudo`;
- leave the existing executable untouched if staging/verification fails;
- report permission failures with the exact path and a suggested manual reinstall/update command.

For an installation under `/usr/local/bin` owned by root, `stegoeggo update` run unprivileged should fail cleanly and instruct the user to rerun with appropriate privileges or use the installer. It must not elevate itself.

## Workstream G: Cargo-Installed Binary Behavior

Self-update may replace a binary originally installed through Cargo. Document that this converts that executable path to a binary-managed installation until the user later runs `cargo install` again.

Consider detecting common Cargo locations only for messaging, not for changing update behavior. Avoid trying to mutate Cargo metadata or uninstall package records.

If this is judged too surprising, an alternative is:

- detect Cargo-managed location;
- perform `cargo install stegoeggo-cli --locked --version =X.Y.Z` rather than binary replacement.

Choose one policy before implementation and test it. Gregg's binary-first replacement model is the simpler precedent.

## Workstream H: CLI and JSON UX

Recommended output:

```text
stegoeggo 0.4.0
Latest stable: 0.4.1
Downloading verified release asset...
Updated 0.4.0 -> 0.4.1
```

Progress belongs on stderr; concise final outcome on stdout if useful for scripting.

Do not overload the global protection `--json` behavior onto updater internals unless there is a clear machine-readable update use case. If `update --json` is added, define a small stable schema such as status/current/latest/source.

## Workstream I: Tests

Unit tests should cover:

```text
stable_version_parser_rejects_prerelease_when_required
target_mapping_matches_release_contract
asset_name_matches_installer_contract
already_current_is_noop
newer_stable_resolves_available
checksum_mismatch_is_fatal
candidate_wrong_program_is_fatal
candidate_wrong_version_is_fatal
asset_404_allows_cargo_fallback
network_failure_does_not_allow_fallback
unwritable_executable_fails_before_download
```

Integration tests should use local fixtures/fake subprocesses or injected executors rather than hitting crates.io/GitHub in ordinary CI.

Release smoke must test the real network path against a prerelease/test release before enabling production self-update documentation.

## Dependency Review

If copying Gregg's implementation shape, evaluate these dependencies:

```text
serde_json
sha2
self-replace
tempfile
thiserror
```

StegoEggo CLI already depends on `serde_json` and `tempfile`; avoid adding duplicates at workspace scope unnecessarily. Confirm all added dependencies support MSRV 1.87 and pass `cargo deny` policy.

Do not copy a separate Gregg-specific crate wholesale. Copy/adapt only generic mechanics and rename all repository/program assumptions.

## Documentation Changes

Update:

- `README.md`: show `stegoeggo version` and `stegoeggo update` in CLI section.
- `docs/installation.md`: explain binary-managed update behavior and permissions.
- `docs/cli-usage.md`: command semantics and errors.
- `RELEASING.md`: updater version authority and requirement that matching release assets exist.
- `architecture/cli.md`: update flow and trust boundaries.
- `SECURITY.md`: checksum verification is integrity protection, not code signing; explain GitHub/crates.io trust assumptions.
- `AGENTS.md`: updater invariants and tests.

## Required Checks

```bash
./scripts/check.sh
cargo test -p stegoeggo-cli --all-features
cargo deny check licenses
cargo deny check advisories
cargo run -p stegoeggo-cli -- version
cargo run -p stegoeggo-cli -- update --help
```

Use a controlled test release for end-to-end update verification; do not test destructive self-replacement on the developer's primary installed binary.

## Acceptance Criteria

- `stegoeggo version` prints the exact package version with a stable first line.
- `stegoeggo update` can update a supported binary installation using the matching verified GitHub asset.
- Checksums and candidate identity/version are verified before replacement.
- Failed validation leaves the current executable unchanged.
- No implicit `sudo` or privilege escalation occurs.
- Cargo fallback occurs only for source-only/missing-asset cases.
- Target and asset naming cannot drift from Plan 099 without tests failing.
- Dependencies remain MSRV/license/advisory compliant.
- Installer, updater, release workflow, and docs agree on the same version/asset contract.

## Dependency / Ordering

Implement after Plan 099 establishes the public asset contract. Integrate command parsing according to Plan 098.
