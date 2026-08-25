# ISCC Content Identifiers

**Source:** `src/util/iscc.rs` (371 lines) (feature-gated: `iscc`)

Computes ISCC (International Standard Content Code) identifiers for images. Used for content identification, deduplication, and provenance tracking. Requires the `iscc` feature to compile.

## ContentIdentifiers Struct

```rust
pub struct ContentIdentifiers {
    meta: Option<String>,   // Meta code (present when legal metadata supplied)
    content: String,        // Content code — perceptual image hash (base58)
    data: String,           // Data code (base58, same value as instance)
    instance: String,       // Instance code — exact byte hash (base58)
    full: String,           // Combined ISCC URI
}
```

The `Iscc` type alias is deprecated since 0.4.0 in favor of `ContentIdentifiers`.

## Algorithm

Delegates to the `iscc-lib` crate (dep `iscc-lib 0.4`) for standard ISCC v0 code generation:

1. **Normalize** image to 32×32 grayscale (Lanczos3 resampling)
2. **Content code** — `iscc_lib::gen_image_code_v0(&pixels, 256)` produces a 256-bit CONTENT-IMAGE code from the grayscale pixel data
3. **Instance code** — `iscc_lib::gen_instance_code_v0(&raw_bytes, 256)` produces a 256-bit INSTANCE code from the raw RGBA bytes
4. **Meta code** (optional) — `iscc_lib::gen_meta_code_v0(name, description, meta_payload, 256)` produces a 256-bit META code when `LegalMetadata` is provided via `from_image_with_metadata()`
5. **Full URI** — Components assembled as `ISCC:AA...+EE...+II...` (with meta) or `ISCC:EE...+II...` (without)

The `data` and `instance` fields carry the same value (the instance code). All component codes use standard ISCC type prefixes (AA for META, EE for CONTENT-IMAGE, etc.).

## Standard Compliance

Uses standard ISCC v0 codes via `iscc-lib`. Component codes are interoperable with other ISCC implementations. However, the library's own documentation notes these are "ISCC-like" identifiers intended for in-application deduplication and provenance tracking.

## Functions

```rust
pub fn compute_content_identifiers(img: &DynamicImage) -> Result<ContentIdentifiers>
pub fn compute_content_identifiers_with_metadata(img: &DynamicImage, legal_metadata: &LegalMetadata) -> Result<ContentIdentifiers>
pub fn compute_content_identifiers_from_bytes(bytes: &[u8]) -> Option<Result<ContentIdentifiers>>
pub fn compute_content_identifiers_from_bytes_with_metadata(bytes: &[u8], legal_metadata: &LegalMetadata) -> Option<Result<ContentIdentifiers>>
```

Deprecated aliases (`compute_iscc`, `compute_iscc_from_bytes`, etc.) are available since 0.4.0.

## Use Cases

- Content deduplication across CDN edges
- Provenance tracking for protected images
- Perceptual similarity detection (image code is robust to minor modifications)

## Module Interactions

- **lib.rs**: Functions are re-exported as public API
- Not used in the protection hot path — this is an out-of-band utility
