# Conformance Architecture

The conformance suite validates that stegoeggo's metadata output is
interoperable with external parsers. It is a mandatory CI gate and
release prerequisite.

## Components

| Component | Location | Role |
|-----------|----------|------|
| Report types | `src/conformance.rs` | `ConformanceReport`, `CheckSeverity`, `FixtureManifest`, extraction structs |
| Harness binary | `src/bin/stegoeggo-conformance.rs` | CLI entry point, orchestrates all checks |
| Fixture taxonomy | `tests/fixtures/conformance/` | Test images organized by category |
| Fixture manifest | `tests/fixtures/conformance/manifest.toml` | Machine-readable fixture metadata with SHA-256 digests |
| Shell wrapper | `scripts/verify_metadata_conformance.sh` | Operator-friendly wrapper around harness |
| CI job | `.github/workflows/ci.yml` (conformance) | Mandatory gate on PRs and pushes to main |
| Release gate | `.github/workflows/release.yml` | Blocks publication if conformance fails |

## Conformance Flow

```
Fixture manifest (TOML)
    → SHA-256 digest verification
    → Fixture image (PNG/JPEG/WebP)
    → Format detection (magic bytes)
    → Image decode (image crate)
    → Internal extraction (verify_legal_notice)
    → External extraction (ExifTool -G -a)
    → XMP validation (xmllint --noout)
    → Normalized comparison (field-by-field)
    → Coverage enforcement (minimums per format/category)
    → Structured report (JSON + human-readable)
```

## Fixture Categories

| Category | Purpose | Expected Outcome | Minimum (strict) |
|----------|---------|-----------------|-------------------|
| `canonical/` | Standard PLUS LDF metadata | All checks PASS | PNG≥1, JPEG≥1, WebP≥1 |
| `legacy/` | v0.3 backward compatibility | Extractable, may warn | ≥3 across ≥2 formats |
| `malformed/` | Invalid XML, truncated chunks | Graceful degradation | ≥4 |
| `conflicting/` | Contradictory declarations | Conflicts reported | ≥3 |
| `preservation/` | Existing metadata preservation | Unrelated fields survive | ≥3 (PNG, JPEG, WebP) |

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

## Strict vs Non-Strict Mode

- **Strict**: Missing tools cause failure. Digest mismatches cause failure. Coverage minimums enforced. All checks must pass.
- **Non-strict**: Missing tools produce warnings. Checks still run where possible.

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

## JSON Report Schema

See `src/conformance.rs` for the full type definitions. The JSON output
is an array of `ConformanceReport` objects, one per fixture image.
Each report includes fixture metadata, decode validity, XMP validity,
internal/external extractions, check results, and conflicts.
