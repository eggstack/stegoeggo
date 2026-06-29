---
name: stegoeggo-conventions
description: Use when writing, modifying, or reviewing Rust code in the stegoeggo codebase. Triggers on tasks like "write tests", "add feature", "fix bug", "refactor", or any code change in src/. Covers code style, patterns, and pitfalls specific to this project.
---

# Stegoeggo Code Conventions

## Formatting
- Rustfmt: 4-space indentation, max width 100
- Run `cargo fmt --check` before committing
- Run `cargo clippy --all-targets -- -D warnings` before committing

## Code Style
- No comments in code unless explicitly asked by user
- `#[must_use]` on all builder methods
- `pub(crate)` for internal modules (jpeg_transcoder, protected, util)
- `LazyLock` for static singletons (e.g., `DEFAULT_PIPELINE`)
- `Arc<ProtectionConfig>` for shared heavy config fields
- Private fields with getter methods on public types

## Type Patterns

### ProtectionContext
- All fields private — use builder methods or `new(intensity, seed)`
- `inject_metadata: Option<bool>` — `None` means use level default
- `inject_legal_claims: Option<bool>` — `None` means use level default
- `config: Option<Arc<ProtectionConfig>>` — `#[serde(skip)]`

### StegoPayload
- `protection_level()` returns `u8`, not `ProtectionLevel`
- All fields private — use getters only

### ProtectionConfig
- `mac_key: Option<Vec<u8>>` (not `Vec<u8>`)

## Function Signatures

### Steganography methods (on SteganographyProtector)
```rust
// All are &self methods, NOT free functions
fn extract_payload(&self, img: &DynamicImage) -> Option<StegoPayload>
fn verify_payload(&self, img: &DynamicImage) -> bool
fn verify_payload_with_key(&self, img: &DynamicImage, mac_key: &[u8]) -> Option<bool>
fn verify_payload_from_bytes(&self, img_bytes: &[u8], seed: u64) -> bool
fn verify_payload_from_bytes_with_key(&self, img_bytes: &[u8], mac_key: &[u8]) -> Option<bool>
```

### JPEG transcoder
```rust
// Private, returns Result
fn assemble_jpeg(header: &JpegHeader, scan_data: &[u8]) -> Result<Vec<u8>>
// Returns Option, not Result
fn get_scan_data_start(...) -> Option<usize>
```

## Constants
- `MIN_PAYLOAD_SIZE = 28` (24-byte header + 4-byte CRC32; parsing threshold, not output size)
- `CURRENT_PAYLOAD_VERSION = 2` (V2 header is 32 bytes; V1 is 24 bytes, still supported for extraction)
- `STEGO_SPREAD_FACTOR = 5` (adjacent pixels per LSB bit)
- `estimated_latency_ms()` returns `u32` (not `f64`)

## Common Pitfalls

1. **Two XorShiftRng implementations** — `XorShiftRng` in `util/image.rs` and `F5XorShiftRng` in `stego_f5.rs` use different algorithms. Never interchange.
2. **Metadata injection survives only in byte paths** — `MetadataTrapProtector::apply()` returns `Cow::Borrowed` unchanged. Use `apply_bytes()` or `process_bytes()` for metadata.
3. **Stego seed derivation** — embed/extract functions internally derive `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)`. Match seeds when calling directly.
4. **`subtle` crate** — use `ConstantTimeEq::ct_eq()` for HMAC verification, not `==`
5. **F5 seed embedding** — Precondition check fails if any quantization value < 2. Values of 1 cannot represent 0-bits reliably. Use values >= 2.
6. **ISCC is not standard-compliant** — uses custom component codes (`0x12`, `0x33`), not interoperable with other ISCC implementations.
7. **V2 payload format** — 32-byte header (version, level, seed, intensity, timestamp, content_hash, dmi, flags, reserved). V1 (24-byte) still supported for extraction. MAC payloads: 40 bytes (32 header + 8 HMAC). ECC payloads: 100 bytes (32 × 3 + 4 CRC32).

## Build & Test
```bash
cargo check                              # Quick compilation check
cargo test                               # All tests (~245 total)
cargo test --all-features                # Includes async tests
cargo clippy --all-targets -- -D warnings # Lint
cargo fmt --check                        # Format check
```

## Testing Patterns
- Unit tests live in each source file as `#[cfg(test)] mod tests`
- Integration tests in `tests/` directory
- Test with `ProtectionContext::new(intensity, seed)` for deterministic results
- `ProtectionContext::default()` uses CSPRNG-backed seed (via `getrandom`) — safe for production; use `ProtectionContext::new(intensity, seed)` for reproducibility
