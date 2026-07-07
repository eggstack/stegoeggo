# Plan 014: WebP XMP RDF Structure Patch

## Goal

Fix the remaining WebP external conformance failure where `exiftool` sees WebP DMI/TDM fields but does not see copyright/rights fields. The current WebP XMP implementation appears to emit the legal child elements in the wrong XML location: inside the `<rdf:Description ...>` opening tag rather than after the start tag has closed.

This pass should produce parser-friendly XMP that `exiftool` can read in strict conformance mode.

## Observed Failure

After the WebP legal-XMP corrective pass, internal tests pass:

```text
ok. 12 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 1.72s.
```

But external WebP conformance still fails:

```text
=== Metadata Conformance Check: .../test_input_protected.webp ===

--- exiftool checks (webp) ---
  FAIL: Copyright not found in WebP XMP
  PASS: XMP contains DMI-Prohibited: ProhibitedAiMlTraining
  PASS: XMP contains TDM reservation
  WARN: No UsageTerms or Rights found in WebP XMP (optional)

--- Format-specific checks (webp) ---
  SKIP: vips not available, skipping WebP chunk verification
```

This indicates DMI/TDM attributes are parser-visible, while legal child elements are not.

## Likely Root Cause

Current `generate_xmp_notice()` builds `legal_props` as XML child elements such as:

```xml
<dc:creator>...</dc:creator>
<xmpRights:UsageTerms>...</xmpRights:UsageTerms>
<dc:rights>...</dc:rights>
```

But interpolation currently places `{legal_props}` inside the `<rdf:Description ...>` start tag before the closing `>`.

Conceptually, the emitted structure resembles:

```xml
<rdf:Description rdf:about=""
  Iptc4xmpExt:DMI-Prohibited="ProhibitedAiMlTraining"
  tdm:reserve_tdm="1"
  stegoeggo:ProtectionSeed="..."
  <dc:creator>...</dc:creator>
  <dc:rights>...</dc:rights>>
</rdf:Description>
```

That is malformed or at least parser-hostile RDF/XML. `exiftool` can still recover the attributes before the malformed child element area, which explains why DMI/TDM pass while copyright/rights do not.

## Scope

In scope:

- Move legal XMP child elements outside the `<rdf:Description>` opening tag.
- Use parser-friendly RDF containers for `dc:rights` and `xmpRights:UsageTerms`.
- Keep DMI/TDM/seed as attributes unless there is a reason to move them.
- Add tests that catch this specific structural error.
- Update the conformance script only if tag aliases need adjustment after the XML structure is fixed.
- Update docs/changelog if behavior changes materially.

Out of scope:

- Rewriting the full metadata stack.
- Adding an XML parser dependency.
- Changing PNG/JPEG behavior unless shared helper tests reveal a regression.
- Requiring `vips` for release conformance.

## Files to Inspect

- `src/protected/metadata_trap.rs`
- `src/protected/notice_verification.rs`
- `scripts/verify_metadata_conformance.sh`
- WebP-related tests under `tests/`
- `README.md`
- `docs/legal_notice_model.md`
- `docs/architecture/protected-metadata-trap.md`
- `CHANGELOG.md`
- `AGENTS.md`

## Required Code Change

Update `generate_xmp_notice()` so the `<rdf:Description>` opening element is closed before `legal_props` is inserted.

Target structure:

```xml
<rdf:Description rdf:about=""
  Iptc4xmpExt:DMI-Prohibited="ProhibitedAiMlTraining"
  tdm:reserve_tdm="1"
  stegoeggo:ProtectionSeed="...">
  ...legal child elements...
</rdf:Description>
```

Do not emit child elements before the start tag closes.

## Recommended Legal Field Encoding

Prefer parser-friendly XMP/RDF forms:

```xml
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
<photoshop:Credit>legal@test.com</photoshop:Credit>
<stegoeggo:AIConstraints>No AI training</stegoeggo:AIConstraints>
```

If `exiftool` exposes a different tag name for these fields, adjust `scripts/verify_metadata_conformance.sh` to check the parser-visible alias. Do not weaken strict mode until the actual emitted XML has been validated.

## Tests to Add or Tighten

Add tests that validate XML structure, not just substring presence.

Suggested tests:

```text
webp_xmp_legal_children_after_description_start_tag
webp_xmp_rdf_description_is_well_ordered
webp_xmp_dc_rights_uses_rdf_alt
webp_xmp_usage_terms_uses_rdf_alt
webp_xmp_exiftool_tag_shape_regression
webp_notice_verification_extracts_fields_after_rdf_fix
```

At minimum, assert:

1. `<rdf:Description` appears before `tdm:reserve_tdm`.
2. The opening `>` for `<rdf:Description ...>` appears before `<dc:creator>`, `<dc:rights>`, and `<xmpRights:UsageTerms>`.
3. `</rdf:Description>` appears after those legal child elements.
4. `dc:rights` and `xmpRights:UsageTerms` use valid child content rather than malformed attributes.

Pseudo-check:

```rust
let desc = xmp.find("<rdf:Description").unwrap();
let desc_close = xmp[desc..].find('>').unwrap() + desc;
let rights = xmp.find("<dc:rights>").unwrap();
assert!(desc_close < rights);
```

## Conformance Script Check

After patching XML structure, run:

```bash
./scripts/verify_metadata_conformance.sh --format webp --strict
```

If copyright still fails, inspect exact parser output:

```bash
exiftool -a -G1 -s /path/to/test_input_protected.webp
exiftool -a -G1 -s -XMP:all /path/to/test_input_protected.webp
```

Then adjust the script to check actual tags. Likely candidates:

```bash
exiftool -s3 -XMP-dc:Rights image.webp
exiftool -s3 -XMP-xmpRights:UsageTerms image.webp
exiftool -s3 -XMP-xmpRights:WebStatement image.webp
exiftool -s3 -XMP-photoshop:Credit image.webp
```

Avoid checking only `-Copyright` for WebP if the emitted canonical XMP rights field is `dc:Rights`. It is acceptable for the script to report `Rights found` instead of `Copyright found`, as long as the expected copyright holder text is visible through a standard XMP tag.

## Validation Commands

Targeted validation:

```bash
cargo test --all-features webp
cargo test --all-features xmp
cargo test --all-features notice_verification
cargo build -p stegoeggo-cli
./scripts/verify_metadata_conformance.sh --format webp --strict
```

Full release gate after the patch:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --doc
cargo package --workspace --allow-dirty
cargo deny check licenses
cargo deny check advisories
./scripts/verify_metadata_conformance.sh --all-formats --strict
```

## Acceptance Criteria

- `exiftool` can see the expected copyright holder or rights text in WebP XMP.
- `./scripts/verify_metadata_conformance.sh --format webp --strict` passes on a machine with `exiftool`.
- DMI-Prohibited and TDM reservation remain visible in WebP XMP.
- Internal `verify_legal_notice()` still extracts WebP legal fields.
- Tests catch legal child elements being placed inside the `rdf:Description` opening tag.
- `vips` remains optional and missing `vips` is not a release blocker.
- Full cargo test/clippy/fmt/package/deny gate remains clean.

## Risk Notes

Do not loosen strict mode to hide the failure. The emitted XMP shape should be corrected first.

Do not overfit to `-Copyright` if the correct XMP field is `dc:Rights`; instead, require that the expected copyright holder text is externally visible in a standard rights tag.

Keep the patch narrow. The issue is XML/RDF structure, not the overall legal-notice architecture.
