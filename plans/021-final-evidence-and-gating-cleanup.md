# Plan 021: Final Evidence and Gating Cleanup

## Status

Ready for implementation.

## Context

Plans 016–020 established canonical PLUS rights metadata, corrected legal-field semantics, added independent conformance infrastructure, introduced a manifest-backed fixture corpus, and hardened the primary CI and release workflows. The current repository is close to a credible v0.3.0 release candidate, but the final audit identified several remaining mismatches between the intended closure contract and the executable implementation.

The remaining defects are concentrated in evidence quality and release gating rather than the core image-writing path:

- The JSON artifact is still a bare array of per-fixture reports rather than a versioned run envelope.
- Expected `no_notice` outcomes do not inspect whether extraction actually returned no rights content.
- Independent canonical fixtures do not consistently require canonical DMI discovery.
- Coverage checks no longer enforce independently authored evidence after removal of the blanket percentage threshold.
- Manifest provenance validation permits contradictory or incomplete authoring information.
- External integration tests can return success when tools are unavailable and are not explicitly executed in the dedicated jobs.
- General internal/external field disagreements were globally downgraded to warnings to accommodate one Unicode edge case.
- Release validation still omits advisory checks and bypasses package verification with `--no-verify`.
- Plans 019 and 020 contain stale counts and claims that are not generated from the final report.
- A clean, inspected main-branch CI run and artifact record is still absent from the ledgers.

This plan is the final corrective pass before Release 4. It must not introduce the Release 4 policy architecture, new provenance algorithms, or unrelated refactors.

## Objective

At completion, StegoEggo must have a conformance and release-gating system that can truthfully demonstrate all of the following:

1. Every strict run produces one versioned, machine-readable run record, including failure and incomplete runs.
2. Expected-negative fixtures are evaluated against observed content, not only command success.
3. Independently authored fixtures prove canonical input compatibility and remain mandatory through coverage rules.
4. Fixture provenance is complete, internally consistent, and reproducible or explicitly documented as non-reproducible historical input.
5. External integration tests cannot silently skip in the jobs that claim to execute them.
6. Semantic disagreement is normalized narrowly and fails when a meaningful mismatch remains.
7. Main CI and release validation enforce the same package, security, and conformance contract.
8. Plans 016–020 are closed by generated evidence from a green main-branch run and inspected artifacts.

## Non-goals

This pass must not:

- Add `RightsPolicy`, `ProtectionChannels`, or replace `ProtectionLevel`.
- Change the hidden payload wire format.
- Add Ed25519, detached manifests, C2PA, or trust stores.
- Add image formats.
- Implement TDMRep deployment artifacts.
- Redesign the public verification model beyond fields needed for accurate conformance reporting.
- Perform broad module splitting unless required to make the conformance implementation testable.

## Primary affected files

Expected implementation work is concentrated in:

- `src/bin/stegoeggo-conformance.rs`
- `src/conformance.rs`
- `tests/conformance_harness_tests.rs`
- `tests/external_tools.rs`
- `tests/fixtures/conformance/manifest.toml`
- `tests/fixtures/conformance/README.md`
- fixture generator sources referenced by the manifest
- `scripts/verify_metadata_conformance.sh`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `architecture/conformance.md`
- `architecture/types.md`
- `plans/019-status.md`
- `plans/020-status.md`
- `plans/021-status.md`

## Workstream A: Introduce a versioned run report envelope

### A1. Define the report schema

Add a top-level serializable report type, for example:

```rust
pub struct ConformanceRunReport {
    pub schema_version: u32,
    pub generated_by: String,
    pub crate_version: String,
    pub commit_sha: Option<String>,
    pub strict: bool,
    pub complete: bool,
    pub passed: bool,
    pub started_at: Option<String>,
    pub manifest: Option<ManifestReport>,
    pub tools: Vec<ToolReport>,
    pub coverage_minimums: Option<CoverageMinimums>,
    pub coverage: Option<CoverageCheckResult>,
    pub digest_verification: Vec<DigestCheckResult>,
    pub summary: ConformanceSummary,
    pub incomplete_reasons: Vec<String>,
    pub fixtures: Vec<ConformanceReport>,
}
```

Exact names may differ, but the envelope must carry equivalent information.

### A2. Record tool identity and execution state

For each required tool, report:

- Logical name.
- Resolved executable path.
- Version string.
- Whether discovery succeeded.
- Whether the tool was exercised.
- Number of fixture invocations.
- Number of successes and failures.

Do not infer that a tool was exercised merely because its version command succeeded.

### A3. Record manifest identity

The report must include:

- Requested manifest path.
- Canonicalized path when available.
- Manifest SHA-256.
- Entry count.
- Validation result.
- Duplicate/unlisted/unexercised counts.

### A4. Write the report on every meaningful exit path

When `--json` is supplied, write a valid report for:

- Success.
- Fixture failures.
- Digest failure.
- Coverage failure.
- Missing required tools.
- Missing or invalid manifest.
- Missing or empty fixture directory.
- Unknown argument or invalid CLI value when a report path is already known.

Where argument parsing fails before `--json` is known, a JSON artifact is not required. Otherwise, failure must not prevent the report from being written.

### A5. Define `complete` and `passed`

- `complete = true` only if all required inputs and tools were available and every required check executed.
- `passed = true` only if `complete = true` and no required check failed.
- Non-strict runs that skip tools or omit a manifest must report `complete = false`, even if no observed check failed.

### Acceptance criteria

- The JSON artifact is one top-level object, not a bare fixture array.
- Schema version is explicit and tested.
- Digest and coverage results are present in the emitted JSON.
- Tool versions, paths, and execution counts are present.
- A strict successful run reports `complete = true` and `passed = true`.
- Every tested failure path still emits valid JSON when `--json` was supplied.

## Workstream B: Make expected-negative semantics content-aware

### B1. Add notice-content predicates

Implement helpers for internal and external extraction such as:

```rust
impl InternalExtraction {
    pub fn has_notice_content(&self) -> bool;
}

impl ExternalExtraction {
    pub fn has_notice_content(&self) -> bool;
}
```

The predicate must include all supported rights fields, canonical and legacy DMI, TDM legacy observations, creator lists, and any other field classified as rights-notice evidence.

### B2. Correct `ExtractionExpectation::NoNotice`

For internal extraction:

- Passing requires successful parser execution and `has_notice_content() == false`.
- Parser failure is not equivalent to no notice unless the fixture explicitly expects rejection.

For external extraction:

- Passing requires successful ExifTool execution and no extracted notice content.
- Empty process output, invalid JSON, or command failure is not a valid no-notice result.

### B3. Correct `ExtractionExpectation::Reject`

A rejection fixture passes only when the configured parser rejects the input in the defined way. A parser that succeeds and returns no notice is not equivalent to rejection unless the manifest explicitly permits either result through a separate expectation variant.

### B4. Remove or reconcile `expected_malformed`

`expected_malformed` is now superseded by typed decode/XMP/extraction expectations. Either:

- Remove it through a compatible manifest migration, or
- Validate that it is consistent with the typed expectations and reject contradictory entries.

### Acceptance criteria

- A no-notice fixture containing any rights field fails.
- An external command failure cannot satisfy `NoNotice`.
- A successful empty extraction cannot satisfy `Reject`.
- Contradictory malformed expectations fail manifest validation.
- Regression tests cover internal and external no-notice false positives.

## Workstream C: Strengthen independent canonical evidence

### C1. Audit independent canonical fixtures

For every fixture categorized as canonical and sourced as external, determine whether it is intended to contain canonical `plus:DataMining`.

Fixtures that contain canonical DMI must specify:

- `expected_dmi` with the exact PLUS vocabulary key.
- `required_external_fields = ["canonical_data_mining"]` or equivalent.
- An internal expectation requiring successful canonical parsing.

Fixtures that intentionally test non-DMI legal metadata should be renamed or described so they are not counted as canonical DMI evidence.

### C2. Require exact canonical namespace/value evidence

At least one external fixture per supported format must prove:

- Correct PLUS namespace URI.
- Canonical `DataMining` property.
- Exact controlled-vocabulary key or URI.
- Successful internal parsing.
- Successful ExifTool visibility.

At least one external fixture must use a non-default XML prefix to prove namespace-aware parsing.

### C3. Separate generated-output and external-input assertions

The suite must distinguish:

- StegoEggo-generated output visible to external tools.
- Externally authored input readable by StegoEggo.

Both directions are mandatory and reported separately in coverage.

### Acceptance criteria

- External canonical PNG, JPEG, and WebP fixtures carry non-empty DMI expectations where appropriate.
- At least one alternate-prefix fixture has an exact DMI expectation.
- The run report exposes generated-output and external-input coverage separately.
- Removing all independent canonical fixtures causes coverage failure.

## Workstream D: Add source-aware coverage requirements

### D1. Extend `CoverageMinimums`

Add source-aware minimums, for example:

- `external_canonical_png`.
- `external_canonical_jpeg`.
- `external_canonical_webp`.
- `external_legacy_min`.
- `external_alt_prefix_min`.
- `external_conflict_min` or a documented independent-conflict equivalent.
- `external_preservation_min`.

Use explicit category/format requirements rather than an aggregate percentage.

### D2. Validate coverage taxonomy

Coverage must be computed from validated manifest entries and must reject unknown source/category combinations. Generated historical fixtures must not be counted as independently authored merely because they emulate older output.

### D3. Report coverage observations

The report must include observed counts for every configured minimum, not only violations.

### Acceptance criteria

- A corpus containing only StegoEggo-generated fixtures fails strict coverage.
- External canonical coverage is required for all supported formats.
- Coverage reports include required and observed values.
- Current checked-in corpus passes after truthful classification corrections.

## Workstream E: Enforce fixture provenance integrity

### E1. Define provenance rules by source class

For `source = "external"` require:

- Non-empty authoring tool.
- Non-empty authoring tool version, or a specific `unknown-historical` sentinel with explanation.
- Reproducible command or checked-in generator path.
- License/provenance statement.
- A generation mechanism consistent with the declared authoring tool.

For `source = "historical"` require:

- Source release/tag/commit.
- Retrieval or reproduction procedure.
- Hash of the original fixture.

For `source = "generated"` require:

- Checked-in generator target or script.
- Current crate/tool version.

### E2. Correct contradictory entries

Entries such as `authoring_tool = "exiftool"` with `generation_command = "raw iTXt injection"` must be corrected to identify the actual injector script/tool. ExifTool may be recorded separately as the verification tool.

### E3. Validate generator references

Manifest validation must confirm that referenced checked-in generator paths exist. Commands that use Cargo test targets must reference existing targets.

### E4. Document non-reproducible fixtures

If a fixture cannot be regenerated exactly, document why and preserve the original digest. Do not present it as reproducible.

### Acceptance criteria

- Empty external tool versions are rejected unless an explicit historical exception is documented.
- Contradictory authoring tool/command combinations are rejected or corrected.
- Every generated fixture references an existing generator.
- Fixture README and manifest agree on provenance.

## Workstream F: Make external integration tests mandatory where claimed

### F1. Separate hermetic and external tests

Choose one explicit pattern:

Preferred:

- Mark tests in `tests/external_tools.rs` with `#[ignore = "requires external tools"]`.
- Run them explicitly in the conformance jobs with `cargo test --test external_tools -- --ignored`.

Alternative:

- Gate them behind a private test feature or environment variable.
- Dedicated jobs must enable the gate and fail if tools are absent.

### F2. Eliminate silent success in mandatory mode

When the dedicated job invokes external tests, tool absence must panic/fail with a clear message. A helper may permit skips only in the ordinary hermetic test job.

### F3. Run the same command in CI and release validation

Both workflows must run the external integration test target before the conformance harness.

### Acceptance criteria

- Ordinary `cargo test --all-features` remains hermetic.
- Dedicated external jobs execute every external integration test.
- Missing tools fail the dedicated run.
- CI logs clearly show the number of external tests executed.

## Workstream G: Restore meaningful mismatch severity

### G1. Replace global warning downgrade

Do not classify every internal/external legal-field disagreement as a warning. Implement field-specific normalization and comparison:

- Unicode normalization where applicable.
- Language-alternative container normalization.
- Whitespace normalization only where semantically safe.
- URL normalization limited to well-defined equivalent forms.
- Creator-list comparison preserving order rules defined by the metadata model.

### G2. Handle mojibake explicitly

For the known Unicode fixture, determine whether the defect is:

- StegoEggo encoding.
- Container extraction.
- ExifTool decoding.
- Test-fixture authoring.

Fix the actual issue when possible. If a tool-specific limitation remains, scope the exception to the exact fixture, field, tool version range, and observed transformation. Do not weaken unrelated fields.

### G3. Compare external values to manifest expectations

Where the manifest specifies a legal field and requires it externally, compare the external normalized value directly to the expected value. Agreement with the internal parser alone is insufficient.

### Acceptance criteria

- Meaningful legal-field mismatches fail.
- The Unicode exception, if retained, is narrowly scoped and documented.
- External expected legal fields are asserted directly.
- Tests cover Unicode, whitespace, URL, localized text, and creator arrays.

## Workstream H: Finish release-workflow parity

### H1. Add advisory checking

Release validation must run both:

```bash
cargo deny check licenses
cargo deny check advisories
```

or a single documented equivalent command that covers both policies.

### H2. Remove package-verification bypass

Replace:

```bash
cargo package --workspace --allow-dirty --no-verify
```

with the same verified package command used in CI unless a specific crate cannot be verified. Any exception must be documented in the workflow and status ledger with a follow-up issue.

### H3. Centralize the release validation contract

Use a checked-in script or reusable workflow/composite action for:

- Formatting.
- Clippy.
- Tests and doc tests.
- Package verification.
- License/advisory checks.
- External integration tests.
- Strict conformance.

Avoid duplicated command drift between `ci.yml` and `release.yml`.

### H4. Preserve failure artifacts

Conformance and tool-version artifacts must upload on failure. The run envelope must remain useful when strict checks fail.

### Acceptance criteria

- Release workflow runs advisory checks.
- Package verification is not bypassed without a documented exception.
- CI and release use the same checked-in validation entrypoint or provably identical commands.
- Failed conformance still uploads a complete run report.

## Workstream I: Correct ledgers and close with generated evidence

### I1. Make status counts generated

Do not hard-code fixture or test counts in status files without linking them to a specific run artifact. Record counts from the final run report.

### I2. Correct Plan 019 and Plan 020 claims

Update both ledgers so every item is one of:

- `PASS` with exact evidence.
- `DEFERRED` with rationale and owning future plan.
- `FAIL` if unresolved.

Remove claims that external tests are ignored or explicitly executed unless the final implementation does so.

### I3. Add `plans/021-status.md`

Record:

- Implementation commit SHAs.
- Exact validation commands.
- Main CI run ID and URL.
- Job conclusions.
- Artifact names and IDs.
- Tool versions.
- Report schema version.
- Fixture totals by format/category/source.
- Digest and coverage results.
- Remaining deferred scope.

### I4. Inspect artifacts

Closure requires inspecting, not merely uploading:

- `conformance-report`.
- `tool-versions`.

Confirm the report shows:

- `complete = true`.
- `passed = true`.
- Non-zero fixtures.
- All digests matching.
- All coverage gates passing.
- All required tools exercised.
- No skipped mandatory suite.

### Acceptance criteria

- Plans 019–021 contain consistent counts tied to one green run.
- A clean main-branch CI run is recorded.
- Artifact IDs or stable names are recorded.
- Plans 016–020 are marked closed only after artifact inspection.

## Required regression tests

Add tests covering at least:

### Run report

- Successful strict run envelope serialization.
- Failure envelope serialization.
- Schema version stability.
- `complete = false` when a tool is skipped.
- Digest failure present in JSON.
- Coverage failure present in JSON.
- Tool execution counts present.
- Manifest hash present.

### Expected-negative semantics

- Internal `NoNotice` with actual notice fails.
- External `NoNotice` with actual notice fails.
- Command failure does not satisfy `NoNotice`.
- Empty JSON array does not satisfy `NoNotice`.
- Successful no-content extraction satisfies `NoNotice`.
- `Reject` is distinct from no notice.

### Coverage

- Missing external canonical PNG fails.
- Missing external canonical JPEG fails.
- Missing external canonical WebP fails.
- Generated-only corpus fails.
- External alternate-prefix evidence is required.

### Provenance

- Empty external tool version rejected.
- Contradictory authoring command rejected.
- Missing generator path rejected.
- Historical fixture missing source revision rejected.

### Comparison semantics

- Unicode-equivalent strings pass.
- Meaningfully different Unicode strings fail.
- URL-equivalent normalization passes only for documented cases.
- Different rights URLs fail.
- Creator array mismatch fails where order/content is significant.

### Workflow contract

Where practical, add a script-level test or validation that both workflows invoke the same checked-in validation entrypoint.

## Final validation sequence

### Hermetic phase

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --no-fail-fast
cargo test --doc
cargo package --workspace --allow-dirty
cargo deny check licenses
cargo deny check advisories
```

### External phase

```bash
cargo test --test external_tools -- --ignored
cargo build --release --bin stegoeggo-conformance
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --json conformance-report.json
```

### Artifact assertions

Use a script or test to assert:

```text
schema_version >= 1
complete == true
passed == true
summary.total > 0
summary.failed == 0
all digest_verification.matches == true
coverage.passed == true
all required tools exercised == true
incomplete_reasons is empty
```

### Negative phase

Prove non-zero exit and valid failure JSON for:

- Missing manifest.
- Empty fixture directory.
- Missing ExifTool.
- Missing ImageMagick.
- Missing libvips.
- Digest mismatch.
- Coverage deficit.
- Expected DMI mismatch.
- Expected no-notice mismatch.
- Invalid provenance entry.

## Completion definition

Plan 021 is complete only when:

1. The versioned run envelope is implemented and emitted on success and failure.
2. Expected-negative semantics inspect actual extracted content.
3. Source-aware external coverage is mandatory.
4. Independent canonical fixtures assert canonical DMI in both directions.
5. Provenance validation is strict and the manifest is truthful.
6. Mandatory external tests cannot silently skip.
7. Meaningful metadata disagreements fail after narrow normalization.
8. Release validation includes package verification and advisory checks.
9. A green main CI run and inspected artifacts are recorded in the ledgers.
10. Plans 016–020 can be marked closed without qualification other than explicitly owned future-release deferrals.

Release 4 work must not begin until this plan is closed, because Releases 4–6 are required to preserve the established canonical metadata and external-conformance baseline.