# Stego Library Evolution Milestone 002 — Generic Carrier API Normalization

Status: blocked  
Repository baseline: `d5425d3c73e634fd2276d44507fa94728a18784f`  
Source roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`  
Long-term requirements: `plans/000-long-term-specification.md#2-canonical-api-invariants`  
Applicable ADRs: `ADR-0002`  
Primary class: capability

Blocker: M001 closure.

## 1. Objective

Make the generic carrier API unambiguous for new Rust consumers while preserving every
0.x stable entry point: explicit best-effort JPEG naming, semver-safe report accessors,
and documentation/examples that lead callers toward exact generic semantics.

## 2. Why this milestone is ready

The design is known, but execution waits for M001 so parent/application policy mapping
is clean before new generic names are frozen.

## 3. Current implementation evidence

- `jpeg::embed` and `embed_framed` are historical best-effort operations: they may
  lower redundancy and return seed-hint output with `embedded == false`.
- `jpeg::embed_strict` / `embed_framed_strict` are exact-or-error.
- `EmbedReport` and `InPlaceEmbedReport` expose public fields frozen through 0.x.
- `EmbedOutcome`/`EmbedPath`/`EmbedStatus` are documented as parent-owned
  compatibility vocabulary but physically remain in the generic crate.
- Cargo SemVer guidance confirms adding fields to all-public-field structs can break
  struct-literal consumers; evolution must use methods/new types before v1.

## 4. Invariants that must not regress

Existing function behavior and known-answer bytes are unchanged. No existing public
field is removed, renamed, made private, or supplemented with a new required field.
No `#[non_exhaustive]` retrofit on existing public-field structs in 0.x.

## 5. Scope

In: additive aliases/helpers/getters; docs and examples; explicit v1 disposition.

Out: removing `jpeg::embed`; moving application outcome types; changing defaults;
starting Phase 6.

## 6. Required production changes

- Add `jpeg::embed_best_effort` and `jpeg::embed_framed_best_effort` as explicit
  public names over the current best-effort behavior.
- Keep existing `embed`/`embed_framed` behavior byte-identical and documented as
  compatibility names. Do not add an attribute deprecation unless semver/downstream
  warning impact is explicitly accepted in the implementation review.
- Add getter methods to `EmbedReport` and `InPlaceEmbedReport` for all stable facts
  so v1 can privatize fields without inventing semantics later.
- Add small operation-level helpers only where they reduce caller branching (for
  example `is_embedded`); do not introduce a generic mega-trait.
- Update direct-consumer tests/examples to prefer strict exact operations or explicitly
  named best-effort operations.
- Record v1 mapping for the application outcome family; no physical move in 0.x.

## 7. Ordered work packages

WP1 additive names + parity tests.  
WP2 report accessors + direct consumer tests.  
WP3 rustdoc/README/examples normalization.  
WP4 semver evidence against the latest published carrier baseline.

## 8. Failure, cancellation, restart, contention semantics

Aliases must delegate directly; no second best-effort implementation. Exact operations
continue to fail without output on insufficient capacity.

## 9. Compatibility and migration

Existing users do nothing. New code uses explicit names. v1 removal/renaming remains
a separate breaking-boundary decision.

## 10. Required tests

Extend `stegoeggo-stego/tests/direct_consumer.rs`,
`tests/public_stego_api.rs`, doctests, and known-answer tests. Prove old and new
best-effort names return identical bytes/reports.

## 11. Required verification commands

```bash
cargo test -p stegoeggo-stego --all-features
cargo test --workspace --all-features --test public_stego_api
cargo test --workspace --all-features --test known_answer_vectors
./scripts/check.sh
cargo semver-checks check-release -p stegoeggo-stego
```

If the semver tool invocation differs in the installed version, record the exact command.

## 12. Documentation updates

`stegoeggo-stego/README.md`, `docs/carrier-crate.md`, `STABILITY.md`,
`DEPRECATIONS.md` only for disposition clarity, examples, architecture carrier docs.

## 13. Acceptance criteria

New consumers can choose exact versus best-effort semantics from function names alone;
old code remains source/behavior compatible; report facts are available through
methods; semver evidence is clean or every finding is classified.

## 14. Stop conditions

Stop if an additive alias cannot preserve exact old behavior, or if a proposed API
requires changing default semantics.

## 15. Closure evidence required

API diff, parity tests, semver-check output, package/docs test results, `check.sh`.

## 16. Handoff notes

Do not use this milestone as an excuse to publish v1 or remove compatibility vocabulary.
