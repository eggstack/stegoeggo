# Carrier Crate: stegoeggo-stego

The workspace contains [`stegoeggo-stego`](https://crates.io/crates/stegoeggo-stego), a lower-level, application-neutral carrier crate for callers that want generic LSB/JPEG steganography without StegoEggo's rights-policy layer. It is the standalone package and public API home for those carrier operations; workspace releases currently keep the root and carrier versions in lockstep.

## Operation styles

It exposes four operation styles on the same corrected carrier model:

### Raw

`lsb::embed`/`extract` and `jpeg::embed`/`extract` accept arbitrary bytes with caller-supplied payload length (and, for JPEG, the `actual_redundancy` returned by the embed report).

### In-place (LSB)

`lsb::embed_in_place` mutates the caller's `RgbaImage` and avoids the intentional full-image clone performed by the cloning `lsb::embed`. Both paths share the same corrected carrier mutation core.

### Framed

`lsb::embed_framed`/`lsb::extract_framed` and `jpeg::embed_framed`/`jpeg::extract_framed` add the existing bounded 11-byte frame format. Framed extraction recovers payloads using only the resulting carrier and the same seed/config; JPEG framed extraction probes the configured redundancy down to 1, so the embed report is not needed.

### Tiled (crop-oriented)

`lsb::embed_tiled`/`embed_tiled_in_place`/`extract_tiled`/`embed_tiled_framed`/`extract_tiled_framed` and the `jpeg::` counterparts embed the full payload per tile with a shared `TileConfig { seed, tile_size }`. JPEG tiles require `>= 8` and a multiple of 8 with redundancy 1 per tile. Recovery is bounded by an explicit `max_origins` (`1..=MAX_TILED_ORIGINS`); framed tiled recovery validates CRC32 per candidate and needs no caller-known length. Raw tiled recovery returns the first candidate and cannot authenticate correctness.

## Configuration

For untrusted configuration values, `LsbConfig::try_new`, `LsbConfig::try_with_redundancy`, `JpegConfig::try_new`, and `JpegConfig::try_with_redundancy` return `StegoError::InvalidConfig` instead of panicking on out-of-range redundancy. The original `with_redundancy` builder is retained for compile-time-constant values.

The frame CRC32 detects accidental corruption; it is not adversarial authentication.

## Usage

See [`examples/generic_stego.rs`](https://github.com/eggstack/stegoeggo/blob/main/examples/generic_stego.rs) for raw, in-place, framed, and tiled usage.

```rust
use stegoeggo::stego::{
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

// JPEG round-trip
let jpeg_config = JpegConfig::new(seed).with_redundancy(2);
let report = jpeg::embed(&jpeg_bytes, secret, &jpeg_config)?;
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
