# Plan 019 Status: Closure Pass Acceptance Ledger

## Context

Plan 019 defined a closure pass for Plans 016–018. Plan 020 implemented the corrective work and produced auditable evidence.

## Acceptance Items

### Standards-correctness critical path (Plans 016–018)
- **Status**: CLOSED via Plan 020
- **Evidence**: 
  - Canonical PLUS namespace `http://ns.useplus.org/ldf/xmp/1.0/` enforced in XMP validation
  - `plus:DataMining` with PLUS LDF vocabulary keys is the canonical emission
  - Legacy `Iptc4xmpExt:DMI-*` properties parsed for backward compatibility
  - `DmiValue::plus_vocab_key()` returns canonical keys
  - TDMRep deployment deferred (no image-level TDM properties emitted)

### Fixture manifest and digests
- **Status**: CLOSED via Plan 020
- **Evidence**:
  - `tests/fixtures/conformance/manifest.toml` with 37 entries
  - SHA-256 digests verified by harness in strict mode
  - Manifest validation checks structure before processing

### Semantic and preservation tests
- **Status**: CLOSED via Plan 020
- **Evidence**:
  - `evaluate_manifest_expectations()` checks DMI, conflict, and legal fields
  - `expected_malformed` fixtures evaluated correctly
  - 724 tests pass (8 ignored external tool tests)

### NoticeVerification builder
- **Status**: CLOSED (implemented before Plan 020)
- **Evidence**: `NoticeVerification::builder()` pattern available

### External-tool installation in CI
- **Status**: CLOSED via Plan 020
- **Evidence**:
  - CI installs exiftool, xmllint, imagemagick, libvips-tools
  - Release workflow now installs all four tools
  - Tool versions uploaded as artifacts

### TDMRep deferral
- **Status**: CLOSED via Plan 020
- **Evidence**:
  - No `tdm:reserve_tdm` emitted by current writers
  - `--tdm-reserved` CLI flag deprecated with warning
  - README and architecture docs state deferral

### Conformance harness improvements
- **Status**: CLOSED via Plan 020
- **Evidence**:
  - Strict mode fails on missing/empty/incomplete fixture suites
  - Stable exit codes (0-5)
  - Manifest validated before processing
  - All manifest entries must be exercised
  - Unknown CLI arguments are fatal

### Documentation
- **Status**: CLOSED via Plan 020
- **Evidence**:
  - architecture/conformance.md updated
  - architecture/types.md updated
  - README.md updated
  - AGENTS.md updated

## Explicitly Deferred

- TDMRep web deployment artifacts (HTTP headers, `/.well-known/tdmrep.json`)
- Release 4 `RightsPolicy` and evidence-channel architecture
- New steganographic algorithms, payload versions, cryptographic signatures, C2PA
- New image formats
- Broad module reorganization

## CI Evidence Required

A clean CI run on main after the Plan 020 commit must show:
- All test jobs green
- Lint job green
- Security/deny jobs green
- External conformance job green with `conformance-report` and `tool-versions` artifacts
