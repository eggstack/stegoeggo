# Plan 098 Status: CLI Command Surface Simplification

Status: complete

Implemented the command-oriented CLI surface while retaining the 0.x root
protection syntax and `--verify` compatibility alias.

- Added `protect`, `inspect`, `verify`, `version`, and reserved `update`
  commands; feature-gated signing commands remain flat.
- Routed implicit root protection and explicit `protect` through the existing
  canonical `ProtectionRequest` builder.
- Added grouped protection help and command-oriented root help.
- Split read-only inspection from assertion-oriented verification. New
  `verify` returns exit code 3 for missing or invalid protection evidence;
  compatibility `--verify` retains its historical exit behavior.
- Added parser/help, equivalence, read-only, and verification exit regressions.
- Updated README, CLI usage, architecture, stability, deprecation, and agent
  guidance. `authenticated-provenance` and `maximal` remain equivalent and
  are marked as a v1 consolidation candidate.

Validation completed locally:

```text
cargo test -p stegoeggo-cli --all-features
./scripts/check.sh
```

The full workspace check is run again on `main` after merge, before push.
