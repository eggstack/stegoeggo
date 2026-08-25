# Detached Signed Manifests

**Source:** `src/detached/` (feature-gated: `detached-manifest`)

Provides signed sidecar manifests for distributing provenance evidence outside the image file. Manifests are independent of image format and can be distributed via sidecar files, API responses, or database records.

## Module Structure

```
src/detached/
├── mod.rs              Re-exports
├── manifest.rs         DetachedManifest, SignatureRecord, PublicKeyEntry, TrustMetadata
├── generate.rs         create_manifest_from_image(), compute_image_digest()
└── verify.rs           verify_detached_manifest*(), TrustPolicy, DetachedOverallStatus
```

## `DetachedManifest`

```rust
pub struct DetachedManifest {
    pub schema_version: u8,           // Currently 1
    pub claim: ProvenanceClaim,       // The provenance assertion
    pub signatures: Vec<SignatureRecord>,
    pub public_keys: Vec<PublicKeyEntry>,
    pub embedded_reference: Option<EmbeddedReference>,
    pub trust_metadata: Option<TrustMetadata>,
}
```

### Builder Methods

- `new(claim)` — Create with schema version 1
- `with_signature(SignatureRecord)` — Add signature (max 16, deduplicates by algorithm+key_id)
- `with_public_key(PublicKeyEntry)` — Add public key (max 16, deduplicates by key_id)
- `with_embedded_reference(EmbeddedReference)` — Link to in-image stego payload
- `with_trust_metadata(TrustMetadata)` — Add trust chain metadata

### Validation

`validate()` checks:
- No empty key IDs
- Valid hex encoding of key bytes and signatures
- Correct byte lengths (32-byte public keys, 64-byte signatures)
- Each signature references an existing public key
- No duplicate signatures or public keys

### Serialization

- `canonical_bytes() -> Vec<u8>` — Canonical JSON bytes (sorted keys, compact) for digest computation
- `digest() -> [u8; 32]` — SHA-256 of canonical bytes
- `from_json(bytes: &[u8]) -> Result<Self, Error>` — Deserialize from JSON bytes with size/version validation
- `from_json_with_limits(bytes: &[u8], limits: &ResourceLimits) -> Result<Self, Error>` — Deserialize with explicit resource limits

## `SignatureRecord`

```rust
pub struct SignatureRecord {
    pub algorithm: String,    // e.g. "Ed25519"
    pub key_id: Vec<u8>,      // Key identifier
    pub signature: String,    // Hex-encoded signature bytes
}
```

## `PublicKeyEntry`

```rust
pub struct PublicKeyEntry {
    pub key_id: Vec<u8>,
    pub algorithm: String,     // e.g. "Ed25519"
    pub key_bytes: String,     // Hex-encoded 32-byte public key
}
```

## `EmbeddedReference`

Links a detached manifest to an in-image stego payload:

```rust
pub struct EmbeddedReference {
    pub payload_digest: String,  // SHA-256 of the embedded payload
    pub payload_version: u8,     // Payload format version
}
```

## `TrustMetadata`

Optional trust chain information:

```rust
pub struct TrustMetadata {
    pub trust_model: String,          // e.g. "local", "web-of-trust", "pki"
    pub trusted: bool,
    pub reason: String,
    pub certificate_chain: Option<Vec<String>>,  // DER-encoded, base64
}
```

## Generation

### `create_manifest_from_image(image_bytes, claim)`

1. Computes instance digest (SHA-256 of image bytes) on the claim
2. Detects format, gets dimensions from decoded image
3. Populates source facts (format, width, height, file_size)
4. Returns `DetachedManifest` ready for signing

### `create_manifest_with_claim(image_bytes, claim)`

Same as above but accepts a fully populated claim.

### `compute_image_digest(image_bytes) -> String`

Returns `"sha256:<hex>"` of the raw image bytes.

## Verification

### Entry Points

- `verify_detached_manifest(image_bytes, manifest, trust)` — Basic verification with trust policy
- `verify_detached_manifest_with_keys(image_bytes, manifest, expected_keys: Option<&[Vec<u8>]>)` — With flat key-ID set
- `verify_detached_manifest_with_limits(image_bytes, manifest, trust, limits)` — With resource limits
- `verify_detached_manifest_with_limits_and_mac(image_bytes, manifest, trust, limits, payload_mac_key)` — With limits and HMAC key
- `verify_detached_manifest_with_keys_and_mac(image_bytes, manifest, expected_keys, payload_mac_key)` — With flat key-ID set and HMAC key
- `verify_detached_manifest_with_options(image_bytes, manifest, options)` — Full options via `DetachedVerificationOptions`

### `TrustPolicy`

```rust
pub enum TrustPolicy {
    TrustNone,                         // Never trust any key
    TrustKeys(Vec<Vec<u8>>),           // Trust specific key IDs (no key binding)
    TrustCallback(Box<TrustCallbackFn>), // Callback decides trust per key_id
}
```

For caller-owned public key verification (binding key ID to exact key bytes), use `DetachedVerificationOptions` with `caller_verifying_keys`.

### `DetachedVerificationOptions`

```rust
pub struct DetachedVerificationOptions<'a> {
    pub trust_policy: Option<&'a TrustPolicy>,
    pub caller_verifying_keys: &'a [TrustedVerifyingKey],  // signatures feature
    pub payload_mac_key: Option<&'a [u8]>,
    pub limits: Option<&'a ResourceLimits>,
}
```

### `DetachedOverallStatus`

```rust
pub enum DetachedOverallStatus {
    VerifiedTrusted,        // Valid + trusted signature
    VerifiedUntrusted,      // Valid + no trusted signature
    InvalidConfiguration,   // Parse error or limit exceeded
    BindingFailure,         // Image digest mismatch
    SignatureFailure,       // No valid signature
    EmbeddedReferenceFailure, // Payload reference check failed
    KeyMaterialMismatch,    // Caller key != manifest key for same ID
}
```

### `EmbeddedReferenceStatus`

Tracks whether the in-image stego payload referenced by the manifest is present and valid:

- `NotProvided` — No reference declared
- `Stripped` — Reference declared but no payload found
- `VersionMismatch` — Payload found but wrong version
- `DigestMismatch` — Payload found but digest differs
- `Malformed` — Payload found but unparseable
- `Present` — **Deprecated:** use `PresentValid`
- `PresentValid` — Reference declared and valid payload found
- `AuthenticationKeyMissing` — HMAC-protected reference but no MAC key
- `AuthenticationFailed` — HMAC verification failed
- `UnsupportedVersion` — Payload found but version is not supported

## Limits

| Limit | Default | Description |
|-------|---------|-------------|
| `MAX_MANIFEST_SIZE` | 64 KiB | Maximum manifest JSON size |
| `MAX_SIGNATURES` | 16 | Maximum signature records |
| `MAX_PUBLIC_KEYS` | 16 | Maximum public key entries |
| `MAX_KEY_ID_LEN` | 64 bytes | Maximum key identifier length |

## Exit Codes

| Status | Code | Meaning |
|--------|------|---------|
| `VerifiedTrusted` | 0 | Pass |
| `VerifiedUntrusted` | 4 | Valid but untrusted |
| `InvalidConfiguration` | 2 | Config error |
| `BindingFailure` | 3 | Digest mismatch |
| `SignatureFailure` | 3 | No valid sig |
| `EmbeddedReferenceFailure` | 3 | Payload ref failed |
| `KeyMaterialMismatch` | 3 | Key bytes mismatch |
