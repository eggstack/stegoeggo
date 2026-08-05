# Plan 051: Final Correctness and Evidence Closure Corrective Pass

Status: PARTIAL — remaining closure delegated to Plan 052

Audited baseline: `main` at `b3a08587861a17e9b290ba34fd82ca5e65575a92`

Reopens the premature completion claims in:

- `plans/045-corrective-correctness-closure-roadmap.md`
- `plans/050-post-corrective-evidence-and-documentation-closure.md`

Closes the remaining incomplete criteria from:

- `plans/046-rights-metadata-canonical-classification-corrective-pass.md`
- `plans/047-cli-default-policy-and-equivalence-corrective-pass.md`
- `plans/048-jpeg-dct-preservation-and-entropy-corrective-pass.md`
- `plans/049-webp-xmp-replacement-and-feature-flags-corrective-pass.md`
- `plans/050-post-corrective-evidence-and-documentation-closure.md`

---

## Purpose

Finish the narrow correctness work that remained after commits:

```text
b7e0d13 metadata canonical classification
a3ae07e CLI default-policy correction
3cc4300 JPEG preservation and entropy correction
76343cd WebP XMP/VP8X correction
b3a0858 attempted evidence closure
```

Those commits fixed the main audited regressions, but the repository still does not satisfy the exact closure contracts. The remaining defects are bounded:

1. Legacy `Light` policy defaults are inconsistent between CLI normalization and the retained library metadata resolver.
2. `ProhibitedSeeConstraints` without constraints is silently treated as complete rather than producing an explicit incomplete-rights warning or validation result.
3. Public source documentation still claims `tdm:reserve_tdm` is emitted even though current output intentionally does not emit it.
4. JPEG Huffman construction lacks complete table validation and still permits incorrect fallback to table 0 for missing referenced tables.
5. JPEG supported-input classification ignores trailing post-scan segments, and entropy decoding does not prove exact scan exhaustion.
6. WebP parsing walks physical input rather than the declared RIFF extent and does not reject duplicate/invalid structural chunks.
7. WebP XMP replacement can silently discard unrelated metadata from mixed descriptions, differing packets, or malformed packets.
8. VP8X feature bits are still partly inherited from input state rather than derived and validated from the final emitted container.
9. Required status ledgers and retrospective evidence files were not created; plan headers were marked complete without the evidence required by their own definitions of done.

This plan corrects those items only. It does not add formats, rights models, cryptography, services, release automation, or broader verification machinery.

---

## Governing constraints

1. Required CI remains one stable job invoking `scripts/check.sh`.
2. Do not add a CI matrix, scheduled workflow, publication workflow, release workflow, binary-size gate, mandatory external-tool job, or evidence-artifact pipeline.
3. Release remains manual. Do not bump versions, publish crates, create tags, or create GitHub releases.
4. Preserve `ProtectionRequest` as the sole canonical execution-intent model.
5. Do not restore a second CLI processing engine.
6. Preserve the bounded JPEG DCT subset. Do not implement progressive, restart-bearing, arithmetic-coded, lossless, or multi-scan DCT embedding.
7. Prefer rejecting a valid-but-unsupported JPEG structure over partially rewriting it.
8. Malformed JPEG input must not be relabeled as an unsupported progressive JPEG.
9. Do not add a general JPEG codec dependency.
10. Keep WebP rewriting container-level. Do not add animated-frame pixel steganography.
11. Do not silently drop malformed, mixed, or differing XMP metadata in order to produce output.
12. A small checked XML/XMP tokenizer is acceptable. A new XML dependency is permitted only if the status ledger first records why the bounded parser cannot meet the preservation contract with less complexity.
13. Tests must assert exact raw/container behavior. Project writer/parser agreement alone is not independent evidence.
14. Missing CI or external-tool evidence must be recorded as unavailable, not inferred as passing.
15. Historical timing must remain truthful: ledgers created now are retrospective and must say so.

---

## Required implementation order

Use this order so failures remain attributable:

```text
Phase 0  establish Plan 051 ledger and current baseline
Phase 1  unify legacy policy semantics and incomplete-constraints reporting
Phase 2  complete checked JPEG Huffman and scan-boundary containment
Phase 3  make WebP parsing and XMP replacement lossless or fail closed
Phase 4  derive and validate VP8X state from final chunk inventory
Phase 5  run bounded integration verification and repair documentation
Phase 6  create truthful retrospective ledgers and close planning state
```

Do not mark Plans 045, 050, or 051 complete before Phase 6.

---

## Phase 0: Establish the status ledger before source edits

Create:

```text
plans/051-status.md
```

before modifying product source.

Initialize it with:

```text
Plan baseline SHA: b3a08587861a17e9b290ba34fd82ca5e65575a92
Disposition: OPEN
Legacy Light-policy consistency: OPEN
Missing-constraints reporting: OPEN
TDM documentation correction: OPEN
JPEG canonical Huffman validation: OPEN
JPEG exact table references: OPEN
JPEG entropy exhaustion: OPEN
JPEG post-scan containment: OPEN
WebP declared-RIFF bounds: OPEN
WebP structural inventory: OPEN
WebP mixed/differing XMP preservation: OPEN
WebP malformed-XMP behavior: OPEN
VP8X final-inventory validation: OPEN
Targeted tests: OPEN
Workspace verification: OPEN
Retrospective ledgers: OPEN
CI evidence: OPEN
Publication hold: no publication is part of this plan
```

Add these tables.

### Table A: defect closure

```text
item | audited code path | intended contract | implementation commit | focused test | result | disposition
```

Required rows:

- Light policy mapping;
- missing constraints;
- stale TDM docs;
- Huffman count/value mismatch;
- Huffman oversubscription;
- duplicate Huffman symbols;
- missing referenced DC table;
- missing referenced AC table;
- entropy trailing bytes;
- invalid entropy pad bits;
- trailing post-scan marker;
- RIFF declared size smaller than physical input;
- duplicate VP8X;
- VP8X without primary image/animation payload;
- mixed owned/unrelated XMP description;
- two differing valid XMP packets;
- malformed plus valid XMP packets;
- final VP8X feature consistency.

### Table B: commands and evidence

```text
command/tool | environment/version | expected result | observed result | evidence location | status
```

### Table C: planning reconciliation

```text
plan | current header status | required missing ledger | source criteria still open | final disposition
```

Required rows: 039 through 051.

### Phase 0 acceptance criteria

- `plans/051-status.md` exists before product-source edits;
- the baseline SHA is exact;
- every audited residual defect has an open row;
- publication hold is explicit;
- no prior plan is represented as truthfully complete merely because its header currently says `COMPLETE`.

Suggested commit:

```text
plans: establish final correctness closure ledger
```

---

## Phase 1: Unify legacy policy semantics and report incomplete constraints

Primary files:

```text
src/types.rs
src/protected/metadata_trap.rs
src/request.rs or current request-resolution module
src/lib.rs
stegoeggo-cli/src/main.rs
stegoeggo-cli/tests/cli.rs
tests/cross_format_semantics.rs
```

### 1.1 Resolve the `Light` default once

Current behavior is inconsistent:

```text
CLI legacy_default_dmi(Light) -> Unspecified
RightsMetadataProtector::resolved_dmi(Light) -> Prohibited
```

Choose one compatibility mapping and use it everywhere legacy `ProtectionLevel` is translated into policy.

Decision procedure:

1. Inspect tests and public documentation that existed before Plan 042.
2. Inspect the actual pre-Plan-042 externally observable CLI and library behavior.
3. Prefer explicit historical public behavior over an incidental private fallback.
4. If historical public behavior is ambiguous, use `Unspecified` for `Light` because processing intensity must not silently create an all-data-mining legal restriction.
5. Record the chosen mapping and evidence in `plans/051-status.md`.

Required final mapping table must be explicit:

```text
Disabled -> no rights metadata unless caller explicitly requests it; effective policy Unspecified
Light    -> one recorded compatibility policy used by CLI and library
Standard -> ProhibitedAiMlTraining for legacy default compatibility
```

Do not retain two helpers with independently encoded mappings.

Acceptable implementation shapes:

- one library helper used by both the legacy library adapter and CLI;
- one canonical `ProtectionLevel` compatibility method consumed by CLI and pipeline;
- removal of the private level fallback from the writer so all writers receive already-resolved policy.

Preferred architecture: resolve policy before entering format writers. Writers should serialize the resolved request, not infer legal policy from processing level.

### 1.2 Preserve explicit policy precedence

Required precedence:

```text
explicit canonical RightsPolicy
explicit legacy --dmi value
explicit shorthand after conflict validation
legacy level default
Unspecified
```

Explicit `Unspecified` must remain explicit and must never be replaced by level fallback.

Add direct tests for:

- CLI `--level light`;
- legacy library `ProtectionLevel::Light` byte API;
- canonical request with `RightsPolicy::Unspecified` plus hidden marker;
- metadata-only request with explicit policy;
- Standard compatibility default.

The CLI and library tests must assert the same extracted policy, not merely the same file validity.

### 1.3 Define `ProhibitedSeeConstraints` without constraints

Use this deterministic behavior:

- do not invent legal text;
- emit the canonical `ProhibitedSeeConstraints` URI if the caller selected it;
- emit `plus:OtherConstraints` only when caller-provided constraints exist;
- produce an explicit execution warning that the policy is incomplete without constraints;
- classify the warning as error-severity for strict/complete-validation behavior, while non-strict processing may still produce the accurately incomplete output;
- CLI strict mode must exit through the existing strict-warning path;
- dry-run must show the same warning/validation result as execution where dry-run reports validation;
- verification of an existing file may report the DMI value, but must not imply that the companion constraint was found when it was absent.

Reuse an existing precise warning if one exists. Otherwise add one narrowly scoped variant, for example:

```rust
ProtectionWarning::MissingRightsConstraints
```

Do not encode the condition only as a human string that tests cannot identify.

Required tests:

```text
ProhibitedSeeConstraints + constraints -> no missing-constraints warning; OtherConstraints present
ProhibitedSeeConstraints + no constraints, non-strict -> output succeeds; warning present; no invented text
ProhibitedSeeConstraints + no constraints, strict -> strict failure; no misleading complete result
all other policies + no constraints -> no missing-constraints warning
```

Run the raw XMP assertions for PNG, JPEG, and WebP.

### 1.4 Remove stale TDM emission claims

Correct the `DmiValue` documentation and any current source/docs still claiming that image output includes:

```text
tdm:reserve_tdm
```

Required truth:

- current output does not emit TDM reservation metadata;
- historical TDM fields may remain readable only where compatibility parsing exists;
- TDMRep deployment is not implemented by image metadata alone.

Search at minimum:

```bash
rg -n "reserve_tdm|tdm:|TDM Reservation|TDMRep" src README.md AGENTS.md architecture STABILITY.md DEPRECATIONS.md CHANGELOG.md
```

### Phase 1 acceptance criteria

- one compatibility mapping owns `ProtectionLevel` defaults;
- CLI and library Light behavior are identical and recorded;
- explicit `Unspecified` survives every adapter;
- Standard legacy default remains correct;
- missing constraints produce a structured warning;
- strict and non-strict outcomes are tested;
- no constraints text is invented;
- current documentation no longer claims TDM image emission;
- no second execution path is introduced.

Suggested commits:

```text
policy: unify legacy level rights defaults
metadata: report incomplete see-constraints policy

docs: remove stale TDM image-emission claims
```

---

## Phase 2: Complete checked JPEG Huffman and scan-boundary containment

Primary files:

```text
src/jpeg_transcoder/header.rs
src/jpeg_transcoder/entropy.rs
src/jpeg_transcoder/mod.rs
src/protected/steganography.rs
tests/jpeg_container_preservation.rs
```

### 2.1 Build one checked canonical Huffman representation

Replace unchecked independent encoder/decoder construction with one checked helper that returns canonical entries.

Preferred internal representation:

```rust
struct CanonicalHuffmanEntry {
    symbol: u8,
    code: u16,
    bit_len: u8,
}

fn build_canonical_huffman_entries(
    counts: &[u16; 16],
    values: &[u8],
) -> Result<Vec<CanonicalHuffmanEntry>>
```

The helper must validate before returning entries:

1. `sum(counts) == values.len()`.
2. The table is nonempty when referenced by SOS.
3. The number of values does not exceed 256.
4. Symbols are unique within a table.
5. Code-space construction is not oversubscribed.
6. Arithmetic is checked; no `wrapping_add` or silent truncation.
7. Every code fits its declared bit length.
8. No value index can exceed `values.len()`.

A suitable oversubscription check is the canonical remaining-code-space method:

```text
available = 1
for each length:
    available *= 2
    available -= count[length]
    reject if available < 0
```

Use an integer type large enough for 16 levels and checked operations.

The encoder table and decoder min/max arrays must both derive from the returned entries. Do not maintain subtly different algorithms.

### 2.2 Require exact SOS table references

Remove fallback behavior equivalent to:

```text
requested table N missing -> use table 0
```

For every scan component:

- the exact referenced DC table must exist;
- the exact referenced AC table must exist;
- the table must be structurally valid;
- missing or invalid references produce `MissingTables` or a precise malformed-header error before coefficient decoding.

Do not reinterpret malformed table assignment as a supported standard-table JPEG.

Required fixtures:

- component references DC table 1 but only table 0 exists;
- component references AC table 1 but only table 0 exists;
- empty referenced table;
- count sum shorter than value vector;
- count sum longer than value vector;
- duplicate symbols;
- oversubscribed counts;
- valid custom table with zero-count intermediate lengths.

Each malformed fixture must fail deterministically without panic or partial output.

### 2.3 Bound the entropy slice exactly

Do not pass the complete remainder of the JPEG, including markers and trailing bytes, as an undifferentiated entropy buffer.

Extend the structural scan analysis to return exact offsets for the supported single scan:

```rust
scan_entropy_start
scan_entropy_end
terminating_marker_offset
```

For the supported subset:

- `scan_entropy_start` begins immediately after the SOS header;
- `scan_entropy_end` stops before the first non-stuffed, non-restart marker;
- no restart marker is allowed;
- the terminating structure must be the supported EOI shape;
- marker-like stuffed bytes remain part of entropy.

Decode only the entropy slice.

### 2.4 Verify expected scan exhaustion

After decoding the expected MCU/component/block count:

- permit at most the legal final pad bits needed to reach a byte boundary;
- require legal JPEG pad-bit values rather than arbitrary remaining bits;
- require no additional entropy bytes after the pad bits;
- reject early exhaustion;
- reject extra complete entropy bytes;
- reject unexpected markers within the supported entropy slice.

Expose a checked reader-finalization method rather than inspecting private positions ad hoc in tests.

Example conceptual API:

```rust
bit_reader.finish_scan()?;
```

Required negative tests:

- one extra entropy byte before EOI;
- invalid zero pad bits where all-one padding is required;
- early EOI before the final block;
- truncated stuffed byte;
- unexpected marker before expected blocks finish.

### 2.5 Contain post-scan segments

`JpegStructure::has_trailing_segments_after_scan` is currently computed but ignored.

Use the narrow safe policy:

```text
single scan + any marker segment after entropy before EOI -> unsupported for DCT embedding
```

Add a specific reason:

```rust
DctUnsupportedReason::TrailingSegmentsAfterScan
```

or equivalent.

The fallback must preserve the complete original container. It must not return a DCT-success result and must not discard the trailing segment.

If the current Q-table seed fallback can preserve every segment while replacing only DQT, it may remain. Otherwise use metadata-only fallback for this structure and report the hidden-marker downgrade accurately.

Required fixture:

```text
SOI ... SOS entropy ... COM or DNL segment ... EOI
```

Assert:

- DCT capability is unsupported with the exact reason;
- fallback output retains the trailing segment byte-for-byte;
- output decodes where the fixture is externally valid;
- report does not claim DCT payload success.

### 2.6 Distinguish malformed input from valid unsupported input

Do not catch every transcoder error and report it as progressive/unsupported JPEG.

Required classification:

```text
valid unsupported class -> explicit capability downgrade
malformed header/table/entropy -> processing error, or metadata-only only after an independent decoder proves the outer image is valid and the warning says malformed DCT path
capacity shortage -> SkippedCapacity
```

Do not mutate quantization tables on malformed input merely because coefficient decoding failed.

### Phase 2 acceptance criteria

- one checked canonical builder serves encoder and decoder;
- no wrapping arithmetic remains in Huffman construction;
- inconsistent, duplicate, empty, and oversubscribed tables fail;
- exact referenced tables are required;
- valid custom tables still round-trip;
- entropy is decoded from exact structural bounds;
- extra bytes and invalid padding fail;
- trailing post-scan segments cannot enter the DCT-supported path;
- fallback preserves the original container and reports the actual reason;
- malformed and unsupported cases are not conflated;
- no new JPEG dependency is added.

Suggested commits:

```text
jpeg: validate canonical Huffman tables and exact references
jpeg: enforce entropy exhaustion and post-scan containment
```

---

## Phase 3: Make WebP parsing and XMP replacement lossless or fail closed

Primary files:

```text
src/webp_container.rs
src/protected/metadata_trap.rs
tests/conformance_container_tests.rs
```

### 3.1 Treat the declared RIFF extent as authoritative

Compute:

```rust
let declared_end = 8usize.checked_add(riff_size)?;
```

Required rewrite-input contract:

- `declared_end == data.len()`;
- chunk iteration stops at `declared_end`, never physical `data.len()` independently;
- every chunk header, payload, and required pad byte fits inside `declared_end`;
- final cursor equals `declared_end` exactly;
- unexplained trailing physical bytes are rejected for rewrite operations;
- undersized and oversized declared lengths are both errors;
- arithmetic uses checked operations.

Required tests:

- declared size larger than physical input;
- declared size smaller than physical input with valid-looking trailing chunk bytes;
- chunk payload crosses declared end but not physical end;
- odd chunk missing its pad byte;
- final cursor stops before declared end;
- size arithmetic overflow fixture where constructible.

### 3.2 Inventory structural chunks exactly

Extend parsed state to enumerate indices/counts, not only booleans:

```text
vp8x_indices
vp8_indices
vp8l_indices
alph_indices
anim_indices
anmf_indices
xmp_indices
exif_indices
icc_indices
```

Validate for rewrite:

- at most one VP8X;
- VP8X data length is exactly the supported structural length;
- VP8X alone is not a primary image;
- a non-animated extended image has one valid VP8 or VP8L payload structure;
- an animated container has coherent ANIM/ANMF structure according to the bounded support contract;
- duplicate primary image chunks are rejected;
- reserved VP8X bits/bytes are rejected or normalized according to one documented rule;
- duplicate metadata chunks remain enumerable for policy resolution.

Do not set `image_kind = ExtendedVP8X` merely because VP8X was encountered without verifying a real image/animation payload.

### 3.3 Build a final chunk plan before writing

Do not compute flags while iterating and copying input chunks.

Create a bounded final representation such as:

```rust
struct PlannedChunk {
    fourcc: [u8; 4],
    data: Cow<'a, [u8]>,
}
```

Flow:

```text
parse and validate input
resolve metadata policy
construct zero or one final XMP packet
construct ordered final chunk vector
validate chunk vector
derive VP8X fields from final vector
write RIFF once
```

Required invariant before write:

```text
VP8X count == 1 for extended output
XMP count <= 1
all copied payload/unknown chunks retain exact bytes
```

### 3.4 Preserve mixed owned/unrelated XMP descriptions

The current merge drops an entire `rdf:Description` when the description contains any owned property. Replace that behavior with checked field-level handling.

Use a bounded tokenizer/parser scoped to the packet shapes the writer supports. It must understand:

- start tags and quoted attributes;
- self-closing and paired `rdf:Description` elements;
- namespace declarations;
- exact qualified names;
- child-element boundaries with nesting depth;
- XML comments/processing instructions sufficiently to reject unsupported malformed structures safely.

Do not use unconstrained string replacement.

Define the exact owned QName set in one constant/table. Include current StegoEggo fields and standard fields that the metadata policy intentionally owns. The table must be used by both merge logic and tests.

For each existing description:

- preserve non-owned attributes exactly in value and namespace meaning;
- remove or retain owned attributes according to metadata policy;
- preserve non-owned child elements as bounded raw slices or equivalent parsed nodes;
- remove or retain owned child elements according to metadata policy;
- preserve required namespace declarations;
- reject malformed/ambiguous content before writing rather than dropping it.

### 3.5 Merge all existing XMP packets deterministically

Do not select the first convenient packet and discard the rest.

Required behavior:

#### Identical packets

- deduplicate;
- apply the requested policy once;
- emit one packet.

#### Differing valid packets

- collect preserved non-owned descriptions/properties from every packet;
- deduplicate exact duplicate preserved descriptions;
- merge compatible namespace declarations;
- if the same prefix maps to different namespace URIs and cannot be safely rewritten, fail with a clear metadata conflict;
- apply owned-property policy once;
- emit one packet.

#### Mixed owned/unrelated packet

- preserve unrelated fields from the same description;
- replace/preserve owned fields according to policy;
- emit one packet.

#### Malformed packet

- fail before output under rewrite operations;
- do not emit a generated packet while silently dropping malformed original XMP;
- do not copy malformed plus generated packets as duplicates.

This is a correctness pass, so explicit failure is preferable to silent metadata loss.

### 3.6 Preserve metadata policy semantics

`ReplaceStegoOwned`:

- remove prior owned values from all packets;
- preserve every parseable non-owned value;
- add requested owned values once.

`PreserveExisting`:

- retain existing owned values;
- add only requested owned values that are absent;
- conflicting duplicate existing owned values produce an explicit conflict rather than arbitrary first/last selection.

`FailOnConflict`:

- compare semantic owned values before constructing output;
- fail before writing on conflict;
- unrelated metadata alone does not conflict.

### Phase 3 acceptance criteria

- parsing never crosses declared RIFF extent;
- trailing physical data is rejected for rewrite;
- duplicate/invalid VP8X and missing primary payload are rejected;
- one final chunk plan exists before output;
- output has at most one XMP chunk;
- unrelated fields survive when mixed with owned fields;
- all valid differing XMP packets contribute their unrelated fields;
- malformed XMP causes explicit failure, not silent loss;
- all three metadata update policies have exact tests;
- no general XML dependency is added without a recorded decision.

Suggested commits:

```text
webp: enforce declared RIFF bounds and structural inventory
webp: preserve mixed and multi-packet XMP without silent loss
```

---

## Phase 4: Derive and validate VP8X state from final output

Primary files:

```text
src/webp_container.rs
src/protected/metadata_trap.rs
tests/conformance_container_tests.rs
```

### 4.1 Final chunks are the source of truth

After the final chunk plan is complete, derive:

```text
ICC       <- final ICCP chunk presence
EXIF      <- final EXIF chunk presence
XMP       <- final XMP chunk presence
Animation <- final ANIM/ANMF structure
Alpha     <- final ALPH chunk or verified alpha-bearing primary/frame payload
```

Do not retain a feature solely because the input VP8X bit was set.

For VP8L intrinsic alpha, use one bounded reliable method:

- parse the relevant VP8L header bit correctly; or
- use the existing decoder to determine alpha capability for the final payload.

Record the chosen method and fixture coverage in the ledger.

### 4.2 Add a final-container validator

Before returning rewritten bytes, validate:

```text
one VP8X and it is first
XMP bit iff one XMP chunk exists
EXIF bit iff EXIF chunk exists
ICC bit iff ICCP chunk exists
animation bit iff coherent animation chunks exist
alpha bit matches final payload semantics
canvas dimensions are nonzero and match payload/container semantics
RIFF declared size equals physical output length minus 8
all chunks fit and are correctly padded
```

The validator may be internal and lightweight. Do not create a second parser; reuse the checked parser over final bytes where practical.

### 4.3 Exact regression tests

Add focused fixtures for:

- stale input XMP bit with no XMP chunk;
- stale input EXIF bit with no EXIF chunk;
- EXIF chunk with input bit clear;
- ICCP chunk with input bit clear;
- ALPH chunk with input bit clear;
- VP8L with and without intrinsic alpha;
- animation chunks with stale animation bit;
- output after mixed-XMP replacement;
- output after three replacement rounds.

Assert both chunk inventory and VP8X bits.

### Phase 4 acceptance criteria

- no feature bit is copied solely from stale input state;
- final inventory and bits agree bidirectionally;
- dimensions are valid;
- RIFF length and padding are exact;
- repeated replacement remains bounded and deterministic aside from documented timestamps;
- outputs decode through the existing independent image decoder used by tests.

Suggested commit:

```text
webp: derive and validate VP8X from final container state
```

---

## Phase 5: Bounded integration verification and documentation truth

Do not add a new permanent test apparatus. Reuse focused tests and `scripts/check.sh`.

### 5.1 Required targeted tests

Run at minimum:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p stegoeggo-cli --test cli
cargo test --workspace --all-features --test cross_format_semantics
cargo test --workspace --all-features --test jpeg_container_preservation
cargo test --workspace --all-features --test conformance_container_tests
cargo test --workspace --all-features
cargo check -p stegoeggo --no-default-features
./scripts/check.sh
```

Adjust package/test invocation only to match actual workspace command syntax. Record exact commands and results.

### 5.2 Cross-path policy matrix

For `Disabled`, `Light`, and `Standard`, compare:

```text
legacy CLI
legacy library byte API
canonical request byte API
dry-run resolved request
executed output verification
```

Assert exact policy and channel semantics.

For `ProhibitedSeeConstraints`, compare strict/non-strict behavior and raw XMP across PNG, JPEG, and WebP.

### 5.3 JPEG closure matrix

Required positive cases:

- standard valid tables;
- valid custom tables with empty intermediate code lengths;
- supported single-scan JPEG with APP2/APP13/APP14/COM/unknown APP preservation.

Required negative/contained cases:

- missing exact table reference;
- oversubscribed table;
- count/value mismatch;
- duplicate symbols;
- extra entropy byte;
- invalid pad bits;
- early EOI;
- trailing post-scan segment;
- progressive/restart/multi-scan unsupported classes.

### 5.4 WebP closure matrix

Required cases:

- declared RIFF end exact;
- trailing bytes rejected;
- one valid unrelated packet;
- mixed owned/unrelated description;
- two differing valid packets;
- malformed plus valid packet;
- identical duplicates;
- EXIF/ICC/alpha/animation flag correction;
- three-round idempotence.

### 5.5 Independent tools

Where already available locally, run a compact external pass using a subset of:

```text
ExifTool
webpinfo or webpmux
ImageMagick, libvips, or another decoder
xmllint
```

Do not install these into CI. Record unavailable tools honestly.

At minimum, the existing third-party image decoder in the Rust dependency graph must open representative final JPEG and WebP outputs. Project metadata extraction does not count as independent metadata observation.

### 5.6 Documentation truth search

Run:

```bash
rg -n "COMPLETE|reserve_tdm|tdm:|Light|ProhibitedSeeConstraints|Huffman|table 0|trailing segment|RIFF|VP8X|XMP|lossless|preserve" \
  README.md AGENTS.md STABILITY.md DEPRECATIONS.md CHANGELOG.md architecture src plans
```

Correct current product documentation and planning headers so they describe:

- the selected Light compatibility mapping;
- incomplete-constraints warning behavior;
- strict JPEG table/scan subset;
- trailing-segment fallback;
- WebP declared-bound parsing;
- lossless-or-error XMP merge contract;
- final-inventory VP8X flags;
- no TDM image emission;
- manual release and unchanged CI.

### Phase 5 acceptance criteria

- targeted tests fail under the audited baseline and pass after correction;
- full workspace verification passes;
- minimal library build passes;
- representative outputs decode independently;
- external-tool availability is recorded accurately;
- documentation contains no stronger claim than evidence supports;
- no CI or release expansion occurs.

Suggested commit:

```text
tests: close final policy JPEG and WebP correctness gaps

docs: align correctness scope and limitations with evidence
```

---

## Phase 6: Truthful planning and evidence closure

This phase is required. Do not replace it by changing plan headers only.

### 6.1 Create missing corrective ledgers retrospectively

Create:

```text
plans/046-status.md
plans/047-status.md
plans/048-status.md
plans/049-status.md
plans/050-status.md
```

Each must state prominently:

```text
This ledger was created retrospectively after the original source changes.
The plan's required pre-edit Phase 0 ledger did not exist at implementation time.
```

Record:

- plan baseline;
- actual implementation commits;
- audit findings remaining after those commits;
- Plan 051 corrective commits;
- exact tests and commands;
- unavailable evidence;
- final disposition.

Do not mark Plan 048 or 049 complete unless every Plan 051 criterion owned by that format is closed.

### 6.2 Create missing historical ledgers

Create concise retrospective ledgers:

```text
plans/039-status.md
plans/040-status.md
plans/041-status.md
plans/042-status.md
plans/043-status.md
plans/044-status.md
```

Each should include:

```text
Retrospective ledger creation date
Original claimed disposition
Original implementation commits
Evidence originally missing
Subsequent audit findings
Corrective commits/plans
Final evidence
Final disposition
Publication hold/history
```

Do not imply these existed before implementation.

### 6.3 Complete `plans/051-status.md`

The final ledger must include:

- every implementation commit SHA;
- every focused test name;
- exact command outputs summarized accurately;
- CI status if retrievable;
- explicit statement when GitHub status evidence is absent;
- external-tool evidence or unavailability;
- remaining limitations;
- confirmation that no version, tag, release, or publication was created.

### 6.4 Correct plan headers only after evidence exists

Required final header logic:

- Plan 045: `COMPLETE` only if Plan 051 is complete; otherwise `PARTIAL — OPEN ITEMS IN PLAN 051`.
- Plan 050: `SUPERSEDED BY PLAN 051` or `PARTIAL — CLOSED BY PLAN 051`, with a note that its required ledgers were absent at its original completion commit.
- Plan 051: `COMPLETE` only after its status ledger contains evidence for every blocking criterion.

Do not use test count as closure evidence.

### Phase 6 acceptance criteria

- all required ledgers exist;
- retrospective timing is truthful;
- commits and tests are named exactly;
- missing GitHub/external evidence is not reported as passing;
- plan headers match ledger dispositions;
- no required defect is relabeled as an accepted limitation;
- publication hold is confirmed.

Suggested commits:

```text
plans: backfill truthful correctness evidence ledgers
plans: close final corrective roadmap with verified disposition
```

---

## Explicit definition of done

Plan 051 is complete only when all of the following are true:

1. CLI and library legacy `Light` requests resolve to one recorded policy.
2. Explicit `Unspecified` survives every adapter and writer path.
3. `ProhibitedSeeConstraints` without constraints produces a structured warning and strict-mode failure behavior.
4. Current source and docs no longer claim `tdm:reserve_tdm` emission.
5. JPEG Huffman construction rejects inconsistent, duplicate, empty-referenced, and oversubscribed tables.
6. Encoder and decoder use one checked canonical-code assignment.
7. Exact SOS DC/AC table references are required; table-0 fallback is removed.
8. JPEG entropy decoding proves exact expected exhaustion with valid padding.
9. JPEGs with post-scan segments cannot enter the DCT-supported rewrite path.
10. Valid unsupported JPEGs downgrade explicitly and preserve the complete container.
11. Malformed JPEGs are not mislabeled or mutated through the unsupported fallback.
12. WebP parsing is bounded to the exact declared RIFF extent.
13. Duplicate/invalid VP8X and missing primary payload structures are rejected.
14. Mixed descriptions preserve unrelated XMP fields while owned fields follow policy.
15. Differing valid XMP packets are merged without silent loss or fail explicitly on namespace conflict.
16. Malformed XMP fails before output rather than being silently discarded.
17. Final WebP output has at most one XMP chunk.
18. VP8X bits are derived and validated from final emitted structure.
19. Representative final JPEG and WebP outputs decode independently.
20. `plans/039-status.md` through `plans/051-status.md` exist where required and are truthful.
21. `./scripts/check.sh` passes locally.
22. Current-head CI evidence is recorded if available; absence is recorded as absence.
23. Required CI remains one stable job.
24. No version bump, publication, tag, release, or release automation is introduced.

If any item remains open, Plan 051 and Roadmap 045 must remain `PARTIAL` and name the exact blocker.

---

## Non-goals

Do not use this plan to:

- add new image formats;
- implement progressive or restart-bearing JPEG DCT steganography;
- support arbitrary multi-scan JPEG transcoding;
- add a general image codec;
- add C2PA or TDMRep deployment;
- redesign the protection payload;
- add new cryptography;
- add a daemon, service, or network verifier;
- add exhaustive fuzzing or benchmark infrastructure;
- expand CI beyond the existing stable check;
- automate crates.io publication;
- publish a release.

---

## Handoff summary

The implementation agent should treat this as a closure pass, not a roadmap expansion.

The priority order is:

```text
prevent silent legal-policy inconsistency
prevent malformed or structurally ambiguous JPEG rewriting
prevent unrelated WebP/XMP metadata loss
make final container flags and lengths self-consistent
record evidence truthfully
```

Use explicit failure and bounded fallback when exact preservation cannot be proven. Do not trade silent data loss for broader nominal format acceptance.