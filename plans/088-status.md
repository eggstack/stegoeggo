# Plan 088 Status: Roadmap 081 Corrective Closure

Status: COMPLETE

Baseline: `2bb496e1af8c75495b1ae630522c3afd48adfb45`

Additional closure dependency: Plan 089 (`plans/089-fuzz-assurance-corrective-closure.md`) must be complete with green fuzz workflow evidence before Roadmap 081 can be marked complete.

Plan 089 is tracked and complete in `plans/089-status.md`: manual run
`34506783317`, scheduled-equivalent recheck `34506785941`, and required
`CI / Check` `34507697595` are green on final implementation tree `40b274e`.
The earlier
scheduled-equivalent run `34504712007` found the malformed-JPEG regression
that was fixed. This plan and Roadmap 081 remain open for their independent
closure work.

## Required evidence

- [x] timestamp provenance path audited from `ProtectionRequest` / resolved plan into metadata renderers
- [x] structured JPEG COM marker no longer bypasses explicit timestamp override
- [x] sibling metadata/notice renderers checked for equivalent direct wall-clock reads
- [x] deterministic JPEG byte-equivalence regression added/tightened
- [x] formerly flaky convergence test stress-run across second boundaries without failure
- [x] PNG/WebP convergence remains green
- [x] Plan 083 canonical verification architecture re-validated against current source
- [x] `plans/083-status.md` reconciled with verified implementation state
- [x] Plans 082-087 status/source evidence reviewed for roadmap closure
- [x] Plan 089 fuzz-assurance corrective closure implemented with successful manual and scheduled-equivalent fuzz execution evidence
- [x] `plans/086-status.md` updated with post-implementation fuzz-assurance corrective evidence
- [x] `plans/081-status.md` reconciled and closure checklist updated
- [x] integrated fmt/clippy/check/test gate passes
- [x] `./scripts/check.sh` passes
- [x] required push/PR CI is green after all corrective changes
- [x] no deferred Roadmap 081 item accidentally enters scope

## Implementation notes

Record here:

- Exact bug: canonical `inject_bytes_from_plan()` created a minimal context and
  `generate_structured_com_marker()` read `SystemTime::now()` directly, so
  equivalent calls could disagree on the marker's Unix-seconds field.
- Correction: normalized `RightsNotice::notice_applied_at` is passed through
  JPEG injection to the structured marker and converted to the existing Unix
  seconds field. Requests without a resolved timestamp retain the wall-clock
  fallback; no wire format or default policy changed.
- Audit: PNG, WebP, XMP, and human-readable notice renderers have no equivalent
  lower-level time read. The only remaining wall-clock source is the shared
  notice-time default in `metadata_trap/common.rs` (plus unrelated seed and
  provenance defaults).
- Regression: `jpeg_deterministic_equivalence_survives_wall_clock_second_boundary`
  compares all three synchronous canonical APIs before and after a 1.1-second
  delay; `sync_variants_agree_{png,jpeg,webp}` remains green. Focused result:
  `cargo test --all-features --test request_aux_convergence` — 25 passed.
- Plan 083: current-main revalidation confirms canonical verification ownership,
  centralized projections, CLI use of canonical notice facts, and detached
  embedded-reference single-search behavior. Its ledger is now `COMPLETE`.
- Child review: Plans 082/085/087 are `IMPLEMENTED`; Plans 083/084 are
  `COMPLETE`; Plan 086 and its Plan 089 fuzz follow-up are complete with
  recorded workflow evidence. Roadmap 081 now records implementation commits
  and integrated closure evidence.
- Plan 089 evidence: manual run `34506783317`, scheduled-equivalent recheck
  `34506785941`, and required CI `34507697595` are green on final tree
  `40b274e`.
- Integrated verification: `./scripts/check.sh` passed with fmt, clippy
  `-D warnings`, minimal-feature check, all-feature workspace tests, carrier
  tests, CLI tests, soak tests, focused convergence suites, and doc tests.
- Implementation commit SHA: `6a0cb19` — `fix: close roadmap 081 determinism gap`.
- Required post-push CI: [run 34513562069](https://github.com/eggstack/stegoeggo/actions/runs/34513562069)
  — `CI / Check` passed in 4m7s on `6a0cb19`.
- Final Roadmap 081 status: `COMPLETE`.

## Closure rule

Do not mark this plan or Roadmap 081 complete by ledger inspection alone. The deterministic-output regression, Plan 089 fuzz-assurance correction, successful fuzz workflow evidence, and integrated repository verification gate must all pass on the corrected source first.
