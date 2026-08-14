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

Low-level internals (JPEG header parsing, DCT coefficient processing,
LSB permutations) are deliberately not part of the stable API. The
consuming crate accesses them through a `#[doc(hidden)]` facade.

## License

MIT. See `LICENSE`.