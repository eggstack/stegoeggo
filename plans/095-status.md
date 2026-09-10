# Plan 095 Status

Status: NOT STARTED

Baseline: `27bdd3d429d663948021de43d3e6f818fa613319`
Depends on: Plans 092 and 093.

## Required evidence

- [ ] current logical RGB slot mapping/known-answer baseline recorded
- [ ] validated safe packed/strided RGB8 and RGBA8 view types implemented
- [ ] width/height/stride/backing-length arithmetic checked
- [ ] alpha and row padding are never carriers and remain byte-identical
- [ ] generic views and `RgbaImage` share one private LSB core
- [ ] raw/framed/tiled operations exposed where appropriate
- [ ] capacity failure remains atomic before mutation
- [ ] generic/RgbaImage known-answer equivalence verified
- [ ] tiled subview path avoids unnecessary crop allocation where practical
- [ ] no unsafe code or generic public pixel trait introduced
- [ ] external direct-carrier consumer fixture passes
- [ ] allocation/performance evidence recorded
- [ ] `./scripts/check.sh` passes

Record exact view constructors/layout semantics, invalid-geometry cases, equivalence fixtures, measured allocation disposition, and final commit SHA here during implementation.
