# Roadmap 076: Stego Pipeline and Carrier Library Closure

Status: COMPLETE

Final disposition (Plan 080, evidence in `plans/080-status.md`, all rows
R01-R29 CLOSED, no corrective product patch required):

- Plan 077 routing correction disposition: complete (`16858ce`;
  output-domain carrier routing with 17-test regression matrix).
- Plan 078 measured optimization disposition: complete (`8b1c47b`;
  single-decode verification/embed, in-place tiled LSB, header-only
  preflight; prepared-API disposition `PRIVATE-REUSE-SUFFICIENT`).
- Plan 079 public API/prepared-object disposition: complete (`def2077`;
  public tiled carrier API + root dogfooding; prepared-API disposition
  `NO-PROMOTION`).
- Plan 080 final verification commit: ledger + documentation closure; zero
  product-source edits; `./scripts/check.sh` exit 0; full workspace 1803
  passed / 0 failed.

Audited planning baseline: `main` at `6feb52a90d9afdc0c922cdb219529524ad94c168`

Predecessors: Roadmap 057 and Roadmap 069, both treated as closed baseline work. This roadmap does not reopen their completed carrier split, corrected LSB model, JPEG DCT/F5 implementation, generic framing, or application-adapter decomposition.

Implementation plans:

1. `plans/077-output-domain-carrier-routing-correctness.md`
2. `plans/078-single-decode-and-tiled-allocation-optimization.md`
3. `plans/079-public-tiled-carrier-api-and-parent-dogfooding.md`
4. `plans/080-stego-pipeline-library-evidence-and-documentation-closure.md`

Each implementation plan must create and force-track its own `plans/NNN-status.md` ledger before product-source edits.

---

## 1. Purpose

The earlier stego roadmaps established the correct broad dependency direction:

```text
StegoEggo rights/policy/provenance application
        |
        | application payload bytes + explicit carrier parameters
        v
stegoeggo-stego generic carrier crate
        |
        +-- LSB pixel-domain carrier
        +-- JPEG DCT/F5 encoded-byte carrier
        +-- generic self-describing frame
```

The current architecture is substantially better than the pre-split implementation, but a current-source audit found four residual classes that now justify a focused closure pass:

1. **Carrier selection is not consistently output-domain driven.** The top-level executor routes JPEG output correctly, but `SteganographyProtector::apply_to_image_with_summary_from_plan()` selects its carrier path from `plan.input_format()`. A JPEG input transcoded to PNG/WebP can therefore take a JPEG-DCT path internally and then lose that coefficient-domain marker when the result is decoded and encoded as the final raster format.
2. **Raster execution still performs avoidable decoding/allocation work.** The non-JPEG preflight path calls `ImageReader::into_dimensions()` and then also attempts a full `load_image_from_bytes()` whose decoded image is discarded before normal execution decodes again. Tiled LSB embedding clones the full `RgbaImage` even when the application already owns a mutable buffer.
3. **JPEG reuse is incomplete.** `jpeg::extract_framed()` was already corrected to decode coefficients once, but application verification still calls `carrier_support::jpeg_extract()` repeatedly across candidate redundancies and payload lengths. Tiled JPEG embedding also re-decodes the encoded output solely to self-verify a tile after it already had the mutated coefficients in memory.
4. **The standalone carrier crate is usable but the parent still has privileged paths for ordinary operations.** `stegoeggo-stego` already provides raw, framed, and in-place public APIs, but the root application still reaches through `application_support` for standard LSB/JPEG operations that are representable by the stable public facade. Tiled operations remain application-support-only despite being application-neutral carrier behavior.

This roadmap closes those residuals without redesigning the wire formats or exposing JPEG implementation structs.

---

## 2. Research findings informing this roadmap

### 2.1 Current StegoEggo source

At the audited baseline:

- `process_plan_bytes()` is the canonical executor and `ResolvedProtectionPlan` is the canonical execution state.
- JPEG-in/JPEG-out takes the correct encoded-byte DCT fast path.
- PNG/WebP ordinary LSB embedding already uses an in-place mutation path after conversion to owned RGBA pixels.
- `stegoeggo-stego` is already a separate application-neutral crate with public `lsb`, `jpeg`, `frame`, `error`, and report/config surfaces.
- JPEG parser, entropy/Huffman, coefficient maps, transcoder, and F5 implementation objects remain private.
- `application_support` is feature-gated and hidden, but contains both compatibility operations and generic carrier mechanics such as tiled JPEG embed/search.

The public-library question is therefore no longer “should a generic crate exist?” It already exists. The remaining question is which carrier operations should move from parent-only support into the stable operation-level API, and which compatibility/search machinery should remain private or hidden.

### 2.2 Rust steganography ecosystem

The Rust ecosystem remains relatively sparse for reusable image-steganography libraries. `stegano-rs` is a relevant active comparator and now separates its workspace into `stegano-core`, `stegano-f5`, JPEG codec crates, and CLI tooling. Its higher-level encoder selects LSB versus F5 from the **target/output file extension**, which independently supports the output-domain routing rule this roadmap adopts:

- https://github.com/steganogram/stegano-rs
- https://github.com/steganogram/stegano-rs/blob/main/Cargo.toml
- https://github.com/steganogram/stegano-rs/blob/main/crates/stegano-core/src/lib.rs

`stegano-f5` exposes both high-level JPEG operations and lower-level coefficient-oriented F5 types. That is useful evidence that advanced callers value reuse after JPEG parsing, but StegoEggo should not copy the coefficient-exposure design. An opaque prepared/reusable carrier, if justified by measurements, provides the reuse benefit without making coefficient/Huffman/transcoder representations part of the semver contract:

- https://github.com/steganogram/stegano-rs/blob/main/crates/stegano-f5/src/lib.rs

Other current Rust projects are predominantly LSB-oriented tools or partially implemented DCT systems. This reinforces that `stegoeggo-stego`'s encoded-JPEG DCT/F5 operation API is already a useful differentiator; the priority is to make its existing capabilities coherent and reusable rather than add speculative algorithms.

### 2.3 Image decoding API

The `image` crate distinguishes dimension inspection from full decode. `ImageReader::into_dimensions()` constructs the appropriate decoder and reads dimensions, while full pixel materialization is performed by `decode()`. The crate documentation also explicitly describes dimension-only queries as faster than fully loading the image. The pipeline should therefore retain a header/dimension resource gate without immediately performing a second full decode that is discarded:

- https://docs.rs/image/latest/image/struct.ImageReader.html
- https://docs.rs/image/latest/image/fn.image_dimensions.html

---

## 3. Required architectural end state

The application pipeline should become structurally equivalent to:

```text
ProtectionRequest
    |
    v
ResolvedProtectionPlan
    |
    +-- dimension/container preflight (no unnecessary pixel decode)
    |
    v
select carrier from OUTPUT DOMAIN
    |
    +-- JPEG output
    |     |
    |     +-- JPEG input: reuse encoded bytes
    |     +-- other input: decode once -> encode JPEG once
    |     |
    |     +-- DCT/F5 or seed-hint operation
    |
    +-- PNG/WebP output
          |
          +-- decode pixels once
          +-- own RGBA buffer
          +-- LSB / tiled-LSB / seed-hint operation in place
    |
    v
inject rights metadata
    |
    v
output bytes + warnings/report
```

The carrier/library boundary should become:

```text
normal generic caller --------------------+
                                           |
StegoEggo parent application --------------+--> stable stegoeggo-stego operations
                                           |      - raw LSB/JPEG
                                           |      - framed LSB/JPEG
                                           |      - in-place LSB
                                           |      - generic tiled operations
                                           |      - optional prepared JPEG reuse if evidence justifies it
                                           |
legacy/application compatibility ----------+--> narrow application-support only where necessary
                                                  - V1/V2 legacy LSB recovery
                                                  - historical seed/candidate search behavior
                                                  - no ordinary current carrier operation duplication
```

The default public API must continue to hide:

```text
JpegHeader
Coefficients
JpegTranscoder
DctStegoF5
entropy/Huffman implementation state
LSB permutation internals
raw parser state
```

---

## 4. Governing constraints

1. Preserve `#![forbid(unsafe_code)]` in both library crates.
2. Preserve payload-v3 bytes, generic frame bytes, seed derivation, corrected LSB V2 mapping, JPEG F5 coefficient-selection behavior, and legacy extraction compatibility.
3. Preserve the supported JPEG subset and `JpegUnsupportedReason` classifications unless a focused correctness bug proves otherwise.
4. Preserve JPEG container-preserving behavior for successful DCT embedding.
5. Preserve metadata injection and rights-policy semantics; this roadmap is about carrier routing, reuse, allocation, and public carrier API boundaries.
6. Carrier selection in application execution must be determined by the **final output representation**, never merely by input format.
7. Input format may determine whether encoded bytes can be reused without transcoding, but may not determine the final stego carrier domain.
8. Do not add a generic `Carrier` trait, plugin framework, codec hierarchy, or new media backend.
9. Do not expose coefficient maps, JPEG parser structs, F5 objects, permutation functions, or entropy state as stable API.
10. Do not add encryption to the generic carrier frame. CRC32 remains corruption detection, not authentication.
11. Do not change authentication semantics in the root application.
12. Do not reduce bounded verification search coverage merely to improve benchmarks.
13. Keep legacy compatibility in hidden/application support when it is not appropriate for the generic stable API.
14. Prefer one-shot public operations. A reusable/prepared JPEG public object is permitted only after Plan 078 records concrete decode-count and runtime evidence showing a real multi-extraction use case.
15. No new dependency is expected. Any new dependency requires explicit evidence and must be recorded in the relevant status ledger.
16. No version bump, crates.io publication, tag, GitHub Release, or required-CI expansion is authorized by this roadmap.
17. `./scripts/check.sh` remains the required final repository gate.
18. Performance changes must preserve deterministic carrier output where the algorithm is unchanged and must be backed by decode-count, allocation-count, or Criterion evidence.

---

## 5. Execution order

Execute sequentially:

```text
077 output-domain routing correctness
        |
        v
078 single-decode + allocation optimization
        |
        v
079 public tiled API + parent dogfooding
        |
        v
080 evidence/documentation closure
```

Why this order:

- Correct carrier selection must be fixed before performance measurements are meaningful.
- Decode/allocation work should be consolidated before deciding whether any prepared/reuse API deserves public promotion.
- Public API changes should be made only after the carrier's internal reuse model is stable.
- Documentation/evidence closes last so it describes tested behavior rather than desired behavior.

Do not combine all implementation into one monolithic commit. Each numbered plan should remain independently reviewable and revertible.

---

## 6. Roadmap-owned outcomes

### 6.1 Output-format carrier correctness

All format conversions must embed into the carrier domain of the final output:

```text
PNG/WebP output -> LSB-domain marker
JPEG output     -> JPEG DCT-domain marker or documented seed-only fallback
```

This includes JPEG -> PNG/WebP, which is currently the key regression-risk path.

### 6.2 One necessary decode per representation

Expected normal-path targets:

```text
JPEG -> JPEG Standard/Tiled      0 pixel decodes; 1 coefficient decode for embed
JPEG verification               1 coefficient decode per verification search context
PNG/WebP -> PNG/WebP marker      1 pixel decode
PNG/WebP -> JPEG marker          1 pixel decode + 1 JPEG coefficient decode after encode
JPEG -> PNG/WebP marker          1 pixel decode
metadata-only same-format        0 pixel decodes when container injection can operate directly
metadata-only cross-format       1 pixel decode
```

The implementation may perform bounded header parsing in addition to these counts.

### 6.3 Tiled LSB avoids a second full-image buffer

If the caller owns the RGBA image, tiled LSB embedding should mutate that buffer directly after capacity validation. A cloning convenience operation may remain but must delegate to the in-place core.

### 6.4 JPEG verification does not repeatedly entropy-decode the same image

The application currently searches multiple redundancies and payload lengths. That search should retain one private decoded coefficient representation instead of repeatedly calling one-shot extraction functions.

### 6.5 Generic tiled carrier operations become first-class

The stable carrier crate should expose operation-level tiled embedding/extraction for arbitrary bytes and framed payloads without exposing tile-block maps or DCT internals.

### 6.6 Parent application dogfoods stable current-carrier operations

Standard current LSB/JPEG operations in the root should use the same stable public APIs available to downstream consumers. `application-support` should contain only operations needed for legacy compatibility or application-specific bounded search that cannot cleanly fit the stable generic API.

### 6.7 Public prepared JPEG reuse is evidence-gated

Plan 078 must measure the application verification path. Plan 079 may expose an opaque prepared JPEG carrier only if the evidence shows a concrete benefit and the type can keep all codec/coefficient state private. Otherwise reuse remains private/hidden and the default stable API stays one-shot.

---

## 7. Verification budget

Required across the roadmap:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo test -p stegoeggo-stego --all-features
cargo test -p stegoeggo-stego --doc
cargo test -p stegoeggo --test public_stego_api --all-features
cargo test -p stegoeggo --test request_api --all-features
cargo test -p stegoeggo --all-features stego
cargo test -p stegoeggo --all-features tiled
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Add focused Criterion measurements for the exact changed hot paths. Benchmarks remain local evidence and must not become required CI gates.

---

## 8. Roadmap acceptance criteria

Roadmap 076 is complete only when all are true:

1. Cross-format hidden-marker tests prove carrier selection follows output format.
2. JPEG -> PNG/WebP BestEffort and Tiled outputs contain recoverable raster-domain markers rather than transient JPEG coefficient markers.
3. PNG/WebP -> JPEG retains DCT embedding behavior.
4. Same-format JPEG preserves the encoded-byte fast path.
5. Non-JPEG preflight no longer performs a discarded full decode before execution.
6. Tiled LSB has a shared in-place mutation core and the parent uses it when it owns the buffer.
7. Tiled JPEG embed no longer re-decodes its own encoded output solely for production self-verification.
8. Standard JPEG application verification retains one decoded coefficient context across redundancy/payload-length probing.
9. Verification search coverage and error classification remain compatible.
10. Generic public tiled LSB and JPEG operations exist at the operation level.
11. Generic framed tiled recovery does not require caller-retained payload length.
12. The default public carrier API still cannot name JPEG codec/coefficient/F5 internals or LSB permutation internals.
13. Root current-carrier paths use stable public operations wherever equivalent APIs exist.
14. `application-support` is materially smaller and limited to compatibility/application-specific search needs.
15. Any public prepared JPEG reuse API is justified by recorded evidence; if evidence does not justify it, no such API is added.
16. Public API examples and compile-fail boundary tests pass.
17. Architecture docs state the output-domain carrier invariant explicitly.
18. Benchmark/allocation/decode evidence is recorded in the status ledgers.
19. `./scripts/check.sh` passes at final closure.
20. No unrelated release, dependency, CI, rights-policy, payload, or metadata redesign is introduced.

---

## 9. Explicit non-goals

This roadmap does not authorize:

- a new steganography algorithm;
- steganalysis/detection functionality;
- claims of forensic or adversarial undetectability;
- visible watermark rendering;
- JPEG XL, AVIF, audio, video, PDF, or arbitrary binary carriers;
- progressive-JPEG DCT embedding support;
- arbitrary JPEG parser/coefficient public access;
- payload-v4;
- changes to HMAC/Ed25519 semantics;
- compression/encryption inside the generic frame;
- Python/C/WASM bindings;
- a general carrier trait/plugin framework;
- a new workspace crate;
- publication or release automation.

---

## 10. Final handoff condition

After Plan 080, a maintainer should be able to answer all of the following without tracing private codec internals:

- Which carrier is selected for any input/output format pair?
- How many pixel/coefficient decodes occur on each normal path?
- How does a generic caller embed raw, framed, in-place, and tiled payloads?
- Which operations are stable public API versus parent-only compatibility support?
- Why are JPEG coefficient/transcoder types still private?
- What evidence justified or rejected a prepared/reusable JPEG public object?

If any answer still requires understanding rights metadata, JPEG entropy code, and application verification simultaneously, Plan 080 must mark this roadmap `PARTIAL` rather than papering over the residual.