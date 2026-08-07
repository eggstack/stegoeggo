# Metadata Trap Protector

**Source:** `src/protected/metadata_trap.rs` (~2043 lines)

The largest module. Injects metadata into image files for the `Light` protection level. Operates at the byte level — the `DynamicImage` API cannot preserve injected text chunks through encode/decode cycles. Estimated latency: 2ms.

## Key Behavior

- **`apply()` returns `Cow::Borrowed(img)` unchanged** — metadata injection cannot survive through the `DynamicImage` API
- **`apply_bytes()` / `inject_bytes()`** — The actual metadata injection happens here
- **Pipeline routes `Light` level through `apply_light_bytes()`** which encodes, injects metadata, then decodes

## Metadata Types

### IPTC Data Mining Inhibitor (DMI)

Seven variants mapped to canonical PLUS LDF vocabulary keys. Legacy `Iptc4xmpExt:DMI-*` properties are parsed for backward compatibility but not emitted by default:

| DmiValue | Canonical Key (emitted) |
|----------|--------------------------------------|
| `Unspecified` | Not injected — no `plus:DataMining` property emitted |
| `Allowed` | `DMI-Allowed` |
| `Prohibited` | `DMI-Prohibited` |
| `ProhibitedAiMlTraining` | `DMI-PROHIBITED-AIMLTRAINING` |
| `ProhibitedGenAiMlTraining` | `DMI-PROHIBITED-GENAIMLTRAINING` |
| `ProhibitedExceptSearchEngineIndexing` | `DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING` |
| `ProhibitedSeeConstraints` | `DMI-PROHIBITED-SEECONSTRAINT` |

Note: These are bare `plus_vocab_key()` values. XMP `plus:DataMining` attributes emit the full URI form (`http://ns.useplus.org/ldf/vocab/{key}`).

### DMI Auto-Mapping

When no explicit DMI value is set, the protector auto-maps from `ProtectionLevel`:

- Light → `Prohibited`
- Standard → `ProhibitedAiMlTraining`

### Legal Metadata

When `inject_legal_claims` is enabled (or auto-enabled by the presence of `LegalMetadata`), injects only the fields explicitly provided:
- Copyright holder (if set)
- Contact email (PNG tEXt / JPEG COM only — not mapped to `photoshop:Credit` in XMP)
- License URL
- Usage terms (if set)
- Creation date (only if caller-supplied — never synthesized from processing time)
- AI training constraints (if set)
- Web statement of rights (if set)
- Creator (if set)
- Credit line (if set)
- Copyright owner (if set)
- Licensor name (if set)
- Licensor email (if set)
- Licensor URL (if set)
- Metadata date (if set)
- Notice applied at (auto-computed RFC 3339 timestamp if not explicitly provided)

No synthetic defaults are emitted when no `LegalMetadata` is provided — no "All Rights Reserved", no default usage terms, no processing-time DateCreated.

## Format-Specific Injection

### JPEG

Injects four marker types:
1. **EXIF (APP1)** — Basic EXIF orientation
2. **IPTC-IIM (APP13)** — Photoshop 3.0 identifier + IPTC dataset records
3. **XMP (APP1)** — Full XMP packet with canonical `plus:DataMining` rights signals and legal metadata
4. **COM** — Human-readable key-value pairs (Copyright, Creator, Contact, CreditLine, CopyrightOwner, LicensorName, LicensorEmail, LicensorURL, MetadataDate, NoticeAppliedAt, UsageTerms, AIConstraints) + structured binary COM (`cloakrs:v1:`)

**Container preservation**: `inject_text_chunks_jpeg` walks the raw JPEG byte stream, copying all pre-SOS segments verbatim and inserting StegoEggo metadata before the SOS marker. The SOS scan data and all segments after it (including EOI) are copied byte-for-byte. This ensures that unrelated APP0, APP1, APP2, APP13, APP14, COM, DRI, and unknown segments survive metadata injection unchanged.

**External visibility**: Legal fields are stored as separate COM markers. External tools like `exiftool` require the `-a` flag to read all COM markers (not just the first). XMP DMI fields use canonical `plus:DataMining` and are visible via standard XMP extraction. TDM reservation is no longer emitted by default.

### PNG

Injects two chunk types:
1. **tEXt** — Key-value text chunks (Copyright, Creator, Contact, CreditLine, CopyrightOwner, LicensorName, LicensorEmail, LicensorURL, MetadataDate, NoticeAppliedAt, UsageTerms, AIConstraints, X-Protection-Seed, Description, etc.)
2. **iTXt** — International text chunks (for XMP data, UTF-8 encoded)

**External visibility**: tEXt chunks map to standard exiftool fields (e.g., `exiftool -Copyright`). XMP in iTXt is extractable via standard XMP tools.

### WebP

Injects into a valid extended WebP container:
1. **VP8X chunk** — Created when converting simple VP8/VP8L to extended WebP; contains correct canvas dimensions (3-byte LE) and feature flags derived from final output chunks (XMP=0x04, ICC=0x20, EXIF=0x08, Alpha=0x10, Animation=0x02)
2. **XMP chunk** — Canonical rights metadata: `plus:DataMining` DMI, seed (`stegoeggo:ProtectionSeed`), plus legal fields (`dc:rights`, `dc:creator`, `xmpRights:UsageTerms`, `stegoeggo:AIConstraints`, `photoshop:Credit`, etc.). At most one XMP chunk in output; existing non-StegoEggo XMP properties preserved under `ReplaceStegoOwned`.
3. **No EXIF seed emission** — EXIF seed chunks are no longer emitted. Historical EXIF seed data is still parsed for backward compatibility. Seed is available via XMP.

**Container assembly**: VP8X first, then image payload (VP8/VP8L byte-identical), then ancillary chunks (ICCP, unknown), then metadata (XMP). RIFF size updated. Odd-length chunks padded.

**External visibility**: XMP DMI fields use canonical `plus:DataMining` and are visible via `exiftool`. Legal fields are embedded as standard XMP properties. `photoshop:Credit` maps to `credit_line` (not `contact`). Contact is NOT written to `photoshop:Credit`; it is only available in PNG tEXt and JPEG COM markers.

## Metadata Merge Behavior

When an already-protected image is re-processed, the pipeline applies a `MetadataUpdatePolicy` (defined in `src/types.rs`) to control how existing StegoEggo metadata is handled. The policy is applied at the metadata injection stage.

### Normalization Model

All format writers (PNG tEXt, JPEG COM, WebP XMP) consume the same `RightsNotice` struct, which is produced once per processing invocation by `ProtectionContext::normalize_rights_notice()`. This ensures semantically equivalent metadata regardless of output format. The normalization resolves DMI defaults, applies auto-computed timestamps, and merges `LegalMetadata` fields with context-level overrides.

### Policies

| Policy | Default | Behavior |
|--------|---------|----------|
| `ReplaceStegoOwned` | Yes | Strips existing StegoEggo-owned metadata and injects fresh. Unrelated metadata (camera EXIF, color profiles, custom user chunks, non-StegoEggo XMP properties) is preserved. For WebP: existing non-StegoEggo XMP properties are preserved in the merged XMP chunk. |
| `FailOnConflict` | No | Returns `Error::Metadata` if StegoEggo metadata already exists in the image. Use when strict first-write semantics are required. |
| `PreserveExisting` | No | Skips metadata injection entirely if StegoEggo metadata already exists. Existing properties are not overwritten. |

### Format-Specific Merge Rules

- **PNG**: New tEXt chunks are appended before IEND. Existing StegoEggo tEXt chunks are not removed; extraction returns the most recently written value.
- **JPEG**: New COM markers are appended before the Start-of-Scan marker. Existing COM markers are not removed; extraction returns the most recently written value.
- **WebP**: XMP chunk is merged/replaced via `crate::xmp::filter_xmp_packet` (the whole-packet namespace-aware `quick-xml::NsReader` pipeline). Existing non-StegoEggo XMP properties are preserved under `ReplaceStegoOwned`. Output loop skips original XMP chunks to prevent duplicates. VP8X flags are derived from final emitted chunk inventory via `WebPFeatures` and validated independently against the declared flags. Malformed XMP fails the complete rewrite before any output byte is returned.
- **Animated WebP**: `inject_text_chunks_webp_from_notice` copies existing ANIM and ANMF chunk bytes unchanged. It only adjusts VP8X flags, replaces the XMP chunk set, and recomputes RIFF size and padding. Frame payloads are never re-encoded.
- **DMI and seed values**: Always replaced regardless of policy, because they are protection-critical and must match the current processing context.

### XMP Merge Mechanics

`merge_preserved_descriptions` is the single merge site. It parses the canonical new packet structurally via `quick-xml`, identifies the RDF container by namespace URI + local name, streams every event to the output, and inserts deduped preserved descriptions immediately before the matching `</rdf:RDF>` End event. The canonical packet must contain exactly one usable RDF container; malformed input returns `Err` rather than falling back to the unmodified packet.

`deduplicate_descriptions` keeps only the first occurrence of byte-identical preserved XML and excludes any preserved that is byte-identical to a canonical description. Safe sibling-local namespace prefix reuse (the same textual prefix mapping to different URIs in separate descriptions) is not treated as a global conflict.

### Idempotency

With the default `ReplaceStegoOwned` policy, processing an image twice with the same context produces output where the metadata layer is semantically idempotent — extraction returns the same values. The steganographic payload will differ because the seed changes.

## Seed Extraction

```rust
pub fn extract_seed_from_image(img_bytes: &[u8]) -> Option<u64>
```

Extracts the `X-Protection-Seed` value from:
- PNG tEXt chunks
- JPEG COM markers
- WebP XMP chunks (via `stegoeggo:ProtectionSeed` attribute)
- WebP EXIF chunks (historical compatibility, byte search for `Protection seed:`)

Used by the verification pipeline to recover the seed for stego extraction.

## XMP Generation

Generates XMP packets with canonical `plus:DataMining` rights signals. The `<rdf:Description>` opening tag is closed
before any legal child elements are inserted, and `dc:rights` / `xmpRights:UsageTerms`
are wrapped in `<rdf:Alt><rdf:li xml:lang="x-default">…</rdf:li></rdf:Alt>` containers
so external RDF parsers (e.g. `exiftool`) can read the legal fields:

```xml
<x:xmpmeta xmlns:plus="..." xmlns:dc="..." xmlns:xmpRights="..." xmlns:photoshop="...">
  <rdf:RDF>
    <rdf:Description rdf:about=""
      plus:DataMining="http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING"
      stegoeggo:ProtectionSeed="...">
      <dc:creator>
        <rdf:Seq>
          <rdf:li>Test Author</rdf:li>
        </rdf:Seq>
      </dc:creator>
      <dc:rights>
        <rdf:Alt>
          <rdf:li xml:lang="x-default">Copyright (c) Test Corp</rdf:li>
        </rdf:Alt>
      </dc:rights>
      <xmpRights:UsageTerms>
        <rdf:Alt>
          <rdf:li xml:lang="x-default">All rights reserved</rdf:li>
        </rdf:Alt>
      </xmpRights:UsageTerms>
      <xmpRights:WebStatement>https://example.com/rights</xmpRights:WebStatement>
      <stegoeggo:AIConstraints>No AI training</stegoeggo:AIConstraints>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
```

## Utility Functions

- `current_date_iso()` — Manual ISO date computation (test-only, no chrono dependency)
- CRC32 computation for PNG chunk checksums

## Module Interactions

- **lib.rs**: Selected for `Light` and `Standard` levels (always runs)
- **types.rs**: Uses `DmiValue`, `ProtectionLevel`, `LegalMetadata`
- **traits.rs**: Implements `Protector` trait
- **protected/constants.rs**: Uses `STEGO_OFFSET_SEED_1` for seed embedding
