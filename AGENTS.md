# AGENTS.md

## Project Overview

`cloakrs` is a Rust library and CLI for protecting images from unauthorized AI model training through adversarial image poisoning. Designed for CDN/WAF edge deployment with sub-10ms latency targets.

## Tech Stack

- Rust (edition 2021, MSRV 1.87, stable channel)
- Key crates: `image` 0.25, `jpeg-encoder` 0.7, `rayon` 1.10, `sha2`/`hmac` for crypto, `serde`/`serde_json` for serialization, `subtle` 2 (constant-time comparisons), `tokio` (optional, for async)

## Architecture

- **Strategy pattern** via `Protector` trait (`src/traits.rs`) with five protection levels: Disabled, Light, Standard, Enhanced, Strong
- **Pipeline** (`src/lib.rs`): `ProtectionPipeline` orchestrates multiple protectors based on `ProtectionLevel`
- **JPEG fast path**: When input/output are both JPEG, operates directly on DCT coefficients via custom transcoder (`src/jpeg_transcoder/`), bypassing pixel decode/encode
- **Cow returns**: `Protector::apply` returns `Cow<'a, DynamicImage>` to avoid unnecessary cloning

## Module Layout

```
src/
├── lib.rs                 # Pipeline, top-level functions, module exports
├── types.rs               # Core types (ProtectionLevel, ProtectionContext, etc.)
├── traits.rs              # Protector trait, VariantLoader trait
├── error.rs               # Error enum (thiserror)
├── async_api.rs           # Async wrappers (spawn_blocking)
├── protected/             # Protection strategies
│   ├── constants.rs       # Tuning constants
│   ├── passthrough.rs     # No-op (Disabled)
│   ├── noise.rs           # Adversarial noise (Standard)
│   ├── enhanced.rs        # Higher intensity (Enhanced)
│   ├── precomputed.rs     # Precomputed variants (Strong)
│   ├── metadata_trap.rs   # Metadata injection (Light)
│   └── steganography.rs   # LSB/DCT steganographic embedding
├── jpeg_transcoder/       # JPEG-specific processing
│   ├── header.rs          # JPEG header parser
│   ├── entropy.rs         # Huffman entropy codec
│   └── stego_f5.rs        # F5-style DCT steganography
└── util/
    ├── image.rs           # Encoding, perturbation, hash
    ├── iscc.rs            # ISCC content identifiers
    └── seed.rs            # Random seed generation
```

## Key Types

- `ProtectionContext::new(intensity: f32, seed: u64)` — intensity clamped to [0.0, 1.0]
- `ProtectedVariant::new(hash, level, perturbation_data, intensity, width, height)` — no target model parameter
- `ProtectionConfig` — shared heavy config (MAC key, legal metadata) wrapped in `Arc`
- `StegoPayload` — extracted stego data with `protection_level()`, `seed()`, `intensity()`, `version()` getters
- All struct fields on `ProtectionContext`, `ProtectedVariant`, and `StegoPayload` are private — use getter methods (e.g., `ctx.intensity()`, `ctx.seed()`, `variant.perturbation_data()`)
- `ProtectionContext` has `set_input_format()` (public) and `set_protection_level()` (crate-internal) for non-builder mutation

## Architecture Documentation

Architecture docs live in `architecture/`. A consolidated fix plan is at `plans/plan.md` — it tracks all documentation discrepancies between docs and code, organized into 5 waves for parallel execution via sub-agents.

## Build & Test Commands

```bash
cargo check                              # Compilation
cargo test                               # All tests (150 unit + 20 basic + 57 integration)
cargo test --all-features                # Includes async tests (9 tests) — 245 total
cargo clippy --all-targets -- -D warnings # Lint check
cargo fmt --check                        # Format check
cargo bench                              # Criterion benchmarks
```

## Code Conventions

- Rustfmt: 4-space indentation, max width 100
- No comments in code unless explicitly asked
- `#[must_use]` on builder methods
- `pub(crate)` for internal modules (e.g., `jpeg_transcoder`)
- `LazyLock` static singletons for default pipelines
- `Arc<ProtectionConfig>` for shared heavy fields
- Private fields with getter methods on `ProtectionContext`, `ProtectedVariant`, `StegoPayload`, `LegalMetadata`

## Things to Watch Out For

- **`.gitignore`**: `.DS_Store` files exist on disk but are excluded by git
- **Stego payload format**: 24-byte header + 2-byte checksum (or 8-byte HMAC), always padded to 32 bytes total (even in checksum mode). Use `MIN_PAYLOAD_SIZE` (=26) and `MIN_PAYLOAD_BITS` (=208) constants in `steganography.rs`
- **`generate_random_seed()`**: Not cryptographically secure — uses SystemTime + splitmix64 mixing (`seed.rs:16-33`). Uses `unwrap_or_default()` (does NOT panic on pre-UNIX-epoch clocks). Guarantees non-zero output (`if x == 0 { 42 }`)
- **`ProtectionContext::default()`**: Calls `generate_random_seed()` — the seed is predictable. Doc comment on the `Default` impl warns about this. Users needing cryptographic seeds should use `ProtectionContext::new(intensity, seed)` with a CSPRNG
- **JPEG transcoder modules**: `header.rs` and `entropy.rs` have `#![allow(dead_code)]` for JPEG spec reference types (color spaces, standard Huffman tables) — keep these
- **ISCC module** (`src/util/iscc.rs`): Content identification, exported from `lib.rs`. NOT ISCC-standard compliant — uses custom component codes (`0x12`, `0x33`) and non-standard DCT hash. Produces ISCC-like identifiers that are not interoperable with other ISCC implementations
- **No `TargetModel`**: This concept was removed. `ProtectionContext::new` takes `(intensity, seed)` only
- **Steganography seed derivation**: `embed_lsb` and `embed_jpeg_stego` internally derive `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)` before using `stego_permutation`. When calling internal embed/extract functions directly (outside of `apply()`), ensure seeds match. The public API (`apply()` + `extract_payload()`) handles this correctly
- **XorShiftRng — two separate implementations**: `src/util/image.rs` has `XorShiftRng` (general-purpose noise/pixel selection) and `src/jpeg_transcoder/stego_f5.rs` has `F5XorShiftRng` (DCT coefficient shuffling). They use different algorithms and produce different sequences for the same seed. Do NOT interchange them — each is paired with their respective embed/extract code paths
- **MetadataTrapProtector::apply()**: `apply()` returns `Cow::Borrowed(img)` unchanged. Metadata injection operates at the byte level and the `DynamicImage` API cannot preserve injected text chunks through encode/decode cycles. The pipeline routes `Light` level through `apply_light_bytes()` which encodes, injects metadata, then decodes (metadata survives in the byte output). For byte-level output with metadata intact, use `apply_bytes()` or `process_bytes()`
- **`verify_payload_with_key()` returns `Option<bool>`**, not `bool` — use `== Some(true)` or `!= Some(true)` in assertions, not `assert!()` or `assert!(!)` directly
- **JPEG pixel stego redundancy**: `embed_jpeg_stego` (`steganography.rs`) supports `redundancy > 1` — the embedding loop uses `break` to exit inner loops after each pass completes, allowing the outer `for pass` loop to continue. Extraction always runs 5 passes with majority voting
- **HuffmanDecoder/HuffmanEncoderTable caching**: In `entropy.rs`, Huffman decoders and encoder lookup tables are pre-built once before the MCU loop. `HuffmanEncoderTable` uses a `[(u16, u8); 256]` array for O(1) symbol→(code,length) lookup. These are internal types — do not rebuild per-MCU
- **JPEG header parser bounds**: `header.rs:parse()` has guards for `data.len() < 2` (empty input) and `end_pos < 10` (too short for SOF). Segment data end uses `.max(segment_data_start)` to prevent inverted slice ranges when `segment_len` is malformed. Parse errors now include byte offset for debugging
- **PrecomputedProtector auto-caching**: `apply()` auto-registers generated perturbations via `register_variant` on cache miss. The perturbation is no longer cloned — it is moved into the variant after use. Registration failure is silently ignored (best-effort caching). The `register_variant` method has a doc comment explaining the two-phase design (persist without lock, then insert with write lock). NOTE: The cache (`RwLock<HashMap>`) has no eviction policy, size limit, or TTL — unbounded growth under sustained load
- **`subtle` crate for constant-time comparison**: `verify_payload_mac` in `steganography.rs` uses `subtle::ConstantTimeEq::ct_eq()` instead of `==` to prevent timing attacks on HMAC verification
- **Pipeline intensity semantics**: `intensity` controls only the perturbation stage (noise/Enhanced/Precomputed). Steganography and metadata injection run regardless of intensity value. This is by design — they are orthogonal protection layers
- **F5 no-zero variant correctness**: `stego_f5.rs` implements a no-zero F5 variant that increments |coef| when |coef|=1 and LSB mismatches, avoiding detectable zero creation. The embed/extract position alignment is preserved because no coefficient is ever zeroed out. The implementation is correct
- **F5 seed embedding Q-table edge case**: `stego_f5.rs:103-104` clears quantization table LSBs with `&= 0xFE`. A quantization value of 1 becomes 0 (invalid in JPEG). A post-clear clamp (`if val == 0 { val = 1 }`) prevents this, but seed embedding may fail silently if too many values are 1 (0-bits can't stick). Use quantization values >= 2 for reliable seed embedding
- **`#[serde(skip)]` on `config` field**: `ProtectionContext.config` (`Option<Arc<ProtectionConfig>>`) is skipped during serialization. MAC keys and legal metadata are lost in serde roundtrips. A test (`test_config_skipped_in_serde_roundtrip`) documents this behavior
- **Async batch processing**: `process_images_parallel_async` and `process_images_bytes_parallel_async` run the entire batch inside a single `spawn_blocking`, delegating to the synchronous rayon-based parallel functions. This avoids per-image `spawn_blocking` calls that would cause thread pool overlap. Single-image async functions (`process_image_async`, `process_image_bytes_async`) still use one `spawn_blocking` per image, which is appropriate for the WAF hot path
- **`parallel_threshold()`**: Scales with `rayon::current_num_threads()` — returns `cores * 64 * 64`. At 4 cores: 256×256 = 65536. At 1 core: 4096. At 16 cores: 131072. Replaces the old hardcoded `PARALLEL_THRESHOLD_PIXELS` const
- **`PerturbationRuntime`**: Shared setup struct in `util/image.rs` that both serial and parallel perturbation paths use. Pre-computes `NoiseGenerator`, spatial seed, and per-row `y_variations` — eliminates duplicated code between the two paths. `PerturbationParams` retains the `NoiseGenerator` internally and exposes `derive_spatial_seed()` so callers avoid redundant HMAC key initialization
- **Steganography fallback seeds**: Extracted to `FALLBACK_SEEDS` constant in `steganography.rs`. These are common test/dev seeds tried when metadata is stripped
- **Format detection strictness**: `apply_multi_protector_bytes` returns `Error::InvalidFormat` when the input format cannot be determined (from `ctx.input_format()` or magic bytes). Previously it silently defaulted to PNG
- **JPEG segment length bounds**: `get_scan_data_start` now uses `checked_add` to prevent integer overflow when advancing past segments with malformed lengths
- **Entropy decoder natural order**: The entropy decoder stores coefficients directly in natural (row-major) order via `block[ZIGZAG[k]] = magnitude`. The old reorder loop was redundant (identity operation) and has been removed
- **JPEG quantization debug assertion**: `assemble_jpeg` has a `debug_assert!` for 8-bit quantization values exceeding 255
- **`inject_metadata` / `inject_legal_claims` are `Option<bool>`**: Default is `None`, not `true`/`false`. The `None` semantics (use level default) vs explicit `false` (disable) are not well-documented. `with_metadata_injection(false)` and not calling it at all have different effects
- **`process_bytes` skips dimension validation**: `process()` validates dimensions against `max_dimension` (lib.rs:217) but `process_bytes()` does not (lib.rs:318). Large images can bypass the check via the byte path
- **`is_enabled()` is dead code**: Defined in the `Protector` trait with default `true`. `PassthroughProtector` overrides it to return `true`. The pipeline never calls `is_enabled()` — it uses direct `match level` dispatch
- **`ProtectedVariant::cache_key()` format**: Returns `{hash}_{level}_{intensity}` (not `{uuid}_{hash}_{intensity}`). The UUID (`variant_id`) is generated but not included in the cache key
- **Pipeline flow order (JPEG output)**: For JPEG output, the pipeline applies perturbation → encode → DCT stego → metadata (encode happens before stego). For non-JPEG output: perturbation → pixel stego → encode → metadata. The JPEG→JPEG fast path (`apply_multi_protector_bytes`) bypasses perturbation entirely and only applies DCT stego + metadata
- **CLI file path**: The CLI binary lives at `cloakrs-cli/src/main.rs`, not `src/bin/cloakrs/main.rs`
- **MIN_PAYLOAD_SIZE vs actual payload size**: `MIN_PAYLOAD_SIZE` (=26) is the minimum valid payload for parsing (24-byte header + 2-byte checksum). `generate_payload()` always produces 32 bytes (padded with zeros). These are different numbers — the constant is a parsing threshold, not the output size
- **`register_variant` error handling**: The `register_variant()` method returns `Result<()>` and propagates loader errors with `?`. However, `PrecomputedProtector::apply()` calls `let _ = self.register_variant(variant)` — silently discarding errors. The cache is best-effort; registration failure is not fatal
