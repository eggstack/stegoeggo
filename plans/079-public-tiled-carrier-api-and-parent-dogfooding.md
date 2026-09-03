# Plan 079: Public Tiled Carrier API and Parent Dogfooding

Status: Ready for implementation

Roadmap: `plans/076-stego-pipeline-and-carrier-library-closure-roadmap.md`

Depends on: Plans 077-078 complete.

Audited planning baseline: `main` at `6feb52a90d9afdc0c922cdb219529524ad94c168`

Authoritative implementation ledger to create before product edits: `plans/079-status.md`

---

## 1. Purpose

Make the already-separated `stegoeggo-stego` crate fully useful as a standalone generic Rust steganography library for the carrier functionality StegoEggo itself depends on, while shrinking parent-only privileged access.

The current public carrier API is already coherent for:

```text
LSB raw
LSB in-place
LSB framed
JPEG raw
JPEG framed
JPEG support/seed hints
```

The main generic functionality still trapped behind `application-support` is tiled/crop-oriented carrier behavior. Tiling is application-neutral: it embeds arbitrary bytes into independent raster/JPEG regions using deterministic tile-local seeds. It does not inherently depend on rights metadata, payload-v3, HMAC, provenance, or StegoEggo policy.

At the same time, the root application still uses `application_support` wrappers for several ordinary current operations that already have equivalent stable public APIs. That weakens the public boundary because the package's largest consumer is not exercising the same API downstream users receive.

This plan therefore:

1. promotes generic tiled carrier operations to the stable operation-level facade;
2. migrates the root current-carrier paths to stable public operations wherever semantics are equivalent;
3. leaves historical compatibility/search-only mechanics in narrow hidden support;
4. decides, using Plan-078 measurements, whether an opaque prepared JPEG reuse object deserves stable public API.

---

## 2. Research and design rationale

### 2.1 Existing StegoEggo carrier surface

`stegoeggo-stego` already deliberately hides:

```text
JpegHeader
Coefficients
JpegTranscoder
DctStegoF5
Huffman/entropy state
LSB permutations/slot mapping
```

while exposing operation-level `lsb`, `jpeg`, and `frame` modules. Preserve that design.

The hidden `application_support` module is acceptable for V1/V2 compatibility and unusual application search semantics, but ordinary current LSB/JPEG/tiled embedding should not require privileged access when the operations are independently meaningful to generic callers.

### 2.2 Rust ecosystem comparison

`stegano-rs` is a useful comparator because it separates higher-level steganography orchestration from an F5 crate and exposes both high-level JPEG operations and low-level coefficient-oriented F5 types. That demonstrates real interest in reusable F5 components, but it also illustrates a semver cost StegoEggo can avoid: exposing coefficient/F5 implementation objects turns internal representation into public contract.

StegoEggo should retain its operation-level design and, if measured reuse warrants it, expose only an **opaque prepared carrier** whose fields and coefficient representation remain private.

### 2.3 Scope criterion

A function belongs in the stable generic API when all of these are true:

```text
accepts arbitrary application bytes
uses only carrier-domain configuration
has semantics explainable without StegoEggo rights/payload types
can be tested independently of the root application
returns generic carrier reports/errors
```

A function remains hidden application support when it exists primarily for:

```text
legacy V1/V2 compatibility
StegoEggo-specific historical seed search
application-specific candidate ordering/classification
root verification orchestration that cannot be expressed as generic carrier recovery
```

---

## 3. Required end state

The stable public carrier should conceptually provide:

```text
stegoeggo_stego::lsb
    capacity
    embed / extract
    embed_in_place
    embed_framed / extract_framed
    embed_tiled / extract_tiled
    embed_tiled_in_place
    embed_tiled_framed / extract_tiled_framed

stegoeggo_stego::jpeg
    inspect / probe_support / capacity
    embed / extract
    embed_framed / extract_framed
    embed_tiled / extract_tiled
    embed_tiled_framed / extract_tiled_framed
    embed_seed_hint / extract_seed_hint

stegoeggo_stego::frame
    unchanged self-describing generic frame
```

The exact naming may use `*_tiled` rather than `tiled_*`, but raw/framed/in-place naming must remain regular and discoverable.

The root application should then use these stable APIs for current carrier operations.

`application_support` should remain only for the narrow compatibility/search cases that cannot responsibly become stable generic API.

---

## 4. Governing constraints

1. Preserve all existing stable public API signatures.
2. Additive public API only; no breaking rename/removal in this plan.
3. Preserve `#![forbid(unsafe_code)]`.
4. Preserve corrected LSB V2, F5, tile seed derivation, and generic frame wire behavior exactly.
5. Preserve the application-support feature as optional/hidden while reducing its scope.
6. Do not expose JPEG headers, tables, coefficients, parser state, transcoder state, F5 objects, tile block maps, permutations, or internal candidate keys as stable API.
7. Stable APIs accept application-neutral bytes/configuration only.
8. Generic framed APIs use CRC32 only for corruption detection. Do not add authentication/encryption semantics.
9. Tiled extraction must be bounded. Do not expose an unbounded “search everything” convenience.
10. Preserve parent application verification coverage and legacy compatibility.
11. Prefer existing report/error types over parallel tiled-specific result hierarchies.
12. Avoid a generalized carrier trait/framework.
13. A public prepared JPEG object is evidence-gated by Plan 078 and must be opaque if approved.
14. No new dependency, version bump, publication, tag, release, or CI expansion is expected.

---

# Phase 0 — ledger and API inventory

## 0.1 Create `plans/079-status.md` before product edits

Record:

```text
starting HEAD
working tree status
workspace versions
Plan 078 final performance disposition
current stable carrier public symbols
current application_support public symbols under the feature
root call sites using application_support
```

Start these rows `OPEN`:

```text
R01 generic tiled API scope is application-neutral
R02 public tiled configuration validation is fallible
R03 public LSB tiled raw roundtrip exists
R04 public LSB tiled in-place roundtrip exists
R05 public LSB tiled framed recovery exists without caller-known payload length
R06 public JPEG tiled raw roundtrip exists
R07 public JPEG tiled framed recovery exists without caller-known payload length
R08 tiled extraction is explicitly bounded
R09 crop-oriented tiled recovery behavior remains compatible
R10 ordinary root current LSB embed/extract uses stable public API
R11 ordinary root current JPEG embed/extract uses stable public API
R12 root current tiled embedding uses stable public API
R13 application_support no longer duplicates ordinary current carrier operations
R14 legacy V1/V2 compatibility remains available
R15 JPEG implementation structs remain private
R16 LSB implementation helpers remain private
R17 prepared JPEG public API disposition follows Plan-078 evidence
R18 direct standalone carrier consumer compiles with default features
R19 root stegoeggo::stego re-export exposes intended additions
R20 public API/docs/doctests pass
R21 full check.sh passes
R22 no unrelated dependency/version/release/CI change
```

## 0.2 Build a symbol classification table

Classify every current `application_support` export as:

```text
PROMOTE-STABLE
REPLACE-WITH-EXISTING-STABLE
KEEP-HIDDEN-COMPATIBILITY
KEEP-HIDDEN-SEARCH
DELETE-DEAD
```

At minimum classify:

```text
legacy_lsb_required_slots
corrected_lsb_embed
corrected_lsb_embed_in_place
corrected_lsb_extract
legacy_lsb_extract
legacy_lsb_extract_range
tiled_lsb_embed
seed_fallback_embed
seed_fallback_extract
tile_seed
bits_to_bytes
crop_image_region
jpeg_embed
jpeg_extract
jpeg_embed_tiled
TiledJpegSearch
TiledJpegCandidateKey
private reusable JPEG search context introduced by Plan 078
```

The ledger must justify retained hidden exports rather than simply carrying them forward.

### Phase 0 acceptance criteria

- every hidden export has a disposition;
- ordinary operations already covered by stable APIs are identified for root migration;
- proposed promotions contain no root application semantics.

---

# Phase 1 — define minimal public tiled configuration

Do not overload `LsbConfig` or `JpegConfig` with application-specific search state.

## 1.1 Preferred shared tile configuration

Prefer a small stable value type in the carrier crate, conceptually:

```rust
pub struct TileConfig {
    seed: u64,
    tile_size: u32,
}

impl TileConfig {
    pub fn try_new(seed: u64, tile_size: u32) -> StegoResult<Self>;
    pub fn seed(&self) -> u64;
    pub fn tile_size(&self) -> u32;
}
```

Exact placement may be `types` or a small public tiled-related location, but there should not be separate nearly-identical LSB/JPEG tile config structs without a concrete reason.

Generic validation:

```text
tile_size > 0
checked geometry arithmetic
```

Carrier-specific validation may be stricter:

```text
JPEG tile size >= 8
JPEG tile size must map deterministically to DCT blocks;
prefer rejecting non-multiples of 8 rather than silently truncating
```

Existing StegoEggo application tile sizes are already constrained more narrowly and should remain compatible.

## 1.2 Keep extraction bounds explicit

Tiled recovery searches crop-origin candidates. The stable API must require an explicit finite bound or a documented conservative default with a hard maximum.

Preferred explicit argument:

```rust
max_origins: u32
```

Reject `0` via `InvalidConfig` or document a consistent no-search result; choose one contract and test it.

Do not put application `ResourceLimits` types into the carrier API.

### Phase 1 acceptance criteria

- public configuration is small and application-neutral;
- untrusted tile sizes/search limits fail safely;
- no hidden application type leaks into the carrier facade.

---

# Phase 2 — expose generic tiled LSB operations

## 2.1 Raw copied operation

Expose a stable raw copied-image operation conceptually:

```rust
lsb::embed_tiled(
    img: &RgbaImage,
    payload: &[u8],
    config: &TileConfig,
) -> StegoResult<EmbedReport<RgbaImage>>
```

It should delegate to the shared in-place tiled core from Plan 078.

## 2.2 In-place operation

Expose:

```rust
lsb::embed_tiled_in_place(
    img: &mut RgbaImage,
    payload: &[u8],
    config: &TileConfig,
) -> StegoResult<InPlaceEmbedReport>
```

Insufficient capacity must leave the input unchanged.

## 2.3 Raw extraction

Expose bounded recovery when the payload length is known:

```rust
lsb::extract_tiled(
    img: &RgbaImage,
    payload_len: usize,
    config: &TileConfig,
    max_origins: u32,
) -> StegoResult<Vec<u8>>
```

The implementation may try the existing compatible crop-origin/tile-seed search pattern internally. Do not expose candidate keys or permutations.

If raw extraction can return multiple plausible payloads without frame integrity, define a deterministic first-valid/current-compatible rule and document that raw mode cannot authenticate correctness. Prefer framed mode for self-validating recovery.

## 2.4 Framed tiled convenience

Expose:

```rust
lsb::embed_tiled_framed(...)
lsb::extract_tiled_framed(...)
```

Framed extraction must:

```text
recover prefix/header with bounded search
validate declared length before full allocation/extraction
recover full frame from the same candidate identity
validate CRC32
return only payload bytes
```

The caller must not need to retain payload length.

### Phase 2 acceptance criteria

- raw/in-place/framed tiled LSB are usable without root StegoEggo;
- no full-image clone occurs in the in-place path;
- crop recovery is bounded and deterministic;
- frame validation prevents unbounded declared-length allocation.

---

# Phase 3 — expose generic tiled JPEG operations

## 3.1 Raw embed

Expose an operation conceptually:

```rust
jpeg::embed_tiled(
    jpeg_bytes: &[u8],
    payload: &[u8],
    config: &TileConfig,
) -> StegoResult<EmbedReport>
```

Tiled JPEG currently uses redundancy 1 within each tile because the tile grid supplies spatial redundancy. Preserve that behavior and document it rather than exposing a misleading independent redundancy knob.

The operation must preserve the same supported-JPEG subset and container-preserving encode behavior as ordinary JPEG embedding.

## 3.2 Raw bounded extraction

Expose:

```rust
jpeg::extract_tiled(
    jpeg_bytes: &[u8],
    payload_len: usize,
    config: &TileConfig,
    max_origins: u32,
) -> StegoResult<Vec<u8>>
```

It must decode coefficients once per operation and keep tile/candidate identities private.

## 3.3 Framed tiled convenience

Expose:

```rust
jpeg::embed_tiled_framed(...)
jpeg::extract_tiled_framed(...)
```

Framed extraction must recover without caller-retained payload length and must retain one decoded coefficient state across prefix/full candidate validation.

A candidate's prefix/header/full extraction must stay tied to the same tile origin, seed-offset identity, and carrier settings; equal prefix bytes from distinct candidates must not collapse identity.

## 3.4 Unsupported JPEG behavior

Generic public tiled JPEG should return the same structured `UnsupportedJpeg` classification as ordinary JPEG carrier APIs. It should not silently convert progressive/unsupported JPEGs to another format or inject application seed metadata.

The higher-level StegoEggo application may continue its documented seed-only fallback separately.

### Phase 3 acceptance criteria

- generic tiled JPEG raw/framed roundtrips pass;
- extraction decodes supported JPEG coefficients once;
- unsupported structures return structured carrier errors;
- no internal JPEG representation becomes public.

---

# Phase 4 — migrate the root parent to stable current-carrier APIs

## 4.1 Ordinary LSB current operations

Replace hidden wrappers with public equivalents where behavior matches:

```text
corrected_lsb_embed              -> lsb::embed
corrected_lsb_embed_in_place     -> lsb::embed_in_place
corrected_lsb_extract            -> lsb::extract
```

Adapt `EmbedReport` to the root's existing application summaries at the root boundary. Do not add root semantics to the carrier report solely to avoid a small adapter.

## 4.2 Ordinary JPEG current operations

Replace hidden wrappers where equivalent:

```text
jpeg_embed   -> jpeg::embed
jpeg_extract -> jpeg::extract or retained search context where multiple extraction calls must share one decode
```

For one-shot current embedding, stable public `jpeg::embed` should be the parent path.

For application verification multi-candidate search, retain Plan-078's hidden reuse context unless Phase 6 approves a public prepared JPEG object.

## 4.3 Current tiled embedding

Use newly stable:

```text
lsb::embed_tiled_in_place
jpeg::embed_tiled
```

for root current V3 marker emission.

Current tiled application verification may use stable framed/raw tiled extraction only where its semantics are identical. StegoEggo's mixed V3/legacy/application-auth candidate classification can continue to use a hidden search context if it needs richer candidate control.

## 4.4 Do not force legacy compatibility through new stable API

Keep legacy V1/V2 operations hidden if promoting them would misrepresent them as recommended generic carrier formats.

Likely hidden survivors include:

```text
legacy_lsb_extract
legacy_lsb_extract_range
legacy capacity helpers if still needed
historical offset-seed application search
application-specific candidate search/context
seed fallback if no generic user-facing contract justifies it
```

### Phase 4 acceptance criteria

- root's normal V3/current embedding uses stable carrier APIs;
- hidden wrappers no longer duplicate normal stable embed/extract behavior;
- legacy extraction remains fully compatible;
- hidden support shrinks measurably.

---

# Phase 5 — reduce `application_support` to an intentional compatibility boundary

After root migration, delete unused hidden exports.

The remaining module-level documentation must say exactly why each retained operation is not stable generic API.

Preferred residual categories:

```text
legacy compatibility
application-specific bounded candidate search
application seed fallback needed for historical StegoEggo verification
```

Do not retain a hidden wrapper solely to save the root from constructing an existing public `LsbConfig`/`JpegConfig`.

Add a compile/source test or lightweight maintenance check if practical to prevent ordinary public API functions from being mirrored unnecessarily in application support. Do not add brittle source-text tests if normal module tests and review conventions are clearer.

### Phase 5 acceptance criteria

- `application_support` is materially smaller;
- every remaining export has a documented compatibility/search rationale;
- no stable implementation structs are exposed through the hidden feature.

---

# Phase 6 — evidence-gated prepared JPEG API decision

Read the final `plans/078-status.md` disposition before making any public reuse type.

## 6.1 If Plan 078 says `PRIVATE-REUSE-SUFFICIENT`

Do not add a prepared public JPEG type.

Record `NO-PROMOTION` in Plan 079 and keep the stable JPEG API one-shot plus framed/tiled conveniences.

This is a successful outcome.

## 6.2 If Plan 078 says `PUBLIC-REUSE-CANDIDATE`

A prepared API may be added only if it remains opaque and clearly useful to non-StegoEggo callers.

Preferred minimal shape is read-oriented reuse, conceptually:

```rust
pub struct PreparedJpeg {
    /* private decoded carrier state */
}

pub fn prepare(jpeg_bytes: &[u8]) -> StegoResult<PreparedJpeg>;

impl PreparedJpeg {
    pub fn capacity(&self, payload_len: usize, config: &JpegConfig)
        -> StegoResult<CapacityReport>;
    pub fn extract(
        &self,
        payload_len: usize,
        config: &JpegConfig,
        actual_redundancy: usize,
    ) -> StegoResult<Vec<u8>>;
    pub fn extract_framed(&self, config: &JpegConfig)
        -> StegoResult<Vec<u8>>;
}
```

Tiled extraction may be included if it naturally reuses the same private state.

Do not expose:

```text
coefficients
component maps
headers/tables
raw block sets
entropy state
F5 object
mutable coefficient access
```

Avoid a mutable prepared encoder/session unless benchmarks demonstrate a distinct real use case.

## 6.3 Stability wording

If public, document that `PreparedJpeg` is an opaque optimization surface whose internal representation is not observable. Public behavior is operation semantics only.

### Phase 6 acceptance criteria

- decision follows recorded measurements rather than preference;
- `NO-PROMOTION` is explicitly acceptable;
- any promoted type preserves private codec representation and improves a concrete repeated-operation workload.

---

# Phase 7 — external-consumer and public-boundary verification

## 7.1 Positive direct carrier consumer

Create a temporary consumer outside the committed workspace using `stegoeggo-stego` directly and compile examples for:

```text
raw LSB
in-place LSB
framed LSB
raw JPEG
framed JPEG
tiled LSB
tiled JPEG
framed tiled recovery
prepared JPEG only if approved
```

Do not commit the temporary consumer.

## 7.2 Root re-export consumer

Ensure equivalent intended additions are usable under:

```rust
stegoeggo::stego::...
```

without exposing application-support.

## 7.3 Compile-fail containment

Retain/add compile-fail doctests proving downstream code cannot import:

```text
jpeg_transcoder::JpegTranscoder
DctStegoF5
Coefficients
lsb_internal permutation helpers
hidden application search internals from the default facade
```

If `application-support` is explicitly feature-enabled, it still must not expose internal JPEG structs.

### Phase 7 acceptance criteria

- direct carrier-only users can perform all stable operation styles;
- root re-export remains coherent;
- private implementation boundary is mechanically demonstrated.

---

## 8. Documentation requirements

Update at minimum:

```text
stegoeggo-stego/README.md
docs/carrier-crate.md
examples/generic_stego.rs
architecture/protected-steganography.md
architecture/pipeline.md
STABILITY.md if its public API commitment list requires the additions
.skills/stegoeggo-conventions/SKILL.md
```

Document a concise API taxonomy:

```text
raw       -> caller controls payload length
framed    -> self-describing recovery
in-place  -> avoid full-image clone for owned raster buffers
tiled     -> spatial/crop-oriented repetition with bounded recovery
prepared  -> repeated JPEG operations, only if Plan-078 evidence justified it
```

Do not claim crop survival beyond the actual intact-tile/carrier conditions.

---

## 9. Focused verification

Expected commands:

```bash
cargo test -p stegoeggo-stego --all-features
cargo test -p stegoeggo-stego --doc
cargo check -p stegoeggo-stego
cargo check -p stegoeggo-stego --features application-support
cargo test -p stegoeggo --test public_stego_api --all-features
cargo test -p stegoeggo --all-features legacy
cargo test -p stegoeggo --all-features tiled
cargo test -p stegoeggo --all-features jpeg
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

---

## 10. Final acceptance criteria

Plan 079 is complete only when:

1. The status ledger existed before product edits.
2. Stable generic tiled LSB raw/in-place/framed operations exist.
3. Stable generic tiled JPEG raw/framed operations exist.
4. Tiled recovery is bounded and validates declared framed lengths before full allocation.
5. Public API contains no StegoEggo rights/payload/auth types.
6. JPEG implementation structs remain private.
7. LSB permutation helpers remain private.
8. Root current ordinary LSB embedding/extraction uses stable public APIs where semantics match.
9. Root current ordinary JPEG embedding uses stable public APIs.
10. Root current tiled embedding uses stable public tiled APIs.
11. Hidden application support contains only justified compatibility/search functionality.
12. Legacy V1/V2 extraction remains passing.
13. Direct standalone carrier consumer examples compile.
14. Root `stegoeggo::stego` re-export exposes intended stable additions.
15. Prepared JPEG disposition follows Plan-078 evidence and is explicitly recorded.
16. Public/private compile evidence passes.
17. Documentation accurately distinguishes raw/framed/in-place/tiled/prepared operation styles.
18. `./scripts/check.sh` passes.
19. No unrelated version/dependency/release/CI change occurs.

---

## 11. Non-goals

Do not use this plan to:

- expose JPEG coefficient/parser/Huffman/F5 internals;
- make legacy V1/V2 formats recommended public carrier APIs;
- add a generalized `Carrier` trait;
- add dynamic plugins;
- add encryption/authentication to generic framing;
- add new media formats;
- add language bindings;
- change rights-policy or payload-v3 semantics;
- publish a release.