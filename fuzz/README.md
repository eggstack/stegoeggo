# Fuzz Targets

`cargo-fuzz` harnesses for the most untrusted-input-facing surfaces of `stegoeggo`.

## Targets

| Target | Surface | What it exercises |
|--------|---------|-------------------|
| `pipeline_bytes` | `process_image_bytes`, `verify_image_bytes` | Full public bytes-in / bytes-out path. Catches format-detection errors, encoder failures, and verification panics. |
| `tiled_round_trip` | Tiled stego embed/extract | Crop-resistant tiled steganography end-to-end. Verifies tiled payloads survive round-trips. |
| `jpeg_parser` | bounded JPEG dimension inspection used by `parse_jpeg_for_fuzz` | Exercises the public bounded JPEG validation path without exposing parser or coefficient types. |
| `payload_v3_parser` | `parse_payload`, v1/v2/v3 dispatch, TLV extensions | Payload version detection, header parsing, extension deserialization across all wire formats. |
| `png_metadata` | `MetadataTrapProtector::extract_seed_from_image`, `process_image_bytes` (Light) | PNG chunk parsing and metadata injection/update. |
| `webp_riff_parser` | `MetadataTrapProtector::extract_seed_from_image`, `verify_image_bytes` | WebP RIFF chunk parsing and container validation. |
| `xmp_extract` | `verify_legal_notice` | XMP extraction and normalization from PNG/JPEG/WebP metadata. |
| `metadata_merge` | `verify_legal_notice`, `process_image_bytes` (Standard) | Metadata conflict detection and merge policies on repeated processing. |
| `detached_manifest_parse` | `DetachedManifest::from_json` | Detached manifest bounded JSON parser with schema/version/count validation. |
| `detached_manifest_verify` | `verify_detached_manifest` | Detached manifest verification including signature, trust, and embedded-reference checks. |
| `provenance_canonicalize` | `DetachedManifest::canonical_bytes` | Provenance claim canonical serialization and determinism. |
| `verification_report` | `verify_image_bytes`, `verify_legal_notice` | Verification report aggregation across stego, metadata, and signing channels. |

## Running

The supported assurance tuple is Rust `nightly-2026-09-07` with
`cargo-fuzz 0.13.2`. The repository release profile enables LTO for product
builds, but cargo-fuzz's sanitizer-coverage instrumentation cannot be linked
with that LTO setting on the Linux runner. `CARGO_PROFILE_RELEASE_LTO=false`
changes only the fuzz build; AddressSanitizer and libFuzzer coverage remain
enabled.

Use the same explicit environment locally and in CI:

```bash
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run pipeline_bytes -- -max_total_time=60
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run tiled_round_trip -- -max_total_time=60
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run jpeg_parser -- -max_total_time=60
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run payload_v3_parser -- -max_total_time=60
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run png_metadata -- -max_total_time=60
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run webp_riff_parser -- -max_total_time=60
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run xmp_extract -- -max_total_time=60
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run metadata_merge -- -max_total_time=60
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run detached_manifest_parse -- -max_total_time=60
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run detached_manifest_verify -- -max_total_time=60
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run provenance_canonicalize -- -max_total_time=60
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run verification_report -- -max_total_time=60
```

Replace `pipeline_bytes` with any target listed above. To enumerate or build
all targets, use the same environment:

```bash
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz list
RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz build
```

The pinned tuple is updated manually after a bounded compatibility probe:
clean-build all 12 targets, run representative parser and pipeline smokes,
then verify both manual-dispatch and scheduled-equivalent workflow paths.
Do not float the nightly or cargo-fuzz version in the assurance workflow.

The workflow's normal manual dispatch runs the selected target. For remote
scheduled-equivalent evidence, dispatch it with `smoke=true`; this uses the
same rotating three-target job as the weekly schedule (the required `target`
input is ignored in that mode).

For an overnight run, drop the `-max_total_time` flag. The fuzzer will explore
the input space and report any crash, hang, or sanitizer hit.

## What to do on a finding

`cargo fuzz` writes a reproducing artifact under `fuzz/artifacts/<target>/`. To
investigate, add a regression test under `tests/robustness.rs` that feeds the
artifact to the same surface, then fix the underlying code. Do not commit
artifacts — add them to `.gitignore` or delete them after triage.

## Sanitizers

The default profile uses AddressSanitizer. For undefined-behavior coverage
(integer overflow, alignment), pass `--sanitizer=undefined` to
`cargo fuzz run`. The library is
`#![forbid(unsafe_code)]` so the UB surface is limited to upstream dependencies
and the standard library.
