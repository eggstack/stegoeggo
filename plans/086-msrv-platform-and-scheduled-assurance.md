# Plan 086: MSRV, Platform, and Scheduled Assurance

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/081-pre-v1-consolidation-maintainability-and-portability-roadmap.md`

Audited baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`.

## 1. Objective

Make documented toolchain/platform claims continuously evidence-backed without undoing the repository's deliberate simplification of required CI and manual release policy.

At baseline the required CI is one Ubuntu stable job running `./scripts/check.sh`. `SUPPORT.md` declares MSRV 1.87 but stable CI does not pin or test 1.87, and Linux aarch64/macOS/Windows are not CI-tested. External metadata conformance and fuzz workflows are manual-only.

## 2. Required CI shape

Keep one compact primary PR gate. Add separate, clearly named assurance jobs/workflows rather than bloating `scripts/check.sh`.

### 2.1 MSRV proof

Add a Rust 1.87 job that performs the narrowest sufficient compile/test matrix to prove the declared MSRV across publishable crates and important feature combinations.

At minimum verify:

- `stegoeggo-stego` default features;
- root `stegoeggo` default/no-default surface;
- `stegoeggo-cli` build;
- feature combinations that compile on MSRV and are part of documented support.

Do not silently raise MSRV to satisfy a dependency unless separately justified. If a transitive dependency has already made 1.87 impossible, record the evidence and either pin a compatible dependency or update the declared MSRV as an explicit semver/toolchain decision.

### 2.2 Platform assurance

Add appropriate jobs for:

- Linux x86_64 (existing required gate);
- macOS aarch64;
- Windows x86_64.

Linux aarch64 should be added via native/available runner when practical, or via `cargo check --target aarch64-unknown-linux-gnu` with an evidence-backed setup. Do not add brittle cross-linking complexity merely to change a table cell from No to Yes.

Platform jobs may use a reduced matrix if full all-features tests are disproportionately expensive. Document exactly what each proves.

### 2.3 Scheduled external conformance

Retain manual dispatch but add a low-frequency schedule to `external-verification.yml` unless repository policy or runner cost provides a concrete reason not to. Scheduled failure should provide signal without becoming a release/publish trigger.

### 2.4 Scheduled fuzz smoke rotation

Retain manual target selection. Add a scheduled job that rotates or batches a bounded subset/all fuzz targets for a short smoke duration. Keep the maximum runtime bounded and upload crash artifacts as today.

Do not run long fuzzing on every PR.

## 3. Workflow hardening

- Pin only where consistent with repository dependency/update policy; avoid unnecessary workflow complexity.
- Set least-privilege permissions.
- Preserve concurrency cancellation for PR workflows where useful.
- Keep release publication manual; no tag-triggered release automation.
- Avoid external-tool installation in the required fast CI job.
- Cache Rust artifacts only where it materially reduces runtime.

## 4. Documentation

Update `SUPPORT.md` so "CI Tested" reflects exact evidence, not expected portability. Distinguish:

- fully tested on each PR;
- compile-checked only;
- scheduled integration-tested;
- expected but untested.

Update `RELEASING.md` and architecture/CI docs so maintainers understand which checks are blocking vs scheduled/non-blocking.

## 5. Acceptance criteria

- declared MSRV is explicitly exercised by CI;
- macOS aarch64 and Windows x86_64 receive continuous build/test assurance;
- Linux aarch64 has either practical continuous evidence or remains truthfully marked untested with rationale;
- external conformance and fuzzing gain bounded scheduled execution unless status ledger records a concrete blocker;
- required PR CI remains compact and does not acquire release/network-audit work;
- manual release policy is unchanged;
- workflow YAML is validated by actual runs where possible and local command equivalents are recorded;
- `./scripts/check.sh` remains unchanged unless a separate local-gate correctness issue is discovered;
- `plans/086-status.md` records the final support/evidence matrix.

## 6. Non-goals

No automated crates.io release, binary distribution pipeline, benchmark gating, coverage service, dependency bot policy, or exhaustive OS/architecture matrix.
