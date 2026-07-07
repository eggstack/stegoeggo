# Plan 009: External Metadata Conformance

## Goal

Verify that images protected by `stegoeggo` expose legal-notice metadata to external metadata tools, not only to stegoeggo’s own parser. This matters because the project’s evidentiary value depends on rights-reservation notice being present and discoverable by ordinary metadata-aware systems.

The conformance pass should make the external visibility story specific, testable, and conservative.

## Scope

In scope:

- `scripts/verify_metadata_conformance.sh` portability and strict-mode behavior.
- External parser visibility for generated PNG/JPEG/WebP samples.
- Documentation of which fields are expected to be visible in which formats.
- Optional CI/manual workflow for conformance checks.
- Fixes to metadata emission when external parsers cannot see intended fields.

Out of scope:

- Building a full standards-complete XMP/IPTC parser.
- Claiming legal enforceability.
- Supporting every metadata tool.

## Files to Inspect

- `scripts/verify_metadata_conformance.sh`
- `src/protected/metadata_trap.rs`
- `src/protected/notice_verification.rs`
- `README.md`
- `docs/legal_notice_model.md`
- `docs/architecture/protected-metadata-trap.md` if present
- `.github/workflows/*.yml`
- Tests under `tests/`

## External Tools

Primary target:

- `exiftool`

Optional secondary targets:

- ImageMagick `identify -verbose` or `magick identify -verbose`
- `vipsheader` or `vips`
- `file`

Do not require secondary tools for normal success unless they are added to a dedicated conformance CI job.

## Expected Fields

The script should validate externally visible fields where supported:

- Copyright holder.
- Creator/author if emitted.
- Contact field if emitted.
- Rights URL / web statement of rights.
- Usage terms.
- AI constraints.
- DMI/TDM reservation marker where external parser exposes it.
- Internal protection seed only as a secondary diagnostic, not as the primary legal-notice proof.

The script should not pass merely because a seed or internal marker is present. It must check at least one real rights-reservation field.

## Format Matrix

Create or update a documented matrix like this:

```text
Format  Internal verify  exiftool copyright  exiftool rights URL  exiftool AI restriction  Notes
PNG     required         expected            expected              expected/extracted       tEXt/iTXt/XMP visibility
JPEG    required         expected            expected              expected/extracted       COM/XMP/APP markers
WebP    required         expected if supported expected if supported expected if supported  XMP/EXIF chunk parser variability
```

If a field is not reliably visible through a given external tool, document it as `not guaranteed` rather than silently failing or overclaiming.

## Script Behavior

Default mode should be developer-friendly:

```bash
./scripts/verify_metadata_conformance.sh
```

Expected behavior:

- Auto-generates a small sample image if no input is supplied.
- Builds or locates the CLI clearly.
- Emits a protected sample with explicit legal metadata.
- Runs stegoeggo internal verification.
- Runs `exiftool` checks if available.
- Skips external checks cleanly if exiftool is missing.
- Cleans up temporary files.

Strict mode should be release-oriented:

```bash
./scripts/verify_metadata_conformance.sh --strict
```

Expected behavior:

- Fails if `exiftool` is missing.
- Fails if expected legal fields are not visible externally.
- Returns non-zero when any required check fails.

## Tasks

### 1. Review Script Portability

Confirm the script works on macOS and Linux shells using portable Bash. Avoid GNU-only assumptions unless documented.

Check:

- `mktemp` compatibility.
- `find` usage.
- `grep`/`sed` portability.
- Quoting of paths with spaces.
- Cleanup trap behavior.
- Python fallback availability or clear error.

### 2. Validate Auto-Generated Fixtures

Confirm the generated fixture includes explicit legal fields:

- `Test Corp` copyright holder.
- Creator.
- Contact.
- Rights URL.
- Usage terms.
- AI constraints.
- DMI no-AI-training restriction.

Confirm the fixture generation works from a clean checkout after:

```bash
cargo build -p stegoeggo-cli
```

### 3. Add Per-Format Fixture Mode

If not already present, extend the script so it can generate and check multiple formats:

```bash
./scripts/verify_metadata_conformance.sh --format png
./scripts/verify_metadata_conformance.sh --format jpg
./scripts/verify_metadata_conformance.sh --format webp
./scripts/verify_metadata_conformance.sh --all-formats
```

If this is too large for one pass, document it as deferred and ensure PNG/JPEG are covered first.

### 4. Compare External and Internal Reports

For each generated file:

1. Run `stegoeggo --verify --verbose`.
2. Run `exiftool`.
3. Compare expected fields.
4. Fail strict mode if stegoeggo sees fields but external visibility is expected and absent.

### 5. Fix Metadata Emission Gaps

If exiftool cannot see intended XMP or IPTC fields:

- Inspect marker/chunk formatting.
- Validate JPEG XMP standard namespace header.
- Validate PNG iTXt/XMP layout.
- Validate WebP RIFF chunk sizes and chunk names.
- Validate UTF-8 and null separator usage.

Add tests for any fix.

### 6. Documentation

Update README or `docs/legal_notice_model.md` with:

- How to run the conformance script.
- What strict mode means.
- Which external tools are optional.
- Which fields are expected to be externally visible by format.
- Caveat that metadata visibility varies by tool and format.

## Required Commands

```bash
cargo build -p stegoeggo-cli
./scripts/verify_metadata_conformance.sh
./scripts/verify_metadata_conformance.sh --strict
```

If per-format mode is added:

```bash
./scripts/verify_metadata_conformance.sh --all-formats --strict
```

Manual external checks:

```bash
exiftool /tmp/stegoeggo_conformance_*/test_protected.png
exiftool /tmp/stegoeggo_conformance_*/test_protected.jpg
exiftool /tmp/stegoeggo_conformance_*/test_protected.webp
```

## Acceptance Criteria

- Script runs successfully from a clean checkout after building the CLI.
- Non-strict mode skips cleanly when `exiftool` is missing.
- Strict mode fails clearly when `exiftool` is missing.
- Strict mode verifies externally visible legal-notice fields when `exiftool` is installed.
- At least PNG and JPEG are externally validated; WebP is validated or documented as parser-dependent.
- Script does not treat internal seed-only markers as sufficient legal notice.
- Documentation describes the conformance workflow.

## Risk Notes

External metadata tooling is inconsistent across formats. Do not claim universal visibility when only one parser sees a field.

Keep the conformance script optional for normal development unless installation reliability is high enough for CI.

Avoid committing generated binary fixtures unless they are tiny, intentional, and documented.
