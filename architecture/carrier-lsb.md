# Carrier LSB (Pixel Domain)

**Sources:** `stegoeggo-stego/src/lsb.rs` (~400 lines, public facade) · `lsb_internal.rs` (~2100 lines, `pub(crate)` engine) · `pixels.rs` (~1030 lines, public borrowed views).

Pixel-domain carrier for PNG/WebP-raster output. One shared V2 mutation core; the facade, in-place, tiled, framed, and borrowed-view operations are thin projections over it. `#![forbid(unsafe_code)]` throughout.

## Layering

```
pixels::PixelView{,Mut} ─┐
lsb::{embed,extract,...} ─┼──► lsb_internal core (permutation, slot map, V2 codec)
lsb tiled/framed ────────┘
```

- `lsb.rs` — public facade: `capacity`, `embed`, `extract`, `embed_in_place`, `embed_framed`, `extract_framed`, `embed_tiled`, `embed_tiled_in_place`, `extract_tiled`, `embed_tiled_framed`, `extract_tiled_framed`. Re-exports `LsbConfig`, `TileConfig`, `DEFAULT_TILE_SIZE`, `InPlaceEmbedReport`.
- `lsb_internal.rs` — private engine: `stego_permutation` / `stego_permutation_v2`, `carrier_v2_slot_to_pixel_channel`, `lsb_available_slots`, `PixelCarrier` / `PixelCarrierMut` traits, `TileWindow{,Mut}` zero-copy windows, `crop_rgba` / `blit_rgba`, `embed_lsb` / `extract_lsb`, `embed_lsb_v2_in_place`, `embed_lsb_tiled{,_in_place}`, `extract_tiled_carrier{,_framed}`.
- `pixels.rs` — borrowed packed/strided `RGB8`/`RGBA8` views (`PixelLayout`, `PixelView`, `PixelViewMut`) implementing the same `PixelCarrier` traits. No `RgbaImage` conversion; alpha and padding bytes are never carriers. `as_view()` reborrows mut→const for extract-after-embed.

## Configuration

- `LsbConfig { seed, redundancy }` — `new` / `try_new` / `from_redundancy` (validated) / `with_redundancy` (constants-only compat: debug-assert, release-clamp) / `try_with_redundancy` / `with_redundancy_value` / `redundancy_value`. Zero seeds valid.
- `Redundancy` — validated `1..=10` (see `carrier-surface.md`). Capacity units are RGB carrier slots (one slot per non-alpha channel byte).
- `TileConfig::try_new(seed, tile_size)` — `tile_size > 0`; `DEFAULT_TILE_SIZE = 64`, `MIN_TILE_SIZE = 32`. Tiled extraction takes explicit `max_origins` in `1..=MAX_TILED_ORIGINS` (4096). Raw tiled recovery returns the first candidate and cannot authenticate — prefer framed tiled for crops.

## Core semantics

- One in-place core: `embed_lsb_tiled_in_place` is the shared algorithm; cloning `embed_lsb_tiled` delegates to it. Insufficient capacity leaves the caller's buffer unchanged.
- V2 slot mapping is byte-frozen; injectivity is verified for documented domains only — never claim a full-domain bijection.
- Permutation seeds derive `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)` internally; embed and extract must use the same seed.
- Two unrelated RNGs: `PixelSelectionRng` (`src/util/image.rs`, pixel selection) vs `DctCoefficientRng` (`jpeg_transcoder/stego_f5.rs`, DCT shuffle). Do not interchange.

## Operation styles

| Style | Functions | Notes |
|-------|-----------|-------|
| Raw | `embed` / `extract` | Caller knows payload length |
| In-place | `embed_in_place`, `embed_tiled_in_place` | Mutates caller's `RgbaImage`; same V2 core as cloning `embed` |
| Borrowed views | `PixelView{,Mut}` | Same core over caller buffers; stride-aware |
| Framed | `embed_framed` / `extract_framed` | `frame::{encode,decode}` wrapper; length self-describing, CRC32 corruption detection |
| Tiled | `embed_tiled{,_in_place,_framed}` / `extract_tiled{,_framed}` | Full payload per tile; crop-resistant; `TileConfig` + `max_origins` |

Consumer tests: `stegoeggo-stego/tests/direct_consumer.rs`; rights-layer routing (output JPEG → DCT, else LSB): `overview.md` data flow and `output_domain_routing` tests.
