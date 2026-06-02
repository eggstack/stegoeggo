/// Generate a cryptographically secure random seed.
///
/// Uses `getrandom` (OS CSPRNG) for randomness. Falls back to system-time-based
/// mixing if `getrandom` fails (e.g., in unusual sandboxed environments).
/// Returns a non-zero u64.
///
/// This is suitable for generating unpredictable seeds in adversarial settings.
/// For reproducible protection, use `ProtectionContext::new(intensity, seed)` with
/// a seed of your choice.
pub fn generate_random_seed() -> u64 {
    let mut buf = [0u8; 8];
    if getrandom::getrandom(&mut buf).is_ok() {
        let x = u64::from_le_bytes(buf);
        return if x == 0 { 42 } else { x };
    }
    // Fallback: time-based seed (not cryptographically secure)
    eprintln!(
        "[cloakrs] WARNING: getrandom unavailable, using time-based seed (not cryptographically secure). \
         For adversarial settings, use ProtectionContext::new(intensity, external_csprng_seed)"
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_random_seed_returns_nonzero() {
        let seed = generate_random_seed();
        assert_ne!(seed, 0);
    }
}
