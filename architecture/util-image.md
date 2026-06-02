# Image Utilities

**Source:** `src/util/image.rs` (~221 lines)

Core image processing utilities: PRNG, encoding, hashing, and format detection.

## XorShiftRng

General-purpose XorShift64 PRNG for pixel selection in steganography.

```rust
pub struct XorShiftRng { state: u64 }
```

- `new(seed: u64)` — Initializes with seed using `wrapping_add(XORSHIFT_SEED_OFFSET)` (not XOR)
- `next_u64()` — Returns random u64
- `gen_f32()` — Returns random f32 in `[-1.0, 1.0)`
- `gen_range(range: Range<f32>)` — Returns f32 in given range
- `gen_range_usize(range: Range<usize>)` — Returns usize in given range

## Other Utilities

- `compute_image_hash(img) -> String` — SHA-256 hex hash of RGBA pixel data
- `detect_image_format(bytes) -> Option<ImageOutputFormat>` — PNG/JPEG/WebP detection
- `encode_image(img, format) -> Vec<u8>` — Encode to target format
- `encode_image_with_options(img, format, progressive, quality) -> Vec<u8>` — With JPEG options
- `load_image_from_bytes(bytes) -> Result<DynamicImage>` — Decode image bytes
- `parallel_threshold() -> usize` — Returns `cores * 64 * 64` (scales with rayon thread count)

## Module Interactions

- **protected/steganography.rs**: Uses `XorShiftRng` for pixel selection (LSB stego)
- **util/seed.rs**: `generate_random_seed()` used for default context seeds
- **lib.rs**: Uses encoding/detection functions for format routing
