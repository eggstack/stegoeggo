# Plan 091 Status

Status: COMPLETE

Baseline: `27bdd3d429d663948021de43d3e6f818fa613319`

## Compatibility shape (recorded before behavior changes)

Additive 0.x transition; no existing public signature changed or removed.
Distinct methods were chosen over a policy/options enum or boolean flag so
an illegal strict+fallback combination is structurally inexpressible:

```text
strict operation (new)
    jpeg::embed_strict / jpeg::embed_framed_strict
    requested redundancy must fit exactly
    -> embedded output with actual_redundancy == requested
    OR Err(InsufficientCapacity) with no carrier output

best-effort operation (existing, retained byte-for-byte)
    jpeg::embed / jpeg::embed_framed
    selects largest feasible redundancy up to configured value
    -> report with actual redundancy, or embedded == false seed-hint carrier
    documented as the StegoEggo application compatibility path

seed-hint operation (existing signature, corrected contract)
    jpeg::embed_seed_hint
    -> either complete recoverable 96-bit hint (extract_seed_hint round-trips)
    OR Err(InsufficientCapacity { required: 96, available }) in hint-bit units
```

## Required evidence

- [x] JPEG public/parent call-site and byte-contract inventory recorded
- [x] seed-hint eligible-position preflight implemented
- [x] incomplete Q-table hint cannot return successful output
- [x] strict JPEG payload operation has exact-redundancy/no-fallback semantics
- [x] best-effort redundancy reduction is explicit
- [x] 0.x compatibility disposition recorded before behavior changes
- [x] raw/framed/tiled policy parity verified
- [x] parent best-effort/progressive compatibility regression-tested
- [x] JPEG carrier capacity/F5/progressive documentation corrected
- [x] container-preservation and decode-count tests pass
- [x] `./scripts/check.sh` passes (local full gate green, 2026-09-10)

## Implementation notes

- Call-site inventory: `src/protected/steganography/embed.rs:34,45,64,156`
  relies on best-effort `embed` and explicit `embed_seed_hint` progressive
  fallback; `extract.rs:779,798,1370` uses `extract_seed_hint` for seed
  discovery. No caller relied on partial-hint success. Tiled and
  progressive paths had no separate partial-hint defect beyond the shared
  Q-table helper, which is now transactional at the source.
- Preflight: `DctStegoF5::qtable_hint_capacity` counts exactly the eligible
  positions (`values[pos] >= 2` across tables 0..2) the extractor reads;
  `embed_seed_in_quantization_tables` rejects short tables with the new
  `TranscoderError::InsufficientHintCapacity { required: 96, available }`
  before mutating the header. Public mapping:
  `InsufficientHintCapacity -> StegoError::InsufficientCapacity`.
- Payload paths (`embed`, `embed_tiled`, `embed_strict`) attempt the seed
  hint via best-effort `try_write_seed_hint`: a short table never fails an
  otherwise successful payload embed, and previously unrecoverable partial
  hints are no longer written. `embed`/`embed_framed` output bytes are
  unchanged whenever the full 96-bit hint fits (the only previously
  recoverable case).
- Strict core shares the decode/probe path shape with `embed` but keeps its
  own decode call so existing coefficient decode-count tests are unaffected.
  `embed_f5` shrinkage failures inside strict map to `InsufficientCapacity`
  with the same required/available figures. Empty payloads are rejected
  with `InvalidConfig` in strict operations only.
- Tiled embedding was already exact (fixed redundancy 1, no fallback, no
  downgrade); rustdoc now states that contract explicitly.
- Corrected docs: JPEG capacity is eligible AC coefficients with
  `|coef| >= 2` after canonicalization (`jpeg.rs`, `lib.rs`, `error.rs`,
  `types.rs`, carrier README in Plan 097); the carrier is documented as an
  F5-style/no-zero-coefficient StegoEggo variant with no conventional-F5
  interoperability claim; stale `stego_f5.rs` progressive-support wording
  removed.
- Tests: F5-level exact-96 round-trip (seeds 0/42/MAX/magic), 95-position
  rejection with header-unchanged assertion, capacity-counter unit test,
  updated all-ones-table test (was asserting partial-success `Ok`);
  public-level strict fit/downgrade-refusal/no-output-on-shortage/
  empty-payload/framed-strict/progressive-rejection/seed-boundary tests.
  Decode-count tests from Plans 078/080 pass unchanged.
- Fixture counts: no known-answer/container-preservation fixture changed;
  `tests/public_stego_api.rs` untouched by this plan.
- Parent degradation disposition (with Plan 096): the synthetic
  table-less multi-scan fixture in `tests/jpeg_container_preservation.rs`
  caught the fallback-path behavior change during integration testing.
  `progressive_fallback` now maps hint `InsufficientCapacity` to an
  unmodified passthrough `UnsupportedProgressive` outcome (existing
  `ProgressiveJpegFallback` warning, metadata injection continues);
  malformed inputs still error. Light's `apply_qtable_seed_bytes` keeps
  propagating the error because the hint is its only channel.

Record final `./scripts/check.sh` result and implementation commit SHA here
during closure.

Implementation commit SHA: `d42f8ba` (roadmap 090 implementation on `main`; this ledger closure is the follow-up commit).
