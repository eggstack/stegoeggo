# Fuzz Targets

`cargo-fuzz` harnesses for the most untrusted-input-facing surfaces of `stegoeggo`.

## Targets

| Target | Surface | What it exercises |
|--------|---------|-------------------|
| `jpeg_parser` | `JpegHeader::parse`, `JpegTranscoder::decode_coefficients`, `DctStegoF5` | The hand-rolled JPEG parser and DCT coefficient decoder. This is the largest untrusted-byte consumer in the library. |
| `pipeline_bytes` | `process_image_bytes`, `verify_image_bytes` | The full public bytes-in / bytes-out path. Catches format-detection logic errors, encoder failures on degenerate inputs, and verification-time panics. |

## Running

Requires `cargo-fuzz` (already installed locally):

```bash
cargo fuzz run jpeg_parser -- -max_total_time=60
cargo fuzz run pipeline_bytes -- -max_total_time=60
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
