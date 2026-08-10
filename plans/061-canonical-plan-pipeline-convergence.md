# Plan 061: Canonical Plan Pipeline Convergence

Status: Ready for implementation

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Depends on: Plans 058-060 complete.

Audited planning baseline: `main` at `e60b801fda493ddd5e744f5de2a12d7b9489c078`

Authoritative implementation ledger to create before source edits: `plans/061-status.md`

---

## 1. Purpose

Make `ProtectionRequest -> ResolvedProtectionPlan -> execution` the actual canonical runtime architecture rather than a request-resolution layer that reconstructs a legacy `ProtectionContext` before invoking older pipeline methods.

Current canonical request processing resolves a `ProtectionRequest`, then `process_plan_bytes()` uses `plan_to_context()` and routes back through `ProtectionPipeline::process_bytes_pipeline()`. The older `ProtectionLevel`/`ProtectionContext` APIs also retain their own Light/Standard branching and byte/image execution helpers.

This plan converges those paths while preserving legacy public behavior.

The intended result is:

```text
new API:
ProtectionRequest
    -> resolve_request
    -> ResolvedProtectionPlan
    -> execute_plan

legacy API:
ProtectionLevel + ProtectionContext
    -> compatibility request construction
    -> resolve_request
    -> ResolvedProtectionPlan
    -> execute_plan
```

There should be one byte-oriented production execution graph for rights metadata + hidden marker processing.

---

## 2. Governing constraints

1. `ProtectionRequest` remains the canonical policy-first API.
2. `ResolvedProtectionPlan` remains immutable execution state.
3. Existing legacy APIs remain source-compatible unless already documented as deprecated.
4. Do not remove `ProtectionPipeline` merely because its internal role changes; preserve public constructors/methods unless a separate semver plan authorizes removal.
5. Do not create a second “execution context” containing a near-copy of `ResolvedProtectionPlan`.
6. Carrier mechanics must use the generic modules from Plan 059.
7. Metadata injection must preserve the container correctness established in prior plans.
8. `DynamicImage` APIs may remain convenience wrappers, but byte APIs are canonical for full metadata preservation.
9. Do not change rights-policy defaults, payload semantics, warnings, CLI flag behavior, or output-format preservation as part of simplification.
10. No release/CI/dependency changes.

---

## 3. Phase 0 — map every current execution entry point

Create `plans/061-status.md` before edits and record the actual baseline SHA.

Inventory these public/runtime paths:

```text
process_image
process_image_bytes
process_image_bytes_with_info
process_image_bytes_with_warnings
process_request_bytes
process_request_bytes_with_warnings
process_request_bytes_with_report
ProtectionPipeline::process
ProtectionPipeline::process_bytes
async wrappers
parallel wrappers
CLI process_single_file
```

For each path record:

- input domain (`DynamicImage` or encoded bytes);
- canonical/legacy configuration type;
- whether it preserves encoded metadata;
- whether it uses JPEG byte fast path;
- whether it routes through `plan_to_context()`;
- warning/report source;
- output-format decision point.

This inventory is the deletion checklist. Do not remove a path until its behavior is represented by the canonical executor or a documented pixel-only convenience wrapper.

---

## 4. Phase 1 — introduce one direct plan executor

Create or formalize one crate-private function whose input is encoded image bytes plus a `ResolvedProtectionPlan` and an operation budget, for example:

```rust
fn execute_plan_bytes(
    input: &[u8],
    plan: &ResolvedProtectionPlan,
    budget: &mut OperationBudget<'_>,
) -> Result<PipelineResult>;
```

The exact name may remain `process_plan_bytes` if that function is cleaned up rather than replaced.

The executor must directly perform:

```text
resource/input validation
-> metadata-only / hidden-marker decision
-> carrier selection and mechanical config derivation
-> StegoEggo payload generation
-> generic carrier embed
-> metadata injection
-> outcome/report conversion
```

It must not call `plan_to_context()` merely to access values already present in the plan.

When metadata helpers currently require `ProtectionContext`, add a narrow plan-aware internal entry point or refactor the metadata helper to accept the exact resolved values it needs. Prefer consuming `&ResolvedProtectionPlan` internally over creating `MetadataContext` that duplicates the full plan.

Likewise, StegoEggo payload generation should derive its emission fields directly from the resolved plan plus embed path.

---

## 5. Phase 2 — remove `plan_to_context()` from canonical request execution

After direct execution works:

- `process_request_bytes*` must resolve exactly once and call the direct plan executor;
- no canonical request path may reconstruct `ProtectionContext` for execution;
- warning/report generation must use the plan plus actual carrier outcome rather than context-derived guesses;
- metadata-only same-format fast paths remain byte-only;
- JPEG-in/JPEG-out supported stego remains byte/DCT-only;
- non-JPEG pixel stego still decodes once and re-encodes once as required.

If `plan_to_context()` remains temporarily for legacy helpers, mark it compatibility-only and ensure canonical request code has no reference to it. Preferred final state is deletion once Phase 3 is complete.

Required tests:

```text
request_path_does_not_require_protection_context_adapter
metadata_only_request_stays_byte_only_same_format
jpeg_request_uses_dct_carrier_without_pixel_roundtrip
png_request_uses_lsb_carrier
webp_request_uses_lsb_carrier
request_report_reflects_actual_embed_outcome
```

The first test may be structural/compile-time rather than runtime if appropriate; the status ledger must also record source-level removal of the adapter call.

---

## 6. Phase 3 — translate legacy configuration into canonical requests

Create one compatibility conversion path from legacy `ProtectionLevel + ProtectionContext` into a `ProtectionRequest` or equivalent call to the canonical resolver.

The mapping must account for every legacy field that affects observable processing, including at minimum:

```text
level/default policy semantics
input/output format
seed
intensity retained in StegoEggo payload compatibility
stego redundancy/tile mode
JPEG quality
progressive JPEG option
max dimension/resource limits
DMI/effective rights policy
metadata injection override
legal metadata
legal-claims compatibility flags
MAC key/evidence-profile compatibility
metadata update policy
```

Before implementing, enumerate `ProtectionContext` getters and explicitly mark each as:

```text
mapped
intentionally ignored because non-execution-only
not representable -> blocker requiring narrow request extension
```

Do not silently drop a context field.

If the canonical `ProtectionRequest` lacks a mechanical option needed to preserve legacy behavior, extend `ProcessingOptions` or the request builder narrowly rather than bypassing resolution.

The conversion must preserve current default behavior. In particular, do not accidentally reintroduce the previously corrected policy-default bugs.

---

## 7. Phase 4 — make legacy byte APIs call canonical execution

Refactor:

```text
process_image_bytes
process_image_bytes_with_warnings
ProtectionPipeline::process_bytes
```

so they use the compatibility request conversion and canonical plan executor.

Required equivalence tests should construct matching legacy and canonical requests and compare:

- output bytes where deterministic configuration makes exact equality expected;
- effective policy;
- metadata presence;
- carrier outcome/path;
- warnings;
- verification result;
- output format;
- JPEG container preservation.

Required matrix:

```text
Disabled
Light PNG
Light JPEG
Standard PNG
Standard WebP
Standard supported JPEG
Standard progressive JPEG fallback
HMAC-authenticated PNG
HMAC-authenticated JPEG
tiled PNG
tiled JPEG where supported
metadata disabled compatibility case where legal constraints allow it
explicit Unspecified policy
```

Where a legacy semantic cannot be represented exactly, stop and document the blocker rather than adding a second runtime branch.

---

## 8. Phase 5 — simplify `ProtectionPipeline`

Once byte execution is canonical, reassess `ProtectionPipeline` internals.

Its current Arc-held passthrough/metadata/steganography protectors are not required if execution is functionally dispatched through the resolved plan and carrier modules.

Permitted simplification:

- retain `ProtectionPipeline` as a lightweight public compatibility facade;
- keep `new`, `Default`, `Clone`, `process`, and `process_bytes` behavior;
- remove private stored protector instances if they no longer provide state or polymorphism;
- have methods delegate to canonical helpers.

Do not remove the public type in this roadmap.

If keeping internal fields is actually simpler due to compatibility or state, that is acceptable, but they must not restore a second independent execution graph.

---

## 9. Phase 6 — collapse duplicate Light/Standard helpers

Delete or retire helpers whose only purpose was to maintain the old independent pipeline, including candidates such as:

```text
apply_light_bytes
apply_light_bytes_pipeline
apply_standard_pipeline
apply_pipeline_bytes
apply_bytes_pipeline_resolved
context_for_bytes
```

Exact deletions depend on the source after Plans 058-060.

A helper may remain if it implements one distinct format-domain operation used by the canonical executor. It must not remain merely because legacy APIs call it separately.

The final source should not contain two different functions deciding:

- JPEG vs LSB carrier;
- metadata-only vs stego;
- output format;
- Light/Standard execution mechanics.

Policy/request resolution decides intent; the executor decides format mechanics once.

---

## 10. Phase 7 — keep DynamicImage APIs honest and simple

`DynamicImage` cannot preserve encoded-file metadata. Preserve that documented limitation.

Preferred behavior:

- `process_image` remains a pixel convenience API;
- it may encode through the canonical byte executor and decode the result back if that is the simplest compatibility-preserving implementation;
- avoid multiple decode/encode cycles;
- do not pretend returned `DynamicImage` contains file-level XMP/COM/tEXt metadata.

If `ProtectionPipeline::process` retains a more direct pixel path for efficiency, it must reuse the same generic LSB/JPEG carrier components and not become a second policy resolution path.

Document any intentional difference between byte and `DynamicImage` APIs.

---

## 11. Phase 8 — async and parallel wrappers

Verify that async/parallel APIs remain thin wrappers around canonical synchronous functions.

Do not duplicate request resolution or carrier selection inside async modules.

Required checks:

- async byte request processing delegates to the same synchronous canonical byte path inside `spawn_blocking` or current wrapper strategy;
- parallel helpers map canonical processing over items;
- no new runtime dependency/features are added.

---

## 12. Acceptance criteria

Plan 061 is complete only when:

1. `ResolvedProtectionPlan` is consumed directly by canonical encoded-byte execution.
2. Canonical `process_request_bytes*` paths no longer reconstruct `ProtectionContext` for execution.
3. Stego carrier selection and metadata-only decisions occur in one direct plan executor.
4. Legacy `ProtectionLevel`/`ProtectionContext` byte APIs translate into canonical request/plan execution.
5. Every execution-relevant `ProtectionContext` field is either mapped, explicitly non-execution-only, or recorded as a blocker; none is silently lost.
6. Legacy/canonical equivalence tests pass across the required format/protection matrix.
7. Existing policy-default and explicit-Unspecified regressions remain fixed.
8. `ProtectionPipeline` remains public-compatible but no longer owns a second independent execution graph.
9. Duplicate Light/Standard pipeline helpers are removed or reduced to format-local helpers used by the canonical executor.
10. JPEG-in/JPEG-out supported requests still avoid pixel decode/re-encode.
11. Same-format metadata-only requests remain byte-only.
12. PNG/WebP hidden-marker requests use the generic LSB carrier from Plan 059.
13. Supported JPEG hidden-marker requests use the generic JPEG carrier from Plan 059.
14. Existing warnings/reports remain derived from the resolved plan plus actual runtime outcome.
15. `DynamicImage` metadata limitations remain documented and behavior-compatible.
16. Async and parallel wrappers do not introduce another execution path.
17. No public API removal, new image format, dependency addition, CI change, version bump, or release action occurs.
18. `./scripts/check.sh` passes.
19. `plans/061-status.md` records deleted/retained pipeline helpers, full legacy-field mapping, equivalence evidence, and final architecture disposition.

---

## 13. Stop conditions

Mark Plan 061 `PARTIAL` if exact legacy behavior cannot be expressed through the canonical request/plan model without an unplanned public contract change.

Do not solve such a blocker by keeping an undocumented second pipeline. Record the missing request capability explicitly so a narrow follow-up can address it.