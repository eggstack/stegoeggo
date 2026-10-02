# Maintenance Quality Milestone 001 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/maintenance-quality/001-release-workflow-consolidation.md`
Source subsystem roadmap: `plans/subsystems/maintenance-quality-roadmap.md#7-milestones`
Repository baseline reviewed: `d5425d3c73e634fd2276d44507fa94728a18784f` (plan baseline; diff to implementation shows plans-only drift, no code change)
Implementation commits: `f9bee3d` (shared bootstrap actions, SHA pins, contract checker, docs)
Closing commit: this record plus roadmap/registry reconciliation (see git log for `plans: close maintenance-quality M001 release workflow consolidation`).
Final qualification: `release-c` run `37045104837` on immutable SHA `f9bee3d8aa338f9ce996ac2300093870fedb8643` — 11/11 jobs green.
`https://github.com/eggstack/stegoeggo/actions/runs/37045104837`

## 1. Executive finding

Release/binding workflow maintenance is consolidated without changing any
runner, target, artifact, smoke, or publication contract. Every
third-party action in the four release/binding workflows is full-SHA
pinned with an adjacent upstream-release comment; the duplicated
verified-Zig/cargo-zigbuild bootstrap has one maintained source in two
local composite actions consumed by `release-c.yml`; matrices, artifacts,
and smokes are byte-for-byte equivalent before and after; `release-c`
qualifies green on Linux, macOS, and Windows; nothing was published. M001
closes. The optional M002 drift-linting polish remains proposed and still
needs an explicit maintainer decision on CI placement.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Inventory every external `uses:` and pin to a verified full SHA (§6.1) | `scripts/check-release-workflow-contract.py` asserts all 33 `uses:` across the four workflows; pin table in `.github/actions/README.md` (checkout v4.4.0, upload-artifact v4.6.2, download-artifact v4.3.0, setup-python v5.6.0, setup-node v4.4.0, pnpm v4, msvc-dev-cmd v1, dtolnay shared repo pin); SHAs verified 2026-10-02 via `git ls-remote` against upstream release tags (§4) |
| Smallest abstraction: composite actions plus checked shell logic (§6.2) | `.github/actions/install-cargo-zigbuild/action.yml` and `.github/actions/provision-verified-zig/action.yml`; no reusable workflows (no whole-job duplication genuine enough to justify them) (§3) |
| Target-specific build commands stay visible (§6.3) | All `cargo build/zigbuild`, Eggpack capture, smoke, and audit steps untouched; actions take explicit inputs (`zig-arch`, `zig-sha`, `work-dir`, `install-dir`) and only own the bootstrap (§3) |
| Centralize verified Zig 0.14.1 + cargo-zigbuild 0.23.3, keep arch checksums (§6.4) | `release-c.yml` Linux rows consume both actions; per-arch SHA-256 values stay in the workflow matrix as explicit `zig-sha` inputs; action adds traversal/single-top/version fail-closed checks the old C block lacked (§3) |
| Non-required contract checker/fixture (§6.5) | `scripts/check-release-workflow-contract.py` (manual only, never wired into CI); asserts pins, five-way matrices, artifact names, and toolchain versions; green (§4) |
| Pin update procedure obvious (§6.6) | Adjacent `# <release>` comments on every pin plus `.github/actions/README.md` procedure; `RELEASING.md` maintenance section and `architecture/tooling.md` inventory (§9) |
| Full-SHA pinning everywhere in scope (§13) | Contract checker green; `git diff` shows only `uses:`-line changes plus the two intended C step migrations (§3, §4) |
| One maintained bootstrap source (§13) | Zig/cargo-zigbuild logic exists once (two action definitions); `release-binaries.yml` deliberately excluded (see §5) (§3) |
| Target/artifact/smoke contracts unchanged (§13) | Before/after matrix extraction identical (5/5/5/5 IDs); non-`uses:` diff limited to the C bootstrap migration; collector audit green in run `37045104837` (§4) |
| Representative workflow runs green, no publication (§13) | `release-c` run `37045104837` on `f9bee3d`: 5 builds + 5 clean smokes + collector audit, 11/11 green; manual-dispatch only, artifacts only (§4) |
| `check.sh` green (§11) | Exit 0 at implementation tree (§4) |

## 3. Production implementation evidence

Implementation `f9bee3d` (9 files, +435/−58; `release-binaries.yml` zero-diff):

- `.github/actions/install-cargo-zigbuild/action.yml` (new): exact `cargo install cargo-zigbuild --locked` into a caller-supplied isolated root, `test -x` gate, `CARGO_INSTALL_ROOT`/PATH export.
- `.github/actions/provision-verified-zig/action.yml` (new): checksum-pinned Zig download (`--proto '=https'`, 600 s cap), SHA-256 check, archive traversal/single-top/`zig`-binary assertions, `version` assertion, `CARGO_ZIGBUILD_ZIG_PATH` export, optional `CARGO_ZIGBUILD_CACHE_DIR`, PATH addition, `zig-bin`/`zig-dir` outputs.
- `.github/workflows/release-c.yml`: 8 moving tags pinned; inline cargo-zigbuild/Zig blocks replaced by the two actions (same `if: matrix.zigbuild`, same matrix SHAs).
- `.github/workflows/release-python.yml` / `release-node.yml`: 12/13 moving tags pinned (pin-only; one-line steps left visible per §6.2).
- `scripts/check-release-workflow-contract.py` (new, executable, manual-only).
- `.github/actions/README.md` (new): pin table, update procedure, drift-guard note.
- `RELEASING.md` + `architecture/tooling.md`: maintenance section and script inventory.

## 4. Verification executed (exact commands + results)

- `python3 scripts/check-release-workflow-contract.py` → `release workflow contract ok (4 workflows, pins, matrices, toolchains)`, exit 0. Pre-change run recorded 35 violations (baseline evidence).
- `eggpack ci check --workflow-shape ... --workflow .github/workflows/release-binaries.yml` → `ci check: match (55963 bytes)`, exit 0 (drift guard unaffected).
- `python3 scripts/check-release-contract.py` → `release target/asset contract passed`.
- YAML parse of all four workflows plus both actions → ok.
- Before/after matrix extraction (`build_wheels`, `verify_smoke`, `build_addon`, `build_c`, `smoke_artifact`) → identical five-way IDs.
- `./scripts/check.sh` → exit 0 (full workspace; Rust code untouched by this milestone).
- `release-c` run `37045104837` (dispatched 2026-10-02, `source_ref=f9bee3d`): 5 builds (linux-x86_64, linux-aarch64 incl. both shared-action Zig rows, macos-x86_64, macos-arm64, windows-x86_64 with pinned MSVC action), 5 clean smokes, collector audit — 11/11 green.
- `actionlint` not run (not installed in this environment); recorded rather than implied.

## 5. Invariant review

- Manual-only publication: no `on:` trigger changed; all four workflows remain `workflow_dispatch`-gated with no publish step added.
- Target matrices: identical before/after (see §4); glibc 2.17 floor, Rust 1.89/MSRV toolchain selections, cbindgen 0.29.4 gate, Python 3.11/cibuildwheel 2.22.0/maturin 1.5.0, Node 22/24/26 smokes, pnpm 12.6.0 all asserted by the contract checker.
- Required CI unchanged: the contract checker is manual-only; `check.sh` untouched.
- Artifact identity: no asset name, path, or retention change; Eggpack byte-shape preserved exactly (55963-byte match).
- No secrets expansion: actions take only version/arch/SHA/dir inputs; Eggpack install rev and container policies untouched.

## 6. Failure and recovery review

Existing concurrency groups untouched. Shared actions fail closed: `set -euo pipefail` throughout; checksum mismatch aborts before extraction; archive traversal (`^/`, `..`) and multi-top layouts rejected; `zig` binary existence/size/executability plus exact `version` match asserted; cargo-zigbuild `test -x` gate retained. A bad action SHA would fail fast at step setup with an explicit resolution error. No partial-publication path exists (workflows upload inspection artifacts only).

## 7. Migration and compatibility review

No user-facing migration. Maintainer change: Zig/cargo-zigbuild fixes go to `.github/actions/` instead of two inline blocks; pin bumps follow `.github/actions/README.md` and update the contract checker in the same commit. `release-binaries.yml` maintenance stays with the Eggpack producer until it emits a shared reference.

## 8. Security review

Supply-chain surface reduced: 25 moving tags replaced by immutable SHAs verified against upstream release tags. Zig downloads remain HTTPS-pinned with checksum enforcement. Local actions run no network fetch beyond the pinned Zig/crates sources and perform no publication. No new permissions; `release-c.yml` keeps `contents: read`.

## 9. Documentation and operations

`RELEASING.md` (workflow-maintenance section), `architecture/tooling.md` (script inventory 12→13 plus shared-action sources), `.github/actions/README.md` (pin table + update procedure). `SUPPORT.md` unchanged (no evidence-matrix change). Python/Node workflows were pin-only and were not live-dispatched: the SHA swaps are mechanical (each SHA verified present upstream), YAML parse plus contract checker cover them, and release-c proves the shared-action and pin mechanics live on all three OS families.

## 10. Unresolved findings (critical/high/medium/low)

None. Two recorded limitations, neither blocking: `actionlint` unavailable locally (mitigated by strict YAML parse plus the green live dispatch); Python/Node live dispatches not run (mechanical pin-only change, rationale in §9).

## 11. Roadmap disposition

Maintenance-quality M001 closed. M002 (workflow contract/drift linting) remains proposed future polish; its M001 precondition is satisfied and only the maintainer CI-placement decision is outstanding. The roadmap completion definition is met (equivalent matrices/smokes plus immutable action references).

## 12. Registry updates

`registry.md`: M001 row removed from dependency-ready plans; subsystem row marked closed; M001 recorded under recently closed work. Roadmap status table marks M001 closed with this closure record.
