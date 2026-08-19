---
name: stegoeggo-conventions
description: Use when writing, modifying, or reviewing Rust code in the stegoeggo codebase. Triggers on tasks like "write tests", "add feature", "fix bug", "refactor", or any code change in src/. Covers code style, patterns, and pitfalls specific to this project.
---

# Stegoeggo Code Conventions

## Formatting
- Rustfmt: 4-space indentation, max width 100
- Run `./scripts/check.sh` before committing (runs fmt, clippy, no-default-features check, tests)

## Code Style
- No comments in code unless explicitly asked by user
- `#[must_use]` on all builder methods
- `pub(crate)` for internal modules (`protected`, `util`, `webp_container`, `xmp`)
- `LazyLock` for static singletons (e.g., `DEFAULT_PIPELINE`)
- `Arc<ProtectionConfig>` for shared heavy config fields
- Private fields with getter methods on public types
- `#![forbid(unsafe_code)]` throughout the library crate and `stegoeggo-stego`

## Canonical API (Release 4+)

The canonical API is `ProtectionRequest` + `RightsPolicy`. The legacy `ProtectionContext`/`ProtectionLevel` APIs are compatibility adapters that translate to the canonical path internally.

### ProtectionRequest

```rust
// Metadata-only (fastest path)
let request = ProtectionRequest::metadata_only(
    RightsNotice::default(),
    RightsPolicy::ProhibitedAiMlTraining,
);

// With hidden marker
let request = ProtectionRequest::with_hidden_marker(notice, policy);

// From preset
let request = ProtectionRequest::from_preset(
    ProtectionPreset::AuthenticatedProvenance,
    notice, policy,
);
```

### RightsPolicy

Always explicit — never inferred from intensity, format, or channel selection:

```rust
pub enum RightsPolicy {
    Unspecified,
    Allowed,
    ProhibitedAiMlTraining,
    ProhibitedGenerativeAiTraining,
    ProhibitedExceptSearchIndexing,
    ProhibitedAllDataMining,
    ProhibitedSeeConstraints,
}
```

### HiddenMarkerMode

```rust
pub enum HiddenMarkerMode {
    Disabled,
    SeedOnly,
    BestEffort,
    Tiled,
}
```

## Type Patterns

### ProtectionContext (legacy, still functional)
- All fields private — use builder methods or `new(intensity, seed)`
- `inject_metadata: Option<bool>` — `None` means use level default
- `inject_legal_claims: Option<bool>` — `None` means use level default
- `config: Option<Arc<ProtectionConfig>>` — `#[serde(skip)]`

### ProtectionConfig
- `mac_key: Option<Vec<u8>>` (not `Vec<u8>`)

### LegalMetadata
- 16 fields: `copyright_holder`, `contact_email`, `license_url`, `usage_terms`, `usage_terms_lang`, `creation_date`, `ai_constraints`, `web_statement_of_rights`, `creator`, `credit_line`, `copyright_owner`, `licensor_name`, `licensor_email`, `licensor_url`, `metadata_date`, `notice_applied_at`
- `MAX_FIELD_LEN = 8192` bytes per field
- Auto-enables legal claims injection when present (no explicit `with_legal_claims(true)` needed)

### ProcessingOptions
- 8 fields: `output_format`, `jpeg_quality`, `progressive_jpeg`, `max_dimension`, `metadata_update_policy`, `stego_redundancy`, `content_hash`, `timestamp_override`

## Public Module Layout

### Root crate (`src/`)
- **Public**: `conformance` (feature: conformance), `error`, `payload_v3`, `provenance`, `resource_limits`, `traits`, `types`, `verification`
- **Public (feature-gated)**: `stego` (inline re-export of `stegoeggo_stego`), `async_api` (feature: async), `signing` (feature: signatures), `detached` (feature: detached-manifest)
- **`pub(crate)`**: `protected`, `util`, `webp_container`, `xmp`

### Carrier crate (`stegoeggo-stego/src/`)
- **Public**: `constants`, `error`, `frame`, `jpeg`, `lsb`, `types`
- **`pub(crate)`**: `jpeg_transcoder`, `lsb_internal`
- **`pub(crate)` (feature: application-support)**: `application_support` (`#[doc(hidden)]`)

## Function Signatures

### Public entry points (in `src/lib.rs`)
```rust
fn process_request_bytes(img_bytes: &[u8], request: &ProtectionRequest) -> Result<Vec<u8>>
fn process_request_bytes_with_warnings(img_bytes: &[u8], request: &ProtectionRequest) -> Result<(Vec<u8>, Vec<ProtectionWarning>)>
fn process_request_bytes_with_report(img_bytes: &[u8], request: &ProtectionRequest) -> Result<(Vec<u8>, ExecutionReport)>
fn process_image_bytes(img_bytes: &[u8], level: ProtectionLevel, ctx: &ProtectionContext) -> Result<Vec<u8>>
fn verify_image_bytes(img_bytes: &[u8], mac_key: &[u8]) -> VerificationStatus  // NOT Result
fn verify_image_bytes_detailed(img_bytes: &[u8], mac_key: &[u8]) -> VerificationResult
fn verify_legal_notice(img_bytes: &[u8], mac_key: &[u8]) -> NoticeVerification
```

### Legacy compatibility (deprecated but functional)
```rust
fn process_image(img: DynamicImage, level: ProtectionLevel, ctx: &ProtectionContext) -> Result<DynamicImage>
fn process_images_parallel(images: &[DynamicImage], level: ProtectionLevel, ctx: &ProtectionContext) -> Result<Vec<DynamicImage>>  // feature: parallel
```

### Generic carrier API (`stegoeggo::stego`)
```rust
// LSB (pixel-domain)
lsb::capacity(img, payload_len, &config) -> Result<CapacityReport>
lsb::embed(img, payload, &config) -> Result<EmbedReport<RgbaImage>>
lsb::extract(img, payload_len, &config) -> Result<Vec<u8>>
lsb::embed_in_place(img, payload, &config) -> Result<InPlaceEmbedReport>
lsb::embed_framed(img, payload, &config) -> Result<EmbedReport<RgbaImage>>
lsb::extract_framed(img, &config) -> Result<Vec<u8>>

// JPEG (DCT-domain)
jpeg::capacity(jpeg_bytes, payload_len, &config) -> Result<CapacityReport>
jpeg::embed(jpeg_bytes, payload, &config) -> Result<EmbedReport>
jpeg::extract(jpeg_bytes, payload_len, &config, actual_redundancy) -> Result<Vec<u8>>
jpeg::embed_framed(jpeg_bytes, payload, &config) -> Result<EmbedReport>
jpeg::extract_framed(jpeg_bytes, &config) -> Result<Vec<u8>>
jpeg::probe_support(jpeg_bytes) -> Result<JpegSupport>
jpeg::embed_seed_hint(jpeg_bytes, seed) -> Result<Vec<u8>>
jpeg::extract_seed_hint(jpeg_bytes) -> Result<Option<u64>>
jpeg::is_progressive_jpeg(jpeg_bytes) -> bool

// Frame (generic TLV wrapper)
frame::encode(payload) -> Result<Vec<u8>>
frame::decode(data) -> Result<(FrameHeader, Vec<u8>)>
frame::decode_prefix(data) -> Result<(FrameHeader, usize)>
```

## Constants

### Carrier crate (`stegoeggo-stego/src/constants.rs`)
- `STEGO_OFFSET_SEED_1: u64 = 0x517cc1b727220a95`
- `STEGO_SPREAD_FACTOR: usize = 5`
- `SPLITMIX64_SEED: u64 = 0x9e3779b97f4a7c15`
- `MIN_REDUNDANCY: usize = 1`
- `MAX_REDUNDANCY: usize = 10`

### Application crate (`src/protected/constants.rs`)
- `STEGO_OFFSET_SEED_1: u64 = 0x517cc1b727220a95`
- `XORSHIFT_SEED_OFFSET: u64 = 0x123456789ABCDEF0`

### Payload (`src/payload_v3/types.rs`)
- `V3_MAGIC: [u8; 2] = [0x53, 0x45]` ("SE")
- `V3_PAYLOAD_VERSION: u8 = 3`
- `V3_CORE_SIZE: usize = 32`
- `V3_MAX_EMBEDDED_SIZE: usize = 256`
- `V3_MAX_EXTENSION_SIZE: usize = 128`
- `V3_MAX_EXTENSION_COUNT: usize = 32`
- `V3_DOMAIN_STRING: &[u8] = b"StegoEggo-v3"`

### Application (`src/protected/steganography/mod.rs`)
- `MIN_PAYLOAD_SIZE: usize = 28` (parsing threshold, not output size)

### Generic carrier frame (`stegoeggo-stego/src/frame.rs`)
- `FRAME_HEADER_SIZE: usize = 11`
- `MAX_FRAME_PAYLOAD: usize = 16 * 1024 * 1024` (16 MiB)

## Payload Sizes

| Mode | Size | Notes |
|------|------|-------|
| V3 CRC (no MAC) | 36 bytes | 32-byte core + 4-byte CRC32 |
| V3 HMAC | 48 bytes | 32-byte core + 16-byte HMAC-SHA256 |
| V2 ECC (legacy) | 100 bytes | 32-byte header x 3 replication + 4 CRC32 |
| V1 (legacy) | 76 bytes | 24-byte header x 3 + 4 CRC32 |

## Common Pitfalls

1. **Two XorShiftRng implementations** — `PixelSelectionRng` in `util/image.rs` and `DctCoefficientRng` in `stegoeggo-stego/src/jpeg_transcoder/stego_f5.rs` use different algorithms. Never interchange.
2. **Metadata injection survives only in byte paths** — `RightsMetadataProtector::apply()` returns `Cow::Borrowed` unchanged. Use `inject_bytes_from_plan()` or `process_image_bytes()` for metadata.
3. **Stego seed derivation** — embed/extract functions internally derive `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)`. Match seeds when calling directly.
4. **`subtle` crate** — use `ConstantTimeEq::ct_eq()` for HMAC verification, not `==`
5. **F5 seed embedding** — Precondition check fails if any quantization value < 2. Values of 1 cannot represent 0-bits reliably. Use values >= 2.
6. **ISCC is not standard-compliant** — uses custom component codes (`0x12`, `0x33`), not interoperable with other ISCC implementations.
7. **V3 is the current payload format** — V1/V2 are extraction-only legacy. V3 adds TLV extensions with domain-separated authentication.
8. **`verify_payload_from_bytes_with_key` returns `VerificationStatus`** — not `Option<bool>`.
9. **Generic carrier error type** — The public `stego` module uses `StegoError`, not the root crate `Error`. Convert via `From<StegoError> for Error`.
10. **JPEG DCT one-pass embed** — Supported DCT embedding computes max feasible redundancy from capacity, then embeds+encodes once. No retry loop.
11. **Generic carrier operation styles** — The `stegoeggo::stego` facade exposes three styles: raw (`lsb::embed`/`extract`, `jpeg::embed`/`extract`, caller knows payload length and JPEG `actual_redundancy`); in-place (`lsb::embed_in_place` mutates the caller's `RgbaImage` and shares the corrected V2 mutation core with the cloning `lsb::embed`); and framed (`lsb::embed_framed`/`extract_framed`, `jpeg::embed_framed`/`extract_framed` over `frame::{encode, decode_prefix, decode}`). Framed extraction keeps the seed/config explicit, validates capacity before full extraction, and treats CRC32 as corruption detection rather than authentication.
12. **Fallible config constructors** — `LsbConfig::try_new`, `LsbConfig::try_with_redundancy`, `JpegConfig::try_new`, and `JpegConfig::try_with_redundancy` all return `StegoError::InvalidConfig` for out-of-range redundancy. The panicking `with_redundancy` builder is retained for compile-time-constant values; use the fallible path whenever the value comes from runtime configuration.
13. **Decomposed application stego adapter** — `src/protected/steganography/` is split into five responsibility modules behind `SteganographyProtector`: `marker.rs` (V3 payload construction), `embed.rs` (carrier dispatch: LSB, tiled LSB, JPEG DCT/F5, seed-only), `extract.rs` (seed discovery and bounded search), `verify.rs` (integrity and authentication classification), and `legacy.rs` (V1/V2 compatibility). `mod.rs` is a thin facade + shared types + tests; no carrier algorithm is reimplemented there.
14. **JPEG framed extraction is single-decode** — `jpeg::extract_framed` retains private decoded coefficients for its bounded redundancy search. Do not recompose it from public `capacity`/`extract` calls, expose a JPEG session, or reduce the configured search domain.
15. **Benchmark equivalence** — The `lsb_clone_vs_in_place` benchmark uses Criterion batching so each in-place iteration starts from a pristine source image and the preparation clone remains outside the timed operation.
16. **`verify_image_bytes` returns directly** — Returns `VerificationStatus`, not `Result<VerificationStatus>`. Use `verify_image_bytes_detailed` for full `VerificationResult`.

## Build & Test
```bash
./scripts/check.sh                      # Fast local check (fmt, clippy, no-default-features, tests)
cargo test --workspace --exclude stegoeggo-fuzz --all-features  # All tests
cargo clippy --workspace --all-targets --all-features -- -D warnings  # Lint
cargo fmt --all -- --check              # Format check
```

## Testing Patterns
- Unit tests live in each source file as `#[cfg(test)] mod tests`
- Integration tests in `tests/` directory (30 test files)
- Test with `ProtectionContext::new(intensity, seed)` for deterministic results
- `ProtectionContext::default()` uses CSPRNG-backed seed (via `getrandom`) — safe for production; use `ProtectionContext::new(intensity, seed)` for reproducibility
- Feature-gated tests: `tests/async_integration.rs` requires `async` feature
- Public generic carrier tests belong in `tests/public_stego_api.rs`; framed tests must verify recovery without retaining the original payload length, and JPEG tests must cover auto-downgraded redundancy.
