# JPEG Header Parser

**Source:** `src/jpeg_transcoder/header.rs` (~513 lines)

Parses JPEG file headers to extract quantization tables, Huffman tables, component definitions, and structural metadata.

## JpegHeader

```rust
pub struct JpegHeader {
    pub width: u16,
    pub height: u16,
    pub precision: u8,
    pub coding_process: JpegCodingProcess,
    pub color_space: JpegColorSpace,
    pub quantization_tables: Vec<QuantizationTable>,
    pub huffman_tables: Vec<HuffmanTable>,
    pub components: Vec<ScanComponent>,
    pub app_markers: Vec<Vec<u8>>,
    pub restart_interval: u16,
    pub progressive: bool,
}
```

### Parse Method

```rust
pub fn parse(data: &[u8]) -> Result<JpegHeader>
```

Handles:
- Embedded thumbnails (finds last SOI marker)
- Bounds validation (`data.len() < 2`, `end_pos < 10`)
- Segment data end uses `.max(segment_data_start)` to prevent inverted slice ranges

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
    pub class: u8,  // 0=DC, 1=AC
    pub id: u8,
    pub counts: [u8; 16],
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
- **steganography.rs**: Header used for JPEG reassembly after DCT stego
