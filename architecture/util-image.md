# Image Utilities

**Source:** `src/util/image.rs` (~701 lines)

Core image processing utilities: PRNG, noise generation, perturbation application, encoding, hashing, and format detection.

## XorShiftRng

General-purpose XorShift64 PRNG for noise and pixel selection.

```rust
pub struct XorShiftRng { state: u64 }
```

- `new(seed: u64)` — Initializes with seed XOR'd with `XORSHIFT_SEED_OFFSET`
- `next_u32()` — Returns random u32
- `next_u32_range(max: u32)` — Returns value in `[0, max)`

**WARNING:** This is NOT the same as `F5XorShiftRng` in `stego_f5.rs`. They use different algorithms and produce different sequences for the same seed. Do NOT interchange them.

## NoiseGenerator

HMAC-SHA256-based keyed seed derivation for deterministic noise.

```rust
pub struct NoiseGenerator { mac_key: Vec<u8> }
```

- `new(key: &[u8])` — Creates generator with HMAC key
- `derive_seed(&self, component: u8, extra: u64) -> u64` — Derives deterministic seed via HMAC

## PerturbationParams

Pre-computed parameters for perturbation:

- `intensity`, `block_width`, `block_height`, `keyed_seed_base`
- `freq_h`, `freq_v`, `freq_d` — Frequency parameters for sinusoidal noise
- `amplitude` — Noise amplitude derived from intensity

## PerturbationRuntime

Shared setup struct for both serial and parallel perturbation paths:

- Pre-computes `NoiseGenerator` and spatial seed
- Generates per-row `y_variations: Vec<f32>` — sinusoidal Y-axis variation
- Eliminates duplicated code between serial and parallel paths

`PerturbationParams` retains the `NoiseGenerator` internally and exposes `derive_spatial_seed()` so callers avoid redundant HMAC key initialization.

## Perturbation Functions

### Single-pass (auto-selects serial/parallel)

```rust
pub fn apply_perturbation_single_pass(img: &mut RgbaImage, params: &mut PerturbationParams, ctx: &ProtectionContext) -> Vec<u8>
pub fn apply_perturbation_single_pass_keyed(img: &mut RgbaImage, params: &mut PerturbationParams, ctx: &ProtectionContext) -> Vec<u8>
```

- Uses `parallel_threshold()` to decide: if pixels > threshold, uses parallel path
- Returns the perturbation data as `Vec<u8>` (RGBA bytes) for precomputed variant storage

### Parallel path

```rust
pub fn apply_perturbation_single_pass_keyed_par(img: &mut RgbaImage, params: &mut PerturbationParams, ctx: &ProtectionContext) -> Vec<u8>
```

- Uses `rayon::par_chunks_mut` for row-level parallelism
- Each row gets deterministic variation via `y_variations[row]`

### Precomputed application

```rust
pub fn apply_perturbation(img: &mut RgbaImage, perturbation: &[u8], divisor: f32)
pub fn apply_perturbation_par(img: &mut RgbaImage, perturbation: &[u8], divisor: f32)
```

- Applies previously-generated perturbation data (from `PrecomputedProtector`)
- `divisor` scales the perturbation intensity

## Other Utilities

- `compute_image_hash(img) -> String` — SHA-256 hex hash of RGBA pixel data
- `detect_image_format(bytes) -> Option<ImageOutputFormat>` — PNG/JPEG/WebP detection
- `encode_image(img, format) -> Vec<u8>` — Encode to target format
- `encode_image_with_options(img, format, progressive, quality) -> Vec<u8>` — With JPEG options
- `load_image_from_bytes(bytes) -> Result<DynamicImage>` — Decode image bytes
- `parallel_threshold() -> usize` — Returns `cores * 64 * 64` (scales with rayon thread count)
- `SIN_TABLE` — Fast sine lookup table (256 entries) for frequency perturbations

## Module Interactions

- **protected/noise.rs**: Calls `apply_perturbation_single_pass[_keyed]`
- **protected/precomputed.rs**: Calls `apply_perturbation[_par]`
- **protected/steganography.rs**: Uses `XorShiftRng` for pixel selection (LSB stego)
- **util/seed.rs**: `generate_random_seed()` used for default context seeds
- **lib.rs**: Uses encoding/detection functions for format routing
