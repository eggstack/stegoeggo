# Image Utilities

**Source:** `src/util/image.rs` (249 lines)

Core image processing utilities: PRNG, encoding, hashing, and format detection.

## PixelSelectionRng

General-purpose XorShift64 PRNG for pixel selection in steganography.

```rust
pub struct PixelSelectionRng { state: u64 }
```

- `new(seed: u64)` — Initializes with seed using `wrapping_add(XORSHIFT_SEED_OFFSET)` (not XOR)
- `next_u64()` — Returns random u64
- `gen_range_usize(range: Range<usize>)` — Returns usize in given range

## Other Utilities

- `compute_image_hash(img) -> String` — SHA-256 hex hash of RGBA pixel data
- `detect_image_format(bytes) -> Option<ImageFormat>` — PNG/JPEG/WebP detection
- `encode_image(img, format) -> Result<Vec<u8>>` — Encode to target format
- `encode_image_with_options(img, format: Option<ImageOutputFormat>, is_progressive: bool, quality: u8) -> Result<Vec<u8>>` — With JPEG options
- `load_image_from_bytes(bytes) -> Result<DynamicImage>` — Decode image bytes

## Module Interactions

- **protected/steganography/extract.rs**: Owns application extraction search and seed discovery; LSB carrier mechanics are delegated to `stegoeggo-stego`
- **protected/steganography/embed.rs**: Owns carrier selection and embedding dispatch; uses `stegoeggo-stego` for raw/in-place LSB and framed encoded-JPEG ops
- **util/seed.rs**: `generate_random_seed()` used for default context seeds
- **lib.rs**: Uses encoding/detection functions for format routing
