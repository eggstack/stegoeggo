# Maintenance Quality Milestone 006 — Closure Status

Status: conditionally closed
Source implementation plan: `plans/implementation/maintenance-quality/006-root-workspace-dependency-security-refresh.md`
Source subsystem roadmap: `plans/subsystems/maintenance-quality-roadmap.md#7-milestones`
Repository baseline reviewed: `3d2388638b0d5ed3fb01b76f9b80b2a3f25746cb`
Implementation commit: `0b400cf20780e394f3a43d45987208dce4fd3eb8` — `security: refresh workspace and binding dependencies`.

## 1. Executive finding

The root workspace dependency refresh is implemented and passes the local security, updater, and required Rust checks. The fresh RustSec audit was run on 2026-11-17. The six baseline vulnerabilities and the yanked baseline release were remediated; the final root audit and `cargo deny check advisories` succeed. The only production-code adaptation is a narrow update for Eggup's refreshed `FetchLimits` API; the updater's executable response ceiling remains 64 MiB and focused updater coverage passes (43 tests).

This milestone remains **conditionally closed** because M011's workflow-fix follow-up creates a new shared PR head. The required `.github/workflows/ci.yml` Check job passed on the prior candidate SHA `0b400cf20780e394f3a43d45987208dce4fd3eb8` ([run](https://github.com/eggstack/stegoeggo/actions/runs/44165868955/job/134331630837)); a passing Check on the follow-up head is still required before unconditional closure. No merge, release, or publication has occurred.

## 2. Requirement-to-evidence matrix

| Plan requirement | Evidence |
|---|---|
| Reconfirm the advisory baseline | Fresh 2026-11-17 audit identified baseline RUSTSEC-2026-0346, -0347, -0348, -0349, -0382, and -0381 vulnerabilities; warnings included -0190, -0486, and -0386 plus yanked `yoke-derive` (§4). |
| Refresh only the root graph | Root `Cargo.lock` refreshed with Cargo; no root `Cargo.toml` constraint change and no excluded binding lockfile was updated by the root command (§3–4). |
| Remediate root advisories | Final `cargo audit --file Cargo.lock` and `cargo deny check advisories` both exited 0; no unresolved root vulnerability or yanked-package finding was reported (§4). |
| Review CLI graph and updater behavior | Locked CLI dependency tree inspected; updater tests: 43 passed. The Eggup `FetchLimits` adaptation preserves the 64 MiB artifact limit and uses the registry metadata bound (§3–5). |
| Run the required local suite | `./scripts/check.sh` exited 0 (§4). |
| Candidate-specific required CI | Required Check passed on PR #3 at `0b400cf20780e394f3a43d45987208dce4fd3eb8` (run `44165868955`). A follow-up commit for M011 would need its own required-gate result (§4, §10). |

## 3. Production implementation evidence

- `Cargo.lock` is the only root workspace lockfile changed. Cargo's dry-run preview covered 115 package resolutions; the final lockfile includes `rustls 0.23.46`, `rustls-webpki 0.103.17`, `webpki-roots 1.0.12`, `tokio 1.54.1`, `rustix 1.1.6`, and `yoke-derive 0.8.4`. The three excluded binding lockfiles remain separately owned by M011.
- Root package manifests and `deny.toml` were not changed.
- `stegoeggo-cli/src/update.rs` adapts to Eggup's `FetchLimits.max_artifact_bytes: u64` API. `EXECUTABLE_BODY_LIMIT` remains the 64 MiB cap; artifact fetch metadata is bounded by `REGISTRY_BODY_LIMIT`, while the executable-size check remains separately enforced. No updater policy or public CLI behavior was intentionally changed.

## 4. Verification executed (exact commands + results)

Commands run from the repository root unless noted:

```text
cargo update --dry-run --manifest-path Cargo.toml   PASS; compatible root resolution previewed (115 packages)
cargo update                                         PASS; regenerated root Cargo.lock
cargo deny check advisories                          PASS; exit 0
cargo audit --file Cargo.lock                        PASS; fresh 2026-11-17 advisory data; no unresolved root vulnerability/yanked finding
cargo tree --locked -p stegoeggo-cli --edges normal  PASS; reviewed the locked CLI dependency graph
cargo test -p stegoeggo-cli --locked --all-features update::tests
                                                     PASS; 43 passed
./scripts/check.sh                                   PASS; exit 0
cargo deny check licenses                            PASS; four unused allowances reported
git diff --check                                     PASS
git diff --cached --check                            PASS
```

The root dry-run was performed with `--manifest-path Cargo.toml`; binding dry-runs and audits are recorded separately in M011. Candidate-specific remote evidence: required Check job on PR #3, SHA `0b400cf20780e394f3a43d45987208dce4fd3eb8`, **passed** — [workflow run](https://github.com/eggstack/stegoeggo/actions/runs/44165868955/job/134331630837).

## 5. Invariant review

- Root, carrier, and CLI package versions and declared MSRV remain unchanged; root manifests are unchanged.
- The binding workspaces remain outside the root workspace and their lockfiles are not conflated with this audit.
- Updater stable-version authority, checksum/identity validation, response limits, redirect/proxy policy, failure-preservation behavior, and publication policy were not intentionally changed. The only updater source change follows the refreshed Eggup API and retains the executable cap.
- No required CI topology, public API, release behavior, advisory exception, or license exception was added.

## 6. Failure and recovery review

The Eggup API refresh changed `max_artifact_bytes` from `Option<u64>` to `u64`. After adapting that type, three focused updater tests exposed an incorrect metadata-bound value. The fetch helper was corrected to use the registry metadata cap, keeping the executable body limit independent; all 43 updater tests and `./scripts/check.sh` then passed. No known test failure remains unresolved.

## 7. Migration and compatibility review

No public API, crate version, supported target, or Rust MSRV migration was made. The updater source adjustment is internal and limited to the dependency API change. Root locked validation and the focused updater suite pass. Candidate-specific GitHub CI is still required before unconditional closure.

## 8. Security review

The refreshed root lockfile was audited against RustSec data fetched on 2026-11-17. The six baseline vulnerabilities and the baseline yanked `yoke-derive` selection are no longer unresolved in the final root audit; `cargo deny check advisories` passes. No advisory suppression or `deny.toml` exception was introduced. `cargo deny check licenses` also passes; its four unused-allowance notices are recorded as a low, non-blocking finding (§10).

## 9. Documentation and operations

No user-facing documentation or updater contract changed. `plans/implementation/maintenance-quality/006-root-workspace-dependency-security-refresh.md`, this record, `plans/subsystems/maintenance-quality-roadmap.md`, and `plans/registry.md` now reflect the implementation and conditional status. Required CI was not expanded. The separate binding security result is in M011.

## 10. Unresolved findings (critical/high/medium/low)

- **Medium — follow-up-head required Check pending.** `.github/workflows/ci.yml`'s Check passed on candidate SHA `0b400cf20780e394f3a43d45987208dce4fd3eb8` in [PR #3](https://github.com/eggstack/stegoeggo/pull/3) ([job](https://github.com/eggstack/stegoeggo/actions/runs/44165868955/job/134331630837)). M011's workflow-fix follow-up creates a new candidate head; require a passing Check on that final SHA before unconditional closure.
- **Low — four unused license allowances.** `cargo deny check licenses` exits 0 and reported four allowances that are unused by the current graph. They were not removed because allowance cleanup is outside this security refresh.

No unresolved root vulnerability or yanked-package finding is known from the final audit.

## 11. Roadmap disposition

Maintenance-quality M006's required Check passed on prior SHA `0b400cf20780e394f3a43d45987208dce4fd3eb8`. It remains conditionally closed until the required Check passes on M011's workflow-fix follow-up head; M006 does not change CI placement or depend on optional M002.

## 12. Registry updates

`plans/registry.md` records M006 as conditionally closed, links this record, and notes that required Check passed on prior SHA `0b400cf20780e394f3a43d45987208dce4fd3eb8`; the updated PR head needs a fresh required Check before unconditional closure. The maintenance-quality roadmap status table and current-state summary match. No other subsystem status or blocker changed.
