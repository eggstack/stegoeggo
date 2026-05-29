# Enhanced Protector

**Source:** `src/protected/enhanced.rs` (~79 lines)

Higher-intensity noise protector for the `Enhanced` protection level. Estimated latency: 5ms.

## Implementation

```rust
pub struct EnhancedProtector {
    inner: NoiseProtector,
}
```

Wraps `NoiseProtector::enhanced()` which uses a 12x intensity multiplier (vs 10x for Standard).

## Design

The `EnhancedProtector` is a thin wrapper that delegates all `Protector` methods to its inner `NoiseProtector`:

```rust
impl Protector for EnhancedProtector {
    fn apply(&self, img: &DynamicImage, ctx: &ProtectionContext) -> Result<Cow<DynamicImage>> {
        self.inner.apply(img, ctx)
    }

    fn protection_level(&self) -> ProtectionLevel {
        ProtectionLevel::Enhanced
    }
    // ... other methods delegate to self.inner
}
```

## Why a Separate Type?

Although `EnhancedProtector` delegates to `NoiseProtector`, it exists as a separate type because:

1. **Pipeline routing** — `ProtectionPipeline` needs distinct protector instances per level
2. **Different protection level** — `protection_level()` returns `Enhanced`, not `Standard`
3. **Different latency estimate** — `estimated_latency_ms()` returns 5 vs 3
4. **Semantic clarity** — Clearly separates Standard from Enhanced in the codebase

## Module Interactions

- **lib.rs**: Selected for `ProtectionLevel::Enhanced`
- **protected/noise.rs**: Wraps `NoiseProtector::enhanced()` with 12x multiplier
