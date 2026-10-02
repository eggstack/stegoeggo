# Stego Library Evolution Milestone 004 — Transactional In-Place Allocation Hardening

Status: blocked  
Repository baseline: `d5425d3c73e634fd2276d44507fa94728a18784f`  
Source roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`  
Long-term requirements: `plans/000-long-term-specification.md#3-execution-invariants`  
Applicable ADRs: `ADR-0002`  
Primary class: polish

Blocker: M003 closure.

## 1. Objective

Reduce peak auxiliary memory for in-place/tiled LSB embedding while preserving the
existing guarantee that a failed in-place operation leaves the caller's buffer
byte-for-byte unchanged.

## 2. Why this milestone is ready

The behavior is already correct. The specific debt is isolated:
`lsb::embed_tiled_in_place` clones the full `RgbaImage` as rollback state.

## 3. Current implementation evidence

`stegoeggo-stego/src/lsb.rs::embed_tiled_in_place` snapshots `img.clone()`, calls
the shared tiled core, and restores the clone when embedding reports failure. Borrowed
pixel views already demonstrate atomic capacity-failure behavior without requiring a
full owned image conversion.

## 4. Invariants that must not regress

- Any returned non-success leaves caller bytes unchanged.
- Successful bytes remain compatible with current extraction/known-answer behavior.
- Alpha bytes and row padding remain untouched.
- No unsafe code.
- Overflow/capacity checks happen before unchecked indexing/allocation.

## 5. Scope

In: preflight/commit split or bounded mutation journal; shared mutation core for
`RgbaImage` and borrowed views where practical; peak-memory evidence.

Out: changing carrier mapping, redundancy, tiled seed derivation, or introducing
streaming image codecs.

## 6. Required production changes

First attempt to make the tiled mutation phase infallible after a complete checked
preflight: identify viable tiles, capacity, slot schedule, and all failure conditions
before the first write. If this can be proven, remove the full snapshot.

If mutation can still fail after writes, use a rollback journal that records only
changed carrier bytes/bits. Bound journal growth with checked arithmetic and prove its
worst-case memory. Do not replace a full-image clone with an unbounded journal that can
be larger than the image.

Keep one logical carrier mapping shared between `RgbaImage` and `PixelViewMut`.

## 7. Ordered work packages

WP1 benchmark/measure current peak auxiliary memory.  
WP2 formalize preflight versus mutation failure points.  
WP3 implement transactional strategy and parity tests.  
WP4 measure memory/runtime regression and document results.

## 8. Failure, cancellation, restart, contention semantics

The operation is synchronous and caller-owned; cancellation is not exposed. Every error
or `embedded == false` path must restore exact input bytes. Panic is not a rollback
mechanism.

## 9. Compatibility and migration

No API migration. This is an implementation optimization with byte-identical success
and failure semantics.

## 10. Required tests

Success parity with cloning path, insufficient-capacity unchanged buffer, injected
mid-operation failure through a test-only hook if a journal remains, tiled framed/raw
round trips, packed/strided view parity, alpha/padding preservation.

## 11. Required verification commands

```bash
cargo test -p stegoeggo-stego --all-features
cargo test --workspace --all-features --test public_stego_api
cargo bench -p stegoeggo --bench bench -- --noplot
./scripts/check.sh
```

If Criterion selection changes, record the exact benchmark command and before/after
peak-memory method in closure.

## 12. Documentation updates

Carrier rustdoc/README only if allocation guarantees are documented publicly; benchmark
notes in architecture/tooling docs.

## 13. Acceptance criteria

Full-image rollback clone is gone or justified by evidence as the lower-risk/lower-memory
choice; atomic failure is regression-tested; successful output is byte-compatible; peak
auxiliary memory is measured, not inferred.

## 14. Stop conditions

Stop if optimization requires changing mapping bytes, weakening rollback semantics, or
adding unsafe code.

## 15. Closure evidence required

Before/after memory figures, relevant payload/image sizes, output parity hashes/tests,
failure atomicity tests, `check.sh`.

## 16. Handoff notes

Correctness dominates allocation wins. A no-op closure is acceptable if measurement
proves the snapshot is preferable to every bounded alternative.
