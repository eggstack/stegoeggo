# Constants

**Source:** `src/protected/constants.rs` (~24 lines)

Tuning constants used across the protection modules.

## Constants

| Constant | Value | Used In | Purpose |
|----------|-------|---------|---------|
| `NOISE_INTENSITY_MULTIPLIER` | `10.0` | `noise.rs` | Scales intensity for Standard noise |
| `STEGO_OFFSET_SEED_1` | `0x517cc1b727220a95` | `steganography.rs` | Multiplicative offset for stego pixel selection |
| `STEGO_JPEG_AMPLITUDE` | `40` | `steganography.rs` | Pixel amplitude for JPEG stego |
| `STEGO_JPEG_SPREAD` | `5` | `steganography.rs` | Spatial spread for JPEG stego |
| `STEGO_JPEG_BLOCK_STRIDE` | `15` | `steganography.rs` | Block stride for JPEG stego |
| `XORSHIFT_SEED_OFFSET` | `0x123456789ABCDEF0` | `util/image.rs` | XOR offset for XorShiftRng initialization |
| `SPLITMIX64_SEED` | `0x9e3779b97f4a7c15` | `util/seed.rs` | Splitmix64 mixing constant |

## Design Notes

- `STEGO_OFFSET_SEED_1` is a large prime-like constant used in the seed derivation formula: `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)`
- `NOISE_INTENSITY_MULTIPLIER` (10.0) is the Standard multiplier; Enhanced uses 12.0 (hardcoded in `EnhancedProtector`)
- `XORSHIFT_SEED_OFFSET` ensures non-zero initial state for the PRNG

## Module Interactions

- Referenced by `protected/noise.rs`, `protected/steganography.rs`, `util/image.rs`, `util/seed.rs`
