# JPEG Transcoder

**Source:** `src/jpeg_transcoder/mod.rs`

JPEG-specific processing pipeline that operates directly on DCT coefficients, bypassing pixel decode/encode cycles. This is the core of the JPEG fast path.

## JpegTranscoder

Static methods for JPEG coefficient manipulation:

```rust
pub struct JpegTranscoder;

impl JpegTranscoder {
    pub fn decode_coefficients(jpeg_data: &[u8]) -> Result<(JpegHeader, Coefficients)>
    pub fn encode_coefficients(
        header: &JpegHeader,
        coefficients: &Coefficients,
        original_jpeg: Option<&[u8]>,
    ) -> Result<Vec<u8>>
    pub fn encode_coefficients_preserving(
        header: &JpegHeader,
        coefficients: &Coefficients,
        original_jpeg: &[u8],
    ) -> Result<Vec<u8>>
    fn assemble_jpeg(header: &JpegHeader, scan_data: &[u8]) -> Result<Vec<u8>>
}
```

### Decode Flow

1. Parse JPEG header (`JpegHeader::parse`)
2. Probe full DCT support (`probe_dct_support_full`) — checks header properties plus scan count and EOI validity
3. Find scan data start (`scan_utils::get_scan_data_start`)
4. Decode Huffman-encoded coefficients (`CoefficientDecoder`)
5. Return header + coefficients

### Encode Flow

Two paths:

- **`assemble_jpeg`** (when `original_jpeg` is `None`): Rebuilds JPEG from parsed header fields. Preserves APP0, APP1, COM markers from header. Drops unknown segments (APP2, APP13, APP14, DRI, etc.). Used only for non-original round-trip canonicalization.
- **`encode_coefficients_preserving`** (when `original_jpeg` is `Some`): Walks the original byte stream, replacing only DQT markers and SOS scan data. All other segments preserved verbatim in original order. Used for all DCT embedding output (success path and capacity-downgrade fallback).

The DCT success path always uses `encode_coefficients_preserving` (the `Some(original_jpeg)` path) for both roundtrip verification and final output. `assemble_jpeg` is never reachable from the normal original-JPEG DCT success path.

### Assemble

`assemble_jpeg` combines header bytes + scan data bytes. Returns `Result<Vec<u8>>`. Has `debug_assert!` for 8-bit quantization values exceeding 255.

## DctSupport Probe

```rust
pub enum DctSupport {
    Supported,
    Unsupported(DctUnsupportedReason),
}

pub fn probe_dct_support(header: &JpegHeader) -> DctSupport
pub fn probe_dct_support_full(header: &JpegHeader, jpeg_data: &[u8]) -> DctSupport
```

`probe_dct_support` checks header-only properties:
- Progressive mode
- Precision (must be 8-bit)
- Coding process (must be Sequential DCT)
- Restart intervals (must be 0)
- Component validity (must have DC+AC Huffman tables)
- Sampling factors (must be ≤ 4)

`probe_dct_support_full` additionally walks the complete JPEG structure to verify:
- Exactly one sequential scan (rejects multi-scan)
- Valid terminal EOI (rejects truncated input)

Unsupported inputs are routed to metadata-only processing.

## JpegStructure

```rust
pub struct JpegStructure {
    pub scan_count: usize,
    pub has_restart_markers: bool,
    pub has_trailing_segments_after_scan: bool,
    pub eoi_offset: Option<usize>,
}
```

Returned by `JpegHeader::analyze_structure(data)`. Walks the complete JPEG byte stream without decoding coefficients to count scans, detect restart markers, and locate EOI. Handles entropy stuffing (0xFF 0x00) correctly.

## Coefficients Type

```rust
pub type Coefficients = HashMap<u8, Vec<[i16; 64]>>;
```

Component ID → list of 8×8 blocks (64 DCT coefficients each). Stored in natural (row-major) order. Each component's blocks are in MCU traversal order: for each MCU, sub-blocks in (by, bx) order.

## Scan Data Utilities

```rust
pub fn get_scan_data_start(data: &[u8]) -> Option<usize>
```

Finds the SOS (Start of Scan) marker position. Uses `checked_add` to prevent integer overflow with malformed segment lengths. Advances past all non-scan markers (APP, DQT, DHT, COM, etc.).

## is_progressive_jpeg

```rust
pub fn is_progressive_jpeg(jpeg_data: &[u8]) -> bool
```

Checks if the JPEG uses progressive coding (SOF2 marker). Used to decide between full F5 stego (baseline) and seed-only stego (progressive).

## Error Type

```rust
pub enum TranscoderError {
    InvalidFormat(String),
    Unsupported(String),
    HuffmanDecode(String),
    HuffmanEncode(String),
    Io(std::io::Error),
    EmbeddingFailed(String),
}
```

## Canonical Huffman Construction

Both `HuffmanDecoder` and `HuffmanEncoderTable` construct canonical Huffman codes using the standard JPEG algorithm: advance code unconditionally after every bit-length slot (including zero-count lengths). Empty slots get sentinel values (-1 for decoder, zero-length for encoder). The decoder validates code ranges against min/max code arrays; the encoder validates symbol presence via zero-length entries.

## Malformed Entropy Handling

The coefficient decoder fails closed on malformed entropy:
- Missing DC symbol → `HuffmanDecode` error
- Missing AC symbol → `HuffmanDecode` error
- AC run overflow (beyond coefficient 63) → `HuffmanDecode` error
- Invalid zero-size symbols (not EOB or ZRL) → `HuffmanDecode` error
- Truncated magnitude data → `HuffmanDecode` error

Malformed entropy never produces partial successful coefficient maps.

## Module Interactions

- **header.rs**: `JpegHeader::parse` for header parsing; `JpegHeader::analyze_structure` for scan structure detection; `parse_sos` returns `Result<()>` and rejects malformed table IDs
- **entropy.rs**: `CoefficientDecoder` / `CoefficientEncoder` for Huffman codec; decoder fails closed on truncated/malformed entropy data; canonical code construction advances through zero-count lengths
- **stego_f5.rs**: `DctStegoF5` for coefficient manipulation
- **protected/steganography.rs**: `apply_dct_stego_bytes` calls transcoder for JPEG fast path; uses preserving encoding for all DCT output
- **lib.rs**: Used in `apply_bytes_pipeline` when input/output are both JPEG
