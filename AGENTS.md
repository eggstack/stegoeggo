# AGENTS.md

## Project Overview

`stegoeggo` is a Rust library and CLI for protecting images from unauthorized AI use through rights-reservation metadata and AI-training restriction notices.

## Tech Stack

- Rust (edition 2021, MSRV 1.87, stable channel)
- Key crates: `image` 0.25, `jpeg-encoder` 0.7, `rayon` 1.10, `sha2`/`hmac` for crypto, `serde`/`serde_json` for serialization, `subtle` 2 (constant-time comparisons), `iscc-lib` 0.4 (ISO 24138:2024 ISCC), `getrandom` 0.2 (CSPRNG), `tokio` (optional, for async)

## Design Philosophy

- **Legal deterrence, not anti-forensics**: The purpose is making protection detectable in legal proceedings, not preventing a determined adversary from removing it
- **Forgeability is irrelevant**: HMAC/checksum forgeability does not matter — the goal is detection, not authentication. A forger who can strip protection entirely has already lost the deterrence value
- **Robustness is the primary metric**: Steganographic payload survival against common image transformations (compression, resize, format conversion, social media re-encoding) is the most important engineering metric
- **Metadata stripping is a known limitation**: All image processing tools can strip metadata. This is why steganographic embedding is the primary detection channel — metadata is a bonus layer, not the core defense

## Workspace Structure

Three workspace members:
- `.` — Main library crate (`stegoeggo`)
- `stegoeggo-cli/` — CLI binary (`stegoeggo` binary name)
- `fuzz/` — Fuzz harnesses (requires `cargo-fuzz` + nightly)

## Architecture

- **Strategy pattern** via `Protector` trait (`src/traits.rs`) with three protection levels: Disabled, Light, Standard
- **Pipeline** (`src/lib.rs`): `ProtectionPipeline` orchestrates protectors based on `ProtectionLevel`
- **JPEG fast path**: When input/output are both JPEG, operates directly on DCT coefficients via custom transcoder (`src/jpeg_transcoder/`), bypassing pixel decode/encode
- **Cow returns**: `Protector::apply` returns `Result<Cow<'a, DynamicImage>>` to avoid unnecessary cloning

## Module Layout

```
src/
├── lib.rs                 # Pipeline, top-level functions, module exports
├── types.rs               # Core types (ProtectionLevel, ProtectionContext, etc.)
├── traits.rs              # Protector trait
├── error.rs               # Error enum (thiserror)
├── async_api.rs           # Async wrappers (spawn_blocking)
├── protected/             # Protection strategies
│   ├── constants.rs       # Tuning constants
│   ├── passthrough.rs     # No-op (Disabled)
│   ├── metadata_trap.rs   # Metadata injection (Light)
│   ├── steganography.rs   # LSB/DCT steganographic embedding
│   ├── ecc.rs             # 3x repetition ECC with majority voting
│   ├── notice_verification.rs # Legal notice verification (verify_legal_notice)
│   └── stego_cost.rs      # Pixel embedding cost computation (Laplacian-based)
├── jpeg_transcoder/       # JPEG-specific processing
│   ├── header.rs          # JPEG header parser
│   ├── entropy.rs         # Huffman entropy codec
│   └── stego_f5.rs        # F5-style DCT steganography
└── util/
    ├── image.rs           # Encoding, hash
    ├── iscc.rs            # ISCC content identifiers
    └── seed.rs            # Random seed generation
```

## Key Types

- `ProtectionContext::new(intensity: f32, seed: u64)` — intensity clamped to [0.0, 1.0]
- `ProtectionConfig` — MAC key and legal metadata wrapped in `Arc`
- `StegoPayload` — extracted stego data with `protection_level()`, `seed()`, `intensity()`, `version()` getters
- `EvidenceProfile` — enum with 4 variants: `LegalNotice` (default), `LegalNoticeWithStego`, `AuthenticatedProvenance`, `Maximal`. Controls warning interpretation and evidence posture. Access via `ctx.evidence_profile()` (defaults to `LegalNotice` when not set)
- `ProtectionContext` helper constructors: `::legal_notice()`, `::legal_notice_with_stego()`, `::authenticated_provenance()`, `::maximal()`
- All struct fields on `ProtectionContext` and `StegoPayload` are private — use getter methods (e.g., `ctx.intensity()`, `ctx.seed()`)
- `ProtectionContext` has `set_input_format()` (public) and `set_protection_level()` (crate-internal) for non-builder mutation

## Build & Test Commands

```bash
cargo check                              # Compilation
cargo test                               # All tests (448 passed, 7 ignored)
cargo test --all-features                # Includes async tests (9 tests)
cargo clippy --all-targets -- -D warnings # Lint check
cargo fmt --check                        # Format check
cargo bench                              # Criterion benchmarks
```

## CI Pipeline

GitHub Actions (`.github/workflows/ci.yml`) runs:
1. MSRV check (`cargo check --all-features` with Rust 1.87)
2. Tests (`cargo test --all-features`)
3. Doc tests (`cargo test --doc`)
4. Format check (`cargo fmt --check`)
5. Clippy lint (`cargo clippy --all-targets -- -D warnings`)
6. Security audit (`cargo audit`)
7. License/advisory check (`cargo deny check licenses && cargo deny check advisories`)
8. Benchmarks (main branch only)

## Code Conventions

- Rustfmt: 4-space indentation, max width 100
- No comments in code unless explicitly asked
- `#[must_use]` on builder methods
- `pub(crate)` for internal modules (e.g., `jpeg_transcoder`)
- Private fields with getter methods on `ProtectionContext`, `StegoPayload`, `LegalMetadata`

## Architecture Documentation

Architecture docs live in `architecture/` (19 files). All docs have been verified against source code.

## Fuzzing

Three fuzz targets in `fuzz/`:
- `pipeline_bytes` — Full bytes-in/bytes-out path
- `tiled_round_trip` — Tiled steganography end-to-end
- `jpeg_parser` — JPEG header/entropy/DCT parsing

Run with: `cargo +nightly fuzz run <target> -- -max_total_time=60`

## Things to Watch Out For

- **Stego payload format**: 24-byte header + 4-byte CRC32 checksum (or 8-byte HMAC), always padded to 32 bytes total. Use `MIN_PAYLOAD_SIZE` (=28) and `MIN_PAYLOAD_BITS` (=224) constants in `steganography.rs`. Note: `MIN_PAYLOAD_SIZE` is a parsing threshold, not the output size — non-MAC payloads are 76 bytes (ECC-encoded), MAC payloads are 32 bytes
- **`ProtectionContext::default()`**: Uses `getrandom` (CSPRNG) for an unpredictable seed. For reproducible results, use `ProtectionContext::new(intensity, seed)` with an explicit seed
- **XorShiftRng — two separate implementations**: `src/util/image.rs` has `PixelSelectionRng` (pixel selection) and `src/jpeg_transcoder/stego_f5.rs` has `DctCoefficientRng` (DCT shuffling). Different algorithms, different sequences for same seed. Do NOT interchange them
- **MetadataTrapProtector::apply()**: Returns `Cow::Borrowed(img)` unchanged — metadata injection is byte-level. The pipeline routes `Light` through `apply_light_bytes()` which encodes → injects → decodes. For byte-level output with metadata, use `apply_bytes()` or `process_bytes()`
- **`stego_redundancy` is `Option<usize>`**: Default `None` derives from intensity via `effective_redundancy()` (<0.3→1, 0.3-0.7→2, >=0.7→3). Explicit `.with_stego_redundancy(n)` overrides this. Valid range 1-10
- **Verification API**: `verify_payload_with_key()` and `verify_image_bytes()` return `VerificationStatus` (`Verified`, `Invalid`, `NotFound`), not `Option<bool>`. Use `== VerificationStatus::Verified` in assertions. For richer info, use `verify_image_bytes_detailed()` → `VerificationResult`. For comprehensive legal notice verification, use `verify_legal_notice()` → `NoticeVerification` with `EvidenceStrength` and `EvidenceChannel`
- **`inject_metadata` / `inject_legal_claims` are `Option<bool>`**: Default `None` (use level default) vs explicit `false` (disable). `with_metadata_injection(false)` ≠ not calling it at all
- **Pipeline flow order**: JPEG output: encode → DCT stego → metadata. Non-JPEG: pixel stego → encode → metadata. JPEG→JPEG fast path bypasses pixel decode entirely
- **F5 seed embedding Q-table edge case**: `embed_seed_in_quantization_tables()` fails if any quantization value in the first 2 tables is < 2. Values of 1 cannot represent a 0-bit (`1 & 0xFE = 0`, clamped back to 1)
- **`#[serde(skip)]` on `config` field**: `ProtectionContext.config` is skipped during serialization. MAC keys and legal metadata are lost in serde roundtrips
- **Async batch processing**: `process_images_parallel_async` runs the entire batch inside a single `spawn_blocking`, delegating to rayon-based sync functions. Single-image async uses one `spawn_blocking` per image
- **Three seed storage locations**: (1) Q-table LSBs in JPEG (survives byte-preserving metadata insertion; not arbitrary recompression), (2) metadata markers (strippable by any tool), (3) fixed-position LSB in first 64 pixel channels. Extraction chain: metadata → LSB fallback → `FALLBACK_SEEDS`
- **ECC on stego payload**: Non-MAC payloads use 3× repetition with majority voting before CRC32. The `ecc` module provides `ecc_encode`/`ecc_decode`. MAC payloads use 8-byte HMAC instead
- **Spread spectrum LSB**: Each payload bit embedded across `STEGO_SPREAD_FACTOR` (=5) adjacent pixels via majority voting
- **Large-magnitude DCT coefficient preference**: F5 sorts non-zero AC coefficients by |magnitude| descending before shuffling
- **F5 redundancy cap**: Max redundancy is 10. Extraction tries all 10 values
- **Payload version migration**: Current version is 2. V1 (24-byte) still supported for extraction. To add v3: bump `CURRENT_PAYLOAD_VERSION`, add to `SUPPORTED_PAYLOAD_VERSIONS`, add `parse_stego_payload_v3` arm
- **CLI file path**: CLI binary lives at `stegoeggo-cli/src/main.rs`, not `src/bin/`
- **CLI batch filename collisions**: Duplicate output stems get `_protected_1`, `_protected_2`, etc.
- **CLI legal metadata flags**: `--copyright-holder`, `--creator`, `--contact`, `--rights-url`, `--usage-terms`, `--ai-constraints` set `LegalMetadata` fields. `--no-ai-training`, `--no-genai-training`, `--tdm-reserved` are DMI presets that also set default `ai_constraints` text. Any legal flag auto-enables `--legal-claims`. `--metadata false` + legal flags → error. `--strict` exits with error if any warnings have Error severity
- **Tiled steganography** (`with_tile_size(n)`): Crop-resistant mode. Embeds full payload per tile. Tiled F5 limited to tile-aligned crops without re-encode. `tile_seed(master_seed, tile_x, tile_y)` uses splitmix64. Tiled paths are verification fallbacks
- **F5 tiled block set**: MCU-interleaved block ordering: `block_idx = (mcu_y * mcus_per_row + mcu_x) * h * v + sub_y * h + sub_x`. Do NOT assume row-major ordering
- **ProtectionWarning variants**: 6 variants: `MissingMacKey`, `MetadataInjectionDisabled`, `ProgressiveJpegFallback`, `JpegReencodeFragile`, `LsbCapacitySkipped`, `DctCapacityInsufficient`. Returned by `process_image_bytes_with_warnings`
- **WarningCategory / WarningSeverity**: `ProtectionWarning` has `category() -> WarningCategory` and `severity_for_profile(profile) -> WarningSeverity` methods. Categories: `LegalNotice`, `BestEffortStego`, `AuthenticatedProvenance`, `FormatFragility`. Severities: `Info`, `Warning`, `Error`
- **Fuzz harness**: 3 targets in `fuzz/`: `pipeline_bytes`, `tiled_round_trip`, `jpeg_parser`. Run with `cargo +nightly fuzz run <target> -- -max_total_time=60`. Add regression tests in `tests/robustness.rs` for findings
- **EvidenceProfile vs ProtectionLevel**: `ProtectionLevel` controls how much processing occurs (Disabled/Light/Standard). `EvidenceProfile` controls how warnings are interpreted and the default evidence posture. They are orthogonal — you can use any profile with any level
- **MissingMacKey warning is profile-dependent**: Only emitted for `AuthenticatedProvenance` and `Maximal` profiles. `LegalNotice` and `LegalNoticeWithStego` do not warn about missing MAC keys
- **`NoticeVerification`, `EvidenceStrength`, `EvidenceChannel`**: Types in `src/types.rs` for legal-notice conformance reporting. `verify_legal_notice()` in `src/protected/notice_verification.rs` extracts legal fields from PNG tEXt, JPEG COM, and WebP XMP markers, identifies evidence channels (JpegXmp, JpegIptc, PngXmp, etc.), verifies stego payload integrity (LsbPayload for non-JPEG, DctPayload for JPEG), and returns an evidence strength rating
- **Creator metadata injection**: `add_legal_metadata()` in `metadata_trap.rs` injects `Creator` as a tEXt chunk (PNG) or COM marker (JPEG). Ensure `LegalMetadata::creator()` is wired into both injection and extraction paths
