# Plan 020: Final Conformance and Release Closure

## Status

Ready for implementation.

## Context

Plans 016–018 established the standards-correctness critical path for StegoEggo. Plan 019 then defined a closure pass intended to convert the implementation into auditable external evidence. Commit `6da73120fc7ba6cf98ab783cb9efe1abf2c91cb9` implemented most of that surface area: a fixture manifest, fixture digests, broader semantic and preservation tests, a `NoticeVerification` builder, external-tool installation in CI, TDMRep deferral, and additional documentation.

The repository is substantially stronger, but the closure layer is not yet trustworthy enough to gate a release. The current conformance harness can return success when the fixture directory is missing or empty, does not evaluate per-fixture expectations from the manifest, records external extraction as successful even when ExifTool fails, installs ImageMagick and libvips without exercising them against fixtures, and uses an aggregate external-fixture percentage that does not match the checked-in corpus. The release workflow also remains out of parity with the main CI conformance job and retains a permissive license gate.

This plan is a narrowly scoped corrective closure pass. Its purpose is to eliminate false-green conformance outcomes, make the manifest authoritative, align normal CI and release validation, complete fixture provenance, and produce auditable closure evidence for Plans 016–019.

## Objective

At completion, StegoEggo must have a fail-closed conformance system in which:

1. Strict mode cannot succeed without a valid manifest, a non-empty fixture corpus, every required tool, and complete category/format coverage.
2. Every fixture is evaluated against explicit manifest expectations rather than only against agreement between StegoEggo and ExifTool.
3. Expected-negative fixtures pass only when the expected failure is observed.
4. External parser absence, command failure, empty output, or missing required canonical fields is a conformance failure.
5. ImageMagick and libvips are actually exercised against all expected-valid fixtures.
6. Main CI and release validation execute the same conformance contract.
7. The resulting JSON artifact contains tool versions, manifest/digest status, coverage, and per-fixture outcomes in one report.
8. Plans 016–019 have an acceptance ledger backed by commands, commit SHAs, CI runs, and uploaded artifacts.

## Non-goals

This pass must not:

- Begin Release 4 `RightsPolicy` or evidence-channel architecture work.
- Add new steganographic algorithms, payload versions, cryptographic signatures, C2PA, or detached manifests.
- Add new image formats.
- Implement TDMRep deployment artifacts; Plan 019 selected explicit deferral and this pass only verifies that the deferral is consistent.
- Perform a broad module reorganization unrelated to conformance correctness.
- Change canonical PLUS vocabulary semantics except to fix a demonstrated parsing or normalization defect.

## Primary affected files

Expected implementation work is concentrated in:

- `src/bin/stegoeggo-conformance.rs`
- `src/conformance.rs`
- `scripts/verify_metadata_conformance.sh`
- `tests/external_tools.rs`
- `tests/semantic_correctness.rs`
- `tests/preservation_idempotence.rs`
- `tests/fixtures/conformance/manifest.toml`
- `tests/fixtures/conformance/README.md`
- `tests/fixtures/conformance/**`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `README.md`
- `architecture/conformance.md`
- `architecture/types.md`
- `AGENTS.md`
- `CHANGELOG.md`
- `plans/019-status.md`
- `plans/020-status.md`

## Workstream A: Make strict mode fail closed

### A1. Define one strict-mode contract

`--strict` must mean all of the following:

- A manifest argument is required.
- The manifest path must exist and be readable.
- The manifest must parse successfully.
- The manifest must contain at least one fixture.
- The fixture directory must exist and be readable.
- At least one supported image fixture must be discovered.
- Every manifest entry must point to one existing file.
- Every discovered supported image file must have exactly one manifest entry.
- Every required external tool must be present and executable.
- Every required coverage minimum must pass.
- Every fixture digest must match.
- Every fixture expectation must pass.
- No required external check may be skipped.

The harness must return non-zero for every violation above.

### A2. Eliminate successful empty-suite behavior

Correct both the Rust harness and shell wrapper so that:

- Missing fixture directory in strict mode is fatal.
- Empty fixture directory in strict mode is fatal.
- Missing manifest in strict mode is fatal.
- Empty manifest in strict mode is fatal.
- A format filter that selects zero fixtures in strict mode is fatal.
- A manifest that contains only unsupported extensions is fatal.

Non-strict mode may still emit warnings and return a structured incomplete result, but it must clearly report `complete = false`; it must not print an unconditional “all checks passed” message.

### A3. Use stable exit-code classes

Define and document stable process exit codes. A recommended contract is:

- `0`: complete conformance pass.
- `1`: fixture expectation or interoperability failure.
- `2`: command-line/configuration error or missing required tool.
- `3`: manifest/schema/digest failure.
- `4`: coverage failure.
- `5`: internal harness error.

The shell wrapper must preserve or consistently translate these codes. Tests must verify each class.

### A4. Remove ambiguous `--require-complete`

The shell wrapper currently accepts `--require-complete`, but the Rust binary does not parse it. Remove the option from the wrapper and documentation. `--strict` is the single complete-validation mode.

Do not retain an ignored compatibility flag. Unknown arguments must be fatal rather than silently ignored.

### Acceptance criteria

- `stegoeggo-conformance --strict --fixtures /missing --manifest /missing` exits non-zero.
- Strict mode with an empty fixture directory exits non-zero.
- Strict mode without `--manifest` exits non-zero.
- Strict mode with an empty manifest exits non-zero.
- Strict mode with a format filter selecting zero cases exits non-zero.
- Unknown CLI arguments exit non-zero with usage text.
- The shell wrapper returns non-zero for the same cases.
- Unit or integration tests cover every fail-closed branch.

## Workstream B: Make the manifest authoritative

### B1. Validate manifest structure before processing fixtures

Add manifest validation that rejects:

- Duplicate fixture IDs.
- Duplicate fixture paths.
- Empty IDs.
- Absolute paths.
- Paths containing `..` traversal.
- Unsupported format values.
- Unsupported category values.
- Missing or empty SHA-256 values.
- Invalid SHA-256 length or characters.
- Missing provenance fields for externally classified fixtures.
- Contradictory expectation fields.

Manifest validation must run before fixture processing and before writing a success report.

### B2. Join each fixture to exactly one manifest entry

The harness must build a validated path-to-entry index and use it throughout processing.

For every fixture:

- Resolve its relative normalized path.
- Look up exactly one `FixtureEntry`.
- Attach the fixture ID, category, source classification, and expectations to its report.
- Fail if no entry exists.
- Fail if multiple entries resolve to the same path.

After processing:

- Fail if any manifest entry was not exercised.
- Fail if any supported file in the fixture tree was not listed.

### B3. Replace ambiguous expected-negative modeling

The current `expected_malformed: bool` is insufficient because malformed cases can mean different expected outcomes. Replace or extend it with explicit expectation fields. A suitable model is:

```toml
expected_decode = "pass"        # pass | fail | either
expected_xmp = "valid"          # valid | invalid | absent | either
expected_internal = "success"   # success | no_notice | reject
expected_external = "success"   # success | no_notice | reject
expected_conflict = false
required_external_fields = ["canonical_dmi"]
```

Keep compatibility with the current manifest only during migration inside the implementation commit. The checked-in final manifest must use the explicit schema consistently.

### B4. Evaluate expected values directly

For each fixture, compare observed results against manifest expectations for:

- Decode outcome.
- XMP presence and XML validity.
- Internal extraction outcome.
- External extraction outcome.
- Canonical DMI value.
- Legacy DMI value where applicable.
- Conflict presence.
- Copyright holder.
- Creator or creators.
- Copyright owner.
- Usage terms.
- Rights/web-statement URL.
- Credit line.
- Licensor name, email, and URL.
- Content creation/metadata dates where expected.
- AI constraints.
- Preservation assertions.

Agreement between internal and external extraction remains useful, but it is secondary. Two parsers agreeing on the same wrong value must not satisfy a fixture whose manifest expects a different value.

### B5. Handle expected-negative fixtures as passes when correct

A malformed fixture must pass only when the specified expected failure occurs. Examples:

- Truncated image: decode failure expected.
- Invalid XMP entity: image decode may pass, XMP XML validation must fail.
- Unknown PLUS vocabulary URI: image/XMP may be valid, canonical DMI must remain unknown or rejected according to the manifest.
- Invalid marker length: parser rejection or safe no-notice outcome expected, with no panic.

The report must distinguish:

- `PASS: expected failure observed`.
- `FAIL: malformed fixture unexpectedly accepted`.
- `FAIL: valid fixture unexpectedly rejected`.

### B6. Include manifest expectations in JSON

Each fixture JSON object must contain:

- Fixture ID.
- Relative path.
- Category.
- Source/provenance class.
- Expected outcomes.
- Observed outcomes.
- Per-check results.
- Final pass/fail.

### Acceptance criteria

- Every fixture is matched to exactly one manifest entry.
- Unlisted files and unexercised entries fail strict mode.
- Duplicate IDs and paths fail manifest validation.
- Expected-negative fixtures pass only for the expected reason.
- Valid fixtures fail if observed fields differ from manifest values even when internal and external parsers agree.
- All existing manifest expectation fields are either consumed or removed; no dead expectation fields remain.

## Workstream C: Correct external-parser result semantics

### C1. Return typed external extraction results

Change external extraction from an implicit empty object to a typed result such as:

```rust
Result<ExternalExtraction, ExternalToolError>
```

The error should preserve:

- Tool name.
- Executable path.
- Exit status.
- Stderr summary.
- Whether output was empty.
- Whether JSON parsing failed.

### C2. Never record a failed tool call as `Pass`

`external_extraction` may be `Pass` only when:

- The command launches.
- The process exits successfully.
- Output is non-empty when output is required.
- JSON parses.
- At least the fields required by the fixture are externally visible.

In strict mode, command failure or missing required fields is a fixture failure.

### C3. Replace permissive warnings for required fields

The current comparison treats internal-only or external-only values as warnings. Introduce fixture-driven required-field semantics:

- If a field appears in `required_external_fields`, absence externally is `Fail`.
- If a field appears in expected internal values, absence internally is `Fail`.
- Optional parser-specific aliases may remain warnings only when the manifest does not require them.
- Canonical `plus:DataMining` visibility is mandatory for every expected-valid canonical fixture.

### C4. Make namespace and URI checks exact

Canonical PLUS conformance must verify:

- The PLUS XMP namespace is exactly `http://ns.useplus.org/ldf/xmp/1.0/`.
- The Data Mining controlled vocabulary URI is exactly under `http://ns.useplus.org/ldf/vocab/`.
- The controlled vocabulary key is recognized.
- Alternate XML prefixes are accepted when the namespace URI is correct.
- A correct prefix with the wrong namespace URI fails.
- An unknown controlled vocabulary key is not normalized into a known value.

Avoid substring-only acceptance for standards-critical values.

### Acceptance criteria

- Simulated ExifTool non-zero exit produces a failed fixture.
- Empty ExifTool JSON produces a failed fixture when extraction is required.
- Invalid JSON produces a failed fixture.
- Canonical fixtures fail when `plus:DataMining` is not externally visible.
- Alternate prefixes with correct namespace URIs pass.
- Wrong namespaces and unknown vocabulary values fail or remain explicitly unknown according to manifest expectations.

## Workstream D: Exercise ImageMagick and libvips

### D1. Add real tool execution to the harness

For every expected-valid fixture:

- Run ImageMagick identification using a resolver that supports both ImageMagick 6 (`identify`) and ImageMagick 7 (`magick identify`).
- Run libvips header/decode validation using `vipsheader` when available, with a documented fallback to an equivalent `vips` command.
- Capture version, command, exit status, and stderr summary.
- Add one external extraction/result object per tool.

For expected-malformed fixtures, evaluate each tool according to the fixture expectation rather than requiring success.

### D2. Validate common-pipeline decodability

At minimum, expected-valid fixtures must be successfully decoded or identified by:

- Rust `image` crate.
- ImageMagick.
- libvips.

Format-specific unsupported behavior must be explicit in the manifest and may not be silently skipped.

### D3. Keep metadata expectations scoped

ExifTool remains the authoritative external metadata parser for detailed field extraction. ImageMagick and libvips primarily provide common-pipeline decode/visibility evidence unless a stable metadata field is explicitly supported and tested.

### Acceptance criteria

- The JSON report contains ImageMagick and libvips results per expected-valid fixture.
- Removing either tool in strict mode causes a non-zero exit.
- A fixture that fails ImageMagick or libvips unexpectedly fails conformance.
- Tool-version artifacts correspond to tools actually used in fixture checks.

## Workstream E: Redesign coverage requirements

### E1. Remove the blanket 75% external-fixture threshold

A global external-source percentage does not reflect the fixture taxonomy and currently makes the checked-in corpus impossible to validate meaningfully. Replace it with explicit category and format requirements.

Recommended minimums:

- At least one externally authored canonical PNG.
- At least one externally authored canonical JPEG.
- At least one externally authored canonical WebP.
- At least one externally authored alternate-prefix fixture in two formats.
- At least one externally authored structured owner/licensor fixture in two formats.
- At least one pinned historical legacy fixture per supported format.
- At least one expected-negative malformed fixture per supported format.
- At least one preservation fixture per supported format containing unrelated pre-existing metadata.
- At least three conflict fixtures covering DMI disagreement and non-DMI field disagreement.

### E2. Use truthful source classes

Use source classifications that describe how a fixture was produced, for example:

- `external`: authored independently using a named external tool.
- `historical`: produced by a pinned older StegoEggo release or preserved historical binary output.
- `generated-negative`: deliberately malformed by a checked-in deterministic generator.
- `current-generated`: produced by the current StegoEggo implementation.

Do not classify raw-byte injection by a project script as “exiftool authored” unless ExifTool actually wrote the relevant metadata.

### E3. Make coverage configuration explicit

Coverage minimums should be either:

- Versioned in the manifest under a `[coverage]` section, or
- Defined as named constants with a serialized copy included in the run report.

Do not leave closure-critical thresholds undocumented in code only.

### Acceptance criteria

- The checked-in manifest satisfies the new category-specific minimums.
- Removing one required external per-format fixture causes coverage failure.
- Removing one required historical legacy format causes coverage failure.
- Coverage output explains each missing class precisely.
- No aggregate percentage is used as a substitute for category coverage.

## Workstream F: Complete fixture provenance

### F1. Fill every provenance field

For each externally authored fixture, require non-empty:

- `authoring_tool`.
- `authoring_tool_version`.
- Exact generation command or generator script path.
- Source classification.
- License.
- SHA-256 digest.
- Date or source revision when relevant.

### F2. Check in deterministic generators where raw injection is used

If a fixture was produced through raw PNG/JPEG/WebP metadata injection, check in the generator under a stable path such as:

```text
tests/fixtures/conformance/generators/
```

The generator must:

- Be deterministic.
- Document its input base image.
- Reproduce the fixture byte-for-byte or document why byte-for-byte reproduction is not possible.
- Be clearly identified as the authoring tool in the manifest.

ExifTool may be listed separately as the verification tool.

### F3. Pin historical fixtures honestly

Legacy fixtures should be generated by or copied from the actual historical release they claim to represent. Do not label a current `0.3.0` generator as a `v0.2` fixture without evidence that it reproduces the old output format exactly.

Record:

- Historical tag or commit SHA.
- Binary/crate version.
- Exact command.
- Fixture digest.

### Acceptance criteria

- No external fixture has an empty tool version.
- No fixture’s `authoring_tool` contradicts its generation command.
- Legacy fixtures identify an actual historical source revision.
- Raw-injection fixtures reference checked-in deterministic generators.
- Manifest validation enforces these requirements.

## Workstream G: Correct DMI normalization

### G1. Fix precedence

`normalize_dmi_value()` currently checks broad AI/ML prohibition before generative-AI prohibition. Reorder matching from most specific to most general:

1. Prohibited except search-engine indexing.
2. Prohibited generative AI/ML training.
3. Prohibited AI/ML training.
4. Prohibited, see constraint.
5. General prohibited.
6. Allowed.
7. Unknown unchanged.

Prefer exact vocabulary-key parsing before display-string heuristics.

### G2. Add table-driven tests

Cover every supported `DmiValue` input form:

- Exact PLUS vocabulary URI.
- Vocabulary key.
- Internal enum display form.
- ExifTool display form.
- Alternate case where applicable.
- Unknown value.

Ensure `DMI-PROHIBITED-GENAIMLTRAINING` never normalizes to `DMI-PROHIBITED-AIMLTRAINING`.

### Acceptance criteria

- All known vocabulary values normalize uniquely.
- Unknown values remain unknown.
- Exact URI parsing is tested.
- The generative-AI precedence regression has a dedicated test.

## Workstream H: Separate hermetic tests from external integration tests

### H1. Keep normal `cargo test` hermetic

The ordinary CI test job must not depend on system packages that it does not install. Mark external-tool integration tests ignored and execute them explicitly in the conformance job, for example:

```bash
cargo test --test external_tools -- --ignored
```

Alternatively, use a test-only feature if there is a compelling reason, but do not add unnecessary public API surface.

### H2. Use portable executable resolution

External test helpers must resolve:

- `identify` or `magick identify`.
- `vipsheader` or the documented equivalent.
- `exiftool`.
- `xmllint`.

Do not hard-code an ImageMagick 7-only command on Ubuntu runners that commonly provide ImageMagick 6.

### H3. Assert tool availability in the dedicated job

External integration tests must fail immediately with a clear message if launched without their required tools. They must not silently skip after CI has declared the tools mandatory.

### Acceptance criteria

- `cargo test --all-features` passes on a clean Rust environment without ExifTool/ImageMagick/libvips/xmllint.
- The dedicated conformance job installs tools and runs all ignored external tests explicitly.
- No external test is silently skipped in the dedicated job.
- ImageMagick command resolution works with both IM6 and IM7 layouts.

## Workstream I: Align CI and release validation

### I1. Use one reusable command contract

Main CI and release validation must execute the same conformance commands and arguments:

```bash
cargo build --release --bin stegoeggo-conformance
cargo test --test external_tools -- --ignored
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --json conformance-report.json
```

Avoid copy drift by using a checked-in script or composite action if practical.

### I2. Install the complete tool set in both workflows

Both workflows must install and verify:

- ExifTool.
- `xmllint`.
- ImageMagick.
- libvips CLI tools.

Use the correct Ubuntu package for the libvips command-line tools, not only development headers.

### I3. Harden release gates

Update `.github/workflows/release.yml` so that:

- `cargo-deny` is installed before use.
- License and advisory checks are mandatory.
- `continue-on-error: true` is removed.
- Package validation matches main CI and does not bypass verification without a documented reason.
- The conformance manifest is passed.
- Tool versions are uploaded.
- Conformance JSON is uploaded even on failure.
- The release job cannot proceed past a failed conformance gate.

### I4. Produce one report envelope

Replace the current per-fixture-only JSON output with a versioned run envelope containing:

- Report schema version.
- Commit SHA when available.
- Strict-mode flag.
- Tool names, paths, and versions.
- Manifest path and digest.
- Fixture digest results.
- Coverage configuration and result.
- Aggregate counts.
- Per-fixture reports.
- Overall `complete` and `passed` booleans.
- Incomplete/skipped reason list.

The artifact must remain valid and informative on failure.

### Acceptance criteria

- Main CI and release workflows invoke the same strict conformance contract.
- Release validation installs all required tools and passes the manifest.
- No required release check has `continue-on-error`.
- A failed license, advisory, external-tool, digest, coverage, or fixture check fails the workflow.
- Both workflows upload a complete JSON report and tool-version artifact.

## Workstream J: Documentation and API consistency

### J1. Correct stale legal-claim documentation

Update `ProtectionContext` field documentation and architecture text to match actual behavior: providing `LegalMetadata` auto-enables legal claim injection unless explicitly disabled.

### J2. Describe date validation accurately

The current validator checks ISO 8601 shape, not full calendar validity. Either:

- Strengthen validation to reject impossible month/day/time/offset values, or
- Document it explicitly as structural ISO 8601 validation.

Do not claim full date validity if only separators and digit positions are checked.

### J3. Keep TDMRep deferral consistent

Verify that:

- No current writer emits project-defined image-level TDM properties.
- `--tdm-reserved` remains deprecated with an actionable warning.
- README and architecture documents state that web TDMRep deployment artifacts are deferred.
- Verification may still read historical legacy TDM markers without presenting them as canonical.

### J4. Document conformance limitations

Document that:

- ExifTool is the detailed metadata oracle used by this project, not an official certification service.
- ImageMagick and libvips checks demonstrate common-pipeline compatibility, not legal enforceability.
- Fixtures and expected outcomes are versioned project evidence.

### Acceptance criteria

- Public docs, code comments, CLI help, and architecture docs agree.
- No documentation claims that installed-but-unused tools provide evidence.
- TDMRep deferral is unambiguous.
- Date-validation claims match implementation.

## Workstream K: Closure evidence and status ledgers

### K1. Create `plans/020-status.md`

Record:

- Implementation commit SHA or SHAs.
- Files changed.
- Commands run.
- Local tool versions.
- Test counts and ignored-test counts.
- Each Workstream A–J criterion marked `PASS`, `FAIL`, or `DEFERRED`.
- Exact evidence location for every `PASS`.
- Any remaining risk.

### K2. Create or complete `plans/019-status.md`

Because Plan 019 required an acceptance ledger and none is currently present, add a final ledger mapping Plans 016–019 to the evidence produced by Plan 020.

The ledger must not merely restate commit messages. It must reference:

- Test names.
- Fixture IDs.
- CI workflow run URL or ID.
- Artifact names.
- Conformance summary counts.
- Tool versions.
- Explicitly deferred TDMRep deployment scope.

### K3. Require clean CI evidence

Plan 020 cannot be marked complete based only on local output. Record at least one successful `main` CI run after the corrective commit and inspect:

- Test job.
- Lint job.
- Security/deny jobs.
- External conformance job.
- `conformance-report` artifact.
- `tool-versions` artifact.

If a release tag is not appropriate yet, validate the release command path locally or through a non-publishing manual workflow. Do not create a release solely to prove the workflow.

### Acceptance criteria

- `plans/020-status.md` exists and contains criterion-by-criterion evidence.
- `plans/019-status.md` exists and closes or explicitly defers every Plans 016–019 acceptance item.
- A green clean-run CI ID and artifact names are recorded.
- The report artifact shows `complete = true`, `passed = true`, non-zero fixture counts, passing digests, passing coverage, and all required tools used.

## Required regression tests

Add tests covering at least the following cases.

### Harness configuration

- Missing fixture directory, strict.
- Empty fixture directory, strict.
- Missing manifest, strict.
- Empty manifest, strict.
- Invalid TOML.
- Duplicate fixture ID.
- Duplicate fixture path.
- Path traversal.
- Unsupported category.
- Unsupported format.
- Unknown CLI option.
- Zero-result format filter.

### Manifest and expectations

- Manifest entry without file.
- File without manifest entry.
- Digest mismatch.
- Valid fixture with wrong expected DMI.
- Valid fixture with wrong expected legal field.
- Expected malformed image that fails decode.
- Expected malformed XMP with decodable image.
- Malformed fixture unexpectedly accepted.
- Valid fixture unexpectedly rejected.
- Expected conflict observed.
- Expected conflict missing.

### External tools

- ExifTool command failure.
- ExifTool empty output.
- ExifTool invalid JSON.
- Missing externally required canonical DMI.
- ImageMagick failure on valid fixture.
- libvips failure on valid fixture.
- Missing required tool in strict mode.
- IM6 `identify` resolution.
- IM7 `magick identify` resolution.

### DMI normalization

- Every supported canonical URI.
- Every supported vocabulary key.
- Generative-AI value precedence.
- Search-engine exception precedence.
- Unknown vocabulary value preservation.

### Reporting

- Failure report JSON is still written.
- JSON includes digest and coverage results.
- JSON includes tool versions.
- JSON includes manifest expectations and observations.
- Empty or incomplete non-strict run reports `complete = false`.

## Final validation sequence

The implementing agent must run the following from a clean checkout or clean container/VM with the required external packages installed only for the external phase.

### Hermetic Rust phase

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --no-fail-fast
cargo test --doc
cargo package --workspace --allow-dirty
cargo deny check licenses
cargo deny check advisories
```

### External conformance phase

```bash
cargo build --release --bin stegoeggo-conformance
cargo test --test external_tools -- --ignored
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict \
  --json conformance-report.json
```

### Negative fail-closed phase

Run scripted checks proving non-zero exit for:

```bash
# Missing fixtures
./target/release/stegoeggo-conformance \
  --fixtures /tmp/does-not-exist \
  --manifest tests/fixtures/conformance/manifest.toml \
  --strict

# Missing manifest
./target/release/stegoeggo-conformance \
  --fixtures tests/fixtures/conformance \
  --manifest /tmp/does-not-exist.toml \
  --strict

# Empty selected format or empty temporary fixture tree
# Digest-tampered fixture copy
# Manifest with one required external class removed
```

The status ledger must record the observed non-zero exit codes.

## Completion gate

Plan 020 is complete only when all of the following are true:

1. Strict mode fails on missing, empty, incomplete, skipped, or unmanifested fixture suites.
2. Every fixture is evaluated against explicit manifest expectations.
3. Expected-negative fixtures are modeled and evaluated correctly.
4. External parser errors and missing required fields are failures.
5. ImageMagick and libvips are executed per expected-valid fixture.
6. Coverage uses explicit category/format requirements rather than a blanket percentage.
7. Fixture provenance is complete and truthful.
8. DMI normalization correctly distinguishes all supported vocabulary values.
9. Normal Rust tests are hermetic; external tests run only in the declared external job.
10. Main CI and release validation use the same strict command contract.
11. Release license/advisory/conformance gates cannot continue on error.
12. The JSON artifact contains complete run-level evidence and remains available on failure.
13. `plans/019-status.md` and `plans/020-status.md` contain auditable evidence.
14. A clean `main` CI run is green and its conformance/tool-version artifacts have been inspected.
15. No Release 4 scope has been introduced.

Only after this gate passes should Plans 016–019 be marked closed and Release 4 work begin.
