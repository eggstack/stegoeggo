# Traits

**Source:** `src/traits.rs` (~154 lines)

Defines the core trait contracts that all protectors and storage backends implement.

## Protector Trait

```rust
pub trait Protector: Send + Sync {
    fn apply(&self, img: &DynamicImage, ctx: &ProtectionContext) -> Result<Cow<DynamicImage>>;
    fn apply_bytes(&self, img_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>>;
    fn name(&self) -> &str;
    fn protection_level(&self) -> ProtectionLevel;
    fn estimated_latency_ms(&self) -> u32;
    fn modifies_pixels(&self) -> bool;
    fn is_enabled(&self) -> bool;
}
```

### Methods

- **`apply`** — Core protection method. Returns `Cow::Borrowed(img)` when no modification needed (avoids cloning). Returns `Cow::Owned(DynamicImage)` when pixels are modified.
- **`apply_bytes`** — Byte-level processing. Default implementation decodes bytes → calls `apply` → re-encodes. Overrides exist for JPEG fast path and metadata injection.
- **`name`** — Human-readable name for logging/debugging.
- **`protection_level`** — Which `ProtectionLevel` this protector handles.
- **`estimated_latency_ms`** — Expected processing time for performance budgets.
- **`modifies_pixels`** — Whether this protector changes pixel data (metadata-only protectors return false).
- **`is_enabled`** — Whether this protector is active. Default returns `true`. `PassthroughProtector` overrides to return `true`. Note: this method is dead code — the pipeline never calls it (uses direct `match level` dispatch instead).

### Implementations

| Protector | Level | modifies_pixels | estimated_latency_ms |
|-----------|-------|-----------------|---------------------|
| `PassthroughProtector` | Disabled | false | 0 |
| `MetadataTrapProtector` | Light | false | 2 |
| `NoiseProtector` | Standard | true | 3 |
| `EnhancedProtector` | Enhanced | true | 5 |
| `PrecomputedProtector` | Strong | true | 2 |
| `SteganographyProtector` | Standard | true | 2 |

## VariantLoader Trait

Persistent storage backend for `PrecomputedProtector`:

```rust
pub trait VariantLoader: Send + Sync {
    fn load_variant(&self, key: &str) -> Result<Option<ProtectedVariant>>;
    fn store_variant(&self, variant: &ProtectedVariant) -> Result<()>;
}
```

### Implementations

- **`NoOpLoader`** — No-op implementation. `load_variant` always returns `Ok(None)`, `store_variant` returns `Ok(())`.
- Users implement this trait for persistent storage (Redis, filesystem, database, etc.)

### Usage in PrecomputedProtector

The `PrecomputedProtector` uses a two-phase registration:
1. Persist to `VariantLoader` without holding the write lock
2. Insert into in-memory `RwLock<HashMap>` with write lock

This avoids holding locks during I/O operations.

## Module Interactions

- **lib.rs**: Calls `Protector::apply()` and `Protector::apply_bytes()` for each protection level
- **protected/*.rs**: Each protector implements the `Protector` trait
- **protected/precomputed.rs**: Uses `VariantLoader` for persistent variant storage
