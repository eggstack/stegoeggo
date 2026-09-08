# Roadmap 081: Pre-v1 Consolidation, Maintainability, and Portability

Status: READY FOR IMPLEMENTATION

Audited baseline: `main` at `07bd05304a6942a843a76c28013a5bc0f08179cc`.

Predecessors: Roadmaps 057, 069, and 076 are treated as closed baseline work. This roadmap does not reopen the completed carrier-crate split, corrected LSB model, JPEG DCT/F5 implementation, framed/tiled carrier APIs, or output-domain routing work.

Implementation plans:

1. `plans/082-canonical-request-auxiliary-api-convergence.md`
2. `plans/083-verification-model-convergence.md`
3. `plans/084-large-module-decomposition-and-stego-adapter-deduplication.md`
4. `plans/085-container-observer-and-resource-accounting-reuse.md`
5. `plans/086-msrv-platform-and-scheduled-assurance.md`
6. `plans/087-cli-contract-and-dependency-consolidation.md`

Each implementation plan must use its `plans/NNN-status.md` companion as the authoritative execution ledger. Product-source changes are not considered complete until the status ledger records the exact test/evidence disposition.

---

## 1. Purpose

The steganography/carrier architecture is now in substantially good shape. `stegoeggo-stego` is a separate application-neutral crate; the root application consumes it; generic LSB/JPEG raw, framed, in-place, and tiled APIs exist; codec implementation structs remain private; output carrier selection follows final output representation; and the recent closure pass found no remaining carrier-boundary defect requiring a product patch.

The dominant remaining cost is above the carrier layer. The repository currently carries:

- a canonical `ProtectionRequest -> ResolvedProtectionPlan -> execution` model plus a fully supported pre-v1 `ProtectionLevel` / `ProtectionContext` / `EvidenceProfile` compatibility model;
- synchronous request-based APIs, while async and batch convenience APIs still primarily expose the deprecated configuration vocabulary;
- several overlapping verification result types with no single clearly enforced semantic source of truth;
- very large source files whose responsibilities are already separable but remain physically coupled;
- duplicate container traversal for resource accounting beside canonical metadata/container parsing;
- strong Linux-x86_64 local/CI checks, but no continuous proof of the documented MSRV or non-Linux portability;
- a CLI that simultaneously exposes legacy and current configuration vocabularies and enables a broader root feature/dependency set than is obviously needed for ordinary runtime use.

This roadmap addresses those residuals without adding new watermark algorithms, changing payload bytes, or expanding provenance scope.

---

## 2. Architectural end state

### 2.1 One canonical protection model

All new behavior must enter through:

```text
ProtectionRequest
    -> resolve_request(...)
    -> ResolvedProtectionPlan
    -> canonical execution
```

Legacy `ProtectionLevel` / `ProtectionContext` APIs remain functional through the promised 0.x compatibility window, but become translation-only wrappers. No new feature may be implemented first or only in the legacy vocabulary.

Async and parallel/batch convenience APIs must have request-based equivalents. Their implementations should invoke the same canonical synchronous request executor inside the appropriate execution wrapper (`spawn_blocking` / Rayon), rather than duplicating policy or plan construction.

### 2.2 One verification semantic core

`VerificationReport` should become the richest canonical in-memory verification result. Simpler stable APIs such as `VerificationStatus`, `VerificationResult`, and legal-notice summaries should be derived projections/adapters wherever compatibility requires them.

This does not require breaking 0.x APIs. It requires eliminating independent decision logic that can drift among result forms.

### 2.3 Bounded modules with explicit responsibility

Large modules should be decomposed by responsibility without changing public paths unnecessarily. Target areas include:

- `src/types.rs`
- `src/protected/metadata_trap.rs`
- `src/lib.rs` pipeline helpers
- `src/protected/steganography/{mod.rs,extract.rs,embed.rs}` residual adapters
- `src/webp_container.rs`
- `src/xmp.rs`
- `stegoeggo-cli/src/main.rs`

The goal is not an arbitrary line-count target. The goal is independently testable units, reduced merge conflict probability, and elimination of repeated translation/routing code.

### 2.4 One container traversal per purpose

Resource accounting must consume events/results from canonical PNG/JPEG/WebP parsing or shared bounded iterators. `src/lib.rs` should not maintain a second ad hoc structural walker with subtly different malformed-input behavior.

### 2.5 Claims match continuous evidence

If the project declares MSRV 1.87 and platform support, CI must continuously prove the promised subset. The required fast gate may remain intentionally compact, but MSRV and platform jobs should provide explicit evidence. Expensive external conformance and fuzzing may remain non-blocking while gaining scheduled execution.

### 2.6 CLI describes one modern contract

The CLI may retain legacy flags for 0.x compatibility, but request construction and precedence must converge to one canonical builder. Legacy flags should translate into canonical request fields and must not create an independent behavior path. Feature activation should be audited so the installed CLI only enables capabilities required by shipped commands.

---

## 3. Governing constraints

1. Preserve `#![forbid(unsafe_code)]` across the workspace.
2. Preserve payload v1/v2/v3 read compatibility and v3 write bytes.
3. Preserve generic carrier frame bytes, seed derivation, corrected LSB V2 mapping, JPEG F5 behavior, and carrier public API compatibility.
4. Preserve metadata semantics and canonical PLUS/XMP output unless a focused correctness defect is found.
5. Preserve the documented 0.x deprecation promise; do not remove deprecated public APIs before v1.0.0.
6. New APIs may be added in 0.x when additive and justified; avoid gratuitous public surface expansion.
7. Decomposition work must be behavior-preserving and should prefer private modules/re-exports over public path churn.
8. Do not add C2PA, new steganography algorithms, encryption, plugin frameworks, generic codec abstractions, or service daemons under this roadmap.
9. Do not make async codec work artificially asynchronous; CPU-bound work remains blocking work isolated from async executors.
10. Do not weaken resource limits, fuzz bounds, verification search coverage, or malformed-input behavior for simplification.
11. Keep required PR CI reasonably fast; scheduled/non-blocking assurance is acceptable for expensive checks.
12. Every plan must finish with `./scripts/check.sh` and focused regression tests relevant to its change set.

---

## 4. Sequencing

Recommended waves:

### Wave A — semantic convergence

- Plan 082: canonical request APIs for async/batch and compatibility-wrapper tightening.
- Plan 083: verification model convergence.

These can run in parallel if file ownership is kept separate: Plan 082 owns async/batch/request entry points; Plan 083 owns verification construction/projection.

### Wave B — structural simplification

- Plan 084: large-module decomposition and stego adapter deduplication.
- Plan 085: shared container traversal/resource-accounting observer.

Plan 084 should avoid moving the exact container walkers Plan 085 is changing until Plan 085 lands, or the two plans should use disjoint file ownership documented in their status ledgers.

### Wave C — tooling/frontend closure

- Plan 086: MSRV/platform/scheduled assurance.
- Plan 087: CLI contract/dependency consolidation and documentation reconciliation.

Plan 087 should run after Plan 082 so CLI request construction can target the final canonical API shape.

---

## 5. Roadmap completion criteria

Roadmap 081 is complete only when:

- request-based sync, async, and parallel/batch surfaces exist and legacy forms are translation-only wrappers;
- no new-policy decision logic remains unique to `ProtectionContext`-based helpers;
- canonical verification produces one rich report from which compatibility views are derived or explicitly documented exceptions are recorded;
- targeted large modules are decomposed without public behavior regressions;
- resource accounting reuses shared bounded container traversal rather than parallel parsers in `lib.rs`;
- CI explicitly checks Rust 1.87 and at least Linux x86_64, macOS aarch64, and Windows x86_64 compatibility at an appropriate depth;
- fuzz and external-conformance workflows have scheduled non-blocking execution or a documented evidence-based reason not to schedule them;
- CLI request construction has one canonical path, legacy/current precedence is regression-tested, and unnecessary root feature/dependency activation is removed;
- `README.md`, `docs/rust-api.md`, `docs/cli-usage.md`, `SUPPORT.md`, `STABILITY.md`, `DEPRECATIONS.md`, architecture docs, and `CHANGELOG.md` describe the resulting surface accurately;
- `./scripts/check.sh` passes on the final integrated branch.

## 6. Explicitly deferred

C2PA integration remains deferred under `architecture/adr-c2pa.md`. Additional media formats, novel watermark algorithms, GPU acceleration, network services, and v1 removal of deprecated APIs are separate future decisions. This roadmap prepares a clean v1 boundary but does not perform the semver-major removal itself.
