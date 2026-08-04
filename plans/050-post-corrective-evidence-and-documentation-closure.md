# Plan 050: Post-Corrective Evidence and Documentation Closure

Status: Ready for implementation

Baseline: `main` after Plan 049 planning commit `8215c0a210bef6a3ed1d54d737cc36a3d6ecc918`

Depends on completed implementation of:

- `plans/046-rights-metadata-canonical-classification-corrective-pass.md`
- `plans/047-cli-default-policy-and-equivalence-corrective-pass.md`
- `plans/048-jpeg-dct-preservation-and-entropy-corrective-pass.md`
- `plans/049-webp-xmp-replacement-and-feature-flags-corrective-pass.md`

Closes:

- `plans/045-corrective-correctness-closure-roadmap.md`

Reconciles incomplete or missing evidence for:

- `plans/039-plus-iptc-rights-metadata-correctness.md`
- `plans/040-jpeg-dct-container-correctness-and-containment.md`
- `plans/041-webp-container-xmp-exif-correctness.md`
- `plans/042-api-cli-contract-consolidation.md`
- `plans/043-measured-binary-size-reduction.md`
- `plans/044-cross-format-correctness-closure.md`

---

## Purpose

Perform one final bounded verification and documentation pass after Plans 046-049, correct the repository's planning evidence, and determine whether the rights-metadata correctness roadmap can truthfully close.

This plan is necessary because Plans 039-044 were marked or described as complete without the required status ledgers, and some closure tests accepted behavior that contradicted their own plan criteria. The corrective implementation plans address the code defects. This plan addresses evidence truth, final integration, and current documentation.

It must not become a new permanent certification system.

---

## Governing constraints

1. Do not begin this plan until Plans 046-049 have implementation commits and status ledgers.
2. Do not fix substantive Plan 046-049 defects here. Reopen the owning plan or create a narrowly scoped follow-up if a release-blocking defect remains.
3. Required CI remains one stable `Check` job invoking `scripts/check.sh`.
4. Do not add a matrix, required external-tool workflow, release workflow, publication workflow, artifact evidence pipeline, scheduled fuzzing, or binary-size gate.
5. Do not bump versions, publish crates, create tags, or create releases.
6. Use a compact representative fixture matrix. Do not multiply tests merely to increase coverage counts.
7. Internal writer/parser agreement is not independent interoperability evidence.
8. External tools are manual/optional. Unavailable tools are recorded honestly.
9. Historical status ledgers created now must be labeled retrospective.
10. Do not falsify phase timing. If a plan required a ledger before source edits and that did not occur, say so.
11. Current product docs must state no stronger guarantee than the final evidence.
12. Accepted limitations must be safe, explicit, and outside the corrective requirements. A required defect cannot be relabeled as an accepted limitation.

---

## Phase 0: Create the final status ledger

Create `plans/050-status.md` before closure edits.

Initialize it with:

```text
Plan baseline SHA: 8215c0a210bef6a3ed1d54d737cc36a3d6ecc918
Disposition: OPEN
Plan 046: OPEN
Plan 047: OPEN
Plan 048: OPEN
Plan 049: OPEN
Cross-format semantic matrix: OPEN
Container preservation matrix: OPEN
CLI/API equivalence: OPEN
Independent interoperability: OPEN
Feature/build verification: OPEN
Binary measurement: OPEN
Retrospective ledgers 039-044: OPEN
Documentation truth: OPEN
Final CI: OPEN
Remaining limitations: OPEN
Publication hold: no publication is part of this plan
```

Add these tables.

### Table A: corrective plan closure

```text
plan | baseline | implementation commits | status ledger | blocking criteria passed | open defects | disposition
```

Required rows: 046, 047, 048, 049.

### Table B: cross-format semantic evidence

```text
case | PNG raw representation | JPEG raw representation | WebP raw representation | extracted semantics | independent observation | status
```

### Table C: preservation evidence

```text
fixture/format | operation | intended byte changes | preserved inventory | decoder result | independent result | status
```

### Table D: CLI/API equivalence

```text
case | legacy request | canonical request | resolved plan comparison | output/report comparison | status
```

### Table E: retrospective plan reconciliation

```text
plan | original claimed status | missing evidence/process issue | original implementation commits | corrective commits | final disposition
```

### Table F: accepted limitations

```text
limitation | affected input | safe behavior | warning/report | documentation | future work | accepted
```

No table may use `COMPLETE` when an exact blocking criterion remains unverified.

---

## Phase 1: Verify corrective plan ledgers before integration testing

Read:

```text
plans/046-status.md
plans/047-status.md
plans/048-status.md
plans/049-status.md
```

For each plan confirm:

- baseline SHA is correct;
- implementation commits are listed;
- exact tests/commands are recorded;
- test claims match committed test names;
- unavailable external tools are not reported as passing;
- documentation changes are listed;
- no publication occurred;
- open limitations are distinguished from defects.

If a ledger is missing or overclaims completion, correct the ledger before integration testing. If source behavior is missing, return to the owning implementation plan.

### Phase 1 acceptance criteria

- all four corrective ledgers exist;
- all blocking criteria have evidence or an exact open defect;
- no plan is accepted based only on commit messages;
- publication hold is explicit;
- Plan 050 does not silently absorb source corrections.

Suggested commit, only if ledger corrections are needed:

```text
plans: reconcile corrective implementation evidence
```

---

## Phase 2: Run the compact cross-format semantic matrix

Primary tests should already exist after Plans 046-049. Modify them only to correct integration mismatches, not to add broad new coverage.

Required policies:

```text
Unspecified
Allowed
ProhibitedAiMlTraining
ProhibitedGenerativeAiTraining
ProhibitedExceptSearchIndexing
ProhibitedAllDataMining
ProhibitedSeeConstraints with constraints
```

Required formats:

```text
PNG
JPEG
WebP
```

### 2.1 Exact raw representation

For each non-unspecified policy:

- locate the XMP packet independently of the project semantic parser;
- assert the exact full PLUS vocabulary URI;
- assert there is one semantic DataMining value;
- assert no bare key is emitted as the complete property value;
- assert no `noai`, `noindex`, or private `DMI-PROHIBITED` marker is emitted.

For `Unspecified`:

- assert no `plus:DataMining` property;
- assert no `DMI-UNSPECIFIED` bytes;
- assert other requested legal fields remain.

For `ProhibitedSeeConstraints`:

- assert canonical URI;
- assert `plus:OtherConstraints` with exact caller text;
- assert missing-constraint behavior matches Plan 046.

### 2.2 Representation classification

Use independently constructed fixtures for:

- canonical full URI;
- bare known key;
- arbitrary-origin URL ending in a known key;
- unknown key under exact PLUS prefix;
- canonical plus agreeing legacy value;
- canonical plus conflicting legacy value.

Required outcomes:

```text
full URI -> canonical
bare key -> readable legacy/noncanonical
arbitrary origin -> not canonical and no suffix inference
unknown key -> no known canonical policy
canonical conflict -> canonical precedence plus visible conflict
```

### 2.3 Common legal fields

For one representative complete notice, compare across formats:

```text
copyright holder
creator
usage terms
usage terms language
web statement of rights
credit line
copyright owner
licensor fields
metadata date
notice applied timestamp where deterministic
AI constraints
```

Do not require unsupported container-specific legacy fields to be byte-identical. Require the documented semantic values.

### Phase 2 acceptance criteria

- every policy has exact raw and semantic evidence;
- unspecified omission is exact;
- arbitrary-origin suffix inference is gone;
- bare compatibility is noncanonical;
- constraints are standard and observable;
- common legal fields agree across formats;
- no project-owned parser is the sole evidence for raw representation.

Suggested commit, only if final integration tests require correction:

```text
tests: finalize cross-format rights semantics evidence
```

---

## Phase 3: Run container and hidden-marker preservation matrix

### 3.1 PNG

For metadata-only same-format processing:

- IDAT chunk payload bytes remain identical;
- unrelated text/XMP chunks remain according to metadata policy;
- all chunk CRCs validate;
- one owned XMP packet/field set remains after repeated replacement.

For LSB hidden marker:

- expected pixel bytes change;
- output decodes;
- marker verifies with fixed seed/key as applicable;
- rights metadata remains present.

### 3.2 JPEG

Metadata-only:

- SOS header and entropy scan through EOI remain byte-identical;
- unrelated APP/COM/DRI/unknown segments remain;
- only owned metadata segments change.

Supported DCT hidden marker:

- DQT and entropy scan may change;
- APP2 ICC, APP13, APP14, COM, unknown APP, SOF, DHT, and SOS header remain according to Plan 048;
- marker verifies;
- output decodes.

Unsupported JPEG:

- progressive, restart-bearing, and multi-scan inputs do not claim DCT success;
- metadata remains correct and output-preserving;
- warning/report identifies downgrade truthfully.

Malformed entropy:

- no partial success;
- no panic;
- no output claiming a verified marker.

### 3.3 WebP

Metadata rewrite:

- exactly one VP8X;
- exactly one XMP when rights metadata is requested;
- unrelated XMP property survives;
- existing EXIF survives;
- VP8X bits match final ICC/XMP/EXIF/alpha/animation features;
- VP8/VP8L/animation payload chunks remain byte-identical;
- unknown chunks remain according to Plan 049;
- repeated replacement remains bounded.

Hidden marker:

- lossless WebP LSB behavior matches current support contract;
- lossy/unsupported behavior downgrades explicitly;
- metadata remains the primary legal-notice channel.

### 3.4 Authentication distinctions

Across representative supported formats verify reports distinguish:

```text
CRC/best-effort marker valid
HMAC marker valid
key missing
wrong key/authentication failed
marker absent
marker unsupported/fallback
```

Do not describe CRC as authenticated provenance.

### Phase 3 acceptance criteria

- metadata-only payload/container preservation passes;
- supported marker paths verify;
- unsupported paths downgrade without false success;
- JPEG successful DCT output preserves unrelated segments;
- WebP has one coherent XMP packet and accurate flags;
- authentication states remain distinct;
- outputs decode.

Suggested commit, only if integration assertions require correction:

```text
tests: finalize container preservation and marker outcomes
```

---

## Phase 4: Verify CLI and library equivalence

### 4.1 CLI default and syntax pairs

Verify the exact cases from Plan 047:

- normal default invocation;
- omitted `--dmi` versus `--dmi auto`;
- explicit `--dmi unspecified`;
- legacy DMI versus canonical rights policy;
- shorthand versus canonical equivalent;
- preset/channel equivalents where genuinely equivalent;
- conflict cases with exact exit code 2 and no output.

Compare resolved requests/plans, not only successful execution.

### 4.2 Dry-run and execution

For representative metadata-only and metadata-plus-marker requests:

- dry-run resolved policy/channels match execution report;
- JSON and human output report the same material outcome;
- fixed seed/key inputs are used.

### 4.3 Single and batch

Compare:

```text
single file
batch jobs=1
batch jobs>1
```

Every output must preserve requested policy and channel semantics.

### 4.4 Library adapters

Use representative legacy byte API and canonical request byte API configurations.

Compare:

```text
resolved policy
metadata update policy
legal fields
hidden-marker attempt/success
warnings
verification result
```

Pixel-only APIs must continue to state and report that container metadata is not returned.

### Phase 4 acceptance criteria

- normal CLI defaults are correct;
- claimed syntax equivalence is semantic;
- dry-run matches execution;
- JSON/human reports agree materially;
- batch mode preserves request semantics;
- legacy library byte adapters converge on canonical behavior;
- pixel-only contract remains explicit.

Suggested commit, only if integration assertions require correction:

```text
tests: finalize CLI and API semantic equivalence
```

---

## Phase 5: Bounded feature and build verification

Use the existing feature boundaries. Do not test the complete powerset.

Run:

```bash
cargo check -p stegoeggo --no-default-features
cargo test -p stegoeggo --no-default-features
cargo test --workspace --all-features
cargo build --release -p stegoeggo-cli
cargo build --release --bin stegoeggo-conformance --features conformance
```

Run targeted optional paths as applicable:

```bash
cargo test -p stegoeggo --features async --test async_integration
cargo test -p stegoeggo-cli --features signatures
```

If detached-manifest/signature tests are already covered by all-features workspace tests, do not duplicate expensive invocations without reason.

### 5.1 Binary measurement

Run:

```bash
./scripts/measure_binary_size.sh
```

Record:

```text
rustc version
target triple
profile settings
binary bytes
comparison to the recorded Plan 043 result
```

Toolchain drift is informational unless it exposes a major regression. Do not add a size gate.

### Phase 5 acceptance criteria

- minimal library builds without optional heavy dependencies;
- normal CLI builds with intended features;
- all-features tests pass;
- conformance binary remains feature-gated;
- signatures/async paths remain available where documented;
- final size is recorded;
- no new CI job is added.

Suggested commit only for necessary feature/documentation corrections:

```text
build: close post-corrective feature integration
```

---

## Phase 6: One bounded independent interoperability pass

Use available local tools. Recommended set:

```text
ExifTool
ImageMagick identify or libvips
webpinfo or webpmux
xmllint if useful
```

Do not require all tools.

### 6.1 Rights properties

For representative PNG, JPEG, and WebP outputs observe externally:

- exact `plus:DataMining` full URI;
- `plus:OtherConstraints` for the constraints case;
- standard copyright/creator/usage terms where applicable;
- no DataMining property for unspecified.

### 6.2 Container validity

Observe:

- PNG decodes and chunk CRCs are valid through available tooling;
- JPEG decodes and preserved ICC/IPTC/APP metadata remains visible where the tool supports it;
- WebP decodes, contains one XMP packet, and reports matching VP8X features;
- new WebP output generates no malformed EXIF warning.

### 6.3 Record exact evidence

In `plans/050-status.md`, record:

```text
tool name/version
command
fixture
observed output summary
commit SHA
pass/fail/unavailable
```

Do not commit large generated output artifacts unless the repository already has a small fixture convention requiring them. Prefer concise command/output evidence in the ledger.

### Phase 6 acceptance criteria

- at least one independent metadata reader observes canonical properties for each format, when available;
- at least one independent image decoder opens each representative output;
- WebP structure is independently inspected when a tool is available;
- unavailable tools are recorded without false success;
- no automatic external-tool CI is added.

Suggested commit only if fixture or manual workflow correction is needed:

```text
verification: align manual interoperability evidence after corrective pass
```

---

## Phase 7: Create truthful retrospective ledgers for Plans 039-044

Create:

```text
plans/039-status.md
plans/040-status.md
plans/041-status.md
plans/042-status.md
plans/043-status.md
plans/044-status.md
```

Each must begin with a notice such as:

```text
This ledger was created retrospectively during Plan 050. The original plan required a status ledger before source edits, but no such file was committed at that time. This document records the implementation and later corrective evidence without claiming the original process requirement was met.
```

### 7.1 Required contents

Each ledger must include:

```text
original plan baseline
original implementation commits
original claimed completion state
criteria that actually landed
criteria found incomplete by the post-plan audit
corrective plan and commit references
final verification evidence
final disposition
publication status
```

### 7.2 Required final dispositions

Use evidence-based dispositions such as:

```text
COMPLETE AFTER CORRECTIVE PASS
PARTIAL
SUPERSEDED BY 045-050
```

Do not use plain `COMPLETE` without acknowledging corrective work.

Expected likely relationships:

- Plan 039: complete only after Plan 046;
- Plan 040: complete only after Plan 048;
- Plan 041: complete only after Plan 049;
- Plan 042: complete only after Plan 047 plus retained adapter evidence;
- Plan 043: may remain complete if feature and measurement evidence is reproduced;
- Plan 044: superseded because its original completion claim was incorrect; final cross-format closure is Plan 050.

### 7.3 Correct current plan headers

Update the header/status of `plans/044-cross-format-correctness-closure.md` to state that its completion claim was superseded/corrected by Roadmap 045 and Plan 050.

Do not delete or rewrite its original phases.

Update Roadmap 045 and Plans 046-050 status headers only when their ledgers justify it.

### Phase 7 acceptance criteria

- all six retrospective ledgers exist;
- retrospective timing is explicit;
- original and corrective commits are separated;
- no false pre-edit ledger claim appears;
- Plan 044's current status no longer overclaims closure;
- final dispositions match evidence.

Suggested commit:

```text
plans: reconcile 039-044 with corrective closure evidence
```

---

## Phase 8: Current documentation truth pass

Search current docs and source docs for stale claims:

```bash
rg -n "DMI-UNSPECIFIED|canonical|PLUS|DataMining|from_plus_vocab_key|tdm:reserve_tdm|noai|noindex|lossless JPEG|fast path|multi-scan|restart|progressive|WebP|EXIF|XMP|DynamicImage|test-seeds|default policy" \
  README.md src architecture AGENTS.md SUPPORT.md STABILITY.md DEPRECATIONS.md CHANGELOG.md RELEASING.md
```

### 8.1 Rights semantics

Ensure docs state:

- full URI is canonical;
- bare keys are compatibility input only;
- arbitrary-origin suffixes are rejected/noncanonical;
- unspecified omits DataMining;
- constraints use `plus:OtherConstraints`;
- image metadata does not emit TDMRep reservation state.

### 8.2 CLI

Ensure docs state:

- normal default policy;
- omitted versus auto behavior;
- explicit unspecified behavior;
- conflict rules and exit code;
- canonical request path.

### 8.3 JPEG

Ensure docs state the exact supported single-scan baseline subset and explicit fallbacks.

Do not imply arbitrary lossless JPEG editing. Describe intentional DQT/entropy changes and unrelated segment preservation.

### 8.4 WebP

Ensure docs state:

- simple-to-VP8X conversion;
- one-XMP invariant;
- unrelated XMP/EXIF preservation scope;
- output-derived feature flags;
- XMP-only new seed emission;
- backward-compatible EXIF extraction.

### 8.5 Limitations

Document only safe remaining limitations, likely including:

- no progressive/restart/multi-scan JPEG DCT embedding;
- no guarantee of hidden-marker survival after arbitrary transformations;
- lossy WebP hidden-marker limitations;
- external rights metadata is a machine-readable reservation signal, not DRM or proof of enforcement.

### Phase 8 acceptance criteria

- current docs match tested behavior;
- no stale contradictory claims remain;
- limitations are bounded and truthful;
- release procedure remains manual;
- no version bump is made.

Suggested commit:

```text
docs: close post-corrective product and support claims
```

---

## Phase 9: Final bounded verification and disposition

Run exactly the repository-required check:

```bash
./scripts/check.sh
```

Also run the targeted commands recorded in prior phases if they were not already included in the final commit state.

Record in `plans/050-status.md`:

```text
final HEAD SHA
command
start/end or duration if readily available
exit status
relevant test count
known ignored tests
CI run URL/status when available
external-tool availability
binary size
remaining limitations
publication status
```

Do not claim CI success from a local command. If no GitHub Actions result is available at handoff time, record:

```text
Local required check: PASS
Remote CI: NOT YET OBSERVED
```

Do not wait for background completion or create additional workflows.

### Final disposition rules

Mark Plan 050 and Roadmap 045 `COMPLETE` only if every blocking criterion below passes.

Mark `PARTIAL` if any remains open, and list exact source/test ownership.

---

## Blocking closure criteria

1. Unspecified output contains no DataMining property or `DMI-UNSPECIFIED` value.
2. Canonical full URIs and legacy bare keys are classified distinctly.
3. Arbitrary-origin URLs do not establish canonical policy by suffix.
4. The normal CLI default policy is restored and tested through resolved-request comparison.
5. Omitted and auto DMI behavior are equivalent; explicit unspecified is distinct.
6. Successful JPEG DCT embedding uses the preserving path and retains unrelated original segments.
7. JPEG canonical Huffman construction handles zero-count lengths correctly.
8. JPEG malformed symbols/runs fail closed.
9. Multi-scan/restart/progressive JPEGs downgrade explicitly.
10. WebP replacement emits one XMP packet and preserves unrelated XMP content.
11. WebP VP8X bits match final XMP/EXIF/ICC/alpha/animation features.
12. PNG/JPEG/WebP outputs decode after representative operations.
13. Authentication and fallback reports remain truthful.
14. Minimal, normal CLI, and all-feature builds pass.
15. Required CI remains one job and no release automation is added.
16. Plans 039-050 have truthful status records.
17. Current documentation matches final behavior.
18. No version bump, publication, tag, or release occurs.

---

## Definition of done

Plan 050 is complete only when:

- all blocking closure criteria pass;
- `plans/050-status.md` contains exact final evidence;
- retrospective ledgers 039-044 exist and are explicitly retrospective;
- Plan 044 no longer claims unqualified completion;
- Roadmap 045 and Plans 046-050 have evidence-backed final statuses;
- `./scripts/check.sh` passes on final HEAD;
- remote CI status is recorded accurately if observed;
- unavailable external tools are recorded honestly;
- binary size is recorded without a gate;
- no release side effect occurs.

If closure is partial, leave a short exact follow-up section in `plans/050-status.md`. Do not create another broad roadmap unless the remaining work genuinely spans multiple independent subsystems.