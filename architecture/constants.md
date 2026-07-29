# Constants

**Source:** `src/protected/constants.rs` (~13 lines), `src/protected/steganography.rs`

Tuning constants used across the protection modules.

## Constants

| Constant | Value | Source File | Purpose |
|----------|-------|-------------|---------|
| `STEGO_OFFSET_SEED_1` | `0x517cc1b727220a95` | `protected/constants.rs` | Multiplicative offset for stego pixel selection |
| `STEGO_SPREAD_FACTOR` | `5` | `protected/constants.rs` | Number of adjacent pixels each LSB bit is spread across |
| `XORSHIFT_SEED_OFFSET` | `0x123456789ABCDEF0` | `util/image.rs` | XOR offset for XorShiftRng initialization |
| `SPLITMIX64_SEED` | `0x9e3779b97f4a7c15` | `protected/constants.rs`, `util/seed.rs` | Splitmix64 mixing constant |
| `DEFAULT_TILE_SIZE` | `64` | `protected/steganography.rs` | Default crop-resistant tile size |
| `MIN_TILE_SIZE` | `32` | `protected/steganography.rs` | Minimum tile size for crop resistance |
| `MIN_PAYLOAD_SIZE` | `28` | `protected/steganography.rs` | Parsing threshold (not output size) |
| `V3_PAYLOAD_VERSION` | `3` | `payload_v3/types.rs` | Current payload format version |

## Design Notes

- `STEGO_OFFSET_SEED_1` is a large prime-like constant used in the seed derivation formula: `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)`
- `XORSHIFT_SEED_OFFSET` ensures non-zero initial state for the PRNG
- Tile size is clamped to `32..=1024` (0 disables tiling)

## Module Interactions

- Referenced by `protected/steganography.rs`, `util/image.rs`, `util/seed.rs`, `payload_v3/types.rs`
