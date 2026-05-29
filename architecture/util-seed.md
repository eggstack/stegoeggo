# Seed Generation

**Source:** `src/util/seed.rs` (~44 lines)

Pseudo-random seed generation from system time.

## Function

```rust
pub fn generate_random_seed() -> u64
```

Uses `SystemTime::now()` as entropy source. Extracts seconds and nanoseconds separately, combines them with golden-ratio-based mixing, then applies splitmix64-style bit mixing:

```rust
let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
let s = now.as_secs();
let ns = now.subsec_nanos() as u64;
let mut x = s ^ (ns.wrapping_mul(0x9E3779B97F4A7C15));
x ^= x >> 30;
x = x.wrapping_mul(0xBF58476D1CE4E5B9);
x ^= x >> 27;
x = x.wrapping_mul(0x94D049BB133111EB);
x ^= x >> 31;
if x == 0 { 42 } else { x }
```

Uses `unwrap_or_default()` (does NOT panic on pre-UNIX-epoch clocks). Guarantees non-zero output (returns `42` if mixing produces zero).

## Security Warning

This is **NOT cryptographically secure**. The seed is predictable if the system time is known (±clock skew). Use cases:

- Default seed for `ProtectionContext::default()` — documented as predictable
- Users needing cryptographic seeds should use `ProtectionContext::new(intensity, seed)` with a CSPRNG

## Module Interactions

- **types.rs**: Called by `ProtectionContext::default()` to generate default seed
- **lib.rs**: Re-exported as public API for user convenience
