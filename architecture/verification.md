# Verification Report

**Source:** `src/verification/`

Provides a structured, machine-readable verification report with per-channel sub-results. Used by the CLI `--verify` flag and programmatic verification APIs.

## Module Structure

```
src/verification/
├── mod.rs        Re-exports
├── report.rs     VerificationReport and all sub-verification types
└── builder.rs    VerificationReportBuilder (fluent API)
```

## `VerificationReport`

The top-level report type. Serializable to JSON. Contains:

```rust
pub struct VerificationReport {
    rights: RightsVerification,
    hidden_marker: HiddenMarkerVerification,
    authentication: AuthenticationVerification,
    signatures: Vec<SignatureVerification>,
    bindings: BindingVerification,
    trust: TrustEvaluation,
    evidence_strength: EvidenceStrength,
    diagnostics: Vec<Diagnostic>,
}
```

### Evidence Strength

Computed automatically by the builder from the sub-results:

```rust
pub enum EvidenceStrength {
    NoNoticeFound,     // No rights metadata or stego found
    MetadataOnly,      // Rights metadata found, no stego
    StegoOnly,         // Stego found, no rights metadata
    MetadataAndStego,  // Both channels present
    Authenticated,     // Both + HMAC/Ed25519 verification
}
```

## Sub-Verification Types

### `RightsVerification`

Rights and legal-notice metadata extraction results:

- `found: bool` — Whether any rights metadata was found
- `copyright_holder`, `creator`, `contact`, `rights_url`, `usage_terms`, `ai_constraints` — Extracted fields
- `dmi: Option<u8>` — Data-mining policy byte
- `source: FieldSource` — Where the data came from
- `channels: Vec<EvidenceChannel>` — Which evidence channels contributed

DMI values are extracted from canonical `plus:DataMining` XMP properties containing full PLUS LDF URIs (e.g., `http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING`). Legacy bare keys (`DMI-PROHIBITED-AIMLTRAINING`) and `Iptc4xmpExt:DMI-*` properties are parsed for backward compatibility but classified as legacy signals, not canonical.

### `HiddenMarkerVerification`

Steganographic payload extraction results:

- `status: VerificationStatus` — NotFound / Verified / Invalid
- `payload_version: Option<u8>` — Detected version (1, 2, or 3)
- `seed: Option<u64>` — Extracted PRNG seed
- `intensity: Option<f32>` — Embedding intensity
- `source: FieldSource` — Extraction source
- `tiled: bool` — Whether tiled steganography was detected

### `AuthenticationVerification`

HMAC or Ed25519 authentication results:

- `attempted: bool` — Whether auth was attempted
- `hmac_status: Option<VerificationStatus>` — HMAC result
- `key_id: Option<Vec<u8>>` — Extracted key identifier
- `algorithm: String` — Auth algorithm name
- `key_matched: bool` — Whether supplied key matched

### `SignatureVerification`

Ed25519 signature results (one per signature in the manifest):

- `present: bool` — Signature found
- `structurally_valid: bool` — Well-formed signature bytes
- `cryptographically_valid: bool` — Signature verifies against public key
- `public_key_id`, `expected_key_id` — Key identifiers
- `key_id_matched: bool` — Key IDs match
- `key_material_matched: bool` — Caller-owned key bytes match manifest key bytes
- `trusted: bool` — Signature from a trusted key
- `claim: Option<Vec<u8>>` — Canonical claim bytes that were signed
- `source: FieldSource` — Embedded or detached

### `BindingVerification`

Image-to-claim binding results:

- `instance_digest_match: bool` — SHA-256 of image matches manifest
- `embedded_payload_digest_match: Option<bool>` — Embedded payload digest match
- `embedded_payload_version_match: Option<bool>` — Embedded payload version match

### `TrustEvaluation`

Trust chain evaluation:

- `trusted: bool` — Overall trust decision
- `policy: String` — Trust policy used
- `reason: String` — Human-readable trust reason
- `chain_valid: Option<bool>` — Certificate chain validity (if applicable)

### `Diagnostic`

Informational or warning messages:

```rust
pub struct Diagnostic {
    level: DiagnosticLevel,  // Info, Warning, Error
    message: String,
}
```

## `FieldSource`

Indicates where verification data was obtained:

```rust
pub enum FieldSource {
    Xmp,                // XMP metadata
    Legacy,             // Non-XMP metadata (tEXt, COM)
    EmbeddedPayloadV1,  // V1 stego payload
    EmbeddedPayloadV2,  // V2 stego payload
    EmbeddedPayloadV3,  // V3 stego payload
    DetachedManifest,   // Detached sidecar manifest
    CallerSupplied,     // Provided by the caller
    Computed,           // Derived or computed
}
```

## Builder API

```rust
let report = VerificationReportBuilder::new()
    .with_rights(rights_verification)
    .with_hidden_marker(hidden_marker_verification)
    .with_authentication(auth_verification)
    .add_signature(sig_verification)
    .with_bindings(binding_verification)
    .with_trust(trust_evaluation)
    .add_diagnostic(diagnostic)
    .build();  // Computes evidence_strength automatically
```

## JSON Output

`VerificationReport` implements `Serialize`. The JSON structure:

```json
{
  "rights": { "found": true, "copyright_holder": "...", ... },
  "hidden_marker": { "status": "Verified", "payload_version": 3, ... },
  "authentication": { "attempted": true, "hmac_status": "Verified", ... },
  "signatures": [...],
  "bindings": { "instance_digest_match": true, ... },
  "trust": { "trusted": true, "policy": "TrustKeys", ... },
  "evidence_strength": "Authenticated",
  "diagnostics": [...]
}
```

## Relationship to NoticeVerification

`NoticeVerification` (`src/protected/notice_verification.rs`) is the internal extraction engine. It parses image bytes and extracts rights metadata, DMI values, and stego seeds. The `VerificationReport` is the structured, public-facing output that wraps extraction results with per-channel sub-results and evidence strength rating.
