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
- `pub(crate)` for internal modules (protected, util, webp_container, xmp)
- `LazyLock` for static singletons (e.g., `DEFAULT_PIPELINE`)
- `Arc<ProtectionConfig>` for shared heavy config fields
- Private fields with getter methods on public types
- `#![forbid(unsafe_code)]` throughout the library crate

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
    ProhibitedExceptSearchEngineIndexing,
    ProhibitedAllDataMining,
    ProhibitedSeeConstraints,
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

## Function Signatures

### Public entry points (in `src/lib.rs`)
```rust
fn process_request_bytes(img_bytes: &[u8], request: &ProtectionRequest) -> Result<Vec<u8>>
fn process_request_bytes_with_warnings(img_bytes: &[u8], request: &ProtectionRequest) -> Result<(Vec<u8>, Vec<ProtectionWarning>)>
fn process_request_bytes_with_report(img_bytes: &[u8], request: &ProtectionRequest) -> Result<(Vec<u8>, ExecutionReport)>
fn process_image_bytes(img_bytes: &[u8], level: ProtectionLevel, ctx: &ProtectionContext) -> Result<Vec<u8>>
fn verify_image_bytes(img_bytes: &[u8], mac_key: &[u8]) -> Result<VerificationStatus>
```

### Legacy compatibility (deprecated but functional)
```rust
fn process_image(img: DynamicImage, level: ProtectionLevel, ctx: &ProtectionContext) -> Result<DynamicImage>
fn process_images_parallel(images: &[DynamicImage], level: ProtectionLevel, ctx: &ProtectionContext) -> Result<Vec<DynamicImage>>  // feature: parallel
```

### Steganography methods (on SteganographyProtector)
```rust
// All are &self methods, NOT free functions
fn extract_payload(&self, img: &DynamicImage) -> Option<StegoPayload>
fn verify_payload(&self, img: &DynamicImage) -> bool
fn verify_payload_with_key(&self, img: &DynamicImage, mac_key: &[u8]) -> Option<bool>
fn verify_payload_from_bytes(&self, img_bytes: &[u8], seed: u64) -> bool
fn verify_payload_from_bytes_with_key(&self, img_bytes: &[u8], mac_key: &[u8]) -> VerificationStatus
```

## Constants
- `MIN_PAYLOAD_SIZE = 28` (24-byte header + 4-byte CRC32; parsing threshold, not output size)
- `V3_PAYLOAD_VERSION = 3` (V3 is the current default; V1/V2 still supported for extraction only)
- `STEGO_SPREAD_FACTOR = 5` (adjacent pixels per LSB bit)
- `DEFAULT_TILE_SIZE = 64` (crop-resistant tile size default)
- `MIN_TILE_SIZE = 32` (minimum tile size for crop resistance)
- `estimated_latency_ms()` returns `u32` (not `f64`)

## Payload Sizes

| Mode | Size | Notes |
|------|------|-------|
| V3 CRC (no MAC) | 36 bytes | 32-byte core + 4-byte CRC32 |
| V3 HMAC | 48 bytes | 32-byte core + 16-byte HMAC-SHA256 |
| V2 ECC (legacy) | 100 bytes | 32-byte header × 3 replication + 4 CRC32 |
| V1 (legacy) | 76 bytes | 24-byte header × 3 + 4 CRC32 |

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
11. **Generic carrier operation styles** — The `stegoeggo::stego` facade exposes three styles on the same corrected carrier model: raw (`lsb::embed`/`extract`, `jpeg::embed`/`extract`, caller knows payload length and JPEG `actual_redundancy`); in-place (`lsb::embed_in_place` mutates the caller's `RgbaImage` and shares the corrected V2 mutation core with the cloning `lsb::embed`); and framed (`lsb::embed_framed`/`extract_framed`, `jpeg::embed_framed`/`extract_framed` over `frame::{encode, decode_prefix, decode}`). Framed extraction keeps the seed/config explicit, validates capacity before full extraction, and treats CRC32 as corruption detection rather than authentication.
12. **Plan 073 fallible config** — `LsbConfig::try_new`, `LsbConfig::try_with_redundancy`, `JpegConfig::try_new`, and `JpegConfig::try_with_redundancy` all return `StegoError::InvalidConfig` for out-of-range redundancy. The panicking `with_redundancy` builder is retained for compile-time-constant values; use the fallible path whenever the value comes from runtime configuration.
13. **Decomposed application stego adapter** — `src/protected/steganography/` is split into five responsibility modules behind `SteganographyProtector`: `marker.rs` (V3 payload construction), `embed.rs` (carrier dispatch: LSB, tiled LSB, JPEG DCT/F5, seed-only), `extract.rs` (seed discovery and bounded search), `verify.rs` (integrity and authentication classification), and `legacy.rs` (V1/V2 compatibility). `mod.rs` is a thin facade + shared types + tests; no carrier algorithm is reimplemented there.

## Build & Test
```bash
./scripts/check.sh                      # Fast local check (fmt, clippy, no-default-features, tests)
cargo test --workspace --exclude stegoeggo-fuzz --all-features  # All tests
cargo clippy --workspace --all-targets --all-features -- -D warnings  # Lint
cargo fmt --all -- --check              # Format check
```

## Testing Patterns
- Unit tests live in each source file as `#[cfg(test)] mod tests`
- Integration tests in `tests/` directory
- Test with `ProtectionContext::new(intensity, seed)` for deterministic results
- `ProtectionContext::default()` uses CSPRNG-backed seed (via `getrandom`) — safe for production; use `ProtectionContext::new(intensity, seed)` for reproducibility
- Feature-gated tests: `tests/async_integration.rs` requires `async` feature
- Public generic carrier tests belong in `tests/public_stego_api.rs`; framed tests must verify recovery without retaining the original payload length, and JPEG tests must cover auto-downgraded redundancy.
