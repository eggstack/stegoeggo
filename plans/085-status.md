# Plan 085 Status

Status: READY FOR IMPLEMENTATION

Baseline: `07bd05304a6942a843a76c28013a5bc0f08179cc`

## Required evidence

- [ ] all PNG/JPEG/WebP structural walkers inventoried
- [ ] canonical traversal selected per format
- [ ] resource accounting consumes shared traversal/observer facts
- [ ] duplicate `lib.rs` structural walker removed
- [ ] malformed-input and boundary tests added
- [ ] `ResourceUsage` compatibility checked
- [ ] relevant fuzz targets build/smoke-run where available
- [ ] repository-wide walker re-audit completed
- [ ] `./scripts/check.sh` passes

## Notes

Record traversal ownership, bounds/error semantics, test names/counts, fuzz disposition, and final commit SHA here during execution.
