# Plan 041: WebP Container, XMP, and EXIF Correctness

Status: Ready for implementation

Baseline: `main` after Plan 040 planning commit `4b58414964a1ddfb45b1cfa56c92ab7bd8667258`

Depends on:

- `plans/038-rights-metadata-format-correctness-roadmap.md`
- `plans/039-plus-iptc-rights-metadata-correctness.md`

May proceed in parallel with:

- `plans/040-jpeg-dct-container-correctness-and-containment.md`

Must be completed before:

- `plans/042-api-cli-contract-consolidation.md`
- `plans/044-cross-format-correctness-closure.md`

---

## Purpose

Replace the current append-only WebP metadata behavior with a checked RIFF/VP8X rewrite that produces externally valid WebP files, preserves unrelated chunks, and does not depend on StegoEggo-specific byte searching to validate malformed EXIF.

The existing writer can append `XMP ` and `EXIF` chunks to a simple `VP8 ` or `VP8L` file without creating a `VP8X` chunk. It can append duplicate metadata chunks and does not update VP8X feature bits. Its EXIF writer produces a TIFF structure that StegoEggo can search for a seed but independent EXIF readers may reject or misinterpret.

This plan corrects the container and metadata transport while preserving scope. It does not add lossy WebP encoding, animation editing, or a general RIFF framework.

---

## Required end state

For same-format WebP processing:

- input RIFF boundaries and chunk sizes are checked with overflow-safe arithmetic;
- simple WebP is converted to extended WebP when XMP or EXIF is added;
- a valid VP8X chunk is present and its feature flags reflect actual chunks;
- image dimensions encoded in VP8X match the underlying image;
- unknown chunks are preserved in order where the WebP specification permits;
- StegoEggo produces at most one effective XMP chunk and at most one effective EXIF chunk;
- existing non-StegoEggo metadata is preserved or merged according to `MetadataUpdatePolicy`;
- XMP uses the canonical rights semantics established by Plan 039;
- EXIF output is externally parseable, or EXIF seed redundancy is removed rather than emitted malformed;
- output opens in the pinned Rust decoder and at least one independent WebP implementation.

---

## Non-goals

Do not use this plan to:

- implement WebP animation editing;
- add lossy WebP encoding;
- optimize VP8/VP8L bitstreams;
- rewrite image payload chunks;
- build a generic RIFF crate;
- add arbitrary EXIF editing;
- add ICC color-management behavior beyond preserving existing chunks and flags;
- add required CI jobs or an external-tool matrix;
- publish a release.

---

## Phase 0: Establish a WebP capability ledger

Create `plans/041-status.md` before product edits.

Initialize it with:

```text
Plan baseline SHA: 4b58414964a1ddfb45b1cfa56c92ab7bd8667258
Disposition: OPEN
RIFF checked parsing: OPEN
Simple-to-extended conversion: OPEN
VP8X feature flags: OPEN
XMP replace/merge behavior: OPEN
EXIF decision: OPEN
Unknown chunk preservation: OPEN
External decoding: OPEN
Publication hold: no publication is part of this plan
```

Add these tables.

### Table A: input classes

```text
input class | current behavior | required behavior | fixture | output decoder | status
```

Required rows:

- simple lossy `VP8 `;
- simple lossless `VP8L`;
- extended `VP8X` without metadata;
- extended `VP8X` with existing XMP;
- extended `VP8X` with existing EXIF;
- existing ICC profile;
- alpha-bearing WebP;
- animated WebP;
- unknown ancillary chunk;
- malformed chunk size;
- truncated RIFF;
- duplicate XMP input;
- duplicate EXIF input.

### Table B: feature flags

```text
feature | VP8X bit | source of truth | preserved/set/cleared behavior | test | status
```

Include ICC, alpha, EXIF, XMP, animation, and any reserved-bit validation required by the implemented spec.

### Table C: EXIF decision

```text
option | correctness | dependency cost | binary cost | compatibility | chosen | rationale
```

Required options:

- implement a minimal standards-correct TIFF/EXIF UserComment writer;
- use an existing already-present capability if available;
- remove new EXIF seed emission and use XMP-only seed redundancy;
- add a new EXIF dependency only if the measured cost and maintenance benefit are justified.

The plan does not require EXIF emission. Correct XMP-only behavior is preferable to malformed EXIF.

### Phase 0 acceptance criteria

- the ledger exists before source changes;
- simple and extended WebP are distinguished;
- the EXIF choice is explicit before adding dependencies;
- animation is inventoried even if only preserved or rejected;
- no broad RIFF/EXIF expansion is accepted implicitly.

Suggested commit:

```text
plans: establish WebP correctness ledger
```

---

## Phase 1: Implement one checked RIFF/WebP parser

Primary paths:

```text
src/protected/metadata_trap.rs
src/protected/notice_verification.rs
src/resource_limits.rs
```

A small internal module may be introduced if it reduces duplicate parsing, for example:

```text
src/webp_container.rs
```

Do not expose it publicly unless a stable public API need already exists.

### 1.1 Validate RIFF envelope

Require:

```text
bytes[0..4] == "RIFF"
bytes[8..12] == "WEBP"
declared RIFF size + 8 is consistent with the accepted file policy
```

Decide and document whether trailing bytes are rejected or preserved. Do not silently ignore inconsistent lengths.

Use checked arithmetic for:

```text
chunk_start + 8
chunk_data_start + chunk_size
padding byte
next chunk position
output total size
```

### 1.2 Retain ordered chunks

Represent each chunk as an ordered opaque range plus FourCC and declared size.

The parser must not copy truncated chunks using `min(data.len())`; truncation is an error.

### 1.3 Validate mandatory image structure

Recognize enough structure to distinguish:

- simple `VP8 `;
- simple `VP8L`;
- extended `VP8X` with one image payload path;
- animated extended WebP.

Do not decode or rewrite VP8/VP8L payload bytes during metadata-only processing.

### 1.4 Resource limits

Retain or add bounded:

- RIFF chunk count;
- individual chunk size;
- XMP size;
- EXIF size;
- total copied metadata size.

Reuse `ResourceLimits`; do not create a parallel limit system.

### Phase 1 acceptance criteria

- writer and verifier use the same checked chunk iterator/parser;
- malformed/truncated chunk sizes return errors;
- odd-size padding is handled correctly;
- unknown chunks are retained as opaque bytes;
- image payload bytes are not decoded for same-format metadata-only work;
- current resource limits remain effective.

Suggested commit:

```text
webp: centralize checked RIFF chunk parsing
```

---

## Phase 2: Convert simple WebP to valid extended WebP

### 2.1 Create VP8X when metadata is added

When input contains `VP8 ` or `VP8L` without `VP8X`, metadata insertion requires a valid VP8X chunk.

The implementation must derive canvas dimensions from the underlying image header using a small format-specific parser or an existing reliable decoder path.

Do not derive dimensions by trusting caller-supplied output names or unrelated metadata.

### 2.2 Preserve image payload

The original `VP8 ` or `VP8L` chunk bytes must remain unchanged.

The conversion should be container-only:

```text
RIFF/WEBP
VP8X with correct flags and dimensions
preserved ancillary chunks in compliant order
original VP8 or VP8L payload
new/updated metadata chunks
```

Use the ordering required or recommended by the applicable WebP container specification.

### 2.3 Existing VP8X

For extended input:

- validate reserved bits;
- preserve existing nonmetadata feature bits;
- set XMP/EXIF flags when those chunks are present after rewrite;
- clear XMP/EXIF flags only when the corresponding metadata is intentionally removed;
- preserve animation and alpha flags based on actual structure;
- do not discard ICC signaling.

### 2.4 Dimensions

VP8X stores canvas dimensions minus one. Handle:

- zero/invalid dimensions;
- maximum representable dimensions;
- mismatch between VP8X and underlying payload;
- animated canvas dimensions.

For mismatch, prefer rejection or a documented canonical source rather than silently rewriting dimensions.

### Phase 2 acceptance criteria

- simple VP8 and VP8L fixtures become valid extended WebP after metadata insertion;
- original image payload chunk bytes are identical;
- VP8X dimensions are correct;
- feature flags match output chunks;
- reserved bits are not emitted;
- externally generated simple inputs open after conversion;
- animation is preserved without frame rewriting or rejected before mutation if safe preservation is not implemented.

Suggested commit:

```text
webp: create and maintain valid VP8X headers
```

---

## Phase 3: Replace or merge XMP instead of appending duplicates

### 3.1 Define ownership

A WebP `XMP ` chunk is not automatically StegoEggo-owned.

Ownership detection must inspect properties in the packet.

Under `ReplaceStegoOwned`:

- preserve unrelated XMP properties;
- remove/replace StegoEggo-owned rights properties;
- emit one resulting XMP packet.

Under `PreserveExisting`:

- retain existing values;
- add only missing requested fields;
- surface conflicts where required.

Under `FailOnConflict`:

- detect actual semantic conflict before writing;
- do not reject merely because an XMP chunk exists.

### 3.2 One effective XMP chunk

If the input has duplicate XMP chunks:

- parse within limits;
- establish a deterministic policy;
- prefer merging recognized properties into one packet;
- preserve unknown properties where practical;
- emit one XMP chunk after successful replacement.

If safe merge is not possible, return a specific metadata conflict/error rather than appending a third chunk.

### 3.3 Canonical rights semantics

Use the common XMP builder from Plan 039. Do not reimplement policy mapping in the WebP writer.

### 3.4 Verification

The verifier should scan the valid chunk structure and report duplicates/conflicts. It should not accept data beyond a malformed declared chunk boundary.

### Phase 3 acceptance criteria

- new output contains one effective XMP chunk;
- unrelated XMP fields survive replacement;
- canonical full PLUS URI is visible;
- `OtherConstraints` survives transport;
- update policies have distinct tested behavior;
- duplicate input does not cause blind append;
- limited and ordinary verification paths agree.

Suggested commit:

```text
webp: merge rights metadata into one XMP packet
```

---

## Phase 4: Correct or remove EXIF seed emission

### 4.1 Required TIFF/EXIF structure if retained

A retained minimal EXIF writer must produce a structurally valid TIFF payload, including:

- valid byte-order marker;
- TIFF magic;
- correct first-IFD offset;
- IFD entry count;
- correct 12-byte entries;
- correct type/count/value-or-offset handling;
- mandatory next-IFD offset;
- valid UserComment representation, including the character-code prefix if using EXIF UserComment;
- offsets relative to the correct TIFF base;
- padding/alignment as required.

Do not validate by searching raw bytes for `Protection seed:`.

### 4.2 Preferred simplification option

If EXIF seed redundancy has no external interoperability value and a correct writer would add disproportionate code/dependencies, stop emitting new EXIF seed chunks.

Store the seed in canonical/private XMP where it is already structurally valid. Continue parsing historical StegoEggo EXIF seed data for compatibility if practical.

This does not remove the primary feature. EXIF seed redundancy is an implementation detail, not the rights notice itself.

### 4.3 Existing EXIF preservation

If the input contains unrelated EXIF:

- preserve it byte-for-byte when not modifying it;
- do not replace the entire EXIF block merely to add a seed;
- if safe merge is unavailable, use XMP-only seed output rather than destroy camera metadata.

### 4.4 Feature flag consistency

Set the VP8X EXIF flag if and only if an EXIF chunk remains in output.

### Phase 4 acceptance criteria

One of the following is true:

**Retained EXIF path**

- ExifTool or another independent parser reads the emitted EXIF without structural warnings;
- the seed is extracted through parsed EXIF semantics, not byte search;
- existing unrelated EXIF is preserved or safely merged;
- one EXIF chunk is emitted.

**XMP-only path**

- no malformed EXIF is emitted;
- existing EXIF is preserved;
- the seed remains available through XMP where required;
- historical EXIF parsing compatibility is documented;
- VP8X flags match output.

Suggested commit:

```text
webp: correct or retire private EXIF seed emission
```

---

## Phase 5: Reassemble with compliant ordering and flags

### 5.1 Output assembly

Create one assembler that:

- writes `RIFF` and final size after all chunks are known;
- writes `WEBP`;
- emits one VP8X when output is extended;
- preserves ICC, animation, alpha, image payload, unknown, and metadata chunks in compliant order;
- pads odd chunk payloads with one byte not included in the declared chunk size;
- rejects total sizes beyond RIFF limits.

### 5.2 Duplicate image chunks

Reject malformed structures containing incompatible multiple primary image payloads unless the valid animation structure accounts for them.

### 5.3 Update-policy idempotence

Processing the same request twice under `ReplaceStegoOwned` should not grow the file by appending duplicate XMP/EXIF chunks.

### 5.4 Preserve animation safely

If metadata insertion into animated WebP can be performed by container-only rewriting without touching frame chunks, support it and test it.

If not, reject or preserve metadata-only input unchanged with a clear unsupported warning. Do not flatten animation or decode/re-encode frames.

### Phase 5 acceptance criteria

- RIFF size is exact;
- every chunk boundary and pad byte is correct;
- VP8X is first where required by the chosen compliant ordering;
- repeat processing is idempotent in chunk count;
- image/animation payload bytes remain unchanged;
- output is accepted by the pinned decoder and independent tooling.

Suggested commit:

```text
webp: reassemble compliant extended containers
```

---

## Phase 6: Focused fixtures and external validation

### 6.1 Fixtures

Use a bounded fixture set:

- simple lossy WebP;
- simple lossless WebP;
- extended alpha WebP;
- WebP with ICC;
- WebP with existing unrelated XMP;
- WebP with existing EXIF;
- animated WebP if supported/preserved;
- unknown ancillary chunk;
- duplicate XMP/EXIF malformed or compatibility input;
- truncated chunk.

Record source/generator and license/provenance.

### 6.2 Structural assertions

Implement a test helper that inventories chunks and VP8X flags independently from the production writer logic where practical.

Assert:

```text
one VP8X
expected feature bits
one effective XMP
zero or one EXIF according to chosen design
preserved payload digests
preserved unknown chunk digests
correct RIFF size
```

### 6.3 External tools

Use the existing manual external verification path. Suitable commands include:

```bash
webpmux -info <file>
webpinfo <file>
exiftool -G1 -a -s <file>
vipsheader -a <file>
identify <file>
```

Use available tools; do not require every tool if one or more provide sufficient independent evidence.

### Phase 6 acceptance criteria

- simple-to-extended conversion is externally recognized;
- XMP and constraints are externally visible;
- EXIF has no structural warnings or is intentionally not newly emitted;
- alpha/ICC/animation flags match structure;
- payload hashes remain stable;
- malformed files fail without panic;
- no new automatic workflow is added.

Suggested commit:

```text
tests: add WebP container interoperability fixtures
```

---

## Phase 7: Documentation and closure

Update as applicable:

```text
README.md
src/lib.rs
architecture/metadata.md
architecture/steganography.md
architecture/resource-limits.md
architecture/verification.md
CHANGELOG.md
STABILITY.md
AGENTS.md
```

Required statements:

- metadata insertion converts simple WebP to extended WebP while preserving image payload bytes;
- VP8X flags are maintained;
- XMP is merged/replaced according to update policy;
- EXIF behavior matches the chosen retained or XMP-only design;
- lossy WebP hidden-marker support remains out of scope if unchanged;
- animation support is explicitly described;
- no release has occurred.

### Phase 7 acceptance criteria

- docs match actual chunk behavior;
- no append-only or malformed EXIF assumptions remain;
- status ledger records the EXIF decision and measured dependency implications;
- `./scripts/check.sh` passes;
- external verification passes or exact tool/environment blocker is recorded;
- required CI remains unchanged.

Suggested commit:

```text
docs: describe standards-correct WebP metadata behavior
```

---

## Difficult implementation examples

### Example A: simple VP8L input

Before:

```text
RIFF WEBP
VP8L
```

After adding XMP:

```text
RIFF WEBP
VP8X  [XMP flag set, dimensions encoded]
VP8L  [byte-identical payload]
XMP   [canonical rights packet]
```

Forbidden:

```text
RIFF WEBP
VP8L
XMP
```

without a valid VP8X header.

### Example B: existing unrelated XMP

Input packet contains camera/workflow fields but no StegoEggo rights fields.

Expected under `ReplaceStegoOwned`:

```text
one output XMP packet
existing unrelated fields retained
canonical StegoEggo rights fields added
```

Forbidden:

```text
original XMP chunk
new StegoEggo XMP chunk appended
```

### Example C: existing EXIF camera metadata

If safe EXIF merge is not implemented:

```text
preserve existing EXIF byte-for-byte
store StegoEggo seed in XMP
set EXIF flag because original EXIF remains
set XMP flag because new XMP exists
```

Do not replace camera EXIF with a private seed-only EXIF block.

---

## Required verification commands

At minimum:

```bash
cargo test -p stegoeggo webp
cargo test -p stegoeggo metadata
cargo test -p stegoeggo notice_verification
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Run the bounded WebP fixtures through the existing manual external verification workflow or equivalent local commands once.

---

## Final acceptance criteria

Plan 041 is closed only when:

- checked RIFF parsing rejects malformed lengths;
- simple VP8 and VP8L become valid extended WebP when metadata is added;
- VP8X dimensions and flags are correct;
- image payload and unknown chunks are preserved;
- one effective XMP packet is emitted;
- update policies do not blindly append duplicates;
- canonical PLUS metadata and constraints are externally visible;
- newly emitted EXIF is externally valid or EXIF seed emission is removed;
- existing unrelated EXIF is preserved;
- repeat processing is idempotent in metadata chunk count;
- supported animation is preserved or unsupported animation is safely contained;
- no lossy WebP scope expansion occurred;
- no CI/release expansion occurred;
- `plans/041-status.md` contains exact evidence.

---

## Completion definition

The plan is complete when WebP output is valid according to the container structure understood by independent WebP tools, not merely readable by StegoEggo's own byte searches.