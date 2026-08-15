# Constants

**Source:** `src/protected/constants.rs` (~13 lines), `stegoeggo-stego/src/constants.rs`, and the application adapter under `src/protected/steganography/`

Tuning constants used across the protection modules. The carrier crate
(`stegoeggo-stego`) keeps its own copy of the carrier-only constants
(`STEGO_OFFSET_SEED_1`, `STEGO_SPREAD_FACTOR`, `SPLITMIX64_SEED`) because the
root crate's copy is needed only by the application adapter for legacy
extraction probing. The two copies are byte-identical.

## Constants

| Constant | Value | Source File | Purpose |
|----------|-------|-------------|---------|
| `STEGO_OFFSET_SEED_1` | `0x517cc1b727220a95` | `protected/constants.rs`, `stegoeggo-stego/src/constants.rs` | Multiplicative offset for stego pixel selection (legacy) |
| `STEGO_SPREAD_FACTOR` | `5` | `protected/constants.rs`, `stegoeggo-stego/src/constants.rs` | Replicas per payload bit per redundancy level in the V2 carrier (total replicas = `STEGO_SPREAD_FACTOR * redundancy`) |
| `XORSHIFT_SEED_OFFSET` | `0x123456789ABCDEF0` | `protected/constants.rs` | XOR offset for XorShiftRng initialization (legacy `PixelSelectionRng`) |
| `SPLITMIX64_SEED` | `0x9e3779b97f4a7c15` | `stegoeggo-stego/src/constants.rs`, `util/seed.rs` | Splitmix64 mixing constant |
| `DEFAULT_TILE_SIZE` | `64` | `stegoeggo-stego/src/lsb_internal.rs` (re-exported via `lsb.rs`) | Default crop-resistant tile size |
| `MIN_PAYLOAD_SIZE` | `28` | `protected/steganography/mod.rs` | Parsing threshold (not output size) |
| `V3_PAYLOAD_VERSION` | `3` | `payload_v3/types.rs` | Current payload format version |

## Design Notes

- `STEGO_OFFSET_SEED_1` is a large prime-like constant used in the seed derivation formula for legacy offset seeds: `offset_seed = seed * (STEGO_OFFSET_SEED_1 + pass)`. The corrected V2 carrier uses the raw seed directly without this offset
- `STEGO_SPREAD_FACTOR` is the base replication factor per payload bit; the corrected V2 carrier multiplies this by the redundancy parameter to get total replicas per bit
- `XORSHIFT_SEED_OFFSET` ensures non-zero initial state for the `PixelSelectionRng` PRNG
- Tile size is clamped to `32..=1024` (0 disables tiling)

## Module Interactions

- `src/protected/constants.rs` is referenced by `src/protected/steganography/extract.rs`
  (legacy seed offset derivation) and the legacy `util::image::PixelSelectionRng`
  via `XORSHIFT_SEED_OFFSET`
- `stegoeggo-stego/src/constants.rs` is referenced by `stegoeggo-stego/src/lsb_internal.rs`
  and the carrier-level `splitmix64` mixer
- `payload_v3/types.rs` defines the payload wire format version
