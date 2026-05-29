# Pipeline & Public API

**Source:** `src/lib.rs` (~745 lines)

The pipeline is the central orchestration layer. It selects and composes protectors based on the requested `ProtectionLevel`, handles format routing (JPEG fast path vs pixel path), and provides both sync and parallel entry points.

## ProtectionPipeline

The main struct. Holds `Arc`-wrapped protectors for all five levels.

```rust
pub struct ProtectionPipeline {
    passthrough: Arc<PassthroughProtector>,
    noise: Arc<NoiseProtector>,
    enhanced: Arc<EnhancedProtector>,
    precomputed: Arc<PrecomputedProtector>,
    metadata_trap: Arc<MetadataTrapProtector>,
    steganography: Arc<SteganographyProtector>,
}
```

### Key Methods

- `process(&img, level, &ctx) -> Cow<DynamicImage>` — Pixel-level processing
- `process_bytes(&img_bytes, level, &ctx) -> Vec<u8>` — Byte-level processing (JPEG fast path)
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

The JPEG fast path (`apply_multi_protector_bytes`) operates directly on DCT coefficients via `JpegTranscoder`, bypassing pixel decode/encode cycles. This is critical for the sub-10ms latency target.

## Convenience Functions

Free functions that use a `LazyLock<ProtectionPipeline>` singleton:

- `process_image(img, level, &ctx)` — Single image, pixel path
- `process_image_bytes(bytes, level, &ctx)` — Single image, byte path
- `process_images_parallel(images, level, &ctx)` — Rayon parallel batch
- `process_images_bytes_parallel(images, level, &ctx)` — Parallel batch, byte path
- `verify_image_bytes(bytes, mac_key) -> Option<bool>` — Verify protection signature

## Format Routing

- `ImageOutputFormat::from_magic_bytes(bytes)` — Detects format from magic bytes
- `ImageOutputFormat::from_extension(path)` — Detects from file extension
- The pipeline checks if input and output are both JPEG to decide on the fast path
- If output format cannot be determined, returns `Error::InvalidFormat` (previously defaulted to PNG)

## Module Interactions

- **types.rs**: Uses `ProtectionLevel`, `ProtectionContext`, `ImageOutputFormat`
- **traits.rs**: Calls `Protector::apply()` and `Protector::apply_bytes()`
- **protected/*.rs**: Delegates to specific protector implementations
- **jpeg_transcoder/**: Used for JPEG fast path in `apply_multi_protector_bytes`
- **util/image.rs**: Used for encoding, format detection, image loading
