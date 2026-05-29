# Error Types

**Source:** `src/error.rs` (~69 lines)

Uses `thiserror` for ergonomic error derivation.

## Error Enum

```rust
pub enum Error {
    ImageDecode(String),
    ImageEncode(String),
    Io(std::io::Error),
    Serialization(String),
    VariantNotFound(String),
    InvalidVariant(String),
    Metadata(String),
    Config(String),
    Image(String),
    Steganography(String),
    JpegTranscode(String),
    HashError(String),
    InvalidFormat(String),
    Dimensions(String),
    PayloadVerification(String),
    Crypto(String),
    #[cfg(feature = "async")]
    Task(String),
}
```

## Variants

| Variant | Source | Description |
|---------|--------|-------------|
| `ImageDecode` | `image` crate | Failed to decode image bytes |
| `ImageEncode` | `image`/`jpeg-encoder` | Failed to encode image |
| `Io` | `std::io` | File I/O errors |
| `Serialization` | `serde_json` | JSON serialization/deserialization failures |
| `VariantNotFound` | `PrecomputedProtector` | Requested variant not in cache or loader |
| `InvalidVariant` | `PrecomputedProtector` | Variant data is corrupted or incompatible |
| `Metadata` | `MetadataTrapProtector` | Metadata injection/extraction failures |
| `Config` | `ProtectionContext` | Invalid configuration values |
| `Image` | General | Image processing errors (dimensions, format, etc.) |
| `Steganography` | `SteganographyProtector` | Stego embed/extract failures |
| `JpegTranscode` | `jpeg_transcoder` | JPEG coefficient decode/encode failures |
| `HashError` | `util::image` | SHA-256 hashing failures |
| `InvalidFormat` | Pipeline | Input format cannot be determined |
| `Dimensions` | Pipeline | Image dimensions exceed limits |
| `PayloadVerification` | `SteganographyProtector` | HMAC/checksum verification failed |
| `Crypto` | `SteganographyProtector` | Cryptographic operation failures |
| `Task` | `async_api` | Tokio task join errors (async feature only) |

## Result Type

```rust
pub type Result<T> = std::result::Result<T, Error>;
```

## Design Notes

- All variants wrap `String` for simplicity (no lifetime issues)
- `Io` variant wraps `std::io::Error` directly for proper error chaining
- The `#[cfg(feature = "async")]` on `Task` avoids requiring tokio for non-async builds
- Error messages are descriptive enough for debugging but don't leak internal details
