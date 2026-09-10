# Plan 095: Generic Strided Pixel-Buffer LSB API

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/090-generic-carrier-v1-semantics-correctness-and-reuse-roadmap.md`

Depends on: Plans 092 and 093.

Audited baseline: `27bdd3d429d663948021de43d3e6f818fa613319`.

## 1. Objective

Make the LSB carrier usable with borrowed packed/strided image buffers without requiring callers to allocate or convert into `image::RgbaImage`, while preserving the exact finalized LSB mapping and keeping the current `RgbaImage` API as a convenience adapter.

This is an API portability plan, not a new LSB algorithm. For equivalent RGB/RGBA pixels, carrier slot ordering and resulting pixel bytes must match the established mapping exactly.

## 2. Supported initial buffer model

Support the two layouts directly demonstrated by existing carrier semantics:

- `Rgb8`: three participating channels per pixel;
- `Rgba8`: three participating RGB channels plus one non-carrier alpha channel that must remain unchanged.

Support row stride so callers can use padded camera/video/GUI/FFI buffers. Do not add BGR/BGRA, planar YUV, arbitrary channel masks, or generic pixel traits in the first version without an evidenced consumer need.

Conceptual types:

```rust
pub enum PixelLayout {
    Rgb8,
    Rgba8,
}

pub struct PixelView<'a> { /* private validated fields */ }
pub struct PixelViewMut<'a> { /* private validated fields */ }
```

Exact names are implementation-owned. Public fields should remain private so validation cannot be bypassed.

## 3. Constructor invariants

Constructors must validate with checked arithmetic:

- width/height representation;
- bytes per pixel implied by layout;
- `row_stride >= width * bytes_per_pixel` for non-empty rows;
- required backing length for the final row without overflow;
- zero-size behavior consistent with the existing `EmptyCarrier` contract;
- no indexing can exceed the supplied slice;
- padding bytes are outside the carrier and must never be modified;
- RGBA alpha bytes are outside the carrier and must never be modified.

The crate is `#![forbid(unsafe_code)]`; retain that invariant. A safe slice-based implementation is required. Do not introduce unsafe pointer APIs under this plan.

## 4. Carrier slot model

The current LSB capacity unit is an RGB carrier slot. Preserve this abstraction across layouts:

```text
available slots = width * height * 3
```

The logical carrier ordering must be pixel-major RGB order independent of physical row padding:

```text
pixel(0,0).R
pixel(0,0).G
pixel(0,0).B
pixel(1,0).R
...
pixel(width-1,height-1).B
```

For RGBA, alpha is skipped. For strided data, row padding is skipped. This means equivalent packed RGB and RGBA views should use the same logical slot index for the same RGB channel even though their physical byte offsets differ.

Do not change the permutation merely to accommodate buffer views. Plan 093's finalized V2/versioned mapping is authoritative.

## 5. API surface

The generic buffer path should support the same useful operation styles as the pixel carrier where technically appropriate:

- capacity query;
- raw extraction;
- raw in-place embedding;
- framed extraction;
- framed in-place embedding;
- tiled extraction;
- tiled in-place embedding;
- framed+tiled extraction/embedding.

A borrowed immutable view is sufficient for extraction/capacity; a mutable view is required for in-place embedding. Do not provide a cloning `embed` that returns a new arbitrary buffer unless there is a clear ownership type to return. Callers already own the buffer and can clone it themselves if desired.

The existing `RgbaImage` cloning/in-place APIs remain. Refactor them to construct/use the same private carrier-access core so there is only one mutation/extraction algorithm.

## 6. Implementation phases

### Phase 0 — map current `RgbaImage` semantics

Record current known-answer fixtures and exact logical-slot-to-pixel/channel mapping. Record every direct use of `RgbaImage::get_pixel*`, `crop_rgba`, width/height capacity calculations, and tiled subimage allocation.

### Phase 1 — validated views

Implement immutable/mutable validated view types with private indexing helpers. Keep indexing code small and independently tested. Derive/implement only appropriate traits; mutable views should not be `Clone` merely for convenience.

Provide getters for dimensions/stride/layout only where useful. Avoid exposing raw internal offsets that callers might treat as a stable algorithm contract.

### Phase 2 — carrier access abstraction

Refactor the LSB V2 core around a private operation-level accessor that can:

- read one logical RGB carrier slot;
- mutate one logical RGB carrier slot by ±1 while preserving boundary behavior;
- expose width/height/available slots;
- create bounded tiled views without allocating full cropped images when possible.

Do not make this accessor a public trait. The public API should remain concrete and discoverable.

### Phase 3 — direct view operations

Implement raw/framed/tiled functions for `PixelView`/`PixelViewMut`. Reuse Plan 092's validated configuration types and Plan 093's finalized mapping. Capacity failure must be checked before the first mutation and leave the buffer byte-for-byte unchanged.

For tiled extraction, avoid allocating a copied tile for each origin. Use a subview/window with checked coordinates where possible. Keep `max_origins` bounds and current candidate ordering/compatibility.

### Phase 4 — adapt `RgbaImage`

Change `RgbaImage` APIs to delegate through the same logical carrier core. Existing signatures and valid outputs must remain compatible through 0.x. Known-answer tests are the hard gate.

Do not make `image` optional as part of this plan unless a separate semver/dependency audit proves the public `RgbaImage` compatibility API can remain available under the repository's promises. Decoupling the algorithm from `RgbaImage` is the goal; dependency feature redesign is not required.

## 7. Required tests

Add deterministic tests for:

- packed RGB8 and RGBA8 known-answer equivalence at the RGB-channel level;
- `RgbaImage` output byte-for-byte equivalence with the generic RGBA view;
- stride exactly packed width;
- stride with 1-byte, cache-line-like, and large padding;
- every padding byte unchanged after embedding;
- every RGBA alpha byte unchanged;
- invalid stride rejected;
- truncated backing slice rejected;
- checked overflow for extreme width/height/stride combinations;
- zero-sized carrier semantics;
- raw and framed round-trips for RGB8/RGBA8;
- exact-capacity success and one-slot-short atomic failure;
- tiled/framed-tiled round-trips with padded rows;
- cropped/subview recovery with no tile-copy requirement where implemented;
- zero and maximum seeds;
- several redundancy values using Plan 092's validated type;
- V2 known-answer vector unchanged.

If Plan 093 introduces a versioned successor mapping, duplicate relevant tests for each supported mapping and make the selected mapping explicit in configuration.

## 8. Performance evidence

Record allocation and/or wall-time evidence showing the new path removes the mandatory `RgbaImage` conversion/allocation for an external pre-existing buffer. For tiled recovery, record whether copied-tile allocations were removed.

The performance claim should be narrow: “operate directly on caller-owned validated bytes,” not a broad throughput claim unless benchmark data supports it.

## 9. Acceptance criteria

- Generic safe borrowed packed/strided RGB8 and RGBA8 LSB APIs exist.
- Equivalent pixels use the same logical carrier mapping as the established `RgbaImage` path.
- Alpha and row padding are never carriers and are never mutated.
- All geometry arithmetic is checked and malformed views return structured errors.
- Capacity failure is atomic before mutation.
- `RgbaImage` and generic-view operations share one private mutation/extraction core.
- Tiled view operations remain bounded and avoid unnecessary crop copies where practical.
- `#![forbid(unsafe_code)]` remains intact.
- Direct standalone consumer examples compile without application dependencies.
- Focused tests, known-answer tests, Clippy, and `./scripts/check.sh` pass.

## 10. Non-goals

No unsafe FFI pointer API, no planar/YUV carrier, no BGR/BGRA matrix, no generic pixel trait, no GPU buffer API, no image decoder/encoder facade, and no algorithm-version change beyond what Plan 093 explicitly requires.
