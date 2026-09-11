# XMP Packet Handling

**Source:** `src/xmp.rs` (~1790 lines, `pub(crate)`) — namespace-aware XMP parse, filter, and merge on top of `quick-xml`.

Parses XMP packets without a full RDF/DOM. All decisions are made on resolved `(namespace URI, local name)` pairs via `NsReader`, never on raw prefixes, so `plus:DataMining` and `p:DataMining` compare equal when bound to the same URI.

## Owned fields

```rust
pub(crate) const OWNED_FIELDS: &[OwnedField]
pub(crate) struct OwnedField { namespace_uri, local_name }
```

11 entries: `plus:DataMining`, `plus:OtherConstraints`, and nine `stegoeggo/*` fields (`ProtectionSeed`, `ProtectionLevel`, `RightsPolicy`, `AIConstraints`, `CopyrightOwner`, `LicensorName`, `LicensorEmail`, `LicensorURL`, `NoticeAppliedAt`). `is_owned_field()` UTF-8-gates both halves before comparing.

Namespaces:

| Constant | URI |
|----------|-----|
| `PLUS_NAMESPACE` | `http://ns.useplus.org/ldf/xmp/1.0/` |
| `STEGOEGGO_NAMESPACE` | `https://github.com/eggstack/stegoeggo` |
| `RDF_NAMESPACE` | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` |

## Operations

- **Strip owned fields** — walks `rdf:Description` elements (nested and attribute form), drops owned properties/attributes, keeps unrelated third-party descriptions byte-stable via `PreservedDescription { xml, has_unrelated }`.
- **Merge** — `MetadataUpdatePolicy` (`src/types/compat.rs`: `ReplaceStegoOwned` / `PreserveExisting` / `FailOnConflict`) decides per-field whether an incoming owned value overwrites, is skipped, or errors. Namespace conflicts (same local name, different URI) are detected before merging.
- **Escape** — `escape_metadata_value()` XML-escapes `& < > " '` for attribute/element emission; shared by PNG/JPEG/WebP writers.
- **Errors** — `quick-xml` and attribute failures map to `Error::Metadata("XMP ...")`; internal state violations map to `Error::Metadata("XMP internal state error: ...")`, never panics.

## Consumers

- `protected/metadata_trap/notice.rs` — renders the canonical rights XMP block.
- `protected/metadata_trap/{png,jpeg,webp}.rs` — format-specific XMP injection points.
- `protected/notice_verification.rs` — legal-notice extraction and DMI classification.
- Field-level mapping across containers: `legal-metadata-field-mapping.md`.
- Fuzz coverage: `xmp_extract`, `metadata_merge` targets (see `testing.md`).
