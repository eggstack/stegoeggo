# Plan 089 Status: Fuzz Assurance Corrective Closure

Status: COMPLETE

Baseline: `f7c397eceda4ec6922ce7444ff0e4559e6114497`

Failed assurance evidence: GitHub Actions Fuzz run `34340957485` (scheduled), `metadata_merge` link failure with unresolved `__sancov_gen_*` sanitizer-coverage symbols before fuzz execution.

## Required evidence

- [x] failing fuzz build reproduced or bounded compatibility diagnosis completed
- [x] exact failing Rust nightly / cargo-fuzz / target tuple recorded
- [x] clean-cache behavior checked
- [x] adjacent-nightly and cargo-fuzz compatibility probes recorded
- [x] interaction with repository `rust-toolchain.toml` audited
- [x] root cause or defensible compatibility boundary documented
- [x] fuzz toolchain selection made explicit/reproducible
- [x] no `continue-on-error`, blanket retry, or failure suppression introduced
- [x] manual and scheduled fuzz jobs use the intended effective toolchain
- [x] `scripts/check_fuzz_sync.sh` passes with all 12 canonical targets
- [x] all 12 fuzz targets build/enumerate under the selected tuple
- [x] `metadata_merge` executes a short smoke successfully
- [x] parser/pipeline representative smokes execute successfully
- [x] manual-dispatch fuzz workflow evidence is green
- [x] scheduled-equivalent 3-target smoke workflow evidence is green
- [x] required push/PR CI remains unchanged
- [x] `./scripts/check.sh` passes
- [x] `plans/086-status.md` receives concise corrective evidence
- [x] `plans/088-status.md` records Plan 089 as a closure prerequisite
- [x] Roadmap 081 remains open until Plans 088 and 089 are complete

## Implementation notes

Record here:

- Root cause: the repository release profile enables `lto = true`; on the
  Ubuntu x86_64 runner, cargo-fuzz's sanitizer-coverage instrumentation then
  leaves unresolved `__sancov_gen_*` symbols at link time. The failure occurs
  before fuzz execution. The failed scheduled run was `34340957485`, using
  rustc `1.100.0-nightly (4aa1fbcf4 2026-09-08)` (the floating nightly
  resolved on 2026-09-09), cargo-fuzz `0.13.2`, and target `metadata_merge`.
- Clean-cache and bounded compatibility checks covered that nightly plus
  adjacent 2026-09-07, 2026-09-10, and installed 2026-09-03 toolchains.
  macOS cannot reproduce the Linux linker failure, but the exact nightly
  builds successfully when `CARGO_PROFILE_RELEASE_LTO=false`. cargo-fuzz
  0.13.1 was also installed and its explicit aarch64 macOS build passed; its
  default x86_64 macOS target selection was unavailable on this host. The
  selected workflow version remains 0.13.2 because it is the current known-
  good version in the failing-run environment and the workflow is now
  explicitly pinned.
- The first corrected scheduled-equivalent run (`34504712007`) reached all
  three targets and exposed a separate `pipeline_bytes` panic in
  `strip_stego_owned_jpeg`: a malformed JPEG segment with declared length 0
  made the `pos + 4..segment_end` slice reverse its bounds. The minimized
  reproducer was 355 bytes. `strip_stego_owned_jpeg` now rejects segment
  lengths below the JPEG minimum of 2 with `ImageTruncated`, and
  `tests/robustness.rs` contains the regression. The reproducer now exits 0
  under the local ASan/libFuzzer harness.
- The corrected tuple is Rust `nightly-2026-09-07` plus cargo-fuzz `0.13.2`,
  with job-level `RUSTUP_TOOLCHAIN` and
  `CARGO_PROFILE_RELEASE_LTO=false` in `.github/workflows/fuzz.yml`. LTO is
  disabled only for fuzz binaries; ASan and libFuzzer coverage remain enabled,
  and product release builds retain LTO. No retry, suppression, or required-CI
  change was introduced.
- `fuzz/README.md`, `README.md`, `AGENTS.md`, `architecture/overview.md`, and
  the relevant local skills now document the tuple, compatibility boundary,
  and update policy. `smoke=true` on a manual dispatch exercises the exact
  rotating scheduled-smoke job for timely remote evidence.
- `scripts/check_fuzz_sync.sh` passed with all 12 canonical targets. A clean
  all-target build passed under the selected tuple and
  `CARGO_PROFILE_RELEASE_LTO=false`; `metadata_merge`,
  `payload_v3_parser`, and `pipeline_bytes` each completed a one-run
  libFuzzer smoke. `./scripts/check.sh` passed.
- Manual workflow run ID/result: `34506783317` — green on `6cf2d93`;
  `metadata_merge` built and completed the bounded run.
- Initial scheduled-equivalent smoke run ID/result: `34504712007` — reached
  execution on `7c5d149` but failed on the newly discovered `pipeline_bytes`
  panic; artifact downloaded and reproduced locally, then fixed.
- Scheduled-equivalent smoke recheck ID/result: `34506785941` — green on
  `6cf2d93`; the rotating subset was `metadata_merge`, `payload_v3_parser`,
  and `pipeline_bytes`, with 120 seconds per target and no crash artifacts.
- Required CI run/result after the correction: `34506386814` — success on
  `6cf2d93` (`CI / Check`).
- Implementation commit SHA: `6cf2d93` (final merge on `main`; this ledger
  closure is the follow-up commit).
- Plan 086 and Plan 088 ledger reconciliation: both record the failed
  discovery run, regression fix, and final green manual, scheduled-equivalent,
  and required-CI run IDs. Roadmap 081 remains open; Plan 088's independent
  timestamp and architecture closure work is still incomplete.

## Closure rule

Do not mark this plan complete solely because the workflow configuration was changed. Completion requires successful fuzz execution in both manual and scheduled-equivalent modes plus a green required repository gate.
