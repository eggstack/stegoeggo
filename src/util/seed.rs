/// Generate a pseudo-random seed from the current system time.
///
/// Uses a hash-like mixing function to combine seconds and nanoseconds,
/// avoiding predictable patterns when two calls share the same second.
/// Returns a non-zero u64.
///
/// # Security
///
/// **Not cryptographically secure.** The output is deterministic given the
/// system clock. If seed unpredictability is required (e.g., adversarial
/// settings where an attacker knows the approximate request time), use a
/// CSPRNG like `getrandom` instead.
///
/// This is suitable for determinism within a single request (reproducible
/// protection from a known seed), not for generating secret keys or nonces.
pub fn generate_random_seed() -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let s = now.as_secs();
    let ns = now.subsec_nanos() as u64;
    let mut x = s ^ (ns.wrapping_mul(0x9E3779B97F4A7C15));
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    if x == 0 {
        42
    } else {
        x
    }
}
