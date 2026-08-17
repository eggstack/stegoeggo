# Pipeline & Public API

**Source:** `src/lib.rs` (~2282 lines)

The pipeline is the central orchestration layer. It provides two execution paths: the canonical request-based path (via direct plan executor) and the legacy level-based compatibility path (via a stateless `ProtectionPipeline`). Both paths share the same carrier operations and metadata injection. Parallel batch processing functions require the `parallel` feature.

## Direct Plan Executor (canonical path)

The canonical execution path for `process_request_bytes*` functions:

```
ProtectionRequest → resolve_request() → ResolvedProtectionPlan → execute_plan_bytes()
```

Three crate-private functions perform the actual work:

- `execute_metadata_only()` — Same-format and cross-format metadata injection using plan fields directly (no `ProtectionContext` reconstruction)
- `execute_stego_and_metadata()` — Standard hidden marker: DCT/LSB stego + metadata injection
- `execute_stego_and_metadata_tiled()` — Tiled variant for crop-resistant mode

These functions use `RightsMetadataProtector::inject_bytes_from_plan()` for metadata injection, which accepts `&ResolvedProtectionPlan` directly. The steganography side exposes `*_from_plan` methods (`SteganographyProtector::apply_dct_stego_bytes_from_plan`, `apply_to_image_with_summary_from_plan`, `embed_lsb_tiled_*`) that consume the plan directly. The `apply_dct_stego_bytes_from_plan` dispatcher lives in `src/protected/steganography/embed.rs`; the plan-driven embed path uses the carrier's narrow `application_support` operation layer and never reconstructs a `ProtectionContext` from the plan. There is no `plan_to_context()` adapter: the resolved plan is the only execution state for the canonical path.

## ProtectionPipeline (legacy path)

The legacy struct for level-based APIs. It is stateless; its methods adapt the legacy level/context inputs into a `ProtectionRequest` and then use the canonical resolver and plan executor.

```rust
pub struct ProtectionPipeline {
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

The JPEG fast path (`apply_bytes_pipeline`) calls the carrier crate's encoded-byte JPEG operations. Those operations privately decode and re-encode DCT coefficients, bypassing pixel decode/encode cycles. It only triggers when **both** input and output are JPEG — format conversion always takes the full pipeline. This is critical for the sub-10ms latency target. The root-side call site is `SteganographyProtector::apply_dct_stego_bytes_from_plan` in `src/protected/steganography/embed.rs`; the carrier-side helpers live behind `stegoeggo_stego::application_support::jpeg_embed` so JPEG parser, coefficient, and F5 types stay private to the carrier crate.

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
- `verify_image_bytes(bytes, mac_key) -> VerificationStatus` — Free function (not a pipeline method). Checks DCT stego first, then metadata seed extraction, then falls back to LSB stego payload extraction for non-JPEG formats. Returns `VerificationStatus` (`Verified`, `Invalid`, `NotFound`).
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

- **types.rs**: Uses `ProtectionLevel`, `ProtectionContext`, `ImageOutputFormat`, `ProtectionRequest`, `ResolvedProtectionPlan`
- **traits.rs**: Calls `Protector::apply()` and `Protector::apply_bytes()`
- **protected/*.rs**: Delegates to specific protector implementations
- **protected/steganography/**: Decomposed application stego adapter. `mod.rs` is the facade; `marker.rs` builds V3 application payloads, `embed.rs` dispatches embedding operations (LSB, tiled LSB, JPEG DCT/F5, seed-only), `extract.rs` owns seed discovery and bounded search, `verify.rs` classifies payload integrity and authentication, `legacy.rs` isolates V1/V2 compatibility-only decoding
- **stegoeggo-stego/src/jpeg.rs**: Public encoded-byte JPEG carrier operations used by the application adapter; `jpeg_transcoder/` remains private.
- **stegoeggo-stego/src/lsb.rs** + **stegoeggo-stego/src/application_support.rs**: Public generic LSB operations (raw, in-place, framed) and the narrow parent-crate support layer. `lsb_internal.rs` remains private.
- **stegoeggo-stego/src/frame.rs**: Self-describing framed payload (`encode`/`decode`/`decode_prefix`) used by the LSB and JPEG framed convenience methods.
- **util/image.rs**: Used for encoding, format detection, image loading
