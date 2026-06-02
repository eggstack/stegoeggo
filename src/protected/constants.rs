/// Multiplicative offset for steganography pixel selection seeds.
/// Used to derive per-pass seeds from the context seed.
pub const STEGO_OFFSET_SEED_1: u64 = 0x517cc1b727220a95;

/// Spatial spread for JPEG steganography embedding.
/// Bits are distributed across a `spread x spread` region per block.
pub const STEGO_JPEG_SPREAD: usize = 5;

/// Stride between embedding blocks for JPEG steganography.
pub const STEGO_JPEG_BLOCK_STRIDE: usize = 15;

/// Number of adjacent pixels each LSB bit is spread across.
pub const STEGO_SPREAD_FACTOR: usize = 5;

/// Offset added to seeds before XorShiftRng initialization.
/// Ensures the RNG state is never zero.
pub const XORSHIFT_SEED_OFFSET: u64 = 0x123456789ABCDEF0;

/// Minimum amplitude for content-adaptive JPEG steganography.
/// Used in smooth regions where large perturbations would be visible.
pub const STEGO_JPEG_MIN_AMPLITUDE: f32 = 10.0;

/// Maximum amplitude for content-adaptive JPEG steganography.
/// Used in textured regions where perturbations are masked by local detail.
pub const STEGO_JPEG_MAX_AMPLITUDE: f32 = 60.0;

/// Splitmix64 mixing constant (golden ratio bits).
pub const SPLITMIX64_SEED: u64 = 0x9e3779b97f4a7c15;
