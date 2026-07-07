# Plan 007: Cleanup and Verification Closeout

## Goal

Close out the legal-notice-first work with a final cleanup and verification pass. The repo is now architecturally aligned with the clarified product goal: legal notice, rights-reservation metadata, AI-training restriction metadata, optional best-effort steganographic evidence, and optional authenticated provenance. The remaining work should verify that the implementation is coherent, CI-visible, externally testable, well documented, and ready for a public release.

This is a coordination plan. The detailed execution phases are split into follow-up files:

- `plans/008-ci-and-release-gates.md`
- `plans/009-external-metadata-conformance.md`
- `plans/010-notice-verification-api-audit.md`
- `plans/011-docs-terminology-and-release-notes.md`
- `plans/012-robustness-and-cli-polish.md`

## Current State

Recent work appears to have completed the major legal-notice shift:

- README and crate docs now lead with rights-reservation and AI-training restriction notice.
- CLI supports explicit legal fields such as copyright holder, creator, contact, rights URL, usage terms, AI constraints, no-AI-training, no-genAI-training, and TDM reservation.
- `EvidenceProfile` separates legal notice from authenticated provenance.
- Missing MAC key behavior is profile-aware instead of warning in ordinary legal-notice mode.
- `verify_legal_notice()` and `NoticeVerification` report legal fields, evidence strength, and evidence channels.
- The conformance script now auto-generates fixtures, supports `exiftool`, supports strict mode, and skips cleanly when tools are missing.
- The latest polish commit fixed channel-reporting issues and JPEG XMP namespace handling.

The remaining work should be conservative: validate and tighten what exists rather than redesigning the product.

## Closeout Principles

1. Prefer verification over new features.
2. Do not broaden claims into forensic watermarking, DRM, model-training proof, or data poisoning.
3. Ensure every evidence channel reported by the API is actually detected by code.
4. Ensure every public example works as written.
5. Ensure CI status is visible and release gates are reproducible.
6. Ensure optional external tools are handled explicitly and do not create confusing failures.
7. Preserve public API compatibility unless a deliberate version bump is planned.

## Phase Overview

### Phase 008: CI and Release Gates

Confirm GitHub Actions visibility, workflow triggers, MSRV checks, cargo package readiness, cargo deny policy, and whether Criterion benches belong in normal CI. This phase turns local claims such as “tests passed” into visible, reproducible release gates.

### Phase 009: External Metadata Conformance

Harden the external conformance story. Validate that exiftool and any optional metadata readers can see the rights-reservation fields in generated PNG/JPEG/WebP samples. Document exact expectations and avoid overclaiming unsupported parser visibility.

### Phase 010: Notice Verification API Audit

Audit `verify_legal_notice()`, `NoticeVerification`, `EvidenceStrength`, and `EvidenceChannel` for conservative behavior. Ensure channels are only reported when actually found. Ensure wrong-key, metadata-only, no-notice, DMI allowed, and TDM-reserved cases behave correctly.

### Phase 011: Docs, Terminology, and Release Notes

Ensure the README, crate docs, CLI docs, architecture docs, AGENTS, and changelog all match the current implementation. Remove or quarantine adversarial terminology. Make release notes clear about legal-notice-first behavior and optional provenance.

### Phase 012: Robustness and CLI Polish

Run final parser, malformed input, oversized metadata, CLI batch, output collision, strict-mode, and format-preservation checks. Fix any small UX bugs or panics that would affect real users or integrators.

## Global Validation Matrix

Run these commands before considering the closeout complete:

```bash
cargo fmt --check
cargo test --all-features
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo package --workspace --allow-dirty
cargo deny check licenses
cargo deny check advisories
```

If MSRV is supported locally:

```bash
rustup run 1.87 cargo check --all-features
```

If external tools are available:

```bash
./scripts/verify_metadata_conformance.sh
./scripts/verify_metadata_conformance.sh --strict
```

If nightly/cargo-fuzz is available:

```bash
cargo +nightly fuzz run pipeline_bytes -- -max_total_time=30
cargo +nightly fuzz run tiled_round_trip -- -max_total_time=30
cargo +nightly fuzz run jpeg_parser -- -max_total_time=30
```

## Final Acceptance Criteria

The closeout is complete when all of the following are true:

- GitHub Actions status is visible and green, or workflow visibility gaps are corrected.
- Local test, clippy, fmt, package, and deny checks pass.
- MSRV claim is verified or corrected.
- External conformance script works in normal and strict modes.
- README examples run successfully for at least PNG and JPEG, preferably WebP as well.
- `verify_legal_notice()` conservatively reports legal fields and channels.
- Wrong-key verification preserves metadata evidence while marking authenticated provenance invalid or unauthenticated.
- Legal-notice mode does not warn about missing MAC.
- Authenticated-provenance mode does warn or fail under strict policy when key material is missing.
- Public docs do not imply forensic watermarking, DRM, model-training proof, or data poisoning.
- Remaining deferred items, if any, are documented in a follow-up plan rather than hidden in commit messages.

## Non-Goals

Do not replace the steganography algorithm.

Do not add heavyweight metadata parser dependencies unless a concrete bug requires one.

Do not rename public API types in a breaking way during this closeout unless release policy explicitly permits it.

Do not attempt to give legal advice or claim legal enforceability.
