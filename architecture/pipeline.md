# Pipeline & Public API

**Source:** `src/lib.rs` (~745 lines)

The pipeline is the central orchestration layer. It selects and composes protectors based on the requested `ProtectionLevel`, handles format routing (JPEG fast path vs pixel path), and provides both sync and parallel entry points.

## ProtectionPipeline

The main struct. Holds `Arc`-wrapped protectors for all levels.

```rust
pub struct ProtectionPipeline {
    passthrough: Arc<PassthroughProtector>,
    metadata_trap: Arc<MetadataTrapProtector>,
    steganography: Arc<SteganographyProtector>,
}
```

### Key Methods

- `process(&img, level, &ctx) -> Cow<DynamicImage>` — Pixel-level processing (validates dimensions)
- `process_bytes(&img_bytes, level, &ctx) -> Vec<u8>` — Byte-level processing (validates dimensions for JPEG via header parse, and for non-JPEG via validate_dimensions after decode)

### Pipeline Flow (Standard)

```
1. If JPEG output:
   a. Encode to JPEG bytes first
   b. Apply DCT stego to JPEG bytes
   c. Inject metadata to JPEG bytes
2. If non-JPEG output:
   a. Apply pixel stego to DynamicImage
   b. Encode to target format
   c. Inject metadata to bytes
```

The JPEG fast path (`apply_bytes_pipeline`) operates directly on DCT coefficients via `JpegTranscoder`, bypassing pixel decode/encode cycles. It only triggers when **both** input and output are JPEG — format conversion always takes the full pipeline. This is critical for the sub-10ms latency target.

### Light Level Flow

`process_bytes` preserves the detected input format unless `ProtectionContext::with_format()` requests conversion. For JPEG output, Light injects metadata and stores the seed in quantization tables. For PNG/WebP output, Light embeds a minimal LSB payload with redundancy=1, encodes to the target format, then injects metadata.

### JPEG→JPEG Fast Path (bypasses pixel decode/encode)

When both input and output are JPEG, `apply_bytes_pipeline` skips pixel decode/encode entirely and only applies DCT steganography + metadata injection. This preserves original quality and avoids lossy re-encoding artifacts.

## Convenience Functions

Free functions that use a `LazyLock<ProtectionPipeline>` singleton:

- `process_image(img, level, &ctx)` — Single image, pixel path
- `process_image_bytes(bytes, level, &ctx)` — Single image, byte path. Auto-detects input format from magic bytes and sets `input_format` on context if not already set.
- `process_images_parallel(images, level, &ctx)` — Rayon parallel batch
- `process_images_bytes_parallel(images, level, &ctx)` — Parallel batch, byte path
- `process_image_bytes_with_warnings(bytes, level, &ctx) -> (Vec<u8>, Vec<ProtectionWarning>)` — Recommended reverse-proxy API. Keeps processing byte-oriented and returns advisory/degradation warnings for proxy policy/logging.
- `verify_image_bytes(bytes, mac_key) -> Option<bool>` — Free function (not a pipeline method). Checks DCT stego first, then metadata seed extraction, then falls back to LSB stego payload extraction for non-JPEG formats. Uses HMAC key via `verify_payload_integrity`.
- `verify_image_bytes_detailed(bytes, mac_key) -> VerificationResult` — Distinguishes verified payloads from metadata-only evidence.

## Dimension Validation

`process()` validates image dimensions against `max_dimension` from the context and returns an error if exceeded. `process_bytes()` validates dimensions for active protection levels: JPEG inputs are checked from parsed headers before DCT/Q-table processing, and non-JPEG inputs are checked after decode. Reverse proxies should still enforce input byte-size limits before calling the library.

## Reverse Proxy Integration

For tight reverse-proxy serving, prefer:

1. Cache lookup in the proxy before calling stegoeggo.
2. `process_image_bytes_with_warnings()` on cache misses.
3. A `ProtectionContext` with `with_mac_key()`, `with_max_dimension()`, explicit `with_format()`, and bounded `with_stego_redundancy()`.
4. Policy/logging based on `ProtectionWarning`.

The library intentionally does not own proxy-level cache policy, concurrency limits, request body limits, or timeout/cancellation behavior.

## Format Routing

- `ImageOutputFormat::from_magic_bytes(bytes)` — Detects format from magic bytes
- `ImageOutputFormat::from_extension(path)` — Detects from file extension
- The pipeline checks if input and output are both JPEG to decide on the fast path
- If input format cannot be determined, returns `Error::InvalidFormat`

## Module Interactions

- **types.rs**: Uses `ProtectionLevel`, `ProtectionContext`, `ImageOutputFormat`
- **traits.rs**: Calls `Protector::apply()` and `Protector::apply_bytes()`
- **protected/*.rs**: Delegates to specific protector implementations
- **jpeg_transcoder/**: Used for JPEG fast path in `apply_bytes_pipeline`
- **util/image.rs**: Used for encoding, format detection, image loading
