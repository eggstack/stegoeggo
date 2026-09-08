# Plan 086 Status

Status: IMPLEMENTED

Baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`

## Maintainer decision (CI complexity guardrails)

`AGENTS.md` requires one required push/PR workflow and one required job. This
plan adds scheduled/manual assurance without touching that surface: `ci.yml`
is byte-identical, `scripts/check.sh` is unchanged, and the new/changed
workflows (`assurance.yml`, `external-verification.yml` schedule,
`fuzz.yml` smoke) are non-blocking, publish nothing, and react to no tags.
Scheduled-only triggers were chosen over per-PR triggers to keep ordinary
push/PR cost at one job. This ledger records that explicit decision.

## Required evidence

- [x] Rust 1.87 MSRV job added and passing
- [x] macOS aarch64 assurance added
- [x] Windows x86_64 assurance added
- [x] Linux aarch64 disposition recorded and implemented (native runner)
- [x] scheduled external-conformance execution added
- [x] scheduled bounded fuzz smoke added
- [x] required fast CI remains compact (ci.yml + check.sh unchanged)
- [x] manual release policy unchanged (no publish/tag/release workflows)
- [x] `SUPPORT.md`/`RELEASING.md` evidence matrix updated
- [x] local `./scripts/check.sh` still passes

## Workflows (final)

| Workflow | File | Trigger | Blocking | Jobs |
|---|---|---|---|---|
| CI | `.github/workflows/ci.yml` | push/PR to `main` | Yes (sole required check: `Check`) | `check` (stable, ubuntu-latest, `./scripts/check.sh`) — unchanged |
| Assurance | `.github/workflows/assurance.yml` | weekly Mon 06:00 UTC + `workflow_dispatch` | No | `msrv` (Rust 1.87, ubuntu-latest); `platform-macos` (stable, `macos-latest`); `platform-windows` (stable, `windows-latest`); `platform-linux-aarch64` (stable, `ubuntu-24.04-arm`) |
| External Verification | `.github/workflows/external-verification.yml` | monthly (1st 06:00 UTC) + `workflow_dispatch` | No | `verify` (unchanged steps) |
| Fuzz | `.github/workflows/fuzz.yml` | `workflow_dispatch` (single target) + weekly Wed 06:00 UTC (smoke) | No | `fuzz` (guarded to dispatch only); `smoke` (guarded to schedule only) |

Least privilege (`contents: read`), PR concurrency cancellation on `CI` and
`Assurance`, and existing action/cache usage are preserved; no new pinning
was introduced, consistent with repository policy. No workflow installs cargo
subcommands or apt packages in the required gate (unchanged). No fuzzing runs
on pull requests.

## MSRV matrix (job `msrv`, all `--locked`)

Local evidence, 2026-09-08, `rustc 1.87.0 (17067e9ac 2025-05-09)` /
`cargo 1.87.0 (99624be96 2025-05-06)` against the committed `Cargo.lock`
(version 4):

- `cargo check -p stegoeggo-stego` — pass
- `cargo check -p stegoeggo` — pass
- `cargo check -p stegoeggo --no-default-features` — pass
- `cargo check -p stegoeggo --all-features` — pass
- `cargo check -p stegoeggo-cli` — pass
- `cargo check -p stegoeggo-cli --all-features` — pass
- `cargo check --workspace --all-targets --all-features --exclude stegoeggo-fuzz` — pass
- `cargo test -p stegoeggo-stego` — pass
- `cargo test -p stegoeggo --lib --all-features` — pass (622 passed, 0 failed)

No transitive dependency has made 1.87 impossible: no pin and no MSRV bump
was needed. A future 1.87 break is policy-defined as an explicit
semver/toolchain decision (recorded in `SUPPORT.md`).

## Platform evidence

- macOS aarch64: the `platform-macos` job replays the required gate's
  compile+test commands on stable. Local equivalent host is
  `aarch64-apple-darwin` (stable 1.98.1); `./scripts/check.sh` passed locally
  2026-09-08 (exit 0, 43 `test result: ok` suites, 0 failures), covering both
  job commands. CI run evidence: see Assurance dispatch below.
- Windows x86_64 / Linux aarch64: no local runner available; job commands are
  shell-portable (`cargo check` / `cargo test`, no scripts, no apt). CI run
  evidence: see Assurance dispatch below.
- macOS x86_64: deliberately left untested — `macos-latest` runners are
  aarch64, and adding Intel runners is cost without signal. `SUPPORT.md`
  marks it "No (expected)" with this rationale.

## Fuzz smoke rotation

Weekly `smoke` job selects 3 of the 12 targets by week-of-year
(`START = (WEEK * 3) % 12`, wraparound slice over `cargo fuzz list | sort`),
120s each with the same `-max_len=4096 -timeout=10` bounds as manual
dispatch, uploading `fuzz/artifacts/` on failure (`if-no-files-found: warn`).
The target list is derived at runtime so it cannot drift; `scripts/check_fuzz_sync.sh`
still verifies the dispatch `inputs.options` list (passes: 12 targets
synchronized). Selection logic verified locally (`cargo fuzz list` returns
the 12 documented targets; week 37 → offset 3).

## External conformance schedule

Monthly `0 6 1 * *` added to `external-verification.yml`; steps unchanged.
Scheduled failure is signal only (documented in workflow header,
`SUPPORT.md`, and `RELEASING.md`).

## Documentation updates

- `SUPPORT.md`: toolchain section gains MSRV matrix + policy; platform table
  gains PR/Scheduled evidence columns; new Assurance Cadence table.
- `RELEASING.md`: new "CI Evidence: Blocking vs Scheduled" section; manual
  release policy unchanged.
- `README.md`: MSRV sentence notes scheduled assurance pointer.
- `AGENTS.md`: CI Pipeline section lists scheduled assurance; guardrails gain
  the scheduled-workflows informational-only rule; next plan number 081+ → 088+.
- `architecture/overview.md`: 4-workflow layout, Continuous assurance
  subsection, fuzz smoke note, `check_fuzz_sync.sh` row, test-file count
  30 → 35 (actual).
- `architecture/conformance.md`: workflow row + dispatch wording →
  monthly schedule + manual dispatch, non-blocking.
- `.skills/stegoeggo-conventions/SKILL.md`: architecture file count 30 → 31
  (actual); `.skills/plan-execution/SKILL.md`: next plan number 076+ → 088+.

## Local validation

- `python3 -c yaml.safe_load` over `.github/workflows/*.yml` — parse OK.
- `./scripts/check_fuzz_sync.sh` — 12 targets synchronized.
- `./scripts/check.sh` — exit 0 (2026-09-08, stable 1.98.1, macOS aarch64).
- MSRV matrix above — all pass on Rust 1.87.0.
- `git diff --stat scripts/check.sh` — empty (gate untouched).

## CI run evidence (post-push)

- Required `CI / Check` on the push commit: <record URL + result>.
- `Assurance` manual dispatch validating all four jobs (incl. Windows and
  Linux aarch64, which have no local runner): <record URL + result>.

## Notes

- Final commit SHA: <record after push>.
- No separate local-gate correctness issue was found, so `check.sh` was left
  unchanged per the plan.
- Non-goals respected: no release automation, no tag triggers, no benchmark
  gating, no coverage service, no OS/arch matrix beyond the four jobs above.
