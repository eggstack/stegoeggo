# Async API

**Source:** `src/async_api.rs` (~148 lines)

Behind the `async` feature flag. Provides tokio-based async wrappers using `spawn_blocking`.

## Functions

```rust
pub async fn process_image_async(img: DynamicImage, level: ProtectionLevel, ctx: ProtectionContext) -> Result<DynamicImage>
pub async fn process_image_bytes_async(img_bytes: Vec<u8>, level: ProtectionLevel, ctx: ProtectionContext) -> Result<Vec<u8>>
pub async fn process_images_parallel_async(images: Vec<DynamicImage>, level: ProtectionLevel, ctx: ProtectionContext) -> Result<Vec<DynamicImage>>
pub async fn process_images_bytes_parallel_async(images: Vec<Vec<u8>>, level: ProtectionLevel, ctx: ProtectionContext) -> Result<Vec<Vec<u8>>>
pub async fn verify_image_bytes_async(img_bytes: Vec<u8>, mac_key: Vec<u8>) -> Result<Option<bool>>
```

## Design Decisions

### Batch functions

`process_images_parallel_async` and `process_images_bytes_parallel_async` run the **entire batch** inside a single `spawn_blocking`. This delegates to the synchronous rayon-based parallel functions (`process_images_parallel` / `process_images_bytes_parallel`).

**Why:** Avoids per-image `spawn_blocking` calls that would cause thread pool overlap — rayon already manages its own thread pool for parallelism.

### Single-image functions

`process_image_async` and `process_image_bytes_async` use one `spawn_blocking` per image. This is appropriate for the WAF hot path where individual images arrive as separate requests.

## Ownership

Async functions take owned types (`Vec<u8>`, `DynamicImage`, `ProtectionContext`) rather than references, since `spawn_blocking` requires `'static` futures.

## Module Interactions

- **lib.rs**: Async functions delegate to the synchronous `process_image`, `process_image_bytes`, `process_images_parallel`, `process_images_bytes_parallel`, and `verify_image_bytes` functions
- **Error mapping**: `tokio::task::JoinError` is mapped to `Error::Task`
