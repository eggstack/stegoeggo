# Architecture Overview

`stegoeggo` is a Rust library and CLI for protecting images from unauthorized AI model training through rights-reservation metadata and steganographic markers. It applies multiple layers of protection — metadata injection and steganographic embedding — to serve as legal evidence of image ownership.

**Version:** 0.3.3 · **MSRV:** Rust 1.87 · **License:** see root `Cargo.toml`

## What This Document Is

This is the top-level index. It gives you a bird's-eye view of every module, its role in the system, and where to find the deep-dive for each component. All deep-dive docs live in `architecture/`.

## Repository Layout

```
stegoeggo/                          Workspace root (4 crates)
│
├── src/                            Root library crate (stegoeggo)
│   ├── lib.rs                      Pipeline orchestration + public API
│   ├── types.rs                    Core types (~5100 lines)
│   ├── traits.rs                   Protector trait
│   ├── error.rs                    Error enum (19 variants)
│   ├── protected/                  Protection strategies (all implement Protector)
│   ├── payload_v3/                 V3 payload wire format
│   ├── provenance/                 Provenance claim model
│   ├── signing/                    Ed25519 signing (feature: signatures)
│   ├── detached/                   Detached manifests (feature: detached-manifest)
│   ├── verification/               Structured verification reports
│   ├── util/                       Internal utilities (image, seed, ISCC)
│   ├── bin/                        Conformance harness binary
│   ├── xmp.rs                      XMP namespace-aware filtering
│   ├── webp_container.rs           WebP RIFF container parsing
│   ├── resource_limits.rs          Parser hardening / DoS prevention
│   ├── conformance.rs              Conformance types (feature: conformance)
│   └── async_api.rs                Async wrappers (feature: async)
│
├── stegoeggo-stego/                Generic carrier crate (LSB, JPEG DCT)
│   ├── lsb.rs                      Pixel-domain LSB carrier (public)
│   ├── jpeg.rs                     JPEG DCT carrier (public)
│   ├── frame.rs                    Self-describing framed payload (public)
│   ├── application_support.rs      Parent-crate bridge (feature-gated, hidden)
│   ├── lsb_internal.rs             Permutation / slot mapping (private)
│   └── jpeg_transcoder/            JPEG DCT internals (private)
│
├── stegoeggo-cli/                  CLI binary (stegoeggo)
│   └── main.rs                     Clap-based CLI, ~2490 lines
│
├── fuzz/                           12 fuzz targets (libfuzzer-sys)
├── tests/                          30 integration test files
├── examples/                       4 usage examples
├── benches/                        Criterion benchmarks
├── scripts/                        7 validation scripts
├── architecture/                   30 deep-dive docs (this directory)
└── .github/workflows/              CI (3 workflows)
```

**Crate dependency direction:** `cli → root → carrier`. The carrier crate knows nothing about rights-protection. The root crate re-exports the carrier's public API through `stegoeggo::stego`.

## Module Map at a Glance

Every component below links to a dedicated deep-dive in `architecture/`. Use this as your starting point.

```
                        ┌─────────────────────┐
                        │   stegoeggo-cli     │  CLI binary: 30+ flags,
                        │   main.rs           │  3 subcommands, 5 exit codes
                        └─────────┬───────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        PUBLIC API LAYER                                  │
│  process_image() | process_image_bytes() | process_request_bytes()      │
│  process_images_parallel() | process_images_bytes_parallel()            │
│  verify_image_bytes() | verify_legal_notice()                           │
│  stego::embed/extract/inspect (generic carrier re-exports)              │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│              ProtectionRequest → ResolvedProtectionPlan                  │
│                    src/lib.rs (orchestration)                            │
│                                                                         │
│  resolve_request() validates input, builds immutable plan.              │
│  Legacy ProtectionContext/level APIs are adapters into this path.       │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
             ┌───────────────────┼───────────────────┐
             │                   │                   │
             ▼                   ▼                   ▼
  ┌─────────────────┐ ┌───────────────────┐ ┌─────────────────────┐
  │  Passthrough     │ │ MetadataTrap      │ │  Carrier JPEG API   │
  │  (Disabled)      │ │ (Light)           │ │  (jpeg_transcoder/) │
  │  No-op           │ │ Metadata inject   │ │  DCT coefficients   │
  └─────────────────┘ └───────────────────┘ │  directly (fast)    │
             │                   │           └─────────────────────┘
             │                   │                   │
             ▼                   ▼                   ▼
  ┌─────────────────┐ ┌───────────────────┐ ┌─────────────────────┐
  │ Steganography    │ │ MetadataTrap      │ │  Private DCT core   │
  │ (LSB / DCT F5)  │ │ (marker inject)   │ │  (stego_f5, huff)   │
  └─────────────────┘ └───────────────────┘ └─────────────────────┘
                                 │
                                 ▼
                    ┌──────────────────────┐
                    │  Types & Traits      │
                    │  types.rs, traits.rs │
                    └──────────────────────┘
```

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `async` | Tokio-based async API wrappers | No |
| `signatures` | Ed25519 signing via `ed25519-dalek` | No |
| `detached-manifest` | Detached signed manifest sidecar | No |
| `iscc` | ISCC content identifier computation | No |
| `conformance` | Conformance harness binary + manifest parsing (TOML) | No |
| `parallel` | Rayon-based parallel batch processing | No |
| `test-seeds` | Fallback seed guessing (test infra only, never production) | No |
| `fuzz` | Bounded JPEG dimension inspection for fuzz harnesses | No |

Carrier crate: `application-support` exposes internal bridge for parent crate (`#[doc(hidden)]`).

## Protection Levels

| Level | Perturbation | Stego | Metadata | Use Case |
|-------|--------------|-------|----------|----------|
| `Disabled` | None | None | None | Testing, passthrough |
| `Light` | Minimal seed marker | Q-table seed (JPEG) or LSB redundancy=1 (PNG/WebP) | Seed + DMI | Minimal overhead |
| `Standard` | Noise | LSB/DCT | Seed + DMI | General protection |

Each level above `Disabled` activates metadata injection. `Light` adds the cheapest recoverable seed marker for the output format. `Standard` applies the full LSB or DCT payload.

## Protection Strategies — Three Layers

1. **Metadata injection** (`MetadataTrapProtector`): Embeds visible and machine-readable rights signals — `plus:DataMining` XMP, IPTC DMI, copyright notices, usage terms — into image containers (PNG tEXt/iTXt, JPEG COM/APP13, WebP XMP). These survive stripping and provide legal evidence.

2. **Steganographic embedding** (`SteganographyProtector`): Hides a verifiable payload using LSB (PNG/WebP) or F5-style DCT (JPEG). Payloads carry seed, rights policy, optional HMAC authentication, and provenance digest. Supports tiled crop-resistant mode.

3. **Detached manifests** (feature: `detached-manifest`): Signed JSON sidecar containing image digest, provenance claim, and Ed25519 signatures. Enables out-of-band verification.

## Request-Based API Flow (Release 4+)

Release 4 introduced a policy-first architecture where `ProtectionRequest` is the canonical entry point:

```
ProtectionRequest (user constructs)
        │
        ▼
resolve_request()  ──► validates input, resolves channels/policy
        │
        ▼
ResolvedProtectionPlan (immutable)
        │
        ├── process_request_bytes()                    → Vec<u8>
        ├── process_request_bytes_with_warnings()      → (Vec<u8>, Vec<ProtectionWarning>)
        └── process_request_bytes_with_report()        → (Vec<u8>, ExecutionReport)
```

## Data Flow

### Canonical byte path

```
Input bytes
       │
       ▼
process_request_bytes() ──► resolve_request() → ResolvedProtectionPlan
       │
       ▼
process_plan_bytes()
       │
       ├── HiddenMarkerMode::Disabled ─────────────► execute_metadata_only()
       │                                              └── RightsMetadataProtector::inject_bytes_from_plan()
       │
       ├── HiddenMarkerMode::SeedOnly ─────────────► execute_seed_only_and_metadata()
       │                                              ├── JPEG: embed seed in Q-tables → inject metadata
       │                                              └── PNG/WebP: LSB seed fallback → encode → inject
       │
       ├── HiddenMarkerMode::BestEffort ──────────► execute_stego_and_metadata()
       │                                              ├── JPEG→JPEG fast path (byte-only):
       │                                              │     DCT inspect → apply_dct_stego → inject metadata
       │                                              └── Non-JPEG path:
       │                                                    LSB stego → encode → inject metadata
       │
       └── HiddenMarkerMode::Tiled ───────────────► execute_stego_and_metadata_tiled()
                                                      └── Same as BestEffort with tile_size parameter
```

Legacy `process_image_bytes()` adapters translate `ProtectionLevel` + `ProtectionContext` to a `ProtectionRequest` via `request_from_legacy()` and re-enter the canonical path.

### JPEG fast path (input=JPEG, output=JPEG)

Skips pixel decode/encode entirely. The carrier privately operates on DCT coefficients:
1. Parse supported header and decode Huffman data
2. F5 implementation modifies DCT coefficients
3. Seed stored in quantization table LSBs
4. Re-encode Huffman data, replacing only DQT and SOS scan data; APP/COM/unknown segments survive verbatim

Progressive JPEGs fall back to seed-in-Q-tables only (coefficient manipulation unsupported).

## Component Index — Deep Dives

### Core Pipeline & API

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Pipeline** | [pipeline.md](pipeline.md) | `ProtectionPipeline` orchestration, format routing, JPEG fast path, LazyLock singletons, parallel threshold scaling |
| **Request Resolution** | [resolve.md](resolve.md) | `resolve_request()` single validation point, immutable plan construction |
| **Types** | [types.md](types.md) | `ProtectionLevel`, `ProtectionContext`, `RightsPolicy`, `ProtectionRequest`, `ProtectionPreset`, `ProtectionChannels`, `ExecutionReport`, v0.3→v0.4 migration |
| **Traits** | [traits.md](traits.md) | `Protector` trait contract, `apply`/`apply_bytes` methods, implementation table |
| **Error Types** | [error.md](error.md) | `Error` enum variants, structured resource-limit errors, async `Task` variant |
| **Constants** | [constants.md](constants.md) | All tuning constants: `STEGO_SPREAD_FACTOR`, `XORSHIFT_SEED_OFFSET`, `SPLITMIX64_SEED`, tile defaults, payload version |

### Protection Strategies

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Metadata Trap** | [protected-metadata-trap.md](protected-metadata-trap.md) | Canonical `plus:DataMining` XMP injection, seed embedding, legal metadata fields, format-specific injection (PNG tEXt, JPEG COM/APP, WebP XMP), metadata merge policies, idempotency |
| **Steganography** | [protected-steganography.md](protected-steganography.md) | Decomposed application adapter (`marker.rs`/`embed.rs`/`extract.rs`/`verify.rs`/`legacy.rs`) wrapping LSB (PNG/WebP) + DCT F5 (JPEG); v3/v2/v1 payload generation/verification, majority voting, redundancy, tiled crop-resistant embedding. Also documents the public generic carrier API (`stegoeggo::stego`): raw, in-place, and framed operation styles for arbitrary payload bytes |
| **Passthrough** | [protected-passthrough.md](protected-passthrough.md) | No-op for Disabled level, zero-allocation `Cow::Borrowed` return |

### JPEG DCT Subsystem

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **JPEG Header** | [jpeg-header.md](jpeg-header.md) | `JpegHeader` parser: DQT/SOF/DHT/SOS markers, component extraction, checked scan structure analysis |
| **JPEG Entropy** | [jpeg-entropy.md](jpeg-entropy.md) | Huffman codec: `CoefficientDecoder`/`CoefficientEncoder`, `BitReader`/`BitWriter`, zigzag order, standard Huffman tables |
| **F5 DCT Stego** | [jpeg-stego-f5.md](jpeg-stego-f5.md) | F5-style embedding, no-zero variant, seed in Q-table LSBs, `F5XorShiftRng`, tiled F5 |
| **JPEG Transcoder** | [jpeg-transcoder.md](jpeg-transcoder.md) | `JpegTranscoder` decode/encode flow, `DctSupport` probe, canonical Huffman construction, malformed entropy handling |

### Payload & Encoding

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Payload v3** | [payload-v3.md](payload-v3.md) | TLV wire format, domain-separated authentication, ECC encoding, backward compatibility, parsing algorithm, security model |
| **Provenance** | [provenance.md](provenance.md) | `ProvenanceClaim` builder, canonical JSON serialization, `TypedDigest`, usage in v3 payloads and detached manifests |
| **Provenance Claim Spec** | [provenance-claim.md](provenance-claim.md) | 15-field schema, rights policy discriminants, binary encoding, test vectors |

### Authentication & Signing

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Signing** | [signing.md](signing.md) | Ed25519 signing (feature-gated `signatures`), `SigningKey`/`VerifyingKey` with zeroize, capacity check |
| **Detached Manifests** | [detached.md](detached.md) | Signed sidecar manifests (feature-gated `detached-manifest`), `TrustPolicy`, verification flow |
| **Detached Manifest Spec** | [detached-manifest.md](detached-manifest.md) | Full JSON schema, signing protocol, size bounds, error handling |
| **Verification** | [verification.md](verification.md) | Structured `VerificationReport`, per-channel sub-results, `EvidenceStrength` computation, builder API |

### Utilities & Integration

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Image Utilities** | [util-image.md](util-image.md) | `PixelSelectionRng` (XorShift64 PRNG), encoding, format detection, image hashing |
| **ISCC Identifiers** | [util-iscc.md](util-iscc.md) | Non-standard ISCC-like perceptual hashing, `ContentIdentifiers` |
| **Seed Generation** | [util-seed.md](util-seed.md) | CSPRNG via `getrandom`, time-based splitmix64 fallback |
| **Async API** | [async-api.md](async-api.md) | Tokio `spawn_blocking` wrappers for WAF/CDN integration |
| **Resource Limits** | [resource-limits.md](resource-limits.md) | Parser hardening, DoS prevention, configurable limits with structured errors |
| **Legal Metadata Mapping** | [legal-metadata-field-mapping.md](legal-metadata-field-mapping.md) | Field mapping across PNG/JPEG/WebP, round-trip issues |

### Testing & Operations

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **Conformance** | [conformance.md](conformance.md) | External tool integration (ExifTool, xmllint), fixture manifest, strict mode, exit codes |
| **CLI** | [cli.md](cli.md) | Command-line interface, all flags, batch processing, verification mode, subcommands |

### Design Records

| Component | Deep Dive | What It Covers |
|-----------|-----------|----------------|
| **C2PA ADR** | [adr-c2pa.md](adr-c2pa.md) | Architecture Decision Record: deferred C2PA integration |

## Root Crate — Module Map

```
src/
├── lib.rs                     Pipeline orchestration, public API, LazyLock singletons
├── types.rs                   ProtectionLevel, ProtectionContext, ProtectionRequest,
│                              RightsPolicy, LegalMetadata, ExecutionReport, etc.
├── traits.rs                  Protector trait (apply/apply_bytes)
├── error.rs                   Error enum (19 variants), Result type
├── async_api.rs               Tokio spawn_blocking wrappers (feature: async)
├── conformance.rs             Conformance reporting types (feature: conformance)
├── resource_limits.rs         ResourceLimits for parser hardening (DoS prevention)
├── xmp.rs                     XMP parsing, namespace-aware filtering (pub(crate))
├── webp_container.rs          WebP RIFF parsing, VP8X/VP8/VP8L/ANMF (pub(crate))
│
├── protected/                 Protection strategies (all implement Protector)
│   ├── passthrough.rs         No-op for Disabled level
│   ├── metadata_trap.rs       Metadata injection (tEXt/COM/XMP, plus:DataMining)
│   ├── steganography/         Rights-aware hidden-marker application adapter
│   │   ├── mod.rs             Facade, shared contracts, public entry points
│   │   ├── marker.rs          Current V3 marker construction
│   │   ├── embed.rs           Carrier selection and embedding dispatch
│   │   ├── extract.rs         Seed discovery and bounded extraction search
│   │   ├── verify.rs          Payload integrity/authentication classification
│   │   └── legacy.rs          V1/V2 and compatibility-only decoding
│   ├── ecc.rs                 3× repetition ECC with majority voting
│   ├── notice_verification.rs Legal notice extraction, evidence strength rating
│   ├── resolve.rs             ProtectionRequest → ResolvedProtectionPlan
│   ├── constants.rs           Tuning constants (STEGO_*, XORSHIFT_*, SPLITMIX64_*)
│   └── stego_cost.rs          Pixel embedding cost (feature: fuzz)
│
├── payload_v3/                Payload v3 wire format
│   ├── header.rs              PayloadV3Header parsing/serialization
│   ├── parser.rs              Multi-version parser (v1/v2/v3)
│   ├── writer.rs              PayloadBuilder for v3 payloads
│   ├── types.rs               AuthAlgorithm, PayloadFlags, constants
│   └── errors.rs              PayloadV3ParseError
│
├── provenance/                Provenance claim model
│   ├── claim.rs               ProvenanceClaim builder, canonical serialization
│   ├── digest.rs              TypedDigest (SHA-256)
│   └── canonical.rs           Canonical JSON helpers
│
├── signing/                   Ed25519 signing (feature: signatures)
│   ├── ed25519_impl.rs        SigningKey, VerifyingKey, SignatureResult
│   └── config.rs              SigningConfig, SignaturePlacement
│
├── detached/                  Detached signed manifests (feature: detached-manifest)
│   ├── manifest.rs            DetachedManifest, SignatureRecord, TrustMetadata
│   ├── generate.rs            create_manifest_from_image()
│   └── verify.rs              verify_detached_manifest*(), TrustPolicy
│
├── verification/              Structured verification report
│   ├── report.rs              VerificationReport, sub-verification types
│   └── builder.rs             VerificationReportBuilder (fluent API)
│
├── bin/
│   └── stegoeggo-conformance.rs  Conformance harness binary
│
└── util/
    ├── image.rs               PixelSelectionRng, encoding, format detection
    ├── iscc.rs                ContentIdentifiers (feature: iscc)
    └── seed.rs                generate_random_seed() via getrandom (CSPRNG)
```

## Carrier Crate — Module Map

```
stegoeggo-stego/src/
├── lib.rs                     Public API surface, carrier reports
├── constants.rs               Carrier-level tuning constants
├── error.rs                   StegoError, JpegUnsupportedReason
├── types.rs                   EmbedOutcome, EmbedPath, EmbedStatus, in-place report
├── frame.rs                   Self-describing frame (magic, version, CRC32)
├── lsb.rs                     LSB carrier (raw, in-place, and framed operations)
├── lsb_internal.rs            Permutations, V2 carrier, slot mapping (private)
├── jpeg.rs                    JPEG carrier: raw/framed DCT operations, seed hint
├── application_support.rs     Parent-crate operation layer (feature: application-support)
└── jpeg_transcoder/           JPEG DCT internals (private)
    ├── header.rs              JpegHeader, HuffmanTable parsing
    ├── entropy.rs             CoefficientDecoder/Encoder (Huffman codec)
    └── stego_f5.rs            DctStegoF5, F5XorShiftRng
```

`jpeg_transcoder/` and `lsb_internal.rs` are private implementation modules. The root crate uses the narrow `application-support` feature internally and adopts the in-place operation when it already owns a mutable decoded RGBA image.

## Fuzz Targets

12 targets in `fuzz/fuzz_targets/`, built on `libfuzzer-sys`. Run with: `cargo +nightly fuzz run <target> -- -max_total_time=60`.

| Target | What It Fuzzes |
|--------|----------------|
| `pipeline_bytes` | Full protection pipeline on arbitrary bytes |
| `tiled_round_trip` | Tiled stego embed/extract round-trip |
| `jpeg_parser` | JPEG header/structure parsing |
| `payload_v3_parser` | V3 payload wire format parsing |
| `png_metadata` | PNG metadata injection/extraction |
| `webp_riff_parser` | WebP RIFF container parsing |
| `xmp_extract` | XMP packet field filtering |
| `metadata_merge` | XMP metadata merging |
| `detached_manifest_parse` | Detached manifest JSON parsing |
| `detached_manifest_verify` | Detached manifest verification |
| `provenance_canonicalize` | Provenance claim canonical JSON |
| `verification_report` | Verification report building |

## Integration Test Coverage

30 test files in `tests/`:

| File | Coverage Area |
|------|---------------|
| `basic.rs` | Basic protection pipeline |
| `integration.rs` | Full integration tests |
| `request_api.rs` | ProtectionRequest canonical API |
| `semantic_correctness.rs` | Semantic correctness of protected output |
| `preservation.rs` | Metadata preservation across formats |
| `preservation_idempotence.rs` | Idempotent re-processing |
| `cross_format_semantics.rs` | Cross-format (PNG/JPEG/WebP) semantic equivalence |
| `cross_format_closure.rs` | Cross-format output closure |
| `canonical_rights.rs` | Canonical rights notice handling |
| `merge_policy.rs` | MetadataUpdatePolicy behavior |
| `jpeg_container_preservation.rs` | JPEG segment preservation |
| `public_stego_api.rs` | Public generic stego API |
| `payload_v3_roundtrip.rs` | V3 payload round-trip |
| `independent_v3_parser.rs` | Independent V3 parser validation |
| `known_answer_vectors.rs` | Known-answer test vectors |
| `verification_report_tests.rs` | VerificationReport builder |
| `signing_tests.rs` | Ed25519 signing (feature: signatures) |
| `detached_manifest_tests.rs` | Detached manifest (feature: detached-manifest) |
| `provenance_claim_tests.rs` | Provenance claim canonical form |
| `conformance_parser_tests.rs` | Conformance manifest parsing |
| `conformance_negative.rs` | Conformance negative test cases |
| `conformance_container_tests.rs` | Conformance container validation |
| `conformance_harness_tests.rs` | Conformance harness (feature: conformance) |
| `external_tools.rs` | External tool integration (`#[ignore]`) |
| `generate_conformance_fixtures.rs` | Fixture generation |
| `robustness.rs` | Fuzz regression tests |
| `soak_tests.rs` | Long-running soak tests |
| `async_integration.rs` | Async API (feature: async) |
| `plan026_gate1_2_3_tests.rs` | Plan-gate regression tests |
| `plan065_legacy_compat.rs` | Legacy API compatibility |

## Validation Scripts

| Script | Purpose |
|--------|---------|
| `scripts/check.sh` | Fast CI checks: fmt, clippy, no-default-features, workspace tests |
| `scripts/release-check.sh` | Pre-release validation (staged: pre, root, cli) |
| `scripts/verify_metadata_conformance.sh` | External tool conformance (exiftool, xmllint) |
| `scripts/validate-docs-rs.sh` | Docs.rs-equivalent rustdoc validation (nightly) |
| `scripts/validate-msrv-package.sh` | Fresh MSRV consumer resolution (Rust 1.87) |
| `scripts/check_fuzz_sync.sh` | Verify fuzz target parity with CI workflow |
| `scripts/measure_binary_size.sh` | Binary size measurement |

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
    fn modifies_pixels(&self) -> bool { true }
    fn requires_bytes_level(&self) -> bool { false }
}
```

The canonical executor resolves a `ProtectionRequest` into a plan and runs the selected operations directly. The legacy `ProtectionPipeline` is a stateless compatibility adapter that preserves the older level-based entry points while delegating to that same request path.

### Cow Returns

`Protector::apply` returns `Cow<'a, DynamicImage>` to avoid unnecessary cloning. When `intensity == 0.0`, protectors return `Cow::Borrowed(img)` without any allocation.

### JPEG Fast Path

When **both** input and output are JPEG, the application adapter calls the carrier's encoded-byte JPEG operation. The carrier privately operates directly on DCT coefficients, avoiding pixel decode/encode cycles that would introduce additional lossy compression artifacts. Format conversion (JPEG → PNG) always takes the full pixel pipeline.

### Two XorShiftRng Implementations

- **`PixelSelectionRng`** in `util/image.rs` — general-purpose pixel selection for steganography
- **`DctCoefficientRng`** in `stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs` — DCT coefficient shuffling

They use different algorithms and produce different sequences for the same seed. **Do NOT interchange them.**

### Private Fields with Getters

`ProtectionContext`, `StegoPayload`, and `LegalMetadata` all use private fields with getter methods. This enforces consistent access patterns and allows internal refactoring without breaking consumers.

### Arc for Shared Config

`ProtectionConfig` (MAC key + legal metadata) is wrapped in `Arc<ProtectionConfig>` and stored in `ProtectionContext`. This allows cheap cloning of context while sharing the heavy config.

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
| `stegoeggo-stego` | 0.3 | Generic carrier core (LSB, JPEG DCT, transcoder) | — |
| `image` | 0.25 | Image loading, decoding, encoding (PNG, JPEG, WebP) | — |
| `jpeg-encoder` | 0.7 | Direct JPEG encoding with quality/progressive control | — |
| `quick-xml` | 0.41 | XMP parsing and serialization | — |
| `rayon` | 1.10 | Parallel image processing | `parallel` |
| `sha2` | 0.10 | HMAC-SHA256 for keyed payload verification | — |
| `hmac` | 0.12 | HMAC construction | — |
| `serde` / `serde_json` | 1.0 | Serialization | — |
| `subtle` | 2 | Constant-time HMAC comparison | — |
| `ed25519-dalek` | 3 | Ed25519 signing | `signatures` |
| `tokio` | 1.0 | Async runtime for WAF/CDN integration | `async` |
| `clap` | 4 | Command-line argument parsing | CLI-only |
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


