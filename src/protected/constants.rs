/// Multiplicative offset for steganography pixel selection seeds.
/// Used to derive per-pass seeds from the context seed.
pub const STEGO_OFFSET_SEED_1: u64 = 0x517cc1b727220a95;

/// Number of adjacent pixels each LSB bit is spread across.
pub const STEGO_SPREAD_FACTOR: usize = 5;

/// Offset added to seeds before XorShiftRng initialization.
/// Ensures the RNG state is never zero.
pub const XORSHIFT_SEED_OFFSET: u64 = 0x123456789ABCDEF0;

/// Splitmix64 mixing constant (golden ratio bits).
pub const SPLITMIX64_SEED: u64 = 0x9e3779b97f4a7c15;
