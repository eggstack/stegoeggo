# Async API

**Source:** `src/async_api.rs`

Behind the `async` feature flag. Provides tokio-based async wrappers using `spawn_blocking`.

## Canonical functions

```rust
pub async fn process_request_bytes_async(img_bytes: Vec<u8>, request: ProtectionRequest) -> Result<Vec<u8>>
pub async fn process_request_bytes_with_warnings_async(img_bytes: Vec<u8>, request: ProtectionRequest) -> Result<(Vec<u8>, Vec<ProtectionWarning>)>
pub async fn process_request_bytes_with_report_async(img_bytes: Vec<u8>, request: ProtectionRequest) -> Result<(Vec<u8>, ExecutionReport)>
pub async fn process_request_bytes_parallel_async(images: Vec<Vec<u8>>, request: ProtectionRequest) -> Result<Vec<Vec<u8>>>  // requires `parallel` feature
pub async fn process_request_bytes_with_warnings_parallel_async(images: Vec<Vec<u8>>, request: ProtectionRequest) -> Result<Vec<(Vec<u8>, Vec<ProtectionWarning>)>>  // requires `parallel` feature
pub async fn process_request_bytes_with_report_parallel_async(images: Vec<Vec<u8>>, request: ProtectionRequest) -> Result<Vec<(Vec<u8>, ExecutionReport)>>  // requires `parallel` feature
pub async fn verify_image_bytes_async(img_bytes: Vec<u8>, mac_key: Vec<u8>) -> Result<VerificationStatus>
```

Each request-based function calls its synchronous canonical counterpart
(`process_request_bytes`, `process_request_bytes_with_warnings`,
`process_request_bytes_with_report`, or the Rayon batch variants) inside one
`spawn_blocking` closure. There is no policy, routing, validation, or warning
duplication in `async_api.rs`.

## Compatibility functions

```rust
pub async fn process_image_async(img: DynamicImage, level: ProtectionLevel, ctx: ProtectionContext) -> Result<DynamicImage>
pub async fn process_image_bytes_async(img_bytes: Vec<u8>, level: ProtectionLevel, ctx: ProtectionContext) -> Result<Vec<u8>>
pub async fn process_image_bytes_with_warnings_async(img_bytes: Vec<u8>, level: ProtectionLevel, ctx: ProtectionContext) -> Result<(Vec<u8>, Vec<ProtectionWarning>)>
pub async fn process_images_parallel_async(images: Vec<DynamicImage>, level: ProtectionLevel, ctx: ProtectionContext) -> Result<Vec<DynamicImage>>  // requires `parallel` feature
pub async fn process_images_bytes_parallel_async(images: Vec<Vec<u8>>, level: ProtectionLevel, ctx: ProtectionContext) -> Result<Vec<Vec<u8>>>  // requires `parallel` feature
```

These translate once through the legacy sync path (which itself translates to
`ProtectionRequest`) and remain functional for 0.x callers. New code must use
the request-based forms.

## Design Decisions

### Pixel-only contract

`process_image_async` and `process_images_parallel_async` operate on `DynamicImage` and do not preserve file-level metadata (PNG tEXt, JPEG COM/XMP, WebP XMP). For full metadata injection, use the byte-path async functions.

### Batch functions

`process_request_bytes_parallel_async` (and its warnings/report variants) plus
the legacy `process_images_parallel_async` /
`process_images_bytes_parallel_async` run the **entire batch** inside a single
`spawn_blocking`. This delegates to the synchronous rayon-based parallel
functions, avoiding per-image `spawn_blocking` calls that would cause thread
pool overlap — rayon already manages its own thread pool for parallelism.
Output order matches input order.

### Single-image functions

Request-based single-image functions use one `spawn_blocking` per image. For reverse proxies, prefer `process_request_bytes_with_warnings_async` so the proxy can log or enforce degraded protection states before serving.

## Ownership

Async functions take owned types (`Vec<u8>`, `DynamicImage`, `ProtectionRequest`, `ProtectionContext`) rather than references, since `spawn_blocking` requires `'static` futures.

## Module Interactions

- **lib.rs**: Request-based async functions delegate to the synchronous `process_request_bytes`, `process_request_bytes_with_warnings`, `process_request_bytes_with_report`, `process_request_bytes_parallel`, `process_request_bytes_with_warnings_parallel`, `process_request_bytes_with_report_parallel`, and `verify_image_bytes` functions
- **Error mapping**: `tokio::task::JoinError` is mapped to `Error::Task`
