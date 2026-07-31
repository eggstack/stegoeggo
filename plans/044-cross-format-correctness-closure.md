# Plan 044: Cross-Format Correctness and Scope Closure

Status: Ready for implementation

Baseline: `main` after Plan 043 planning commit `6dc16ccab97e42b4694d38d728ded306e059a295`

Depends on:

- `plans/038-rights-metadata-format-correctness-roadmap.md`
- `plans/039-plus-iptc-rights-metadata-correctness.md`
- `plans/040-jpeg-dct-container-correctness-and-containment.md`
- `plans/041-webp-container-xmp-exif-correctness.md`
- `plans/042-api-cli-contract-consolidation.md`
- `plans/043-measured-binary-size-reduction.md`

This is the final integration, evidence, and documentation pass for the roadmap. It is not an additional implementation roadmap and must not reopen completed design decisions without a concrete failed closure criterion.

---

## Purpose

Demonstrate that the corrected repository accomplishes its intended bounded goal across PNG, JPEG, and WebP, close planning evidence truthfully, remove stale claims, and record any remaining safe limitations.

This plan exists because independent implementation phases can each pass locally while leaving cross-format inconsistencies:

- the same policy may serialize differently by format;
- metadata update policies may preserve unrelated fields in one container but not another;
- execution reports may differ between single/batch/legacy/request paths;
- documentation may still describe pre-correction behavior;
- optional-feature or binary-size changes may omit a capability unintentionally;
- internal verification may pass without external tools observing the expected metadata.

The closure pass addresses those integration risks with a compact matrix. It must not create a permanent certification apparatus.

---

## Governing constraints

1. No new product feature is authorized.
2. No new image format is authorized.
3. No full JPEG/WebP scope expansion is authorized.
4. Required CI remains one stable job invoking `scripts/check.sh`.
5. External interoperability remains manual/targeted.
6. No release-candidate workflow, tag gate, publication workflow, or evidence artifact pipeline is added.
7. No crate is published and no tag/release is created.
8. A limitation may close as documented/contained if the image remains valid, metadata behavior is correct, and the downgrade is explicit.
9. One representative fixture per meaningful structure is preferable to dozens of near-duplicates.
10. Historical plans remain historical; current docs and status ledgers must be accurate.

---

## Required end state

At completion:

- canonical PLUS rights metadata is externally visible in each supported format;
- `ProhibitedSeeConstraints` transports standard constraints in each XMP-capable format;
- private legacy markers do not contradict canonical output;
- PNG, JPEG, and WebP outputs decode after processing;
- metadata-only operations preserve pixels/image payload and unrelated metadata according to their format contracts;
- optional hidden markers succeed only on supported inputs and fall back explicitly elsewhere;
- one canonical request/CLI path reports actual outcomes;
- pixel-only APIs no longer imply file metadata;
- the normal CLI retains documented capabilities and has a recorded final size;
- current documentation contains no stronger claim than the evidence supports;
- each plan status ledger is complete or truthfully records a maintainer/external blocker;
- the roadmap can be considered closed without adding ongoing process burden.

---

## Non-goals

Do not use this plan to:

- implement missing functionality identified as an accepted safe limitation;
- add C2PA, TDMRep deployment, new provenance trust models, or new cryptography;
- redesign payload v3;
- add a benchmark suite;
- add performance SLOs;
- add an exhaustive feature matrix;
- add scheduled fuzzing or external-tool checks;
- add binary-size gates;
- restructure the workspace into additional crates unless a prior plan already implemented and justified it;
- publish a version.

---

## Phase 0: Create the closure ledger

Create `plans/044-status.md` before integration corrections.

Initialize it with:

```text
Plan baseline SHA: 6dc16ccab97e42b4694d38d728ded306e059a295
Disposition: OPEN
Plan 039 evidence: OPEN
Plan 040 evidence: OPEN
Plan 041 evidence: OPEN
Plan 042 evidence: OPEN
Plan 043 evidence: OPEN
Cross-format policy equivalence: OPEN
External interoperability: OPEN
Documentation truth: OPEN
Final CI: OPEN
Remaining limitations: OPEN
Publication hold: no publication is part of this plan
```

Add these tables.

### Table A: implementation-plan closure

```text
plan | implementation commits | status ledger | key acceptance criteria met | open blockers | disposition
```

Required rows: 039, 040, 041, 042, 043.

### Table B: cross-format semantic matrix

```text
case | PNG | JPEG | WebP | expected common semantics | evidence | status
```

Required cases:

- Allowed;
- Prohibited AI/ML training;
- Prohibited generative AI training;
- prohibited except search indexing;
- all data mining prohibited;
- prohibited see constraints;
- unspecified/no policy;
- copyright/creator/usage terms;
- localized usage terms;
- web statement of rights;
- replace StegoEggo-owned metadata;
- preserve existing fields;
- fail on conflict;
- canonical plus legacy agreement;
- canonical plus legacy conflict.

### Table C: container/payload preservation

```text
format/fixture | operation | before digest/inventory | after digest/inventory | intended changes | external decode | status
```

### Table D: API/CLI equivalence

```text
operation | legacy syntax/API | request syntax/API | resolved plan equivalent | output/report equivalent | status
```

### Table E: limitations

```text
limitation | affected input | safe behavior | warning/report | documentation location | future work required | accepted
```

### Phase 0 acceptance criteria

- the ledger exists before integration edits;
- every prior plan is linked to exact evidence;
- unresolved criteria are not marked closed by assumption;
- accepted limitations are separated from defects;
- publication remains explicitly out of scope.

Suggested commit:

```text
plans: establish cross-format closure ledger
```

---

## Phase 1: Verify prior-plan evidence before integration work

Review each status ledger and implementation commit.

### 1.1 Plan 039

Confirm:

- complete canonical PLUS vocabulary URI emission;
- standard constraints emission;
- legacy bare-key classification is noncanonical;
- Allowed/Unspecified do not emit private prohibitions;
- independent metadata output evidence exists.

### 1.2 Plan 040

Confirm:

- metadata-only JPEG scan preservation;
- segment preservation;
- bounded DCT capability decision;
- block-order and Huffman fixes if DCT retained;
- restart support or containment;
- explicit fallback/reporting.

### 1.3 Plan 041

Confirm:

- checked RIFF parsing;
- VP8X conversion/flags;
- XMP merge/replace behavior;
- valid EXIF or XMP-only decision;
- external WebP decoding.

### 1.4 Plan 042

Confirm:

- one request builder/path;
- deterministic option conflicts;
- key resolver consistency;
- DynamicImage/pixel-only contract correction;
- production test-seed removal.

### 1.5 Plan 043

Confirm:

- reproducible baseline/final measurements;
- behavior-preserving feature boundaries;
- retained parallel behavior;
- no speculative complexity;
- normal CLI capability preservation.

If a prior criterion is missing, reopen only the relevant plan or make the smallest corrective edit under this plan. Do not rewrite entire phases.

### Phase 1 acceptance criteria

- every prior plan has an evidence-based disposition;
- missing evidence is obtained or recorded as an exact blocker;
- no plan is marked closed solely because commits exist;
- no unrelated refactor begins.

Suggested commit, only if ledger corrections are needed:

```text
plans: reconcile rights correctness implementation evidence
```

---

## Phase 2: Build the compact cross-format semantic fixture matrix

### 2.1 Fixture strategy

Use small deterministic images:

- PNG: simple RGBA fixture with unrelated text/XMP metadata;
- JPEG: supported baseline fixture plus an unsupported/progressive or restart fixture;
- WebP: simple lossless or lossy container plus an extended metadata fixture.

Reuse fixtures from Plans 039-041. Do not duplicate files merely to give this plan its own directory.

### 2.2 Semantic assertions

For equivalent requests, assert common extracted semantics:

```text
effective rights policy
canonical DMI value
rights signal kind
copyright holder
creator
usage terms
usage terms language
web statement
constraints
metadata conflict state
```

Container-specific evidence channels may differ; semantic rights values should not.

### 2.3 Raw representation assertions

Assert the canonical full URI appears in XMP for each format.

For `ProhibitedSeeConstraints`, assert standard constraints content appears in the same packet.

Do not require byte-identical XMP packets across formats if container wrappers differ. Require semantic equivalence.

### 2.4 Negative cases

Required negative cases:

- Allowed plus legacy prohibition conflict;
- bare-key historical XMP;
- unknown-origin URI ending in a known DMI key;
- prohibited-see-constraints without constraints;
- malformed/truncated container metadata;
- unsupported JPEG hidden-marker request;
- duplicate WebP XMP input.

### Phase 2 acceptance criteria

- the compact matrix covers every rights policy;
- equivalent requests extract equivalent semantic values;
- canonical and legacy conflicts are visible;
- negative cases fail or downgrade as designed;
- fixtures remain small and provenance is documented;
- no format-specific policy fork is discovered.

Suggested commit:

```text
tests: add cross-format rights semantics matrix
```

---

## Phase 3: Verify preservation and idempotence

### 3.1 Metadata-only preservation

Required format-specific invariants:

#### PNG

- IDAT payload/chunk data remains unchanged for metadata-only same-format processing;
- unrelated chunks remain present and ordered according to the implemented policy;
- CRCs remain valid.

#### JPEG

- SOS header/scan/EOI region remains byte-identical for metadata-only same-format processing;
- unrelated APP/COM/DRI/SOF/DQT/DHT segments remain preserved;
- StegoEggo-owned segments change only as requested.

#### WebP

- VP8/VP8L/animation payload chunks remain byte-identical;
- simple-to-extended conversion changes only container metadata structure;
- unknown/ICC/alpha/animation chunks remain preserved;
- RIFF size and VP8X flags remain valid.

### 3.2 Idempotence

For `ReplaceStegoOwned`, process the same request twice.

Expected:

- no duplicate StegoEggo metadata fields/chunks/segments;
- no unbounded file growth;
- extracted semantic values unchanged;
- hidden marker behavior follows the documented algorithm and seed semantics;
- second output is either byte-identical where deterministic or differs only in explicitly time-varying fields.

Timestamp handling must be controlled in tests through an existing override or deterministic context.

### 3.3 PreserveExisting and FailOnConflict

Confirm these policies behave consistently across formats:

- `PreserveExisting` does not overwrite existing StegoEggo values;
- missing fields are added where supported;
- `FailOnConflict` detects semantic conflict, not mere metadata container presence;
- unrelated metadata never triggers a false conflict.

### Phase 3 acceptance criteria

- all metadata-only payload-preservation invariants pass;
- repeated replacement does not append duplicates;
- timestamp nondeterminism is controlled in tests;
- update policies have equivalent semantics across formats;
- no unrelated metadata is lost;
- any accepted format limitation is entered in Table E.

Suggested commit:

```text
tests: verify metadata preservation and idempotence
```

---

## Phase 4: Verify hidden-marker success and fallback reporting

### 4.1 Supported inputs

For each retained hidden-marker path:

- embed using an explicit seed;
- verify using the expected key/seed path;
- confirm `ExecutionReport` marks attempted/succeeded accurately;
- confirm rights metadata is still present;
- confirm output decodes externally.

### 4.2 Unsupported inputs

At minimum include:

- an unsupported JPEG class identified by Plan 040;
- lossy WebP if hidden-marker embedding remains unsupported;
- insufficient-capacity image;
- progressive JPEG if DCT editing is unsupported.

Expected:

- rights metadata succeeds where safe;
- hidden marker does not claim success;
- warning is specific enough to explain the downgrade;
- strict mode behavior matches the selected preset/evidence posture;
- output remains valid and preserved.

### 4.3 Authentication distinctions

Confirm reporting distinguishes:

```text
CRC/best-effort marker verified
HMAC-authenticated marker verified
marker present but key missing
marker authentication failed
marker not found
marker unsupported/fallback
```

Do not treat CRC verification as authenticated provenance.

### Phase 4 acceptance criteria

- supported marker paths verify;
- unsupported paths downgrade explicitly;
- metadata remains the primary success condition for legal-notice presets;
- authenticated and unauthenticated evidence are distinct;
- strict mode behavior is tested without inventing a second warning policy;
- reports match human and JSON output.

Suggested commit:

```text
tests: close hidden-marker reporting across formats
```

---

## Phase 5: Verify API, CLI, feature, and binary integration

### 5.1 API equivalence

For representative configurations, compare:

```text
legacy byte API -> adapter -> request
canonical request byte API
async request wrapper
parallel request helper
```

Expected semantic output/report equivalence. Byte equality is not required where timestamps or ordering are intentionally variable; control those inputs where possible.

### 5.2 CLI equivalence

Run representative legacy and canonical syntax pairs and compare resolved plans/reports.

Required conflict tests remain exit code 2 with no output written.

### 5.3 Pixel-only API

Compile examples/tests proving:

- pixel-only operation returns pixels only;
- it does not report rights metadata as injected;
- byte API replacement is clear.

### 5.4 Feature builds

Run the bounded feature checks established by Plan 043:

```text
minimal/no-default library
normal CLI
all features
signatures/detached manifests
ISCC feature if introduced
conformance feature/binary if introduced
async feature
parallel feature if introduced
```

Do not test the full powerset.

### 5.5 Binary measurement confirmation

Reproduce the final Plan 043 stripped size from a clean build environment sufficiently similar to the recorded environment. Small toolchain drift should be documented rather than treated as a correctness failure.

### Phase 5 acceptance criteria

- API/CLI semantic equivalence passes;
- conflict behavior is stable;
- pixel-only contract is explicit;
- bounded feature builds pass;
- no capability is accidentally inaccessible;
- final size measurement is reproducible within explained environmental variation;
- no size gate is added to CI.

Suggested commit:

```text
tests: close API feature and artifact integration
```

---

## Phase 6: Run one bounded external interoperability pass

Use the existing manual external verification workflow or equivalent local setup once.

### 6.1 Required observations

For representative PNG, JPEG, and WebP outputs:

- external tool reads the canonical `plus:DataMining` URI;
- external tool reads `plus:OtherConstraints` when applicable;
- external tool reads standard copyright/creator/usage terms where emitted;
- image decoder opens the output;
- WebP tool reports valid VP8X/chunk structure;
- JPEG tool reports preserved ICC/IPTC/APP metadata fixture where applicable;
- no malformed EXIF warning appears for new WebP output.

### 6.2 Tool economy

Use a small set, such as:

```text
ExifTool
xmllint
ImageMagick or libvips
webpmux/webpinfo for WebP
```

Not every tool is required if unavailable. Record exact unavailable prerequisites.

### 6.3 Workflow policy

Do not make this pass automatic. The existing `workflow_dispatch` external workflow may be updated to enable coherent features or use corrected fixtures, but it remains one manual workflow/job.

### Phase 6 acceptance criteria

- one exact external run or local command set is recorded;
- canonical properties are independently observed;
- all representative outputs decode;
- exact tool versions are recorded;
- unavailable tools are recorded as blockers without false success;
- required CI remains unchanged.

Suggested commit, only if workflow/fixtures require correction:

```text
verification: align manual interoperability checks with corrected formats
```

---

## Phase 7: Documentation truth pass

Audit current documentation, not historical plans, for stale claims.

Required search themes:

```bash
rg -n "canonical|PLUS|DataMining|DMI-|lossless|fast path|progressive|restart|WebP|EXIF|DynamicImage|test-seeds|poison|trap|noai|noindex" \
  README.md src architecture AGENTS.md STABILITY.md DEPRECATIONS.md CHANGELOG.md
```

### 7.1 Product positioning

Current docs must say:

- StegoEggo writes rights-reservation metadata;
- metadata is visible/removable and is not DRM;
- hidden markers are optional best-effort evidence;
- HMAC/signatures prove key possession/integrity, not copyright ownership;
- detached manifests are optional provenance sidecars;
- no claim proves that a model trained on an image.

### 7.2 Standards claims

Current docs must show complete canonical URIs or describe them accurately.

Historical private forms must be labeled compatibility input.

### 7.3 Format claims

JPEG:

- metadata-only preservation is described;
- DCT supported subset/fallback is explicit;
- “lossless” is scoped or removed;
- progressive/restart/color limitations match implementation.

WebP:

- simple-to-extended conversion and VP8X flags are described;
- XMP/EXIF behavior is accurate;
- lossy hidden-marker limitations remain explicit.

PNG:

- metadata and LSB behavior are accurately separated.

### 7.4 API/CLI claims

- request-based bytes API is canonical;
- pixel-only APIs cannot preserve file metadata;
- mixed CLI option behavior and key sources match implementation;
- test-seed guessing is not normal production behavior.

### 7.5 Release claims

- no current documentation implies CI publication;
- manual crates.io release remains maintainer-owned;
- any future release requires an unused version;
- this roadmap has not published a version.

### Phase 7 acceptance criteria

- all current documentation claims are supportable by evidence;
- stale poisoning/trap terminology is absent from user-facing current docs except compatibility/deprecation notes;
- examples compile where intended;
- historical plans remain truthful;
- changelog records unreleased changes without a version claim;
- no publication automation is restored.

Suggested commit:

```text
docs: close rights metadata and format correctness roadmap
```

---

## Phase 8: Final verification and closure disposition

### 8.1 Required local verification

Run:

```bash
./scripts/check.sh
```

Run the targeted cross-format tests added by this roadmap.

Do not rerun every specialist diagnostic unless a changed file affects it.

### 8.2 Required CI evidence

Record one green run of the existing single `Check` job at the final implementation SHA.

Do not add or rename required jobs merely to create closure evidence.

### 8.3 Manual external evidence

Record the manual external verification result from Phase 6.

A tool-installation or environment blocker may remain `BLOCKED-EXTERNAL` while source correctness is otherwise closed, but the roadmap cannot claim external interoperability evidence that was not observed.

### 8.4 Complexity audit

Confirm:

```text
automatic push/PR workflows: 1
required jobs: 1
required matrices: 0
automated publication paths: 0
tag-triggered release mutation: 0
new permanent evidence frameworks: 0
new format-specific workflow fan-out: 0
```

### 8.5 Final limitations

Every remaining limitation must have:

- affected input class;
- safe behavior;
- warning/report behavior;
- documentation location;
- whether future work is necessary.

Examples of acceptable closure limitations:

- progressive JPEG receives metadata-only fallback;
- restart-bearing JPEG receives metadata-only fallback;
- lossy WebP hidden marker unsupported while metadata works;
- animated WebP metadata support is preserved-only or explicitly unsupported;
- historical malformed EXIF is parsed only heuristically for compatibility.

Examples of defects that cannot close as limitations:

- output image does not decode;
- Allowed emits a prohibition marker;
- canonical URI uses an arbitrary/bare key;
- metadata-only path drops unrelated data contrary to contract;
- report claims hidden marker succeeded when it did not;
- CLI silently discards explicit policy input.

### Phase 8 acceptance criteria

- `./scripts/check.sh` passes;
- final single-job CI is green;
- external evidence is recorded or explicitly blocked;
- complexity budget remains intact;
- status ledgers 039-044 are complete;
- limitations are safe and documented;
- no release side effect occurred.

Suggested commit:

```text
plans: close rights metadata and format correctness roadmap
```

---

## Minimal closure fixture matrix

The final fixture set should remain approximately this size, reusing files across cases:

```text
PNG
  1 simple metadata/LSB fixture
  1 existing unrelated metadata fixture

JPEG
  1 supported baseline 4:2:0 fixture with metadata
  1 opaque APP/ICC/IPTC preservation fixture
  1 unsupported progressive or restart fixture

WebP
  1 simple VP8L/VP8 conversion fixture
  1 extended XMP/EXIF/ICC fixture
  1 malformed/duplicate metadata fixture
```

Rights policies can generally be applied dynamically to these base images. Do not commit one binary image for every policy.

---

## Required roadmap closure criteria

The roadmap started by asking whether StegoEggo accomplishes its intended goal, where it is over-engineered, where binary size can be reduced, and what bugs remain.

The answer is closed only when the repository demonstrates:

### Intended goal

- standards-correct rights metadata is written and independently readable;
- unrelated image/container data is preserved where promised;
- optional hidden evidence is bounded and truthful;
- output remains valid.

### Improvement areas corrected

- canonical URI and constraints semantics corrected;
- JPEG corruption/segment-loss risks corrected or contained;
- WebP VP8X/XMP/EXIF structure corrected;
- duplicated API/CLI policy paths consolidated;
- metadata-losing pixel API contract corrected.

### Over-engineering reduced

- no second policy engine remains;
- custom JPEG scope is bounded rather than expanded;
- malformed private EXIF is removed or corrected;
- verification remains targeted, not a new CI apparatus;
- speculative size changes are rejected.

### Binary size addressed

- baseline and final measurements exist;
- coherent optional capabilities are gated where beneficial;
- production test-only features are absent;
- no user-visible capability is deleted;
- retained dependencies have documented reasons.

### Bugs closed

- every review finding is either fixed or entered as a safe explicit limitation;
- no release-blocking correctness defect remains hidden behind self-round-trip tests.

---

## Completion definition

Plan 044 and Roadmap 038 are complete when StegoEggo is a coherent rights-metadata tool rather than a collection of individually impressive but inconsistently integrated evidence mechanisms.

The final repository may still have a narrow DCT subset and format-specific limitations. Closure requires those boundaries to be safe, externally observable, accurately reported, and proportionate to the project—not erased through additional complexity.