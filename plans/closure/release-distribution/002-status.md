# Release and Distribution Milestone 002 — Closure Status

Status: conditionally closed
Source implementation plan: `plans/implementation/release-distribution/002-eggpack-producer-adoption-and-second-consumer-qualification.md`
Source subsystem roadmap: `plans/subsystems/release-distribution-roadmap.md#milestone-2--eggpack-producer-adoption-and-second-consumer-qualification`
Repository baseline reviewed: `a8d20ce`
Implementation commits: `3b96fae` — feat: adopt Eggpack producer for native CLI releases (M002 implementation)

## 1. Executive finding

M002 implementation and cutover are landed: Eggpack is now the producer
authority for the five-target native CLI release via checked-in
`release/eggpack/` configuration plus the generated, drift-gated
`.github/workflows/release-binaries.yml` (exact immutable Eggpack pin
`56ed7e747fd39e4d6a32a9f1fe3e09dd44355069`). All locally executable
qualification is green: zero drift, contract parity, 15 new
`release_eggpack` tests, `check.sh`, CLI tests, installer rehearsal,
MSRV checks, deny, release smoke on the release-profile binary, and a
mock 15-asset audit (accept/reject paths). The remaining evidence is
operational, not code: no ordinary stable B newer than 0.4.2 has been
published (latest tag `v0.4.2`, crates.io `stegoeggo-cli 0.4.2`), and the
plan forbids throwaway versions. WP9 (first live Eggpack-produced stable
release) and WP10 (shared A-to-B updater proof) therefore remain
outstanding, shared with the existing M001/Plan 106 blocker. M002 is
conditionally closed on that named live evidence.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Static `release/eggpack/` config, five aliases, no embedded digests | 10 files in `release/eggpack/`; aliases `linux-x64`, `linux-arm64`, `macos-x64`, `macos-arm64`, `windows-x64`; no tag/SHA/digest in any static file |
| Build bindings `stegoeggo-cli`/`stegoeggo`, direct selector, `signatures` equivalence guarded | `build-bindings.toml` per-target direct bindings; `tests/release_eggpack.rs::cli_default_signatures_feature_equivalence_is_guarded`; preflight lockstep check |
| Pack/toolchain policy (Zig 0.14.1, cargo-zigbuild 0.23.3, glibc 2.17, native qualification) | `pack.toml`; `cross_toolchain_pins_preserve_glibc_floor`; official Zig digests in `github-policy.json` cross-tools |
| Core smoke `version` with finite bounds | `qualification-bindings.toml` argv `["version"]`, 10000 ms, 8192/8192 limits, all five targets |
| Bounded product validator + Linux GLIBC ceiling | `scripts/smoke-release-binary.py`; live smoke on release binary passes; fake-candidate happy/wrong-version/nonzero/cleanup tests; GLIBC parser accept/reject test |
| Product wrappers + generated exact installers | `installer-presentation.json` maps `packaging/install.sh` to `install.sh` and `packaging/install.ps1` to `install.ps1` plus generated `install-exact.sh`/`install-exact.ps1` |
| Generated workflow, draft-only, one writer, no clobber | `eggpack ci check` match (55963 bytes); exactly one `contents: write` (stage job); no `--clobber`/publish/tag-mutation; `release_tag` dispatch input; no push trigger |
| Preflight/asset audit consume Eggpack authority; 15-asset inventory | `release-binary-preflight.sh` derives targets from `distribution.toml`; `release-check-assets.sh` enforces 5 binaries + 5 sidecars + 2 wrappers + 2 exact installers + manifest; mock audit accepts 15, rejects extra/missing |
| Immutable drift guard | `.github/workflows/release-drift.yml` installs the exact pinned Eggpack revision and runs `eggpack ci check` + `check-release-contract.py` |
| Release docs describe draft-first sequence | `RELEASING.md`, `docs/installation.md`, `architecture/cli.md`, `STABILITY.md`, `AGENTS.md`, conventions skill updated |
| First normal post-cutover stable release (WP9) | OUTSTANDING — no B newer than 0.4.2 exists; throwaway versions forbidden |
| Shared B with Plan 106 A-to-B proof (WP10) | OUTSTANDING — same operational blocker as M001 |

## 3. Production implementation evidence

- `release/eggpack/`: `distribution.toml`, `pack.toml`,
  `build-bindings.toml`, `qualification-bindings.toml`,
  `consumer-validators.json`, `installer-presentation.json`,
  `install-policy.toml`, `github-template.json`, `github-policy.json`
  (Eggpack pin `56ed7e747fd39e4d6a32a9f1fe3e09dd44355069`, stegoeggo
  runner labels `ubuntu-latest` / `ubuntu-24.04-arm` / `macos-15-intel` /
  `macos-latest` / `windows-latest`), `workflow-shape.json`.
- `.github/workflows/release-binaries.yml`: regenerated (1059 lines,
  55963 bytes), the only active native binary producer; predecessor kept
  in Git history.
- `.github/workflows/release-drift.yml`: new Eggpack drift + contract
  guard on push/PR/dispatch.
- `scripts/smoke-release-binary.py`: bounded version/help/
  protect/inspect/verify validator with ELF-gated GLIBC_2.17 ceiling
  (`readelf` required for ELF, missing tool fails closed).
- `scripts/check-release-contract.py`: parity guard (contract expansion,
  workflow shape, pins, validators, installer/updater fragments).
- `scripts/release-binary-preflight.sh` / `scripts/release-check-assets.sh`:
  Eggpack-derived; `scripts/release-targets.txt` removed.
- `scripts/check-docs-contract.sh`: validates the generated workflow
  against the Eggpack contract on the required CI path.
- `tests/release_eggpack.rs`: 15 tests (config resolution, asset parity,
  updater mapping, feature guard, drift shape, validators, pins, GLIBC
  parsing, validator live behavior, audit inventory).
- Before/after authority map: producer facts moved from
  `release-binaries.yml` + `release-targets.txt` + preflight-embedded
  matrix into `release/eggpack/` (Eggpack authority); installer/updater
  runtime policy, crates.io authority, and human publication stay
  product-owned.
- Five-target parity matrix: triples, public asset names (`.exe` only on
  `x86_64-pc-windows-msvc`), runners, and updater/installer mappings are
  unchanged from the 0.4.2 predecessor.
- Cross-tool versions/digests: Zig 0.14.1
  (`24aeeec8…` x86_64 / `f7a654ac…` aarch64 official archive SHA-256),
  cargo-zigbuild 0.23.3, Rust stable, glibc floor 2.17.

## 4. Verification executed (exact commands + results)

- `eggpack ci generate` to scratch + `eggpack ci check` after cutover:
  `ci check: match (55963 bytes)` — pass, zero drift.
- `python3 scripts/check-release-contract.py`: `release target/asset
  contract passed` — pass.
- `python3 scripts/smoke-release-binary.py ./target/release/stegoeggo`
  (release profile, `stegoeggo-cli` 0.4.2): `release smoke passed:
  stegoeggo 0.4.2` — pass (Mach-O host, GLIBC gate correctly skipped for
  non-ELF).
- Mock 15-asset audit: accept with `--native-smoke` — pass; extra file —
  correctly rejected; missing sidecar — correctly rejected.
- `./scripts/check.sh`: pass (fmt, clippy `-D warnings`,
  no-default-features check, full workspace tests incl. 15 new
  `release_eggpack` tests, docs contract).
- `cargo test -p stegoeggo-cli --all-features`: 16 passed.
- `./scripts/test-release-installers.sh`: `Release installer tests
  passed`.
- `./scripts/release-check.sh --stage=pre --allow-dirty`: `Release check
  passed`.
- `cargo +1.89.0 check -p stegoeggo-stego --locked`,
  `-p stegoeggo --locked --all-features`,
  `-p stegoeggo-cli --locked --all-features`: pass (MSRV 1.89).
- `cargo deny check licenses` / `advisories`: pass.
- `bash -n packaging/install.sh`: pass.
- `./scripts/test-release-updater.sh`: FAILS at baseline too (exit 101,
  `eggup-eggfetch` nested-runtime panic) — proven pre-existing at
  untouched `8c89e8c` in an isolated worktree; not caused by M002 files.
  Recorded as an unresolved medium finding owned outside M002.
- Local Zig is 0.16.0 (pinned 0.14.1 provisioned only inside generated
  Linux jobs); the final-pin CargoZigbuild build and Windows
  double-build digest comparison are deferred to the live run on native
  runners. PowerShell installer parsing likewise awaits Windows
  evidence.

## 5. Invariant review

All §4 invariants hold: crate lockstep + exact `=X.Y.Z` deps + ordered
publication preserved; tags never moved by CI; automation publishes
neither crates nor the draft; five asset names and sidecars unchanged;
installer/updater fallback limited to unsupported-target/exact-404 with
checksum/identity/network hard failures; crates.io remains version
authority; Eggup transaction ownership untouched; glibc 2.17 floor
retained in config plus final-ELF inspection; no Python/Node
publication absorbed; no new Eggpack schema introduced (all 10 config
files use the already-qualified shape proven by the first consumer).

## 6. Failure and recovery review

Static/render failure stops before cutover (proven in scratch first).
Staging semantics are Eggpack-owned: partial failure retains the draft
for inspection; rerun reconciles exact assets and refuses differing
bytes (no `--clobber` anywhere in the generated workflow). Crates.io
partial-publication recovery is unchanged (immutable versions, never
overwrite). Tag movement remains prohibited. The pre-existing updater
rehearsal panic is a recovery-path defect outside M002 scope (see §10);
it does not touch release staging.

## 7. Migration and compatibility review

Public binary names are byte-identical; installer URLs, updater asset
construction, and user docs are unchanged. Inventory grows 12 to 15
(`release-manifest.json`, `install-exact.sh`, `install-exact.ps1`) as
producer evidence, not a naming break. Rollback before the first live
Eggpack release is a Git revert of `3b96fae`; after a live release,
producer changes require a corrective plan, never a silent legacy
restore.

## 8. Security review

Release attack surface narrows: immutable action pins and the exact
Eggpack tool revision are enforced in both the generated workflow and
the drift guard; Zig archives are SHA-256-verified over HTTPS with
downgrade denial; staging holds the sole `contents: write`; no secrets
in static config; bounded subprocess time/output in the validator;
GLIBC gate fails closed when `readelf` is absent for ELF candidates.

## 9. Documentation and operations

Updated: `RELEASING.md` (draft-first sequence, 15-file audit),
`docs/installation.md` (Eggpack contract source, staging note),
`SUPPORT.md` left intact (no runner/toolchain wording drift beyond the
already-documented matrix), `architecture/cli.md`, `STABILITY.md`,
`AGENTS.md`, `.skills/stegoeggo-conventions/SKILL.md`. New operator
entry points: `release/eggpack/` authority,
`scripts/check-release-contract.py`, `scripts/smoke-release-binary.py`,
`release-drift` workflow. Distinguished in docs: crates.io authority vs
Eggpack producer authority vs Eggup updater ownership vs product
wrapper policy vs human publication.

## 10. Unresolved findings (critical/high/medium/low)

- Medium (outside M002, pre-existing): `test-release-updater.sh`
  rehearsal panics in `eggup-eggfetch` (nested Tokio runtime), exit 101,
  reproduced at untouched baseline `8c89e8c`. Owner: updater runtime
  (not M002 release files). Must be resolved before WP10 live A-to-B
  proof can run green; does not block M002 cutover.
- Low (deferred to live run): final-pin Linux CargoZigbuild build +
  GLIBC ELF proof, Windows double-build digest comparison, PowerShell
  installer parsing on Windows evidence. Owner: first live
  Eggpack-produced release (WP9).
- None critical/high. No new Eggpack capability was needed; no stop
  condition triggered.

## 11. Roadmap disposition

M002 implementation/cutover is done; live operational evidence (WP9/WP10)
is outstanding and shares one release event with M001/Plan 106: the next
ordinary stable B newer than 0.4.2 serves as both M002's first live
Eggpack-produced release and, after publication, M001's real public
A-to-B self-update proof. No throwaway version may be cut for either.
M001 stays blocked on that same event; M002 is conditionally closed.
Subsystem remains active until both closures complete.

## 12. Registry updates

`plans/registry.md`: M002 moved to conditionally closed with this
closure record; M001/Plan 106 blocker unchanged (stable B newer than
0.4.2). `plans/subsystems/release-distribution-roadmap.md`: M002 status
table and current state updated to conditionally closed with live
evidence outstanding.
## 13. Post-closure updater corrective resolution — 2026-10-01

The medium consumer-owned updater-runtime finding recorded in §10 is resolved.
StegoEggo Release-Distribution M003 implemented the synchronous updater bridge
at `e611c913f97c804620f26c7514d59e0cb84d34cc` and closed at
`f80eebe3fb983a5e8b13f7e082fe573636d65756` with:

- `./scripts/test-release-updater.sh` green end to end;
- ten focused seam/runtime regressions covering the prior failure shape;
- direct Tokio moved from production dependencies to dev-dependencies;
- `eggpack ci check` still byte-matching the M002-generated workflow;
- hosted CI run `36902385826` green;
- hosted release-drift run `36902385784` green;
- no remaining critical/high/medium updater-runtime finding.

This addendum does not fully close M002. The sole remaining M002 condition is
operational: the next ordinary stable B > 0.4.2 must provide the first live
Eggpack-produced five-target draft/publication evidence. The same B can then
provide M001 / Plan 106's real public 0.4.2 -> B updater proof. No throwaway
release is authorized.
