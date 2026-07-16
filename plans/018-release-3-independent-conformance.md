# Plan 018: Release 3 — Independent Interoperability and Conformance

## Objective

Make independent interoperability a mandatory correctness and release criterion for StegoEggo.

The project must no longer rely on a closed loop in which its writer emits metadata, its parser recognizes the same representation, and internal tests declare success. Release 3 establishes externally authored fixtures, external parser validation, structured conformance reporting, metadata-preservation checks, and blocking CI gates across PNG, JPEG, and WebP.

This plan assumes Plans 016 and 017 are complete: newly written rights metadata is canonical, legal field semantics are normalized, and format writers receive one common rights-notice model.

## Problem statement

The repository already contains an external metadata conformance script, but it is not a mandatory CI gate and much of its behavior depends on shell parsing, project-specific expectations, and tools being present locally. A standards-oriented rights metadata tool requires stronger evidence:

- Independently authored canonical fixtures.
- Namespace-aware validation.
- External parser extraction.
- Cross-format normalized comparison.
- Conflict and malformed-input detection.
- Non-destructive metadata updates.
- Machine-readable conformance results.
- Blocking release integration.

## Scope

### In scope

- Mandatory external-conformance CI.
- A structured Rust conformance harness or test binary.
- Golden fixtures from independent sources.
- Canonical and legacy read tests.
- Namespace and RDF-structure validation.
- Duplicate, unknown, malformed, and conflicting declaration tests.
- Metadata preservation and idempotence tests.
- Machine-readable conformance reports.
- Release workflow integration.
- Documentation for reproducing conformance locally.

### Out of scope

- New rights metadata semantics beyond corrections required to pass conformance.
- Evidence-profile architecture redesign.
- New steganographic algorithms.
- Public-key provenance or C2PA.
- Broad performance optimization.
- Supporting additional image formats.

## Affected areas

Expected areas:

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `scripts/verify_metadata_conformance.sh`
- new conformance binary, example, or integration-test harness
- `tests/`
- fixture directories and fixture provenance documentation
- `src/protected/notice_verification.rs`
- format writers where preservation failures are discovered
- `README.md`
- `docs/legal_notice_model.md`
- contributor/agent documentation
- `CHANGELOG.md`

Confirm the current layout before implementation.

## Target conformance architecture

Use a layered model:

```text
Fixture or generated image
    -> StegoEggo writer/update operation
    -> container validation
    -> namespace-aware XMP validation
    -> external parser extraction
    -> internal parser extraction
    -> normalized semantic comparison
    -> structured conformance report
```

No single parser should be treated as the sole source of truth. The harness should compare independently produced observations and report discrepancies explicitly.

## Execution phases

## Phase 1: Define the conformance schema

Create a machine-readable report model. JSON is preferred.

Suggested shape:

```rust
pub struct ConformanceReport {
    pub fixture: String,
    pub format: String,
    pub generated_by: String,
    pub decode_valid: bool,
    pub xmp_valid: Option<bool>,
    pub canonical_data_mining: Option<String>,
    pub legacy_data_mining: Vec<String>,
    pub copyright_notice: Option<String>,
    pub creators: Vec<String>,
    pub copyright_owners: Vec<String>,
    pub licensors: Vec<ConformanceLicensor>,
    pub credit_line: Option<String>,
    pub usage_terms: Option<String>,
    pub rights_url: Option<String>,
    pub content_creation_date: Option<String>,
    pub notice_applied_at: Option<String>,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub passed: bool,
}
```

The exact public/private placement is flexible. Prefer keeping the harness model outside the stable library API unless it is generally useful.

Define explicit pass/fail rules for:

- Canonical writer output.
- Legacy fixture readability.
- Unknown vocabulary handling.
- Conflicts.
- Missing optional fields.
- Missing required fields for each fixture profile.
- External tool unavailability.
- Fixture generation failure.

### Acceptance criteria

- Every conformance check maps to a structured field or error.
- Pass/fail is deterministic.
- Missing tools cannot silently produce a successful strict report.

## Phase 2: Build a Rust conformance harness

Move semantic assertions out of shell/grep logic into a Rust binary or integration-test helper.

Possible locations:

- `tools/conformance/`
- `src/bin/stegoeggo-conformance.rs`
- `tests/conformance_harness.rs`
- a small workspace member if dependencies should remain isolated

The harness should:

1. Accept one image or a fixture directory.
2. Detect format by magic bytes.
3. Run StegoEggo internal verification.
4. Invoke configured external tools and capture output.
5. Parse external output into normalized fields.
6. Validate raw XMP with a namespace-aware parser.
7. Compare internal and external normalized results.
8. Emit JSON and a concise human-readable summary.
9. Return nonzero on strict failure.
10. Distinguish infrastructure failure from metadata failure.

Do not shell out through interpolated command strings. Use argument arrays and bounded output capture.

### External tool adapters

Implement adapters for:

- ExifTool: authoritative external metadata extraction target.
- ImageMagick: decode and metadata visibility smoke checks.
- libvips: decode and container metadata smoke checks.
- XML validator/parser: raw XMP well-formedness and namespace assertions.

The harness may treat ExifTool plus namespace-aware XML parsing as required and the others as supplementary, but CI policy must be explicit.

### Acceptance criteria

- The harness runs deterministically on Ubuntu CI.
- Tool errors are captured and reported.
- Output is valid JSON.
- No shell grep is required to decide semantic conformance.

## Phase 3: Create fixture taxonomy and provenance records

Create a dedicated fixture tree, for example:

```text
tests/fixtures/conformance/
    README.md
    canonical/
        png/
        jpeg/
        webp/
    legacy-v0.2/
    malformed/
    conflicting/
    preservation/
```

Each fixture must have provenance documented:

- How it was generated.
- Tool and version.
- Whether it was hand-authored.
- Expected normalized fields.
- License or public-domain status.
- Expected pass/fail classification.

Required fixture categories:

### Canonical

- PLUS Data Mining attribute form.
- PLUS Data Mining element form.
- Alternate namespace prefixes.
- Complete normalized legal notice.
- Minimal policy-only notice.
- Unicode legal text.
- Multiple creators and structured licensor, if supported by Release 2.

### Legacy

- v0.2 PNG.
- v0.2 JPEG.
- v0.2 WebP.
- Legacy DMI and TDM properties.
- Legacy hidden payloads where practical.

### Malformed

- Truncated XMP.
- Invalid XML entity.
- Invalid namespace binding.
- Unknown controlled-vocabulary URI.
- Oversized or pathological metadata within safe fixture limits.
- Truncated PNG chunk.
- Invalid JPEG marker length.
- Invalid WebP RIFF chunk length.

### Conflicting

- Canonical allowed plus legacy prohibited.
- Two canonical Data Mining values.
- Conflicting rights URLs.
- Conflicting copyright owner values.
- TDM reservation conflicting with normalized data-mining policy.

### Preservation

- Camera EXIF.
- Orientation.
- ICC profile.
- Existing unrelated XMP namespace.
- Existing IPTC records.
- Existing creator metadata.
- Existing animation/chunks for WebP where supported.

### Acceptance criteria

- Canonical fixtures are not all generated by StegoEggo.
- Legacy fixtures are retained permanently.
- Every fixture has an expected result record.
- Fixture tests are deterministic and small enough for repository use.

## Phase 4: Namespace-aware XMP validation

Implement validation that reasons over namespace URIs rather than fixed prefixes.

Required checks:

- PLUS namespace resolves correctly.
- `DataMining` resolves under the PLUS namespace.
- Controlled-vocabulary URI is exact and recognized or explicitly unknown.
- Standard legal fields resolve under their correct namespaces.
- RDF containers have expected shape where required.
- Duplicate declarations are collected.
- Multiple `rdf:Description` elements are supported.
- Attribute and element forms normalize identically.
- XML remains well formed after escaping legal text.

Bound parser work:

- Maximum XMP bytes.
- Maximum XML depth.
- Maximum element/property count.
- Maximum text length.

If the selected parser lacks native limits, enforce byte and property bounds around it and document residual risk.

### Acceptance criteria

- Prefix substitution does not alter results.
- Malformed XML fails cleanly.
- Unknown values remain observable.
- Duplicate/conflicting properties are reported.

## Phase 5: External extraction normalization

Define stable adapters that normalize external tool aliases into the same semantic model.

For example, ExifTool may expose equivalent properties under names such as:

- `DataMining`
- `XMP-plus:DataMining`
- `Rights`
- `XMP-dc:Rights`
- `UsageTerms`
- `WebStatement`

Do not hard-code one display alias without testing actual tool output. Prefer ExifTool JSON output to line-oriented text when available.

External extraction must preserve multiplicity for arrays and repeated properties. Do not collapse multiple values before conflict analysis.

### Acceptance criteria

- External values normalize without grep.
- Arrays and repeated fields remain arrays.
- Internal and external reports can be compared field by field.
- Tool-version differences are documented and covered where practical.

## Phase 6: Cross-format conformance matrix

For each canonical notice profile, generate or update PNG, JPEG, and WebP outputs, then compare normalized results.

Required profiles:

1. Policy only.
2. Copyright notice plus policy.
3. Creator and owner.
4. Licensor/contact.
5. Usage terms and rights URL.
6. Date fields.
7. Complete notice.
8. Unicode and XML-sensitive text.
9. Existing unrelated metadata.
10. Reapplied identical notice.
11. Replaced specified fields.
12. Conflict-preservation mode, if supported.

Assertions:

- Internal parser result matches external extraction.
- All formats match the expected normalized notice.
- Container-specific carriers may differ, but semantics must not.
- Missing fields are errors unless a documented format limitation is explicitly accepted.

### Acceptance criteria

- A single matrix test identifies format divergence.
- The complete notice passes all formats.
- Known limitations are explicit and not hidden as generic warnings.

## Phase 7: Metadata preservation and idempotence

Test the update path against preservation fixtures.

Required invariants:

- Pixel/image content remains decodable.
- Existing ICC profile remains present where supported.
- Orientation remains present unless intentionally normalized.
- Camera EXIF remains present.
- Unrelated XMP namespaces remain present.
- Existing IPTC records remain present where the container path claims preservation.
- Applying the same notice twice does not create unbounded duplicate metadata.
- Repeated processing produces the same normalized rights notice.
- Conflicts follow the documented update policy.

When a format writer cannot preserve a category, it must return a structured warning and the limitation must be documented. Silent loss is not acceptable.

### Acceptance criteria

- Preservation tests pass for all claimed preserved fields.
- Unsupported preservation cases are explicit.
- Idempotence tests pass.

## Phase 8: Convert the shell script into a thin wrapper

Retain `scripts/verify_metadata_conformance.sh` for operator convenience, but make it:

1. Discover/build the conformance harness.
2. Check required tools.
3. Invoke the harness with strict or non-strict mode.
4. Print the human-readable summary.
5. Preserve the harness exit code.

Remove semantic grep logic from the shell script once equivalent harness checks exist.

Supported modes should include:

```text
--format png|jpeg|webp
--all-formats
--strict
--json PATH
--fixtures PATH
```

Exact flags may differ, but backward-compatible aliases should be retained where inexpensive.

### Acceptance criteria

- Shell and direct harness invocation produce the same result.
- Strict mode cannot pass when required tools are missing.
- Generated-fixture failure is a failure, not a skip.

## Phase 9: Add mandatory CI conformance job

Add a dedicated GitHub Actions job that:

1. Checks out the repository.
2. Installs the supported Rust toolchain.
3. Installs ExifTool, ImageMagick, libvips, and required XML tooling.
4. Builds the library, CLI, and conformance harness.
5. Runs canonical fixture tests.
6. Runs legacy fixture tests.
7. Runs malformed/conflict tests.
8. Runs generated cross-format conformance in strict mode.
9. Uploads JSON conformance reports as workflow artifacts.
10. Fails on any required check.

Pin or constrain external tool versions where practical. Record versions in the report.

The conformance job must run on:

- Pull requests to `main`.
- Pushes to `main`.
- Release tags before publication.

Do not mark it `continue-on-error`.

### Acceptance criteria

- CI visibly reports conformance status.
- Failure blocks merge/release according to repository policy.
- Reports are retained as artifacts.
- Tool versions are included.

## Phase 10: Integrate release gating

Update the release workflow so publication cannot proceed unless:

- Unit and integration tests pass.
- External conformance passes.
- Package checks pass.
- Required security/license checks pass.
- Tag and versions match.

If the release workflow remains validation-only, rename it accordingly. If it publishes crates, ensure conformance precedes publication.

Attach or publish the conformance report with the release where practical.

### Acceptance criteria

- A release cannot be produced from nonconformant metadata code.
- Required checks are blocking.
- Release documentation links to the conformance result.

## Phase 11: Documentation and contributor workflow

Document:

- Required external tools.
- Ubuntu installation commands.
- macOS installation commands where supported.
- How to run one fixture, one format, or the full suite.
- How to add a fixture and expected result.
- How fixture provenance is recorded.
- Difference between internal verification and external conformance.
- What conformance does and does not prove.

Add a contributor checklist:

```text
[ ] Canonical writer test updated
[ ] Legacy reader test preserved
[ ] External fixture added or reviewed
[ ] Namespace-aware validation passes
[ ] Cross-format matrix passes
[ ] Preservation/idempotence passes
[ ] Strict external conformance passes
```

## Required tests

### Harness tests

- JSON report serialization.
- Missing-tool behavior.
- External command failure.
- Invalid external JSON.
- Tool-version capture.
- Strict versus non-strict mode.

### Parser tests

- Namespace aliases.
- Attribute and element forms.
- Multiple descriptions.
- Duplicate values.
- Conflicts.
- Unknown vocabulary values.
- XML limits.

### Container tests

- PNG chunk integrity.
- JPEG marker integrity.
- WebP RIFF integrity.
- Independent decode.
- Preservation and idempotence.

### Regression tests

- All Release 1 canonical mappings.
- All Release 2 semantic fields.
- Legacy v0.2 fixtures.
- Existing stego verification unaffected.

## Testing commands

Run at minimum:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --no-fail-fast
cargo test --doc
cargo package --workspace --allow-dirty
./scripts/verify_metadata_conformance.sh --all-formats --strict
```

The CI environment should also run the conformance harness directly and upload JSON reports.

## Risks and mitigations

### Risk: External tool output changes across versions

Mitigation: use structured output, capture versions, normalize aliases, and pin versions where feasible.

### Risk: Fixture suite becomes self-referential

Mitigation: require documented independently authored canonical fixtures.

### Risk: XML parser introduces resource-exhaustion risk

Mitigation: enforce byte, depth, property-count, and text-length limits.

### Risk: Preservation requirements expose writer limitations

Mitigation: report limitations explicitly and fix high-value losses before claiming preservation.

### Risk: CI becomes slow or brittle

Mitigation: keep fixtures small, cache Rust dependencies, separate deterministic conformance from optional broad tool smoke tests, and preserve detailed artifacts for diagnosis.

### Risk: Conformance is mistaken for legal validation

Mitigation: documentation must state that the suite validates syntax, discoverability, and semantic consistency, not legal enforceability or infringement.

## Completion gate

Release 3 is complete only when:

- A structured conformance harness exists and emits machine-readable reports.
- Canonical, legacy, malformed, conflicting, and preservation fixtures are committed with provenance.
- At least one canonical fixture path per supported format is independently authored or externally generated.
- Namespace-aware validation passes.
- Internal and external normalized extraction agree.
- Cross-format semantic-equivalence tests pass.
- Preservation and idempotence tests pass for all claimed behavior.
- The shell script delegates semantic decisions to the harness.
- External conformance runs in strict mode on CI and is blocking.
- Release validation includes the conformance gate.
- Documentation explains reproduction and limitations.

## Handoff notes

Do not close this plan with only a green Rust test suite. The defining evidence is a green, strict, externally tooled conformance job using independently authored fixtures. Any skipped tool, skipped format, or unavailable external evidence must remain an explicit open blocker rather than being converted into a passing status.
