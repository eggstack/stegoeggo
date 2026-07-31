# Plan 040: JPEG DCT and Container Correctness with Explicit Containment

Status: Ready for implementation

Baseline: `main` after Plan 039 planning commit `ac13a5caeba962a15b20f462f2a489c82b764352`

Depends on:

- `plans/038-rights-metadata-format-correctness-roadmap.md`
- `plans/039-plus-iptc-rights-metadata-correctness.md`

May proceed in parallel with:

- `plans/041-webp-container-xmp-exif-correctness.md`

Must be completed before:

- `plans/042-api-cli-contract-consolidation.md`
- `plans/044-cross-format-correctness-closure.md`

---

## Purpose

Make JPEG handling safe, truthful, and maintainable without turning StegoEggo into a complete JPEG codec.

The current custom JPEG path performs three distinct jobs:

1. parse enough of a JPEG to inspect dimensions and tables;
2. inject or replace rights metadata in the JPEG container;
3. decode, modify, and re-encode DCT coefficients for a hidden marker.

Those responsibilities currently share a reduced semantic `JpegHeader` representation. Reassembling a JPEG from that representation can drop opaque segments, change coding-process details, mishandle restart structure, and reorder coefficient blocks. The metadata-only product path should not inherit those risks.

This plan separates container-safe metadata rewriting from optional coefficient processing, fixes the bounded DCT subset if it remains, and routes unsupported inputs to an explicit metadata-only fallback.

---

## Governing constraint

Do not implement the complete JPEG standard.

The retained DCT path may support only a narrow subset such as:

```text
8-bit precision
sequential Huffman DCT
single scan
1-4 components with supported sampling factors
known/valid Huffman and quantization tables
no arithmetic coding
no lossless coding process
no unsupported multiple-scan structure
restart structure only if correctly decoded and reproduced
```

Everything else must be:

- processed metadata-only without touching scan data; or
- rejected before modification if safe metadata-only processing is not possible.

An explicit downgrade warning is an acceptable product outcome. Silent corruption or an inaccurate “lossless” claim is not.

---

## Required end state

### Metadata-only JPEG

Same-format metadata-only processing:

- copies the original scan data byte-for-byte;
- preserves unrelated APP, COM, DQT, DHT, DRI, SOF, and unknown segments unless a StegoEggo-owned metadata segment is intentionally replaced;
- does not decode or re-encode coefficients;
- does not change color interpretation or coding process;
- produces a JPEG accepted by the pinned Rust decoder and independent tooling.

### Hidden-marker JPEG

The DCT path, if retained in the normal build:

- is entered only for a documented supported subset;
- uses correct component block ordering;
- constructs and decodes canonical Huffman codes correctly;
- reports malformed/truncated entropy as an error;
- handles restart intervals correctly or declines DCT processing for restart-bearing images;
- preserves required opaque container information during reassembly;
- cannot claim success when it has fallen back to metadata-only behavior.

### Unsupported inputs

Unsupported JPEGs:

- remain byte-preserved except for intended metadata edits;
- return a structured warning such as `HiddenMarkerUnsupportedForInput` or a narrowly named existing warning;
- report `stego_attempted` and `stego_succeeded` accurately;
- are not passed through an optimistic partial decoder.

---

## Non-goals

Do not use this plan to:

- support arithmetic-coded JPEG;
- support lossless JPEG coding;
- implement full progressive coefficient editing;
- implement every legal multi-scan arrangement;
- add a generalized image editor;
- add perceptual quality scoring infrastructure;
- replace the `image` crate across the repository;
- add a second required CI job or OS matrix;
- expand hidden-marker guarantees;
- publish a release.

---

## Phase 0: Establish the JPEG decision ledger

Create `plans/040-status.md` before source edits.

Initialize it with:

```text
Plan baseline SHA: ac13a5caeba962a15b20f462f2a489c82b764352
Disposition: OPEN
Metadata-only segment preservation: OPEN
DCT supported subset: UNDECIDED
Block ordering: OPEN
Huffman decoding: OPEN
Truncation behavior: OPEN
Restart handling: OPEN
Opaque metadata preservation: OPEN
External fixtures: OPEN
Publication hold: no publication is part of this plan
```

Add these tables.

### Table A: JPEG capability matrix

```text
input class | metadata-only | DCT marker | required fallback | fixture | observed result | status
```

Required rows:

- grayscale baseline sequential;
- 4:4:4 baseline sequential;
- 4:2:2 baseline sequential;
- 4:2:0 baseline sequential;
- progressive;
- restart interval present;
- APP2 ICC profile;
- APP13 IPTC/Photoshop resources;
- APP14 Adobe marker;
- CMYK/YCCK;
- custom Huffman tables with empty code lengths;
- truncated entropy data;
- multiple scans or unsupported SOS structure;
- unknown APP segment.

### Table B: DCT decision

```text
decision | evidence | implementation cost | corruption risk | binary/maintenance cost | chosen | rationale
```

Required options:

- retain and correct a bounded custom DCT subset;
- feature-gate the DCT path and default to metadata-only;
- replace the DCT backend with a mature compatible dependency;
- disable DCT embedding while preserving extraction compatibility.

The plan does not require replacement. Choose the smallest safe path supported by evidence.

### Table C: preservation evidence

```text
fixture | segment inventory before | segment inventory after | scan digest before | scan digest after | expected changed segments | status
```

### Phase 0 acceptance criteria

- the ledger exists before edits;
- the supported subset is not assumed from current behavior;
- fallback behavior is specified for every unsupported class;
- the implementation agent records the chosen DCT strategy before broad refactoring;
- no full-codec scope is accepted implicitly.

Suggested commit:

```text
plans: establish JPEG correctness decision ledger
```

---

## Phase 1: Separate opaque container rewriting from coefficient processing

Primary paths:

```text
src/jpeg_transcoder/header.rs
src/jpeg_transcoder/mod.rs
src/protected/metadata_trap.rs
src/lib.rs
```

### 1.1 Introduce an ordered segment representation

The metadata-only path needs a parser that retains the exact ordered byte slices before SOS.

A suitable internal representation is:

```rust
struct JpegSegment<'a> {
    marker: u8,
    offset: usize,
    bytes: &'a [u8],
}

struct JpegContainer<'a> {
    soi: &'a [u8],
    pre_scan_segments: Vec<JpegSegment<'a>>,
    sos_header_and_scan: &'a [u8],
}
```

The exact structure may differ. The requirements are:

- checked offsets and lengths;
- original ordering retained;
- standalone markers handled explicitly;
- SOS and remaining scan stream preserved as opaque bytes for metadata-only operations;
- no marker-like byte search inside segment payloads;
- no semantic reconstruction of unrelated segments.

### 1.2 Replace only StegoEggo-owned metadata

`MetadataUpdatePolicy` must operate on payload ownership, not marker type alone.

Examples:

- preserve an unrelated APP1 EXIF segment;
- preserve an unrelated APP1 XMP packet unless merge policy intentionally updates it;
- preserve APP13 resources that do not contain StegoEggo-owned fields;
- preserve unrelated COM markers;
- remove/replace only StegoEggo-owned COM/XMP/IPTC data under `ReplaceStegoOwned`;
- fail on actual conflict under `FailOnConflict`;
- merge missing fields under `PreserveExisting`.

### 1.3 Metadata-only scan invariance

For same-format metadata-only output, hash the original SOS marker through EOI and require byte equality after processing.

If the implementation inserts metadata before SOS, the scan slice should remain identical.

### 1.4 Keep semantic header parsing separate

`JpegHeader` may remain for the DCT path, but the metadata-only path must not reassemble the file from it.

### Phase 1 acceptance criteria

- same-format metadata-only processing does not invoke coefficient decode/encode;
- ordered unknown segments survive byte-for-byte;
- APP2, APP13, APP14, DRI, EXIF, XMP, COM, and unknown APP fixtures have explicit preservation tests;
- the scan digest remains identical;
- malformed segment lengths fail safely;
- resource limits still bound segment count and size;
- no new general-purpose JPEG framework is introduced.

Suggested commit:

```text
jpeg: preserve opaque container segments for metadata-only writes
```

---

## Phase 2: Correct block storage and indexing

The decoder currently pushes blocks in MCU traversal order. The encoder must use the exact inverse ordering.

### 2.1 Define the invariant

For each component:

```text
component_blocks = concatenation of that component's blocks in MCU order
within each MCU: by from 0..v_sampling, then bx from 0..h_sampling
```

Use one shared helper for block index calculation.

Expected shape:

```rust
fn component_block_index(
    mcu_x: usize,
    mcu_y: usize,
    mcus_per_row: usize,
    h_sampling: usize,
    v_sampling: usize,
    bx: usize,
    by: usize,
) -> usize {
    (((mcu_y * mcus_per_row + mcu_x) * v_sampling + by) * h_sampling) + bx
}
```

The implementation may instead store a structured MCU representation. Do not retain separate formulas in decoder and encoder.

### 2.2 Edge MCUs

Do not skip legal padded blocks merely because their full 8×8 area extends beyond visible image dimensions. JPEG MCU structure includes padded edge blocks according to sampling and frame dimensions.

Validate the current skip logic against real fixtures. If it causes block-count asymmetry, correct it using component block dimensions derived from MCU geometry.

### 2.3 Tests

Required tests must use images larger than one MCU in both dimensions.

Minimum fixture matrix:

```text
grayscale 17x17 or larger
4:4:4, at least 2x2 MCUs
4:2:2, at least 2x2 MCUs
4:2:0, at least 2x2 MCUs
odd dimensions requiring padded edges
```

Tests should compare decoded coefficient maps before and after encode/decode when no stego mutation is applied.

### Phase 2 acceptance criteria

- decoder and encoder share one block-index invariant;
- 4:2:0 and 4:2:2 fixtures do not reorder blocks;
- edge MCU block counts are stable;
- no test relies only on a one-MCU image;
- coefficient no-op round trips are exact for the supported subset.

Suggested commit:

```text
jpeg: correct MCU component block ordering
```

---

## Phase 3: Correct Huffman and malformed-stream behavior

Primary path:

```text
src/jpeg_transcoder/entropy.rs
```

### 3.1 Canonical Huffman construction

Decoder code generation must advance to the next bit length even when the current length has zero symbols.

The decoder and encoder should derive from one validated table model where practical.

Reject:

- symbol count exceeding the values array;
- oversubscribed tables;
- invalid table class/id;
- impossible code lengths;
- SOS references to absent tables.

Do not clamp malformed SOS table IDs into range. Return an error.

### 3.2 Structured read errors

Replace `Option`-based entropy exhaustion on required data with a structured result that distinguishes:

```text
end of image at legal boundary
restart marker encountered
truncated bitstream
unexpected marker
invalid Huffman code
invalid magnitude length
```

Inner decode loops must not break and push partially initialized blocks after malformed input.

### 3.3 Coefficient range handling

Do not silently clamp coefficient differences merely to make the encoder continue if doing so changes image data unexpectedly.

For unsupported coefficient magnitude/table combinations, return a bounded error or route out of the DCT path before modification.

### 3.4 Tests

Required focused tests:

- a valid custom Huffman table with empty intermediate code lengths;
- an oversubscribed table;
- missing table reference;
- truncated DC magnitude;
- truncated AC magnitude;
- unknown marker in entropy data;
- EOI before all expected blocks;
- valid stuffed `0xFF 0x00` data.

### Phase 3 acceptance criteria

- decoder and encoder agree for valid custom tables;
- malformed SOS IDs are rejected, not clamped;
- truncation cannot produce `Ok` with partial zero-filled blocks;
- unexpected markers are explicit errors;
- supported fixture coefficient round trips remain exact;
- fuzz-visible parser entry points return errors rather than panic.

Suggested commit:

```text
jpeg: harden Huffman and entropy decoding semantics
```

---

## Phase 4: Handle restart intervals correctly or contain them

The implementation must choose one of two acceptable outcomes.

### Outcome A: full bounded restart support

If restart support is retained:

- parse and preserve DRI;
- track MCU count against `restart_interval`;
- byte-align at the restart boundary;
- consume and validate the expected RST sequence;
- reset DC predictors before decoding the next MCU;
- reset encoder predictors at the same interval;
- emit byte-aligned RST markers in the output;
- preserve the configured interval.

Do not detect restarts opportunistically in the middle of `read_bit()` and reset predictors after the following MCU.

### Outcome B: safe metadata-only fallback

If full bounded restart support is disproportionate:

- detect nonzero DRI or RST markers before coefficient modification;
- skip the DCT path;
- inject rights metadata through the opaque container path;
- return a specific warning;
- report hidden marker as attempted/unsupported or not attempted according to the resolved execution model;
- preserve the original scan bytes and restart structure exactly.

Outcome B is preferred over a partial restart implementation.

### Required tests

Use at least one independently generated restart-bearing baseline JPEG.

Verify:

```text
metadata-only output decodes
DRI remains present
RST sequence remains present
scan digest remains unchanged
DCT path either succeeds correctly or explicitly falls back
```

### Phase 4 acceptance criteria

- predictors are never reset after decoding the first post-restart MCU;
- restart-bearing input is not silently transcoded without restart preservation;
- fallback is visible in warnings/reporting;
- no corruption is accepted as best-effort behavior;
- the chosen outcome is recorded in the status ledger.

Suggested commit:

```text
jpeg: correct or contain restart-marker processing
```

---

## Phase 5: Define and enforce the supported DCT subset

### 5.1 Central capability probe

Add one function that decides whether DCT embedding is supported for the parsed input.

Example shape:

```rust
enum DctSupport {
    Supported,
    Unsupported(DctUnsupportedReason),
}
```

Possible reasons:

```text
Progressive
MultipleScans
ArithmeticCoding
LosslessCoding
UnsupportedPrecision
RestartIntervals
UnsupportedColorProcess
MissingTables
MalformedHeader
```

The exact enum may remain internal.

### 5.2 Color/process containment

APP14 Adobe transform and component layout can affect CMYK/YCCK interpretation. Unless the DCT path has explicit fixture coverage and preserves APP14, do not process those files through semantic reconstruction.

Metadata-only processing should still succeed and preserve APP14/ICC data.

### 5.3 Progressive and multiple scans

Progressive or unsupported multi-scan JPEGs must remain metadata-only. Do not claim DCT stego was applied based on a Q-table seed alone unless the execution report clearly identifies that weaker path.

### 5.4 Fallback semantics

The resolved plan may request a hidden marker, but the execution result must distinguish:

```text
requested and embedded
requested but unsupported, metadata written
requested but insufficient capacity, metadata written
requested and failed with fatal error
not requested
```

Do not return a generic success with `stego_succeeded=true` after metadata-only fallback.

### Phase 5 acceptance criteria

- one capability probe controls DCT entry;
- progressive, unsupported process, and color-sensitive fixtures avoid unsafe reconstruction;
- fallback preserves bytes outside intended metadata changes;
- execution reports and warnings are truthful;
- README/API docs list the supported subset accurately;
- no promise of arbitrary JPEG survival remains.

Suggested commit:

```text
jpeg: enforce bounded DCT support and explicit fallback
```

---

## Phase 6: Preserve container semantics in supported DCT output

If the DCT path remains enabled, do not assemble output solely from the reduced `JpegHeader` fields.

### 6.1 Reuse original segment stream

Preferred approach:

- retain the original pre-SOS segment byte sequence;
- update only DQT if the selected marker intentionally changes it;
- retain SOF/DHT/DRI/APP/COM ordering and bytes where unchanged;
- replace SOS scan payload with newly encoded scan bytes only for the supported single-scan subset;
- preserve EOI and trailing-policy behavior explicitly.

### 6.2 Metadata injection order

Apply StegoEggo metadata using the same ownership-aware container rewriter from Phase 1.

Do not let coefficient reassembly discard preexisting rights metadata before `MetadataUpdatePolicy` evaluates it.

### 6.3 “Lossless” terminology

Use “coefficient-preserving” only if coefficients not selected for stego remain exact and container semantics are preserved.

Use “lossless JPEG fast path” only if all documented preservation criteria are verified. Otherwise rename the path to a bounded DCT transcode path.

### Phase 6 acceptance criteria

- ICC, APP13, APP14, COM, EXIF/XMP, DRI, and unknown APP segments survive supported DCT processing where applicable;
- segment order is stable except for intentional StegoEggo metadata insertion/replacement;
- no unrelated metadata is dropped;
- scan replacement occurs only for the supported subset;
- documentation terminology is accurate.

Suggested commit:

```text
jpeg: preserve original segment stream during DCT embedding
```

---

## Phase 7: Fixtures and independent validation

### 7.1 Fixture provenance

Use a compact independently generated fixture set. Record generator/tool and options.

Suggested sources:

- ImageMagick/libjpeg-generated baseline subsampling variants;
- ExifTool-injected ICC/IPTC/XMP/unknown metadata;
- a restart-bearing fixture from libjpeg tooling;
- a progressive fixture;
- a CMYK fixture with APP14;
- a custom-Huffman fixture if available or generated by a focused test helper.

### 7.2 Required assertions

For metadata-only fixtures:

```text
scan bytes unchanged
unrelated segment bytes unchanged
StegoEggo-owned metadata updated according to policy
output externally decodes
```

For supported DCT fixtures:

```text
output externally decodes
image dimensions and component layout preserved
nonselected coefficients preserved
selected coefficient changes bounded to algorithm expectations
hidden payload verifies
unrelated segments preserved
```

For unsupported fixtures:

```text
metadata succeeds
hidden-marker fallback reported
scan bytes unchanged
```

### 7.3 External tools

Use the existing manual verification path. Suitable commands may include:

```bash
identify <file>
exiftool -G1 -a -s <file>
vipsheader -a <file>
```

Do not add an automatic external-tool matrix.

### Phase 7 acceptance criteria

- every capability-matrix row has a fixture or an explicit documented blocker;
- 4:2:0 block ordering is independently exercised;
- restart behavior is independently exercised;
- APP2/APP13/APP14/unknown segment preservation is observed;
- malformed fixtures return errors without panic;
- fixture set remains bounded.

Suggested commit:

```text
tests: add bounded JPEG interoperability fixtures
```

---

## Phase 8: Documentation and closure

Update as applicable:

```text
README.md
src/lib.rs
architecture/jpeg-transcoder.md
architecture/steganography.md
architecture/metadata.md
architecture/verification.md
CHANGELOG.md
STABILITY.md
AGENTS.md
```

Required statements:

- metadata-only JPEG is the safe general path;
- DCT embedding supports a bounded subset only;
- unsupported cases fall back or fail explicitly;
- progressive support is not implied by metadata seed preservation;
- restart behavior matches the chosen Outcome A or B;
- unrelated metadata preservation claims are scoped to tested behavior;
- no release has occurred.

### Phase 8 acceptance criteria

- docs match the capability probe;
- no stale “all JPEG” or unqualified “lossless” claim remains;
- status ledger records the chosen DCT strategy and rationale;
- `./scripts/check.sh` passes;
- existing manual external verification passes or the exact external blocker is recorded;
- required CI remains unchanged.

Suggested commit:

```text
docs: scope JPEG guarantees to verified behavior
```

---

## Required verification commands

At minimum:

```bash
cargo test -p stegoeggo jpeg_transcoder
cargo test -p stegoeggo jpeg
cargo test --test image_formats test_jpeg_round_trip
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Run focused external fixtures through the existing manual workflow or equivalent local commands once.

Do not create a new required job.

---

## Final acceptance criteria

Plan 040 is closed only when:

- metadata-only JPEG processing preserves original scan bytes;
- unrelated ordered segments survive;
- block indexing is correct for grayscale, 4:4:4, 4:2:2, and 4:2:0 supported fixtures;
- Huffman tables with empty intermediate lengths decode correctly;
- malformed/truncated entropy is a hard error;
- restart-bearing input is correctly supported or explicitly routed metadata-only;
- unsupported process/color/multi-scan inputs cannot enter unsafe reconstruction;
- DCT success/fallback is accurately reported;
- any retained DCT output preserves required opaque container data;
- documentation does not overstate losslessness or coverage;
- no full JPEG codec scope was introduced;
- no release or CI expansion occurred;
- `plans/040-status.md` contains exact evidence.

---

## Completion definition

The plan is complete when JPEG handling favors image preservation over hidden-marker maximalism. Metadata must work broadly and safely; DCT embedding may remain narrower, but every supported and unsupported case must be explicit.