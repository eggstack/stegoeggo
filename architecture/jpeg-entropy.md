# JPEG Entropy Codec

**Source:** `stegoeggo-stego/src/jpeg_transcoder/entropy.rs` (~991 lines)

Huffman entropy encoding/decoding of JPEG DCT coefficients. This is the core compression layer.

## CoefficientDecoder

Decodes Huffman-encoded DCT coefficients from JPEG scan data.

```rust
pub struct CoefficientDecoder { header: JpegHeader, scan_data: Vec<u8> }

impl CoefficientDecoder {
    pub fn new(header: JpegHeader, scan_data: Vec<u8>) -> Self
    pub fn decode(&self) -> Result<HashMap<u8, Vec<[i16; 64]>>>
}
```

`decode` rejects progressive headers up front (`TranscoderError::Unsupported`);
only the sequential entropy slice selected by the checked structural analyzer
reaches this decoder.

### Decode Flow

1. Pre-build Huffman decoders from header tables (once, before MCU loop)
2. Process MCUs (Minimum Coded Units) in raster order
3. For each MCU:
   - Decode DC coefficient (differential encoding)
   - Decode AC coefficients (run-length + magnitude coding)
4. Store coefficients in natural (row-major) order via `block[ZIGZAG[k]] = magnitude`

### Key Details

- Handles DC prediction (previous block's DC subtracted from current)
- Decodes the sequential, non-restart entropy slice selected by the checked JPEG
  structural analyzer
- Restart-bearing and progressive scans are rejected by the DCT support probe before
  this decoder is used

## CoefficientEncoder

Encodes DCT coefficients back to Huffman-compressed scan data.

```rust
pub struct CoefficientEncoder { header: JpegHeader }

impl CoefficientEncoder {
    pub fn new(header: JpegHeader) -> Self
    pub fn encode(&self, coefficients: &HashMap<u8, Vec<[i16; 64]>>) -> Result<Vec<u8>>
}
```

Like the decoder, `encode` rejects progressive headers. DC differences and AC
magnitudes that exceed the table-supported size are clamped to the encodable
range (see below), never passed through raw.

### HuffmanDecoder / HuffmanEncoderTable (private)

```rust
struct HuffmanDecoder { by_len: [Vec<(u16, u8)>; 16] }
struct HuffmanEncoderTable {
    entries: [(u16, u8); 256],  // code, bit length; missing symbols are (0, u8::MAX)
}
```

Both are built from one shared canonical-code constructor
(`build_canonical_huffman_entries`), which fails closed on empty tables, symbol
counts above 256, duplicate symbols, count/value length mismatch, code-space
oversubscription, and code overflow. The decoder indexes entries per bit-length
bucket and binary-searches each bucket; the encoder is an O(1) symbol →
(code, length) map with zero-length (`u8::MAX`) miss detection. Canonical codes
advance unconditionally after every bit-length slot, including zero-count
lengths.

Pre-built decoder/encoder tables from the header (once, before the MCU loop):

### Encode Flow

1. Pre-build encoder lookup tables from Huffman tables
2. For each MCU:
   - Encode DC coefficient (differential)
   - Encode AC coefficients (run-length + magnitude)
3. Handle byte stuffing (0xFF → 0xFF 0x00)
4. Pad to byte boundary

### Coefficient Clamping

Large coefficients are clamped to table-supported ranges:
- DC: 11 bits maximum
- AC: 10 bits maximum

Coefficients exceeding these ranges are clamped to the maximum representable value.

## BitReader / BitWriter

Low-level bit I/O:

```rust
struct BitReader<'a> { data: &'a [u8], byte_pos: usize, bit_pos: u8, eoi_reached: bool, restart_seen: bool }
struct BitWriter { data: Vec<u8>, current_byte: u8, bits_in_byte: u8 }
```

- `BitReader` reads bits from entropy data in MSB-first order (`bit_pos` initializes to 7). Handles marker stuffing (0xFF 0x00 → 0xFF data bit). An `FF D0`–`FF D7` sequence sets `restart_seen` and recurses past the marker — not silent: after each MCU the decoder clears DC predictors when `restart_seen` is set. Any other marker (including EOI) sets `eoi_reached` and ends the bit stream.
- `BitWriter` writes bits with byte alignment and stuffing

### Scan Validation (`finish_scan`) and Magnitude Decoding

`decode` ends with `finish_scan(expected_blocks, decoded_blocks)`, which fails
closed on early termination (fewer blocks than the MCU geometry requires),
block-count overflow, non-all-ones pad bits, and trailing entropy bytes after
the pad. `read_magnitude(size)` returns 0 for size 0, `None` for size ≥ 16, and
otherwise applies JPEG sign extension (values below `1 << (size-1)` are biased
down by `(1 << size) - 1`).

## Zigzag Order

```rust
const ZIGZAG: [usize; 64] = [0, 1, 8, 16, 9, 2, 3, 10, ...];
const REVERSE_ZIGZAG: [usize; 64] = [0, 1, 5, 6, 14, 15, 27, 28, ...];
```

Standard JPEG zigzag scan order for converting between 8×8 block order and 1D coefficient array.

## Standard Huffman Tables

Reference constants for luminance/chrominance DC/AC Huffman tables
(`STD_LUMINANCE_DC`, `STD_LUMINANCE_AC`, `STD_CHROMINANCE_DC`,
`STD_CHROMINANCE_AC`). They are reference data only — nothing in the decode or
encode path falls back to them. A scan component referencing a table the header
does not carry fails with a missing-table `HuffmanDecode` error.

## Module Interactions

- **mod.rs**: `CoefficientDecoder` / `CoefficientEncoder` used by `JpegTranscoder`
- **header.rs**: Huffman tables from `JpegHeader` used to build decoders/encoders
- **stego_f5.rs**: Modifies decoded coefficients for F5 steganography
- **steganography/embed.rs**: `apply_dct_stego_bytes_from_plan` and its legacy `apply_dct_stego_bytes` (legacy context path) call the transcoder through `stegoeggo_stego::application_support`; the root crate never imports `CoefficientDecoder`/`CoefficientEncoder` directly
