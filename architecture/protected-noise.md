# Noise Protector

**Source:** `src/protected/noise.rs` (~129 lines)

Standard adversarial noise protector for the `Standard` protection level. Estimated latency: 3ms.

## Implementation

```rust
pub struct NoiseProtector {
    intensity_multiplier: f32,  // 10.0 for Standard, 12.0 for Enhanced
}
```

## How It Works

1. Convert input image to RGBA
2. Create `PerturbationParams` with intensity scaled by `intensity_multiplier`
3. Call `apply_perturbation_single_pass[_keyed]` from `util::image`
4. Return modified image as `Cow::Owned`

### Intensity Scaling

The raw `intensity` (0.0–1.0) is multiplied by 10.0 (Standard) or 12.0 (Enhanced) before being used to scale noise amplitude. This gives Standard and Enhanced different noise characteristics even at the same intensity value.

### Zero Intensity Optimization

When `intensity == 0.0`, returns `Cow::Borrowed(img)` — no allocation, no processing.

### Keyed vs Unkeyed

- Without MAC key: Uses `apply_perturbation_single_pass` — seed-based but not HMAC-derived
- With MAC key: Uses `apply_perturbation_single_pass_keyed` — HMAC-SHA256 key derivation for stronger determinism

## Enhanced Variant

`NoiseProtector::enhanced()` creates a protector with 12x multiplier (vs standard 10x):

```rust
impl NoiseProtector {
    pub fn enhanced() -> Self {
        Self { intensity_multiplier: 12.0 }
    }
}
```

## Module Interactions

- **lib.rs**: Selected for `ProtectionLevel::Standard` (10x) and `Enhanced` (12x via `EnhancedProtector`)
- **util/image.rs**: Delegates to `apply_perturbation_single_pass[_keyed]`
- **protected/enhanced.rs**: `EnhancedProtector` wraps `NoiseProtector::enhanced()`
- **protected/constants.rs**: Uses `NOISE_INTENSITY_MULTIPLIER` (10.0)
