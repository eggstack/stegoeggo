# Format Support

## Image formats

| Format | Read | Write | Rights metadata | Hidden marker |
|--------|------|-------|----------------|---------------|
| PNG | Yes | Yes | Yes | Pixel-domain LSB, best effort |
| JPEG | Yes | Yes | Yes | DCT-domain embedding on supported JPEG structures; fallback signals may be used when full embedding is unsupported |
| WebP | Yes | Yes | Yes | Pixel-domain LSB for lossless WebP; lossy WebP stego is not supported |

Input format is detected from the image data. Unless `--format` is supplied, the CLI preserves the input format.

If hidden-marker recoverability matters, PNG is the most predictable output format. Do not treat any hidden marker as guaranteed to survive arbitrary image transformations.

## What is written

Rights metadata is the primary signal. The byte-processing path builds one normalized rights notice and writes format-appropriate metadata. A specified `RightsPolicy` is represented with canonical PLUS `plus:DataMining` XMP metadata. Optional fields include copyright notice, creator, rights URL, usage terms, AI constraints, credit/licensor information, and relevant dates.

Existing unrelated metadata is preserved where the format-specific update path supports it, while StegoEggo-owned fields are replaced by default when an image is processed again. The library also exposes metadata conflict/update policies for callers that need stricter behavior.

Hidden markers are an optional secondary signal using:

- **LSB embedding** (PNG/WebP): Payload embedded in least-significant bits of pixel channels, with spread-spectrum and error-correcting codes for robustness.
- **DCT embedding** (JPEG): F5-style perturbation of DCT coefficients, with seed storage in quantization tables.
- **Tiled embedding**: Crop-resistant mode that embeds the full payload in each tile of the image.

## Common transformations and their effects

| Transformation | Metadata | Stego |
|---------------|----------|-------|
| File copy / re-hosting | Survives | Survives |
| Social media upload | Usually stripped | Usually destroyed |
| Screenshot | Lost | Lost |
| Generic JPEG re-encode | Stripped | Destroyed (unless using stegoeggo fast path) |
| Resize | Resampled | Destroyed |
| Crop | Clipped | Destroyed (unless tiled) |
| Metadata stripping tools | Stripped | Survives (still extractable) |

## Payload versions

| Version | Read | Write |
|---------|------|-------|
| v1 | Yes | No |
| v2 | Yes | No |
| v3 | Yes | Yes |

Write output always uses payload v3. Older payload versions are read for backward compatibility.

## Standards compatibility

Current rights-policy output uses the PLUS License Data Format `plus:DataMining` property with the canonical controlled-vocabulary URI for the selected policy. The verifier also recognizes older bare PLUS values, legacy `Iptc4xmpExt:DMI-*` data, and legacy TDM reservation metadata so existing files can still be inspected.

The current hidden payload writer emits V3 payloads. V1 and V2 payloads remain readable for compatibility. Detached manifests use their current V1 format when that feature is enabled.

StegoEggo deliberately does not claim C2PA compatibility or robust forensic watermarking. Those are separate trust and provenance models.
