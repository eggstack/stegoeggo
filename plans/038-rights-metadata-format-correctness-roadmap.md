# Plan 038: Rights Metadata and Format Correctness Roadmap

Status: Ready for implementation

Baseline: `main` at `36fb7d5797f0d60dca3fa701f4cb53d66c578ae4`

Depends on:

- the current rights-metadata, steganography, request-resolution, verification, and CLI implementation at the baseline above;
- the process simplification established by Plans 033 through 037, which must remain intact.

Implementation plans:

- `plans/039-plus-iptc-rights-metadata-correctness.md`
- `plans/040-jpeg-dct-container-correctness-and-containment.md`
- `plans/041-webp-container-xmp-exif-correctness.md`
- `plans/042-api-cli-contract-consolidation.md`
- `plans/043-measured-binary-size-reduction.md`
- `plans/044-cross-format-correctness-closure.md`

This roadmap is a product-correctness and simplification line. It does not reopen automated release work, expand required CI, or authorize publication.

---

## Purpose

StegoEggo has a valid, bounded product goal:

1. write machine-readable rights-reservation and AI-training restriction metadata into supported image formats;
2. preserve unrelated image data and metadata where the selected operation does not require transcoding;
3. optionally add a best-effort hidden marker as redundant evidence;
4. report what was actually written and what can actually be verified without overstating legal, forensic, or cryptographic guarantees.

The current repository substantially expresses that goal, but the implementation still has correctness gaps at the standards, container, codec, API, and CLI boundaries. Several paths are self-consistent only because StegoEggo's writer and parser accept the same private or malformed representation. That is insufficient for a metadata interoperability tool.

This roadmap corrects those gaps without broadening the product into a general image metadata framework, a complete JPEG implementation, a DRM system, a C2PA replacement, or a production-scale provenance service.

The desired result is a smaller conceptual surface with stronger external correctness:

```text
canonical rights metadata first
best-effort hidden marker second
format preservation where feasible
explicit fallback where unsupported
one request model
measured size work only
manual release remains maintainer-owned
```

---

## Problem statement

At the baseline, the repository has the following material issues.

### Standards output

- `plus:DataMining` is emitted with a bare vocabulary key rather than the complete PLUS controlled-vocabulary URI.
- the parser accepts that bare key and classifies it as canonical, allowing StegoEggo to certify its own noncanonical output;
- `DMI-PROHIBITED-SEECONSTRAINT` is not paired with the standard PLUS Other Constraints property;
- private fields such as `DMI-PROHIBITED` and `noai=noindex` can contradict the canonical policy, especially for `Allowed` or `Unspecified`;
- internal terminology still uses `trap` and `poison` names even though the public product is a rights-notice tool, not a poisoning tool.

### JPEG processing

- the coefficient encoder does not invert the decoder's block order for common vertically subsampled components;
- restart handling resets DC predictors after, rather than before, the first MCU following a restart marker;
- the custom encoder does not reproduce DRI/restart structure;
- the Huffman decoder's canonical-code construction is asymmetric with the encoder for tables containing empty code lengths;
- malformed or truncated entropy data can produce partial coefficient output instead of a hard error;
- the reconstructed JPEG drops or reorders APP2, APP13, APP14, DRI, unknown APP segments, and other opaque data;
- the byte path is therefore not generally lossless as a JPEG document, despite preserving some coefficient data;
- unsupported JPEG coding processes and color-space-sensitive metadata require explicit containment rather than optimistic reconstruction.

### WebP processing

- XMP and EXIF chunks are appended to simple WebP files without creating a VP8X extended header;
- existing VP8X feature flags are not updated;
- existing XMP or EXIF chunks can be duplicated instead of replaced or merged;
- the generated EXIF/TIFF structure is not standards-correct even though StegoEggo can recover its seed by byte searching;
- chunk ordering, duplicate handling, and unknown-chunk preservation need a single checked RIFF rewrite path.

### API and CLI behavior

- the `DynamicImage` processing API cannot preserve file-level metadata and therefore cannot return the primary product output;
- the CLI has two configuration and execution paths, with legacy options and request-based options interpreted separately;
- some mixed-option combinations silently discard DMI overrides;
- verification does not consistently use the documented key resolver;
- the production CLI enables test-seed guessing unconditionally;
- deprecated and canonical APIs coexist as independent decision systems rather than thin adapters.

### Binary composition

- the workspace has no measured binary-size baseline;
- several dependencies serve optional capabilities but are unconditional in the library or CLI graph;
- the repository has duplicate or overlapping image/JPEG machinery that may contribute code size;
- size work has not been separated from correctness work, making it difficult to tell whether a simplification is safe or merely smaller.

---

## Governing decisions

The implementation agent must treat these as project policy for this line of work.

### Decision 1: external semantics outrank self-round-trips

A writer/parser round trip is not evidence of standards conformance when both ends are implemented by StegoEggo.

Canonical claims require at least one independent representation check, such as:

- a golden byte fixture derived from the applicable format/metadata specification;
- ExifTool output;
- `xmllint` or an equivalent XML parser;
- `webpmux`, libwebp, ImageMagick, or libvips inspection where applicable;
- an independently generated fixture that StegoEggo must preserve and read.

These checks remain targeted/manual. Do not expand required CI.

### Decision 2: metadata is the primary product channel

Standards-correct metadata writing and preservation take priority over hidden-marker coverage.

A hidden marker may fall back, be skipped, or become unavailable for an unsupported input while rights metadata still succeeds. The operation must report that downgrade truthfully.

### Decision 3: do not complete the JPEG specification

This roadmap does not authorize building a full JPEG codec.

The custom DCT path must either:

- become demonstrably correct for a clearly bounded subset of sequential JPEGs; or
- be constrained/feature-gated so unsupported inputs use metadata-only processing with an explicit warning.

Progressive, multi-scan, arithmetic-coded, lossless, restart-bearing, CMYK/YCCK, or otherwise unsupported JPEGs must not be rewritten through a path that cannot preserve their semantics.

### Decision 4: preserve opaque container data

Where the operation is same-format and metadata-only, unrelated chunks/segments must be copied byte-for-byte unless a format rule requires a bounded rewrite.

The implementation must not discard unknown metadata merely because StegoEggo does not understand it.

### Decision 5: one canonical request model

`ProtectionRequest` and `ResolvedProtectionPlan` are the canonical configuration and execution model.

Legacy level/context/profile APIs may remain for compatibility, but they must translate into the canonical request model rather than execute independent policy logic.

The CLI must assemble one request and invoke one processing path.

### Decision 6: byte APIs are authoritative for metadata

File-level metadata cannot be represented by `image::DynamicImage`.

The byte API is the authoritative full-product API. Pixel-only APIs must be explicitly named and documented as pixel-only, or deprecated if their current names imply complete protection.

### Decision 7: size changes require measurements

Do not add feature flags, replace dependencies, or introduce custom infrastructure based only on intuition.

Every binary-size change must have:

- a reproducible baseline command;
- before/after stripped size;
- dependency or symbol evidence where relevant;
- behavior-preservation evidence;
- a complexity check.

If a change does not yield a meaningful reduction, do not retain extra architectural complexity solely for theoretical size savings.

### Decision 8: verification remains bounded

Required CI remains the single stable job established by the process simplification plans.

This roadmap may add focused unit/integration fixtures and update the existing manual external-verification workflow, but it must not add:

- a required OS matrix;
- a required format matrix job;
- scheduled external verification;
- required fuzzing;
- release-candidate certification;
- automated publication;
- a new evidence framework.

### Decision 9: no release is part of this roadmap

Implementation may require a future version bump because crates.io versions are immutable, but publication, tagging, and GitHub release creation are outside these plans.

---

## Target end state

At roadmap closure, the repository must satisfy all of the following.

### Rights metadata

- emitted `plus:DataMining` values use the complete canonical controlled-vocabulary URI;
- `ProhibitedSeeConstraints` emits the applicable standard constraints property;
- private StegoEggo fields cannot contradict the canonical policy;
- bare-key and historical private values may be parsed for compatibility but are not classified as canonical;
- `Allowed`, `Unspecified`, and each prohibition policy have explicit golden-output tests;
- external tooling can read the canonical property from PNG, JPEG, and WebP output where the format supports XMP.

### JPEG

- metadata-only same-format JPEG processing preserves unrelated segments and scan bytes;
- unsupported JPEGs do not enter unsafe coefficient rewrite paths;
- the supported DCT subset, if retained, has correct block order, Huffman handling, truncation behavior, and restart containment;
- claims such as “lossless JPEG fast path” are used only where proven and accurately scoped;
- ICC, APP13, APP14, EXIF/XMP, COM, DRI, and unknown-segment preservation behavior is tested or explicitly unsupported with fallback.

### WebP

- simple WebP is converted to extended WebP before metadata chunks are added;
- VP8X flags match the actual XMP/EXIF/alpha/animation features;
- at most one effective XMP and EXIF chunk is emitted by StegoEggo;
- unknown chunks are preserved;
- EXIF output is standards-correct, or EXIF seed redundancy is removed in favor of correct XMP-only behavior;
- outputs open in the pinned Rust decoder and at least one external WebP implementation.

### API and CLI

- all CLI modes resolve to one `ProtectionRequest` and one execution path;
- mixed legacy/new options are either normalized deterministically or rejected with a configuration error;
- all documented key sources work consistently in protection and verification modes;
- test-seed guessing is not enabled in the normal production CLI build;
- metadata-losing pixel APIs are renamed, deprecated, or documented so callers cannot mistake them for complete byte-level protection;
- legacy public APIs remain thin compatibility adapters where retained.

### Binary composition

- a stripped release-size baseline and final measurement are recorded;
- optional capabilities are feature-gated only where this reduces the normal artifact without removing the capability;
- the release profile is evaluated and documented;
- duplicate codec/dependency paths are retained only when their distinct behavior is required;
- no size optimization introduces a second orchestration system or weakens correctness.

### Process

- required CI remains one stable Ubuntu job;
- external interoperability remains manual/targeted;
- no workflow publishes crates or creates releases;
- no version is published during implementation;
- each implementation plan has a truthful status ledger with exact commit evidence.

---

## Roadmap sequence

### Milestone A: establish canonical rights semantics

Implement Plan 039 first.

Reason: metadata is the primary product channel. JPEG, WebP, API, and size work must target the correct rights representation rather than preserving or optimizing a noncanonical one.

Expected outcome:

- canonical URI emission and parsing classification;
- correct constraints semantics;
- contradictory private fields removed from default output;
- independent metadata fixtures;
- terminology aligned with rights metadata rather than poisoning.

### Milestone B: contain and correct JPEG processing

Implement Plan 040 after Plan 039.

Reason: the JPEG DCT path is the largest correctness and maintenance risk. It must be bounded before API consolidation can describe stable behavior.

Expected outcome:

- byte-preserving metadata-only path;
- explicit supported JPEG subset;
- unsafe inputs routed to metadata-only behavior;
- corrected coefficient order and entropy handling if DCT embedding remains enabled;
- no requirement to implement every JPEG mode.

### Milestone C: repair WebP container semantics

Implement Plan 041 after Plan 039. It may run in parallel with Plan 040 if separate agents do not modify shared metadata-writing helpers without coordination.

Expected outcome:

- checked RIFF parser/rewriter;
- VP8X creation and feature-bit maintenance;
- replace/merge semantics for XMP and EXIF;
- standards-correct EXIF or deliberate XMP-only seed handling;
- external decoder interoperability.

### Milestone D: consolidate API and CLI contracts

Implement Plan 042 after Plans 039 through 041 establish final format behavior.

Expected outcome:

- one canonical request translation;
- one CLI processing path;
- consistent key resolution;
- explicit pixel-only versus byte-level API contracts;
- deprecated adapters that do not duplicate policy.

### Milestone E: reduce measured binary size

Implement Plan 043 after behavior and API topology stabilize.

Expected outcome:

- measured baseline and final artifact;
- size-oriented release profile if beneficial;
- optional dependency gating where behavior is preserved;
- removal of production test-seed support;
- no speculative architecture added for negligible savings.

### Milestone F: cross-format closure

Implement Plan 044 last.

Expected outcome:

- focused golden fixture matrix;
- one manual external verification pass;
- documentation aligned with proven behavior;
- status ledgers closed with exact evidence;
- remaining limitations recorded rather than hidden;
- no release side effects.

---

## Dependency and parallelism rules

```text
Plan 039 ─────┬────> Plan 040 ──┐
              ├────> Plan 041 ──┼────> Plan 042 ─────> Plan 043 ─────> Plan 044
              └─────────────────┘
```

Rules:

1. Plan 039 must establish canonical metadata semantics before format-specific closure.
2. Plans 040 and 041 may proceed in parallel only if ownership of shared metadata code is explicit.
3. Plan 042 must not freeze API behavior around uncorrected JPEG or WebP semantics.
4. Plan 043 must not begin dependency removal until Plans 039 through 042 identify which capabilities are actually required by the normal artifact.
5. Plan 044 is evidence closure, not another redesign phase.

---

## Scope boundaries

### In scope

- canonical PLUS Data Mining output and compatibility parsing;
- standard constraints output for `ProhibitedSeeConstraints`;
- private rights-marker conflict removal;
- PNG/JPEG/WebP XMP generation and extraction relevant to rights metadata;
- JPEG segment preservation and bounded DCT processing;
- WebP RIFF/VP8X/XMP/EXIF rewriting;
- request resolution, legacy adapters, CLI argument normalization, and key resolution;
- pixel-only versus byte-level API contracts;
- dependency and release-profile measurements;
- focused tests, fixtures, docs, and existing manual external verification.

### Out of scope

Do not use this roadmap to:

- implement C2PA;
- implement DRM or anti-screenshot behavior;
- claim that metadata creates or proves copyright ownership;
- claim that a hidden marker proves model training or infringement;
- add data poisoning;
- add TDMRep HTTP headers or `/.well-known/tdmrep.json` deployment artifacts;
- add new image formats;
- add lossy WebP encoding;
- implement a complete JPEG codec;
- add a generalized XML library unless a measured correctness need cannot be met with the current bounded approach;
- replace Ed25519, HMAC, provenance, or detached-manifest semantics unrelated to the findings;
- redesign the entire crate workspace;
- add production network services;
- add databases, telemetry, policy engines, or plugin systems;
- restore complex CI/release workflows;
- publish crates or create tags/releases.

---

## Verification budget

The implementation plans may require focused tests, but the final routine verification surface remains:

```bash
./scripts/check.sh
```

Targeted local/manual commands may include:

```bash
cargo test -p stegoeggo <focused test filter>
cargo test --test external_tools -- --ignored
cargo build --release -p stegoeggo-cli
cargo bloat --release -p stegoeggo-cli --bin stegoeggo -n 50
cargo tree -p stegoeggo-cli -e features
```

External tools should be reused through the existing manual external-verification path. Do not create one workflow per format or one job per fixture.

Fixture guidance:

- prefer a small number of representative independently generated files;
- keep fixture provenance documented;
- include difficult structures, not dozens of visually redundant images;
- use exact semantic assertions rather than broad snapshot churn;
- do not commit large corpora solely for this roadmap.

---

## Cross-plan invariants

Every implementation plan must preserve these invariants.

1. Unknown container data is preserved on same-format metadata-only operations.
2. Unsupported hidden-marker processing degrades explicitly; it does not corrupt the image or silently claim success.
3. `Allowed` is never accompanied by a private prohibition marker.
4. `Unspecified` does not invent a prohibition.
5. canonical and legacy metadata are reported separately when both are present.
6. metadata-only operations do not alter pixel or scan payload bytes unless a format requires a narrowly documented container conversion.
7. output bytes must be decodable by the pinned Rust image stack before they are returned.
8. warnings and execution reports describe actual channel outcomes.
9. legacy APIs cannot bypass request validation.
10. no plan may add release publication or required-CI fan-out.

---

## Roadmap-level acceptance criteria

The roadmap is closed only when all of the following are true.

### Planning and evidence

- Plans 039 through 044 each have a status ledger.
- Each status ledger records the baseline SHA, implementation commits, commands, results, and remaining limitations.
- No plan is marked closed based solely on code review without execution evidence.

### Product correctness

- canonical PLUS URI output is independently observed in PNG, JPEG, and WebP fixtures;
- `ProhibitedSeeConstraints` emits standard constraints content;
- private compatibility markers do not contradict canonical policy;
- metadata-only JPEG preserves opaque pre-scan segments and original scan bytes;
- unsupported JPEG DCT cases use explicit fallback or rejection;
- supported DCT fixtures survive protect/verify without block reordering or malformed entropy acceptance;
- WebP output has valid VP8X flags and nonduplicated effective metadata;
- WebP EXIF is externally parseable or is no longer emitted;
- CLI legacy/new option combinations have deterministic behavior;
- normal verification honors literal, file, stdin, and environment key sources as documented;
- full-product APIs return bytes, while pixel-only APIs are unmistakably pixel-only.

### Simplification and size

- duplicated policy decision paths are removed;
- the custom JPEG surface is bounded rather than expanded toward full-spec support;
- a stripped release binary before/after measurement is recorded;
- retained feature gating produces measurable value or is removed;
- the normal CLI does not enable test-seed guessing;
- required CI and manual release policy remain unchanged.

### Documentation

- README and API documentation distinguish rights notice, best-effort stego, authenticated provenance, and detached manifests;
- format limitations match actual behavior;
- no documentation describes noncanonical output as canonical;
- no documentation calls a path lossless unless preservation criteria are met;
- no documentation implies that `DynamicImage` can carry file metadata;
- release notes identify that any future publication requires an unused version.

---

## Completion definition

This roadmap is complete when StegoEggo reliably performs its bounded intended function:

```text
write standards-correct rights metadata
preserve unrelated image structure where promised
add hidden evidence only where safely supported
report downgrades truthfully
expose one coherent API/CLI contract
ship a measured, not speculative, smaller artifact
```

Known format limitations may remain. They must be explicit, safe, and tested. The roadmap does not require feature maximalism; it requires that every retained feature mean what the repository says it means.