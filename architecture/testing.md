# Testing (Integration Tests, Fuzz, Examples)

**Sources:** `tests/` (35 files) · `fuzz/fuzz_targets/` (12 harnesses, `cargo-fuzz` + nightly only, excluded from workspace tests) · `examples/` (4) · `stegoeggo-stego/tests/direct_consumer.rs`.

## Integration tests (`tests/`)

Grouped by area; run with `cargo test --workspace --exclude stegoeggo-fuzz --all-features`, single test via `-- <name>`. `tests/external_tools.rs` is `#[ignore]` (run with `--ignored`); `tests/async_integration.rs` needs `async`; conformance harness tests need `conformance`.

| Group | Files |
|-------|-------|
| Pipeline & request API | `basic.rs`, `integration.rs`, `request_api.rs`, `request_aux_convergence.rs`, `semantic_correctness.rs` |
| Formats & preservation | `preservation.rs`, `preservation_idempotence.rs`, `metadata_only_idempotence.rs`, `cross_format_semantics.rs`, `cross_format_closure.rs`, `output_domain_routing.rs`, `jpeg_container_preservation.rs`, `container_accounting.rs`, `canonical_rights.rs`, `merge_policy.rs` |
| Payload & verify | `payload_v3_roundtrip.rs`, `independent_v3_parser.rs`, `known_answer_vectors.rs`, `verification_convergence.rs`, `verification_report_tests.rs`, `public_stego_api.rs` |
| Signing & manifests | `signing_tests.rs` (`signatures`), `detached_manifest_tests.rs` (`detached-manifest`), `provenance_claim_tests.rs` |
| Conformance | `conformance_parser_tests.rs`, `conformance_negative.rs`, `conformance_container_tests.rs`, `conformance_harness_tests.rs` (`conformance`), `generate_conformance_fixtures.rs` |
| Robustness & soak | `robustness.rs` (fuzz regression tests — add new findings here), `soak_tests.rs` |
| Gates & compat | `plan026_gate1_2_3_tests.rs`, `plan065_legacy_compat.rs`, `async_integration.rs` (`async`), `external_tools.rs` (`#[ignore]`) |

Conformance harness exit codes: 0 pass / 1 fail / 2 config / 3 digest / 4 coverage / 5 internal.

## Fuzz (`fuzz/`)

Reproducible tuple: `nightly-2026-09-07`, `cargo-fuzz 0.13.2`, `RUSTUP_TOOLCHAIN=nightly-2026-09-07 CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run <target> -- -max_total_time=60`. Release-profile LTO stays on for artifacts but must be off for sanitizer linking. Sync parity checked by `scripts/check_fuzz_sync.sh`.

| Target | Fuzzes |
|--------|--------|
| `pipeline_bytes` | Full protection pipeline on arbitrary bytes |
| `tiled_round_trip` | Tiled stego embed/extract round-trip |
| `jpeg_parser` | JPEG header/structure parsing |
| `payload_v3_parser` | V3 payload wire format parsing |
| `png_metadata` | PNG metadata injection/extraction |
| `webp_riff_parser` | WebP RIFF container parsing |
| `xmp_extract` | XMP packet field filtering |
| `metadata_merge` | XMP metadata merging |
| `detached_manifest_parse` | Detached manifest JSON parsing |
| `detached_manifest_verify` | Detached manifest verification |
| `provenance_canonicalize` | Provenance claim canonical JSON |
| `verification_report` | Verification report building |

Policy: `fuzz/README.md` (tuple, LTO boundary, update policy). Never runs on PRs.

## Examples (`examples/`)

Must keep compiling on public-API change: `protect_and_verify.rs` (protect → save → verify), `verify_saved.rs` (verify a file), `legal_metadata.rs` (copyright + legal fields), `generic_stego.rs` (direct `stegoeggo-stego` carrier use).

## Carrier consumer tests

`stegoeggo-stego/tests/direct_consumer.rs` exercises the public API as a third party (no `application-support`): LSB raw/framed/in-place/tiled, strided views, JPEG strict/best-effort/tiled, `PreparedJpeg` reuse, seed hints, progressive rejection, error exhaustiveness.
