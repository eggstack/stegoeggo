# Release and Distribution Milestone 005 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/release-distribution/005-0.5.0-release-and-planning-reconciliation.md`
Source subsystem roadmap: `plans/subsystems/release-distribution-roadmap.md#milestone-5--050-release-and-planning-reconciliation`
Repository baseline reviewed: `57ca94c910269e080b06aa8cb34c3767b3bf669d`
Implementation commits: `57ca94c` — release: prepare StegoEggo 0.5.0; closure is planning/evidence only

## 1. Executive finding

StegoEggo 0.5.0 was qualified and published on 2026-10-04. The carrier,
library, and CLI crates are public on crates.io. The immutable `v0.5.0` tag
points to the qualified source commit. Eggpack run `37181914252` passed all
five native builds, clean-host qualifications, aggregation, and staging. Its
draft carried the audited 15-file release set and was manually published.
The public macOS x86_64 0.4.2 CLI then updated to 0.5.0 through production
crates.io and GitHub endpoints, satisfying Plan 106 / Release-Distribution
M001 and the final live condition for M002. M005 is closed.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence |
|---|---|
| Lockstep 0.5.0 Rust crates | `stegoeggo-stego`, `stegoeggo`, and `stegoeggo-cli` 0.5.0 published in carrier → library → CLI order; each verified with crates.io `cargo info` |
| Qualified release source and immutable tag | Source `57ca94c910269e080b06aa8cb34c3767b3bf669d`; `v0.5.0` points to this SHA and was pushed once |
| Native five-target release | Hosted run `37181914252`: preflight, resolve, all five builds, clean-host qualifications, all five validations, required gate, aggregate, and stage succeeded |
| Exact release inventory | Draft staging receipt `release_id=v0.5.0`, source SHA matched, `uploaded=15`, `reused=0`; `scripts/release-check-assets.sh --dir=...` passed |
| Binary identity | Downloaded public `stegoeggo-x86_64-apple-darwin` reports `stegoeggo 0.5.0` |
| Public updater A→B | Isolated public v0.4.2 macOS x86_64 asset updated to 0.5.0; no endpoint overrides; exact release asset and sidecar acquired and verified; no Cargo fallback |
| Post-update CLI smoke | Version/help and protect→inspect→verify passed; second update returned already-current |
| Curl independence | Updater and command smokes passed with an empty `PATH` |
| Future plan reconciliation | Plan 106/M001 and M002 live gates closed; M006 remains blocked only on ADR-0007 acceptance; M007 remains proposed |

Crates.io package records: [stegoeggo-stego 0.5.0](https://crates.io/crates/stegoeggo-stego/0.5.0),
[stegoeggo 0.5.0](https://crates.io/crates/stegoeggo/0.5.0), and
[stegoeggo-cli 0.5.0](https://crates.io/crates/stegoeggo-cli/0.5.0). All were
published and resolvable on 2026-10-04; the final CLI `cargo info` readback
passed before tagging. GitHub release publication completed at
`2026-10-04T06:34:19Z`.

## 3. Production implementation evidence

The source release contains the 0.5.0 version/lockfile set, dated changelog,
stability and security documentation updates, corrected release workflows,
and the previously qualified carrier and library work. No Python, Node, or
C binding registry publication occurred; the C ABI remains 1.0. No source
changes were made after the crates were published or after the tag was pushed.

## 4. Verification executed (exact commands + results)

- `./scripts/release-check.sh --stage=pre` — pass on source SHA.
- `./scripts/release-check.sh --stage=root` — pass; full tests and root package verification.
- `./scripts/release-check.sh --stage=cli` — first run had one
  `validator_temp_state_is_cleaned` failure under parallel integration-test
  load; isolated rerun passed, then the entire CLI stage passed, including
  all workspace tests and CLI package verification.
- `cargo test -p stegoeggo --test release_eggpack validator_temp_state_is_cleaned -- --nocapture` — 1 passed.
- `cargo publish -p stegoeggo-cli --dry-run --locked` — pass; actual
  `cargo publish -p stegoeggo-cli --locked` — published successfully.
- `cargo info --registry crates-io stegoeggo-cli@0.5.0` — pass.
- `./scripts/release-binary-preflight.sh --tag=v0.5.0` — pass, including fmt, clippy, and workspace test suite.
- `gh workflow run release-binaries.yml --ref v0.5.0 -f release_tag=v0.5.0` — hosted run `37181914252` success.
- `gh run download 37181914252 -n eggpack-staging-receipt ...` — receipt source matched release SHA; draft staged with 15 uploaded assets.
- `./scripts/release-check-assets.sh --dir=<downloaded-assets>` — pass: five binaries, SHA-256 sidecars, wrappers, exact installers, and manifest.
- `stegoeggo-x86_64-apple-darwin version` from the public draft — `stegoeggo 0.5.0`.
- `./scripts/test-release-updater.sh` — pass at 0.5.0.
- Public v0.4.2 binary `version` — `stegoeggo 0.4.2`; production `update` — `Updated 0.4.2 -> 0.5.0.`
- Post-update `version`, `--help`, protect/inspect/verify, second update — all pass; second update reports 0.5.0 already current.
- Updater and CLI smoke commands with empty `PATH` — pass; curl-independent.
- Prior specialized gates recorded before publication: semver checks, MSRV package, docs.rs-equivalent, strict external metadata conformance, cargo-deny, Python/Node/C release qualification, hosted binding CI, and 60-second JPEG parser fuzz all passed; see M005 implementation evidence and release run records.

## 5. Invariant review

All release invariants hold: 0.5.0 crate lockstep and exact dependencies;
ordered manual crates.io publication; immutable tag; Eggpack draft-first
five-target process; 15-file inventory; no registry publication credentials
in automation; C ABI 1.0; no PyPI/npm/C bundle publication; crates.io remains
the updater version authority. No source/package bytes changed after
publication. The first parallel-test cleanup failure was transient: the
focused test and full serial release stage passed on the same source.

## 6. Failure and recovery review

M004 stopped before publication because two stable public enum changes were
not semver-compatible with 0.4.3. The maintainer selected 0.5.0; no 0.4.3
artifact was created. All three 0.5.0 crates published from the qualified
source. Eggpack staged exact assets without reuse/collision recovery; the
audited draft was published. No recovery action remains.

## 7. Migration and compatibility review

0.5.0 is the next 0.x minor boundary and contains the stable enum additions
that require exhaustive Rust matches to add arms. Release notes describe this
migration. M008's legacy JPEG compatibility domain and explicit bounded APIs
remain intact. Carrier vectors, metadata bytes, updater policy, and C ABI
remain unchanged.

## 8. Security review

Crates.io packages and release assets were fetched over their configured
production HTTPS endpoints. Updater SHA-256 sidecar, identity, version,
ownership, and replacement gates passed. No secret values were recorded.
Prior advisory/license scans passed; no unresolved critical/high release or
carrier finding was identified.

## 9. Documentation and operations

`CHANGELOG.md` has the dated 0.5.0 entry. `RELEASING.md`, `STABILITY.md`,
`SECURITY.md`, and release distribution docs reflect 0.5.0 and its manual
publication boundaries. The public release includes all Eggpack-generated
installers and binaries. The 0.4.2→0.5.0 operational evidence is recorded in
`plans/106-status.md`.

## 10. Unresolved findings (critical/high/medium/low)

- None critical/high/medium/low identified in the release scope.
- Windows A→B updater was not performed; the plan requires one supported
  target, which macOS x86_64 satisfies. No Windows transition is claimed.

## 11. Roadmap disposition

Release-Distribution M001, M002, M003, and M005 are closed. M004 is
superseded by M005 and retains its accurate pre-publication stop record.
There are no remaining active release-distribution milestones; the subsystem
is closed. The unrelated Stego Library Evolution M006 remains blocked solely
on ADR-0007 acceptance; M007 remains proposed. No future milestone was
unblocked by this release beyond the shared live gates explicitly closed.

## 12. Registry updates

`plans/106-status.md`, `plans/closure/release-distribution/002-status.md`,
`plans/registry.md`, and `plans/subsystems/release-distribution-roadmap.md`
record the successful shared release evidence. The registry marks M001,
M002, M003, and M005 closed, M004 superseded, and the subsystem closed.
