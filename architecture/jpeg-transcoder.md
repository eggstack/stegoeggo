# JPEG Transcoder

**Source:** `src/jpeg_transcoder/mod.rs` (~325 lines)

JPEG-specific processing pipeline that operates directly on DCT coefficients, bypassing pixel decode/encode cycles. This is the core of the JPEG fast path.

## JpegTranscoder

Static methods for JPEG coefficient manipulation:

```rust
pub struct JpegTranscoder;

impl JpegTranscoder {
    pub fn decode_coefficients(jpeg_data: &[u8]) -> Result<(JpegHeader, Coefficients)>
    pub fn encode_coefficients(header: &JpegHeader, coefficients: &Coefficients) -> Result<Vec<u8>>
    fn assemble_jpeg(header: &JpegHeader, scan_data: &[u8]) -> Result<Vec<u8>>
}
```

### Decode Flow

1. Parse JPEG header (`JpegHeader::parse`)
2. Find scan data start (`scan_utils::get_scan_data_start`)
3. Decode Huffman-encoded coefficients (`CoefficientDecoder`)
4. Return header + coefficients

### Encode Flow

1. Re-encode coefficients to Huffman format (`CoefficientEncoder`)
2. Assemble JPEG from header + encoded scan data

### Assemble

`assemble_jpeg` combines header bytes + scan data bytes. Returns `Result<Vec<u8>>`. Has `debug_assert!` for 8-bit quantization values exceeding 255.

## Coefficients Type

```rust
pub type Coefficients = HashMap<u8, Vec<[i16; 64]>>;
```

Component ID → list of 8×8 blocks (64 DCT coefficients each). Stored in natural (row-major) order.

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

- **header.rs**: `JpegHeader::parse` for header parsing
- **entropy.rs**: `CoefficientDecoder` / `CoefficientEncoder` for Huffman codec
- **stego_f5.rs**: `DctStegoF5` for coefficient manipulation
- **protected/steganography.rs**: `apply_dct_stego_bytes` calls transcoder for JPEG fast path
- **lib.rs**: Used in `apply_bytes_pipeline` when input/output are both JPEG
