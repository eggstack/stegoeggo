# JPEG Header Parser

**Source:** `stegoeggo-stego/src/jpeg_transcoder/header.rs`

Parses JPEG file headers to extract quantization tables, Huffman tables, component definitions, and structural metadata.

## JpegHeader

```rust
pub struct JpegHeader {
    pub width: u16,
    pub height: u16,
    pub precision: u8,
    pub coding_process: JpegCodingProcess,
    pub color_space: JpegColorSpace,
    pub quantization_tables: [Option<QuantizationTable>; 4],
    pub huffman_tables_dc: Vec<Option<HuffmanTable>>,
    pub huffman_tables_ac: Vec<Option<HuffmanTable>>,
    pub components: Vec<ScanComponent>,
    pub app0_marker: Option<Vec<u8>>,
    pub app1_markers: Vec<Vec<u8>>,
    pub restart_interval: u16,
    pub progressive: bool,
}
```

### Parse Method

```rust
pub fn parse(data: &[u8]) -> Result<JpegHeader>
```

Header parsing validates the SOI marker, segment lengths, marker-specific payloads,
Huffman tables, and SOS table references. Structural decisions for the DCT fast path
are made separately by the checked scan analyzer described below.

### Parsed Markers

| Marker | Content |
|--------|---------|
| `DQT` | Quantization tables (8-bit or 16-bit precision) |
| `SOF0`/`SOF2` | Frame header (baseline/progressive) |
| `DHT` | Huffman tables (DC/AC, luminance/chrominance) |
| `SOS` | Scan definition (component mappings) |
| `APP0`–`APP15` | Application data (preserved verbatim) |
| `COM` | Comments |
| `DRI` | Restart interval |

## QuantizationTable

```rust
pub struct QuantizationTable {
    pub table_id: u8,
    pub precision: u8,  // 8 or 16 bits
    pub values: [u16; 64],
}
```

## HuffmanTable

```rust
pub struct HuffmanTable {
    pub table_class: u8,  // 0=DC, 1=AC
    pub table_id: u8,
    pub counts: [u16; 16],
    pub values: Vec<u8>,
}
```

## ScanComponent

```rust
pub struct ScanComponent {
    pub component_id: u8,
    pub h_sampling: u8,
    pub v_sampling: u8,
    pub quant_table_id: u8,
    pub dc_table_id: u8,
    pub ac_table_id: u8,
}
```

## Enums (Spec Reference)

`JpegCodingProcess` (Baseline, Extended, Progressive, Lossless) and `JpegColorSpace` (Grayscale, YCbCr, CMYK, etc.) are defined with `#[allow(dead_code)]` as JPEG spec reference types.

## Module Interactions

- **mod.rs**: `JpegHeader::parse` called by `JpegTranscoder::decode_coefficients`
- **entropy.rs**: Header data used to build Huffman decoders/encoders
- **stego_f5.rs**: Quantization tables modified for seed embedding
- **steganography/embed.rs**: Header is accessed only through `stegoeggo_stego::application_support` operations; the root crate never imports `JpegHeader` directly

## Checked Scan Structure

`JpegHeader::analyze_structure_checked(data) -> Result<JpegStructure>` walks the
complete marker stream without decoding DCT coefficients. It rejects truncated marker
runs, missing or short segment lengths, segment extents beyond the input, malformed SOS
boundaries, and entropy scans that reach EOF without a terminating marker.

The analyzer treats `SOI`, `EOI`, `RST0..RST7`, and `TEM` as standalone markers. Other
markers use their declared two-byte segment length, which must be at least two bytes.
Inside entropy, exactly `FF 00` is retained as stuffed data. A repeated `FF` run before
a real marker is excluded from the entropy span: both `sos_marker_offset` and the span's
`terminating_marker_offset` point to the first `FF` in the run. `FF FF 00` is rejected as
malformed rather than treated as stuffing. Restart markers are recorded and scanning
continues structurally, but the DCT support probe still rejects restart-bearing input.

`analyze_structure(data)` remains only as a compatibility wrapper and returns an empty
structure for malformed data. Support probing and coefficient decoding use the checked
analyzer directly, so malformed input cannot become a supported or partially analyzed
JPEG.
