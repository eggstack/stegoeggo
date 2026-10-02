# Stego Library Evolution Milestone 005 — Lossless WebP Byte Carrier

Status: blocked  
Repository baseline: `d5425d3c73e634fd2276d44507fa94728a18784f`  
Source roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`  
Long-term requirements: `plans/000-long-term-specification.md#2-canonical-api-invariants`, `#3-execution-invariants`  
Applicable ADRs: `ADR-0003`  
Primary class: capability

Blocker: M003 interface contract.

## 1. Objective

Add an opt-in generic still-lossless-WebP encoded-byte convenience API that decodes to
the existing LSB pixel carrier and re-encodes losslessly, without importing rights
metadata semantics or claiming preservation it cannot provide.

## 2. Why this milestone is ready

The generic LSB core and borrowed pixel views are mature. The current `image` stack
already provides VP8L lossless WebP encode/decode when its `webp` feature is enabled.
The missing work is format gating, bounded decode, contract design, and tests.

Current `image` documentation states `WebPEncoder::new_lossless` writes VP8L and the
encoder supports RGB8/RGBA8. The repo's root already uses this path for lossless WebP.

## 3. Current implementation evidence

`stegoeggo-stego` depends on `image = 0.25.6` with default features disabled and
only `png`/`jpeg` enabled. Generic WebP consumers therefore manually decode/encode.
The root has private WebP container parsing that MUST NOT simply be copied wholesale
into the carrier crate.

## 4. Invariants that must not regress

- Existing default feature/dependency behavior remains unchanged unless separately
  justified.
- Only still lossless WebP is accepted for this carrier facade.
- Lossy VP8 and animated WebP are rejected explicitly, not silently transcoded.
- Embedded pixel mapping matches the existing LSB carrier exactly.
- The API does not claim unrelated ICC/EXIF/XMP preservation unless implemented and
  tested.
- Root rights byte paths do not switch to this facade implicitly.

## 5. Scope

In: an optional `webp` carrier feature/module; bounded format probe; still VP8L
decode; raw/framed/tiled embed/extract convenience; explicit preservation contract.

Out: lossy WebP stego, animation embedding, rights metadata rendering, generic RIFF
editing toolkit, byte-identical WebP recompression.

## 6. Required production changes

- Add an opt-in carrier feature forwarding the required `image/webp` capability.
  Keep it off by default unless dependency evidence shows the codec is already
  unavoidable.
- Add a small `stegoeggo_stego::webp` operation facade. Reuse `lsb`/pixel core;
  do not fork permutation logic.
- Probe RIFF/VP8X structure sufficiently to distinguish still VP8L from VP8/animation,
  using M003 carrier limits for bounded chunk traversal.
- Decode to RGBA, embed/extract through the same LSB mapping, and encode VP8L lossless.
- Define metadata behavior explicitly. Minimum acceptable contract: output is a new
  lossless WebP carrier and unrelated container metadata preservation is not promised.
  If preservation is implemented, it requires dedicated round-trip tests and must not
  create a second rights-metadata implementation.
- Expose raw/framed/tiled convenience only where semantics map directly to existing LSB
  calls.

## 7. Ordered work packages

WP1 feature/dependency and RIFF probe.  
WP2 still-lossless raw/framed round trip.  
WP3 tiled path and bounded extraction.  
WP4 metadata/non-animation/lossy negative tests.  
WP5 direct-consumer docs and package-feature qualification.

## 8. Failure, cancellation, restart, contention semantics

Decode/probe/embed/encode is all-or-error and returns no partial bytes. Unsupported
lossy/animated inputs use a structured `StegoError` classification rather than
silently producing a different carrier domain.

## 9. Compatibility and migration

Additive opt-in feature. Existing consumers see no default dependency or behavior
change. The root application remains owner of rights metadata/container preservation.

## 10. Required tests

Lossless RGB/RGBA round trips; alpha preservation; raw/framed/tiled extraction;
insufficient capacity; lossy rejection; animated rejection; malformed RIFF/truncation;
limit exhaustion; output decodes as VP8L; direct consumer feature build.

## 11. Required verification commands

```bash
cargo test -p stegoeggo-stego --features webp
cargo test -p stegoeggo-stego --no-default-features
cargo package -p stegoeggo-stego --allow-dirty --list
./scripts/check.sh
```

Record `cargo tree -p stegoeggo-stego --features webp --edges normal` in closure.

## 12. Documentation updates

Carrier README/rustdoc, `docs/carrier-crate.md`, `STABILITY.md` if promoted stable,
direct consumer/example, architecture module map.

## 13. Acceptance criteria

A direct carrier-crate consumer can embed/extract arbitrary bytes from a still VP8L file
without manually handling pixels; lossy/animation behavior is explicit; default builds
do not gain accidental codec/dependency surface.

## 14. Stop conditions

Stop if the only implementation path duplicates root rights metadata/container logic,
silently converts lossy/animated input, or requires unsafe/native libwebp.

## 15. Closure evidence required

Feature/dependency tree, negative-format matrix, round-trip tests, package contents,
MSRV impact, `check.sh`.

## 16. Handoff notes

Pure-Rust alternatives such as `webp-rust` are prior art, but adopting another codec
is out of scope unless the existing `image` feature cannot satisfy the contract.
