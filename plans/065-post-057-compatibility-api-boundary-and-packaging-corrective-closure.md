# Plan 065: Post-057 Compatibility, API-Boundary, and Packaging Corrective Closure

Status: Ready for implementation

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Corrects the over-broad completion claim recorded by Plan 064.

Audited corrective baseline: `main` at `703d20edfa43d21caa8a975bef072877e45c2efb`

Authoritative implementation ledger to create before source edits: `plans/065-status.md`

---

## 1. Purpose

Plans 058-064 materially improved StegoEggo's steganography architecture, but the post-closure source audit found several concrete residuals that make Roadmap 057 only partially complete:

1. the Plan 061 legacy adapter does not preserve all `ProtectionContext` semantics;
2. legacy `Light` was collapsed into the same full-payload hidden-marker mode as `Standard`;
3. `ProtectionPipeline::process()` still retains an independent level-based execution implementation even though the top-level convenience APIs now use the canonical request/plan executor;
4. the standalone `stegoeggo-stego` crate exposes JPEG parser/coefficient/F5 internals and many low-level LSB implementation helpers as stable public API;
5. the shared LSB pixel mutation helper can wrap channel values `0 -> 255` or `255 -> 0` instead of making a bounded ±1 change;
6. the new carrier package manifest and release check disagree about version wiring, and the package declares files that are not present in the carrier crate;
7. Plan 064 claims that status ledgers 058, 059, 060, 061, and 063 are present, but they are not committed on `main`;
8. Roadmap 057 still says `Ready for implementation` while Plan 064 claims `COMPLETE`.

This plan closes those exact residuals. It does not reopen the carrier algorithms, container correctness work, rights semantics, XMP/WebP correctness, release cadence, or CI design beyond the minimum needed to prove the new carrier package can actually be packaged.

The desired end state remains:

```text
legacy API ---- compatibility translation ----+
                                              |
canonical ProtectionRequest ------------------+
                                              v
                                  ResolvedProtectionPlan
                                              |
                                              v
                                     one executor path
                                    /                \
                         rights/application       generic carrier
                                                   crate facade
```

---

## 2. Retained work — do not rewrite it

The following work is accepted unless a focused regression test proves otherwise:

- V2 LSB uses RGB carrier slots and the corrected arbitrary-size permutation;
- legacy LSB extraction remains supported;
- generic carrier mechanics live in `stegoeggo-stego`;
- `SteganographyProtector` delegates carrier mechanics rather than owning duplicate LSB/F5 implementations;
- supported JPEG embedding uses one normal coefficient embed/encode pass instead of the old repeated round-trip retry loop;
- tiled LSB embedding operates directly on regions without allocating temporary tile images;
- canonical request execution uses `ResolvedProtectionPlan` directly for stego and metadata stages;
- public generic raw LSB/JPEG APIs and generic framing remain conceptually supported;
- the carrier/application dependency direction remains one-way: `stegoeggo -> stegoeggo-stego`;
- no release workflow or CI matrix is desired;
- release remains manual.

Do not redesign payload-v3, PLUS/DMI rules, XMP filtering, WebP container handling, JPEG Huffman correctness, provenance, signatures, detached manifests, or unrelated CLI behavior in this plan.

---

## 3. Primary files and scope

Expected primary product files:

```text
src/lib.rs
src/types.rs
src/protected/resolve.rs
src/protected/steganography.rs
stegoeggo-stego/Cargo.toml
stegoeggo-stego/src/lib.rs
stegoeggo-stego/src/lsb.rs
stegoeggo-stego/src/jpeg.rs
stegoeggo-stego/src/jpeg_transcoder/mod.rs
Cargo.toml
scripts/release-check.sh
```

Expected tests/documentation/planning files:

```text
tests/integration.rs
tests/public_stego_api.rs
stegoeggo-stego/README.md
stegoeggo-stego/LICENSE                  # only if chosen packaging approach requires a local license file
architecture/pipeline.md
architecture/protected-steganography.md
architecture/overview.md
README.md
plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md
plans/058-status.md
plans/059-status.md
plans/060-status.md
plans/061-status.md
plans/063-status.md
plans/064-status.md
plans/065-status.md
```

Files outside this list may be changed only when a direct compiler/test dependency requires it. Record every such expansion in `065-status.md`.

---

## 4. Explicit non-goals

Do not use Plan 065 to:

- add new image formats;
- add a new payload version;
- change F5 carrier math except if required by a focused regression discovered during this plan;
- remove legacy LSB extraction;
- add encryption to the generic frame;
- add new authentication algorithms;
- add Python/C/WASM bindings;
- introduce a carrier trait hierarchy;
- split `stegoeggo-stego` into additional crates;
- add benchmarks as CI gates;
- add release automation;
- bump versions, publish crates, create tags, or create GitHub Releases;
- rewrite historical plans to pretend evidence existed earlier than it did.

---

# Phase 0 — establish a truthful corrective ledger before source edits

## 0.1 Create `plans/065-status.md`

The first implementation commit must create the ledger before product source changes.

Because `plans/` is ignored by `.gitignore`, explicitly verify the file is tracked. A local implementer should use:

```bash
git add -f plans/065-status.md
git ls-files --error-unmatch plans/065-status.md
```

The ledger starts with these rows as `OPEN`:

```text
legacy Light policy equivalence
legacy Light marker-mode equivalence
legacy explicit DMI override
legacy metadata-injection equivalence
legacy legal-claims default equivalence
legacy explicit stego redundancy propagation
legacy content-hash propagation
legacy timestamp propagation
legacy output/JPEG options propagation
legacy resource-limit propagation
ProtectionPipeline canonical delegation
LSB 0/255 bounded mutation
carrier JPEG internal visibility containment
carrier LSB helper visibility containment
carrier package README/license completeness
carrier exact-version dependency wiring
carrier cargo package dry-run
missing 058/059/060/061/063 status ledgers
Plan 064 correction note
Roadmap 057 truthful status
final check.sh
```

Do not mark rows closed from commit messages alone.

## 0.2 Record the actual implementation baseline

Record:

```text
starting HEAD
working tree status
current workspace members
current root/carrier/CLI versions
current root -> carrier dependency declaration
```

Do not assume the planning baseline is still HEAD.

## 0.3 Correct planning disposition without erasing history

Before final closure, Plan 065 must ensure:

- Roadmap 057 is marked `PARTIAL — reopened by Plan 065` while this work is open;
- `plans/064-status.md` receives a short correction note stating its COMPLETE disposition was superseded by the post-closure Plan 065 audit;
- the existing Plan 064 evidence remains intact below that note.

Do not rewrite Plan 064 as though it originally discovered these defects.

### Phase 0 acceptance criteria

- `plans/065-status.md` exists and is tracked before product edits;
- every corrective row starts `OPEN`;
- actual baseline SHA is recorded;
- Roadmap 057 and Plan 064 no longer make an unqualified current COMPLETE claim while Plan 065 is open.

---

# Phase 1 — reconstruct and lock the legacy compatibility contract

This phase is required before changing `request_from_legacy()`.

## 1.1 Use the pre-Plan-061 implementation as behavioral evidence

Inspect source immediately before Plan 061 canonicalization, preferably the retained state around Plan 060 (`effcb0a0209140c890d50bb1ba494802608bfe68`) and the immediately preceding public API behavior.

Build a field-by-field matrix for `ProtectionContext` and `ProtectionLevel`.

At minimum include:

| Legacy input | Required canonical meaning |
|--------------|----------------------------|
| `Disabled` | byte-for-byte passthrough |
| `Light` default policy | `ProtectionLevel::default_policy()` result (`Unspecified`) |
| `Standard` default policy | `ProhibitedAiMlTraining` |
| explicit `DmiValue` | overrides level default |
| `inject_metadata` | preserves three-state legacy behavior |
| `inject_legal_claims` | preserves documented legacy default and explicit true/false behavior |
| `seed` | exact propagation |
| `intensity` | exact propagation |
| explicit `stego_redundancy` | exact propagation; do not silently replace with intensity-derived redundancy |
| `content_hash` | exact propagation into v3 payload generation |
| output format | exact propagation |
| JPEG quality | exact propagation |
| progressive JPEG | exact propagation |
| tile size | exact propagation |
| max dimension | exact propagation |
| metadata update policy | exact propagation |
| timestamp override | exact propagation |
| MAC key | exact propagation and warning semantics |
| legal metadata | injected only under the same legacy conditions as before Plan 061 |
| resource limits | exact propagation |

If source behavior and current comments disagree, record the discrepancy in `065-status.md` and use the last intentional documented/tested contract, not an accidental intermediate implementation.

## 1.2 Add focused failing tests before the adapter fix

Add tests that demonstrate the current residuals. Suggested names:

```text
legacy_light_default_policy_is_unspecified
legacy_light_uses_seed_only_marker_png
legacy_light_uses_qtable_seed_only_jpeg
legacy_standard_uses_full_hidden_payload
legacy_explicit_dmi_overrides_level_default
legacy_legal_claims_none_does_not_inject_claim_fields
legacy_legal_claims_true_injects_claim_fields
legacy_explicit_redundancy_reaches_resolved_plan
legacy_content_hash_reaches_v3_payload
legacy_timestamp_override_reaches_notice
legacy_resource_limits_reach_resolved_plan
```

These tests must verify semantics, not merely that processing succeeds.

For Light vs Standard, verify actual hidden-marker behavior:

```text
Light PNG/WebP:
  seed marker recoverable
  no full v3 payload reported as embedded

Light JPEG:
  Q-table seed hint recoverable when structurally supported
  no full DCT payload reported as embedded

Standard:
  full v3 payload attempted according to normal carrier/capacity rules
```

### Phase 1 acceptance criteria

- the compatibility matrix exists in `065-status.md`;
- every legacy field with externally observable behavior has a disposition;
- focused tests expose the currently incorrect Light/default/field propagation behavior before the implementation fix;
- no product source is changed before the matrix is recorded.

---

# Phase 2 — represent the existing Light marker semantics in the canonical model

The current adapter maps both Light and Standard to `HiddenMarkerMode::BestEffort`, which loses information before execution. Do not fix this by adding another independent legacy executor.

## 2.1 Add one explicit canonical seed-only marker mode

Preferred model:

```rust
pub enum HiddenMarkerMode {
    Disabled,
    SeedOnly,
    BestEffort,
    Tiled { tile_size: u32 },
}
```

`SeedOnly` is not a new feature. It gives a canonical name to the existing Light behavior already implemented historically:

```text
PNG / lossless WebP -> minimal fixed seed LSB marker
JPEG               -> quantization-table seed marker
```

If `HiddenMarkerMode` is `#[non_exhaustive]`, adding the variant remains additive for downstream exhaustive matching constraints. If it is not non-exhaustive, evaluate semver impact in the ledger before editing and use the smallest compatible representation that still keeps execution canonical.

Do not introduce names such as `LegacyLight` into the canonical API.

## 2.2 Route `SeedOnly` through `ResolvedProtectionPlan`

`resolve_request()` and `process_plan_bytes()` must understand this mode directly.

Required executor behavior:

```text
Disabled  -> metadata-only/no marker as configured
SeedOnly  -> minimal seed marker + configured rights metadata
BestEffort -> full payload carrier + configured rights metadata
Tiled     -> full tiled payload carrier + configured rights metadata
```

Use existing plan-aware helpers such as the retained minimal LSB/Q-table functions. Do not reconstruct `ProtectionContext`.

## 2.3 Fix legacy level translation

`request_from_legacy()` must map:

```text
Disabled -> HiddenMarkerMode::Disabled
Light    -> HiddenMarkerMode::SeedOnly
Standard -> HiddenMarkerMode::BestEffort or Tiled when requested
```

Default policy must use the single canonical compatibility mapping:

```rust
level.default_policy()
```

unless the caller supplied an explicit DMI/policy override.

Do not hard-code a second Light policy table inside `request_from_legacy()`.

## 2.4 Authentication semantics for SeedOnly

A seed-only marker does not carry an authenticated hidden payload.

Do not report HMAC payload authentication as successful merely because a seed marker exists.

Preserve the historical warning/report behavior for legacy Light + MAC configuration. If the canonical `ProtectionRequest` permits `SeedOnly + Hmac`, resolver/report behavior must make the absence of an authenticated hidden payload explicit rather than silently upgrading SeedOnly to a full payload.

### Phase 2 acceptance criteria

- canonical resolved plans can represent seed-only hidden-marker intent without a legacy side channel;
- legacy Light maps to the seed-only plan path;
- legacy Standard maps to full payload behavior;
- Light default policy comes from `ProtectionLevel::default_policy()` and is `Unspecified` absent an explicit override;
- Light PNG/WebP/JPEG focused tests prove minimal-marker behavior;
- no new independent Light executor exists outside the canonical plan path.

---

# Phase 3 — preserve every remaining legacy `ProtectionContext` field

## 3.1 Explicit stego redundancy

The canonical request/plan model must be able to preserve an explicit legacy redundancy setting.

Preferred location: processing mechanics, for example:

```rust
ProcessingOptions {
    ...
    stego_redundancy: Option<usize>,
}
```

Requirements:

- validate the existing supported range;
- `request_from_legacy()` copies `ProtectionContext::stego_redundancy()`;
- `ResolvedProtectionPlan` exposes the resolved value;
- plan-driven embedding uses the explicit value when present;
- intensity-derived redundancy is only the fallback when no explicit redundancy was requested.

Do not keep a second redundancy calculation in the legacy adapter.

## 3.2 Content hash

The canonical request/plan path currently must not zero a legacy caller's content hash.

Add the smallest request/plan representation needed to carry the existing optional `[u8; 4]` content hash through payload generation.

Preferred requirements:

```text
ProtectionRequest optional content hash
  -> ResolvedProtectionPlan optional content hash
  -> generate_payload_for_plan
```

Do not redesign the content-binding format or expand hash length in this plan.

## 3.3 Legal-claim default behavior

Remove any `unwrap_or(true)` behavior that contradicts the documented legacy three-state contract.

The compatibility adapter must use one helper/decision for whether legal fields are included. Tests must distinguish:

```text
None
Some(false)
Some(true)
```

and verify emitted metadata, not only request construction.

## 3.4 Other fields

Use the Phase 1 matrix to prove exact propagation of:

- seed;
- intensity;
- explicit DMI;
- metadata injection;
- JPEG quality;
- progressive flag;
- tile size;
- max dimension;
- metadata update policy;
- timestamp override;
- MAC key;
- resource limits;
- output format.

If any field cannot be represented in the canonical request/plan model, add only the smallest semantically appropriate field. Do not create a `LegacyOverrides` bag or carry an entire `ProtectionContext` inside the plan.

### Phase 3 acceptance criteria

- explicit redundancy is no longer lost;
- content hash is no longer zeroed/lost when set by legacy callers;
- legal-claim default behavior matches the established contract;
- every Phase 1 matrix row has a focused passing test or a recorded intentionally unsupported/deprecated disposition approved by the existing API contract;
- no `ResolvedProtectionPlan -> ProtectionContext` conversion is reintroduced.

---

# Phase 4 — make `ProtectionPipeline` a real compatibility adapter

The top-level convenience functions now route through canonical request execution, but the public `ProtectionPipeline::process()` still owns an independent Light/Standard implementation.

## 4.1 Inventory public `ProtectionPipeline` methods

Before deleting private helpers, record all public methods and external behavior.

At minimum inspect:

```text
ProtectionPipeline::new
ProtectionPipeline::process
any public byte-processing method
Clone / Default behavior
```

## 4.2 Delegate public processing into the canonical path

Preferred `process()` shape:

```text
Disabled:
  preserve Cow::Borrowed no-op behavior when possible

Light / Standard:
  encode the supplied DynamicImage using the legacy context/output semantics
  call the same legacy->request->plan byte executor used by process_image_bytes
  decode result to DynamicImage
  return Cow::Owned
```

It is acceptable for `ProtectionPipeline::process()` to call the already-canonical top-level `process_image()` if that does not recurse.

After delegation is proven, delete private duplicate helpers that are no longer reachable, including old Light/Standard orchestration functions where possible.

Do not retain dead `Arc<...Protector>` fields solely to preserve the old implementation shape. If the fields become unused and are private, simplify the struct. Preserve public constructor/Clone/Default behavior.

## 4.3 Prove equivalence

Required focused tests:

```text
protection_pipeline_light_matches_process_image
protection_pipeline_standard_matches_process_image
protection_pipeline_disabled_borrows_input
protection_pipeline_and_byte_api_match_policy_and_marker_semantics
```

Do not use byte equality for formats where deterministic metadata timestamps make byte equality inappropriate. Compare semantic output/verification results using deterministic timestamp override when practical.

### Phase 4 acceptance criteria

- no public `ProtectionPipeline` method executes an independent Light/Standard pipeline;
- all non-disabled processing delegates into canonical request/plan execution;
- obsolete duplicate orchestration helpers are removed;
- Disabled retains its no-op/borrow behavior where currently promised;
- focused equivalence tests pass.

---

# Phase 5 — fix bounded LSB mutation at channel extrema

`embed_bit_in_pixel()` currently uses wrapping arithmetic. A required LSB change at a channel boundary can therefore produce a full-scale mutation such as `0 -> 255` or `255 -> 0`.

## 5.1 Required mutation rule

When the existing LSB already equals the requested bit:

```text
new == old
```

When it differs:

```text
old == 0   -> new == 1
old == 255 -> new == 254
1..=254    -> new == old - 1 or old + 1, chosen by the existing deterministic direction rule
```

In every changed case:

```text
abs(new - old) == 1
(new & 1) == requested_bit
```

Do not use wrapping arithmetic for pixel mutation.

## 5.2 Test all boundary combinations

Add direct carrier tests for each RGB channel and representative values:

```text
0
1
2
127
128
253
254
255
```

For both requested bits, verify:

- correct LSB;
- no change when already matching;
- absolute delta ≤ 1;
- alpha unchanged;
- other RGB channels unchanged.

Also add one black-image and one white-image arbitrary payload round-trip to ensure embedding does not introduce 0↔255 salt-and-pepper changes.

### Phase 5 acceptance criteria

- no LSB mutation uses wrapping add/subtract;
- every changed carrier byte differs by exactly 1;
- channel-boundary tests pass;
- arbitrary payload round-trips still pass.

---

# Phase 6 — shrink the standalone carrier's stable public surface

Roadmap 057 and Plan 062 required JPEG codec internals to remain opaque. After Plan 063, `stegoeggo-stego` currently exposes them directly because `jpeg_transcoder` and its internals are public.

This must be fixed before treating the standalone package API as stable.

## 6.1 Establish an explicit public API allowlist

The intended standalone API should be approximately:

```text
stegoeggo_stego::lsb
  LsbConfig
  capacity
  embed
  extract

stegoeggo_stego::jpeg
  JpegConfig
  JpegSupport
  probe_support
  capacity
  embed
  extract
  embed_seed_hint
  extract_seed_hint

stegoeggo_stego::frame
  FrameHeader
  encode
  decode
  decode_prefix
  documented frame constants

stegoeggo_stego root
  CapacityReport
  EmbedReport or final carrier-neutral public result types
  StegoError
  StegoResult
  JpegUnsupportedReason
  only the embed status/path summary types required by the root application API
  is_progressive_jpeg if retained as an intentional public compatibility API
```

Everything else is private implementation unless a pre-existing published root API requires a specific compatibility re-export.

## 6.2 Hide JPEG internals

Do not publicly expose as stable standalone API:

```text
jpeg_transcoder module
JpegHeader
HuffmanTable
CoefficientDecoder
CoefficientEncoder
JpegTranscoder
DctStegoF5
Coefficients = HashMap<u8, Vec<[i16; 64]>>
DctUnsupportedReason internal codec form
entropy/header parser internals
```

Preferred module declaration:

```rust
mod jpeg_transcoder;
```

Public `jpeg` facade functions use those types internally and map errors/reasons into the small public `StegoError` / `JpegUnsupportedReason` vocabulary.

The root crate's fuzz helper may require low-level access. Do not make the entire module public merely for fuzzing. Preferred solutions, in order:

1. move the fuzz-only parser facade into a deliberately feature-gated public function on `stegoeggo-stego` that returns only the minimum fuzz-compatible result required;
2. move the fuzz target implementation into the carrier crate's internal/unit fuzz surface if practical;
3. expose one explicitly unstable `#[cfg(feature = "fuzz")]` support module only if required.

Do not expose low-level internals under default features.

## 6.3 Hide low-level LSB implementation helpers

Review every `pub fn` in `stegoeggo-stego/src/lsb.rs`.

Functions such as these should normally be `pub(crate)` or private:

```text
splitmix64
stego_permutation
stego_permutation_v2
carrier_v2_slot_to_pixel_channel
lsb_available_slots
lsb_required_capacity_v2
lsb_required_slots_legacy
bytes_to_bits
bits_to_bytes
embed_bit_in_pixel
legacy embed/extract primitives
seed-fallback embed/extract helpers
raw tiled implementation helpers
```

Keep public only what is intentionally documented for generic callers or required by an established root compatibility surface.

The root application adapter can call crate-public carrier helpers only if they are actually part of the carrier crate API. Prefer adding a small explicitly named application-support facade over making every algorithm helper public.

If root-private compatibility behavior needs access that cannot cross crate privacy, create a narrowly named `#[doc(hidden)]`/unstable support module only for the root application and document that it is not stable public API. Do not put JPEG coefficient structures into that support API.

## 6.4 Re-check root re-exports

`stegoeggo::stego` must continue to expose the intended Plan 062 generic facade.

The root should not accidentally broaden visibility beyond the standalone crate's intended surface.

### Phase 6 acceptance criteria

- default `stegoeggo-stego` API does not expose `jpeg_transcoder`;
- `JpegHeader`, coefficient maps, Huffman codec types, and `DctStegoF5` are inaccessible to normal downstream consumers;
- low-level LSB permutation/bit helpers are not part of the normal public API;
- public generic LSB/JPEG/frame examples still compile;
- root `stegoeggo::stego` facade remains usable;
- root fuzzing still compiles under the `fuzz` feature without making default JPEG internals public.

---

# Phase 7 — decide and correct the pixel-domain LSB result boundary

The new generic LSB API accepts `RgbaImage` but currently returns PNG-encoded bytes through the same `EmbedReport` used by JPEG. That couples a pixel-domain carrier operation to PNG encoding and forces generic callers to decode the result back to pixels for extraction.

This is a corrective API-boundary decision, not a new feature.

## 7.1 First determine whether the new API has been released

Record in `065-status.md` whether the Plan 062/063 generic API has appeared in a published immutable crate version/tag.

Use repository/tag/package evidence available in the environment. Do not publish anything.

## 7.2 If the current generic API is not yet released

Prefer correcting it now.

Acceptable target shapes include:

```rust
pub struct EmbedReport<T> {
    pub embedded: bool,
    pub output: T,
    ...
}

pub fn lsb::embed(...) -> Result<EmbedReport<RgbaImage>, StegoError>;
pub fn jpeg::embed(...) -> Result<EmbedReport<Vec<u8>>, StegoError>;
```

or two small domain-specific result types if that is clearer.

Requirements:

- `lsb::embed` does not PNG-encode internally;
- `lsb::extract` accepts the returned pixel image directly;
- JPEG remains encoded-byte in/out;
- capacity fields remain mechanically consistent;
- examples no longer encode/decode merely to continue an LSB round-trip.

Do not add a generic abstraction more complicated than a single generic report or two concrete result structs.

## 7.3 If the current API is already published and changing it would be breaking

Do not silently break it in Plan 065.

Record `DEFERRED — semver` in the ledger, keep current public behavior, and optionally add a new additive pixel-domain method only if it is clearly smaller than carrying the awkward boundary forward.

This conditional row does not block the rest of Plan 065 if a published semver contract prevents cleanup.

### Phase 7 acceptance criteria

One of these is recorded and proven:

```text
CORRECTED BEFORE RELEASE
or
DEFERRED — PUBLISHED SEMVER CONTRACT
```

If corrected, LSB embedding no longer performs an implicit PNG encode.

---

# Phase 8 — make the standalone carrier package actually packageable

## 8.1 Correct root dependency version wiring

The root manifest and release check must agree.

For a publishable path dependency, use both path and exact version, consistent with the project's lockstep release policy, for example:

```toml
stegoeggo-stego = { path = "stegoeggo-stego", version = "=0.3.2" }
```

Use the actual current package version; do not bump it in this plan.

The release check's exact-version assertion must match the manifest's declared requirement.

## 8.2 Supply package metadata files

`stegoeggo-stego/Cargo.toml` declares a local README and package include entries. Make the declaration truthful.

Preferred simple solution:

- add `stegoeggo-stego/README.md` describing only the generic carrier crate;
- ensure the package contains applicable MIT license text using the simplest Cargo-valid repository convention;
- remove any nonexistent include path rather than relying on it accidentally.

The carrier README should cover:

```text
purpose: arbitrary-payload image steganography
LSB API
JPEG API and supported subset
frame API
seed is not encryption
CRC is not authentication
fragility under recompression/transformation
relationship to root stegoeggo rights/provenance crate
```

Do not duplicate the root project's large legal-rights README.

## 8.3 Prove package construction

Required local commands:

```bash
cargo package -p stegoeggo-stego --allow-dirty
cargo package -p stegoeggo-stego --list
./scripts/release-check.sh --allow-dirty
```

If `release-check.sh` intentionally cannot complete a later package because an exact upstream package version is not yet published, distinguish that expected publication-order condition from a carrier package failure. The carrier's own dry-run must succeed.

No `cargo publish` is authorized.

### Phase 8 acceptance criteria

- root dependency has path + exact current version;
- carrier package metadata refers only to files that exist/package correctly;
- carrier README exists and is carrier-specific;
- `cargo package -p stegoeggo-stego --allow-dirty` succeeds;
- `cargo package -p stegoeggo-stego --list` contains the expected source, README, and license metadata/files;
- release-check version-lockstep logic agrees with Cargo metadata;
- nothing is published.

---

# Phase 9 — backfill the missing planning evidence truthfully

Plan 064 claims all Plans 058-064 have status ledgers, but `058-status.md`, `059-status.md`, `060-status.md`, `061-status.md`, and `063-status.md` are absent from `main`.

## 9.1 Create concise retrospective ledgers

Create:

```text
plans/058-status.md
plans/059-status.md
plans/060-status.md
plans/061-status.md
plans/063-status.md
```

Each must clearly say:

```text
RETROSPECTIVE LEDGER — created by Plan 065 after implementation
```

Do not claim it existed before source changes.

Each ledger needs only:

```text
original plan baseline
implementation commits
actual implemented scope
focused tests/evidence known from commits/current source
residuals discovered later
final disposition after Plan 065
```

For Plan 061 and Plan 063, explicitly record that Plan 065 corrected residuals after their original implementation.

## 9.2 Ensure ignored files are tracked

Because `plans/` is ignored, use force-add where necessary and verify:

```bash
git ls-files plans/058-status.md plans/059-status.md plans/060-status.md plans/061-status.md plans/063-status.md plans/065-status.md
```

All six files must appear.

### Phase 9 acceptance criteria

- every missing status ledger exists on the branch and is tracked;
- retrospective ledgers are labeled retrospective;
- no ledger fabricates pre-edit evidence;
- Plan 061/063 ledgers acknowledge the Plan 065 corrections.

---

# Phase 10 — final semantic and architecture closure

## 10.1 Required focused test matrix

At minimum run focused tests proving:

### Legacy compatibility

```text
Disabled byte passthrough
Light default policy
Light seed-only PNG
Light seed-only WebP
Light seed-only JPEG
Standard full payload PNG
Standard full payload JPEG
explicit DMI
metadata injection true/false/default
legal claims None/false/true
explicit redundancy
content hash
timestamp override
tile size
resource limits
MAC warning/report behavior
```

### Pipeline convergence

```text
ProtectionPipeline vs top-level canonical API
DynamicImage vs byte API
parallel wrappers use canonical byte path
async wrappers remain delegating wrappers
```

### Carrier correctness/API

```text
V2 LSB round-trip
legacy extraction
LSB 0/255 bounded mutation
raw arbitrary LSB payload
raw arbitrary JPEG payload
JPEG container preservation
unsupported/progressive JPEG classification
frame round-trip/corruption/trailing bytes
public API examples compile
internal JPEG types not default public API
```

### Packaging

```text
carrier cargo package dry-run
carrier cargo package --list
version lockstep check
```

## 10.2 Required project verification

Run:

```bash
./scripts/check.sh
```

Record exact result in `plans/065-status.md`.

Do not add more mandatory CI jobs.

## 10.3 Documentation reconciliation

Update architecture/docs only after final source shape is stable.

At minimum verify:

```text
README generic carrier examples
architecture/overview.md
architecture/pipeline.md
architecture/protected-steganography.md
architecture/types.md if HiddenMarkerMode/processing options change
RELEASING.md / scripts/release-check.sh package order
```

Documentation must distinguish:

```text
SeedOnly/minimal marker
BestEffort/full payload marker
legacy compatibility adapter
canonical plan executor
standalone carrier stable facade
private JPEG codec internals
```

## 10.4 Correct final planning status

Only after all blocking rows are closed:

- mark Plan 065 `COMPLETE` in `065-status.md`;
- update Roadmap 057 to `COMPLETE — closed by Plan 065`;
- add a final note to `064-status.md` that its original closure was superseded and corrected by Plan 065;
- preserve all historical evidence.

If any blocking criterion remains open, Roadmap 057 must remain `PARTIAL` and identify the exact row.

---

# 11. Plan 065 acceptance criteria

Plan 065 is complete only when every blocking item below is true:

1. `plans/065-status.md` was created and tracked before product edits.
2. Legacy `Light` defaults to `RightsPolicy::Unspecified` unless explicitly overridden.
3. Legacy `Light` uses canonical seed-only marker behavior rather than a full V3 hidden payload.
4. Legacy `Standard` retains full hidden-payload behavior.
5. The canonical plan model can represent seed-only hidden-marker intent without a legacy side channel.
6. Explicit legacy DMI overrides are preserved.
7. Legacy metadata-injection semantics are preserved.
8. Legacy legal-claim `None`/false/true behavior matches the established pre-Plan-061 contract.
9. Explicit legacy stego redundancy reaches plan-driven embedding unchanged.
10. Legacy content hash reaches plan-driven v3 payload generation unchanged.
11. Legacy seed, intensity, timestamp, JPEG options, tile size, max dimension, metadata update policy, MAC key, output format, and resource limits are preserved.
12. No `ResolvedProtectionPlan -> ProtectionContext` adapter is reintroduced.
13. `ProtectionPipeline` no longer contains an independent Light/Standard execution implementation.
14. Disabled `ProtectionPipeline` behavior remains a no-op/borrow where promised.
15. LSB mutation never wraps `0 <-> 255`; every changed carrier byte differs by exactly 1.
16. Default standalone carrier API does not expose JPEG parser/header/Huffman/coefficient/F5 internals.
17. Low-level LSB permutation/bit/legacy implementation helpers are not normal stable public API unless explicitly justified by an existing compatibility contract.
18. Public generic LSB/JPEG/frame APIs remain usable from both `stegoeggo-stego` and the root `stegoeggo::stego` facade.
19. Phase 7 records a truthful LSB pixel-result boundary disposition; if unreleased, implicit PNG encoding is removed from the pixel-domain API.
20. Root `stegoeggo` declares the carrier dependency with both path and exact current version.
21. `stegoeggo-stego` package README/license/include declarations are truthful.
22. `cargo package -p stegoeggo-stego --allow-dirty` succeeds.
23. The release-check version-lockstep test agrees with the manifests.
24. `plans/058-status.md`, `059-status.md`, `060-status.md`, `061-status.md`, and `063-status.md` exist, are tracked, and are labeled retrospective.
25. `plans/064-status.md` contains a correction/supersession note rather than an unqualified current closure claim.
26. Roadmap 057 is marked complete only after all blocking Plan 065 rows are closed.
27. Existing payload-v1/v2/v3 extraction, rights metadata, verification, JPEG container preservation, progressive fallback, and WebP/XMP tests remain passing.
28. `./scripts/check.sh` passes at final closure.
29. No version bump, publication, tag, release, CI matrix, or automated release workflow is added.

---

# 12. Suggested implementation commits

Keep the corrective work reviewable. A suitable sequence is:

```text
plans: open Plan 065 corrective ledger and reopen Roadmap 057

compat: restore legacy Light and ProtectionContext translation semantics

pipeline: make ProtectionPipeline delegate to canonical plan execution

stego: bound LSB mutation and contain carrier internals

stego: correct pixel-domain public result boundary
    # only if Phase 7 chooses CORRECTED BEFORE RELEASE

package: make stegoeggo-stego package metadata and version wiring valid

plans: backfill truthful 058-063 retrospective ledgers

docs: close Plan 065 and Roadmap 057 with exact evidence
```

Do not combine all source corrections and evidence closure into one large commit if avoidable.

---

# 13. Stop conditions

Stop and mark Plan 065 `PARTIAL` rather than escalating scope if fixing these residuals appears to require:

- a third workspace crate;
- reintroducing `ProtectionContext` into canonical request execution;
- a payload-v4 format;
- making JPEG coefficient internals public to satisfy the root application;
- a new CI matrix;
- an automated publication system;
- breaking a confirmed already-published generic API without a separately approved semver plan.

A small amount of additive request/plan state is acceptable when it represents behavior the legacy public API already exposed. A new parallel configuration system is not.

---

# 14. Final handoff condition

The implementer should leave the repository with this single mental model:

```text
ProtectionLevel + ProtectionContext
        |
        | compatibility translation only
        v
ProtectionRequest
        |
        v
ResolvedProtectionPlan
        |
        +--> Disabled
        +--> SeedOnly
        +--> BestEffort
        +--> Tiled
        |
        v
one plan executor
        |
        +--> rights metadata application
        +--> StegoEggo payload adapter
                   |
                   v
             stegoeggo-stego
             small stable facade
             private carrier internals
```

The standalone carrier package must be independently understandable and packageable, while the root rights-protection API must preserve its legacy callers without maintaining a second processing implementation.
