# Language Bindings Milestone 006 — Post-Core Compatibility Corrective and Requalification

Status: ready for handoff

Repository baseline: `f80eebe3fb983a5e8b13f7e082fe573636d65756`

Source roadmap:
`plans/subsystems/language-bindings-roadmap.md#m006--post-core-compatibility-corrective-and-requalification`

Corrects:

- post-M005 compatibility drift against current canonical Rust;
- `plans/closure/language-bindings/005-status.md` only prospectively — the
  historical M005 evidence remains valid for its qualified SHA;
- Python error-contract drift discovered after M003/M004 closure.

Interface dependency:

- `plans/implementation/verification-conformance/006-unstructured-candidate-false-positive-corrective.md`

Long-term requirements:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/000-long-term-specification.md#5-release-invariants`
- `plans/001-terminology-and-domain-model.md#verification-model`
- `plans/002-long-term-roadmap.md#phase-5--language-bindings-active`

Applicable ADRs:

- `plans/adrs/ADR-0001-canonical-protection-request.md`
- `plans/adrs/ADR-0003-byte-vs-pixel-paths.md`
- `plans/adrs/ADR-0005-foreign-language-bindings.md`

Primary class: invariant

## 1. Objective

Restore Python and Node compatibility with the current canonical Rust surface
after post-M005 core correctness changes introduced three observable drifts:

1. both foreign bindings now see an unprotected valid image as hidden-marker
   `Invalid` instead of `NotFound`; this is owned by verification-
   conformance M006 and must be fixed in Rust, not hidden in wrappers;
2. the new public Rust `Error::ResourceLimitExceeded(String)` falls through
   each binding's non-exhaustive fallback instead of the existing
   resource-limit error family;
3. Python freezes one descriptive metadata-limit `kind` string
   (`"tEXt field"`) even though canonical Rust now emits
   `"metadata field"` for that path.

After semantic reconciliation, requalify the executable Python and Node
bindings on their documented five-target matrices because native binding code
will change.

No C ABI design or implementation begins until this corrective closes.

## 2. Why this milestone is ready

The failures are concrete and independently reproduced.

On `8c89e8cb1677a355d639ca1ead93c3dd2587e317`:

- root CI run `36874105954`: success;
- Node binding run `36874106064`: failure after successful native build and
  declaration drift check, 60/64 tests passing;
- Python binding run `36874106092`: failure, 68 passed / 2 failed / 1
  skipped.

The four Node failures all expose canonical hidden-marker status
`INVALID` where absence is expected. Python independently reproduces the
unprotected-image `Invalid`/ `NotFound` mismatch.

Python additionally fails its metadata-limit assertion because the canonical
descriptive `kind` value changed from `"tEXt field"` to
`"metadata field"`.

A prior core commit added
`Error::ResourceLimitExceeded(String)`. Current projections are stale:

- Node `NativeError::from_rust` wildcard maps it to
  `ERR_STEGOEGGO_INTERNAL`;
- Python `map_error` wildcard maps it to generic `StegoEggoError`.

The binding architecture itself remains sound. M005's original five-target
qualification run `36488540671` is valid historical evidence for SHA
`abc035400d1b693f238ff133bb46c5a1dedb9d7c`; it is not evidence for
executable binding changes made by this corrective.

## 3. Current implementation evidence

### Node

`bindings/node/src/error.rs` already has the stable public family
`ERR_STEGOEGGO_RESOURCE_LIMIT` and structured optional fields. The new
carrier-level Rust variant simply lacks an explicit match arm.

The Node metadata-limit JS test correctly checks that `kind` is a string
rather than freezing the old implementation-specific wording.

The Node CI failure occurs after:

- Rust/napi build succeeds;
- generated `index.js`/`index.d.ts` drift check succeeds.

This is behavioral drift, not packaging/compiler breakage.

### Python

`bindings/python/src/lib.rs` has `ResourceLimitError` and explicit mappings
for all previously known structured limit variants. The new carrier-level
variant is not mapped.

`bindings/python/tests/test_errors.py` asserts exact descriptive
`kind == "tEXt field"`, which is stricter than the Node contract and stricter
than the stable semantic requirement of “metadata resource limit with a
descriptive kind”.

### Verification dependency

The cross-binding `Invalid` regression is owned by
verification-conformance M006. This plan consumes its written interface
contract:

- no credible hidden marker => `NotFound`;
- structured/provenance-bearing corrupted marker => `Invalid`.

Bindings must project that result verbatim.

## 4. Invariants that must not regress

- Foreign bindings do not reinterpret canonical verification status.
- Python and Node expose the same canonical status semantics.
- Existing binding-specific error classes/codes remain stable.
- Resource exhaustion never becomes an internal/generic error merely because
  Rust gained a new non-exhaustive variant.
- Do not invent unavailable structured facts for
  `ResourceLimitExceeded(String)`.
- Secret material never appears in error messages/properties/repr/JSON.
- Python and Node continue to tolerate future non-exhaustive Rust additions
  safely.
- `kind` remains a descriptive canonical Rust-provided string, not a
  language-binding enum unless the Rust API explicitly creates such a stable
  taxonomy.
- CPU-detachment/event-loop/GIL behavior remains unchanged.
- Node input-copy/thread-safety invariants remain unchanged.
- Python file-helper atomicity remains unchanged.
- Binding crates remain outside the root Cargo workspace.
- `./scripts/check.sh` remains free of Python/Node prerequisites.
- No PyPI/npm publication occurs.
- M005 closure is historical and is not rewritten to pretend its old SHA
  qualified future code.

## 5. Scope

### In scope

- Explicit Python mapping for `Error::ResourceLimitExceeded(String)`.
- Explicit Node mapping for `Error::ResourceLimitExceeded(String)`.
- Stable cross-language representation of that carrier resource limit:
  - resource family is the existing resource-limit family;
  - `resource = "carrier"`;
  - human message preserved;
  - unavailable `size`, `limit`, `kind`, `count`, dimensions remain
    absent/null rather than fabricated.
- Focused projection tests in Rust/Python/Node.
- Python metadata-limit test/contract correction so descriptive `kind`
  remains observable but exact parser wording is not frozen without a
  canonical Rust guarantee.
- Python/Node verification tests after verification-conformance M006 lands.
- Cross-language parity rerun against current Rust.
- Lightweight Python and Node CI green on the same current core.
- Fresh manual Python and Node five-target artifact qualification after final
  executable binding changes.
- SUPPORT/RELEASING/closure/registry/roadmap reconciliation.

### Explicitly out of scope

- Fixing canonical verification logic inside the binding plan.
- Mapping canonical `Invalid` to `NotFound` in Python/JavaScript.
- New binding APIs.
- New platforms or runtimes.
- npm/PyPI publication.
- C ABI design/implementation.
- Shared bindings-core extraction.
- Changing Rust resource-limit variants merely for wrapper convenience.
- Converting descriptive `kind` strings into a new public enum.

## 6. Required production changes

### A. Python carrier resource-limit projection

Add an explicit `RustError::ResourceLimitExceeded(message)` match in
`bindings/python/src/lib.rs`.

Project it as `ResourceLimitError` with:

- `resource = "carrier"`;
- canonical human message;
- no fabricated numeric fields.

If the Python exception class currently implies that all optional attributes
always exist, normalize the runtime object contract so documented optional
attributes are either consistently absent by documented design or consistently
present as `None`. Do not create a broader typing cleanup unless needed to
make the carrier case truthful.

Add a focused test that constructs/reaches the variant through a binding-side
projection helper if no safe public input reliably triggers it.

### B. Node carrier resource-limit projection

Add an explicit `RustError::ResourceLimitExceeded(_)` arm in
`NativeError::from_rust`:

- `code = ERR_STEGOEGGO_RESOURCE_LIMIT`;
- `resource = "carrier"`;
- all unavailable structured numeric/kind fields `null`;
- preserve the canonical message.

Add a Rust unit projection test and JS public-error test.

The existing wildcard remains for genuinely unknown future variants and still
maps to `ERR_STEGOEGGO_INTERNAL`.

### C. Metadata-limit `kind` contract

Do not hard-code `"tEXt field"` in Python unless the Rust API explicitly
documents that exact string as stable.

The stable binding contract is:

- Python/Node resource category is `metadata`;
- `kind` is present and non-empty when Rust supplies it;
- `size` and `limit` are preserved exactly;
- the binding does not rewrite the canonical descriptive `kind`.

Update the Python test to assert those semantics and, where useful, that it
equals the actual Rust-provided value for the chosen fixture without
presenting the wording as an enum/API taxonomy.

Document that `kind` is descriptive diagnostic context. Consumers needing a
stable branch key use `resource` / error class/code.

### D. Verification status reconciliation

After verification-conformance M006 lands:

- retain Python and Node expectations that unprotected valid images produce
  hidden-marker `NotFound`;
- retain metadata-only expectation: rights may be present while hidden marker
  is `NotFound`;
- do not change wrappers to massage statuses.

Add/retain explicit tests in both languages so future canonical drift triggers
both workflows.

### E. Cross-language error parity

Add a compact parity matrix proving Python and Node classify equivalent Rust
failures into equivalent semantic families:

| Rust condition | Python | Node |
|---|---|---|
| `InputTooLarge` | `ResourceLimitError/resource=input_bytes` | resource-limit / input_bytes |
| `DimensionsExceeded` | resource/dimensions | resource/dimensions |
| `ContainerLimitExceeded` | resource/container | resource/container |
| `MetadataLimitExceeded` | resource/metadata | resource/metadata |
| `VerificationBudgetExceeded` | resource/verification_budget | resource/verification_budget |
| `ResourceLimitExceeded` | resource/carrier | resource/carrier |

This can be enforced through language-local tests over equivalent synthetic
projection inputs where production reachability is not deterministic.

### F. Requalification rule

Because this corrective changes executable native binding code, old native
artifact qualification does not cover the final implementation SHA.

After all code/test changes settle, manually dispatch:

- `.github/workflows/release-python.yml`;
- `.github/workflows/release-node.yml`.

Every documented five-target row must build and pass its same-native smoke.
Node package assembly must pass. Python sdist direct-install smoke must pass.

No registry publication.

## 7. Ordered work packages

### WP1 — Consume verification M006 interface and add regression assertions

Ensure both bindings express the canonical absence/corruption contract
directly in tests.

Acceptance:

- tests still fail against the broken core rather than being weakened;
- once verification M006 lands, unprotected and metadata-only cases pass
  without wrapper translation.

### WP2 — Python resource-limit reconciliation

Changes:

1. map carrier resource limit explicitly;
2. add projection test;
3. relax metadata `kind` wording assertion to the stable semantic contract;
4. preserve all prior structured fields.

Acceptance:

- all Python error tests pass;
- carrier resource limit is never generic `StegoEggoError`;
- metadata test preserves `resource`, `kind`, `size`, `limit`.

### WP3 — Node resource-limit reconciliation

Changes:

1. explicit carrier mapping;
2. Rust mapping unit test;
3. JS public-error projection test;
4. verify future-unknown fallback remains internal.

Acceptance:

- carrier condition has `ERR_STEGOEGGO_RESOURCE_LIMIT`;
- `resource === "carrier"`;
- no invented numeric fields.

### WP4 — Binding-native and cross-language parity

Run:

- Python full suite;
- Node Rust/unit suite;
- Node JS tests + typecheck;
- Rust↔Node parity;
- Python/Rust parity where the existing harness supports it.

Acceptance:

- both language suites green against the same core commit;
- verification status agrees across Rust/Python/Node;
- error family table is satisfied.

### WP5 — Lightweight remote CI

Push the final binding/core combination and require:

- standard Rust CI success;
- `python-binding` success;
- `node-binding` success including Node 22/24/26 smoke.

Record all three run IDs and exact common/final SHA relationships.

### WP6 — Fresh five-target Python qualification

Manually dispatch `release-python.yml` on the final executable SHA.

Acceptance:

- Linux x86_64/aarch64 wheel build + same-native smoke;
- macOS x86_64/arm64 wheel build + same-native smoke;
- Windows x86_64 wheel build + native smoke;
- direct sdist `pip install` smoke;
- no PyPI publication.

### WP7 — Fresh five-target Node qualification

Manually dispatch `release-node.yml` on the final executable SHA.

Acceptance:

- five native addon builds/smokes;
- Linux napi-cross behavior retained;
- Node 22/24/26 Linux x86 runtime smoke retained;
- package assembly/audit/local pack-install smoke green;
- no npm publication.

### WP8 — Closure and C ABI gate reconciliation

Write closure and update support/planning only after all required evidence is
green.

Acceptance:

- M006 closed;
- prior M005 closure remains untouched as historical evidence;
- new qualification run IDs are the current binding evidence;
- C ABI design becomes M007 and only then is dependency-ready.

## 8. Failure, cancellation, restart, and contention semantics

- A red Python or Node compatibility run blocks closure even if root Rust CI is
  green.
- A wrapper must not compensate for a still-broken canonical verifier.
- If verification-conformance M006 changes its written interface, stop and
  reconcile before modifying wrapper expectations.
- Cancelled native qualification is not evidence.
- Any executable Python binding change after the Python qualification SHA
  invalidates that run for closure.
- Any executable Node binding/package/release workflow change after the Node
  qualification SHA invalidates that run for closure.
- Documentation-only planning commits do not invalidate artifacts when the
  executable diff is proven empty.
- Partial platform success does not qualify the matrix.
- Do not publish to a registry to test optional-package resolution.
- Do not invent structured fields from the carrier resource-limit message.
- Unknown future Rust variants continue to fall back safely rather than
  causing a non-exhaustive match build failure.

## 9. Compatibility and migration

This corrective preserves the public error families and verification status
model.

Intentional clarification:

- `resource` is the stable machine category;
- `kind` is descriptive Rust-provided context unless separately documented
  by core as a stable taxonomy.

The new `resource="carrier"` value is additive within the existing
resource-limit family and gives callers a stable way to classify the new Rust
variant.

No package version bump/publication is required merely for local corrective
qualification; version policy remains tied to the next actual StegoEggo source
release.

No migration impact to Rust/CLI consumers.

## 10. Required tests

Python:

1. unprotected image => `NotFound`;
2. metadata-only => rights found + marker `NotFound`;
3. input/dimension/container/metadata/verification-budget limits preserved;
4. carrier resource limit => `ResourceLimitError`, resource `carrier`;
5. metadata `kind` non-empty/descriptive without stale exact wording;
6. no secret leakage;
7. full existing 71-test-or-later suite.

Node:

8. unprotected image => `NOT_FOUND`;
9. metadata-only => marker `NOT_FOUND`;
10. concurrent operations retain corrected status;
11. carrier resource limit => resource-limit code + `carrier`;
12. unavailable carrier fields remain null/undefined;
13. future synthetic unknown remains internal fallback;
14. all prior resource-limit projection tests;
15. full JS suite;
16. TypeScript compile contract;
17. generated declaration drift check;
18. Node 22/24/26 runtime smoke.

Parity:

19. equivalent resource-limit family table;
20. Rust/Python/Node unprotected status agreement;
21. deterministic protect/verify parity still passes;
22. HMAC correct/wrong/missing behavior still agrees.

## 11. Required verification commands

Root:

```bash
./scripts/check.sh
```

Python:

```bash
cd bindings/python
maturin build --release --out dist-ci
python -m pip install --force-reinstall dist-ci/stegoeggo-*.whl
python -m pytest -v
```

Node:

```bash
cargo check --manifest-path bindings/node/Cargo.toml --locked
cargo clippy --manifest-path bindings/node/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path bindings/node/Cargo.toml --locked
cd bindings/node
pnpm install --frozen-lockfile
pnpm build
git diff --exit-code -- index.js index.d.ts
pnpm test
pnpm typecheck
STEGOEGGO_CLI=../../target/debug/stegoeggo pnpm test:parity
```

Build the CLI before parity:

```bash
cargo build -p stegoeggo-cli
```

Remote:

- final standard CI run;
- final `python-binding.yml` run;
- final `node-binding.yml` run;
- manual `release-python.yml` run;
- manual `release-node.yml` run.

Record exact run IDs/SHAs; do not cite older M003/M005 artifact runs as current
qualification for changed native code.

## 12. Documentation updates

Update:

- `bindings/python/README.md` if structured resource-limit docs enumerate
  categories;
- `bindings/node/README.md` error-code/resource docs;
- `SUPPORT.md` with new qualification run evidence;
- `RELEASING.md` only if the qualification process itself changes;
- `plans/subsystems/language-bindings-roadmap.md`;
- `plans/registry.md`;
- new closure `plans/closure/language-bindings/006-status.md`.

Do not rewrite M001–M005 closure records except factual cross-reference
corrections if absolutely necessary.

## 13. Acceptance criteria

M006 closes only when:

- verification-conformance M006 is closed;
- Python and Node project canonical `NotFound`/ `Invalid` without wrapper
  reinterpretation;
- `ResourceLimitExceeded(String)` maps explicitly to the existing
  resource-limit family in both languages;
- both expose stable `resource="carrier"`;
- neither invents size/count/limit facts absent from Rust;
- metadata `kind` tests no longer freeze implementation wording as a stable
  enum;
- all Python tests pass;
- all Node Rust/JS/type/parity tests pass;
- Rust CI, Python binding CI, and Node binding CI are green on the reconciled
  source;
- fresh Python five-target + sdist qualification is green;
- fresh Node five-target + assembly qualification is green;
- no PyPI/npm publication occurred;
- SUPPORT reflects only newly recorded current evidence;
- C ABI design remains blocked until this closure is accepted.

## 14. Stop conditions

Stop and report if:

- verification-conformance M006 concludes `Invalid` is actually correct for
  the supposedly unprotected canonical fixture;
- the carrier resource-limit variant gains structured fields in core before
  implementation and this plan's `resource="carrier"`/message-only projection
  becomes stale;
- preserving the Python exception hierarchy requires a breaking redesign;
- Node structured rejection would require changing the public error-code
  family;
- a binding fix requires canonical Rust semantics to diverge by language;
- native qualification exposes a platform-specific regression;
- fixing metadata `kind` requires declaring a new stable cross-language enum
  without an ADR/core contract.

## 15. Closure evidence required

Create `plans/closure/language-bindings/006-status.md` and record:

- verification-conformance M006 closure reference;
- implementation commits for Python and Node;
- exact final core/binding SHA;
- before/after Python and Node failing test disposition;
- Python carrier resource-limit projection evidence;
- Node carrier resource-limit projection evidence;
- metadata `kind` contract disposition;
- full Python test count/result;
- full Node Rust/JS/type/parity counts/results;
- `./scripts/check.sh` result;
- final Rust CI run ID;
- final Python binding CI run ID;
- final Node binding CI run ID;
- fresh release-python run ID/SHA, wheel/sdist artifacts, five smokes;
- fresh release-node run ID/SHA, five addon artifacts/smokes, assembly result;
- confirmation no PyPI/npm publication occurred;
- SUPPORT/README changes;
- unresolved findings;
- registry/roadmap transition making M007 C ABI design dependency-ready.

## 16. Handoff notes

This is a compatibility corrective, not a reason to redesign either binding.

Implement projection fixes independently while verification-conformance M006
is being repaired, but keep the `NotFound` expectations intact. The binding
tests are downstream contract guards and should remain red against a broken
core rather than being weakened.

Use the existing resource-limit families. The new Rust carrier variant has
less structure than the older limit variants; preserve that fact instead of
parsing its message.

Fresh native qualification is required because the executable binding code
changes. Do not reuse M003/M005 native artifact runs as current evidence.

After closure, the roadmap proceeds to M007 C ABI contract design and M008 C
ABI implementation/qualification.
