# Plan 015: Standards-Correctness and Interoperability Roadmap

## Status

Proposed execution roadmap for the next major line of work after v0.2.1.

## Purpose

StegoEggo is intended to create an explicit, independently discoverable record that a content owner reserved specified uses of an image at distribution time. The primary mechanism is visible, machine-readable rights metadata. Optional steganographic markers provide redundant evidence but are not forensic watermarks, DRM, proof of model training, or a substitute for standards-conformant rights signaling.

The current implementation has a sound high-level threat model and substantial format, verification, robustness, and release infrastructure. However, its primary rights signals are encoded using project-defined XMP properties that are not the canonical IPTC/PLUS Data Mining representation, and its TDM reservation property does not match the current W3C TDM Reservation Protocol. Additional inconsistencies exist in legal-claim activation, field semantics, evidence-profile behavior, and external conformance gating.

This roadmap corrects those issues in dependency order. Standards correctness and independent interoperability are the critical path. New steganographic robustness and performance work should remain secondary until Releases 1 through 3 are complete.

## Guiding principles

1. Rights policy, metadata encoding, processing intensity, and provenance must be separate concepts.
2. Externally recognized metadata is the primary product behavior.
3. Internal round-trip tests do not establish interoperability.
4. Legacy StegoEggo files must remain readable during migration.
5. The default library behavior must not silently invent ownership or rights claims.
6. Verification must report ambiguity and conflicts rather than selecting the first matching property.
7. Hidden markers must be described and scored according to their actual authentication and transformation-survival properties.
8. Format-specific encoders must consume one normalized rights-notice model.
9. Release gates must fail when external conformance cannot be demonstrated.
10. Documentation must distinguish technical notice, authenticated provenance, legal effect, and proof of infringement.

## Release sequence

### Release 1: Canonical rights metadata

Replace the current self-defined DMI and TDM encodings with canonical representations while retaining legacy parsing.

Primary outcomes:

- Emit `plus:DataMining` using official PLUS controlled-vocabulary URIs.
- Stop emitting `Iptc4xmpExt:DMI-Prohibited`, `Iptc4xmpExt:DMI-Allowed`, and `Iptc4xmpExt:DMI` by default.
- Correct TDM reservation handling and distinguish image-embedded rights metadata from web-distribution TDMRep artifacts.
- Parse canonical and legacy forms.
- Report canonical, legacy, unknown, malformed, and conflicting signals separately.
- Add independently authored fixtures and exact mapping tests.

Detailed execution plan: `plans/016-release-1-canonical-rights-metadata.md`.

### Release 2: Legal metadata semantic correctness

Unify legal-claim behavior across formats and correct field semantics.

Primary outcomes:

- Introduce one normalized rights-notice model.
- Make supplied legal metadata activate consistently across PNG, JPEG, and WebP.
- Remove implicit ownership-language synthesis unless an explicit preset is selected.
- Correct contact, credit-line, creation-date, metadata-date, and rights-owner mappings.
- Add structured licensor and copyright-owner support.
- Define deterministic merge and conflict behavior for pre-existing metadata.
- Add cross-format semantic-equivalence tests.

Detailed execution plan: `plans/017-release-2-legal-metadata-semantics.md`.

### Release 3: Independent interoperability and conformance

Make external interoperability a mandatory correctness and release criterion.

Primary outcomes:

- Add a mandatory external-conformance CI job.
- Move normalized conformance assertions from shell/grep logic into a structured Rust harness.
- Add externally authored golden fixtures.
- Validate XMP namespaces, values, RDF structure, duplicate declarations, and conflict behavior.
- Test metadata preservation and idempotent updates.
- Produce machine-readable conformance reports.

Detailed execution plan: `plans/018-release-3-independent-conformance.md`.

### Release 4: Policy and processing architecture separation

Separate legal meaning from processing cost and evidence-channel selection.

Primary outcomes:

- Replace implicit level-derived DMI defaults with an explicit `RightsPolicy`.
- Introduce explicit `ProtectionChannels` or equivalent configuration.
- Make evidence profiles configure actual processing rather than warning interpretation alone.
- Add a true metadata-only fast path as the default legal-notice path.
- Retain `ProtectionLevel` as a compatibility preset for one deprecation cycle.
- Centralize configuration validation.

### Release 5: Provenance and evidence hardening

Strengthen optional provenance without overstating evidentiary effect.

Primary outcomes:

- Define a stable, versioned hidden-payload wire format.
- Increase HMAC tag length.
- Add optional Ed25519 public-key signatures and key identifiers.
- Distinguish structural validity, checksum validity, authentication, trust, and ownership claims.
- Add detached signed manifests.
- Evaluate optional C2PA integration through a design decision record.
- Rename nonstandard ISCC-like public APIs.

### Release 6: Production and release closure

Close packaging, API, security, and release-process gaps.

Primary outcomes:

- Separate release validation from publication.
- Make all required gates blocking.
- Verify packaged documentation and crate contents.
- Complete public API deprecations and naming cleanup.
- Expand fuzzing to all container parsers, XMP parsing, merging, and repeated updates.
- Add explicit resource budgets for metadata, XML, chunks, verification work, and tiled extraction.
- Restructure documentation around rights metadata, web deployment, and optional provenance.

## Cross-release architecture target

The target processing model is:

```text
Caller policy
    -> validated RightsNotice
    -> selected ProtectionChannels
    -> format-independent normalized metadata graph
    -> format encoder/merger
    -> optional hidden provenance marker
    -> external and internal verification report
```

The legal policy must not be inferred from the selected processing level. The format encoder must not invent legal defaults. Verification must preserve source attribution for every extracted field and signal.

Suggested conceptual types:

```rust
pub enum RightsPolicy {
    Unspecified,
    Allowed,
    ProhibitedAiMlTraining,
    ProhibitedGenerativeAiTraining,
    ProhibitedExceptSearchIndexing,
    ProhibitedAllDataMining,
    ProhibitedSeeConstraints,
}

pub struct RightsNotice {
    pub copyright_notice: Option<String>,
    pub creators: Vec<String>,
    pub usage_terms: Option<LocalizedText>,
    pub web_statement: Option<String>,
    pub creation_date: Option<String>,
    pub notice_applied_at: Option<String>,
    pub data_mining_policy: RightsPolicy,
    pub copyright_owners: Vec<Party>,
    pub licensors: Vec<Licensor>,
}

pub struct ProtectionChannels {
    pub rights_metadata: bool,
    pub hidden_marker: HiddenMarkerMode,
    pub authentication: AuthenticationMode,
}
```

These names are illustrative. Detailed design should preserve semver compatibility where practical and use staged deprecation rather than gratuitous breakage.

## Compatibility and migration policy

### Read compatibility

The verifier must continue to read v0.2-era metadata and payloads. Legacy properties should be labeled as legacy rather than silently normalized as canonical.

### Write compatibility

New releases should emit canonical properties only by default. A temporary explicit compatibility option may dual-write legacy properties, but it must be opt-in, documented as noncanonical, and removed after a defined migration window.

### API compatibility

Public types and methods that encode incorrect semantics should be deprecated with replacements and migration examples. Breaking removals should be deferred to the next major version unless retaining the API would continue producing incorrect output.

### Fixture compatibility

Legacy fixtures must remain in the test suite. Every format should have canonical, legacy, malformed, conflicting, and independently authored fixtures.

## Verification model target

Verification results should distinguish:

- No notice found.
- Canonical metadata notice found.
- Legacy metadata notice found.
- Canonical and legacy notices agree.
- Conflicting rights signals found.
- Best-effort stego verified by checksum.
- HMAC-authenticated payload verified.
- Public-key signature valid but key untrusted.
- Public-key signature valid and key trusted.
- Payload malformed or authentication invalid.

Evidence strength must never promote a legacy or malformed signal to canonical merely because the project parser recognizes it.

## Testing strategy

Every release must add tests at four levels:

1. Unit tests for mappings, parsers, serializers, and validation.
2. Cross-format integration tests for PNG, JPEG, and WebP.
3. External parser tests using independently installed tools.
4. Golden fixture and regression tests, including legacy files.

Additional required properties:

- No parser panics on malformed input.
- Declared container sizes cannot trigger unbounded allocation.
- Output remains decodable by independent tools.
- Unrelated metadata is preserved unless policy explicitly replaces it.
- Reapplying the same notice is idempotent or deterministically merged.
- Conflicts are reported, not silently overwritten.

## Documentation strategy

Documentation should lead with three workflows:

1. Standards-based image rights notice.
2. Web/CDN TDM reservation deployment.
3. Optional hidden or authenticated provenance.

Every user-facing claim should state whether it refers to:

- Standards syntax.
- External parser visibility.
- Transformation survival.
- Authentication.
- Trust or identity.
- Legal effect.

The README must not describe custom or legacy properties as IPTC-standard.

## Release gates

### Release 1 gate

Canonical PLUS Data Mining metadata is emitted and externally visible for every supported format; canonical and legacy fixtures parse correctly; no default writer emits legacy DMI properties.

### Release 2 gate

Equivalent caller input yields semantically equivalent rights notices in PNG, JPEG, and WebP; ownership and contact fields use correct semantics; no format bypasses legal-claim policy.

### Release 3 gate

External-conformance CI is mandatory and green; independently authored fixtures pass; malformed, duplicate, and conflicting metadata are detected; metadata preservation tests pass.

### Releases 4-6 gate

Each later release must preserve Releases 1-3 conformance. Performance, provenance, or API work cannot weaken canonical metadata output or external verification.

## Recommended implementation order

1. Complete Plan 016 without mixing in architecture redesign.
2. Complete Plan 017 on top of canonical mappings.
3. Complete Plan 018 and make external conformance blocking.
4. Reassess the public API after correctness is externally demonstrated.
5. Execute architecture separation, provenance hardening, and release closure in that order.

## Non-goals for Releases 1-3

- Improving steganographic survival under resize, screenshots, or arbitrary recompression.
- Claiming that metadata creates or determines legal rights.
- Proving that a model trained on an image.
- Implementing DRM or adversarial data poisoning.
- Replacing a complete C2PA implementation.
- Broad performance optimization unrelated to conformance.
- Removing support for existing v0.2 files.

## Completion definition

This roadmap is complete when StegoEggo can truthfully state that it emits canonical, independently observable image rights metadata; handles legacy output explicitly; behaves consistently across formats; and enforces external interoperability as a release requirement. Optional steganographic and cryptographic evidence should then build on that reliable metadata foundation rather than compensate for it.
