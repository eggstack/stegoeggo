# Plan 106 Status: Real Eggfetch-to-Eggfetch Self-Update Closure

Status: closed — public 0.4.2 → 0.5.0 transition verified on macOS x86_64,
2026-10-04. Windows transition was not performed.

## Objective

Prove one real public stable self-update transition where both the starting
binary and replacement contain the native updater.

## Release identities

- A = 0.4.2, tag `v0.4.2`, source
  `3b75c70660f7d3842288188131dd88c709130d11`.
- B = 0.5.0, tag `v0.5.0`, source
  `57ca94c910269e080b06aa8cb34c3767b3bf669d`.
- `stegoeggo-cli 0.5.0` is published at
  https://crates.io/crates/stegoeggo-cli/0.5.0.
- Public GitHub release: https://github.com/eggstack/stegoeggo/releases/tag/v0.5.0
  (published 2026-10-04 with the complete audited 15-file Eggpack set).

## Required closeout evidence

- Host/target: macOS x86_64 (`Darwin`, `x86_64`). Initial executable was the
  public `stegoeggo-x86_64-apple-darwin` asset from v0.4.2 downloaded into an
  isolated `/tmp/stegoeggo-public-update.*` directory; it reported
  `stegoeggo 0.4.2`.
- Endpoint overrides: `STEGOEGGO_CRATES_API_URL` and
  `STEGOEGGO_RELEASES_URL` were unset for the production update.
- Public update command: `stegoeggo update` completed with
  `Updated 0.4.2 -> 0.5.0.` The updater selected 0.5.0 from crates.io and
  fetched the exact host binary and SHA-256 sidecar from the public v0.5.0
  GitHub release. Eggup checksum, identity, version, and replacement gates
  passed; no Cargo fallback was used on this supported target.
- Post-update `version`: `stegoeggo 0.5.0`.
- CLI `--help` passed. `protect`, then `inspect` and `verify` on the generated
  protected PNG all returned JSON `status: ok` with verified evidence.
- Second `update`: `stegoeggo 0.5.0 is up to date (latest stable 0.5.0).`
- Curl independence: updater plus protect/inspect/verify smokes passed with an
  empty `PATH`; no external curl executable was available to the process.
- Deterministic updater rehearsal: `./scripts/test-release-updater.sh` passed
  at the 0.5.0 source commit. The focused real public transition supplements,
  and does not replace, this local regression.
- Windows A→B transition: not performed. No Windows proof is claimed.

## Disposition

The supported-target live transition satisfies Plan 106's closure gate. Its
release evidence is shared with Release-Distribution M001 and M002 and is
recorded in `plans/closure/release-distribution/005-status.md`.
