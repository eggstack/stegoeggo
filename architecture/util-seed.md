# Seed Generation

**Source:** `src/util/seed.rs` (~44 lines)

Pseudo-random seed generation from system time.

## Function

```rust
pub fn generate_random_seed() -> u64
```

Uses `SystemTime::now()` as entropy source, mixed with `splitmix64` algorithm:

```rust
let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
splitmix64(time)
```

## Security Warning

This is **NOT cryptographically secure**. The seed is predictable if the system time is known (±clock skew). Use cases:

- Default seed for `ProtectionContext::default()` — documented as predictable
- Users needing cryptographic seeds should use `ProtectionContext::new(intensity, seed)` with a CSPRNG

## Module Interactions

- **types.rs**: Called by `ProtectionContext::default()` to generate default seed
- **lib.rs**: Re-exported as public API for user convenience
