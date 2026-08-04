# Plan 046: Rights Metadata Canonical Classification Corrective Pass

Status: Ready for implementation

Baseline: `main` after Roadmap 045 planning commit `65d98d41240ee406280e87be16312857b7da805f`

Depends on:

- `plans/045-corrective-correctness-closure-roadmap.md`

Corrects incomplete criteria from:

- `plans/039-plus-iptc-rights-metadata-correctness.md`
- `plans/044-cross-format-correctness-closure.md`

Must complete before:

- `plans/050-post-corrective-evidence-and-documentation-closure.md`

---

## Purpose

Correct the remaining distinction between canonical PLUS rights signals and backward-compatible noncanonical inputs.

Current output uses full PLUS vocabulary URIs for normal rights policies, but three contract defects remain:

1. `Unspecified` can still fall back to the bare key `DMI-UNSPECIFIED` and be emitted as `plus:DataMining`.
2. The XMP parser uses the permissive bare-key parser for `plus:DataMining`, so bare values are reported as canonical.
3. The bare-key parser strips the final path component from any URL, so an arbitrary-origin URL ending in a recognized key can establish a canonical policy.

This plan fixes serialization and classification while preserving deliberate compatibility for historical StegoEggo output.

---

## Governing decisions

1. A canonical PLUS signal is a complete URI under the exact controlled-vocabulary prefix:

   ```text
   http://ns.useplus.org/ldf/vocab/
   ```

2. A recognized bare `DMI-*` key may remain readable for backward compatibility, but it is noncanonical.
3. An arbitrary-origin URI must not be normalized by taking its last path component.
4. `Unspecified` means absence of the `plus:DataMining` property, not a generated vocabulary value.
5. `ProhibitedSeeConstraints` remains valid only when the standard companion constraints field is handled consistently. Input without constraints may be represented as incomplete/contradictory according to existing warning/report machinery; it must not be silently invented unless the caller used an explicit CLI shorthand whose documented purpose supplies default text.
6. Do not remove legacy `Iptc4xmpExt:DMI-*` parsing in this pass.
7. Do not add a general RDF parser.
8. Do not emit `tdm:reserve_tdm` in image metadata.
9. Do not reintroduce private `noai`, `noindex`, or `DMI-PROHIBITED` output markers.

---

## Phase 0: Create the status ledger

Create `plans/046-status.md` before source edits.

Initialize it with:

```text
Plan baseline SHA: 65d98d41240ee406280e87be16312857b7da805f
Disposition: OPEN
Unspecified omission: OPEN
Canonical URI parser: OPEN
Bare-key compatibility classification: OPEN
Unknown-origin rejection: OPEN
Constraint semantics: OPEN
Tests: OPEN
Documentation: OPEN
CI: OPEN
Publication hold: no publication is part of this plan
```

Add a mapping table:

```text
input/output representation | accepted | semantic DMI | RightsSignalKind | emitted by writer | expected test | status
```

Required rows:

- no property;
- full URI for every supported non-unspecified policy;
- bare known key;
- exact PLUS prefix plus unknown key;
- `https` variant of the vocabulary URL if not explicitly canonical;
- arbitrary-origin URL ending in a known key;
- arbitrary text containing a known key;
- legacy `Iptc4xmpExt:DataMiningAttribute`;
- legacy `Iptc4xmpExt:DMI-Prohibited`;
- conflicting canonical and legacy values;
- `ProhibitedSeeConstraints` with constraints;
- `ProhibitedSeeConstraints` without constraints.

No row may be marked complete until its exact test passes.

---

## Phase 1: Separate parser responsibilities

Primary files:

```text
src/types.rs
src/protected/notice_verification.rs
```

### 1.1 Keep external URI parsing strict

`DmiValue::from_plus_vocab_uri()` must:

- require the exact `PLUS_VOCAB_PREFIX` at byte zero;
- parse only the remaining bare key;
- reject an empty suffix;
- reject embedded slashes, query strings, fragments, whitespace, and unknown keys;
- return `None` for arbitrary origins.

Example required behavior:

```rust
assert_eq!(
    DmiValue::from_plus_vocab_uri(
        "http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING"
    ),
    Some(DmiValue::ProhibitedAiMlTraining)
);

assert_eq!(
    DmiValue::from_plus_vocab_uri(
        "http://evil.example/DMI-PROHIBITED-AIMLTRAINING"
    ),
    None
);
```

Do not implement URI recognition with `rsplit('/')`.

### 1.2 Make bare-key parsing actually bare

Refactor `from_plus_vocab_key()` so it accepts a bare key only.

It must reject values containing:

```text
/
:
?
#
leading/trailing whitespace
```

If backward compatibility requires one helper that accepts either representation, introduce an internal classifier rather than keeping the public bare-key function ambiguous.

Preferred bounded shape:

```rust
enum ParsedDmiRepresentation {
    CanonicalUri(DmiValue),
    LegacyBareKey(DmiValue),
    Unknown,
}

fn classify_plus_data_mining_value(value: &str) -> ParsedDmiRepresentation {
    if let Some(v) = DmiValue::from_plus_vocab_uri(value) {
        return ParsedDmiRepresentation::CanonicalUri(v);
    }
    if let Some(v) = DmiValue::from_plus_vocab_key(value) {
        return ParsedDmiRepresentation::LegacyBareKey(v);
    }
    ParsedDmiRepresentation::Unknown
}
```

The exact type/name may differ. Keep it local unless public exposure is already required by the API.

### 1.3 Preserve compatibility intentionally

Historical bare values remain readable, but the report must distinguish them from canonical values.

Use an existing suitable `RightsSignalKind` variant if one exists. Otherwise add one narrowly named variant such as:

```rust
LegacyBarePlusVocabularyKey
```

Do not report a bare value as `CanonicalPlusDataMining`.

If adding a non-exhaustive enum variant affects serialization, update tests and stability documentation explicitly.

### Phase 1 acceptance criteria

- canonical URI parsing is exact-prefix only;
- bare-key parsing no longer accepts URLs;
- arbitrary-origin values cannot become canonical through suffix extraction;
- recognized bare keys remain readable;
- reports distinguish canonical and legacy representations;
- unknown values do not silently set `canonical_dmi`;
- no new dependency is added.

Suggested commit:

```text
metadata: separate canonical PLUS URIs from legacy bare keys
```

---

## Phase 2: Correct writer omission behavior

Primary file:

```text
src/protected/metadata_trap.rs
```

### 2.1 Do not build a DataMining attribute for Unspecified

The shared XMP builder currently selects:

```rust
plus_vocab_uri().unwrap_or(plus_vocab_key())
```

Remove this fallback for external XMP generation.

The builder must conditionally include the complete attribute:

```text
plus:DataMining="..."
```

only when `plus_vocab_uri()` returns `Some`.

Expected output shape for unspecified policy with other legal metadata:

```xml
<rdf:Description rdf:about="" stegoeggo:ProtectionSeed="42">
  ...copyright/creator/usage terms...
</rdf:Description>
```

There must be no:

```text
DMI-UNSPECIFIED
plus:DataMining=
Iptc4xmpExt:DMI-
noai
noindex
```

### 2.2 Preserve metadata-only legal notices

Omitting DMI must not suppress unrelated legal metadata.

Test at least:

- copyright-only notice;
- creator-only notice;
- usage-terms-only notice;
- seed plus legal metadata;
- completely empty notice where no XMP packet is needed.

### 2.3 Constraint companion field

For `ProhibitedSeeConstraints`:

- emit the canonical URI;
- emit `plus:OtherConstraints` when caller-provided constraints exist;
- preserve the custom StegoEggo constraints field only if it remains part of backward-compatible extraction;
- do not use the custom field as a substitute when asserting external canonical completeness;
- ensure XML escaping and language tagging remain correct.

When constraints are absent, choose and document one deterministic outcome using existing warning/validation machinery. Preferred behavior:

- request validation or execution warning indicates incomplete policy;
- writer does not invent caller-owned legal text;
- emitted canonical DMI may remain if the API intentionally permits incomplete notices, but tests and report must identify the missing companion field.

Do not silently treat missing constraints as complete.

### Phase 2 acceptance criteria

- Unspecified emits no DataMining property in PNG, JPEG, or WebP XMP;
- unrelated legal fields still emit and extract;
- every non-unspecified policy emits exactly one complete canonical URI;
- `ProhibitedSeeConstraints` includes standard constraints when supplied;
- private contradictory markers remain absent;
- output remains valid XML.

Suggested commit:

```text
metadata: omit DataMining for unspecified rights policy
```

---

## Phase 3: Correct XMP extraction classification

Primary file:

```text
src/protected/notice_verification.rs
```

### 3.1 Attribute and element forms

Apply the same strict classifier to both:

```text
plus:DataMining="..."
<plus:DataMining>...</plus:DataMining>
```

and namespace-alias equivalents discovered through `PLUS_NAMESPACE`.

Canonical status must depend on the value, not merely the property namespace.

### 3.2 Do not let unknown canonical-property values establish policy

For this input:

```xml
plus:DataMining="http://evil.example/DMI-PROHIBITED-AIMLTRAINING"
```

required behavior:

- `canonical_dmi == None`;
- `rights_signal_kind != CanonicalPlusDataMining`;
- the value may be ignored or represented as unknown according to current report capabilities;
- no recognized policy is inferred from the suffix.

For a known bare value:

```xml
plus:DataMining="DMI-PROHIBITED-AIMLTRAINING"
```

required behavior:

- backward-compatible `dmi` may be set;
- `canonical_dmi == None` unless that field is deliberately documented as semantic rather than representational;
- `rights_signal_kind` reports legacy bare PLUS vocabulary input.

Prefer keeping `canonical_dmi` representation-specific. If changing that contract would be disruptive, add an explicit representation field and document the distinction. Do not preserve a misleading name solely for convenience.

### 3.3 Conflict handling

When canonical and legacy values conflict:

- canonical full URI wins semantic precedence;
- both values remain observable through existing canonical/legacy fields;
- conflict warning/report remains present;
- legacy prohibition must not override canonical `Allowed`;
- repeated processing under `ReplaceStegoOwned` replaces StegoEggo-owned legacy signals with the requested canonical output.

### Phase 3 acceptance criteria

- all XMP extraction paths use one representation classifier;
- canonical/legacy signal kinds are correct;
- unknown-origin inputs do not set a canonical policy;
- canonical-versus-legacy conflict remains visible;
- namespace aliasing still works;
- resource-limited and unlimited extraction paths behave identically except for configured limits.

Suggested commit:

```text
verification: classify PLUS DataMining representations accurately
```

---

## Phase 4: Replace permissive tests with exact assertions

Primary tests:

```text
tests/cross_format_semantics.rs
tests/semantic_correctness.rs
tests/conformance_container_tests.rs
```

### 4.1 Unspecified exact test

Replace assertions that allow either absence or `Some(Unspecified)`.

Required raw and semantic assertions for each format:

```text
raw XMP does not contain plus:DataMining
raw bytes do not contain DMI-UNSPECIFIED
report.dmi() is None
report.canonical_dmi() is None
unrelated legal fields remain present
```

### 4.2 Unknown-origin exact test

Replace the test that currently expects an evil-origin URL to be recognized.

Required assertions before rewriting:

```text
report.canonical_dmi() is None
report does not identify CanonicalPlusDataMining
recognized policy is not inferred solely from suffix
```

Then process under `ReplaceStegoOwned` with an explicit requested policy and verify that the new canonical output is correct.

### 4.3 Bare-key compatibility test

Construct a fixture manually with a bare value. Do not generate it using the current writer.

Assert:

```text
semantic value remains readable
representation is legacy/noncanonical
new writer never emits the bare form
```

### 4.4 Full mapping matrix

For every non-unspecified policy and PNG/JPEG/WebP:

- assert exact full URI in raw XMP;
- assert internal semantic value;
- assert canonical signal kind;
- assert absence of private prohibition markers for `Allowed`;
- assert standard constraints for `ProhibitedSeeConstraints` with text.

Keep this matrix compact. Parameterize rather than creating dozens of near-identical tests.

### 4.5 Independent evidence test behavior

Any ExifTool-dependent test may skip when ExifTool is unavailable, but must:

- assert the exact `plus:DataMining` value when run;
- assert `plus:OtherConstraints` for the constraints case;
- record the skip in the status ledger;
- not be required CI.

### Phase 4 acceptance criteria

- no test accepts `DMI-UNSPECIFIED` as valid new output;
- no test expects arbitrary-origin suffix parsing;
- bare-key fixtures are independent from the writer;
- exact raw XMP assertions accompany project-owned extraction assertions;
- tests remain focused and deterministic.

Suggested commit:

```text
tests: enforce exact canonical PLUS representation semantics
```

---

## Phase 5: Documentation correction

Required files to inspect:

```text
README.md
src/types.rs
architecture/legal-metadata-field-mapping.md
architecture/protected-metadata-trap.md
architecture/verification.md
architecture/conformance.md
CHANGELOG.md
AGENTS.md
```

Correct at least:

- stale claim that image XMP includes `tdm:reserve_tdm`;
- any claim that bare keys are canonical external values;
- representation and reporting behavior for historical bare values;
- `Unspecified` omission semantics;
- unknown-origin rejection behavior;
- constraints completeness behavior.

Do not edit historical plan text to pretend the original phase succeeded. Plan 050 will add truthful status ledgers.

### Phase 5 acceptance criteria

- public docs match exact output behavior;
- API docs distinguish keys from URIs;
- TDMRep image-emission claims are removed;
- compatibility behavior is explicit;
- no new product guarantee is introduced.

Suggested commit:

```text
docs: align rights signal classification and omission semantics
```

---

## Required verification

Run at minimum:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --all-features --test cross_format_semantics
cargo test --all-features --test semantic_correctness
cargo test --all-features --test conformance_container_tests
cargo test --workspace --all-features
cargo check -p stegoeggo --no-default-features
```

Then run:

```bash
./scripts/check.sh
```

Do not add new CI jobs.

---

## Definition of done

Plan 046 is complete only when:

1. `Unspecified` never emits `plus:DataMining`.
2. `from_plus_vocab_uri()` accepts only exact canonical vocabulary URIs.
3. `from_plus_vocab_key()` accepts only bare known keys.
4. Arbitrary-origin URLs cannot establish a canonical policy.
5. Historical bare values remain readable and are reported noncanonically.
6. Full canonical URIs are reported canonically.
7. Canonical/legacy conflicts remain visible and canonical values take precedence.
8. `ProhibitedSeeConstraints` emits standard constraints when supplied and missing constraints are not silently treated as complete.
9. PNG, JPEG, and WebP tests assert raw representation and semantic extraction.
10. Permissive negative tests have been corrected.
11. Stale TDM image-emission documentation is removed.
12. `plans/046-status.md` contains exact commits, commands, and results.
13. `./scripts/check.sh` passes.
14. No release or publication action occurs.