# Stability Tiers

## Stable API

The following API surfaces are stable and follow semantic versioning guarantees. Breaking changes will only occur in a semver-major release.

### Library Functions

| Function | Module |
|----------|--------|
| `process_image_bytes` | `stegoeggo` |
| `process_request_bytes` | `stegoeggo` |
| `verify_legal_notice` | `stegoeggo::protected` |
| `verify_image_bytes` | `stegoeggo` |
| `compute_content_identifiers` | `stegoeggo::util` |

### Core Types

| Type | Module |
|------|--------|
| `ProtectionRequest` | `stegoeggo::types` |
| `RightsPolicy` | `stegoeggo::types` |
| `RightsNotice` | `stegoeggo::types` |
| `LegalMetadata` | `stegoeggo::types` |
| `VerificationReport` | `stegoeggo::types` |
| `NoticeVerification` | `stegoeggo::types` |
| `ProtectionContext` | `stegoeggo::types` |
| `StegoPayload` | `stegoeggo::types` |
| `ProtectionPreset` | `stegoeggo::types` |
| `ExecutionReport` | `stegoeggo::types` |
| `ProtectionWarning` | `stegoeggo::types` |

## Deprecated Compatibility API

The following API surfaces are deprecated. They remain functional but will be removed in v1.0.0 or the next semver-major release.

| Item | Replacement |
|------|-------------|
| `ProtectionLevel` | `ProtectionRequest` with `ProtectionPreset` |
| `EvidenceProfile` | `ProtectionPreset` |
| `with_dmi()` | `RightsPolicy` on `ProtectionRequest` |
| `with_metadata_injection()` | `ProtectionChannels::metadata_only()` or `ProtectionRequest` builder |
| `with_inject_legal_claims()` | Auto-enabled with `LegalMetadata` on `ProtectionRequest` |
| `compute_iscc()` | `compute_content_identifiers()` |
| `VerificationStatus` | `VerificationReport` |

## Experimental Feature-Gated API

The following modules are experimental. API surfaces within them may change without notice between minor releases.

| Module | Feature Gate | Notes |
|--------|-------------|-------|
| `stegoeggo::signing` | `signatures` | Uses `ed25519-dalek` (real Ed25519). Private keys are not serialized. Trust evaluation is caller-owned. |
| `stegoeggo::detached` | `detached-manifest` | Signed sidecar manifests for distributing provenance outside the image. |

## CLI Stable Commands and Flags

The following CLI commands and their primary flags are stable.

The CLI uses a single-binary design with positional arguments for the `protect`/`verify` mode and subcommands for signing operations.

| Command | Feature Gate | Description |
|---------|-------------|-------------|
| `stegoeggo <input>...` | (default) | Protect an image with legal metadata and steganography (positional args + flags) |
| `stegoeggo --verify <input>` | (default) | Verify protection status of an image |
| `stegoeggo keygen` | `signatures` | Generate an Ed25519 key pair |
| `stegoeggo sign --manifest <path> --key <path>` | `signatures` | Sign a detached manifest |
| `stegoeggo verify-manifest --manifest <path> --image <path>` | `signatures` | Verify a detached manifest against an image |

### CLI Exit Codes

| Code | Constant | Meaning |
|------|----------|---------|
| 0 | `EXIT_OK` | Success |
| 1 | `EXIT_ERROR` | General error (I/O, image decode/encode, etc.) |
| 2 | `EXIT_CONFIG` | Malformed manifest, config error, or input validation failure |
| 3 | `EXIT_INTEGRITY` | Digest mismatch, binding failure, or signature/integrity failure |
| 4 | `EXIT_TRUST` | Cryptographically valid but untrusted (no matching public key) |
| 5 | `EXIT_INTERNAL` | Internal or unexpected error |

The `verify-manifest` subcommand returns structured `ManifestVerification` results (via `verify_detached_manifest()` from `stegoeggo::detached::verify`) and uses `--json` for machine-readable output. Exit codes follow the table above.

### CLI Bounded Parsing

Both `sign` and `verify-manifest` use `DetachedManifest::from_json_with_limits` with `ResourceLimits::default()` instead of raw `serde_json::from_slice`, enforcing bounded parsing of manifest JSON.

## Machine-Readable Schemas

The following output formats are stable and can be consumed by tooling.

| Schema | Description |
|--------|-------------|
| Conformance JSON report | Produced by `stegoeggo-conformance --json` |
| `VerificationReport` JSON | Produced by `verify_image_bytes` or CLI `--json` |

## Internal Implementation Details

The following modules are public but oriented toward internal use. API stability is best-effort; they may change in minor releases without notice.

| Module | Description |
|--------|-------------|
| `jpeg_transcoder/` | JPEG DCT coefficient processing |
| `payload_v3/` | Payload v3 wire format and TLV extensions |

## Deprecation Removal Policy

Deprecated API surfaces introduced in v0.2.2 are guaranteed to remain available through the v0.x series. They will be removed in v1.0.0 or the next semver-major release, whichever comes first.

## Retention Promises

The following capabilities are guaranteed for the lifetime of the 0.x series and into v1.0.0:

- **Canonical metadata reading and writing**: Standard legal metadata (copyright, creator, contact, rights URL, usage terms, AI constraints) in PNG tEXt, JPEG COM/XMP, and WebP XMP will continue to be read and written.
- **Legacy metadata reading**: Legacy `Iptc4x3mpExt:DMI-*` properties and `tdm:reserve_tdm` will continue to be parsed for backward compatibility, even though they are no longer emitted.
- **Payload v1/v2/v3 reading**: Extraction of all three payload versions will continue to be supported. Write output uses v3 only.
- **Detached manifest v1 reading**: Verification of v1 detached manifests will continue to be supported.
- **CLI JSON schemas**: The JSON output format for conformance reports and verification results will maintain backward-compatible schema versions. New fields may be added; existing fields will not be removed or renamed within the same major version.
