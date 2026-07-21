# Provenance Claim — Unified Serializable Model

**Status:** Design specification (Plan 023, Release 5)
**Purpose:** Canonical, deterministic serializable type for rights/provenance assertions about an image, shared by embedded payloads and detached manifests.

## Motivation

Embedded payloads and detached manifests currently encode overlapping but structurally different information. Release 5 unifies them behind a single `ProvenanceClaim` type. Embedded payloads carry a compact subset of extraction-essential fields; detached manifests carry the full field set including large metadata (notice text, creator, etc.). Both derive from the same schema, share the same canonical serialization, and can be signed with the same algorithm.

## Design Principles

1. **Canonical and deterministic.** Given identical inputs, any implementation produces the same JSON bytes. This is a hard requirement for signing.
2. **Forward-compatible.** A schema version field allows adding fields without breaking existing consumers. Unknown fields are ignored by tolerant parsers.
3. **Compact embedded.** Embedded payloads carry only the fields needed for extraction and verification. The full claim is reconstructible from the embedded claim plus the image bytes.
4. **Explicit algorithm identifiers.** Every digest is prefixed with its algorithm (e.g. `sha256:abcd...`) so consumers never guess the hash function.

## Schema Version

The current schema version is `1`. All fields defined in this document are present in version 1. Future versions may add optional fields; existing fields must never change meaning or encoding.

## Field Table

| # | Field | Type | Size | Required | Description |
|---|-------|------|------|----------|-------------|
| 1 | `schema_version` | u8 | 1 | always | Schema version. Currently `1`. |
| 2 | `claim_id` | bytes | 16 | always | Random UUID v4 (or 16 random bytes). Unique identifier for this claim instance. |
| 3 | `rights_policy` | u8 | 1 | always | Discriminant of the rights/data-mining policy. See [Rights Policy Discriminant](#rights-policy-discriminant). |
| 4 | `notice_digest` | string | variable | always | `"sha256:<hex>"` — SHA-256 of the normalized rights-notice text (NFC-normalized, trimmed). Empty string when no notice is provided. |
| 5 | `content_code` | string | variable | always | Perceptual/content identifier. `"iscc:<hex>"` for ISCC-derived codes (8 bytes, 16 hex chars) or `"local:<hex>"` for project-local DCT hashes. |
| 6 | `instance_digest` | string | variable | always | `"sha256:<hex>"` — SHA-256 of the exact file bytes this claim was generated from. 32 bytes, 64 hex chars. |
| 7 | `format` | string | variable | always | Image format identifier: `"png"`, `"jpeg"`, or `"webp"`. |
| 8 | `width` | u32 | 4 | always | Image width in pixels. |
| 9 | `height` | u32 | 4 | always | Image height in pixels. |
| 10 | `file_size` | u64 | 8 | always | Original file size in bytes. |
| 11 | `created_at` | u64 | 8 | always | Unix epoch seconds when this claim was created/signed. |
| 12 | `issuer_id` | string | variable | always | Base64url-encoded issuer or key identifier. Max 32 raw bytes (44 base64url chars). |
| 13 | `software` | string | variable | always | Software identifier and version, e.g. `"stegoeggo/0.5.0"`. |
| 14 | `parent_claim_id` | string | null | optional | Base64url-encoded 16-byte claim ID of a parent claim. `null` when absent. Used for claim chains (e.g. derivative works). |
| 15 | `statement_uri` | string | null | optional | URI to an external rights statement or license. Length-prefixed in binary encoding; `null` in JSON when absent. Max 2048 bytes. |

### Field Ordering

Fields are always serialized in the order shown above (1–15). This order is the canonical field order and must not be changed. Canonical JSON sorts keys lexicographically (see [Canonical Serialization](#canonical-serialization)), so the natural key order matches the field order for these specific keys.

## Rights Policy Discriminant

The `rights_policy` field is a single byte encoding the rights/data-mining policy:

| Byte | Variant | PLUS LDF Key |
|------|---------|--------------|
| `0x00` | Unspecified | `DMI-UNSPECIFIED` |
| `0x01` | Allowed | `DMI-ALLOWED` |
| `0x02` | ProhibitedAiMlTraining | `DMI-PROHIBITED-AIMLTRAINING` |
| `0x03` | ProhibitedGenAiMlTraining | `DMI-PROHIBITED-GENAIMLTRAINING` |
| `0x04` | ProhibitedExceptSearchEngineIndexing | `DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING` |
| `0x05` | ProhibitedAllDataMining | `DMI-PROHIBITED` |
| `0x06` | ProhibitedSeeConstraints | `DMI-PROHIBITED-SEECONSTRAINT` |

Values `0x07`–`0xFF` are reserved for future use. Parsers must reject claims with unknown policy bytes.

This mapping aligns with `RightsPolicy::to_byte()` / `RightsPolicy::from_byte()` in `src/types.rs` and the PLUS controlled-vocabulary keys in `DmiValue::plus_vocab_key()`.

## Digest Encoding

All digests use the format `<algorithm>:<hex-lowercase>`:

| Algorithm | Prefix | Hex Length | Raw Size |
|-----------|--------|------------|----------|
| SHA-256 | `sha256:` | 64 | 32 bytes |
| ISCC content (truncated) | `iscc:` | 16 | 8 bytes |
| Project-local DCT hash | `local:` | 16 | 8 bytes |

Examples:
- `"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"` (empty-input SHA-256)
- `"iscc:a1b2c3d4e5f6a7b8"` (8-byte truncated content code)
- `"local:f0e1d2c3b4a59687"` (project-local 8-byte code)

Parsers must reject digests with unknown algorithm prefixes.

## Canonical Serialization

The canonical byte representation of a `ProvenanceClaim` is **canonical JSON**:

1. **Keys sorted lexicographically** (Unicode code-point order, byte-level comparison of UTF-8 encoded keys).
2. **No whitespace** between tokens.
3. **UTF-8 encoding** (no BOM).
4. **Null values omitted.** `parent_claim_id` and `statement_uri` are not present in the JSON when null. This ensures two claims with identical non-null fields serialize identically regardless of whether optional fields were explicitly set to null.
5. **Numbers encoded without quotes.** `schema_version`, `rights_policy`, `width`, `height`, `file_size`, and `created_at` are JSON numbers, not strings.
6. **Strings are NFC-normalized** before serialization. Unicode normalization is applied to all string values (notice text is already normalized before hashing, but this rule applies to all fields).

The canonical JSON object is then serialized to a byte string using the above rules. The resulting bytes are what gets signed.

### Canonical Key Order

For the current field set, lexicographic key order is:

```
content_code
created_at
file_size
format
height
instance_digest
issuer_id
notice_digest
parent_claim_id      (omitted when null)
rights_policy
schema_version
software
statement_uri        (omitted when null)
width
```

Note: The field numbering (1–15 in the field table) is a specification convenience. The canonical serialization key order is lexicographic, which happens to partially coincide but is the authoritative order for signing.

## Embedded vs Detached

| Aspect | Embedded Payload | Detached Manifest |
|--------|-----------------|-------------------|
| **Purpose** | Extraction-essential fields for in-band verification | Full provenance record with large metadata |
| **Fields carried** | `schema_version`, `claim_id`, `rights_policy`, `notice_digest`, `content_code`, `instance_digest`, `created_at`, `software` | All 15 fields |
| **Large metadata** | Not included (notice text, creator, etc. live in image metadata channels) | Included in the manifest envelope (not inside the claim itself; the claim references them via digests) |
| **Signing** | Canonical JSON of embedded fields, signed with issuer key | Canonical JSON of full claim, signed with issuer key |
| **Verification** | Extract claim from payload → verify signature → verify digests against image bytes and metadata | Deserialize manifest → verify signature → verify digests |

The `ProvenanceClaim` type itself always defines all 15 fields. The decision of which fields to populate is made at the call site. Embedded payloads populate a strict subset; detached manifests populate all fields. The canonical JSON for signing includes only the fields that are present (non-null).

## Binary Encoding (Embedded Payload)

When embedded in a steganographic payload, the claim is serialized as:

| Offset | Size | Field |
|--------|------|-------|
| 0 | 1 | `schema_version` |
| 1 | 16 | `claim_id` |
| 17 | 1 | `rights_policy` |
| 18 | 2 | `notice_digest_len` (u16 big-endian, 0 when absent) |
| 20 | `notice_digest_len` | `notice_digest` (UTF-8 bytes) |
| 20+N | 2 | `content_code_len` (u16 big-endian) |
| 22+N | `content_code_len` | `content_code` (UTF-8 bytes) |
| 22+N+M | 2 | `instance_digest_len` (u16 big-endian) |
| 24+N+M | `instance_digest_len` | `instance_digest` (UTF-8 bytes) |
| ... | ... | `format_len` (u8) + `format` (UTF-8) |
| ... | 4 | `width` (u32 big-endian) |
| ... | 4 | `height` (u32 big-endian) |
| ... | 8 | `file_size` (u64 big-endian) |
| ... | 8 | `created_at` (u64 big-endian) |
| ... | 1 | `issuer_id_len` (u8) |
| ... | `issuer_id_len` | `issuer_id` (raw bytes) |
| ... | 1 | `software_len` (u8) |
| ... | `software_len` | `software` (UTF-8) |
| ... | 1 | `parent_present` (0x00 = absent, 0x01 = present) |
| ... | 16 (if present) | `parent_claim_id` (raw bytes) |
| ... | 1 | `statement_present` (0x00 = absent, 0x01 = present) |
| ... | 2 (if present) | `statement_uri_len` (u16 big-endian) |
| ... | `statement_uri_len` (if present) | `statement_uri` (UTF-8) |

The binary format is **not** canonical and is not used for signing. It is only used for compact embedding. The canonical JSON is used for signing and detached manifests.

## Canonical JSON Example

```json
{
  "content_code":"iscc:a1b2c3d4e5f6a7b8",
  "created_at":1753084800,
  "file_size":204800,
  "format":"png",
  "height":600,
  "instance_digest":"sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
  "issuer_id":"c2hlbHRhZ28tbWFzdGVyLWtleQ",
  "notice_digest":"sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abcd",
  "rights_policy":2,
  "schema_version":1,
  "software":"stegoeggo/0.5.0",
  "width":800
}
```

Note: `parent_claim_id` and `statement_uri` are omitted (null). The JSON is compact (no whitespace). Keys are sorted lexicographically.

## Test Vector Format

Test vectors are stored as JSON objects in a JSON array. Each test vector includes:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Human-readable test vector name |
| `input` | object | Field values used to construct the claim |
| `canonical_json` | string | Expected canonical JSON byte string (the exact bytes to sign) |
| `canonical_hex` | string | Hex encoding of `canonical_json` for byte-level comparison |
| `signature_hex` | string | Hex-encoded signature over `canonical_json` (test-only key) |

### Test Vector File

```json
[
  {
    "name": "minimal-claim-no-optional-fields",
    "input": {
      "schema_version": 1,
      "claim_id": "01234567-89ab-cdef-0123-456789abcdef",
      "rights_policy": 2,
      "notice_digest": "sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abcd",
      "content_code": "iscc:a1b2c3d4e5f6a7b8",
      "instance_digest": "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
      "format": "png",
      "width": 800,
      "height": 600,
      "file_size": 204800,
      "created_at": 1753084800,
      "issuer_id": "c2hlbHRhZ28tbWFzdGVyLWtleQ",
      "software": "stegoeggo/0.5.0"
    },
    "canonical_json": "{\"content_code\":\"iscc:a1b2c3d4e5f6a7b8\",\"created_at\":1753084800,\"file_size\":204800,\"format\":\"png\",\"height\":600,\"instance_digest\":\"sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08\",\"issuer_id\":\"c2hlbHRhZ28tbWFzdGVyLWtleQ\",\"notice_digest\":\"sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abcd\",\"rights_policy\":2,\"schema_version\":1,\"software\":\"stegoeggo/0.5.0\",\"width\":800}",
    "canonical_hex": "7b22636f6e74656e745f636f6465223a22697363633a61316232633364346535663661376238222c22637265617465645f6174223a313735333038343830302c2266696c655f73697a65223a3230343830302c22666f726d6174223a22706e67222c22686569676874223a3630302c22696e7374616e63655f646967657374223a227368613235363a39663836643038313838346337643635396132666561613063353561643031356133626634663162326230623832326364313564366331356230663030613038222c226973737565725f6964223a226332686c624852685a3238746257467a644745796257467a64475579625754647a644755796257553030222c226e6f746963655f646967657374223a227368613235363a61626331323364656634353661626331323364656634353661626331323364656634353661626331323364656634353661626331323364656634353661626364222c227269676874735f706f6c696379223a322c22736368656d615f76657273696f6e223a312c22736f667477617265223a22737465676f6567676f2f302e352e30222c227769647468223a3830307d",
    "signature_hex": "TODO: compute with test key"
  },
  {
    "name": "full-claim-with-parent-and-statement",
    "input": {
      "schema_version": 1,
      "claim_id": "01234567-89ab-cdef-0123-456789abcdef",
      "rights_policy": 5,
      "notice_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
      "content_code": "iscc:0000000000000000",
      "instance_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
      "format": "jpeg",
      "width": 1920,
      "height": 1080,
      "file_size": 512000,
      "created_at": 1753084900,
      "issuer_id": "dGVzdC1pc3N1ZXI",
      "software": "stegoeggo/0.5.0",
      "parent_claim_id": "AQIDBAUGBwgJmtCvlBRNnQ",
      "statement_uri": "https://example.com/license/cc-by-4.0"
    },
    "canonical_json": "{\"content_code\":\"iscc:0000000000000000\",\"created_at\":1753084900,\"file_size\":512000,\"format\":\"jpeg\",\"height\":1080,\"instance_digest\":\"sha256:0000000000000000000000000000000000000000000000000000000000000000\",\"issuer_id\":\"dGVzdC1pc3N1ZXI\",\"notice_digest\":\"sha256:0000000000000000000000000000000000000000000000000000000000000000\",\"parent_claim_id\":\"AQIDBAUGBwgJmtCvlBRNnQ\",\"rights_policy\":5,\"schema_version\":1,\"software\":\"stegoeggo/0.5.0\",\"statement_uri\":\"https://example.com/license/cc-by-4.0\",\"width\":1920}",
    "canonical_hex": "TODO",
    "signature_hex": "TODO: compute with test key"
  }
]
```

### Test Vector Construction

When computing `canonical_hex`:

1. Construct the canonical JSON object from `input` with nulls omitted.
2. Serialize with `serde_json::to_string()` (no whitespace, which `serde_json` does by default).
3. Encode the resulting byte string as lowercase hex.

When computing `canonical_json`:

1. Take the raw bytes from step 2 above.
2. The `canonical_json` field in the test vector is the string representation of those bytes (i.e. the JSON string itself).

The two fields are redundant but serve different verification purposes: `canonical_json` for human readability, `canonical_hex` for byte-exact comparison.

### Signature Computation

Test vectors use a fixed ed25519 keypair (deterministic, derived from a seed). The signature is computed over the raw canonical JSON bytes using ed25519 sign-then-base64url:

```
signature = base64url(ed25519_sign(canonical_json_bytes, test_private_key))
```

The `signature_hex` field contains the hex encoding of the raw 64-byte ed25519 signature.

## Determinism Properties

Two claims are canonically identical if and only if:

1. All present (non-null) fields have identical values.
2. Both omit the same optional fields.
3. String values are NFC-normalized and encoded as UTF-8.
4. Numbers are within the same integer range (no floating-point).

Implementation must ensure:

- `serde_json::to_string()` produces deterministic output for the same `serde_json::Value`.
- `claim_id` and `created_at` are the only non-deterministic fields (caller-supplied).
- `notice_digest` is computed from NFC-normalized notice text.
- `content_code` is derived from the image via a deterministic algorithm.
- `instance_digest` is SHA-256 of exact file bytes (caller-supplied, not computed by the claim type itself).

## Migration from Current Payload Format

Current stego payloads (v2) carry a 24-byte compact header. The provenance claim is a new payload format that replaces this header in v3:

| Version | Format | Signing |
|---------|--------|---------|
| v1 (legacy) | 24-byte fixed header | None |
| v2 (current) | 24-byte header + optional HMAC | HMAC-SHA256 |
| v3 (planned) | Provenance claim (JSON or binary) | ed25519 or HMAC-SHA256 |

v3 payloads embed the binary-encoded claim in the steganographic layer. Extraction reconstructs the claim, verifies the signature, and checks all digests. Backward compatibility is maintained: v3 extractors must also extract v1/v2 payloads.

## Open Questions

1. **Binary vs JSON for embedded.** The binary encoding adds ~6 bytes of length prefixes over the JSON encoding for a typical claim. JSON is simpler to debug but larger. Decision: use binary for embedded (compactness matters for stego capacity), JSON for detached manifests.
2. **Ed25519 vs HMAC for signing.** Ed25519 provides non-repudiation; HMAC provides symmetric authentication. The claim supports both: `issuer_id` identifies the key, and the payload header indicates the signature algorithm.
3. **Claim chain depth limit.** The `parent_claim_id` field allows chains but no depth limit is specified yet. A depth limit of 8 may be appropriate to prevent abuse.
