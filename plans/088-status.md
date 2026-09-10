# Plan 088 Status: Roadmap 081 Corrective Closure

Status: READY FOR IMPLEMENTATION

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

- [ ] timestamp provenance path audited from `ProtectionRequest` / resolved plan into metadata renderers
- [ ] structured JPEG COM marker no longer bypasses explicit timestamp override
- [ ] sibling metadata/notice renderers checked for equivalent direct wall-clock reads
- [ ] deterministic JPEG byte-equivalence regression added/tightened
- [ ] formerly flaky convergence test stress-run across second boundaries without failure
- [ ] PNG/WebP convergence remains green
- [ ] Plan 083 canonical verification architecture re-validated against current source
- [ ] `plans/083-status.md` reconciled with verified implementation state
- [ ] Plans 082-087 status/source evidence reviewed for roadmap closure
- [ ] Plan 089 fuzz-assurance corrective closure implemented with successful manual and scheduled-equivalent fuzz execution evidence
- [ ] `plans/086-status.md` updated with post-implementation fuzz-assurance corrective evidence
- [ ] `plans/081-status.md` reconciled and closure checklist updated
- [ ] integrated fmt/clippy/check/test gate passes
- [ ] `./scripts/check.sh` passes
- [ ] required push/PR CI is green after all corrective changes
- [ ] no deferred Roadmap 081 item accidentally enters scope

## Implementation notes

Record here:

- exact timestamp provenance bug and affected helper/call path;
- exact correction and why default-time behavior remains compatible;
- direct-time-read audit findings for JPEG/PNG/WebP/XMP/notice paths;
- regression test names and stress command/result;
- Plan 083 re-validation findings;
- child-plan status review summary;
- Plan 089 implementation/result and successful fuzz workflow run IDs;
- integrated verification commands/results;
- implementation commit SHA;
- final Roadmap 081 status.

## Closure rule

Do not mark this plan or Roadmap 081 complete by ledger inspection alone. The deterministic-output regression, Plan 089 fuzz-assurance correction, successful fuzz workflow evidence, and integrated repository verification gate must all pass on the corrected source first.
