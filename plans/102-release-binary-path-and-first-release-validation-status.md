# Plan 102 Status: Release Binary Path Hardening and First-Release Validation

Status: implementation complete; real-release validation pending publication
of the first binary-enabled `0.4.x` release.

## Completed in this change

- `.github/workflows/release-binaries.yml` now records the Cargo executable
  name separately from the public asset name and stages only
  `target/<triple>/release/stegoeggo` or `stegoeggo.exe` on Windows.
- The broad `find target ... stegoeggo*` fallback was removed. The workflow
  fails immediately when the exact source file is absent and preserves the
  staged file mode.
- `scripts/check-docs-contract.sh` now proves that the workflow matrix matches
  `scripts/release-targets.txt` and that the deterministic source-path
  contract remains present. This runs through required `scripts/check.sh`.
- `scripts/release-check-assets.sh` now requires the complete five-binary,
  five-sidecar, and two-installer contract, verifies the sidecar filename as
  well as its digest, and rejects extra release files.
- README, maintainer guidance, the Rust-conventions skill, release procedure,
  and CLI/tooling architecture docs now describe the deterministic staging and
  strict asset contract.

## Local validation

The following checks passed before the change was committed:

```text
./scripts/check.sh
./scripts/check-docs-contract.sh
./scripts/test-release-installers.sh
./scripts/test-release-updater.sh
./scripts/release-check-assets.sh --dir=<synthetic five-target assets> --version=0.4.0 --native-smoke
./scripts/release-binary-preflight.sh --tag=v0.4.0 --skip-check
```

Results: all commands passed. The full `check.sh` run included 627 root
unit tests, all workspace integration suites, 186 carrier tests, 55 CLI unit
tests, 73 CLI integration tests, and doctests. The native local release-path
check also built `target/x86_64-apple-darwin/release/stegoeggo` and verified
`stegoeggo 0.4.0` on Darwin x86_64 (Darwin 25.6.0). The installer and updater
rehearsals use disposable fixture servers; they do not create a GitHub
Release, publish crates, or replace the developer's primary executable.
The preflight was run against a temporary local `v0.4.0` tag pointing at this
commit and passed; that tag was deleted and was never pushed.

The asset validator's negative extra-file case also passed: an unexpected
`stegoeggo-extra` entry was rejected.

## Release-dependent validation

As of 2026-09-11, `stegoeggo-cli` on crates.io is `0.3.3`, the latest GitHub
Release is `v0.3.3`, and no `v0.4.0` GitHub Release or binary assets exist.
The repository is therefore not yet eligible for the plan's real-release
preconditions. No tag, release, or ad hoc asset upload was created to fake
this evidence.

Pending the first published binary-enabled `0.4.x` release:

- dispatch and verify the complete five-target GitHub Actions workflow;
- validate the exact `releases/latest/download/install.sh` command on a
  supported Unix host and the pinned installer URL;
- perform native Windows installer validation when a Windows environment is
  available;
- record the release tag, workflow run URL, tested hosts, and command results;
- perform the real A→B self-update rehearsal after a second compatible stable
  binary release exists. Fixture failure-mode coverage remains complete.
