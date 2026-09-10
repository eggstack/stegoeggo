# Stability Tiers

## Stable API

The following API surfaces are stable and follow semantic versioning guarantees. Breaking changes will only occur in a semver-major release.

### Library Functions

| Function | Module |
|----------|--------|
| `process_image_bytes` | `stegoeggo` |
| `process_request_bytes` | `stegoeggo` |
| `process_request_bytes_with_warnings` | `stegoeggo` |
| `process_request_bytes_with_report` | `stegoeggo` |
| `verify_image_bytes` | `stegoeggo` |
| `verify_image_bytes_detailed` | `stegoeggo` |
| `verify_image_bytes_report` | `stegoeggo` |
| `verify_legal_notice` | `stegoeggo` |
| `compute_content_identifiers` | `stegoeggo` (`iscc` feature) |

### Core Types

| Type | Module |
|------|--------|
| `ProtectionRequest` | `stegoeggo::types` |
| `RightsPolicy` | `stegoeggo::types` |
| `RightsNotice` | `stegoeggo::types` |
| `LegalMetadata` | `stegoeggo::types` |
| `VerificationReport` | `stegoeggo::verification` |
| `VerificationStatus` | `stegoeggo::types` |
| `VerificationResult` | `stegoeggo::types` |
| `NoticeVerification` | `stegoeggo::types` |
| `ProtectionContext` | `stegoeggo::types` |
| `StegoPayload` | `stegoeggo::types` |
| `ProtectionPreset` | `stegoeggo::types` |
| `ExecutionReport` | `stegoeggo::types` |
| `ProtectionWarning` | `stegoeggo::types` |

### Generic Carrier API

The standalone carrier surface is stable: raw (`embed`/`extract`), strict
JPEG (`embed_strict`/`embed_framed_strict`), in-place
(`lsb::embed_in_place`), borrowed views (`PixelView`/`PixelViewMut`),
framed (`embed_framed`/`extract_framed`), prepared JPEG reuse
(`PreparedJpeg`), and tiled (`embed_tiled`, `embed_tiled_in_place`,
`extract_tiled`, `embed_tiled_framed`, `extract_tiled_framed`) operations
over LSB and JPEG carriers, with `Redundancy`, `LsbConfig`, `JpegConfig`,
`TileConfig`, `MAX_TILED_ORIGINS`, `StegoError`, and the capacity/embed
 report types. `jpeg::embed`/`embed_framed` remain best-effort
 compatibility operations (redundancy reduction plus seed-hint
 degradation); `EmbedOutcome`/`EmbedStatus`/`EmbedPath` are parent-owned
 application vocabulary retained through 0.x. `EmbedReport` and
 `InPlaceEmbedReport` public fields are frozen through 0.x (no new public
 fields will be added); the v1 boundary moves them to private fields plus
 getters for semver-safe evolution.

| Item | Module |
|------|--------|
| `lsb`, `jpeg`, `frame`, `error`, `types`, `pixels`, `prepared` carrier modules | `stegoeggo::stego` / `stegoeggo-stego` |
| `TileConfig`, `MAX_TILED_ORIGINS`, `Redundancy`, `PreparedJpeg`, `PixelView`, `PixelViewMut`, `PixelLayout` | `stegoeggo::stego` / `stegoeggo-stego` |

## Deprecated Compatibility API

The following API surfaces are deprecated. They remain functional but will be removed in v1.0.0 or the next semver-major release.

| Item | Replacement |
|------|-------------|
| `ProtectionLevel` | `ProtectionRequest` with `ProtectionPreset` |
| `EvidenceProfile` | `ProtectionPreset` |
| `with_dmi()` | `RightsPolicy` on `ProtectionRequest` |
| `with_metadata_injection()` | `ProtectionChannels::metadata_only()` or `ProtectionRequest` builder |
| `with_legal_claims()` | Auto-enabled with `LegalMetadata` (explicit `false` emits `ContradictoryLegalClaims`) |
| `compute_iscc()` | `compute_content_identifiers()` |

## Experimental Feature-Gated API

The following modules are experimental. API surfaces within them may change without notice between minor releases.

| Module | Feature Gate | Notes |
|--------|-------------|-------|
| `stegoeggo::signing` | `signatures` | Uses `ed25519-dalek` (real Ed25519). Private keys are not serialized. Trust evaluation is caller-owned. |
| `stegoeggo::detached` | `detached-manifest` | Signed sidecar manifests for distributing provenance outside the image. |
| `stegoeggo::iscc` | `iscc` | ISCC-like content-identifier computation (`compute_content_identifiers`). Requires `iscc-lib`, `blake3`, `xxhash`. |
| `stegoeggo::conformance` | `conformance` | Machine-readable conformance reporting types. Requires `toml`, `unicode-normalization`. |
| `stegoeggo` (async) | `async` | Tokio `spawn_blocking` wrappers. Canonical: `process_request_bytes_async`, `process_request_bytes_with_warnings_async`, `process_request_bytes_with_report_async` (plus `parallel` batch variants). Legacy level/context wrappers remain as compatibility adapters. Requires `tokio`. |
| `stegoeggo` (parallel) | `parallel` | Rayon-based parallel batch processing. Canonical: `process_request_bytes_parallel`, `process_request_bytes_with_warnings_parallel`, `process_request_bytes_with_report_parallel` (one shared request, order-preserving). Legacy `process_images_parallel` / `process_images_bytes_parallel` remain as compatibility adapters. Requires `rayon`. |

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
| `stegoeggo-conformance --json` | `conformance` | Run metadata conformance checks against ExifTool and xmllint |

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

### CLI Request Contract

All protect modes share one canonical `ProtectionRequest` builder with explicit
modern-vs-legacy precedence (modern wins, legacy translates only when modern
absent, contradictions exit 2). See `docs/cli-usage.md` and `architecture/cli.md`.

## Machine-Readable Schemas

The following output formats are stable and can be consumed by tooling.

| Schema | Description |
|--------|-------------|
| Conformance JSON report | Produced by `stegoeggo-conformance --json` |
| `VerificationReport` JSON | Produced by `verify_image_bytes_report` or CLI `--verify --json` (compatibility projection) |

## Verification model

`verify_image_bytes_report` is the canonical verification operation for rich
integrations. It performs one rights parse plus one hidden-marker search and
returns `VerificationReport`. `verify_image_bytes`, `verify_image_bytes_detailed`,
and `verify_legal_notice` are compatibility projections derived from the same
canonical facts. `VerificationStatus` remains stable and is not deprecated.

- Authentication: `attempted` means an HMAC payload was found; `hmac_status`
  distinguishes verified, failed, and missing-key cases; `key_matched` is true
  only when the supplied key verified the tag; `authenticated` requires all three.
- Binding: embedded verification reports default (empty) bindings; detached
  manifest verification reports instance-digest, format, dimensions, and file-size
  binding.
- Trust: embedded verification is untrusted by default (`caller-owned` model
  without a caller key); detached trust comes solely from the caller `TrustPolicy`.
- Marker validity: `Verified` means integrity passed; `Invalid` covers corruption,
  malformed v3, unsupported version, missing/failed auth, and resource-limit
  exhaustion (see report diagnostics for which); `NotFound` means no marker found.
  `summary_status` upgrades metadata-only `NotFound` to `Verified` for overall
  evidence; the coarse `VerificationStatus` projection stays stego-only.

## Internal Implementation Details

The following implementation details are oriented toward internal use. API stability is best-effort; they may change in minor releases without notice.

| Module | Description |
|--------|-------------|
| `jpeg_transcoder/` (`stegoeggo-stego`, private) | JPEG DCT coefficient processing |
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
