# Architecture Overview

`stegoeggo` is a Rust library and CLI for protecting images from unauthorized AI model training through rights-reservation metadata and steganographic markers. It applies multiple layers of protection — metadata injection and steganographic embedding — to serve as legal evidence of image ownership.

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
| `Light` | Minimal seed marker | Q-table seed (JPEG) or LSB redundancy=1 (PNG/WebP) | Seed + DMI | Minimal overhead |
| `Standard` | Noise | LSB/DCT | Seed + DMI | General protection |

Each level above `Disabled` activates metadata injection. `Light` adds the cheapest recoverable seed marker for the output format. `Standard` applies the full LSB or DCT payload.

## Request-Based API Flow (Release 4)

Release 4 introduces a policy-first architecture where `ProtectionRequest` is the canonical entry point. The flow separates request construction, resolution, and execution:

```
ProtectionRequest (user constructs)
        │
        ▼
resolve_request()  ──► validates input, resolves channels/policy
        │
        ▼
ResolvedProtectionPlan (immutable)
        │
        ├── process_request_bytes()          → Vec<u8>
        ├── process_request_bytes_with_warnings() → (Vec<u8>, Vec<ProtectionWarning>)
        └── process_request_bytes_with_report()   → (Vec<u8>, ExecutionReport)
```

### Why Resolution Runs Once

`resolve_request()` validates all inputs and produces an immutable `ResolvedProtectionPlan`. Pipeline stages consume the plan rather than re-querying mutable context. This ensures:
- Single validation point (no repeated checks)
- Immutable execution plan (no mid-flight mutations)
- Clean separation between request construction and execution

### Presets vs Direct Channels

Presets (`ProtectionPreset`) expand into `ProtectionChannels` at construction time:

| Preset | Channels |
|--------|----------|
| `LegalNotice` | `{ rights_metadata: true, hidden_marker: Disabled, authentication: None }` |
| `LegalNoticeWithStego` | `{ rights_metadata: true, hidden_marker: BestEffort, authentication: None }` |
| `AuthenticatedProvenance` | `{ rights_metadata: true, hidden_marker: BestEffort, authentication: Hmac }` |
| `Maximal` | `{ rights_metadata: true, hidden_marker: BestEffort, authentication: Hmac }` |

For finer control, construct `ProtectionChannels` directly instead of using a preset.

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
       ├── [Light]   → minimal stego → encode → MetadataTrapProtector::inject_bytes() → decode → return Cow::Owned
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
       ├── [Light]    → preserve/convert format → minimal stego → MetadataTrapProtector::apply_bytes() → return Vec<u8>
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
│                              ImageOutputFormat, DmiValue, LegalMetadata, ProtectionConfig,
│                              ProtectionRequest, RightsPolicy, ProtectionPreset,
│                              ProtectionChannels, ExecutionReport
├── traits.rs                  Protector trait (apply/apply_bytes)
├── error.rs                   Error enum (thiserror), Result type
├── async_api.rs               Tokio spawn_blocking wrappers (feature: async)
├── conformance.rs             Machine-readable conformance reporting types
├── resource_limits.rs         ResourceLimits for parser hardening (DoS prevention)
│
├── protected/                 Protection strategies (all implement Protector trait)
│   ├── constants.rs           Tuning constants (STEGO_*, XORSHIFT_*, SPLITMIX64_*)
│   ├── passthrough.rs         No-op for Disabled level
│   ├── metadata_trap.rs       Metadata injection (tEXt/COM/XMP markers, seed,
│   │                          canonical plus:DataMining DMI, DmiValue mapping)
│   ├── steganography.rs       LSB embedding (PNG/WebP) + DCT F5 (JPEG),
│   │                          payload v1/v2/v3 generation and extraction
│   ├── ecc.rs                 3× repetition ECC with majority voting
│   ├── notice_verification.rs Legal notice extraction from image bytes,
│   │                          evidence strength rating, DMI/seed resolution
│   ├── resolve.rs             ProtectionRequest → ResolvedProtectionPlan resolution
│   └── stego_cost.rs          Pixel embedding cost computation (Laplacian, fuzz feature)
│
├── jpeg_transcoder/           JPEG-specific DCT coefficient processing
│   ├── mod.rs                 JpegTranscoder (decode/encode_coefficients, assemble_jpeg)
│   ├── header.rs              JpegHeader, HuffmanTable parsing (DQT/SOF/DHT/SOS)
│   ├── entropy.rs             CoefficientDecoder, CoefficientEncoder (Huffman codec)
│   └── stego_f5.rs            DctStegoF5, F5XorShiftRng (F5 DCT coefficient embedding)
│
├── payload_v3/                Payload v3 wire format
│   ├── mod.rs                 Re-exports
│   ├── types.rs               AuthAlgorithm, ProtectionChannels, PayloadFlags, constants
│   ├── header.rs              PayloadV3Header parsing and serialization
│   ├── parser.rs              Multi-version parser (v1/v2/v3), ParsedPayload enum
│   ├── writer.rs              PayloadBuilder for constructing v3 payloads
│   └── errors.rs              PayloadV3ParseError
│
├── provenance/                Provenance claim model
│   ├── claim.rs               ProvenanceClaim builder and canonical serialization
│   ├── digest.rs              TypedDigest for content hashing (SHA-256)
│   └── canonical.rs           Canonical JSON serialization helpers
│
├── signing/                   Ed25519 signing (feature: signatures)
│   ├── mod.rs                 ED25519_OVERHEAD_BYTES, check_signature_capacity()
│   ├── config.rs              SigningConfig, SignaturePlacement
│   └── ed25519_impl.rs        SigningKey, VerifyingKey, SignatureResult
│
├── detached/                  Detached signed manifests (feature: detached-manifest)
│   ├── manifest.rs            DetachedManifest, SignatureRecord, PublicKeyEntry,
│   │                          TrustMetadata, EmbeddedReference
│   ├── generate.rs            create_manifest_from_image(), compute_image_digest()
│   └── verify.rs              verify_detached_manifest*(), TrustPolicy,
│                               DetachedOverallStatus, EmbeddedReferenceStatus
│
├── verification/              Structured verification report
│   ├── report.rs              VerificationReport, RightsVerification,
│   │                          HiddenMarkerVerification, AuthenticationVerification,
│   │                          SignatureVerification, BindingVerification,
│   │                          TrustEvaluation, Diagnostic, FieldSource
│   └── builder.rs             VerificationReportBuilder (fluent API)
│
├── bin/
│   └── stegoeggo-conformance.rs  Conformance harness binary
│
└── util/
    ├── mod.rs                 Module re-exports
    ├── image.rs               PixelSelectionRng (XorShift64), encoding, format detection
    ├── iscc.rs                ContentIdentifiers (ISCC-like perceptual hashing)
    └── seed.rs                generate_random_seed() via getrandom (OS CSPRNG)
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
| **Metadata Trap** | [protected-metadata-trap.md](protected-metadata-trap.md) | Canonical `plus:DataMining` XMP injection, seed embedding, legacy IPTC DMI parsing; notice_verification.rs handles legal-notice verification and evidence strength rating |
| **Steganography** | [protected-steganography.md](protected-steganography.md) | LSB + DCT F5, payload generation/verification |
| **JPEG Transcoder** | [jpeg-transcoder.md](jpeg-transcoder.md) | DCT decode/encode, assemble, scan data utilities |
| **JPEG Header** | [jpeg-header.md](jpeg-header.md) | Marker parsing (DQT/SOF/DHT/SOS), component extraction |
| **JPEG Entropy** | [jpeg-entropy.md](jpeg-entropy.md) | Huffman encoding/decoding, CoefficientEncoder/Decoder |
| **F5 DCT Stego** | [jpeg-stego-f5.md](jpeg-stego-f5.md) | F5-style embedding, no-zero variant, quantization table seed |
| **Constants** | [constants.md](constants.md) | All tuning constants (STEGO_*, XORSHIFT_*, SPLITMIX64_*) |
| **CLI** | [cli.md](cli.md) | Command-line interface, batch processing, filename collision handling |
| **Payload v3** | [payload-v3.md](payload-v3.md) | TLV extension format, domain-separated authentication |
| **Provenance** | [provenance.md](provenance.md) | Canonical provenance claims, digest binding, serialization |
| **Signing** | [signing.md](signing.md) | Ed25519 signing and verification (feature-gated: signatures) |
| **Detached Manifests** | [detached.md](detached.md) | Signed sidecar manifests for out-of-band provenance (feature-gated: detached-manifest) |
| **Verification** | [verification.md](verification.md) | Structured verification report with per-channel sub-results |
| **Conformance** | [conformance.md](conformance.md) | Machine-readable conformance reporting, external tool integration |
| **Resource Limits** | [resource-limits.md](resource-limits.md) | Parser hardening, DoS prevention, configurable limits |
| **Request Resolution** | [resolve.md](resolve.md) | ProtectionRequest → ResolvedProtectionPlan validation and resolution |

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

- V3 is the current default: 32-byte core + TLV extensions (CRC: 36 bytes, HMAC: 48 bytes)
- V2 (legacy, extraction only): 32-byte header, ECC-encoded: 100 bytes
- V1 (legacy, extraction only): 24-byte header, ECC-encoded: 76 bytes
- `MIN_PAYLOAD_SIZE = 28`: Parsing threshold, not output size

### Payload v3

Payload v3 adds TLV (Type-Length-Value) extensions with domain-separated authentication:

```
v3 Header (24 bytes)
├── Version (1 byte, =3)
├── Flags (1 byte)
├── Seed (8 bytes, little-endian)
├── Intensity (2 bytes)
├── Timestamp (8 bytes)
└── Extension count + reserved (4 bytes)

Extensions (variable, TLV format)
├── Type (1 byte: domain + key)
├── Length (1 byte)
└── Value (N bytes)

Authentication
├── Per-domain MAC keys (domain separation)
└── Truncated HMAC-SHA256
```

Domain separation ensures cross-domain forgery is infeasible — each extension domain uses a distinct MAC key derived from the master key.

### Provenance Claims (Release 5)

`ProvenanceClaim` provides a canonical provenance assertion:

```rust
let claim = ProvenanceClaim::builder()
    .with_creator("Jane Artist")
    .with_copyright("© 2025 Jane Artist")
    .with_source_hash(image_bytes)
    .with_timestamp(Utc::now())
    .build();
```

Claims are serialized using canonical JSON (sorted keys, no whitespace) for deterministic signing. The claim digest is bound to the image content via SHA-256.

### Detached Manifests (Release 5)

`DetachedManifest` is a signed sidecar for distributing provenance outside the image file:

```
{
  "stegoeggo_manifest_version": 1,
  "provenance_claim": { ... },
  "signature": "base64-encoded Ed25519 signature",
  "verifying_key": "base64-encoded public key"
}
```

Manifests are independent of image format — they can be distributed alongside images via sidecar files, API responses, or database records. Verification checks the signature against the embedded claim and image digest.

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
| `ed25519-dalek` | 3 | Ed25519 signing (signatures feature) |
| `tokio` | 1.0 (opt) | Async runtime for WAF/CDN integration |
| `clap` | 4 (CLI) | Command-line argument parsing |
| `crc32fast` | 1.4 | CRC32 for PNG chunk checksums |
| `thiserror` | 1.0 | Error enum derive |

## Security Notes

- **Default seed is CSPRNG-backed**: `ProtectionContext::default()` calls `generate_random_seed()` which uses `getrandom` (OS CSPRNG). Use `ProtectionContext::new(intensity, seed)` when you need reproducible results across runs.
- **Without MAC key**: Stego verification uses a non-cryptographic CRC32 checksum, not HMAC. Payloads are forgeable.
- **Primary deterrence is metadata**: Visible XMP/EXIF markers (including canonical `plus:DataMining` rights signals) remain even if stego payload is stripped. Metadata provides legal evidence of intent.
- **JPEG stego limitations**: F5 DCT embedding may not survive re-compression. Quantization-table seed embedding is only reliable when the tables themselves are preserved.

## Verification Priority (JPEG)

When extracting from JPEG, check in order:
1. Metadata seed (tEXt/XMP markers) — most robust
2. Quantization table seed — detection only when the tables are preserved
3. DCT coefficient extraction — fragile
4. Pixel-based LSB — not applicable to JPEG
