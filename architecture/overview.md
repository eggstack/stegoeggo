# Architecture Overview

`cloakrs` is a Rust library and CLI for protecting images from unauthorized AI model training through adversarial image poisoning. It applies multiple layers of protection — metadata injection, adversarial noise perturbation, steganographic embedding, and precomputed variants — designed for CDN/WAF edge deployment with sub-10ms latency targets.

## High-Level Flow

```
Input Image (bytes or DynamicImage)
        │
        ▼
┌─────────────────────────┐
│  ProtectionPipeline     │  Selects protectors based on ProtectionLevel
│  (src/lib.rs)           │
└────────┬────────────────┘
         │
         ├── Disabled  → PassthroughProtector (no-op)
         │
         ├── Light     → MetadataTrapProtector (metadata only, encodes and re-decodes via apply_light_bytes)
         │
         ├── Standard  → NoiseProtector + SteganographyProtector + MetadataTrapProtector
         │
         ├── Enhanced  → EnhancedProtector + SteganographyProtector + MetadataTrapProtector
         │
         └── Strong    → PrecomputedProtector + SteganographyProtector + MetadataTrapProtector
         │
         ▼
Output Image (bytes or DynamicImage)
```

## Module Map

```
src/
├── lib.rs                         Pipeline orchestration, public API
├── types.rs                       Core types and configuration
├── traits.rs                      Protector trait, VariantLoader trait
├── error.rs                       Error types
├── async_api.rs                   Async wrappers (tokio)
├── protected/                     Protection strategies
│   ├── constants.rs               Tuning constants
│   ├── passthrough.rs             No-op (Disabled)
│   ├── noise.rs                   Adversarial noise (Standard)
│   ├── enhanced.rs                Higher intensity (Enhanced)
│   ├── precomputed.rs             Precomputed variants (Strong/CDN)
│   ├── metadata_trap.rs           Metadata injection (Light)
│   └── steganography.rs           LSB/DCT steganographic embedding
├── jpeg_transcoder/               JPEG-specific processing
│   ├── mod.rs                     Transcoder entry, scan data utilities
│   ├── header.rs                  JPEG header parser
│   ├── entropy.rs                 Huffman entropy codec
│   └── stego_f5.rs                F5-style DCT steganography
└── util/
    ├── mod.rs                     Module re-exports
    ├── image.rs                   Encoding, perturbation, hashing, RNG
    ├── iscc.rs                    ISCC content identifiers
    └── seed.rs                    Random seed generation
```

## Component Deep Dives

| Component | File | Description |
|-----------|------|-------------|
| Pipeline & Public API | [pipeline.md](pipeline.md) | Orchestration, format routing, parallel processing |
| Core Types | [types.md](types.md) | ProtectionLevel, ProtectionContext, ProtectedVariant, config types |
| Traits | [traits.md](traits.md) | Protector trait, VariantLoader trait |
| Error Types | [error.md](error.md) | Error enum and error handling strategy |
| Async API | [async-api.md](async-api.md) | Tokio-based async wrappers |
| Image Utilities | [util-image.md](util-image.md) | XorShiftRng, NoiseGenerator, perturbation, encoding |
| ISCC Identifiers | [util-iscc.md](util-iscc.md) | Perceptual content hashing |
| Seed Generation | [util-seed.md](util-seed.md) | Pseudo-random seed from SystemTime |
| Passthrough Protector | [protected-passthrough.md](protected-passthrough.md) | No-op for Disabled level |
| Noise Protector | [protected-noise.md](protected-noise.md) | Standard adversarial noise |
| Enhanced Protector | [protected-enhanced.md](protected-enhanced.md) | Higher intensity noise |
| Precomputed Protector | [protected-precomputed.md](protected-precomputed.md) | CDN/WAF precomputed variants |
| Metadata Trap | [protected-metadata-trap.md](protected-metadata-trap.md) | IPTC/XMP/EXIF metadata injection |
| Steganography | [protected-steganography.md](protected-steganography.md) | LSB and DCT steganographic embedding |
| JPEG Transcoder | [jpeg-transcoder.md](jpeg-transcoder.md) | JPEG decode/encode bypass pipeline |
| JPEG Header Parser | [jpeg-header.md](jpeg-header.md) | DQT/SOF/DHT/SOS marker parsing |
| JPEG Entropy Codec | [jpeg-entropy.md](jpeg-entropy.md) | Huffman encoding/decoding of DCT coefficients |
| F5 DCT Steganography | [jpeg-stego-f5.md](jpeg-stego-f5.md) | F5-style coefficient manipulation |
| CLI Tool | [cli.md](cli.md) | Command-line interface and batch processing |

## Key Design Decisions

- **Strategy pattern**: All protectors implement `Protector` trait, allowing pipeline composition
- **Cow returns**: `Protector::apply` returns `Cow<'a, DynamicImage>` to avoid unnecessary cloning
- **JPEG fast path**: When input/output are both JPEG, operates directly on DCT coefficients via custom transcoder, bypassing pixel decode/encode
- **Two XorShiftRng implementations**: `XorShiftRng` (general-purpose noise) and `F5XorShiftRng` (DCT shuffling) — different algorithms, do NOT interchange
- **Private fields with getters**: ProtectionContext, ProtectedVariant, StegoPayload all use private fields with getter methods
- **Arc for shared config**: ProtectionConfig (MAC key, legal metadata) wrapped in Arc for cheap cloning
- **LazyLock singletons**: Default pipeline instances use LazyLock for thread-safe one-time initialization

## Dependencies

| Crate | Role |
|-------|------|
| `image` 0.25 | Image loading, decoding, encoding (PNG, JPEG, WebP) |
| `jpeg-encoder` 0.7 | Direct JPEG encoding with quality/progressive control |
| `rayon` 1.10 | Parallel image processing |
| `sha2`/`hmac` 0.10/0.12 | HMAC-SHA256 for keyed payload verification |
| `serde`/`serde_json` 1.0 | Serialization of context and variants |
| `subtle` 2 | Constant-time HMAC comparison |
| `tokio` 1.0 (optional) | Async runtime for WAF/CDN integration |
| `clap` 4 (CLI) | Command-line argument parsing |
| `crc32fast` 1.4 | CRC32 for PNG chunk checksums |
