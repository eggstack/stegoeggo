# Stego Library Evolution Roadmap

Status: active

Repository baseline reviewed: `245fc94767c3fddb0e41c72de162aac3c1eaeb96`

Long-term references:

- `plans/000-long-term-specification.md#2-canonical-api-invariants`
- `plans/000-long-term-specification.md#3-execution-invariants`
- `plans/001-terminology-and-domain-model.md#marker-channel`
- `plans/002-long-term-roadmap.md#phase-2--generic-stego-carrier`
- `plans/002-long-term-roadmap.md#phase-6--v1-boundary-deferred`

Related ADRs:

- `plans/adrs/ADR-0002-output-domain-carrier-routing.md`
- `plans/adrs/ADR-0003-byte-vs-pixel-paths.md`
- `plans/adrs/ADR-0007-keyed-carrier-selection.md` (proposed; only M006 depends on acceptance)

Predecessor evidence: closed `plans/subsystems/stego-carrier-roadmap.md`,
Plans 090–097, especially `plans/096-status.md` and `plans/097-status.md`.

## 1. Purpose and ownership boundary

This workstream evolves the already-stable generic steganography library without
reopening the completed Phase 2 carrier-extraction project or beginning Phase 6's
breaking v1 cleanup.

It owns the boundary between `stegoeggo-stego` and the parent rights application,
generic carrier API ergonomics, bounded-resource behavior, allocation/performance
hardening, and additive generic carrier capabilities.

The carrier crate continues to know nothing about rights policy, legal metadata,
StegoEggo payload interpretation, warning severity, or application fallback policy.
The parent owns those semantics.

## 2. Work classification

### Invariants

- Existing seed-based LSB/JPEG/tiled carrier bytes remain compatible.
- `stegoeggo-stego` remains application-neutral and `#![forbid(unsafe_code)]`.
- Strict operations remain exact-or-fail and never emit partial carrier output.
- CRC32 framing is corruption detection, never authentication.
- Explicit bounded APIs cover untrusted-input parsing/search; pre-v0.4.2 one-shot APIs preserve their historical effective input/error contract through 0.x.
- Output-domain routing and root byte-vs-pixel semantics do not change.

### Capabilities

- Clear explicit strict versus best-effort JPEG entry points.
- Bounded generic JPEG preparation/inspection.
- Direct still lossless-WebP byte carrier convenience over the same LSB core.
- Future keyed carrier placement after ADR-0007 acceptance.

### Infrastructure

- Parent-local metadata write specification replacing legacy-context reconstruction.
- Reduction of hidden `application-support` coupling when public carrier primitives
  can cover the same operation without exposing codec internals.
- Reusable prepared-carrier helpers.

### Polish

- Semver-friendly getters and names before v1.
- Lower peak allocation for in-place/tiled paths.
- Dependency and documentation cleanup.

## 3. Non-goals

- Starting Phase 6 removal of `ProtectionContext`, `ProtectionPipeline`, or
  `Protector`.
- Exposing `jpeg_transcoder`, Huffman tables, raw coefficient maps, or permutation
  internals as stable generic API.
- Claiming recompression robustness for F5/LSB/tiled carriers.
- Shipping HILL/STC, J-UNIWARD, STDM, Reed-Solomon, or another adaptive/robust
  algorithm in the immediate milestones.
- Encrypting arbitrary generic payloads inside the carrier crate.
- Changing root rights-policy or verification trust semantics.

## 4. Current state

At current main after M001–M005:

1. M001 removed canonical-path `ProtectionContext` reconstruction, centralized
   metadata-write dispatch, preserved six golden output hashes, and removed the dead
   root `PixelSelectionRng`.
2. M002 added explicit JPEG best-effort names and report accessors without changing
   carrier bytes or removing compatibility names.
3. M003 added carrier-owned `CarrierLimits`, explicit `*_with_limits` JPEG APIs,
   `PreparedJpeg::new_with_limits`, and prepared tiled parity.
4. M004 replaced tiled in-place full-image rollback cloning with checked preflight,
   reducing measured auxiliary allocation from 1,053,952 to 5,376 bytes on the recorded
   1 MiB fixture with byte-compatible behavior.
5. M005 added the opt-in still-lossless WebP byte facade with explicit bounded and
   metadata-drop semantics.
6. A post-M003 compatibility audit found one unresolved defect: existing v0.4.2 JPEG
   one-shot APIs and `PreparedJpeg::new` now inherit M003's new 100 MiB input and
   16,384-dimension policy caps, and pre-existing parser-limit errors can map to
   `ResourceLimitExceeded` instead of their historical `MalformedInput`. This is
   tracked by ready corrective M008 and has not shipped in a release newer than v0.4.2.
7. M006 keyed placement remains blocked on ADR-0007 acceptance and M008 closure.
   M007 adaptive/robust work remains research-only.

The generic carrier boundary is otherwise coherent: arbitrary payload carriers and
resource controls stay below; rights policy, application degradation, metadata,
authentication interpretation, and warnings stay in the parent crate.

## 5. Target architecture

```text
stegoeggo
  rights policy / metadata / V1-V3 application payloads
  warnings / verification / best-effort application degradation
        |
        v
stegoeggo-stego
  arbitrary payload carrier operations
  exact capacity + bounded parsing/search
  LSB / JPEG DCT / tiled / framed / prepared / borrowed views / lossless WebP facade
        |
        v
private codec + permutation implementation
```

Through 0.x, compatibility vocabulary may remain reachable where `STABILITY.md`
requires it, but new generic callers should need only generic reports/errors/configs.
Breaking removals remain deferred to Phase 6.

## 6. Dependency graph

M001 boundary convergence → M002 API normalization → M003 resource/prepared hardening.

M003 → M004 transactional in-place allocation hardening.

M003 → M005 direct lossless-WebP byte facade.

M003 closure → M008 default-limit compatibility corrective. M008 is dependency-ready
and gates the next carrier release because it corrects a behavioral discrepancy found
after M003 closure.

M002 + M003 + M008 closure + accepted ADR-0007 → M006 keyed carrier placement.

M007 adaptive/robust algorithm experiment is future research only and has no
implementation handoff until M008 closes and a maintainer explicitly authorizes the
algorithm/threat-model expansion.

## 7. Milestones

- **M001 — Parent/carrier boundary and metadata-write convergence.**
  Class: infrastructure. Remove canonical-path reconstruction of legacy context,
  centralize metadata update-policy dispatch, audit hidden bridge/dead root stego
  helpers without breaking 0.x.
- **M002 — Generic carrier API normalization.**
  Class: capability. Add explicit best-effort names, semver-safe getters/helpers,
  direct-crate examples, and freeze v1 naming/disposition without changing current
  defaults.
- **M003 — Carrier resource limits and prepared-operation parity.**
  Class: invariant. Introduce carrier-owned bounded decode/search configuration and
  reuse decoded JPEG state for repeated supported operations.
- **M004 — Transactional in-place allocation hardening.**
  Class: polish. Replace full-image rollback snapshots where possible with preflight
  or bounded change journals while preserving atomic failure semantics.
- **M005 — Lossless WebP encoded-byte carrier facade.**
  Class: capability. Add still lossless-WebP decode→LSB→lossless-encode convenience
  with explicit metadata/preservation/non-animation contract.
- **M006 — Keyed placement scheme.**
  Class: capability/invariant. Blocked until ADR-0007 is accepted.
- **M007 — Adaptive/robust carrier experiment.**
  Class: infrastructure research. Future only; evaluate HILL/STC for lossless images
  and JPEG J-UNIWARD/STC versus robust STDM/ECC-style approaches without promising a
  stable API.
- **M008 — M003 default-limit compatibility corrective.**
  Class: invariant. Restore the v0.4.2 effective acceptance/error contract for
  pre-existing JPEG one-shot APIs and `PreparedJpeg::new` while retaining M003's
  explicit bounded APIs and conservative `CarrierLimits::default()`.

## 8. Cross-cutting requirements

Compatibility: no 0.x removals or reinterpretation of current seeds. Cargo's SemVer
guidance treats additions to all-public-field structs as breaking for struct-literal
users; existing public report fields therefore stay frozen and additive evolution uses
methods/new types until the breaking boundary.

Security: secret-key placement, if accepted, is not encryption/authentication. No
secret bytes in diagnostics. Bounded parsing applies before allocations derived from
untrusted lengths.

Dependency discipline: do not replace the private JPEG transcoder merely because
`dct-io` exists; it is useful prior art for low-level coefficient APIs, but StegoEggo
must preserve its current byte/container/known-answer behavior. New WebP dependency
surface must be justified against the existing `image` stack.

Research reviewed 2026-10-02:
- Cargo SemVer compatibility: https://doc.rust-lang.org/cargo/reference/semver.html
- `dct-io` 0.1.1: https://docs.rs/dct-io/latest/dct_io/
- `stenoxide-core` adaptive HILL/STC: https://docs.rs/stenoxide-core/latest/
- `phasm-core` J-UNIWARD/STC and STDM/RS split: https://docs.rs/phasm-core/latest/phasm_core/
- image WebP lossless encoder: https://docs.rs/image/latest/image/codecs/webp/struct.WebPEncoder.html

## 9. Verification strategy

Every implementation milestone runs `./scripts/check.sh` plus focused carrier
direct-consumer/public API/known-answer tests. Public-surface milestones also record
`cargo-semver-checks` evidence. Resource-limit work adds adversarial/truncation tests
and fuzz-regression fixtures. Allocation claims require Criterion or explicit
allocation/peak-memory evidence rather than code inspection alone.

No new specialist check becomes required CI without maintainer direction.

## 10. Risks and decision points

- M001 can accidentally alter canonical metadata bytes; golden/conformance fixtures must
  prove byte/semantic equivalence.
- M002 naming can become another compatibility layer if both old/new names are kept
  indefinitely; v1 disposition must be explicit.
- M003 limits must not create two contradictory resource models between root and carrier.
- M004 rollback optimization can corrupt caller buffers on mid-operation failure if
  transactional semantics are weakened.
- M005 may erase unrelated WebP metadata if implemented as a simplistic decode/re-encode;
  the public preservation contract must be explicit.
- M006 creates a permanent algorithm compatibility obligation after release.
- M007 algorithms have fundamentally different threat models; one "robust stego" mode
  must not blur stealth and recompression-survival claims.
- M008 must distinguish published 0.x compatibility from new bounded safety policy:
  raising defaults globally or weakening root resource limits are both incorrect fixes.

## 11. Completion definition

The workstream cannot close until M008 has closure evidence resolving the M003
behavioral-compatibility discrepancy. After M008, M006 may either close after ADR-0007
acceptance or be explicitly deferred. Generic docs must distinguish legacy 0.x
compatibility entry points from explicit bounded untrusted-input APIs, and the parent
must continue enforcing its own resource policy.

M007 is not required for workstream closure unless separately promoted by maintainer
direction.

## 12. Milestone status table

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M001 boundary/metadata convergence | closed | `plans/implementation/stego-library-evolution/001-boundary-and-metadata-convergence.md` | `plans/closure/stego-library-evolution/001-status.md` | none |
| M002 API normalization | closed | `plans/implementation/stego-library-evolution/002-generic-carrier-api-normalization.md` | `plans/closure/stego-library-evolution/002-status.md` | none |
| M003 limits/prepared parity | closed | `plans/implementation/stego-library-evolution/003-carrier-resource-and-prepared-hardening.md` | `plans/closure/stego-library-evolution/003-status.md` | none |
| M004 in-place allocation hardening | closed | `plans/implementation/stego-library-evolution/004-transactional-in-place-allocation-hardening.md` | `plans/closure/stego-library-evolution/004-status.md` | none |
| M005 lossless WebP byte facade | closed | `plans/implementation/stego-library-evolution/005-lossless-webp-byte-carrier.md` | `plans/closure/stego-library-evolution/005-status.md` | none |
| M006 keyed placement | blocked | `plans/implementation/stego-library-evolution/006-keyed-carrier-placement.md` | pending | ADR-0007 acceptance + M008 closure |
| M007 adaptive/robust experiment | proposed | none | none | M008 closure + explicit maintainer authorization |
| M008 M003 default-limit compatibility corrective | ready | `plans/implementation/stego-library-evolution/008-m003-default-limit-compatibility-corrective.md` | pending | none |
