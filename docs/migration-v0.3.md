# Migration Guide: v0.2.x to v0.3.0

## Overview

v0.3.0 introduces semantic correctness for legal metadata across all formats (PNG, JPEG, WebP). The same `LegalMetadata` input now produces semantically equivalent output regardless of output format.

## Breaking Changes

### `with_legal_claims()` deprecated

The `with_legal_claims(bool)` method on `ProtectionContext` is deprecated. Legal claims are now automatically enabled when `LegalMetadata` is present.

**Before:**
```rust
let ctx = ProtectionContext::new(0.5, 42)
    .with_legal_metadata(legal)
    .with_legal_claims(true);
```

**After:**
```rust
let ctx = ProtectionContext::new(0.5, 42)
    .with_legal_metadata(legal);
```

Calling `with_legal_claims(false)` while legal metadata is present now emits a `ContradictoryLegalClaims` warning.

### New fields on `LegalMetadata`

Seven new fields were added:
- `credit_line` — Maps to `photoshop:Credit` (was incorrectly mapped from `contact_email`)
- `copyright_owner` — Distinct from `copyright_holder` (the notice text)
- `licensor_name`, `licensor_email`, `licensor_url` — Structured licensor records
- `metadata_date` — `xmp:MetadataDate` timestamp
- `notice_applied_at` — Auto-computed RFC 3339 timestamp when not provided

### `photoshop:Credit` semantic fix

`photoshop:Credit` now maps to `credit_line`, not `contact_email`. If you were using `with_contact_email()` expecting it to appear as `photoshop:Credit` in WebP XMP, use `with_credit_line()` instead.

### Date validation

Date fields (`creation_date`, `metadata_date`, `notice_applied_at`) now validate ISO 8601 format:
- `YYYY-MM-DD`
- `YYYY-MM-DDTHH:MM:SSZ`
- `YYYY-MM-DDTHH:MM:SS+HH:MM`

### URL validation

URL fields (`license_url`, `web_statement_of_rights`, `licensor_url`) now validate basic syntax (scheme + authority).

### `MetadataUpdatePolicy` enforcement

- `FailOnConflict` now returns `Error::Metadata` if StegoEggo metadata already exists
- `PreserveExisting` now skips injection if StegoEggo metadata already exists

### New `RightsNotice` type

A normalized `RightsNotice` struct is now produced once per processing invocation and consumed by all format writers. This is a public type but primarily for advanced use cases.

## CLI Changes

### `--copyright-notice` replaces `--copyright-holder`

The `--copyright-holder` flag is deprecated. Use `--copyright-notice` instead. The old flag still works as an alias for backward compatibility.

**Before:**
```bash
stegoeggo protect --copyright-holder "Jane Doe" image.png
```

**After:**
```bash
stegoeggo protect --copyright-notice "Jane Doe" image.png
```

### New flags

- `--credit-line` — Required credit line text
- `--copyright-owner` — Copyright owner name (distinct from notice text)
- `--licensor-name`, `--licensor-email`, `--licensor-url` — Structured licensor records
- `--content-created-at` — Content creation date (ISO 8601)

## New Types

- `RightsNotice` — Normalized rights notice (public, for advanced use)
- `LocalizedText` — Text with language tag (already public since v0.3.0)
- `ProtectionWarning::ContradictoryLegalClaims` — Warning variant

## Verification API

`NoticeVerification` now exposes 9 additional fields:
- `license_url()`, `web_statement_of_rights()`, `credit_line()`
- `copyright_owner()`, `licensor_name()`, `licensor_email()`, `licensor_url()`
- `metadata_date()`, `notice_applied_at()`
