# AGENTS.md

## Project Overview

`cloakrs` is a Rust library and CLI for protecting images from unauthorized AI use through steganographic watermarking and metadata injection for legal deterrence.

## Tech Stack

- Rust (edition 2021, MSRV 1.87, stable channel)
- Key crates: `image` 0.25, `jpeg-encoder` 0.7, `rayon` 1.10, `sha2`/`hmac` for crypto, `serde`/`serde_json` for serialization, `subtle` 2 (constant-time comparisons), `iscc-lib` 0.4 (ISO 24138:2024 ISCC), `getrandom` 0.2 (CSPRNG), `tokio` (optional, for async)

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
│   └── steganography.rs   # LSB/DCT steganographic embedding
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
- All struct fields on `ProtectionContext` and `StegoPayload` are private — use getter methods (e.g., `ctx.intensity()`, `ctx.seed()`)
- `ProtectionContext` has `set_input_format()` (public) and `set_protection_level()` (crate-internal) for non-builder mutation

## Architecture Documentation

Architecture docs live in `architecture/` (21 files). All docs have been verified against source code.

## Build & Test Commands

```bash
cargo check                              # Compilation
cargo test                               # All tests (168 unit + 20 basic + 63 integration)
cargo test --all-features                # Includes async tests (9 tests) — 264+ total
cargo clippy --all-targets -- -D warnings # Lint check
cargo fmt --check                        # Format check
cargo bench                              # Criterion benchmarks
```

## Code Conventions

- Rustfmt: 4-space indentation, max width 100
- No comments in code unless explicitly asked
- `#[must_use]` on builder methods
- `pub(crate)` for internal modules (e.g., `jpeg_transcoder`)
- Private fields with getter methods on `ProtectionContext`, `StegoPayload`, `LegalMetadata`

## Things to Watch Out For

- **`.gitignore`**: `.DS_Store` files exist on disk but are excluded by git
- **Stego payload format**: 24-byte header + 2-byte checksum (or 8-byte HMAC), always padded to 32 bytes total (even in checksum mode). Use `MIN_PAYLOAD_SIZE` (=26) and `MIN_PAYLOAD_BITS` (=208) constants in `steganography.rs`
- **`generate_random_seed()`**: Uses `getrandom` (OS CSPRNG) for cryptographically secure randomness. Falls back to system-time-based mixing if `getrandom` fails. Guarantees non-zero output (`if x == 0 { 42 }`)
- **`ProtectionContext::default()`**: Calls `generate_random_seed()` — the seed is predictable. Doc comment on the `Default` impl warns about this. Users needing cryptographic seeds should use `ProtectionContext::new(intensity, seed)` with a CSPRNG
- **JPEG transcoder modules**: `header.rs` and `entropy.rs` have `#![allow(dead_code)]` for JPEG spec reference types (color spaces, standard Huffman tables) — keep these
- **ISCC module** (`src/util/iscc.rs`): Content identification, exported from `lib.rs`. Uses `iscc-lib` crate (ISO 24138:2024) for standard-compliant ISCC generation. Produces interoperable ISCC identifiers with proper varnibble headers and Base32 encoding
- **No `TargetModel`**: This concept was removed. `ProtectionContext::new` takes `(intensity, seed)` only
- **Steganography seed derivation**: `embed_lsb` and `embed_jpeg_stego` internally derive `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)` before using `stego_permutation`. When calling internal embed/extract functions directly (outside of `apply()`), ensure seeds match. The public API (`apply()` + `extract_payload()`) handles this correctly
- **XorShiftRng — two separate implementations**: `src/util/image.rs` has `PixelSelectionRng` (general-purpose pixel selection for steganography) and `src/jpeg_transcoder/stego_f5.rs` has `DctCoefficientRng` (DCT coefficient shuffling). They use different algorithms and produce different sequences for the same seed. Do NOT interchange them — each is paired with their respective embed/extract code paths
- **MetadataTrapProtector::apply()**: `apply()` returns `Cow::Borrowed(img)` unchanged. Metadata injection operates at the byte level and the `DynamicImage` API cannot preserve injected text chunks through encode/decode cycles. The pipeline routes `Light` level through `apply_light_bytes()` which encodes, injects metadata, then decodes (metadata survives in the byte output). For byte-level output with metadata intact, use `apply_bytes()` or `process_bytes()`
- **`stego_redundancy` is `Option<usize>`**: Default is `None` (derive from intensity). Explicit `.with_stego_redundancy(n)` sets `Some(n)`. Internal code uses `ctx.effective_redundancy()` which maps intensity to redundancy when not explicitly set. The public `stego_redundancy()` getter returns `unwrap_or(2)` for backward compatibility.
- **`verify_payload_with_key()` returns `Option<bool>`**, not `bool` — use `== Some(true)` or `!= Some(true)` in assertions, not `assert!()` or `assert!(!)` directly
- **JPEG stego redundancy bug** (`steganography.rs:788-841`): Fixed — `embedded = 0;` reset added between redundancy passes so all passes embed when `redundancy > 1`
- **HuffmanDecoder/HuffmanEncoderTable caching**: In `entropy.rs`, Huffman decoders and encoder lookup tables are pre-built once before the MCU loop. `HuffmanEncoderTable` uses a `[(u16, u8); 256]` array for O(1) symbol→(code,length) lookup. These are internal types — do not rebuild per-MCU
- **JPEG header parser bounds**: `header.rs:parse()` has guards for `data.len() < 2` (empty input) and `end_pos < 10` (too short for SOF). Segment data end uses `.max(segment_data_start)` to prevent inverted slice ranges when `segment_len` is malformed. Parse errors now include byte offset for debugging
- **`subtle` crate for constant-time comparison**: `verify_payload_mac` in `steganography.rs` uses `subtle::ConstantTimeEq::ct_eq()` instead of `==` to prevent timing attacks on HMAC verification
- **Pipeline intensity semantics**: Steganography and metadata injection run regardless of intensity value. When `stego_redundancy` is not explicitly set, `intensity` controls redundancy via `effective_redundancy()` (<0.3→1, 0.3-0.7→2, >=0.7→3). Explicit `with_stego_redundancy()` overrides this.
- **F5 no-zero variant correctness**: `stego_f5.rs` implements a no-zero F5 variant that increments |coef| when |coef|=1 and LSB mismatches, avoiding detectable zero creation. The embed/extract position alignment is preserved because no coefficient is ever zeroed out. The implementation is correct
- **F5 seed embedding Q-table edge case**: `stego_f5.rs` has a precondition check in `embed_seed_in_quantization_tables()` that fails if any quantization value in the first 2 tables is < 2. Values of 1 cannot represent a 0-bit (`1 & 0xFE = 0`, clamped back to 1). Use quantization values >= 2 for reliable seed embedding
- **`#[serde(skip)]` on `config` field**: `ProtectionContext.config` (`Option<Arc<ProtectionConfig>>`) is skipped during serialization. MAC keys and legal metadata are lost in serde roundtrips. A test (`test_config_skipped_in_serde_roundtrip`) documents this behavior
- **Async batch processing**: `process_images_parallel_async` and `process_images_bytes_parallel_async` run the entire batch inside a single `spawn_blocking`, delegating to the synchronous rayon-based parallel functions. This avoids per-image `spawn_blocking` calls that would cause thread pool overlap. Single-image async functions (`process_image_async`, `process_image_bytes_async`) still use one `spawn_blocking` per image
- **Steganography fallback seeds**: Extracted to `FALLBACK_SEEDS` constant in `steganography.rs`. These are common test/dev seeds tried when metadata is stripped
- **Format detection strictness**: `apply_multi_protector_bytes` returns `Error::InvalidFormat` when the input format cannot be determined (from `ctx.input_format()` or magic bytes). Previously it silently defaulted to PNG
- **JPEG segment length bounds**: `get_scan_data_start` now uses `checked_add` to prevent integer overflow when advancing past segments with malformed lengths
- **Entropy decoder natural order**: The entropy decoder stores coefficients directly in natural (row-major) order via `block[ZIGZAG[k]] = magnitude`. The old reorder loop was redundant (identity operation) and has been removed
- **JPEG quantization debug assertion**: `assemble_jpeg` has a `debug_assert!` for 8-bit quantization values exceeding 255
- **`inject_metadata` / `inject_legal_claims` are `Option<bool>`**: Default is `None`, not `true`/`false`. The `None` semantics (use level default) vs explicit `false` (disable) are well-documented in `types.rs`. `with_metadata_injection(false)` and not calling it at all have different effects
- **`process_bytes` dimension validation**: `process_bytes()` now validates `max_dimension` for both JPEG (via header parse) and non-JPEG paths. Previously only `process()` enforced this
- **CLI batch filename collisions**: Batch mode in `cloakrs-cli` detects duplicate output stems and appends `_protected_1`, `_protected_2`, etc. to prevent silent overwrites
- **Pipeline flow order (JPEG output)**: For JPEG output, the pipeline applies encode → DCT stego → metadata (encode happens before stego). For non-JPEG output: pixel stego → encode → metadata. The JPEG→JPEG fast path bypasses pixel decode entirely and only applies DCT stego + metadata
- **CLI file path**: The CLI binary lives at `cloakrs-cli/src/main.rs`, not `src/bin/cloakrs/main.rs`
- **MIN_PAYLOAD_SIZE vs actual payload size**: `MIN_PAYLOAD_SIZE` (=26) is the minimum valid payload for parsing (24-byte header + 2-byte checksum). `generate_payload()` always produces 32 bytes (padded with zeros). These are different numbers — the constant is a parsing threshold, not the output size
- **`PassthroughProtector::modifies_pixels()`**: Returns `false` (passthrough.rs:50-52) for efficiency. The default `Protector` trait implementation returns `true`, so other protectors that don't modify pixels should also override this
- **`ImageTruncated` error variant**: Added to `error.rs` for handling truncated JPEG parsing. Used in `metadata_trap.rs` when JPEG segment parsing encounters truncated data.
- **`extract_seed_from_jpeg` returns `Option<u64>`**: Returns `None` on truncated data or when no seed is found. Callers receive `Option` directly without needing `.ok()`.
- **`bits_to_bytes` runtime check**: Now validates `bits.len() % 8 != 0` at runtime instead of only in debug builds. Returns empty Vec for invalid input.
