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

The remaining Phase 4 evidence gate does not block language-binding work:
bindings consume the already-stable library API and have independent
distribution qualification.

## Phase 5 — Language bindings (active)

Maintainer-directed extension accepted in
`plans/adrs/ADR-0005-foreign-language-bindings.md`.

Sequence:

1. Python binding foundation and distribution qualification.
2. Node.js binding using the same canonical request/verification semantics.
3. C ABI contract design and implementation after Python and Node have
   exercised the foreign-runtime boundary.

Bindings are leaf frontends over the canonical byte API; they do not
reimplement protection/container/carrier logic and do not use the future C
ABI as the implementation substrate for Python or Node. Binding dependencies
remain outside core/carrier/CLI dependency graphs. The root and carrier retain
`#![forbid(unsafe_code)]`; any C-ABI unsafe is confined to the future leaf
binding crate.

Operational sequencing and evidence:
`plans/subsystems/language-bindings-roadmap.md`.

## Phase 6 — v1 boundary (deferred)

Remove deprecated adapters (`ProtectionLevel`, `EvidenceProfile`,
`ProtectionContext` builders, `Protector`/`ProtectionPipeline` v1
disposition) per `DEPRECATIONS.md` and `plans/096-status.md`. Not in 0.x;
no implementation plan may start it without a maintainer decision.

## Cross-phase execution rules

Every phase MUST preserve `000` invariants, keep byte-vs-pixel and
output-domain routing semantics, add regression tests for defects
(`tests/robustness.rs` for fuzz findings), update `architecture/` with
code, and record closure evidence — never a bare "closed" commit.
