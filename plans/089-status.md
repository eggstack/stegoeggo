# Plan 089 Status: Fuzz Assurance Corrective Closure

Status: READY FOR IMPLEMENTATION

Baseline: `f7c397eceda4ec6922ce7444ff0e4559e6114497`

Failed assurance evidence: GitHub Actions Fuzz run `34340957485` (scheduled), `metadata_merge` link failure with unresolved `__sancov_gen_*` sanitizer-coverage symbols before fuzz execution.

## Required evidence

- [ ] failing fuzz build reproduced or bounded compatibility diagnosis completed
- [ ] exact failing Rust nightly / cargo-fuzz / target tuple recorded
- [ ] clean-cache behavior checked
- [ ] adjacent-nightly and cargo-fuzz compatibility probes recorded
- [ ] interaction with repository `rust-toolchain.toml` audited
- [ ] root cause or defensible compatibility boundary documented
- [ ] fuzz toolchain selection made explicit/reproducible
- [ ] no `continue-on-error`, blanket retry, or failure suppression introduced
- [ ] manual and scheduled fuzz jobs use the intended effective toolchain
- [ ] `scripts/check_fuzz_sync.sh` passes with all 12 canonical targets
- [ ] all 12 fuzz targets build/enumerate under the selected tuple
- [ ] `metadata_merge` executes a short smoke successfully
- [ ] parser/pipeline representative smokes execute successfully
- [ ] manual-dispatch fuzz workflow evidence is green
- [ ] scheduled-equivalent 3-target smoke workflow evidence is green
- [ ] required push/PR CI remains unchanged
- [ ] `./scripts/check.sh` passes
- [ ] `plans/086-status.md` receives concise corrective evidence
- [ ] `plans/088-status.md` records Plan 089 as a closure prerequisite
- [ ] Roadmap 081 remains open until Plans 088 and 089 are complete

## Implementation notes

Record here:

- root-cause diagnosis and reproduction commands;
- failing and known-good toolchain tuples;
- workflow/script files changed;
- rationale for any dated-nightly or cargo-fuzz pin;
- update/bump policy for the fuzz tuple;
- all-target build/list result;
- focused fuzz smoke commands/results;
- manual workflow run ID/result;
- scheduled-equivalent smoke run ID/result;
- required CI run/result after the correction;
- implementation commit SHA;
- Plan 086/088 ledger reconciliation notes.

## Closure rule

Do not mark this plan complete solely because the workflow configuration was changed. Completion requires successful fuzz execution in both manual and scheduled-equivalent modes plus a green required repository gate.
