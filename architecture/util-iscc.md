# ISCC Content Identifiers

**Source:** `src/util/iscc.rs` (~308 lines)

Computes ISCC (Immutable Self-Certifying Constituent Content) identifiers for images. Used for content identification, deduplication, and provenance tracking.

## Iscc Struct

```rust
pub struct Iscc {
    pub meta: Option<String>, // Component code (currently always None)
    pub content: String,      // DCT perceptual hash (base58)
    pub data: String,         // SHA-256 data hash (base58)
    pub instance: String,     // Instance hash (base58)
    pub full: String,         // Combined ISCC code
}
```

## Algorithm

1. **Normalize** image to 32×32 grayscale
2. **Compute 2D DCT** (Discrete Cosine Transform) on the normalized image
3. **Extract perceptual hash** from DCT coefficients using median-based bit pattern:
   - Compute median of low-frequency DCT coefficients in four quadrants
   - Each bit = 1 if coefficient > median, else 0
   - Produces a 256-bit perceptual hash, truncated to 8 bytes (64 bits) for the content component
4. **Compute data hash** — SHA-256 of raw RGBA bytes, truncated to 8 bytes for the data component
5. **Compute instance hash** — identical to data hash (both are SHA-256 of raw RGBA bytes)
6. **Encode** all components in base58

## Standard Compliance

**This is NOT standard-compliant ISCC.** Uses custom component codes (`0x12` for content, `0x33` for data) and a non-standard DCT hash. Produces ISCC-like identifiers that are not interoperable with other ISCC implementations.

## Functions

```rust
pub fn compute_iscc(img: &DynamicImage) -> Iscc
pub fn compute_iscc_from_bytes(bytes: &[u8]) -> Option<Iscc>
```

## Use Cases

- Content deduplication across CDN edges
- Provenance tracking for protected images
- Perceptual similarity detection (DCT hash is robust to minor modifications)

## Module Interactions

- **lib.rs**: `compute_iscc` and `compute_iscc_from_bytes` are re-exported as public API
- Not used in the protection hot path — this is an out-of-band utility
