# Plan 017: Release 2 — Legal Metadata Semantic Correctness

## Objective

Unify StegoEggo's legal metadata behavior across PNG, JPEG, and WebP and correct field mappings so every emitted property means what its governing metadata standard says it means.

This release builds on Plan 016. It assumes canonical PLUS Data Mining output and legacy-read compatibility already exist. Do not begin this release by reintroducing format-specific legal defaults or custom rights semantics.

## Problem statement

The current implementation can produce different legal metadata from the same caller input depending on output format. Legal metadata storage and legal-claim activation are independently configured, but WebP may consume the stored metadata directly while PNG and JPEG remain gated by a separate flag. The writer can synthesize generic ownership language and processing dates into fields whose semantics are more specific. Contact information is mapped to a credit-line property, and processing time may be represented as image creation time.

These behaviors undermine the core promise of a reliable rights notice. The same normalized caller intent must produce semantically equivalent metadata in every supported container.

## Scope

### In scope

- One normalized rights-notice model shared by all format writers.
- A single, explicit legal-claim activation contract.
- Correct copyright, creator, contact, licensor, credit, rights URL, usage terms, content-creation date, metadata date, and notice-application timestamp semantics.
- Equivalent output and verification across PNG, JPEG, and WebP.
- Deterministic merge behavior for existing metadata.
- Conflict reporting and preservation of unrelated metadata.
- CLI and library validation for contradictory configurations.
- Cross-format semantic-equivalence tests.

### Out of scope

- Evidence-profile architecture redesign from Release 4.
- New hidden-marker algorithms.
- Public-key signatures.
- Full generic XMP editing for arbitrary external schemas beyond preservation requirements.
- Legal advice or jurisdiction-specific conclusions.

## Affected areas

Expected primary areas:

- `src/types.rs`
- `src/protected/metadata_trap.rs`
- `src/protected/notice_verification.rs`
- `src/lib.rs`
- `stegoeggo-cli/src/main.rs`
- format-specific tests and fixtures
- `README.md`
- `docs/legal_notice_model.md`
- architecture documentation
- `CHANGELOG.md`

Confirm the current repository layout before editing.

## Target behavioral contract

Adopt one explicit contract for legal metadata:

```text
No legal metadata supplied:
    emit no ownership-specific claims

Legal metadata supplied:
    emit exactly the supplied claims and standard-required structure

Explicit rights preset selected:
    emit the preset's documented language and policy

Contradictory or incomplete configuration:
    fail validation or return a structured warning according to documented policy
```

Do not silently synthesize `All Rights Reserved`, a creator identity, a copyright owner, or a content creation date unless the caller explicitly selected a preset that documents this behavior.

## Proposed normalized model

Create or evolve a single format-independent model. Names may change, but the semantics must remain explicit.

```rust
pub struct RightsNotice {
    pub copyright_notice: Option<String>,
    pub creators: Vec<Party>,
    pub copyright_owners: Vec<Party>,
    pub licensors: Vec<Licensor>,
    pub required_credit_line: Option<String>,
    pub usage_terms: Option<LocalizedText>,
    pub web_statement: Option<String>,
    pub content_creation_date: Option<String>,
    pub metadata_date: Option<String>,
    pub notice_applied_at: Option<String>,
    pub ai_constraints: Option<String>,
    pub data_mining_policy: RightsPolicyOrExistingDmiValue,
}

pub struct Party {
    pub name: String,
}

pub struct Licensor {
    pub name: Option<String>,
    pub email: Option<String>,
    pub url: Option<String>,
}
```

A minimal first implementation may preserve the existing `LegalMetadata` public type and convert it into an internal normalized model. Avoid an unnecessary breaking change if semantic correction can be achieved through internal normalization and additive builder methods.

## Execution phases

## Phase 1: Audit and specify field semantics

Before editing writers, produce a mapping table in code comments or architecture documentation covering every supported field:

| Caller concept | Canonical metadata target | Notes |
|---|---|---|
| Copyright notice | `dc:rights` and/or standard copyright notice field | Free-text notice, not necessarily owner identity |
| Creator | `dc:creator` sequence | One or more creators |
| Copyright owner | PLUS structured owner property | Distinct from creator and notice string |
| Licensor contact | PLUS structured licensor record | Name/email/URL |
| Required credit line | `photoshop:Credit` | Only actual publication credit text |
| Usage terms | `xmpRights:UsageTerms` | Prefer localized text container |
| Rights URL | `xmpRights:WebStatement` | URL to authoritative rights statement |
| Content creation date | `photoshop:DateCreated` or applicable standard field | Only actual content creation date |
| Metadata modification date | `xmp:MetadataDate` | Processing/update timestamp |
| Notice application time | project namespace or documented standard mapping | Must not masquerade as content creation time |
| Data mining policy | canonical `plus:DataMining` | From Release 1 |
| Human AI constraints | project namespace or rights terms | Supplementary text, not replacement for canonical policy |

Verify exact structures and namespaces against current standards before implementation.

### Acceptance criteria

- Every public legal metadata field has one documented semantic meaning.
- No field is mapped to a standard property with incompatible semantics.
- Owner, creator, licensor, credit line, and copyright notice are distinct concepts.

## Phase 2: Normalize configuration before format encoding

1. Add a normalization function that converts `ProtectionContext`, existing legal metadata, rights policy, and explicit presets into one validated `RightsNotice`.
2. Run normalization once before selecting a format writer.
3. Remove format-specific default synthesis from PNG, JPEG, and WebP paths.
4. Ensure all writers receive the same normalized structure.
5. Preserve the distinction between absent fields and explicit empty values. Prefer rejecting empty required strings rather than emitting empty properties.
6. Validate maximum field sizes after normalization and before encoding.

Suggested internal flow:

```text
ProtectionContext
    -> validate configuration
    -> normalize legal intent
    -> RightsNotice
    -> encode/merge into selected container
```

### Acceptance criteria

- Format writers no longer inspect independent legal-claim flags differently.
- Equivalent configuration produces the same normalized notice before encoding.
- Validation failures occur before partial output mutation.

## Phase 3: Unify legal-claim activation

Choose and implement one of these contracts:

### Preferred contract

Supplying nonempty `LegalMetadata` automatically requests emission of those exact fields. The existing `with_legal_claims(true)` method becomes deprecated or a compatibility no-op with warnings for contradictory use.

### Acceptable compatibility contract

Retain an explicit enable flag, but enforce it uniformly:

- Metadata supplied plus enable false: return a configuration error.
- Metadata supplied plus enable absent: either enable consistently or return a clear error.
- Enable true with no metadata: require an explicit preset or return an error.

The CLI must mirror the selected library contract. Individual flags such as `--copyright-holder`, `--creator`, `--contact`, and `--rights-url` must behave identically across formats.

### Acceptance criteria

- The same command produces equivalent legal fields in PNG, JPEG, and WebP.
- No format bypasses the legal-claim policy.
- CLI help and examples match actual behavior.
- Contradictory configurations are rejected explicitly.

## Phase 4: Correct owner, creator, contact, and credit fields

1. Stop writing contact email or URL to `photoshop:Credit`.
2. Add a dedicated required-credit-line field if credit-line support is retained.
3. Encode creators as a sequence, not a single ambiguous scalar where the standard expects an array.
4. Add structured copyright-owner support.
5. Add structured licensor support with name, email, and URL.
6. Keep compatibility accessors for the current single creator/contact model where practical.
7. Update verification to extract normalized owner, creator, licensor, contact, and credit-line concepts separately.

### Acceptance criteria

- `photoshop:Credit` is emitted only when a credit line is explicitly provided.
- Contact information remains discoverable through semantically appropriate properties.
- Creator and owner values do not overwrite one another.
- Verification reports each concept separately.

## Phase 5: Correct date semantics

1. Stop populating content-creation date from the current processing time.
2. Preserve caller-supplied content creation date unchanged after validation.
3. Add metadata modification time using an appropriate XMP field.
4. Add an explicit notice-application timestamp if useful for provenance and auditing.
5. Use RFC 3339 or the exact date/time format expected by the target metadata property.
6. Add deterministic clock injection for tests rather than depending directly on wall-clock time.
7. Document whether timestamps are UTC and whether subsecond precision is retained.

Suggested internal clock abstraction:

```rust
trait Clock {
    fn now_utc(&self) -> Timestamp;
}
```

A simpler injectable timestamp parameter is acceptable if it avoids excessive abstraction.

### Acceptance criteria

- Processing an image without a supplied content-creation date does not invent one.
- Metadata/notice timestamps are distinguishable from content creation.
- Tests are deterministic.

## Phase 6: Correct rights URL and usage terms structure

1. Encode rights URL as `xmpRights:WebStatement` or the exact applicable property.
2. Encode usage terms using the expected localized text structure.
3. Validate URL syntax without requiring network access.
4. Preserve non-ASCII text and language tags.
5. Define a default language behavior, such as `x-default`, and document it.
6. Ensure XML escaping and Unicode normalization do not alter legal text unexpectedly.

### Acceptance criteria

- ExifTool and a namespace-aware parser expose rights URL and usage terms for all formats.
- Invalid URLs are rejected before writing.
- Unicode and XML-sensitive characters round-trip.

## Phase 7: Define metadata merge behavior

StegoEggo must not blindly append contradictory duplicate blocks on repeated processing.

Define field-level policies:

- Preserve unrelated namespaces and properties.
- Replace StegoEggo-owned diagnostic properties deterministically.
- For canonical rights properties, choose one documented mode:
  - preserve existing and report conflict,
  - replace only when explicitly requested,
  - or merge structured arrays where semantically valid.
- Do not silently discard camera EXIF, orientation, color profiles, creator records, or unrelated XMP.
- Preserve unknown XMP where container rewriting permits it.

Add an explicit update policy if necessary:

```rust
pub enum MetadataUpdatePolicy {
    FailOnConflict,
    PreserveExisting,
    ReplaceSpecifiedFields,
}
```

A private internal policy is acceptable for this release if public exposure would destabilize the API, but behavior must still be deterministic and documented.

### Acceptance criteria

- Applying the same notice twice is idempotent or yields a deterministic normalized result.
- Unrelated metadata survives.
- Conflicts are not silently hidden.
- Repeated processing does not grow duplicate XMP indefinitely.

## Phase 8: Cross-format semantic-equivalence matrix

Build table-driven integration tests that apply one normalized notice to PNG, JPEG, and WebP, then inspect output through both internal verification and external tools.

Required scenarios:

1. Copyright notice only.
2. Creator only.
3. Copyright owner only.
4. Licensor with email and URL.
5. Required credit line.
6. Usage terms with non-ASCII text.
7. Rights URL.
8. Content creation date.
9. Canonical Data Mining policy.
10. Complete notice with all fields.
11. Empty/default metadata.
12. Contradictory configuration.
13. Existing unrelated metadata.
14. Existing conflicting rights metadata.
15. Reapplication/idempotence.

Normalize external parser aliases before comparison. The semantic result, not byte-for-byte XMP identity, is the assertion target.

### Acceptance criteria

- All supported formats produce equivalent normalized reports.
- No legal field appears in only one format unless a documented container limitation requires it.
- Container limitations are reported as warnings and tested.

## Phase 9: Verification model update

Extend `NoticeVerification` or its successor to expose:

- Copyright notice.
- Creators.
- Copyright owners.
- Licensors/contact endpoints.
- Required credit line.
- Usage terms.
- Rights URL.
- Content creation date.
- Metadata date.
- Notice application timestamp.
- Canonical data-mining policy and source.
- Conflicts and malformed fields.

Avoid preserving an API shape that conflates copyright holder, notice text, and creator identity.

If compatibility methods remain, document exactly how they select among normalized values.

### Acceptance criteria

- Verification can distinguish all newly separated concepts.
- Compatibility accessors remain deterministic.
- CLI output does not label a credit line as contact information or a creator as copyright owner.

## Phase 10: CLI and documentation migration

CLI changes should include:

- Clear distinction between copyright notice, copyright owner, creator, licensor/contact, and credit line.
- Explicit date semantics.
- Validation errors for contradictory flags.
- Updated examples that produce the same output across formats.
- Migration guidance for deprecated flags or methods.

Potential additive flags:

```text
--copyright-notice
--copyright-owner
--creator
--licensor-name
--licensor-email
--licensor-url
--credit-line
--rights-url
--usage-terms
--content-created-at
```

Preserve existing flags as aliases for one deprecation cycle where their meaning can be mapped safely. Do not preserve an alias if it would continue writing semantically incorrect metadata.

## Required tests

### Unit tests

- Normalization from every current builder/CLI field.
- Empty and oversized fields.
- URL validation.
- Date validation and deterministic clock behavior.
- XML escaping and Unicode.
- Owner/creator/licensor/credit separation.

### Integration tests

- Cross-format matrix listed above.
- Existing metadata preservation.
- Conflict behavior.
- Idempotent repeated processing.
- Canonical DMI from Plan 016 remains intact.
- Legacy files remain readable.

### External tests

- ExifTool extraction of all supported standard fields.
- Namespace-aware XML validation.
- ImageMagick and libvips decode/preservation smoke tests.

## Testing commands

Run at minimum:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --no-fail-fast
cargo test --doc
cargo package --workspace --allow-dirty
./scripts/verify_metadata_conformance.sh --all-formats --strict
```

Release 2 must preserve the Release 1 external conformance gate.

## Risks and mitigations

### Risk: Public API expansion becomes excessive

Mitigation: normalize internally first, add only high-value structured types, and stage deprecations.

### Risk: Existing users relied on implicit generic claims

Mitigation: provide explicit presets and migration documentation; do not silently retain ambiguous defaults.

### Risk: Metadata merge logic corrupts unrelated XMP

Mitigation: add golden fixtures, preserve unknown properties, and test repeated updates.

### Risk: Format limitations prevent exact structural parity

Mitigation: compare normalized semantics and document container-specific carriers without changing meaning.

### Risk: Contact and ownership records become too complex

Mitigation: support a minimal structured subset first while preserving extensibility.

## Completion gate

Release 2 is complete only when:

- One normalized legal-notice model drives PNG, JPEG, and WebP writers.
- Equivalent caller configuration produces semantically equivalent external metadata across formats.
- Legal metadata activation is consistent and contradictory configurations fail clearly.
- Contact, credit line, creator, owner, and licensor concepts are no longer conflated.
- Processing time is not written as content creation time.
- Rights URL and usage terms use correct structures.
- Unrelated metadata is preserved and repeated application is deterministic.
- Verification exposes normalized fields and conflicts.
- Release 1 canonical Data Mining conformance remains green.
- Documentation and CLI examples match actual behavior.

## Handoff notes

Do not treat this as a cosmetic rename pass. The main deliverable is semantic equivalence across formats and independently observable correctness. Every field should be traced from caller input through normalization, format encoding, external extraction, and `NoticeVerification` output. Where standards permit several encodings, choose the simplest externally interoperable representation and document the decision.
