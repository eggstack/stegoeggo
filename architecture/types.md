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

**Deprecated** — use `ProtectionPreset` instead. `EvidenceProfile` is retained as a compatibility adapter and will be removed in the next major version.

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

## Policy-First Architecture (Release 4)

Release 4 separates rights policy from processing mechanics. The canonical API entry point is `ProtectionRequest`, which combines a rights notice, policy, channel configuration, and processing options. `resolve_request()` validates all inputs and produces an immutable `ResolvedProtectionPlan` consumed by pipeline stages.

### RightsPolicy

Explicit data-mining policy enum. Maps 1:1 to `DmiValue`:

```rust
pub enum RightsPolicy {
    Unspecified,
    Allowed,
    ProhibitedAiMlTraining,
    ProhibitedGenerativeAiTraining,
    ProhibitedExceptSearchEngineIndexing,
    ProhibitedAllDataMining,
    ProhibitedSeeConstraints,
}
```

- `to_dmi_value()` — Converts to `Option<DmiValue>` (None for Unspecified)
- `from_dmi_value()` — Converts from `DmiValue` back to `RightsPolicy`
- Never inferred from processing intensity — always explicit

### HiddenMarkerMode

Controls steganographic embedding:

```rust
pub enum HiddenMarkerMode {
    Disabled,
    BestEffort,
    Tiled { tile_size: u32 },
}
```

### AuthenticationMode

Controls payload authentication:

```rust
pub enum AuthenticationMode {
    None,
    Hmac,
}
```

### ProtectionChannels

Configuration of protection channels:

```rust
pub struct ProtectionChannels {
    pub rights_metadata: bool,
    pub hidden_marker: HiddenMarkerMode,
    pub authentication: AuthenticationMode,
}
```

- `metadata_only()` — Rights metadata only, no stego or auth
- `with_hidden_marker()` — Adds best-effort steganography
- `authenticated()` — Best-effort stego + HMAC authentication

### ProcessingOptions

Image processing options:

```rust
pub struct ProcessingOptions {
    pub output_format: Option<ImageOutputFormat>,
    pub jpeg_quality: Option<u8>,
    pub progressive_jpeg: Option<bool>,
    pub max_dimension: Option<u32>,
    pub metadata_update_policy: MetadataUpdatePolicy,
}
```

### ProtectionRequest

Request-based API entry point:

```rust
pub struct ProtectionRequest {
    notice: RightsNotice,
    policy: RightsPolicy,
    channels: ProtectionChannels,
    options: ProcessingOptions,
    mac_key: Option<Vec<u8>>,
    legal_metadata: Option<LegalMetadata>,
    context: ProtectionContext,
}
```

- `metadata_only(notice, policy)` — Fastest path: same-format output with metadata only
- `with_hidden_marker(notice, policy)` — Adds best-effort steganography
- `from_preset(preset, notice, policy)` — Creates from a `ProtectionPreset`
- Builder methods: `with_mac_key()`, `with_legal_metadata()`, `with_processing_options()`

### ResolvedProtectionPlan

Immutable execution plan produced by `resolve_request()`. Consumed by pipeline stages:

```rust
pub struct ResolvedProtectionPlan {
    // Immutable — pipeline stages read from this, never mutate
}
```

### ProtectionPreset

Executable presets that expand into `ProtectionChannels`:

```rust
pub enum ProtectionPreset {
    LegalNotice,
    LegalNoticeWithStego,
    AuthenticatedProvenance,
    Maximal,
}
```

- `to_channels()` — Expands preset into `ProtectionChannels`

### ExecutionReport

Tracks which channels executed and any degradation:

```rust
pub struct ExecutionReport {
    pub metadata_injected: bool,
    pub stego_succeeded: bool,
    pub authentication_performed: bool,
    pub warnings: Vec<ProtectionWarning>,
}
```

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

## Migration Guide: v0.3 → v0.4 (Release 4)

### Deprecated APIs and their replacements

| Deprecated | Replacement | Notes |
|-----------|-------------|-------|
| `EvidenceProfile` | `ProtectionPreset` | Presets control actual channel behavior, not just warning severity |
| `ProtectionContext::with_dmi()` | `RightsPolicy` in `ProtectionRequest` | Policy is never inferred from processing intensity |
| `ProtectionContext::with_metadata_injection()` | `ProtectionChannels::rights_metadata` | Channel configuration is explicit |
| `ProtectionContext::with_inject_legal_claims()` | `LegalMetadata` in request | Auto-injected when metadata is present |

### Before/after examples

**Metadata-only legal notice (simplest workflow):**

```rust
// v0.3 (deprecated)
let ctx = ProtectionContext::new(0.0, 42)
    .with_dmi(DmiValue::ProhibitedAiMlTraining);
process_image_bytes(&bytes, ProtectionLevel::Light, &ctx)

// v0.4 (recommended)
let request = ProtectionRequest::metadata_only(
    RightsNotice::new().with_copyright_holder("Author"),
    RightsPolicy::ProhibitedAiMlTraining,
);
process_request_bytes(&bytes, &request)
```

**Metadata + hidden marker:**

```rust
// v0.3
let ctx = ProtectionContext::new(0.5, 42)
    .with_dmi(DmiValue::ProhibitedAiMlTraining);
process_image_bytes(&bytes, ProtectionLevel::Standard, &ctx)

// v0.4
let request = ProtectionRequest::with_hidden_marker(
    RightsNotice::new().with_copyright_holder("Author"),
    RightsPolicy::ProhibitedAiMlTraining,
)
.with_seed(42)
.with_intensity(0.5);
process_request_bytes(&bytes, &request)
```

**Authenticated provenance:**

```rust
// v0.3
let ctx = ProtectionContext::new(0.7, 42)
    .with_mac_key(b"secret".to_vec())
    .with_dmi(DmiValue::Prohibited);
process_image_bytes(&bytes, ProtectionLevel::Standard, &ctx)

// v0.4
let request = ProtectionRequest::from_preset(
    ProtectionPreset::AuthenticatedProvenance,
    RightsNotice::new().with_copyright_holder("Author"),
    RightsPolicy::ProhibitedAllDataMining,
)
.with_seed(42)
.with_mac_key(b"secret".to_vec());
process_request_bytes(&bytes, &request)
```

**Legacy compatibility adapter:**

```rust
// Old code still works with deprecation warnings
let ctx = ProtectionContext::new(0.5, 42);
process_image_bytes(&bytes, ProtectionLevel::Standard, &ctx)

// Equivalent new code
let plan = resolve_request(
    &ProtectionRequest::with_hidden_marker(notice, policy).with_seed(42),
    ImageOutputFormat::Png,
).unwrap();
process_request_bytes(&bytes, &request)
```

### Key behavioral changes

1. `ProtectionLevel::to_request()` provides a compatibility bridge — maps Disabled/Light/Standard to equivalent `ProtectionRequest` configurations
2. Metadata-only is now the default legal-notice path (no steganographic processing)
3. `resolve_request()` validates all inputs before processing begins — invalid combinations fail fast
4. `ExecutionReport` tracks which channels were requested, executed, and degraded
5. `RightsPolicy` is always explicit — never inferred from intensity, format, or channel selection

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

Minimum counts required per category/format for coverage enforcement, including 7 source-aware external minimums:

```rust
pub struct CoverageMinimums {
    // Internal minimums
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
    // Source-aware external minimums
    pub external_canonical_png: usize,   // default: 1
    pub external_canonical_jpeg: usize,  // default: 1
    pub external_canonical_webp: usize,  // default: 1
    pub external_legacy_min: usize,      // default: 1
    pub external_alt_prefix_min: usize,  // default: 1
    pub external_conflict_min: usize,    // default: 1
    pub external_preservation_min: usize,// default: 1
}
```

Note: The blanket `external_coverage_pct` field has been removed. Coverage is now enforced via these explicit per-category and per-format minimums, including source-aware external minimums that ensure externally-authored fixtures exist across categories. The `malformed_per_format` field ensures malformed fixtures cover each format individually.

## ConformanceReport

The JSON report type for per-fixture conformance results. In addition to the
core fields (`fixture`, `format`, `decode_valid`, `xmp_valid`, `internal`,
`external`, `checks`, `conflicts`, `passed`), it includes manifest-linked metadata:

- `fixture_id: Option<String>` — Manifest fixture ID, if matched
- `category: Option<String>` — Fixture category from manifest, if matched
- `source: Option<String>` — Fixture source classification, if matched

These fields are populated from the manifest entry when the fixture is found in the manifest. They are skipped in JSON serialization when `None`.

## ConformanceRunReport

Versioned run report envelope wrapping all conformance results. Emitted as the top-level JSON object.

```rust
pub struct ConformanceRunReport {
    pub schema_version: u32,           // Report format version (currently 1)
    pub generated_by: String,          // Tool identifier
    pub crate_version: String,         // Crate version
    pub commit_sha: Option<String>,    // Git commit, if available
    pub strict: bool,                  // Whether strict mode was enabled
    pub complete: bool,                // Whether all required inputs/tools available
    pub passed: bool,                  // complete && no required check failed
    pub started_at: Option<String>,    // ISO 8601 timestamp
    pub manifest: Option<ManifestReport>,
    pub tools: Vec<ToolReport>,
    pub coverage_minimums: Option<CoverageMinimums>,
    pub coverage: Option<CoverageCheckResult>,
    pub digest_verification: Vec<DigestCheckResult>,
    pub summary: ConformanceSummary,
    pub incomplete_reasons: Vec<String>,
    pub fixtures: Vec<ConformanceReport>,
}
```

- `complete` is false when tools are missing, fixtures are incomplete, or other required inputs are unavailable
- `passed` requires `complete == true` and no check with `Fail` severity
- The envelope is written on every meaningful exit path (pass, fail, coverage violation)

## ToolReport

Report on a single external tool used during a conformance run:

```rust
pub struct ToolReport {
    pub name: String,            // Logical name (e.g., "exiftool")
    pub path: Option<String>,    // Resolved executable path
    pub version: Option<String>, // Version string
    pub discovered: bool,        // Whether discovery succeeded
    pub exercised: bool,         // Whether tool was run on fixtures
    pub invocations: u32,        // Total fixture invocations
    pub successes: u32,          // Successful invocations
    pub failures: u32,           // Failed invocations
}
```

## ManifestReport

Report on the manifest used during a conformance run:

```rust
pub struct ManifestReport {
    pub requested_path: String,
    pub canonical_path: Option<String>,
    pub sha256: String,               // SHA-256 of the manifest file
    pub entry_count: usize,
    pub validation: Result<(), Vec<String>>,
    pub duplicate_count: usize,
    pub unlisted_count: usize,        // On disk but not in manifest
    pub unexercised_count: usize,     // In manifest but not processed
}
```

## DigestCheckResult

Result of SHA-256 digest verification for a single fixture:

```rust
pub struct DigestCheckResult {
    pub fixture_id: String,
    pub fixture_path: String,
    pub expected: String,   // Expected SHA-256 hex digest
    pub observed: String,   // Actual SHA-256 hex digest
    pub matches: bool,
}
```

## InternalExtraction / ExternalExtraction — has_notice_content()

Both `InternalExtraction` and `ExternalExtraction` implement `has_notice_content() -> bool`. This predicate returns true when any legal/rights-notice content is present (copyright, creators, usage terms, DMI, AI constraints, TDM reservation, etc.). It is used by the harness to evaluate `ExtractionExpectation::NoNotice` — a "no notice" expectation requires successful extraction AND `has_notice_content() == false`.

## CoverageCheckResult

Result of coverage enforcement. Includes both pass/fail status and observed counts:

```rust
pub struct CoverageCheckResult {
    pub passed: bool,
    pub violations: Vec<String>,
    // Internal observed counts
    pub observed_canonical_png: usize,
    pub observed_canonical_jpeg: usize,
    pub observed_canonical_webp: usize,
    pub observed_legacy: usize,
    pub observed_conflict: usize,
    pub observed_malformed: usize,
    pub observed_preservation: usize,
    // External source-aware observed counts
    pub observed_external_canonical_png: usize,
    pub observed_external_canonical_jpeg: usize,
    pub observed_external_canonical_webp: usize,
    pub observed_external_legacy: usize,
    pub observed_external_alt_prefix: usize,
    pub observed_external_conflict: usize,
    pub observed_external_preservation: usize,
}
```

## validate_manifest()

```rust
pub fn validate_manifest(manifest: &FixtureManifest) -> Result<(), Vec<String>>
```

Validates manifest structure before processing fixtures. Checks for duplicate IDs, duplicate paths, empty IDs, path traversal, unsupported formats/categories/sources, and SHA-256 validity (64 hex characters). Returns `Ok(())` if valid, or `Err(Vec<String>)` with all violation messages. Called early in the conformance harness to catch manifest issues before any fixtures are processed.

## Serialization Notes

- `ProtectionContext.config` is `#[serde(skip)]` — MAC keys and legal metadata are lost in serde roundtrips
- A test (`test_config_skipped_in_serde_roundtrip`) documents this behavior
