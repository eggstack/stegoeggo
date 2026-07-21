# Detached Signed Manifest

**Plan:** 023 (Release 5)
**Status:** Specification
**Replaces:** N/A (new format)

## Motivation

Image-embedded payloads have hard capacity limits (32–100 bytes). Distribution systems that need auditable provenance, multi-party signatures, or rich provenance claims cannot fit everything into the image itself. A detached manifest provides a sidecar that:

- Carries a full provenance claim beyond embedded capacity
- Supports public-key signatures (ECDSA, Ed25519) for third-party attestation
- Links to the image via content hashes, surviving embedded-payload stripping
- Works storage-neutral (local sidecar, HTTP header, IPFS, email attachment — anything)

## Terminology

| Term | Meaning |
|------|---------|
| **Manifest** | The complete detached signed JSON object |
| **Claim** | The `ProvenanceClaim` — the canonical provenance assertion being signed |
| **Image** | The protected image file the manifest describes |
| **Embedded reference** | Optional digest of the image-embedded stego payload |
| **Canonical bytes** | Deterministic byte representation used for signing/verification |

## JSON Serialization

Manifests are serialized as canonical JSON. Canonicalization rules:

- Keys are sorted lexicographically at every nesting level
- No trailing commas
- Unicode NFC normalization of all string values
- Numbers are serialized without unnecessary trailing zeros
- Booleans are lowercase (`true`/`false`)
- Null is literal `null`
- Encoding: UTF-8 with no BOM
- Line endings: LF (`\n`)
- Indentation: none (compact canonical) for signing; pretty-printed (2-space indent) for human display

The canonical byte form for signing is the compact canonical JSON (no indentation, sorted keys) encoded as UTF-8.

## CLI Suffix

By convention, detached manifests use the suffix `.stegoeggo.json`. For an image `photo.jpg`, the manifest is `photo.stegoeggo.json`. The CLI uses this suffix when `--manifest` is not explicitly provided.

## Schema

```json
{
  "schema_version": 1,
  "claim": { },
  "signatures": [ ],
  "public_keys": [ ],
  "embedded_reference": null,
  "trust_metadata": null
}
```

### Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `schema_version` | `u8` | Yes | Schema version. Starts at `1`. Reject if > `MAX_MANIFEST_VERSION` (library constant, initially `1`). |
| `claim` | `ProvenanceClaim` | Yes | The canonical provenance claim being attested. See [Provenance Claim](#provenance-claim). |
| `signatures` | `Vec<SignatureRecord>` | Yes | One or more signature records. Empty array is valid (unsigned claim). |
| `public_keys` | `Vec<PublicKeyEntry>` | No | Public keys or key references. Optional; only needed when keys are embedded directly. |
| `embedded_reference` | `EmbeddedReference \| null` | No | Digest of the image-embedded stego payload. Null when no embedded marker exists. |
| `trust_metadata` | `TrustMetadata \| null` | No | Bounded trust metadata. Feature-gated behind `trust-metadata` feature flag. Null when not used. |

## Provenance Claim

The `ProvenanceClaim` is the core data structure being signed. It is a canonical, deterministic representation of the image's provenance and rights posture. See the full specification in `provenance-claim.md` (Plan 023 companion document). The structure is defined here for self-containment.

```json
{
  "image_digest": {
    "algorithm": "sha256",
    "hex": "abcdef0123456789..."
  },
  "iscc": {
    "content": "base58...",
    "data": "base58...",
    "instance": "base58..."
  },
  "protection_seed": 12345678,
  "rights_policy": "prohibited-ai-ml-training",
  "rights_notice": {
    "copyright_holder": "Jane Doe",
    "creator": ["Jane Doe"],
    "license_url": "https://example.com/license",
    "usage_terms": "All rights reserved",
    "ai_constraints": "No AI/ML training",
    "web_statement_of_rights": "https://example.com/rights",
    "contact": "jane@example.com",
    "credit_line": "Jane Doe Photography",
    "copyright_owner": "Jane Doe",
    "licensor_name": "Jane Doe",
    "licensor_email": "jane@example.com",
    "licensor_url": "https://janedoe.com",
    "creation_date": "2026-01-15",
    "metadata_date": "2026-01-15T12:00:00Z",
    "notice_applied_at": "2026-01-15T12:00:00Z"
  },
  "protection_level": "standard",
  "protection_channels": {
    "rights_metadata": true,
    "hidden_marker": "best-effort",
    "authentication": "hmac"
  },
  "created_at": "2026-01-15T12:00:00Z",
  "tool_version": "0.5.0",
  "tool_commit": "abc1234"
}
```

### Claim Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `image_digest` | `ContentDigest` | Yes | SHA-256 (or SHA-512) hash of the original protected image bytes. The manifest's `image_digest` is computed over the image bytes **after** all protection processing (metadata + stego embedded). |
| `iscc` | `Iscc \| null` | No | ISCC content identifiers (non-standard, see `util-iscc.md`). Null when ISCC was not computed. |
| `protection_seed` | `u64` | Yes | The seed used for steganographic embedding and pixel selection. |
| `rights_policy` | `string` | Yes | Canonical rights policy string. Mapped 1:1 from `RightsPolicy` enum. See table below. |
| `rights_notice` | `RightsNotice \| null` | No | Legal/copyright metadata fields. Only fields explicitly provided by the caller are present. Null when no legal metadata was supplied. |
| `protection_level` | `string` | Yes | One of `"disabled"`, `"light"`, `"standard"`. |
| `protection_channels` | `ProtectionChannels` | Yes | Which protection channels were active. |
| `created_at` | `string` | Yes | ISO 8601 timestamp of manifest creation (RFC 3339, UTC). |
| `tool_version` | `string` | Yes | Semver version of the stegoeggo tool/library that generated this claim. |
| `tool_commit` | `string \| null` | No | Git commit SHA of the tool, if available. |

### Rights Policy Strings

| String Value | `RightsPolicy` Enum |
|-------------|---------------------|
| `"unspecified"` | `Unspecified` |
| `"allowed"` | `Allowed` |
| `"prohibited-ai-ml-training"` | `ProhibitedAiMlTraining` |
| `"prohibited-generative-ai-training"` | `ProhibitedGenerativeAiTraining` |
| `"prohibited-except-search-engine-indexing"` | `ProhibitedExceptSearchEngineIndexing` |
| `"prohibited-all-data-mining"` | `ProhibitedAllDataMining` |
| `"prohibited-see-constraints"` | `ProhibitedSeeConstraints` |

### Protection Channel Values

The `hidden_marker` field uses string values:

| Value | `HiddenMarkerMode` |
|-------|-------------------|
| `"disabled"` | `Disabled` |
| `"best-effort"` | `BestEffort` |
| `"tiled"` | `Tiled { tile_size }` (tile_size stored separately) |

The `authentication` field uses string values:

| Value | `AuthenticationMode` |
|-------|---------------------|
| `"none"` | `None` |
| `"hmac"` | `Hmac` |

### Tiled Mode Extension

When `hidden_marker` is `"tiled"`, the claim includes an additional field:

| Field | Type | Description |
|-------|------|-------------|
| `tile_size` | `u32` | The tile size used for crop-resistant embedding. Range: 32–1024. |

## SignatureRecord

Each signature record represents a single cryptographic signature over the canonical claim bytes.

```json
{
  "algorithm_id": "ed25519",
  "key_id": "key-abc123",
  "signature_bytes": "base64url...",
  "signed_at": "2026-01-15T12:01:00Z",
  "signer_info": null
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `algorithm_id` | `string` | Yes | Signature algorithm identifier. See algorithm table below. |
| `key_id` | `string` | Yes | Identifier for the signing key. Matches a `PublicKeyEntry.key_id` when keys are embedded, or a known key out-of-band. |
| `signature_bytes` | `string` | Yes | Base64url-encoded signature bytes (no padding). |
| `signed_at` | `string` | Yes | ISO 8601 timestamp of when the signature was created. |
| `signer_info` | `string \| null` | No | Free-text signer description (e.g., name, role, organization). Not used in verification. |

### Algorithm Identifiers

| `algorithm_id` | Description | Signature Size | Notes |
|----------------|-------------|---------------|-------|
| `"ed25519"` | Ed25519 (RFC 8032) | 64 bytes | Recommended. Fast, constant-time, small keys. |
| `"ecdsa-p256-sha256"` | ECDSA P-256 with SHA-256 (FIPS 186-4) | 64 bytes | Standard curve, wide tooling support. |
| `"ecdsa-p384-sha384"` | ECDSA P-384 with SHA-384 | 96 bytes | Higher security margin. |
| `"hmac-sha256"` | HMAC-SHA256 (shared secret) | 32 bytes | Not a public-key algorithm. Use only for internal/automated signing where key distribution is not a concern. |

**Signing input**: The signer signs the canonical bytes of the `claim` field (the `ProvenanceClaim` JSON object, compact canonical form). Signatures are **not** over the full manifest — they are over the claim alone. This allows claim re-composition without re-signing.

## PublicKeyEntry

Optional entries for embedding public keys directly in the manifest.

```json
{
  "key_id": "key-abc123",
  "algorithm": "ed25519",
  "key_bytes": "base64url...",
  "fingerprint": "sha256:abcdef...",
  "label": null
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `key_id` | `string` | Yes | Unique identifier for this key. Referenced by `SignatureRecord.key_id`. |
| `algorithm` | `string` | Yes | Key algorithm. Must match the `algorithm_id` in referencing signatures. Same values as the algorithm table above. |
| `key_bytes` | `string` | Yes | Base64url-encoded public key bytes. |
| `fingerprint` | `string \| null` | No | Optional key fingerprint (e.g., `"sha256:..."` format). For human identification, not used in verification. |
| `label` | `string \| null` | No | Human-readable label (e.g., "Author's key", "CI signing key"). Not used in verification. |

### Key Encoding

| Algorithm | `key_bytes` Encoding |
|-----------|---------------------|
| Ed25519 | 32-byte public key (RFC 8032) |
| ECDSA P-256 | 65-byte uncompressed point (0x04 prefix) or 33-byte compressed |
| ECDSA P-384 | 97-byte uncompressed point or 49-byte compressed |

## EmbeddedReference

Links the manifest to the image-embedded stego payload.

```json
{
  "payload_digest": {
    "algorithm": "sha256",
    "hex": "abcdef0123456789..."
  },
  "payload_version": 2,
  "payload_size_bytes": 100,
  "stego_algorithm": "lsb",
  "redundancy": 3
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `payload_digest` | `ContentDigest` | Yes | SHA-256 hash of the raw embedded payload bytes (the ECC-encoded or HMAC-authenticated payload, not the full image). |
| `payload_version` | `u8` | Yes | Payload format version (`1` or `2`). |
| `payload_size_bytes` | `u32` | Yes | Size of the embedded payload in bytes (e.g., 100 for ECC V2, 40 for HMAC V2). |
| `stego_algorithm` | `string` | Yes | Steganography algorithm used: `"lsb"`, `"dct-f5"`, or `"qtable-seed"`. |
| `redundancy` | `u8 \| null` | No | Redundancy level used (1–10). Null for tiled mode (per-tile redundancy is always 1). |

### Verification Behavior

- If `embedded_reference` is present, verification checks the embedded payload digest against the actual extracted payload
- If the embedded reference was stripped from the image, verification proceeds but the result notes only detached evidence remains
- If `embedded_reference` is null, the manifest was generated without embedded markers

## TrustMetadata

Bounded, optional trust context. Feature-gated behind `trust-metadata` cargo feature.

```json
{
  "trust_chain": [
    {
      "role": "author",
      "key_id": "key-abc123",
      "attested_at": "2026-01-15T12:01:00Z"
    }
  ],
  "distribution_policy": null,
  "expiry": null
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `trust_chain` | `Vec<TrustRecord>` | Yes | Ordered list of trust attestations. Maximum length: 8. |
| `distribution_policy` | `string \| null` | No | Free-text distribution policy hint (e.g., `"no-cdn"`, `"internal-only"`). Not enforced by the library. |
| `expiry` | `string \| null` | No | ISO 8601 expiry timestamp. Advisory only — the library does not enforce expiry. |

### TrustRecord

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `role` | `string` | Yes | Role of the attesting party: `"author"`, `"publisher"`, `"verifier"`, or a custom string. |
| `key_id` | `string` | Yes | Key that performed the attestation. Must match a `PublicKeyEntry.key_id` or be resolvable out-of-band. |
| `attested_at` | `string` | Yes | ISO 8601 timestamp of attestation. |

## ContentDigest

Shared type for content hashes.

```json
{
  "algorithm": "sha256",
  "hex": "abcdef0123456789..."
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `algorithm` | `string` | Yes | Hash algorithm. Currently supported: `"sha256"`, `"sha512"`. |
| `hex` | `string` | Yes | Lowercase hexadecimal-encoded digest. Length must match algorithm (64 for SHA-256, 128 for SHA-512). |

## Complete JSON Schema Example

```json
{
  "schema_version": 1,
  "claim": {
    "image_digest": {
      "algorithm": "sha256",
      "hex": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
    },
    "iscc": {
      "content": "2tMFpHpR3VzK4ePZ",
      "data": "3kQz8vN7mR2xL5bJ",
      "instance": "3kQz8vN7mR2xL5bJ"
    },
    "protection_seed": 42,
    "rights_policy": "prohibited-ai-ml-training",
    "rights_notice": {
      "copyright_holder": "Jane Doe",
      "creator": ["Jane Doe"],
      "license_url": "https://example.com/license",
      "usage_terms": "All rights reserved",
      "ai_constraints": "No AI/ML training on this image",
      "web_statement_of_rights": "https://example.com/rights",
      "contact": "jane@example.com",
      "credit_line": "Jane Doe Photography",
      "copyright_owner": "Jane Doe",
      "licensor_name": "Jane Doe",
      "licensor_email": "jane@example.com",
      "licensor_url": "https://janedoe.com",
      "creation_date": "2026-01-15",
      "metadata_date": "2026-01-15T12:00:00Z",
      "notice_applied_at": "2026-01-15T12:00:00Z"
    },
    "protection_level": "standard",
    "protection_channels": {
      "rights_metadata": true,
      "hidden_marker": "best-effort",
      "authentication": "hmac"
    },
    "created_at": "2026-01-15T12:00:00Z",
    "tool_version": "0.5.0",
    "tool_commit": "abc1234def56789"
  },
  "signatures": [
    {
      "algorithm_id": "ed25519",
      "key_id": "key-author-001",
      "signature_bytes": "dGVzdHNpZ25hdHVyZQ",
      "signed_at": "2026-01-15T12:01:00Z",
      "signer_info": "Jane Doe"
    },
    {
      "algorithm_id": "ecdsa-p256-sha256",
      "key_id": "key-publisher-001",
      "signature_bytes": "cHVibGlzaGVyc2lnbmF0dXJl",
      "signed_at": "2026-01-15T12:05:00Z",
      "signer_info": "Example Publisher Inc."
    }
  ],
  "public_keys": [
    {
      "key_id": "key-author-001",
      "algorithm": "ed25519",
      "key_bytes": "q4elHlUoGB1xVzPmKjLqH2tN8wR3vY6bA1cD5fG7hI9",
      "fingerprint": "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
      "label": "Jane Doe - Author"
    },
    {
      "key_id": "key-publisher-001",
      "algorithm": "ecdsa-p256-sha256",
      "key_bytes": "BHJk5Qx7eVz3mLp8nT2wY6bA1cD5fG7hI9kM0oP1qR2",
      "fingerprint": "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
      "label": "Example Publisher Inc."
    }
  ],
  "embedded_reference": {
    "payload_digest": {
      "algorithm": "sha256",
      "hex": "deadbeef01234567deadbeef01234567deadbeef01234567deadbeef01234567"
    },
    "payload_version": 2,
    "payload_size_bytes": 100,
    "stego_algorithm": "dct-f5",
    "redundancy": 3
  },
  "trust_metadata": {
    "trust_chain": [
      {
        "role": "author",
        "key_id": "key-author-001",
        "attested_at": "2026-01-15T12:01:00Z"
      },
      {
        "role": "publisher",
        "key_id": "key-publisher-001",
        "attested_at": "2026-01-15T12:05:00Z"
      }
    ],
    "distribution_policy": null,
    "expiry": null
  }
}
```

## Signing Protocol

### Signing (generate)

1. Construct the `ProvenanceClaim` from the protected image and protection context
2. Serialize the claim to compact canonical JSON (sorted keys, no indentation, UTF-8)
3. For each signer: compute signature over the canonical claim bytes using the specified algorithm
4. Assemble the manifest with `schema_version`, serialized claim, signature records, and optional fields
5. Serialize the full manifest to pretty-printed JSON for storage/display

### Verification (validate)

1. Parse and validate `schema_version` — reject if > `MAX_MANIFEST_VERSION`
2. Deserialize the claim
3. Serialize the claim to compact canonical JSON (same canonicalization as signing)
4. For each `SignatureRecord`:
   a. Locate the public key (from `public_keys` by `key_id`, or out-of-band)
   b. Compute expected digest of the canonical claim bytes (SHA-256 for Ed25519/ECDSA)
   c. Verify the signature against the digest using the public key
5. Verify `embedded_reference` (if present):
   a. Extract the stego payload from the image
   b. Compute SHA-256 of the extracted payload bytes
   c. Compare against `payload_digest`
   d. If extraction fails, note that only detached evidence remains
6. Return verification result with per-signature status and embedded reference status

### Detached-Only Verification

When the embedded payload has been stripped from the image but a manifest exists:

1. The image can still be linked to the manifest via `image_digest` (SHA-256 of the image bytes)
2. Signatures on the claim remain valid (the claim is self-contained)
3. The verification result must include a note: `embedded_reference_status: "detached-only"` — indicating the only evidence channel is the detached manifest
4. The `EvidenceChannel::DetachedManifest` variant (new in Release 5) is emitted when verification succeeds via manifest alone

## Size Bounds

The library enforces maximum sizes to prevent abuse:

| Constraint | Limit | Error |
|-----------|-------|-------|
| Total manifest JSON size | 64 KiB | `Error::Manifest` |
| `claim` serialized size | 32 KiB | `Error::Manifest` |
| `signatures` array length | 16 entries | `Error::Manifest` |
| `public_keys` array length | 16 entries | `Error::Manifest` |
| `trust_chain` array length | 8 entries | `Error::Manifest` |
| Single `signature_bytes` | 256 bytes (decoded) | `Error::Manifest` |
| Single `key_bytes` | 512 bytes (decoded) | `Error::Manifest` |
| String field max length | 4096 bytes (UTF-8) | `Error::Manifest` |
| Nesting depth | 8 levels | `Error::Manifest` |
| `rights_notice` fields | 15 (max defined) | `Error::Manifest` |

These limits are library constants, not protocol-mandated. A future schema version may relax them.

## Error Handling

All manifest errors use `Error::Manifest` with a descriptive message:

| Condition | Message |
|-----------|---------|
| Schema version too high | `"unsupported manifest schema version: {version}"` |
| Claim missing | `"manifest missing required claim"` |
| Signature array empty | (valid — unsigned claims are permitted) |
| Signature size mismatch | `"signature size mismatch for algorithm {alg}"` |
| Key size mismatch | `"key size mismatch for algorithm {alg}"` |
| JSON parse failure | `"invalid manifest JSON: {details}"` |
| Nesting depth exceeded | `"manifest nesting depth exceeds limit of {max}"` |
| Size limit exceeded | `"manifest field '{field}' exceeds maximum size"` |
| Digest length mismatch | `"digest length {len} does not match algorithm {alg}"` |

## Feature Gating

| Feature Flag | What It Enables | Default |
|-------------|----------------|---------|
| `trust-metadata` | `TrustMetadata` deserialization and validation | Off (off by default) |
| `ed25519` | Ed25519 signature verification via `ed25519-dalek` | On |
| `ecdsa` | ECDSA P-256/P-384 verification via `p256`/`p384` crates | Off |

Without the `trust-metadata` feature, `TrustMetadata` is deserialized as `null` and ignored. This keeps the default dependency set small.

## Migration from Embedded-Only

Images processed by earlier versions (before Release 5) have no detached manifest. To add one:

1. Re-process the image with Release 5+ to generate the embedded markers
2. Extract the claim from the processing context
3. Sign the claim with the desired key(s)
4. Write the manifest as `{image_stem}.stegoeggo.json`

The manifest can be generated post-hoc for any protected image by extracting metadata from the image itself and constructing the claim. The `image_digest` is computed over the final protected image bytes.

## Storage-Neutral Design

The manifest format makes no assumptions about how it is stored or distributed:

| Distribution Channel | Manifest Location |
|---------------------|-------------------|
| Local filesystem | `{image}.stegoeggo.json` sidecar |
| HTTP | Custom header (`X-StegoEggo-Manifest`) or `/.well-known/` path |
| IPFS | Separate CID, linked via CIDs in both image and manifest |
| Email attachment | Attached alongside the image |
| Database | Stored as a JSON blob, keyed by image digest |
| CDN edge | Embedded in response headers or separate URL |

The library can serialize to bytes (`to_json()`) and deserialize from bytes (`from_json()`). The caller handles transport.

## Backward Compatibility

- `schema_version: 1` is the only defined version
- Future versions may add new fields to the claim or new record types
- New fields in the claim are always optional — verification must not fail on unknown fields
- New signature algorithm IDs may be added without bumping the schema version
- The `public_keys` array may be empty or absent in v1 manifests (keys resolved out-of-band)
- Unknown top-level fields in the JSON must be ignored by parsers (forward compatibility)
