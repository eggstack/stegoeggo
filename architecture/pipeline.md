# Pipeline & Public API

**Source:** `src/lib.rs` (~2260 lines)

The pipeline is the central orchestration layer. It provides two execution paths: the canonical request-based path (via direct plan executor) and the legacy level-based compatibility path (via a stateless `ProtectionPipeline`). Both paths share the same carrier operations and metadata injection. Parallel batch processing functions require the `parallel` feature.

## Direct Plan Executor (canonical path)

The canonical execution path for `process_request_bytes*` functions:

```
ProtectionRequest → resolve_request() → ResolvedProtectionPlan → execute_plan_bytes()
```

Three crate-private functions perform the actual work:

- `execute_metadata_only()` — Same-format and cross-format metadata injection using plan fields directly (no `ProtectionContext` reconstruction)
- `execute_full_marker_and_metadata()` — BestEffort and Tiled hidden markers with one carrier router (`tile_size: None` vs `Some`); DCT/LSB stego + metadata injection
- `execute_seed_only_and_metadata()` — Seed-only marker + metadata injection

These functions use `RightsMetadataProtector::inject_bytes_from_plan()` for metadata injection, which accepts `&ResolvedProtectionPlan` directly. The steganography side exposes `*_from_plan` methods (`SteganographyProtector::apply_dct_stego_bytes_from_plan`, `apply_lsb_to_image_with_summary_from_plan`, `embed_lsb_tiled_*`) that consume the plan directly. The `apply_dct_stego_bytes_from_plan` dispatcher lives in `src/protected/steganography/embed.rs`; the plan-driven embed path uses the carrier's narrow `application_support` operation layer and never reconstructs a `ProtectionContext` from the plan. There is no `plan_to_context()` adapter: the resolved plan is the only execution state for the canonical path.

## Output-Domain Carrier Invariant

Carrier family is selected from the final output format; input format controls fast-path reuse only:

```
output_format == JPEG ? DCT/F5 carrier : LSB carrier
```

- JPEG output → DCT/F5 (`EmbedPath::DctF5`) or tiled DCT/F5 (`EmbedPath::DctF5Tiled`), after either reusing the original JPEG bytes (JPEG→JPEG) or decoding once and encoding JPEG once (PNG/WebP→JPEG).
- PNG/WebP output → LSB (`EmbedPath::Lsb`) or tiled LSB (`EmbedPath::LsbTiled`), executed directly on decoded pixels. JPEG→PNG/WebP is one pixel decode followed by raster LSB; there is no transient JPEG DCT step.
- `EmbedPath` is derived from the operation actually executed, never from `plan.input_format()`.
- `apply_lsb_to_image_with_summary_from_plan()` is explicitly raster-domain (`DynamicImage → owned RGBA → LSB/tiled-LSB → DynamicImage::ImageRgba8`) and cannot select JPEG DCT. JPEG DCT remains in the encoded-byte helper `apply_dct_stego_bytes_from_plan()`.

## ProtectionPipeline (legacy path)

The legacy struct for level-based APIs. It is stateless; its methods adapt the legacy level/context inputs into a `ProtectionRequest` and then use the canonical resolver and plan executor.

```rust
pub struct ProtectionPipeline {
}
```

### Key Methods

- `process(&img, level, &ctx) -> Result<Cow<DynamicImage>>` — Pixel-level processing (validates dimensions)
- `process_bytes(&img_bytes, level, &ctx) -> Result<Vec<u8>>` — Byte-level processing (validates dimensions for JPEG via header parse, and for non-JPEG via a header-only dimension gate before the single full decode; the preflight never performs a discarded decode)

### Pipeline Flow (Standard)

`execute_full_marker_and_metadata()` is the single current full-marker router:

```
1. If JPEG input + JPEG output: reuse original bytes → DCT stego → inject metadata
2. Else if JPEG output: decode source once → encode JPEG once → DCT stego → inject metadata
3. Else (PNG/WebP output): decode source pixels → LSB/tiled-LSB → encode final format → inject metadata
```

The JPEG fast path (`execute_full_marker_and_metadata()` JPEG→JPEG branch) calls the carrier crate's encoded-byte JPEG operations. Those operations privately decode and re-encode DCT coefficients, bypassing pixel decode/encode cycles. It only triggers when **both** input and output are JPEG — format conversion always takes the full pipeline. This is critical for the sub-10ms latency target. The root-side call site is `SteganographyProtector::apply_dct_stego_bytes_from_plan` in `src/protected/steganography/embed.rs`; the carrier-side helpers live behind `stegoeggo_stego::application_support::jpeg_embed` so JPEG parser, coefficient, and F5 types stay private to the carrier crate.

### Light Level Flow

`process_bytes` preserves the detected input format unless `ProtectionContext::with_format()` requests conversion. For JPEG output, Light injects metadata and stores the seed in quantization tables. For PNG/WebP output, Light embeds a minimal LSB payload with redundancy=1, encodes to the target format, then injects metadata.

### JPEG→JPEG Fast Path (bypasses pixel decode/encode)

When both input and output are JPEG, `execute_full_marker_and_metadata()` skips pixel decode/encode entirely and only applies DCT steganography + metadata injection. This preserves original quality and avoids lossy re-encoding artifacts.

## Convenience Functions

Free functions that delegate to the canonical request/plan execution path via `request_from_legacy()` and `process_request_bytes()`:

- `process_image(img, level, &ctx) -> Result<DynamicImage>` — Single image, pixel path. Takes owned `DynamicImage`.
- `process_image_bytes(bytes, level, &ctx) -> Result<Vec<u8>>` — Single image, byte path. Auto-detects input format from magic bytes and sets `input_format` on context if not already set.
- `process_images_parallel(images, level, &ctx)` — Rayon parallel batch
- `process_images_bytes_parallel(images, level, &ctx)` — Parallel batch, byte path
- `process_image_bytes_with_info(bytes, level, &ctx) -> Result<(Vec<u8>, Option<ProtectionWarning>)>` — Convenience wrapper returning the first warning.
- `process_image_bytes_with_warnings(bytes, level, &ctx) -> Result<(Vec<u8>, Vec<ProtectionWarning>)>` — Recommended reverse-proxy API. Keeps processing byte-oriented and returns advisory/degradation warnings for proxy policy/logging.
- `verify_image_bytes(bytes, mac_key) -> VerificationStatus` — Free function (not a pipeline method). Checks DCT stego first, then metadata seed extraction, then falls back to LSB stego payload extraction for non-JPEG formats. Returns `VerificationStatus` (`Verified`, `Invalid`, `NotFound`).
- `verify_image_bytes_detailed(bytes, mac_key) -> VerificationResult` — Distinguishes verified payloads from metadata-only evidence.

## Dimension Validation

`process()` validates image dimensions against `max_dimension` from the context and returns an error if exceeded. `process_bytes()` validates dimensions for active protection levels: JPEG inputs are checked from parsed headers before DCT/Q-table processing, and non-JPEG inputs pass a header-only dimension gate before their single full decode (whose executor re-checks dimensions defensively). Reverse proxies should still enforce input byte-size limits before calling the library.

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
- Carrier family follows the output domain only (`output_format == JPEG ? DCT : LSB`); the `(input_format, output_format)` pair is used solely to decide whether original JPEG bytes can be reused without transcoding
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
