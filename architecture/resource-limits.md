# Resource Limits

**Source:** `src/resource_limits.rs`

Configurable limits applied to externally reachable parsers to prevent resource exhaustion and denial-of-service attacks. Limits are enforced before allocation, returning structured errors rather than panicking.

## `ResourceLimits`

```rust
pub struct ResourceLimits {
    max_input_bytes: usize,           // 100 MiB
    max_width: u32,                   // 16384
    max_height: u32,                  // 16384
    max_png_chunks: usize,            // 500
    max_png_chunk_bytes: usize,       // 16 MiB
    max_jpeg_segments: usize,         // 256
    max_jpeg_segment_bytes: usize,    // 65535
    max_webp_riff_chunks: usize,      // 500
    max_webp_riff_bytes: usize,       // 16 MiB
    max_xmp_bytes: usize,             // 65535
    max_xml_depth: usize,             // 32
    max_xml_properties: usize,        // 256
    max_metadata_fields: usize,       // 64
    max_metadata_field_bytes: usize,  // 8192
    max_payload_bytes: usize,         // 256
    max_detached_manifest_bytes: usize, // 1 MiB
    max_tile_extraction_origins: usize, // 16
    max_verification_seeds: usize,    // 32
}
```

## Builder API

```rust
let limits = ResourceLimits::builder()
    .max_input_bytes(50 * 1024 * 1024)
    .max_png_chunks(1000)
    .max_jpeg_segments(512)
    .build();
```

## Enforcement Points

Limits are checked at these points in the pipeline:

1. **Input size** — `check_input_size()` before any processing
2. **Image dimensions** — `check_dimensions()` after format detection (JPEG via header parse, non-JPEG via decode)
3. **Container counts** — PNG chunks, JPEG segments, WebP RIFF chunks during parsing
4. **Metadata size** — XMP packet size, individual metadata fields during extraction
5. **Payload size** — Stego payload bytes during extraction
6. **Verification budget** — Tile origins, seed candidates during verification

## Error Variants

Limits violations produce structured errors:

- `Error::InputTooLarge { size, limit }` — Input exceeds `max_input_bytes`
- `Error::DimensionsExceeded { width, height, max_width, max_height }` — Image too large
- `Error::ContainerLimitExceeded { kind, count, limit }` — Too many chunks/segments
- `Error::MetadataLimitExceeded { kind, size, limit }` — Metadata too large
- `Error::VerificationBudgetExceeded { kind, count, limit }` — Too many candidates

## `OperationBudget`

Tracks resource consumption during a single processing operation:

```rust
pub struct OperationBudget<'a> {
    limits: &'a ResourceLimits,
    input_size: usize,
    png_chunks: usize,
    jpeg_segments: usize,
    webp_chunks: usize,
    metadata_fields: usize,
    metadata_bytes: usize,
    alloc_bytes: usize,
}
```

Created at the start of `process_plan_bytes()` and consumed at the end to produce a `ResourceUsage` report.

## Integration with ProtectionRequest

`ProtectionRequest` carries an optional `ResourceLimits`. If not set, the pipeline default (`ResourceLimits::default()`) is used. Limits are always enforced unconditionally — even when the caller did not set an explicit `max_dimension`.
