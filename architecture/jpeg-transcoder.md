# JPEG Transcoder

**Source:** `stegoeggo-stego/src/jpeg_transcoder/mod.rs`

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
2. Run checked structural analysis and the full DCT support probe — checks header properties, exact scan count, EOI validity, restart markers, and post-scan segments
3. Slice each supported scan using `entropy_start..entropy_end`; marker fill, EOI, and trailing segments are excluded
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
- No restart markers, including scans without a DRI marker
- No post-scan marker segments
- Checked marker/segment boundaries and exact entropy spans

Unsupported inputs are routed to metadata-only processing.

## JpegStructure

```rust
pub struct JpegScanSpan {
    pub sos_marker_offset: usize,
    pub sos_header_end: usize,
    pub entropy_start: usize,
    pub entropy_end: usize,
    pub terminating_marker_offset: usize,
    pub terminating_marker: u8,
}

pub struct JpegStructure {
    pub scan_count: usize,
    pub has_restart_markers: bool,
    pub has_trailing_segments_after_scan: bool,
    pub eoi_offset: Option<usize>,
    pub scan_spans: Vec<JpegScanSpan>,
}
```

Each `JpegScanSpan` records the SOS marker and header end plus the exact entropy slice
boundaries. `JpegHeader::analyze_structure_checked(data)` returns `Result` and fails
closed on malformed marker runs, short or overlong segments, malformed SOS extents, and
unterminated entropy. It preserves the first `FF` of repeated marker-fill runs as the
marker offset, keeps exactly `FF 00` inside entropy, and rejects `FF FF 00`. Restart
markers are recorded without ending the structural scan; `probe_dct_support_full`
continues to classify them as unsupported.

`analyze_structure(data)` is retained as a compatibility wrapper only. Supported-path
probing and decoding never use its best-effort compatibility behavior.

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

- **stegoeggo-stego/src/jpeg_transcoder/header.rs**: `JpegHeader::parse` for header parsing; `JpegHeader::analyze_structure_checked` for scan structure detection; `parse_sos` returns `Result<()>` and rejects malformed table IDs
- **stegoeggo-stego/src/jpeg_transcoder/entropy.rs**: `CoefficientDecoder` / `CoefficientEncoder` for Huffman codec; decoder fails closed on truncated/malformed entropy data; canonical code construction advances through zero-count lengths
- **stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs**: `DctStegoF5` for coefficient manipulation
- **stegoeggo/src/protected/steganography.rs**: `apply_dct_stego_bytes` calls transcoder for JPEG fast path; uses preserving encoding for all DCT output
- **stegoeggo/src/lib.rs**: Used in `apply_bytes_pipeline` when input/output are both JPEG
