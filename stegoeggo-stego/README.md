# stegoeggo-stego

Generic image steganography carriers for arbitrary payloads.

This crate provides application-neutral LSB (pixel-domain) and JPEG DCT
embedding/extraction primitives. It is consumed by the higher-level
[`stegoeggo`](https://crates.io/crates/stegoeggo) rights-protection
pipeline but can also be used directly as a generic carrier.

## API surface

- `stegoeggo_stego::lsb::{embed, extract, embed_framed, extract_framed, capacity, LsbConfig}`
- `stegoeggo_stego::jpeg::{embed, extract, embed_framed, extract_framed, capacity, JpegConfig, probe_support}`
- `stegoeggo_stego::frame::{encode, decode, decode_prefix}`
- `stegoeggo_stego::{EmbedReport, CapacityReport, StegoError, EmbedOutcome}`

JPEG header parsing, DCT coefficient processing, Huffman state, F5 objects,
LSB permutations, and raw carrier helpers are private implementation details.
The default API exposes only the operation-level modules listed above.
An unstable `application-support` feature exists solely for the parent
rights-protection crate's compatibility adapter; it does not expose codec or
coefficient types. Its tiled-JPEG operations return an opaque candidate key so
prefix, header, full-payload, and legacy extraction lengths remain bound to one
tile/grid-seed/redundancy identity. Tiled-JPEG extraction creates an
operation-local search context that decodes the coefficient container once and
reuses the private state for every bounded candidate; it is dropped when the
search ends and is not part of the default API.

## Raw and framed operations

The raw `embed`/`extract` functions are for callers that retain the exact
payload length and, for JPEG, the report's `actual_redundancy`. The framed
convenience functions add the existing 11-byte frame header and CRC32:

```rust
use stegoeggo_stego::lsb::{self, LsbConfig};

let config = LsbConfig::new(42);
let report = lsb::embed_framed(&image, b"payload", &config)?;
let recovered = lsb::extract_framed(&report.output, &config)?;
assert_eq!(recovered, b"payload");
```

`jpeg::extract_framed` tries the configured redundancy and lower valid values
in a bounded search, so it also recovers outputs whose embedding was
auto-downgraded for capacity. The frame CRC detects accidental corruption; it
does not authenticate payloads against an adversary. Framed reports count the
encoded frame bytes, including overhead, in `payload_bytes`.

## License

MIT. See `LICENSE`.
