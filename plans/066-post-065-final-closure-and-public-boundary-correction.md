# Plan 066: Post-065 Final Closure and Public-Boundary Correction

Status: Ready for implementation

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Corrective predecessor: `plans/065-post-057-compatibility-api-boundary-and-packaging-corrective-closure.md`

Audited implementation baseline: `main` at `1c530a7fb7bd5935bd30946c72e1dc27409de451`

Authoritative implementation ledger to create before source edits: `plans/066-status.md`

---

## 1. Purpose

Plan 065 landed most of the intended corrective work successfully: canonical `SeedOnly` execution exists, Light's default policy is again `Unspecified`, explicit redundancy/content hash/timestamp settings reach the resolved plan, `ProtectionPipeline` delegates to the canonical path, LSB mutation is bounded at channel extrema, the standalone carrier package has local README/license files and exact-version wiring, and the public pixel-domain LSB API now returns pixels instead of an unnecessary PNG encoding.

A post-implementation audit found a much smaller residual set. Roadmap 057 must not be treated as complete until these specific items are closed:

1. legacy `Light` with a nonzero `tile_size` still maps to `HiddenMarkerMode::Tiled`, promoting Light from seed-only marking to full-payload tiled stego;
2. `stegoeggo-stego` makes `jpeg_transcoder`/`lsb_internal` private only nominally, then republishes their internals through public `#[doc(hidden)] __internal_*_facade` modules;
3. the public root crate contains a Plan-specific `#[doc(hidden)]` integration-test hook, `_plan065_internal_request_from_legacy`;
4. `ProtectionPipeline` still stores now-unused metadata/steganography protector `Arc`s after its execution methods were converted to canonical delegation;
5. public/documented legal-claim semantics disagree with the established pre-Plan-061 runtime contract for `inject_legal_claims == None`;
6. LSB rustdoc still describes/uses the old PNG-byte output after the API was corrected to return `RgbaImage`;
7. the release check does not model the actual manual publication dependency order cleanly: full root package verification can be blocked until the new carrier version exists on crates.io, just as CLI verification is blocked until the root library exists;
8. `plans/065-status.md` is still OPEN with all rows OPEN even though Roadmap 057 was changed to COMPLETE;
9. the backfilled 058/059/060/061/063 ledgers are not consistently labeled/reconciled as retrospective evidence, and at least 061/063 describe intermediate architecture as though it were current.

This plan closes only those items. It is intended to be the final corrective pass for Roadmap 057.

---

## 2. Required end state

At completion, the relevant architecture must be:

```text
legacy ProtectionLevel + ProtectionContext
                |
                v
      request_from_legacy()
                |
                v
        ProtectionRequest
                |
                v
     ResolvedProtectionPlan
                |
                v
       one executor path
          /         \
         /           \
rights/application   generic carrier facade
                         |
                         +-- public lsb API
                         +-- public jpeg API
                         +-- public frame API
                         +-- narrow non-default application support only if unavoidable

NOT public from the carrier:
  JpegHeader
  Coefficients
  JpegTranscoder
  DctStegoF5
  Huffman decoder/encoder types
  permutation functions
  bit conversion helpers
  raw legacy carrier helpers
```

Roadmap 057 may be marked COMPLETE only when current source, public API, package checks, tests, and planning ledgers all agree with that state.

---

## 3. Retained work — do not reopen

Unless a focused regression test proves a defect, retain the following work exactly:

- corrected V2 RGB-slot LSB carrier math and arbitrary-dimension permutation;
- legacy LSB extraction compatibility;
- bounded LSB mutation rule introduced by Plan 065;
- one-pass supported JPEG embedding optimization from Plan 060;
- direct tiled LSB region operation without per-tile image allocation;
- `stegoeggo-stego` as a separate workspace crate;
- public generic LSB/JPEG/frame concepts from Plan 062;
- generic LSB `EmbedReport<RgbaImage>` result boundary;
- `HiddenMarkerMode::SeedOnly` as the canonical representation of legacy Light marking;
- explicit `stego_redundancy`, `content_hash`, and `timestamp_override` propagation through `ProcessingOptions`/`ResolvedProtectionPlan`;
- exact path+version dependency wiring for root -> carrier;
- manual release cadence and simple CI policy.

Do not reopen PLUS/DMI vocabulary, metadata container correctness, WebP parsing, payload-v3 wire format, provenance/signatures, F5 math, or release automation.

---

## 4. Explicit non-goals

Do not use Plan 066 to:

- add a payload version;
- add image formats or new stego algorithms;
- alter LSB spread/redundancy math;
- redesign JPEG entropy coding;
- add encryption/authentication to the generic frame;
- add another crate;
- add a generic carrier trait hierarchy;
- add a new CI matrix or publication workflow;
- publish any crate;
- bump package versions;
- create a tag or GitHub Release;
- rewrite historical commits/plans to pretend the residuals were known earlier.

---

# Phase 0 — create a truthful Plan 066 ledger and reopen closure status

## 0.1 Create and track `plans/066-status.md` first

Before product edits, create `plans/066-status.md` and force-track it because `plans/` is ignored.

Required local verification:

```bash
git add -f plans/066-status.md
git ls-files --error-unmatch plans/066-status.md
```

Record:

```text
starting HEAD
working tree status
root/carrier/CLI versions
current root -> carrier dependency declaration
current Roadmap 057 status
current Plan 065 status
```

## 0.2 Initial rows

Start every row below as `OPEN`:

```text
R01 Light + tile_size remains SeedOnly
R02 Standard + tile_size remains Tiled
R03 default carrier API cannot name JPEG codec/coefficient internals
R04 default carrier API cannot name low-level LSB internals
R05 root application no longer consumes internal carrier structs/functions directly
R06 root fuzz support remains functional without default internal exposure
R07 Plan-specific public compatibility test hook removed
R08 ProtectionPipeline stale state removed
R09 legal-claim rustdoc matches established legacy runtime semantics
R10 LSB pixel-output rustdoc/examples match RgbaImage API
R11 carrier README/API claims match actual visibility
R12 release check models carrier -> root -> CLI publication ordering truthfully
R13 carrier package full dry-run succeeds before publication
R14 root/CLI pre-publication structural package checks succeed without false failure
R15 retrospective 058/059/060/061/063 ledgers are clearly labeled and current-disposition-correct
R16 Plan 065 status is reconciled truthfully
R17 Plan 064 correction note points to final Plan 066 closure
R18 Roadmap 057 is PARTIAL while Plan 066 is open
R19 focused test/API matrix passes
R20 ./scripts/check.sh passes
R21 final Roadmap 057 closure evidence is internally consistent
```

## 0.3 Reopen the roadmap before implementation

Change Roadmap 057 from its current COMPLETE claim to:

```text
Status: PARTIAL — final residuals tracked by Plan 066
```

Do not erase Plan 064/065 history. Add a concise note to `plans/065-status.md` that its implementation landed but post-audit residuals are owned by Plan 066.

### Phase 0 acceptance criteria

- `066-status.md` exists and is tracked before source edits;
- all rows begin OPEN;
- actual baseline SHA is recorded;
- Roadmap 057 does not claim current COMPLETE status while Plan 066 is open;
- Plan 065 is not falsely rewritten as a perfect closure.

---

# Phase 1 — close the remaining legacy Light + tiling semantic bug

## 1.1 Preserve the pre-Plan-061 Light contract

Pre-Plan-061 Light used the minimal seed marker path regardless of `ProtectionContext::tile_size`. Tiling belongs to the full-payload Standard path; it must not silently promote Light to Standard-style stego.

Current problematic translation shape is effectively:

```text
Light or Standard:
  if tile_size > 0 -> Tiled
  else Light -> SeedOnly / Standard -> BestEffort
```

Required translation:

```text
Disabled -> Disabled
Light    -> SeedOnly
Standard -> Tiled { tile_size } when tile_size > 0
Standard -> BestEffort otherwise
```

`tile_size` may remain present in processing/context state for compatibility, but it must not change Light's marker mode.

## 1.2 Add focused tests before/with the fix

Required tests:

```text
legacy_light_with_tile_size_maps_to_seed_only
legacy_light_with_tile_size_png_has_no_full_payload
legacy_light_with_tile_size_jpeg_has_no_full_payload
legacy_standard_with_tile_size_maps_to_tiled
legacy_standard_with_tile_size_attempts_full_tiled_payload
```

Use semantic assertions:

- inspect resolved/request marker mode in a crate-private unit test when needed;
- Light report has no full payload embed summary;
- detailed verification for Light remains metadata/seed-only behavior;
- Standard retains tiled full-payload behavior.

Do not add a special Light+tiled executor.

### Phase 1 acceptance criteria

- Light always maps to `SeedOnly` regardless of `tile_size`;
- Standard remains the only legacy level that maps to `Tiled`;
- no existing Standard tiled tests regress;
- no second execution pipeline is introduced.

---

# Phase 2 — make carrier internals genuinely opaque

This is the most important remaining architecture correction.

`pub(crate) mod jpeg_transcoder` followed by a public `#[doc(hidden)]` module that re-exports `JpegHeader`, `Coefficients`, `JpegTranscoder`, and `DctStegoF5` is not private API. `#[doc(hidden)]` changes generated documentation visibility only; downstream Rust code can still name those symbols.

The same issue applies to the current `__internal_lsb_facade` re-export of permutations, bit helpers, legacy carrier functions, seed fallback primitives, and raw tiled helpers.

## 2.1 Delete the broad `__internal_*_facade` exports

Remove:

```text
stegoeggo_stego::__internal_jpeg_facade
stegoeggo_stego::__internal_lsb_facade
```

in their current broad form.

After this phase, a normal downstream consumer must not be able to compile references to:

```text
stegoeggo_stego::JpegHeader
stegoeggo_stego::Coefficients
stegoeggo_stego::JpegTranscoder
stegoeggo_stego::DctStegoF5
stegoeggo_stego::__internal_jpeg_facade::...
stegoeggo_stego::__internal_lsb_facade::stego_permutation_v2
stegoeggo_stego::__internal_lsb_facade::embed_bit_in_pixel
```

## 2.2 First migrate root application code to the intended carrier facades

Before inventing support API, audit every root use of the current hidden facades.

Classify each use into one of these categories:

### A. Already representable by public generic API

Prefer the existing public APIs:

```text
stegoeggo_stego::jpeg::{probe_support, capacity, embed, extract, embed_seed_hint, extract_seed_hint}
stegoeggo_stego::lsb::{capacity, embed, extract}
```

Do not keep a low-level facade merely because the root used the old implementation first.

### B. Simple carrier information missing from public API

If root only needs information such as JPEG dimensions/support classification, add a small value-returning function or extend an existing small public report.

Allowed examples:

```rust
pub struct JpegInfo {
    pub width: u32,
    pub height: u32,
    pub support: JpegSupport,
}

pub fn inspect(bytes: &[u8]) -> Result<JpegInfo, StegoError>;
```

Do not expose parsed headers/tables to supply dimensions.

### C. Application compatibility operation not suitable as normal generic API

If legacy StegoEggo extraction requires an operation that cannot be expressed through the public facade, add the smallest possible explicitly unstable application-support operation.

Preferred shape:

```text
feature = "application-support"

#[cfg(feature = "application-support")]
#[doc(hidden)]
pub mod application_support {
    // operation-level functions only
}
```

The root dependency may enable this feature explicitly.

Such functions may accept/return only:

```text
byte slices / Vec<u8>
RgbaImage or primitive pixel dimensions
u64 seed
usize payload length / redundancy
small public enums/status structs
Result<T, StegoError>
```

They must NOT expose or return:

```text
JpegHeader
quantization/Huffman table structs
Coefficients / HashMap component coefficient maps
DctStegoF5
JpegTranscoder
entropy decoder/encoder objects
permutation function pointers/indices
internal raw parser state
```

Examples of acceptable operation-level support, only if genuinely required:

```text
legacy_lsb_extract(...)
legacy_lsb_extract_range(...)
seed_fallback_embed(...)
seed_fallback_extract(...)
tiled_lsb_embed_application(...)
tiled_lsb_extract_application(...)
jpeg_dimensions(...)
jpeg_tiled_embed_application(...)
jpeg_tiled_extract_application(...)
```

Prefer fewer coarse operations over dozens of algorithm primitives.

## 2.3 Keep JPEG internals private inside the carrier

The carrier owns:

```text
jpeg_transcoder/
JpegHeader
Coefficients
DctStegoF5
JpegTranscoder
Huffman implementation
```

These remain `pub(crate)` or private and are consumed by `jpeg.rs` / optional operation-level support inside the carrier crate.

The root application must stop importing those types entirely.

## 2.4 Keep LSB internals private inside the carrier

`lsb_internal` may remain `pub(crate)`.

The root must not import individual helpers such as:

```text
splitmix64
stego_permutation
stego_permutation_v2
carrier_v2_slot_to_pixel_channel
bytes_to_bits
bits_to_bytes
embed_bit_in_pixel
crop_rgba / blit_rgba
raw V1/V2 embed/extract primitives
```

If the root needs legacy behavior, call a narrow operation-level application-support function instead.

## 2.5 Fuzzing

Do not reopen default public internals for fuzzing.

Preferred options, in order:

1. expose a `#[cfg(feature = "fuzz")]` function that accepts bytes and returns only success/error or simple dimensions/status;
2. move the relevant fuzz target to call a carrier-level fuzz support function;
3. only if necessary, expose a feature-gated fuzz module whose signatures still do not contain JPEG internal structs.

The existing root `parse_jpeg_for_fuzz` must not return `JpegHeader` or `Coefficients` from the standalone carrier.

### Phase 2 acceptance criteria

- no default public module re-exports JPEG codec/coefficient internals;
- no default public module re-exports LSB permutations/bit helpers/raw legacy primitives;
- root source contains no import/reference to `JpegHeader`, `Coefficients`, `DctStegoF5`, `JpegTranscoder`, or old `__internal_*_facade` paths from the carrier;
- any `application-support` surface is explicitly feature-gated, operation-oriented, and contains no internal structs in its signatures;
- public `lsb`, `jpeg`, and `frame` APIs remain usable;
- fuzz feature still compiles without exposing internal structs under default features.

---

# Phase 3 — prove public API containment mechanically

Do not close Phase 2 from source inspection alone.

## 3.1 Add compile-fail visibility evidence

Use the smallest existing mechanism; do not add a heavy compile-test dependency solely for this.

Preferred: crate-level `compile_fail` doctests showing that normal/default consumers cannot import internal codec/helper paths.

At minimum cover attempts conceptually equivalent to:

```rust,compile_fail
use stegoeggo_stego::__internal_jpeg_facade::JpegHeader;
```

```rust,compile_fail
use stegoeggo_stego::__internal_lsb_facade::stego_permutation_v2;
```

and, if the old module names are removed entirely:

```rust,compile_fail
use stegoeggo_stego::jpeg_transcoder::JpegTranscoder;
```

These tests should run with default carrier features.

## 3.2 Positive public consumer coverage

Retain/add positive examples/tests for:

```text
lsb capacity/embed/extract
jpeg probe/capacity/embed/extract
frame encode/decode/decode_prefix
root stegoeggo::stego re-export facade
```

## 3.3 Optional application-support feature coverage

If `application-support` is introduced:

```bash
cargo check -p stegoeggo-stego --features application-support
```

and root `cargo check` must prove the intended dependency feature wiring.

### Phase 3 acceptance criteria

- default compile-fail tests prove the old internal paths are inaccessible;
- positive public API examples compile;
- optional application-support feature, if present, compiles and does not export internal JPEG structs;
- no new testing dependency is added unless existing tooling cannot reasonably provide compile-fail evidence.

---

# Phase 4 — remove plan-specific/test-only public API and stale pipeline state

## 4.1 Remove `_plan065_internal_request_from_legacy`

A Plan-specific integration-test hook must not remain in the shipped root API.

Remove:

```rust
#[doc(hidden)]
pub fn _plan065_internal_request_from_legacy(...)
```

Move tests that genuinely need direct adapter inspection into a crate-private `#[cfg(test)]` unit-test module adjacent to `request_from_legacy()`.

Keep end-to-end compatibility tests in `tests/` using actual public APIs.

Required result:

```text
rg '_plan065_internal_request_from_legacy|plan065_internal' src/ tests/
```

returns no shipped/public helper; test file names may retain historical plan numbering if desired, but product symbols may not.

## 4.2 Simplify `ProtectionPipeline`

After Plan 065 delegation, `metadata_trap` and `steganography` fields are no longer execution dependencies. Remove unused stored protectors rather than retaining the old architecture cosmetically.

Preferred result is a stateless/zero-sized compatibility adapter while preserving:

```text
ProtectionPipeline::new()
Clone
Default
process()
process_bytes()
Disabled Cow::Borrowed behavior
```

For Disabled, return `Cow::Borrowed(img)` directly if no protector object is needed.

Do not alter public method signatures.

### Phase 4 acceptance criteria

- no Plan-specific public function remains;
- adapter translation tests use crate-private tests or public semantic behavior;
- `ProtectionPipeline` stores no unused protector state;
- `ProtectionPipeline` Light/Standard still delegate to canonical execution;
- Disabled still returns borrowed input from `process()`.

---

# Phase 5 — reconcile documented legacy semantics and public LSB rustdoc

## 5.1 Legal claims

The pre-Plan-061 runtime implementation used:

```text
inject_legal_claims.unwrap_or(legal.is_some())
```

Therefore with `LegalMetadata` present:

```text
None        -> inject claims
Some(true)  -> inject claims
Some(false) -> suppress claims
```

Plan 065 preserved this runtime behavior, but current `ProtectionContext` rustdoc still says `None` means legal claims are never injected.

Correct documentation/comments to the actual established compatibility contract. Do not change runtime behavior again unless a stronger historical test proves the old runtime implementation was itself unintended.

Required focused test names or equivalents:

```text
legal_claims_none_with_metadata_injects
legal_claims_none_without_metadata_has_no_claims
legal_claims_true_with_metadata_injects
legal_claims_false_with_metadata_suppresses_and_warns
```

## 5.2 LSB pixel-domain API docs

`lsb::embed()` now returns `EmbedReport<RgbaImage>`.

Remove stale statements such as:

```text
output image bytes (encoded as PNG)
```

and stale examples that call:

```rust
image::load_from_memory(&embedded.output)
```

Required example shape:

```rust
let embedded = lsb::embed(&img, payload, &config)?;
let recovered = lsb::extract(&embedded.output, payload.len(), &config)?;
```

Audit:

```text
stegoeggo-stego/src/lsb_internal.rs rustdoc
stegoeggo-stego/src/lsb.rs module docs
stegoeggo-stego/README.md
root README generic carrier examples
examples/generic_stego.rs
```

## 5.3 Carrier README visibility claim

The README currently says low-level internals are not part of the stable API while acknowledging a public hidden facade. After Phase 2, make this claim literally true.

Document optional application-support/fuzz features only if they remain, and describe them as unstable implementation support rather than public generic API.

### Phase 5 acceptance criteria

- legal-claim public docs match actual compatibility behavior;
- LSB docs/examples use `RgbaImage` directly;
- no public example performs a decode cycle after `lsb::embed`;
- carrier README's API allowlist matches the actual default public surface;
- doctests for the carrier pass.

---

# Phase 6 — make manual release/package verification match publication order

Release remains manual. Do not create CI publication automation.

The package dependency order is:

```text
1. stegoeggo-stego
2. stegoeggo
3. stegoeggo-cli
```

The root package has an exact registry dependency on the carrier once packaged, so full root package verification may legitimately be unavailable before the corresponding carrier version has been published. The CLI has the same relationship to the root library.

The release check must distinguish:

```text
structural pre-publication package validation
vs
full post-dependency-publication cargo package verification
```

instead of treating the root's expected pre-carrier registry failure differently from the CLI's equivalent dependency-order failure.

## 6.1 Keep carrier full verification mandatory

Before any publication:

```bash
cargo package -p stegoeggo-stego --allow-dirty
cargo package -p stegoeggo-stego --list
```

must succeed.

## 6.2 Validate root/CLI structurally before prerequisites exist

Use the simplest Cargo-supported approach, preferably `cargo package --no-verify`, for packages whose exact dependency is intentionally not yet published.

A reasonable pre-publication sequence is:

```bash
cargo package -p stegoeggo-stego --allow-dirty
cargo package -p stegoeggo --allow-dirty --no-verify
cargo package -p stegoeggo-cli --allow-dirty --no-verify
```

plus package listing/manifest lockstep checks.

If `--no-verify` does not work for the actual Cargo behavior/version, use the smallest equivalent structural package check and record the reason. Do not add network probing machinery unless absolutely necessary.

## 6.3 Document full verification checkpoints

`RELEASING.md` must state:

```text
before carrier publish:
  carrier full cargo package verification
  root/CLI structural package checks

after carrier publish, before root publish:
  full cargo package -p stegoeggo

after root publish, before CLI publish:
  full cargo package -p stegoeggo-cli
```

The release script may expose a minimal mode/flag if needed, but avoid a complex state machine.

Examples of acceptable simple designs:

```text
./scripts/release-check.sh --allow-dirty             # pre-publication structural check
./scripts/release-check.sh --stage root               # full root check after carrier exists
./scripts/release-check.sh --stage cli                # full CLI check after root exists
```

or an equivalently small interface.

Do not query crates.io repeatedly merely to infer the stage if an explicit stage is simpler.

### Phase 6 acceptance criteria

- pre-publication release check exits successfully on a clean/unpublished three-crate workspace when carrier packaging itself is valid;
- carrier full package verification remains mandatory;
- root and CLI package contents/manifests are structurally checked before publication;
- full root verification is explicitly required after carrier publication;
- full CLI verification is explicitly required after root publication;
- no failure is mislabeled as successful full verification;
- no crate is published by the script.

---

# Phase 7 — repair retrospective planning evidence

The status files must be useful as historical evidence and must not describe stale intermediate source as current HEAD.

## 7.1 Required retrospective banner

Ensure each backfilled file starts with an explicit statement equivalent to:

```text
RETROSPECTIVE LEDGER — created/tracked after implementation during Plan 065.
This file records historical plan execution; current architecture corrections are noted separately below.
```

Apply to:

```text
plans/058-status.md
plans/059-status.md
plans/060-status.md
plans/061-status.md
plans/063-status.md
```

Do not claim these files existed before implementation.

## 7.2 Reconcile Plan 061

`061-status.md` currently contains stale statements such as:

```text
plan_to_context retained
legacy APIs retain independent pipeline
canonical metadata fully plan-aware but stego still translates
```

Keep historical implementation notes if useful, but add a clear final/current disposition that records the later Plan 061 commits and Plan 065/066 corrections:

```text
current HEAD: no plan_to_context
legacy APIs adapt into ProtectionRequest/ResolvedProtectionPlan
ProtectionPipeline delegates to canonical path
SeedOnly is canonical Light mode
```

Do not erase history; distinguish "state at intermediate commit" from "final disposition".

## 7.3 Reconcile Plan 063

`063-status.md` still describes pre-split `src/stego/` locations and an obsolete public API inventory.

Add/update a final disposition section recording:

```text
carrier lives under stegoeggo-stego/
public default allowlist after Plan 066
jpeg_transcoder and lsb_internal private
any optional application-support feature and its narrow purpose
root re-export behavior
actual measured split rationale remains valid
```

Do not rerun dependency measurements unless source/dependency changes in Plan 066 materially invalidate them.

## 7.4 Reconcile Plan 065

`plans/065-status.md` must not remain `OPEN` with all rows OPEN after the implementation commits.

Use a truthful final disposition such as:

```text
Status: CLOSED WITH FOLLOW-UP — core Plan 065 implementation landed in 70845eba and 1c530a7; residuals found by post-implementation audit are closed by Plan 066.
```

Then close/annotate each R01-R25 row with one of:

```text
CLOSED IN 065
CORRECTED BY 066
SUPERSEDED BY 066
```

and concrete evidence.

Do not mark an unmet Plan 065 criterion as though commit 1c530a7 satisfied it.

## 7.5 Reconcile Plan 064 and Roadmap 057

Update Plan 064's correction note to state that Plan 066 is the final closure owner after the Plan 065 post-audit.

Roadmap 057 remains PARTIAL until Phase 9 final closure.

### Phase 7 acceptance criteria

- 058/059/060/061/063 are explicitly retrospective;
- 061/063 distinguish historical intermediate state from current architecture;
- 065 no longer remains an all-OPEN ledger after its implementation;
- 064 points to the final closure chain accurately;
- no planning document claims current COMPLETE while blocking Plan 066 rows are open.

---

# Phase 8 — focused final verification matrix

Run focused verification before the full repository check.

## 8.1 Legacy compatibility

At minimum:

```text
Disabled byte passthrough
Light default policy Unspecified
Light seed-only PNG
Light seed-only WebP
Light seed-only JPEG
Light + tile_size remains seed-only
Standard full payload PNG/JPEG
Standard + tile_size remains tiled
explicit DMI
metadata injection true/false/default
legal claims None/false/true
explicit redundancy
content hash
timestamp override
resource limits
MAC warning/report behavior
```

## 8.2 Pipeline convergence

```text
ProtectionPipeline::process vs process_image semantics
ProtectionPipeline::process_bytes vs process_image_bytes
Disabled process returns Cow::Borrowed
no duplicate Light/Standard executor helpers
no plan_to_context
```

## 8.3 Carrier correctness/public boundary

```text
V2 arbitrary payload LSB round-trip
legacy LSB extraction
0/255 bounded mutation
raw JPEG round-trip
JPEG container preservation
progressive/unsupported classification
generic frame round-trip/corruption/trailing bytes
public carrier examples compile
compile-fail internal visibility tests pass
root stego facade remains usable
```

## 8.4 Package/release structure

```text
cargo package -p stegoeggo-stego --allow-dirty
cargo package -p stegoeggo-stego --list
pre-publication root structural package check
pre-publication CLI structural package check
release-check pre-publication mode
```

Do not require the unpublished exact dependency versions to exist on crates.io merely to complete this source-code roadmap.

## 8.5 Targeted source audits

Required grep/audit equivalents:

```text
no _plan065_internal_request_from_legacy
no __internal_jpeg_facade
no __internal_lsb_facade
no root references to JpegHeader/Coefficients/DctStegoF5/JpegTranscoder from carrier
no plan_to_context
no stale "encoded as PNG" LSB public rustdoc
no stale legal-claims None="never inject" documentation
```

If an optional `application-support` module exists, explicitly inspect every public signature for internal codec types.

### Phase 8 acceptance criteria

- every required focused semantic test passes;
- compile-fail/default-public-surface evidence passes;
- package structural checks pass;
- targeted source audits are clean;
- any exception is recorded as a blocker, not hidden behind documentation.

---

# Phase 9 — final repository check and truthful Roadmap 057 closure

Only execute this phase after Phases 1-8 are complete.

## 9.1 Run the repository gate

Required:

```bash
./scripts/check.sh
```

Also run carrier doctests explicitly if `check.sh` does not make their coverage obvious:

```bash
cargo test -p stegoeggo-stego --doc
```

If optional support features were added:

```bash
cargo check -p stegoeggo-stego --features application-support
```

or the actual feature name selected in Phase 2.

## 9.2 Close `plans/066-status.md`

For every R01-R21 row record:

```text
CLOSED / BLOCKED
implementation commit(s)
focused test(s)
source audit evidence
commands executed
```

No row may remain implicitly open.

## 9.3 Final planning disposition

If and only if every blocking row is CLOSED:

```text
plans/066-status.md -> COMPLETE
plans/057...roadmap.md -> COMPLETE — final closure verified by Plan 066
```

Plan 064 and Plan 065 should remain historical records with correction/follow-up notes, not be rewritten as if they independently achieved the final state.

If any blocking row remains open:

```text
Roadmap 057 -> PARTIAL
Plan 066 -> PARTIAL / BLOCKED
```

and name the exact unresolved item. Do not create another plan merely to obtain a COMPLETE label unless real product work remains.

### Phase 9 acceptance criteria

- `./scripts/check.sh` exits 0;
- carrier doctests pass;
- `066-status.md` contains concrete closure evidence for every blocking row;
- Roadmap 057 status agrees with actual source/test/package state;
- no version bump/tag/publication/release side effect occurred.

---

# 10. Explicit completion criteria

Plan 066 and Roadmap 057 are complete only when all of the following are true:

1. legacy Light never becomes full tiled stego because `tile_size` is set;
2. legacy Standard tiled behavior remains intact;
3. default downstream `stegoeggo-stego` users cannot name/import JPEG header/coefficient/F5/Huffman/transcoder internals;
4. default downstream users cannot name/import raw LSB permutation/bit/legacy helper internals;
5. root `stegoeggo` no longer imports carrier internal structs to implement its application logic;
6. any optional root application-support surface is narrow, feature-gated, operation-oriented, and free of codec-internal types;
7. fuzzing compiles without reopening default internals;
8. `_plan065_internal_request_from_legacy` is removed from product API;
9. `ProtectionPipeline` contains no obsolete unused protector state and still preserves public behavior;
10. legal-claim rustdoc says `None + LegalMetadata` auto-enables claims, matching established pre-061 runtime behavior;
11. LSB public rustdoc/examples use returned `RgbaImage` directly and contain no stale PNG-byte language;
12. carrier README accurately describes the actual default public surface;
13. carrier full `cargo package` dry-run succeeds;
14. pre-publication root/CLI structural package checks do not fail merely because their exact dependency has not yet been published;
15. release documentation explicitly requires full root verification after carrier publication and full CLI verification after root publication;
16. 058/059/060/061/063 ledgers are explicitly retrospective and distinguish historical intermediate state from current disposition;
17. Plan 065 status accurately records what it closed and what Plan 066 corrected;
18. Plan 064 correction note and Roadmap 057 status are consistent with the final closure chain;
19. public/internal compile-fail evidence passes;
20. focused compatibility/carrier/package tests pass;
21. `./scripts/check.sh` passes;
22. no versions/tags/releases/publications were created.

---

# 11. Suggested implementation commit structure

Keep the corrective pass easy to review. A reasonable sequence is:

```text
1. plans: create 066 status ledger and reopen Roadmap 057
2. fix: preserve seed-only Light when legacy tile size is set
3. refactor: remove carrier internal public facades and migrate root to narrow operations
4. cleanup: remove Plan 065 public test hook and stale ProtectionPipeline state
5. docs: reconcile legal-claim and pixel-domain LSB API contracts
6. release: align package checks with manual dependency publication order
7. docs: reconcile retrospective ledgers and close Roadmap 057
```

Combining adjacent commits is acceptable when tests/source are inseparable. Do not mix unrelated formatting or dependency churn into these commits.

---

# 12. Handoff guidance for a smaller implementation model

When executing this plan:

1. create `066-status.md` first and force-track it;
2. do not trust Plan 065's COMPLETE-adjacent documentation over current source;
3. fix Light+tile mapping before broader refactoring;
4. for carrier visibility, remember that `#[doc(hidden)] pub` is still public Rust API;
5. do not solve cross-crate privacy by re-exporting internal structs under another hidden module name;
6. prefer the existing public carrier facade, then small simple result types, then a narrow feature-gated application-support operation only as a last resort;
7. never expose `JpegHeader`, `Coefficients`, `DctStegoF5`, `JpegTranscoder`, Huffman types, permutations, or bit helpers across the crate boundary;
8. move direct adapter-inspection tests into crate-private tests instead of adding another public test hook;
9. preserve the manual carrier -> root -> CLI release sequence without adding CI automation;
10. do not mark Roadmap 057 COMPLETE until `066-status.md` has concrete passing evidence for every blocking row.
