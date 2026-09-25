# Image Utilities

**Source:** `src/util/image.rs` (328 lines)

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
- `detect_image_format(bytes) -> Option<ImageFormat>` — PNG/JPEG/WebP detection via `ImageOutputFormat::from_magic_bytes`
- `encode_image(img, format) -> Result<Vec<u8>>` — Encode to target format (JPEG at quality 90; PNG/WebP lossless)
- `encode_image_with_quality(img, format, quality) -> Result<Vec<u8>>` — Core encoder; quality affects JPEG only
- `encode_image_with_options(img, format: Option<ImageOutputFormat>, is_progressive: bool, quality: u8) -> Result<Vec<u8>>` — With JPEG options; `None` format falls back to `DEFAULT_OUTPUT_FORMAT`
- `load_image_from_bytes(bytes) -> Result<DynamicImage>` — Decode image bytes via `image::load_from_memory`

JPEG encoding rejects zero or over-`u16::MAX` dimensions with `Error::DimensionsExceeded` (plus the default `ResourceLimits::check_dimensions`), and caps the initial buffer reservation at 8 MiB. Test builds count full decodes via a thread-local counter (`reset_load_decode_count` / `load_decode_count`, `pub(crate)`).

## Module Interactions

- **protected/steganography/extract.rs**: Owns application extraction search and seed discovery; LSB carrier mechanics are delegated to `stegoeggo-stego`
- **protected/steganography/embed.rs**: Owns carrier selection and embedding dispatch; uses `stegoeggo-stego` for raw/in-place LSB and framed encoded-JPEG ops
- **util/seed.rs**: `generate_random_seed()` used for default context seeds
- **lib.rs**: Uses encoding/detection functions for format routing
