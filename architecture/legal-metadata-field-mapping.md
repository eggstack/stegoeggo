# Legal Metadata Field Mapping

## Field Mapping Table

| Caller concept | LegalMetadata field | PNG tEXt/COM key | JPEG COM key | WebP XMP property | Extraction key | Standard | Notes |
|---|---|---|---|---|---|---|---|
| Copyright notice | `copyright_holder` | `Copyright` tEXt | `Copyright:` COM | `dc:rights` (rdf:Alt) | `Copyright` tEXt, `Copyright:` COM, `dc:rights` | Dublin Core `dc:rights` | Free-text rights statement. Prefix "Copyright (c) " added if missing. |
| Creator | `creator` | `Creator` tEXt | `Creator:` COM | `dc:creator` (rdf:Seq) | `Creator` tEXt, `Creator:` COM, `dc:creator` | Dublin Core `dc:creator` | One or more creators in rdf:Seq. |
| Contact email | `contact_email` | `Contact` tEXt | `Contact:` COM | **Not injected** (extraction reads `photoshop:Credit`) | `Contact` tEXt, `Contact:` COM, `photoshop:Credit` (WebP only) | IPTC `Iptc4xmpExt:CreatorContactInfo` (not used) | **WebP round-trip broken**: extraction reads `photoshop:Credit` but writer never injects it. `photoshop:Credit` semantically is credit line, not contact. |
| License URL | `license_url` | `License` tEXt | `License:` COM | **Not injected** | `License` tEXt → `rights_url`, `License:` COM → `rights_url`, `xmpRights:WebStatement` (WebP) | `xmpRights:WebStatement` | **WebP round-trip broken**: writer never injects `license_url` into WebP XMP. Conflated with `web_statement_of_rights` in extraction. |
| Usage terms | `usage_terms` | `UsageTerms` tEXt | `UsageTerms:` COM | `xmpRights:UsageTerms` (rdf:Alt) | `UsageTerms` tEXt, `UsageTerms:` COM, `xmpRights:UsageTerms` | Adobe XMP Rights `xmpRights:UsageTerms` | rdf:Alt container with xml:lang="x-default". |
| Content creation date | `creation_date` | `DateCreated` tEXt | `DateCreated:` COM | **Not injected** | **Not extracted** (no match arm) | IPTC `Iptc4xmpExt:DateCreated` | **Broken round-trip**: written to PNG/JPEG but never extracted. Not in XMP. |
| AI constraints | `ai_constraints` | `AIConstraints` tEXt | `AIConstraints:` COM | `stegoeggo:AIConstraints` | `AIConstraints` tEXt, `AIConstraints:` COM, `stegoeggo:AIConstraints` | Custom `stegoeggo:` namespace | No standard property exists. Custom namespace is correct approach. |
| Web statement of rights | `web_statement_of_rights` | `WebStatementOfRights` tEXt | `WebStatementOfRights:` COM | `xmpRights:WebStatement` | `WebStatementOfRights` tEXt → `rights_url`, `WebStatementOfRights:` COM → `rights_url`, `xmpRights:WebStatement` | Adobe XMP Rights `xmpRights:WebStatement` | Conflated with `license_url` in extraction output. |
| DMI (Data Mining) | `dmi_value` (on ProtectionContext) | `DMI-PROHIBITED` tEXt + XMP `plus:DataMining` | XMP `plus:DataMining` + EXIF UserComment + IPTC Tag 120 | XMP `plus:DataMining` | `plus:DataMining` attr/element, legacy `Iptc4xmpExt:DMI-*` | PLUS LDF ISO 24138:2024 | Canonical PLUS mapping. Legacy backward compat maintained. |
| Protection seed | `seed` (on ProtectionContext) | `X-Protection-Seed` tEXt + `Description` tEXt + XMP `stegoeggo:ProtectionSeed` attr | COM `X-Protection-Seed:` + structured COM `cloakrs:v1:` + IPTC Tag 5 + XMP attr | XMP `stegoeggo:ProtectionSeed` attr + EXIF UserComment | Multiple extraction paths | Custom `stegoeggo:` namespace | IPTC Tag 5/120 technically misused. |
| noai/noindex | (always injected) | `noai` tEXt = `noindex` | COM `noai: noindex` | **Not injected** | **Not extracted** | No standard | Informal scraper poison pill. Asymmetry: not in WebP. |

## Critical Issues

1. **WebP contact round-trip broken**: `photoshop:Credit` read during extraction but never written during injection
2. **`creation_date` dead field**: Written to PNG tEXt/JPEG COM but never extracted; not in XMP
3. **`license_url` missing from WebP XMP**: Only `web_statement_of_rights` writes to `xmpRights:WebStatement`
4. **`rights_url` field collision**: `license_url` and `web_statement_of_rights` both map to `rights_url` in NoticeVerification, causing silent data loss when both are set
5. **`photoshop:` namespace declared but unused** in XMP packets

## Non-Issues (by design)

- Custom `stegoeggo:` namespace for AIConstraints and ProtectionSeed — no standard properties exist
- IPTC Tag 5/120 misuse — pragmatic for seed storage; no standard alternative
- `noai`/`noindex` — informal scraper poison pill, not a standards-based property
