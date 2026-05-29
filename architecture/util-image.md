# Image Utilities

**Source:** `src/util/image.rs` (~701 lines)

Core image processing utilities: PRNG, noise generation, perturbation application, encoding, hashing, and format detection.

## XorShiftRng

General-purpose XorShift64 PRNG for noise and pixel selection.

```rust
pub struct XorShiftRng { state: u64 }
```

- `new(seed: u64)` — Initializes with seed XOR'd with `XORSHIFT_SEED_OFFSET`
- `next_u64()` — Returns random u64
- `gen_f32()` — Returns random f32 in `[-1.0, 1.0)`
- `gen_range(range: Range<f32>)` — Returns f32 in given range
- `gen_range_usize(range: Range<usize>)` — Returns usize in given range

## NoiseGenerator

HMAC-SHA256-based keyed seed derivation for deterministic noise.

```rust
pub struct NoiseGenerator { seed: u64, mac_key: Option<Arc<[u8]>> }
```

- `new(seed: u64)` — Creates generator with no MAC key
- `with_mac_key(seed: u64, mac_key: impl Into<Arc<[u8]>>)` — Creates generator with HMAC key
- `derive_keyed_seed(&self, pixel_pos: u64) -> u64` — Derives deterministic seed via HMAC (returns `self.seed` if no MAC key)

## PerturbationParams

Private struct with pre-computed perturbation parameters:

- `intensity`, `blocks_x`, `keyed_seed_base`
- `inv_pattern_scale`, `intensity_factor`, `phase_offset` — Sinusoidal noise parameters
- `noise_gen` — Retained `NoiseGenerator` for deriving additional seeds

## PerturbationRuntime

Shared setup struct for both serial and parallel perturbation paths:

- Pre-computes `NoiseGenerator` and spatial seed
- Generates per-row `y_variations: Vec<f32>` — sinusoidal Y-axis variation
- Eliminates duplicated code between serial and parallel paths

`PerturbationParams` retains the `NoiseGenerator` internally and exposes `derive_spatial_seed()` so callers avoid redundant HMAC key initialization.

## Perturbation Functions

### Single-pass (auto-selects serial/parallel)

```rust
pub fn apply_perturbation_single_pass(img: &RgbaImage, seed: u64, intensity: f32, intensity_multiplier: f32) -> DynamicImage
pub fn apply_perturbation_single_pass_keyed(img: &RgbaImage, seed: u64, intensity: f32, intensity_multiplier: f32, mac_key: &[u8]) -> DynamicImage
```

- Uses `parallel_threshold()` to decide: if pixels > threshold, uses parallel path
- Returns perturbed image as `DynamicImage`

### Parallel path

```rust
pub fn apply_perturbation_single_pass_keyed_par(img: &RgbaImage, seed: u64, intensity: f32, intensity_multiplier: f32, mac_key: &[u8]) -> DynamicImage
```

- Uses `rayon::par_chunks_mut` for row-level parallelism
- Each row gets deterministic variation via `y_variations[row]`

### Precomputed application

```rust
pub fn apply_perturbation(img: &RgbaImage, perturbation: &[u8], divisor: i16) -> Result<RgbaImage>
pub fn apply_perturbation_par(img: &RgbaImage, perturbation: &[u8], divisor: i16) -> Result<RgbaImage>
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
- `SIN_TABLE` — Fast sine lookup table (1024 entries) for frequency perturbations

## Module Interactions

- **protected/noise.rs**: Calls `apply_perturbation_single_pass[_keyed]`
- **protected/precomputed.rs**: Calls `apply_perturbation[_par]`
- **protected/steganography.rs**: Uses `XorShiftRng` for pixel selection (LSB stego)
- **util/seed.rs**: `generate_random_seed()` used for default context seeds
- **lib.rs**: Uses encoding/detection functions for format routing
