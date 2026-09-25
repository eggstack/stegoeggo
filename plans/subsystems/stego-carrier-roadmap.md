# Stego Carrier Roadmap

Status: closed

Long-term references:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/001-terminology-and-domain-model.md#marker-channel`
- `plans/002-long-term-roadmap.md#phase-2--generic-stego-carrier`

Related ADRs:

- `plans/adrs/ADR-0002-output-domain-carrier-routing.md`

## 1. Purpose and ownership boundary

Owns the redundant marker channel end to end: generic carrier crate
(`stegoeggo-stego`: `lsb`, `jpeg`, `frame`, `pixels`, `prepared`,
`types`, `error`, `constants`), V3/V2/V1 payload handling, application
adapter (`src/protected/steganography/` five modules), and pipeline
routing (`src/pipeline.rs`). Carrier knows nothing about rights.

## 2. Work classification

### Invariants

- Output-domain routing; single-decode JPEG extraction; one tiled-LSB
  in-place core; validated `Redundancy` (`1..=10`); `Redundancy`/`TileConfig`
  debug/release-identical semantics.
- V3 written; V1/V2 extract-only; framed CRC32 is corruption detection,
  not authentication.

### Capabilities

- Raw, strict, in-place, borrowed-view, prepared-JPEG, framed, and tiled
  operations over LSB and JPEG carriers.

### Infrastructure

- `frame` TLV, `PreparedJpeg`, `PixelView{,Mut}`, hidden
  `application-support` bridge (parent-only compat).

### Polish

- Ergonomics hardening, allocation optimization, public-boundary cleanup.

## 3. Non-goals

Rights policy, container metadata shape, CLI surface, release logistics.

## 4. Current state

Closed at 0.4.2. Public API frozen per `STABILITY.md` (report fields
frozen through 0.x; v1 moves to getters). `jpeg::embed` best-effort is
explicit application policy; `*_strict` exact-or-fail. Evidence:
`tests/public_stego_api.rs`, `tests/payload_v3_roundtrip.rs`,
`tests/output_domain_routing.rs`, `tests/known_answer_vectors.rs`,
`examples/generic_stego.rs`.

## 5. Target architecture

No change through 0.x; v1 boundary work is deferred to Phase 5
(private fields + getters, `Protector` disposition).

## 6. Dependency graph

Container-correctness M1 (interface: JPEG segment contract, stable) →
carrier milestones. Pipeline convergence (`061`) bridges to
api-cli-contract (soft). All closed.

## 7. Milestones

- M1 library/pipeline simplification + LSB correctness — flat `057`,
  `058`. Class: infrastructure.
- M2 generic core extraction + runtime/allocation optimization — flat
  `059`, `060`. Class: infrastructure.
- M3 pipeline convergence + public framing API — flat `061`, `062`.
  Class: capability.
- M4 crate split + architecture evidence + post-split correctives —
  flat `063`, `064`, `065`, `066`. Class: infrastructure.
- M5 tiled JPEG integrity + single-decode search — flat `067`, `068`.
  Class: invariant.
- M6 ergonomics/adapters/framed/pipeline closure — flat `069`–`080`.
  Class: polish/capability.
- M7 v1 semantics/transactional/validated-config/proofs/prepared/views/
  boundary/packaging — flat `090`–`097`. Class: invariant/capability.

## 8. Cross-cutting requirements

Compat: V1/V2 extraction retained; `application_support` 18 symbols per
`096-status.md`. Security: capacity pre-checks, `InsufficientCapacity`
without partial output on strict paths. Docs: `docs/carrier-crate.md`,
`architecture/carrier-*.md`.

## 9. Verification strategy

Public-API tests (framed without retained length; JPEG downgraded
redundancy covered), known-answer vectors, soak tests, fuzz regression
(`tests/robustness.rs`), benches for allocation claims.

## 10. Risks and decision points

None open. Two-RNG non-interchange and capacity-unit definitions are
recorded gotchas, not decisions.

## 11. Completion definition

Met: stable generic surface per `STABILITY.md`, dogfooded by the parent
pipeline, with v1 dispositions recorded (`096-status.md`).

## 12. Milestone status table

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M1 simplification/LSB | closed | flat `057`/`058` | `058-status.md` | — |
| M2 core/perf | closed | flat `059`/`060` | `059-status.md`, `060-status.md` | — |
| M3 convergence/framing | closed | flat `061`/`062` | `061-status.md`, `062-status.md` | — |
| M4 split/evidence | closed | flat `063`–`066` | `063-status.md`–`066-status.md` | — |
| M5 tiled/single-decode | closed | flat `067`/`068` | `067-status.md`, `068-status.md` | — |
| M6 ergonomics/pipeline | closed | flat `069`–`080` | `070-status.md`–`080-status.md` (as present) | — |
| M7 v1 semantics/packaging | closed | flat `090`–`097` | `090-status.md`–`097-status.md` (as present) | — |
