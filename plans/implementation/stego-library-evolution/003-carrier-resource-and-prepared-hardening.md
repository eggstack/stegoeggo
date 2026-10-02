# Stego Library Evolution Milestone 003 — Carrier Resource and Prepared Hardening

Status: blocked  
Repository baseline: `d5425d3c73e634fd2276d44507fa94728a18784f`  
Source roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`  
Long-term requirements: `plans/000-long-term-specification.md#3-execution-invariants`  
Applicable ADRs: `ADR-0002`  
Primary class: invariant

Blocker: M002 closure.

## 1. Objective

Give generic carrier consumers an explicit bounded-untrusted-input contract and extend
`PreparedJpeg` only where repeated operations can reuse the single decoded coefficient
state without exposing codec internals.

## 2. Why this milestone is ready

The parent already has bounded parsing and the carrier already has internal parse limits,
`MAX_TILED_ORIGINS`, frame bounds, and a single-decode `PreparedJpeg`. The missing
piece is one carrier-owned public limits contract rather than importing parent policy.

## 3. Current implementation evidence

- `jpeg::inspect` accepts raw segment limits, but ordinary generic JPEG operations do
  not expose one coherent limits object.
- `PreparedJpeg::new` decodes once and exposes capacity/raw/framed/tiled extraction
  plus strict embed, while codec state remains private.
- Tiled search has explicit `max_origins <= 4096`; frame payload is capped at 16 MiB.
- Parent `ResourceLimits` is application/parser-wide and must not leak downward.

## 4. Invariants that must not regress

Limits are checked before allocation/work derived from untrusted lengths. Default
one-shot APIs retain existing behavior. Prepared and one-shot results remain identical.
Unsupported JPEG classification remains stable.

## 5. Scope

In: a new carrier-owned limits type with private fields/builders; bounded constructors
and operation variants; prepared-state parity where decode reuse is real; adversarial
tests.

Out: importing root `ResourceLimits`; exposing JPEG headers/coefficient maps; changing
existing defaults; making scheduled fuzz required CI.

## 6. Required production changes

- Introduce a carrier-owned `CarrierLimits` (name may adjust before first public
  release) with private fields and getters/builders. Cover at minimum input bytes,
  JPEG segment count/segment bytes, dimensions/pixels, frame bytes, and tiled-search
  bound where applicable.
- Provide `PreparedJpeg::new_with_limits` (or equivalent) and bounded JPEG one-shot
  variants without duplicating decode code.
- Thread limits through private header/coefficient decode paths before large work.
- Keep `jpeg::inspect` compatible; it may delegate through the new object.
- Add prepared operations only when they can reuse decoded state and exactly match the
  one-shot contract. Candidate: tiled exact embed from decoded state; do not add
  best-effort application degradation to `PreparedJpeg`.
- Map limit failures to `StegoError::ResourceLimitExceeded` with no secret/input dump.

## 7. Ordered work packages

WP1 inventory all allocation/work amplification points and define units/defaults.  
WP2 limits type + bounded parser plumbing.  
WP3 prepared-state helper extraction/parity.  
WP4 adversarial/regression/fuzz corpus additions.  
WP5 docs and direct-consumer examples.

## 8. Failure, cancellation, restart, contention semantics

Limit failure returns before output mutation/encoding where possible. Prepared handles
remain reusable after failed queries/embeds. No global mutable budget.

## 9. Compatibility and migration

All existing functions remain. New bounded APIs are additive. Root may later translate
its `ResourceLimits` into carrier limits privately, but the carrier never depends on
the root type.

## 10. Required tests

Boundary values for each limit, overflow geometry, oversized declared frame, excessive
JPEG segments, malformed/truncated JPEG, tiled-origin bounds, prepared reuse after
failure, one-shot/prepared byte parity, direct consumer compile/runtime tests.

## 11. Required verification commands

```bash
cargo test -p stegoeggo-stego --all-features
cargo test --workspace --all-features --test robustness
cargo test --workspace --all-features --test public_stego_api
cargo test --workspace --all-features --test known_answer_vectors
./scripts/check.sh
```

Run relevant fuzz targets manually when practical and record exact duration/result.

## 12. Documentation updates

Carrier README/rustdoc, `docs/carrier-crate.md`, architecture testing/tooling docs,
roadmap status.

## 13. Acceptance criteria

A network-facing generic consumer can bound carrier parsing through public API;
prepared-state reuse remains opaque and parity-tested; limit failures are deterministic.

## 14. Stop conditions

Stop if limits require exposing parser internals, changing existing default limits, or
creating a second incompatible resource-unit vocabulary with no documented translation.

## 15. Closure evidence required

Limit table, adversarial test evidence, prepared decode-count/parity evidence,
fuzz evidence if run, `check.sh`.

## 16. Handoff notes

Prefer builders/private fields from introduction so this new public type can evolve
semver-safely.
