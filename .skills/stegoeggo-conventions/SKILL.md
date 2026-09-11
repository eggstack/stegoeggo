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
- `pub(crate)` for internal modules (`protected`, `util`, `webp_container`, `container_walk`, `xmp`, `pipeline`)
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
- **`pub(crate)`**: `protected`, `util`, `webp_container`, `container_walk`, `xmp`, `pipeline`

### Carrier crate (`stegoeggo-stego/src/`)
- **Public**: `constants`, `error`, `frame`, `jpeg`, `lsb`, `pixels`, `prepared`, `types`
- **`pub(crate)`**: `jpeg_transcoder`, `lsb_internal`
- **`pub(crate)` (feature: application-support)**: `application_support` (`#[doc(hidden)]`)

## Function Signatures

### Public entry points (in `src/lib.rs`)
```rust
fn process_request_bytes(img_bytes: &[u8], request: &ProtectionRequest) -> Result<Vec<u8>>
fn process_request_bytes_with_warnings(img_bytes: &[u8], request: &ProtectionRequest) -> Result<(Vec<u8>, Vec<ProtectionWarning>)>
fn process_request_bytes_with_report(img_bytes: &[u8], request: &ProtectionRequest) -> Result<(Vec<u8>, ExecutionReport)>
fn process_request_bytes_parallel(images: &[Vec<u8>], request: &ProtectionRequest) -> Result<Vec<Vec<u8>>>  // feature: parallel
fn process_request_bytes_with_warnings_parallel(images: &[Vec<u8>], request: &ProtectionRequest) -> Result<Vec<(Vec<u8>, Vec<ProtectionWarning>)>>  // feature: parallel
fn process_request_bytes_with_report_parallel(images: &[Vec<u8>], request: &ProtectionRequest) -> Result<Vec<(Vec<u8>, ExecutionReport)>>  // feature: parallel
fn process_image_bytes(img_bytes: &[u8], level: ProtectionLevel, ctx: &ProtectionContext) -> Result<Vec<u8>>
fn verify_image_bytes(img_bytes: &[u8], mac_key: &[u8]) -> VerificationStatus  // NOT Result; centralized projection, NOT deprecated
fn verify_image_bytes_detailed(img_bytes: &[u8], mac_key: &[u8]) -> VerificationResult  // centralized projection
fn verify_image_bytes_report(img_bytes: &[u8], mac_key: &[u8]) -> VerificationReport  // canonical rich operation
fn verify_legal_notice(img_bytes: &[u8], mac_key: &[u8]) -> NoticeVerification  // centralized projection; build with NoticeVerification::builder()
```

### Async entry points (in `src/async_api.rs`, feature: `async`)
```rust
fn process_request_bytes_async(img_bytes: Vec<u8>, request: ProtectionRequest) -> Result<Vec<u8>>
fn process_request_bytes_with_warnings_async(img_bytes: Vec<u8>, request: ProtectionRequest) -> Result<(Vec<u8>, Vec<ProtectionWarning>)>
fn process_request_bytes_with_report_async(img_bytes: Vec<u8>, request: ProtectionRequest) -> Result<(Vec<u8>, ExecutionReport)>
// + parallel batch async variants when `parallel` is also enabled
```

### No-new-legacy-features invariant
- New processing features must be expressed in `ProtectionRequest` / `ProcessingOptions` / `ProtectionChannels` first
- Legacy `ProtectionContext` builders may only translate into those fields via `request_from_legacy()`; no independent policy, routing, or warning behavior
- `async_api.rs` request functions call canonical sync functions inside one `spawn_blocking` with no duplication
- Parallel batch reuses `process_request_bytes*` through Rayon, preserves order, no second executor

### Verification type surface

Canonical operation is `verify_image_bytes_report` (`src/verification/canonical.rs` performs one rights parse plus one hidden-marker search, building `VerificationReport` once). The other types are centralized projections from the same facts — do not add independent searches:

| Type | Location | Use |
|------|----------|-----|
| `VerificationReport` | `src/verification/report.rs` | Canonical rich result via `verify_image_bytes_report`; `VerificationReportBuilder` computes evidence strength |
| `VerificationStatus` | `src/types/verification.rs` (re-exported via `stegoeggo::types`) | Coarse stego-only projection of `verify_image_bytes`; live, NOT deprecated (`summary_status` rights fallback is separate) |
| `VerificationResult` | `src/types/verification.rs` (enum, re-exported via `stegoeggo::types`) | Detailed projection via `verify_image_bytes_detailed` (`Verified`/`Corrupted`/`MetadataOnly`/`NotFound`) |
| `NoticeVerification` | `src/types/verification.rs` (re-exported via `stegoeggo::types`) | Legal-notice projection via `verify_legal_notice`; build with `NoticeVerification::builder()` |

### Legacy compatibility (deprecated but functional)
```rust
fn process_image(img: DynamicImage, level: ProtectionLevel, ctx: &ProtectionContext) -> Result<DynamicImage>
fn process_images_parallel(images: &[DynamicImage], level: ProtectionLevel, ctx: &ProtectionContext) -> Result<Vec<DynamicImage>>  // feature: parallel
fn process_image_bytes_with_warnings(img_bytes: &[u8], level: ProtectionLevel, ctx: &ProtectionContext) -> Result<(Vec<u8>, Vec<ProtectionWarning>)>  // adds only MissingMacKey, ContradictoryLegalClaims, JpegReencodeFragile
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
lsb::embed_tiled(img, payload, &tile) -> Result<EmbedReport<RgbaImage>>
lsb::embed_tiled_in_place(img, payload, &tile) -> Result<InPlaceEmbedReport>
lsb::extract_tiled(img, payload_len, &tile, max_origins) -> Result<Vec<u8>>
lsb::embed_tiled_framed(img, payload, &tile) -> Result<EmbedReport<RgbaImage>>
lsb::extract_tiled_framed(img, &tile, max_origins) -> Result<Vec<u8>>

// Borrowed pixel buffers (same LSB core, no RgbaImage conversion)
PixelView::new(bytes, w, h, layout, stride) -> Result<PixelView>  // packed/strided RGB8/RGBA8
view.capacity(len, &config) / view.extract(len, &config) / view.extract_framed(&config)
view.extract_tiled(len, &tile, max_origins) / view.extract_tiled_framed(&tile, max_origins)
PixelViewMut::new(bytes, w, h, layout, stride) -> Result<PixelViewMut>
view.embed(payload, &config) / view.embed_framed(payload, &config)  // -> InPlaceEmbedReport
view.embed_tiled(payload, &tile) / view.embed_tiled_framed(payload, &tile)
view.extract(len, &config) / view.extract_framed(&config)  // convenience via as_view
view.extract_tiled(len, &tile, max_origins) / view.extract_tiled_framed(&tile, max_origins)
view.as_view() -> PixelView  // reborrow for extraction

// JPEG (DCT-domain)
jpeg::capacity(jpeg_bytes, payload_len, &config) -> Result<CapacityReport>
jpeg::embed(jpeg_bytes, payload, &config) -> Result<EmbedReport>  // best-effort: may downgrade, seed-only fallback
jpeg::embed_strict(jpeg_bytes, payload, &config) -> Result<EmbedReport>  // exact redundancy or InsufficientCapacity, no output
jpeg::extract(jpeg_bytes, payload_len, &config, actual_redundancy) -> Result<Vec<u8>>
jpeg::embed_framed(jpeg_bytes, payload, &config) -> Result<EmbedReport>
jpeg::embed_framed_strict(jpeg_bytes, payload, &config) -> Result<EmbedReport>
jpeg::extract_framed(jpeg_bytes, &config) -> Result<Vec<u8>>
jpeg::embed_tiled(jpeg_bytes, payload, &tile) -> Result<EmbedReport>
jpeg::extract_tiled(jpeg_bytes, payload_len, &tile, max_origins) -> Result<Vec<u8>>
jpeg::embed_tiled_framed(jpeg_bytes, payload, &tile) -> Result<EmbedReport>
jpeg::extract_tiled_framed(jpeg_bytes, &tile, max_origins) -> Result<Vec<u8>>
jpeg::probe_support(jpeg_bytes) -> Result<JpegSupport>
jpeg::embed_seed_hint(jpeg_bytes, seed) -> Result<Vec<u8>>  // transactional: full 96-bit hint or InsufficientCapacity
jpeg::extract_seed_hint(jpeg_bytes) -> Result<Option<u64>>
jpeg::is_progressive_jpeg(jpeg_bytes) -> bool

// Opaque prepared JPEG (one decode across repeated ops, borrows source)
PreparedJpeg::new(&jpeg_bytes) -> Result<PreparedJpeg>
prepared.support() / .capacity(len, &config) / .extract(len, &config, r)
prepared.extract_framed(&config) / .extract_tiled(len, &tile, n) / .extract_tiled_framed(&tile, n)
prepared.embed_strict(payload, &config) / .embed_framed_strict(payload, &config) / .seed_hint()

// Validated redundancy (identical debug/release semantics)
Redundancy::new(u8) / Redundancy::from_usize(usize) -> Result<Redundancy>  // 1..=10, Copy
LsbConfig::from_redundancy(seed, r) / .with_redundancy_value(r) / .redundancy_value()
JpegConfig::from_redundancy(seed, r) / .with_redundancy_value(r) / .redundancy_value()

// Shared tiled config + bound
TileConfig::try_new(seed, tile_size) -> Result<TileConfig>  // tile_size > 0; JPEG tiled needs >= 8 and multiple of 8
MAX_TILED_ORIGINS: u32 = 4096  // hard maximum for max_origins; 0 is InvalidConfig

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
5. **F5 seed embedding is transactional** — `embed_seed_in_quantization_tables` counts eligible positions (values >= 2) across the first 2 tables before mutating and returns `InsufficientHintCapacity` unless all 96 hint bits fit. Values of 1/0 carry no hint bit.
6. **ISCC is not standard-compliant** — uses custom component codes (`0x12`, `0x33`), not interoperable with other ISCC implementations.
7. **V3 is the current payload format** — V1/V2 are extraction-only legacy. V3 adds TLV extensions with domain-separated authentication.
8. **`verify_payload_from_bytes_with_key` returns `VerificationStatus`** — not `Option<bool>`.
9. **Generic carrier error type** — The public `stego` module uses `StegoError`, not the root crate `Error`. Convert via `From<StegoError> for Error`.
10. **JPEG DCT one-pass embed** — Supported DCT embedding computes max feasible redundancy from capacity, then embeds+encodes once. No retry loop.
11. **Generic carrier operation styles** — The `stegoeggo::stego` facade exposes raw (`lsb::embed`/`extract`, `jpeg::embed`/`extract`, caller knows payload length and JPEG `actual_redundancy`); strict JPEG (`jpeg::embed_strict`/`embed_framed_strict`, exact redundancy or `InsufficientCapacity` with no output); in-place (`lsb::embed_in_place` mutates the caller's `RgbaImage` and shares the corrected V2 mutation core with the cloning `lsb::embed`); borrowed views (`PixelView`/`PixelViewMut` for packed/strided RGB8/RGBA8, same core, alpha/padding never carriers); prepared JPEG (`PreparedJpeg` borrows encoded bytes, one decode across repeated ops); framed (`lsb::embed_framed`/`extract_framed`, `jpeg::embed_framed`/`extract_framed` over `frame::{encode, decode_prefix, decode}`); and tiled (`embed_tiled`/`embed_tiled_in_place`/`extract_tiled`/`embed_tiled_framed`/`extract_tiled_framed` over shared `TileConfig` with explicit `max_origins` in `1..=MAX_TILED_ORIGINS`). Best-effort `jpeg::embed` (auto-downgrade + seed-only fallback) is the parent's explicit application policy, not generic carrier semantics. Framed extraction keeps the seed/config explicit, validates capacity before full extraction, and treats CRC32 as corruption detection rather than authentication. Raw tiled recovery returns the first candidate and cannot authenticate; prefer framed tiled for crops. Tiled JPEG uses redundancy 1 per tile and rejects non-multiple-of-8 sizes.
12. **Validated configuration** — `Redundancy::new`/`from_usize` (identical debug/release semantics) with `from_redundancy`/`with_redundancy_value`/`redundancy_value` on both configs is the recommended primitive for runtime values. `LsbConfig::try_new`, `LsbConfig::try_with_redundancy`, `JpegConfig::try_new`, and `JpegConfig::try_with_redundancy` all return `StegoError::InvalidConfig` for out-of-range redundancy. The infallible `with_redundancy` builder is a compatibility adapter for compile-time constants only (debug-assert/release-clamp); never pass runtime values through it. Zero seeds are valid. JPEG capacity units are eligible AC coefficients with `|coef| >= 2`, not all non-zero AC coefficients. The V2 LSB mapping is byte-frozen with injectivity verified for documented domains only — never claim a full-domain bijection.
13. **Decomposed application stego adapter** — `src/protected/steganography/` is split into five responsibility modules behind `SteganographyProtector`: `marker.rs` (V3 payload construction), `embed.rs` (plan-based dispatch with shared private helpers — `outcome_from_report`, `embed_dct_payload`/`embed_dct_tiled_payload` with progressive fallback, `inplace_summary`, `embed_raster_with_seed_fallback`; context-based wrappers delegate to the same helpers and PNG/WebP share one raster path), `extract.rs` (seed discovery and bounded search, including hidden `JpegSearchContext` reuse), `verify.rs` (integrity and authentication classification), and `legacy.rs` (V1/V2 compatibility). `mod.rs` is a thin facade + shared types + tests; no carrier algorithm is reimplemented there. Hidden `application_support` holds only legacy/seed-fallback/search compat, never ordinary current embed/extract. `src/types/` (rights/compat/legal/context/verification/warnings/request behind `src/types.rs` re-exports), `src/protected/metadata_trap/` (notice/png/jpeg/webp/common behind `RightsMetadataProtector`), and `src/pipeline.rs` (canonical executors) follow the same facade pattern: stable public paths, private submodules.
14. **JPEG extraction is single-decode per operation** — `jpeg::extract_framed` retains private decoded coefficients for its bounded redundancy search; application verification shares one hidden `JpegSearchContext` across standard probing and tiled fallback. Do not recompose either from public `capacity`/`extract` calls, add per-redundancy `jpeg_extract` calls in `dct_candidates`, expose coefficient/header types, or reduce the configured search domain.
18. **Tiled LSB has one in-place core** — `lsb_internal::embed_lsb_tiled_in_place` is the shared algorithm; the cloning `embed_lsb_tiled` delegates to it and the parent raster path mutates its owned RGBA directly. Insufficient capacity leaves the caller's buffer unchanged.
19. **Raster preflight is header-only** — non-JPEG dimension gating uses `into_dimensions()` only; the executor owns the single full decode and re-checks dimensions defensively. Same-format metadata-only performs zero pixel decodes.
15. **Benchmark equivalence** — The `lsb_clone_vs_in_place` benchmark uses Criterion batching so each in-place iteration starts from a pristine source image and the preparation clone remains outside the timed operation.
20. **Container accounting has one owner** — `src/container_walk.rs::observe_container_work` is the only bounded PNG/JPEG/WebP walk for `OperationObserver`. Do not add hand-written chunk/marker loops in `lib.rs` or `pipeline.rs`; strict injection/detection walkers live in their format modules with different error dispositions. All container arithmetic uses `checked_add` with lenient break on overflow.
16. **`verify_image_bytes` returns directly** — Returns `VerificationStatus`, not `Result<VerificationStatus>`. Use `verify_image_bytes_detailed` for full `VerificationResult`.
17. **Output-domain carrier routing** — Carrier family is selected from the final output format (`output_format == JPEG ? DCT : LSB`); input format controls fast-path reuse only. `execute_full_marker_and_metadata()` in `src/pipeline.rs` is the sole current-carrier router; `apply_lsb_to_image_with_summary_from_plan()` in `src/protected/steganography/embed.rs` is explicitly raster-domain and must never branch on `plan.input_format()`. JPEG→PNG/WebP is one pixel decode plus LSB, never a transient DCT step. `EmbedPath` follows the operation actually executed (`Lsb`/`LsbTiled` for raster output, `DctF5`/`DctF5Tiled` for JPEG output).
21. **Timestamp provenance** — Canonical metadata writers consume the resolved `RightsNotice`; JPEG structured COM rendering must receive `notice_applied_at` from that same notice. An explicit `ProtectionRequest::with_timestamp_override(...)` must not be replaced by a lower-level `SystemTime::now()` read, while requests without an override retain wall-clock defaults.

## Build & Test
```bash
./scripts/check.sh                      # Fast local check (fmt, clippy, no-default-features, tests)
cargo test --workspace --exclude stegoeggo-fuzz --all-features  # All tests
cargo clippy --workspace --all-targets --all-features -- -D warnings  # Lint
cargo fmt --all -- --check              # Format check
```

## Testing Patterns
- Unit tests live in each source file as `#[cfg(test)] mod tests`
- Integration tests in `tests/` directory (35 test files, including `container_accounting.rs` for Plan 085 resource-accounting regression)
- Test with `ProtectionContext::new(intensity, seed)` for deterministic results
- `ProtectionContext::default()` uses CSPRNG-backed seed (via `getrandom`) — safe for production; use `ProtectionContext::new(intensity, seed)` for reproducibility
- Feature-gated tests: `tests/async_integration.rs` requires `async` feature
- Public generic carrier tests belong in `tests/public_stego_api.rs`; framed tests must verify recovery without retaining the original payload length, and JPEG tests must cover auto-downgraded redundancy

## Where Documentation Lives
- User-facing guides: `docs/` (`cli-usage.md`, `rust-api.md`, `formats.md`, `carrier-crate.md`, `legal_notice_model.md`, `migration-v0.3.md`)
- Architecture deep-dives: `architecture/` (31 files), indexed by `architecture/overview.md`
- Agent conventions: this file plus `AGENTS.md` gotchas (CLI flags, exit codes, container correctness)
- Fuzz assurance: `fuzz/README.md` (pinned nightly/cargo-fuzz tuple, LTO compatibility boundary, and update policy)
- Runnable examples: `examples/` (`protect_and_verify.rs`, `verify_saved.rs`, `legal_metadata.rs`, `generic_stego.rs`) — keep these compiling when changing public APIs
