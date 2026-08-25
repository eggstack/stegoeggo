# Legal Metadata Field Mapping

## Field Mapping Table

| Caller concept | LegalMetadata field | PNG tEXt/COM key | JPEG COM key | WebP XMP property | Extraction key | Standard | Notes |
|---|---|---|---|---|---|---|---|
| Copyright notice | `copyright_holder` | `Copyright` tEXt | `Copyright:` COM | `dc:rights` (rdf:Alt) | `Copyright` tEXt, `Copyright:` COM, `dc:rights` | Dublin Core `dc:rights` | Free-text rights statement. Prefix "Copyright (c) " added if missing. |
| Creator | `creator` | `Creator` tEXt | `Creator:` COM | `dc:creator` (rdf:Seq) | `Creator` tEXt, `Creator:` COM, `dc:creator` | Dublin Core `dc:creator` | One or more creators in rdf:Seq. |
| Contact email | `contact_email` | `Contact` tEXt | `Contact:` COM | **Not injected** (extraction reads `photoshop:Credit`) | `Contact` tEXt, `Contact:` COM, `photoshop:Credit` (WebP only) | IPTC `Iptc4xmpExt:CreatorContactInfo` (not used) | **WebP round-trip broken**: writer never injects `contact_email` into WebP XMP. `photoshop:Credit` is written but sourced from `credit_line`, not `contact_email`. Extraction maps `photoshop:Credit` to `credit_line`; `contact` stays hard-coded `None`. |
| License URL | `license_url` | `License` tEXt | `License:` COM | `xmpRights:WebStatement` (fallback when `web_statement_of_rights` unset) | `License` tEXt → `rights_url`, `License:` COM → `rights_url`, `xmpRights:WebStatement` (WebP) | `xmpRights:WebStatement` | Writer injects `license_url` into `xmpRights:WebStatement` as fallback. Conflated with `web_statement_of_rights` in extraction. |
| Usage terms | `usage_terms` | `UsageTerms` tEXt | `UsageTerms:` COM | `xmpRights:UsageTerms` (rdf:Alt) | `UsageTerms` tEXt, `UsageTerms:` COM, `xmpRights:UsageTerms` | Adobe XMP Rights `xmpRights:UsageTerms` | rdf:Alt container with xml:lang="x-default". |
| Content creation date | `creation_date` | `DateCreated` tEXt | `DateCreated:` COM | `photoshop:DateCreated` in XMP | **Not extracted** (no match arm in any format) | IPTC `Iptc4xmpExt:DateCreated` | Written in all three formats (PNG tEXt, JPEG COM, WebP XMP), never extracted. |
| AI constraints | `ai_constraints` | `AIConstraints` tEXt | `AIConstraints:` COM | `stegoeggo:AIConstraints` | `AIConstraints` tEXt, `AIConstraints:` COM, `stegoeggo:AIConstraints` | Custom `stegoeggo:` namespace | No standard property exists. Custom namespace is correct approach. |
| Web statement of rights | `web_statement_of_rights` | `WebStatementOfRights` tEXt | `WebStatementOfRights:` COM | `xmpRights:WebStatement` | `WebStatementOfRights` tEXt → `rights_url`, `WebStatementOfRights:` COM → `rights_url`, `xmpRights:WebStatement` | Adobe XMP Rights `xmpRights:WebStatement` | Conflated with `license_url` in extraction output. |
| DMI (Data Mining) | `dmi_value` (on ProtectionContext) | XMP `plus:DataMining` (full URI) | XMP `plus:DataMining` (full URI) + EXIF UserComment + IPTC Tag 120 | XMP `plus:DataMining` | `plus:DataMining` attr/element (full URI), legacy `Iptc4xmpExt:DMI-*` (bare keys) | PLUS LDF ISO 24138:2024 | Canonical PLUS mapping with full URIs. Legacy bare keys parsed but not emitted. |
| Protection seed | `seed` (on ProtectionContext) | `X-Protection-Seed` tEXt + `Description` tEXt + XMP `stegoeggo:ProtectionSeed` attr | COM `X-Protection-Seed:` + structured COM `cloakrs:v1:` + IPTC Tag 5 + XMP attr | XMP `stegoeggo:ProtectionSeed` attr + EXIF UserComment | Multiple extraction paths | Custom `stegoeggo:` namespace | IPTC Tag 5/120 technically misused. |
| noai/noindex | (removed) | **Not emitted** | **Not emitted** | **Not injected** | **Not extracted** | No standard | Removed in Plan 039. Previously an informal scraper poison pill; no longer emitted in new output. |

## Critical Issues

1. **WebP contact round-trip broken**: `contact_email` never injected into WebP XMP; `photoshop:Credit` is written but sourced from `credit_line`, and extraction maps it to `credit_line` (not `contact`)
2. **`creation_date` dead field**: Written to PNG tEXt/JPEG COM/WebP XMP but never extracted
3. **`license_url` / `web_statement_of_rights` conflation**: Both map to `xmpRights:WebStatement` in extraction, causing silent data loss when both are set
4. **`rights_url` field collision**: `license_url` and `web_statement_of_rights` both map to `rights_url` in NoticeVerification, causing silent data loss when both are set

## Non-Issues (by design)

- Custom `stegoeggo:` namespace for AIConstraints and ProtectionSeed — no standard properties exist
- IPTC Tag 5/120 misuse — pragmatic for seed storage; no standard alternative
- ~~`noai`/`noindex` — informal scraper poison pill, not a standards-based property~~ (Removed in Plan 039; no longer emitted)
