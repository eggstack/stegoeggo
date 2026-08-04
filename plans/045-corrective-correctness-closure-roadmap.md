# Roadmap 045: Post-044 Correctness and Evidence Closure

Status: Ready for implementation

Audited baseline: `main` at `4d2e849f40049bef6416cfdc4970ba576d269869`

Supersedes the completion claim in:

- `plans/044-cross-format-correctness-closure.md`

Corrects remaining defects from:

- `plans/039-plus-iptc-rights-metadata-correctness.md`
- `plans/040-jpeg-dct-container-correctness-and-containment.md`
- `plans/041-webp-container-xmp-exif-correctness.md`
- `plans/042-api-cli-contract-consolidation.md`
- `plans/044-cross-format-correctness-closure.md`

Implementation plans:

1. `plans/046-rights-metadata-canonical-classification-corrective-pass.md`
2. `plans/047-cli-default-policy-and-equivalence-corrective-pass.md`
3. `plans/048-jpeg-dct-preservation-and-entropy-corrective-pass.md`
4. `plans/049-webp-xmp-replacement-and-feature-flags-corrective-pass.md`
5. `plans/050-post-corrective-evidence-and-documentation-closure.md`

---

## Purpose

Complete the narrow correctness work that remained after Plans 039-044 without reopening the product architecture or rebuilding the large verification apparatus that was intentionally removed.

The prior implementation materially improved the repository, but the final audit found that several tests and completion claims did not match the required contracts:

- `Unspecified` rights policy can still emit a noncanonical `DMI-UNSPECIFIED` `plus:DataMining` value;
- bare keys and arbitrary-origin URLs ending in a known key are still classified as canonical PLUS signals;
- the normal CLI invocation no longer defaults `Standard` protection to the documented AI/ML prohibition policy;
- successful JPEG DCT embedding still canonicalizes through a container-dropping path rather than the preserving path;
- JPEG Huffman construction and malformed-symbol handling remain incomplete;
- multi-scan sequential JPEGs are not explicitly rejected by the DCT capability probe;
- WebP XMP replacement can copy the original XMP and append a replacement, yielding duplicates;
- preserved WebP EXIF is not reflected in regenerated VP8X feature flags;
- required status/evidence ledgers for Plans 039-044 were never committed;
- several closure tests accept the incorrect behavior instead of enforcing the intended contract.

This roadmap treats those as bounded corrective defects. It does not authorize new formats, new rights policies, new cryptography, a new image framework, or more CI.

---

## Governing constraints

1. Required CI remains one stable job invoking `scripts/check.sh`.
2. Do not add a CI matrix, scheduled workflow, release workflow, publication workflow, binary-size gate, conformance gate, or mandatory external-tool job.
3. Release remains manual and out of scope. Do not bump versions, publish crates, create tags, or create GitHub releases.
4. Preserve the canonical `ProtectionRequest` execution model introduced by Plan 042.
5. Preserve the current feature split from Plan 043 unless a feature boundary is demonstrably broken.
6. Do not add a general XML/RDF library merely to correct the bounded PLUS parsing problem.
7. Do not add a general-purpose JPEG codec. Either make the retained DCT subset correct or reject unsupported structures explicitly.
8. Do not add a general-purpose WebP muxing dependency unless the current bounded RIFF implementation cannot satisfy the exact acceptance criteria with less code.
9. Prefer exact semantic tests and small fixtures over broad test multiplication.
10. Existing historical plans remain historical. Correct their status through truthful ledgers; do not rewrite history to imply evidence existed when it did not.
11. Accepted limitations must be observable through warnings/reports and current documentation.
12. No behavior may be marked complete solely because project-owned writer and parser agree.

---

## Corrective workstreams

### Workstream A: Canonical rights serialization and classification

Plan 046 owns:

- omission of `plus:DataMining` for `Unspecified`;
- strict canonical PLUS URI recognition;
- separate backward-compatible bare-key recognition;
- rejection or noncanonical classification of arbitrary-origin URLs;
- correct `RightsSignalKind` reporting;
- exact `ProhibitedSeeConstraints` companion-field behavior;
- removal of stale TDM image-emission documentation;
- replacement of permissive negative tests with exact contract assertions.

This workstream changes semantic classification only. It must not redesign rights policy values or remove compatibility parsing.

### Workstream B: CLI default and equivalence correction

Plan 047 owns:

- restoration of the documented default mapping for normal legacy CLI use;
- one explicit legacy-to-request policy function;
- exact equivalence tests between legacy and canonical request syntax;
- verification that dry-run, JSON, human output, single-file, and batch paths share the same resolved request;
- deterministic conflict handling without a second execution path.

This workstream must not revive the old duplicated processing pipeline.

### Workstream C: JPEG DCT correctness and containment

Plan 048 owns:

- using the original-container preservation path for successful DCT embedding;
- removing or tightly containing canonicalization that drops APP/COM/DRI/unknown segments;
- correcting canonical Huffman decoder table construction;
- failing closed on missing/invalid DC and AC symbols;
- rejecting multiple-scan sequential JPEGs unless they are implemented correctly;
- retaining the current restart-bearing/progressive fallback policy;
- exact fixtures for supported 4:4:4, 4:2:2, and 4:2:0 baseline JPEGs and unsupported structures;
- preservation assertions for APP2 ICC, APP13 IPTC/Photoshop, APP14 Adobe, COM, and unknown APP segments.

This workstream must not broaden the DCT subset beyond what can be proven correct.

### Workstream D: WebP replacement and feature-bit correctness

Plan 049 owns:

- exactly one XMP chunk after replace/merge operations;
- preservation of unrelated XMP properties in the surviving packet;
- correct removal/replacement of StegoEggo-owned properties;
- preservation of unknown RIFF chunks and image payload chunks;
- accurate VP8X flags derived from actual output chunks, including EXIF;
- checked handling of malformed or duplicate input metadata;
- exact idempotence tests.

This workstream must retain the XMP-only seed-emission decision from Plan 041.

### Workstream E: Evidence and documentation closure

Plan 050 owns:

- one compact final cross-format semantic matrix;
- correction of tests that currently encode permissive behavior;
- bounded independent-tool evidence where tools are available;
- retrospective status ledgers for Plans 039-044 that distinguish original implementation from later correction;
- status ledgers for Plans 046-050;
- exact final CI evidence when available;
- documentation truth and accepted limitations;
- a final disposition that does not claim publication.

Plan 050 is not authorized to hide implementation defects by documenting them as limitations if Plans 046-049 require correction.

---

## Execution order

Recommended order:

```text
046 canonical rights semantics
047 CLI defaults/equivalence
048 JPEG correctness/containment
049 WebP replacement/flags
050 evidence and documentation closure
```

Plans 046-049 are logically separable, but implementation on one branch should remain sequential to avoid misleading status and merge conflicts in shared tests/documentation.

Plan 050 must run last.

---

## Required status discipline

Each implementation plan must create its own status ledger before product edits:

```text
plans/046-status.md
plans/047-status.md
plans/048-status.md
plans/049-status.md
plans/050-status.md
```

The ledger must include:

```text
Plan baseline SHA
Disposition
Acceptance-criterion table
Implementation commits
Commands executed
Tests added/changed
Observed results
Known blockers
Documentation changes
CI evidence
Publication hold
```

Plan 050 must also create truthful retrospective ledgers for the missing prior files:

```text
plans/039-status.md
plans/040-status.md
plans/041-status.md
plans/042-status.md
plans/043-status.md
plans/044-status.md
```

Those retrospective ledgers must state that they were created after implementation. They must not claim that Phase 0 ledgers existed before source changes. They should record:

- original implementation commits;
- audit findings that remained afterward;
- corrective plan/commit links;
- final disposition only after the corrective pass;
- exact evidence available and unavailable.

---

## Verification budget

Required automatic verification remains:

```bash
./scripts/check.sh
```

Targeted local commands are permitted where directly relevant:

```bash
cargo test --all-features --test cross_format_semantics
cargo test --all-features --test jpeg_container_preservation
cargo test --all-features --test conformance_container_tests
cargo test -p stegoeggo-cli --test cli
```

One manual interoperability pass may use available tools such as ExifTool, `webpinfo`, `webpmux`, ImageMagick, or libvips. Missing tools are recorded as unavailable; they are not installed into CI.

Do not add:

- exhaustive feature powersets;
- long fuzz campaigns;
- repeated clean builds as a routine gate;
- multiple operating-system CI jobs;
- artifact retention pipelines;
- evidence upload workflows.

---

## Roadmap acceptance criteria

This roadmap is complete only when all of the following are true:

1. `Unspecified` emits no `plus:DataMining` property.
2. Full canonical PLUS URIs are reported as canonical.
3. Bare known keys remain readable but are reported as legacy/noncanonical.
4. Arbitrary-origin URLs ending in known keys are not reported as canonical and do not silently establish a canonical rights policy.
5. The normal CLI default invocation retains the documented `Standard` policy behavior.
6. Equivalent legacy and canonical CLI syntax resolve to equivalent policy, channels, seed, format, metadata, authentication, and reports.
7. Successful supported JPEG DCT embedding preserves unrelated container segments from the original input.
8. Unsupported JPEG scan structures downgrade explicitly rather than being partially rewritten.
9. JPEG Huffman decoding rejects malformed/truncated symbols rather than returning partial coefficient maps.
10. WebP replacement leaves exactly one XMP chunk and preserves unrelated XMP properties.
11. WebP VP8X feature bits reflect the chunks actually present after rewriting, including EXIF.
12. Corrected outputs decode through the project decoder and at least one independent decoder where available.
13. The final tests assert exact required behavior and no longer accept known-invalid alternatives.
14. Plans 039-050 have truthful status records.
15. Required CI remains the existing single `Check` job.
16. No version bump, crate publication, tag, or release occurs.

---

## Explicit non-goals

Do not use this roadmap to:

- add C2PA;
- implement TDMRep HTTP publication;
- add AVIF, TIFF, GIF, HEIF, or video support;
- guarantee watermark survival under arbitrary transformation;
- implement progressive JPEG DCT steganography;
- implement restart-bearing JPEG DCT steganography;
- support arbitrary multi-scan JPEG transcoding;
- replace the `image` crate;
- split the workspace into additional crates;
- redesign payload v3;
- add new cryptographic algorithms;
- add network services, daemons, or remote verification;
- increase release or CI ceremony.

---

## Final handoff condition

After Plan 050, the implementer must leave `main` with:

- corrected source and focused tests;
- complete status ledgers;
- current documentation that matches behavior;
- one recorded final `scripts/check.sh` result;
- any unavailable external-tool evidence recorded honestly;
- no publication side effects.

If any release-blocking acceptance criterion remains open, Plan 050 must end `PARTIAL` and name the exact corrective item. It must not mark the roadmap complete by increasing test count or documenting a required defect as an accepted limitation.