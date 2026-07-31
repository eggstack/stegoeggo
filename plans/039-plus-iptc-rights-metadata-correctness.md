# Plan 039: PLUS and IPTC Rights Metadata Correctness

Status: Ready for implementation

Baseline: `main` after Plan 038 roadmap commit `73c95cc1f69f299c522358d87dab580877f57646`

Depends on:

- `plans/038-rights-metadata-format-correctness-roadmap.md`

Must be completed before:

- `plans/040-jpeg-dct-container-correctness-and-containment.md`
- `plans/041-webp-container-xmp-exif-correctness.md`
- `plans/042-api-cli-contract-consolidation.md`
- `plans/044-cross-format-correctness-closure.md`

---

## Purpose

Make StegoEggo's primary product channel—rights-reservation metadata—standards-correct and independently verifiable.

The current writer and parser agree with each other, but agreement between two project-owned implementations is not sufficient. The output must use the canonical PLUS controlled-vocabulary representation, must carry the required constraint field when the policy says “see constraints,” and must not emit private markers that contradict the canonical policy.

This plan deliberately addresses rights metadata before JPEG/WebP mechanics. Format-specific plans will preserve and transport the semantic representation established here.

---

## Required end state

For every supported rights policy, the emitted XMP has one unambiguous canonical interpretation.

Expected canonical mapping shape:

```text
RightsPolicy::Allowed
  plus:DataMining = http://ns.useplus.org/ldf/vocab/DMI-ALLOWED

RightsPolicy::ProhibitedAiMlTraining
  plus:DataMining = http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING

RightsPolicy::ProhibitedGenerativeAiTraining
  plus:DataMining = http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-GENAIMLTRAINING

RightsPolicy::ProhibitedExceptSearchIndexing
  plus:DataMining = http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING

RightsPolicy::ProhibitedAllDataMining
  plus:DataMining = http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED

RightsPolicy::ProhibitedSeeConstraints
  plus:DataMining = http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-SEECONSTRAINT
  plus:OtherConstraints = <caller-supplied constraints>
```

The implementation agent must verify exact vocabulary spellings against the applicable PLUS/IPTC documentation before coding. Do not rely solely on the current constants or tests.

Bare keys such as:

```text
DMI-PROHIBITED-AIMLTRAINING
```

may remain accepted for backward compatibility, but they must be reported as legacy/noncanonical StegoEggo input, not `CanonicalPlusDataMining`.

---

## Non-goals

Do not use this plan to:

- implement TDMRep HTTP headers or `/.well-known/tdmrep.json`;
- add C2PA assertions;
- add new image formats;
- redesign the payload-v3 wire format;
- change HMAC or Ed25519 behavior;
- implement a general RDF/XML library;
- rewrite JPEG or WebP containers;
- add required CI jobs;
- publish a new crate version;
- remove backward-compatible parsing without a separate compatibility decision.

---

## Phase 0: Create a standards-correctness ledger

Create `plans/039-status.md` before product edits.

Initialize it with:

```text
Plan baseline SHA: 73c95cc1f69f299c522358d87dab580877f57646
Disposition: OPEN
Canonical PLUS URI emission: OPEN
OtherConstraints emission: OPEN
Private-marker conflict removal: OPEN
Legacy parsing classification: OPEN
Independent fixture validation: OPEN
Documentation: OPEN
Publication hold: no publication is part of this plan
```

Add these tables.

### Table A: policy mapping

```text
policy | expected canonical URI | required companion fields | current emitted form | corrected form | test | status
```

Include every `RightsPolicy`/`DmiValue` variant, including `Unspecified`.

### Table B: compatibility inputs

```text
input representation | canonical | accepted | reported signal kind | emitted by new writer | status
```

Required rows:

- full PLUS vocabulary URI;
- bare `DMI-*` key;
- current `Iptc4xmpExt:DMI-*` private/legacy properties;
- historical `tdm:reserve_tdm` input;
- unknown PLUS URI;
- conflicting canonical and legacy values.

### Table C: external evidence

```text
format | fixture | tool/command | observed property | expected | commit SHA | status
```

Required formats: PNG, JPEG, WebP.

### Phase 0 acceptance criteria

- the ledger exists before source changes;
- every policy is inventoried;
- canonical and compatibility behavior are separated;
- no evidence is marked complete before execution;
- exact spec references used by the implementer are recorded.

Suggested commit:

```text
plans: establish rights metadata correctness ledger
```

---

## Phase 1: Introduce explicit canonical URI mapping

### 1.1 Separate keys from URIs

Update `src/types.rs` so the API no longer encourages a bare key to be used as the external value.

Preferred shape:

```rust
impl DmiValue {
    pub fn plus_vocab_key(self) -> &'static str { /* compatibility/internal identifier */ }

    pub fn plus_vocab_uri(self) -> Option<&'static str> {
        match self {
            DmiValue::Unspecified => None,
            DmiValue::Allowed => Some("http://ns.useplus.org/ldf/vocab/DMI-ALLOWED"),
            // exact verified values for remaining variants
        }
    }
}
```

`Unspecified` should normally produce no `plus:DataMining` property. Do not emit a made-up `DMI-UNSPECIFIED` URI unless the external vocabulary explicitly defines and recommends it.

If `plus_vocab_key()` is public and stable enough that removal would be disruptive, retain it but document that it is not the complete XMP value.

### 1.2 Parse canonical URIs explicitly

Update `from_plus_vocab_key()` or replace it with clearer entry points:

```rust
from_plus_vocab_uri(value: &str) -> Option<Self>
from_legacy_plus_vocab_key(value: &str) -> Option<Self>
```

Do not classify a value as canonical merely because its final path segment resembles a known key.

Required behavior:

```text
http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING
  => canonical

DMI-PROHIBITED-AIMLTRAINING
  => accepted legacy bare key, not canonical

https://example.invalid/DMI-PROHIBITED-AIMLTRAINING
  => unknown, not canonical
```

Avoid an `rsplit('/')` implementation that accepts arbitrary origins.

### 1.3 Preserve unknown values

Verification should report an unknown/unrecognized rights signal without coercing it into a known policy.

Do not silently map malformed URIs to a known prohibition.

### Phase 1 acceptance criteria

- complete canonical URIs are represented explicitly;
- canonical parsing verifies the namespace/prefix, not only the last path segment;
- bare keys remain compatibility inputs only;
- `Unspecified` does not invent a canonical restriction;
- unit tests cover all variants and invalid origins;
- public docs distinguish key and URI semantics.

Suggested commit:

```text
metadata: model canonical PLUS vocabulary URIs explicitly
```

---

## Phase 2: Emit canonical XMP and standard constraints

Primary paths:

```text
src/protected/metadata_trap.rs
src/protected/notice_verification.rs
src/types.rs
```

### 2.1 Use one semantic XMP builder

The legacy `generate_xmp_notice` and request-based `generate_xmp_notice_from_notice` paths must not independently encode standards semantics.

Create or identify one internal normalized rights-XMP builder that consumes a `RightsNotice` plus effective policy and returns the XMP packet. Legacy callers should normalize into the same input.

Do not create a generalized XML framework. A small deterministic builder is sufficient if values are escaped correctly and tests parse the resulting XML.

### 2.2 Emit the full URI

The `plus:DataMining` attribute or element must contain the verified full URI.

Example target form:

```xml
<rdf:Description
    rdf:about=""
    plus:DataMining="http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING">
</rdf:Description>
```

Attribute and element forms may both be parsed, but the writer should emit one stable form.

### 2.3 Emit Other Constraints

For `ProhibitedSeeConstraints`, emit the standard PLUS constraints field using the effective caller-supplied text.

Resolution precedence should be explicit and tested. Recommended precedence:

```text
RightsNotice/LegalMetadata explicit standard constraints text
  > existing AI constraints text normalized into the standard field
  > configuration error when no meaningful constraints are available
```

A rights URL alone is not automatically the same as `plus:OtherConstraints` text. If the external standard permits a URL form, implement that exact representation; otherwise retain the URL in `xmpRights:WebStatement` and require actual constraint text.

The private `stegoeggo:AIConstraints` property may be retained for backward compatibility only if it carries the same effective text and cannot contradict `plus:OtherConstraints`. Prefer not to emit duplicate private data unless a documented compatibility need exists.

### 2.4 Preserve standard legal fields

Do not regress existing valid fields such as:

- `dc:creator`;
- `dc:rights`;
- `xmpRights:WebStatement`;
- `xmpRights:UsageTerms` with `rdf:Alt` and `xml:lang`;
- `photoshop:Credit`;
- `photoshop:DateCreated`;
- `xmp:MetadataDate`.

Private owner/licensor fields should not be described as standard PLUS fields unless they use the correct PLUS namespace and structure.

### Phase 2 acceptance criteria

- all new PNG/JPEG/WebP XMP uses the same canonical URI mapping;
- generated XML parses successfully with a real XML parser in targeted validation;
- `ProhibitedSeeConstraints` emits `plus:OtherConstraints` or the exact verified standard equivalent;
- missing constraints produce a configuration error rather than an incomplete canonical claim;
- `Allowed` and `Unspecified` output are semantically nonprohibitory;
- legacy and request paths generate semantically identical XMP for equivalent inputs.

Suggested commit:

```text
metadata: emit canonical PLUS rights properties
```

---

## Phase 3: Remove contradictory private markers from default output

Current private metadata generation includes generic fields such as:

```text
DMI-PROHIBITED
noai = noindex
```

These must not be emitted unconditionally.

### 3.1 Establish authoritative precedence

Canonical `plus:DataMining` is the authoritative machine-readable policy.

Private compatibility markers must never override or contradict it.

Required outcomes:

```text
Allowed
  no private prohibition marker
  no noindex implication

Unspecified
  no invented prohibition marker

ProhibitedAiMlTraining
  canonical PLUS property required
  optional compatibility marker only if explicitly enabled and semantically equivalent

ProhibitedExceptSearchIndexing
  must not emit noindex
```

### 3.2 Default writer behavior

Preferred result: stop emitting `DMI-PROHIBITED`, `noai`, and similar private fields in new output.

Continue parsing historical values for backward compatibility.

If maintainers require legacy emission, place it behind an explicit compatibility option with a name that makes the nonstandard behavior clear. Do not add that option merely to preserve unused code.

### 3.3 Conflict reporting

When verifying an existing image with canonical and legacy values:

- canonical value remains the effective standard value;
- conflicting legacy value is surfaced as a conflict diagnostic;
- the image is not rewritten automatically during verification;
- replacement processing removes StegoEggo-owned contradictory values according to `MetadataUpdatePolicy`.

### 3.4 Terminology cleanup

Rename internal functions such as `generate_poison_metadata*` to rights-notice terminology.

Consider renaming the internal implementation type from `MetadataTrapProtector` to `RightsMetadataProtector`.

Because `MetadataTrapProtector` is publicly re-exported, preserve source compatibility with a deprecated alias if the rename is performed:

```rust
#[deprecated(note = "Use RightsMetadataProtector")]
pub type MetadataTrapProtector = RightsMetadataProtector;
```

Do not make a public rename mandatory if it causes disproportionate churn. Internal function names must still be corrected.

### Phase 3 acceptance criteria

- `Allowed` cannot produce any StegoEggo-owned prohibition/noindex marker;
- `Unspecified` produces no restriction marker;
- search-indexing policy is not conflated with AI-training policy;
- new output has one authoritative policy representation;
- historical private markers remain readable;
- conflict diagnostics remain available;
- internal poisoning terminology is removed from active implementation paths or explicitly documented as legacy.

Suggested commit:

```text
metadata: remove contradictory private rights markers
```

---

## Phase 4: Correct verification classification

### 4.1 Signal kinds

Update `RightsSignalKind` or related structured verification types so the report can distinguish at least:

```text
CanonicalPlusDataMining
LegacyBarePlusVocabularyKey
LegacyStegoEggoDmi
LegacyTdmReservation
Unknown
```

If adding a public enum variant is undesirable before the next breaking release, represent the distinction in diagnostics while ensuring bare keys are not labeled canonical.

### 4.2 Canonical precedence

When canonical and legacy signals coexist:

- preserve both extracted values;
- select the canonical value as effective;
- emit `ContradictoryLegalClaims` or the appropriate diagnostic when values disagree;
- do not silently collapse disagreement.

### 4.3 Limits-aware and ordinary parsers

The ordinary and resource-limited XMP parsers must produce equivalent semantic results for the same packet within limits.

Avoid maintaining separate policy logic in `parse_xmp_for_dmi` and `parse_xmp_for_dmi_with_limits`. Extract a shared bounded parser core if practical.

### 4.4 XML extraction scope

The current string-based parser may remain if it is bounded and handles the exact emitted forms plus compatibility fixtures. Do not turn this plan into a general XML parser project.

It must not classify values found inside unrelated text or a different namespace as canonical.

### Phase 4 acceptance criteria

- bare keys are not canonical;
- arbitrary-origin URIs ending in a known key are not canonical;
- canonical/legacy disagreement is visible;
- limited and unlimited paths agree;
- unknown values remain unknown;
- existing compatibility fixtures continue to parse where intended.

Suggested commit:

```text
verification: distinguish canonical and legacy rights signals
```

---

## Phase 5: Add focused independent fixtures

### 5.1 Golden XMP packets

Add a small fixture set covering:

- Allowed;
- Prohibited AI/ML training;
- Prohibited generative AI training;
- prohibited except search indexing;
- all data mining prohibited;
- prohibited see constraints;
- unspecified/no property;
- canonical plus legacy agreement;
- canonical plus legacy conflict;
- unknown URI origin;
- historical bare-key output.

The golden expected values must contain complete URIs.

### 5.2 Format transport fixtures

For PNG, JPEG, and WebP, create one representative output per essential semantic class rather than one file per permutation.

Minimum targeted evidence:

```text
format | canonical DataMining visible | OtherConstraints visible when required | image decodes | unrelated metadata preserved
```

Format mechanics beyond XMP transport belong to Plans 040 and 041; this phase may initially use controlled simple images.

### 5.3 External commands

Use the existing manual external verification surface. Suitable commands include:

```bash
exiftool -XMP-plus:DataMining -XMP-plus:OtherConstraints <fixture>
exiftool -XMP-xmpRights:WebStatement -XMP-xmpRights:UsageTerms <fixture>
xmllint --noout <extracted-xmp.xml>
```

Record actual tool versions and output in `plans/039-status.md` or a compact fixture provenance document.

Do not make external tools part of required CI.

### 5.4 Negative interoperability

At least one test must prove that StegoEggo no longer labels its historical bare-key output as canonical.

### Phase 5 acceptance criteria

- every policy mapping has a semantic unit test;
- at least one independent tool observes the full URI in each supported format;
- `OtherConstraints` is independently visible;
- XML packets are well formed;
- historical bare-key input remains readable but noncanonical;
- fixture size remains small and provenance is documented.

Suggested commit:

```text
tests: add independent rights metadata fixtures
```

---

## Phase 6: Documentation and compatibility closure

Update current operational documentation, including as applicable:

```text
README.md
src/lib.rs
architecture/metadata.md
architecture/verification.md
architecture/conformance.md
CHANGELOG.md
DEPRECATIONS.md
STABILITY.md
AGENTS.md
```

Required documentation statements:

- `plus:DataMining` contains a complete PLUS controlled-vocabulary URI;
- bare keys and `Iptc4xmpExt:DMI-*` are compatibility inputs only;
- `ProhibitedSeeConstraints` requires and emits constraints;
- private StegoEggo fields are not substitutes for standard metadata;
- metadata expresses a rights reservation/notice and does not prove ownership or legal enforceability;
- TDMRep deployment artifacts remain out of scope;
- no release is performed by this plan.

Remove or correct stale text claiming that the former bare key is canonical.

### Phase 6 acceptance criteria

- current docs match emitted bytes;
- examples use complete URIs where showing raw XMP;
- private/legacy semantics are clearly labeled;
- no poisoning terminology remains in user-facing current docs;
- changelog records the compatibility significance without claiming publication;
- `./scripts/check.sh` passes.

Suggested commit:

```text
docs: align rights metadata claims with canonical output
```

---

## Difficult implementation examples

### Example A: canonical parser must validate origin

Incorrect:

```rust
let bare = value.rsplit('/').next().unwrap_or(value);
match bare { /* known key => canonical */ }
```

Correct shape:

```rust
const PLUS_VOCAB_PREFIX: &str = "http://ns.useplus.org/ldf/vocab/";

fn from_plus_vocab_uri(value: &str) -> Option<DmiValue> {
    let key = value.strip_prefix(PLUS_VOCAB_PREFIX)?;
    DmiValue::from_legacy_plus_vocab_key(key)
}
```

Use the exact verified scheme and namespace. If the standard permits more than one canonical base URI, enumerate them explicitly rather than accepting arbitrary origins.

### Example B: `ProhibitedSeeConstraints`

Input:

```text
policy = ProhibitedSeeConstraints
ai_constraints = "No training or model evaluation without a written license."
rights_url = "https://example.org/rights/123"
```

Expected semantic output:

```xml
<rdf:Description
  plus:DataMining="http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-SEECONSTRAINT">
  <plus:OtherConstraints>No training or model evaluation without a written license.</plus:OtherConstraints>
  <xmpRights:WebStatement>https://example.org/rights/123</xmpRights:WebStatement>
</rdf:Description>
```

Do not replace the constraint text with the URL unless that is explicitly allowed by the standard representation being implemented.

### Example C: Allowed must remain allowed

Input:

```text
policy = Allowed
```

Forbidden output:

```text
DMI-PROHIBITED = Allowed
noai = noindex
```

Expected output:

```text
plus:DataMining = full DMI-ALLOWED URI
no StegoEggo-owned prohibition marker
```

---

## Required verification commands

At minimum:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p stegoeggo rights
cargo test -p stegoeggo metadata
cargo test -p stegoeggo notice_verification
./scripts/check.sh
```

Run the existing external verification path once after fixtures are complete.

Do not add these focused commands as separate required CI jobs.

---

## Final acceptance criteria

Plan 039 is closed only when:

- `plus:DataMining` emits a verified complete vocabulary URI;
- `Unspecified` does not emit an invented canonical restriction;
- `ProhibitedSeeConstraints` emits standard constraints content;
- `Allowed` and search-indexing exceptions cannot be contradicted by private markers;
- bare keys and historical private properties remain compatibility inputs but are not canonical;
- canonical versus legacy conflicts remain visible;
- legacy and request-based writers share one semantic XMP builder;
- PNG, JPEG, and WebP controlled fixtures expose the full URI to an independent tool;
- current documentation matches the corrected representation;
- required CI architecture is unchanged;
- no crate version is published;
- `plans/039-status.md` contains exact implementation and verification evidence.

---

## Completion definition

The plan is complete when StegoEggo's primary rights signal is valid outside StegoEggo itself. Internal round trips remain useful regression coverage, but external canonical representation is the source of truth.