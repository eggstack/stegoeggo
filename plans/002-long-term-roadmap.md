# StegoEggo Long-Term Roadmap

Status: execution roadmap for `plans/000-long-term-specification.md`.
Terminology: `plans/001-terminology-and-domain-model.md`.
Dependency-ordered, not calendar-ordered. Every phase leaves the repo
coherent with `scripts/check.sh` green.

## Phase 0 — Foundation (closed)

Canonical rights metadata, legal-metadata semantics, independent
conformance, policy/processing separation, provenance hardening.
Predecessor evidence: flat plans `001`–`024` and their status companions.

## Phase 1 — Container correctness (closed)

PNG/JPEG/WebP injection correctness, JPEG DCT containment, XMP/EXIF
shape, animated WebP, cross-format closure.
Predecessor evidence: flat plans `038`–`056`, `085`.

## Phase 2 — Generic stego carrier (closed)

Carrier extraction, public LSB/JPEG/framed/tiled/prepared APIs, crate
split, V1 semantics, validated `Redundancy`/`TileConfig` contracts.
Predecessor evidence: flat plans `057`–`080`, `090`–`097`.

## Phase 3 — API/CLI consolidation (closed)

Canonical-request convergence, verification-model convergence, module
decomposition, CLI surface simplification.
Predecessor evidence: flat plans `042`–`043`, `047`, `050`–`051`,
`061`–`062`, `081`–`084`, `087`–`088`, `098`.

## Phase 4 — Release and distribution (active, blocked)

Manual release pipeline, binary matrix + sidecars, installers, version and
eggfetch self-update, first eggfetch-enabled release qualification.
Predecessor evidence: flat plans `008`, `024`–`025`, `032`–`037`,
`086`, `089`, `099`–`105` (all closed). Remaining milestone:
`release-distribution` M001 — real eggfetch-to-eggfetch A→B transition
(flat Plan 106, blocked: no stable B exists yet; closes at the first
ordinary stable release after 0.4.2, never via a throwaway version).

## Phase 5 — v1 boundary (deferred)

Remove deprecated adapters (`ProtectionLevel`, `EvidenceProfile`,
`ProtectionContext` builders, `Protector`/`ProtectionPipeline` v1
disposition) per `DEPRECATIONS.md` and `plans/096-status.md`. Not in 0.x;
no implementation plan may start it without a maintainer decision.

## Cross-phase execution rules

Every phase MUST preserve `000` invariants, keep byte-vs-pixel and
output-domain routing semantics, add regression tests for defects
(`tests/robustness.rs` for fuzz findings), update `architecture/` with
code, and record closure evidence — never a bare "closed" commit.
