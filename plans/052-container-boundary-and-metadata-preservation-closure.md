# Plan 052: Container Boundary and Metadata Preservation Closure

Status: Ready for implementation

Audited baseline: `main` at `b414939b0b14083d5c56ae09ae87cade53736776`

Reopens the unsupported completion claims in:

- `plans/045-corrective-correctness-closure-roadmap.md`
- `plans/045-status.md`
- `plans/051-final-correctness-and-evidence-closure-corrective-pass.md`
- `plans/051-status.md`

Completes only the remaining criteria from Plans 048, 049, 051, and Roadmap 045 that are not satisfied by current source.

---

## Purpose

The Plan 051 implementation materially improved the repository. The following work is complete and must not be reopened without a new independently demonstrated defect:

- one legacy `ProtectionLevel` to `RightsPolicy` mapping;
- `Light -> Unspecified` and `Standard -> ProhibitedAiMlTraining` compatibility behavior;
- structured `MissingRightsConstraints` reporting;
- removal of stale current-output TDM emission claims;
- basic JPEG Huffman count/value, empty-table, duplicate-symbol, and oversubscription checks;
- exact SOS DC/AC table references rather than table-0 fallback;
- rejection of JPEG post-scan marker segments from the supported DCT path;
- duplicate top-level WebP VP8X rejection;
- basic final-output VP8X ICC, EXIF, XMP, animation, and ALPH-chunk flag derivation;
- creation of retrospective planning ledgers;
- retention of one required CI job and manual release policy.

The repository is not yet closed because the final audit found bounded defects in the remaining container and evidence contracts:

1. JPEG coefficient decoding still receives the complete JPEG remainder rather than an exact entropy span.
2. `finish_scan()` does not reject extra complete entropy bytes and is called with an expected MCU count passed as both expected and observed.
3. JPEG DHT table classes other than 0 and 1 are accepted as AC tables.
4. Encoder and decoder canonical Huffman construction still use separate algorithms rather than one checked representation.
5. WebP parsing rejects declared lengths larger than physical input but still accepts declared lengths smaller than physical input.
6. WebP parsing does not prove that chunk iteration ends exactly at the declared RIFF extent or that an odd-sized final chunk has its required pad byte.
7. VP8X by itself is still accepted as an image container, and conflicting or duplicate primary image payloads are not rejected.
8. VP8X structural length, reserved bits/bytes, animation shape, and primary payload rules are not validated before rewrite.
9. WebP XMP replacement skips an entire packet when any StegoEggo-owned property is detected, discarding unrelated fields in mixed packets.
10. The current XMP string scanner can partially parse malformed XML and return success after silently omitting content.
11. VP8X alpha derivation does not detect intrinsic VP8L alpha or alpha inside ANMF frame payloads.
12. No final-output WebP validator proves that emitted flags and chunk structure agree.
13. `plans/051-status.md` marks these defects closed and cites tests or CI evidence that do not prove the stated contracts.

This plan corrects those items only. It does not authorize format expansion, public API redesign, additional protection policies, new cryptography, release work, or broader CI.

---

## Governing constraints

1. Required CI remains one stable job invoking `scripts/check.sh`.
2. Do not add an OS matrix, scheduled workflow, publication workflow, release workflow, artifact evidence pipeline, binary-size gate, fuzz gate, or mandatory external-tool job.
3. Release remains manual and out of scope. Do not bump versions, publish crates, create tags, or create GitHub releases.
4. Do not modify the completed legacy policy/default behavior unless a focused regression test proves it is broken.
5. Preserve `ProtectionRequest` as the sole canonical execution-intent model.
6. Preserve the existing bounded JPEG DCT support policy. Do not add progressive, restart-bearing, arithmetic-coded, lossless, or multi-scan DCT embedding.
7. Prefer explicit unsupported or malformed classification over partially rewriting input.
8. Do not add a general JPEG codec.
9. Keep WebP work container-level. Do not add animated-frame pixel steganography.
10. Do not silently discard malformed or unrelated metadata to produce output.
11. Unknown JPEG marker segments and unknown WebP chunks must remain byte-preserved on successful supported rewrites.
12. Tests must prove raw structural behavior, not merely that the project writer and project parser agree.
13. Independent tools are optional local evidence only; absence must not expand CI.
14. Current-head CI evidence must name the exact SHA. A pass for an earlier SHA cannot be recorded as current-head verification.
15. Planning documents must remain truthful while work is in progress. No affected roadmap or closure plan may remain marked complete during an open corrective pass.
16. Keep implementation changes local to the existing JPEG transcoder, WebP container, metadata writer, and focused tests. Do not introduce a new framework or crate split.

---

## Required execution order

Use this order so defects remain attributable and reviewable:

```text
Phase 0  reopen planning state and establish Plan 052 ledger
Phase 1  make JPEG scan spans and entropy exhaustion exact
Phase 2  finish checked JPEG Huffman construction and DHT classification
Phase 3  make WebP RIFF and primary-payload validation exact
Phase 4  replace XMP packet skipping with field-level preservation
Phase 5  derive VP8X alpha/feature state from final payload semantics
Phase 6  validate final containers and add focused integration fixtures
Phase 7  reconcile evidence, documentation, and planning state
```

Phases 1 and 2 may share one implementation commit if the changes are inseparable. Phases 3 through 5 should remain separate commits unless a mechanical refactor is required across them.

Do not mark Plan 052 complete before Phase 7.

---

## Phase 0: Reopen closure state and establish the ledger

Before changing product source:

1. Update `plans/051-status.md` from `COMPLETE` to `PARTIAL — reopened by Plan 052`.
2. Update the status line in `plans/051-final-correctness-and-evidence-closure-corrective-pass.md` to `PARTIAL — remaining closure delegated to Plan 052`.
3. Update Roadmap 045 and `plans/045-status.md` to `PARTIAL — reopened by Plan 052`.
4. Do not rewrite historical implementation claims. Add a dated/current audit note explaining which Plan 051 criteria remained open.
5. Create `plans/052-status.md` before product-source edits.

Initialize `plans/052-status.md` with:

```text
Plan baseline SHA: b414939b0b14083d5c56ae09ae87cade53736776
Disposition: OPEN
JPEG exact entropy span: OPEN
JPEG exact exhaustion: OPEN
JPEG DHT class validation: OPEN
Shared canonical Huffman representation: OPEN
WebP declared RIFF equality: OPEN
WebP final cursor and pad validation: OPEN
WebP primary payload validation: OPEN
WebP VP8X structural validation: OPEN
Mixed XMP field preservation: OPEN
Malformed XMP fail-closed behavior: OPEN
VP8L alpha detection: OPEN
ANMF alpha detection: OPEN
Final WebP validator: OPEN
Focused fixtures: OPEN
Workspace verification: OPEN
Current-head CI evidence: OPEN
Publication hold: no publication is part of this plan
```

Add the following tables.

### Table A: defect closure

```text
item | audited source path | exact contract | implementation commit | focused fixture/test | observed result | disposition
```

Required rows:

- JPEG exact entropy start/end;
- JPEG extra entropy byte rejection;
- JPEG invalid final pad-bit rejection;
- JPEG early entropy exhaustion;
- JPEG truncated stuffed byte;
- JPEG unexpected marker before expected blocks;
- invalid DHT table class;
- shared encoder/decoder canonical entries;
- RIFF declaration smaller than physical input;
- RIFF declaration larger than physical input;
- final cursor before declared RIFF end;
- missing odd-chunk pad byte;
- VP8X wrong payload length;
- VP8X reserved flags/bytes;
- VP8X without primary or animation payload;
- duplicate/conflicting VP8 and VP8L payloads;
- incoherent ANIM/ANMF structure;
- mixed owned/unrelated XMP attributes;
- mixed owned/unrelated XMP child elements;
- malformed owned packet plus valid unrelated packet;
- differing valid packets with compatible namespaces;
- differing valid packets with namespace conflict;
- VP8L alpha true;
- VP8L alpha false;
- ANMF frame alpha;
- final VP8X flag/chunk consistency;
- three-round rewrite idempotence.

### Table B: commands and evidence

```text
command/tool | environment/version | expected result | observed result | exact SHA or evidence location | status
```

### Table C: planning reconciliation

```text
plan | pre-052 claim | open criteria at 052 baseline | corrective commit(s) | final disposition
```

Required rows: 045, 048, 049, 050, 051, and 052.

### Phase 0 acceptance criteria

- the four affected planning documents no longer claim closure while the corrective pass is open;
- `plans/052-status.md` exists before product edits;
- every remaining audited defect has an explicit open row;
- the exact baseline SHA is recorded;
- already completed policy work is listed as retained rather than reopened;
- publication hold is explicit.

Suggested commit:

```text
plans: reopen container correctness closure
```

---

## Phase 1: Make JPEG scan spans and entropy exhaustion exact

Primary files:

```text
src/jpeg_transcoder/header.rs
src/jpeg_transcoder/entropy.rs
src/jpeg_transcoder/mod.rs
tests/jpeg_container_preservation.rs
```

### 1.1 Return exact scan offsets from structural analysis

The current supported path computes scan structure but then passes:

```rust
jpeg_data[scan_start..].to_vec()
```

into coefficient decoding. This includes EOI and any remaining bytes. Replace that behavior with an exact structural result.

Introduce or extend an internal structure equivalent to:

```rust
struct JpegScanSpan {
    sos_marker_offset: usize,
    sos_header_end: usize,
    entropy_start: usize,
    entropy_end: usize,
    terminating_marker_offset: usize,
    terminating_marker: u8,
}

struct JpegStructure {
    scan_count: usize,
    scans: Vec<JpegScanSpan>,
    has_restart_markers: bool,
    has_trailing_segments_after_scan: bool,
    eoi_offset: Option<usize>,
}
```

For the supported single-scan subset:

- `entropy_start` is the first byte immediately after the complete SOS segment;
- `entropy_end` is the offset of the first non-stuffed, non-fill, non-restart marker;
- `terminating_marker_offset` points at the leading `0xFF` of that marker;
- `terminating_marker` must be EOI for the supported path;
- `0xFF 0x00` remains an entropy data byte plus stuffing byte;
- repeated fill `0xFF` bytes before a marker are parsed deterministically;
- restart markers are detected and keep the input unsupported under the existing policy;
- a trailing COM, DNL, DHT, SOS, APP, or other marker after entropy remains unsupported rather than being folded into entropy;
- truncated `0xFF` at end of input is malformed;
- no offset arithmetic may wrap.

Do not use a second independent marker scanner for decoding. The support probe and decoder must consume the same checked scan-span analysis.

### 1.2 Decode only the exact entropy slice

Update `JpegTranscoder::decode_coefficients()` so the decoder receives only:

```rust
&jpeg_data[scan.entropy_start..scan.entropy_end]
```

The entropy reader should no longer need to identify EOI from within its buffer. Any marker recognition retained inside `BitReader` must exist only to reject a malformed unexpected marker, not to delimit the normal supported scan.

Preferred flow:

```text
parse header
analyze structure and exact scan span
classify supported/unsupported/malformed
slice entropy exactly
calculate expected MCU/block traversal
fully decode expected blocks
finalize reader and prove only legal pad bits remain
```

### 1.3 Track observed progress honestly

Remove the ineffective call shape where expected MCU count is passed as both expected and observed.

Track one of:

- actual MCUs completed; or
- actual component blocks completed compared with the exact expected count.

A suitable finalizer shape is:

```rust
fn finish_scan(
    &self,
    expected_blocks: usize,
    decoded_blocks: usize,
) -> Result<()>
```

or:

```rust
fn finish_scan(&self) -> Result<()>
```

where the reader itself already knows the expected boundary and the decode loop cannot return before all expected blocks are complete.

The finalizer must not receive a fabricated observed value.

### 1.4 Prove exact bitstream exhaustion

After the final expected coefficient block:

1. Determine the remaining bits in the current entropy byte.
2. Permit only JPEG legal all-one pad bits required to reach the next byte boundary.
3. Reject any zero or mixed pad-bit pattern.
4. Reject any unread complete entropy byte.
5. Reject a dangling stuffed-byte prefix or incomplete byte-stuffing pair.
6. Reject any attempt to read past `entropy_end` before all expected blocks are complete.
7. Reject an unexpected marker found inside the exact entropy span.
8. Do not require the reader to consume EOI; EOI is outside the exact entropy slice and is already proven by structural analysis.

The invariant after successful finalization must be:

```text
all expected blocks decoded
and
reader position == end of entropy slice after accounting for legal final pad bits
```

`bytes_consumed()` may remain as a diagnostic, but success must use it or equivalent state to prove equality.

### 1.5 Required JPEG entropy fixtures

Create minimal deterministic fixtures or mutation helpers for:

```text
valid baseline grayscale scan
valid baseline 4:4:4 scan
valid baseline 4:2:2 scan
valid baseline 4:2:0 scan
one extra 0x00 entropy byte before EOI
one extra 0xFF00 stuffed entropy byte before EOI
invalid zero final pad bits
invalid mixed final pad bits
early EOI before final MCU/block
truncated final entropy byte
truncated 0xFF stuffing prefix
unexpected non-EOI marker before expected blocks complete
marker-like 0xFF00 sequence that must remain valid entropy
```

For each malformed fixture assert:

- no panic;
- no partial coefficient map returned;
- no DCT-success report;
- deterministic malformed/entropy error text or typed reason;
- fallback behavior does not mislabel malformed data as a supported DCT rewrite.

For supported fixtures assert:

- coefficient decode succeeds;
- DCT encode succeeds through the preserving path;
- independent `image` decode succeeds;
- APP/COM/unknown segment preservation tests remain green.

### Phase 1 acceptance criteria

- coefficient decoding receives an exact entropy slice, not the JPEG remainder;
- the structural analyzer is the sole source of scan offsets;
- EOI is proven structurally outside the entropy reader;
- actual decoded progress is tracked or structurally guaranteed without fabricated arguments;
- legal all-one pad bits are the only unread bits permitted;
- extra complete entropy bytes fail;
- early exhaustion fails;
- stuffed bytes remain correctly handled;
- all existing supported baseline sampling fixtures remain supported;
- no public API change is introduced.

Suggested commit:

```text
jpeg: enforce exact entropy span and exhaustion
```

---

## Phase 2: Finish JPEG DHT classification and shared canonical construction

Primary files:

```text
src/jpeg_transcoder/header.rs
src/jpeg_transcoder/entropy.rs
tests/jpeg_container_preservation.rs
```

### 2.1 Reject invalid DHT table classes

JPEG Huffman tables used by this subset have only:

```text
class 0 -> DC
class 1 -> AC
```

Reject any class greater than 1 before storing the table. Do not route every nonzero class into the AC table collection.

Add fixtures for at least:

- class 2, table 0;
- class 15, table 3;
- a valid class 0 table;
- a valid class 1 table.

Invalid classes are malformed input, not unsupported valid input.

### 2.2 Use one checked canonical representation

The current source validates tables centrally but still builds encoder and decoder lookup state through separate canonical-code algorithms. Replace this with one checked representation used by both.

Recommended internal model:

```rust
#[derive(Clone, Copy)]
struct CanonicalHuffmanEntry {
    symbol: u8,
    code: u16,
    bit_len: u8,
}

struct CanonicalHuffmanTable {
    entries: Vec<CanonicalHuffmanEntry>,
}

fn build_canonical_huffman_table(
    counts: &[u16; 16],
    values: &[u8],
) -> Result<CanonicalHuffmanTable>
```

The builder must own all validation:

- exact count/value equality;
- nonempty referenced table;
- at most 256 symbols;
- unique symbols;
- non-oversubscribed code space;
- checked code increment and shift;
- each code fits its declared bit length;
- every value index is in range.

Construct:

- decoder min/max/offset arrays or direct decode lookup; and
- encoder symbol-to-code lookup

from the same returned entries.

Do not duplicate canonical progression logic in `HuffmanDecoder::from_table()` and `HuffmanEncoderTable::build()`.

### 2.3 Keep incomplete but legal code space distinct from oversubscription

Do not reject a structurally valid incomplete Huffman tree solely because unused code space remains. The required rejection is oversubscription, duplicate symbols, malformed lengths, or missing referenced symbols needed by actual entropy.

Add a valid incomplete custom table fixture to prevent accidental over-restriction.

### 2.4 Required Huffman tests

Direct unit tests should cover:

```text
count sum shorter than values
count sum longer than values
empty table
over 256 symbols
duplicate symbol
oversubscribed lengths
invalid class
checked progression across zero-count intermediate lengths
valid incomplete table
encoder and decoder derive identical code/length for every symbol
missing referenced DC table
missing referenced AC table
```

The missing-reference tests must actually mutate SOS references. Do not cite unrelated disabled-level behavior as evidence.

### Phase 2 acceptance criteria

- DHT classes outside 0/1 fail before table storage;
- encoder and decoder derive from one canonical table builder;
- no `wrapping_*` arithmetic remains in Huffman construction;
- all malformed fixtures fail deterministically;
- valid custom and standard tables round-trip;
- the existing exact-reference behavior remains intact;
- no new JPEG feature support is added.

Suggested commit:

```text
jpeg: unify canonical Huffman construction
```

---

## Phase 3: Make WebP RIFF and primary-payload validation exact

Primary files:

```text
src/webp_container.rs
src/protected/metadata_trap.rs
tests/conformance_container_tests.rs
```

### 3.1 Require exact declared RIFF extent for rewrite

For rewrite-capable parsing, require:

```rust
declared_end == data.len()
```

Reject both:

- `declared_end > data.len()`; and
- `declared_end < data.len()`.

Do not silently ignore physical trailing bytes outside the declared RIFF form.

If a looser parser is required for read-only inspection elsewhere, separate the policy explicitly:

```rust
parse_webp_for_rewrite(...)
parse_webp_for_inspection(...)
```

Do not weaken rewrite validation to accommodate inspection.

### 3.2 Require exact final cursor and pad-byte containment

After iterating chunks:

- the final cursor must equal `declared_end` exactly;
- fewer than eight unexplained bytes before `declared_end` are malformed;
- every payload must fit;
- every odd-sized chunk must have one physical pad byte inside the declared extent;
- checked arithmetic must be used for payload end and padded end;
- a missing final pad byte is malformed even if the payload itself fits;
- an existing pad byte may contain any value for input compatibility, but output should normalize pad bytes to zero and document that pad bytes are not payload;
- chunk payload bytes, including unknown chunks, remain byte-identical on successful rewrite.

Recommended parsing shape:

```rust
let payload_end = data_start.checked_add(chunk_size)?;
let padded_end = payload_end.checked_add(chunk_size & 1)?;
if padded_end > declared_end { error }
pos = padded_end;
...
if pos != declared_end { error }
```

### 3.3 Validate VP8X structure before rewrite

Require:

- at most one VP8X;
- VP8X payload length exactly 10 bytes;
- reserved VP8X flag bits are zero;
- reserved VP8X bytes 1 through 3 are zero;
- dimensions are nonzero after plus-one decoding;
- dimensions fit the 24-bit VP8X representation;
- VP8X alone does not satisfy the primary image requirement.

For the current VP8X feature-byte layout, permit only the known feature bits already used by the writer. Reject or explicitly normalize unknown/reserved bits before output; this plan chooses rejection for rewrite input because it avoids silently changing unknown semantics.

### 3.4 Validate non-animated primary image structure

For a non-animated WebP rewrite, require exactly one primary image payload:

```text
one VP8
or
one VP8L
```

Reject:

- zero primary image payloads;
- more than one VP8;
- more than one VP8L;
- both VP8 and VP8L;
- top-level ANMF without animation structure;
- ALPH paired with VP8L;
- duplicate ALPH chunks;
- ALPH without a VP8 payload.

Preserve the current image-payload bytes verbatim.

### 3.5 Validate bounded animation structure

StegoEggo does not add pixel steganography to animation frames, but metadata rewrite must not emit an incoherent container.

For an animated input accepted for metadata rewrite, require:

- VP8X animation bit or structurally detected animation that can be normalized;
- exactly one ANIM chunk;
- at least one ANMF chunk;
- no top-level VP8 or VP8L primary image alongside ANMF frames;
- every ANMF payload is at least the fixed frame-header length;
- each ANMF nested payload parses as a bounded RIFF-style subchunk sequence;
- each frame has one VP8 or VP8L payload;
- ALPH is permitted only with frame VP8;
- nested payload and pad arithmetic is checked;
- unknown nested frame chunks are preserved verbatim if the frame is otherwise structurally accepted.

Do not broaden this phase into frame decode or frame steganography.

### 3.6 Metadata chunk multiplicity policy

Use an explicit rewrite policy:

- multiple XMP chunks are permitted as input because Phase 4 merges or rejects them deterministically;
- duplicate VP8X, VP8, VP8L, ALPH, ANIM, ICCP, or EXIF chunks are rejected before output;
- unknown duplicate chunks are preserved in order;
- do not silently select the first or last structural chunk.

If existing compatibility requirements demand duplicate ICCP or EXIF preservation, record that exception in the ledger and add exact tests. The default for this plan is fail closed because output validity cannot be claimed after arbitrary collapse.

### 3.7 Required RIFF/structure fixtures

Add exact fixtures for:

```text
declared RIFF size larger than physical input
declared RIFF size smaller than physical input
trailing physical bytes outside RIFF extent
payload fits but required odd-byte pad is missing
1-7 unexplained bytes before declared end
chunk padded end crosses declared end
VP8X length 9
VP8X length 11
VP8X reserved flag bit set
VP8X reserved byte nonzero
VP8X-only container
single VP8 container
single VP8L container
duplicate VP8
duplicate VP8L
VP8 plus VP8L
duplicate ALPH
ALPH without VP8
ANMF without ANIM
ANIM without ANMF
duplicate ANIM
animated container with top-level primary payload
valid bounded animation with one frame
```

For accepted fixtures assert exact unknown-chunk and primary-payload preservation. For rejected fixtures assert no output is produced.

### Phase 3 acceptance criteria

- rewrite parsing requires declared and physical extent equality;
- final cursor equality is proven;
- required pad-byte containment is proven;
- VP8X length and reserved fields are validated;
- VP8X alone is rejected;
- non-animated containers have exactly one primary payload;
- bounded animation structure is coherent;
- duplicate structural chunks are rejected deterministically;
- successful rewrites preserve payload and unknown chunks;
- no new image or animation processing feature is introduced.

Suggested commit:

```text
webp: enforce exact RIFF and payload structure
```

---

## Phase 4: Replace XMP packet skipping with field-level preservation

Primary files:

```text
Cargo.toml
src/protected/metadata_trap.rs
src/protected/notice_verification.rs
tests/conformance_container_tests.rs
tests/cross_format_semantics.rs
```

### 4.1 Remove packet-level StegoEggo skipping

Delete behavior equivalent to:

```rust
if xmp_has_stego_properties(existing) {
    continue;
}
```

Every existing XMP packet must be parsed before rewrite. A packet containing one owned field can also contain unrelated fields that must survive.

The output policy remains:

```text
all StegoEggo-owned fields replaced according to current request
all unrelated valid fields preserved
exactly one output XMP chunk
malformed or ambiguous input fails before output
```

### 4.2 Use a real bounded XML event parser

The current string scanning approach is not sufficient for mixed attributes/elements, namespace scope, quoted `>` characters, comments, processing instructions, or malformed closing tags.

Use a small streaming XML parser rather than extending ad hoc substring matching. The preferred implementation is one lightweight direct dependency such as `quick-xml` with only required features enabled.

Before adding it, record in `plans/052-status.md`:

- the current parser silently skips mixed packets and can return partial success on malformed XML;
- field-level namespace-aware filtering is required;
- a maintained streaming parser is less code and lower risk than a new bespoke XML parser;
- no serde, async, or unrelated XML features are enabled unless actually needed;
- dependency impact from `cargo tree` is recorded.

Do not add a DOM framework or general RDF library.

### 4.3 Inventory exact owned properties

Before filtering, create one internal explicit ownership set based on properties the current writer emits and verification code recognizes.

The set must use namespace URI plus local name, not prefix text. At minimum inventory:

- PLUS DataMining;
- PLUS OtherConstraints if emitted;
- every StegoEggo namespace property emitted by current PNG/JPEG/WebP writers;
- legacy StegoEggo-owned aliases that replacement policy already claims ownership over.

Do not treat every property in the PLUS namespace as StegoEggo-owned. Preserve unrelated PLUS fields.

Document the ownership set in code comments and focused tests.

### 4.4 Filter at attribute and child-element granularity

For every `rdf:Description` in every input packet:

- remove only attributes whose expanded name is in the owned-property set;
- remove only child elements whose expanded name is in the owned-property set, including their nested content;
- preserve unrelated attributes on the same description;
- preserve unrelated child elements on the same description;
- preserve `rdf:about` and other RDF structural attributes;
- preserve text, entities, and nested content of unrelated elements semantically;
- discard the description only if nothing unrelated remains after owned-field removal;
- do not match ownership by raw prefix because prefixes are caller-controlled.

Add the newly generated owned properties in one canonical writer-owned description. Do not scatter them across copied descriptions.

### 4.5 Merge multiple valid packets deterministically

For all valid input XMP packets:

1. Parse all packets before writing any output.
2. Extract filtered unrelated descriptions in input packet/chunk order.
3. Track namespace URI bindings required by preserved content.
4. Reject a namespace-prefix conflict if the same prefix is required for different URIs and safe deterministic renaming is not implemented.
5. Preserve compatible differently prefixed namespaces.
6. Deduplicate only byte/semantic-identical filtered descriptions; do not collapse differing values.
7. Emit one packet containing preserved unrelated descriptions plus one new canonical owned description.
8. Emit exactly one WebP XMP chunk.

A namespace conflict must fail before output rather than silently selecting one URI.

### 4.6 Fail closed on malformed packets

Any malformed input XMP packet participating in rewrite must fail the entire rewrite before output, including when another packet is valid.

Required malformed cases:

- non-UTF-8 bytes;
- missing `rdf:RDF` root;
- unclosed `rdf:Description`;
- mismatched element close;
- truncated attribute quote;
- malformed namespace declaration;
- malformed packet containing an owned property;
- malformed packet followed by a valid unrelated packet;
- valid packet followed by malformed packet.

Do not return a partial list after a parse error. Do not skip malformed owned packets.

### 4.7 Preserve metadata-update policy behavior

Retain the current external policy meanings:

- `ReplaceStegoOwned`: replace owned fields, preserve unrelated fields;
- `FailOnConflict`: detect conflicting owned values before replacement and fail;
- `PreserveExisting`: retain existing owned values according to current documented behavior and add only missing values.

If the current WebP path does not fully distinguish these modes, do not redesign the public API in this plan. Implement the exact behavior already documented and add mode-specific tests.

### 4.8 Required XMP fixtures

Add raw packet fixtures for:

```text
owned attribute only
unrelated attribute only
owned and unrelated attributes in one rdf:Description
owned child element and unrelated child element in one rdf:Description
unrelated nested child content
unrelated PLUS namespace property
same namespace URI with different prefixes
same prefix with conflicting namespace URIs
two differing valid packets
two identical valid packets
owned packet plus unrelated packet
mixed owned/unrelated packet
malformed owned packet plus valid packet
valid packet plus malformed packet
three repeated replacement rounds
```

For successful cases assert:

- exactly one XMP chunk;
- exactly one current owned property set;
- unrelated values remain present;
- obsolete owned values are absent for replace mode;
- no unbounded packet growth over three rounds;
- output parses through the independent XML parser.

For error cases assert no output bytes are returned.

### Phase 4 acceptance criteria

- no packet is skipped merely because it contains an owned property;
- ownership is matched by namespace URI/local name;
- mixed descriptions preserve unrelated attributes and elements;
- unrelated PLUS fields survive;
- all packets are parsed before output;
- malformed input fails the full rewrite;
- namespace conflict behavior is deterministic;
- one output XMP packet/chunk remains;
- three-round rewriting is bounded and idempotent for owned fields;
- the XML dependency, if added, remains narrowly configured and justified.

Suggested commit:

```text
webp: preserve mixed XMP fields with checked parsing
```

---

## Phase 5: Derive VP8X alpha and feature state from final payload semantics

Primary files:

```text
src/webp_container.rs
src/protected/metadata_trap.rs
tests/conformance_container_tests.rs
```

### 5.1 Parse intrinsic VP8L alpha

VP8L stores alpha capability in its lossless bitstream header rather than a separate ALPH chunk.

Add a bounded helper equivalent to:

```rust
fn vp8l_has_alpha(payload: &[u8]) -> Result<bool>
```

Validate before reading:

- minimum lossless header length;
- VP8L signature byte;
- nonzero decoded dimensions;
- supported version bits.

Read the alpha-used bit from the checked VP8L header. Do not infer alpha from stale VP8X input flags and do not decode the full image solely to answer this bit.

Add fixtures for VP8L with the alpha bit clear and set.

### 5.2 Parse alpha inside ANMF frames

For every accepted ANMF frame:

- parse the nested frame payload after the fixed frame header;
- report alpha if a valid nested ALPH chunk exists with VP8;
- report alpha if the nested VP8L header has intrinsic alpha;
- reject incoherent nested ALPH/VP8L combinations under Phase 3 rules.

A container-level alpha flag is true if any final frame requires alpha.

### 5.3 Derive all final feature bits from final output inventory

Before VP8X emission compute:

```text
ICCP      <- final ICCP chunk presence
Alpha     <- final top-level ALPH, intrinsic final VP8L alpha, or any final ANMF frame alpha
EXIF      <- final EXIF chunk presence
XMP       <- final sole XMP chunk presence
Animation <- coherent final ANIM plus ANMF structure
```

Do not retain a bit solely because the input VP8X bit was set.

### 5.4 Add a final-output validator

After constructing output but before returning it:

1. Parse the emitted bytes with the strict rewrite parser from Phase 3.
2. Recompute the final chunk/payload feature inventory.
3. Read the emitted VP8X flags.
4. Require exact equality between flags and recomputed features.
5. Require exactly one VP8X for metadata-bearing extended output.
6. Require exactly one XMP chunk when XMP was requested.
7. Require structural primary/animation validity.
8. Require declared RIFF size and physical output length equality.
9. Require all copied unknown and primary payload chunks to match the source bytes recorded before rewrite.

The validator should be a small internal function used by production rewrite and directly tested. Do not rely only on assertions compiled out of release builds.

### 5.5 Required final-flag fixtures

Add exact tests for:

```text
stale input XMP flag with no XMP chunk
stale input EXIF flag with no EXIF chunk
stale input ICC flag with no ICCP chunk
stale input alpha flag with opaque VP8
clear input alpha flag with ALPH chunk
clear input alpha flag with alpha-bearing VP8L
set input alpha flag with opaque VP8L
animation bit clear with coherent ANIM/ANMF
animation bit set without coherent animation
alpha-bearing ANMF frame
opaque ANMF frame
mixed-XMP replacement output
three-round replacement output
```

For each accepted case assert both raw VP8X bits and recomputed semantic inventory.

### Phase 5 acceptance criteria

- intrinsic VP8L alpha is parsed correctly;
- ANMF frame alpha is detected without frame pixel processing;
- every VP8X feature bit derives from final output semantics;
- stale input flags cannot survive without supporting chunks/payload;
- the production writer validates its final output before return;
- output remains byte-preserving for copied payload/unknown chunks;
- no animated-frame steganography is added.

Suggested commit:

```text
webp: derive and validate final VP8X feature state
```

---

## Phase 6: Focused integration verification

Primary tests:

```text
tests/jpeg_container_preservation.rs
tests/conformance_container_tests.rs
tests/cross_format_semantics.rs
tests/request_api.rs
```

Do not add a new broad verification framework. Extend the existing focused suites or add at most one narrowly named container-boundary integration test file if fixture organization requires it.

### 6.1 JPEG integration matrix

Required rows:

```text
fixture | expected DCT capability | expected decode result | expected fallback/report | container preservation
```

Include:

- supported grayscale;
- supported 4:4:4;
- supported 4:2:2;
- supported 4:2:0;
- extra entropy byte;
- invalid pad bits;
- early exhaustion;
- malformed stuffing;
- invalid DHT class;
- missing exact table reference;
- trailing post-scan segment;
- progressive;
- restart-bearing;
- multi-scan.

Differentiate:

- valid supported;
- valid but unsupported;
- malformed.

Malformed inputs must not be reported as merely progressive or capacity-limited.

### 6.2 WebP integration matrix

Required rows:

```text
fixture | parser result | rewrite result | XMP count | expected flags | payload preservation
```

Include:

- exact RIFF extent;
- smaller declared extent;
- larger declared extent;
- missing final pad;
- VP8X-only;
- valid VP8;
- valid opaque VP8L;
- valid alpha VP8L;
- valid bounded animation;
- malformed animation;
- mixed owned/unrelated XMP;
- differing compatible XMP packets;
- namespace conflict;
- malformed plus valid packets;
- unknown chunk preservation;
- three-round idempotence.

### 6.3 Cross-format policy regressions

Run the existing focused tests proving the completed Plan 051 policy work remains intact:

- Light compatibility mapping;
- Standard default policy;
- explicit Unspecified precedence;
- missing constraints warning;
- strict missing-constraints behavior;
- no current TDM emission.

Do not expand policy tests beyond regression protection for this plan.

### 6.4 Independent local evidence

Where already available locally, run:

```text
image crate decode for JPEG and WebP
webpinfo or webpmux for WebP structure
ExifTool for XMP inspection
jpegtran or ImageMagick identify for supported JPEG output
```

Record tool version, exact command, exact fixture, and result in `plans/052-status.md`.

Do not install these into CI. If unavailable, record `UNAVAILABLE` rather than inferring success.

### Phase 6 acceptance criteria

- every residual defect has a focused failing-before/passing-after test;
- supported JPEG sampling modes remain green;
- malformed JPEG entropy cases fail closed;
- strict WebP structural fixtures fail closed;
- mixed XMP unrelated metadata survives;
- VP8L and frame alpha flags are correct;
- copied payload and unknown chunks remain exact;
- completed policy behavior has no regression;
- no broad matrix or long-running gate is introduced.

Suggested commit:

```text
test: close container boundary regressions
```

---

## Phase 7: Evidence, documentation, and planning closure

Primary files:

```text
plans/045-corrective-correctness-closure-roadmap.md
plans/045-status.md
plans/048-status.md
plans/049-status.md
plans/051-final-correctness-and-evidence-closure-corrective-pass.md
plans/051-status.md
plans/052-status.md
AGENTS.md
architecture/jpeg-transcoder.md
architecture/protected-metadata-trap.md
README.md
CHANGELOG.md
```

### 7.1 Correct documentation to exact behavior

Document:

- JPEG supported scans are structurally bounded before entropy decode;
- only legal final pad bits are accepted;
- malformed entropy does not downgrade to a supported DCT result;
- unsupported post-scan/restart/progressive/multi-scan inputs retain the existing fallback policy;
- WebP rewrite requires exact RIFF extent and coherent primary/animation structure;
- mixed XMP is filtered at expanded-name field granularity;
- malformed XMP causes rewrite failure rather than metadata loss;
- VP8L and ANMF alpha contribute to VP8X alpha state;
- final WebP output is structurally revalidated;
- release remains manual.

Do not claim byte-identical XMP lexical formatting. The contract is semantic preservation of unrelated valid fields plus byte preservation of unrelated non-XMP chunks and image payloads.

### 7.2 Run bounded required verification

Required commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Also run focused commands matching actual test targets, for example:

```bash
cargo test --all-features --test jpeg_container_preservation
cargo test --all-features --test conformance_container_tests
cargo test --all-features --test cross_format_semantics
cargo test --all-features --test request_api
```

Do not add repeated clean builds, exhaustive feature powersets, or long fuzz runs as closure requirements.

### 7.3 Record current-head CI honestly

After the final implementation commit:

- record the exact final SHA;
- query CI for that SHA;
- record the workflow/run URL or identifier if available;
- if the connector or GitHub does not expose a run, record `UNAVAILABLE/NOT OBSERVED`;
- do not cite an earlier SHA as proof for the final head;
- a local `scripts/check.sh` pass remains valid local evidence but must be labeled local.

### 7.4 Reconcile planning status only after all criteria pass

Update statuses as follows only when the source and evidence support them:

```text
Roadmap 045 -> COMPLETE — final closure by Plan 052
Plan 051    -> PARTIAL — completed policy/basic containment work; residuals closed by Plan 052
Plan 052    -> COMPLETE
```

`plans/051-status.md` should remain historically truthful. Do not rewrite it to imply Plan 051 itself implemented the Plan 052 corrections. Add cross-references to the exact Plan 052 commits.

If any release-blocking criterion remains open:

```text
Roadmap 045 -> PARTIAL
Plan 052    -> PARTIAL
```

and list the exact open item. Test count alone is not closure evidence.

### 7.5 Publication hold

Confirm explicitly:

- no version bump;
- no crates.io publication;
- no tag;
- no GitHub release;
- no release workflow;
- no change from the current manual-release policy.

### Phase 7 acceptance criteria

- documentation matches exact supported and fail-closed behavior;
- all required local commands have exact observed results;
- current-head CI evidence is exact or honestly unavailable;
- Plan 051 overclaims are corrected rather than preserved;
- Plan 052 status rows map to real tests and commits;
- Roadmap 045 closes only if every remaining criterion is proven;
- no publication side effects occur.

Suggested commit:

```text
plans: close container correctness with truthful evidence
```

---

## Definition of done

Plan 052 is complete only when all of the following are true:

1. JPEG structural analysis returns exact entropy start and end offsets.
2. Coefficient decoding receives only the exact entropy slice.
3. The decoder proves all expected blocks were decoded.
4. Only legal all-one final pad bits may remain.
5. Extra complete entropy bytes are rejected.
6. Early entropy exhaustion and malformed stuffing are rejected.
7. EOI is proven structurally outside the entropy slice.
8. Invalid DHT classes are rejected.
9. Encoder and decoder Huffman state derive from one checked canonical representation.
10. Existing exact DC/AC SOS table-reference checks remain intact.
11. WebP rewrite requires declared RIFF extent to equal physical input length.
12. WebP chunk iteration ends exactly at the declared extent.
13. Odd-sized chunks require a contained physical pad byte.
14. VP8X payload length and reserved fields are validated.
15. VP8X without a primary image or coherent animation is rejected.
16. Duplicate or conflicting primary payloads are rejected.
17. Bounded animation structure is validated without adding frame steganography.
18. No XMP packet is skipped merely because it contains an owned property.
19. Owned XMP fields are removed by namespace URI/local name at attribute and element granularity.
20. Unrelated fields in mixed descriptions survive.
21. Unrelated PLUS fields survive.
22. Malformed XMP in any packet fails the rewrite before output.
23. Compatible differing packets merge into one XMP packet.
24. Namespace conflicts fail deterministically.
25. VP8L intrinsic alpha is detected.
26. ANMF frame alpha is detected.
27. Final VP8X bits exactly match final chunks and payload semantics.
28. A production final-output validator proves RIFF, structure, XMP count, and flags before return.
29. Unknown WebP chunks and image payloads remain byte-preserved on successful rewrite.
30. Supported JPEG APP/COM/unknown segments remain preserved.
31. Three replacement rounds do not duplicate owned XMP or grow without bound.
32. Completed Plan 051 policy behavior remains unchanged.
33. Focused tests prove every item above.
34. `cargo fmt`, clippy, workspace tests, and `scripts/check.sh` pass locally.
35. Current-head CI is recorded exactly or marked unavailable.
36. Roadmap and plan statuses are truthful.
37. Required CI remains one job.
38. No version, tag, release, or publication action occurs.

---

## Explicit non-goals

Do not use this plan to:

- add new image formats;
- add AVIF, TIFF, GIF, HEIF, video, or PDF support;
- implement progressive JPEG DCT embedding;
- implement restart-bearing JPEG DCT embedding;
- implement multi-scan JPEG transcoding;
- add arithmetic or lossless JPEG support;
- add animated WebP frame steganography;
- normalize or re-encode VP8/VP8L image payloads;
- implement a general RDF model;
- add C2PA;
- add TDMRep publication;
- redesign rights policies;
- change legacy policy defaults;
- replace the `image` crate;
- split the workspace;
- add fuzzing as a required gate;
- increase CI or release ceremony;
- publish a crate or create a release.

---

## Handoff guidance

The implementer should treat this as a closure pass, not an invitation to refactor adjacent systems.

Recommended commit sequence:

```text
1. plans: reopen container correctness closure
2. jpeg: enforce exact entropy span and exhaustion
3. jpeg: unify canonical Huffman construction
4. webp: enforce exact RIFF and payload structure
5. webp: preserve mixed XMP fields with checked parsing
6. webp: derive and validate final VP8X feature state
7. test: close container boundary regressions
8. plans: close container correctness with truthful evidence
```

Before each implementation commit:

- update the corresponding `plans/052-status.md` rows;
- run the smallest focused test that proves the change;
- avoid touching completed policy paths;
- keep malformed, unsupported, and capacity-limited outcomes distinct.

Final handoff must include:

- exact implementation SHAs;
- exact focused tests per defect;
- exact local command output summaries;
- exact current-head CI disposition;
- explicit publication hold;
- `PARTIAL` rather than `COMPLETE` if any definition-of-done item remains open.
