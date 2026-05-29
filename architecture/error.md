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
    HashError(String),
    InvalidFormat(String),
    ImageTruncated(String),
    PayloadVerification(String),
    Crypto(String),
    #[cfg(feature = "async")]
    Task(String),
}
```

## Variants

| Variant | Source | Description |
|---------|--------|-------------|
| `ImageDecode` | `image` crate / `jpeg_transcoder` | Failed to decode image bytes or Huffman data |
| `ImageEncode` | `image`/`jpeg_transcoder` | Failed to encode image |
| `Io` | `std::io` | File I/O errors |
| `Serialization` | `serde_json` | JSON serialization/deserialization failures |
| `VariantNotFound` | `PrecomputedProtector` | Requested variant not in cache or loader |
| `InvalidVariant` | `PrecomputedProtector` | Variant data is corrupted or incompatible |
| `Metadata` | `MetadataTrapProtector` | Metadata injection/extraction failures |
| `Config` | `ProtectionContext` | Invalid configuration values |
| `Image` | General / `jpeg_transcoder` | Image processing errors (unsupported features, etc.) |
| `Steganography` | `SteganographyProtector` | Stego embed/extract failures |
| `HashError` | `util::image` | SHA-256 hashing failures |
| `InvalidFormat` | Pipeline / `jpeg_transcoder` | Input format cannot be determined |
| `ImageTruncated` | Pipeline | Image data was truncated |
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
