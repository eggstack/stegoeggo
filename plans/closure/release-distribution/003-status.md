# Release and Distribution Milestone 003 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/release-distribution/003-synchronous-eggup-updater-bridge-corrective.md`
Source subsystem roadmap: `plans/subsystems/release-distribution-roadmap.md#milestone-3--synchronous-eggup-updater-bridge-corrective`
Repository baseline reviewed: `df62d60` (plan named `c75132a`; two planning-only commits followed)
Implementation commits: `e611c91` — fix: make CLI updater synchronous around Eggup seam (M003)

## 1. Executive finding

M003 is closed. The consumer-owned nested Tokio runtime panic in the
deterministic updater rehearsal is eliminated by removing StegoEggo's
unnecessary outer runtime: `update_to` is now a plain sync fn and
`run_update` calls `eggup_get_bytes`/`seam_download` directly from
synchronous CLI code. No release-selection, transport-policy, fallback,
Eggup-transaction, installer, or public CLI-surface change was made.
`./scripts/test-release-updater.sh` now passes end to end, and ten new
seam-level regression tests pin the old failure shape. M001 and M002 remain
operationally gated only on the next ordinary stable B newer than 0.4.2, as
the plan requires; no throwaway version was cut.

## 2. Requirement-to-evidence matrix

| Plan requirement (§5/§6/§10) | Evidence |
|---|---|
| §5.1 production updater synchronous around the sync Eggup seam; no `spawn_blocking` to preserve async shape | `stegoeggo-cli/src/update.rs`: `fn update_to`, `pub(crate) fn run_update` sync; `run_update_async` deleted; no `spawn_blocking` (`e611c91`) |
| §5.2 Tokio ownership audit; move or remove direct dep; record result | Audit: only production use was the removed `run_update` runtime; direct `tokio` moved from `[dependencies]` to `[dev-dependencies]` (`rt`); `cargo tree -p stegoeggo-cli` shows no production `tokio` |
| §5.3 test-only policy harness stays separated; no nested shape via shared helpers | `fetch_to_file`/`download_required`/`build_client_with_env` remain `#[cfg(test)]`; each `block_on` constructs its runtime outside any runtime; production acquisition goes only through `seam_download`/`eggup_get_bytes` |
| §6.1 structural test: production creates/blocks on no Tokio runtime | `production_updater_is_synchronous_without_nested_runtime` — pass |
| §6.2 registry acquisition through the Eggup seam from the production sync context | `seam_registry_acquisition_from_sync_context_resolves_latest` — pass |
| §6.3 supported-target artifact path through `seam_download` without nested panic | `seam_artifact_download_from_sync_context_writes_bytes` — pass |
| §6.4 already-current stays no-op after registry selection | `already_current_update_needs_no_preflight_or_download` (now sync) — pass |
| §6.5 exact 404 permits Cargo fallback only at the binary boundary | `seam_asset_404_permits_fallback_only_at_binary_boundary` + `eggup_fallback_classifier_matches_eggsact_policy` — pass |
| §6.6 checksum 404/mismatch, 5xx, timeout, malformed proxy, wrong candidate stay hard | `seam_sidecar_404_is_hard_failure`, `seam_registry_500_is_hard_failure_without_fallback`, `seam_artifact_500_is_hard_failure_without_fallback`, `seam_oversized_registry_body_is_rejected`, `seam_timeout_is_hard_failure`, `seam_malformed_proxy_fails_closed`, plus retained `checksum_mismatch_is_fatal`, `candidate_wrong_program/version_is_fatal`, `native_500/timeout_is_hard_failure` — pass |
| §6.7 `./scripts/test-release-updater.sh` completes without panic | `Release updater tests passed` — pass |
| §10 no production nested runtime on registry/asset/sidecar paths; `update_to` not async; invariants unchanged; CI + drift green; no medium+ runtime finding | All recorded below |

## 3. Production implementation evidence

- `stegoeggo-cli/src/update.rs` (`e611c91`):
  - `async fn update_to` → `fn update_to` (body unchanged).
  - `async fn run_update_async` deleted; `pub(crate) fn run_update` now parses the current version, fetches the registry via `eggup_get_bytes`, resolves the latest stable, and calls `update_to` synchronously.
  - `already_current_update_needs_no_preflight_or_download` calls `update_to` directly.
  - Ten new tests (structural + seven seam-level + timeout + malformed proxy).
- `stegoeggo-cli/Cargo.toml` (`e611c91`): `tokio = { version = "1", features = ["rt"] }` moved from `[dependencies]` to `[dev-dependencies]`; production `cargo tree --depth 1` lists clap, eggup-acquisition, eggup-core, eggup-eggfetch, hex, rayon, serde, serde_json, stegoeggo, tempfile only.
- `architecture/cli.md` (`e611c91`): production dependency paragraph updated — updater calls the synchronous Eggup seam from synchronous code; `tokio` documented as CLI dev-only for the test harness.
- Before/after call graph:
  - Before: `run_update -> new_current_thread runtime -> block_on(run_update_async) -> eggup_get_bytes/seam_download -> EggfetchTransport::fetch_* -> block_on(private runtime)` — nested runtime, panics.
  - After: `run_update -> eggup_get_bytes(registry) -> resolve latest -> update_to -> seam_download/sidecar fetch -> Eggup prepare/verify/commit -> fallback/replacement`, with the Eggup private runtime the only runtime on the path.

## 4. Verification executed (exact commands + results)

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — pass (one new `needless_borrows_for_generic_args` fixed before commit).
- `cargo test -p stegoeggo-cli --all-features --locked` — pass: 91 bin + 73 `cli.rs` + 16 `cli_detached_verification` = 180 passed, 0 failed.
- `cargo test --workspace --exclude stegoeggo-fuzz --all-targets --all-features --locked` — pass, exit 0.
- `./scripts/check.sh` — pass (fmt, clippy, no-default-features check, workspace tests, docs contract).
- `./scripts/test-release-updater.sh` — pass: `Release updater tests passed` (no-op current, verified 0.4.2→9.9.9 replacement, checksum-mismatch preservation, identity-mismatch preservation).
- `./scripts/release-check.sh --stage=pre --allow-dirty --skip-check` — pass: `Release check passed` (run after the already-green `check.sh`).
- `python3 scripts/check-release-contract.py` — pass: `release target/asset contract passed`.
- `eggpack ci check --workflow-shape release/eggpack/workflow-shape.json --contract release/eggpack/distribution.toml --github-policy release/eggpack/github-policy.json --workflow .github/workflows/release-binaries.yml` — pass: `ci check: match (55963 bytes)`, zero drift from the M002 cutover.
- `cargo +1.89.0 check -p stegoeggo-cli --locked --all-features` — pass (MSRV 1.89); `-p stegoeggo-stego` and `-p stegoeggo --all-features` also pass (carrier shows its 5 pre-existing warnings only).
- `git diff --check` — pass.
- Note: plan §9 names `cargo test --workspace --all-targets --all-features --locked` without the fuzz exclusion; the fuzz crate requires nightly `cargo-fuzz` and is excluded from workspace tests by repo convention (`AGENTS.md`, `check.sh`). The equivalent with `--exclude stegoeggo-fuzz` was run and passed.

## 5. Invariant review

All §4 invariants hold, verified by the matrix above plus the unchanged policy code: crates.io stable-version authority with prerelease/yanked filtering; exact version-tagged asset/sidecar names; Cargo fallback only for unsupported targets and exact binary 404 (`fallback_allowed` unchanged); checksum-sidecar failure fatal after a found binary (rehearsal checksum case preserves the executable); strict HTTPS-downgrade denial (`https_to_http_redirect_policy_is_strict` still passes); explicit environment proxy handling; unchanged connect/total/body bounds; no curl dependency; exact Eggup ownership/integrity/version validation with `AbsentPolicy::DenyCreate` and all three receipt dispositions mapped; Windows self-replacement path untouched; already-current path performs no download/replacement; CLI surface and output semantics unchanged except the panic is gone.

## 6. Failure and recovery review

No error category was changed. Transport/transaction failures still return bounded `UpdateError` values, never panics: 5xx and timeouts map to `Transport`, oversize to `BodyTooLarge`, sidecar 404 to `HttpStatus`, malformed proxy fails closed at transport construction, wrong candidates fail identity validation. Eggup transaction dispositions (`Committed`/`RolledBack`/`RecoveryRequired`) are mapped exactly as before. The rehearsal proves recovery behavior: checksum and identity failures preserve the current executable.

## 7. Migration and compatibility review

No public CLI/API/schema or release-asset change. The `tokio` dependency move is an internal footprint cleanup: production binaries no longer link a direct Tokio runtime (transitive Tokio remains inside `eggup-eggfetch`'s private bridge, as designed). No release version was consumed; the Eggpack-generated release workflow and configuration are untouched (drift check byte-identical). Rollback is a revert of `e611c91`.

## 8. Security review

Attack surface narrows slightly: one fewer direct runtime in the updater path; all trust gates (sidecar SHA-256, exact `version` identity, current-executable ownership, deny-create, bounded bodies/timeouts, proxy-fails-closed, downgrade denial) are unchanged and re-tested through the seam. No secrets logged; no `sudo`; no new network surface.

## 9. Documentation and operations

Updated: `architecture/cli.md` (synchronous seam ownership + dev-only Tokio). No user-guide change: `docs/installation.md`, `RELEASING.md`, `STABILITY.md`, and CLI help are behaviorally accurate as-is. Operator entry point unchanged: `./scripts/test-release-updater.sh` is now green and remains the rehearsal gate before any live B evidence run.

## 10. Unresolved findings (critical/high/medium/low)

- None critical/high/medium. The M002 medium finding (updater rehearsal nested-runtime panic) is resolved by this corrective.
- Low (pre-existing, unchanged): final-pin Linux CargoZigbuild build + GLIBC ELF proof, Windows double-build digest comparison, and PowerShell installer parsing still await the first live Eggpack-produced release (owned by WP9, shared with M001/M002).
- No stop condition triggered: the Eggup adapter still documents and implements the synchronous seam; no production subsystem required `update_to` to stay async; the panic is gone without touching fallback, trust, release-selection, or transaction policy.

## 11. Roadmap disposition

M003 closes here. It removes the code blocker for the shared next-release evidence but does not close M001 or M002 by itself: both still wait for the next ordinary stable B newer than 0.4.2 (no throwaway version authorized). After M003, M001/M002 are operationally gated only on that ordinary stable B event — M002 for the first live Eggpack-produced five-target draft/publication evidence, M001/Plan 106 for the real public 0.4.2→B updater transition. Subsystem `release-distribution` remains active until those closures complete.

## 12. Registry updates

`plans/registry.md`: M003 moved to closed with this closure record; M002 row note updated (updater-runtime blocker resolved by closed M003; live B evidence still outstanding); M001/M002 blocked rows unchanged except the shared code-blocker reference now points at closed M003. `plans/subsystems/release-distribution-roadmap.md`: current state and milestone table updated (M003 closed; M001 blocked and M002 conditionally closed on the same ordinary stable B event).
