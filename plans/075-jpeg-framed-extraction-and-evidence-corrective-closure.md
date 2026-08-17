# Plan 075: JPEG Framed Extraction and Evidence Corrective Closure

Status: Ready for implementation

Roadmap: `plans/069-stego-api-ergonomics-and-adapter-simplification-roadmap.md`

Corrective predecessors: Plans 070-074

Audited implementation baseline: `main` at `09eba2a9da9eeb1787665e9a2ce789917a5c335d`

Authoritative implementation ledger to create before product edits: `plans/075-status.md`

---

## 1. Purpose

Plans 070-073 landed the intended architecture and API direction:

- the root steganography application adapter is decomposed by responsibility;
- generic carrier mechanics remain in `stegoeggo-stego`;
- raw, framed, and LSB in-place operations exist;
- framed LSB/JPEG recovery no longer requires caller-retained payload length;
- framed JPEG recovery no longer requires caller-retained `actual_redundancy`;
- corrected LSB embedding/extraction no longer materializes one byte per logical bit;
- fallible LSB/JPEG configuration APIs exist;
- JPEG parser/coefficient/Huffman/F5 internals remain private.

The post-Plan-074 audit found a small set of residuals that do **not** justify reopening or redesigning that architecture, but they do prevent Roadmap 069 from being considered perfectly closed:

1. `plans/073-status.md` is missing even though Plans 069/073/074 claim that a Plan-073 implementation ledger exists and Plan 073 required it before source edits.
2. `jpeg::extract_framed()` composes public `capacity()` and `extract()` calls for every redundancy candidate. Those operations re-parse/probe and fully decode JPEG coefficients, producing repeated coefficient decodes during one bounded framed-recovery operation.
3. `jpeg::extract_framed()` can return `InsufficientCapacity` remembered from an impossible higher-redundancy candidate even after lower-redundancy candidates had sufficient capacity and failed for a more relevant frame/seed/integrity reason.
4. JPEG raw capacity/extraction arithmetic is less defensive than the corrected LSB path: `payload_len * 8` and required-capacity multiplication are not checked, and raw `jpeg::extract()` passes `actual_redundancy` to an internal constructor that silently clamps it to `1..=10` rather than rejecting invalid public input.
5. The Criterion `lsb_clone_vs_in_place` benchmark reuses one already-watermarked mutable image for all in-place iterations, while the cloning side starts from pristine source state every iteration. This biases the in-place measurement and is not valid comparative evidence.
6. Root rustdoc says the standalone carrier crate has its own "version cadence" while the workspace currently exact-pins root to carrier `=0.3.2` and release checks enforce lockstep. The useful claim is that the carrier has its own package/API surface, not an independent release cadence.

This plan closes only those residuals.

It must **not** redesign the carrier algorithms, public framing format, application payload, root module decomposition, or CI/release model.

---

## 2. Required end state

At completion:

```text
one jpeg::extract_framed() call
    |
    +-- validate JPEG/support once
    +-- decode coefficient container once
    +-- compute reusable available capacity once
    |
    +-- candidate redundancy N
    |     +-- extract fixed frame prefix from retained coefficients
    |     +-- validate prefix/bounds
    |     +-- extract full frame from SAME retained coefficients
    |     +-- CRC/frame validation
    |
    +-- candidate redundancy N-1 ... 1
          +-- reuse SAME retained coefficients
```

Public raw APIs remain operation-oriented:

```rust
jpeg::capacity(...)
jpeg::embed(...)
jpeg::extract(...)
jpeg::embed_framed(...)
jpeg::extract_framed(...)
```

Do **not** expose a public `JpegSession`, coefficient container, parser, transcoder, or F5 object merely to fix this internal reuse problem.

The Plan-073 evidence history must also be truthful:

```text
Plan 073 implementation exists and is verified
BUT
its required pre-edit ledger chronology was missed
```

Do not fabricate a historical ledger timestamp or imply that a retrospectively-created status file existed before commit `8fd0153`.

---

## 3. Governing constraints

1. Preserve `#![forbid(unsafe_code)]`.
2. Preserve the corrected LSB V2 carrier mapping and all legacy extraction compatibility.
3. Preserve JPEG F5 coefficient-selection behavior and seed derivation.
4. Preserve the generic frame wire format exactly: magic, version, 11-byte header, 16 MiB bound, CRC32 semantics.
5. Preserve raw public API signatures unless an existing signature cannot be made safe without a breaking change. This plan expects no signature break.
6. Preserve framed public API signatures.
7. Preserve the supported JPEG subset and existing `JpegUnsupportedReason` behavior.
8. Preserve container-preserving JPEG encoding behavior.
9. Keep `jpeg_transcoder`, coefficient types, entropy/Huffman types, and `DctStegoF5` private.
10. Do not expose the parent-only `application-support` layer through the default public facade.
11. Do not add a general carrier/session/codec trait hierarchy.
12. Do not add new dependencies.
13. Do not add new media formats.
14. Do not change StegoEggo rights metadata, payload-v3, authentication, legacy payload semantics, or seed-discovery policy.
15. Do not broaden required CI. `./scripts/check.sh` remains the required repository check.
16. Do not bump versions, publish crates, create tags/releases, or add release automation.
17. Do not optimize by reducing the valid JPEG framed redundancy search domain.
18. Do not silently clamp invalid public `actual_redundancy`; reject it through the existing operation-level error model.
19. Performance evidence must compare equivalent work and equivalent starting carrier state.

---

# Phase 0 — truthful status and roadmap state

## 0.1 Create `plans/075-status.md` before product edits

The first implementation change must create and track `plans/075-status.md`.

Record:

- starting HEAD;
- working-tree status;
- workspace versions;
- root -> carrier dependency declaration;
- current Roadmap 069 header;
- presence of `070-status.md`, `071-status.md`, `072-status.md`, `074-status.md`;
- **absence of `073-status.md` at the starting baseline**;
- current CI state for baseline `09eba2a` if available.

Start all rows `OPEN`:

```text
R01 Roadmap 069 reopened truthfully while corrective work is active
R02 missing Plan-073 ledger reconciled without fabricated chronology
R03 Plan-073 implementation evidence independently reconstructed
R04 framed JPEG full coefficient decode count <= 1 per operation
R05 framed JPEG redundancy search domain/order preserved
R06 framed JPEG prefix/full extraction reuse one retained decoded state
R07 capacity-only candidates cannot mask viable frame/extraction failures
R08 full-frame/CRC candidate failure has deterministic precedence over prefix-only noise
R09 JPEG payload-length arithmetic is overflow-safe
R10 raw JPEG actual_redundancy is explicitly validated
R11 valid raw JPEG behavior remains compatible
R12 framed JPEG downgrade recovery remains compatible
R13 wrong-seed/malformed-frame errors remain bounded and non-panicking
R14 JPEG internals remain private
R15 no public session/framework API added
R16 LSB benchmark starts each in-place iteration from equivalent pristine carrier state
R17 benchmark setup does not charge the in-place API for a preparation clone
R18 carrier cadence wording matches actual lockstep packaging policy
R19 focused carrier/public API tests pass
R20 full workspace check passes
R21 no dependency/version/release/CI expansion
R22 Roadmap 069 final closure wording is evidence-consistent
```

## 0.2 Reopen Roadmap 069 before product-source edits

Change Roadmap 069 header from `COMPLETE` to something equivalent to:

```text
Status: PARTIAL — post-closure residuals tracked by Plan 075
```

Add a concise correction note stating:

- Plans 070-073 implementation remains valid;
- Plan 074 correctly verified the broad architectural/API outcome;
- a later audit found a missing Plan-073 ledger, framed-JPEG repeated decode/error-selection issues, JPEG numeric validation gaps, and biased benchmark evidence;
- Plan 075 owns those residuals only.

Do not rewrite Plans 070-074 as failed work.

### Phase 0 acceptance criteria

- Plan 075 ledger is tracked before product edits;
- Roadmap 069 is not `COMPLETE` while corrective work is active;
- the missing 073 ledger is recorded as a historical process defect, not hidden.

---

# Phase 1 — reconstruct the missing Plan-073 status ledger truthfully

Create `plans/073-status.md` if it is still absent.

This is a **retrospective evidence ledger**. It cannot satisfy the original chronology requirement retroactively.

Required header language should make that explicit, for example:

```text
Status: COMPLETE — implementation verified retrospectively;
pre-edit ledger chronology requirement was missed and is documented by Plan 075.
```

Record:

```text
implementation commit: 8fd01530443ee73d77377ca8a8018a59e04d5b32
predecessor: 9125e0f
later independent closure audit: 09eba2a / plans/074-status.md
chronology defect: plans/073-status.md was absent at Plan-075 baseline
```

Reconstruct the original Plan-073 rows R01-R14 against current source rather than merely copying Plan 074 prose.

At minimum verify:

- `LsbConfig::try_new`;
- `LsbConfig::try_with_redundancy`;
- `JpegConfig::try_new`;
- `JpegConfig::try_with_redundancy`;
- shared `validate_redundancy`;
- capacity-unit rustdoc;
- CRC-not-authentication documentation;
- compile-fail/private-boundary doctests;
- root re-export shape;
- dependency list unchanged;
- public tests for malformed/untrusted config;
- no version/release/CI change in Plan 073.

Do not claim the ledger was created before `8fd0153`.

### Phase 1 acceptance criteria

- `plans/073-status.md` exists;
- its evidence is source-backed;
- its chronology statement is explicitly retrospective;
- Plan 074 may reference it only after this correction, with a note that the original closure audit relied on equivalent source evidence rather than a then-existing ledger.

---

# Phase 2 — introduce a private reusable decoded-JPEG operation context

## 2.1 Problem to remove

Current framed extraction approximately does:

```text
for redundancy in requested..=1:
    capacity(jpeg, FRAME_HEADER_SIZE, candidate_config)
        -> probe/parse
        -> decode coefficients

    extract(jpeg, FRAME_HEADER_SIZE, candidate_config, redundancy)
        -> probe/parse
        -> decode coefficients

    if prefix parses:
        capacity(jpeg, total_len, candidate_config)
            -> probe/parse
            -> decode coefficients

        extract(jpeg, total_len, candidate_config, redundancy)
            -> probe/parse
            -> decode coefficients
```

This is bounded, but unnecessarily expensive. A 10-redundancy no-match search can repeatedly decode the same JPEG dozens of times.

## 2.2 Preferred private design

Inside `stegoeggo-stego/src/jpeg.rs`, add a private operation context or equivalent private helper shape.

Example only:

```rust
struct DecodedJpegCarrier {
    coefficients: crate::jpeg_transcoder::Coefficients,
    available_capacity: usize,
}
```

If the retained header is useful, include it privately. Exact naming is not important.

Preferred private helpers:

```rust
fn decode_supported_carrier(jpeg_bytes: &[u8]) -> Result<DecodedJpegCarrier, StegoError>;

fn capacity_from_decoded(
    decoded: &DecodedJpegCarrier,
    payload_len: usize,
    redundancy: usize,
) -> Result<CapacityReport, StegoError>;

fn extract_from_decoded(
    decoded: &DecodedJpegCarrier,
    payload_len: usize,
    seed: u64,
    redundancy: usize,
) -> Result<Vec<u8>, StegoError>;
```

Equivalent decomposition is acceptable.

The key contract is:

```text
one framed extraction operation -> at most one JpegTranscoder::decode_coefficients()
```

Header parsing/support probing may occur separately if needed to fail unsupported structures cleanly. The acceptance target concerns **full coefficient decode**, not trivial local arithmetic.

## 2.3 Do not expose the context

The decoded carrier helper/context must remain private to the carrier crate/module.

Do not add public:

```text
JpegSession
JpegCarrierHandle
DecodedCoefficients
JpegHeader
DctStegoF5
```

Do not route it through root `stegoeggo::stego`.

## 2.4 Reuse helpers where mechanically beneficial

Public `capacity()` and public raw `extract()` may delegate through the same private checked helpers if this reduces duplicated arithmetic/validation.

Do not force them into a long-lived session model. Independent raw calls are allowed to decode independently because they are independent operations.

### Phase 2 acceptance criteria

- private decoded state exists or equivalent retained-state helper is implemented;
- no stable API expansion occurs;
- framed extraction does not call public `capacity()`/`extract()` in a way that re-decodes per candidate;
- public raw operations remain behavior-compatible.

---

# Phase 3 — make framed JPEG extraction single-decode

Refactor `jpeg::extract_framed()` to:

```text
1. validate encoded JPEG + supported structure
2. decode coefficient container once
3. compute reusable available coefficient capacity once
4. iterate redundancy from config.redundancy() down to 1
5. for each candidate:
   a. capacity-check frame prefix using arithmetic against retained available capacity
   b. extract prefix from retained coefficients
   c. decode/validate prefix
   d. capacity-check exact total frame length using retained available capacity
   e. extract exact full frame from SAME retained coefficients and SAME redundancy
   f. frame::decode / CRC
6. return first complete valid frame
```

Preserve exact candidate order:

```rust
(config.redundancy()..=1) // conceptual descending order
```

Actual idiomatic Rust remains:

```rust
(1..=config.redundancy()).rev()
```

Do not:

- use Q-table seed hints to infer redundancy;
- use application metadata;
- increase search beyond configured redundancy down to 1;
- accept a prefix as success without a complete frame+CRC validation;
- reduce validation to improve speed.

## 3.1 Decode-count evidence

Add a focused test-only decode counter at the narrowest sensible boundary.

Preferred approaches:

- `cfg(test)` counter around the carrier helper's call to `JpegTranscoder::decode_coefficients`, or
- a test-only injected helper that proves one context construction / one full decode.

Avoid global production telemetry.

Required tests:

```text
configured redundancy = 1 -> one full decode
configured redundancy = 3 with first candidate valid -> one full decode
configured redundancy = 3 with embed auto-downgraded -> one full decode
configured redundancy = 10 wrong seed / no valid frame -> one full decode
```

The no-match case is important because it exercises maximum candidate breadth.

### Phase 3 acceptance criteria

- full coefficient decode count <= 1 for one framed extraction operation;
- candidate count/search breadth is unchanged;
- downgrade recovery remains successful;
- wrong seed remains bounded.

---

# Phase 4 — correct framed-JPEG error precedence

## 4.1 Current defect

Current code retains an `insufficient_capacity` result whenever a higher-redundancy candidate cannot fit the prefix/full frame. At the end of the search it may return that capacity error even if lower-redundancy candidates had sufficient capacity and actually failed due to wrong seed, malformed frame, or CRC failure.

A capacity failure from a candidate that could never contain the frame must not mask failure evidence from a candidate that was fully viable.

## 4.2 Required precedence model

Track candidate progress explicitly.

Recommended conceptual states:

```text
CapacityRejected
PrefixAttempted(error)
PrefixValidThenFullAttempted(error)
Success(payload)
```

At final failure, use deterministic precedence:

1. a full-frame failure from a candidate whose prefix parsed and whose full length fit capacity;
2. otherwise a failure from a capacity-viable prefix extraction/parse attempt;
3. otherwise `InsufficientCapacity` if **all** candidates were rejected by capacity before meaningful extraction;
4. otherwise `FrameNotFound` only when no more specific operation error exists.

Do not let a later low-information bad-magic prefix overwrite an earlier full-frame CRC failure.

A small private enum is acceptable if it makes precedence explicit. Do not add new public error variants solely for this.

## 4.3 Tests

Add focused tests for:

- high redundancy insufficient, lower redundancy viable but wrong seed -> result is a frame/extraction failure, not `InsufficientCapacity`;
- one candidate reaches full frame and fails CRC, later candidates fail prefix -> CRC/full-frame error remains the stronger result;
- every redundancy candidate truly lacks capacity -> `InsufficientCapacity`;
- valid later redundancy after wrong earlier candidate -> success still wins.

Where crafting exact JPEG corruption is awkward, add a private/helper-level deterministic test around candidate-failure selection in addition to public integration coverage.

### Phase 4 acceptance criteria

- impossible high-redundancy candidates cannot mask viable candidate failures;
- error precedence is documented in local code comments/tests;
- first valid complete frame still terminates successfully.

---

# Phase 5 — harden JPEG length arithmetic and public redundancy validation

## 5.1 Checked payload-bit conversion

Replace unchecked public-path arithmetic such as:

```rust
let payload_bits = payload_len * 8;
let required = payload_bits * redundancy;
```

with checked helpers.

Preferred private helper:

```rust
fn checked_payload_bits(payload_len: usize) -> Result<usize, StegoError>;
fn checked_required_capacity(payload_len: usize, redundancy: usize) -> Result<usize, StegoError>;
```

Use existing `StegoError::InvalidConfig(...)` or another already-existing operation-level error when caller-supplied lengths cannot be represented safely. Do not add a new error variant unless clearly necessary.

Apply consistently to at least:

- `jpeg::capacity()`;
- `jpeg::embed()` where relevant;
- raw `jpeg::extract()`;
- private framed helper arithmetic.

Frame-declared lengths remain bounded by `MAX_FRAME_PAYLOAD`, but still route through the checked helpers so one arithmetic contract exists.

## 5.2 Validate raw `actual_redundancy`

At the public raw extraction boundary:

```rust
pub fn extract(..., actual_redundancy: usize) -> Result<Vec<u8>, StegoError>
```

call the shared carrier redundancy validator before invoking private F5 mechanics.

Required behavior:

```text
0 -> Err(StegoError::InvalidConfig(...))
1 -> accepted
10 -> accepted
11 -> Err(StegoError::InvalidConfig(...))
usize::MAX -> Err(StegoError::InvalidConfig(...))
```

Do not rely on `DctStegoF5::with_redundancy(...).clamp(1, 10)` for public-input validation.

The private constructor may retain defensive clamping if it is useful internally, but no public generic carrier call should silently reinterpret invalid redundancy.

## 5.3 Tests

Add public tests that use `catch_unwind` or ordinary `Result` assertions to prove:

- huge payload lengths passed to `capacity()` do not panic/wrap;
- huge payload lengths passed to raw extraction do not panic/wrap;
- invalid raw `actual_redundancy` returns `InvalidConfig`;
- valid 1 and 10 behavior remains accepted.

Do not attempt to allocate a `usize::MAX` payload. The APIs already take an explicit length for capacity/extraction; use that parameter directly.

### Phase 5 acceptance criteria

- no unchecked payload-bit/required-capacity multiplication remains on public JPEG paths;
- public raw extraction does not silently clamp redundancy;
- valid behavior remains compatible.

---

# Phase 6 — repair the LSB clone-vs-in-place benchmark

## 6.1 Current benchmark defect

Current benchmark shape:

```rust
let mut in_place = source.clone();
b.iter(|| {
    lsb::embed_in_place(&mut in_place, ...)
})
```

After the first iteration, `in_place` already contains the same payload at the same carrier locations. Later iterations therefore perform different mutation work from the clone API, whose internal clone always begins from the pristine immutable source.

Do not use numbers from this benchmark as comparative evidence.

## 6.2 Required Criterion shape

Use `iter_batched` or equivalent so each timed in-place operation starts from an equivalent pristine mutable image while setup is outside the timed operation.

Preferred pattern:

```rust
use criterion::BatchSize;

b.iter_batched(
    || source.clone(),
    |mut image| {
        black_box(lsb::embed_in_place(
            black_box(&mut image),
            black_box(&payload),
            black_box(&config),
        ))
    },
    BatchSize::LargeInput,
);
```

The setup clone is deliberately outside the timed closure because the in-place API contract assumes the caller already owns the mutable buffer. The cloning `lsb::embed()` benchmark should continue to include its internal clone because that is part of that API's cost.

If memory pressure makes `LargeInput` unsuitable at 4096x4096, choose a bounded Criterion batch size that still guarantees a fresh source image per measured iteration. Document the choice.

## 6.3 Benchmark correctness check

Before accepting performance numbers, add a debug/test assertion or separate ordinary test showing both benchmarked paths produce pixel-identical output from the same pristine source for the fixed seed/payload/config.

The existing public equivalence test may satisfy this if it exactly covers the same algorithmic contract.

### Phase 6 acceptance criteria

- every measured in-place iteration starts from pristine/equivalent source state;
- setup clone is outside timed in-place operation;
- clone API benchmark still measures its intentional internal clone;
- no performance threshold is added to CI.

---

# Phase 7 — correct package/version-cadence wording

Audit root `src/lib.rs`, README, carrier README, and architecture docs for statements implying that `stegoeggo-stego` currently has an independent release cadence.

Current workspace policy is exact version lockstep:

```toml
stegoeggo-stego = { path = "stegoeggo-stego", version = "=0.3.2", ... }
```

and the staged release check verifies version lockstep.

Preferred wording:

```text
For standalone generic use, prefer the dedicated stegoeggo-stego crate directly;
it has its own package and public API surface.
```

Do not say "own version cadence" unless release policy is actually changed in a separate approved line of work.

This plan does **not** authorize changing lockstep versioning.

### Phase 7 acceptance criteria

- public docs describe the standalone package/API boundary accurately;
- no release-policy change is made.

---

# Phase 8 — focused verification

Minimum focused commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p stegoeggo-stego --all-features
cargo test -p stegoeggo-stego --doc
cargo test -p stegoeggo --test public_stego_api --all-features
cargo test -p stegoeggo --all-features jpeg
cargo test -p stegoeggo --all-features tiled_jpeg
cargo test --workspace --exclude stegoeggo-fuzz --all-features
./scripts/check.sh
```

Run the repaired benchmark manually for evidence, not as a gate:

```bash
cargo bench --bench bench lsb_clone_vs_in_place
```

If the benchmark harness supports filtering differently, use the actual local command and record it truthfully.

Also run the existing structural package check used by this repository if still current:

```bash
./scripts/release-check.sh --allow-dirty --skip-check --stage=pre
```

This remains a structural pre-release check only; do not publish.

---

# Phase 9 — final closure reconciliation

Only after all product/tests/evidence above are complete:

1. update `plans/075-status.md` with exact implementation commit(s), tests, decode-count evidence, and benchmark correction;
2. add a concise correction note to `plans/074-status.md` stating that a later audit found the Plan-073 ledger chronology defect and framed-JPEG/runtime/evidence residuals, all owned by Plan 075;
3. preserve Plan 074's historical evidence rather than rewriting it as if the broad architecture audit failed;
4. update Roadmap 069 back to `COMPLETE` only if every Plan-075 row is closed;
5. make the Roadmap 069 closure note reference Plan 075 as the final corrective pass;
6. state truthfully that `plans/073-status.md` was reconstructed retrospectively under Plan 075 rather than existing before Plan-073 source edits.

If any code-level residual remains, Roadmap 069 stays `PARTIAL`.

---

## 10. Explicit acceptance criteria

Plan 075 is COMPLETE only when all are true:

1. `plans/075-status.md` was tracked before Plan-075 product edits.
2. Roadmap 069 was marked `PARTIAL` while Plan 075 was active.
3. `plans/073-status.md` exists and explicitly records that its chronology is retrospective.
4. Plan-073 implementation commit/evidence is independently verified rather than invented from the missing ledger.
5. One `jpeg::extract_framed()` call performs at most one full JPEG coefficient decode regardless of configured redundancy or candidate success position.
6. Framed JPEG search still checks configured redundancy down to 1 in deterministic order.
7. Prefix, full-frame extraction, and CRC validation for each candidate use the same retained decoded coefficient state.
8. No public JPEG parser/coefficient/session/F5 type is introduced.
9. A capacity failure from an impossible candidate cannot mask a frame/extraction failure from a capacity-viable candidate.
10. A candidate that reached full-frame validation has deterministic error precedence over later prefix-only noise when no candidate succeeds.
11. If all redundancy candidates are rejected solely by capacity, framed extraction still returns `InsufficientCapacity`.
12. Framed JPEG auto-downgrade recovery still succeeds without caller-retained `actual_redundancy`.
13. Wrong-seed framed JPEG recovery remains bounded and non-panicking.
14. Public JPEG payload-length/required-capacity arithmetic is checked for overflow.
15. Raw `jpeg::extract()` rejects `actual_redundancy` outside `1..=10` with `StegoError::InvalidConfig` rather than silently clamping.
16. Valid raw JPEG redundancy 1 and 10 remain accepted.
17. Existing supported/unsupported/progressive/container-preservation JPEG tests remain passing.
18. Corrected LSB carrier known-answer output remains unchanged.
19. The `lsb_clone_vs_in_place` benchmark uses a fresh equivalent source carrier for every measured in-place iteration.
20. The preparation clone for the in-place benchmark occurs outside the timed closure.
21. No benchmark threshold or benchmark job is added to CI.
22. Documentation no longer claims an independent carrier version cadence while lockstep versioning is enforced.
23. Carrier direct dependencies do not grow.
24. No version bump, publish, tag, release, or CI expansion occurs.
25. `./scripts/check.sh` passes.
26. Current GitHub CI passes after the implementation commit if CI is triggered.
27. Roadmap 069 is restored to `COMPLETE` only after all above evidence is recorded.

---

## 11. Explicit non-goals

Do not use Plan 075 to:

- redesign LSB permutation or redundancy;
- redesign F5 or seed hints;
- change generic frame magic/version/layout;
- add authenticated framing;
- add encryption;
- add a public JPEG session/context API;
- expose coefficient maps or JPEG parser internals;
- redesign `application-support`;
- change tiled JPEG search architecture;
- change root application module decomposition;
- add raw RGB/stride APIs;
- add a general carrier trait;
- add formats/codecs;
- change metadata or rights semantics;
- redesign CI/release workflows;
- independently version the carrier crate;
- chase unrelated micro-optimizations.

If an unrelated issue is found, record it separately rather than expanding this corrective pass.

---

## 12. Smaller-model execution notes

The critical distinction for this pass is **public operation API vs private reusable implementation state**.

Good:

```text
public jpeg::extract_framed(bytes, config)
    -> private decode_supported_carrier(bytes)
    -> private retained coefficients
    -> private repeated candidate extraction
```

Do not implement:

```text
public JpegSession::open(bytes)
public Coefficients
public JpegHeader
```

For error precedence, do not merely reverse one `if`. Track whether any candidate was actually viable and how far it progressed.

For arithmetic hardening, do not allocate huge buffers to test overflow. Exercise explicit `payload_len` parameters with `usize::MAX`/large values and assert structured errors.

For the benchmark, do not clone the pristine source inside the measured in-place closure. The purpose of the in-place API is precisely to measure operation cost when the caller already owns a mutable image.

For the missing 073 ledger, never write language such as "created before implementation". The correct historical statement is that Plan 073 implementation was completed in `8fd0153`, Plan 074 later verified it from source, and Plan 075 discovered that the required standalone status ledger itself had never been committed.

---

## 13. Final handoff condition

A final reviewer should be able to verify all of the following without trusting commit messages:

```text
Plan-073 evidence history is truthful
JPEG framed extraction decodes once per operation
redundancy search remains complete and bounded
error reporting reflects the most meaningful viable candidate failure
public numeric inputs fail safely
LSB in-place benchmark compares equivalent starting work
carrier remains a small operation-level standalone library
```

If any of these requires accepting a known contradiction in the status files, Roadmap 069 is not closed.
