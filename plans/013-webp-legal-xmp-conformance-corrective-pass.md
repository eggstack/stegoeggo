# Plan 013: WebP Legal XMP Conformance Corrective Pass

## Goal

Fix the remaining WebP external metadata conformance gap: `verify_metadata_conformance.sh --format webp --strict` currently sees DMI/TDM through XMP, but `exiftool` does not see the copyright/legal notice fields. This should be corrected so WebP outputs carry the same first-class legal-notice fields as PNG/JPEG where practical.

The goal is not to make WebP forensic or tamper-proof. The goal is to make WebP protected outputs externally expose the same rights-reservation notice fields that the CLI accepts and the library reports internally.

## Observed Failure

Current strict/local conformance output can look like:

```text
=== Metadata Conformance Check: .../test_input_protected.webp ===

--- exiftool checks (webp) ---
  WARN: Copyright not found in WebP metadata (parser-dependent)
  PASS: XMP contains DMI-Prohibited: ProhibitedAiMlTraining
  PASS: XMP contains TDM reservation

--- Format-specific checks (webp) ---
  SKIP: vips not available, skipping WebP chunk verification
```

This is not a catastrophic failure because the script currently treats WebP copyright visibility as parser-dependent. However, code inspection shows the WebP injection path only emits DMI/TDM/seed XMP, not the richer legal metadata fields such as copyright holder, creator, contact, rights URL, or usage terms. That makes the warning expected and correct, but it leaves WebP below PNG/JPEG for legal-notice parity.

## Root Cause Hypothesis

`MetadataTrapProtector::inject_text_chunks_webp()` builds the WebP XMP chunk with:

```rust
Self::create_webp_xmp_chunk(&Self::generate_xmp_dmi(dmi_val, seed))
```

That helper only emits DMI/TDM/seed properties. It does not consume the `metadata` tuple list or `LegalMetadata` object. As a result, WebP XMP can expose `DMI-Prohibited` and `tdm:reserve_tdm`, while external tools have no standard XMP copyright field to report.

There is also a stale namespace in `generate_xmp_dmi()`:

```xml
xmlns:stegoeggo="https://github.com/anomalyco/stegoeggo"
```

That should be updated to:

```xml
xmlns:stegoeggo="https://github.com/eggstack/stegoeggo"
```

## Scope

In scope:

- Add legal metadata fields to WebP XMP emission.
- Preserve existing DMI/TDM/seed behavior.
- Update internal WebP `verify_legal_notice()` parsing if needed.
- Update external conformance expectations for WebP.
- Fix stale `anomalyco/stegoeggo` namespace.
- Add regression tests for WebP legal-field round trip and conformance behavior.
- Update docs to reflect WebP support accurately.

Out of scope:

- New metadata parser dependencies.
- Full arbitrary XMP round-trip support.
- Changing PNG/JPEG behavior except shared helper updates.
- Breaking public API names.

## Files to Inspect

- `src/protected/metadata_trap.rs`
- `src/protected/notice_verification.rs`
- `scripts/verify_metadata_conformance.sh`
- `tests/`
- `README.md`
- `docs/legal_notice_model.md`
- `docs/architecture/protected-metadata-trap.md`
- `AGENTS.md`
- `CHANGELOG.md`

## Implementation Plan

### 1. Split XMP Generation Into DMI-Only and Notice-Aware Paths

Keep `generate_xmp_dmi()` only if still useful for compatibility, but introduce a richer helper such as:

```rust
fn generate_xmp_notice(
    dmi: Option<DmiValue>,
    seed: Option<u64>,
    legal: Option<&LegalMetadata>,
    fallback_metadata: &[(Vec<u8>, Vec<u8>)],
) -> Vec<u8>
```

or a smaller helper if passing `LegalMetadata` directly is easier from the call site.

The helper should emit:

- Existing IPTC DMI property:
  - `Iptc4xmpExt:DMI-Prohibited`
  - `Iptc4xmpExt:DMI-Allowed`
  - or whatever `DmiValue::to_iptc_property()` currently returns.
- Existing TDM reservation:
  - `tdm:reserve_tdm="1"` for restricted values.
  - `tdm:reserve_tdm="0"` for allowed values.
- Existing stegoeggo seed attribute if seed is present.
- Legal notice fields, preferably standard XMP namespaces:
  - `dc:rights` or `xmpRights:UsageTerms` for copyright/usage terms.
  - `dc:creator` for creator.
  - `xmpRights:WebStatement` for rights URL / web statement of rights.
  - `photoshop:Credit` or another conservative field only if already accepted by exiftool; avoid overfitting to one parser if the field is not standard.
  - `stegoeggo:AIConstraints` for AI constraints if no better standard field applies.
  - `stegoeggo:Contact` for contact if no better standard field applies.

Prefer standard XMP forms that `exiftool` reports clearly. Use simple RDF properties rather than complex bags unless needed.

### 2. Wire WebP Injection to the Notice-Aware XMP Helper

Change `inject_text_chunks_webp()` so it builds WebP XMP from legal metadata, not just DMI:

```rust
let xmp = Self::generate_xmp_notice(dmi, seed, legal, metadata);
let xmp_chunk = Self::create_webp_xmp_chunk(&xmp);
```

The current function signature only receives `metadata`, `dmi`, and `seed`. Options:

- Reconstruct legal fields from the metadata tuple list.
- Or change the call chain so `inject_text_chunks_webp()` receives `legal: Option<&LegalMetadata>`.

Prefer passing `LegalMetadata` if it avoids brittle tuple parsing. If that causes too much churn, write a small, well-tested tuple extraction helper for known keys:

```rust
fn metadata_value(metadata: &[(Vec<u8>, Vec<u8>)], key: &[u8]) -> Option<&str>
```

### 3. Keep PNG/JPEG Behavior Stable

If the XMP helper is shared with PNG/JPEG, ensure existing tests still pass. Do not regress:

- PNG iTXt XMP DMI/TDM emission.
- JPEG APP1 XMP DMI/TDM emission.
- JPEG XMP namespace header.
- Existing extraction behavior.

If broad sharing creates risk, use the notice-aware helper only for WebP in this pass, then refactor later.

### 4. Fix the Stale Namespace

Replace:

```xml
xmlns:stegoeggo="https://github.com/anomalyco/stegoeggo"
```

with:

```xml
xmlns:stegoeggo="https://github.com/eggstack/stegoeggo"
```

Search the repo for `anomalyco` and remove any remaining stale references unless they are historical notes that must remain.

### 5. Update WebP Notice Verification Parsing

Inspect `src/protected/notice_verification.rs` for WebP extraction. Ensure WebP XMP parsing can extract:

- Copyright holder.
- Creator.
- Contact.
- Rights URL / web statement.
- Usage terms.
- AI constraints.
- DMI.
- TDM reservation.

If parser support already exists for these fields but they were not emitted, add tests only. If parser support is missing, add lightweight extraction for the emitted XMP fields.

Keep parsing conservative. Do not mark arbitrary strings as legal notice.

### 6. Update External Conformance Script

In `scripts/verify_metadata_conformance.sh`:

- Change WebP copyright from warning to required PASS in strict mode once legal XMP is emitted.
- Keep non-strict mode friendly, but report clear failures when expected fields are absent.
- Validate at least:
  - Copyright holder.
  - DMI-Prohibited.
  - TDM reservation.
- Prefer also checking rights URL if exiftool exposes it reliably.
- Keep `vips` optional. `vips not available` should remain a skip unless a dedicated chunk-level verification requirement is added.

Expected strict WebP output after this pass:

```text
--- exiftool checks (webp) ---
  PASS: Copyright found in WebP XMP
  PASS: XMP contains DMI-Prohibited: ProhibitedAiMlTraining
  PASS: XMP contains TDM reservation
```

### 7. Add Tests

Add unit/integration tests for:

```text
webp_xmp_includes_copyright_holder
webp_xmp_includes_creator_and_rights_url
webp_xmp_includes_ai_constraints
webp_notice_verification_extracts_legal_fields
webp_notice_verification_reports_webp_xmp_channel
webp_notice_verification_dmi_tdm_still_present
webp_xmp_namespace_uses_eggstack_repo
webp_external_conformance_script_expectations_documented
```

If exact names differ, preserve the intent.

At minimum, generate a WebP output with:

- `--copyright-holder "Test Corp"`
- `--creator "Test Author"`
- `--rights-url "https://example.com/rights"`
- `--usage-terms "All rights reserved"`
- `--ai-constraints "No AI training"`
- `--no-ai-training`

Then verify the internal report returns those fields.

### 8. Documentation Updates

Update:

- README field visibility matrix: WebP copyright should be expected if exiftool can see it after the fix.
- `docs/legal_notice_model.md`: WebP now has legal-field XMP parity with caveats about parser support.
- `docs/architecture/protected-metadata-trap.md`: WebP XMP includes legal fields, DMI/TDM, and seed.
- `CHANGELOG.md`: add fixed entry for WebP legal metadata XMP conformance.
- `AGENTS.md`: add gotcha about WebP external parser checks and XMP legal fields.

## Validation Commands

Run the standard release gate:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --doc
cargo package --workspace --allow-dirty
cargo deny check licenses
cargo deny check advisories
```

Run targeted tests:

```bash
cargo test --all-features webp
cargo test --all-features notice_verification
cargo test --all-features metadata
```

Run conformance with exiftool installed:

```bash
cargo build -p stegoeggo-cli
./scripts/verify_metadata_conformance.sh --format webp --strict
./scripts/verify_metadata_conformance.sh --all-formats --strict
```

Manual inspection:

```bash
exiftool -a -G1 -s /path/to/test_input_protected.webp
```

Confirm visible output includes at least copyright/legal notice and DMI/TDM fields.

## Acceptance Criteria

- WebP protected outputs externally expose copyright/legal notice through XMP when inspected by exiftool.
- Existing WebP DMI/TDM external visibility remains intact.
- `verify_legal_notice()` extracts WebP legal fields.
- `EvidenceChannel::WebPXmp` is reported only when actual WebP XMP notice data is detected.
- The conformance script passes `--format webp --strict` with exiftool installed.
- `vips` remains optional and missing `vips` does not block release.
- Stale `anomalyco/stegoeggo` namespace is removed or replaced.
- PNG/JPEG conformance tests continue to pass.
- Full release gate passes.

## Risk Notes

External metadata field naming can vary by parser. Prefer standard XMP namespaces where possible, but do not chase every parser-specific alias.

Do not silently duplicate contradictory rights fields. If both tuple-derived metadata and `LegalMetadata` exist, use a deterministic precedence order and document it.

Avoid treating WebP as stronger than it is. WebP metadata can still be stripped by downstream transformations; the point is external notice visibility when metadata is preserved.

Keep this pass narrow. If broader XMP refactoring becomes attractive, document it as follow-up rather than mixing it into this corrective fix.
