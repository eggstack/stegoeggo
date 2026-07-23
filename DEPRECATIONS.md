# Deprecation Inventory

This document tracks all deprecated public API surfaces, their replacements, and removal timeline.

## Deprecated in v0.2.2 (Release 4/5)

| Deprecated API | Replacement | Introduced | Earliest Removal |
|----------------|-------------|------------|------------------|
| `ProtectionLevel` | `ProtectionRequest` + `RightsPolicy` | v0.2.2 | v1.0.0 |
| `EvidenceProfile` | `ProtectionPreset` | v0.2.2 | v1.0.0 |
| `with_dmi()` | `RightsPolicy` in `ProtectionRequest` | v0.2.2 | v1.0.0 |
| `with_metadata_injection()` | `ProtectionChannels` | v0.2.2 | v1.0.0 |
| `with_inject_legal_claims()` | Auto-enabled with `LegalMetadata` | v0.2.2 | v1.0.0 |
| `compute_iscc()` | `compute_content_identifiers()` | v0.2.2 | v1.0.0 |
| `compute_iscc_detailed()` | `compute_content_identifiers()` | v0.2.2 | v1.0.0 |
| `VerificationStatus` | `VerificationReport` | v0.2.2 | v1.0.0 |
| `NoticeVerification::new()` (positional) | `NoticeVerification::builder()` | v0.2.2 | v1.0.0 |

## Migration Examples

### ProtectionLevel → ProtectionRequest
```rust
// Old
let ctx = ProtectionContext::default().with_dmi(DmiValue::ProhibitedAiMlTraining);
let result = process_image_bytes(&bytes, ProtectionLevel::Standard, &ctx)?;

// New
let request = ProtectionRequest::metadata_only(
    RightsNotice::default(),
    RightsPolicy::ProhibitedAiMlTraining,
);
let result = process_request_bytes(&bytes, &request)?;
```

### EvidenceProfile → ProtectionPreset
```rust
// Old
let ctx = ProtectionContext::legal_notice();

// New
let request = ProtectionRequest::from_preset(
    ProtectionPreset::LegalNotice,
    RightsNotice::default(),
    RightsPolicy::ProhibitedAiMlTraining,
);
```

### compute_iscc → compute_content_identifiers
```rust
// Old
let iscc = compute_iscc(&img);

// New
let iscc = compute_content_identifiers(&img);
```

## Policy

- Deprecated APIs are still functional and tested
- All deprecated APIs will be removed in v1.0.0 (next semver-major release)
- No deprecated APIs will be removed in any 0.x release
- New code should use the replacement APIs exclusively
