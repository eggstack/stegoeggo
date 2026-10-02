# Stego Library Evolution Milestone 001 — Boundary and Metadata Convergence

Status: ready for handoff  
Repository baseline: `d5425d3c73e634fd2276d44507fa94728a18784f`  
Source roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`  
Long-term requirements: `plans/000-long-term-specification.md#2-canonical-api-invariants`, `#3-execution-invariants`  
Applicable ADRs: `ADR-0001`, `ADR-0002`, `ADR-0003`  
Primary class: infrastructure

## 1. Objective

Remove the remaining internal legacy-context dependency from canonical metadata execution
and collapse duplicated metadata write dispatch into one private resolved operation,
without changing emitted bytes, rights semantics, public API, or 0.x compatibility.

## 2. Why this milestone is ready

All hard dependencies are closed. `ProtectionRequest` and
`ResolvedProtectionPlan` are canonical; Plans 096/097 explicitly identified the
temporary `ProtectionContext` reconstruction inside `RightsMetadataProtector` as a
v1-boundary cleanup candidate. This plan performs the non-breaking internal portion now.

## 3. Current implementation evidence

At baseline:

- `src/protected/metadata_trap.rs::inject_bytes` and
  `inject_bytes_from_plan` each implement format/update-policy dispatch.
- The plan-based path constructs `limits_ctx = ProtectionContext::new(...)` solely
  because JPEG metadata helpers/rendering still accept legacy context-shaped inputs.
- `src/protected/steganography/` otherwise delegates ordinary current carrier
  operations into `stegoeggo-stego`.
- `src/util/image.rs::PixelSelectionRng` is marked dead-code allowed and must be
  audited against current/legacy call paths rather than assumed necessary.
- `stegoeggo-stego::application_support` remains hidden and intentionally supports
  legacy/application-specific search behavior; Plan 096 counted 18 symbols.

## 4. Invariants that must not regress

- Canonical metadata bytes/semantics for PNG/JPEG/WebP remain equivalent.
- Existing metadata update policies remain exact:
  `FailOnConflict`, `PreserveExisting`, `ReplaceStegoOwned`.
- Byte APIs remain the only path claiming container metadata.
- Legacy `ProtectionContext` callers remain functional through 0.x.
- No change to V1/V2/V3 payload bytes or carrier selection.
- No public visibility increase for private container/XMP/JPEG internals.
- Root and carrier retain `#![forbid(unsafe_code)]`.

## 5. Scope

In:

- Introduce one private resolved metadata-write input, e.g. `MetadataWriteSpec`,
  carrying only the explicit fields format writers require.
- Translate legacy context and canonical plan into that input exactly once.
- Route conflict detection, preservation filtering, stripping, and format dispatch
  through one implementation.
- Refactor JPEG helpers so they consume explicit resolved fields instead of
  `ProtectionContext`.
- Audit and remove genuinely unused root stego helper/RNG code only when no
  compatibility path references it.
- Re-audit the hidden `application-support` bridge and record symbol count/reasons.

Out:

- Removing or deprecating public legacy APIs.
- Changing metadata serialization format.
- Moving codec internals into public carrier API.
- Reducing hidden support by reimplementing legacy verification elsewhere.

## 6. Required production changes

1. Add a private resolved metadata-write representation under
   `src/protected/metadata_trap/`; fields should include only effective notice/DMI,
   output format, update policy, resource limits, and the rendering parameters actually
   required by the format modules.
2. Convert `inject_bytes_from_plan` directly from `ResolvedProtectionPlan`.
3. Convert legacy `inject_bytes` through a compatibility translator and then call the
   same internal executor.
4. Change JPEG metadata helper signatures to explicit values/spec references; remove
   canonical-path construction of `ProtectionContext`.
5. Deduplicate update-policy and format dispatch. Do not duplicate a second executor in
   `png.rs`, `jpeg.rs`, or `webp.rs`.
6. Remove `PixelSelectionRng` only if grep/tests prove no retained compatibility or
   known-answer dependency. Otherwise move/document it at its true owner.
7. Audit `application_support`; remove only exact duplicates now representable by
   stable generic APIs. Any application-specific candidate classification stays hidden.

## 7. Ordered work packages

WP1: capture baseline metadata fixtures and call graph. Acceptance: focused tests show
current output hashes/semantic fields for representative PNG/JPEG/WebP cases.

WP2: introduce resolved metadata-write spec and canonical executor. Acceptance: plan
path contains no `ProtectionContext::new` construction.

WP3: route legacy adapter through the same executor. Acceptance: request/legacy
equivalence suites remain green.

WP4: dead-code/hidden-bridge audit. Acceptance: every retained support symbol has a
named consumer; removed code has a regression test or grep guard where appropriate.

WP5: docs/architecture reconciliation. Acceptance: overview no longer describes
legacy-context construction as required architecture.

## 8. Failure, cancellation, restart, contention semantics

Metadata writes remain all-or-error `Vec<u8>` transformations. No partial output may
escape on parse/conflict/render failure. Refactor must preserve current bounded parsing.
If byte identity differs unexpectedly, stop and classify the difference before updating
fixtures.

## 9. Compatibility and migration

No public migration. This is internal convergence. Public legacy adapters keep their
current behavior. Do not add new fields/methods to legacy context to make the refactor
easier.

## 10. Required tests

Run/add focused coverage for:

- `tests/request_api.rs`
- `tests/cross_format_semantics.rs`
- `tests/preservation.rs`
- `tests/preservation_idempotence.rs`
- `tests/merge_policy.rs`
- `tests/canonical_rights.rs`
- JPEG structured COM/XMP/EXIF paths and existing conflict-policy unit tests
- legacy context/request equivalence
- a regression asserting the canonical plan path does not require legacy-only state

## 11. Required verification commands

```bash
cargo test --workspace --all-features --test request_api
cargo test --workspace --all-features --test cross_format_semantics
cargo test --workspace --all-features --test preservation
cargo test --workspace --all-features --test preservation_idempotence
cargo test --workspace --all-features --test merge_policy
cargo test --workspace --all-features --test canonical_rights
./scripts/check.sh
```

Run `./scripts/verify_metadata_conformance.sh --strict` when ExifTool/xmllint are
available; closure must state whether it ran rather than implying it did.

## 12. Documentation updates

Update `architecture/overview.md`, the relevant metadata architecture deep dive,
`docs/rust-api.md` only if internal wording references context reconstruction, and the
stego-library roadmap status.

## 13. Acceptance criteria

- Canonical plan metadata execution constructs no `ProtectionContext`.
- One private executor owns update-policy + format dispatch.
- Existing public API and encoded metadata semantics remain compatible.
- Hidden support has a current counted inventory and no unexplained duplicates.
- `./scripts/check.sh` passes.

## 14. Stop conditions

Stop rather than improvise if eliminating context requires changing public metadata
bytes, a stable function signature, legacy extraction behavior, or an accepted ADR.

## 15. Closure evidence required

Record implementation SHA, before/after call graph, hidden-support symbol inventory,
focused test counts/results, check.sh result, and any conformance run.

## 16. Handoff notes

Prefer a small private data structure over threading a long scalar argument list.
Do not expose the resolved write spec publicly; it is an internal execution contract.
