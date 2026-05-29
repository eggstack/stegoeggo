# JPEG Entropy Codec

**Source:** `src/jpeg_transcoder/entropy.rs` (~808 lines)

Huffman entropy encoding/decoding of JPEG DCT coefficients. This is the core compression layer.

## CoefficientDecoder

Decodes Huffman-encoded DCT coefficients from JPEG scan data.

```rust
pub struct CoefficientDecoder { /* ... */ }
```

### Decode Flow

1. Pre-build Huffman decoders from header tables (once, before MCU loop)
2. Process MCUs (Minimum Coded Units) in raster order
3. For each MCU:
   - Decode DC coefficient (differential encoding)
   - Decode AC coefficients (run-length + magnitude coding)
4. Store coefficients in natural (row-major) order via `block[ZIGZAG[k]] = magnitude`

### Key Details

- Handles DC prediction (previous block's DC subtracted from current)
- Processes restart markers (RST0–RST7) for error recovery
- Handles spectral selection limits for progressive JPEG

## CoefficientEncoder

Encodes DCT coefficients back to Huffman-compressed scan data.

```rust
pub struct CoefficientEncoder { /* ... */ }
```

### HuffmanEncoderTable

Pre-built lookup table for O(1) symbol → (code, length) mapping:

```rust
struct HuffmanEncoderTable {
    entries: [(u16, u8); 256],  // code, bit length
}
```

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
struct BitReader<'a> { data: &'a [u8], byte_pos: usize, bit_pos: u8 }
struct BitWriter { bytes: Vec<u8>, current_byte: u8, bit_pos: u8 }
```

- `BitReader` reads bits from entropy data, handles marker stuffing (0xFF 0x00 → 0xFF)
- `BitWriter` writes bits with byte alignment and stuffing

## Zigzag Order

```rust
const ZIGZAG: [usize; 64] = [0, 1, 8, 16, 9, 2, 3, 10, ...];
const REVERSE_ZIGZAG: [usize; 64] = [0, 1, 5, 6, 14, 15, 27, 28, ...];
```

Standard JPEG zigzag scan order for converting between 8×8 block order and 1D coefficient array.

## Standard Huffman Tables

Reference constants for luminance/chrominance DC/AC Huffman tables. Used as defaults when the image doesn't specify custom tables.

## Module Interactions

- **mod.rs**: `CoefficientDecoder` / `CoefficientEncoder` used by `JpegTranscoder`
- **header.rs**: Huffman tables from `JpegHeader` used to build decoders/encoders
- **stego_f5.rs**: Modifies decoded coefficients for F5 steganography
- **steganography.rs**: `apply_dct_stego_bytes` uses decoder/encoder for JPEG fast path
