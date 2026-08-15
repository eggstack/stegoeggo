# Plan 068: Tiled-JPEG Single-Decode Search Context Corrective Closure

Status: Ready for implementation

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Corrective predecessor: `plans/067-tiled-jpeg-candidate-integrity-and-final-evidence-closure.md`

Audited implementation baseline: `main` at `22f76d0b1b1e540160dee5c6138d6e7a3aca301b`

Authoritative implementation ledger to create as the first implementation change: `plans/068-status.md`

---

## 1. Purpose

Plan 067 corrected the important tiled-JPEG correctness regression introduced by the carrier-boundary move:

- tiled candidate identity is now preserved with an opaque primitive key;
- prefix, header, full-payload, and legacy extraction lengths use the same candidate identity;
- a wrong early `NotV3`, malformed, unsupported, integrity-failed, or authentication-failed candidate no longer masks later valid candidates;
- tiled embedding now verifies the encoded output before reporting `EmbedOutcome::Embedded`;
- the root crate no longer needs JPEG parser/coefficient/F5 types;
- legacy adapter tests now exercise the real private production adapter.

A post-Plan-067 source audit found one remaining runtime/resource regression in the new support boundary.

Current behavior at baseline `22f76d0` is approximately:

```text
root verification
    |
    v
jpeg_tiled_prefix_candidates(...)
    |
    +-- decode JPEG coefficients once
    +-- enumerate up to bounded candidate keys + prefixes
    |
    v
for each candidate
    |
    +-- V3 candidate:
    |      jpeg_tiled_extract_candidate(header_bits)
    |          -> decode JPEG coefficients again
    |      jpeg_tiled_extract_candidate(full_bits)
    |          -> decode JPEG coefficients again
    |
    +-- NotV3 candidate:
           jpeg_tiled_extract_candidate(v2_bits)
               -> decode JPEG coefficients again
           jpeg_tiled_extract_candidate(v1_bits)
               -> decode JPEG coefficients again
```

With the default `max_tile_extraction_origins = 16`, the existing nearby-grid search (`3 x 3`) and redundancy search (`1..=10`) allow up to roughly:

```text
16 origins * 9 nearby seed coordinates * 10 redundancy values
= 1,440 candidate variants
```

before image-edge reductions.

A no-match or wrong-seed JPEG can therefore cause thousands of complete JPEG coefficient decodes during one verification operation. The old pre-boundary implementation decoded the coefficient container once and searched all bounded candidates against that in-memory representation.

This plan fixes only that repeated-decode regression while preserving every correctness and encapsulation property established by Plan 067.

---

## 2. Required end state

At completion, one tiled-JPEG extraction/verification search must have this semantic shape:

```text
encoded JPEG
    |
    v
carrier creates one opaque tiled-JPEG search context
    |
    +-- bounded JPEG decode / coefficient parse ONCE
    +-- context owns all private header/coefficient state
    |
    v
enumerate candidate keys + prefixes from that context
    |
    +-- candidate A
    |      extract header A from same context
    |      extract full/legacy A from same context
    |
    +-- candidate B
    |      extract header B from same context
    |      extract full/legacy B from same context
    |
    ...
    |
    v
context dropped at end of operation
```

The root crate must still see only operation-level carrier types and primitive results. It must not gain access to:

```text
JpegHeader
Coefficients
JpegTranscoder
DctStegoF5
Huffman state
entropy decoder state
DCT block vectors / block-set internals
```

The default `stegoeggo-stego` API must remain unchanged.

---

## 3. Explicit goals

Plan 068 must:

1. eliminate repeated `JpegTranscoder::decode_coefficients()` calls for candidate-stage extraction within one tiled-JPEG search;
2. preserve exact Plan-067 candidate identity semantics;
3. preserve existing candidate order, nearby-grid range, redundancy range, and `max_origins` meaning;
4. preserve legacy V1/V2 tiled extraction fallback;
5. preserve V3 prefix -> exact-key header -> exact-key full-payload behavior;
6. preserve verification failure classification semantics;
7. keep JPEG codec/coefficient internals private to `stegoeggo-stego`;
8. keep the support mechanism behind the existing `application-support` feature;
9. add direct evidence that one search does not perform a full coefficient decode per candidate;
10. reconcile Roadmap 057 / Plan 067 status only after the runtime correction is verified.

---

## 4. Explicit non-goals

Do not use Plan 068 to:

- change the F5 embedding algorithm;
- change payload framing or payload versions;
- change LSB carrier math;
- change `tile_seed()`;
- increase or reduce the nearby-grid search from `0..=2` in x/y;
- increase or reduce redundancy probing from `1..=10`;
- change the meaning of `max_tile_extraction_origins`;
- change default resource-limit values merely to hide the repeated-decode problem;
- skip candidate variants for performance;
- cache decoded JPEG state globally;
- add cross-request caches;
- add `unsafe` code;
- expose JPEG parser/coefficient objects publicly;
- add another crate;
- add a generic steganography trait/session framework;
- redesign the normal non-tiled JPEG API;
- redesign tiled embedding unless a mechanical shared helper is clearly smaller;
- modify unrelated `e694209` conformance/XMP/WebP/payload changes;
- expand CI;
- add release automation;
- bump versions;
- publish crates;
- create tags or GitHub Releases.

If an unrelated defect is discovered, record it separately rather than expanding this pass.

---

# Phase 0 — create truthful Plan 068 status and reopen Roadmap 057

## 0.1 Create `plans/068-status.md` before product edits

Because `plans/` is ignored, implementation must force-track the ledger before editing source:

```bash
git add -f plans/068-status.md
git ls-files --error-unmatch plans/068-status.md
git status --short
```

Record the actual:

```text
starting HEAD
working-tree status
root/carrier/CLI versions
root -> carrier dependency declaration
Roadmap 057 status
Plan 067 status
```

If source edits accidentally precede the tracked ledger, state that truthfully. Do not fabricate chronology.

## 0.2 Initial status rows

Start every row `OPEN`:

```text
R01 one carrier-owned decoded search context per tiled-JPEG search
R02 prefix enumeration uses the retained decoded context
R03 exact-candidate header extraction uses the retained decoded context
R04 exact-candidate full/legacy extraction uses the retained decoded context
R05 no per-candidate coefficient re-decode in normal tiled search
R06 candidate identity semantics from Plan 067 preserved
R07 wrong-first/later-valid regression preserved
R08 V3 authentication/failure classification preserved
R09 legacy V1/V2 candidate fallback preserved
R10 max_origins still bounds tile origins, not candidate variants
R11 nearby seed range remains 0..=2 and redundancy remains 1..=10
R12 carrier codec/coefficient internals remain private
R13 default public carrier API unchanged
R14 root uses only operation-level application-support type(s)
R15 decode-count/instrumentation regression proves bounded single decode per search
R16 focused tiled-JPEG carrier tests pass
R17 focused root tiled-JPEG tests pass
R18 carrier boundary/doctests pass
R19 ./scripts/check.sh passes
R20 staged pre-release structural check passes
R21 Plan 067 residual note reconciled truthfully
R22 Roadmap 057 final closure is evidence-consistent
```

## 0.3 Reopen Roadmap 057 while this plan is active

Set the Roadmap header to:

```text
Status: PARTIAL — final tiled-JPEG runtime residual tracked by Plan 068
```

Add a concise correction note to `plans/067-status.md` that:

- Plan 067 correctly closed candidate-identity correctness;
- a later audit found repeated coefficient decoding in the new exact-key operation;
- Plan 068 owns only that runtime/resource correction;
- Plan 067 historical correctness evidence remains valid.

Do not rewrite Plan 067 as though its correctness work failed.

### Phase 0 acceptance criteria

- 068 ledger is tracked before source edits;
- Roadmap is PARTIAL while 068 is open;
- Plan 067 is preserved as historical correctness evidence;
- no source work begins with false COMPLETE status.

---

# Phase 1 — lock the runtime contract

Before refactoring, document the exact runtime requirement in `plans/068-status.md`.

## 1.1 Decode-count contract

For one call to either root tiled-JPEG extraction/verification entry path using one encoded JPEG input:

```text
full coefficient decode count for candidate search <= 1
```

A second decode is acceptable only if it belongs to a logically separate operation outside the candidate search itself, and that must be explicitly justified in the ledger.

Examples:

- `jpeg_tiled_search(...)` creates a context: 1 decode.
- enumerating 1 candidate or 1,440 candidates: still 1 decode.
- extracting prefix/header/full for any candidate count: still 1 decode.

Do not satisfy the acceptance criterion by redefining a full decode or hiding decode calls behind another helper.

## 1.2 Search-coverage contract

Runtime improvement must not change search coverage:

```text
origins: bounded by max_origins
nearby seed x: 0..=2
nearby seed y: 0..=2
redundancy: 1..=10
```

All existing candidate variants must remain eligible in the same deterministic order unless a focused compatibility test proves ordering is irrelevant. Prefer preserving order exactly.

## 1.3 Candidate-identity contract

Retain the Plan-067 identity requirement:

```text
candidate key selected at prefix stage
    == candidate used for header extraction
    == candidate used for full V3 extraction
    == candidate used for legacy fallback lengths
```

Do not replace the key with byte-based candidate matching or independently regenerated result lists.

### Phase 1 acceptance criteria

- runtime contract is written before implementation;
- implementation cannot pass by reducing search breadth;
- Plan-067 correctness semantics are explicitly frozen.

---

# Phase 2 — introduce one opaque carrier-owned search context

This is the preferred corrective design.

## 2.1 Preferred type shape

Under the existing feature-gated `stegoeggo-stego::application_support` module, introduce a small opaque search object, for example:

```rust
pub struct TiledJpegSearch {
    // PRIVATE carrier-owned fields only
    header: JpegHeader,
    coefficients: Coefficients,
    tile_size: u32,
    tiles_x: u32,
    tiles_y: u32,
}
```

The exact private fields may differ.

Important:

- the type itself may be publicly nameable only because the parent crate must hold it across calls;
- all codec/coefficient fields remain private;
- provide no getters returning parser/coefficient/F5/block state;
- do not implement `Deref`, conversion traits, or debug output that exposes internal coefficient data;
- do not re-export this type from the default root `stegoeggo::stego` facade;
- the type exists only under the carrier's `application-support` feature.

A name such as `TiledJpegSearch`, `TiledJpegSearchContext`, or equivalent is acceptable.

## 2.2 Constructor

Preferred constructor:

```rust
pub fn jpeg_tiled_search(
    jpeg_bytes: &[u8],
    tile_size: u32,
) -> Option<TiledJpegSearch>
```

or:

```rust
impl TiledJpegSearch {
    pub fn new(jpeg_bytes: &[u8], tile_size: u32) -> Option<Self>;
}
```

Constructor responsibilities:

1. validate non-zero / minimum viable `tile_size`;
2. decode coefficients once;
3. derive reusable image/tile geometry once;
4. retain only the private carrier state needed by subsequent operations;
5. return failure without panicking on malformed/unsupported input.

Do not store a copy of the full encoded JPEG unless there is a concrete need.

## 2.3 Context methods

Preferred operation-level methods:

```rust
impl TiledJpegSearch {
    pub fn prefix_candidates(
        &self,
        master_seed: u64,
        max_origins: u32,
        prefix_bits: usize,
    ) -> Vec<(TiledJpegCandidateKey, Vec<u8>)>;

    pub fn extract_candidate(
        &self,
        master_seed: u64,
        candidate: TiledJpegCandidateKey,
        payload_bits: usize,
    ) -> Option<Vec<u8>>;
}
```

Exact names may differ.

The methods must operate entirely against the already-decoded context.

`extract_candidate()` must not call:

```rust
JpegTranscoder::decode_coefficients(...)
```

or any equivalent full JPEG reparse/decode operation.

## 2.4 Keep `TiledJpegCandidateKey` opaque

Retain the current primitive identity concept:

```text
tile_x
tile_y
seed_x
seed_y
redundancy
```

Fields may remain private.

Do not put references, indexes into raw coefficient vectors, parser objects, or pointers into the key.

The search context must defensively validate a key before using it, even if the key originated from the same context. In particular:

- tile coordinates within context geometry;
- seed coordinates within the existing 16x16 grid bound;
- redundancy `1..=10`.

## 2.5 Lifetimes and ownership

Prefer a simple owned context rather than a self-referential or borrowed parser design.

Do not introduce complex lifetimes merely to avoid a small clone of header metadata.

The priority order is:

1. one full decode;
2. private internals;
3. simple ownership;
4. no unsafe;
5. minimal API surface.

### Phase 2 acceptance criteria

- one opaque carrier-owned context contains decoded search state;
- context creation performs the only full candidate-search coefficient decode;
- candidate operations reuse that state;
- no codec/coefficient types cross the boundary;
- no unsafe or global cache is introduced.

---

# Phase 3 — migrate root tiled extraction to one context per operation

## 3.1 Current root shape to remove

At baseline, root roughly does:

```rust
let candidates = jpeg_tiled_prefix_candidates(jpeg_bytes, ...)?;

evaluate_tiled_candidates(candidates, ..., |candidate, bits| {
    jpeg_tiled_extract_candidate(jpeg_bytes, ..., candidate, bits)
})
```

This closure causes a full coefficient decode per call.

## 3.2 Required root shape

Change both ordinary tiled extraction and verification to something equivalent to:

```rust
let search = carrier_support::jpeg_tiled_search(jpeg_bytes, tile_size)?;
let candidates = search.prefix_candidates(master_seed, max_origins, prefix_bits);

self.evaluate_tiled_candidates(candidates, mac_key, |candidate, bits| {
    search.extract_candidate(master_seed, candidate, bits)
})
```

The same `search` object must be reused for:

- prefix enumeration;
- V3 header extraction;
- V3 full extraction;
- V2 fallback extraction;
- V1 fallback extraction.

## 3.3 Preserve shared evaluator

Keep the useful Plan-067 `evaluate_tiled_candidates` abstraction unless changing it is strictly necessary.

It already centralizes:

- candidate continuation;
- V3 detection;
- exact-candidate staged extraction;
- legacy fallback;
- auth/integrity failure classification.

Do not duplicate this logic again between `extract_f5_tiled_candidates()` and `verify_extract_f5_tiled()`.

## 3.4 Avoid accidental second context creation

Audit call flow carefully.

For one invocation of:

```text
extract_f5_tiled_candidates
verify_extract_f5_tiled
```

there should be one search-context constructor call.

Do not create a fresh context inside the evaluator closure or inside `extract_candidate()`.

### Phase 3 acceptance criteria

- ordinary tiled extraction uses one search context;
- verification tiled extraction uses one search context;
- shared evaluator remains single-source-of-truth;
- no per-candidate re-decode remains in root call flow.

---

# Phase 4 — remove/supersede the repeated-decode free functions

The baseline feature support layer exposes:

```rust
jpeg_tiled_prefix_candidates(jpeg_bytes, ...)
jpeg_tiled_extract_candidate(jpeg_bytes, ...)
```

The second function necessarily re-decodes because it receives only bytes + key.

## 4.1 Preferred cleanup

After root migration, remove these free-function forms if they have no remaining internal use.

Prefer the context API as the only tiled-search support surface:

```text
jpeg_tiled_search(...) -> TiledJpegSearch
TiledJpegSearch::prefix_candidates(...)
TiledJpegSearch::extract_candidate(...)
```

Do not retain old free functions merely as wrappers that reconstruct a context and therefore preserve the runtime bug.

## 4.2 Compatibility note

`application-support` is explicitly unstable and parent-crate-specific. This plan may change that feature-gated API without treating it as stable public API.

Do not alter the normal default carrier API:

```text
lsb
jpeg
frame
reports/errors
```

## 4.3 Primitive helper restraint

Do not expand `application_support.rs` with additional parser or coefficient helpers as part of this change.

This plan should preferably reduce the number of tiled-JPEG support entry points.

### Phase 4 acceptance criteria

- no support API remains whose normal exact-candidate use necessarily requires a fresh full decode;
- default carrier API is unchanged;
- application-support surface stays narrow or becomes narrower.

---

# Phase 5 — direct decode-count evidence

Passing correctness tests is not enough; this plan exists because correctness tests missed the runtime regression.

## 5.1 Add test-only decode instrumentation at the narrowest point

Preferred approach: under `#[cfg(test)]`, instrument the carrier's full JPEG coefficient decode entry point used by tiled search.

Examples:

```rust
#[cfg(test)]
static TILED_SEARCH_DECODE_COUNT: AtomicUsize = AtomicUsize::new(0);
```

or a test-only wrapper/counter local to `application_support`.

Requirements:

- production builds must not pay synchronization overhead for the counter;
- do not expose instrumentation as public API;
- reset counter deterministically per test;
- avoid flaky cross-test interference. If a global counter is used, serialize only the small affected test set or use a scoped injection seam.

An even cleaner alternative is a test-only constructor seam that increments a local counter supplied by the test, if implementation remains simple.

## 5.2 Required decode-count tests

At minimum add tests proving:

### A. One-candidate search

```text
construct search
enumerate candidate(s)
extract prefix/header/full from one exact candidate
assert full decode count == 1
```

### B. Wrong-first/later-valid search

Use the existing offset/wrong-first fixture.

```text
first candidate wrong
later candidate valid
multiple exact-candidate extractions occur
assert valid payload recovered
assert full decode count == 1
```

### C. No-match / broad bounded search

Use a JPEG and master seed/payload setup where many candidate variants are evaluated but no payload verifies.

Assert:

```text
candidate count > 1
search exhausts normally
full coefficient decode count == 1
```

The test does not need to enumerate all theoretical 1,440 candidates if that makes the suite slow. It must, however, prove decode count is invariant with multiple candidates.

### D. Legacy fallback

Ensure a `NotV3` candidate that triggers both V2 and V1 extraction lengths does not cause extra full decodes.

## 5.3 Structural source assertion

In addition to runtime instrumentation, audit `application_support.rs` so only the search-context constructor path for tiled extraction calls full coefficient decode.

Do not rely exclusively on grep if helper indirection makes the call graph non-obvious; inspect the call path.

### Phase 5 acceptance criteria

- tests explicitly count full tiled-search coefficient decodes;
- multiple candidate-stage extractions remain at one decode;
- no-match case remains at one decode;
- legacy fallback remains at one decode.

---

# Phase 6 — preserve Plan-067 correctness regressions

Run and retain all Plan-067 tiled-JPEG behavioral tests.

At minimum preserve coverage for:

```text
tiled_jpeg_candidate_key_roundtrips_same_identity
tiled_jpeg_wrong_first_candidate_does_not_mask_later_v3
tiled_jpeg_equal_prefix_bytes_do_not_collapse_distinct_candidate_keys
tiled_jpeg_candidate_extraction_uses_requested_redundancy
tiled_jpeg_max_origins_still_bounds_origin_scan
tiled_jpeg_embed_reports_embedded_only_after_encoded_roundtrip
tiled_jpeg_embed_roundtrip_uses_recorded_successful_tile
tiled_jpeg_failed_roundtrip_is_not_reported_embedded
root wrong-first/later-valid evaluator regression
wrong MAC behavior
legacy V1/V2 extraction tests
```

If test names change due to the context API, preserve the semantic assertions.

## 6.1 Add context-specific identity regression

Add a focused test proving a key from one search context cannot silently operate against incompatible geometry from another context.

Acceptable behavior:

```text
None / structured failure
```

Do not panic or index outside bounds.

The key need not carry a context ID; geometry validation is sufficient if simple and correct.

## 6.2 Do not weaken failed-candidate continuation

Ensure the runtime refactor does not accidentally restore early return on:

```text
NotV3
MalformedV3
UnsupportedVersion
AuthenticationFailed
Invalid integrity
```

### Phase 6 acceptance criteria

- all Plan-067 correctness tests still pass;
- runtime optimization changes no candidate search semantics;
- foreign/incompatible key use fails safely.

---

# Phase 7 — resource-limit and complexity review

This phase is a source audit, not a redesign.

## 7.1 Confirm theoretical bounded work

Record in `068-status.md` the effective default upper bound:

```text
max origins = ResourceLimits default
nearby variants per origin <= 9
redundancy variants <= 10
```

The important post-fix complexity distinction should be documented as:

```text
BEFORE:
O(candidate_count * full_jpeg_decode)

AFTER:
O(1 * full_jpeg_decode + candidate_count * block extraction)
```

Do not claim exact wall-clock improvement without benchmarks.

## 7.2 Do not add a new resource limit

A new `max_tiled_candidate_variants` setting is not required for this fix because existing ranges are already bounded.

Only add a new limit if implementation discovers a concrete independent unbounded path. Otherwise this is out of scope.

## 7.3 No global decoded-JPEG cache

The context is operation-local.

It must not outlive the root verification/extraction call or be stored globally.

### Phase 7 acceptance criteria

- complexity improvement is accurately documented;
- existing resource bounds are unchanged;
- no cross-request cache/state is introduced.

---

# Phase 8 — documentation and historical evidence reconciliation

## 8.1 Update architecture docs narrowly

Update only docs that currently describe the Plan-067 tiled support boundary.

Likely files:

```text
AGENTS.md
architecture/jpeg-stego-f5.md
architecture/protected-steganography.md
stegoeggo-stego/README.md
```

Required concept:

> Tiled JPEG verification uses an operation-local carrier-owned search context. The JPEG coefficient container is decoded once per tiled search; opaque candidate keys are then evaluated against the retained private carrier state. JPEG headers, coefficient maps, F5 objects, and block sets never cross into the root application crate.

Do not expand documentation unrelated to this runtime correction.

## 8.2 Correct Plan 067 status without erasing history

Add a concise post-closure note to `plans/067-status.md`:

```text
Plan 067 corrected candidate integrity and encoded-output success semantics.
A subsequent runtime audit found exact-candidate extraction re-decoded the JPEG
for every staged candidate request. Plan 068 owns that runtime correction.
```

Do not change Plan 067's correctness rows to FAILED; they were valid for the behavior they measured.

## 8.3 Final Roadmap closure

Only after all Plan 068 checks pass, set Roadmap 057 to:

```text
Status: COMPLETE — final tiled-JPEG runtime residual closed by Plan 068
```

Add a short note referencing `plans/068-status.md`.

### Phase 8 acceptance criteria

- docs describe one-decode operation-local context accurately;
- Plan 067 remains truthful historical correctness evidence;
- Roadmap COMPLETE is not restored before verification.

---

# Phase 9 — focused verification matrix

Run all commands from a clean or explicitly documented working tree.

## 9.1 Carrier tiled-search tests

Run focused tests covering:

```text
candidate identity
wrong-first/later-valid
no byte-value dedup
redundancy identity
max_origins semantics
legacy fallback
post-encode embed verification
decode-count invariance
incompatible-key safety
```

Example command shape:

```bash
cargo test -p stegoeggo-stego tiled_jpeg --features application-support
```

Use actual test filters that match the implementation.

## 9.2 Root tiled extraction tests

Run the root tests covering:

```text
extract_f5_tiled_candidates
verify_extract_f5_tiled
wrong-first/later-valid
wrong MAC
max origins
legacy compatibility
```

## 9.3 Carrier docs / boundary

Run:

```bash
cargo test -p stegoeggo-stego --doc
```

Also rerun any existing compile-fail/default-surface boundary test proving codec internals and `application_support` are not part of the normal root `stego` facade.

## 9.4 Strict static checks

Run the repository's normal strict checks rather than inventing new CI:

```bash
./scripts/check.sh
```

Do not weaken clippy settings to make this pass.

## 9.5 Pre-release structural check

Run the existing staged pre-publication check:

```bash
./scripts/release-check.sh --allow-dirty --skip-check --stage=pre
```

This is verification only. Do not publish anything.

### Phase 9 acceptance criteria

- focused carrier tests pass;
- focused root tests pass;
- decode-count regression passes;
- doctests/boundary tests pass;
- `./scripts/check.sh` passes;
- staged pre-release check passes.

---

# Phase 10 — final source audit and closure

Before marking COMPLETE, inspect actual source rather than relying on test names or status claims.

## 10.1 Required source audit questions

Answer all of these in `plans/068-status.md`:

1. Where is the full JPEG coefficient decode performed for tiled candidate search?
2. Can `prefix_candidates()` trigger another full decode?
3. Can `extract_candidate()` trigger another full decode?
4. Can V3 header extraction trigger another full decode?
5. Can V3 full extraction trigger another full decode?
6. Can legacy V2/V1 fallback trigger another full decode?
7. Does root create more than one search context per tiled search entry call?
8. Does candidate identity still include tile/grid/redundancy semantics?
9. Are parser/coefficient/F5/block types still private to carrier internals?
10. Is the default carrier API unchanged?
11. Did any search bound change?
12. Did any package version/release/CI behavior change?

Expected answers for 1–7:

```text
1. search-context construction only
2. no
3. no
4. no
5. no
6. no
7. no
```

## 10.2 Final status rows

Close rows only with concrete evidence.

Examples:

```text
R05 CLOSED — decode counter remains 1 while N>1 candidates are staged
R12 CLOSED — jpeg_transcoder and coefficient types remain private
R15 CLOSED — wrong-first/no-match/legacy fallback decode-count tests all == 1
R19 CLOSED — ./scripts/check.sh output recorded
```

Do not use `CLOSED` merely because code was written.

## 10.3 Final commit/evidence record

Record:

```text
implementation commit(s)
final HEAD
commands run
actual pass/fail results
any intentionally ignored tests and why
```

If any acceptance criterion remains open, leave Roadmap 057 PARTIAL.

### Phase 10 acceptance criteria

- source audit proves one decode per tiled candidate search;
- all 22 status rows are closed with evidence;
- Roadmap 057 is COMPLETE only if every row is closed;
- no unrelated work was bundled into the closure.

---

# 11. Detailed implementation guidance for smaller models

## 11.1 Minimal preferred change set

The expected implementation should be concentrated in roughly:

```text
stegoeggo-stego/src/application_support.rs
src/protected/steganography.rs
plans/068-status.md
plans/067-status.md
plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md
small relevant architecture docs/tests
```

Avoid broad source churn.

## 11.2 Suggested carrier skeleton

A simple implementation may look like:

```rust
pub struct TiledJpegSearch {
    header: crate::jpeg_transcoder::JpegHeader,
    coefficients: crate::jpeg_transcoder::Coefficients,
    tile_size: u32,
    tiles_x: u32,
    tiles_y: u32,
}

impl TiledJpegSearch {
    pub fn new(jpeg_bytes: &[u8], tile_size: u32) -> Option<Self> {
        let (header, coefficients) =
            crate::jpeg_transcoder::JpegTranscoder::decode_coefficients(jpeg_bytes).ok()?;
        // validate tile size + derive tiles_x / tiles_y once
        Some(Self { ... })
    }

    pub fn prefix_candidates(
        &self,
        master_seed: u64,
        max_origins: u32,
        prefix_bits: usize,
    ) -> Vec<(TiledJpegCandidateKey, Vec<u8>)> {
        // same candidate order and ranges as Plan 067
    }

    pub fn extract_candidate(
        &self,
        master_seed: u64,
        candidate: TiledJpegCandidateKey,
        payload_bits: usize,
    ) -> Option<Vec<u8>> {
        // validate candidate against self.tiles_x/self.tiles_y
        // rebuild exact tile_block_set from self.header/self.coefficients
        // derive exact local seed from candidate
        // extract against self.coefficients
        // NO decode_coefficients call
    }
}
```

This is guidance, not a mandate on exact names.

## 11.3 Avoid cloning the entire coefficient map per candidate

`extract_candidate()` should borrow retained coefficient state.

Do not clone `Coefficients` merely to satisfy ownership unless the F5 extraction API requires mutation. Current extraction is read-only; preserve that.

## 11.4 Do not accidentally hold mutable embed state

This search context is for extraction/verification.

Do not reuse the mutable embedding coefficient container unless doing so clearly reduces code without mixing concerns.

## 11.5 Preserve one-pass tiled embed verification separately

Plan 067's post-encode tiled embedding verification performs a decode of the newly encoded output. That is a different semantic operation and should not be removed merely to satisfy the candidate-search decode-count rule.

The rule is specifically:

```text
one full decode per tiled extraction/verification search
```

not:

```text
one decode for the entire lifetime of every tiled JPEG operation
```

Do not weaken the embed success check.

---

# 12. Acceptance criteria summary

Plan 068 is complete only when all of the following are true:

1. A tiled-JPEG candidate search decodes coefficients once into an operation-local carrier-owned context.
2. Prefix enumeration reuses that context.
3. Exact-key header extraction reuses that context.
4. Exact-key full V3 extraction reuses that context.
5. Exact-key V1/V2 legacy fallback reuses that context.
6. No normal candidate-stage call performs another `decode_coefficients()`.
7. Candidate identity semantics from Plan 067 are unchanged.
8. Wrong-first/later-valid behavior remains correct.
9. Malformed/unsupported/auth/integrity failures continue scanning appropriately.
10. `max_origins` semantics are unchanged.
11. Nearby seed search remains `0..=2` in both axes.
12. Redundancy probing remains `1..=10`.
13. JPEG parser/coefficient/F5/block internals remain private.
14. The normal/default carrier API is unchanged.
15. Root holds only the opaque application-support search context and candidate keys.
16. A test explicitly proves multiple candidate extractions use one full decode.
17. A no-match or wrong-first regression explicitly proves decode count remains one.
18. Legacy fallback explicitly proves decode count remains one.
19. Existing Plan-067 correctness tests pass unchanged or with semantics preserved.
20. `cargo test -p stegoeggo-stego --doc` and carrier boundary checks pass.
21. `./scripts/check.sh` passes.
22. `./scripts/release-check.sh --allow-dirty --skip-check --stage=pre` passes.
23. `plans/068-status.md` contains actual command/evidence results.
24. Plan 067 contains a truthful post-closure runtime-residual note.
25. Roadmap 057 is marked COMPLETE only after all preceding criteria are closed.
26. No version bump, publication, tag, release, CI expansion, new algorithm, or unrelated cleanup lands in this pass.

---

# 13. Handoff notes

This plan is intentionally narrower than Plans 065–067.

The implementation agent should not reinterpret the problem as a reason to merge the root and carrier crates, expose coefficient objects, reduce candidate search coverage, or build a generic caching/session framework.

The required architectural correction is simple:

```text
KEEP:
opaque candidate keys
exact-key staged extraction
bounded search
private carrier internals

CHANGE:
bytes + key -> re-decode every time

TO:
one operation-local opaque search context -> reuse decoded state for all keys
```

Because `plans/` is ignored, remember to force-track `plans/068-status.md` during implementation:

```bash
git add -f plans/068-status.md
```

No release action is part of this plan.
