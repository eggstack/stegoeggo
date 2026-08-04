# Plan 049: WebP XMP Replacement and Feature-Flag Corrective Pass

Status: Ready for implementation

Baseline: `main` after Plan 048 planning commit `873810fc97fc07cadb3c9af2a4fb476996e2910f`

Depends on:

- `plans/045-corrective-correctness-closure-roadmap.md`

Corrects incomplete criteria from:

- `plans/041-webp-container-xmp-exif-correctness.md`
- `plans/044-cross-format-correctness-closure.md`

Must complete before:

- `plans/050-post-corrective-evidence-and-documentation-closure.md`

---

## Purpose

Complete the bounded WebP container rewrite so metadata replacement produces one coherent XMP packet and VP8X feature bits describe the chunks actually present in the output.

The current implementation correctly introduced checked RIFF parsing and simple-to-extended VP8X conversion, but two central defects remain:

1. `merge_or_replace_webp_xmp()` prepares a new XMP chunk while the output loop still copies the original XMP chunk, so replacement can yield multiple XMP chunks.
2. Existing EXIF chunks are copied, but `ParsedWebP` does not track EXIF and the regenerated VP8X flags do not set the EXIF bit.

The current tests mostly check readability and high-level idempotence. They do not consistently assert exactly one XMP chunk, preservation of unrelated XMP properties, or equality between output chunks and VP8X feature bits.

This plan corrects those contracts without reintroducing malformed EXIF seed generation or adding a full WebP muxing framework.

---

## Governing decisions

1. New StegoEggo output stores the protection seed and rights metadata in XMP, not generated EXIF.
2. Existing unrelated EXIF must be preserved.
3. A rewritten output must contain at most one XMP chunk.
4. `ReplaceStegoOwned` replaces StegoEggo-owned XMP properties while preserving unrelated XMP properties.
5. `PreserveExisting` preserves existing StegoEggo values and adds only missing requested values according to the established metadata policy.
6. `FailOnConflict` reports semantic conflicts rather than duplicating packets.
7. Unknown RIFF chunks and image/animation payload chunks remain byte-identical unless the format conversion itself requires a new VP8X header.
8. VP8X feature bits are derived from the final emitted chunk inventory, not from stale input booleans.
9. Do not generate a new EXIF seed payload.
10. Do not add required `webpinfo`/`webpmux` CI jobs.
11. Do not add animated-WebP steganographic pixel processing in this pass.
12. Prefer a bounded XMP packet merge over a general XML library.

---

## Phase 0: Create the status ledger and chunk-policy table

Create `plans/049-status.md` before source edits.

Initialize it with:

```text
Plan baseline SHA: 873810fc97fc07cadb3c9af2a4fb476996e2910f
Disposition: OPEN
Chunk inventory: OPEN
Single-XMP replacement: OPEN
Unrelated XMP preservation: OPEN
EXIF flag correctness: OPEN
VP8X output-derived flags: OPEN
Unknown/payload preservation: OPEN
Malformed input behavior: OPEN
Idempotence: OPEN
Documentation: OPEN
CI: OPEN
Publication hold: no publication is part of this plan
```

Add a chunk-policy table:

```text
input structure | operation/policy | expected output chunks | expected VP8X flags | preservation rule | fixture | test | status
```

Required rows:

- simple VP8 without metadata;
- simple VP8L without metadata;
- VP8X plus ICCP;
- VP8X plus EXIF;
- VP8X plus XMP without StegoEggo properties;
- VP8X plus StegoEggo-owned XMP;
- VP8X plus unrelated and StegoEggo properties in one packet;
- duplicate identical XMP chunks;
- duplicate differing XMP chunks;
- unknown RIFF chunk;
- alpha-bearing WebP;
- animated WebP container;
- truncated RIFF chunk;
- RIFF size smaller/larger than physical input;
- malformed existing XMP.

---

## Phase 1: Make parsed chunk inventory complete

Primary file:

```text
src/webp_container.rs
```

### 1.1 Track actual metadata chunks

Extend parsed state to record at least:

```text
has_exif
xmp_indices
exif_indices
icc_indices
image/animation chunk indices as needed
```

A single boolean is insufficient for duplicate handling. Keep the existing generic chunk list as the source of truth.

### 1.2 Validate RIFF bounds consistently

The parser should walk only the declared RIFF payload range:

```text
RIFF size + 8
```

Define behavior for trailing bytes after the declared RIFF container. Preferred bounded behavior:

- reject unexplained trailing bytes for rewrite operations; or
- preserve them only if the current API intentionally supports trailing data and tests prove it.

Do not accidentally parse chunks beyond the declared container because physical input is longer.

Reject:

- chunk headers crossing declared RIFF end;
- payload or pad byte crossing declared RIFF end;
- arithmetic overflow;
- duplicate VP8X chunks;
- missing primary image/animation structure;
- invalid VP8X data length for rewriting.

### 1.3 Preserve reserved/unknown VP8X state deliberately

When regenerating VP8X, reserved bits/bytes must follow the WebP specification. Do not copy invalid reserved bits merely because they were present.

Record whether invalid reserved input causes rejection or normalization.

### Phase 1 acceptance criteria

- EXIF presence is observable;
- duplicate metadata chunks are enumerable;
- parser respects declared RIFF bounds;
- malformed padding and chunk lengths fail safely;
- rewriting has an exact primary-image structure;
- no panic or unchecked offset arithmetic remains.

Suggested commit:

```text
webp: complete checked metadata chunk inventory
```

---

## Phase 2: Build one final chunk plan before writing

Primary files:

```text
src/protected/metadata_trap.rs
src/webp_container.rs
```

The current writer mixes input iteration and metadata append decisions. Replace that with a small final chunk plan.

Preferred conceptual flow:

```text
parse input
classify existing metadata
resolve metadata policy
construct one final XMP packet or none
construct ordered final chunk inventory
compute flags from final inventory
write RIFF once
```

### 2.1 Explicit chunk disposition

For each input chunk assign one disposition:

```rust
enum ChunkDisposition {
    CopyVerbatim,
    ReplaceWithFinalXmp,
    DropDuplicateXmp,
    ReplaceVp8x,
}
```

The exact enum may differ. The important invariant is that original XMP chunks are not copied after a replacement packet has been scheduled.

### 2.2 Exactly one XMP chunk

After planning:

```text
count(final XMP chunks) == 0 or 1
```

When requested rights metadata is nonempty, expect exactly one.

Do not append a new XMP chunk while copying the original one.

### 2.3 Preserve non-XMP chunks

Except for VP8X replacement and intentional metadata changes, preserve the original bytes of:

```text
VP8/VP8L
ALPH
ANIM/ANMF
ICCP
EXIF
unknown chunks
```

Preserve original pad-byte semantics when copying. When rewriting a chunk, write a canonical zero pad byte for odd lengths.

### 2.4 Chunk ordering

Produce a valid extended WebP order.

At minimum:

```text
VP8X first
ICCP before image data when present
image/alpha/animation payload in valid order
EXIF and XMP in valid metadata positions
unknown chunks preserved in a safe relative position
```

Do not reorder animation frame payloads.

If unknown chunks cannot retain exact relative order while satisfying mandatory VP8X ordering, document and test the minimal movement.

### Phase 2 acceptance criteria

- one final chunk plan exists before bytes are emitted;
- original XMP is not copied when replaced;
- final output has at most one XMP chunk;
- payload and unknown chunks remain byte-identical;
- VP8X is first and unique;
- RIFF size is calculated from final bytes;
- rewriting is deterministic for fixed metadata/timestamps.

Suggested commit:

```text
webp: plan one coherent extended container rewrite
```

---

## Phase 3: Implement bounded XMP merge/replace semantics

Primary file:

```text
src/protected/metadata_trap.rs
```

### 3.1 Do not use packet duplication as preservation

Preserving unrelated XMP does not mean retaining one old packet and adding a second StegoEggo packet. The surviving packet must carry both unrelated properties and the resolved StegoEggo rights properties.

### 3.2 Bounded merge strategy

Use a strategy scoped to the packet shapes StegoEggo supports.

Preferred approach:

1. Select one structurally valid XMP packet as the base.
2. Locate its `rdf:RDF` container and `rdf:Description` content with checked string/XML-boundary helpers.
3. Remove only StegoEggo-owned attributes/elements and prior canonical rights fields that this operation owns under `ReplaceStegoOwned`.
4. Preserve unrelated namespace declarations, attributes, descriptions, and element content byte-for-byte where possible.
5. Insert one normalized StegoEggo rights `rdf:Description` or update the owned description.
6. Emit one packet.

A separate owned `rdf:Description` is acceptable and can reduce invasive modification of unrelated descriptions.

Do not implement XML merging using unconstrained global string replacement. Match complete property names and checked element/attribute boundaries.

### 3.3 Ownership scope

Define the exact owned fields, including at least:

```text
plus:DataMining
plus:OtherConstraints when emitted by StegoEggo
stegoeggo:ProtectionSeed
stegoeggo:AIConstraints
stegoeggo:CopyrightOwner
stegoeggo:LicensorName
stegoeggo:LicensorEmail
stegoeggo:LicensorURL
stegoeggo:NoticeAppliedAt
standard fields emitted from the requested RightsNotice where replacement policy treats them as StegoEggo-owned
```

Be careful with standard fields such as `dc:creator` and `dc:rights`: replacing them may erase externally authored metadata if ownership cannot be established. Use the existing ownership markers/packet structure and policy contract. Record the decision in the status ledger.

### 3.4 Multiple existing XMP chunks

Define deterministic behavior:

#### Identical duplicate packets

- collapse to one;
- apply the requested metadata policy;
- do not warn unless current reporting has a suitable informational warning.

#### Differing valid packets

Preferred bounded behavior:

- merge preserved non-owned `rdf:Description` content into one packet when the helper can do so without loss;
- otherwise return a clear metadata conflict/error rather than silently dropping one packet.

Do not copy both packets.

#### Malformed packet

- do not silently drop unrelated malformed bytes and claim preservation;
- either fail the rewrite or use an explicitly documented policy-specific fallback;
- output must not contain a generated packet plus the malformed original as duplicate XMP.

### 3.5 Policy semantics

`ReplaceStegoOwned`:

- remove prior owned properties;
- add requested owned properties;
- preserve unrelated properties;
- one packet.

`PreserveExisting`:

- retain existing owned values;
- add missing requested values only where the contract allows;
- one packet.

`FailOnConflict`:

- compare semantic owned values;
- fail before writing on conflict;
- unrelated XMP does not trigger conflict;
- no output is produced on failure.

### Phase 3 acceptance criteria

- one surviving packet contains both preserved unrelated and requested rights metadata;
- identical duplicates collapse;
- differing packets never cause silent data loss;
- malformed input behavior is explicit;
- all three metadata update policies are tested;
- repeated `ReplaceStegoOwned` is idempotent;
- no general XML dependency is added unless documented as unavoidable.

Suggested commit:

```text
webp: merge rights metadata into one preserved XMP packet
```

---

## Phase 4: Derive VP8X flags from final output chunks

Primary files:

```text
src/protected/metadata_trap.rs
src/webp_container.rs
```

### 4.1 Final inventory is authoritative

Compute VP8X flags only after final chunk dispositions are resolved.

Required mapping from actual final chunks/features:

```text
ICCP present -> ICC bit
ALPH or equivalent alpha feature -> alpha bit
EXIF present -> EXIF bit
XMP present -> XMP bit
ANIM/ANMF present -> animation bit
```

Do not set XMP merely because a temporary metadata vector was nonempty. Do not omit EXIF because StegoEggo no longer generates it.

### 4.2 Preserve dimensions accurately

For existing VP8X, read and validate dimensions.

For simple VP8/VP8L conversion, derive dimensions from checked payload parsing or the existing decoder. A decode failure must not silently produce zero dimensions.

Remove fallback behavior that can call `encode_vp8x_chunk(0, 0, ...)` after swallowing a dimension error.

### 4.3 Validate flag/chunk consistency

Add an internal validator used by tests, and optionally by rewrite completion, that confirms:

```text
XMP bit iff XMP chunk exists
EXIF bit iff EXIF chunk exists
ICC bit iff ICCP chunk exists
animation bit iff animation chunks exist
alpha bit matches the represented alpha feature
```

Be precise about alpha in VP8L, where alpha may be intrinsic rather than a separate ALPH chunk. Follow the WebP container specification and record the chosen detection method.

### Phase 4 acceptance criteria

- existing EXIF causes the EXIF bit to remain set;
- removed metadata clears the corresponding bit;
- one XMP packet sets exactly the XMP bit;
- ICC/animation/alpha behavior is correct for fixtures;
- dimensions are nonzero and accurate;
- output-derived validation passes.

Suggested commit:

```text
webp: derive VP8X features from emitted chunks
```

---

## Phase 5: Correct and expand focused tests

Primary tests:

```text
tests/conformance_container_tests.rs
tests/preservation.rs
tests/cross_format_closure.rs
tests/cross_format_semantics.rs
```

### 5.1 Exactly-one-XMP assertions

Replace readability-only duplicate tests with:

```text
count XMP chunks == 1
count VP8X chunks == 1
raw XMP contains requested canonical rights URI
raw XMP retains unrelated sentinel property
output decodes
```

Run after one, two, and three identical replacement operations to catch unbounded growth.

### 5.2 Existing unrelated XMP fixture

Create an independent fixture containing fields StegoEggo does not own, such as:

```xml
dc:subject
xmp:Rating
custom namespace sentinel
```

After `ReplaceStegoOwned`, assert exact sentinel values remain in the sole XMP packet.

### 5.3 Duplicate fixtures

Required:

- two identical XMP chunks;
- one unrelated packet plus one StegoEggo-owned packet;
- two differing unrelated packets that can be merged;
- malformed plus valid packet.

Assert the documented deterministic outcome, not merely successful decode.

### 5.4 EXIF flag fixture

Create a structurally valid WebP with an unrelated EXIF chunk. After rights injection:

```text
EXIF bytes unchanged
EXIF count unchanged
VP8X EXIF bit set
XMP count == 1
VP8X XMP bit set
```

No new StegoEggo EXIF chunk should appear.

### 5.5 Payload preservation

For VP8, VP8L, and representative animation/alpha structures where fixtures are available:

- hash or byte-compare payload chunks before and after metadata rewriting;
- verify only container metadata changed;
- preserve unknown chunk bytes.

### 5.6 Malformed boundaries

Test:

- declared RIFF size truncation;
- chunk payload crossing declared end;
- missing odd-byte pad;
- duplicate VP8X;
- zero/invalid dimensions;
- malformed XMP merge input.

No panic; no partially written output.

### Phase 5 acceptance criteria

- tests assert exact XMP count;
- unrelated XMP survives in the sole packet;
- EXIF flag and chunk presence agree;
- payload chunks remain byte-identical;
- repeated processing is bounded and semantically stable;
- malformed cases fail safely;
- fixtures remain small.

Suggested commit:

```text
tests: enforce WebP XMP replacement and feature consistency
```

---

## Phase 6: Documentation correction

Inspect:

```text
README.md
architecture/protected-metadata-trap.md
architecture/overview.md
architecture/verification.md
AGENTS.md
SUPPORT.md
CHANGELOG.md
```

Document:

- simple WebP conversion to VP8X when metadata is added;
- one-XMP output invariant;
- preservation of unrelated XMP/EXIF/ICC/unknown chunks;
- XMP-only new seed emission;
- backward-compatible EXIF seed extraction only;
- output-derived feature flags;
- explicit behavior for duplicate or malformed XMP.

Do not claim arbitrary lossless XML merging beyond the tested packet shapes.

### Phase 6 acceptance criteria

- docs no longer imply new EXIF seed generation;
- one-XMP behavior is explicit;
- preservation scope is accurate;
- malformed/duplicate behavior is stated;
- no broader WebP feature claim is introduced.

Suggested commit:

```text
docs: define WebP metadata replacement and preservation contract
```

---

## Required verification

Run at minimum:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --all-features --test conformance_container_tests
cargo test --all-features --test preservation
cargo test --all-features --test cross_format_closure
cargo test --all-features --test cross_format_semantics
cargo test --workspace --all-features
cargo check -p stegoeggo --no-default-features
./scripts/check.sh
```

Run one bounded `webpinfo` or `webpmux` inspection if available and record it in `plans/049-status.md`. Do not add it to CI.

---

## Definition of done

Plan 049 is complete only when:

1. Rewritten WebP contains at most one VP8X and one XMP chunk.
2. Rights-bearing output contains exactly one XMP chunk.
3. Original XMP is not copied after replacement is scheduled.
4. Unrelated XMP properties survive in the sole packet.
5. Identical duplicates collapse deterministically.
6. Differing/malformed packets never cause silent data loss.
7. Existing EXIF remains byte-identical and sets the VP8X EXIF bit.
8. XMP, ICC, alpha, animation, and EXIF flags match final output features.
9. VP8/VP8L/animation payload and unknown chunks remain preserved according to the documented contract.
10. Repeated replacement does not grow metadata duplicates.
11. Corrected outputs decode internally and through one independent tool when available.
12. New output remains XMP-only for StegoEggo seed metadata.
13. `plans/049-status.md` records exact commits, commands, fixtures, and results.
14. `./scripts/check.sh` passes.
15. No release or publication action occurs.