# Architecture Overview

`stegoeggo` is a Rust library and CLI for protecting images from unauthorized AI model training through rights-reservation metadata and steganographic markers. It applies multiple layers of protection — metadata injection and steganographic embedding — to serve as legal evidence of image ownership.

## Getting Oriented

Start with the component you're investigating. This document is the index — each component links to a dedicated deep-dive.

| I want to understand... | Read this |
|-------------------------|-----------|
| How the pipeline routes and orchestrates protection | [pipeline.md](pipeline.md) |
| What `ProtectionRequest`, `ProtectionContext`, and other core types look like | [types.md](types.md) |
| How protectors implement the `Protector` trait | [traits.md](traits.md) |
| Error handling and all failure modes | [error.md](error.md) |
| How the CLI works end-to-end | [cli.md](cli.md) |
| How metadata gets injected into images | [protected-metadata-trap.md](protected-metadata-trap.md) |
| How steganographic payloads are embedded and extracted | [protected-steganography.md](protected-steganography.md) |
| How the JPEG DCT fast path works | [jpeg-transcoder.md](jpeg-transcoder.md) |
| The payload wire format (v3 with TLV extensions) | [payload-v3.md](payload-v3.md) |
| Provenance claims and canonical serialization | [provenance.md](provenance.md) |
| Ed25519 signing for provenance | [signing.md](signing.md) |
| Detached signed manifests | [detached.md](detached.md) |
| Verification reports and evidence strength | [verification.md](verification.md) |
| The conformance testing suite | [conformance.md](conformance.md) |
| Async API for WAF/CDN integration | [async-api.md](async-api.md) |
| Parser hardening and DoS prevention | [resource-limits.md](resource-limits.md) |
| How `ProtectionRequest` resolves into an execution plan | [resolve.md](resolve.md) |
| Why C2PA integration is deferred | [adr-c2pa.md](adr-c2pa.md) |
| The full detached manifest specification | [detached-manifest.md](detached-manifest.md) |
| Legal metadata field mapping across formats | [legal-metadata-field-mapping.md](legal-metadata-field-mapping.md) |
| Provenance claim design specification | [provenance-claim.md](provenance-claim.md) |

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              PUBLIC API                                      │
│  process_image() | process_image_bytes() | process_images_*_parallel()     │
│  process_request_bytes() | verify_image_bytes()                            │
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
│(LSB/DCT)   │ │  Trap    │ │             │ │                 │
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

## Request-Based API Flow (Release 4+)

Release 4 introduced a policy-first architecture where `ProtectionRequest` is the canonical entry point. The flow separates request construction, resolution, and execution:

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
├── conformance.rs             Machine-readable conformance reporting types (feature: conformance)
├── resource_limits.rs         ResourceLimits for parser hardening (DoS prevention)
├── xmp.rs                     XMP parsing, namespace-aware filtering, packet merging (pub(crate))
├── webp_container.rs          WebP RIFF parsing, VP8X/VP8/VP8L/ANMF handling (pub(crate))
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
    ├── iscc.rs                ContentIdentifiers (ISCC-like perceptual hashing, feature: iscc)
    └── seed.rs                generate_random_seed() via getrandom (OS CSPRNG)
```

## Component Index — Deep Dives

### Core Pipeline & API

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Pipeline** | [pipeline.md](pipeline.md) | `ProtectionPipeline` orchestration, format routing, JPEG fast path, LazyLock singletons, parallel threshold scaling |
| **Types** | [types.md](types.md) | `ProtectionLevel`, `ProtectionContext`, `RightsPolicy`, `ProtectionRequest`, `ProtectionPreset`, `ProtectionChannels`, `ExecutionReport`, v0.3→v0.4 migration |
| **Traits** | [traits.md](traits.md) | `Protector` trait contract, `apply`/`apply_bytes` methods, implementation table |
| **Error Types** | [error.md](error.md) | `Error` enum variants, structured resource-limit errors, async `Task` variant |
| **Request Resolution** | [resolve.md](resolve.md) | `resolve_request()` single validation point, immutable plan construction |

### Protection Strategies

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Passthrough** | [protected-passthrough.md](protected-passthrough.md) | No-op for Disabled level, zero-allocation `Cow::Borrowed` return |
| **Metadata Trap** | [protected-metadata-trap.md](protected-metadata-trap.md) | Canonical `plus:DataMining` XMP injection, seed embedding, legal metadata fields, format-specific injection (PNG tEXt, JPEG COM/APP, WebP XMP), metadata merge policies, idempotency |
| **Steganography** | [protected-steganography.md](protected-steganography.md) | LSB (PNG/WebP) + DCT F5 (JPEG), v3/v2/v1 payload generation/verification, majority voting, redundancy, tiled crop-resistant embedding |
| **Constants** | [constants.md](constants.md) | All tuning constants: `STEGO_SPREAD_FACTOR`, `XORSHIFT_SEED_OFFSET`, `SPLITMIX64_SEED`, tile defaults, payload version |

### JPEG DCT Subsystem

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **JPEG Transcoder** | [jpeg-transcoder.md](jpeg-transcoder.md) | `JpegTranscoder` decode/encode flow, `DctSupport` probe, canonical Huffman construction, malformed entropy handling |
| **JPEG Header** | [jpeg-header.md](jpeg-header.md) | `JpegHeader` parser: DQT/SOF/DHT/SOS markers, component extraction, checked scan structure analysis |
| **JPEG Entropy** | [jpeg-entropy.md](jpeg-entropy.md) | Huffman codec: `CoefficientDecoder`/`CoefficientEncoder`, `BitReader`/`BitWriter`, zigzag order, standard Huffman tables |
| **F5 DCT Stego** | [jpeg-stego-f5.md](jpeg-stego-f5.md) | F5-style embedding, no-zero variant, seed in Q-table LSBs, `F5XorShiftRng`, tiled F5 |

### Payload & Encoding

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Payload v3** | [payload-v3.md](payload-v3.md) | TLV wire format, domain-separated authentication, ECC encoding, backward compatibility, parsing algorithm, security model |
| **Provenance** | [provenance.md](provenance.md) | `ProvenanceClaim` builder, canonical JSON serialization, `TypedDigest`, usage in v3 payloads and detached manifests |

### Authentication & Signing

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Signing** | [signing.md](signing.md) | Ed25519 signing (feature-gated `signatures`), `SigningKey`/`VerifyingKey` with zeroize, capacity check |
| **Detached Manifests** | [detached.md](detached.md) | Signed sidecar manifests (feature-gated `detached-manifest`), `TrustPolicy`, verification flow |
| **Verification** | [verification.md](verification.md) | Structured `VerificationReport`, per-channel sub-results, `EvidenceStrength` computation, builder API |

### Utilities & Integration

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Image Utilities** | [util-image.md](util-image.md) | `PixelSelectionRng` (XorShift64 PRNG), encoding, format detection, image hashing |
| **ISCC Identifiers** | [util-iscc.md](util-iscc.md) | Non-standard ISCC-like perceptual hashing, `ContentIdentifiers` |
| **Seed Generation** | [util-seed.md](util-seed.md) | CSPRNG via `getrandom`, time-based splitmix64 fallback |
| **Async API** | [async-api.md](async-api.md) | Tokio `spawn_blocking` wrappers for WAF/CDN integration |
| **Resource Limits** | [resource-limits.md](resource-limits.md) | Parser hardening, DoS prevention, configurable limits with structured errors |
| **Conformance** | [conformance.md](conformance.md) | External tool integration (ExifTool, xmllint), fixture manifest, strict mode, exit codes |
| **CLI** | [cli.md](cli.md) | Command-line interface, all flags, batch processing, verification mode |

### Specifications & Design Records

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Detached Manifest Spec** | [detached-manifest.md](detached-manifest.md) | Full JSON schema, signing protocol, size bounds, error handling |
| **Provenance Claim Spec** | [provenance-claim.md](provenance-claim.md) | 15-field schema, rights policy discriminants, binary encoding, test vectors |
| **Legal Metadata Mapping** | [legal-metadata-field-mapping.md](legal-metadata-field-mapping.md) | Field mapping across PNG/JPEG/WebP, round-trip issues |
| **C2PA ADR** | [adr-c2pa.md](adr-c2pa.md) | Architecture Decision Record: deferred C2PA integration |

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

### Metadata Injection Semantics

Three-state control (`Option<bool>`) for metadata injection:
- `None` (default): use level-based defaults — inject for all levels except `Disabled`
- `Some(true)`: force-enable
- `Some(false)`: force-disable

`None` and `Some(false)` have different semantics for non-Disabled levels — the former injects, the latter suppresses.

### Parallel Threshold Scaling

`parallel_threshold()` returns `cores * 64 * 64` — scales with rayon thread count. At 1 core: 4096 pixels. At 4 cores: 16384 pixels. At 16 cores: 65536 pixels.

## Dependencies

| Crate | Version | Role | Feature |
|-------|---------|------|---------|
| `image` | 0.25 | Image loading, decoding, encoding (PNG, JPEG, WebP) | — |
| `jpeg-encoder` | 0.7 | Direct JPEG encoding with quality/progressive control | — |
| `rayon` | 1.10 | Parallel image processing | `parallel` |
| `sha2` | 0.10 | HMAC-SHA256 for keyed payload verification | — |
| `hmac` | 0.12 | HMAC construction | — |
| `serde` | 1.0 | Serialization of context and variants | — |
| `serde_json` | 1.0 | JSON serialization | — |
| `subtle` | 2 | Constant-time HMAC comparison | — |
| `ed25519-dalek` | 3 | Ed25519 signing | `signatures` |
| `tokio` | 1.0 | Async runtime for WAF/CDN integration | `async` |
| `clap` | 4 (CLI) | Command-line argument parsing | CLI-only |
| `crc32fast` | 1.4 | CRC32 for PNG chunk checksums | — |
| `thiserror` | 1.0 | Error enum derive | — |
| `iscc-lib` | 0.4 | ISCC content identifier computation | `iscc` |
| `toml` | 1.0 | Conformance manifest parsing | `conformance` |

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
