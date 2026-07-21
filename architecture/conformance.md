# Conformance Architecture

The conformance suite validates that stegoeggo's metadata output is
interoperable with external parsers. It is a mandatory CI gate and
release prerequisite.

## Components

| Component | Location | Role |
|-----------|----------|------|
| Report types | `src/conformance.rs` | `ConformanceReport`, `ConformanceRunReport`, `ToolReport`, `ManifestReport`, `CheckSeverity`, `FixtureManifest`, extraction structs |
| Harness binary | `src/bin/stegoeggo-conformance.rs` | CLI entry point, orchestrates all checks |
| Regression tests | `tests/conformance_harness_tests.rs` | Report type serialization, expected-negative semantics, coverage |
| External tool tests | `tests/external_tools.rs` | External tool integration tests (`#[ignore]`, run with `--ignored`) |
| Fixture taxonomy | `tests/fixtures/conformance/` | Test images organized by category |
| Fixture manifest | `tests/fixtures/conformance/manifest.toml` | Machine-readable fixture metadata with SHA-256 digests |
| Shell wrapper | `scripts/verify_metadata_conformance.sh` | Operator-friendly wrapper around harness |
| Release validation | `scripts/validate-release.sh` | Centralized validation script (hermetic + external phases) |
| CI job | `.github/workflows/ci.yml` (conformance) | Mandatory gate on PRs and pushes to main |
| Release gate | `.github/workflows/release.yml` | Blocks publication if conformance fails |

## Conformance Flow

```
Fixture manifest (TOML)
    → Manifest structural validation (validate_manifest)
    → SHA-256 digest verification
    → Fixture image (PNG/JPEG/WebP)
    → Format detection (magic bytes)
    → Image decode (image crate)
    → Internal extraction (verify_legal_notice)
    → External extraction (ExifTool -G -a)
    → XMP validation (xmllint --noout)
    → Normalized comparison (field-by-field, Unicode NFC, URL canonicalization, whitespace normalization, creator array ordering)
    → Coverage enforcement (minimums per format/category, source-aware external minimums)
    → Versioned run report envelope (ConformanceRunReport)
    → Structured report (JSON + human-readable)
```

## Fixture Categories

| Category | Purpose | Expected Outcome | Minimum (strict) |
|----------|---------|-----------------|-------------------|
| `canonical/` | Standard PLUS LDF metadata | All checks PASS | PNG≥1, JPEG≥1, WebP≥1 |
| `legacy/` | v0.3 backward compatibility | Extractable, may warn | ≥3 across ≥2 formats |
| `malformed/` | Invalid XML, truncated chunks | Graceful degradation | ≥4 total, ≥1 per format (png, jpeg, webp) |
| `conflicting/` | Contradictory declarations | Conflicts reported | ≥3 |
| `preservation/` | Existing metadata preservation | Unrelated fields survive | ≥3 across all 3 formats |

Coverage minimums are enforced by `CoverageMinimums` (explicit per-category and per-format thresholds, plus 7 source-aware external minimums). The blanket 75% external-fixture threshold has been replaced with these explicit minimums. The `malformed_per_format` field ensures malformed fixtures cover all three formats, not just a total count.

## Check Results

Each check produces a `CheckResult` with:
- `name`: check identifier (e.g., "copyright", "creators", "canonical_dmi")
- `severity`: `Pass`, `Warn`, or `Fail`
- `message`: human-readable description
- `details`: optional technical details

A report passes only when no checks have `Fail` severity.

## External Tools

| Tool | Role | Required |
|------|------|----------|
| ExifTool | Authoritative metadata extraction | Yes (strict mode) |
| xmllint | XMP well-formedness validation | Yes (strict mode) |
| ImageMagick | Format smoke tests | Yes (strict mode) |
| libvips | Container metadata checks | Yes (strict mode) |

## Strict Mode

- **Strict**: Missing tools cause failure. Digest mismatches cause failure. Coverage minimums enforced. All checks must pass. Requires `--manifest`. This is the single complete-validation mode.
- **Non-strict**: Missing tools produce warnings. Checks still run where possible. `--require-complete` has been removed; `--strict` is the only way to enable full validation.

## Field-Specific Normalization

Comparisons use field-specific normalization to avoid false mismatches:

- **Unicode**: NFC normalization via `unicode-normalization` crate for all text fields
- **URLs**: Canonical form comparison (trailing slash normalization, scheme/host case folding)
- **Whitespace**: Leading/trailing whitespace stripped, internal runs collapsed
- **Creator arrays**: Sorted before comparison (ordering is not semantically significant)
- **Mojibake exception**: `canonical_unicode` fixtures have a narrow exception for known UTF-8/mojibake encoding differences between internal and external extraction

## Fixture Manifest

The manifest (`tests/fixtures/conformance/manifest.toml`) is a TOML file
documenting every fixture with:
- Stable identifier and file path
- Image format and category
- Authoring tool and version
- SHA-256 digest for integrity verification
- Expected DMI values, conflict state, and legal fields
- Generation command and provenance

Digest verification runs before any conformance checks. In strict mode,
a digest mismatch is a hard failure.

## Manifest Validation

Before processing fixtures, the harness validates manifest structure via
`validate_manifest()`. This checks for:
- Duplicate fixture IDs
- Duplicate fixture paths
- Empty IDs
- Absolute paths or path traversal (`..`)
- Unsupported formats (only `png`, `jpeg`, `webp`)
- Unsupported categories (only `canonical`, `legacy`, `conflicting`, `malformed`, `preservation`)
- Unsupported sources (only `generated`, `external`, `historical`, `generated-negative`, `current-generated`)
- Empty or invalid SHA-256 digests (must be 64 hex characters)

Validation failures produce `EXIT_CONFIG` (2) before any fixtures are processed.

## Exit Codes

| Code | Constant | Meaning |
|------|----------|---------|
| 0 | `EXIT_PASS` | All checks passed |
| 1 | `EXIT_FAIL` | One or more checks failed |
| 2 | `EXIT_CONFIG` | Configuration error (missing manifest, invalid manifest, missing tools in strict mode) |
| 3 | `EXIT_DIGEST` | Digest mismatch between manifest and fixture file |
| 4 | `EXIT_COVERAGE` | Coverage minimums not met |
| 5 | `EXIT_INTERNAL` | Internal harness error |

Exit codes are stable and should not change without a version bump.

## External Tools in CI

ImageMagick and libvips are installed in both CI and release workflows.
The CI workflow installs all tools in a single `apt-get` step. The release
workflow installs them as separate named steps. Neither workflow uses
`continue-on-error` for conformance checks.

## JSON Report Schema

See `src/conformance.rs` for the full type definitions. The JSON output
is a `ConformanceRunReport` envelope wrapping per-fixture `ConformanceReport`
objects.

### ConformanceRunReport (top-level envelope)

| Field | Type | Description |
|-------|------|-------------|
| `schema_version` | `u32` | Report format version (currently 1) |
| `generated_by` | `String` | Tool identifier |
| `crate_version` | `String` | Crate version of the harness |
| `commit_sha` | `Option<String>` | Git commit, if available |
| `strict` | `bool` | Whether strict mode was enabled |
| `complete` | `bool` | Whether all required inputs/tools were available |
| `passed` | `bool` | Whether complete=true and no required check failed |
| `started_at` | `Option<String>` | ISO 8601 timestamp |
| `manifest` | `Option<ManifestReport>` | Manifest identity and validation |
| `tools` | `Vec<ToolReport>` | Per-tool discovery and invocation counts |
| `coverage_minimums` | `Option<CoverageMinimums>` | Minimums used for this run |
| `coverage` | `Option<CoverageCheckResult>` | Coverage enforcement result |
| `digest_verification` | `Vec<DigestCheckResult>` | Per-fixture SHA-256 results |
| `summary` | `ConformanceSummary` | Aggregate counts |
| `fixtures` | `Vec<ConformanceReport>` | Per-fixture reports |

### ConformanceReport (per-fixture)

Each report includes fixture metadata (with `fixture_id`, `category`, `source` from manifest), decode validity, XMP validity,
internal/external extractions, check results, and conflicts.

### External Integration Tests

External tool tests in `tests/external_tools.rs` use `#[ignore = "requires external tools: exiftool, xmllint, imagemagick, libvips"]`
and are run explicitly with `cargo test --test external_tools -- --ignored`.
The CI `external-integration` job and release workflow both execute these tests.

### Centralized Validation

`scripts/validate-release.sh` runs the full validation pipeline (hermetic + external phases).
Use `--skip-external` for environments without external tools installed.
