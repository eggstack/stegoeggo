pub(crate) fn try_generate_random_seed() -> std::result::Result<u64, getrandom::Error> {
    let mut buf = [0u8; 8];
    getrandom::getrandom(&mut buf)?;
    let x = u64::from_le_bytes(buf);
    Ok(if x == 0 { 42 } else { x })
}

/// Generate a random seed for APIs that cannot propagate entropy errors.
///
/// Uses `getrandom` (OS CSPRNG) for randomness and falls back to system-time-based
/// mixing if `getrandom` fails. Request-based protection APIs propagate that failure
/// instead; use an explicit seed for reproducible protection.
pub fn generate_random_seed() -> u64 {
    if let Ok(seed) = try_generate_random_seed() {
        return seed;
    }
    eprintln!("stegoeggo: getrandom failed, falling back to time-based seed (weak entropy)");

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
