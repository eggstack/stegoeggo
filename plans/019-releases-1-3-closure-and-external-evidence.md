# Plan 019: Releases 1–3 Closure and External-Evidence Pass

## Status

Ready for implementation.

## Purpose

Plans 016–018 established the standards-correctness critical path for StegoEggo:

- Plan 016: canonical PLUS rights metadata and legacy compatibility.
- Plan 017: legal metadata semantic correctness and cross-format consistency.
- Plan 018: independent interoperability and conformance.

The implementation commits for those plans landed the majority of the intended functionality. The repository now emits canonical `plus:DataMining`, parses legacy StegoEggo DMI forms, detects canonical/legacy disagreement, no longer emits the project-defined image-level TDM property, corrects several legal-field mappings, introduces cross-format semantics and metadata update policies, and provides a Rust conformance harness integrated into CI and release workflows.

This closure pass exists because implementation breadth is not the same as demonstrated closure. The current repository still lacks sufficient checked-in external evidence to prove that the conformance suite exercises independently authored assets, covers every required fixture category, fails closed on zero coverage, and executes all required external parsers in clean CI. Several secondary correctness and maintainability issues also remain, including ambiguous TDMRep scope, a permissive release gate, and a brittle positional verification constructor.

The objective of this plan is to close Plans 016–018 with auditable evidence and explicit acceptance criteria. It must not expand into Release 4 architecture work except where a narrowly scoped refactor is necessary to make the existing implementation safe and testable.

## Outcome

At completion:

1. The repository contains real, provenance-documented metadata fixtures authored or modified by tools other than StegoEggo.
2. Strict conformance fails when required tools, formats, categories, or fixtures are absent.
3. CI executes the complete external conformance matrix and uploads an inspectable machine-readable report.
4. Canonical PLUS output, legacy input compatibility, semantic field mappings, conflict handling, metadata preservation, and idempotence are all demonstrated against external evidence.
5. TDMRep web-distribution support is either implemented as a narrowly scoped deployment artifact generator or explicitly deferred with the public API and documentation adjusted accordingly.
6. Release gating is internally consistent and cannot pass required checks through `continue-on-error` or skipped coverage.
7. `NoticeVerification` construction no longer depends on a large positional argument list.
8. Plans 016–018 receive a final acceptance ledger that records each criterion as passed, deferred, or failed with evidence.

## Non-goals

This pass must not:

- Introduce the Release 4 `RightsPolicy` and protection-channel architecture.
- Redesign steganographic embedding algorithms.
- Add Ed25519, C2PA, detached manifests, or payload-version changes.
- Expand supported image formats.
- Add network retrieval or live policy validation.
- Claim that metadata proves copyright ownership or model-training use.
- Replace PLUS rights signaling with TDMRep.
- Add broad CLI UX redesign unrelated to closure.
- Perform large module reorganization unless required to safely replace the positional constructor.

## Current baseline

The implementation after Plan 018 includes:

- Canonical `plus:DataMining` output with PLUS vocabulary mappings.
- Legacy DMI parsing and canonical/legacy conflict detection.
- Removal of newly emitted `tdm:reserve_tdm` image metadata.
- Corrected legal-claim activation and elimination of synthetic legal defaults.
- Separation of contact from `photoshop:Credit`.
- Distinct credit, owner, licensor, metadata-date, and notice-applied fields.
- Cross-format semantic tests.
- Metadata update policies and merge tests.
- `stegoeggo-conformance` with JSON reporting.
- ExifTool and `xmllint` integration.
- CI and release workflow conformance jobs.

The main closure risks are:

- The checked-in fixture tree appears to contain taxonomy documentation but not a sufficient set of actual independently authored assets.
- Strict mode may not enforce minimum per-format and per-category coverage.
- ImageMagick and libvips are not both mandatory in CI.
- No successful workflow evidence is presently attached to the implementation head.
- TDMRep deployment artifacts from Plan 016 remain ambiguous.
- The release workflow retains at least one required check with `continue-on-error`.
- `NoticeVerification::new()` has grown into a high-risk positional constructor.

## Workstream A: Establish a real external fixture corpus

### A1. Define fixture manifest format

Add a machine-readable fixture manifest under:

```text

tests/fixtures/conformance/manifest.toml
```

Each fixture entry must include:

- Stable fixture identifier.
- Relative file path.
- Image format.
- Fixture category.
- Expected canonical DMI value, if any.
- Expected legacy DMI value, if any.
- Expected conflict state.
- Expected legal fields.
- Expected malformed state.
- Expected preservation fields.
- Authoring tool.
- Authoring tool version.
- Exact generation or mutation command.
- Source image provenance.
- License or repository ownership statement.
- SHA-256 digest of the checked-in fixture.

Suggested schema:

```toml
[[fixture]]
id = "canonical-plus-png-exiftool"
path = "canonical/exiftool-plus.png"
format = "png"
category = "canonical"
authoring_tool = "ExifTool"
authoring_tool_version = "13.x"
generation_command = "..."
source = "repository-generated 16x16 test image"
license = "CC0 / repository-owned test fixture"
sha256 = "..."
expected_dmi = "DMI-PROHIBITED-AIMLTRAINING"
expected_conflict = false
```

The harness must load this manifest instead of inferring expectations solely from filenames.

### A2. Add canonical external fixtures

Check in at least one externally authored canonical fixture for each supported format:

- PNG.
- Baseline JPEG.
- Progressive JPEG.
- WebP.

Each must contain canonical `plus:DataMining` with a full PLUS vocabulary URI and at least two standard legal fields, such as:

- `dc:rights`.
- `dc:creator`.
- `xmpRights:UsageTerms`.
- `xmpRights:WebStatement`.
- `photoshop:Credit` used only as a credit line.

At least one fixture must use attribute-form RDF and at least one must use element-form RDF.

At least one fixture must use a non-`plus` namespace prefix bound to the correct PLUS namespace URI. Verification must be namespace-aware rather than prefix-dependent.

### A3. Add legacy fixtures

Check in representative files produced by the pre-0.3 implementation or reconstructed from an exact archived output path:

- Legacy `Iptc4xmpExt:DMI-Prohibited`.
- Legacy `Iptc4xmpExt:DMI-Allowed`.
- Legacy generic DMI property.
- Legacy `tdm:reserve_tdm` image property.
- At least one file containing both legacy rights and legal notice fields.

Legacy fixtures must be parsed but never reproduced by default output.

### A4. Add conflict fixtures

Create externally mutated files containing:

- Canonical prohibited plus legacy allowed.
- Canonical allowed plus legacy prohibited.
- Two canonical `plus:DataMining` values that disagree.
- Duplicate XMP packets with different rights policies.
- Conflicting ownership or rights-statement fields.

The verifier must report the conflict deterministically and must not silently select the first value.

### A5. Add malformed and unknown-value fixtures

Add fixtures covering:

- Truncated XMP.
- Invalid RDF/XML.
- Correct XML with an unknown PLUS vocabulary URI.
- Correct namespace with malformed property structure.
- Wrong namespace with a familiar `DataMining` local name.
- Oversized metadata within configured test bounds.
- Duplicate properties with identical values.

Unknown values must remain distinguishable from malformed metadata and from absence.

### A6. Add preservation fixtures

Add externally authored images containing unrelated metadata that must survive StegoEggo processing:

- Camera EXIF fields.
- Orientation.
- ICC profile.
- Existing creator fields.
- Existing non-StegoEggo XMP namespace.
- JPEG IPTC record.
- WebP ancillary metadata.

Preservation fixtures should include a before/after normalized metadata snapshot generated by external tools.

### A7. Fixture integrity enforcement

The conformance harness must verify every fixture digest against the manifest before execution. Digest mismatch is a hard failure in strict mode.

No test may rewrite checked-in fixtures in place.

## Workstream B: Make conformance fail closed

### B1. Enforce minimum coverage counts

Strict mode must fail unless all minimums are met:

- Canonical PNG fixtures: at least 1.
- Canonical baseline JPEG fixtures: at least 1.
- Canonical progressive JPEG fixtures: at least 1.
- Canonical WebP fixtures: at least 1.
- Legacy fixtures: at least 3 across at least 2 formats.
- Conflict fixtures: at least 3.
- Malformed or unknown-value fixtures: at least 4.
- Preservation fixtures: at least 3, covering PNG, JPEG, and WebP.
- Externally authored fixtures: at least 75% of the corpus.

The report must expose counts by format, category, authoring tool, and result.

### B2. Distinguish generated smoke tests from independent fixtures

Generated StegoEggo outputs remain useful, but they must be classified as `generated-smoke`, not `external-fixture`.

A strict conformance pass must include both:

1. StegoEggo-generated output read by external tools.
2. Externally authored input read and updated by StegoEggo.

The report must show these as separate suites.

### B3. Required-tool behavior

In strict mode, absence or execution failure of a required tool is fatal.

Required tools:

- ExifTool.
- `xmllint` or an equivalent independently maintained namespace-aware XML validator.
- ImageMagick.
- libvips.

Non-strict local mode may skip unavailable tools, but the report must clearly mark the suite incomplete and return a distinct non-success state when `--require-complete` is set.

### B4. Parser independence rules

The harness must not use StegoEggo parsing logic to construct the expected external result.

Expected values come from the fixture manifest. Internal and external extraction are compared independently against the manifest.

The validation sequence for each fixture is:

1. Verify fixture digest.
2. Read expected values from manifest.
3. Run StegoEggo internal extraction.
4. Run ExifTool extraction.
5. Validate XMP XML independently.
6. Run ImageMagick decode/identify.
7. Run libvips decode/header inspection.
8. Compare each result against manifest expectations.
9. Compare internal and external normalized fields.
10. Record all discrepancies without early termination so the JSON report is complete.

### B5. Zero-case and skip detection

The harness must fail strict mode when:

- Fixture discovery returns zero files.
- Any required category has zero files.
- Any supported format has zero canonical cases.
- A required external-tool suite executes zero assertions.
- All cases in a suite are skipped.
- The JSON report cannot be written.
- The external parser returns no usable metadata for a fixture expected to contain metadata.

Add explicit regression tests for each fail-closed condition.

## Workstream C: Close canonical PLUS and semantic correctness

### C1. Verify exact URI handling

Add table-driven tests for every supported `DmiValue` mapping:

- Enum to full canonical PLUS URI.
- Full URI to enum.
- Vocabulary key to enum where compatibility parsing is supported.
- Unknown URI handling.
- Case sensitivity.
- Whitespace normalization policy.

The emitter must always write the full canonical URI expected by the governing vocabulary. Human-readable keys must not replace URIs in serialized XMP.

### C2. Namespace-aware extraction

Replace any prefix-dependent string matching in the conformance path with namespace-aware parsing.

At minimum, test:

```xml
<plus:DataMining>...</plus:DataMining>
<p:DataMining xmlns:p="http://ns.useplus.org/ldf/xmp/1.0/">...</p:DataMining>
<rdf:Description p:DataMining="..." ... />
```

Use an XML parser capable of resolving namespace URIs. Lightweight byte scanning may remain as a defensive fallback for malformed legacy files, but canonical conformance must be parser-based.

### C3. Legal-field external visibility matrix

For each format, define the expected external visibility of every supported legal field:

- Copyright notice.
- Creator.
- Contact.
- Credit line.
- Copyright owner.
- Licensor name.
- Licensor email.
- Licensor URL.
- License URL.
- Web statement.
- Usage terms.
- Content creation date.
- Metadata date.
- Notice-applied timestamp.
- AI constraints.
- Canonical data-mining policy.

The matrix must identify:

- Canonical XMP property.
- Container-specific fallback, if any.
- Whether ExifTool exposes it.
- Whether StegoEggo verifies it.
- Whether round-trip preservation is required.

No field may be represented as supported in public documentation unless the matrix and tests demonstrate its actual output and extraction behavior.

### C4. URL validation closure

Ensure all URL-bearing fields receive syntax validation before injection:

- License URL.
- Web statement URL.
- Licensor URL.
- Any future TDM policy URL.

Validation must reject malformed, control-character-containing, or whitespace-corrupted values while allowing valid non-HTTP URI schemes only where the field standard permits them.

Network availability is not checked.

### C5. Date semantics closure

Add tests proving:

- `creation_date` is emitted only when caller supplied.
- `metadata_date` and `notice_applied_at` are distinct.
- Automatically generated notice timestamps are RFC 3339 and deterministic under an injected test clock.
- Reprocessing under `PreserveExisting` does not unexpectedly replace content creation dates.
- Reprocessing under `ReplaceStegoOwned` updates only fields owned by StegoEggo according to documented policy.

Avoid wall-clock-dependent assertions.

## Workstream D: Preservation, merge, and idempotence

### D1. Formalize metadata ownership

Document which fields are considered StegoEggo-owned and which are externally owned.

StegoEggo-owned examples:

- Project namespace diagnostics.
- Notice-applied timestamp generated by StegoEggo.
- Canonical DMI value explicitly requested for the operation.

Externally owned examples:

- Existing camera EXIF.
- Existing creator and copyright values not explicitly replaced.
- ICC profiles.
- Unknown XMP namespaces.

The merge policy must operate on this ownership model rather than on broad packet replacement.

### D2. Idempotence tests

For each format and each update policy:

- Apply the same notice twice.
- Verify output contains one effective canonical DMI value.
- Verify no unbounded growth in XMP packets or container metadata blocks.
- Verify normalized legal fields are unchanged.
- Verify unrelated metadata is preserved.

Byte-for-byte identity is not required where timestamps or container ordering legitimately change, but normalized metadata must be stable.

### D3. Conflict-policy tests

`FailOnConflict` must return a typed error before mutating output.

`PreserveExisting` must retain preexisting conflicting values and report that the requested notice was not fully applied.

`ReplaceStegoOwned` must replace only fields defined as StegoEggo-owned and report unresolved external conflicts.

### D4. Decode validation

Every updated fixture must remain decodable by:

- The Rust `image` crate where supported.
- ImageMagick.
- libvips.

Progressive JPEG fixtures must remain progressive if the operation is metadata-only and does not require image re-encoding.

## Workstream E: Resolve TDMRep scope

Choose exactly one of the following closure paths and document the decision in an ADR or dedicated section of the plan status record.

### Option E1: Implement deployment artifact generation

Add a small, deterministic TDMRep deployment module that can generate:

- `tdm-reservation` header value.
- Optional `tdm-policy` header value.
- `/.well-known/tdmrep.json` body.
- Optional static-server configuration snippets only if they are generated from the same core model.

Requirements:

- No network server is added.
- No image XMP TDM property is emitted.
- Artifacts are generated from an explicit configuration object.
- JSON and headers are covered by specification-derived fixtures.
- CLI support, if added, is a separate subcommand or explicit output mode.

### Option E2: Explicitly defer TDMRep deployment artifacts

If deployment artifact generation is not appropriate for Release 1 closure:

- Record it as a deferred roadmap item.
- Remove or deprecate CLI flags that imply image-level TDMRep support, including ambiguous `--tdm-reserved` behavior.
- Ensure documentation states that StegoEggo currently emits PLUS image metadata only.
- Keep legacy TDM parsing strictly for compatibility diagnostics.
- Add a migration note describing the removal or reinterpretation of legacy flags.

Acceptance requires an explicit choice. Leaving the current state ambiguous is not acceptable.

## Workstream F: Harden CI and release gates

### F1. Mandatory dependency installation

Update the conformance CI job to install:

- ExifTool.
- libxml2 utilities or chosen XML validator.
- ImageMagick.
- libvips tools.

Print tool versions into the log and JSON report.

Pin package versions only where the project has a reproducibility mechanism that can maintain them. Otherwise record resolved versions in artifacts.

### F2. Required test commands

CI must run at least:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc
cargo package --workspace --allow-dirty
cargo deny check licenses
cargo deny check advisories
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --require-complete \
  --json conformance-report.json
```

### F3. Remove permissive required gates

Remove `continue-on-error: true` from all checks described as release requirements, including license checks.

Optional informational jobs must be named and documented as optional rather than appearing in the required release gate.

### F4. Artifact retention

Upload:

- Conformance JSON report.
- Human-readable summary.
- External tool version manifest.
- Normalized before/after preservation snapshots.
- Failure logs when a conformance case fails.

Artifacts must upload under `if: always()` while the job still fails when required checks fail.

### F5. Local reproduction script

The shell wrapper should provide one command that reproduces CI behavior locally and clearly reports missing dependencies.

Do not duplicate conformance logic in shell.

## Workstream G: Replace the positional verification constructor

### G1. Introduce structured internal model

Replace the 26-argument `NoticeVerification::new()` call pattern with structured components.

Recommended shape:

```rust
pub struct ExtractedRightsNotice {
    pub data_mining: Vec<RightsSignal>,
    pub copyright_notice: Option<String>,
    pub creators: Vec<String>,
    pub contact: Option<String>,
    pub credit_line: Option<String>,
    pub copyright_owner: Option<String>,
    pub licensor: Option<Licensor>,
    pub license_url: Option<String>,
    pub web_statement: Option<String>,
    pub usage_terms: Option<String>,
    pub creation_date: Option<String>,
    pub metadata_date: Option<String>,
    pub notice_applied_at: Option<String>,
    pub ai_constraints: Option<String>,
}

pub struct VerificationDiagnostics {
    pub conflicts: Vec<NoticeConflict>,
    pub malformed_fields: Vec<MalformedField>,
    pub warnings: Vec<VerificationWarning>,
}

pub struct NoticeVerification {
    pub notice: ExtractedRightsNotice,
    pub evidence: EvidenceAssessment,
    pub diagnostics: VerificationDiagnostics,
}
```

Exact names may vary, but construction must be field-named and type-checked.

### G2. Compatibility

Preserve public getters where practical.

If `NoticeVerification::new()` is public, deprecate it or replace it in the intended `0.3.0` breaking release with a documented migration path.

Add compile-time and runtime tests demonstrating that contact, credit, owner, licensor, date, and URL fields cannot be silently swapped by positional order.

### G3. Scope restraint

Do not turn this into the full Release 4 type-system refactor. The goal is to remove positional-construction risk and create a safe base for later work.

## Workstream H: Acceptance ledger and status documentation

### H1. Create closure status file

Add:

```text
plans/019-status.md
```

The status file must list every acceptance criterion from Plans 016, 017, 018, and this plan with:

- State: `PASS`, `DEFERRED`, or `FAIL`.
- Evidence command.
- Evidence file or test name.
- Commit SHA.
- Notes explaining any limitation.

No criterion may be marked `PASS` based only on code inspection when an executable or external-evidence requirement exists.

### H2. Record CI evidence

Record:

- Successful workflow URL or run identifier.
- Commit SHA tested.
- External tool versions.
- Fixture counts.
- Report artifact name.
- Total passes, warnings, failures, and skips.

If the environment cannot run a required external tool, the status remains `FAIL` or `DEFERRED`; it must not be silently converted into a pass.

### H3. Update plan statuses

Update Plans 016–018 status sections only after the closure ledger demonstrates their acceptance criteria.

Recommended final states:

- `Closed` only when all mandatory criteria pass.
- `Closed with explicit deferral` only for noncritical TDMRep deployment scope if Option E2 is chosen and documented.
- `Partially implemented` if external evidence remains unavailable.

## Ordered implementation sequence

Execute in this order:

1. Add fixture manifest schema and loader.
2. Add real external canonical, legacy, conflict, malformed, and preservation fixtures.
3. Add digest verification and minimum coverage enforcement.
4. Separate generated smoke tests from external fixture tests.
5. Make strict mode fail on zero cases, skips, missing tools, and incomplete formats.
6. Add mandatory ImageMagick and libvips execution.
7. Complete namespace-aware canonical PLUS parsing and exact URI tests.
8. Complete legal-field visibility, URL validation, and date-semantic tests.
9. Complete preservation, merge, conflict-policy, and idempotence tests.
10. Resolve TDMRep through Option E1 or E2.
11. Replace the positional `NoticeVerification` constructor.
12. Tighten CI and release gates.
13. Run the complete clean-environment validation matrix.
14. Add `plans/019-status.md` with evidence.
15. Update Plans 016–018 statuses only after evidence review.

## Required test matrix

### Canonical rights tests

- Every supported DMI variant emits the exact canonical PLUS URI.
- Canonical element-form RDF parses.
- Canonical attribute-form RDF parses.
- Alternate namespace prefixes parse.
- Unknown canonical URI remains unknown.
- Legacy values parse but are not emitted.
- Canonical/legacy disagreement produces conflict.
- Duplicate identical values do not produce a false conflict.

### Format tests

For PNG, baseline JPEG, progressive JPEG, and WebP:

- External canonical fixture parses internally.
- External canonical fixture parses through ExifTool.
- XML validates where applicable.
- ImageMagick decodes.
- libvips decodes.
- Applying an equivalent notice preserves unrelated metadata.
- Applying the same notice twice is semantically idempotent.

### Semantic field tests

- Contact never maps to credit line.
- Credit line maps to `photoshop:Credit` where XMP is used.
- Copyright owner is distinct from copyright notice.
- Licensor fields remain distinct from creator and contact.
- License URL is distinct from web statement.
- Content creation date is not synthesized.
- Metadata date and notice-applied timestamp remain distinct.
- Invalid URLs are rejected before output mutation.

### Failure tests

- Empty fixture directory fails strict mode.
- Missing manifest fails strict mode.
- Digest mismatch fails strict mode.
- Missing required category fails strict mode.
- Missing required format fails strict mode.
- Missing ExifTool fails strict mode.
- Missing XML validator fails strict mode.
- Missing ImageMagick fails strict mode.
- Missing libvips fails strict mode.
- External parser produces no expected fields and fails.
- JSON artifact write failure returns failure.
- All-skipped external suite returns failure.

### Release tests

- Required license check failure fails release job.
- Conformance failure fails release job.
- Artifacts upload on failure.
- No publication step can run before conformance succeeds.

## Explicit acceptance criteria

This plan is complete only when all mandatory criteria below are satisfied.

### Fixture evidence

- [ ] `manifest.toml` exists and validates every checked-in conformance fixture.
- [ ] Fixture digests are verified before test execution.
- [ ] Canonical external fixtures exist for PNG, baseline JPEG, progressive JPEG, and WebP.
- [ ] Legacy fixtures exist across at least two formats.
- [ ] Conflict, malformed, unknown-value, and preservation categories contain real assets.
- [ ] At least 75% of strict-suite fixtures were not authored by StegoEggo.
- [ ] Fixture provenance and generation commands are documented.

### Conformance behavior

- [ ] Strict mode fails on zero coverage.
- [ ] Strict mode enforces per-format and per-category minimums.
- [ ] Generated smoke tests and external fixtures are reported separately.
- [ ] ExifTool, XML validation, ImageMagick, and libvips are mandatory in CI.
- [ ] Internal and external extraction compare independently against manifest expectations.
- [ ] JSON report records tool versions, coverage counts, skips, warnings, and failures.

### Standards and semantics

- [ ] Every supported DMI value round-trips through the exact canonical PLUS URI.
- [ ] Canonical parsing is namespace-aware and prefix-independent.
- [ ] Legacy properties remain readable but are not emitted by default.
- [ ] Conflicting rights signals are reported without silent precedence.
- [ ] Publicly documented legal fields have verified external visibility or a clearly documented format limitation.
- [ ] URL-bearing fields receive syntax validation.
- [ ] Date semantics are deterministic and independently tested.

### Preservation and mutation

- [ ] Unrelated EXIF, XMP, IPTC, orientation, ICC, and ancillary metadata are preserved according to policy.
- [ ] Repeated application is semantically idempotent.
- [ ] Merge policies have explicit externally verified behavior.
- [ ] All updated assets remain decodable by Rust, ImageMagick, and libvips.
- [ ] Metadata-only progressive JPEG updates preserve progressive encoding.

### TDMRep

- [ ] Option E1 or E2 is explicitly selected and documented.
- [ ] No newly emitted image metadata claims to be W3C TDMRep unless the standard explicitly defines it for that container.
- [ ] Legacy image TDM properties remain compatibility diagnostics only.
- [ ] CLI and documentation match the selected scope.

### API and release safety

- [ ] The large positional `NoticeVerification` constructor is removed or deprecated in favor of named structured construction.
- [ ] Existing public getters have a documented migration path.
- [ ] Required release checks do not use `continue-on-error`.
- [ ] A clean CI run passes the complete matrix at the implementation head.
- [ ] Conformance artifacts are uploaded and inspected.
- [ ] `plans/019-status.md` records evidence for every criterion.
- [ ] Plans 016–018 statuses are updated based on evidence rather than implementation intent.

## Final release gate

The closure pass is successful only if the following command sequence succeeds in a clean Ubuntu environment with no preexisting build artifacts:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc
cargo package --workspace --allow-dirty
cargo deny check licenses
cargo deny check advisories
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --require-complete \
  --json conformance-report.json
```

Additionally:

- The report must contain nonzero cases for every mandatory format and category.
- No required external suite may be skipped.
- No required check may be downgraded to informational.
- The checked-in fixture corpus must pass digest verification.
- The status ledger must identify the exact tested commit.

## Handoff notes

The implementing agent should treat this as a closure and evidence-gathering pass, not a feature-expansion pass. The most important deliverable is not additional code volume; it is a reproducible chain of evidence showing that StegoEggo emits and consumes standards-correct metadata across independent tools and externally authored files.

When a requirement cannot be demonstrated, preserve the failure in `plans/019-status.md`. Do not weaken strict mode, reduce fixture minimums, classify generated assets as external, or convert missing-tool failures into skips merely to obtain a green workflow.
