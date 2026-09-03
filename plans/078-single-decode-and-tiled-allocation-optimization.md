# Plan 078: Single-Decode and Tiled Allocation Optimization

Status: Ready for implementation

Roadmap: `plans/076-stego-pipeline-and-carrier-library-closure-roadmap.md`

Depends on: Plan 077 complete.

Audited planning baseline: `main` at `6feb52a90d9afdc0c922cdb219529524ad94c168`

Authoritative implementation ledger to create before product edits: `plans/078-status.md`

---

## 1. Purpose

After Plan 077 makes carrier selection correct, remove the remaining avoidable decode and full-image allocation costs in the stego pipeline without changing wire behavior or verification coverage.

The current source has four high-value residuals:

1. **Raster preflight can fully decode an image that execution immediately decodes again.** `process_plan_bytes()` uses `ImageReader::into_dimensions()` and then also attempts `load_image_from_bytes()` before the actual executor performs its own decode.
2. **Tiled LSB still clones the complete `RgbaImage`.** The tiled implementation already writes directly into carrier coordinates and allocates no temporary tile images, but begins from `let mut output = img.clone()` even when the parent owns a mutable pixel buffer.
3. **Tiled JPEG embedding decodes twice.** It decodes coefficients, mutates/encodes them, and then decodes the encoded JPEG again solely to self-check one embedded tile.
4. **Application JPEG verification repeatedly entropy-decodes the same image.** `dct_candidates()` calls encoded-byte extraction for every redundancy candidate; prefix and full-payload probes repeat that loop. Generic `jpeg::extract_framed()` already solved the analogous problem by retaining one private decoded coefficient representation, but root application verification has not adopted the same reuse principle.

This plan addresses those costs with private/shared implementation state first. It deliberately does not decide the final stable public reuse API; Plan 079 owns that evidence-gated decision.

---

## 2. Required end state

Normal execution should meet these representation-level targets:

```text
same-format JPEG BestEffort/Tiled embed
    pixel decodes:       0
    coefficient decodes: 1

PNG/WebP -> PNG/WebP marker
    pixel decodes:       1

JPEG -> PNG/WebP marker
    pixel decodes:       1
    coefficient decodes: 0

PNG/WebP -> JPEG marker
    pixel decodes:       1
    coefficient decodes: 1 after JPEG encode

metadata-only same-format container rewrite
    pixel decodes:       0 where current container injector supports it

application JPEG verification
    coefficient decodes: 1 per verification operation/search context,
                         including standard candidate probing and tiled fallback
                         when both can share the same decoded carrier
```

Bounded header parsing/dimension inspection does not count as a pixel or coefficient decode.

Tiled LSB should have this ownership model:

```text
public/current cloning convenience
    clone once
    -> shared in-place tiled core

parent application already owns RGBA
    -> shared in-place tiled core directly
```

---

## 3. Governing constraints

1. Preserve all carrier wire output for unchanged `(carrier, payload, seed, redundancy, tile_size)` inputs where the algorithm itself is unchanged.
2. Preserve corrected LSB V2 permutation/mutation rules and legacy extraction compatibility.
3. Preserve JPEG F5 coefficient selection, seed derivation, tile seed derivation, and supported syntax subset.
4. Preserve verification search domains, candidate ordering, malformed-input handling, and resource limits.
5. Preserve container-preserving JPEG encode behavior.
6. Preserve public one-shot carrier APIs.
7. Keep decoded JPEG coefficient/header types private.
8. Any reusable JPEG context introduced in this plan is private or hidden application-support implementation state only. Plan 079 separately decides whether a public opaque form is justified.
9. Do not replace safe bounded validation with unchecked fast paths merely for benchmark gains.
10. Insufficient-capacity in-place operations must leave the caller's carrier unchanged.
11. No new dependency is expected.
12. No version bump, publication, tag, release, or CI expansion.
13. Performance claims require decode-count, allocation-count, or Criterion evidence.

---

# Phase 0 — create the ledger and baseline measurements

## 0.1 Create `plans/078-status.md` before product edits

Record:

```text
starting HEAD
working tree status
workspace versions
Plan 077 disposition/commit
current carrier application-support feature wiring
```

Start every row `OPEN`:

```text
R01 non-JPEG preflight does not perform a discarded full pixel decode
R02 raster marker path performs one pixel decode
R03 same-format JPEG marker path performs zero pixel decodes
R04 same-format JPEG marker path performs one coefficient decode
R05 tiled LSB in-place core exists
R06 tiled LSB insufficient-capacity path leaves mutable input unchanged
R07 tiled cloning path delegates to in-place core
R08 parent tiled raster path avoids second full-image buffer
R09 tiled JPEG embed performs one coefficient decode
R10 tiled JPEG production self-check does not re-decode encoded output
R11 standard JPEG application verification performs one coefficient decode
R12 standard+tiled JPEG verification can reuse one decoded carrier when both are attempted
R13 JPEG candidate search coverage/order remains unchanged
R14 malformed/wrong-seed/error precedence remains compatible
R15 deterministic LSB/JPEG known-answer behavior remains passing
R16 benchmark/decode/allocation evidence recorded
R17 public JPEG implementation structs remain private
R18 focused carrier/root tests pass
R19 full check.sh passes
R20 no API/dependency/release/CI scope expansion
```

## 0.2 Establish instrumentation before optimizing

Use test-only counters rather than production telemetry.

Existing JPEG code already uses test-only thread-local decode counters for framed/tiled search. Extend or consolidate that pattern so tests can assert coefficient decode counts for:

```text
normal JPEG embed
normal application JPEG verification
normal verification followed by tiled fallback
JPEG tiled embed
```

For raster decode counts, prefer a narrow test seam around the root decode helper rather than modifying `image` or adding global production state. If direct decode counting would create invasive plumbing, prove removal by source structure plus focused benchmark/allocation evidence and record that limitation in the ledger.

### Phase 0 acceptance criteria

- current repeated JPEG verification decode count is measured rather than assumed;
- tiled JPEG embed baseline decode count is measured;
- tiled LSB clone/allocation baseline is recorded for at least one representative image size;
- all counters are test-only.

---

# Phase 1 — remove the discarded raster preflight decode

## 1.1 Keep a cheap dimension/resource gate

For non-JPEG inputs, retain `ImageReader::with_guessed_format()` plus `into_dimensions()` or an equivalent metadata-only dimension query before expensive decode.

Use it to enforce:

```text
ResourceLimits::check_dimensions
ProcessingOptions::max_dimension
```

Do not eagerly call `load_image_from_bytes()` merely to confirm what execution will confirm immediately afterward.

## 1.2 Let the consuming executor own the full decode

When processing requires pixels because of:

```text
format conversion
raster LSB marker
raster seed marker
```

perform the full decode once in that executor and use the result through the remainder of the operation.

For same-format metadata-only container injection, preserve the zero-pixel-decode path.

If dimension inspection fails because the image is malformed or unsupported, choose one deterministic error path. Do not perform a speculative full decode solely as a second validation mechanism unless an existing compatibility test proves it is necessary.

## 1.3 Avoid duplicate dimension checks after the successful header gate

The executor may still defensively validate dimensions when a decode is necessary, but do not repeat expensive parsing merely to restate already-checked dimensions. Keep code simple and fail closed.

### Phase 1 acceptance criteria

- no successful normal raster operation fully decodes the same input twice;
- same-format metadata-only remains container-oriented;
- malformed/truncated input handling remains bounded and tested;
- resource-limit tests remain passing.

---

# Phase 2 — introduce one private decoded-JPEG search context

## 2.1 Consolidate retained decoded state inside the carrier crate

The generic `jpeg.rs` already has a private `DecodedJpegCarrier` used by `extract_framed()`. Generalize that private implementation state only as much as required so current operations can share:

```text
parsed/supported JPEG state
quantized coefficients
available DCT capacity
original bytes/header data needed for encode
```

Do not expose the representation outside `stegoeggo-stego`.

Preferred implementation direction:

```text
private DecodedJpegCarrier
    +-- capacity_from_decoded(...)
    +-- extract_from_decoded(...)
    +-- tiled block/candidate helpers
    +-- encode/mutation helpers where appropriate
```

Avoid creating two independent private decoded-carrier representations for standard and tiled search if one can safely serve both.

## 2.2 Add a narrow hidden application verification/search operation

The root application needs to inspect arbitrary application payload prefixes, headers, full payloads, legacy payload sizes, multiple redundancies, and tiled candidates while retaining private JPEG state.

Do not solve this by exposing coefficient structs.

A permitted hidden shape is an opaque operation-level support object or callback API under `application-support`, conceptually:

```rust
pub struct JpegSearchContext { /* private fields */ }

impl JpegSearchContext {
    pub fn new(bytes: &[u8]) -> StegoResult<Self>;
    pub fn extract(&self, payload_len: usize, seed: u64, redundancy: usize)
        -> StegoResult<Vec<u8>>;
    // tiled candidate operations as needed
}
```

The exact name/signature may differ, but all fields must remain private and no JPEG implementation types may appear in the signature.

Because this feature is parent-only, stability guarantees may remain explicitly narrower than the stable carrier facade.

## 2.3 Migrate `dct_candidates()` / `dct_outcome_with_seed()` to one context

Current application verification must stop calling a one-shot encoded-JPEG extraction function once per redundancy candidate.

Preferred flow:

```text
create search context once
  -> prefix candidates across redundancy 1..=10
  -> classify application prefix
  -> full candidates using SAME context
  -> tiled fallback using SAME context when possible
```

Preserve the current candidate redundancy order unless existing tests/documentation establish a different intentional order.

## 2.4 Share standard and tiled verification decode where possible

`verify_extract_dct_with_seed()` currently performs standard DCT search and then constructs a tiled search separately. If both use the same encoded JPEG, seed, and operation, they should reuse one decoded coefficient state.

If sharing cannot be done without materially complicating the hidden interface, record that constraint and require at minimum one decode for the standard search plus one for tiled fallback. Do not pretend one decode was achieved if the implementation remains two.

The preferred target remains one total coefficient decode for the combined application JPEG verification operation.

### Phase 2 acceptance criteria

- standard application JPEG candidate probing decodes once;
- no coefficient/header implementation type crosses the carrier crate boundary;
- standard and tiled search share retained state unless a documented concrete blocker remains;
- search coverage and application verification semantics are unchanged.

---

# Phase 3 — remove tiled JPEG embed re-decode

## 3.1 Verify against the already-mutated coefficients

`application_support::jpeg_embed_tiled()` currently:

```text
decode coefficients
embed tiles
encode JPEG
re-decode encoded JPEG
extract first embedded tile
compare payload
```

Replace the production post-encode re-decode with an in-memory verification using the already-mutated coefficient set and the same selected tile identity before serialization.

Conceptually:

```text
mutate coefficients
  -> identify first successful tile
  -> extract from SAME coefficients for invariant check
  -> encode once
```

If the in-memory invariant fails, preserve the existing best-effort skipped/error semantics rather than emitting a false successful report.

## 3.2 Keep true encoded roundtrip evidence in tests

The production optimization must not eliminate proof that encode/decode roundtrips preserve the embedded payload.

Add/retain tests that:

```text
embed tiled -> encoded output -> fresh public/hidden extraction -> payload match
```

This moves redundant validation out of every production call and into deterministic verification evidence.

### Phase 3 acceptance criteria

- normal tiled JPEG embed decodes coefficients once;
- normal tiled JPEG embed encodes once;
- encoded-output roundtrip remains covered by tests;
- no reliability contract is weakened.

---

# Phase 4 — add an in-place tiled LSB core

## 4.1 Add carrier-internal in-place tiled mutation

Introduce a shared tiled mutation core accepting `&mut RgbaImage`.

It must:

1. validate tile size and image geometry;
2. compute total/usable capacity before the first mutation when the API contract requires unchanged-on-insufficient-capacity behavior;
3. embed directly into tile regions of the original image;
4. use the existing corrected V2 permutation and tile-seed derivation exactly;
5. return a structured report compatible with existing summary/report types.

Do not allocate/crop each tile.

## 4.2 Make cloning tiled embed a convenience wrapper

Existing cloning behavior should become conceptually:

```rust
let mut output = img.clone();
let report = embed_tiled_in_place(&mut output, payload, ...)?;
return report_with_output(output, report);
```

so there is one algorithm implementation.

## 4.3 Make the parent use in-place tiled mutation

After Plan 077, raster current execution already owns an `RgbaImage`. Route tiled LSB directly into the in-place core instead of calling a cloning tiled operation.

The seed fallback can then mutate the same buffer when embedding succeeded.

## 4.4 Preserve exact deterministic output

Add a regression asserting cloned and in-place tiled operations produce identical pixels for the same pristine input/configuration.

If existing known-answer fixtures cover tiled output, retain them unchanged.

### Phase 4 acceptance criteria

- parent tiled raster protection allocates one owned RGBA carrier, not a second same-sized output buffer;
- cloning and in-place tiled results are byte/pixel identical;
- insufficient-capacity mutable carrier is unchanged;
- tile geometry and edge-tile behavior remain compatible.

---

# Phase 5 — benchmark and decide whether public prepared JPEG state is justified

This plan does not publish a prepared JPEG object. It produces the evidence Plan 079 will use.

## 5.1 Required local evidence

Add/extend Criterion groups for representative operations:

```text
JPEG standard verification: before/after retained decode
JPEG standard + tiled fallback verification: before/after
JPEG tiled embed: before/after second-decode removal
LSB tiled cloning vs in-place: at least 1024x1024 and one larger image
cross-format raster operation if the preflight decode removal has measurable impact
```

Use equivalent pristine input state on every iteration. Keep setup allocations outside the timed closure where appropriate.

## 5.2 Record decode/allocation counts, not only wall-clock time

Wall-clock measurements can be noisy. The ledger must also record deterministic evidence such as:

```text
coefficient decode count per operation
full-image clone count / allocated bytes where measurable
```

## 5.3 Prepared-public-API decision input

The ledger must conclude one of:

```text
PUBLIC-REUSE-CANDIDATE
PRIVATE-REUSE-SUFFICIENT
```

Use `PUBLIC-REUSE-CANDIDATE` only if a plausible external workflow performs multiple capacity/extraction operations on one JPEG and the retained state materially reduces work beyond what one-shot framed APIs already solve.

The decision is input to Plan 079, not authorization to expose internals here.

### Phase 5 acceptance criteria

- measurements compare equivalent work;
- decode/allocation counts substantiate the claimed optimization;
- no CI benchmark threshold is introduced;
- the prepared-public-API disposition is explicit.

---

## 6. Focused verification

Expected commands:

```bash
cargo test -p stegoeggo-stego --all-features
cargo test -p stegoeggo-stego --doc
cargo test -p stegoeggo --test public_stego_api --all-features
cargo test -p stegoeggo --all-features jpeg
cargo test -p stegoeggo --all-features tiled
cargo test -p stegoeggo --all-features verification
cargo test --workspace --exclude stegoeggo-fuzz --all-features
cargo bench --bench bench -- <focused filters>
./scripts/check.sh
```

Record exact benchmark filters/results in `078-status.md`.

---

## 7. Final acceptance criteria

Plan 078 is complete only when:

1. The status ledger existed before product edits.
2. Raster preflight does not perform a discarded full decode.
3. Normal raster marker processing fully decodes the source once.
4. Same-format JPEG current marker processing performs zero pixel decodes.
5. Tiled LSB has one shared in-place algorithm core.
6. Parent raster tiled processing avoids an extra full-image clone.
7. Tiled JPEG embedding no longer re-decodes encoded output in production.
8. Encoded tiled JPEG roundtrip remains externally verified by tests.
9. Standard application JPEG verification retains one decoded coefficient state across redundancy/payload-size probes.
10. Standard+tiled fallback reuses the same decoded state where technically reasonable, with any residual additional decode explicitly documented.
11. Search domain/order/resource limits are unchanged.
12. JPEG implementation structs remain private.
13. Deterministic carrier output tests pass.
14. Benchmark/decode/allocation evidence is recorded.
15. The ledger gives a clear `PUBLIC-REUSE-CANDIDATE` or `PRIVATE-REUSE-SUFFICIENT` disposition for Plan 079.
16. `./scripts/check.sh` passes.
17. No version, publication, dependency, or CI expansion occurs.

---

## 8. Non-goals

Do not use this plan to:

- add stable tiled carrier APIs;
- expose prepared JPEG state publicly;
- expose coefficient/parser/F5 types;
- change carrier algorithms;
- reduce verification search coverage;
- add codecs/media formats;
- redesign application payload/authentication/metadata;
- publish a release.