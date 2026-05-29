# Core Types

**Source:** `src/types.rs` (~668 lines)

Defines all core data structures used across the codebase. Uses builder pattern with `#[must_use]` on builder methods.

## ProtectionLevel

Enum with five variants:

```rust
pub enum ProtectionLevel {
    Disabled,    // No protection (PassthroughProtector)
    Light,       // Metadata injection only (MetadataTrapProtector)
    Standard,    // Noise + stego + metadata (default)
    Enhanced,    // Higher intensity noise + stego + metadata
    Strong,      // Precomputed variants + stego + metadata
}
```

- `to_byte()` / `from_byte()` — For stego payload serialization
- `Default` returns `Standard`

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

Data Mining Inhibitor enum for IPTC metadata:

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

Auto-mapped from `ProtectionLevel` via helper in `metadata_trap.rs`: Light→Prohibited, Standard→ProhibitedAiMlTraining, Enhanced→ProhibitedGenAiMlTraining, Strong→Prohibited. No `impl From<ProtectionLevel> for DmiValue>` exists — use the helper function directly.

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
| `dmi_value` | `Option<DmiValue>` | None | IPTC DMI override |
| `max_dimension` | `Option<u32>` | None | Resize constraint |
| `inject_metadata` | `Option<bool>` | None | Enable metadata injection |
| `inject_legal_claims` | `Option<bool>` | None | Enable legal metadata |
| `stego_redundancy` | `usize` | 2 | Stego passes (1–5) |
| `jpeg_quality` | `u8` | 90 | JPEG encoding quality |
| `progressive_jpeg` | `bool` | false | Progressive JPEG encoding |
| `config` | `Option<Arc<ProtectionConfig>>` | None | `#[serde(skip)]` — MAC key + legal metadata |

**Note:** `None` for `inject_metadata`/`inject_legal_claims` means "use level default" (enabled for Standard+). Explicit `false` disables injection.

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
    pub mac_key: Option<Vec<u8>>,
    pub legal_metadata: Option<LegalMetadata>,
}
```

## LegalMetadata

Builder-pattern struct for legal/copyright metadata:

Fields: `copyright_holder`, `contact_email`, `license_url`, `usage_terms`, `creation_date`, `ai_constraints`, `web_statement_of_rights`.

## ProtectedVariant

Precomputed perturbation storage for CDN/WAF:

```rust
pub struct ProtectedVariant {
    variant_id: uuid::Uuid,
    original_hash: String,
    protection_level: ProtectionLevel,  // Added in code but missing from docs
    perturbation_data: Vec<u8>,
    intensity: f32,
    width: u32,
    height: u32,
}
```

- `cache_key()` — Returns `{hash}_{level}_{intensity}` for CDN caching
- `new(original_hash, protection_level, perturbation_data, intensity, width, height)`

## StegoPayload

Extracted stego data (returned from `SteganographyProtector::extract_payload`):

- `protection_level() -> u8`
- `seed() -> u64`
- `intensity() -> f32`
- `version() -> u8`

## Serialization Notes

- `ProtectionContext.config` is `#[serde(skip)]` — MAC keys and legal metadata are lost in serde roundtrips
- A test (`test_config_skipped_in_serde_roundtrip`) documents this behavior
