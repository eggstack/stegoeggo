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
2. Probe DCT support (`probe_dct_support`) — rejects progressive, restart-bearing, missing-table, and other unsupported inputs
3. Find scan data start (`scan_utils::get_scan_data_start`)
4. Decode Huffman-encoded coefficients (`CoefficientDecoder`)
5. Return header + coefficients

### Encode Flow

Two paths:

- **`assemble_jpeg`** (when `original_jpeg` is `None`): Rebuilds JPEG from parsed header fields. Preserves APP0, APP1, COM markers from header. Drops unknown segments (APP2, APP13, APP14, DRI, etc.). Used for canonicalization round-trips.
- **`encode_coefficients_preserving`** (when `original_jpeg` is `Some`): Walks the original byte stream, replacing only DQT markers and SOS scan data. All other segments preserved verbatim in original order. Used for final DCT embedding output.

### Assemble

`assemble_jpeg` combines header bytes + scan data bytes. Returns `Result<Vec<u8>>`. Has `debug_assert!` for 8-bit quantization values exceeding 255.

## DctSupport Probe

```rust
pub enum DctSupport {
    Supported,
    Unsupported(DctUnsupportedReason),
}

pub fn probe_dct_support(header: &JpegHeader) -> DctSupport
```

Central capability probe that decides whether DCT embedding is supported. Checks:
- Progressive mode
- Precision (must be 8-bit)
- Coding process (must be Sequential DCT)
- Restart intervals (must be 0)
- Component validity (must have DC+AC Huffman tables)
- Sampling factors (must be ≤ 4)

Unsupported inputs are routed to metadata-only processing.

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

## Module Interactions

- **header.rs**: `JpegHeader::parse` for header parsing; `parse_sos` returns `Result<()>` and rejects malformed table IDs
- **entropy.rs**: `CoefficientDecoder` / `CoefficientEncoder` for Huffman codec; decoder rejects truncated entropy data
- **stego_f5.rs**: `DctStegoF5` for coefficient manipulation
- **protected/steganography.rs**: `apply_dct_stego_bytes` calls transcoder for JPEG fast path
- **lib.rs**: Used in `apply_bytes_pipeline` when input/output are both JPEG
