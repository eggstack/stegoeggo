# Plan 053: XMP, Animated WebP, and JPEG Exactness Closure

Status: Completed after residual corrective Plans 054 and 055

Audited baseline: `main` at `40cdea8dbc9e110ed6e6bb3d325a10d25903b0b2`

Reopens the unsupported completion claim in:

- `plans/052-status.md`

Completes only the remaining criteria from Plans 048, 049, 051, 052, and Roadmap 045 that are not satisfied by current source.

---

## Purpose

The Plan 052 implementation closed most of the original container-boundary work. The following work is already complete and must not be reopened without a focused failing fixture:

- one canonical legacy protection-level to rights-policy mapping;
- structured missing-constraints reporting;
- exact JPEG entropy-only slicing for the normal single-scan baseline path;
- actual JPEG decoded-block counting;
- rejection of extra complete entropy bytes and invalid final pad bits;
- DHT class rejection for classes outside DC `0` and AC `1`;
- exact referenced SOS Huffman tables rather than table-0 fallback;
- strict RIFF declared-length equality;
- strict top-level RIFF padded-end and final-cursor checks;
- duplicate top-level VP8X rejection;
- basic duplicate/conflicting VP8 and VP8L rejection;
- VP8X payload-length and reserved-field validation;
- a production call to a final WebP validator;
- retention of one required CI job and manual release policy.

The repository is not yet closed because the post-Plan-052 audit found bounded residual defects:

1. The new XMP filter never reliably recognizes `rdf:Description` after splitting its QName.
2. XMP filtering is still gated by literal prefixes and misses the same owned fields under alternate prefixes.
3. Namespace mappings declared outside an isolated description are not available to the filter.
4. Malformed XMP can still be partially skipped or silently omitted rather than failing the rewrite.
5. The writer and parser still contain substring-based XML control flow alongside `quick-xml`.
6. Animated WebP coherence is not validated: ANIM/ANMF multiplicity, frame structure, nested payloads, and top-level conflicts remain open.
7. ANMF alpha is detected during parsing but is not used by the writer when deriving the new VP8X flags.
8. VP8L width, height, alpha, and version bits are decoded with the wrong bit layout.
9. Valid one-pixel VP8X dimensions are rejected because raw width-minus-one or height-minus-one value zero is treated as invalid.
10. Duplicate ANIM, ICCP, and EXIF chunks are not rejected under the selected fail-closed rewrite policy.
11. The final WebP validator compares against feature state partly derived from the declared VP8X flags, making validation circular.
12. The JPEG decoder validates through canonical entries but reconstructs its lookup state through a second canonical-code algorithm.
13. Restart markers detected structurally are not checked by the full DCT support probe when no DRI interval is present.
14. JPEG scan-span bookkeeping records the entropy offset as the SOS marker offset and does not handle marker fill bytes exactly.
15. `src/lib.rs` exposes the internal `xmp` module publicly only to support tests, adding unnecessary public API.
16. Planning and CI evidence still overstate closure and do not name verified evidence for final head `40cdea8`.

This plan corrects those items only. It does not authorize format expansion, public API redesign, new rights policies, new cryptography, release work, or broader CI.

---

## Governing constraints

1. Required CI remains one stable job invoking `scripts/check.sh`.
2. Do not add an OS matrix, scheduled workflow, publication workflow, release workflow, artifact pipeline, binary-size gate, fuzz gate, or mandatory external-tool job.
3. Release remains manual and out of scope. Do not bump versions, publish crates, create tags, or create GitHub releases.
4. Retain `quick-xml = 0.41` with minimal features. Do not add another XML crate.
5. Do not return to substring-based XML parsing for packet, RDF, description, attribute, or child-element boundaries.
6. Keep `ProtectionRequest` as the sole canonical execution-intent model.
7. Keep WebP work container-level. Do not add animated-frame pixel steganography.
8. Keep the existing bounded JPEG DCT policy. Do not add progressive, restart-bearing, arithmetic-coded, lossless, or multi-scan DCT embedding.
9. Unknown top-level WebP chunks and unknown pre-scan JPEG marker segments must remain byte-preserved on successful supported rewrites.
10. Malformed XMP or malformed structural WebP input must fail before output bytes are returned.
11. XMP serialization may normalize insignificant XML formatting, but it must preserve unrelated expanded names, values, element order, and RDF semantics.
12. Tests must prove observable raw/container behavior. Project writer/parser agreement alone is not sufficient.
13. Do not expose parser helpers publicly for integration tests. Put helper tests in unit-test modules and test public behavior through the existing image APIs.
14. Do not change the already-correct policy/default work unless a new focused regression test proves it is broken.
15. Do not add a new framework, crate split, or generic metadata abstraction.
16. A local or CI pass does not close a source contract unless the focused fixture for that contract also passes.
17. Current-head CI evidence must name the exact SHA. If connector evidence is unavailable, record `UNAVAILABLE`; do not infer a pass.
18. No affected roadmap or plan may be marked complete while any Plan 053 ledger row remains open.

---

## Small-model execution rules

These rules are mandatory because this handoff is intended for a smaller implementation model.

1. Complete phases in order. Do not start the next phase until the current phase's focused tests pass.
2. Make only the file changes listed for the current phase unless a compiler error proves one additional file is required.
3. Record every unexpected scope change in `plans/053-status.md` before implementing it.
4. Do not mark a row closed from code inspection alone. A named test or command must exist in the row.
5. Prefer deleting obsolete helpers over leaving two implementations.
6. Do not use `unwrap`, `expect`, lossy UTF-8 conversion, or silent `continue`/`break` behavior in production parsing paths.
7. For every parser error, return before constructing or returning output bytes.
8. Do not weaken a failing fixture to make it pass. Correct the implementation or leave the row open.
9. Use the suggested commit boundaries. If two phases must share a commit, explain why in the ledger.
10. Do not perform release, version, dependency-upgrade, formatting-only, or unrelated cleanup work.

---

## Required execution order

```text
Phase 0  establish truthful Plan 053 planning state and ledger
Phase 1  replace XMP substring extraction with one strict namespace-aware event pipeline
Phase 2  make XMP merge deterministic, fail closed, and private
Phase 3  correct VP8L and validate complete animated-WebP structure
Phase 4  derive final VP8X features from payload semantics and validate output strictly
Phase 5  finish JPEG canonical lookup and structural marker exactness
Phase 6  run focused cross-format verification and bounded cleanup
Phase 7  reconcile evidence and close planning state truthfully
```

Suggested implementation commits:

```text
xmp: replace substring filtering with strict namespace-aware events
xmp: preserve unrelated fields and keep parser internals private
webp: validate VP8L and coherent animation structure
webp: derive and verify final features from payload semantics
jpeg: finish canonical lookup and scan marker exactness
plans: close final container correctness with exact evidence
```

---

## Phase 0: Establish truthful planning state

Primary files:

```text
plans/045-status.md
plans/051-status.md
plans/052-status.md
plans/053-status.md
```

Before product-source edits:

1. Confirm `plans/053-status.md` exists and records baseline `40cdea8dbc9e110ed6e6bb3d325a10d25903b0b2`.
2. Keep Roadmap 045, Plan 051, and Plan 052 `PARTIAL` while Plan 053 is open.
3. Preserve historical commit/test claims as historical claims; do not rewrite them as current-head evidence.
4. Record that `quick-xml` is retained because field-level namespace-aware XMP preservation cannot be implemented safely by substring scanning.
5. Record publication hold explicitly.

Required open rows in `plans/053-status.md`:

```text
XMP whole-packet strict parse
XMP RDF/Description expanded-name recognition
XMP namespace-scope preservation
XMP alternate-prefix owned-field removal
XMP mixed attribute preservation
XMP mixed child-element preservation
XMP malformed fail-closed behavior
XMP multi-packet deterministic merge
XMP three-round idempotence
XMP internal API privacy
VP8L header layout
VP8L version validation
VP8X one-pixel dimensions
WebP structural multiplicity
WebP coherent animation
ANMF nested payload validation
ANMF alpha propagation
Non-circular feature derivation
Strict final WebP validation
JPEG canonical decoder derivation
JPEG restart-marker classification
JPEG SOS/span offsets
JPEG fill-byte handling
Focused tests
Workspace verification
Current-head CI evidence
Publication hold
```

### Phase 0 acceptance criteria

- the exact baseline SHA is recorded;
- all residual findings are open, not prematurely closed;
- no product source is changed before the ledger exists;
- the existing dependency decision is recorded;
- no release action is authorized.

---

## Phase 1: Replace XMP substring extraction with one strict event pipeline

Primary files:

```text
src/xmp.rs
src/protected/metadata_trap.rs
```

### 1.1 Delete the split parsing architecture

The current flow mixes:

- `str::find("<rdf:RDF")`;
- `str::find("<rdf:Description")`;
- literal prefix checks such as `plus:DataMining`;
- a `quick-xml` helper operating on an isolated description.

Replace this with one whole-packet `quick-xml` event pipeline. Do not keep a substring parser as fallback.

The final production flow should be equivalent to:

```rust
fn filter_existing_xmp_packet(packet: &[u8]) -> Result<FilteredXmpPacket>;

struct FilteredXmpPacket {
    descriptions: Vec<PreservedDescription>,
}

struct PreservedDescription {
    xml: Vec<u8>,
}
```

Names may differ, but the responsibilities must remain separate:

```text
parse and validate the entire packet
resolve namespaces in scope
identify RDF and Description by expanded name
remove only owned fields
serialize self-contained preserved descriptions
return an error for malformed or unsupported packet structure
```

### 1.2 Use expanded names, not prefixes

Use namespace URI plus local name for all structural and ownership decisions.

Required namespace constants:

```text
RDF:       http://www.w3.org/1999/02/22-rdf-syntax-ns#
PLUS:      http://ns.useplus.org/ldf/xmp/1.0/
StegoEggo: https://github.com/eggstack/stegoeggo
```

Required RDF description recognition:

```text
namespace URI == RDF URI
local name == Description
```

Do not compare the local name to `rdf:Description` after splitting the QName.

Use `quick_xml::NsReader` or equivalent namespace-resolving functionality. A prefix is only a serialization choice. These must behave identically:

```xml
<rdf:Description .../>
<r:Description xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#" .../>
```

### 1.3 Track namespace scope correctly

Namespace declarations may appear on:

- `x:xmpmeta`;
- `rdf:RDF`;
- `rdf:Description`;
- a nested child element.

The filter must use the namespace scope in effect at each event. It must not build the prefix map only from the isolated description text.

Every serialized preserved description must be self-contained. Before returning it, ensure the description root contains all namespace declarations needed by:

- its own expanded name;
- preserved attributes;
- preserved child elements and their attributes;
- nested descendants.

Preferred bounded strategy:

1. Capture the in-scope prefix-to-URI map when entering the description.
2. Preserve any local declarations.
3. Add missing required declarations to the serialized description root.
4. Keep the original prefixes where they are unambiguous.
5. If a prefix cannot be represented safely without collision, return a metadata error rather than dropping or renaming content ad hoc.

Do not globally merge prefix maps with substring scans.

### 1.4 Remove only explicitly owned expanded names

Retain a single owned-field table in `src/xmp.rs`.

Required owned fields:

```text
PLUS namespace:
- DataMining
- OtherConstraints

StegoEggo namespace:
- ProtectionSeed
- ProtectionLevel
- RightsPolicy
- AIConstraints
- CopyrightOwner
- LicensorName
- LicensorEmail
- LicensorURL
- NoticeAppliedAt
```

For attributes:

- remove an attribute only when its resolved namespace URI and local name match an owned field;
- an unprefixed attribute has no namespace and must not be treated as PLUS or StegoEggo merely because its local name matches;
- the same local name under another namespace must survive.

For elements:

- remove the complete owned element subtree;
- track skip depth exactly;
- do not treat an end tag for an unrelated element as the end of an owned subtree;
- preserve unrelated nested content and order.

Delete literal gates equivalent to:

```rust
if desc.contains("plus:DataMining") || desc.contains("stegoeggo:")
```

### 1.5 Parse and serialize without lossy conversions

Production parsing must not use `String::from_utf8_lossy` for XML names, attributes, or values.

Required behavior:

- invalid UTF-8 in an XMP packet is an error;
- invalid XML is an error;
- invalid attribute syntax is an error;
- unclosed elements are an error;
- unexpected EOF is an error;
- quoted `>` characters in attribute values remain valid;
- comments and processing instructions do not break structural parsing;
- entity escaping remains valid after serialization.

Use `quick_xml::Writer` or event-preserving output construction. Do not construct attribute XML with `format!(" {}=\"{}\"")` using undecoded raw values.

### 1.6 Required Phase 1 unit tests

Put helper tests in `src/xmp.rs`; do not expose the module publicly.

Required tests:

```text
recognizes_rdf_description_with_standard_prefix
recognizes_rdf_description_with_alternate_prefix
recognizes_namespace_declared_on_outer_xmpmeta
recognizes_namespace_declared_on_rdf_element
removes_owned_plus_attribute_under_alternate_prefix
removes_owned_stegoeggo_attribute_under_alternate_prefix
preserves_same_local_name_under_unrelated_namespace
preserves_unprefixed_same_local_name
removes_owned_child_element_subtree
preserves_unrelated_child_before_and_after_owned_child
preserves_nested_unrelated_elements_and_text
preserves_attribute_value_containing_greater_than
preserves_comments_and_processing_instructions_where_supported
rejects_invalid_utf8
rejects_unclosed_description
rejects_mismatched_end_tag
rejects_truncated_owned_element
owned_only_description_is_removed
mixed_description_is_returned_without_owned_fields
```

For preservation tests, parse the filtered output again and assert expanded names and decoded values. Do not assert only substring presence.

### Phase 1 acceptance criteria

- one whole-packet XML parser owns XMP structure;
- RDF descriptions are recognized by expanded name;
- namespace scope is inherited correctly;
- alternate prefixes behave identically;
- only explicitly owned fields are removed;
- malformed XML returns `Err`;
- no substring structural parser remains in the rewrite path;
- no lossy UTF-8 conversion remains in production XMP parsing;
- all named unit tests pass.

Suggested commit:

```text
xmp: replace substring filtering with strict namespace-aware events
```

---

## Phase 2: Make XMP merge deterministic, fail closed, and private

Primary files:

```text
src/xmp.rs
src/protected/metadata_trap.rs
src/lib.rs
tests/conformance_container_tests.rs
```

### 2.1 Parse all input packets before writing output

For every existing top-level WebP XMP chunk:

1. Read the complete packet bytes.
2. Parse and filter it through the Phase 1 function.
3. Accumulate preserved descriptions in input chunk order.
4. If any packet fails, return an error before constructing or returning rewritten bytes.

Do not skip malformed packets because another valid packet exists.

Do not silently `continue` when a chunk's bounds are invalid; top-level WebP parsing should already have made that impossible, and a violated invariant must be an error.

### 2.2 Merge preserved descriptions deterministically

Required output policy:

- emit exactly one XMP chunk;
- preserve filtered unrelated descriptions in original packet/chunk order;
- append or retain exactly one newly generated canonical StegoEggo-owned description;
- remove prior owned fields from all existing packets;
- do not collapse descriptions with differing values;
- deduplicate only filtered descriptions that are byte-identical after the same canonical serialization;
- preserve self-contained namespace declarations on injected descriptions;
- do not use one global prefix map as a reason to discard content.

If safe self-contained serialization cannot be produced, fail the rewrite.

### 2.3 Remove obsolete helpers

Delete or make unreachable all production helpers that perform any of the following:

```text
find <rdf:RDF> using substring search
find <rdf:Description> using substring search
extract namespace declarations using string scanning
skip packets based on literal StegoEggo/PLUS prefixes
return Option on XML parse error
```

There must be one authoritative XMP filter implementation.

### 2.4 Remove the accidental public API

Change:

```rust
pub mod xmp;
```

to crate-private visibility, preferably:

```rust
pub(crate) mod xmp;
```

Move tests that directly call XMP helpers into `src/xmp.rs` unit tests.

Integration tests must exercise XMP behavior through existing public APIs such as byte injection and verification. Do not add a replacement public test-only wrapper.

### 2.5 Required end-to-end WebP XMP fixtures

Add or retain helpers that construct structurally valid WebP containers with controlled XMP chunks.

Required tests:

```text
webp_mixed_owned_and_unrelated_attributes_preserved
webp_mixed_owned_and_unrelated_children_preserved
webp_owned_fields_under_alternate_prefix_replaced
webp_same_local_name_wrong_namespace_preserved
webp_two_compatible_packets_preserve_order
webp_two_differing_packets_preserve_both_values
webp_identical_filtered_descriptions_deduplicate_once
webp_malformed_packet_plus_valid_packet_fails_without_output
webp_outer_namespace_declarations_survive_as_self_contained_description
webp_rewrite_emits_exactly_one_xmp_chunk
webp_rewrite_contains_exactly_one_current_data_mining_claim
webp_three_round_rewrite_is_semantically_idempotent
```

For the three-round test, assert after every round:

- exactly one XMP chunk;
- exactly one current `plus:DataMining` field by expanded name;
- unrelated fields remain present once unless intentionally duplicated in input;
- no owned fields from an earlier round remain;
- the output reparses successfully.

### Phase 2 acceptance criteria

- all XMP packets are parsed before output;
- one malformed packet fails the whole rewrite;
- unrelated fields survive mixed packets;
- exactly one current owned claim remains;
- merge order is deterministic;
- three-round semantic idempotence is proven;
- `xmp` is not part of the public API;
- obsolete substring helpers are removed;
- all named end-to-end tests pass.

Suggested commit:

```text
xmp: preserve unrelated fields and keep parser internals private
```

---

## Phase 3: Correct VP8L and validate complete animated-WebP structure

Primary files:

```text
src/webp_container.rs
```

Focused integration tests may remain in:

```text
tests/conformance_container_tests.rs
```

### 3.1 Separate declared flags from derived features

Introduce or use an internal structure equivalent to:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct WebPFeatures {
    icc: bool,
    alpha: bool,
    exif: bool,
    xmp: bool,
    animation: bool,
}
```

`ParsedWebP` must distinguish:

```text
declared VP8X flags
features derived from actual chunks and payload headers
```

Do not initialize derived alpha or animation from the declared VP8X flags.

The rewrite parser may accept stale non-reserved feature bits so the writer can normalize them. The final-output validator must require exact equality between declared and derived features.

### 3.2 Correct VP8L header parsing

The VP8L payload header is:

```text
byte 0: signature 0x2f
next 32 little-endian bits:
  bits  0..13: width_minus_one
  bits 14..27: height_minus_one
  bit      28: alpha_is_used
  bits 29..31: version, required to be zero
```

Use this exact extraction:

```rust
let bits = u32::from_le_bytes([p1, p2, p3, p4]);
let width = (bits & 0x3fff) + 1;
let height = ((bits >> 14) & 0x3fff) + 1;
let alpha = ((bits >> 28) & 1) != 0;
let version = (bits >> 29) & 0x7;
```

Required behavior:

- reject payload shorter than five bytes;
- reject signature other than `0x2f`;
- reject nonzero version;
- accept stored width or height value zero as actual dimension one;
- return parsed dimensions and alpha from one helper rather than separately reinterpreting the bits;
- do not ignore parser errors for a chunk already classified as VP8L.

Preferred shape:

```rust
struct Vp8lHeader {
    width: u32,
    height: u32,
    alpha: bool,
}

fn parse_vp8l_header(payload: &[u8]) -> Result<Vp8lHeader>;
```

### 3.3 Accept valid one-pixel VP8X dimensions

VP8X stores width-minus-one and height-minus-one in three-byte little-endian fields.

Required behavior:

- raw stored zero decodes to actual one;
- do not reject raw zero;
- reject only arithmetic overflow or impossible encoded length;
- continue to reject caller requests to encode actual width or height zero.

Add tests for:

```text
1x1
1xN
Nx1
maximum representable canvas values
```

### 3.4 Enforce top-level structural multiplicity

Keep multiple XMP chunks permitted as rewrite input because Phase 2 merges them.

Reject before output:

```text
more than one VP8X
more than one VP8
more than one VP8L
VP8 and VP8L together
more than one top-level ALPH
ALPH with VP8L
more than one ANIM
more than one ICCP
more than one EXIF
```

Unknown duplicate top-level chunks remain preserved in order.

### 3.5 Validate coherent animation

Introduce one bounded frame parser, for example:

```rust
struct AnmfFrameInfo {
    has_alpha: bool,
    image_kind: FrameImageKind,
}

fn parse_anmf_frame(frame_payload: &[u8], canvas: (u32, u32)) -> Result<AnmfFrameInfo>;
```

For an animated container, require:

- exactly one ANIM chunk;
- ANIM payload length exactly six bytes;
- at least one ANMF chunk;
- no top-level VP8, VP8L, or ALPH chunk;
- VP8X is present for extended animation input;
- each ANMF payload is at least sixteen bytes;
- ANMF reserved flag bits are zero;
- decoded frame width and height are at least one;
- frame rectangle fits inside the VP8X canvas using checked arithmetic;
- nested payload parsing begins exactly after the sixteen-byte frame header;
- every nested chunk header, payload, and pad byte fits inside the ANMF payload;
- final nested cursor equals the ANMF payload length;
- each frame contains exactly one VP8 or VP8L image payload;
- a frame may contain at most one ALPH chunk;
- frame ALPH is valid only with frame VP8, not frame VP8L;
- duplicate/conflicting frame image payloads are errors;
- unsupported nested structural chunks fail closed rather than being ignored.

For a non-animated container, reject any ANIM or ANMF chunk.

Do not use `break` or `continue` to convert malformed nested frame data into an opaque frame.

### 3.6 Required Phase 3 tests

```text
vp8l_header_parses_1x1_opaque
vp8l_header_parses_dimensions_and_alpha
vp8l_header_rejects_nonzero_version
vp8l_header_rejects_bad_signature
vp8x_accepts_1x1_canvas
vp8x_accepts_1xn_and_nx1_canvas
duplicate_anim_rejected
duplicate_iccp_rejected
duplicate_exif_rejected
anmf_without_anim_rejected
anim_without_anmf_rejected
animation_with_top_level_primary_rejected
anmf_short_header_rejected
anmf_nested_chunk_overrun_rejected
anmf_missing_nested_pad_rejected
anmf_without_image_payload_rejected
anmf_duplicate_vp8_rejected
anmf_vp8_and_vp8l_conflict_rejected
anmf_alph_with_vp8l_rejected
valid_opaque_vp8_frame_accepted
valid_alpha_vp8_frame_accepted
valid_opaque_vp8l_frame_accepted
valid_alpha_vp8l_frame_accepted
frame_rectangle_outside_canvas_rejected
```

### Phase 3 acceptance criteria

- VP8L fields use the correct bit layout;
- VP8L version is checked;
- valid one-pixel dimensions are accepted;
- selected duplicate structural chunks fail closed;
- animation coherence is fully validated;
- every ANMF nested payload is bounded and exact;
- frame alpha is derived from actual frame payloads;
- malformed animation cannot reach the writer;
- all named tests pass.

Suggested commit:

```text
webp: validate VP8L and coherent animation structure
```

---

## Phase 4: Derive final VP8X features from payload semantics

Primary files:

```text
src/webp_container.rs
src/protected/metadata_trap.rs
```

### 4.1 Derive features only from actual output content

For the final emitted container, derive:

```text
ICC       <- exactly one final ICCP chunk
Alpha     <- final top-level ALPH, intrinsic final VP8L alpha, or any final ANMF frame alpha
EXIF      <- exactly one final EXIF chunk
XMP       <- exactly one final XMP chunk
Animation <- coherent final ANIM plus one or more valid ANMF frames
```

Do not derive any feature from the prior declared VP8X bit.

The writer should use the structurally derived input features plus its planned chunk modifications:

```text
preserve derived ICC
preserve derived alpha
preserve derived EXIF
set XMP because one canonical XMP chunk is emitted
preserve derived coherent animation
```

For animation alpha, use the parsed frame information. Do not independently rescan only top-level chunks in `metadata_trap.rs`.

Preferred architecture:

```rust
let final_features = parsed.derived_features.with_xmp(true);
let vp8x_flags = final_features.to_vp8x_flags();
```

### 4.2 Remove circular validation

`validate_webp_output()` must:

1. Parse the emitted output structurally.
2. Read the declared VP8X flags.
3. Independently derive features from actual chunks and payload headers.
4. Require declared flags exactly equal derived flags.
5. Require the output multiplicity and animation invariants from Phase 3.
6. Require exactly one XMP chunk.
7. Require exact RIFF extent and final cursor.

A declared alpha bit must not cause `derived.alpha = true`.

### 4.3 Preserve valid input while normalizing stale feature bits

For structurally valid rewrite input:

- stale feature bits may be normalized to actual payload semantics;
- reserved bits remain errors;
- a set bit with no corresponding payload must be cleared in output;
- a clear bit with corresponding payload must be set in output;
- malformed payload structure remains an error, not a normalization case.

### 4.4 Required Phase 4 tests

```text
stale_input_alpha_bit_cleared_for_opaque_vp8
clear_input_alpha_bit_set_for_alph_vp8
clear_input_alpha_bit_set_for_alpha_vp8l
set_input_alpha_bit_cleared_for_opaque_vp8l
clear_input_alpha_bit_set_for_alpha_anmf
set_input_animation_bit_cleared_without_animation_payload
clear_input_animation_bit_set_for_coherent_animation
stale_icc_bit_normalized
stale_exif_bit_normalized
stale_xmp_bit_normalized
final_validator_rejects_declared_derived_feature_mismatch
valid_1x1_webp_rewrite_succeeds
valid_opaque_animation_metadata_rewrite_succeeds
valid_alpha_animation_metadata_rewrite_succeeds
```

For successful outputs assert:

- exact VP8X flags;
- exact structural chunk counts;
- one XMP chunk;
- full reparse through `validate_webp_output()`;
- independent image decode where the external decoder supports the shape;
- exact unknown top-level chunk preservation.

### Phase 4 acceptance criteria

- declared flags and derived features are distinct values;
- the writer consumes derived payload semantics;
- ANMF alpha reaches the emitted alpha bit;
- validation is not circular;
- stale feature bits normalize correctly;
- valid animated metadata rewrites succeed;
- invalid structures fail before output;
- all named tests pass.

Suggested commit:

```text
webp: derive and verify final features from payload semantics
```

---

## Phase 5: Finish JPEG canonical lookup and marker exactness

Primary files:

```text
src/jpeg_transcoder/header.rs
src/jpeg_transcoder/entropy.rs
src/jpeg_transcoder/mod.rs
```

Focused tests may be added to existing JPEG test modules.

### 5.1 Derive decoder lookup state from canonical entries

The checked builder currently returns canonical entries, but `HuffmanDecoder::from_table()` discards them and independently recomputes code ranges from counts.

Change the decoder so its lookup representation is built directly from `CanonicalHuffmanEntry` values.

Acceptable bounded representation:

```rust
struct HuffmanDecoder {
    by_len: [Vec<(u16, u8)>; 16],
}
```

Construction:

```text
call the one canonical builder
for each entry, append (code, symbol) to by_len[bit_len - 1]
```

Decoding:

```text
read one bit at a time
at each bit length, search the corresponding small sorted vector for the accumulated code
return its symbol when found
```

A compact min/max representation is also acceptable only if every min/max/offset value is derived by iterating the canonical entries rather than repeating canonical-code arithmetic from counts.

The encoder and decoder must call the same builder and consume its returned entries.

### 5.2 Classify restart markers consistently

`JpegHeader::analyze_structure()` already records restart markers.

Update the full support probe so:

```text
structure.has_restart_markers == true -> Unsupported(RestartIntervals)
```

This applies even when the DRI marker is missing or malformed. The existing bounded policy does not support restart-bearing entropy.

Do not allow the entropy reader to silently consume restart markers in the supported path.

### 5.3 Make structural analysis checked

Prefer:

```rust
fn analyze_structure(data: &[u8]) -> Result<JpegStructure>
```

or an equivalent checked helper used by both probing and decoding.

It must return an error for:

- truncated trailing `0xff`;
- truncated segment length;
- segment length smaller than two where a length is required;
- segment extending beyond input;
- malformed SOS length;
- malformed marker-fill sequence;
- arithmetic overflow.

Do not return a partially populated structure as though analysis succeeded.

### 5.4 Correct scan-span offsets

Required fields:

```text
sos_marker_offset       -> leading 0xff of the SOS marker
sos_header_end          -> first byte after complete SOS segment
entropy_start           -> equal to sos_header_end
entropy_end             -> first 0xff of the marker/fill run terminating entropy
terminating_marker_offset -> leading 0xff of the terminating marker run
terminating_marker      -> decoded marker value
```

Do not store `entropy_start` in `sos_marker_offset`.

### 5.5 Handle entropy marker fill bytes exactly

When scanning entropy and encountering `0xff`:

1. Record the first `0xff` offset as the candidate marker-run start.
2. If the next byte is `0x00`, treat exactly `0xff 0x00` as stuffed entropy and continue.
3. If one or more `0xff` fill bytes precede a marker, set `entropy_end` to the first `0xff` in the run.
4. Decode the marker from the first non-`0xff` byte.
5. A dangling run at end of input is malformed.
6. A malformed multiple-`0xff` sequence followed by `0x00` must not be accepted as ordinary stuffed data.

The exact entropy slice must not include marker fill bytes.

### 5.6 Required Phase 5 tests

```text
decoder_lookup_derived_from_canonical_entries
valid_zero_count_intermediate_lengths_decode
restart_marker_without_dri_is_unsupported
restart_marker_with_dri_remains_unsupported
sos_marker_offset_points_to_sos
entropy_start_equals_sos_header_end
entropy_end_excludes_single_marker_prefix
entropy_end_excludes_repeated_fill_bytes
stuffed_ff00_remains_inside_entropy
multiple_ff_before_00_rejected
truncated_ff_run_rejected
truncated_segment_length_rejected
segment_length_smaller_than_two_rejected
supported_baseline_roundtrip_still_decodes
```

### Phase 5 acceptance criteria

- encoder and decoder derive from one canonical entry sequence;
- no second canonical-code algorithm remains;
- restart-bearing scans are consistently unsupported;
- structural analysis fails on malformed boundaries;
- SOS and entropy offsets have exact meanings;
- fill bytes are excluded from entropy while stuffed bytes remain included;
- supported baseline behavior remains green;
- no public JPEG API expansion occurs;
- all named tests pass.

Suggested commit:

```text
jpeg: finish canonical lookup and scan marker exactness
```

---

## Phase 6: Focused cross-format verification and bounded cleanup

Primary files:

```text
src/xmp.rs
src/webp_container.rs
src/protected/metadata_trap.rs
src/jpeg_transcoder/header.rs
src/jpeg_transcoder/entropy.rs
src/jpeg_transcoder/mod.rs
src/lib.rs
tests/conformance_container_tests.rs
plans/053-status.md
```

### 6.1 Required focused test commands

Run focused suites first:

```bash
cargo test -p stegoeggo xmp --all-features
cargo test -p stegoeggo webp --all-features
cargo test -p stegoeggo jpeg --all-features
cargo test -p stegoeggo --test conformance_container_tests --all-features
```

If test-name filters do not select the intended tests, record the actual commands used rather than claiming coverage.

### 6.2 Required workspace verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Do not alter CI architecture to obtain these results.

### 6.3 Required static searches

```bash
rg -n "pub mod xmp" src
rg -n "from_utf8_lossy" src/xmp.rs src/protected/metadata_trap.rs
rg -n "find\(.*rdf:RDF|find\(.*rdf:Description|contains\(.*plus:DataMining|contains\(.*stegoeggo:" src/xmp.rs src/protected/metadata_trap.rs
rg -n "Corrective pass SHA: pending|Disposition: \*\*COMPLETE\*\*" plans/052-status.md plans/053-status.md
```

Expected results:

- no public `xmp` module;
- no lossy conversion in production XMP parsing;
- no substring structural XMP parsing in the rewrite path;
- no pending SHA or premature completion claim.

### 6.4 Optional independent evidence

If available locally, record but do not require in CI:

```text
webpmux -info on generated static and animated WebP
exiftool extraction of final XMP
an independent XML parser reading emitted XMP
an independent JPEG decoder reading supported rewritten fixtures
```

If unavailable, record `UNAVAILABLE` without blocking local source closure.

### 6.5 Bounded cleanup

Allowed cleanup:

- remove obsolete XMP helpers;
- remove now-unused fields and imports;
- correct comments that still describe the old parser or feature derivation;
- keep `quick-xml` minimal-feature configuration;
- update `AGENTS.md` only for durable new invariants.

Not allowed:

- unrelated dependency updates;
- generalized XML framework code;
- new WebP encoding support;
- release/version work;
- CI expansion.

### Phase 6 acceptance criteria

- all focused tests pass;
- all workspace commands pass or are truthfully recorded with failure details;
- static searches prove the obsolete/public paths are gone;
- no scope expansion occurred;
- optional evidence is truthfully labeled;
- the status ledger names exact commands and observed results.

---

## Phase 7: Evidence and planning closure

Primary files:

```text
plans/045-corrective-correctness-closure-roadmap.md
plans/045-status.md
plans/051-final-correctness-and-evidence-closure-corrective-pass.md
plans/051-status.md
plans/052-container-boundary-and-metadata-preservation-closure.md
plans/052-status.md
plans/053-xmp-animated-webp-and-jpeg-exactness-closure.md
plans/053-status.md
AGENTS.md
```

### 7.1 Complete the Plan 053 ledger

Every row must contain:

```text
audited source path
exact contract
implementation commit SHA
focused test name
observed result
final disposition
```

Do not use `pending`, `this session`, or an abbreviated SHA when naming final evidence.

### 7.2 Record exact current-head verification

After the final source/test commit:

1. Record its full SHA.
2. Run the required local commands against that SHA.
3. Query GitHub status/workflow evidence for that exact SHA.
4. Record one of:

```text
PASS — exact run/check identified
FAIL — exact run/check identified
UNAVAILABLE — connector returned no evidence
```

Do not record a pass from `c092fe0`, `40cdea8`, or another earlier SHA as final-head evidence.

### 7.3 Reconcile planning state

Only after all Plan 053 rows are closed:

- mark Plan 053 `COMPLETE`;
- mark Plan 052 `COMPLETE — residual closure completed by Plan 053`;
- update Plan 051 and Roadmap 045 to `COMPLETE` only if no delegated criteria remain open;
- preserve Plan 050 as superseded;
- update status ledgers with the exact closure commits;
- remove stale `Ready for implementation`, `PARTIAL`, and `pending` claims that contradict final state.

If any source, fixture, verification, or evidence row remains open, keep all affected plans `PARTIAL`.

### 7.4 Publication hold

Record explicitly:

```text
No version bump, crates.io publication, tag, GitHub release, or release automation was performed.
```

### Phase 7 acceptance criteria

- every defect row is evidence-backed;
- final source head and final planning head are named exactly;
- CI evidence is exact or honestly unavailable;
- no earlier SHA is presented as current-head proof;
- all planning documents agree;
- publication remains manual and separate.

Suggested commit:

```text
plans: close final container correctness with exact evidence
```

---

## Complete fixture matrix

The implementation is not complete until this matrix is represented by named tests or explicitly documented equivalent tests.

### XMP

```text
standard RDF prefix
alternate RDF prefix
namespace on xmpmeta
namespace on RDF
namespace on Description
owned attribute under standard prefix
owned attribute under alternate prefix
same local name under unrelated namespace
unprefixed same local name
owned child element
unrelated child before and after owned child
nested unrelated subtree
quoted greater-than in attribute
comments/processing instructions
invalid UTF-8
unclosed Description
mismatched end tag
truncated owned subtree
owned-only description
mixed attribute description
mixed child description
multiple valid packets
malformed plus valid packets
identical filtered descriptions
differing filtered descriptions
three rewrite rounds
```

### WebP

```text
RIFF exact extent
odd chunk final pad
VP8X 1x1
VP8X 1xN
VP8X Nx1
opaque VP8
VP8 plus ALPH
opaque VP8L
alpha VP8L
invalid VP8L signature
nonzero VP8L version
duplicate VP8X
duplicate VP8
duplicate VP8L
VP8 plus VP8L
duplicate ALPH
ALPH plus VP8L
duplicate ANIM
duplicate ICCP
duplicate EXIF
ANMF without ANIM
ANIM without ANMF
animation plus top-level primary
short ANMF
nested overrun
missing nested pad
frame without image
frame duplicate image
frame image conflict
frame ALPH plus VP8L
opaque VP8 frame
alpha VP8 frame
opaque VP8L frame
alpha VP8L frame
frame outside canvas
stale input flags normalized
final flag mismatch rejected
unknown top-level chunks preserved
```

### JPEG

```text
canonical entries used by encoder
canonical entries used by decoder
zero-count intermediate code lengths
restart without DRI
restart with DRI
exact SOS marker offset
exact SOS header end
exact entropy start
single marker prefix excluded
marker fill run excluded
stuffed FF00 included
multiple FF before 00 rejected
truncated FF run
truncated segment length
invalid short segment length
supported baseline roundtrip
```

---

## Definition of done

Plan 053 is complete only when all statements below are true.

1. XMP packets are parsed as complete XML documents by one event pipeline.
2. RDF descriptions are recognized by namespace URI and local name.
3. Namespace scope inherited from outer elements is preserved.
4. Alternate prefixes remove the same owned fields.
5. Same local names under unrelated namespaces survive.
6. Unprefixed same local names survive.
7. Owned attributes are removed individually.
8. Owned element subtrees are removed exactly.
9. Unrelated attributes, children, text, and order survive.
10. Malformed XMP fails the complete rewrite.
11. No production substring XMP parser remains.
12. No lossy UTF-8 conversion remains in production XMP parsing.
13. All existing XMP packets are parsed before output construction.
14. Exactly one final XMP chunk is emitted.
15. Exactly one current owned DataMining claim remains.
16. Three-round semantic idempotence is proven.
17. The `xmp` module is crate-private.
18. VP8L width, height, alpha, and version are parsed correctly.
19. Valid one-pixel VP8X dimensions are accepted.
20. Duplicate ANIM, ICCP, and EXIF are rejected.
21. ANIM and ANMF coherence is validated.
22. Every ANMF nested payload is bounded exactly.
23. Every frame has exactly one valid image payload.
24. Frame alpha is derived from actual payloads.
25. Derived features do not use declared VP8X bits.
26. The writer uses derived alpha and animation state.
27. The final validator independently derives all feature bits.
28. Stale valid feature bits are normalized.
29. Invalid structural WebP input returns no output.
30. Unknown top-level chunks remain byte-preserved.
31. JPEG decoder lookup derives from canonical entries.
32. No second canonical-code construction algorithm remains.
33. Restart markers are unsupported regardless of DRI presence.
34. JPEG structural analysis reports malformed boundaries as errors.
35. SOS and entropy offsets have exact documented meanings.
36. Marker fill bytes are excluded from entropy.
37. Stuffed FF00 remains valid entropy.
38. Supported baseline JPEG roundtrips remain green.
39. Focused XMP, WebP, JPEG, and container tests pass.
40. Formatting, clippy, no-default-features, workspace tests, and `scripts/check.sh` pass.
41. Static searches prove obsolete/public parsing paths are gone.
42. Every ledger row names an implementation SHA and focused evidence.
43. Final-head CI evidence is exact or honestly unavailable.
44. Roadmap 045 and Plans 051-053 have mutually consistent status.
45. No version, tag, publication, release, or release automation change occurred.

---

## Explicit non-goals

Do not add:

- general-purpose XML editing APIs;
- public XMP parser APIs;
- arbitrary RDF canonicalization;
- safe namespace-prefix renaming beyond the bounded self-contained-description strategy;
- animated-frame steganography;
- WebP image encoding or pixel decoding beyond bounded header inspection;
- progressive or restart-bearing JPEG DCT embedding;
- new rights vocabulary or legal policy behavior;
- CI matrices or release automation;
- version bumps or publication.

---

## Handoff summary

The next agent should begin with `plans/053-status.md`, then execute phases in order. The highest-risk work is the XMP parser replacement; it must be completed and tested before touching animated WebP. The WebP parser must derive payload semantics separately from declared VP8X flags. JPEG work is a final bounded exactness pass and must not become a codec rewrite.

If a smaller model cannot satisfy a phase without broadening scope, it must leave that phase open in the ledger rather than inventing a partial completion claim.
