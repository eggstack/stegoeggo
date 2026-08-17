# stegoeggo-stego

Generic image steganography carriers for arbitrary payloads.

This crate provides application-neutral LSB (pixel-domain) and JPEG DCT
embedding/extraction primitives. It is consumed by the higher-level
[`stegoeggo`](https://crates.io/crates/stegoeggo) rights-protection
pipeline but is designed to be used directly as a standalone generic
carrier.

**StegoEggo is not required.** This crate does not know about rights
metadata, XMP, plus:DataMining, HMAC, or any application semantics. It
moves arbitrary bytes into and out of supported image carriers.

The carrier is a standalone package with its own public API surface. It is
currently versioned in workspace lockstep with the root crate; the package
boundary does not imply an independent release cadence.

## What this crate is

A small, focused library that exposes two image steganography carriers —
pixel-domain LSB and JPEG DCT — for callers that want to embed arbitrary
bytes that can be recovered later. Three operation styles are supported:

- **Raw** — caller knows the payload length and (for JPEG) the actual
  redundancy returned by the embed report. Lowest overhead.
- **In-place** (LSB) — caller already owns a mutable `RgbaImage` buffer
  and wants to avoid the full-image clone a copied `embed` would perform.
- **Framed** — wraps the payload in a self-describing header with a
  CRC32. Recovered without the caller-retained payload length, and (for
  JPEG) without the embed report's `actual_redundancy`. JPEG framed
  extraction decodes the supported carrier once and reuses that state while
  probing the configured redundancy down to 1.

## Supported carriers and limitations

| Carrier | Domain | Capacity unit | Limitations |
|---|---|---|---|
| `lsb` | Pixel (R, G, B channels of `RgbaImage`) | RGB carrier slots (`width * height * 3`) | Fragile under lossy re-encoding; survives lossless WebP and PNG only |
| `jpeg` | DCT coefficients (F5-style) | Non-zero AC coefficients across all components | Bounded supported subset: 8-bit, sequential, single-scan, Huffman, ≤4 components, ≤4 sampling factor, no restart intervals |

The alpha channel is never a carrier. The JPEG DCT path operates on the
encoded JPEG byte stream directly — pixels are not decoded.

The generic frame's CRC32 is **corruption detection**, not authentication.
It catches accidental corruption and lossy re-encoding damage. It does
not prove a payload was produced by any specific party. For
authentication, layer HMAC or Ed25519 over the unframed payload before
embedding.

## Raw API

When the caller knows the payload length (and, for JPEG, the
`actual_redundancy` from the embed report), the raw API is the lowest
overhead option:

```rust
use stegoeggo_stego::lsb::{self, LsbConfig};
use image::RgbaImage;

let image = RgbaImage::new(128, 128);
let config = LsbConfig::new(42);
let payload = b"hello";

let report = lsb::embed(&image, payload, &config)?;
assert!(report.embedded);

let recovered = lsb::extract(&report.output, payload.len(), &config)?;
assert_eq!(recovered, payload);
```

For JPEG:

```rust
use stegoeggo_stego::jpeg::{self, JpegConfig};

let jpeg_bytes = std::fs::read("photo.jpg")?;
let config = JpegConfig::new(42);

if jpeg::probe_support(&jpeg_bytes)? == jpeg::JpegSupport::Supported {
    let report = jpeg::embed(&jpeg_bytes, payload, &config)?;
    let recovered = jpeg::extract(
        &report.output,
        payload.len(),
        &config,
        report.actual_redundancy,
    )?;
}
```

## Framed convenience API

The framed convenience API wraps the payload in a self-describing
header so it can be recovered later without retaining the original
payload length. For JPEG, the framed extractor also probes the
configured redundancy down to 1, so callers do not need to retain the
embed report's `actual_redundancy`.

```rust
use stegoeggo_stego::lsb::{self, LsbConfig};

let config = LsbConfig::new(42);
let report = lsb::embed_framed(&image, b"payload", &config)?;
let recovered = lsb::extract_framed(&report.output, &config)?;
assert_eq!(recovered, b"payload");
```

The frame overhead is 11 bytes (magic + version + length + CRC32).
Maximum payload size is 16 MiB. The framed operations count this
overhead in `payload_bytes` reports.

## In-place LSB API

When the caller already owns a mutable `RgbaImage`, `embed_in_place`
mutates the buffer in place and does not allocate a replacement image:

```rust
use stegoeggo_stego::lsb::{self, LsbConfig};
use image::RgbaImage;

let mut image = RgbaImage::new(128, 128);
let report = lsb::embed_in_place(&mut image, b"payload", &LsbConfig::new(42))?;
assert!(report.embedded);
```

Capacity is checked before the first pixel mutation, so an insufficient
carrier is left unchanged. The cloning `embed` and in-place
`embed_in_place` paths share the same corrected V2 mutation core.

## Capacity and redundancy

Capacity is reported in carrier-specific units:

- LSB: RGB carrier slots (`width * height * 3`).
- JPEG: eligible non-zero AC coefficients across all components.

The same unit applies to both `required` and `available` in a
`CapacityReport`, so `is_sufficient()` is a direct comparison. Each
embedded LSB payload bit occupies `STEGO_SPREAD_FACTOR * redundancy`
slots, and each JPEG payload bit occupies `redundancy` AC coefficients.

Redundancy is configurable in the range `1..=10`. Higher redundancy
increases robustness at the cost of reduced capacity. Use `try_new`
or `try_with_redundancy` for runtime-validated values:

```rust
use stegoeggo_stego::lsb::LsbConfig;

let user_redundancy: usize = 3;
let config = LsbConfig::try_new(42, user_redundancy)?;
```

Invalid redundancy values (`0`, `11`, `usize::MAX`) return
`StegoError::InvalidConfig` instead of panicking.

## JPEG support probing

Only a bounded JPEG subset is embeddable. Call `probe_support` before
embedding to classify the input:

```rust
use stegoeggo_stego::jpeg;

match jpeg::probe_support(&jpeg_bytes)? {
    jpeg::JpegSupport::Supported => { /* embed normally */ }
    jpeg::JpegSupport::Unsupported(reason) => {
        // Progressive, arithmetic, multi-scan, restart, etc.
        // Seed hint may still be embeddable.
    }
}
```

Successful embedding uses the original-JPEG byte-preserving encode path,
so APP2, APP13, APP14, COM, and unknown marker segments survive
byte-for-byte.

## Error handling

All public operations return a `StegoResult<T>`. The error type
`StegoError` distinguishes:

- `InvalidConfig` — caller-supplied configuration is invalid (e.g.,
  redundancy out of range, tile size too small).
- `InsufficientCapacity` — the carrier cannot hold the payload. The
  structured fields expose `required` and `available` in carrier units.
- `MalformedInput` — the input image/JPEG cannot be decoded.
- `UnsupportedJpeg(reason)` — the JPEG structure is well-formed but
  not embeddable (with a structured `JpegUnsupportedReason`).
- `FrameNotFound`, `MalformedFrame`, `FrameChecksumMismatch` —
  generic frame decode failures.
- `ResourceLimitExceeded` — a parser resource limit was hit.
- `EmptyCarrier` — the carrier image has zero pixels.

`StegoError` converts into the application crate's root error type via
`From` for callers that use a unified error type.

## Security and robustness limits

This crate makes **no** forensic, steganalysis-resistance, or
adversarial-robustness claims. The carriers are deterministic and
recoverable; they are not designed to hide payload existence from an
attacker with access to the carrier model.

- The seed is not a cryptographic secret. It selects the carrier
  permutation but does not authenticate the payload. Any caller can
  extract a payload once given the seed and the carrier model.
- Lossy re-encoding (PNG→JPEG, JPEG requantization, lossy WebP)
  destroys the LSB payload. Tiled JPEG survives only crops that
  preserve DCT coefficients.
- The generic frame CRC32 detects corruption; it does not authenticate.

Use HMAC or Ed25519 over the unframed payload before embedding for
authenticated provenance. Use StegoEggo (the application crate) for
rights-policy semantics and metadata.

## Relationship to the StegoEggo application crate

`stegoeggo` (the application crate) is the rights-reservation /
provenance layer. It uses `stegoeggo-stego` as a generic carrier for
its hidden-marker channel. The two crates have crisply separate
responsibilities:

- `stegoeggo-stego` knows about pixel bits, DCT coefficients, frame
  headers, and CRC32. It does not know about rights, XMP, plus:DataMining,
  HMAC, or Ed25519.
- `stegoeggo` knows about PLUS License Data Format, copyright notices,
  provenance chains, and authenticated payload verification. It uses
  `stegoeggo-stego` only for the low-level embed/extract primitives.

The narrow `application-support` feature on `stegoeggo-stego` exposes
a small set of helper functions to the parent crate. It is intentionally
`#[doc(hidden)]` and is not part of the stable generic API.

## API surface summary

```text
stegoeggo_stego::lsb                          → LsbConfig, capacity, embed, embed_in_place,
                                                extract, embed_framed, extract_framed
stegoeggo_stego::jpeg                         → JpegConfig, JpegSupport, probe_support,
                                                capacity, embed, extract, embed_framed,
                                                extract_framed, inspect, is_progressive_jpeg,
                                                embed_seed_hint, extract_seed_hint
stegoeggo_stego::frame                        → FRAMED_MAGIC, FRAME_VERSION, MAX_FRAME_PAYLOAD,
                                                FRAME_HEADER_SIZE, FrameHeader, encode, decode,
                                                decode_prefix
stegoeggo_stego::error                        → StegoError, StegoResult, JpegUnsupportedReason
stegoeggo_stego::{CapacityReport,             → structured reports
                 EmbedReport, InPlaceEmbedReport,
                 EmbedOutcome, EmbedOutcomeSummary,
                 EmbedPath, EmbedStatus,
                 DEFAULT_TILE_SIZE}
```

JPEG header parsing, DCT coefficient processing, Huffman state, F5
objects, LSB permutations, and raw carrier helpers are private. The
default API exposes only the operation-level modules listed above.

## License

MIT. See `LICENSE`.
