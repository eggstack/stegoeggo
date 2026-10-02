# Stego Library Evolution Milestone 002 — Closure Status

Status: closed
Source implementation plan: `plans/implementation/stego-library-evolution/002-generic-carrier-api-normalization.md`
Source subsystem roadmap: `plans/subsystems/stego-library-evolution-roadmap.md#7-milestones`
Repository baseline reviewed: `beb4145` (M002 implementation; plan baseline `d5425d3` plus M001 implementation `ed1dd6d`, M001 closure `9c304bc`, and this milestone's additive carrier API work)
Implementation commits: `beb4145` — stego: normalize generic carrier API with explicit best-effort names and report accessors
Closing commit: this record plus roadmap/registry reconciliation (see git log for `plans: close stego-library-evolution M002 generic carrier API normalization`).

## 1. Executive finding

The generic carrier API is unambiguous for new Rust consumers with zero
behavior change: `jpeg::embed_best_effort` and
`jpeg::embed_framed_best_effort` are explicit public names delegating
directly to the historical best-effort behavior, `jpeg::embed` /
`jpeg::embed_framed` remain byte-identical compatibility names with no
attribute deprecation, and every stable report fact is available through
semver-safe accessor methods so v1 can privatize fields without inventing
semantics. Direct-consumer tests, public API tests, doctests, and
known-answer vectors prove old/new parity; semver evidence is clean;
`./scripts/check.sh` is green. M002 closes and unblocks M003.

## 2. Requirement-to-evidence matrix

| Plan requirement (§6/§13) | Evidence |
|---|---|
| Explicit public best-effort names over current behavior (§6.1) | `stegoeggo-stego/src/jpeg.rs`: `embed_best_effort` delegates to `embed`; `embed_framed_best_effort` delegates to `embed_framed`; no second implementation (§3) |
| Keep `embed`/`embed_framed` byte-identical compatibility names, no attribute deprecation without accepted review (§6.2) | Original bodies untouched; docs reworded to compatibility names; repo-wide grep shows no `#[deprecated]` in `stegoeggo-stego/src/`; parity tests assert identical bytes/reports (§3, §4) |
| Getters on `EmbedReport` and `InPlaceEmbedReport` for all stable facts (§6.3) | `EmbedReport`: `is_embedded`/`embedded`/`output`/`payload_bytes`/`required_capacity`/`available_capacity`/`actual_redundancy` plus existing `capacity`/`into_output`/`into_parts`; `InPlaceEmbedReport`: same fact coverage plus `capacity()` (§3) |
| Small operation-level helpers only, no generic mega-trait (§6.4) | Only `is_embedded()` branching helpers on both report types; no new traits (§3) |
| Direct-consumer tests/examples prefer strict or explicitly named best-effort (§6.5) | `direct_consumer.rs` uses `embed_strict`/`embed_framed_strict` for exact cases and `embed_best_effort`/`embed_framed_best_effort` for best-effort cases; `public_stego_api.rs` migrated the same way; `examples/generic_stego.rs` demonstrates explicit names (§3, §4) |
| v1 mapping for application outcome family, no physical move (§6.6) | `EmbedOutcome`/`EmbedStatus`/`EmbedPath`/`EmbedOutcomeSummary` stay in place; rustdoc + `STABILITY.md` + `DEPRECATIONS.md` record v1 removal from the recommended generic surface with the parent keeping its own vocabulary (§3, §9) |
| New consumers choose exact vs best-effort from names alone (§13) | Module docs, README, `docs/carrier-crate.md`, and architecture docs lead with `embed_best_effort` vs `embed_strict` (§9) |
| Old code source/behavior compatible (§13) | No public field removed/renamed/privatized; no `#[non_exhaustive]` retrofit; semver-checks clean (§4, §7) |
| Report facts available through methods (§13) | Accessor coverage tests in both consumer suites (§4) |
| Semver evidence clean or classified (§13) | `cargo semver-checks check-release -p stegoeggo-stego`: 196 checks pass, 58 skip, no semver update required (§4) |

## 3. Production implementation evidence

Implementation `beb4145` (12 files):

- `stegoeggo-stego/src/jpeg.rs`: `embed_best_effort` / `embed_framed_best_effort` additive aliases with direct delegation; `embed` / `embed_framed` docs reworded to compatibility names; module docs distinguish explicit vs compat vs strict.
- `stegoeggo-stego/src/lib.rs`: seven additive `EmbedReport<T>` accessors; report docs name the getter set for the v1 privatization.
- `stegoeggo-stego/src/types.rs`: new `InPlaceEmbedReport` accessor impl (`is_embedded`/`embedded`/`payload_bytes`/`required_capacity`/`available_capacity`/`actual_redundancy`/`capacity`) with frozen-fields v1 note.
- Tests: `stegoeggo-stego/tests/direct_consumer.rs` gains `direct_consumer_best_effort_alias_parity` (fitting, downgraded-redundancy, and seed-only parity) plus `direct_consumer_report_accessors_cover_stable_facts`; `tests/public_stego_api.rs` gains `public_jpeg_best_effort_alias_parity` plus `public_report_accessors_cover_stable_facts` and migrates six existing JPEG cases to strict/explicit names.
- Example: `examples/generic_stego.rs` uses `embed_best_effort` / `embed_framed_best_effort` for the best-effort demonstrations.
- Docs: `stegoeggo-stego/README.md` (explicit vs compat naming, API summary), `docs/carrier-crate.md` (strict section + usage), `STABILITY.md` (carrier surface + v1 getter disposition), `DEPRECATIONS.md` (compat row names explicit best-effort aliases), `architecture/carrier-jpeg.md` + `architecture/carrier-surface.md` (function tables + v1 mapping).

No default-semantics change: alias bodies are single-expression delegations; strict paths untouched; tiled/prepared/frame paths untouched.

## 4. Verification executed (exact commands + results)

- `cargo test -p stegoeggo-stego --all-features` → 192 lib + 10 direct-consumer + 35 doctests passed, 0 failed (includes the two new alias/accessor tests and the two new doctests on the alias functions).
- `cargo test --workspace --all-features --test public_stego_api` → 55 passed, 0 failed (includes the new parity/accessor tests; six migrated cases green on strict/explicit names).
- `cargo test --workspace --all-features --test known_answer_vectors` → 10 passed, 0 failed (no known-answer byte change).
- `cargo semver-checks check-release -p stegoeggo-stego` (cargo-semver-checks 0.50.0) → 196 checks pass, 58 skip, summary no semver update required (additive-only evolution).
- `./scripts/check.sh` → exit 0 (fmt, clippy `-D warnings`, no-default-features check, 2045 passed / 0 failed / 25 ignored across 45 suites, docs contract valid: 5 release targets).

## 5. Invariant review

- Existing function behavior and known-answer bytes unchanged: parity tests assert identical `output` bytes and all report facts across fitting, downgraded, and seed-only inputs; known-answer suite green.
- No public field removed, renamed, made private, or supplemented with a new required field: only methods added; struct literals still compile (workspace suite green).
- No `#[non_exhaustive]` retrofit on existing public-field structs: none added.
- `#![forbid(unsafe_code)]` retained; rustfmt 4-space/100-col clean; clippy `-D warnings` clean.

## 6. Failure and recovery review

Aliases delegate directly with no second best-effort implementation to diverge. Exact operations continue to fail without output on insufficient capacity (`InsufficientCapacity`, no partial bytes); best-effort paths preserve the established downgrade then seed-hint-only fallback. No new failure mode was introduced; the downgrade and seed-only parity cases are exercised by both consumer suites.

## 7. Migration and compatibility review

Existing users do nothing: `embed` / `embed_framed` keep behavior and signatures. New code uses `embed_best_effort` / `embed_framed_best_effort` for explicit best-effort or `embed_strict` / `embed_framed_strict` for exact redundancy. Report field access still compiles; method access is available for forward-compatibility toward the v1 privatization. No deprecation warnings were introduced, so no downstream warning impact.

## 8. Security review

No secret handling changed (seed remains `u64`, never logged). No parser, capacity-unit, or redundancy-range change. Accessors expose only already-public facts; no new data leaves the report types.

## 9. Documentation and operations

`stegoeggo-stego/README.md`, `docs/carrier-crate.md`, `STABILITY.md`, `DEPRECATIONS.md` (disposition clarity only), `examples/generic_stego.rs`, and `architecture/carrier-jpeg.md` + `architecture/carrier-surface.md` now lead new callers toward strict or explicitly named best-effort operations and record the application-outcome v1 mapping. No operator action required.

## 10. Unresolved findings (critical/high/medium/low)

None.

## 11. Roadmap disposition

Stego-library-evolution M002 closed. M003 (carrier resource/prepared
hardening) is unblocked: its hard M002 dependency is satisfied with the
explicit best-effort names frozen and the report getter vocabulary in
place, so it promotes to ready. M004 remains hard-blocked behind M003;
M005 remains interface-blocked behind the M003 limits contract; M006
remains blocked behind ADR-0007 acceptance plus M002/M003 closure (M002
now satisfied, M003 still outstanding).

## 12. Registry updates

`registry.md`: M002 row removed from dependency-ready plans; stego M003
promoted blocked→ready with its plan linked; subsystem current milestone
advanced to M003; M002 recorded under recently closed work; blocked-work
M003 row removed. Roadmap status table marks M002 closed with this
closure record and M003 ready.
