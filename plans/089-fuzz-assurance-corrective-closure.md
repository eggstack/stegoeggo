# Plan 089: Fuzz Assurance Corrective Closure

Status: READY FOR IMPLEMENTATION

Baseline: `f7c397eceda4ec6922ce7444ff0e4559e6114497` (`plans: add roadmap 081 corrective closure`)

Depends on: Plan 086 scheduled-assurance implementation and Plan 088 Roadmap 081 corrective closure.

## Purpose

Restore the scheduled fuzz assurance introduced by Plan 086 to a reproducibly green state so Roadmap 081 can close with all of its claimed assurance surfaces operational.

The scheduled fuzz workflow on run `34340957485` failed before executing any fuzz cases. The selected target set was `metadata_merge`, `payload_v3_parser`, and `pipeline_bytes`; compilation of `metadata_merge` failed at link time with unresolved sanitizer-coverage symbols such as `__sancov_gen_*` under `rustc 1.100.0-nightly (4aa1fbcf4 2026-09-08)` and `cargo-fuzz 0.13.2`.

This is an assurance-toolchain corrective plan. It must not weaken fuzzing, suppress workflow failures, or broaden into unrelated CI modernization.

## Verified failure mode

The failure is distinct from a fuzz-discovered crash:

- checkout and toolchain setup succeeded;
- `cargo-fuzz` installed successfully;
- target synchronization/selection succeeded;
- the first selected target failed while linking the libFuzzer/ASan instrumented binary;
- the linker reported unresolved `__sancov_gen_*` symbols;
- no crash artifact was produced because fuzz execution never began.

The implementation must first determine whether the break is caused by the current nightly compiler, `cargo-fuzz`/`libfuzzer-sys` compatibility, repository toolchain override interaction, or workflow invocation. Do not assume a root cause solely from the symptom.

## Scope

### A. Reproduce and isolate the compatibility failure

Reproduce the failing build using the same effective environment as run `34340957485`:

- Linux x86_64;
- nightly dated 2026-09-09 / compiler commit `4aa1fbcf4` where available;
- `cargo-fuzz 0.13.2`;
- target `metadata_merge`;
- the same sanitizer/fuzzer build path used by `cargo fuzz run`.

Record:

1. `rustc +nightly --version --verbose`;
2. `cargo fuzz --version`;
3. exact failing command/result;
4. whether the failure reproduces with an empty `target/` directory and no restored cache;
5. whether the same target succeeds with one or more adjacent dated nightlies;
6. whether changing only `cargo-fuzz` changes the outcome;
7. whether repository `rust-toolchain.toml` causes any command in `.github/workflows/fuzz.yml` to use stable when nightly is required.

Use a bounded compatibility probe, not an open-ended toolchain hunt. The goal is to identify a defensible known-good toolchain tuple and the narrow reason the current tuple fails.

### B. Make the fuzz toolchain deterministic

Once the failure is isolated, make `.github/workflows/fuzz.yml` use an explicit, reproducible compatibility tuple for both manual and scheduled fuzz jobs.

Preferred characteristics:

- pin the Rust nightly to a dated known-good nightly rather than floating `nightly` if the failure is a compiler/tooling regression;
- pin `cargo-fuzz` to the verified compatible version rather than relying on the newest crates.io release at every run;
- use the same effective nightly for target discovery and fuzz execution where nightly behavior matters;
- make toolchain selection explicit enough that `rust-toolchain.toml` cannot silently redirect fuzz commands to stable;
- keep the fix localized to fuzz assurance unless evidence demonstrates a shared workflow defect.

If the actual root cause is not a floating-nightly incompatibility, implement the smallest correction supported by the reproduction evidence and document why a nightly/version pin is unnecessary.

Do not add `continue-on-error`, `|| true`, blanket retries, failure suppression, or a workflow-level exemption that turns a broken fuzz build green.

### C. Preserve the assurance contract from Plan 086

The following behavior must remain intact:

- required push/PR CI remains unchanged and compact;
- fuzz remains non-blocking for merges and never runs on pull requests;
- manual dispatch still runs a caller-selected target for a bounded duration;
- scheduled smoke still rotates a bounded subset of 3 canonical targets;
- `scripts/check_fuzz_sync.sh` remains authoritative for dispatch target synchronization;
- fuzz failures remain visible as failed fuzz jobs;
- crash/corpus artifacts remain available when actual fuzz execution produces them;
- no publish/tag/release behavior is introduced.

If a dated nightly is pinned, document the update policy so the pin does not become permanent accidental infrastructure. A simple periodic/manual compatibility bump procedure is sufficient; do not add automated dependency-update machinery as part of this plan.

### D. Add a cheap preflight that distinguishes build failure from fuzz failure where practical

Consider a small workflow or script-level preflight that builds or minimally starts the selected fuzz target before entering the longer smoke loop, only if it materially improves diagnosis without duplicating expensive compilation.

The objective is diagnostic clarity, not another CI layer. It is acceptable to rely on `cargo fuzz run` directly if a separate preflight would merely duplicate the same build.

When the workflow fails before fuzz execution, logs should make it obvious that the failure is toolchain/build related rather than a discovered crashing input.

### E. Re-validate the full fuzz surface

After the correction:

1. run `scripts/check_fuzz_sync.sh`;
2. build/list all 12 fuzz targets with the selected compatibility tuple;
3. run a short smoke on `metadata_merge`, because it is the target that exposed the linker failure;
4. run short smokes on at least one parser target and `pipeline_bytes`;
5. run or dispatch the scheduled-equivalent 3-target smoke path;
6. verify a manual-dispatch-equivalent single-target invocation;
7. record the successful GitHub Actions run ID(s) in `plans/089-status.md`.

A full multi-minute fuzz campaign on all targets is not required for this corrective closure. The requirement is that every target can build under the supported fuzz tuple and both workflow modes demonstrably execute fuzzing.

### F. Reconcile Plan 086 and Plan 088 closure evidence

After the scheduled fuzz surface is green:

- append a concise corrective note to `plans/086-status.md` recording the post-implementation toolchain compatibility repair and evidence; do not rewrite its historical implementation record;
- mark Plan 089 complete only after successful workflow evidence exists;
- update `plans/088-status.md` to require and record Plan 089 completion before Roadmap 081 closure;
- only then allow Plan 088 to reconcile `plans/081-status.md` and mark Roadmap 081 complete.

If the fuzz workflow remains red for any unresolved reason, Plan 088 and Roadmap 081 must remain open.

## Verification gate

Run at minimum:

```bash
./scripts/check_fuzz_sync.sh
./scripts/check.sh
```

With the selected fuzz toolchain tuple, also run equivalent commands to:

```bash
cargo +<selected-nightly> fuzz list
cargo +<selected-nightly> fuzz run metadata_merge -- -runs=1 -max_len=4096 -timeout=10
cargo +<selected-nightly> fuzz run pipeline_bytes -- -runs=1 -max_len=4096 -timeout=10
```

Use the actual pinned invocation/version mechanism chosen by the implementation rather than mechanically preserving the placeholders above.

GitHub Actions evidence must include:

- one successful manual-dispatch fuzz run; and
- one successful scheduled-equivalent smoke run, preferably the actual `schedule` job or a temporary/manual invocation of the exact same smoke path if waiting for the calendar trigger is unnecessary.

Do not mark the plan complete from local-only evidence.

## Acceptance criteria

- [ ] root cause of the `__sancov_gen_*` linker failure is isolated and recorded
- [ ] fuzz toolchain selection is explicit and reproducible
- [ ] `metadata_merge` builds and executes under the selected tuple
- [ ] all 12 canonical fuzz targets can be enumerated/built without the observed linker failure
- [ ] manual single-target fuzz mode executes successfully
- [ ] scheduled 3-target smoke mode executes successfully
- [ ] failures are not suppressed or converted to success
- [ ] required push/PR CI is unchanged
- [ ] Plan 086 assurance policy remains intact
- [ ] `plans/086-status.md` records the corrective evidence
- [ ] `plans/089-status.md` records successful local and GitHub Actions evidence
- [ ] `plans/088-status.md` records Plan 089 as a closure prerequisite
- [ ] `./scripts/check.sh` remains green
- [ ] Roadmap 081 is not marked complete until this plan and Plan 088 are both complete

## Non-goals

- no new fuzz targets unless the diagnosis exposes a directly relevant missing regression target
- no redesign of the fuzz harnesses
- no migration away from cargo-fuzz/libFuzzer
- no required per-PR fuzzing
- no expansion of required CI
- no broad dependency upgrades
- no release automation
- no changes to watermark, metadata, carrier, verification, or CLI semantics
- no masking of nightly/toolchain failures

## Handoff guidance

Treat this as a reproducibility problem in the assurance layer. The clean closure state is not merely “the workflow YAML parses” or “a rerun happened to pass.” The implementer should leave behind a known-good fuzz tuple, a clear update policy, and successful execution evidence from both workflow modes.

Plan 089 is a closure dependency of Plan 088. Roadmap 081 should remain open until the timestamp determinism correction from Plan 088 and the fuzz-assurance correction from this plan are both verified.