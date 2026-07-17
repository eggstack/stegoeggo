# Conformance Test Fixtures

This directory contains test fixtures for the independent interoperability and
conformance test suite (Plan 018).

## Directory Structure

```
conformance/
├── README.md              # This file
├── canonical/             # Canonical PLUS LDF rights metadata fixtures
├── legacy/                # v0.2 legacy format fixtures for backward compat
├── malformed/             # Malformed XMP, invalid XML, truncated chunks
├── conflicting/           # Conflicting rights declarations
└── preservation/          # Metadata preservation test inputs
```

## Fixture Categories

### canonical/
Canonical PLUS LDF metadata fixtures. At least one fixture per supported
format (PNG, JPEG, WebP) should be independently authored — not generated
by stegoeggo — to avoid closed-loop testing.

### legacy/
v0.2 legacy fixtures retained permanently for backward compatibility.
These contain `Iptc4xmpExt:DMI-*` properties and legacy TDM reservations.

### malformed/
Invalid XML, truncated chunks, unknown namespace bindings, and oversized
metadata. These test graceful degradation and error reporting.

### conflicting/
Fixtures with contradictory rights declarations: canonical allowed vs
legacy prohibited, dual canonical values, conflicting rights URLs.

### preservation/
Input images with existing camera EXIF, ICC profiles, unrelated XMP
namespaces, IPTC records, and orientation data. Used to verify metadata
preservation through the update path.

## Provenance

Each fixture category must have documented provenance:
- How it was generated (tool, version, or hand-authored)
- Expected normalized fields
- Expected pass/fail classification
- License or public-domain status

## Adding Fixtures

1. Place the image in the appropriate category directory
2. Document provenance in this README or a per-category PROVENANCE.md
3. Add expected field assertions in the conformance test harness
4. Verify the fixture passes `cargo test --all-features`
5. Verify the fixture passes `./scripts/verify_metadata_conformance.sh --strict`

## Running Conformance Checks

```bash
# Run all fixtures through the Rust harness
cargo run --bin stegoeggo-conformance -- --fixtures tests/fixtures/conformance --strict

# Run through the shell wrapper
./scripts/verify_metadata_conformance.sh --all-formats --strict

# Generate JSON report
cargo run --bin stegoeggo-conformance -- --fixtures tests/fixtures/conformance --strict --json conformance-report.json
```
