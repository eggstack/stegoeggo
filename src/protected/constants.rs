/// Multiplier applied to noise intensity for the standard noise protector.
pub const NOISE_INTENSITY_MULTIPLIER: f32 = 10.0;

/// Multiplicative offset for steganography pixel selection seeds.
/// Used to derive per-pass seeds from the context seed.
pub const STEGO_OFFSET_SEED_1: u64 = 0x517cc1b727220a95;

/// Pixel amplitude adjustment for JPEG-robust steganography.
/// Controls how much each pixel channel value is shifted per embedded bit.
pub const STEGO_JPEG_AMPLITUDE: i16 = 40;

/// Spatial spread for JPEG steganography embedding.
/// Bits are distributed across a `spread x spread` region per block.
pub const STEGO_JPEG_SPREAD: usize = 5;

/// Stride between embedding blocks for JPEG steganography.
pub const STEGO_JPEG_BLOCK_STRIDE: usize = 15;

/// Offset added to seeds before XorShiftRng initialization.
/// Ensures the RNG state is never zero.
pub const XORSHIFT_SEED_OFFSET: u64 = 0x123456789ABCDEF0;

/// Splitmix64 mixing constant (golden ratio bits).
pub const SPLITMIX64_SEED: u64 = 0x9e3779b97f4a7c15;

/// Default capacity for the PrecomputedProtector in-memory LRU cache.
pub const PRECOMPUTED_CACHE_CAPACITY: usize = 100;
