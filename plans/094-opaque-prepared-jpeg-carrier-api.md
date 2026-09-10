# Plan 094: Opaque Prepared JPEG Carrier API

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/090-generic-carrier-v1-semantics-correctness-and-reuse-roadmap.md`

Depends on: Plans 091 and 092.

Audited baseline: `27bdd3d429d663948021de43d3e6f818fa613319`.

## 1. Objective

Provide a stable generic API for callers that need several JPEG carrier operations against the same encoded JPEG without decoding DCT coefficients for every query/extraction attempt, while keeping all parser, Huffman, coefficient, and F5 implementation types private.

The application already has hidden `JpegSearchContext` single-decode reuse for verification. This plan promotes the reusable *capability*, not that application-specific search type or its internal representation.

## 2. Use cases

A generic consumer may reasonably need to perform several operations on one JPEG:

```text
parse/support classification
capacity(payload A)
capacity(payload B)
raw extract attempt
framed extraction
bounded tiled recovery
```

The current one-shot functions correctly prioritize API simplicity but repeat coefficient decoding across separate calls. A reusable opaque handle is justified if it can preserve the existing one-shot API and keep implementation state private.

## 3. Public API shape

Evaluate and record the ownership design before implementation. Preferred shapes are:

```rust
pub struct PreparedJpeg<'a> { /* private */ }

impl<'a> PreparedJpeg<'a> {
    pub fn new(encoded: &'a [u8]) -> Result<Self, StegoError>;
    pub fn support(&self) -> JpegSupport;
    pub fn capacity(&self, payload_len: usize, config: &JpegConfig) -> Result<CapacityReport, StegoError>;
    pub fn extract(...);
    pub fn extract_framed(...);
    pub fn extract_tiled(...);
    pub fn extract_tiled_framed(...);
}
```

or an equivalent name/module arrangement.

A borrowing form is preferred because it lets the caller control allocation and avoids copying the original encoded JPEG solely so container-preserving re-encoding can refer to it. If an owned form has a demonstrated service/batch use case, add it separately (`from_vec`, `into_owned`, or a second type) rather than forcing an allocation on every caller.

Do not expose lifetimes or ownership complexity beyond what is justified by measurable allocation/decode savings. Record the selected API and rationale in `plans/094-status.md` before product edits.

## 4. Visibility and stability constraints

The prepared type is opaque. It may internally own/borrow:

- parsed JPEG header state;
- coefficient maps;
- available-capacity facts;
- original encoded bytes needed for preservation;
- cached geometry needed by tiled operations.

It must **not** publicly expose:

- `JpegHeader`;
- `Coefficients` / coefficient maps;
- scan/Huffman decoder state;
- `DctStegoF5`;
- raw mutable access to parsed tables;
- application `TiledJpegCandidateKey` or StegoEggo payload search semantics.

Private field layout may change in minor releases. Do not derive `Clone` if cloning implies a large coefficient-map copy without a clear contract. Implement `Debug` manually if derived output would expose noisy/private internals. Audit `Send`/`Sync`; if the representation is naturally thread-safe, add compile-time assertions/tests and document it, but do not add synchronization merely to obtain these traits.

## 5. Implementation phases

### Phase 0 — measure baseline decode ownership

Record coefficient decode counts for representative sequences using the one-shot API. Reuse existing test-only decode counters from Plans 078/080 where possible rather than adding production instrumentation.

Baseline scenarios:

- `capacity` + `extract`;
- `capacity` + `extract_framed`;
- repeated raw extraction with different expected payload sizes/redundancies;
- tiled framed recovery;
- parent verification standard + tiled fallback.

### Phase 1 — extract a private decoded-carrier core

Use the current `DecodedJpegCarrier`/equivalent as the semantic core. Move reusable operations onto references to decoded state so one-shot functions and the new prepared API delegate to the same implementation.

Avoid two implementations of capacity, F5 extraction, framed redundancy probing, or tiled grid geometry.

### Phase 2 — public prepared wrapper

Expose only the stable operation-level methods. Constructor parsing must return the same malformed/unsupported classification as the one-shot functions.

Decide whether unsupported-but-well-formed JPEGs can construct a prepared probe object or whether construction itself returns `UnsupportedJpeg`. The selected behavior must make support probing useful without requiring callers to parse twice. One acceptable design is a cheap header-only `probe_support` plus `PreparedJpeg::new` only for supported coefficient carriers; another is a prepared object carrying structured support state. Record the choice and measure whether it creates duplicate parsing.

### Phase 3 — embedding support

After Plan 091's strict/best-effort contract exists, determine which embedding methods belong on the prepared type.

For strict embedding, modifying a prepared carrier must not corrupt reusable original state on a failed attempt. Prefer immutable `&self` methods that clone only the coefficient structures needed for an output, or a consuming/mutable prepared operation with clearly transactional semantics if measurement shows that is materially better.

Do not expose mutable coefficients to avoid the clone.

### Phase 4 — parent reuse

Audit hidden `application_support::JpegSearchContext`. If the public prepared API can provide the same generic single-decode primitives without exposing application candidate classification, migrate the parent to the stable prepared carrier and delete/reduce the redundant hidden context.

Application-specific candidate enumeration may remain hidden if it needs semantics deliberately absent from the generic API. Avoid wrapping the prepared API in another state object solely to preserve old internal names.

## 6. Performance/evidence requirements

The plan is justified by reuse, so record measurable evidence rather than only API tests.

At minimum establish:

- one coefficient decode for a prepared object's lifetime across repeated supported read operations;
- one-shot functions retain their current decode counts;
- prepared capacity/extraction results are exactly equal to the one-shot API;
- no full encoded JPEG copy is forced by the borrowing constructor;
- no extra output encode occurs during read-only operations;
- tiled extraction still performs a bounded origin search;
- peak allocation or wall-time evidence for a representative repeated-operation workload is improved or, at minimum, decode count is deterministically reduced.

Do not add an always-on benchmark dependency. Existing Criterion/dev infrastructure may be extended if useful.

## 7. Required tests

Add tests for:

- malformed input parity with one-shot APIs;
- every `JpegUnsupportedReason` fixture available in the suite;
- capacity parity;
- raw extraction parity at several redundancy levels;
- framed extraction parity and lower-redundancy probing;
- tiled raw/framed parity and bounded search;
- seed-hint read/write behavior where included;
- supported embedding parity with one-shot strict/best-effort policy from Plan 091;
- failed embedding leaves reusable prepared state unchanged;
- APP/COM/unknown segment preservation on prepared embedding;
- test-only coefficient decode count exactly one across repeated operations;
- compiler privacy tests proving JPEG transcoder/header/coefficient types remain inaccessible.

## 8. Acceptance criteria

- A documented opaque prepared JPEG carrier is available for generic consumers.
- Repeated supported operations reuse one coefficient decode.
- One-shot APIs delegate to the same decoded-state implementation and remain available.
- No raw codec/F5 type becomes public.
- Borrowed construction avoids copying encoded input unless a documented operation requires an owned output.
- Failed operations do not poison or partially mutate reusable prepared state.
- Hidden parent JPEG reuse is reduced where the public prepared API genuinely subsumes it.
- Direct standalone consumer examples compile without `application-support`.
- Focused tests, doctests, Clippy, and `./scripts/check.sh` pass.

## 9. Non-goals

No public JPEG editing toolkit, no arbitrary coefficient access, no new JPEG coding-process support, no long-lived global cache, no async wrapper (coefficient work remains CPU-bound), and no generalized `Carrier` trait.
