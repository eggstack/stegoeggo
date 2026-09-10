# Plan 092: Validated Carrier Configuration and Result Contracts

Status: READY FOR IMPLEMENTATION

Parent roadmap: `plans/090-generic-carrier-v1-semantics-correctness-and-reuse-roadmap.md`

Depends on: Plan 091 semantic disposition for JPEG strict/best-effort embedding.

Audited baseline: `27bdd3d429d663948021de43d3e6f818fa613319`.

## 1. Objective

Define a v1-quality generic carrier configuration/result vocabulary whose invariants do not depend on build profile, whose types do not embed StegoEggo application policy, and whose evolution does not require adding public fields to already-exposed structs.

The transition must respect the repository's promise that deprecated 0.x APIs remain available through the v0.x series. This is therefore an additive-and-deprecating plan before any explicit v1 removal step.

## 2. Problems to solve

### 2.1 Build-profile-dependent configuration

`LsbConfig::with_redundancy` and `JpegConfig::with_redundancy` use `debug_assert!` for `1..=10`, then clamp invalid values in release builds. A caller passing runtime data can therefore observe panic in debug and silent coercion in release. Existing `try_new`/`try_with_redundancy` are deterministic and should inform the v1 design.

### 2.2 Repeated primitive validation

Redundancy is logically one constrained value shared by LSB and JPEG. Tile-size validation has both carrier-neutral and JPEG-specific constraints. Search-origin bounds are validated separately. The public API should make common invariants explicit without creating a generic abstraction that erases actual carrier differences.

### 2.3 Result types expose mutable compatibility surface

`EmbedReport<T>` exposes all fields publicly. Adding another field later is source-breaking to exhaustive struct literals/patterns. `EmbedOutcome`/`EmbedOutcomeSummary`/`EmbedPath`/`EmbedStatus` also contain application-oriented fallback states such as `UnsupportedProgressive` and `QTableSeedOnly`.

### 2.4 Error vocabulary/documentation drift

`StegoError::InvalidConfig` documentation currently lists zero seed as an example even though zero seed is accepted and has public round-trip tests. Error units and capacity semantics should be exact enough for third-party callers.

## 3. Desired contract

### 3.1 Validated redundancy value

Introduce one public validated redundancy representation, conceptually:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Redundancy(/* private */ u8);

impl Redundancy {
    pub const MIN: Self = ...;
    pub const MAX: Self = ...;
    pub fn new(value: u8) -> Result<Self, StegoError>;
    pub const fn get(self) -> u8;
}
```

Exact naming is implementation-owned. The type should have private representation, common traits, inexpensive copy semantics, and deterministic validation. Do not use `Deref` to the primitive.

Evaluate whether constants such as `ONE`, `DEFAULT_LSB`, and `DEFAULT_JPEG` materially improve call sites. Avoid a large constant vocabulary without demonstrated use.

`LsbConfig` and `JpegConfig` should gain constructors/setters using the validated type. Existing usize-based APIs remain compatibility wrappers/deprecated until the breaking boundary.

### 3.2 Explicit JPEG capacity policy

Consume Plan 091's semantic decision through a small explicit policy/options type. If an enum is public and may grow, make it future-proof (`#[non_exhaustive]` or private constructor representation as appropriate). Avoid `bool` arguments such as `best_effort: true`.

### 3.3 Future-proof embed result

Design the recommended v1 embed result so callers can inspect:

- disposition/status;
- output carrier;
- payload bytes attempted/embedded;
- required/available capacity in the documented carrier unit;
- actual redundancy when meaningful.

Prefer private fields plus getters and decomposition helpers (`into_output`, `into_parts`) over another all-public struct. If a non-exhaustive enum is used for disposition, ensure ordinary callers do not need exhaustive matching merely to retrieve the output.

Do not put `QTableSeedOnly`, progressive fallback, rights/provenance state, warnings, or application policy into the generic result.

During 0.x, retain `EmbedReport`, `EmbedOutcome`, and related public items when compatibility requires it. Mark recommended versus compatibility surfaces clearly in rustdoc/STABILITY/DEPRECATIONS. Plan 096 owns migration of parent application outcomes away from generic-carrier application-shaped types.

### 3.4 Capacity units

Keep `CapacityReport` carrier-specific but make unit semantics impossible to misread in docs:

- LSB: RGB channel slots participating in the V2 mapping;
- JPEG DCT: currently eligible AC coefficient positions with `|coef| >= 2` after canonicalization;
- seed-hint capacity, if reported, must not masquerade as normal DCT payload capacity without an explicit unit/context.

Do not introduce a unit-generic trait unless it solves an actual public use case.

## 4. Implementation phases

### Phase 0 — public API/semver inventory

Record every public configuration/result item and all root re-exports. Record external struct-literal/pattern compatibility hazards. Use Cargo's SemVer guidance as the governing rule: additive items are preferred in the current compatibility line; removals/renames/public-field changes are reserved for a breaking version.

### Phase 1 — validated scalar types

Implement the shared validated redundancy type and migrate internal carrier code to consume it as near to the core as practical. Keep a single primitive validation function only as a compatibility adapter if needed.

Audit `TileConfig` and `MAX_TILED_ORIGINS`. `TileConfig` already has private fields and a fallible constructor; retain that good shape. Do not merge JPEG-specific tile constraints into generic `TileConfig`; JPEG's multiple-of-8/minimum-8 requirement belongs to the JPEG operation boundary.

### Phase 2 — deterministic compatibility builders

Remove build-profile dependence from the recommended API. For existing infallible `with_redundancy` methods, choose and document one transition:

- retain exact legacy behavior through 0.x but deprecate and ensure all first-party code uses validated APIs; or
- normalize behavior in a compatible bug-fix direction only if semver/repository policy permits it.

Do not silently choose during implementation. Record the compatibility decision and tests in the status ledger.

No first-party current path may depend on debug-only assertion behavior after this phase.

### Phase 3 — result model

Introduce/migrate to the future-proof carrier-neutral result model. Add conversion helpers from legacy `EmbedReport`/`EmbedOutcome` only where needed for 0.x compatibility; do not make both result models co-equal recommended APIs.

Ensure in-place operations have the same disposition/capacity vocabulary without forcing an owned output field.

### Phase 4 — errors and docs

Correct zero-seed documentation. Audit each `StegoError` variant for actionable distinction and avoid string-only errors where callers demonstrably need structured data. Do not proliferate variants for implementation details.

Ensure all new public types implement applicable standard traits (`Debug`, `Clone`/`Copy`, `Eq`, `Hash`, `Display` only when stable textual semantics exist) and are documented with examples.

## 5. Test requirements

Tests must establish:

- every accepted redundancy value constructs and round-trips;
- 0, 11, and boundary-overflow/runtime-invalid values return the same error in debug and release semantics of the recommended API;
- LSB/JPEG configs using the new validated value produce the same carrier bytes as equivalent valid legacy configs;
- zero seed remains explicitly valid;
- tile/search bounds retain current rejection behavior;
- new result getters/decomposition preserve output and capacity facts exactly;
- legacy report conversions are lossless for states they represent;
- generic results contain no application-only progressive/seed-only policy state;
- public doctests compile against both direct `stegoeggo-stego` and root convenience re-export where promised;
- no first-party production code calls deprecated infallible runtime configuration paths.

## 6. Acceptance criteria

- A validated shared redundancy type is the recommended configuration primitive.
- Recommended config APIs have identical invalid-input semantics in debug and release.
- Existing valid carrier bytes are unchanged by config migration.
- The recommended generic result surface is carrier-neutral and future-proofed with private state/getters or an equivalently semver-safe design.
- Existing 0.x public result/config APIs remain available according to the documented compatibility promise and are clearly classified as compatibility surfaces where superseded.
- `StegoError` docs match actual behavior, including zero-seed acceptance.
- No generic `Carrier` trait is introduced.
- `cargo test -p stegoeggo-stego`, public API tests, doctests, Clippy, and `./scripts/check.sh` pass.

## 7. Non-goals

No algorithm byte changes, no payload/frame version change, no authentication API, no broad error-framework rewrite, no typestate hierarchy, and no removal of deprecated APIs before the explicit breaking-version phase.
