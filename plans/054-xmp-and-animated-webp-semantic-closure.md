# Plan 054: XMP and Animated WebP Semantic Closure

Status: Ready for implementation

Audited baseline: `main` at `e683c87785d7e4ec60b17fa8ec961d983e4b6fac`

Depends on the retained, correctly landed portions of Plans 051-053. This plan reopens only the XMP semantic-preservation and animated-WebP rows that the post-Plan-053 source audit found still incomplete.

Authoritative implementation ledger: `plans/054-status.md`

---

## 1. Purpose

Plan 053 materially improved the repository, but its completion claim is still too broad. The remaining WebP/XMP defects are narrow and source-local:

1. Preserved RDF descriptions are serialized as unqualified `<Description>` rather than an RDF-qualified description, so a subsequent namespace-aware parse can fail to recognize them as RDF descriptions.
2. Owned XMP element removal is not a true subtree skip. Nested unrelated-looking elements inside an owned subtree can be serialized, and nested end events can reduce the skip depth incorrectly.
3. The final XMP merge path still uses substring searches for `<rdf:RDF>` boundaries even though Plan 053 required the rewrite path to be structurally XML-driven.
4. Filtered descriptions are accumulated without a real byte-identical deduplication pass.
5. The three-round idempotence test proves only one XMP chunk and substring presence, not expanded-name uniqueness or preservation of unrelated semantic values.
6. `ANMF` parsing does not decode the actual 16-byte frame header fields. It currently treats the first four bytes as a reserved word and therefore does not validate frame X/Y, width/height, duration, flag reserved bits, or frame rectangle containment in the canvas.
7. Nested frame `ALPH` + `VP8L` rejection is order-dependent.
8. Animated containers do not explicitly reject top-level `ALPH`.
9. VP8L header failures are ignored in feature derivation in some top-level and frame paths instead of failing the malformed container.

This plan closes those defects without adding format support, public API, release automation, generalized XML infrastructure, or broader CI.

---

## 2. Explicitly retained work — do not rewrite it

The following Plan 053 work is accepted and must remain intact unless a focused regression demonstrates otherwise:

- `quick-xml = 0.41` with minimal features;
- `src/xmp.rs` remains crate-private;
- XMP input is rejected on invalid UTF-8;
- RDF and owned-property recognition uses namespace URI plus local name;
- alternate prefixes are supported;
- inherited namespace bindings are collected;
- VP8L width/height/alpha bit extraction is corrected;
- VP8L version bits are checked by `parse_vp8l_header()`;
- raw VP8X dimension zero means actual dimension one;
- duplicate top-level VP8X, VP8, VP8L, ALPH, ANIM, ICCP, and EXIF checks remain;
- declared VP8X flags remain distinct from derived `WebPFeatures`;
- final WebP validation continues to compare declared flags to independently derived features;
- unknown top-level RIFF chunks continue to be preserved in original order;
- one-job CI and manual release policy remain unchanged.

Do not refactor unrelated policy, JPEG, cryptography, CLI, signing, or detached-manifest code in this plan.

---

## 3. Scope boundaries

Primary files:

```text
src/xmp.rs
src/protected/metadata_trap.rs
src/webp_container.rs
tests/conformance_container_tests.rs
plans/054-status.md
```

Documentation may be corrected only if it describes one of these exact invariants:

```text
AGENTS.md
architecture/protected-metadata-trap.md
```

Not authorized:

- new public XMP API;
- new XML crate or XML DOM framework;
- WebP image encoding/re-encoding;
- VP8 or VP8L codec implementation;
- generalized metadata canonicalization framework;
- dependency upgrades unrelated to `quick-xml` correctness;
- CI matrix expansion;
- release, tag, version, crates.io, or GitHub Release work.

---

# Phase 0 — establish truthful pre-edit state

Before changing product source:

1. Read `plans/054-status.md`.
2. Confirm every residual row is `OPEN`.
3. Record the actual starting SHA.
4. Run only enough focused tests to confirm the current baseline builds; do not rewrite the ledger to `CLOSED` based on broad-suite success.

Expected starting state:

```text
XMP qualified RDF description serialization: OPEN
XMP owned-subtree suppression: OPEN
XMP structural merge without substring parsing: OPEN
XMP deterministic description deduplication: OPEN
XMP semantic three-round idempotence: OPEN
ANMF header field decoding: OPEN
ANMF rectangle bounds: OPEN
ANMF flag validation: OPEN
ANMF ALPH/VP8L order independence: OPEN
Animated top-level ALPH rejection: OPEN
Malformed VP8L propagation: OPEN
Animated rewrite/feature proof: OPEN
```

Do not mark Plan 054 complete until every row has focused evidence.

Suggested commit only after Phase 1 implementation begins; do not create an evidence-only commit before code unless the repository workflow specifically requires it.

---

# Phase 1 — make XMP filtering structurally exact

Primary file: `src/xmp.rs`

## 1.1 Preserve RDF identity when serializing descriptions

Current failure mode:

```text
input:  <rdf:Description ...> ... </rdf:Description>
output: <Description xmlns:rdf="..."> ... </Description>
```

Adding `xmlns:rdf` does not place the unprefixed `Description` element in the RDF namespace.

Required output policy:

- Normalize every preserved RDF description to `rdf:Description`.
- Always include the canonical RDF namespace declaration on the preserved description if it is not already self-contained.
- Preserve all unrelated attribute expanded names and values.
- Preserve all unrelated child expanded names, text, CDATA, comments, and processing instructions that the existing parser supports.
- Preserve required namespace declarations for every prefix used by the serialized description or its descendants.

Preferred serialized shape:

```xml
<rdf:Description
  xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
  xmlns:dc="http://purl.org/dc/elements/1.1/"
  rdf:about="">
  <dc:creator>Example</dc:creator>
</rdf:Description>
```

Do not emit bare `<Description>`.

### Required tests

```text
preserved_description_remains_rdf_qualified
preserved_description_reparses_as_rdf_description
preserved_description_outer_namespace_becomes_self_contained
alternate_rdf_prefix_normalizes_without_semantic_loss
```

Each test must parse the serialized result through `NsReader` and assert the expanded element name. A substring assertion alone is insufficient.

## 1.2 Replace owned-element handling with a true subtree skip depth

The parser must suppress the complete subtree once an owned field begins.

Use logic equivalent to:

```text
on Start while inside rdf:Description:
    push namespace scope

    if owned_skip_depth > 0:
        owned_skip_depth += 1
        serialize nothing
        continue

    if current expanded name is owned:
        owned_skip_depth = 1
        serialize nothing
        continue

    serialize unrelated Start

on Empty while inside rdf:Description:
    if owned_skip_depth > 0:
        serialize nothing
    else if current expanded name is owned:
        serialize nothing
    else:
        serialize unrelated Empty

on Text / CData / Comment / PI:
    serialize only when owned_skip_depth == 0

on End:
    if owned_skip_depth > 0:
        owned_skip_depth -= 1
        pop namespace scope
        serialize nothing
        continue

    otherwise handle normal unrelated element or the rdf:Description close
```

Critical rule:

> While `owned_skip_depth > 0`, no nested event may be interpreted independently for ownership or serialized as unrelated content.

The description-closing check must also use the RDF namespace URI plus local name, not `local_name == "Description"` alone.

### Required owned-subtree fixtures

```text
owned_other_constraints_with_rdf_alt_is_removed_whole
owned_data_mining_with_nested_rdf_structure_is_removed_whole
owned_subtree_between_two_unrelated_children_preserves_both_neighbors
owned_subtree_with_nested_same_local_wrong_namespace_still_removed_whole
owned_empty_element_removed_without_affecting_following_sibling
owned_nested_depth_returns_to_zero_exactly_once
```

Use a canonical nested example such as:

```xml
<plus:OtherConstraints>
  <rdf:Alt>
    <rdf:li xml:lang="x-default">old constraints</rdf:li>
  </rdf:Alt>
</plus:OtherConstraints>
```

The complete `plus:OtherConstraints` subtree must disappear.

## 1.3 Make namespace serialization deterministic

The current custom namespace stack may be retained, but serialization must satisfy:

- every prefix used in the preserved description resolves when reparsed standalone;
- no duplicate namespace declaration with conflicting URI appears on the same element;
- namespace declaration ordering is deterministic;
- unrelated attributes retain decoded values exactly after one parse/serialize cycle;
- `rdf:about` remains correctly RDF-qualified;
- unprefixed attributes remain unqualified;
- XML namespace attributes such as `xml:lang` remain valid.

Prefer deterministic ordering by prefix when adding inherited namespace declarations that are not already present.

Do not introduce a generalized XML canonicalization standard. The goal is only stable self-contained preserved descriptions.

## Phase 1 acceptance criteria

Phase 1 is complete only when:

- preserved descriptions are RDF-qualified after serialization;
- serialized preserved descriptions reparse standalone;
- owned nested element subtrees are completely removed;
- skip depth cannot be reduced by unrelated nested end tags;
- description close recognition uses expanded name;
- alternate prefixes remain equivalent;
- namespace declarations are deterministic and sufficient;
- all required tests pass.

Suggested commit:

```text
xmp: make preserved RDF descriptions and owned subtree filtering exact
```

---

# Phase 2 — eliminate substring-based XMP merge and prove semantic idempotence

Primary files:

```text
src/xmp.rs
src/protected/metadata_trap.rs
```

## 2.1 Move XMP packet merge structure into `src/xmp.rs`

The production rewrite path must no longer find RDF boundaries using:

```text
.find("<rdf:RDF")
.rfind("</rdf:RDF>")
.find('>')
.rfind('<')
```

Create one crate-private helper with a narrow contract, for example:

```rust
pub(crate) fn merge_preserved_descriptions(
    canonical_new_packet: &[u8],
    preserved: &[PreservedDescription],
) -> Result<Vec<u8>>;
```

Required behavior:

1. Strictly parse the complete canonical new packet using `quick-xml`.
2. Identify the RDF container by namespace URI + local `RDF`.
3. Stream the canonical packet to a writer/event buffer.
4. Immediately before the matching RDF end event, insert the preserved descriptions.
5. Preserve the canonical current StegoEggo description exactly once.
6. Return `Err` if the canonical packet is malformed or does not contain exactly one usable RDF container.
7. Never return the unmodified packet as a fallback after a structural parse failure.

A generated packet under repository control is still required to pass the same structural parser. Do not special-case it with string scanning.

`metadata_trap.rs` should orchestrate packet collection and call this helper; it should not understand RDF delimiters.

## 2.2 Deduplicate only byte-identical filtered descriptions

After every existing XMP packet has been strictly parsed and filtered:

- preserve first-seen order;
- deduplicate only descriptions whose filtered serialized `xml` bytes are exactly identical;
- do not deduplicate descriptions merely because they share namespace, subject, or element names;
- do not collapse differing values;
- do not use a global prefix map as a semantic identity key.

Simple bounded implementation:

```text
seen: HashSet<Vec<u8>>
ordered: Vec<Vec<u8>>

for filtered description in packet/chunk order:
    if seen.insert(description.xml.clone()):
        ordered.push(description.xml)
```

This is acceptable because the same deterministic filter serializer produces the bytes being compared.

## 2.3 Remove obsolete structural helpers

After the event merge works, production code must not contain an XMP structural helper that:

- finds `rdf:RDF` by substring;
- finds `rdf:Description` by substring;
- uses `rfind` to locate an XML closing element;
- returns unchanged XMP after failing to locate a structural delimiter;
- scans namespace declarations with raw string matching.

Keep textual formatting helpers only for generating leaf legal-property strings if they are not being used to parse existing XML.

## 2.4 Make semantic assertions parser-based

Add a crate-private test helper that parses final XMP and reports semantic facts by expanded name, for example:

```rust
struct XmpSemanticFacts {
    data_mining_count: usize,
    owned_field_counts: BTreeMap<(String, String), usize>,
    unrelated_values: Vec<...>,
    rdf_description_count: usize,
}
```

It is test-only; do not expose it publicly.

Use it to prove the following after each of three rewrite rounds:

- exactly one top-level WebP XMP chunk;
- final XMP parses successfully;
- exactly one current PLUS `DataMining` property by expanded name;
- stale prior owned properties are absent;
- unrelated attributes and child values from round zero remain exactly once unless input intentionally duplicated different descriptions;
- preserved descriptions remain recognized as RDF descriptions;
- byte-identical filtered descriptions remain deduplicated;
- differing descriptions remain distinct and ordered.

Do not use `contains("plus:DataMining")` as the principal proof.

## 2.5 Namespace conflicts must be scoped correctly

Self-contained preserved descriptions may legally reuse the same prefix for different URIs if the declarations are scoped to different descriptions.

Required policy:

- reject a malformed or internally contradictory declaration within one XML scope;
- do not reject two self-contained sibling descriptions solely because they use the same textual prefix for different URIs;
- generated canonical namespace declarations remain deterministic;
- owned-field recognition always follows the namespace URI in the source packet, not the textual prefix.

If the existing global `check_namespace_conflict()` rejects safe sibling-local mappings, narrow or remove that global conflict rule.

### Required end-to-end tests

```text
webp_nested_owned_other_constraints_removed_whole
webp_preserved_description_is_rdf_qualified_after_rewrite
webp_preserved_description_survives_three_rewrites
webp_two_identical_filtered_descriptions_deduplicate_once
webp_two_differing_descriptions_preserve_both_in_order
webp_same_prefix_different_uri_in_separate_descriptions_is_safe
webp_malformed_existing_packet_fails_before_output
webp_canonical_new_packet_structural_failure_is_error
webp_exactly_one_data_mining_expanded_name_each_round
webp_stale_owned_fields_absent_each_round
webp_unrelated_attribute_value_survives_each_round
webp_unrelated_nested_child_survives_each_round
```

## Phase 2 acceptance criteria

- no substring RDF structural parsing remains in the rewrite path;
- all existing packets are parsed before output construction;
- malformed XMP fails the rewrite;
- filtered descriptions are deduplicated exactly by deterministic serialized bytes;
- differing descriptions remain distinct;
- preserved descriptions remain RDF-qualified;
- exactly one current DataMining expanded-name claim exists after each round;
- three-round semantic idempotence is proven by parser-based assertions;
- no new public API is introduced.

Suggested commit:

```text
xmp: merge packets structurally and prove semantic idempotence
```

---

# Phase 3 — parse the actual ANMF frame header

Primary file: `src/webp_container.rs`

Reference contract:

```text
https://developers.google.com/speed/webp/docs/riff_container
```

Do not infer ANMF layout from the current implementation. Implement the container fields exactly.

## 3.1 Add a checked uint24 reader

Use one bounded helper:

```rust
fn read_u24_le(bytes: &[u8]) -> Result<u32>;
```

or an offset-taking equivalent.

It must require exactly enough bytes and use checked arithmetic where later operations can overflow.

## 3.2 Decode all 16 ANMF header bytes

ANMF frame header layout:

```text
bytes  0..3   Frame X stored value (uint24)
bytes  3..6   Frame Y stored value (uint24)
bytes  6..9   Frame Width Minus One (uint24)
bytes  9..12  Frame Height Minus One (uint24)
bytes 12..15  Frame Duration in milliseconds (uint24)
byte      15  Reserved[7:2], Blend[1], Dispose[0]
```

Semantic decoding:

```text
frame_x = stored_x * 2
frame_y = stored_y * 2
frame_width = width_minus_one + 1
frame_height = height_minus_one + 1
frame_duration_ms = duration
blend = flags & 0x02 != 0
dispose = flags & 0x01 != 0
reserved bits require flags & 0xFC == 0
```

Use checked multiplication/addition.

Preferred structure:

```rust
struct AnmfFrameInfo {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    duration_ms: u32,
    blend: bool,
    dispose: bool,
    has_alpha: bool,
    image_kind: FrameImageKind,
}
```

Do not treat bytes 0..4 as a reserved `u32`.

## 3.3 Validate the frame against the VP8X canvas

Change the frame parser to receive canvas dimensions:

```rust
fn parse_anmf_frame(
    frame_data: &[u8],
    canvas_width: u32,
    canvas_height: u32,
) -> Result<AnmfFrameInfo>;
```

Require:

```text
right = frame_x + frame_width
bottom = frame_y + frame_height
right <= canvas_width
bottom <= canvas_height
```

Use checked addition.

Stored width/height zero is valid and decodes to one pixel.

The frame parser does not need to decode image pixels.

## 3.4 Keep nested chunk cursor exact

Nested frame data begins at offset 16.

For every subchunk:

- require full 8-byte header;
- require payload end within frame;
- require odd-size pad byte within frame;
- require final nested cursor exactly equal frame payload length;
- require exactly one nested `VP8 ` or `VP8L` image payload;
- permit at most one nested `ALPH`;
- `ALPH` is valid only with `VP8 `;
- reject `ALPH` with `VP8L` regardless of which chunk appears first;
- if enforcing reconstruction order, require `ALPH` before `VP8 `;
- propagate malformed VP8L header errors;
- unknown frame subchunks may be preserved only if they are structurally bounded and do not masquerade as reconstruction chunks; do not let them satisfy image/alpha requirements.

The safest small implementation for order independence is to inventory nested chunk kinds first and validate combinations after the loop rather than rejecting only based on what has already been seen.

Example state:

```text
vp8_count
vp8l_count
alph_count
has_alpha
```

After iteration:

```text
vp8_count + vp8l_count == 1
alph_count <= 1
vp8l_count == 1 && alph_count != 0 -> error
```

## 3.5 Reject animated top-level ALPH

When ANIM/ANMF make the container animated:

```text
top-level VP8 count == 0
top-level VP8L count == 0
top-level ALPH count == 0
exactly one ANIM
at least one ANMF
VP8X present
ANIM payload length == 6
```

Top-level ALPH belongs to a still lossy extended image, not the animated frame data path.

## 3.6 Propagate malformed VP8L errors

Current feature derivation must not use:

```rust
if let Ok(alpha) = vp8l_has_alpha(...) { ... }
```

for a chunk already classified as VP8L.

Required behavior:

```text
top-level VP8L classified -> parse_vp8l_header(payload)?
nested VP8L classified -> parse_vp8l_header(payload)?
```

A bad signature, short header, or non-zero version is malformed input and must fail the rewrite/validator.

Do not silently convert malformed VP8L into opaque/no-alpha semantics.

## 3.7 Keep declared and derived features independent

Retain the current good architecture:

```text
parse structure/payloads -> derived WebPFeatures
writer -> derives flags from parsed semantics + intended XMP insertion
validator -> reparses output and independently derives features
```

Do not read the declared alpha or animation bits into the derived values.

ANMF frame alpha must be ORed across all validated frames.

### Required Phase 3 tests

```text
anmf_decodes_x_y_width_height_duration_and_flags
anmf_reserved_flag_bits_rejected
anmf_1x1_frame_accepted
anmf_frame_right_edge_exactly_on_canvas_accepted
anmf_frame_bottom_edge_exactly_on_canvas_accepted
anmf_frame_right_outside_canvas_rejected
anmf_frame_bottom_outside_canvas_rejected
anmf_coordinate_multiply_overflow_rejected_if_representable_path_exists
anmf_nested_chunk_overrun_rejected
anmf_missing_pad_rejected
anmf_final_cursor_mismatch_rejected
anmf_missing_image_rejected
anmf_duplicate_vp8_rejected
anmf_duplicate_vp8l_rejected
anmf_vp8_plus_vp8l_rejected
anmf_duplicate_alph_rejected
anmf_alph_then_vp8l_rejected
anmf_vp8l_then_alph_rejected
animated_top_level_alph_rejected
malformed_top_level_vp8l_header_rejected
malformed_nested_vp8l_header_rejected
valid_opaque_vp8_frame_accepted
valid_alpha_vp8_frame_accepted
valid_opaque_vp8l_frame_accepted
valid_alpha_vp8l_frame_accepted
```

Use correctly encoded ANMF fixtures. Do not initialize the 16-byte header with arbitrary zeros and then assume that proves a general valid frame.

## Phase 3 acceptance criteria

- the actual ANMF 16-byte layout is decoded;
- reserved bits are validated at byte 15;
- X/Y and dimensions use the WebP stored-value semantics;
- frame rectangles are bounded by the VP8X canvas;
- nested chunk boundaries and pads are exact;
- ALPH/VP8L conflict is order-independent;
- animated top-level ALPH is rejected;
- malformed VP8L errors propagate;
- alpha is derived from valid payload semantics only;
- all required tests pass.

Suggested commit:

```text
webp: validate standards-correct ANMF frame semantics
```

---

# Phase 4 — prove animated metadata rewrite, not only parsing

Primary files:

```text
src/webp_container.rs
src/protected/metadata_trap.rs
tests/conformance_container_tests.rs
```

## 4.1 Rewrite policy

The metadata rewrite must continue to copy existing ANIM and ANMF chunk bytes unchanged.

It may change only:

- VP8X flags as required by final actual features;
- XMP chunk set, replacing zero-or-more old XMP packets with one canonical merged XMP packet;
- RIFF size fields and padding required by reconstruction.

Do not re-encode frame image data.

## 4.2 Required successful fixtures

Construct or retain structurally valid fixtures for:

```text
opaque VP8 animation
VP8 + ALPH animation
opaque VP8L animation
alpha VP8L animation
multiple-frame animation with mixed alpha/opaque frames
frame smaller than canvas at nonzero X/Y
```

For each successful rewrite assert:

- output reparses through `validate_webp_output()`;
- exactly one VP8X;
- exactly one XMP;
- exactly one ANIM;
- original ANMF count unchanged;
- each copied ANMF payload byte-for-byte equals its input payload;
- final animation flag is set;
- final alpha flag matches OR of actual frame alpha;
- unknown top-level chunks are preserved;
- metadata values are present through semantic XMP parsing.

## 4.3 Stale declared flags remain normalization cases

For otherwise structurally valid input:

```text
clear declared animation + coherent ANIM/ANMF -> output sets animation
set declared animation + no animation payload -> output clears animation if the input is otherwise accepted by rewrite policy
clear declared alpha + alpha frame -> output sets alpha
set declared alpha + all opaque frames -> output clears alpha
```

Reserved VP8X bits remain hard errors.

If current parser policy intentionally rejects a stale animation bit/payload combination before rewrite, document the policy and make tests match it consistently. Do not claim normalization if the code rejects it.

## 4.4 Optional independent validation

If available locally, use but do not add to required CI:

```text
webpmux -info <fixture>
webpmux -info <rewritten>
```

Record `UNAVAILABLE` if the binary is missing.

## Phase 4 acceptance criteria

- valid animated input can actually be metadata-rewritten;
- copied frame payloads are byte-identical;
- final feature bits match actual frame payload semantics;
- final output passes the production validator;
- alpha animation and opaque animation both succeed;
- no WebP pixel/frame encoding was added;
- all focused fixtures pass.

Suggested commit:

```text
webp: prove metadata rewrite for validated animations
```

---

# Phase 5 — Plan 054 verification and handoff to Plan 055

## 5.1 Focused commands

Run:

```bash
cargo test -p stegoeggo xmp --all-features
cargo test -p stegoeggo webp --all-features
cargo test -p stegoeggo --test conformance_container_tests --all-features
```

Record actual selected/pass counts. Do not copy counts from an earlier SHA.

## 5.2 Required static searches

Run and record:

```bash
rg -n 'pub mod xmp' src
rg -n 'from_utf8_lossy' src/xmp.rs src/protected/metadata_trap.rs
rg -n 'find\(.*rdf:RDF|rfind\(.*rdf:RDF|find\(.*rdf:Description|rfind\(.*rdf:Description' src/xmp.rs src/protected/metadata_trap.rs
rg -n 'if let Ok\(.*vp8l|vp8l_has_alpha.*if let Ok' src/webp_container.rs
```

Expected production-path result:

```text
no public xmp module
no lossy XMP parser conversion
no substring RDF structural merge
no swallowed VP8L parsing error
```

Test code may contain substring assertions as secondary diagnostics, but semantic closure tests must use parser-based assertions.

## 5.3 Workspace checks

Before marking Plan 054 complete:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Do not modify CI to obtain a pass.

## 5.4 Closure behavior

When all Plan 054 rows pass:

- mark `plans/054-status.md` `COMPLETE`;
- leave Roadmap 045 / Plans 051-053 `PARTIAL` because Plan 055 still owns JPEG structural exactness and final evidence reconciliation;
- do not mark historical plans complete yet;
- do not publish or version-bump.

---

# Definition of Done — Plan 054

Every item below must be individually true.

## XMP

1. Preserved RDF descriptions serialize as RDF-qualified elements.
2. Preserved descriptions reparse standalone with the same expanded names.
3. Inherited namespace declarations are sufficient and deterministic.
4. Owned attributes are removed by namespace URI + local name.
5. Owned elements are removed as complete subtrees.
6. Nested events inside an owned subtree are never serialized.
7. Description closing is recognized by expanded RDF name.
8. No substring RDF structural merge remains in production.
9. Canonical new XMP is structurally parsed before merge.
10. All existing packets are parsed before output construction.
11. Any malformed existing XMP packet fails the rewrite.
12. Byte-identical filtered descriptions deduplicate once.
13. Differing filtered descriptions remain distinct and ordered.
14. Safe sibling-local namespace prefix reuse is not treated as a global conflict.
15. Exactly one output XMP chunk is emitted.
16. Exactly one current PLUS DataMining expanded-name property exists.
17. Stale owned fields from earlier rounds are absent.
18. Unrelated attributes survive three rounds.
19. Unrelated child elements survive three rounds.
20. Preserved RDF descriptions remain recognizable after three rounds.
21. XMP semantic idempotence is proven with parser-based assertions.
22. `xmp` remains crate-private.

## Animated WebP / VP8L

23. ANMF X is decoded from bytes 0..3 and multiplied by two.
24. ANMF Y is decoded from bytes 3..6 and multiplied by two.
25. ANMF width-minus-one is decoded from bytes 6..9.
26. ANMF height-minus-one is decoded from bytes 9..12.
27. ANMF duration is decoded from bytes 12..15.
28. ANMF byte-15 reserved bits `0xFC` are rejected when nonzero.
29. Blend/dispose bits are decoded without treating them as reserved.
30. Frame rectangle arithmetic is checked.
31. Frame rectangle must fit the VP8X canvas.
32. Nested frame chunk header/payload/pad bounds are exact.
33. Final nested cursor equals frame payload length.
34. Every frame has exactly one VP8 or VP8L payload.
35. Nested ALPH occurs at most once.
36. ALPH + VP8L is rejected in either ordering.
37. Animated top-level ALPH is rejected.
38. Malformed top-level VP8L header is an error.
39. Malformed nested VP8L header is an error.
40. Frame alpha derives only from validated payload semantics.
41. Final VP8X alpha flag equals actual final frame/static alpha semantics.
42. Final VP8X animation flag equals coherent final animation semantics.
43. Valid opaque animation metadata rewrite succeeds.
44. Valid alpha animation metadata rewrite succeeds.
45. Rewritten ANMF payloads remain byte-identical to input.
46. Production final WebP validation passes all successful fixtures.

## Process

47. Focused XMP tests pass.
48. Focused WebP tests pass.
49. Conformance container tests pass.
50. Workspace fmt passes.
51. Workspace clippy with `-D warnings` passes.
52. No-default-features check passes.
53. Workspace tests pass.
54. `./scripts/check.sh` passes.
55. `plans/054-status.md` contains exact implementation SHAs and observed results.
56. No CI architecture expansion occurred.
57. No release/version/tag/publication work occurred.

If any one of these is false, Plan 054 remains `PARTIAL`.
