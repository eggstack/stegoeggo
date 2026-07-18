# Core Types

**Source:** `src/types.rs` (~1964 lines)

Defines all core data structures used across the codebase. Uses builder pattern with `#[must_use]` on builder methods.

## ProtectionLevel

Enum with three variants:

```rust
pub enum ProtectionLevel {
    Disabled,    // No protection (PassthroughProtector)
    Light,       // Metadata + minimal seed stego
    Standard,    // Noise + stego + metadata (default)
}
```

- `to_byte()` / `from_byte()` — For stego payload serialization
- `Default` returns `Standard`

## EvidenceProfile

Enum controlling how protection warnings are interpreted and the default evidence posture:

```rust
pub enum EvidenceProfile {
    LegalNotice,           // default — metadata notice, no MAC required
    LegalNoticeWithStego,  // metadata + best-effort stego, no MAC required
    AuthenticatedProvenance, // MAC key expected, cryptographic proof
    Maximal,               // all channels
}
```

- `as_str()` — Returns lowercase string: `"legal-notice"`, `"legal-notice-stego"`, `"authenticated-provenance"`, `"maximal"`
- `Default` returns `LegalNotice`
- Serialized as serde-compatible enum variants

### Interaction with ProtectionLevel

`ProtectionLevel` controls *how much processing occurs*. `EvidenceProfile` controls *how warnings are interpreted*. They are orthogonal:
- Any profile can be used with any protection level
- `MissingMacKey` warning is only emitted for `AuthenticatedProvenance` and `Maximal` profiles
- `LegalNotice` and `LegalNoticeWithStego` never warn about missing MAC keys

## ImageOutputFormat

```rust
pub enum ImageOutputFormat {
    Png,   // default
    Jpeg,
    WebP,
}
```

- `from_magic_bytes(bytes)` — PNG: `[0x89, P, N, G]`, JPEG: `[0xFF, 0xD8, 0xFF]`, WebP: `RIFF....WEBP`
- `from_extension(path)` — Extracts from file extension
- `extension()` — Returns `"png"`, `"jpg"`, `"webp"`
- `to_image_format()` — Converts to `image::ImageFormat`

## DmiValue

Data Mining Inhibitor enum for rights metadata:

```rust
pub enum DmiValue {
    Unspecified,
    Allowed,
    ProhibitedAiMlTraining,
    ProhibitedGenAiMlTraining,
    ProhibitedExceptSearchEngineIndexing,
    Prohibited,
    ProhibitedSeeConstraints,
}
```

Auto-mapped from `ProtectionLevel` via helper in `metadata_trap.rs`: Light→Prohibited, Standard→ProhibitedAiMlTraining. No `impl From<ProtectionLevel> for DmiValue` exists — use the helper function directly.

`plus_vocab_key()` returns the canonical PLUS LDF vocabulary key (e.g., `DMI-PROHIBITED-AIMLTRAINING`). `from_plus_vocab_key()` parses a canonical key back to the enum variant. Legacy `Iptc4xmpExt:DMI-*` values are still parsed for backward compatibility but not emitted by default.

## ProtectionContext

The main configuration object. Created via builder pattern:

```rust
ProtectionContext::new(intensity, seed)  // intensity clamped to [0.0, 1.0]
```

### Fields (all private, use getters)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `intensity` | `f32` | 0.5 | Perturbation strength (0.0–1.0) |
| `seed` | `u64` | random | Reproducibility seed |
| `input_format` | `Option<ImageOutputFormat>` | None | Hint for format detection |
| `output_format` | `Option<ImageOutputFormat>` | None | Target output format |
| `protection_level` | `Option<ProtectionLevel>` | None | Override level (crate-internal) |
| `dmi_value` | `Option<DmiValue>` | None | DMI override (canonical `plus:DataMining` emitted) |
| `max_dimension` | `Option<u32>` | None | Resize constraint |
| `inject_metadata` | `Option<bool>` | None | Enable metadata injection |
| `inject_legal_claims` | `Option<bool>` | None | Enable legal metadata |
| `stego_redundancy` | `Option<usize>` | None | Stego passes (1–10). Default derived from intensity: <0.3→1, 0.3-0.7→2, >=0.7→3 |
| `jpeg_quality` | `u8` | 90 | JPEG encoding quality |
| `progressive_jpeg` | `bool` | false | Progressive JPEG encoding |
| `tile_size` | `Option<u32>` | None | Crop-resistant tile size (32..=1024). None/0 = disabled |
| `tile_extraction_max_origins` | `u32` | 64 | Max candidate tile origins for extraction |
| `content_hash` | `Option<[u8; 4]>` | None | Truncated content hash for provenance tracking (v2 payloads) |
| `evidence_profile` | `Option<EvidenceProfile>` | None | Warning interpretation and evidence posture (defaults to `LegalNotice` when not set) |
| `config` | `Option<Arc<ProtectionConfig>>` | None | `#[serde(skip)]` — MAC key + legal metadata |

**Note:** `None` for `inject_metadata`/`inject_legal_claims` means "use level default" (enabled for Standard). Explicit `false` disables injection.

### Builder Methods

All `with_*` methods return `Self` with `#[must_use]`. Example:

```rust
ctx.with_intensity(0.8)
   .with_seed(42)
   .with_format(ImageOutputFormat::Jpeg)  // Use with_format(), not with_output_format()
   .with_stego_redundancy(3)
```

## ProtectionConfig

Shared heavy configuration wrapped in `Arc`:

```rust
pub struct ProtectionConfig {
    mac_key: Option<Vec<u8>>,
    legal_metadata: Option<LegalMetadata>,
}
```

All fields are private — use builder methods (`with_mac_key`, `with_legal_metadata`) and getters (`mac_key()`, `legal_metadata()`).

## LegalMetadata

Builder-pattern struct for legal/copyright metadata:

Fields: `copyright_holder`, `contact_email`, `license_url`, `usage_terms`, `creation_date`, `ai_constraints`, `web_statement_of_rights`, `creator`.

## NoticeVerification

Comprehensive legal-notice verification report returned by `verify_legal_notice()`:

```rust
pub struct NoticeVerification {
    copyright_holder: Option<String>,
    creator: Option<String>,
    contact: Option<String>,
    rights_url: Option<String>,
    usage_terms: Option<String>,
    ai_constraints: Option<String>,
    dmi: Option<DmiValue>,
    tdm_reserved: Option<bool>,
    rights_signal_kind: RightsSignalKind,
    canonical_dmi: Option<DmiValue>,
    legacy_dmi: Option<DmiValue>,
    protection_seed: Option<u64>,
    stego_status: VerificationStatus,
    stego_payload: Option<StegoPayload>,
    authenticated: bool,
    evidence_strength: EvidenceStrength,
    channels: Vec<EvidenceChannel>,
    license_url: Option<String>,
    web_statement_of_rights: Option<String>,
    credit_line: Option<String>,
    copyright_owner: Option<String>,
    licensor_name: Option<String>,
    licensor_email: Option<String>,
    licensor_url: Option<String>,
    metadata_date: Option<String>,
    notice_applied_at: Option<String>,
}
```

**Construction**: Use `NoticeVerification::builder()` for field-named construction. The 26-argument positional `new()` is deprecated.

```rust
let nv = NoticeVerification::builder()
    .copyright_holder(Some("Jane Doe".into()))
    .dmi(Some(DmiValue::ProhibitedAiMlTraining))
    .evidence_strength(EvidenceStrength::MetadataNoticeOnly)
    .build();
```

All fields are private — use getter methods (e.g., `report.copyright_holder()`, `report.evidence_strength()`). `has_notice()` returns true if any legal field or DMI value is present. Additional diagnostics: `canonical_dmi()`, `legacy_dmi()`, `rights_signal_kind()`, `has_dmi_conflict()`.

## EvidenceStrength

Enum classifying the overall evidence posture of a protected image:

```rust
pub enum EvidenceStrength {
    NoNoticeFound,                          // No metadata or stego
    MetadataNoticeOnly,                     // Metadata present, no stego verified
    MetadataNoticeAndBestEffortStego,       // Metadata + unauthenticated stego
    MetadataNoticeAndAuthenticatedProvenance, // Metadata + MAC-authenticated stego
}
```

## EvidenceChannel

Enum identifying which evidence channels were detected during verification:

```rust
pub enum EvidenceChannel {
    PngText,     // PNG tEXt chunks (copyright, contact, etc.)
    PngXmp,      // PNG iTXt XMP data
    JpegComment, // JPEG COM markers
    JpegXmp,     // JPEG APP1 XMP data
    JpegIptc,    // JPEG APP13 IPTC-IIM data
    WebPXmp,     // WebP XMP chunk
    WebPExif,    // WebP EXIF chunk
    LsbPayload,  // LSB steganographic payload
    DctPayload,  // DCT/F5 steganographic payload
    QTableSeed,  // Quantization table seed (reserved, not currently emitted)
}
```

## StegoPayload

Extracted stego data (returned from `SteganographyProtector::extract_payload`):

- `protection_level() -> u8`
- `seed() -> u64`
- `intensity() -> f32`
- `version() -> u8`

## ProtectionWarning

Enum with 6 variants emitted by `process_image_bytes_with_warnings`:

| Variant | Category | Description |
|---------|----------|-------------|
| `MissingMacKey` | AuthenticatedProvenance | No MAC key configured |
| `MetadataInjectionDisabled` | LegalNotice | Metadata injection explicitly disabled |
| `ProgressiveJpegFallback` | FormatFragility | Progressive JPEG fell back to Q-table seed only |
| `JpegReencodeFragile` | FormatFragility | JPEG output fragile under downstream re-encoding |
| `LsbCapacitySkipped` | BestEffortStego | Image too small for LSB embedding |
| `DctCapacityInsufficient` | BestEffortStego | JPEG has too few DCT coefficients for F5 |

### Helper Methods

- `category() -> WarningCategory` — Returns the warning's category
- `severity_for_profile(profile) -> WarningSeverity` — Returns severity relative to a specific evidence profile

## WarningCategory

Enum classifying warnings by their relevance to evidence models:

```rust
pub enum WarningCategory {
    LegalNotice,            // Metadata/injection warnings
    BestEffortStego,       // Stego capacity limitations
    AuthenticatedProvenance, // MAC/crypto warnings
    FormatFragility,       // Format-specific fragility
}
```

## WarningSeverity

Enum indicating warning severity within an evidence profile:

```rust
pub enum WarningSeverity {
    Info,     // Expected behavior for this profile
    Warning,  // Protection is degraded
    Error,    // Evidence model cannot be satisfied
}
```

### Severity Mappings

| Warning | LegalNotice | LegalNoticeWithStego | AuthenticatedProvenance | Maximal |
|---------|-------------|---------------------|------------------------|---------|
| MissingMacKey | Info | Info | Warning | Warning |
| MetadataInjectionDisabled | Error | Error | Warning | Warning |
| ProgressiveJpegFallback | Warning | Warning | Warning | Warning |
| JpegReencodeFragile | Warning | Warning | Warning | Warning |
| LsbCapacitySkipped | Info | Warning | Warning | Warning |
| DctCapacityInsufficient | Info | Warning | Warning | Warning |

## CoverageMinimums

Minimum counts required per category/format for coverage enforcement:

```rust
pub struct CoverageMinimums {
    pub canonical_png: usize,       // default: 1
    pub canonical_jpeg: usize,      // default: 1
    pub canonical_webp: usize,      // default: 1
    pub legacy_min: usize,          // default: 3
    pub legacy_formats: usize,      // default: 2
    pub conflict_min: usize,        // default: 3
    pub malformed_min: usize,       // default: 4
    pub malformed_per_format: usize,// default: 1 (per format: png, jpeg, webp)
    pub preservation_min: usize,    // default: 3
    pub preservation_formats: usize,// default: 3
}
```

Note: The blanket `external_coverage_pct` field has been removed. Coverage is now enforced via these explicit per-category and per-format minimums. The `malformed_per_format` field ensures malformed fixtures cover each format individually.

## ConformanceReport

The JSON report type for per-fixture conformance results. In addition to the
core fields (`fixture`, `format`, `decode_valid`, `xmp_valid`, `internal`,
`external`, `checks`, `conflicts`, `passed`), it includes manifest-linked metadata:

- `fixture_id: Option<String>` — Manifest fixture ID, if matched
- `category: Option<String>` — Fixture category from manifest, if matched
- `source: Option<String>` — Fixture source classification, if matched

These fields are populated from the manifest entry when the fixture is found in the manifest. They are skipped in JSON serialization when `None`.

## validate_manifest()

```rust
pub fn validate_manifest(manifest: &FixtureManifest) -> Result<(), Vec<String>>
```

Validates manifest structure before processing fixtures. Checks for duplicate IDs, duplicate paths, empty IDs, path traversal, unsupported formats/categories/sources, and SHA-256 validity (64 hex characters). Returns `Ok(())` if valid, or `Err(Vec<String>)` with all violation messages. Called early in the conformance harness to catch manifest issues before any fixtures are processed.

## Serialization Notes

- `ProtectionContext.config` is `#[serde(skip)]` — MAC keys and legal metadata are lost in serde roundtrips
- A test (`test_config_skipped_in_serde_roundtrip`) documents this behavior
