# Precomputed Protector

**Source:** `src/protected/precomputed.rs` (~322 lines)

CDN/WAF edge deployment protector for the `Strong` protection level. Pre-generates and caches perturbation data for instant application.

## Architecture

```
PrecomputedProtector
├── In-memory cache: RwLock<HashMap<String, ProtectedVariant>>
└── Optional persistent storage: Box<dyn VariantLoader>
```

## How It Works

### On Cache Hit

1. Look up variant by cache key (derived from image hash + intensity)
2. Apply precomputed perturbation via `apply_perturbation[_par]`
3. No generation overhead — sub-millisecond

### On Cache Miss

1. Generate perturbation data via `generate_perturbation_data`
2. Auto-register the variant (best-effort caching)
3. Apply the generated perturbation
4. Return result

### Registration (Two-Phase)

```rust
pub fn register_variant(&self, variant: ProtectedVariant) -> Result<()> {
    let key = variant.cache_key();
    // Phase 1: Persist to loader without holding lock
    if let Some(ref loader) = self.loader {
        loader.store_variant(&variant)?;  // Propagates errors
    }

    // Phase 2: Insert into in-memory cache with write lock
    let mut variants = self.variants.write().map_err(...)?;
    variants.insert(key, variant);
    Ok(())
}
```

This design avoids holding locks during I/O operations.

> **Warning:** The in-memory cache (`RwLock<HashMap<String, ProtectedVariant>>`) has no eviction policy, size limit, or TTL. Under sustained load, the cache will grow without bound.

## Key Functions

```rust
pub fn generate_perturbation_data(&self, width: u32, height: u32, ctx: &ProtectionContext) -> Result<Vec<u8>>
```

Creates an RGBA perturbation buffer (4 bytes per pixel) without applying it to an image. The buffer can be stored and applied later.

```rust
pub fn register_variants(&self, variants: Vec<ProtectedVariant>)
```

Batch registration for multiple variants.

### Apply Behavior

- On cache miss: generates, auto-registers (best-effort), applies, returns `Cow::Owned`
- On cache hit: applies precomputed data, returns `Cow::Owned`
- Registration failure is silently ignored (best-effort caching) via `let _ = self.register_variant(variant)`

## ProtectedVariant Storage

Each variant contains:
- `uuid` — Unique identifier
- `original_hash` — SHA-256 of original image pixels
- `perturbation_data` — RGBA perturbation bytes
- `intensity` — The intensity used to generate the perturbation
- `width`, `height` — Image dimensions

Cache key format: `{hash}_{level}_{intensity}`

## Module Interactions

- **lib.rs**: Selected for `ProtectionLevel::Strong`. `register_precomputed_variants` on pipeline
- **traits.rs**: Uses `VariantLoader` trait for persistent storage
- **util/image.rs**: Calls `apply_perturbation[_par]` for application, `generate_perturbation_data` for creation
- **types.rs**: Uses `ProtectedVariant` for storage
