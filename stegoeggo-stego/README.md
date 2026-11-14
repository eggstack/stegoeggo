# stegoeggo-stego

Generic image carriers for embedding arbitrary bytes with pixel-domain LSB or
JPEG DCT operations. This crate does not implement rights metadata or the
higher-level `stegoeggo` policy model.

## Install

```toml
[dependencies]
stegoeggo-stego = "0.4"
```

The optional `webp` feature enables the still-lossless WebP byte API:

```toml
stegoeggo-stego = { version = "0.4", features = ["webp"] }
```

## Quick start

Framed operations store the payload length with the payload, so extraction does
not need the original length:

```rust
use image::RgbaImage;
use stegoeggo_stego::lsb::{self, LsbConfig};

let image = RgbaImage::new(128, 128);
let config = LsbConfig::new(42);
let report = lsb::embed_framed(&image, b"hello", &config)?;
let recovered = lsb::extract_framed(&report.output, &config)?;
assert_eq!(recovered, b"hello");
```

## Choose a carrier

| Carrier | Use | Limits |
|---|---|---|
| LSB | Pixel buffers and `RgbaImage` inputs | Fragile under lossy re-encoding; alpha is not used |
| JPEG DCT | Supported sequential JPEG structures | Not all JPEGs are embeddable; call `jpeg::probe_support` first |
| WebP | Still-lossless VP8L files with the `webp` feature | Lossy and animated input is rejected; output metadata is not preserved |

The JPEG API operates on encoded bytes and preserves unrelated marker segments
on successful embedding. The WebP API decodes and re-encodes the image, so it
does not preserve ICC, EXIF, or XMP chunks.

## Operation styles

| Style | Use it when | Important detail |
|---|---|---|
| Raw | You know the payload length | JPEG extraction also needs the actual redundancy from the embed report |
| Framed | You want extraction without a saved payload length | The frame uses CRC32 for corruption detection, not authentication |
| Tiled | You need additional tolerance to some crops | The full payload is repeated per tile; recovery needs at least one usable tile and a bounded `max_origins` search |
| In-place LSB | You already own a mutable pixel buffer | The input buffer is changed directly |
| Prepared JPEG | You will perform repeated operations on one JPEG | The decoded coefficient state is reused |

Best-effort JPEG embedding may lower redundancy or return a seed hint when the
payload does not fit. Use `jpeg::embed_strict` when the requested redundancy
must be preserved exactly. For untrusted input, use the APIs that accept
`CarrierLimits`.

## Security limits

The seed selects carrier positions; it is not a cryptographic secret. A framed
CRC32 is not authentication. If authenticity matters, authenticate the payload
with HMAC or a public-key signature before embedding it.

For the full API guide and additional examples, see the
[carrier crate documentation](https://github.com/eggstack/stegoeggo/blob/main/docs/carrier-crate.md)
and [`examples/generic_stego.rs`](https://github.com/eggstack/stegoeggo/blob/main/examples/generic_stego.rs).

## License

MIT. See `LICENSE`.
