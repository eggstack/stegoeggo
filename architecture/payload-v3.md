# StegoEggo Payload v3 — Normative Wire-Format Specification

**Status:** Draft (Plan 023, Release 5)
**Supersedes:** Payload v1 (24-byte header), Payload v2 (32-byte header)
**Date:** 2026-07-21

## 1. Introduction

This document specifies the binary wire format of the StegoEggo hidden payload
version 3. It is the normative reference for encoding, decoding, and verifying
v3 payloads. All multi-byte integers are little-endian unless stated otherwise.

Payload v3 is designed to be extensible, self-describing, and capable of
carrying richer protection metadata while staying within steganographic
capacity limits (target: 32–64 bytes core, ≤ 256 bytes total embedded).

### 1.1. Relationship to Prior Versions

| Property | v1 | v2 | v3 |
|---|---|---|---|
| Header size | 24 bytes (fixed) | 32 bytes (fixed) | 32 bytes core + variable extensions |
| Magic bytes | none | none | `0x53 0x45` ("SE") |
| Extensibility | none | reserved bytes | TLV extension section |
| Auth coverage | header bytes only | header bytes only | domain-separated: header + context string |
| ECC payload size | 76 bytes | 100 bytes | configurable (see §6) |
| MAC payload size | 32 bytes | 40 bytes | variable (see §5.5) |

V1 and V2 remain supported for extraction. V3 parsers MUST also accept v1/v2
payloads. The parser tries versions in order: v3 → v2 → v1 (see
`SUPPORTED_PAYLOAD_VERSIONS`).

## 2. Byte-Level Layout

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         Magic (0x53, 0x45)    |  Version (=3) | Header Length |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        Total Length (u16 LE)  |         Flags (u16 LE)        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Protection Channels (u16 LE bitmask)   |  DMI Policy (u8) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                       Seed (u64 LE, 8 bytes)                  |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        Intensity (u16 LE, scaled)  | Content Hash (8 bytes ...)  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Content Hash (cont.)  | Auth Algo (u8) | Auth Tag Len (u8) | Key ID Len (u8) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|              Key ID (0..32 bytes) + Extension Section (TLV) ... |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                 Authentication Tag (variable)                 |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

Wire order after `content_hash` is `auth_algorithm`, `auth_tag_len`,
`key_id_len` (offsets 29/30/31 in `PayloadV3Header::to_bytes` /
`from_bytes`, `src/payload_v3/header.rs`), then key ID, then extensions,
then the auth tag.

### 2.1. Field Summary

| Offset | Size | Field | Type | Description |
|--------|------|-------|------|-------------|
| 0 | 2 | `magic` | `[u8; 2]` | Domain separator: `0x53, 0x45` ("SE") |
| 2 | 1 | `version` | `u8` | Payload version = `3` |
| 3 | 1 | `header_length` | `u8` | Byte offset from start to authentication tag |
| 4 | 2 | `total_length` | `u16 LE` | Total embedded payload size in bytes |
| 6 | 2 | `flags` | `u16 LE` | Feature flags (see §3) |
| 8 | 2 | `channels` | `u16 LE` | Protection channel bitmask (see §4) |
| 10 | 1 | `dmi_policy` | `u8` | DMI rights-policy discriminant (see §5.1) |
| 11 | 8 | `seed` | `u64 LE` | Extraction seed for pixel/DCT permutation |
| 19 | 2 | `intensity` | `u16 LE` | Embedding intensity, `f32 * 100.0` |
| 21 | 8 | `content_hash` | `[u8; 8]` | Truncated content/instance hash (see §5.3) |
| 29 | 1 | `auth_algorithm` | `u8` | Authentication algorithm ID (see §5.5) |
| 30 | 1 | `auth_tag_len` | `u8` | Length of auth tag/signature in bytes |
| 31 | 1 | `key_id_len` | `u8` | Length of `key_id` in bytes (0–32) |
| 32 | 0–32 | `key_id` | `bytes` | Key identifier (see §5.4) |
| 32+N | M | `extensions` | TLV… | Extension section (see §7) |
| 32+N+M | T | `auth_tag` | `bytes` | Authentication tag (see §5.6) |

Where `N = key_id_len` and `M + T = total_length - 32 - N`.

## 3. Flags

The 16-bit flags field encodes feature and policy signals.

| Bit(s) | Name | Description |
|--------|------|-------------|
| 0 | `HAS_EXTENSIONS` | Extension section is non-empty |
| 1 | `HAS_KEY_ID` | `key_id_len > 0` |
| 2 | `TILED` | Payload is embedded in tiled mode (crop-resistant) |
| 3 | `PROGRESSIVE_JPEG` | Source was progressive JPEG (seed-in-Q-tables path) |
| 4–7 | reserved | Reserved, MUST be 0 in encoding; ignored on read |
| 8 | `CRITICAL_EXTENSION` | If set, unknown extensions cause verification failure |
| 9 | `SIGNED` | `auth_algo` is a signature scheme (Ed25519), not MAC |
| 10–15 | reserved | Reserved, MUST be 0 in encoding; ignored on read |

Implementations MUST NOT set reserved bits. Implementations MUST mask unknown
upper bits when reading flags (forward compatibility).

## 4. Protection Channels

A 16-bit bitmask identifying which protection channels are active. Bit 0 is
the LSB.

| Bit | Channel | Description |
|-----|---------|-------------|
| 0 | `RIGHTS_METADATA` | XMP/EXIF/IPTC legal metadata injected |
| 1 | `HIDDEN_MARKER` | Steganographic payload embedded (LSB or DCT) |
| 2 | `AUTHENTICATED` | Payload has HMAC or signature verification |
| 3–15 | reserved | Assigned by future spec; 0 in encoding |

If `HIDDEN_MARKER` is clear, the payload exists in metadata-only form and
`auth_tag` MAY be empty (auth_algo = 0).

## 5. Core Fields

### 5.1. DMI Policy (`dmi_policy`)

One-byte discriminant of the rights policy. Values map 1:1 to `DmiValue`:

| Value | Meaning |
|-------|---------|
| 0 | `Unspecified` |
| 1 | `Allowed` |
| 2 | `ProhibitedAiMlTraining` |
| 3 | `ProhibitedGenAiMlTraining` |
| 4 | `ProhibitedExceptSearchEngineIndexing` |
| 5 | `Prohibited` |
| 6 | `ProhibitedSeeConstraints` |
| 7–255 | Reserved (rejected at parse time) |

This is a *snapshot* of the policy at embed time. It is NOT a mutable
policy copy — the authoritative policy lives in metadata channels. The
`dmi_policy` byte is used for fast stego-only verification without metadata
extraction.

### 5.2. Seed

8-byte little-endian unsigned integer. Used to derive:
- LSB pixel-selection permutation (`stego_permutation`)
- F5 DCT coefficient shuffling order
- Per-tile seed when tiled mode is active (`tile_seed(master_seed, tile_x, tile_y)`)

The seed is also stored in metadata channels (XMP
`stegoeggo:ProtectionSeed`, PNG `X-Protection-Seed` tEXt, JPEG COM markers,
JPEG quantization-table LSBs) for extraction when stego is damaged.

### 5.3. Content Hash

8 bytes — first 8 bytes of a truncated content hash. Used for instance binding
and tamper detection. The hash source is implementation-defined (truncated
ISCC or SHA-256 of the original pixel data).

All-zero bytes indicate no content binding.

### 5.4. Key Identifier (`key_id`)

Length-prefixed field:
- `key_id_len` (1 byte): 0–32. Length of the key identifier.
- `key_id` (0–32 bytes): Opaque key identifier. Conventionally the first
  N bytes of `SHA-256(key)` for HMAC keys, or the Ed25519 public key
  fingerprint.

The key identifier allows extractors to select the correct verification key
without trial. When `key_id_len = 0`, no key identifier is stored (the only
shape the protection pipeline emits today).

`PayloadBuilder` sets the `HAS_KEY_ID` flag automatically from
`!key_id.is_empty()`; the V3 header parser enforces `HAS_EXTENSIONS` ↔
header-length consistency but does NOT enforce `HAS_KEY_ID` consistency.

### 5.5. Authentication Algorithm (`auth_algo`)

| Value | Algorithm | Auth Tag Size |
|-------|-----------|---------------|
| 0 | None | 0 bytes |
| 1 | CRC32 | 4 bytes |
| 2 | HMAC-SHA256-truncated | 16 bytes |
| 3 | Ed25519 | 64 bytes |

Implementations MUST support at least `auth_algo = 0` (none) and
`auth_algo = 2` (HMAC-SHA256-truncated). `auth_algo = 1` (CRC32) is
deprecated but allowed for backward compatibility.

`auth_tag_len` MUST match the expected size for `auth_algo`. For
HMAC-SHA256-truncated, `auth_tag_len` is always 16 (the 16 most significant
bytes of the 32-byte HMAC output, providing 128-bit security).

### 5.6. Authentication Tag

Immediately follows the extension section. Length = `auth_tag_len` bytes.
Position within the payload is: `header_length` is the offset from byte 0
to the first byte of `auth_tag`.

## 6. Payload Sizes

### 6.1. Core (Minimum) Payload

The minimum v3 payload with no key_id, no extensions, no auth:

```
[2 magic] + [1 version] + [1 header_len] + [2 total] + [2 flags]
+ [2 channels] + [1 dmi] + [8 seed] + [2 intensity] + [8 content_hash]
+ [1 key_id_len=0] + [1 auth_algo=0] + [1 auth_tag_len=0]
= 32 bytes core
```

### 6.2. Typical HMAC Payload (application-emitted)

The rights-protection pipeline (`build_v3_payload`,
`src/protected/steganography/marker.rs`) currently emits no key ID and no
extensions (enforced by `debug_assert`/`assert`), so the only emitted sizes
are the two below (`V3_CRC_PAYLOAD_SIZE = 36`, `V3_HMAC_PAYLOAD_SIZE = 48`
in `src/protected/steganography/mod.rs`):

```
32 bytes core + 16 bytes HMAC-SHA256-truncated = 48 bytes total
32 bytes core + 4 bytes CRC32 = 36 bytes total
```

The generic `PayloadBuilder` (`src/payload_v3/writer.rs`) can additionally
emit key IDs (0–32 bytes) and TLV extensions, e.g. 32 + 16 key ID + 16 tag =
64 bytes, but that shape is not produced by the protection pipeline today.

### 6.3. Maximum Embedded Payload

For steganographic embedding, capacity is limited:
- **LSB (PNG/WebP):** Image-dependent. `STEGO_SPREAD_FACTOR=5` means each
  payload bit uses 5 pixels. A 100×100 image has 30,000 LSB slots →
  30,000 / 5 / 8 ≈ 750 bytes. Typical images provide 100–400 bytes.
- **DCT (JPEG):** Depends on image size and quantization. Typical 640×480
  JPEG provides 200–600 bytes.

**`V3_MAX_EMBEDDED_SIZE = 256` bytes.** Encoders MUST reject payloads exceeding
this limit. Encoders SHOULD target ≤ 64 bytes for reliable embedding.

### 6.4. Legacy ECC-Encoded Sizes (V1/V2 only)

ECC 3× replication (`src/protected/ecc.rs`) is legacy-only: `ecc_encode` is
`#[allow(dead_code)]` extraction-compat and V3 application payloads carry
their CRC32/HMAC tag inline with no replication. The table below is the V1/V2
carrier shape, not emitted V3:

| Mode | Core bytes | ECC bytes | + CRC32 | Total |
|------|-----------|-----------|---------|-------|
| V1 (24-byte header) | 24 | 72 | +4 | 76 |
| V2 (32-byte header) | 32 | 96 | +4 | 100 |

Emitted V3 is always 36 bytes (CRC32) or 48 bytes (HMAC); see §6.2.

## 7. Extension Section (TLV)

Extensions follow the core header. Each extension is encoded as:

```
+--------+--------+--------+--------+------------------+
| Type   | Length |         Value (Length bytes)       |
| u16 LE | u16 LE |         (N bytes)                  |
+--------+--------+--------+--------+------------------+
```

### 7.1. Extension Type Allocations

| Type Range | Category | Criticality |
|------------|----------|-------------|
| 0x0000 | Reserved (invalid) | — |
| 0x0001–0x00FF | Standard extensions | Per-type |
| 0x0100–0x01FF | Experimental / private-use | Non-critical |
| 0x0200–0xFFFE | Reserved | — |
| 0xFFFF | End-of-extensions sentinel | — |

### 7.2. Standard Extensions (Initial Registry)

| Type | Name | Criticality | Description |
|------|------|-------------|-------------|
| 0x0001 | `TIMESTAMP` | Non-critical | 8-byte u64 LE, Unix epoch seconds |
| 0x0002 | `CREATOR_FINGERPRINT` | Non-critical | 32-byte truncated SHA-256 of creator identity |
| 0x0003 | `INSTANCE_ID` | Non-critical | 16-byte random instance UUID |
| 0x0004 | `LEGAL_NOTICE_HASH` | Non-critical | 8-byte truncated hash of the legal notice text |
| 0x0005 | `VERSION_STRING` | Non-critical | UTF-8 string, semver of the producing implementation |
| 0x0006 | `PROCESSING_HISTORY` | Non-critical | Array of 1-byte operation codes |
| 0x0010 | `ED25519_PUBLIC_KEY` | Non-critical | 32-byte Ed25519 public key (when embedded key_id is insufficient) |
| 0x0011 | `ED25519_DETACHED_SIG` | Critical | 64-byte Ed25519 signature (when capacity insufficient for inline) |
| 0x00FF | `VENDOR_PREFIX` | Non-critical | 1-byte vendor ID + vendor-defined payload |

### 7.3. Criticality Rules

- The `CRITICAL_EXTENSION` flag (bit 8 of `flags`) controls the overall
  criticality policy:
  - **If set:** Unknown extension types cause `verify_payload` to return
    `VerificationStatus::Invalid`.
  - **If clear:** Unknown extension types are silently skipped.

- Individual extensions do NOT carry their own criticality bit. The flag
  is global. This is a deliberate simplification for the embedded context.

- Extensions in the `0x0100–0x01FF` range are always treated as
  non-critical regardless of the flag.

### 7.4. Extension Size Limits

- **Maximum total extension size:** 128 bytes. Encoders MUST NOT emit
  extensions exceeding this total. The 256-byte global payload cap
  provides headroom for core + extensions + auth.
- **Maximum single extension value:** 128 bytes.
- **Maximum extensions count:** 32. Decoders MUST skip (not reject)
  payloads with > 32 extensions if the flag is clear.

### 7.5. Encoding Rules (spec SHOULD vs implementation IS)

1. Extensions SHOULD be sorted by type in ascending order (canonical ordering
   for signing). `PayloadBuilder::build` (`src/payload_v3/writer.rs`) emits in
   insertion order and does not sort or reject duplicates at build time.
2. Duplicate extension types are rejected at parse time (except the
   `0x0100–0x01FF` private-use range, which never dedups), not at build time.
3. `type = 0xFFFF` is the end-of-extensions sentinel. Parsers treat trailing
   `0xFF` bytes as padding/sentinel area; any other trailing byte is
   `ExtensionsTooLarge`.
4. Padding/sentinel bytes are `0xFF`, not zeroed: the parser requires every
   byte past the last extension (and past the 32-extension cap) to be `0xFF`.

### 7.6. Decoding Rules

1. Start at offset `header_length_start` (30 + `key_id_len` + 2 + 1 = 32 +
   `key_id_len` for the first extension byte).
2. Read 4-byte TLV header (type u16 LE, length u16 LE).
3. If `type == 0xFFFF`: stop, remaining bytes before `auth_tag` are padding.
4. If `length > remaining_bytes`: error (malformed).
5. If `type` is unknown and `CRITICAL_EXTENSION` flag is set: error.
6. If `type` is unknown and flag is clear: skip `length` bytes, continue.
7. If `type == 0x0100–0x01FF`: skip (private-use, always non-critical).
8. Accumulate total extension bytes; if > 128: error.

## 8. Authentication

### 8.1. Domain Separation

All v3 authentication is domain-separated to prevent cross-version
confusion attacks. The domain context is:

```
DOMAIN_STRING = b"StegoEggo-v3"
```

This string is prepended to the authentication input. Implementations MUST
NOT use the same key for v2 and v3 authentication without domain separation.

### 8.2. CRC32 (auth_algo = 1)

Input: `[core header + key ID + extensions]` (everything except the tag).
Output: 4-byte CRC32 (little-endian).

For emitted V3 this is `CRC32(buf[0..32])` appended as 4 bytes (36 total).
No ECC replication is applied to V3. Kept alongside HMAC; new
high-assurance uses SHOULD prefer HMAC.

### 8.3. HMAC-SHA256-truncated (auth_algo = 2)

**Input (canonical byte string, `compute_payload_mac_v3`,
`src/protected/steganography/verify.rs`):**

```
HMAC_input = V3_DOMAIN_STRING || payload_without_tag
```

Concretely:

```
b"StegoEggo-v3"     (12 bytes, V3_DOMAIN_STRING)
|| <bytes 0..header_length>   (core header + key ID + extensions;
    version/auth_algo bytes are already inside, not prepended separately)
```

The 32-byte core case is `HMAC(key, DOMAIN || buf[0..32])`, tag = first 16
bytes of the 32-byte HMAC output (128-bit security).

**Key:** The MAC key (implementation-provided, typically `ProtectionConfig.mac_key`).

**Output:** First 16 bytes of `HMAC-SHA256(key, HMAC_input)`.

**Verification:** Extractor recomputes HMAC over the received header bytes
(with auth tag zeroed) using the same key and domain context. Constant-time
comparison via `subtle::ConstantTimeEq`.

### 8.4. Ed25519 Signature (auth_algo = 3)

For authenticated-provenance and maximal evidence profiles where a
non-repudiable signature is required.

**Canonical claim bytes (for signing):**

```
b"StegoEggo-v3"     (12 bytes)
|| 0x03             (1 byte, payload version)
|| 0x03             (1 byte, auth_algo)
|| header_bytes[0..header_length]   (everything except the 64-byte signature)
```

**Key:** Ed25519 private key (implementation-provided).

**Output:** 64-byte Ed25519 signature.

**Capacity fallback:** When the payload capacity cannot fit a 64-byte
signature inline (common for small images), the current plan is to place the
signature in the `ED25519_DETACHED_SIG` extension (type 0x0011) with the
`auth_tag` empty (`auth_tag_len = 0`). Note this shape is NOT buildable via
`PayloadBuilder::build` today (it requires `auth_tag.len() ==
tag_length(Ed25519) == 64`, else `CorruptTag`) — detached-signature emission
remains a spec reservation, not an implemented path. `embed_signature`
(`signatures` feature) always embeds the 64-byte signature inline plus
duplicates it in the `0x0011` extension.

**Verification:** Extractor reads the Ed25519 public key from the
`ED25519_PUBLIC_KEY` extension (type 0x0010) or from an external key store
identified by `key_id`. Verifies the signature over the canonical claim
bytes (with signature bytes zeroed).

## 9. ECC Encoding (V1/V2 legacy only — NOT applied to V3)

When no MAC key is configured for legacy payloads (`auth_algo = 0` or `1`
on V1/V2), the entire payload is ECC-encoded before embedding. V3 never
takes this path: `build_v3_payload` appends the CRC32/HMAC tag directly
with no 3× replication (`src/protected/steganography/marker.rs`), and
`ecc_encode` is retained only for backward-compatible extraction
(`src/protected/ecc.rs`).

### 9.1. ECC Replication

Same 3× repetition scheme as v1/v2 (`ecc::ecc_encode`):

```
ecc_encoded = header[0..N] || header[0..N] || header[0..N]
```

Total ECC-encoded size: `N × 3` where `N` is the pre-auth payload length.

### 9.2. CRC32 over ECC Bytes

After ECC encoding, a 4-byte CRC32 is appended:

```
ecc_payload = ecc_encoded || crc32(ecc_encoded)
```

Total embedded size: `N × 3 + 4` bytes.

### 9.3. Legacy ECC Payload Sizes (V1/V2 only)

| Pre-auth bytes (N) | ECC (3N) | + CRC32 | Total embedded |
|---------------------|----------|---------|----------------|
| 24 (V1 header) | 72 | +4 | 76 |
| 32 (V2 header) | 96 | +4 | 100 |

**Note:** The 256-byte `V3_MAX_EMBEDDED_SIZE` limit bounds all payloads;
legacy ECC 3× expansion is why large pre-auth buffers require HMAC-size
planning on small carriers.

## 10. Parsing Algorithm

### 10.1. Detection

1. If the first two bytes are `0x53 0x45` and the third byte is `0x03`,
   the payload is v3.
2. Otherwise, try v2 (byte 0 == 2), then v1 (byte 0 == 1).

### 10.2. V3 Parse Steps

```
fn parse_v3(payload: &[u8]) -> Result<StegoPayloadV3> {
    // 1. Validate magic
    require(payload[0] == 0x53 && payload[1] == 0x45);

    // 2. Validate version
    require(payload[2] == 3);

    // 3. Read header length
    let header_length = payload[3] as usize;
    require(header_length >= 32, "header too short");
    require(header_length <= payload.len(), "header exceeds payload");

    // 4. Read total length
    let total_length = u16::from_le_bytes([payload[4], payload[5]]) as usize;
    require(total_length >= header_length, "total < header");
    require(total_length <= payload.len(), "total exceeds payload");

    // 5. Read flags
    let flags = u16::from_le_bytes([payload[6], payload[7]]);

    // 6. Read channels
    let channels = u16::from_le_bytes([payload[8], payload[9]]);

    // 7. Read DMI policy
    let dmi_policy = payload[10];

    // 8. Read seed
    let seed = u64::from_le_bytes(payload[11..19].try_into()?);

    // 9. Read intensity
    let intensity = u16::from_le_bytes([payload[19], payload[20]]);

    // 10. Read content hash
    let content_hash = payload[21..29].try_into()?;

    // 11. Read auth algo, tag length, and key_id length
    let auth_algo = payload[29];
    let auth_tag_len = payload[30];
    let key_id_len = payload[31] as usize;
    require(key_id_len <= 32, "key_id too long");
    require(32 + key_id_len < header_length, "key_id overflows header");
    let key_id = &payload[32..32 + key_id_len];

    // 12. Parse extensions
    let ext_start = 32 + key_id_len;
    let ext_end = header_length;
    let extensions = parse_extensions(&payload[ext_start..ext_end])?;

    // 13. Read auth tag
    let auth_tag = &payload[header_length..header_length + auth_tag_len];

    Ok(StegoPayloadV3 { ... })
}
```

### 10.3. ECC Decode Path

When the raw extracted bits are ECC-encoded:

1. Try ECC decode with data_len derived from expected v3 sizes (32, 48, 64…).
2. Verify CRC32 over the ECC-encoded bytes.
3. Parse the decoded bytes as a v3 header.

## 11. Backward Compatibility

### 11.1. V3 Extractors MUST Accept V2/V1

A conforming v3 extractor MUST attempt parsing in this order:

1. Check for v3 magic (`0x53 0x45`, version `0x03`).
2. If no v3 magic, check version byte for v2 (`0x02`) and parse v2 header.
3. If not v2, check for v1 (`0x01`) and parse v1 header.

This ensures a single binary can read all payload versions without
forcing a coordinated upgrade of protected images in the wild.

### 11.2. V2/V1 Extractors and V3 Payloads

V2 and V1 extractors will fail to parse v3 payloads because:
- The first two bytes (`0x53, 0x45`) do not match a valid version byte (0–2).
- Even if byte 2 (`0x03`) is misinterpreted as version 3 by a future-aware
  parser, the header layout is incompatible.

This is intentional: v3 payloads are unreadable by v1/v2-only extractors,
forcing an upgrade.

### 11.3. Migration Path (completed)

`V3_PAYLOAD_VERSION = 3` (`src/payload_v3/types.rs`):
- `SUPPORTED_PAYLOAD_VERSIONS` is `[1, 2, 3]` (`src/protected/steganography/mod.rs`).
- The protection pipeline writes V3 (36 bytes CRC / 48 bytes HMAC, no
  key ID / extensions).
- `parse_stego_payload()` / `parse_payload()` accept V1/V2/V3 (V3 dispatched
  by magic before the legacy version loop).

## 12. Security Considerations

### 12.1. Domain Separation

The `b"StegoEggo-v3"` domain string prevents an attacker from taking a
valid v2 HMAC and presenting it as a valid v3 HMAC. Without domain
separation, a v2 MAC computed over the same header bytes would verify
against v3 (since v3 has a different version byte but the same structure).

### 12.2. Constant-Time Comparison

HMAC verification MUST use constant-time comparison (`subtle::ConstantTimeEq`)
to prevent timing side-channel attacks on the authentication tag.

### 12.3. Key Separation

v3 HMAC keys SHOULD be derived from a master key via:
```
v3_key = HKDF-SHA256(master_key, info=b"payload-v3-auth", length=32)
```

This prevents key reuse across payload versions even if the same master
key is used.

### 12.4. Payload Size Validation

All length fields are validated before allocation:
- `total_length >= header_length`
- `total_length <= payload.len()`
- `key_id_len <= 32`
- `header_length >= 32`
- `auth_tag_len` matches `auth_algo` expectation
- Extension total ≤ 128 bytes

Implementations MUST reject payloads that fail any of these checks.

### 12.5. Extension Poisoning

Unknown critical extensions cause verification failure. This prevents an
attacker from injecting unrecognized data that a lenient parser might
interpret as meaningful.

The `CRITICAL_EXTENSION` flag is the mechanism for this. Encoders setting
this flag MUST ensure all consumers understand the extensions in use.

## 13. Implementation Notes

### 13.1. Encoder Checklist

- [ ] Write magic bytes `0x53 0x45`.
- [ ] Write version byte `3`.
- [ ] Compute and write `header_length` (32 + key_id_len + 2 + extension_bytes).
- [ ] Compute and write `total_length`.
- [ ] Set `HAS_EXTENSIONS` flag if extensions present.
- [ ] Set `HAS_KEY_ID` flag if `key_id_len > 0`.
- [ ] Sort extensions by type ascending.
- [ ] Compute CRC32 or HMAC over canonical bytes.
- [ ] Validate total ≤ `V3_MAX_EMBEDDED_SIZE` (256).
- [ ] CRC/HMAC the header bytes directly; do NOT apply ECC replication to V3.

### 13.2. Decoder Checklist

- [ ] Check magic bytes before version.
- [ ] Validate `header_length ≥ 32`.
- [ ] Validate `total_length ≥ header_length`.
- [ ] Validate `key_id_len ≤ 32`.
- [ ] Parse extensions with overflow checks.
- [ ] Reject duplicate extension types.
- [ ] Handle unknown extensions per `CRITICAL_EXTENSION` flag.
- [ ] Verify authentication tag using domain-separated input.
- [ ] Use constant-time comparison for HMAC.

### 13.3. Constants Summary

| Constant | Value | Description |
|----------|-------|-------------|
| `V3_MAGIC` | `[0x53, 0x45]` | Domain separator ("SE") |
| `V3_PAYLOAD_VERSION` | `3` | Payload version byte |
| `V3_CORE_SIZE` | `32` | Minimum header size (no key_id, no ext, no auth) |
| `V3_MAX_EMBEDDED_SIZE` | `256` | Maximum total embedded payload |
| `V3_MAX_EXTENSION_SIZE` | `128` | Maximum total extension bytes |
| `V3_MAX_EXTENSION_COUNT` | `32` | Maximum number of extensions |
| `V3_MAX_KEY_ID_LEN` | `32` | Maximum key identifier length |
| `V3_DOMAIN_STRING` | `"StegoEggo-v3"` | Authentication domain context |
| `END_OF_EXTENSIONS` | `0xFFFF` | Extension-section sentinel |

## 14. Formal Grammar

The payload can be expressed as a BNF-like grammar:

```
<payload>        ::= <magic> <version> <header_length> <total_length>
                     <flags> <channels> <dmi_policy> <seed> <intensity>
                     <content_hash> <key_id> <auth_algo> <auth_tag_len>
                     <extensions> <auth_tag>

<magic>          ::= 0x53 0x45
<version>        ::= 0x03
<header_length>  ::= u8                    # offset to auth_tag from start
<total_length>   ::= u16_le                # total embedded size
<flags>          ::= u16_le
<channels>       ::= u16_le
<dmi_policy>     ::= u8                    # DmiValue discriminant
<seed>           ::= u64_le
<intensity>      ::= u16_le                # f32 * 100.0
<content_hash>   ::= u8[8]                 # truncated content hash
<key_id>         ::= u8 <u8[key_id_len]>   # length-prefixed
<auth_algo>      ::= u8                    # 0|1|2|3
<auth_tag_len>   ::= u8                    # expected tag length
<extensions>     ::= <tlv>*                # zero or more TLVs
<auth_tag>       ::= u8[auth_tag_len]

<tlv>            ::= <tlv_type> <tlv_length> <tlv_value>
<tlv_type>       ::= u16_le
<tlv_length>     ::= u16_le
<tlv_value>      ::= u8[tlv_length]
```

## 15. Changelog

| Version | Date | Changes |
|---------|------|---------|
| Draft | 2026-07-21 | Initial specification |

## 16. References

- [stegoeggo source](../src/protected/steganography/) — current V3 marker and V1/V2 compatibility implementation
- [ECC module](../src/protected/ecc.rs) — 3× repetition error correction
- [Architecture: Steganography](protected-steganography.md) — embedding and extraction
- [Architecture: Constants](constants.md) — tuning parameters
- [Types](../src/types.rs) — `DmiValue`, `ProtectionLevel`, `StegoPayload`
