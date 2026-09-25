# Rights Metadata Roadmap

Status: closed

Long-term references:

- `plans/000-long-term-specification.md#1-product-identity`
- `plans/001-terminology-and-domain-model.md#metadata-channel`
- `plans/002-long-term-roadmap.md#phase-0--foundation`

Related ADRs:

- `plans/adrs/ADR-0001-canonical-protection-request.md`
- `plans/adrs/ADR-0003-byte-vs-pixel-paths.md`

## 1. Purpose and ownership boundary

Owns the primary protection channel: rights-notice resolution and its
rendering into PNG tEXt/iTXt, JPEG COM/APP13/XMP/EXIF/IPTC, and WebP
XMP/EXIF. Consumes `RightsNotice`/`RightsPolicy`/`LegalMetadata` from the
request model; owns `src/protected/metadata_trap/` and `src/xmp.rs`.
Does not own carriers, payloads, or verification.

## 2. Work classification

### Invariants

- Canonical `plus:DataMining` emitted; legacy `DMI-*`/`tdm:reserve_tdm` parsed, never emitted.
- Explicit policy only; `ContradictoryLegalClaims` on `false` + legal metadata.

### Capabilities

- Per-format rights injection with merge-policy and idempotent re-processing.

### Infrastructure

- Notice rendering helpers, date/timestamp plumbing, namespace-aware XMP filter.

### Polish

- Terminology/docs alignment (`legal_notice_model.md`).

## 3. Non-goals

Carrier embedding, payload design, verification semantics, C2PA.

## 4. Current state

Closed at 0.4.2. `RightsMetadataProtector::inject_bytes_from_plan()`
carries all formats; `LegalMetadata` validates 16 fields at ≤8192 bytes;
legacy signals remain parse-only. Evidence: `tests/canonical_rights.rs`,
`tests/merge_policy.rs`, `tests/metadata_only_idempotence.rs`,
`tests/preservation.rs`.

## 5. Target architecture

No change: metadata channel stays the primary legal-evidence path with
byte-path-only carriage (ADR-0003).

## 6. Dependency graph

No open dependencies. Historical order: terminology → CLI fields →
Release 1 canonical rights → Release 2 legal semantics → classification
correctives. All hard deps closed.

## 7. Milestones

All closed; predecessor flat plans are the evidence (roadmap defers to
their status companions for per-milestone detail):

- M1 canonical rights metadata — flat `001`, `009`, `011`, `016`,
  `legal-notice-first-roadmap.md`. Class: capability. Exit: canonical
  PLUS/DMI/notice round-trips per format.
- M2 legal-metadata fields and semantics — flat `002`, `017`.
  Class: capability. Exit: 16-field validation, auto-enable, contradiction
  guard.
- M3 format-correctness correctives — flat `013`, `014`, `038`, `039`.
  Class: invariant. Exit: PLUS/IPTC shape conformance.
- M4 canonical classification corrective — flat `046`. Class: invariant.
  Exit: policy-explicit rendering, no inferred semantics.

## 8. Cross-cutting requirements

Storage: none (in-image metadata). Compat: legacy parse retained per
`STABILITY.md` retention promises. Security: bounded XMP parsing via
`resource_limits.rs`. Observability: warnings (`ContradictoryLegalClaims`,
`MissingRightsConstraints`). Docs: `docs/legal_notice_model.md`,
`architecture/protected-metadata-trap.md`.

## 9. Verification strategy

`cargo test` format/metadata suites + `verify_metadata_conformance.sh
--strict` (specialist, exiftool/xmllint) for shape claims.

## 10. Risks and decision points

None open. Historical risk (policy inference) retired by ADR-0001.

## 11. Completion definition

Met: primary channel closed across formats with parse-only legacy
compat and documented retention promises.

## 12. Milestone status table

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| M1 canonical rights | closed | flat `016` | flat `016` has no separate status; covered by `019-status.md` chain | — |
| M2 legal semantics | closed | flat `017` | covered by `019-status.md` chain | — |
| M3 format correctives | closed | flat `039` | `039-status.md` | — |
| M4 classification | closed | flat `046` | `046-status.md` | — |
