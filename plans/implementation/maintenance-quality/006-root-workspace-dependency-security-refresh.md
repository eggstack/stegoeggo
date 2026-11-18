# Maintenance Quality Milestone 006 — Root Workspace Dependency Security Refresh

Status: conditionally closed
Repository baseline: `3d2388638b0d5ed3fb01b76f9b80b2a3f25746cb` (`main`)
Source roadmap: `plans/subsystems/maintenance-quality-roadmap.md#7-milestones`
Long-term requirements: `plans/000-long-term-specification.md#4-cli-invariants`, `plans/000-long-term-specification.md#5-release-invariants`, `plans/003-planning-process.md#10-verification-bar`
Applicable ADRs: `plans/adrs/ADR-0004-verified-self-update.md`
Primary class: infrastructure

## 1. Objective

Refresh the root workspace dependency lockfile to the highest compatible releases and remediate the root-graph RustSec findings observed at the baseline, without changing StegoEggo APIs, updater policy, release authority, or required CI topology.

This milestone covers the single root `Cargo.lock` shared by the root library, carrier, CLI, and fuzz workspace member. The three excluded binding lockfiles and the PyO3 migration are owned by language-bindings M011.

## 2. Why this milestone is ready

At baseline `3d23886`, the 2026-11-17 audit found six vulnerabilities in the root lockfile: four in `rustls 0.23.45`, one in `rustls-webpki 0.103.15`, and one in `webpki-roots 1.0.9`. It also reported unsoundness warnings for `tokio 1.52.3`, `rustix 1.1.4`, and `anyhow 1.0.102`, plus yanked `yoke-derive 0.8.3`. `cargo deny check advisories` failed on Tokio.

A no-write `cargo update --dry-run` resolved the root graph to highest compatible releases, including patched candidates for those findings, without changing the lockfile. No code or manifest migration is known to be required for the root remediation. There are no hard dependencies or open architecture decisions.

## 3. Current implementation evidence

- `Cargo.toml` defines the root workspace and excludes `bindings`; `stegoeggo-cli/Cargo.toml` carries the updater transport dependencies.
- `plans/adrs/ADR-0004-verified-self-update.md` owns updater identity, checksum, stable-version, network-failure, and self-replacement invariants.
- `cargo audit --file Cargo.lock` reported RUSTSEC-2026-0346, -0347, -0348, -0349, -0382, and -0381 as vulnerabilities; warnings were RUSTSEC-2026-0190, -0486, and -0386 plus the yanked `yoke-derive` release.
- The dry-run proposed, among other compatible resolutions, `rustls 0.23.46`, `rustls-webpki 0.103.17`, `webpki-roots 1.0.12`, `tokio 1.54.1`, `rustix 1.1.6`, and `yoke-derive 0.8.4`; `anyhow` was removed from the proposed graph.
- The dry-run reported 115 root graph packages to relock. These are baseline observations, not fixed version requirements; re-resolve against the current index when implementing.
- No dependency changes have been applied at this baseline.

## 4. Invariants that must not regress

- Preserve the accepted updater contract in ADR-0004 and `architecture/cli.md`: crates.io stable authority, checksum and candidate identity validation, bounded subprocess/network behavior, HTTPS-downgrade denial, explicit proxy behavior, no `sudo`, and preservation of the current executable on staging or validation failure.
- Keep root, carrier, and CLI package version relationships and their declared Rust MSRV unchanged unless a separately approved dependency constraint makes that impossible.
- Do not silently expand `scripts/check.sh` or required CI with specialist audit commands.
- Do not add advisory suppressions or license/source exceptions to `deny.toml` merely to obtain a green result.
- Keep the binding crates and their lockfiles outside the root workspace.

## 5. Scope (in / explicitly out)

**In scope**

- Refresh the root workspace `Cargo.lock` to the highest versions allowed by existing manifest constraints.
- Resolve the recorded root advisories and warnings where patched compatible releases exist.
- Make the smallest direct manifest-constraint change only if a documented advisory fix is otherwise unreachable, and test that change against the 1.89 MSRV and workspace API behavior.
- Review the final lockfile diff and root CLI dependency graph for unintended dependency additions/removals.

**Explicitly out of scope**

- `bindings/{c,node,python}/Cargo.lock` and PyO3 migration; those belong to language-bindings M011.
- Unrelated major or pre-1.0 minor migrations. In particular, do not relax the exact `image = "=0.25.6"` pin or change `quick-xml`, `sha2`, `thiserror`, or `eggup-*` version lines solely to chase a newer incompatible release.
- Product/API changes, updater behavior changes, release workflow changes, publication, advisory allowlists, and changes to required CI placement.

## 6. Required production changes

- Regenerate `Cargo.lock` with Cargo; never hand-edit lockfile package records or checksums.
- Prefer lockfile-only compatible updates. Change `Cargo.toml` only when a known advisory's patched release cannot be selected under the current requirement, and record the reason plus compatibility evidence.
- Do not change production Rust code unless an actual compatible dependency API change requires a minimal adaptation. Any such change must preserve the invariants in section 4 and gain focused regression coverage.

## 7. Ordered work packages (intent, changes, acceptance evidence)

1. **Reconfirm the advisory baseline.** Refresh the RustSec database and run the root audit. Record the date, exact advisory IDs, and any changes since this plan's baseline. Acceptance: the implementation report distinguishes vulnerabilities, unsoundness warnings, and yanked-version warnings.
2. **Refresh the root lockfile.** Run Cargo's compatible update from the repository root. Review the diff; keep manifest edits and package-line migrations outside the stated scope unless a named advisory requires one. Acceptance: the lockfile contains patched releases for all resolvable root findings and remains reproducible with `--locked`.
3. **Validate root behavior and updater invariants.** Run `./scripts/check.sh` and the focused CLI updater test suite. Acceptance: required tests pass and no updater-policy invariant or public command behavior changes.
4. **Re-audit and inspect dependency edges.** Run `cargo deny check advisories`, `cargo audit --file Cargo.lock`, and `cargo tree --locked -p stegoeggo-cli --edges normal`. Acceptance: no unresolved root vulnerability/yanked finding; any warning without a compatible fix is explicitly justified with the advisory and dependency path.
5. **Prepare closure evidence.** Record exact commands/results, lockfile changes, the advisory-database date, and residual limits in `plans/closure/maintenance-quality/006-status.md`; update the registry and roadmap only when implementation evidence exists.

## 8. Failure, cancellation, restart, contention semantics

- If registry or RustSec access fails, stop and report the checks that could not be completed; do not present cached data as a fresh audit.
- If Cargo cannot resolve a patched release under the existing constraints, identify the requiring package and stop before broadening unrelated version ranges.
- If a dependency update breaks the updater contract, MSRV, or required check, revert only the failing dependency selection or make a narrowly scoped code adaptation; do not weaken the contract or skip tests.
- If interrupted during lockfile generation, inspect `git diff` and rerun Cargo resolution from the unchanged manifest; do not discard unrelated work or hand-reconstruct the lockfile.
- Do not claim all repository lockfiles are clean until M011's independent binding audits are also complete.

## 9. Compatibility and migration

This is a compatible dependency maintenance pass, not a StegoEggo release or API migration. Keep CLI output/exit behavior, image processing, updater behavior, crate versions, release target support, and Rust MSRV unchanged. If security remediation requires a breaking dependency line, stop and report the smallest migration proposal rather than bundling it into this milestone.

## 10. Required tests

- Full required Rust suite and quality gates via `./scripts/check.sh`.
- Focused updater tests: `cargo test -p stegoeggo-cli --all-features update::tests`.
- Root workspace advisory and license-policy checks, with no change to `deny.toml` unless separately justified and reviewed.
- Verify the root lockfile remains usable with locked resolution.

## 11. Required verification commands

Run from the repository root, with current registry and advisory data:

```bash
cargo update --dry-run
cargo update
cargo deny check advisories
cargo audit --file Cargo.lock
cargo tree --locked -p stegoeggo-cli --edges normal
cargo test -p stegoeggo-cli --all-features update::tests
./scripts/check.sh
```

Record exit status and results for every command. `cargo update` is expected to alter the root `Cargo.lock`; it must not update any excluded binding lockfile. No command in this plan may add a specialist check to required CI.

## 12. Documentation updates

No user-facing documentation change is expected for a lockfile-only refresh. If a direct dependency constraint or production behavior changes, update the relevant `architecture/cli.md`, release notes, or user documentation only to reflect the actual changed contract. Update this plan's closure record, the maintenance-quality roadmap status table, and `plans/registry.md` when evidence is complete.

## 13. Acceptance criteria (externally observable)

- `cargo deny check advisories` and `cargo audit --file Cargo.lock` pass with no unresolved root vulnerabilities or yanked packages.
- `./scripts/check.sh` and the focused updater test suite pass.
- The CLI updater retains its documented stable-version, checksum, identity, boundedness, redirect, proxy, and failure-preservation behavior.
- No new required CI check, publication behavior, public API, crate version, or supported target is introduced.
- The closure record identifies the exact audit date and residual warnings, if any.

## 14. Stop conditions (report rather than improvise)

Stop and report if a fix requires a breaking dependency migration, a Rust MSRV change, a change to updater/release semantics, a `deny.toml` advisory exception, or widening the scope into a binding package. Also stop if a newly fetched advisory cannot be mapped to a resolved package/version or if the security database cannot be refreshed.

## 15. Closure evidence required

Create `plans/closure/maintenance-quality/006-status.md` only after implementation. Include the baseline/head SHA, exact manifest and lockfile diff summary, advisory database date and IDs, each command in section 11 with its result, CI run links, any skipped specialist scope, and the final registry/roadmap status. A successful lockfile update alone is not closure evidence.

## 16. Handoff notes

The binding lockfiles are separate work and are covered by `plans/implementation/language-bindings/011-binding-dependency-security-refresh.md`. Keep the two audit scopes separate in implementation and closure records, then report the combined repository result only after both have completed. Do not commit or publish from this plan without the maintainer's implementation-stage instruction.
