# Ed25519 Signing

**Source:** `src/signing/` (feature-gated: `signatures`)

Provides Ed25519 signing and verification for provenance claims and detached manifests. The module is compiled only when the `signatures` feature is enabled.

## Module Structure

```
src/signing/
├── mod.rs              Capacity check, ED25519_OVERHEAD_BYTES constant
├── config.rs           SigningConfig, SignaturePlacement
└── ed25519_impl.rs     SigningKey, VerifyingKey, SignatureResult
```

## Key Types

### `SigningKey`

Wraps `ed25519_dalek::SigningKey` with a key identifier. Private key material is zeroized on drop. `Debug` does not reveal key bytes. `Serialize`/`Deserialize` are intentionally not implemented.

- `from_bytes([u8; 32], Vec<u8>)` — Create from raw seed + key ID
- `generate()` — Random key with 16-byte random key ID
- `sign(&[u8]) -> Vec<u8>` — Deterministic Ed25519 signature (64 bytes)
- `verifying_key() -> VerifyingKey` — Derive public key
- `zeroize()` — Best-effort key erasure

### `VerifyingKey`

Wraps `ed25519_dalek::VerifyingKey` with a key ID. Implements `Serialize`/`Deserialize` for embedding in metadata.

- `from_bytes([u8; 32], Vec<u8>)` — Create from raw public key + key ID
- `verify(&[u8], &[u8]) -> SignatureResult` — Verify signature against claim bytes

### `SignatureResult`

```rust
pub enum SignatureResult {
    Valid,
    Invalid,
    MalformedSignature,
    MalformedPublicKey,
}
```

### `SigningConfig`

Bundles signing key, key ID, and placement preference. Does not implement `Serialize` (contains secret material).

- `new(SigningKey, SignaturePlacement)` — Full constructor
- `with_key(SigningKey)` — Preferred-embedded placement
- `check_capacity(available_bytes) -> SignatureCapacity` — Fits or NeedsDetached

### `SignaturePlacement`

```rust
pub enum SignaturePlacement {
    Embedded,          // In-image payload (when capacity permits)
    Detached,          // Detached manifest only
    PreferredEmbedded, // Embedded if fits, detached otherwise
}
```

## Capacity Check

`check_signature_capacity()` computes whether an Ed25519 signature fits within the available payload byte budget:

- Core header: 32 bytes (`V3_CORE_SIZE`)
- Key ID: variable (0–32 bytes)
- Ed25519 overhead: 168 bytes (64-byte signature + 36-byte key extension + 68-byte signature extension)

Returns `FitsEmbedded` or `NeedsDetached`.

## Overhead Constant

```rust
pub const ED25519_OVERHEAD_BYTES: usize = 168;
// = 64 (signature) + 36 (key extension: 2+2+32) + 68 (sig extension: 2+2+64)
```

## Security Properties

- Private key material is zeroized on drop via `zeroize` crate
- `Debug` output shows only hex-encoded key ID, not key bytes
- No `Serialize` implementation on `SigningKey` prevents accidental key material serialization
- Ed25519 has no application-level context parameter; the `SIGNATURE_DOMAIN` constant is retained for backward compatibility and is not prepended to existing detached-signature claim bytes
- Domain-separated MAC keys prevent cross-domain forgery in v3 payloads

## Usage in Pipeline

Signing integrates with the v3 payload format. When a `SigningConfig` is provided:

1. The payload builder checks capacity via `check_signature_capacity()`
2. If `FitsEmbedded`: signature and public key are added as v3 extensions
3. If `NeedsDetached`: signature goes into a detached manifest sidecar
4. Verification extracts the public key from the payload or manifest and calls `VerifyingKey::verify()`
