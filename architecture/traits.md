# Traits

**Source:** `src/traits.rs` (~154 lines)

Defines the core trait contracts that all protectors implement.

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

- **`apply`** — Core protection method. Returns `Result<Cow<DynamicImage>>`. Returns `Cow::Borrowed(img)` when no modification needed (avoids cloning). Returns `Cow::Owned(DynamicImage)` when pixels are modified.
- **`apply_bytes`** — Byte-level processing. Returns `Result<Vec<u8>>`. Default implementation decodes bytes → calls `apply` → re-encodes. Overrides exist for JPEG fast path and metadata injection.
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
| `SteganographyProtector` | Standard | true | 2 |

## Module Interactions

- **lib.rs**: Calls `Protector::apply()` and `Protector::apply_bytes()` for each protection level
- **protected/*.rs**: Each protector implements the `Protector` trait
