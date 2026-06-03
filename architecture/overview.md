# Architecture Overview

`cloakrs` is a Rust library and CLI for protecting images from unauthorized AI model training through steganographic watermarking and metadata injection for legal deterrence. It applies multiple layers of protection — metadata injection and steganographic embedding — to serve as legal evidence of image ownership.

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
│  Holds Arc-wrapped protectors for all levels                               │
└─────────────────────────────────┬───────────────────────────────────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
          ▼                       ▼                       ▼
┌─────────────────┐   ┌─────────────────────┐   ┌─────────────────────────┐
│ Passthrough     │   │ MetadataTrapProtector│   │ JpegTranscoder         │
│ (Disabled)      │   │ (Light)              │   │ (jpeg_transcoder/)     │
│                 │   │                     │   │                         │
│ No-op           │   │ Metadata injection  │   │ Operates on DCT         │
│                 │   │                     │   │ coefficients directly   │
└─────────────────┘   └─────────────────────┘   │ for JPEG fast path     │
                                                  └─────────────────────────┘
           │                       │             │
           │                       │             │
           ▼                       ▼             ▼
┌───────────┐ ┌───────────┐ ┌───────────┐ ┌─────────────────┐
│Steganography│ │Metadata  │ │ Precomputed│ │ JpegTranscoder │
│(steganography)│  Trap    │ │             │ │                 │
└───────────┘ └───────────┘ └───────────┘ └─────────────────┘
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
| `Standard` | Noise | LSB/DCT | Seed + DMI | General protection |

Each level above `Disabled` activates metadata injection. Steganography (LSB or DCT) is applied for Standard. Light level only applies metadata injection without steganography.

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
       └── [Standard]
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
       └── [Standard]
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
├── types.rs                   ProtectionLevel, ProtectionContext, StegoPayload,
│                              ImageOutputFormat, DmiValue, LegalMetadata, ProtectionConfig
├── traits.rs                  Protector trait (apply/apply_bytes)
├── error.rs                   Error enum (thiserror), Result type
├── async_api.rs               Tokio spawn_blocking wrappers (when async feature enabled)
│
├── protected/                 Protection strategies (all implement Protector trait)
│   ├── constants.rs           Tuning constants (STEGO_*)
│   ├── passthrough.rs        No-op for Disabled level
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
    ├── image.rs              XorShiftRng, encoding
    ├── iscc.rs               compute_iscc, Iscc content identifiers
    └── seed.rs               generate_random_seed() via SystemTime + splitmix64
```

## Component Index

Each component has a detailed deep-dive document in `architecture/`:

| Component | File | Description |
|-----------|------|-------------|
| **Pipeline & API** | [pipeline.md](pipeline.md) | Orchestration, format routing, parallel processing, LazyLock singletons |
| **Core Types** | [types.md](types.md) | ProtectionLevel, ProtectionContext, config builders, Arc-wrapping |
| **Traits** | [traits.md](traits.md) | Protector trait |
| **Error Types** | [error.md](error.md) | Error enum (ImageDecode, Steganography, JpegTranscode, etc.) |
| **Async API** | [async-api.md](async-api.md) | Tokio spawn_blocking wrappers for batch processing |
| **Image Utilities** | [util-image.md](util-image.md) | XorShiftRng, encoding |
| **ISCC Identifiers** | [util-iscc.md](util-iscc.md) | Perceptual content hashing (non-standard ISCC-like) |
| **Seed Generation** | [util-seed.md](util-seed.md) | getrandom (OS CSPRNG), with time-based splitmix64 fallback |
| **Passthrough** | [protected-passthrough.md](protected-passthrough.md) | No-op for Disabled level |
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

- **`PixelSelectionRng`** in `util/image.rs` — general-purpose pixel selection for steganography
- **`DctCoefficientRng`** in `jpeg_transcoder/stego_f5.rs` — DCT coefficient shuffling

They use different algorithms and produce different sequences for the same seed. **Do NOT interchange them.**

### Private Fields with Getters

`ProtectionContext`, `StegoPayload`, and `LegalMetadata` all use private fields with getter methods. This enforces consistent access patterns and allows internal refactoring without breaking consumers.

### Arc for Shared Config

`ProtectionConfig` (MAC key, legal metadata) is wrapped in `Arc<ProtectionConfig>` and stored in `ProtectionContext`. This allows cheap cloning of context while sharing the heavy config.

### LazyLock Singletons

The default pipeline uses `LazyLock<ProtectionPipeline>` for thread-safe one-time initialization:

```rust
static DEFAULT_PIPELINE: LazyLock<ProtectionPipeline> = LazyLock::new(ProtectionPipeline::new);
```

### Stego Payload Format

- 24-byte header + 4-byte CRC32 checksum = 28 bytes minimum (`MIN_PAYLOAD_SIZE`)
- With HMAC key: 24-byte header + 8-byte HMAC = 32 bytes total
- Non-MAC mode produces 76-byte ECC-encoded payload (24 bytes × 3 replication + 4 CRC32)

### Metadata Injection Semantics

Three-state control (`Option<bool>`) for metadata injection:
- `None` (default): use level-based defaults — inject for all levels except `Disabled`
- `Some(true)`: force-enable
- `Some(false)`: force-disable

`None` and `Some(false)` have different semantics for non-Disabled levels — the former injects, the latter suppresses.

### Parallel Threshold Scaling

`parallel_threshold()` returns `cores * 64 * 64` — scales with rayon thread count. At 1 core: 4096 pixels. At 4 cores: 16384 pixels. At 16 cores: 65536 pixels.

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

## Security Notes

- **Default seed is CSPRNG-backed**: `ProtectionContext::default()` calls `generate_random_seed()` which uses `getrandom` (OS CSPRNG). Use `ProtectionContext::new(intensity, seed)` when you need reproducible results across runs.
- **Without MAC key**: Stego verification uses a non-cryptographic CRC32 checksum, not HMAC. Payloads are forgeable.
- **Primary deterrence is metadata**: Visible XMP/IPTC/EXIF markers remain even if stego payload is stripped. Metadata provides legal evidence of intent.
- **JPEG stego limitations**: F5 DCT embedding may not survive re-compression. Quantization-table seed embedding is only reliable when the tables themselves are preserved.

## Verification Priority (JPEG)

When extracting from JPEG, check in order:
1. Metadata seed (tEXt/XMP markers) — most robust
2. Quantization table seed — detection only when the tables are preserved
3. DCT coefficient extraction — fragile
4. Pixel-based LSB — not applicable to JPEG
