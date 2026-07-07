# Plan 006: Verification Polish and Release Readiness

## Goal

Perform a focused verification and polish pass after the legal-notice-first implementation work. The repo now has the right product posture: legal notice and rights-reservation metadata are primary, steganography is a redundant best-effort evidence channel, and MAC/HMAC is optional authenticated provenance. This pass should confirm the implementation, documentation, CLI behavior, tests, and conformance tooling all agree with that model.

The desired outcome is a release-ready state where a maintainer can trust that:

- Legal-notice workflows are clear and usable from the CLI and library.
- Verification reports actual legal-notice fields without overstating forensic or security claims.
- Evidence channels reported by `NoticeVerification` match what is actually extracted.
- Warning severity is profile-aware and not noisy for legal-notice users.
- External metadata conformance checks exist and are documented.
- CI/test status can be verified from GitHub Actions, not only from local commit messages.

## Background

The recent implementation sequence appears to have completed the main roadmap items:

- Plan 001: README/crate/CLI/docs reframe around legal notice and rights reservation.
- Plan 002: CLI legal metadata fields.
- Plan 003: `EvidenceProfile` and profile-aware `MissingMacKey` handling.
- Plan 004: warning category/severity helpers and strict-mode behavior.
- Plan 005: `verify_legal_notice()`, `NoticeVerification`, evidence strength/channel reporting, and external conformance script.

This plan is not a redesign. Treat it as a closure pass: verify, tighten, reduce ambiguity, and document any deliberately deferred work.

## Primary Files to Inspect

Core API and behavior:

- `src/lib.rs`
- `src/types.rs`
- `src/protected/notice_verification.rs`
- `src/protected/metadata_trap.rs`
- `src/protected/steganography.rs`
- `src/jpeg_transcoder/stego_f5.rs`

CLI:

- `stegoeggo-cli/src/main.rs`
- `docs/architecture/cli.md` if present

Docs:

- `README.md`
- `docs/legal_notice_model.md`
- `docs/architecture/types.md` if present
- `AGENTS.md`
- `CHANGELOG.md`

Tests and tooling:

- `tests/`
- `scripts/verify_metadata_conformance.sh`
- `.github/workflows/ci.yml`
- `deny.toml`
- `Cargo.toml`
- `Cargo.lock`

## Workstream A: Confirm CI and Local Test Cleanliness

### Tasks

1. Confirm the latest `main` commit has a visible GitHub Actions run.
2. If no run exists, inspect workflow triggers and branch settings. The current CI file should run on pushes to `main` and pull requests to `main`; if connector/API status is blank but Actions UI has runs, document that in the closeout notes.
3. Run the complete local validation matrix.
4. If `cargo bench` is still part of push-to-main CI, decide whether to keep it, make it manual/scheduled, or reduce it to a smoke benchmark. Full Criterion benches on every push can be expensive and noisy.
5. Confirm `cargo package --workspace` succeeds if the project is meant to be crates.io-release-ready.
6. Confirm MSRV behavior with Rust 1.87 if the workflow still advertises that MSRV.

### Required Commands

Run at minimum:

```bash
cargo fmt --check
cargo test --all-features
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo package --workspace --allow-dirty
cargo deny check licenses
cargo deny check advisories
```

Run MSRV if available:

```bash
rustup run 1.87 cargo check --all-features
```

Run fuzz smoke only if nightly/cargo-fuzz is available:

```bash
cargo +nightly fuzz run pipeline_bytes -- -max_total_time=30
cargo +nightly fuzz run tiled_round_trip -- -max_total_time=30
cargo +nightly fuzz run jpeg_parser -- -max_total_time=30
```

### Acceptance Criteria

- CI status is confirmed or the lack of connector-visible status is documented.
- All local validation commands pass or failures are filed as concrete follow-up items.
- MSRV claim is either verified or corrected.
- Bench workflow policy is intentional.

## Workstream B: Verify CLI Legal-Notice Workflows End-to-End

### Tasks

1. Generate small PNG, JPEG, and WebP inputs.
2. Run the CLI with explicit legal metadata fields.
3. Verify output with `stegoeggo --verify`.
4. Verify output with `--key` when a key was used.
5. Verify that legal metadata flags automatically enable legal claims and metadata injection.
6. Verify that `--metadata false` plus any legal metadata flag exits with a clear error.
7. Verify that `--profile legal-notice` does not warn about a missing MAC key.
8. Verify that `--profile authenticated-provenance` without `--key` warns or fails under `--strict` according to the documented severity rules.
9. Verify batch mode still handles duplicate filenames and does not lose per-file legal metadata.
10. Verify `--known-seeds` is either used by verification or removed/documented as unsupported. A stale parsed-but-unused option should not remain.

### Manual Smoke Commands

Use generated sample files or existing fixtures:

```bash
cargo run -p stegoeggo-cli -- sample.png -o /tmp/notice.png \
  --profile legal-notice \
  --copyright-holder "Test Artist" \
  --creator "Test Artist" \
  --contact "legal@example.com" \
  --rights-url "https://example.com/rights/test-art" \
  --usage-terms "Copyrighted work. All rights reserved." \
  --no-genai-training \
  --verbose

cargo run -p stegoeggo-cli -- /tmp/notice.png --verify --verbose
```

Authenticated provenance:

```bash
cargo run -p stegoeggo-cli -- sample.png -o /tmp/auth.png \
  --profile authenticated-provenance \
  --key deadbeefcafebabe \
  --copyright-holder "Test Artist" \
  --rights-url "https://example.com/rights/test-art" \
  --no-ai-training \
  --verbose

cargo run -p stegoeggo-cli -- /tmp/auth.png --verify --key deadbeefcafebabe --verbose
cargo run -p stegoeggo-cli -- /tmp/auth.png --verify --key badbadbadbadbadb --verbose
```

Strict mode checks:

```bash
cargo run -p stegoeggo-cli -- sample.png -o /tmp/legal.png --profile legal-notice --strict
cargo run -p stegoeggo-cli -- sample.png -o /tmp/auth-missing-key.png --profile authenticated-provenance --strict
```

### Acceptance Criteria

- CLI produces usable legal-notice outputs with explicit artist/rights fields.
- `--verify` prints legal-notice fields before stego/provenance status.
- `--key` is actually used during verify mode.
- Wrong-key verification does not suppress metadata evidence reporting.
- Legal-notice profile has no missing-MAC noise.
- Authenticated provenance profile has clear missing-key behavior.
- Stale CLI flags are either wired or removed.

## Workstream C: Audit `NoticeVerification` Accuracy

### Tasks

1. Compare `EvidenceChannel` enum variants against actual extraction support in `notice_verification.rs`.
2. Ensure the report never lists a channel unless that channel was actually detected.
3. Ensure public docs distinguish supported extraction from reserved/future enum variants.
4. Confirm `has_notice()` only returns true for meaningful legal notice fields, not merely a seed or internal marker.
5. Confirm `EvidenceStrength` does not overstate evidence when there is stego but no legal metadata.
6. Confirm wrong-key authenticated payloads report stego invalid/authenticated false while preserving legal metadata fields.
7. Confirm metadata-only outputs produce `MetadataNoticeOnly` rather than a generic protected status.
8. Confirm unprotected images produce `NoNoticeFound` and no evidence channels.
9. Confirm DMI extraction distinguishes `Allowed`, `Unspecified`, and prohibition values correctly.
10. Confirm TDM reservation extraction is not conflated with DMI unless intentionally documented.

### Specific Channel Audit

Inspect and test these channels:

- `PngText`: should be listed only when PNG tEXt/iTXt legal fields are found.
- `PngXmp`: should be listed only if PNG XMP/iTXt XMP is emitted and parsed.
- `JpegComment`: should be listed only when JPEG COM legal fields are parsed.
- `JpegXmp`: should be listed only when JPEG APP1 XMP is parsed.
- `JpegIptc`: should be listed only when APP13/IPTC is parsed.
- `WebPXmp`: should be listed only when WebP XMP chunk is parsed.
- `WebPExif`: should be listed only when WebP EXIF legal fields are parsed.
- `LsbPayload`: should be listed only when an LSB payload verifies.
- `DctPayload`: should be listed only when DCT/F5 payload verifies.
- `QTableSeed`: should be listed only when a JPEG quantization-table seed is detected.

If some variants are reserved for future support, either do not expose them yet or document them as currently unused/reserved. Avoid implying conformance that is not implemented.

### Suggested Tests

Add or adjust tests for:

```text
notice_verification_png_text_fields
notice_verification_jpeg_comment_fields
notice_verification_webp_xmp_fields
notice_verification_no_false_channels
notice_verification_wrong_key_preserves_metadata
notice_verification_unprotected_no_notice
notice_verification_dmi_allowed_not_restriction
notice_verification_tdm_reserved_separate_from_dmi
```

### Acceptance Criteria

- Evidence channels are accurate and non-overstated.
- Evidence strength classification is conservative.
- Tests cover wrong-key, metadata-only, unprotected, and per-format legal fields.
- Docs match actual extraction support.

## Workstream D: External Metadata Conformance Polish

### Tasks

1. Inspect `scripts/verify_metadata_conformance.sh` for portability and clear output.
2. Ensure the script gracefully skips optional tools unless `--strict` is passed.
3. Ensure it can generate or locate tiny input fixtures without requiring user assets.
4. Validate at least one external metadata reader when installed, preferably `exiftool`.
5. Document installation hints for optional tools without making them hard dependencies.
6. Consider adding a CI job that runs the script in a container/runner with exiftool installed. If this is too expensive, add a manual workflow.
7. Ensure the script checks for actual legal fields, not just internal seed markers.
8. Ensure outputs are written under `/tmp` or a temporary directory and cleaned up.

### Suggested Script Behavior

Default mode:

```text
[ok] generated PNG legal-notice fixture
[ok] stegoeggo verify sees copyright holder
[skip] exiftool not installed; external parser checks skipped
```

Strict mode:

```text
[fail] exiftool not installed; strict conformance requires external parser visibility
```

### Acceptance Criteria

- Script is documented in README or `docs/legal_notice_model.md`.
- Script verifies externally visible copyright holder, rights URL, and AI restriction where tools support it.
- Missing optional tools are handled clearly.
- CI/manual workflow policy is documented.

## Workstream E: Warning Policy and Strict Mode Polish

### Tasks

1. Inspect `ProtectionWarning::category()` and `severity_for_profile()` mappings.
2. Ensure `MissingMacKey` is never emitted in `LegalNotice` or `LegalNoticeWithStego` unless the user explicitly selected authenticated provenance.
3. Confirm `Maximal` missing-MAC severity matches docs. If `Maximal` says MAC optional, missing MAC should not be an error.
4. Confirm `MetadataInjectionDisabled` severity is an error for legal-notice profiles.
5. Confirm stego capacity warnings are not fatal in pure legal-notice mode.
6. Confirm `--strict` is profile-relative, not a global fail-on-any-warning trap unless explicitly documented.
7. Add tests for all warning/severity profile combinations.
8. Ensure CLI warning output is grouped and actionable.

### Suggested Tests

```text
warning_missing_mac_not_legal_notice
warning_missing_mac_authenticated_error
warning_metadata_disabled_legal_notice_error
warning_stego_capacity_legal_notice_info_or_warning
warning_maximal_missing_mac_not_error_if_optional
cli_strict_profile_relative
```

### Acceptance Criteria

- Warning behavior aligns with the legal-notice-first purpose.
- Strict mode does not make normal legal-notice usage unnecessarily brittle.
- Docs and implementation agree on each profile’s expectations.

## Workstream F: Terminology and API Surface Cleanup

### Tasks

1. Search public docs and CLI output for adversarial/security-heavy terminology:
   - `poison`
   - `trap`
   - `anti-scraping`
   - `forensic`
   - `tamper-proof`
   - `secure watermark`
   - `data poisoning`
2. Keep historical/internal references only when necessary and clearly qualified.
3. Consider adding compatibility aliases for future rename work:
   - `MetadataTrapProtector` -> `RightsMetadataProtector` or `NoticeMetadataProtector`
   - `generate_poison_metadata` -> `generate_notice_metadata`
4. If aliases are added, mark old public names as deprecated only if semver policy allows it.
5. Do not rename files in this pass unless the churn is small and all docs/tests are updated.
6. Ensure README/package description never implies forensic watermarking.

### Acceptance Criteria

- Public docs and CLI help consistently use rights-reservation/legal-notice language.
- Remaining `trap`/`poison` usage is either internal-only or covered by a future deprecation note.
- API compatibility is preserved unless a deliberate version bump/migration note is added.

## Workstream G: Documentation Consistency and Release Notes

### Tasks

1. Update README examples to use explicit legal metadata fields first.
2. Ensure docs explain `EvidenceProfile` versus `ProtectionLevel` clearly.
3. Ensure docs explain `EvidenceStrength` without implying legal conclusions.
4. Ensure `docs/legal_notice_model.md` has a clear no-legal-advice disclaimer.
5. Ensure docs say that metadata stripping can itself be relevant to the evidence story, but the tool does not decide legal outcomes.
6. Ensure JPEG/WebP/PNG limitations remain visible and accurate.
7. Update `CHANGELOG.md` with the legal-notice implementation sequence and any breaking/behavior changes.
8. Update `AGENTS.md` with current test counts, new gotchas, and validation commands.
9. If package keywords include more than crates.io allows, correct them before release.

### Acceptance Criteria

- README quick start demonstrates real legal notice fields.
- Docs do not overstate survival, cryptographic, or forensic capabilities.
- Changelog is coherent for release notes.
- Agent handoff docs reflect current APIs and test counts.

## Workstream H: Security and Robustness Regression Checks

This repo is not a security watermarking product, but image parsing still requires defensive handling.

### Tasks

1. Re-run parser robustness tests for malformed PNG/JPEG/WebP metadata.
2. Confirm `notice_verification.rs` handles malformed chunks without panics.
3. Confirm external-parser conformance fixtures do not depend on unsafe temp paths.
4. Confirm CLI does not overwrite input files unintentionally when output paths collide.
5. Confirm batch mode with parallel jobs does not race output names or shared context mutation.
6. Confirm `--rights-url`, `--contact`, and other user-provided strings are bounded by image format limits and fail gracefully if too large.
7. Confirm JPEG COM/APP marker length handling is safe for long legal metadata values.
8. Confirm PNG chunk length arithmetic remains checked.
9. Confirm WebP RIFF size updates are checked for overflow.

### Acceptance Criteria

- Malformed metadata inputs do not panic.
- Oversized legal metadata returns clear errors.
- Parallel/batch output behavior remains deterministic enough for users.
- Fuzz smoke runs do not discover immediate regressions.

## Closeout Deliverables

At the end of the pass, commit the following if changes were needed:

- Code fixes for verification/report/warning accuracy.
- Tests for any discovered gaps.
- Documentation updates.
- Optional CI/manual conformance workflow updates.
- `CHANGELOG.md` entry.

Also add a short closeout note in `plans/` if there are deferred items that should not block release, for example:

```text
plans/007-deferred-standards-conformance.md
```

Only create that if meaningful deferred work remains.

## Final Acceptance Checklist

The repo is ready after this pass when all of the following are true:

- `cargo fmt --check` passes.
- `cargo test --all-features` passes.
- `cargo test --doc` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo package --workspace --allow-dirty` passes or any package exclusion is intentional and documented.
- `cargo deny check licenses` and `cargo deny check advisories` pass.
- GitHub Actions status is confirmed or the lack of visible status is explained.
- CLI legal-notice examples work for PNG/JPEG/WebP.
- `verify_legal_notice()` conservatively reports legal fields, stego status, authentication, and channels.
- External conformance script works or skips cleanly.
- README/legal notice docs match implementation behavior.
- No public-facing copy suggests forensic watermarking, DRM, data poisoning, or security-grade proof of model training.

## Non-Goals

Do not redesign the core steganography algorithm in this pass.

Do not attempt to prove legal enforceability.

Do not add heavyweight metadata parser dependencies unless a specific correctness gap requires them.

Do not break public API names merely for terminology cleanup unless the release is explicitly allowed to include breaking changes.
