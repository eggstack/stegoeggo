# Deprecation Inventory

This document tracks all deprecated public API surfaces, their replacements, and removal timeline.

## Deprecated in v0.2.2 (Release 4/5)

| Deprecated API | Replacement | Introduced | Earliest Removal |
|----------------|-------------|------------|------------------|
| `ProtectionLevel` | `ProtectionRequest` + `RightsPolicy` | v0.2.2 | v1.0.0 |
| `EvidenceProfile` | `ProtectionPreset` | v0.2.2 | v1.0.0 |
| `with_dmi()` | `RightsPolicy` in `ProtectionRequest` | v0.2.2 | v1.0.0 |
| `with_metadata_injection()` | `ProtectionChannels::metadata_only()` or `ProtectionRequest` builder | v0.2.2 | v1.0.0 |
| `with_legal_claims()` | Auto-enabled with `LegalMetadata` (explicit `false` emits `ContradictoryLegalClaims`) | v0.2.2 | v1.0.0 |
| `compute_iscc()` | `compute_content_identifiers()` | v0.2.2 | v1.0.0 |
| `compute_iscc_with_metadata()` | `compute_content_identifiers_with_metadata()` | v0.2.2 | v1.0.0 |
| `compute_iscc_from_bytes()` | `compute_content_identifiers_from_bytes()` | v0.2.2 | v1.0.0 |
| `NoticeVerification::new()` (positional) | `NoticeVerification::builder()` | v0.2.2 | v1.0.0 |

Note: `VerificationStatus` is **not** deprecated — it remains the return type of
`verify_image_bytes`. The structured reports (`VerificationReport`,
`VerificationResult`, `NoticeVerification`) are richer alternatives for callers
who need per-channel detail, not replacements for a removed API.

## Deprecated in v0.3.0

| Deprecated API | Replacement | Introduced | Earliest Removal |
|----------------|-------------|------------|------------------|
| `EmbeddedReferenceStatus::Present` | `EmbeddedReferenceStatus::PresentValid` | v0.3.0 | v1.0.0 |

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

## Pixel-Only APIs (Plan 042)

`process_image` and `process_images_parallel` accept `DynamicImage` and return `DynamicImage`. The `DynamicImage` roundtrip **does not preserve file-level metadata** (PNG tEXt, JPEG COM/XMP, WebP XMP). These functions embed steganographic markers in pixel data only. For full metadata injection, use the byte-path APIs (`process_image_bytes`, `process_request_bytes`).

The CLI now always routes through `ProtectionRequest` — there are no dual legacy/request code paths.

## Legacy Auxiliary APIs (Plan 082)

The level/context async and parallel wrappers remain functional but are compatibility adapters. New code must use the request-based forms:

| Legacy API | Canonical replacement |
|------------|-----------------------|
| `process_image_async`, `process_image_bytes_async`, `process_image_bytes_with_warnings_async` | `process_request_bytes_async`, `process_request_bytes_with_warnings_async`, `process_request_bytes_with_report_async` (`async` feature) |
| `process_images_parallel`, `process_images_bytes_parallel` | `process_request_bytes_parallel`, `process_request_bytes_with_warnings_parallel`, `process_request_bytes_with_report_parallel` (`parallel` feature) |
| `process_images_parallel_async`, `process_images_bytes_parallel_async` | `process_request_bytes_parallel_async`, `process_request_bytes_with_warnings_parallel_async`, `process_request_bytes_with_report_parallel_async` (`async` + `parallel`) |
| `process_image_bytes_with_warnings` | `process_request_bytes_with_warnings` (legacy adds only `MissingMacKey`, `ContradictoryLegalClaims`, `JpegReencodeFragile` presentation warnings) |

Legacy helpers translate once via `request_from_legacy()` into `ProtectionRequest` and delegate to the canonical path. New processing features must be expressed in `ProtectionRequest` / `ProcessingOptions` / `ProtectionChannels` first.

## CLI v1 removal inventory (Plan 087)

No 0.x CLI flag is removed. Candidates for removal at v1.0.0:

Deprecated syntax with exact modern replacement:

| Legacy CLI flag | Modern replacement |
|-----------------|-------------------|
| `--level` | `--preset` + `--hidden-marker` |
| `--profile` | `--preset` |
| `--dmi` | `--rights-policy` |
| `--no-ai-training` | `--rights-policy prohibited-ai-ml-training` |
| `--no-genai-training` | `--rights-policy prohibited-generative-ai-training` |
| `--tdm-reserved` | `--rights-policy prohibited-see-constraints` (already deprecated) |
| `--metadata`, `--legal-claims` | `ProtectionChannels` via `ProtectionRequest` |

Compatibility behavior that must remain for reading old protected images (not
removal candidates): legacy DMI/TDM metadata parsing, payload v1/v2 extraction,
`--verify` human/JSON output fields.

Stable current syntax that carries forward: `--rights-policy`, `--preset`,
`--hidden-marker`, `--authentication`, `--dry-run`, `--json`, `--key`
(hex/`@file`/`-`/env `STEGOEGGO_KEY`), `--jobs`, `--strict`, and the
`signatures`-gated `keygen`/`sign`/`verify-manifest` subcommands.

See `docs/cli-usage.md` for the precedence contract.

## Policy

- Deprecated APIs are still functional and tested
- All deprecated APIs will be removed in v1.0.0 (next semver-major release)
- No deprecated APIs will be removed in any 0.x release
- New code should use the replacement APIs exclusively
