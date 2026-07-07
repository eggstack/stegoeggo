# Plan 002: CLI Legal Metadata Fields

## Goal

Make the CLI support the actual legal-notice workflow directly. A user should be able to embed meaningful copyright, rights URL, contact, and AI-training restriction metadata without writing Rust code or accepting generic placeholder legal claims.

The current CLI has a coarse `--legal-claims` flag. That is not enough for the stated purpose of the project. Legal deterrence depends on clear notice. Clear notice should include who owns the work, where rights terms are stated, what AI/ML uses are restricted, and how a recipient can identify the claim.

## Scope

This plan covers CLI argument surface, mapping into `LegalMetadata`, default DMI/TDM behavior, CLI examples, CLI tests, and help output. It should not redesign the core verification model; that belongs in Plan 005. It should not add evidence profiles unless Plan 003 is being executed at the same time.

## Files to Inspect

- `stegoeggo-cli/src/main.rs`
- `src/types.rs`
- `src/protected/metadata_trap.rs`
- `README.md`
- `examples/legal_metadata.rs`
- CLI tests, if any exist
- Integration tests that run the CLI, if any exist

## Proposed CLI Flags

Add the following options:

```text
--copyright-holder <NAME>
--creator <NAME>
--contact <EMAIL_OR_URL>
--rights-url <URL>
--usage-terms <TEXT>
--ai-constraints <TEXT>
--no-ai-training
--no-genai-training
--tdm-reserved
```

Retain existing flags:

```text
--legal-claims
--dmi <DMI>
--metadata <BOOL>
--key <HEX>
```

Reword `--legal-claims` as a compatibility convenience. It should enable legal metadata injection, but new examples should prefer explicit fields.

## Desired User Workflows

Primary workflow:

```bash
stegoeggo artwork.png -o artwork_protected.png \
  --copyright-holder "Jane Artist" \
  --rights-url "https://example.com/rights" \
  --no-genai-training
```

More explicit workflow:

```bash
stegoeggo artwork.jpg -o artwork_protected.jpg \
  --format jpg \
  --copyright-holder "Jane Artist" \
  --creator "Jane Artist" \
  --contact "legal@example.com" \
  --rights-url "https://example.com/rights/artwork-123" \
  --usage-terms "Copyrighted work. All rights reserved. No AI training permitted." \
  --ai-constraints "No AI/ML training, no generative AI training, no model fine-tuning." \
  --dmi prohibited-genai
```

Advanced provenance workflow should remain available but not primary:

```bash
stegoeggo artwork.png -o artwork_protected.png \
  --copyright-holder "Jane Artist" \
  --rights-url "https://example.com/rights" \
  --no-genai-training \
  --key deadbeefcafebabe
```

## Mapping Rules

### LegalMetadata Mapping

Map flags into `LegalMetadata` as follows:

- `--copyright-holder` -> `LegalMetadata::with_copyright_holder`
- `--contact` -> existing contact field, or add a URL-capable alias if needed
- `--rights-url` -> `with_web_statement_of_rights`; also consider `with_license_url` only if semantically appropriate
- `--usage-terms` -> `with_usage_terms`
- `--ai-constraints` -> `with_ai_constraints`
- `--creator` -> add a new field to `LegalMetadata` if it does not exist, or map to copyright holder only if no better field exists

If `--creator` requires a new `LegalMetadata` field, implement:

```rust
creator: Option<String>
with_creator(...)
creator(&self) -> Option<&str>
```

Then update metadata injection to emit a creator/artist field for each supported container where practical.

### Preset Flags

`--no-ai-training` should imply an AI/ML training prohibition. It may set:

- `DmiValue::ProhibitedAiMlTraining`
- `ai_constraints = "No AI/ML training permitted."` if not explicitly provided
- usage terms default mentioning no AI/ML training if not explicitly provided

`--no-genai-training` should imply a generative-AI training prohibition. It may set:

- `DmiValue::ProhibitedGenAiMlTraining`
- `ai_constraints = "No generative AI training permitted."` if not explicitly provided
- usage terms default mentioning no generative AI training if not explicitly provided

If both are supplied, choose the stricter or more explicit value. Prefer `ProhibitedGenAiMlTraining` only if the intent is generative-only. If both general and generative restrictions are present, `ProhibitedAiMlTraining` may be the broader DMI value, with `ai_constraints` spelling out both.

### Legal Claims Enablement

If any explicit legal metadata flag is provided, automatically enable legal claim injection. Users should not have to pass both `--copyright-holder` and `--legal-claims`.

Rules:

- If explicit legal fields exist, call `.with_legal_metadata(...)` and `.with_legal_claims(true)`.
- If only `--legal-claims` is supplied, retain current generic legal behavior.
- If `--metadata false` is supplied, legal claims should either still be injected if explicitly requested or the CLI should produce a clear conflict error. Prefer explicit conflict handling over silent omission.

Suggested conflict behavior:

```text
Error: legal metadata flags require metadata injection. Remove --metadata false or omit legal metadata flags.
```

## Validation Rules

Keep validation lightweight and non-invasive:

- Empty strings should be rejected.
- Rights URL should be non-empty and preferably start with `http://` or `https://`. If strict URL parsing is not desired, warn rather than fail.
- Contact may be email or URL; do not over-validate.
- Copyright holder should be non-empty.

Do not add a heavy URL parsing dependency unless already present or justified.

## Help Text Requirements

CLI help should group legal-notice flags before stego/provenance flags if clap supports headings with minimal churn. If not, at least make the flag descriptions clear.

Suggested wording:

```text
--copyright-holder <NAME>
    Copyright holder to embed in rights-reservation metadata.

--rights-url <URL>
    Web statement of rights or licensing terms to embed in metadata.

--no-genai-training
    Add metadata indicating that generative AI training is not permitted.

--key <HEX>
    Optional authenticated steganographic provenance key. Not required for legal-notice mode.
```

## Implementation Steps

1. Add CLI args to `Args` in `stegoeggo-cli/src/main.rs`.
2. Add any missing fields to `LegalMetadata` in `src/types.rs`, especially creator/artist if needed.
3. Update metadata injection to emit the new fields in PNG/JPEG/WebP metadata.
4. Add helper function in CLI to build `LegalMetadata` from args.
5. Automatically enable legal claims when explicit legal metadata flags are present.
6. Resolve DMI presets from `--no-ai-training`, `--no-genai-training`, and `--dmi`.
7. Add conflict handling for `--metadata false` plus explicit legal flags.
8. Update README examples.
9. Add tests.

## Suggested Test Cases

Add unit tests for argument-to-context mapping if the CLI code can be factored into helpers. If not, add integration tests invoking the binary where practical.

Required cases:

1. `--copyright-holder` and `--rights-url` create legal metadata and enable legal claims.
2. `--no-genai-training` maps to `DmiValue::ProhibitedGenAiMlTraining` unless overridden.
3. `--no-ai-training` maps to a general AI/ML prohibition.
4. Explicit `--dmi allowed` overrides presets only if this is an intentional supported behavior; otherwise reject contradictory flags.
5. `--metadata false --copyright-holder X` errors clearly.
6. `--key` remains optional and does not affect legal metadata generation.
7. Generated PNG contains at least the expected text chunks or extractable legal fields.
8. Generated JPEG contains expected COM/XMP metadata.

## Test Commands

Run:

```bash
cargo fmt --check
cargo test --all-features
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo run -p stegoeggo-cli -- --help
```

Manual smoke test:

```bash
cargo run -p stegoeggo-cli -- examples/input.png -o /tmp/protected.png \
  --copyright-holder "Test Artist" \
  --rights-url "https://example.com/rights" \
  --no-genai-training \
  --verbose
```

Then verify with the library CLI and, if available locally, `exiftool`.

## Acceptance Criteria

- CLI can embed non-generic copyright holder and rights URL metadata.
- Legal metadata flags automatically enable legal claim injection.
- DMI/no-training presets are available without requiring users to know internal enum names.
- `--key` is clearly optional authenticated provenance.
- Contradictory metadata/legal flags fail clearly or warn explicitly.
- README quick-start uses explicit legal metadata fields.
- Tests cover CLI mapping and at least one generated image metadata result.

## Risk Notes

Be careful not to over-promise legal meaning. Field names and help text should say that metadata is embedded; they should not claim that this guarantees enforceability.

Be careful with `--dmi allowed` plus `--no-ai-training`. Either define override precedence explicitly or reject the contradiction. Silent contradictory metadata would be worse than an error.

Avoid adding large dependencies solely for URL/email validation unless the dependency impact is acceptable.
