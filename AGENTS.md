# AGENTS.md

## Project Overview

`cloakrs` is a Rust library and CLI for protecting images from unauthorized AI model training through adversarial image poisoning. Designed for CDN/WAF edge deployment with sub-10ms latency targets.

## Tech Stack

- Rust (edition 2021, MSRV 1.87, stable channel)
- Key crates: `image` 0.25, `jpeg-encoder` 0.7, `rayon` 1.10, `sha2`/`hmac` for crypto, `serde`/`serde_json` for serialization, `tokio` (optional, for async)

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

## Build & Test Commands

```bash
cargo check                              # Compilation
cargo test                               # All tests (27 unit + 20 basic + 51 integration)
cargo test --all-features                # Includes async tests (9 tests)
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

## Things to Watch Out For

- **`.gitignore`**: `.DS_Store` files exist on disk but are excluded by git
- **Stego payload format**: 24-byte header + 2-byte checksum (or 8-byte HMAC) = 26 bytes minimum. Use `MIN_PAYLOAD_SIZE` and `MIN_PAYLOAD_BITS` constants in `steganography.rs`
- **`generate_random_seed()`**: Not cryptographically secure — uses SystemTime + splitmix64. Document this if changed
- **JPEG transcoder modules**: `header.rs` and `entropy.rs` have `#![allow(dead_code)]` for JPEG spec reference types (color spaces, standard Huffman tables) — keep these
- **ISCC module** (`src/util/iscc.rs`): Critical component for content identification, exported from `lib.rs`
- **No `TargetModel`**: This concept was removed. `ProtectionContext::new` takes `(intensity, seed)` only
- **Pre-existing uncommitted changes**: The working tree has modifications from a v0.2.0 refactor beyond this session's changes — check `git diff` before committing
