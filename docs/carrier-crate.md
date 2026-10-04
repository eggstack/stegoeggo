# Carrier Crate: stegoeggo-stego

The workspace contains [`stegoeggo-stego`](https://crates.io/crates/stegoeggo-stego), a lower-level, application-neutral carrier crate for callers that want generic LSB/JPEG steganography without StegoEggo's rights-policy layer. It is the standalone package and public API home for those carrier operations; workspace releases currently keep the root and carrier versions in lockstep.

## Operation styles

It exposes the following operation styles on the same corrected carrier model:

### Raw

`lsb::embed`/`extract` and `jpeg::embed`/`extract` accept arbitrary bytes with caller-supplied payload length (and, for JPEG, the `actual_redundancy` returned by the embed report).

### In-place (LSB)

`lsb::embed_in_place` mutates the caller's `RgbaImage` and avoids the intentional full-image clone performed by the cloning `lsb::embed`. Both paths share the same corrected carrier mutation core.

### Framed

`lsb::embed_framed`/`lsb::extract_framed` and `jpeg::embed_framed`/`jpeg::extract_framed` add the existing bounded 11-byte frame format. Framed extraction recovers payloads using only the resulting carrier and the same seed/config; JPEG framed extraction probes the configured redundancy down to 1, so the embed report is not needed.

### Tiled (crop-oriented)

`lsb::embed_tiled`/`embed_tiled_in_place`/`extract_tiled`/`embed_tiled_framed`/`extract_tiled_framed` and the `jpeg::` counterparts embed the full payload per tile with a shared `TileConfig { seed, tile_size }`. JPEG tiles require `>= 8` and a multiple of 8 with redundancy 1 per tile. Recovery is bounded by an explicit `max_origins` (`1..=MAX_TILED_ORIGINS`); framed tiled recovery validates CRC32 per candidate and needs no caller-known length. Raw tiled recovery returns the first candidate and cannot authenticate correctness. Tiled extraction reads tile windows in place without allocating cropped copies.

### Strict JPEG embedding

`jpeg::embed_best_effort` and `jpeg::embed_framed_best_effort` are the
explicit best-effort operations (lower redundancy to fit; seed-hint
carrier when nothing fits). `jpeg::embed` and `jpeg::embed_framed` are
byte-identical compatibility names for the same behavior.
`jpeg::embed_strict` and `jpeg::embed_framed_strict` embed at exactly the
requested redundancy or return `InsufficientCapacity` without emitting
output. New code uses the explicit best-effort names or the strict
operations; the compatibility names remain for 0.x. Seed hints
(`jpeg::embed_seed_hint`) are transactional: success implies the complete
96-bit hint is recoverable.

### Prepared JPEG (repeated operations)

`prepared::PreparedJpeg` borrows encoded JPEG bytes and retains one
coefficient decode across repeated capacity, extraction,
strict-embedding, and tiled exact-embedding operations, with results
identical to the one-shot API. Codec internals stay private.
`PreparedJpeg::new_with_limits` bounds construction through the
carrier-owned `CarrierLimits`; the handle stays reusable after failed
queries or embeds, and no best-effort application degradation is added
to the prepared handle.

### Bounded parsing (`CarrierLimits`)

`limits::CarrierLimits` is the carrier-owned bounded-untrusted-input
contract (private fields, getters, builder): input bytes (100 MiB),
JPEG segments (256 × 65535 bytes), dimensions/pixels (16384 × 16384),
framed totals (16 MiB + 11-byte header), and tiled-search extent
(`MAX_TILED_ORIGINS`). Bounded `*_with_limits` JPEG one-shot variants
share the single decode path with explicit caller limits; limit failures
map to `StegoError::ResourceLimitExceeded` with no secret/input dump.
Legacy 0.x one-shot JPEG operations and `PreparedJpeg::new` retain their
historical input/dimension domain while preserving JPEG segment, frame,
and tiled bounds. `jpeg::inspect` keeps its signature and applies the
caller's segment limits without the later dimension/input policy caps.
The root checks its own `ResourceLimits` before carrier work; the carrier
never depends on the root crate's limits type.

### Borrowed pixel buffers

`pixels::PixelView`/`PixelViewMut` operate directly on caller-owned
packed or strided RGB/RGBA bytes with the identical carrier mapping as
the `RgbaImage` path. Construct with `PixelView::new(bytes, width,
height, layout, stride)` (`PixelLayout::Rgb8`/`Rgba8`, `stride >= width
* bytes_per_pixel`, all geometry checked, over-long backing accepted as
a sub-slice). Alpha bytes and row padding are never carriers and are
never mutated; capacity failure is atomic before the first mutation.
Both views expose capacity/raw/framed/tiled extraction; the mutable
view adds in-place raw/framed/tiled embedding (`as_view()` reborrows
for extraction). Capacity units are RGB slots for LSB, eligible AC
coefficients with `|coef| >= 2` for JPEG DCT, and 96 hint-bit positions
for seed hints.

### Still-lossless WebP byte carrier (optional `webp` feature)

`webp::embed`/`extract` (plus `capacity`, framed, tiled, and
`probe_support` convenience) accept a still VP8L file — plain or
extended-container still — decode it to RGBA, run the LSB carrier, and
re-encode losslessly with `WebPEncoder::new_lossless`. Every operation
takes a `CarrierLimits` bounding input bytes, decoded dimensions,
framed sizes, and tiled-search extent before any large allocation, and
every operation is all-or-error with no partial output. Lossy VP8 and
animated inputs return structured `UnsupportedWebP` (`LossyVp8` /
`Animated`) instead of being transcoded; the decoder's own animation
flag is re-checked after probing as defense in depth. Output is a newly
encoded carrier: ICC/EXIF/XMP chunks are dropped and no preservation is
claimed. Rights metadata rendering stays with the parent crate. Enable
with `stegoeggo-stego = { features = ["webp"] }` (off by default; the
codec is pure Rust via `image-webp`, no native dependency).

## Configuration

`Redundancy` is the recommended validated redundancy primitive with
identical semantics in every build profile; `LsbConfig`/`JpegConfig` gain
`from_redundancy`, `with_redundancy_value`, and `redundancy_value` over
it. For untrusted configuration values, `LsbConfig::try_new`, `LsbConfig::try_with_redundancy`, `JpegConfig::try_new`, and `JpegConfig::try_with_redundancy` return `StegoError::InvalidConfig` instead of panicking on out-of-range redundancy. The original `with_redundancy` builder is a debug-assert/release-clamp compatibility adapter retained for compile-time-constant values only; runtime values must use the fallible or `Redundancy`-based APIs. Zero seeds are valid.

The frame CRC32 detects accidental corruption; it is not adversarial authentication.

## Usage

Generic-only consumers should depend on `stegoeggo-stego` directly;
`stegoeggo::stego` re-exports the same surface as a convenience (no generic
API exists only through the facade). See [`examples/generic_stego.rs`](https://github.com/eggstack/stegoeggo/blob/main/examples/generic_stego.rs) for raw, in-place, framed, and tiled usage.

```rust
use stegoeggo_stego::{
    TileConfig,
    jpeg::{self, JpegConfig},
    lsb::{self, LsbConfig},
};

let secret = b"hello from stegoeggo";
let seed: u64 = 42;

// LSB raw round-trip
let config = LsbConfig::new(seed);
let report = lsb::embed(&img, secret, &config)?;
let recovered = lsb::extract(&report.output, secret.len(), &config)?;

// LSB in-place (no clone)
let mut img = make_image();
let report = lsb::embed_in_place(&mut img, secret, &config)?;

// Framed (no caller-known length needed)
let report = lsb::embed_framed(&img, secret, &config)?;
let recovered = lsb::extract_framed(&report.output, &config)?;

// JPEG round-trip (explicit best-effort; `embed` is the compat name)
let jpeg_config = JpegConfig::new(seed).with_redundancy(2);
let report = jpeg::embed_best_effort(&jpeg_bytes, secret, &jpeg_config)?;
let recovered = jpeg::extract(
    &report.output, secret.len(), &jpeg_config, report.actual_redundancy,
)?;

// Tiled crop-oriented round-trips (bounded recovery)
let tile = TileConfig::try_new(seed, 64)?;
let tiled = lsb::embed_tiled(&img, secret, &tile)?;
let recovered = lsb::extract_tiled(&tiled.output, secret.len(), &tile, 64)?;

let framed = lsb::embed_tiled_framed(&img, secret, &tile)?;
let recovered = lsb::extract_tiled_framed(&framed.output, &tile, 64)?;

let jtiled = jpeg::embed_tiled(&jpeg_bytes, secret, &tile)?;
let recovered = jpeg::extract_tiled_framed(&jtiled.output, &tile, 64)?;
```

## Internal architecture

The rights-aware hidden-marker adapter is organized by responsibility under `src/protected/steganography/`: marker construction, carrier embedding, extraction/search, verification, and legacy compatibility are separate modules behind the `SteganographyProtector` facade.
