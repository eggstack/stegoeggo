# Plan 012: Robustness and CLI Polish

## Goal

Perform the final user-facing and parser-safety polish pass before release. The legal-notice implementation is now feature-complete enough that remaining work should focus on malformed input handling, oversized metadata handling, CLI edge cases, batch/output behavior, strict-mode UX, and regression tests.

This phase should make the tool safe and predictable for real image-processing workflows without expanding the product scope.

## Scope

In scope:

- Malformed PNG/JPEG/WebP metadata handling.
- Oversized legal metadata values.
- CLI output-path and batch behavior.
- CLI strict-mode behavior.
- CLI verify-mode behavior.
- Format-specific edge cases.
- Warning UX.
- Small regression tests and docs.

Out of scope:

- New watermarking algorithms.
- New image formats.
- Full metadata parser dependencies.
- Legal advice or legal workflow automation.

## Files to Inspect

- `stegoeggo-cli/src/main.rs`
- `src/protected/metadata_trap.rs`
- `src/protected/notice_verification.rs`
- `src/protected/steganography.rs`
- `src/jpeg_transcoder/*`
- `src/error.rs`
- `tests/robustness.rs`
- `tests/*cli*`
- `tests/*notice*`
- `fuzz/fuzz_targets/*`
- `README.md`
- `AGENTS.md`

## Workstream A: Malformed Metadata Robustness

### Tasks

1. Review PNG chunk parsing for checked length arithmetic and CRC handling.
2. Review JPEG marker parsing for truncated COM/APP1/APP13 segments.
3. Review WebP RIFF chunk parsing for invalid sizes, odd-size padding, and RIFF length overflow.
4. Ensure `notice_verification.rs` never panics on malformed metadata.
5. Ensure malformed legal-notice metadata results in absent fields or recoverable errors, not crashes.
6. Add regression tests for any uncovered malformed input.

### Suggested Tests

```text
notice_verify_truncated_png_text_no_panic
notice_verify_bad_png_chunk_len_no_panic
notice_verify_truncated_jpeg_xmp_no_panic
notice_verify_bad_jpeg_marker_len_no_panic
notice_verify_truncated_webp_xmp_no_panic
notice_verify_bad_webp_riff_len_no_panic
```

## Workstream B: Oversized Legal Metadata

### Tasks

1. Determine maximum safe field lengths for each target format.
2. Add validation or graceful errors for oversized copyright holder, creator, contact, rights URL, usage terms, and AI constraints.
3. Ensure JPEG marker length limits are respected.
4. Ensure PNG chunk lengths use checked arithmetic.
5. Ensure WebP RIFF size updates cannot overflow.
6. Ensure CLI errors clearly identify which field is too large.

### Suggested Policy

Use conservative limits unless format-specific support is already robust:

- Short identity/contact fields: warn or reject above a practical threshold.
- Usage terms / AI constraints: allow longer values but enforce format limits.
- Rights URL: reject empty and oversized values; do not over-validate URL syntax unless lightweight.

Do not silently truncate legal metadata. Truncation can create misleading notice.

### Suggested Tests

```text
cli_rejects_empty_copyright_holder
cli_rejects_oversized_rights_url
metadata_injection_rejects_oversized_jpeg_com
metadata_injection_large_png_text_checked
metadata_injection_webp_size_overflow_checked
```

## Workstream C: CLI Verify and Strict UX

### Tasks

1. Verify `--verify` uses output file if provided, otherwise input file.
2. Verify `--verify --key` passes the key to `verify_legal_notice()`.
3. Verify wrong-key output is clear: metadata found, authenticated provenance failed/not authenticated.
4. Verify no-key legal-notice output is not scary or security-worded.
5. Verify `--strict` exits non-zero only for profile-relevant error-severity warnings.
6. Verify `--profile legal-notice --strict` does not fail due to missing MAC.
7. Verify `--profile authenticated-provenance --strict` fails when key is missing.
8. Verify warning output is grouped or otherwise readable.

### Suggested CLI Output Rules

Legal notice first:

```text
Rights notice: Found
Copyright holder: ...
Rights URL: ...
AI training restriction: ...
```

Then optional channels:

```text
Stego marker: Verified / Invalid / Not found
Authenticated provenance: Verified / Not configured / Invalid
Evidence strength: ...
Channels: ...
```

Avoid phrasing that implies legal conclusions.

## Workstream D: Batch and Output Path Behavior

### Tasks

1. Test single-file output to explicit file path.
2. Test batch output to directory.
3. Test duplicate input stems producing non-colliding outputs.
4. Test output path collision with existing files. Confirm behavior is documented: overwrite, reject, or suffix.
5. Test output path equal to input path. Prefer reject unless explicitly intended.
6. Test batch mode with `--jobs > 1` and legal metadata flags.
7. Ensure per-file errors are reported without hiding successful outputs.
8. Ensure shared `ProtectionContext` use does not race or mutate incorrectly.

### Suggested Tests

```text
cli_rejects_output_same_as_input
cli_batch_duplicate_stems_suffix_outputs
cli_batch_jobs_preserve_legal_metadata
cli_existing_output_policy_documented
cli_batch_partial_failure_reports_errors
```

## Workstream E: Format-Specific Smoke Tests

### Tasks

Run and, where useful, automate smoke tests for:

- PNG input -> PNG output legal notice.
- PNG input -> JPEG output legal notice.
- PNG input -> WebP output legal notice.
- JPEG input -> JPEG output legal notice using byte/DCT fast path.
- JPEG input -> PNG output legal notice.
- WebP input -> WebP output legal notice.
- Metadata-only/light profile.
- Legal notice with stego profile.
- Authenticated provenance profile.

For each output, run:

- `stegoeggo --verify --verbose`.
- `verify_legal_notice()` test if applicable.
- External conformance script for at least representative PNG/JPEG/WebP.

## Workstream F: Fuzz and Regression Smoke

### Tasks

Run fuzz smoke if available:

```bash
cargo +nightly fuzz run pipeline_bytes -- -max_total_time=30
cargo +nightly fuzz run tiled_round_trip -- -max_total_time=30
cargo +nightly fuzz run jpeg_parser -- -max_total_time=30
```

For any crash, add the minimized input as a regression test if appropriate.

If fuzz tooling is unavailable, run the robustness test suite and document that fuzz was not run in closeout notes.

## Required Commands

```bash
cargo fmt --check
cargo test --all-features robustness
cargo test --all-features notice_verification
cargo test --all-features cli
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

Manual smoke examples:

```bash
cargo run -p stegoeggo-cli -- sample.png -o /tmp/notice.png \
  --profile legal-notice \
  --copyright-holder "Test Artist" \
  --rights-url "https://example.com/rights" \
  --no-genai-training \
  --verbose

cargo run -p stegoeggo-cli -- /tmp/notice.png --verify --verbose

cargo run -p stegoeggo-cli -- sample.png -o /tmp/auth.png \
  --profile authenticated-provenance \
  --key deadbeefcafebabe \
  --copyright-holder "Test Artist" \
  --rights-url "https://example.com/rights" \
  --no-ai-training

cargo run -p stegoeggo-cli -- /tmp/auth.png --verify --key deadbeefcafebabe --verbose
cargo run -p stegoeggo-cli -- /tmp/auth.png --verify --key badbadbadbadbadb --verbose
```

## Acceptance Criteria

- Malformed metadata does not panic.
- Oversized legal metadata is rejected or handled with clear errors; no silent truncation.
- CLI verify output is clear and legal-notice-first.
- Strict mode is profile-relative and predictable.
- Batch mode handles duplicate names and errors safely.
- Output path collision behavior is documented and tested.
- Format-specific smoke tests pass for PNG/JPEG/WebP.
- Fuzz smoke or robustness tests complete without new crashes.

## Risk Notes

Do not silently drop legal fields when they are too large. Users need to know when the notice they requested was not embedded.

Avoid adding dependencies just for rich URL/email validation. Keep validation practical and focused on preventing empty or impossible metadata.

Be careful with batch parallelism: if the context contains random seeds or mutable state, ensure each output gets intended reproducibility behavior.
