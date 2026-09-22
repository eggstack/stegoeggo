# Plan 105 Status: First Eggfetch-Enabled Release and Five-Target Qualification

Status: complete 2026-09-22. Release 0.4.2 is the first stable eggfetch-enabled CLI release, fully qualified through the real five-target workflow.

## Objective

Publish the first stable StegoEggo CLI release whose shipped binary contains
the native `eggfetch-core` updater, then qualify that exact release through
the real five-target GitHub Actions matrix and public installer path.

## Release version

- Selected next unused synchronized workspace version: `0.4.2` (crates.io max was `0.4.1`, Git tags max `v0.4.1`, workspace `0.4.1`).
- All three crates lockstep `0.4.2` with exact `=0.4.2` intra-workspace deps.
- Source commit SHA: `3b75c70660f7d3842288188131dd88c709130d11` (`feat(release): 0.4.2 eggfetch-enabled release source with preflight contract fix`).
- Tag `v0.4.2` points exactly to that commit; tag was not force-moved.

## Crates.io publication (carrier -> library -> CLI)

- `stegoeggo-stego 0.4.2`: https://crates.io/crates/stegoeggo-stego/0.4.2 — published, API confirms `0.4.2` not yanked.
- `stegoeggo 0.4.2`: https://crates.io/crates/stegoeggo/0.4.2 — published after carrier visible, `--stage=root` check passed.
- `stegoeggo-cli 0.4.2`: https://crates.io/crates/stegoeggo-cli/0.4.2 — published after library visible, `--stage=cli` check passed; API `max_version` is now `0.4.2`.
- Sequence followed `RELEASING.md` staged dry-runs and propagation checks; no fixed sleep; no republished version.

## Git tag and GitHub Release

- Tag: `v0.4.2` (`git push stegoeggo v0.4.2`).
- Release: https://github.com/eggstack/stegoeggo/releases/tag/v0.4.2 (created from immutable tag, notes describe eggfetch migration, MSRV 1.89, downgrade denial, bounded bodies, preflight clarification, sidecar integrity scope).
- Preflight: `./scripts/release-binary-preflight.sh --tag=v0.4.2` passes (version lockstep 0.4.2, signatures features, 5 targets).

## Five-target workflow run

- Run: https://github.com/eggstack/stegoeggo/actions/runs/35738855182 (`release-binaries.yml`, dispatch `tag=v0.4.2`, ref `v0.4.2`).
- Conclusion: `success`.
- Jobs (all `success`):
  - Release preflight: https://github.com/eggstack/stegoeggo/actions/runs/35738855182/job/106783023600
  - Build x86_64-unknown-linux-gnu: https://github.com/eggstack/stegoeggo/actions/runs/35738855182/job/106785239335 (ubuntu-latest, zig glibc 2.17, smoke `stegoeggo 0.4.2`, `--help`, protect/inspect, checksum sidecar)
  - Build aarch64-unknown-linux-gnu: https://github.com/eggstack/stegoeggo/actions/runs/35738855182/job/106785239312 (ubuntu-24.04-arm, zig glibc 2.17, same smokes)
  - Build x86_64-apple-darwin: https://github.com/eggstack/stegoeggo/actions/runs/35738855182/job/106785239353 (macos-15-intel native, same smokes)
  - Build aarch64-apple-darwin: https://github.com/eggstack/stegoeggo/actions/runs/35738855182/job/106785239461 (macos-latest native, same smokes)
  - Build x86_64-pc-windows-msvc: https://github.com/eggstack/stegoeggo/actions/runs/35738855182/job/106785239363 (windows-latest native, `stegoeggo 0.4.2` smoke, `--help`, protect/inspect smoke, checksum sidecar)
  - Attach GitHub Release assets: https://github.com/eggstack/stegoeggo/actions/runs/35738855182/job/106788641555

## Windows native build result

- `x86_64-pc-windows-msvc` succeeds on `windows-latest`. This is the missing native Windows qualification for the eggfetch/rustls/ring graph; the old v0.4.1 Windows artifact was not used as evidence.

## Linux ABI verification

- Downloaded `stegoeggo-x86_64-unknown-linux-gnu` and `stegoeggo-aarch64-unknown-linux-gnu` from v0.4.2.
- `strings | grep GLIBC_ | sort -V`: x86_64 max `GLIBC_2.17`, aarch64 max `GLIBC_2.17`. Floor preserved; no raised runtime requirement.

## Public asset audit

- `./scripts/release-check-assets.sh --dir=/tmp/stegoeggo-0.4.2-assets --version=0.4.2`: `Release assets valid: 5 binaries with SHA-256 sidecars`, exit 0.
- Exactly five executables, five `.sha256` sidecars, `install.sh`, `install.ps1`; no extra/misnamed files.
- First-line version identity: `stegoeggo-aarch64-apple-darwin version` → `stegoeggo 0.4.2`; `stegoeggo-x86_64-apple-darwin version` → `stegoeggo 0.4.2` (Linux/Windows assets smoke-tested natively in workflow).
- Release binary sizes (production eggfetch footprint):
  - `stegoeggo-x86_64-unknown-linux-gnu` 4,308,384 bytes
  - `stegoeggo-aarch64-unknown-linux-gnu` 3,782,280 bytes
  - `stegoeggo-x86_64-apple-darwin` 3,952,824 bytes
  - `stegoeggo-aarch64-apple-darwin` 3,438,624 bytes
  - `stegoeggo-x86_64-pc-windows-msvc.exe` 3,816,960 bytes
- Consistent with Plan 104 local eggfetch measurements, not a local anomaly.

## Public installer smoke

- Host: macOS aarch64-apple-darwin. Isolated `HOME=/tmp/tmp.v9aX1YVfVT` plus primary `~/.local/bin` (first run overwrote primary with 0.4.2; isolated run verified separately).
- `curl -fsSL https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh | bash` and `HOME=$TEST_HOME bash install.sh` both install `stegoeggo 0.4.2`.
- Verified: `stegoeggo version` → `stegoeggo 0.4.2`; `--help` prints command topology; `protect canonical_complete.png --rights-policy prohibited-ai-ml-training --preset legal-notice` writes output; `inspect` reports rights notice and policy; `verify` exits 0.
- Windows `install.ps1` not exercised locally; Windows binary build/smoke in workflow is the mandatory evidence.

## Released native-updater smoke

- Binary: isolated `0.4.2` install (`/tmp/tmp.v9aX1YVfVT/.local/bin/stegoeggo`).
- `stegoeggo update` while `0.4.2` is latest stable: `stegoeggo 0.4.2 is up to date (latest stable 0.4.2).`, exit 0, no asset download, no replacement.
- crates.io query succeeds through embedded eggfetch (resolves `0.4.2` as latest stable).
- Curl independence: `PATH=/tmp/empty-path stegoeggo update` succeeds; `rg curl stegoeggo-cli/src/update.rs` has no matches; `cargo tree -p stegoeggo-cli | grep curl` has no matches. Bootstrap installer still uses curl by design; installed update path is native.
- Malformed proxy fails closed: `HTTPS_PROXY=http://user:secret123@[::1 stegoeggo update` exits 1 with `update proxy configuration failed` and does not emit `secret123`.
- No Cargo fallback observed on supported target.

## Preflight wording correction

- Changed stale `before any download` to `before downloading any update artifact` with explicit registry-first note in:
  - `AGENTS.md` updater invariants;
  - `docs/installation.md` update section;
  - `.skills/stegoeggo-conventions/SKILL.md` CLI release conventions;
  - `architecture/cli.md` update flow (added registry-before-preflight sentence; flow diagram already correct).
- Reviewed `SECURITY.md`, `STABILITY.md`, `SUPPORT.md`: no stale preflight wording; no change needed.
- Code unchanged: `run_update_async` fetches registry before `update_to` runs `ensure_replaceable`; `update_to` returns early when `current >= latest`.
- Test: added `already_current_update_needs_no_preflight_or_download` in `stegoeggo-cli/src/update.rs` proving `update_to` with equal versions succeeds without preflight/download; retained `unwritable_executable_fails_before_download` for the `current < latest` preflight-before-asset path.

## Release-specific corrections

- `architecture/overview.md` version `0.4.0` → `0.4.2` and carrier row `0.4.0` → `0.4.2`.
- `CHANGELOG.md` 0.4.2 entry covers eggfetch migration, no runtime curl, bootstrap still curl, MSRV 1.89, downgrade denial, bounded bodies, preflight clarification, sidecar integrity scope; links updated.
- `MSRV=1.89 ./scripts/validate-msrv-package.sh` cannot pass pre-publication for a new exact-dep version (fresh crates.io resolution needs published `0.4.2`); `--locked` MSRV checks pass pre-publish and package validation passes post-publish by construction. Not a release blocker.

## Plan 106 disposition

- Real eggfetch-to-eggfetch A→B replacement remains Plan 106. Only one eggfetch-enabled stable release (`0.4.2`, A) exists; no newer eggfetch-enabled B exists yet. Plan 105 closes; Plan 106 stays open until the first ordinary stable release after 105 provides B. No throwaway version was published to force A→B.
