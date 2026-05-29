# Pipeline & Public API

**Source:** `src/lib.rs` (~745 lines)

The pipeline is the central orchestration layer. It selects and composes protectors based on the requested `ProtectionLevel`, handles format routing (JPEG fast path vs pixel path), and provides both sync and parallel entry points.

## ProtectionPipeline

The main struct. Holds `Arc`-wrapped protectors for all five levels.

```rust
pub struct ProtectionPipeline {
    passthrough: Arc<PassthroughProtector>,
    noise: Arc<NoiseProtector>,           // Standard level
    enhanced: Arc<EnhancedProtector>,    // Enhanced level
    precomputed: Arc<PrecomputedProtector>, // Strong level
    metadata_trap: Arc<MetadataTrapProtector>,
    steganography: Arc<SteganographyProtector>,
}
```

All five protector types are flat fields (not nested under a `Protected/` subgroup).

### Key Methods

- `process(&img, level, &ctx) -> Cow<DynamicImage>` — Pixel-level processing (validates dimensions)
- `process_bytes(&img_bytes, level, &ctx) -> Vec<u8>` — Byte-level processing (validates dimensions for JPEG via header parse, and for non-JPEG via validate_dimensions after decode)
- `register_precomputed_variants(variants)` — Register precomputed perturbations for CDN

### Pipeline Flow (Standard/Enhanced/Strong)

```
1. Apply perturbation (noise/enhanced/precomputed)
2. If JPEG output:
   a. Encode to JPEG bytes first
   b. Apply DCT stego to JPEG bytes
   c. Inject metadata to JPEG bytes
3. If non-JPEG output:
   a. Apply pixel stego to DynamicImage
   b. Encode to target format
   c. Inject metadata to bytes
```

The JPEG fast path (`apply_multi_protector_bytes`) operates directly on DCT coefficients via `JpegTranscoder`, bypassing pixel decode/encode cycles. It only triggers when **both** input and output are JPEG — format conversion always takes the full pipeline. This is critical for the sub-10ms latency target.

### Light Level Flow

`process_bytes` routes `Light` level through `metadata_trap.apply_bytes()`, which internally encodes → injects metadata → decodes. This can alter format/quality due to the encode/decode cycle. For byte-level output with metadata intact, use `process_bytes()` or `apply_bytes()` directly.

### JPEG→JPEG Fast Path (bypasses perturbation)

When both input and output are JPEG, `apply_multi_protector_bytes` skips perturbation entirely (no pixel decode/encode) and only applies DCT steganography + metadata injection. This preserves original quality and avoids lossy re-encoding artifacts.

## Convenience Functions

Free functions that use a `LazyLock<ProtectionPipeline>` singleton:

- `process_image(img, level, &ctx)` — Single image, pixel path
- `process_image_bytes(bytes, level, &ctx)` — Single image, byte path. Auto-detects input format from magic bytes and sets `input_format` on context if not already set.
- `process_images_parallel(images, level, &ctx)` — Rayon parallel batch
- `process_images_bytes_parallel(images, level, &ctx)` — Parallel batch, byte path
- `verify_image_bytes(bytes, mac_key) -> Option<bool>` — Free function (not a pipeline method). Checks DCT stego first, then metadata seed extraction, then falls back to LSB stego payload extraction. Uses HMAC key via `verify_payload_integrity`.

## Dimension Validation

`process()` validates image dimensions against `max_dimension` from the context and returns an error if exceeded. `process_bytes()` does **not** perform this validation — large images can bypass the check via the byte path.

## Format Routing

- `ImageOutputFormat::from_magic_bytes(bytes)` — Detects format from magic bytes
- `ImageOutputFormat::from_extension(path)` — Detects from file extension
- The pipeline checks if input and output are both JPEG to decide on the fast path
- If input format cannot be determined, returns `Error::InvalidFormat`

## Module Interactions

- **types.rs**: Uses `ProtectionLevel`, `ProtectionContext`, `ImageOutputFormat`
- **traits.rs**: Calls `Protector::apply()` and `Protector::apply_bytes()`
- **protected/*.rs**: Delegates to specific protector implementations
- **jpeg_transcoder/**: Used for JPEG fast path in `apply_multi_protector_bytes`
- **util/image.rs**: Used for encoding, format detection, image loading
