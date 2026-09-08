# Plan 087 Status

Status: READY FOR IMPLEMENTATION

Baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`
Dependency: Plan 082

## Required evidence

- [ ] one canonical CLI `ProtectionRequest` builder/path
- [ ] modern/legacy precedence table implemented and tested
- [ ] contradictory combinations fail consistently
- [ ] single/batch/dry-run paths share request construction
- [ ] CLI source decomposed into cohesive private modules
- [ ] production dependency/root-feature call-site audit recorded
- [ ] unused/redundant features/dependencies removed or justified
- [ ] default and `signatures` builds/package checks pass
- [ ] v1 removal/migration inventory documented
- [ ] `./scripts/check.sh` passes

## Notes

Record before/after `cargo tree`, exact precedence rules, module layout, dependency removals/retentions, test counts, and final commit SHA here during execution.
