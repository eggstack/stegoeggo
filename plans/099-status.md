# Plan 099 Status: CLI Binary Release and Bootstrap Installer

Status: complete

Implemented the manual GitHub Release binary distribution path and bootstrap
installers while preserving manual crates.io publication.

- Added the five-target release matrix and stable versionless asset contract
  in `scripts/release-targets.txt`.
- Added manual-dispatch `.github/workflows/release-binaries.yml`; it checks out
  the exact tag, verifies workspace/tag lockstep, builds the `signatures`
  feature set, runs native version/help/protect/inspect smoke checks, creates
  SHA-256 sidecars, and attaches assets only to an existing GitHub Release.
- Added Unix Bash and Windows PowerShell installers with pinned-version
  support, user-local destinations, checksum and candidate identity checks,
  and strict fallback behavior: only unsupported targets and binary-asset 404s
  may use Cargo.
- Made `stegoeggo-cli`'s package default feature `signatures`, keeping Cargo
  and prebuilt binaries feature-consistent.
- Added `release-binary-preflight.sh`, `release-check-assets.sh`, and local
  fixture/server installer tests. The local tests cover target mapping, exact
  pinned URLs, checksum and identity failures, 404/unsupported fallback,
  network-failure behavior, and user/root destinations.
- Updated README, installation/CLI guides, release/security/support policy,
  architecture indexes, agent guidance, and the StegoEggo conventions skill.

Validation completed locally:

```text
bash -n packaging/install.sh scripts/release-binary-preflight.sh scripts/release-check-assets.sh scripts/test-release-installers.sh
shellcheck packaging/install.sh scripts/release-binary-preflight.sh scripts/release-check-assets.sh scripts/test-release-installers.sh
./scripts/test-release-installers.sh
./scripts/release-binary-preflight.sh --help
./scripts/release-check-assets.sh --dir=<fixture-assets> --version=0.4.0 --native-smoke
./scripts/check.sh
```

The first real GitHub Release attachment and remote CI run are performed after
this commit is pushed; no crates.io publication or GitHub Release creation is
part of this change.
