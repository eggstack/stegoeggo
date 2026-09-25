# Provenance Claim Model

**Source:** `src/provenance/`

Provides a canonical, deterministic serializable type for rights/provenance assertions about images. Used by detached manifests (the embedded v3 stego payload does not currently carry a `ProvenanceClaim` — it uses its own compact header plus `ExtensionType` TLV extensions).

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
    pub instance_digest: String,       // "sha256:<hex>" of file bytes
    pub issuer_id: String,             // Base64url-encoded issuer/key ID
    pub notice_digest: String,         // SHA-256 of normalized rights-notice text
    pub parent_claim_id: Option<String>, // Base64url-encoded parent claim ID
    pub rights_policy: u8,             // Rights/data-mining policy discriminant
    pub schema_version: u8,            // Currently 1
    pub software: String,              // e.g. "stegoeggo/0.5.0"
    pub statement_uri: Option<String>, // URI to rights statement
    pub width: u32,
}
```

### Builder Pattern

```rust
let claim = ProvenanceClaim::new(policy_discriminant)
    .with_content_code("iscc:abc123".to_string())
    .with_instance_digest(image_bytes)
    .with_source_facts("png", 1920, 1080, 1024000)
    .with_issuer_id(key_id_base64url)
    .with_notice_digest(rights_text_bytes)
    .with_statement_uri("https://example.com/license")
    .with_parent_claim(parent_claim_id_base64url);
```

### Key Methods

- `new(rights_policy: u8)` — Create a claim with a random ID; chain `with_*` methods
- `with_notice_digest(&[u8])` / `with_notice_digest_raw(String)` — Set notice digest (computed SHA-256 vs pre-formatted `"sha256:<hex>"`)
- `with_content_code(String)` — Set content identifier (e.g. `"iscc:<hex>"`)
- `with_instance_digest(&[u8])` / `with_instance_digest_raw(String)` — Compute SHA-256 of image bytes, or set pre-formatted digest
- `with_source_facts(&str, u32, u32, u64)` — Set image metadata (format, width, height, file_size)
- `with_creation_time(u64)` — Set Unix epoch seconds
- `with_issuer_id(String)` — Set base64url issuer/key identifier
- `with_software(&str)` — Set software identifier (e.g. `"stegoeggo/0.5.0"`)
- `with_parent_claim(String)` / `with_statement_uri(&str)` — Set optional chain/statement fields
- `canonical_bytes() -> Vec<u8>` — Deterministic JSON for signing/hashing (`claim_id` excluded; see below)
- `digest() -> [u8; 32]` — SHA-256 of canonical bytes (raw bytes, not hex)
- `try_random_claim_id() -> Result<[u8; 16], getrandom::Error>` / `random_claim_id() -> [u8; 16]` — Claim ID generation (the latter falls back to time-based mixing if entropy is unavailable)

## Canonical JSON

`canonical_json(claim)` produces deterministic JSON:
- Sorted keys
- No whitespace
- Null omission (`skip_serializing_if`)
- `claim_id` excluded (each claim instance signs identically regardless of its random ID)

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
    value: String,
}
```

- `sha256(&[u8])` — SHA-256 of raw bytes
- `iscc(&[u8])` — 8-byte truncated content code (`"iscc:<hex>"`)
- `local_fingerprint(&[u8])` — Project-local fingerprint (`"local:<hex>"`)
- `parse(&str) -> Option<Self>` — Parse an `"algorithm:value"` string
- `to_string_value()` — `"sha256:<hex>"` format

## Version

```rust
pub const PROVENANCE_CLAIM_VERSION: u8 = 1;
```

## Usage

1. **Detached manifests** — Claim is the primary content of the manifest, signed by Ed25519 over `canonical_bytes()`
2. **Verification** — The detached path verifies the claim's instance digest against the image bytes

## Relationship to Other Modules

- **`payload_v3`** — V3 payloads do **not** embed `ProvenanceClaim`; they carry a compact header (`dmi_policy` byte, seed, content hash) plus `ExtensionType` TLV extensions (`Ed25519PublicKey`/`Ed25519DetachedSig` for embedded signatures)
- **`detached`** — Detached manifests wrap a `ProvenanceClaim` with signatures
- **`signing`** — Ed25519 signs the claim's canonical bytes on the detached path
- **`util::iscc`** — Content identifiers populate `content_code`
