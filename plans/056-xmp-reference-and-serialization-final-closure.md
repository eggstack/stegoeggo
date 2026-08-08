# Plan 056: XMP Reference and Serialization Final Closure

Status: COMPLETE — implementation and verification recorded in `plans/056-status.md`

Audited baseline: `main` at `81c934d02dd43578482e01a15ea645a62ec0209b`

Authoritative execution ledger: `plans/056-status.md`

This is a narrow corrective pass for the final XMP semantic-preservation defects found after Plans 054 and 055 were marked complete. It does **not** reopen the JPEG structural work or animated-WebP container work that was materially closed by those plans.

No version bump, tag, crates.io publication, GitHub release, release automation, CI expansion, new validation framework, or unrelated refactor is authorized.

---

## 1. Purpose

The Plan 054/055 implementation substantially closed the Roadmap 045 correctness line:

- JPEG supported-path structure is checked and exact;
- JPEG entropy slicing excludes markers/fill and malformed boundaries fail closed;
- animated WebP ANMF fields, bounds, flags, nested payloads, and alpha semantics are validated;
- XMP filtering uses namespace-expanded ownership rather than prefix spelling;
- preserved descriptions are RDF-qualified and self-contained;
- XMP merge is event-driven rather than substring-based;
- filtered descriptions are deduplicated;
- end-to-end XMP/WebP idempotence tests exist.

A post-closure source audit found a small remaining XMP gap in `src/xmp.rs`:

1. `Event::GeneralRef(_)` is rejected unconditionally by both filtering and structural merge. This rejects valid XML 1.0 predefined entity references and numeric character references that may appear in ordinary XMP text.
2. `merge_preserved_descriptions()` serializes `Attribute::value` raw bytes through a second XML escaping pass. `quick-xml` documents `Attribute::value` as raw bytes that may already contain escape sequences, so an input such as `&amp;` can be serialized as `&amp;amp;`.
3. The filter's `Event::End` branch checks for `rdf:Description` closure before checking `owned_depth`. A nested `rdf:Description` inside an owned StegoEggo/PLUS subtree can therefore be mistaken for the enclosing preserved description's close.
4. The claimed owned-subtree suppression is not complete for every event class: `Comment` and `PI` are currently emitted whenever a description output buffer exists, even while `owned_depth > 0`.

These are semantic-preservation defects, not a reason to redesign the XMP subsystem.

---

## 2. Scope

Primary product file:

```text
src/xmp.rs
```

Touch only if required by a focused public-path regression:

```text
src/protected/metadata_trap.rs
tests/conformance_container_tests.rs
```

Planning/evidence files:

```text
plans/045-status.md
plans/051-status.md
plans/052-status.md
plans/053-status.md
plans/054-status.md
plans/055-status.md
plans/056-status.md
```

Do not modify:

- JPEG codec/transcoder behavior;
- WebP ANMF parsing or frame validation;
- WebP VP8X feature derivation;
- image pixel processing;
- rights-policy defaults;
- public API surface unless an existing API must be used by a test;
- dependency set unless an implementation proves impossible with the already-present `quick-xml` 0.41 API;
- CI job count/matrix;
- release/version/publication machinery.

The expected implementation should need **no new dependency**. `quick-xml = "0.41.0"` is already present and exposes the required reference and normalized-attribute APIs.

---

## 3. Baseline source facts

At audited head `81c934d...`, `src/xmp.rs` already contains a correct normalized attribute helper:

```rust
fn attr_raw_value(attr: &Attribute, reader: &NsReader<&[u8]>) -> Result<Vec<u8>> {
    let v = attr
        .decoded_and_normalized_value(
            quick_xml::XmlVersion::Implicit1_0,
            reader.decoder(),
        )
        .map_err(xmp_xml_error)?;
    Ok(v.into_owned().into_bytes())
}
```

Despite the historical name `attr_raw_value`, the helper returns the decoded and XML-normalized semantic value. Other serialization helpers then escape that semantic value exactly once.

The merge path does **not** currently reuse that behavior. It performs:

```rust
let raw_value = attr.value.as_ref();
write_attr_value(raw_value, &mut output);
```

This is the core double-escaping hazard.

The filter and merge loops also currently contain:

```rust
Event::GeneralRef(_) => {
    return Err(Error::Metadata(
        "Entity references are not supported in XMP".to_string(),
    ));
}
```

`quick-xml` 0.41 represents both named general references and numeric character references as `Event::GeneralRef(BytesRef)`.

---

# Phase 0 — establish truthful OPEN state

Before product-code changes:

1. Read `plans/056-status.md`.
2. Record the actual implementation starting SHA.
3. Confirm the residual rows below are OPEN.
4. Preserve all Plan 055 JPEG rows as CLOSED.
5. Preserve all Plan 054 animated-WebP rows as CLOSED.
6. Do not treat the earlier Plan 054/055 workspace pass as evidence for the new reference fixtures; they did not exercise these cases.

Expected open workstream:

```text
Predefined XML reference handling: OPEN
Numeric character-reference handling: OPEN
Unknown/custom entity rejection policy: OPEN
Attribute semantic decode-before-escape: OPEN
Owned-depth end-event precedence: OPEN
Owned-depth comment suppression: OPEN
Owned-depth processing-instruction suppression: OPEN
Reference handling inside owned subtrees: OPEN
Three-round reference/idempotence proof: OPEN
Public WebP rewrite reference preservation: OPEN
Focused verification: OPEN
Workspace verification: OPEN
Final status reconciliation: OPEN
Publication hold: RETAINED
```

---

# Phase 1 — define one bounded XML-reference policy

Primary file: `src/xmp.rs`

Do not add a general XML entity engine.

The supported reference policy is intentionally bounded to XML 1.0 values that do not require a DTD or external entity resolution.

## 1.1 Accept predefined XML entity references

Accept and preserve the semantic values of:

```text
&amp;
&lt;
&gt;
&apos;
&quot;
```

These are ordinary predefined XML references and require no DTD.

The output does not need to preserve the original lexical spelling. For example:

```text
input semantic text:  A &amp; B
output semantic text: A &amp; B
```

is correct even if the implementation resolves `&amp;` to `&` internally and escapes it back to `&amp;` during serialization.

## 1.2 Accept numeric character references

Accept valid decimal and hexadecimal XML character references, for example:

```text
&#38;
&#x26;
&#169;
&#x1F642;
```

Resolve them to their Unicode scalar value and serialize the semantic character safely.

Lexical identity is not required:

```text
&#38;
```

may become:

```text
&amp;
```

because the semantic XML value is identical.

## 1.3 Reject unresolved custom named entities

Do not add DTD/entity expansion support.

Examples that must fail closed unless they are one of the five predefined XML entities:

```text
&companyName;
&externalThing;
```

Likewise, keep `DOCTYPE` unsupported in the XMP rewrite path. This avoids introducing custom entity expansion, external-entity behavior, recursive entity limits, or XXE-like complexity into a local metadata transformer.

## 1.4 Reject invalid numeric references

Invalid code points or syntactically invalid numeric references must fail closed.

At minimum cover:

```text
&#0;
&#x0;
invalid/overflow numeric form
out-of-Unicode-range numeric form
```

Do not silently replace invalid references with U+FFFD.

## 1.5 Preferred helper shape

Add one small helper rather than duplicating reference behavior between filter and merge. Conceptually:

```rust
fn append_xml_reference(reference: &quick_xml::events::BytesRef<'_>, out: &mut Vec<u8>) -> Result<()>;
```

or a semantic equivalent.

Required behavior:

1. If `reference.resolve_char_ref()` returns a character, validate/serialize it as XML 1.0 text.
2. Otherwise decode the reference name.
3. Resolve only the five predefined XML names.
4. Serialize the resolved semantic value through the existing text escaping helper.
5. Return `Error::Metadata` for unknown named entities or invalid numeric references.

Do not concatenate raw `&...;` strings without validation merely to make tests pass.

## Phase 1 tests

Add focused unit tests covering both filtering and structural merge:

```text
filter_accepts_amp_reference_in_unrelated_text
filter_accepts_lt_gt_apos_quot_references
filter_accepts_decimal_character_reference
filter_accepts_hex_character_reference
merge_accepts_amp_reference_in_canonical_text
merge_accepts_decimal_character_reference
merge_accepts_hex_character_reference
unknown_named_entity_is_rejected
invalid_zero_character_reference_is_rejected
invalid_numeric_reference_is_rejected
```

Acceptance:

- valid predefined references no longer cause a rewrite failure;
- valid numeric references no longer cause a rewrite failure;
- output reparses as XML;
- output semantic text is unchanged;
- custom/unresolved entity expansion remains unsupported;
- malformed references fail closed.

---

# Phase 2 — remove raw-attribute double escaping

Primary file: `src/xmp.rs`

## 2.1 Reuse semantic attribute decoding

`merge_preserved_descriptions()` must not serialize `attr.value.as_ref()` through another escaping pass.

Use the existing normalized helper or an equivalently named replacement:

```rust
let value = attr_raw_value(&attr, &reader)?;
append_attr(&mut output, attr.key.as_ref(), &value);
```

The exact implementation may rename `attr_raw_value` to `attr_normalized_value` if that improves clarity, but do not turn this into a broad helper rename across unrelated files.

The invariant is:

```text
raw XML attribute bytes
    -> quick-xml decode + XML normalization
    -> semantic attribute value
    -> exactly one StegoEggo escaping pass
    -> output XML attribute
```

Never:

```text
raw escaped bytes -> escape again
```

## 2.2 Apply to both Start and Empty events

The same rule must be used for attributes on:

```text
Event::Start
Event::Empty
```

Do not fix only one path.

## 2.3 Preserve names and namespaces

This phase changes attribute **value** handling only.

Retain:

- original qualified attribute names;
- namespace declarations;
- attribute order unless the existing writer already normalizes it;
- namespace-expanded semantic interpretation.

Do not add attribute sorting solely for this pass.

## Required tests

```text
merge_attribute_ampersand_is_not_double_escaped
merge_attribute_lt_gt_quotes_roundtrip_semantically
merge_attribute_numeric_reference_roundtrips_semantically
merge_empty_element_attribute_reference_roundtrips
merge_attribute_reference_output_reparses
```

Hard example:

Input attribute:

```xml
<rdf:Description dc:title="A &amp; B" />
```

Required semantic result after merge and reparse:

```text
A & B
```

Forbidden output behavior:

```xml
A &amp;amp; B
```

Acceptance:

- attributes containing XML references are escaped exactly once;
- semantic value survives a parse/serialize/reparse cycle;
- Start and Empty element paths behave identically.

---

# Phase 3 — make owned-subtree suppression event-complete

Primary file: `src/xmp.rs`

The filter already uses `owned_depth` for nested Start/End/Text/CData handling, but event precedence is not complete.

## 3.1 End events: owned depth wins before description-close detection

Current order conceptually does:

```text
if end is rdf:Description:
    close preserved description
else if owned_depth > 0:
    decrement owned depth
```

Reverse the precedence while inside an owned subtree:

```text
if owned_depth > 0:
    decrement owned depth
    pop namespace frame
    emit nothing
    continue

if end is enclosing rdf:Description:
    close preserved description
else:
    serialize unrelated end event
```

This prevents a nested RDF description inside an owned StegoEggo/PLUS field from terminating the outer preserved description.

## 3.2 Comments inside owned subtrees

Current comments are appended whenever `current_out` exists.

Required:

```text
owned_depth > 0 -> suppress comment
owned_depth == 0 and inside preserved description -> preserve comment according to current behavior
outside preserved description -> existing behavior
```

## 3.3 Processing instructions inside owned subtrees

Apply the same rule to `Event::PI`.

An owned subtree must not leak a processing instruction into preserved unrelated metadata.

## 3.4 General references inside owned subtrees

After Phase 1 adds valid `GeneralRef` support:

```text
owned_depth > 0 -> consume/validate as needed, emit nothing
owned_depth == 0 -> preserve semantic value
```

Do not let reference handling accidentally reintroduce owned subtree content.

## 3.5 Empty elements remain depth-neutral

An empty element inside an owned subtree has no matching End event. Preserve the existing correct depth-neutral rule:

```text
owned_depth > 0 + Event::Empty -> emit nothing; do not increment owned_depth
```

## Required tests

```text
owned_nested_rdf_description_does_not_close_outer_description
owned_nested_rdf_description_followed_by_unrelated_field_is_preserved
owned_subtree_comment_is_removed
owned_subtree_processing_instruction_is_removed
owned_subtree_general_reference_is_removed
unrelated_comment_outside_owned_subtree_is_preserved
unrelated_processing_instruction_outside_owned_subtree_is_preserved
owned_depth_returns_to_zero_once_after_nested_rdf_description
```

Use a fixture shaped like:

```xml
<rdf:Description xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns:plus="http://ns.useplus.org/ldf/xmp/1.0/"
                 xmlns:dc="http://purl.org/dc/elements/1.1/">
  <plus:OtherConstraints>
    <rdf:Description>
      <rdf:value>owned &amp; nested</rdf:value>
      <!-- owned comment -->
      <?owned test?>
    </rdf:Description>
  </plus:OtherConstraints>
  <dc:title>must survive</dc:title>
</rdf:Description>
```

Required final semantics:

```text
plus:OtherConstraints subtree absent
dc:title == "must survive"
owned comment absent
owned PI absent
one valid outer rdf:Description remains
```

Acceptance:

- every event inside an owned non-empty subtree is suppressed until its matching End;
- nested `rdf:Description` cannot terminate the outer preserved description;
- unrelated events after the owned subtree remain preserved.

---

# Phase 4 — public WebP/XMP rewrite regression coverage

Primary test file:

```text
tests/conformance_container_tests.rs
```

Unit tests in `src/xmp.rs` prove parser helpers. This phase proves the actual metadata injection path.

## 4.1 Mixed owned/unrelated packet with references

Create or adapt a WebP fixture containing one XMP packet with:

- an owned StegoEggo or PLUS property;
- unrelated attribute value containing `&amp;`;
- unrelated element text containing a decimal numeric reference;
- unrelated element text containing a hexadecimal numeric reference.

Rewrite it through the public metadata injection API.

Verify by namespace-aware parse, not substring matching:

- old owned value removed/replaced according to current policy;
- unrelated semantic values preserved;
- exactly one final XMP chunk;
- final XMP reparses successfully.

## 4.2 Three-round semantic stability

Run the same output through the metadata rewrite three times.

After each round assert:

```text
exactly one final XMP chunk
exactly one effective owned field for each canonical owned property
unrelated attribute semantic value unchanged
unrelated text semantic values unchanged
no &amp;amp; style semantic drift
no duplicate preserved descriptions
```

Do not compare entire XMP packets byte-for-byte if canonical serialization legitimately changes lexical reference spelling.

The invariant is semantic idempotence, not lexical XML identity.

## 4.3 Owned nested-description regression through public API

Build a WebP XMP fixture with the Phase 3 nested owned `rdf:Description` shape and rewrite it.

Verify:

- rewrite succeeds;
- owned subtree is absent;
- unrelated field after that subtree survives;
- final packet is valid XML;
- exactly one XMP chunk is emitted.

## Required conformance tests

Names may follow repository conventions, but evidence must cover equivalents of:

```text
webp_xmp_reference_values_survive_rewrite
webp_xmp_reference_values_are_semantically_stable_three_rounds
webp_xmp_attribute_reference_is_not_double_escaped
webp_owned_nested_rdf_description_does_not_truncate_preserved_metadata
```

---

# Phase 5 — focused verification

Do not create another verification framework or CI matrix.

Run the existing focused and workspace commands.

Required commands:

```bash
cargo fmt --all -- --check
cargo test -p stegoeggo xmp --all-features
cargo test -p stegoeggo webp --all-features
cargo test -p stegoeggo --test conformance_container_tests --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Record exact command, exact result, and exact implementation SHA in `plans/056-status.md`.

Optional external-tool evidence remains optional. Do not block closure on `exiftool`, `webpinfo`, or other tools not already available in the execution environment.

Do not add a second CI job just for XML fixtures.

---

# Phase 6 — truthful closure reconciliation

Only after every Plan 056 source/test criterion passes:

1. Mark `plans/056-status.md` COMPLETE.
2. Update `plans/054-status.md` to explain that its original XMP closure was partial and the final reference/serialization residuals were closed by Plan 056.
3. Update `plans/055-status.md` without reopening JPEG: Plan 055's JPEG work remains complete, while its historical cross-plan closure statement is finalized through Plan 056.
4. Reconcile current dispositions in `plans/045-status.md`, `plans/051-status.md`, `plans/052-status.md`, and `plans/053-status.md`.
5. Use exact implementation-head evidence. If GitHub Actions evidence for the exact implementation SHA cannot be observed, record `UNAVAILABLE`; do not claim PASS from an older SHA.
6. Retain the publication hold.

A passing broad test run cannot override an open source-level Plan 056 contract.

---

## 7. Definition of done

Plan 056 is complete only when all of the following are true.

### XML reference semantics

- [ ] `&amp;` is accepted in preserved XMP content.
- [ ] `&lt;`, `&gt;`, `&apos;`, and `&quot;` are accepted where XML permits them.
- [ ] valid decimal numeric character references are accepted.
- [ ] valid hexadecimal numeric character references are accepted.
- [ ] reference output preserves semantic character values after reparse.
- [ ] unresolved custom named entities remain rejected.
- [ ] invalid numeric references remain rejected.
- [ ] no DTD/external-entity machinery is added.

### Attribute serialization

- [ ] merge Start-event attributes use decoded/normalized semantic values before escaping.
- [ ] merge Empty-event attributes use the same rule.
- [ ] an input `&amp;` attribute does not become semantic `&amp;` text or lexical `&amp;amp;` after rewrite.
- [ ] numeric references in attributes remain semantically stable.
- [ ] final XMP reparses successfully.

### Owned-subtree filtering

- [ ] `owned_depth > 0` is checked before `rdf:Description` close handling.
- [ ] nested owned `rdf:Description` cannot close the outer preserved description.
- [ ] comments inside owned subtrees are suppressed.
- [ ] processing instructions inside owned subtrees are suppressed.
- [ ] general references inside owned subtrees do not leak into preserved output.
- [ ] unrelated content following an owned subtree survives.
- [ ] empty owned elements remain depth-neutral.

### End-to-end behavior

- [ ] mixed owned/unrelated XMP with references rewrites successfully.
- [ ] exactly one final XMP chunk is emitted.
- [ ] unrelated reference-bearing attributes survive semantically.
- [ ] unrelated reference-bearing element text survives semantically.
- [ ] three rewrite rounds do not create semantic drift or duplicate descriptions.
- [ ] nested owned RDF regression passes through the public WebP metadata path.

### Scope/evidence

- [ ] no JPEG source is changed unless a focused regression proves it necessary.
- [ ] no ANMF/WebP frame-parser behavior is changed unless a focused regression proves it necessary.
- [ ] no new dependency is added without a documented blocker in the already-present `quick-xml` 0.41 API.
- [ ] CI remains the existing small one-job verification model.
- [ ] no version/tag/release/publication change occurs.
- [ ] all required focused tests pass.
- [ ] workspace verification passes.
- [ ] `./scripts/check.sh` passes.
- [ ] exact implementation SHA and evidence are recorded truthfully.
- [ ] historical completion ledgers are reconciled only after the above criteria pass.

---

## 8. Implementation guidance for a smaller agent

Keep this pass localized.

Preferred order:

```text
1. Create/confirm Plan 056 OPEN ledger.
2. Add focused failing unit fixtures first.
3. Add one bounded reference helper.
4. Reuse normalized attribute decoding in merge Start/Empty paths.
5. Fix owned-depth event precedence/suppression.
6. Run XMP unit tests.
7. Add public WebP rewrite fixtures.
8. Run focused WebP/conformance tests.
9. Run workspace/check.sh verification.
10. Reconcile status ledgers last.
```

Avoid these tempting but unnecessary expansions:

```text
- replacing quick-xml
- adding a DOM
- adding serde XML
- supporting arbitrary DTD entities
- preserving exact lexical XML spelling
- canonicalizing all namespaces/attributes
- rewriting the XMP writer wholesale
- touching JPEG because the roadmap contains JPEG history
- adding release automation
- adding a CI matrix
```

The target is a final semantic-correctness patch, not another architecture pass.

Suggested implementation commits:

```text
xmp: preserve XML reference semantics during rewrite
xmp: make owned subtree suppression event-complete
tests: prove reference-safe XMP rewrite idempotence
plans: reconcile final XMP closure evidence
```

Commit grouping may be smaller if preferred, but status reconciliation must remain last.
