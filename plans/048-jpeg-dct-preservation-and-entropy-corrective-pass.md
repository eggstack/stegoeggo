# Plan 048: JPEG DCT Preservation and Entropy Corrective Pass

Status: Ready for implementation

Baseline: `main` after Plan 047 planning commit `73216c0e6be2f5bd97f052b0ea5f76ddd1b74ce8`

Depends on:

- `plans/045-corrective-correctness-closure-roadmap.md`

Corrects incomplete criteria from:

- `plans/040-jpeg-dct-container-correctness-and-containment.md`
- `plans/044-cross-format-correctness-closure.md`

Must complete before:

- `plans/050-post-corrective-evidence-and-documentation-closure.md`

---

## Purpose

Make the retained baseline JPEG DCT path honor its central containment contract: supported hidden-marker embedding may alter quantization tables and entropy-coded coefficients, but it must preserve unrelated original JPEG container segments and must reject malformed or unsupported entropy/scan structures rather than partially rewriting them.

The current code includes `encode_coefficients_preserving()`, but the successful DCT embedding path first canonicalizes through `encode_coefficients(..., None)` and returns another `None` assembly result. That assembly path drops APP2, APP13, APP14, DRI, and unknown segments. The preserving path is primarily reached on capacity downgrade, so successful embedding can be less container-safe than skipped embedding.

Additional retained decoder defects are:

- canonical Huffman codes are shifted only when the current bit length has symbols;
- missing DC or AC Huffman symbols can produce zero/partial blocks instead of an error;
- overrun AC runs can terminate a block instead of rejecting malformed entropy;
- header parsing stops at the first SOS and the capability probe does not prove there is only one scan;
- a supported classification can therefore be applied to sequential multi-scan input that the writer truncates after the replacement scan.

This plan fixes or contains those issues. It does not add progressive, restart-bearing, arithmetic-coded, lossless, or general multi-scan DCT embedding.

---

## Governing decisions

1. The supported DCT subset remains bounded to JPEG structures the implementation can prove correct.
2. Progressive JPEGs remain unsupported for DCT embedding.
3. Restart-bearing JPEGs remain unsupported for DCT embedding unless the implementer can correct the full boundary behavior with less complexity than explicit fallback. The expected outcome is continued fallback.
4. Multi-scan JPEGs remain unsupported unless full scan preservation/re-encoding is implemented. The expected outcome is explicit rejection/fallback.
5. Metadata-only JPEG processing must preserve original entropy scan bytes.
6. Successful DCT embedding may change DQT and entropy scan bytes, but unrelated APP/COM/DRI/unknown segments must survive byte-for-byte and in order.
7. Do not canonicalize through a lossy container assembler merely to make the project decoder accept its own output.
8. Malformed Huffman data must return an error or unsupported/fallback outcome; partial coefficient maps are not valid success.
9. Keep the current one-job CI policy.
10. Do not replace the entire JPEG implementation with a new codec dependency in this pass unless the bounded internal fix proves impossible and the status ledger records the tradeoff before dependency addition.

---

## Phase 0: Create the status ledger and capability table

Create `plans/048-status.md` before source edits.

Initialize it with:

```text
Plan baseline SHA: 73216c0e6be2f5bd97f052b0ea5f76ddd1b74ce8
Disposition: OPEN
Supported subset inventory: OPEN
Successful-path preservation: OPEN
Canonical Huffman construction: OPEN
Malformed symbol rejection: OPEN
Multi-scan detection: OPEN
Fallback reporting: OPEN
Fixtures: OPEN
Documentation: OPEN
CI: OPEN
Publication hold: no publication is part of this plan
```

Add a capability table:

```text
JPEG structure | DCT support target | safe fallback | warning/report | fixture | test | status
```

Required rows:

- baseline sequential grayscale single scan;
- baseline sequential 4:4:4 single scan;
- baseline sequential 4:2:2 single scan;
- baseline sequential 4:2:0 single scan;
- baseline custom Huffman single scan;
- progressive JPEG;
- restart interval/RST JPEG;
- sequential multiple scans;
- arithmetic coding;
- lossless coding;
- 12-bit precision;
- malformed/truncated Huffman symbol;
- malformed SOS table identifier;
- excessive sampling factor;
- APP2 ICC;
- APP13 Photoshop/IPTC;
- APP14 Adobe marker;
- unknown APP marker;
- COM marker.

Supported rows require positive embed/extract evidence. Unsupported rows require explicit capability/fallback evidence.

---

## Phase 1: Detect the complete scan structure before declaring support

Primary files:

```text
src/jpeg_transcoder/header.rs
src/jpeg_transcoder/mod.rs
```

### 1.1 Inventory markers beyond the first SOS

The capability probe must know whether the input contains exactly one sequential scan.

Add a bounded structural scan that walks:

- header marker segments before SOS;
- entropy-coded bytes with stuffed `0xFF 0x00` handling;
- restart markers if present;
- the marker terminating each scan;
- subsequent DHT/DQT/SOS segments;
- EOI.

The walker need not decode coefficients. Its purpose is to classify scan count and gross structure safely.

Acceptable representations include:

```rust
pub struct JpegStructure {
    pub scan_count: usize,
    pub has_restart_markers: bool,
    pub has_trailing_segments_after_scan: bool,
    pub eoi_offset: usize,
}
```

or smaller fields on `JpegHeader`.

Do not search for raw `0xFFDA` without respecting entropy stuffing and marker boundaries.

### 1.2 Reject multi-scan input from the DCT subset

`probe_dct_support()` must return:

```text
Unsupported(MultipleScans)
```

when scan count is not exactly one.

The processing pipeline must route that outcome through the existing explicit JPEG fallback/report path.

Do not label a sequential multi-scan JPEG as progressive unless it is actually progressive. Preserve the exact unsupported reason internally even if the current public warning remains generalized.

### 1.3 Require structural EOI

Supported DCT input must have a valid terminal EOI after the single scan. Truncated entropy data or a missing EOI must not be treated as a complete supported image.

### Phase 1 acceptance criteria

- capability detection counts scans correctly;
- marker-like bytes in entropy data do not create false scans;
- multi-scan sequential input is unsupported;
- missing EOI/truncated scan is not supported;
- progressive/restart decisions remain explicit;
- resource limits bound marker/segment walking;
- no full entropy decode is performed during the probe.

Suggested commit:

```text
jpeg: classify complete scan structure before DCT embedding
```

---

## Phase 2: Correct canonical Huffman decoder construction

Primary file:

```text
src/jpeg_transcoder/entropy.rs
```

### 2.1 Advance code length unconditionally

Canonical Huffman table construction must advance to the next bit length after every count slot, including zero-count lengths.

Required conceptual algorithm:

```rust
let mut code = 0;
let mut value_index = 0;
for length_index in 0..16 {
    let count = counts[length_index];
    if count > 0 {
        min_code[length_index] = code;
        max_code[length_index] = code + count - 1;
        val_offset[length_index] = value_index - code;
        code += count;
        value_index += count;
    } else {
        min_code[length_index] = -1;
        max_code[length_index] = -1;
    }
    code <<= 1;
}
```

Use checked arithmetic and validate impossible/oversubscribed code spaces rather than relying on integer wrap.

### 2.2 Validate table consistency

Before constructing decoder/encoder tables, reject:

- count sum greater than value count;
- unused trailing values if the current format requires exact equality;
- oversubscribed canonical code space;
- empty tables referenced by SOS;
- invalid table class or identifier.

Do not silently use table 0 when a referenced nonzero table is malformed. Existing fallback-to-table-0 behavior should be reviewed; retain it only where JPEG semantics and current compatibility require it. Record the decision.

### 2.3 Share canonical code generation where practical

The encoder and decoder currently construct canonical tables separately. Avoid two subtly different algorithms.

A small shared helper may generate:

```text
(symbol, code, bit_length)
```

entries used to populate both directions.

Do not introduce a generalized Huffman crate solely for this.

### Phase 2 acceptance criteria

- zero-count intermediate lengths produce correct later codes;
- standard tables still decode/encode;
- custom valid tables with empty intermediate lengths round-trip;
- oversubscribed or inconsistent tables are rejected;
- encoder and decoder use equivalent code assignments;
- no panic or out-of-bounds access occurs for malformed tables.

Suggested commit:

```text
jpeg: correct canonical Huffman table construction
```

---

## Phase 3: Fail closed on malformed entropy symbols

Primary file:

```text
src/jpeg_transcoder/entropy.rs
```

### 3.1 DC symbol is mandatory per block

A block cannot silently retain DC zero because `decode_symbol()` returned `None`.

Required behavior:

```rust
let size = dc_decoder
    .decode_symbol(&mut bit_reader)
    .ok_or_else(|| HuffmanDecode("missing or truncated DC symbol ..."))?;
```

Include sufficient context for debugging without exposing unbounded data:

```text
MCU index
component id
block index
byte/bit position where available
```

### 3.2 AC symbol failures are errors

`decode_symbol() == None` before a valid EOB/completed block must return an error.

Do not convert truncated entropy into an implicit EOB.

### 3.3 Validate AC run semantics

Reject:

- a nonzero-size symbol whose run moves beyond coefficient 63;
- invalid zero-size symbols other than EOB and ZRL;
- ZRL that advances beyond the block;
- unsupported magnitude sizes;
- unexpected markers inside a supported no-restart scan.

### 3.4 Verify expected block counts and scan exhaustion

After decoding the expected MCU/component/block count:

- verify the entropy reader is at a valid scan termination boundary;
- reject substantial unconsumed entropy data that indicates geometry or scan misunderstanding;
- allow legal pad bits before the terminating marker;
- do not accept an early EOI as a complete block stream.

### 3.5 Error propagation and fallback

Malformed entropy is not the same as an unsupported-but-valid JPEG class.

Choose a deterministic policy:

- metadata-only operation may still preserve and inject metadata if the outer image passes independent decode validation;
- hidden-marker DCT attempt reports a specific failure/downgrade rather than claiming unsupported progressive JPEG;
- invalid JPEG input must still fail rather than be rewritten.

Reuse existing warning/report variants where truthful. Add a narrow variant only if the current variant would materially misreport corruption.

### Phase 3 acceptance criteria

- missing DC symbol errors;
- missing AC symbol errors;
- AC run overflow errors;
- early EOI errors;
- malformed entropy never yields `Ok` with partial blocks;
- valid standard/custom fixtures still round-trip;
- fallback does not claim marker success.

Suggested commit:

```text
jpeg: reject partial and malformed entropy decoding
```

---

## Phase 4: Preserve the original container on successful DCT embedding

Primary files:

```text
src/protected/steganography.rs
src/jpeg_transcoder/mod.rs
```

### 4.1 Remove container-dropping canonicalization from the success path

Do not use this topology for normal supported input:

```text
original JPEG
  -> decode coefficients
  -> assemble canonical JPEG without original container
  -> decode again
  -> embed
  -> assemble without original container
```

Target:

```text
original JPEG
  -> parse/probe/decode original coefficients and original tables
  -> embed into coefficient copy
  -> encode using original Huffman/SOS structure
  -> reassemble with original-container preservation
  -> decode/extract verification of the preserved candidate
  -> return preserved candidate
```

### 4.2 Use preserving output for every successful attempt

Each candidate attempt used for round-trip verification must be encoded through:

```rust
encode_coefficients(..., Some(original_jpeg))
```

or an equivalent preserving API.

The final returned embedded output must be the verified preserving candidate, not a later canonical assembly.

### 4.3 Preserve intended versus unrelated changes

Allowed changes:

- DQT values used for seed storage;
- SOS entropy-coded scan bytes resulting from coefficient embedding;
- byte stuffing/pad bits within the newly encoded scan;
- StegoEggo-owned metadata added later by the metadata layer.

Must remain byte-identical and ordered relative to the original DCT operation:

- APP0;
- APP1 EXIF/XMP not owned by StegoEggo;
- APP2 ICC;
- APP13 Photoshop/IPTC;
- APP14 Adobe;
- COM;
- DRI on inputs that are not DCT-supported but pass through fallback;
- unknown APP markers;
- SOF and DHT segments for supported single-scan input;
- unknown non-scan segments.

DQT is intentionally replaced. SOS header should remain byte-identical for the supported subset.

### 4.4 Do not conflate metadata injection with DCT preservation tests

Test the DCT protector directly before metadata injection so segment differences are attributable.

Then add one pipeline-level test confirming rights metadata and preserved unrelated segments coexist.

### 4.5 Capacity downgrade

When the image lacks DCT capacity:

- preserve the original container;
- quantization-table seed behavior must match documented fallback semantics;
- report `SkippedCapacity` accurately;
- do not return a canonicalized container.

### Phase 4 acceptance criteria

- successful DCT output preserves unrelated original segments;
- attempt verification uses the preserving candidate;
- custom Huffman input remains custom after supported embedding;
- APP2/APP13/APP14/unknown APP/COM survive;
- output decodes independently;
- embedded payload verifies;
- capacity downgrade remains preserving;
- no `assemble_jpeg` call is reachable from the normal original-JPEG DCT success path.

Suggested commit:

```text
jpeg: preserve original container during successful DCT embedding
```

---

## Phase 5: Focused fixture and regression matrix

Primary tests:

```text
tests/jpeg_container_preservation.rs
tests/cross_format_closure.rs
tests/cross_format_semantics.rs
```

Add or correct small fixtures with documented provenance.

### 5.1 Supported positive fixtures

Required:

- grayscale baseline;
- 4:4:4 baseline;
- 4:2:2 baseline;
- 4:2:0 baseline;
- custom Huffman table containing at least one zero-count intermediate length;
- APP2 ICC plus APP13 plus APP14 plus COM plus unknown APP marker.

For each supported fixture:

```text
probe == Supported
embed outcome == Embedded when capacity is sufficient
payload verifies
image decodes
required unrelated segments preserved byte-for-byte
SOS header preserved
```

Do not require original scan bytes to remain identical after DCT embedding.

### 5.2 Unsupported valid fixtures

Required:

- progressive;
- restart-bearing;
- sequential multi-scan.

For each:

```text
probe gives exact unsupported reason
hidden marker is not reported successful
metadata path remains valid/preserving
output decodes
```

### 5.3 Malformed fixtures

Required:

- truncated DC code;
- truncated AC code;
- AC run overflow;
- missing EOI;
- inconsistent Huffman count/value table.

Assert error/fallback classification, no panic, and no partial success.

### 5.4 Metadata-only invariant

For supported and unsupported fixtures under metadata-only same-format processing:

- original SOS-through-EOI bytes remain byte-identical where the metadata inserter does not need to move them;
- unrelated segments remain present;
- only StegoEggo-owned metadata segments change.

### Phase 5 acceptance criteria

- fixtures cover every row in the capability table;
- tests distinguish metadata-only from DCT-changing operations;
- preservation checks compare segment payloads, not only decoder success;
- malformed cases fail closed;
- fixture set remains compact.

Suggested commit:

```text
tests: close JPEG DCT subset and preservation regressions
```

---

## Phase 6: Documentation correction

Inspect:

```text
README.md
architecture/jpeg-transcoder.md
architecture/protected-steganography.md
architecture/pipeline.md
AGENTS.md
SUPPORT.md
CHANGELOG.md
```

Document the exact supported subset:

```text
8-bit sequential DCT
single scan
supported sampling factors proven by fixtures
valid referenced Huffman/quantization tables
no restart interval
```

Document explicit fallback for:

```text
progressive
restart-bearing
multi-scan
arithmetic/lossless
malformed entropy
```

Do not claim “lossless JPEG transcoding” without clarifying:

- DCT coefficients are intentionally changed for the hidden marker;
- unrelated container metadata is preserved;
- pixels are not decoded/re-encoded on the supported path;
- unsupported structures use metadata/seed fallback rather than DCT embedding.

### Phase 6 acceptance criteria

- public support claims match the probe;
- successful-path container preservation is documented;
- unsupported structures are explicit;
- no claim implies arbitrary JPEG support;
- historical plan text is not rewritten as evidence.

Suggested commit:

```text
docs: define the bounded JPEG DCT support contract
```

---

## Required verification

Run at minimum:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --all-features --test jpeg_container_preservation
cargo test --all-features --test cross_format_closure
cargo test --all-features --test cross_format_semantics
cargo test --workspace --all-features
cargo check -p stegoeggo --no-default-features
./scripts/check.sh
```

A bounded decoder check with one available independent tool is encouraged and recorded in the status ledger. Do not add it to required CI.

---

## Definition of done

Plan 048 is complete only when:

1. DCT support requires exactly one valid sequential scan.
2. Progressive, restart-bearing, and multi-scan JPEGs downgrade explicitly.
3. Canonical Huffman code construction advances through zero-count lengths correctly.
4. Invalid Huffman tables are rejected.
5. Missing DC/AC symbols and run overflow return errors.
6. Malformed entropy never produces partial successful coefficients.
7. Successful DCT attempts are encoded and verified through the original-container preserving path.
8. Successful output preserves APP2, APP13, APP14, COM, unknown APP, and other unrelated segments.
9. Supported grayscale/4:4:4/4:2:2/4:2:0/custom-Huffman fixtures embed and verify.
10. Metadata-only same-format processing preserves scan bytes.
11. Independent decode succeeds for corrected positive outputs where a tool is available.
12. Documentation states the exact bounded subset.
13. `plans/048-status.md` records exact commits, commands, fixtures, and results.
14. `./scripts/check.sh` passes.
15. No release or publication action occurs.