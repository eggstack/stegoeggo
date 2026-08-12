/// Multiplicative offset for steganography pixel selection seeds.
/// Used to derive per-pass seeds from the context seed.
pub(crate) const STEGO_OFFSET_SEED_1: u64 = 0x517cc1b727220a95;

/// Number of adjacent pixels each LSB bit is spread across.
#[allow(dead_code)]
pub(crate) const STEGO_SPREAD_FACTOR: usize = 5;

/// Offset added to seeds before XorShiftRng initialization.
/// Ensures the RNG state is never zero.
pub(crate) const XORSHIFT_SEED_OFFSET: u64 = 0x123456789ABCDEF0;
