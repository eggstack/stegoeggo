# Plan 100 Status: CLI Version and Self-Update

Status: complete

Implemented the explicit offline `version` command contract and the verified
`update` path in `stegoeggo-cli/src/update.rs`.

- Stable versions come from the non-yanked crates.io `stegoeggo-cli` release
  list; prereleases and malformed versions are ignored.
- Supported host targets and versionless asset names match Plan 099, with
  release preflight checks guarding the updater mapping.
- Updates use bounded `curl` downloads, private staging, SHA-256 sidecars,
  executable permissions, candidate identity/version validation, and
  `self-replace` without implicit privilege escalation.
- Cargo fallback is restricted to unsupported targets or HTTP 404 for the
  exact binary asset. Checksum, candidate, server, and network failures are
  fatal and preserve the current executable.
- Unit and CLI integration tests cover version output, parser/resolution,
  target and asset contracts, checksum and candidate failures, fallback
  classification, and replaceability preflight.
- README, installation/CLI guides, release/security/stability policy,
  architecture docs, AGENTS.md, and the StegoEggo conventions skill were
  updated.

The destructive self-replacement path was not run against the developer’s
primary executable. Local tests use isolated temporary fixtures; a real
controlled prerelease/test-release smoke remains a maintainer release step.
