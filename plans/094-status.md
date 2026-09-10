# Plan 094 Status

Status: NOT STARTED

Baseline: `27bdd3d429d663948021de43d3e6f818fa613319`
Depends on: Plans 091 and 092.

## Required evidence

- [ ] one-shot repeated-operation decode baseline recorded
- [ ] prepared JPEG ownership/borrowing API decision recorded before implementation
- [ ] reusable decoded-state core shared by one-shot and prepared APIs
- [ ] public prepared type exposes no header/coefficient/Huffman/F5 internals
- [ ] repeated prepared read operations use exactly one coefficient decode
- [ ] capacity/raw/framed/tiled parity with one-shot APIs verified
- [ ] failed prepared embedding leaves reusable state unchanged
- [ ] container preservation verified for prepared embedding
- [ ] `Send`/`Sync` disposition recorded from actual representation
- [ ] hidden parent `JpegSearchContext` reuse/reduction audited
- [ ] direct standalone consumer compiles without `application-support`
- [ ] `./scripts/check.sh` passes

Record selected signatures, decode/allocation evidence, privacy tests, parent-support disposition, and final commit SHA here during implementation.
