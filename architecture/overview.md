# Architecture Overview

`cloakrs` is a Rust library and CLI for protecting images from unauthorized AI model training through adversarial image poisoning. It applies multiple layers of protection — metadata injection, adversarial noise perturbation, steganographic embedding, and precomputed variants — designed for CDN/WAF edge deployment with sub-10ms latency targets.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              PUBLIC API                                      │
│  process_image() | process_image_bytes() | process_images_*_parallel()     │
│  verify_image_bytes()                                                        │
└─────────────────────────────────┬───────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         ProtectionPipeline                                   │
│                    (src/lib.rs - orchestration)                             │
│                                                                              │
│  Orchestrates protector selection, format routing, and pipeline composition │
│  Holds Arc-wrapped protectors for all five levels                           │
└─────────────────────────────────┬───────────────────────────────────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
          ▼                       ▼                       ▼
┌─────────────────┐   ┌─────────────────────┐   ┌─────────────────────────┐
│ Passthrough     │   │ Protected/          │   │ JpegTranscoder         │
│ (Disabled)     │   │   noise.rs          │   │ (jpeg_transcoder/)     │
│                 │   │   enhanced.rs       │   │                         │
│ No-op           │   │   precomputed.rs    │   │ Operates on DCT         │
│                 │   │                     │   │ coefficients directly   │
└─────────────────┘   └─────────────────────┘   │ for JPEG fast path     │
          │                       │             └─────────────────────────┘
          │                       │
          │           ┌───────────┼───────────┐
          │           │           │           │
          │           ▼           ▼           ▼
          │   ┌───────────┐ ┌───────────┐ ┌───────────┐
          │   │Steganography│ │Metadata  │ │ Precomputed│
          │   │(steganography)│  Trap    │ │ (precomputed)│
          │   │           │ │(metadata_) │ │             │
          │   └───────────┘ └───────────┘ └───────────┘
          │       │               │               │
          └───────┴───────────────┴───────────────┘
                              │
                              ▼
                  ┌─────────────────────┐
                  │   Types & Traits   │
                  │   (types.rs, traits.rs) │
                  └─────────────────────┘
```

## Protection Levels

| Level | Perturbation | Stego | Metadata | Use Case |
|-------|--------------|-------|----------|----------|
| `Disabled` | None | None | None | Testing, passthrough |
| `Light` | None | None | Seed + DMI | Minimal overhead |
| `Standard` | Noise (10x) | LSB/DCT | Seed + DMI | General protection |
| `Enhanced` | Noise (12x) | LSB/DCT | Seed + DMI | Higher intensity |
| `Strong` | Precomputed | LSB/DCT | Seed + DMI | CDN edge, pre-generated |

Each level above `Disabled` activates all three protection layers (perturbation, steganography, metadata). The perturbation intensity is the only thing that varies between Standard/Enhanced/Strong. Light level only applies metadata injection without pixel perturbation.

## Data Flow

### Image → Image (pixel path)

```
Input DynamicImage
       │
       ▼
ProtectionPipeline::process()
       │
       ├── [Disabled] → PassthroughProtector::apply() → return Cow::Borrowed
       │
       ├── [Light]   → encode → MetadataTrapProtector::inject_bytes() → decode → return Cow::Owned
       │
       └── [Standard/Enhanced/Strong]
              │
              ▼
         apply_perturbation()  ──► NoiseProtector / EnhancedProtector / PrecomputedProtector
              │
              ▼ (output_format == Jpeg?)
              │
              ├── YES → encode → steganography.apply_dct_stego_bytes() → metadata_trap.inject_bytes()
              │
              └── NO  → steganography.apply() → encode → metadata_trap.inject_bytes()
              │
              ▼
         return Cow::Owned
```

### Image Bytes → Image Bytes (byte path)

```
Input bytes
       │
       ▼
process_image_bytes() ──► detect format via magic bytes
       │
       ▼
ProtectionPipeline::process_bytes()
       │
       ├── [Disabled] → return bytes.clone()
       │
       ├── [Light]    → MetadataTrapProtector::apply_bytes() → return Vec<u8>
       │
       └── [Standard/Enhanced/Strong]
              │
              ▼ (input == JPEG && output == JPEG?)
              │
              ├── YES → JPEG fast path
              │        steganography.apply_dct_stego_bytes() → metadata_trap.inject_bytes()
              │
              └── NO  → Full pixel pipeline
                       decode → apply_protector_pipeline() → encode → metadata.inject_bytes()
```

### JPEG Fast Path (input=JPEG, output=JPEG)

Skips pixel decode/encode entirely. Operates directly on DCT coefficients:

1. `JpegTranscoder::decode_coefficients()` — parse header, decode Huffman
2. `DctStegoF5::embed_f5()` — modify DCT coefficients with F5 steganography
3. `DctStegoF5::embed_seed_in_quantization_tables()` — store seed in Q-tables
4. `JpegTranscoder::encode_coefficients()` — re-encode Huffman, assemble JPEG

Progressive JPEGs are handled via seed-in-Q-tables only (coefficient manipulation unsupported).

## Module Map

```
src/
├── lib.rs                     Pipeline orchestration, public API, LazyLock singletons
├── types.rs                   ProtectionLevel, ProtectionContext, ProtectedVariant,
│                              ImageOutputFormat, DmiValue, LegalMetadata, ProtectionConfig
├── traits.rs                  Protector trait (apply/apply_bytes), VariantLoader trait
├── error.rs                   Error enum (thiserror), Result type
├── async_api.rs               Tokio spawn_blocking wrappers (when async feature enabled)
│
├── protected/                 Protection strategies (all implement Protector trait)
│   ├── constants.rs           Tuning constants (NOISE_INTENSITY_MULTIPLIER, STEGO_*)
│   ├── passthrough.rs        No-op for Disabled level
│   ├── noise.rs              Adversarial noise (Standard level, 10x intensity)
│   ├── enhanced.rs           Wraps NoiseProtector with 12x intensity (Enhanced level)
│   ├── precomputed.rs        Precomputed variants with LRU cache (Strong level)
│   ├── metadata_trap.rs      Metadata injection (tEXt/COM/XMP markers, seed, DMI)
│   └── steganography.rs       LSB embedding (PNG/WebP) + DCT F5 (JPEG)
│
├── jpeg_transcoder/           JPEG-specific DCT coefficient processing
│   ├── mod.rs                JpegTranscoder (decode/encode_coefficients, assemble_jpeg)
│   ├── header.rs             JpegHeader, HuffmanTable parsing (DQT/SOF/DHT/SOS)
│   ├── entropy.rs            CoefficientDecoder, CoefficientEncoder (Huffman codec)
│   └── stego_f5.rs           DctStegoF5, F5XorShiftRng (F5 DCT coefficient embedding)
│
└── util/
    ├── mod.rs                Module re-exports
    ├── image.rs              XorShiftRng, NoiseGenerator, perturbation, encoding
    ├── iscc.rs               compute_iscc, Iscc content identifiers
    └── seed.rs               generate_random_seed() via SystemTime + splitmix64
```

## Component Index

Each component has a detailed deep-dive document in `architecture/`:

| Component | File | Description |
|-----------|------|-------------|
| **Pipeline & API** | [pipeline.md](pipeline.md) | Orchestration, format routing, parallel processing, LazyLock singletons |
| **Core Types** | [types.md](types.md) | ProtectionLevel, ProtectionContext, ProtectedVariant, config builders, Arc-wrapping |
| **Traits** | [traits.md](traits.md) | Protector trait, VariantLoader trait, NoOpLoader |
| **Error Types** | [error.md](error.md) | Error enum (ImageDecode, Steganography, JpegTranscode, etc.) |
| **Async API** | [async-api.md](async-api.md) | Tokio spawn_blocking wrappers for batch processing |
| **Image Utilities** | [util-image.md](util-image.md) | XorShiftRng, NoiseGenerator, perturbation functions, encoding |
| **ISCC Identifiers** | [util-iscc.md](util-iscc.md) | Perceptual content hashing (non-standard ISCC-like) |
| **Seed Generation** | [util-seed.md](util-seed.md) | SystemTime + splitmix64 mixing (not CSPRNG) |
| **Passthrough** | [protected-passthrough.md](protected-passthrough.md) | No-op for Disabled level |
| **Noise Protector** | [protected-noise.md](protected-noise.md) | Standard adversarial noise (10x multiplier) |
| **Enhanced Protector** | [protected-enhanced.md](protected-enhanced.md) | Noise with 12x intensity |
| **Precomputed Protector** | [protected-precomputed.md](protected-precomputed.md) | LRU cache + VariantLoader for CDN edge |
| **Metadata Trap** | [protected-metadata-trap.md](protected-metadata-trap.md) | IPTC/XMP/EXIF injection, seed embedding |
| **Steganography** | [protected-steganography.md](protected-steganography.md) | LSB + DCT F5, payload generation/verification |
| **JPEG Transcoder** | [jpeg-transcoder.md](jpeg-transcoder.md) | DCT decode/encode, assemble, scan data utilities |
| **JPEG Header** | [jpeg-header.md](jpeg-header.md) | Marker parsing (DQT/SOF/DHT/SOS), component extraction |
| **JPEG Entropy** | [jpeg-entropy.md](jpeg-entropy.md) | Huffman encoding/decoding, CoefficientEncoder/Decoder |
| **F5 DCT Stego** | [jpeg-stego-f5.md](jpeg-stego-f5.md) | F5-style embedding, no-zero variant, quantization table seed |
| **Constants** | [constants.md](constants.md) | All tuning constants (STEGO_*, XORSHIFT_*, SPLITMIX64_*) |
| **CLI** | [cli.md](cli.md) | Command-line interface, batch processing, filename collision handling |

## Key Design Decisions

### Strategy Pattern

All protectors implement the `Protector` trait:

```rust
pub trait Protector: Send + Sync {
    fn apply<'a>(&self, img: &'a DynamicImage, ctx: &ProtectionContext) -> Result<Cow<'a, DynamicImage>>;
    fn apply_bytes(&self, img_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>>;
    fn name(&self) -> &'static str;
    fn protection_level(&self) -> ProtectionLevel;
    fn estimated_latency_ms(&self) -> u32;
}
```

The pipeline holds `Arc<Protector>` instances and dispatches based on `ProtectionLevel`. This allows composable protection and easy testing.

### Cow Returns

`Protector::apply` returns `Cow<'a, DynamicImage>` to avoid unnecessary cloning. When `intensity == 0.0`, protectors return `Cow::Borrowed(img)` without any allocation.

### JPEG Fast Path

When **both** input and output are JPEG, the pipeline operates directly on DCT coefficients via `JpegTranscoder`. This avoids pixel decode/encode cycles that would introduce additional lossy compression artifacts. Format conversion (JPEG → PNG) always takes the full pixel pipeline.

### Two XorShiftRng Implementations

- **`XorShiftRng`** in `util/image.rs` — general-purpose noise/pixel selection
- **`F5XorShiftRng`** in `jpeg_transcoder/stego_f5.rs` — DCT coefficient shuffling

They use different algorithms and produce different sequences for the same seed. **Do NOT interchange them.**

### Private Fields with Getters

`ProtectionContext`, `ProtectedVariant`, `StegoPayload`, and `LegalMetadata` all use private fields with getter methods. This enforces consistent access patterns and allows internal refactoring without breaking consumers.

### Arc for Shared Config

`ProtectionConfig` (MAC key, legal metadata) is wrapped in `Arc<ProtectionConfig>` and stored in `ProtectionContext`. This allows cheap cloning of context while sharing the heavy config.

### LazyLock Singletons

The default pipeline uses `LazyLock<ProtectionPipeline>` for thread-safe one-time initialization:

```rust
static DEFAULT_PIPELINE: LazyLock<ProtectionPipeline> = LazyLock::new(ProtectionPipeline::new);
```

### Stego Payload Format

- 24-byte header + 2-byte checksum = 26 bytes minimum (`MIN_PAYLOAD_SIZE`)
- With HMAC key: 24-byte header + 8-byte HMAC = 32 bytes total
- Always padded to 32 bytes in `generate_payload()`

### Metadata Injection Semantics

Three-state control (`Option<bool>`) for metadata injection:
- `None` (default): use level-based defaults — inject for all levels except `Disabled`
- `Some(true)`: force-enable
- `Some(false)`: force-disable

`None` and `Some(false)` have different semantics for non-Disabled levels — the former injects, the latter suppresses.

### Parallel Threshold Scaling

`parallel_threshold()` returns `cores * 64 * 64` — scales with rayon thread count. At 4 cores: 65536 pixels. At 16 cores: 131072 pixels.

## Dependencies

| Crate | Version | Role |
|-------|---------|------|
| `image` | 0.25 | Image loading, decoding, encoding (PNG, JPEG, WebP) |
| `jpeg-encoder` | 0.7 | Direct JPEG encoding with quality/progressive control |
| `rayon` | 1.10 | Parallel image processing |
| `sha2` | 0.10 | HMAC-SHA256 for keyed payload verification |
| `hmac` | 0.12 | HMAC construction |
| `serde` | 1.0 | Serialization of context and variants |
| `serde_json` | 1.0 | JSON serialization |
| `subtle` | 2 | Constant-time HMAC comparison |
| `tokio` | 1.0 (opt) | Async runtime for WAF/CDN integration |
| `clap` | 4 (CLI) | Command-line argument parsing |
| `crc32fast` | 1.4 | CRC32 for PNG chunk checksums |
| `thiserror` | 2 (error.rs) | Error enum derive |
| `uuid` | 1.x | Variant ID generation |

## Security Notes

- **Default seed is not CSPRNG**: `ProtectionContext::default()` uses `generate_random_seed()` which derives from `SystemTime` — predictable if request time is known. Use `ProtectionContext::new(intensity, csprng_seed)` for adversarial settings.
- **Without MAC key**: Stego verification uses trivial additive checksum, not HMAC. Payloads are forgeable.
- **Primary deterrence is metadata**: Visible XMP/IPTC/EXIF markers remain even if stego payload is stripped. Metadata provides legal evidence of intent.
- **JPEG stego limitations**: F5 DCT embedding may not survive re-compression. Quantization table seed embedding is more robust (survives re-encode).

## Verification Priority (JPEG)

When extracting from JPEG, check in order:
1. Metadata seed (tEXt/XMP markers) — most robust
2. Quantization table seed — survives re-encoding
3. DCT coefficient extraction — fragile
4. Pixel-based LSB — not applicable to JPEG