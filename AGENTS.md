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
- **Stego payload format**: 24-byte header + 2-byte checksum (or 8-byte HMAC), always padded to 32 bytes total (even in checksum mode). Use `MIN_PAYLOAD_SIZE` and `MIN_PAYLOAD_BITS` constants in `steganography.rs`
- **`generate_random_seed()`**: Not cryptographically secure — uses SystemTime + splitmix64. Document this if changed
- **`ProtectionContext::default()`**: Calls `generate_random_seed()` — the seed is predictable. Doc comment on the `Default` impl warns about this. Users needing cryptographic seeds should use `ProtectionContext::new(intensity, seed)` with a CSPRNG
- **JPEG transcoder modules**: `header.rs` and `entropy.rs` have `#![allow(dead_code)]` for JPEG spec reference types (color spaces, standard Huffman tables) — keep these
- **ISCC module** (`src/util/iscc.rs`): Critical component for content identification, exported from `lib.rs`
- **No `TargetModel`**: This concept was removed. `ProtectionContext::new` takes `(intensity, seed)` only
- **Steganography seed derivation**: `embed_lsb` and `embed_jpeg_stego` internally derive `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)` before using `stego_permutation`. When calling internal embed/extract functions directly (outside of `apply()`), ensure seeds match. The public API (`apply()` + `extract_payload()`) handles this correctly
- **XorShiftRng — two separate implementations**: `src/util/image.rs` has `XorShiftRng` (general-purpose noise/pixel selection) and `src/jpeg_transcoder/stego_f5.rs` has `F5XorShiftRng` (DCT coefficient shuffling). They use different algorithms and produce different sequences for the same seed. Do NOT interchange them — each is paired with its respective embed/extract code paths
- **MetadataTrapProtector::apply() vs apply_bytes()**: `apply()` injects metadata into bytes then re-decodes to `DynamicImage`, which strips PNG/JPEG text chunks. Use `apply_bytes()` when metadata must survive in the byte stream (e.g., seed extraction tests)
- **`verify_payload_with_key()` returns `Option<bool>`**, not `bool` — use `== Some(true)` or `!= Some(true)` in assertions, not `assert!()` or `assert!(!)` directly
- **JPEG pixel stego redundancy**: `embed_jpeg_stego` (`steganography.rs`) supports `redundancy > 1` — the embedding loop uses `break` to exit inner loops after each pass completes, allowing the outer `for pass` loop to continue. Extraction always runs 5 passes with majority voting
- **HuffmanDecoder/HuffmanEncoderTable caching**: In `entropy.rs`, Huffman decoders and encoder lookup tables are pre-built once before the MCU loop. `HuffmanEncoderTable` uses a `[(u16, u8); 256]` array for O(1) symbol→(code,length) lookup. These are internal types — do not rebuild per-MCU
- **JPEG header parser bounds**: `header.rs:parse()` has guards for `data.len() < 2` (empty input) and `end_pos < 10` (too short for SOF). Segment data end uses `.max(segment_data_start)` to prevent inverted slice ranges when `segment_len` is malformed
- **PrecomputedProtector auto-caching**: `apply()` auto-registers generated perturbations via `register_variant` on cache miss. This uses `clone()` on perturbation data — a known suboptimal allocation that could be eliminated by borrowing from the variant instead
- **`subtle` crate for constant-time comparison**: `verify_payload_mac` in `steganography.rs` uses `subtle::ConstantTimeEq::ct_eq()` instead of `==` to prevent timing attacks on HMAC verification
- **Pipeline intensity semantics**: `intensity` controls only the perturbation stage (noise/Enhanced/Precomputed). Steganography and metadata injection run regardless of intensity value. This is by design — they are orthogonal protection layers.
