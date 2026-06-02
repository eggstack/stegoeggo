# Passthrough Protector

**Source:** `src/protected/passthrough.rs` (~94 lines)

No-op protector for the `Disabled` protection level.

## Implementation

```rust
pub struct PassthroughProtector;

impl Protector for PassthroughProtector {
    fn apply(&self, img: &DynamicImage, _ctx: &ProtectionContext) -> Result<Cow<DynamicImage>> {
        Ok(Cow::Borrowed(img))  // No modification
    }

    fn name(&self) -> &'static str { "passthrough" }
    fn protection_level(&self) -> ProtectionLevel { ProtectionLevel::Disabled }
    fn estimated_latency_ms(&self) -> u32 { 0 }
    fn modifies_pixels(&self) -> bool { false }
}
```

## Design Notes

- `modifies_pixels()` returns `false` — used by pipeline to decide optimization paths
- `apply()` returns `Cow::Borrowed` — zero allocation, zero copy
- `apply_bytes()` uses default implementation (decode → apply → re-encode), but since `apply` is a no-op, the bytes pass through unchanged

## Module Interactions

- **lib.rs**: Selected when `ProtectionLevel::Disabled` is requested
- **traits.rs**: Implements `Protector` trait
