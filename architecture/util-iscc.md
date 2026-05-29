# ISCC Content Identifiers

**Source:** `src/util/iscc.rs` (~202 lines)

Computes ISCC (Immutable Self-Certifying Constituent Content) identifiers for images. Used for content identification, deduplication, and provenance tracking.

## Iscc Struct

```rust
pub struct Iscc {
    pub meta: String,      // Component code
    pub content: String,   // DCT perceptual hash (base58)
    pub data: String,      // SHA-256 data hash (base58)
    pub instance: String,  // Instance hash (base58)
    pub full: String,      // Combined ISCC code
}
```

## Algorithm

1. **Normalize** image to 32×32 grayscale
2. **Compute 2D DCT** (Discrete Cosine Transform) on the normalized image
3. **Extract perceptual hash** from DCT coefficients using median-based bit pattern:
   - Compute median of low-frequency DCT coefficients
   - Each bit = 1 if coefficient > median, else 0
   - Produces a 64-bit perceptual hash
4. **Compute data hash** — SHA-256 of raw image bytes
5. **Compute instance hash** — SHA-256 of the normalized grayscale pixels
6. **Encode** all components in base58

## Functions

```rust
pub fn compute_iscc(img: &DynamicImage) -> Iscc
pub fn compute_iscc_from_bytes(bytes: &[u8]) -> Result<Iscc>
```

## Use Cases

- Content deduplication across CDN edges
- Provenance tracking for protected images
- Perceptual similarity detection (DCT hash is robust to minor modifications)

## Module Interactions

- **lib.rs**: `compute_iscc` and `compute_iscc_from_bytes` are re-exported as public API
- Not used in the protection hot path — this is an out-of-band utility
