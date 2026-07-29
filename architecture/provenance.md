# Provenance Claim Model

**Source:** `src/provenance/`

Provides a canonical, deterministic serializable type for rights/provenance assertions about images. Shared by embedded v3 payloads and detached manifests.

## Module Structure

```
src/provenance/
├── mod.rs        Re-exports
├── claim.rs      ProvenanceClaim builder and canonical serialization
├── canonical.rs  Canonical JSON serialization helpers
└── digest.rs     TypedDigest for content hashing
```

## `ProvenanceClaim`

The core provenance assertion type:

```rust
pub struct ProvenanceClaim {
    pub claim_id: [u8; 16],           // Random 16-byte identifier (hex-encoded)
    pub content_code: String,          // ISCC or local content identifier
    pub created_at: u64,               // Unix epoch seconds
    pub file_size: u64,                // File size in bytes
    pub format: String,                // "png", "jpeg", "webp"
    pub height: u32,
    pub width: u32,
    pub instance_digest: String,       // "sha256:<hex>" of file bytes
    pub issuer_id: String,             // Base64url-encoded issuer/key ID
    pub notice_digest: String,         // SHA-256 of normalized rights-notice text
    pub parent_claim_id: Option<String>, // Base64url-encoded parent claim ID
    pub rights_policy: u8,             // Rights/data-mining policy discriminant
    pub schema_version: u8,            // Currently 1
    pub software: String,              // e.g. "stegoeggo/0.5.0"
    pub statement_uri: Option<String>, // URI to rights statement
}
```

### Builder Pattern

```rust
let claim = ProvenanceClaim::builder()
    .with_creator("Jane Artist")
    .with_copyright("© 2025 Jane Artist")
    .with_content_code("iscc:abc123")
    .with_instance_digest(image_bytes)
    .with_source_facts("png", 1920, 1080, 1024000)
    .with_issuer_id(key_id_bytes)
    .with_notice_digest(rights_text)
    .with_statement_uri("https://example.com/license")
    .with_parent_claim(parent_claim_id)
    .build();
```

### Key Methods

- `builder()` — Start building a claim
- `with_instance_digest(&[u8])` — Compute SHA-256 of image bytes
- `with_source_facts(format, width, height, file_size)` — Set image metadata
- `canonical_bytes() -> Vec<u8>` — Deterministic JSON for signing/hashing
- `claim_digest() -> String` — SHA-256 hex of canonical bytes

## Canonical JSON

`canonical_json(claim)` produces deterministic JSON:
- Sorted keys
- No whitespace
- Null omission (`skip_serializing_if`)

Used for:
- Signing (Ed25519 signs canonical bytes)
- Digest computation (SHA-256 of canonical form)
- Cross-implementation interoperability

`verify_canonical_stability(claim)` asserts that canonical bytes are identical across calls.

## `TypedDigest`

Content digest computation:

```rust
pub struct TypedDigest {
    algorithm: String,  // e.g. "sha256"
    hex: String,
}
```

- `from_image_bytes(&[u8])` — SHA-256 of raw bytes
- `to_string()` — `"sha256:<hex>"` format

## Version

```rust
pub const PROVENANCE_CLAIM_VERSION: u8 = 1;
```

## Usage

1. **Embedded in v3 payloads** — Claim is serialized as a v3 extension and embedded in the stego payload
2. **Detached manifests** — Claim is the primary content of the manifest, signed by Ed25519
3. **Verification** — Both paths verify the claim's instance digest against the image bytes

## Relationship to Other Modules

- **`payload_v3`** — V3 payloads can carry a `ProvenanceClaim` as an extension
- **`detached`** — Detached manifests wrap a `ProvenanceClaim` with signatures
- **`signing`** — Ed25519 signs the claim's canonical bytes
- **`util::iscc`** — Content identifiers populate `content_code`
