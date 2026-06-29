# Constants

**Source:** `src/protected/constants.rs` (~13 lines)

Tuning constants used across the protection modules.

## Constants

| Constant | Value | Used In | Purpose |
|----------|-------|---------|---------|
| `STEGO_OFFSET_SEED_1` | `0x517cc1b727220a95` | `steganography.rs` | Multiplicative offset for stego pixel selection |
| `STEGO_SPREAD_FACTOR` | `5` | `steganography.rs` | Number of adjacent pixels each LSB bit is spread across |
| `XORSHIFT_SEED_OFFSET` | `0x123456789ABCDEF0` | `util/image.rs` | XOR offset for XorShiftRng initialization |
| `SPLITMIX64_SEED` | `0x9e3779b97f4a7c15` | `steganography.rs`, `util/seed.rs` | Splitmix64 mixing constant |

## Design Notes

- `STEGO_OFFSET_SEED_1` is a large prime-like constant used in the seed derivation formula: `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)`
- `XORSHIFT_SEED_OFFSET` ensures non-zero initial state for the PRNG

## Module Interactions

- Referenced by `protected/steganography.rs`, `util/image.rs`, `util/seed.rs`
