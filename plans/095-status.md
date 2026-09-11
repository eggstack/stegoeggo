# Plan 095 Status

Status: COMPLETE

Baseline: `27bdd3d429d663948021de43d3e6f818fa613319`
Depends on: Plans 092 and 093.

## Required evidence

- [x] current logical RGB slot mapping/known-answer baseline recorded
- [x] validated safe packed/strided RGB8 and RGBA8 view types implemented
- [x] width/height/stride/backing-length arithmetic checked
- [x] alpha and row padding are never carriers and remain byte-identical
- [x] generic views and `RgbaImage` share one private LSB core
- [x] raw/framed/tiled operations exposed where appropriate
- [x] capacity failure remains atomic before mutation
- [x] generic/RgbaImage known-answer equivalence verified
- [x] tiled subview path avoids unnecessary crop allocation where practical
- [x] no unsafe code or generic public pixel trait introduced
- [x] external direct-carrier consumer fixture passes
- [x] allocation/performance evidence recorded
- [x] `./scripts/check.sh` passes (local full gate green, 2026-09-10; re-verified 2026-09-11 after follow-up below)

## Follow-up re-verification (2026-09-11)

- Added `PixelViewMut::extract_tiled`/`extract_tiled_framed` conveniences
  (delegate to `as_view()`), so both views expose the full
  capacity/raw/framed/tiled surface symmetrically. Additive only; no
  signature or mapping change.
- Added six view tests in `stegoeggo-stego/src/pixels.rs`: 1-byte and
  1024-byte padding preservation, exact-capacity success plus `max_len+1`
  atomic failure, zero/`u64::MAX` seeds on both layouts, RGB framed
  round-trip, aligned-crop tiled-framed recovery via views, and
  mutable/immutable tiled-extract parity.
- Docs pruning: `architecture/protected-steganography.md` now lists
  `pixels.rs` in module interactions and names borrowed views in
  operation styles; `architecture/overview.md` steganography row names
  the full operation-style set; `docs/carrier-crate.md` records the
  constructor contract and per-view method split;
  `.skills/stegoeggo-conventions/SKILL.md` records the new mutable-view
  tiled-extract conveniences. `README.md`/`AGENTS.md` left unchanged
  (carrier detail stays in `docs/carrier-crate.md`).
- `./scripts/check.sh` passes locally (exit 0, 2026-09-11).

## Implementation notes

- Baseline mapping: pixel-major RGB order, `slot -> (slot/3, slot%3)`,
  `(x, y) = (pixel_index % width, pixel_index / width)`, Plan 093 V2
  mapping authoritative and unchanged. Known-answer vector
  `tests/public_stego_api.rs:40-59` untouched and passing.
- Views (`pixels::PixelView`/`PixelViewMut`, `PixelLayout::Rgb8/Rgba8`):
  private fields, `new(bytes, width, height, layout, stride)` with fully
  checked arithmetic (`row_len = width*bpp`, `stride >= row_len`,
  `(height-1)*stride + row_len` against backing length, all
  `checked_*`). Backing longer than required is accepted (documented
  sub-slice use); shorter is `InvalidConfig`. Zero-size `(0,0)` views
  construct and mirror the `RgbaImage` `EmptyCarrier`/zero-capacity
  contract. No BGR/planar/trait abstraction added.
- One private core: `lsb_internal::{PixelCarrier, PixelCarrierMut}`
  (crate-internal traits, never public) with `RgbaImage`, view, and
  `TileWindow`/`TileWindowMut` implementations. `embed_v2_in_place_carrier`,
  `extract_v2_carrier`, `embed_tiled_carrier`, `extract_tiled_carrier`,
  `extract_tiled_framed_carrier` serve both representations; `RgbaImage`
  wrappers delegate. The bit rule lives in one pure
  `apply_lsb_bit(old, x, y)` helper used with absolute coordinates in both
  paths. Tiled search helpers moved from `lsb.rs` into `lsb_internal`
  unchanged.
- Coverage: view methods `capacity`, `embed`, `extract`, `embed_framed`,
  `extract_framed`, `embed_tiled`, `extract_tiled`,
  `embed_tiled_framed`, `extract_tiled_framed` (+ `as_view` reborrow).
  No cloning embed returning a new buffer: callers own the bytes.
  `Redundancy`-based configs work through the shared setters.
- Equivalence: packed RGBA view output is byte-for-byte identical to
  `lsb::embed_in_place` (asserted on raw bytes); RGB and RGBA views agree
  on all RGB channels; tiled view embed bytes equal the `RgbaImage` tiled
  path. Every padding byte (1-byte, cache-line-like 37-byte, and 16-byte
  cases) and every alpha byte verified unchanged after embedding.
- Tiled recovery no longer allocates a cropped copy per origin on either
  path (immutable tile windows); the per-candidate 5-pass seed history,
  neighbourhood, ordering, and `max_origins` bounds are unchanged.
- Allocation evidence: view paths touch only caller-owned bytes plus
  payload-proportional bit/byte vectors; no `RgbaImage` conversion or
  per-origin crop allocation exists in the view path by construction
  (`pixels.rs` production code has no `image::` dependency). Narrow claim
  only: operate directly on caller-owned validated bytes.
- `#![forbid(unsafe_code)]` intact (safe slice indexing with checked
  offsets; out-of-range coordinates fail gracefully as `None`).
- Direct-consumer fixture: carrier unit/integration coverage in Plan 097
  exercises views without `application-support`.

Record exact invalid-geometry cases, equivalence fixtures, measured
allocation disposition, final `./scripts/check.sh`, and implementation
commit SHA here during closure.

Implementation commit SHA: `d42f8ba` (roadmap 090 implementation on `main`; this ledger closure is the follow-up commit).
