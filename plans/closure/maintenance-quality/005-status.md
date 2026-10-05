# Maintenance Quality Milestone 005 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/maintenance-quality/005-release-contract-test-isolation-and-readme-reverification.md`
Source subsystem roadmap: `plans/subsystems/maintenance-quality-roadmap.md#7-milestones`
Repository baseline reviewed: `f19985e58a609ae8714cda93ed17b1e08cd7c236`
Final qualification: `./scripts/check.sh` exit 0. A required-CI flake fixed; no
public API, version, publication, or CI placement change; nothing published.

## 1. Executive finding

The `validator_temp_state_is_cleaned` flake first reported in M003 and re-deferred
in M004 was a test-isolation defect, not a product defect. The test asserted that
the count of `stegoeggo-release-smoke-*` entries in the shared system temp
directory was unchanged across its own validator run. `scripts/smoke-release-binary.py:105`
creates one such directory for the lifetime of its subprocess, and
`validator_happy_path_with_fake_candidate` runs the same happy-mode validator
concurrently under Cargo's test harness. The sibling's live directory was therefore
counted as this test's residue.

The racer was identified by pairing the test with each sibling rather than by
inference, and the failure rate was measured by looping the binary rather than by
observing a single CI failure. Pre-fix: 8/60 at default 14-thread parallelism,
5/40 at 4 threads, 0/40 at 2 threads. Because `scripts/check.sh:16` runs
`cargo test --workspace --all-features`, this was a flake in the single required
gate, and 4-vCPU CI runners sit inside the measured failure band.

The fix scopes the observation instead of the timing. The child validator is given
a private temp root through `TMPDIR`/`TEMP`/`TMP`, which Python's `tempfile`
already honors, and the test counts entries there. `scripts/smoke-release-binary.py`
is byte-identical to HEAD, and the `assert_eq!(before.len(), after.len())` shape
is retained.

Re-verifying the M004 README by execution found the documentation substantially
accurate and produced one small fix: the Rust example computed `output` and never
used it, so copying it produced an `unused variable` warning.

## 2. Requirement-to-evidence matrix

| Plan requirement | Evidence |
|---|---|
| Reproduce and isolate the racer (WP1) | Full binary 60 runs at default parallelism: 8 failures, always `left: 0, right: 1`; the test alone 40 runs: 0 failures. Per-sibling pairing at 8 threads over 30 runs: happy 4/30, wrong-version 0/30, nonzero-help 0/30, alone 0/30 (§3) |
| Fix by observation scoping (WP2) | `run_validator_with_temp_root` sets `TMPDIR`/`TEMP`/`TMP` on the child; `validator_temp_state_is_cleaned` counts `scratch` under its own `tempfile::tempdir()`; the duplicated filter is now `smoke_temp_entries` (§3) |
| Script unchanged (WP3) | `git diff -- scripts/smoke-release-binary.py` empty after the teeth check restored it from a backup copy (§3) |
| Prove the fix (WP3) | Same 60-iteration loop, default parallelism: **0/60**; a single pass reports 15 passed, 0 failed (§3) |
| Prove the test still has teeth (§4, §13) | With the validator's `rmtree` replaced by `pass`, the test failed **10/10**; script restored byte-identical (§3) |
| `check.sh` green (§11, §13) | Exit 0 (§3) |
| Every README command runs verbatim (WP4) | protect / inspect / verify on a clean unmarked PNG; `--copyright-notice`, `--creator`, `--rights-url` confirmed to exist in `stegoeggo-cli/src/args.rs:125,132,146` and accepted at runtime (§3) |
| Preset table matches observed behavior (WP4) | All four presets re-run on a clean PNG: `legal-notice` → `MetadataNoticeOnly`; `legal-notice-with-stego` → `MetadataNoticeAndBestEffortStego`; `authenticated-provenance` and `maximal` → `Authenticated provenance: Verified` with the correct key, `Not verified` with a wrong key (§3) |
| Documented exit codes match (§13) | `inspect` unprotected 0, `verify` unprotected 3, `verify` protected 0, `--preset`+`--level` 2, `--dry-run` 0, missing file 1; the full table including 4 and 5 confirmed against the source mapping at `src/detached/verify.rs:168-178` (§3) |
| README Rust example compiles and runs (WP4) | Extracted verbatim from the README into a scratch crate; runs and its output verifies as `Rights notice: Found` / `MetadataNoticeOnly`, consistent with `metadata_only` (§3) |
| All internal links resolve (WP4) | Every relative link target in `README.md`, `docs/*.md`, `SUPPORT.md`, `STABILITY.md`, `DEPRECATIONS.md` exists (§3) |
| Doc touch-up (WP5) | `architecture/testing.md` release-contract row now names the per-test `TMPDIR` isolation (§3) |
| README fix (WP5, §12) | Rust example writes its `output` to `protected.png`, removing the warning and completing the example (§3) |
| `check-docs-contract.sh` green (§15) | "Documentation contracts valid: 5 release targets" (§3) |

## 3. Production implementation evidence

No production code changed. The functional diff is confined to
`tests/release_eggpack.rs`:

- `run_validator` now delegates to `run_validator_with_temp_root`, and only the
  latter builds the command. With no temp root the three existing callers behave
  exactly as before; with one, the child receives `TMPDIR`/`TEMP`/`TMP`, so
  Python's `tempfile.mkdtemp` lands inside a directory only this test can write.
- `smoke_temp_entries` replaces the `read_dir` filter that was duplicated
  verbatim at both observation points.
- `validator_temp_state_is_cleaned` creates `scratch` under its own
  `tempfile::tempdir()`, counts there before and after, and keeps the original
  `assert_eq!(before.len(), after.len())` and the `stegoeggo-release-smoke-`
  prefix filter. Its `assert!(ok)` also now reports the captured output.

`scripts/smoke-release-binary.py` — unchanged; `git diff` is empty. The teeth
check modified it temporarily and restored it from a copy taken beforehand.

## 4. Verification executed (exact commands + results)

```text
# defect reproduction
$ BIN                                                   (default 14 threads)
  run 1  FAILED  assertion `left == right` failed  left: 0  right: 1
  run 6  FAILED   run 7  FAILED   run 30  FAILED
  run 33 FAILED   run 41 FAILED   run 42 FAILED   run 44 FAILED
  → 8/60 failed

$ BIN validator_temp_state_is_cleaned --exact   ×40    → 0/40   (passes alone)

# racer isolation, --test-threads=8, 30 runs each
  temp_state + happy            →  4/30 failed
  temp_state + wrong_version    →  0/30 failed
  temp_state + nonzero_help     →  0/30 failed
  temp_state alone              →  0/30 failed

# rate by harness parallelism (pre-fix)
  --test-threads=2 → 0/40      --test-threads=4 → 5/40
  --test-threads=8 → 3/40      default (14)    → 8/60

# after the fix
$ BIN                                                   → 15 passed; 0 failed
$ BIN  ×60 (default parallelism)                        → 0/60 failed

# teeth
  (rmtree replaced by `pass`)  validator_temp_state_is_cleaned ×10
    → leaked in 10/10 runs, `left: 0  right: 1`
  restored from backup; `git diff -- scripts/smoke-release-binary.py` → empty

# TMPDIR redirection is what makes it work
  (slow fake candidate, TMPDIR=/tmp/eggpriv2, sampled mid-run)
    /tmp/eggpriv2/stegoeggo-release-smoke-4amx5war   ← created in the private root
    system temp                                    → (none)
  after the run                                     → (cleaned up)

# README, verbatim, from a scratch directory (clean unmarked PNG)
$ stegoeggo protect image.png -o protected.png --rights-policy prohibited-ai-ml-training \
      --preset legal-notice --copyright-notice "© 2026 Example Artist. All rights reserved." \
      --creator "Example Artist" --rights-url "https://example.com/rights"
  protected.png                                                              exit 0
$ stegoeggo inspect protected.png   Rights notice: Found                     exit 0
$ stegoeggo verify  protected.png   Rights notice: Found                     exit 0

# preset table, observed evidence strength
  legal-notice               → Stego marker: Not found   MetadataNoticeOnly
  legal-notice-with-stego    → Stego marker: Found       MetadataNoticeAndBestEffortStego
  authenticated-provenance   → Authenticated provenance: Verified  (correct key)
  maximal                    → Authenticated provenance: Verified  (correct key)
  authenticated-provenance   → Not verified (integrity check failed)  (wrong key)

# exit codes
  inspect unprotected → 0      verify unprotected  → 3      verify protected → 0
  --preset + --level  → 2      --dry-run           → 0      missing file      → 1
  verify-manifest (malformed manifest) → 2
  source mapping: src/detached/verify.rs:168-178
    VerifiedTrusted 0 · VerifiedUntrusted 4 · InvalidConfiguration 2
    BindingFailure | SignatureFailure | EmbeddedReferenceFailure | KeyMaterialMismatch 3

# README Rust example, extracted verbatim into a scratch crate
  before the fix: warning: unused variable: `output`
  after  the fix: clean compile and run; protected.png →
    Rights notice: Found · Stego marker: Not found · MetadataNoticeOnly

# links and contracts
  every relative link target in README.md, docs/*.md, SUPPORT.md,
    STABILITY.md, DEPRECATIONS.md                              → all exist
$ ./scripts/check-docs-contract.sh
  Documentation contracts valid: 5 release targets
$ ./scripts/check.sh                                                            exit 0
```

## 5. Invariant review

- **The test still detects a real leak** — the central risk of isolating the
  observation is that the test stops seeing anything. Disabled-cleanup simulation
  failed 10/10, so it still sees.
- **Validator script byte-identical** — no production release-path file was
  adapted to suit a test; verified with an empty `git diff` after restore.
- **Sibling tests unchanged** — `validator_happy_path_with_fake_candidate`,
  `validator_wrong_version_fails`, and `validator_nonzero_help_fails` still call
  `run_validator` with no temp root and were not otherwise modified.
- **Assertion shape retained** — `assert_eq!(before.len(), after.len())` and the
  `stegoeggo-release-smoke-` filter are both preserved.
- **No public API, version, publication, or CI placement change** — the diff
  touches one test target, two documentation files, and the plan/closure records.
  `scripts/check.sh` is unchanged.

## 6. Failure and recovery review

The test change holds no persistent state: the private root lives under the
test's own `tempfile::tempdir()` and is removed with it. A deliberately chosen
alternative — a shared mutex serializing the validator tests — was rejected
because it would mask the shared-global assumption instead of removing it, and
would couple tests that are otherwise independent. If the validator is killed
mid-run, its directory is now confined to the private root rather than the system
temp, so an aborted test no longer perturbs anything outside its own fixture.

## 7. Migration and compatibility review

None required. The test's public behavior is unchanged; only the directory it
observes differs. `TMPDIR`/`TEMP`/`TMP` are set on a child process this test
spawns, not on the test process, so no other test or the harness itself is
affected. The three sibling tests are untouched.

## 8. Security review

No security-relevant change. Setting a temp root for a child process confines
that child's temporary files; it does not relax any sandbox, and the validator's
own cleanup and bounded-subprocess behavior are untouched. `TMPDIR` is a test-owned
directory, not attacker-influenced input, and the value is a `Path` produced by
`tempfile::tempdir()`.

## 9. Documentation and operations

`architecture/testing.md` (release-contract row names the per-test `TMPDIR`
isolation property, so the next agent does not re-introduce a shared-directory
assertion), `README.md` (the Rust example now writes its result), this plan, this
record, plus the registry and roadmap entries. `check-docs-contract.sh` is green,
so the installer/command/target wording contract is unaffected.

## 10. Unresolved findings

- **Low — `scripts/smoke-release-binary.py:138` uses
  `shutil.rmtree(tmpdir, ignore_errors=True)`,** which silently swallows a failed
  cleanup in the production validator. With M005's isolation, a genuine leak
  would now be reported by this test instead of being masked, but making the
  validator's own cleanup strict is a behavior change to a release-path script
  and was deliberately left out of scope. Owner: a future release-distribution
  milestone, with a maintainer decision.
- **Low — documentation still has no automated example execution.** This
  milestone again verified the README by hand, and again found a real papercut.
  A script that builds the CLI and runs the documented commands, asserting exit
  codes, would prevent recurrence, but adding it to required CI needs the same
  maintainer decision as M002. Unchanged from M004.
- **Low — M002 remains proposed.** No workflow-contract lint may be added to
  required CI without a maintainer decision. Untouched by this milestone.

## 11. Roadmap disposition

Maintenance-quality M005 is closed. The roadmap remains `closed`; M001, M003,
M004, and M005 are done and M002 remains proposed pending a maintainer decision
on CI placement. M004's §10 "Medium — required-CI flake persists" finding is
resolved by this milestone and needs no corrective successor.

## 12. Registry updates

`plans/registry.md`: the maintenance-quality row now reads "M001, M003, M004,
M005 closed; M002 proposed optional polish", and a Recently-closed entry records
this work. `plans/subsystems/maintenance-quality-roadmap.md` gains the M005
current-state paragraph, milestone entry, and status-table row. No other
subsystem's status or blocker changed, and no blocked item was newly satisfied.
