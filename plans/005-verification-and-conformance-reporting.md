# Plan 005: Verification and Metadata Conformance Reporting

## Goal

Make verification report legal-notice evidence explicitly and add a path for validating generated files with external metadata tools.

The current verification model is useful for stego payloads, but it compresses the legal-notice use case into `MetadataOnly`. That is too vague for the intended purpose. Legal deterrence depends on specific fields being present and discoverable: copyright holder, rights URL, DMI/TDM reservation, AI constraints, creator/contact, and optional hidden marker status.

This plan introduces a more explicit verification report and an external conformance harness.

## Scope

This plan covers:

- New legal-notice verification data structures.
- Field extraction/reporting from generated images.
- CLI verification output that separates legal notice from stego status.
- External metadata conformance scripts or optional integration tests.
- README/docs updates for verification and parser visibility.

This plan should not attempt to provide legal conclusions. It should report what metadata is found, where possible, and what optional stego/provenance channels verify.

## Files to Inspect

- `src/lib.rs`
- `src/types.rs`
- `src/protected/metadata_trap.rs`
- `src/protected/steganography.rs`
- `stegoeggo-cli/src/main.rs`
- `README.md`
- `tests/robustness.rs`
- Any existing verification tests

## Desired Verification Model

Add a report type similar to:

```rust
pub struct NoticeVerification {
    pub copyright_holder: Option<String>,
    pub creator: Option<String>,
    pub contact: Option<String>,
    pub rights_url: Option<String>,
    pub usage_terms: Option<String>,
    pub ai_constraints: Option<String>,
    pub dmi: Option<DmiValue>,
    pub tdm_reserved: Option<bool>,
    pub protection_seed: Option<u64>,
    pub stego_status: VerificationStatus,
    pub stego_payload: Option<StegoPayload>,
    pub authenticated: bool,
    pub evidence_strength: EvidenceStrength,
    pub channels: Vec<EvidenceChannel>,
}
```

Use owned `String`s if parsing creates new values. Borrowed data is not worth the lifetime complexity for this report.

Suggested supporting enums:

```rust
pub enum EvidenceStrength {
    NoNoticeFound,
    MetadataNoticeOnly,
    MetadataNoticeAndBestEffortStego,
    MetadataNoticeAndAuthenticatedProvenance,
}

pub enum EvidenceChannel {
    PngText,
    PngXmp,
    JpegComment,
    JpegXmp,
    JpegIptc,
    WebPXmp,
    WebPExif,
    LsbPayload,
    DctPayload,
    QTableSeed,
}
```

Keep these simple. If full channel detection is too much for the first pass, implement the report fields and a smaller `channels_found: Vec<String>` or omit channel enumeration until later.

## Verification API

Add a new function rather than breaking the existing API:

```rust
pub fn verify_legal_notice(img_bytes: &[u8], mac_key: &[u8]) -> NoticeVerification
```

Alternative name:

```rust
pub fn verify_notice_metadata(img_bytes: &[u8], mac_key: &[u8]) -> NoticeVerification
```

Existing APIs should remain:

- `verify_image_bytes`
- `verify_image_bytes_detailed`

The new API should call existing stego verification internally and then parse legal-notice metadata.

## Metadata Extraction Requirements

The current metadata extractor can find a seed. Extend extraction to find notice fields.

Required fields for first pass:

- Protection seed.
- DMI value where embedded.
- Copyright holder or copyright string.
- Usage terms.
- Rights URL / web statement of rights.
- AI constraints.
- Contact.
- TDM reservation if extractable from XMP.

Support all currently emitted formats where practical:

- PNG tEXt/iTXt chunks.
- JPEG COM markers.
- JPEG XMP APP1 markers.
- JPEG IPTC/Photoshop APP13 markers if currently emitted.
- WebP XMP chunk.
- WebP EXIF chunk.

Do not overbuild a full XMP parser unless necessary. A conservative parser that extracts known generated fields is acceptable for first pass, but document that it is a stegoeggo metadata report, not a general-purpose metadata parser.

## CLI Verification Output

Update `stegoeggo image.png --verify` to report legal-notice fields first.

Suggested output:

```text
Rights notice: Found
Copyright holder: Jane Artist
Creator: Jane Artist
Contact: legal@example.com
Rights URL: https://example.com/rights
AI training restriction: ProhibitedGenAiMlTraining
TDM reservation: reserved
Usage terms: Copyrighted work. No AI training permitted.
Protection seed: 12345

Stego marker: Found, checksum verified
Authenticated provenance: Not configured
Evidence strength: MetadataNoticeAndBestEffortStego
```

If no legal notice exists but stego exists:

```text
Rights notice: Not found
Stego marker: Found
Evidence strength: Metadata/stego marker only; no rights-reservation metadata found.
```

If only metadata exists:

```text
Rights notice: Found
Stego marker: Not found
Evidence strength: MetadataNoticeOnly
```

For keyed verification, pass `--key` into verification. The current CLI should not ignore `--key` during verify mode after this plan.

## External Conformance Harness

Add a script:

```text
scripts/verify_metadata_conformance.sh
```

The script should be optional and gracefully skip checks if external tools are missing.

Suggested tools:

- `exiftool` if installed.
- `identify -verbose` from ImageMagick if installed.
- `vipsheader` or `vips` if installed.

Minimum behavior:

1. Generate tiny PNG/JPEG/WebP test images using the CLI or a Rust example.
2. Embed explicit fields:
   - copyright holder
   - rights URL
   - no-genai restriction
   - AI constraints
3. Run external metadata tools.
4. Assert that at least one external tool can see expected strings for each supported format.
5. Print a clear skipped status if no external tools are installed.

Example output:

```text
[ok] PNG: exiftool sees copyright holder
[ok] PNG: exiftool sees XMP DMI/TDM marker
[ok] JPEG: exiftool sees COM/XMP notice
[skip] WebP: exiftool not installed
```

## Optional Fixture Strategy

Do not commit large binaries. If fixtures are needed, use tiny generated images or scripts that generate them at test time.

Potential layout:

```text
tests/fixtures/README.md
scripts/generate_notice_fixtures.rs or examples/generate_notice_fixtures.rs
```

If tiny binary fixtures are committed, document how they were generated and keep them minimal.

## Documentation Updates

Update README verification section to explain:

- Legal-notice verification is field-based.
- Stego verification is separate.
- Authenticated provenance is separate and only applies when a MAC key was configured.
- External parser conformance is the strongest practical validation that notice is discoverable outside stegoeggo.

Add a small table:

```text
Result                                Meaning
NoNoticeFound                         No recognized rights-reservation metadata found
MetadataNoticeOnly                    Legal notice found, no hidden marker verified
MetadataNoticeAndBestEffortStego      Legal notice plus non-authenticated hidden marker
MetadataNoticeAndAuthenticatedProvenance Legal notice plus MAC-authenticated hidden marker
```

## Implementation Steps

1. Design `NoticeVerification`, `EvidenceStrength`, and optional channel enum in `src/types.rs`.
2. Add metadata extraction helpers in `metadata_trap.rs` or a new module such as `protected/notice_metadata.rs`.
3. Add `verify_legal_notice` or `verify_notice_metadata` public API in `src/lib.rs`.
4. Update CLI verify path to use the new API and pass `--key` when provided.
5. Add unit tests for extraction from generated PNG/JPEG/WebP outputs.
6. Add CLI snapshot-style tests if the repo has a CLI test pattern.
7. Add optional external conformance script.
8. Update README and legal notice model docs.

## Test Cases

Required library tests:

1. PNG legal-notice output verifies copyright holder, rights URL, DMI, and seed.
2. JPEG legal-notice output verifies available notice fields.
3. WebP legal-notice output verifies available notice fields.
4. Metadata-only output returns `MetadataNoticeOnly`.
5. Metadata plus unkeyed stego returns `MetadataNoticeAndBestEffortStego`.
6. Metadata plus keyed stego with correct key returns `MetadataNoticeAndAuthenticatedProvenance`.
7. Keyed stego with wrong key reports unauthenticated/corrupted stego without erasing metadata report.
8. Unprotected image returns `NoNoticeFound`.

Required CLI tests or smoke tests:

1. `--verify --key <HEX>` uses the key.
2. Verify output lists rights URL and DMI before stego status.
3. Verify output does not imply legal conclusions.

## Test Commands

Run:

```bash
cargo fmt --check
cargo test --all-features
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
```

Optional conformance:

```bash
./scripts/verify_metadata_conformance.sh
```

The conformance script should not fail just because optional external tools are not installed, unless an explicit strict mode is provided:

```bash
./scripts/verify_metadata_conformance.sh --strict
```

## Acceptance Criteria

- New verification API reports legal-notice fields explicitly.
- CLI verification displays rights metadata separately from stego status.
- CLI verification uses `--key` when provided.
- Evidence strength names are legal-notice oriented rather than security-oriented.
- External conformance script exists and can validate generated metadata when tools are installed.
- README documents how to interpret metadata-only, best-effort stego, and authenticated provenance results.

## Risk Notes

Avoid building or claiming a full standards-complete metadata parser unless one is actually implemented. The internal extractor can be scoped to metadata generated by stegoeggo.

Do not say that discovered metadata proves infringement. Verification should report evidence channels, not legal conclusions.

Make sure wrong-key verification does not cause metadata evidence to be hidden. Legal notice should remain reportable even when hidden authenticated provenance fails.
