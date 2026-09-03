# Error Types

**Source:** `src/error.rs` (~347 lines)

Uses `thiserror` for ergonomic error derivation.

## Error Enum

19 total variants: 19 always-available + 1 async-only (`Task`).

```rust
#[non_exhaustive]
pub enum Error {
    ImageDecode(String),
    ImageEncode(String),
    Io(#[from] std::io::Error),
    Serialization(#[from] serde_json::Error),
    Metadata(String),
    Config(String),
    Image(#[from] ImageError),
    Steganography(String),
    InsufficientCapacity { required: usize, available: usize },
    InvalidFormat(String),
    ImageTruncated(String),
    PayloadVerification(String),
    Crypto(String),
    Iscc(String),
    InputTooLarge { size: usize, limit: usize },
    DimensionsExceeded { width: u32, height: u32, max_width: u32, max_height: u32 },
    ContainerLimitExceeded { kind: &'static str, count: usize, limit: usize },
    MetadataLimitExceeded { kind: &'static str, size: usize, limit: usize },
    VerificationBudgetExceeded { kind: &'static str, count: usize, limit: usize },
    #[cfg(feature = "async")]
    Task(String),
}
```

## Variants

| Variant | Source | Description |
|---------|--------|-------------|
| `ImageDecode` | `image` crate / `stegoeggo-stego::jpeg_transcoder` | Failed to decode image bytes or Huffman data |
| `ImageEncode` | `image`/`stegoeggo-stego::jpeg_transcoder` | Failed to encode image |
| `Io` | `std::io` | File I/O errors |
| `Serialization` | `serde_json` | JSON serialization/deserialization failures |
| `Metadata` | `MetadataTrapProtector` | Metadata injection/extraction failures |
| `Config` | `ProtectionContext` | Invalid configuration values |
| `Image` | General / `stegoeggo-stego::jpeg_transcoder` | Image processing errors (unsupported features, etc.) |
| `Steganography` | `SteganographyProtector` (and `stegoeggo-stego::StegoError` via `From`) | Stego embed/extract failures |
| `InsufficientCapacity` | `StegoError::InsufficientCapacity` via `From` | Carrier capacity failure with structured counts |
| `InvalidFormat` | Pipeline / `stegoeggo-stego::jpeg_transcoder` | Input format cannot be determined |
| `ImageTruncated` | Pipeline | Image data was truncated |
| `PayloadVerification` | `SteganographyProtector` | HMAC/checksum verification failed |
| `Crypto` | `SteganographyProtector` | Cryptographic operation failures |
| `Iscc` | `util/iscc.rs` | ISCC content identifier generation failures |
| `InputTooLarge` | Resource limits | Input image exceeds configured size limit |
| `DimensionsExceeded` | Resource limits | Image dimensions exceed configured maximum |
| `ContainerLimitExceeded` | Resource limits | Container (e.g., PNG chunks) exceeds count limit |
| `MetadataLimitExceeded` | Resource limits | Metadata section exceeds size limit |
| `VerificationBudgetExceeded` | Resource limits | Verification attempts exceed budget |
| `Task` | `async_api` | Tokio task join errors (async feature only) |

## Result Type

```rust
pub type Result<T> = std::result::Result<T, Error>;
```

## Design Notes

- All string variants wrap `String` for simplicity (no lifetime issues)
- `Io` variant wraps `std::io::Error` directly for proper error chaining
- The `#[cfg(feature = "async")]` on `Task` avoids requiring tokio for non-async builds
- Structured variants (`InputTooLarge`, `DimensionsExceeded`, etc.) carry typed fields for programmatic error handling
- Error messages are descriptive enough for debugging but don't leak internal details

## Generic Carrier Error Conversion

The public `stego` module exposes [`stegoeggo_stego::StegoError`] for generic carrier
operations. It has its own `From` conversions into the root `Error` enum so that
callers using a unified error type can treat generic carrier failures the same as
application-level failures:

```rust
impl From<StegoError> for Error {
    fn from(e: StegoError) -> Self { /* ... */ }
}
```

Similarly, `TranscoderError` from the carrier crate converts into `Error` via
`From` (see `src/error.rs`).
