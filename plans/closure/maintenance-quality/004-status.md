# Maintenance Quality Milestone 004 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/maintenance-quality/004-cli-relative-path-and-readme-verification.md`
Source subsystem roadmap: `plans/subsystems/maintenance-quality-roadmap.md#7-milestones`
Repository baseline reviewed: `1f23b40f1861bcd2b70c78ade80714acae65aeed`
Final qualification: `./scripts/check.sh` exit 0. One user-facing bug fixed; no public API, version, or CI placement change; nothing published.

## 1. Executive finding

Running the README instead of reading it exposed a real defect. The quickstart's
own first command — `stegoeggo protect image.png -o image_protected.png …` —
failed with `Error: IO error: resolve output path: No such file or directory`,
because `Path::parent()` returns `Some("")` for a bare file name, so the
`unwrap_or_else(|| Path::new("."))` fallback in `check_input_output_disjoint`
never fired and canonicalizing the empty path failed. Absolute `-o` paths and
paths with a directory component worked, which is why this survived: the CLI
test suite builds inputs in `tempfile::tempdir()` and only ever passes absolute
paths. The default output path (no `-o`) failed the same way, so single-file and
batch use were both affected.

The documentation itself was substantially accurate — the policy values, preset
names, rights-metadata flags, exit codes, and Rust API all matched the built
binary. Two smaller inaccuracies were found and corrected: the carrier guide
attributed the WebP codec to an `image-webp` dependency that does not exist (it
is the `image` crate's `webp` feature), and the SUPPORT.md feature matrix omitted
the `webp` feature. The README itself was accurate but long; it is now
quickstart-first, with installation, feature, platform, and API detail behind
links into `docs/` and `SUPPORT.md`.

## 2. Requirement-to-evidence matrix

| Plan requirement | Evidence |
|---|---|
| Reproduce and isolate (§7 WP1) | README command failed; absolute and `sub/`-prefixed variants succeeded; `Path::new("out.png").parent()` == `Some("")` and `Path::new("").canonicalize()` is `NotFound`, confirmed with a standalone Rust probe (§4) |
| Fix both `parent()` sites (§6.1, WP2) | `check_input_output_disjoint` and `write_atomic` now treat an empty parent as `"."` (`stegoeggo-cli/src/protect.rs:76-82`, `44-47`) (§3) |
| Preserve collision and error behavior (§4) | New test `same_file_input_and_output_is_rejected` passes; `bare_relative_output_path_resolves_against_the_current_directory` also asserts a missing output *directory* still errors (§4) |
| Add three unit tests (§6.2, WP3) | `bare_relative_output_path_resolves_against_the_current_directory`, `write_atomic_writes_bare_relative_names_into_the_current_directory`, `same_file_input_and_output_is_rejected` — all pass (§4) |
| Test genuinely catches the bug (§15) | Reverting only the fix reproduces the failure: `test result: FAILED. 93 passed; 1 failed`; restoring gives `94 passed` (§4) |
| Every README command runs verbatim (§13) | protect / inspect / verify, both stego and authenticated presets, all exit 0 from a scratch directory (§4) |
| README Rust example compiles and runs (§13) | Extracted verbatim into a scratch crate against the local path; compiled and produced a 1257-byte PNG (§4) |
| Exit codes still correct (§4, §13) | `verify` on an unprotected file exits 3; `inspect` on the same file exits 0; `--preset` with `--level` exits 2; wrong HMAC key exits 3 (§4) |
| README links resolve (§13) | All 13 link targets checked to exist (§4) |
| Documentation corrections (§6.4, WP5) | `SUPPORT.md` `webp` row added; `docs/carrier-crate.md` codec attribution corrected to the `image` crate's `webp` feature (§3) |
| `check.sh` green (§11, §13) | Exit 0 (§4) |

## 3. Production implementation evidence

`stegoeggo-cli/src/protect.rs`:

- `check_input_output_disjoint` now uses `match output.parent() { Some(p) if
  !p.as_os_str().is_empty() => p, _ => Path::new(".") }`. A bare relative output
  name now resolves against the current directory instead of failing to
  canonicalize. The inode/device collision check and the input-collision error
  are untouched.
- `write_atomic` received the same treatment for its temp-file directory, so the
  temp file is still created beside the destination and `persist` stays on one
  filesystem.
- Three unit tests added, covering the bare-relative case, the atomic-write
  destination, and continued rejection of same-file input/output.

`README.md` — rewritten to lead with a working quickstart (install, protect,
inspect, verify), keep the `--rights-policy` and `--preset` tables inline
because they are the core vocabulary, and delegate installation detail, the
feature matrix, platform support, API semantics, and stability/deprecation
information to `docs/`, `SUPPORT.md`, `STABILITY.md`, and `DEPRECATIONS.md`. The
byte-vs-`DynamicImage` warning is retained inline because it is the most common
way to silently lose rights metadata. Verified claim, not an assumption:
`process_image` takes and returns `DynamicImage`, which cannot preserve
file-level container metadata.

`SUPPORT.md` — added the missing `webp` row to the Cargo feature table.

`docs/carrier-crate.md` — corrected the WebP codec attribution from a
non-existent `image-webp` dependency to the `image` crate's `webp` feature
(`webp = ["image/webp"]` in the carrier manifest).

## 4. Verification executed (exact commands + results)

```text
# the defect
$ stegoeggo protect image.png -o image_protected.png --rights-policy … --preset legal-notice …
  Error: IO error: resolve output path: No such file or directory (os error 2)   exit 1
$ stegoeggo protect /abs/path/image.png -o /abs/path/out.png …                   succeeded
$ stegoeggo protect image.png -o sub/out.png …                                   succeeded
$ stegoeggo protect image.png …                      (no -o)                     failed identically

# root cause probe
$ Path::new("out.png").parent()        → Some("")      canonicalize → Err(NotFound)
$ Path::new("sub/out.png").parent()    → Some("sub")
$ Path::new(".").canonicalize()        → Ok
$ std::fs::create_dir_all("")          → Ok(())        (so only canonicalize is unsafe)

# after the fix — every README command, verbatim, from a scratch directory
$ stegoeggo protect image.png -o protected.png --rights-policy prohibited-ai-ml-training \
      --preset legal-notice --copyright-notice "© 2026 Example Artist. All rights reserved." \
      --creator "Example Artist" --rights-url "https://example.com/rights"
  protected.png                                                              exit 0
$ stegoeggo inspect protected.png    → "Rights notice: Found"                exit 0
$ stegoeggo verify  protected.png    → "Rights notice: Found"                exit 0
$ stegoeggo protect … --preset legal-notice-with-stego                        exit 0
$ stegoeggo protect … --preset authenticated-provenance --key 0011…eeff        exit 0
$ stegoeggo verify   auth.png --key 0011…eeff                                  exit 0
$ stegoeggo verify   auth.png --key ffff…ffff   → auth verification failed    exit 3
$ stegoeggo verify   image.png       → "no protection evidence found"        exit 3
$ stegoeggo inspect  image.png       → "NoNoticeFound"                       exit 0
$ stegoeggo protect  image.png -o x.png --preset legal-notice --level light
  → "Cannot combine --preset with --level/--profile"                          exit 2
$ stegoeggo protect image.png -o out.jpg  …   (DCT path)  → "Rights notice: Found"
$ stegoeggo protect image.png -o out.webp …   (LSB path)  → "Rights notice: Found"

# README Rust example, extracted verbatim into a scratch crate
  ok 1257 bytes                                                              exit 0

# regression tests
$ cargo test -p stegoeggo-cli --bin stegoeggo        → 94 passed; 0 failed
$ (fix reverted)                                      → 93 passed; 1 failed
                                                      validator_temp_state_is_cleaned unaffected
$ ./scripts/check.sh                                                                exit 0
```

## 5. Invariant review

- **Collision protection preserved** — `same_file_input_and_output_is_rejected`
  passes; the `dev`/`ino` inode check is unchanged.
- **Missing output directory still errors** — asserted explicitly in the bare
  relative test with a nested missing directory, so the fix does not silently
  relocate writes.
- **Atomicity preserved** — `write_atomic` still creates its temp file in the
  destination directory; only the empty-parent case is normalized, so
  `persist` remains on the same filesystem.
- **No public API, flag, or format change** — the diff touches one CLI source
  file and three documentation files.
- **No CI placement change** — `check.sh` is byte-identical.

## 6. Failure and recovery review

The fix is a pure path-resolution change with no persistent state and a clean
revert. The one risk was masking a genuine error behind the more permissive
parent handling, which is why the regression test asserts that a missing output
*directory* still fails. Verified by running against a missing directory.

## 7. Migration and compatibility review

None required. Previously-failing bare relative invocations now succeed; no
flag, output, or API changes. The pre-existing `tests/release_eggpack.rs`
flake observed during M003 verification was still present but unrelated to this
milestone (see §10).

## 8. Security review

No security-relevant change. The fix does not weaken the input/output collision
guard, which exists partly to stop a protect operation from overwriting its own
source. Correct behavior was verified with an explicit same-file test.

## 9. Documentation and operations

`README.md` (rewritten), `SUPPORT.md` (one added row), `docs/carrier-crate.md`
(one corrected sentence), this plan, this record, plus the registry and roadmap
entries. All 13 README link targets were checked to exist.
`check-docs-contract.sh` remains green, so the installer/command/target wording
contract is unaffected.

## 10. Unresolved findings

- **Medium — required-CI flake persists.** `tests/release_eggpack.rs::validator_temp_state_is_cleaned`
  still races sibling validator tests over the shared system temp directory
  (first reported in M003). It did not fail this milestone's `check.sh` run, but
  the underlying test-isolation defect is unchanged and still deserves its own
  corrective. Owner: a future release-distribution or maintenance-quality
  milestone.
- **Low — documentation still has no automated example execution.** This
  milestone found a shipped bug by running the README by hand; nothing in CI
  executes documented commands. A script that builds the CLI and runs the
  README quickstart, asserting exit codes, would prevent recurrence, but adding
  it to required CI needs the same maintainer decision as M002.
- **Low — the `webp` feature is now in SUPPORT.md but not yet exercised in a
  documented end-to-end CLI example.** The root byte paths do not route to the
  carrier WebP facade implicitly, so this is expected rather than a gap.

## 11. Roadmap disposition

Maintenance-quality M004 is closed. The roadmap remains `closed`; M001, M003,
and M004 are done, and M002 (workflow contract/drift linting) remains proposed
pending a maintainer decision on CI placement. The roadmap's ownership boundary
already covers documentation currency following M003, so no change was needed
beyond recording the milestone.

## 12. Registry updates

`plans/registry.md`: the maintenance-quality row now reads "M001, M003, and M004
closed; M002 proposed optional polish", and a Recently-closed entry records this
work including the user-facing bug fix. No other subsystem's status or blocker
changed.
