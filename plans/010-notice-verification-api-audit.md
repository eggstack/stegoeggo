# Plan 010: Notice Verification API Audit

## Goal

Audit `verify_legal_notice()`, `NoticeVerification`, `EvidenceStrength`, and `EvidenceChannel` for conservative, accurate, legal-notice-first behavior. The API should report what was actually found, avoid overstating evidence, and keep legal notice separate from steganographic and authenticated provenance status.

This phase is especially important because downstream users may rely on `NoticeVerification` reports as part of evidence collection. Reports must be precise and boring: no false confidence, no invented channels, no conflation of metadata notice with cryptographic provenance.

## Scope

In scope:

- Field extraction accuracy.
- Evidence channel accuracy.
- Evidence strength classification.
- Wrong-key behavior.
- Metadata-only behavior.
- No-notice behavior.
- DMI/TDM interpretation.
- Public docs and tests for the verification model.

Out of scope:

- Full XMP/IPTC parser implementation beyond stegoeggo-emitted metadata.
- Legal conclusions.
- New watermarking algorithms.

## Files to Inspect

- `src/protected/notice_verification.rs`
- `src/protected/metadata_trap.rs`
- `src/protected/steganography.rs`
- `src/jpeg_transcoder/stego_f5.rs`
- `src/types.rs`
- `src/lib.rs`
- `stegoeggo-cli/src/main.rs`
- `tests/*notice*`
- `tests/*verification*`
- `README.md`
- `docs/architecture/types.md`

## API Invariants

The audit should enforce these invariants:

1. `has_notice()` is true only when meaningful rights-reservation/legal-notice metadata is found.
2. A seed alone is not legal notice.
3. A stego payload alone is not legal notice.
4. Wrong MAC key must not erase or hide metadata fields.
5. Wrong MAC key must not be reported as authenticated provenance.
6. Evidence channels are only listed when the corresponding channel was actually detected.
7. `EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance` requires both legal notice and authenticated payload verification.
8. `EvidenceStrength::MetadataNoticeAndBestEffortStego` requires legal notice plus verified unauthenticated stego payload.
9. `EvidenceStrength::MetadataNoticeOnly` requires legal notice and no verified stego payload.
10. `EvidenceStrength::NoNoticeFound` applies when no legal notice is found, even if internal non-notice artifacts are present, unless the enum is intentionally documented otherwise.

If current implementation treats stego-only as a non-`NoNoticeFound` state, either adjust the enum/model or document the distinction very explicitly. Prefer conservative legal-notice semantics.

## Evidence Channel Audit

Check each channel variant against implementation:

- `PngText`: only when PNG text/iTXt legal fields are parsed.
- `PngXmp`: only when PNG XMP metadata is actually parsed.
- `JpegComment`: only when JPEG COM legal fields are parsed.
- `JpegXmp`: only when JPEG APP1 XMP is parsed.
- `JpegIptc`: only when JPEG IPTC/APP13 legal fields are parsed.
- `WebPXmp`: only when WebP XMP chunk is parsed.
- `WebPExif`: only when WebP EXIF chunk is parsed.
- `LsbPayload`: only when LSB payload verifies.
- `DctPayload`: only when DCT/F5 payload verifies.
- `QTableSeed`: only when JPEG quantization-table seed is detected.

If a channel is reserved but not yet emitted/extracted, do not report it. Consider marking such enum variants doc-hidden or documenting them as reserved for future support.

## Field Extraction Audit

Verify extraction of:

- Copyright holder.
- Creator.
- Contact.
- Rights URL / web statement of rights.
- Usage terms.
- AI constraints.
- DMI value.
- TDM reservation.
- Protection seed.

For each field, answer:

- Which formats emit it?
- Which formats extract it?
- Does CLI verify print it?
- Does external conformance check it?
- Are docs accurate about support?

## DMI/TDM Semantics

Audit DMI and TDM carefully:

- `DmiValue::Allowed` should not be treated as a restriction.
- `DmiValue::Unspecified` should not be treated as restriction evidence.
- Prohibition values should be rendered and reported clearly.
- TDM reservation should be separate from DMI where possible.
- `--tdm-reserved` should not silently produce contradictory DMI output.

If DMI allowed plus no-AI-training flags can be combined from CLI, reject or define precedence. Contradictory notice is worse than an error.

## Wrong-Key Behavior

Add or verify tests for:

1. Correct key -> authenticated true when keyed payload verifies.
2. Wrong key -> authenticated false and stego invalid/not authenticated.
3. Wrong key -> legal metadata still visible in report.
4. Missing key on keyed payload -> metadata report still visible, authenticated false.
5. Unkeyed payload -> best-effort stego, not authenticated provenance.

CLI verify should behave the same way.

## Metadata-Only Behavior

Generate metadata-only or legal-notice-light samples and verify:

- Legal fields are reported.
- Stego status is `NotFound` or equivalent.
- Evidence strength is `MetadataNoticeOnly`.
- Channels include only metadata channels.
- CLI output does not imply hidden payload verification.

## No-Notice Behavior

Verify unprotected images:

- No legal fields.
- No channels.
- Evidence strength `NoNoticeFound`.
- CLI output exits successfully or with documented status.
- No scary false-positive language.

## Required Tests

Add or verify tests named approximately:

```text
notice_verification_unprotected_no_notice
notice_verification_metadata_only_strength
notice_verification_wrong_key_preserves_metadata
notice_verification_correct_key_authenticated
notice_verification_unkeyed_stego_best_effort
notice_verification_no_false_channels_png
notice_verification_no_false_channels_jpeg
notice_verification_no_false_channels_webp
notice_verification_dmi_allowed_not_restriction
notice_verification_dmi_unspecified_not_restriction
notice_verification_tdm_reserved_separate
cli_verify_prints_legal_fields_before_stego_status
```

## Required Commands

```bash
cargo test --all-features notice_verification
cargo test --all-features verification
cargo test --all-features cli_verify
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

## Acceptance Criteria

- `NoticeVerification` report is conservative and field-accurate.
- Evidence channels are not over-reported.
- Evidence strength is not overstated.
- Wrong-key behavior preserves metadata evidence while refusing authenticated provenance.
- DMI allowed/unspecified are not misrepresented as restrictions.
- CLI verification output reflects the same model as the library API.
- README and architecture docs match actual behavior.

## Risk Notes

Do not add a permissive parser that extracts arbitrary strings and marks them as legal notice. The verifier should recognize known metadata shapes and be explicit about uncertainty.

Avoid breaking public API names during this audit unless a correctness bug requires it.

Legal evidence reports should avoid legal conclusions. Report facts about embedded fields and verification status only.
