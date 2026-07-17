# Conformance Architecture

The conformance suite validates that stegoeggo's metadata output is
interoperable with external parsers. It is a mandatory CI gate and
release prerequisite.

## Components

| Component | Location | Role |
|-----------|----------|------|
| Report types | `src/conformance.rs` | `ConformanceReport`, `CheckSeverity`, extraction structs |
| Harness binary | `src/bin/stegoeggo-conformance.rs` | CLI entry point, orchestrates all checks |
| Fixture taxonomy | `tests/fixtures/conformance/` | Test images organized by category |
| Shell wrapper | `scripts/verify_metadata_conformance.sh` | Operator-friendly wrapper around harness |
| CI job | `.github/workflows/ci.yml` (conformance) | Mandatory gate on PRs and pushes to main |
| Release gate | `.github/workflows/release.yml` | Blocks publication if conformance fails |

## Conformance Flow

```
Fixture image (PNG/JPEG/WebP)
    → Format detection (magic bytes)
    → Image decode (image crate)
    → Internal extraction (verify_legal_notice)
    → External extraction (ExifTool -G -a)
    → XMP validation (xmllint --noout)
    → Normalized comparison (field-by-field)
    → Structured report (JSON + human-readable)
```

## Fixture Categories

| Category | Purpose | Expected Outcome |
|----------|---------|-----------------|
| `canonical/` | Standard PLUS LDF metadata | All checks PASS |
| `legacy/` | v0.2 backward compatibility | Extractable, may warn |
| `malformed/` | Invalid XML, truncated chunks | Graceful degradation |
| `conflicting/` | Contradictory declarations | Conflicts reported |
| `preservation/` | Existing metadata preservation | Unrelated fields survive |

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
| ImageMagick | Format smoke tests | No |
| libvips | Container metadata checks | No |

## Strict vs Non-Strict Mode

- **Strict**: Missing tools cause failure. All checks must pass.
- **Non-strict**: Missing tools produce warnings. Checks still run where possible.

## JSON Report Schema

See `src/conformance.rs` for the full type definitions. The JSON output
is an array of `ConformanceReport` objects, one per fixture image.
