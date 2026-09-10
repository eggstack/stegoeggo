# Plan 096: Parent/Carrier Boundary and v1 API Cleanup

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/090-generic-carrier-v1-semantics-correctness-and-reuse-roadmap.md`

Depends on: Plans 091-095 as applicable to the final carrier surface.

Audited baseline: `27bdd3d429d663948021de43d3e6f818fa613319`.

## 1. Objective

Finish the architectural boundary between `stegoeggo-stego` as a generic arbitrary-payload carrier crate and `stegoeggo` as the rights/provenance application, then record and implement the v1 disposition of residual parent APIs that still expose the deprecated configuration model.

This plan must preserve StegoEggo's current application behavior while moving fallback/policy semantics to the layer that owns them.

## 2. Boundary problems to resolve

### 2.1 Application outcomes in the generic crate

`EmbedOutcome`, `EmbedOutcomeSummary`, `EmbedPath`, and `EmbedStatus` currently include states such as `UnsupportedProgressive` and `QTableSeedOnly` and are consumed by the parent pipeline for warnings/reporting. Those concepts are not necessary to describe a generic arbitrary-payload carrier once Plan 091 provides strict/explicit JPEG operations.

The parent should own application-level concepts such as:

- progressive JPEG degradation to seed-only protection;
- best-effort redundancy reduction;
- hidden-marker path classification used in `ExecutionReport`;
- warnings for skipped/degraded protection;
- mapping generic carrier failures into policy/report semantics.

### 2.2 Hidden `application-support`

The carrier's hidden `application-support` feature currently retains legacy LSB compatibility, historical seed fallback, tile-seed/candidate search, and private JPEG single-decode verification reuse. Plans 094/095 may make some of the generic reuse helpers unnecessary.

Audit each remaining symbol. Stable public carrier APIs should replace hidden support only when their semantics are genuinely generic. Legacy V1 and StegoEggo-specific candidate classification may remain hidden.

### 2.3 Legacy parent extension model

The root `ProtectionPipeline` is an empty nominal object whose public processing methods accept `ProtectionLevel`/`ProtectionContext` and translate into canonical request/plan execution. The public `Protector` trait is also defined in terms of `ProtectionContext` and `ProtectionLevel`, despite the modern invariant that new behavior enters through:

```text
ProtectionRequest -> resolve_request -> ResolvedProtectionPlan -> canonical execution
```

`ProtectionLevel` and related configuration are already documented for removal at v1, but the disposition of `ProtectionPipeline`, `Protector`, and protector structs must be explicit before that boundary.

## 3. Parent-owned embedding semantics

After Plan 091, implement the parent JPEG path conceptually as:

```text
resolve request/plan
  -> attempt generic strict or explicitly best-effort DCT carrier operation
  -> on unsupported/progressive or application-authorized degradation:
       explicitly call generic seed-hint operation
  -> convert generic facts into parent-local execution outcome/warnings
```

The generic carrier should never need to know that StegoEggo calls this `SeedOnly`, `BestEffort`, `Tiled`, or a rights/provenance protection channel.

Similarly, raster seed fallback after LSB payload embedding remains an application compatibility decision. Keep it behind hidden support if it is part of existing StegoEggo verification compatibility rather than a generally useful carrier feature.

## 4. Public API disposition audit

Before removing or deprecating anything, write a table in `plans/096-status.md` for:

- `ProtectionPipeline`;
- `Protector`;
- `SteganographyProtector`;
- `RightsMetadataProtector`;
- `MetadataTrapProtector`;
- `PassthroughProtector`;
- `ProtectionContext`;
- `ProtectionLevel`;
- `EvidenceProfile`;
- root `stego` convenience re-export;
- carrier application-shaped outcome types;
- `application-support` exports.

For each, record: current public/hidden status, demonstrated external use if known, canonical replacement, 0.x compatibility requirement, v1 disposition, and exact migration documentation required.

### Preferred v1 direction

- `ProtectionRequest`/`ResolvedProtectionPlan` and canonical top-level processing remain the main parent API.
- `ProtectionPipeline` should either disappear at the breaking boundary or gain real request-based state/value that justifies its existence. Do not keep an empty wrapper solely for object-oriented symmetry.
- `Protector` should not survive in its current legacy-context form. If third-party protector extensibility is a real requirement, design a new plan/request-oriented extension contract separately; do not mechanically rename the old trait.
- Concrete protector structs should remain public only where they expose a coherent independently useful operation not better served by canonical request functions.
- `stegoeggo::stego` may remain as a convenience re-export if maintenance cost is negligible, but documentation should name `stegoeggo-stego` as the canonical generic package. No generic example should require the parent crate unnecessarily.

Because repository policy promises deprecated API retention throughout 0.x, actual removals occur only in an explicit v1/breaking-version step. Prior changes should deprecate and migrate first-party use.

## 5. Implementation phases

### Phase 0 — behavior and dependency baseline

Record parent output bytes, warning sets, `EmbedPath`/report fields, and verification behavior for:

- PNG/WebP standard LSB;
- JPEG baseline DCT success;
- JPEG lower-redundancy success;
- JPEG insufficient capacity;
- progressive JPEG seed-only fallback;
- tiled LSB/JPEG;
- metadata-only/seed-only requests;
- legacy level/context equivalents.

### Phase 1 — parent-local outcome vocabulary

Create private or parent-owned application outcome types with only the fields required for `ExecutionReport`, warning generation, and strict CLI behavior. Prefer using existing `ExecutionReport`/warning types directly if a separate public type is unnecessary.

Migrate parent adapters away from application-oriented outcome types re-exported by the generic carrier. Avoid adding another permanently public result hierarchy merely to move code.

### Phase 2 — explicit JPEG fallback orchestration

Use Plan 091's strict/explicit APIs. Progressive/unsupported and insufficient-capacity decisions belong in `src/protected/steganography/embed.rs` or the canonical pipeline layer. The final output/warning behavior should remain equivalent for current requests unless a specifically documented correctness fix requires a change.

The seed-hint correctness fix may turn a formerly silent partial hint into a warning/error/degraded outcome. Define that parent behavior explicitly and test it.

### Phase 3 — reduce hidden support

After Plan 094, determine whether `application_support::JpegSearchContext` can be replaced by the public opaque prepared JPEG carrier. After Plan 095, determine whether application crop/search code can use generic buffer views without exposing candidate-classification semantics.

Delete hidden wrappers that are now exact duplicates. Retain and document only:

- legacy carrier compatibility required to read existing images;
- historical seed fallback required by StegoEggo verification;
- application-specific bounded candidate classification not appropriate for generic API.

### Phase 4 — v1 legacy parent cleanup

Apply the status-ledger disposition to deprecated parent APIs only when executing an explicit breaking version. Update `DEPRECATIONS.md`, `STABILITY.md`, `docs/rust-api.md`, architecture docs, and migration guide with exact replacements.

No replacement trait/framework should be invented unless a current use case requires one. Top-level request functions are a valid final API.

### Phase 5 — first-party dogfooding

Search production source for compatibility-only APIs. New/current code must not construct `ProtectionContext` or select `ProtectionLevel` except inside translation/compatibility modules and tests specifically covering deprecated behavior.

Generic carrier examples/tests should prefer direct `stegoeggo-stego`; parent examples use root APIs only when demonstrating rights/provenance behavior.

## 6. Required tests

Add/retain tests proving:

- parent baseline JPEG success bytes/verification remain compatible after fallback ownership moves;
- progressive JPEG still follows the same documented application degradation path;
- insufficient-capacity warnings/report disposition remain correct;
- seed-hint preflight failures from Plan 091 are represented truthfully rather than reported as successful seed-only protection;
- PNG/WebP LSB outputs and verification are unchanged;
- tiled paths retain bounded recovery and application report classification;
- legacy `ProtectionLevel`/`ProtectionContext` adapters remain equivalent through the promised 0.x window;
- no production current path depends on deprecated builders;
- direct carrier consumers do not enable `application-support`;
- hidden application-support export count is recorded before/after;
- if prepared JPEG replaces hidden search context, verification still uses one coefficient decode per operation.

## 7. Acceptance criteria

- Progressive/seed-only/best-effort warning policy is owned by `stegoeggo`, not encoded in the recommended generic carrier result contract.
- Parent uses explicit carrier operations from Plans 091/094/095 and does not duplicate their algorithms.
- `application-support` is reduced to evidenced legacy/application-specific compatibility only.
- Every legacy parent API has a documented v1 keep/remove/replace decision.
- `ProtectionPipeline` and `Protector` do not remain accidental parallel architectures after the v1 boundary.
- Existing 0.x compatibility promises are honored until that boundary.
- Root `stego` re-export disposition is documented and generic direct-crate usage is first-class.
- Application regression/verification suites, CLI tests, Clippy, and `./scripts/check.sh` pass.

## 8. Non-goals

No plugin architecture, no general third-party protector framework unless separately justified, no new provenance/signature model, no payload wire redesign, no C2PA work, and no CLI feature expansion beyond migration required by API cleanup.
