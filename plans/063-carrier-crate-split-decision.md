# Plan 063: Carrier Crate Split Decision

Status: Ready for implementation

Roadmap: `plans/057-stego-carrier-library-and-pipeline-simplification-roadmap.md`

Depends on: Plans 058-062 complete.

Audited planning baseline: `main` at `e60b801fda493ddd5e744f5de2a12d7b9489c078`

Authoritative implementation ledger to create before source edits: `plans/063-status.md`

---

## 1. Purpose

Decide from the now-real public carrier API whether the generic stego layer should remain a module in the `stegoeggo` crate or become a small dedicated workspace crate.

This is deliberately a decision gate rather than a predetermined crate split. The line of work is successful if the generic API is usable and maintainable inside `stegoeggo`; a new package is justified only when it materially reduces dependency/consumer cost and keeps the architecture simpler overall.

Valid final dispositions are:

```text
SPLIT
NO-SPLIT
PARTIAL / BLOCKED
```

`NO-SPLIT` is not a failure.

---

## 2. Questions this plan must answer

The implementer must answer with measurements/source inspection rather than preference:

1. What direct dependencies does a consumer of only `stegoeggo::stego` currently compile because the generic carrier shares the root crate?
2. Which of those dependencies are actually required by LSB/JPEG carrier and generic-frame functionality?
3. Would moving the carrier into a child crate create a one-way dependency `stegoeggo -> carrier`, or would application types leak back and create a cycle?
4. Can the current public API be re-exported unchanged from the root crate if a split occurs?
5. How much workspace/release complexity would a second published package create?
6. Does the project already have enough package-management complexity that a second crate would be disproportionate?
7. Is there a concrete consumer benefit such as materially smaller dependency tree, compilation surface, or ability to use the stego carrier without serde/XML/provenance dependencies?
8. Does a split reduce or increase code duplication?

---

## 3. Phase 0 — create the evidence ledger

Before moving any source:

1. Create `plans/063-status.md`.
2. Record the actual baseline SHA.
3. Record the complete public API under `stegoeggo::stego` from Plan 062.
4. Record the source files that implement it.
5. Run `./scripts/check.sh` before any split work.

The status ledger must include a decision table with at least:

```text
Criterion
Observed current state
Benefit of split
Cost/risk of split
Disposition
Evidence/command
```

Do not move files until the decision threshold in Phase 3 is evaluated.

---

## 4. Phase 1 — dependency-surface measurement

Record the root crate dependency tree relevant to a default/no-default-feature consumer.

Permitted commands include:

```bash
cargo tree -p stegoeggo --no-default-features
cargo tree -p stegoeggo --no-default-features --edges normal
cargo metadata --no-deps --format-version 1
```

If useful, create a temporary local consumer outside the committed tree that depends on the current crate and imports only `stegoeggo::stego`. Record build/dependency observations in the ledger; do not commit generated target directories or a benchmark project.

Classify direct dependencies as:

```text
carrier-required
frame-required
StegoEggo-application-only
feature-gated/irrelevant to default generic consumer
```

Expected carrier candidates may include the image/JPEG machinery and a small error/checksum dependency set. Do not assume the exact list; inspect the post-Plan-062 source.

A split has real dependency value only if it excludes a meaningful set of application-only dependencies from a generic consumer.

---

## 5. Phase 2 — source-boundary audit

Inspect every public generic type/function and ensure it can move without importing application types.

A split is blocked if `src/stego/` still requires any of:

```text
ProtectionRequest
ResolvedProtectionPlan
ProtectionContext
RightsPolicy
DmiValue
LegalMetadata
StegoPayload
payload_v3 types
notice verification types
```

Shared low-level JPEG code may move with the carrier or remain in a private support module only if the dependency direction stays acyclic.

Preferred split layout if approved:

```text
stegoeggo-stego/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── lsb.rs
    ├── jpeg.rs
    ├── frame.rs
    └── jpeg_transcoder/ ...

stegoeggo (root crate)
    -> dependency on stegoeggo-stego
    -> re-export `stegoeggo_stego` API under `stegoeggo::stego`
    -> retains rights/provenance/metadata/application adapters
```

Exact package naming is subject to crates.io/name availability only when release planning happens later. This plan does not publish or reserve a name remotely.

If a separate package would force root application types into the carrier crate, record `NO-SPLIT` until the boundary is cleaner rather than creating abstraction traits to evade the cycle.

---

## 6. Phase 3 — explicit split threshold

Proceed with a split only if all mandatory criteria are true:

1. The carrier public API is already stable enough that moving it does not require redesign.
2. Dependency direction is strictly one-way from `stegoeggo` to the carrier crate.
3. No rights/provenance/application type is needed by the carrier crate.
4. Root re-exports can preserve the Plan 062 public API with no or minimal source breakage.
5. The dedicated crate removes at least several application-only direct dependencies from a carrier-only consumer, or otherwise demonstrates a comparably concrete build/dependency benefit.
6. The split does not require adding a release workflow, CI matrix, or automated publication orchestration.
7. Workspace verification remains manageable with the existing `scripts/check.sh` model.
8. The carrier crate has a coherent independent description: arbitrary-payload image steganography carriers, not a miscellaneous internal-support crate.

Use `NO-SPLIT` if any of the following dominate:

- only one or two trivial dependencies would be avoided;
- the carrier still shares many root-only types;
- re-exporting would produce awkward duplicate error/config types;
- a second package adds more publication/versioning complexity than the consumer benefit warrants;
- generic consumers are already adequately served by `stegoeggo::stego` with `default-features = false`.

The status ledger must state the decision before implementation begins.

---

## 7. Phase 4A — NO-SPLIT path

If the decision is `NO-SPLIT`:

1. Do not create a placeholder workspace crate.
2. Keep the Plan 062 API and module boundary intact.
3. If simple feature/dependency adjustments can reduce generic-consumer cost without API churn, they may be made only when they do not complicate the root crate.
4. Record the measured dependency tree and why a split was not justified.
5. Add no special CI or release mechanism.
6. Mark the implementation portion of this plan complete after focused/public API tests and `./scripts/check.sh` pass.

Do not treat “maybe later” as a blocker. The project can revisit the decision if the carrier grows materially.

---

## 8. Phase 4B — SPLIT path

If the decision is `SPLIT`, create one small workspace member containing only generic carrier functionality.

### 8.1 Workspace/package constraints

- Add exactly one carrier crate.
- Do not split LSB and JPEG into separate crates.
- Do not create a shared “core” crate unless a cycle proves it absolutely necessary; if a third crate becomes necessary, stop and reassess because the split is likely over-engineered.
- Preserve MSRV 1.87 unless separately approved.
- Preserve `#![forbid(unsafe_code)]`.
- Use minimal dependency features.

### 8.2 Root compatibility

The root `stegoeggo` crate should re-export the generic API so existing Plan 062 examples remain valid, preferably:

```rust
pub mod stego {
    pub use stegoeggo_stego::*;
}
```

or an equivalent namespace-preserving re-export.

The root application adapter must depend on the carrier crate rather than retaining copied carrier source.

### 8.3 Error/result compatibility

Avoid two incompatible parallel public error systems.

Preferred approaches:

- carrier crate owns `StegoError`; root converts it into root `Error` for application APIs and re-exports `StegoError` under `stego`; or
- carrier API uses a mechanical error type already designed in Plan 062 that moves unchanged.

Do not make the carrier depend on root `Error`, because that reverses the dependency direction.

### 8.4 Tests

Move raw carrier tests with the carrier crate. Keep root integration tests proving:

- root re-export works;
- rights/provenance application uses the carrier;
- legacy LSB extraction remains working through the root adapter;
- JPEG container behavior remains identical.

### 8.5 Package/release restraint

Do not:

- publish the new crate;
- bump the root version;
- create tags/releases;
- add automated multi-crate publication;
- add crates.io credentials/workflows.

Record that future manual release ordering would need to publish the carrier before the root crate if/when a release is separately authorized.

---

## 9. Phase 5 — optional footprint measurement

If practical, record before/after:

- number of direct normal dependencies for carrier-only package/consumer;
- transitive package count from `cargo tree`;
- release binary size using existing `scripts/measure_binary_size.sh` where relevant;
- clean compile wall time only as a rough observation, not an acceptance gate.

Do not overfit to noisy compile timing.

The main evidence should be structural dependency removal and architecture simplicity.

---

## 10. Acceptance criteria

Plan 063 is complete when either the `NO-SPLIT` or `SPLIT` criteria are satisfied.

### Common criteria

1. `plans/063-status.md` records a measured decision, not a preference.
2. The dependency tree and source boundary are audited after Plan 062.
3. The decision explicitly evaluates consumer benefit versus workspace/release complexity.
4. No application/right-policy type is introduced into the generic carrier boundary.
5. The Plan 062 public generic API remains usable.
6. No release, version bump, tag, publication, CI matrix, or automated publication workflow is added.
7. `./scripts/check.sh` passes.

### `NO-SPLIT` criteria

8. The ledger states why a second crate would not materially improve dependency/consumer cost or would create disproportionate complexity.
9. No empty/placeholder carrier crate is committed.
10. The internal module boundary remains clean and documented.

### `SPLIT` criteria

8. Exactly one new carrier workspace crate exists.
9. Dependency direction is `stegoeggo -> carrier`; no cycle exists.
10. Carrier crate source contains no rights/provenance/application dependencies.
11. Root `stegoeggo::stego` API is preserved by re-export or an equivalently compatible facade.
12. Root rights/provenance processing uses the new carrier crate rather than copied carrier implementations.
13. Generic carrier-only dependency surface is demonstrably smaller in the ledger.
14. Existing LSB/JPEG/public API/application compatibility tests pass in both carrier and root crates.
15. Future manual publication ordering is documented but not executed.

---

## 11. Stop conditions

Stop and choose `NO-SPLIT` rather than escalating architecture if the split requires:

- a third workspace crate;
- new abstraction traits solely to break cycles;
- public JPEG internals;
- duplicated carrier errors/configs;
- significant API churn immediately after Plan 062;
- expanded CI/release machinery.

The objective is lower consumer and maintenance cost, not crate-count purity.