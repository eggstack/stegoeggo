# Verification Report

**Source:** `src/verification/` (canonical core: `canonical.rs`; types: `report.rs`; builder: `builder.rs`)

Provides a structured, machine-readable verification report with per-channel sub-results. `verify_image_bytes_report` is the canonical operation for rich integrations: one rights parse plus one hidden-marker search produces `CanonicalFacts`, converted to `VerificationReport` exactly once. `verify_image_bytes`, `verify_image_bytes_detailed`, and `verify_legal_notice` are centralized compatibility projections from the same facts. CLI `--verify` consumes the canonical-backed `NoticeVerification` projection and derives auth presentation from its `authenticated`/`stego_status` fields.

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
    NoNoticeFound,                          // No metadata or stego found
    MetadataNoticeOnly,                     // Rights metadata found, no stego verified
    MetadataNoticeAndBestEffortStego,       // Metadata + unauthenticated stego
    MetadataNoticeAndAuthenticatedProvenance, // Metadata + MAC-authenticated stego
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

DMI values are extracted from canonical `plus:DataMining` XMP properties containing full PLUS LDF URIs (e.g., `http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING`). Legacy bare keys (`DMI-PROHIBITED-AIMLTRAINING`) are parsed for backward compatibility but classified as `LegacyBarePlusVocabularyKey`, not `CanonicalPlusDataMining`. `Iptc4xmpExt:DMI-*` properties are similarly classified as legacy signals. Unknown-origin URLs (e.g., `http://evil.example.com/DMI-PROHIBITED-AIMLTRAINING`) are rejected and do not set `canonical_dmi`.

### `HiddenMarkerVerification`

Steganographic payload extraction results:

- `status: VerificationStatus` — NotFound / Verified / Invalid
- `payload_version: Option<u8>` — Detected version (1, 2, or 3)
- `seed: Option<u64>` — Extracted PRNG seed
- `intensity: Option<f32>` — Embedding intensity
- `source: FieldSource` — Extraction source
- `tiled: bool` — Whether tiled steganography was detected

**JPEG DCT support scope**: DCT embedding requires baseline sequential JPEG with 8-bit precision, single scan, 1-4 components with sampling factors up to 4, no restart intervals, no progressive or arithmetic coding. Unsupported inputs (progressive, restart-bearing, multi-scan, CMYK) fall back to metadata-only processing; the stego status is `NotFound` for those inputs.

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

- `instance_digest_present: bool` — Whether instance digest was found
- `instance_digest_valid: bool` — SHA-256 of image matches manifest
- `content_hash_present: bool` — Whether content hash was found
- `content_hash_valid: bool` — Content hash matches expected value
- `format_valid: bool` — Format matches expected format
- `dimensions_valid: bool` — Dimensions match expected dimensions
- `file_size_valid: bool` — File size matches expected size
- `source: FieldSource` — Where binding data was obtained

### `TrustEvaluation`

Trust chain evaluation:

- `trusted: bool` — Overall trust decision
- `trust_model: String` — Trust model used
- `reason: String` — Human-readable trust reason

### `Diagnostic`

Informational or warning messages:

```rust
pub struct Diagnostic {
    level: DiagnosticLevel,  // Info, Warning, Error
    message: String,
    source: String,
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
  "trust": { "trusted": true, "trust_model": "TrustKeys", ... },
  "evidence_strength": "MetadataNoticeAndAuthenticatedProvenance",
  "diagnostics": [...]
}
```

## Canonical core and projections

`src/verification/canonical.rs` owns verification semantics. `verify_canonical_with_limits` extracts rights metadata via `src/protected/notice_verification.rs` helpers (format-specific notice plus `*_with_limits` XMP/DMI parsers) and runs one `SteganographyProtector::verify_payload_from_bytes_outcome` search, preserving `Valid`, `Invalid`, `MalformedV3`, `UnsupportedVersion`, `AuthenticationKeyMissing`, `AuthenticationFailed`, `ResourceLimitExceeded`, and `NotFound`. It builds `VerificationReport` once, with diagnostics for malformed, unsupported, missing/failed auth, corruption, resource exhaustion, and metadata-only cases.

Centralized projections, named and tested in `tests/verification_convergence.rs`:

- `project_status_from_canonical` -> `VerificationStatus` (stego-only; `summary_status` rights fallback is separate)
- `project_result_from_canonical` -> `VerificationResult` (`Verified`/`Corrupted`/`MetadataOnly`/`NotFound`)
- `project_notice_from_canonical` -> `NoticeVerification` (full legal fields)
- `project_report_from_canonical` -> `VerificationReport` clone

## Relationship to NoticeVerification

`NoticeVerification` (`src/types.rs`) is a compatibility projection carrying full legal-notice fields (license URL, web statement, credit line, copyright owner, licensor fields, metadata date, notice-applied-at, TDM reservation, rights-signal kind, canonical/legacy DMI, protection seed, full `StegoPayload`) not present in `RightsVerification`. Both projections come from the same `CanonicalFacts`; there are not two independent searches. Detached-manifest embedded-reference checks reuse the single-outcome raw-plus-parse path; full detached signature/binding/trust reporting remains manifest-aware by necessity (documented exception).
