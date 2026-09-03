# Plan 077: Output-Domain Carrier Routing Correctness

Status: Ready for implementation

Roadmap: `plans/076-stego-pipeline-and-carrier-library-closure-roadmap.md`

Audited planning baseline: `main` at `6feb52a90d9afdc0c922cdb219529524ad94c168`

Authoritative implementation ledger to create before product edits: `plans/077-status.md`

---

## 1. Purpose

Fix the carrier-routing invariant before performing further optimization or public-API work.

The canonical executor in `src/lib.rs` already treats JPEG output specially, but `SteganographyProtector::apply_to_image_with_summary_from_plan()` in `src/protected/steganography/embed.rs` currently derives its local carrier choice from:

```rust
let format = plan.input_format();
```

That is unsafe for format conversion. A JPEG input converted to PNG/WebP enters the raster-output branch in `execute_stego_and_metadata()`, is decoded to a `DynamicImage`, and then the steganography adapter sees the original input format `Jpeg`. It can therefore:

```text
JPEG bytes
  -> pixel decode
  -> JPEG re-encode
  -> DCT/F5 embed
  -> JPEG decode back to pixels
  -> PNG/WebP encode
```

The DCT payload is tied to the transient JPEG representation and is not the carrier of the final PNG/WebP output. The correct invariant is:

```text
final JPEG output     => DCT/F5 carrier
final PNG/WebP output => pixel-domain LSB carrier
```

Input format may determine whether encoded bytes can be reused, but it must never determine the final carrier domain.

This plan fixes that invariant and consolidates Standard/Tiled execution enough that the same defect cannot reappear in two near-duplicate functions.

---

## 2. Required end state

The canonical execution decision tree should be equivalent to:

```text
ResolvedProtectionPlan
        |
        +-- metadata-only ----------------------> metadata executor
        |
        +-- SeedOnly ---------------------------> seed executor
        |
        +-- BestEffort / Tiled
                |
                v
          output_format?
            /       \
          JPEG     PNG/WebP
           |          |
           |          +-- decode source pixels
           |          +-- LSB or tiled LSB
           |          +-- encode final format
           |
           +-- input JPEG? reuse original bytes
           |   otherwise decode source + encode JPEG
           +-- DCT/F5 or tiled DCT/F5
                |
                v
          inject rights metadata
```

The application steganography adapter should not contain a second independent “which carrier family?” decision based on `plan.input_format()`.

---

## 3. Governing constraints

1. Preserve all existing public API signatures unless a private helper signature needs to change.
2. Preserve `ProtectionRequest -> ResolvedProtectionPlan -> process_plan_bytes()` as the canonical path.
3. Preserve legacy `ProtectionLevel` / `ProtectionContext` as adapters into canonical execution.
4. Preserve JPEG-in/JPEG-out encoded-byte fast path.
5. Preserve current JPEG progressive/unsupported fallback behavior.
6. Preserve seed-only semantics.
7. Preserve payload-v3 bytes, authentication, rights metadata, and warnings.
8. Preserve corrected LSB V2 and F5 carrier algorithms exactly.
9. Do not add new public carrier APIs in this plan; Plan 079 owns that.
10. Do not optimize away validation/search coverage in this correctness pass.
11. Do not add a new carrier abstraction trait or framework.
12. No version/release/dependency/CI changes.

---

# Phase 0 — create the status ledger and capture the failing matrix

## 0.1 Create `plans/077-status.md` first

Before any product-source edit, create and force-track `plans/077-status.md`.

Record:

```text
starting HEAD
working tree status
workspace versions
root -> carrier dependency declaration
Roadmap 076 status
```

Start these rows `OPEN`:

```text
R01 output-format carrier invariant encoded in tests
R02 JPEG->PNG BestEffort uses LSB carrier
R03 JPEG->WebP BestEffort uses LSB carrier
R04 JPEG->PNG Tiled uses tiled LSB carrier
R05 JPEG->WebP Tiled uses tiled LSB carrier
R06 PNG/WebP->JPEG uses DCT carrier
R07 JPEG->JPEG encoded-byte fast path preserved
R08 PNG/WebP same-format LSB behavior preserved
R09 SeedOnly cross-format behavior matches final output domain
R10 embed summary path matches actual final carrier
R11 Standard/Tiled executor duplication reduced without behavior drift
R12 legacy compatibility remains passing
R13 focused request/public tests pass
R14 full check.sh passes
R15 no unrelated API/dependency/release change
```

## 0.2 Add regression tests before the fix

Use sufficiently large/textured fixtures so capacity rather than carrier-routing does not dominate the result.

Required current-request tests should cover at least:

```text
jpeg_to_png_best_effort_verifies_lsb_output
jpeg_to_webp_best_effort_verifies_lsb_output
jpeg_to_png_tiled_verifies_lsb_output
jpeg_to_webp_tiled_verifies_lsb_output
png_to_jpeg_best_effort_reports_dct_path
webp_to_jpeg_best_effort_reports_dct_path
jpeg_to_jpeg_best_effort_reports_dct_path
png_to_png_best_effort_reports_lsb_path
webp_to_webp_best_effort_reports_lsb_path
```

Use `process_request_bytes_with_report()` when possible so tests can assert both:

```text
output magic/format
ExecutionReport.embed_summary.path
```

and then verify the produced marker through the public verification API.

For JPEG DCT tests, use a deterministic textured image large enough to produce useful AC capacity. Do not use an all-black/all-zero image as the only fixture.

### Phase 0 acceptance criteria

- the cross-format matrix is represented by focused tests;
- at least the JPEG -> raster regression fails or exposes the incorrect carrier path at the audited baseline;
- the ledger records the actual observed baseline rather than assuming the defect.

---

# Phase 1 — make the top-level executor the sole current-carrier router

## 1.1 Remove input-format carrier selection from the pixel helper

`apply_to_image_with_summary_from_plan()` must no longer infer carrier family from `plan.input_format()`.

Preferred disposition is to narrow it into an explicitly raster-domain helper, for example conceptually:

```rust
apply_lsb_to_image_with_summary_from_plan(
    img: &DynamicImage,
    plan: &ResolvedProtectionPlan,
    tile_size: Option<u32>,
) -> Result<(DynamicImage, Option<EmbedOutcomeSummary>)>
```

The exact name may differ, but the helper's contract must make it impossible for it to choose JPEG DCT simply because the original input was JPEG.

It should:

```text
DynamicImage -> owned RGBA -> LSB/tiled-LSB -> DynamicImage::ImageRgba8
```

and nothing else.

JPEG DCT remains in the encoded-byte helper:

```text
apply_dct_stego_bytes_from_plan(...)
```

## 1.2 Derive `EmbedPath` from the operation actually executed

For raster-domain output:

```text
BestEffort -> EmbedPath::Lsb
Tiled      -> EmbedPath::LsbTiled
```

For JPEG output:

```text
BestEffort -> EmbedPath::DctF5
Tiled      -> EmbedPath::DctF5Tiled
```

Do not derive the payload emission path from the original input format.

This matters because payload-v3 records channel/path semantics used later by verification/reporting.

## 1.3 Keep format reuse and carrier choice separate

The executor may still branch on `(input_format, output_format)` to avoid unnecessary transcoding:

```text
input JPEG + output JPEG -> use original encoded JPEG
input != JPEG + output JPEG -> decode + encode JPEG once
```

But the carrier family decision itself is only:

```text
output_format == JPEG ? DCT : LSB
```

### Phase 1 acceptance criteria

- no current full-marker helper chooses carrier family from `plan.input_format()`;
- JPEG -> PNG/WebP executes LSB directly on decoded pixels;
- PNG/WebP -> JPEG continues to execute DCT after JPEG encoding;
- payload emission path matches actual carrier family.

---

# Phase 2 — consolidate BestEffort and Tiled orchestration

`execute_stego_and_metadata()` and `execute_stego_and_metadata_tiled()` are near-duplicates. Keeping both encourages routing drift.

## 2.1 Preferred private helper

Consolidate them into one private operation that accepts the current marker mode or a narrow tile option, for example:

```rust
fn execute_full_marker_and_metadata(
    img_bytes: &[u8],
    plan: &ResolvedProtectionPlan,
    tile_size: Option<u32>,
    steganography: &SteganographyProtector,
    metadata_trap: &RightsMetadataProtector,
    budget: &mut OperationObserver,
) -> Result<PipelineResult>
```

The helper should obtain `input_format` / `output_format` from the plan rather than accept redundant copies unless passing them demonstrably improves clarity.

Do not introduce an additional public enum; `HiddenMarkerMode` already represents the application state.

## 2.2 Do not merge SeedOnly merely for symmetry

SeedOnly has materially different carrier operations and no embed summary. It may remain a separate executor if merging it would add branching noise.

The goal is to remove duplicate full-payload Standard/Tiled routing, not to force all modes into one giant function.

## 2.3 Metadata remains the final application stage

For full markers, preserve:

```text
carrier modification -> final-format encoding if needed -> rights metadata injection
```

Do not move metadata before a later lossy/container conversion that would discard it.

For SeedOnly, preserve existing JPEG ordering if tests show it is required by container semantics; fix only carrier-domain mistakes.

### Phase 2 acceptance criteria

- Standard/Tiled current execution has one carrier-routing implementation;
- SeedOnly remains clear rather than over-generalized;
- metadata is injected into the final encoded representation;
- warnings/reporting remain equivalent.

---

# Phase 3 — complete the cross-format verification matrix

After the fix, add/retain tests for all important format directions.

## 3.1 BestEffort matrix

At minimum:

```text
PNG  -> PNG
PNG  -> JPEG
PNG  -> WebP
JPEG -> PNG
JPEG -> JPEG
JPEG -> WebP
WebP -> PNG
WebP -> JPEG
WebP -> WebP
```

The test may be table-driven.

Assertions should include:

- output magic equals requested output format;
- `format_transcoded` is true exactly when input != output;
- embed summary path is consistent with output carrier family;
- output remains decodable;
- when capacity/support is sufficient, verification succeeds from final bytes.

## 3.2 Tiled matrix

The critical tiled conversions are:

```text
JPEG -> PNG
JPEG -> WebP
PNG/WebP -> JPEG
same-format raster tiled
same-format JPEG tiled
```

Use a tile size and image dimensions known to have capacity.

## 3.3 SeedOnly matrix

At minimum verify:

```text
JPEG -> PNG/WebP uses raster seed fallback
PNG/WebP -> JPEG uses JPEG seed hint
JPEG -> JPEG preserves seed hint fast path
PNG/WebP -> same-format preserves raster seed behavior
```

Do not reinterpret SeedOnly as a full payload merely to make verification assertions easier.

### Phase 3 acceptance criteria

- cross-format output is both structurally valid and carrier-correct;
- tests prove the final bytes, not an intermediate object;
- report paths and marker semantics agree.

---

# Phase 4 — remove obsolete private routing code and align architecture docs

## 4.1 Delete or narrow obsolete methods

After callers migrate, remove private helpers whose only purpose was the old mixed-domain dispatch.

Candidates include:

```text
apply_to_image_with_summary_from_plan   # if replaced by raster-specific helper
legacy/current branches that re-encode JPEG inside the raster helper
```

Do not delete legacy compatibility methods still used by the deprecated public API unless their behavior is now provided through canonical delegation.

## 4.2 Update architecture documentation

At minimum update:

```text
architecture/pipeline.md
architecture/protected-steganography.md
architecture/overview.md
```

State the invariant explicitly:

```text
carrier family is selected from final output format;
input format controls fast-path reuse only.
```

Document JPEG -> PNG/WebP as one pixel decode followed by raster LSB; do not describe a transient JPEG DCT step.

## 4.3 Update repository conventions if needed

If the output-domain rule is not already in `.skills/stegoeggo-conventions/SKILL.md`, add it during final documentation reconciliation so future changes do not regress it.

### Phase 4 acceptance criteria

- docs match source;
- no obsolete input-domain dispatch remains;
- the invariant is discoverable in both architecture and maintainer conventions.

---

## 5. Focused verification

Expected commands:

```bash
cargo test -p stegoeggo --test request_api --all-features
cargo test -p stegoeggo --test cross_format_semantics --all-features
cargo test -p stegoeggo --all-features stego
cargo test -p stegoeggo --all-features tiled
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

If new tests live in another existing integration file, record the exact command in `077-status.md`.

---

## 6. Final acceptance criteria

Plan 077 is complete only when:

1. The status ledger was created before product edits.
2. A regression test captures the audited JPEG -> raster carrier defect or records why the exact baseline behaved differently than source inspection predicted.
3. Current carrier family is selected from `plan.output_format()` / the final output domain.
4. Input format is used only for reuse/transcoding decisions.
5. JPEG -> PNG/WebP BestEffort produces an LSB-domain marker in the final output.
6. JPEG -> PNG/WebP Tiled produces a tiled-LSB-domain marker in the final output.
7. PNG/WebP -> JPEG still produces DCT/F5 markers.
8. JPEG -> JPEG still bypasses pixel decode/re-encode.
9. SeedOnly follows final output carrier domain.
10. Embed summary paths reflect actual operations.
11. Standard/Tiled orchestration duplication is reduced to one current full-marker router.
12. No rights/payload/metadata semantics change.
13. Focused tests pass.
14. `./scripts/check.sh` passes.
15. No version, dependency, publication, or CI expansion occurs.

---

## 7. Non-goals

Do not use this plan to:

- add tiled public APIs;
- introduce a prepared JPEG public object;
- optimize coefficient search loops;
- redesign LSB/F5 algorithms;
- change supported JPEG syntax;
- add formats;
- alter generic framing;
- alter authentication or rights-policy semantics;
- publish a release.