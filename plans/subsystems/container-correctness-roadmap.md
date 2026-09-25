# Container Correctness Roadmap

Status: closed

Long-term references:

- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/001-terminology-and-domain-model.md#metadata-channel`
- `plans/002-long-term-roadmap.md#phase-1--container-correctness`

Related ADRs:

- `plans/adrs/ADR-0002-output-domain-carrier-routing.md`

## 1. Purpose and ownership boundary

Owns container integrity: PNG chunk handling, JPEG segment-preserving
encode (DQT/SOS-only mutation), WebP RIFF/VP8X rebuild, XMP/EXIF shape,
animated WebP. Owns `src/webp_container.rs`, `src/container_walk.rs`,
format modules under `metadata_trap/`, carrier `jpeg_transcoder/`
container behavior. Does not own policy or payload semantics.

## 2. Work classification

### Invariants

- JPEG preserving encode; VP8X reserved-bit normalization; bounded
  container walks with `checked_add`, lenient break on overflow.
- No hand-written chunk/marker loops outside owning modules.

### Capabilities

- Metadata-preserving, spec-compliant output across PNG/JPEG/WebP
  including animated WebP.

### Infrastructure

- `observe_container_work` as the sole accounting walker.

### Polish

- Reference/serialization final closures.

## 3. Non-goals

Policy model, carrier algorithms, detached manifests.

## 4. Current state

Closed at 0.4.2. JPEG fast path preserves APP/COM/unknown segments;
WebP rebuilds VP8X from derived features; animated WebP and XMP
serialization closed. Evidence: `tests/jpeg_container_preservation.rs`,
`tests/preservation_idempotence.rs`, `tests/cross_format_closure.rs`,
`tests/container_accounting.rs`.

## 5. Target architecture

No change: strict injection walkers per format module, lenient
accounting walker shared.

## 6. Dependency graph

Rights-metadata M1–M2 (interface: notice rendering contract, stable)
→ container milestones (hard). All closed.

## 7. Milestones

- M1 JPEG DCT containment — flat `040`. Class: invariant.
- M2 WebP XMP/EXIF correctness — flat `041`. Class: invariant.
- M3 cross-format + corrective closure chain — flat `044`, `045`, `048`,
  `049`, `052`. Class: invariant.
- M4 animated WebP / JPEG exactness / XMP reference — flat `053`, `054`,
  `055`, `056`. Class: capability.
- M5 resource-accounting reuse — flat `085`. Class: infrastructure.

## 8. Cross-cutting requirements

DoS: bounded walks, `resource_limits.rs`. Compat: malformed-input
normalization documented (VP8X reserved bits). Docs:
`architecture/jpeg-*.md`, `architecture/protected-*.md`,
`architecture/webp-container.md`.

## 9. Verification strategy

Container/preservation/idempotence suites + specialist external
conformance where shape claims apply.

## 10. Risks and decision points

None open. WebP-has-no-JPEG-equivalent-preserving-path is accepted and
documented in `architecture/overview.md`.

## 11. Completion definition

Met: spec-compliant output, preservation evidence, single accounting
owner.

## 12. Milestone status table

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M1 JPEG containment | closed | flat `040` | `040-status.md` | — |
| M2 WebP correctness | closed | flat `041` | `041-status.md` | — |
| M3 corrective chain | closed | flat `048`/`049`/`052` | `048-status.md`, `049-status.md`, `052-status.md` | — |
| M4 animated/exactness | closed | flat `053`/`054`/`055`/`056` | `053-status.md`–`056-status.md` | — |
| M5 accounting reuse | closed | flat `085` | `085-status.md` | — |
