# Metadata Trap Protector

**Source:** `src/protected/metadata_trap.rs` (~1283 lines)

The largest module. Injects metadata into image files for the `Light` protection level. Operates at the byte level — the `DynamicImage` API cannot preserve injected text chunks through encode/decode cycles. Estimated latency: 2ms.

## Key Behavior

- **`apply()` returns `Cow::Borrowed(img)` unchanged** — metadata injection cannot survive through the `DynamicImage` API
- **`apply_bytes()` / `inject_bytes()`** — The actual metadata injection happens here
- **Pipeline routes `Light` level through `apply_light_bytes()`** which encodes, injects metadata, then decodes

## Metadata Types

### IPTC Data Mining Inhibitor (DMI)

Seven variants mapped to IPTC XMP property names:

| DmiValue | IPTC Property |
|----------|---------------|
| `Unspecified` | Not injected |
| `Allowed` | `Iptc4xmpExt:DMI-Allowed` |
| `Prohibited` | `Iptc4xmpExt:DMI-Prohibited` |
| `ProhibitedAiMlTraining` | `Iptc4xmpExt:DMI-Prohibited` |
| `ProhibitedGenAiMlTraining` | `Iptc4xmpExt:DMI-Prohibited` |
| `ProhibitedExceptSearchEngineIndexing` | `Iptc4xmpExt:DMI-Prohibited` |
| `ProhibitedSeeConstraints` | `Iptc4xmpExt:DMI-Prohibited` |

### DMI Auto-Mapping

When no explicit DMI value is set, the protector auto-maps from `ProtectionLevel`:

- Light → `Prohibited`
- Standard → `ProhibitedAiMlTraining`
- Enhanced → `ProhibitedGenAiMlTraining`
- Strong → `Prohibited` (same as Light — DMI strength does not increase with protection level)

### Legal Metadata

When `inject_legal_claims` is enabled, injects:
- Copyright holder, contact email, license URL
- Usage terms, creation date, AI training constraints
- Web statement of rights

## Format-Specific Injection

### JPEG

Injects four marker types:
1. **EXIF (APP1)** — Basic EXIF orientation
2. **IPTC-IIM (APP13)** — Photoshop 3.0 identifier + IPTC dataset records
3. **XMP (APP1)** — Full XMP packet with IPTC extension properties
4. **COM** — Human-readable comment with protection seed

### PNG

Injects two chunk types:
1. **tEXt** — Key-value text chunks (e.g., `X-Protection-Seed: <seed>`)
2. **iTXt** — International text chunks (for XMP data, UTF-8 encoded)

### WebP

Injects:
1. **META chunk** — Contains XMP data
2. **XML chunk** — Additional metadata
3. Updates RIFF file size header

## Seed Extraction

```rust
pub fn extract_seed_from_image(img_bytes: &[u8]) -> Option<u64>
```

Extracts the `X-Protection-Seed` value from:
- PNG tEXt chunks
- JPEG COM markers
- WebP META chunks

Used by the verification pipeline to recover the seed for stego extraction.

## XMP Generation

Generates IPTC-compliant XMP packets:

```xml
<x:xmpmeta xmlns:Iptc4xmpExt="..."/>
  <rdf:RDF>
    <rdf:Description Iptc4xmpExt:DMI_ProhibitedAiMlTraining="">
      ...
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
```

## Utility Functions

- `current_date_iso()` — Manual ISO date computation (no chrono dependency)
- CRC32 computation for PNG chunk checksums

## Module Interactions

- **lib.rs**: Selected for `Light`, `Standard`, `Enhanced`, and `Strong` levels (always runs)
- **types.rs**: Uses `DmiValue`, `ProtectionLevel`, `LegalMetadata`
- **traits.rs**: Implements `Protector` trait
- **protected/constants.rs**: Uses `STEGO_OFFSET_SEED_1` for seed embedding
