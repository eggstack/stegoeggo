# Seed Generation

**Source:** `src/util/seed.rs` (~48 lines)

CSPRNG-backed random seed generation. Used to derive a fresh `u64` seed for `ProtectionContext::default()` and any other call site that needs an unpredictable per-instance seed.

## Function

```rust
pub fn generate_random_seed() -> u64
```

## Implementation

The function fills an 8-byte buffer from `getrandom::getrandom` (which reads from the OS CSPRNG — e.g. `/dev/urandom`, `getrandom(2)`, or `BCryptGenRandom` depending on platform) and interprets it as a little-endian `u64`:

```rust
let mut buf = [0u8; 8];
if getrandom::getrandom(&mut buf).is_ok() {
    let x = u64::from_le_bytes(buf);
    return if x == 0 { 42 } else { x };
}
```

### Fallback (rare)

`getrandom` is essentially always available on supported platforms, but the function guards the failure case anyway. If `getrandom` returns an error (for example in an unusual sandboxed environment), the function falls back to a `SystemTime`-derived splitmix64 mix and emits a warning to `stderr`:

```rust
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
if x == 0 { 42 } else { x }
```

The fallback uses `unwrap_or_default()` (does NOT panic on pre-UNIX-epoch clocks) and applies splitmix64-style bit mixing to spread entropy. The result is guaranteed non-zero (returns `42` if mixing produces zero).

## Security

In the common case, the returned seed is **cryptographically secure** — drawn directly from the OS CSPRNG via `getrandom` and unbiased by the `non-zero` guard. This is suitable for generating unpredictable seeds in adversarial settings.

The time-based fallback path is **not** cryptographically secure: an attacker who knows the request time (within ~ms) can reproduce the seed. The fallback is only reached in unusual sandboxed environments where `getrandom` is unavailable, and a warning is logged to make the situation observable.

For **reproducible** protection across runs (e.g. tests, deterministic pipelines), pass an explicit seed via `ProtectionContext::new(intensity, seed)` instead of relying on `generate_random_seed()`.

## Module Interactions

- **types.rs**: Called by `ProtectionContext::default()` to generate the default seed
- **lib.rs**: Re-exported as public API for user convenience
