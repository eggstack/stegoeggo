# stegoeggo-stego

Generic image steganography carriers for arbitrary payloads.

This crate provides application-neutral LSB (pixel-domain) and JPEG DCT
embedding/extraction primitives. It is consumed by the higher-level
[`stegoeggo`](https://crates.io/crates/stegoeggo) rights-protection
pipeline but can also be used directly as a generic carrier.

## API surface

- `stegoeggo_stego::lsb::{embed, extract, capacity, LsbConfig}`
- `stegoeggo_stego::jpeg::{embed, extract, capacity, JpegConfig, probe_support}`
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

## License

MIT. See `LICENSE`.
