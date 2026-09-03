# Plan 080: Stego Pipeline, Library Evidence, and Documentation Closure

Status: Ready for implementation

Roadmap: `plans/076-stego-pipeline-and-carrier-library-closure-roadmap.md`

Depends on: Plans 077-079 complete.

Audited planning baseline: `main` at `6feb52a90d9afdc0c922cdb219529524ad94c168`

Authoritative implementation ledger to create before product edits: `plans/080-status.md`

---

## 1. Purpose

Perform the final independent closure audit for Roadmap 076 after routing correctness, decode/allocation optimization, and public carrier API work have landed.

This plan is not another architecture redesign. It exists to prove that the resulting implementation is internally consistent, externally usable, measured, documented, and actually satisfies the roadmap's dependency/API boundary rather than merely compiling.

The final state must demonstrate three independent contracts:

```text
APPLICATION CONTRACT
ProtectionRequest -> ResolvedProtectionPlan -> output-domain carrier -> metadata

PERFORMANCE CONTRACT
no avoidable repeated full-image/coefficient decode on normal hot paths

LIBRARY CONTRACT
standalone generic carrier users receive the same current operation-level
carrier APIs that the parent application relies on, without codec internals
```

Roadmap 076 may be marked COMPLETE only from evidence collected in this plan.

---

## 2. Governing constraints

1. Treat Plans 077-079 implementation as the candidate final state, not presumed-correct truth.
2. If this audit finds a material correctness, performance, or public-boundary residual, record it and leave Roadmap 076 `PARTIAL`; do not paper over it with documentation.
3. Do not introduce a new architecture during closure unless a small corrective patch is necessary to satisfy an already-approved acceptance criterion.
4. Any corrective source patch must be narrow, covered by focused regression tests, and recorded explicitly in `080-status.md`.
5. Preserve all Roadmap-076 non-goals.
6. No new dependency is expected.
7. No version bump, publication, tag, GitHub Release, release workflow, or required-CI expansion is authorized.
8. `./scripts/check.sh` is the required final repository gate.

---

# Phase 0 — create the final evidence ledger

## 0.1 Create `plans/080-status.md` before any corrective product edit

Record:

```text
starting HEAD
working tree status
workspace versions
Plan 077 implementation/status commit
Plan 078 implementation/status commit
Plan 079 implementation/status commit
root -> carrier dependency declaration
current Roadmap 076 status
```

Start every row below `OPEN`:

```text
R01 output-domain carrier invariant verified from source and tests
R02 full input/output format matrix verified
R03 same-format JPEG encoded-byte fast path verified
R04 non-JPEG raster processing uses one full pixel decode
R05 same-format metadata-only container path avoids pixel decode where supported
R06 tiled LSB parent path avoids second full-image clone
R07 tiled JPEG embed performs one coefficient decode
R08 standard JPEG application verification decode count meets Plan-078 disposition
R09 standard+tiled JPEG verification reuse meets recorded disposition
R10 verification search coverage/resource limits preserved
R11 public raw carrier APIs verified
R12 public framed carrier APIs verified
R13 public in-place carrier APIs verified
R14 public tiled carrier APIs verified
R15 prepared JPEG API disposition is evidence-consistent
R16 parent current operations dogfood stable public carrier API where intended
R17 application_support residual surface is justified and narrow
R18 JPEG implementation structs remain private
R19 LSB implementation helpers remain private
R20 standalone direct carrier consumer compiles
R21 root stego re-export consumer compiles
R22 docs/examples match source signatures and behavior
R23 architecture docs match actual routing/decode ownership
R24 benchmark/decode/allocation evidence is reproducible
R25 cargo package/release structural checks remain healthy
R26 full workspace tests pass
R27 ./scripts/check.sh passes
R28 no unapproved dependency/version/release/CI change
R29 Roadmap 076 final disposition is truthful
```

### Phase 0 acceptance criteria

- ledger is tracked before any corrective product edit;
- every roadmap-owned outcome maps to one or more ledger rows;
- no row starts CLOSED based solely on predecessor claims.

---

# Phase 1 — independent source architecture audit

Read current implementations completely enough to trace every normal path.

At minimum inspect:

```text
src/lib.rs
src/protected/steganography/mod.rs
src/protected/steganography/embed.rs
src/protected/steganography/extract.rs
src/protected/steganography/verify.rs
stegoeggo-stego/src/lib.rs
stegoeggo-stego/src/lsb.rs
stegoeggo-stego/src/lsb_internal.rs
stegoeggo-stego/src/jpeg.rs
stegoeggo-stego/src/application_support.rs
stegoeggo-stego/src/frame.rs
```

## 1.1 Prove carrier family follows final output domain

For current full markers, source must reduce to:

```text
output JPEG     -> JPEG DCT/F5
output PNG/WebP -> raster LSB
```

Search for remaining current-path carrier decisions using `input_format`.

Input format may still appear in:

```text
encoded-byte reuse
format conversion decisions
metadata preservation decisions
validation
```

but not in the final carrier-family selection.

## 1.2 Trace decode ownership

Produce a concise table in the status ledger:

| Path | Pixel decodes | Coefficient decodes | Full-image clones | Notes |
|---|---:|---:|---:|---|
| JPEG->JPEG BestEffort | | | | |
| JPEG->JPEG Tiled | | | | |
| JPEG verify standard | | | | |
| JPEG verify standard+tiled fallback | | | | |
| JPEG->PNG/WebP | | | | |
| PNG/WebP->same raster | | | | |
| PNG/WebP->JPEG | | | | |
| tiled raster parent | | | | |
| metadata-only same-format | | | | |

Use test counters and source ownership, not estimates.

## 1.3 Audit parent/carrier boundary

Classify every root reference to `stegoeggo_stego::application_support`.

For each remaining call, record one of:

```text
LEGACY-COMPATIBILITY
APPLICATION-SPECIFIC-SEARCH
TEMPORARY-RESIDUAL
```

`TEMPORARY-RESIDUAL` prevents closure unless corrected in this plan or explicitly moved to a new numbered corrective plan and Roadmap 076 remains PARTIAL.

### Phase 1 acceptance criteria

- routing and representation ownership can be explained from small local functions;
- no unexplained privileged current-carrier operation remains;
- decode/clone table is evidence-backed.

---

# Phase 2 — complete behavior matrix and regression verification

## 2.1 Full BestEffort format matrix

Verify all nine input/output combinations:

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

For each successful/capable fixture assert:

```text
requested output magic/format
output decodes
format_transcoded report correctness
carrier EmbedPath matches final format
final-byte verification succeeds
rights metadata remains present when requested
```

Use deterministic sufficiently textured fixtures.

## 2.2 Tiled behavior matrix

Verify at least:

```text
same-format raster tiled
JPEG -> raster tiled
raster -> JPEG tiled
same-format JPEG tiled
crop recovery with an intact eligible tile
bounded no-match search
```

Do not assert generalized crop robustness beyond actual tile semantics.

## 2.3 SeedOnly behavior matrix

Verify final output-domain seed carrier behavior independently from full payload modes.

## 2.4 Legacy compatibility

Run existing V1/V2 and deprecated compatibility tests. Public-library cleanup must not strand images produced by older releases.

### Phase 2 acceptance criteria

- all format directions are represented by final-byte tests;
- tiled/seed-only modes agree with output carrier domain;
- legacy tests remain green.

---

# Phase 3 — standalone generic carrier consumer proof

The carrier crate is intended to solve a real Rust ecosystem gap. Prove it as an external package consumer, not only through workspace-internal imports.

## 3.1 Temporary direct consumer

Outside the repository, create a temporary crate using a path dependency on `stegoeggo-stego` with default features.

Compile and, where practical, run examples covering:

```text
LSB raw roundtrip
LSB in-place roundtrip
LSB framed roundtrip
LSB tiled raw/framed roundtrip
JPEG raw roundtrip
JPEG framed roundtrip
JPEG tiled raw/framed roundtrip
support probing
prepared JPEG reuse if Plan 079 approved it
```

The consumer must not enable `application-support`.

Record the command and result; do not commit the temporary crate.

## 3.2 Root re-export consumer

Create a second temporary consumer or a dedicated compile test that uses:

```rust
stegoeggo::stego
```

for the intended generic operations.

Verify that the root re-export does not accidentally expose less than the direct carrier package for the supported facade.

## 3.3 Dependency-surface sanity

Run:

```bash
cargo tree -p stegoeggo-stego --edges normal
```

Record direct dependencies and confirm no root application dependency has leaked into the standalone carrier.

Expected carrier dependencies remain narrowly image/JPEG/checksum/error oriented.

### Phase 3 acceptance criteria

- direct default-feature consumer succeeds;
- root re-export consumer succeeds;
- no rights/provenance/metadata dependency leaks into carrier package.

---

# Phase 4 — private-boundary and semver audit

## 4.1 Mechanical privacy checks

Compile-fail/public API evidence must prove a default downstream consumer cannot name:

```text
jpeg_transcoder
JpegTranscoder
JpegHeader
Coefficients
DctStegoF5
Huffman/entropy objects
lsb_internal
stego_permutation_v2
carrier slot-mapping helpers
application_support without explicitly enabling its feature
```

If a public opaque prepared JPEG type exists, ensure it has no getters/traits that leak implementation state through debug formatting, conversion, deref, or public fields.

## 4.2 Public API stability review

Audit new public tiled/config/prepared types for:

```text
private fields
#[must_use] builders where applicable
fallible runtime validation
non_exhaustive where future enum growth matters
clear units in capacity reports
bounded allocation/search behavior
consistent StegoError variants
```

Do not add `#[non_exhaustive]` mechanically to structs where it harms straightforward construction and no construction was promised; follow existing crate conventions.

## 4.3 Root dogfooding proof

Search current current-path source for hidden wrappers that duplicate stable public operations.

Expected ordinary current operations should route through stable:

```text
lsb::...
jpeg::...
```

Hidden support may remain for explicit compatibility/search exceptions recorded in R17.

### Phase 4 acceptance criteria

- privacy boundary is compiler-proven;
- no internal representation is part of stable semver;
- root meaningfully exercises the public carrier facade.

---

# Phase 5 — performance evidence closure

Re-run the focused measurements introduced by Plan 078 after all Plan-079 API migrations so the final numbers represent the shipped architecture.

At minimum record:

```text
JPEG standard verification coefficient-decode count
JPEG standard+tiled fallback coefficient-decode count
JPEG tiled embed coefficient-decode count
LSB tiled in-place vs copied allocation behavior
representative raster bytes-in/bytes-out processing
JPEG-in/JPEG-out fast path timing
```

Do not use wall-clock time alone to prove structural decode/allocation claims.

If Plan 079 exposed `PreparedJpeg`, include a repeated-operation benchmark comparing:

```text
N one-shot operations
vs
prepare once + N operations
```

for a realistic `N` such as 5 or 10.

If measurements do not substantiate a public prepared type, remove it before closure or leave Roadmap 076 PARTIAL and create a corrective plan. Do not retain an unjustified public abstraction because it has already been written.

### Phase 5 acceptance criteria

- final architecture still meets measured structural goals after API migration;
- benchmark methodology uses equivalent pristine inputs/setup;
- evidence is recorded without CI thresholds.

---

# Phase 6 — documentation and examples reconciliation

Audit and update all user/maintainer documentation that describes stego routing or generic carrier APIs.

At minimum:

```text
README.md
stegoeggo-stego/README.md
docs/carrier-crate.md
docs/rust-api.md
architecture/overview.md
architecture/pipeline.md
architecture/protected-steganography.md
architecture/jpeg-stego-f5.md
architecture/jpeg-transcoder.md
examples/generic_stego.rs
.skills/stegoeggo-conventions/SKILL.md
STABILITY.md
CHANGELOG.md only if the repository convention records unreleased API changes there
```

## 6.1 Required architecture wording

Document explicitly:

```text
carrier domain follows final output format
input format controls byte reuse/transcoding only
JPEG-in/JPEG-out operates on encoded DCT coefficients
raster current marker execution owns one RGBA buffer
application JPEG verification retains decoded coefficient state across bounded search
```

## 6.2 Required public API wording

Document operation styles as:

```text
raw
framed
in-place (raster)
tiled
prepared/reuse only if actually public
```

Every public example must compile in doctests or a dedicated example check.

## 6.3 Security wording

Continue to state:

```text
CRC32 is corruption detection, not authentication
seed is not a cryptographic secret
no steganalysis-resistance/undetectability guarantee
lossy/transcode robustness depends on carrier domain
```

Do not turn engineering improvements into stronger security claims.

### Phase 6 acceptance criteria

- docs match current signatures and actual behavior;
- no stale input-domain carrier description remains;
- no hidden application-support API is presented as stable public API;
- examples compile.

---

# Phase 7 — package and repository gates

Run the complete final matrix.

Required:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p stegoeggo --no-default-features
cargo check -p stegoeggo-stego
cargo check -p stegoeggo-stego --features application-support
cargo test -p stegoeggo-stego --all-features
cargo test -p stegoeggo-stego --doc
cargo test -p stegoeggo --test public_stego_api --all-features
cargo test -p stegoeggo --test request_api --all-features
cargo test -p stegoeggo --test cross_format_semantics --all-features
cargo test -p stegoeggo --all-features legacy
cargo test -p stegoeggo --all-features jpeg
cargo test -p stegoeggo --all-features tiled
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Run the repository's existing structural/package/release check in its non-publishing mode if it is still part of current conventions, for example:

```bash
./scripts/release-check.sh --allow-dirty --skip-check --stage=pre
```

Use the actual script interface at implementation time; do not force stale arguments if it has changed.

No publication is authorized.

### Phase 7 acceptance criteria

- all required checks pass;
- carrier package remains independently packageable according to current release conventions;
- no unapproved version/release/CI change occurred.

---

# Phase 8 — final roadmap disposition

## 8.1 COMPLETE

Mark Roadmap 076 `COMPLETE` only if every ledger row R01-R28 is CLOSED and all acceptance criteria are evidence-backed.

Update the roadmap header with a concise final note including:

```text
Plan 077 routing correction disposition
Plan 078 measured optimization disposition
Plan 079 public API/prepared-object disposition
Plan 080 final verification commit
```

## 8.2 PARTIAL

If a material residual remains:

```text
Status: PARTIAL — residual tracked by Plan 081+ <name>
```

Create a new numbered corrective plan only for concrete residuals. Do not append implementation work informally to this closure plan after it has been declared complete.

## 8.3 Historical predecessor status

Do not reopen or rewrite Roadmaps 057/069 unless the audit finds that their historical completion evidence was actually false. Roadmap 076 is an additive refinement/closure line, not a reason to erase predecessor history.

---

## 9. Final acceptance criteria

Plan 080 is complete only when:

1. `plans/080-status.md` was created before any corrective source edit.
2. Source audit confirms final-output-domain carrier routing.
3. Full BestEffort, tiled, and SeedOnly format matrices pass.
4. Representation decode/clone counts are measured and documented.
5. Generic standalone raw/framed/in-place/tiled APIs compile and roundtrip externally.
6. Any prepared JPEG public API is evidence-justified; otherwise no such API exists.
7. Root current carrier operations dogfood stable APIs to the intended extent.
8. Remaining application-support exports are justified compatibility/search operations.
9. JPEG and LSB implementation internals remain downstream-inaccessible.
10. Legacy V1/V2 compatibility remains passing.
11. Documentation and examples match source.
12. Performance evidence is reproducible and does not depend only on timing noise.
13. Package/release structural checks pass without publishing anything.
14. Full workspace tests pass.
15. `./scripts/check.sh` passes.
16. No unapproved dependency/version/release/CI change occurs.
17. Roadmap 076 final status is evidence-consistent.

---

## 10. Non-goals

Do not use closure to add:

- new algorithms;
- new carrier formats;
- visible watermarking;
- steganalysis functionality;
- stronger security/robustness claims;
- payload-v4;
- new authentication algorithms;
- language bindings;
- publication automation;
- a generalized carrier framework.