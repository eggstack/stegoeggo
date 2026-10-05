# Maintenance Quality Milestone 004 — CLI relative-path fix and README quickstart verification

Status: implemented
Repository baseline: `1f23b40f1861bcd2b70c78ade80714acae65aeed`
Source roadmap: `plans/subsystems/maintenance-quality-roadmap.md#7-milestones`
Primary class: bug fix (user-facing) + documentation

## 1. Objective

Verify the user-facing documentation by executing it, fix the product bug that
verification exposed, and make the README quickstart-first with detail delegated
to `docs/`.

## 2. Why this milestone is ready

Documentation review only becomes meaningful when the documented commands are
actually run. Building the CLI and executing the README quickstart revealed that
`stegoeggo protect image.png -o out.png` fails on every bare relative output
path, which is the most common invocation and the exact form the README
documents. Independent of the blocked release-distribution work and of proposed
M002 linting.

## 3. Current implementation evidence

Baseline `f44b6bb` state, established by building the CLI and running the
documented commands:

- `stegoeggo protect image.png -o image_protected.png …` → `Error: IO error:
  resolve output path: No such file or directory (os error 2)`, exit 1.
- The same command with an absolute `-o` path succeeded.
- The same command with a directory component (`-o sub/out.png`) succeeded.
- The default output path (no `-o`) also failed, so batch and single-file use
  were both affected.

Root cause: in `stegoeggo-cli/src/protect.rs::check_input_output_disjoint`, the
non-canonicalizing fallback used `output.parent().unwrap_or_else(|| Path::new("."))`.
For a bare file name, `Path::parent()` returns `Some("")` rather than `None`, so
the fallback never fired and `Path::new("").canonicalize()` failed with
`NotFound`. The same `parent()` pattern in `write_atomic` resolved the temp-file
directory to `""` as well.

CI could not see this: the CLI integration tests build inputs in
`tempfile::tempdir()` and pass absolute paths, so no test ever exercised a bare
relative output name.

## 4. Invariants that must not regress

`check_input_output_disjoint` must still reject an output that resolves to the
input (including via the inode/device check); a missing output *directory* must
still surface a real error rather than silently writing elsewhere; the atomic
temp-file write must still place its temp file next to the intended output;
no public API, version, or CI placement change.

## 5. Scope

In: `stegoeggo-cli/src/protect.rs` (two `parent()` sites + unit tests),
`README.md`, `SUPPORT.md` (missing `webp` feature row),
`docs/carrier-crate.md` (wrong codec dependency attribution).

Out: `docs/` guides otherwise (verified accurate), `architecture/` (no
architectural change), the other CLI tests, any new script or CI check.

## 6. Required production changes

1. Treat an empty `parent()` as `"."` in `check_input_output_disjoint` and
   `write_atomic`, so a bare relative name resolves against the current
   directory.
2. Add three unit tests: bare relative output resolves, `write_atomic` writes a
   bare relative name into the current directory, and same-file input/output is
   still rejected.
3. Rewrite `README.md` around a working quickstart, moving installation,
   feature-matrix, and platform detail behind links into `docs/` and
   `SUPPORT.md`.
4. Correct the two documentation inaccuracies found while verifying.

## 7. Ordered work packages

**WP1 — Reproduce and isolate.** Build the CLI, run the README quickstart
verbatim, and reduce the failure to `Path::parent()` returning `Some("")` for a
bare name. Confirm a test that passes alone but fails in the suite is unrelated
pre-existing flakiness (`tests/release_eggpack.rs`).

**WP2 — Fix.** Correct both `parent()` sites; keep the inode/device
collision check and the missing-directory error path intact.

**WP3 — Regression tests.** Add the three unit tests; confirm the bare-relative
test fails against the pre-fix code and passes after.

**WP4 — README rewrite.** Quickstart first; policy and preset tables kept inline
because they are the core vocabulary; installation, features, platforms, and
API detail delegated to `docs/` and `SUPPORT.md`; all links verified to resolve.

**WP5 — Documentation corrections.** Add the missing `webp` row to the SUPPORT.md
feature table; correct the carrier doc's `image-webp` attribution to the
`image` crate's `webp` feature.

## 8. Failure, cancellation, restart, contention semantics

The fix is a pure path-resolution change with no runtime state, safe to revert.
The rewrite of `write_atomic` must not change atomicity: the temp file must
still be created in the destination directory so `persist` stays on the same
filesystem.

## 9. Compatibility and migration

Strictly a bug fix — previously-failing relative-path invocations now succeed.
No API, flag, or output-format change. Consumers relying on the old failure are
not a real population, since the documented example never worked.

## 10. Required tests

The three new unit tests in `stegoeggo-cli/src/protect.rs`, plus the existing
`stegoeggo-cli` suite (`cargo test -p stegoeggo-cli`) and `./scripts/check.sh`.

## 11. Required verification commands

```bash
cargo build --release -p stegoeggo-cli --bin stegoeggo
cd <scratch> && stegoeggo protect image.png -o protected.png \
  --rights-policy prohibited-ai-ml-training --preset legal-notice …
stegoeggo inspect protected.png     # exit 0
stegoeggo verify protected.png      # exit 0
stegoeggo verify unprotected.png    # exit 3
./scripts/check.sh
```

## 12. Documentation updates

`README.md` (rewrite), `SUPPORT.md` (`webp` feature row), `docs/carrier-crate.md`
(codec attribution).

## 13. Acceptance criteria (externally observable)

- Every command in the new README runs verbatim from a scratch directory and
  produces the documented result and exit code.
- The README's Rust example compiles and runs unchanged.
- `stegoeggo protect in.png -o out.png` succeeds.
- `verify` still exits 3 for missing evidence and `inspect` still exits 0.
- All README links resolve to existing files.
- `./scripts/check.sh` exits 0.

## 14. Stop conditions (report rather than improvise)

Any remaining failure that would require changing a documented contract (exit
codes, policy semantics, evidence strength) is reported, not edited. Claims that
cannot be reproduced by running the code are reported as unverified.

## 15. Closure evidence required

`./scripts/check.sh` green; the executed README command transcript; the
pre-fix/post-fix test outcome for the new regression test.

## 16. Handoff notes

Closure record: `plans/closure/maintenance-quality/004-status.md`.
