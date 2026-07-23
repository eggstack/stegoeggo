# Fuzz Targets

`cargo-fuzz` harnesses for the most untrusted-input-facing surfaces of `stegoeggo`.

## Targets

| Target | Surface | What it exercises |
|--------|---------|-------------------|
| `pipeline_bytes` | `process_image_bytes`, `verify_image_bytes` | Full public bytes-in / bytes-out path. Catches format-detection errors, encoder failures, and verification panics. |
| `tiled_round_trip` | Tiled stego embed/extract | Crop-resistant tiled steganography end-to-end. Verifies tiled payloads survive round-trips. |
| `jpeg_parser` | `JpegHeader::parse`, `JpegTranscoder::decode_coefficients`, `DctStegoF5` | Hand-rolled JPEG parser and DCT coefficient decoder. Largest untrusted-byte consumer. |
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

Requires `cargo-fuzz` and nightly toolchain:

```bash
cargo +nightly fuzz run pipeline_bytes -- -max_total_time=60
cargo +nightly fuzz run tiled_round_trip -- -max_total_time=60
cargo +nightly fuzz run jpeg_parser -- -max_total_time=60
cargo +nightly fuzz run payload_v3_parser -- -max_total_time=60
cargo +nightly fuzz run png_metadata -- -max_total_time=60
cargo +nightly fuzz run webp_riff_parser -- -max_total_time=60
cargo +nightly fuzz run xmp_extract -- -max_total_time=60
cargo +nightly fuzz run metadata_merge -- -max_total_time=60
cargo +nightly fuzz run detached_manifest_parse -- -max_total_time=60
cargo +nightly fuzz run detached_manifest_verify -- -max_total_time=60
cargo +nightly fuzz run provenance_canonicalize -- -max_total_time=60
cargo +nightly fuzz run verification_report -- -max_total_time=60
```

For an overnight run, drop the `-max_total_time` flag. The fuzzer will explore
the input space and report any crash, hang, or sanitizer hit.

## What to do on a finding

`cargo fuzz` writes a reproducing artifact under `fuzz/artifacts/<target>/`. To
investigate, add a regression test under `tests/robustness.rs` that feeds the
artifact to the same surface, then fix the underlying code. Do not commit
artifacts — add them to `.gitignore` or delete them after triage.

## Sanitizers

The default profile uses AddressSanitizer. For undefined-behavior coverage
(integer overflow, alignment), switch the libfuzzer profile in `fuzz/Cargo.toml`
or pass `--sanitizer=undefined` to `cargo fuzz run`. The library is
`#![forbid(unsafe_code)]` so the UB surface is limited to upstream dependencies
and the standard library.
