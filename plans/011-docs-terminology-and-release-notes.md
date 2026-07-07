# Plan 011: Docs, Terminology, and Release Notes

## Goal

Make the public documentation, crate metadata, architecture docs, CLI help, AGENTS notes, and changelog match the current legal-notice-first implementation. The repo should read consistently as a rights-reservation and AI-training restriction metadata tool with optional best-effort stego and optional authenticated provenance.

This phase should remove ambiguity, reduce adversarial/security-heavy phrasing, and prepare release notes that accurately describe the behavior shift.

## Scope

In scope:

- README quick start and examples.
- Crate-level docs.
- CLI help and architecture docs.
- Legal-notice model documentation.
- Changelog/release notes.
- AGENTS handoff accuracy.
- Terminology audit.
- Crate metadata/package metadata.

Out of scope:

- Legal advice.
- API-breaking rename work unless already scheduled for a breaking release.
- New implementation features.

## Files to Inspect

- `README.md`
- `src/lib.rs`
- `stegoeggo-cli/src/main.rs`
- `docs/legal_notice_model.md`
- `docs/architecture/*.md`
- `AGENTS.md`
- `CHANGELOG.md`
- `Cargo.toml`
- `stegoeggo-cli/Cargo.toml` if present
- Any examples under `examples/`
- Any skills or agent docs if tracked in-repo

## Terminology Policy

Preferred public terms:

- legal notice
- rights-reservation metadata
- AI-training restriction notice
- DMI/TDM reservation
- evidence channel
- best-effort steganographic marker
- authenticated provenance
- verification report

Terms to avoid in public-facing docs unless explicitly qualified:

- poison
- trap
- anti-scraping
- forensic watermark
- tamper-proof
- secure watermark
- DRM
- data poisoning
- proof of model training

Internal file names such as `metadata_trap.rs` can remain for compatibility and churn control, but public docs should explain them in neutral terms or defer rename work to a separate compatibility plan.

## README Requirements

The first screen of the README should communicate:

1. The project embeds rights-reservation and AI-training restriction metadata.
2. Stego is optional/redundant evidence.
3. MAC/HMAC is optional authenticated provenance.
4. The project is not forensic watermarking, DRM, or proof that a model trained on an image.

The quick start should prioritize explicit legal fields:

```bash
stegoeggo artwork.png -o artwork_protected.png \
  --copyright-holder "Jane Artist" \
  --creator "Jane Artist" \
  --rights-url "https://example.com/rights/artwork" \
  --no-genai-training
```

Then show optional authenticated provenance separately:

```bash
stegoeggo artwork.png -o artwork_auth.png \
  --profile authenticated-provenance \
  --key deadbeefcafebabe \
  --copyright-holder "Jane Artist" \
  --rights-url "https://example.com/rights/artwork" \
  --no-ai-training
```

## CLI Help Requirements

CLI help should make legal notice the default surface:

- Legal metadata flags should be easy to discover.
- `--profile legal-notice` should be documented as default.
- `--key` should say optional authenticated provenance, not mandatory production protection.
- `--verify` should mention legal-notice report, evidence strength, and channels.
- `--strict` should say it fails on profile-relevant error-severity warnings.

If clap help grouping is not currently used, consider adding headings for:

- Input/output.
- Legal notice metadata.
- Evidence profile.
- Stego/provenance advanced options.
- Batch/diagnostics.

## Legal Notice Model Document

Review `docs/legal_notice_model.md` for:

- Clear no-legal-advice disclaimer.
- Explanation of what the tool helps show: notice present and discoverable at distribution time.
- Explanation that metadata stripping is not necessarily a technical failure under the evidence model.
- Explanation that the tool does not prove infringement or model training.
- Operational recommendations: keep originals, protected outputs, hashes, timestamps, rights URLs, generation logs, and conformance outputs.
- Format limitations for PNG/JPEG/WebP.

## Architecture Docs

Update architecture docs to match current code:

- `notice_verification.rs` module map.
- Current CLI line count or remove brittle line counts if they churn often.
- `EvidenceProfile` versus `ProtectionLevel` distinction.
- `NoticeVerification` fields and invariants.
- Warning category/severity behavior.
- Conformance script behavior.

If line counts are updated repeatedly and add little value, consider replacing exact counts with approximate or removing them from docs.

## Changelog Requirements

Add coherent unreleased entries:

```markdown
### Added
- Legal-notice verification API: `verify_legal_notice()` and `NoticeVerification`.
- Evidence profiles for legal notice, legal notice with stego, authenticated provenance, and maximal mode.
- CLI legal metadata flags for copyright holder, creator, contact, rights URL, usage terms, AI constraints, and training restrictions.
- External metadata conformance script.

### Changed
- Default public framing is legal notice and rights-reservation metadata.
- Missing MAC warning is profile-aware and no longer emitted for legal-notice mode.
- CLI verification now reports legal-notice fields and evidence strength.

### Fixed
- JPEG XMP evidence channel reporting.
- JPEG XMP namespace marker.
- Evidence channel misclassification for payload type.
```

If any change is behaviorally breaking, call it out.

## AGENTS Requirements

Update AGENTS with:

- Current validation commands.
- Current test count only if stable; otherwise use command-based guidance.
- New gotchas around `EvidenceProfile`, `NoticeVerification`, `--strict`, and conformance script behavior.
- A caution not to reintroduce forensic/security watermarking claims.
- A note that `MetadataTrapProtector` naming is historical/internal and public copy should say rights metadata or notice metadata.

## Crate Metadata Requirements

Inspect `Cargo.toml` for:

- Description matching current product posture.
- Keyword count compatible with crates.io.
- Categories appropriate.
- README path valid.
- Repository/homepage/docs links correct.
- License correct.
- MSRV accurate.

Crates.io keyword policy is strict; if too many keywords are present, reduce to the strongest few.

## Required Commands

```bash
cargo fmt --check
cargo test --doc
cargo package --workspace --allow-dirty
cargo run -p stegoeggo-cli -- --help
```

Manual doc checks:

```bash
grep -Rni "poison\|anti-scraping\|forensic\|tamper-proof\|data poisoning" README.md docs AGENTS.md src stegoeggo-cli || true
```

Review matches manually. Some source references may be legitimate, but public-facing copy should be neutral.

## Acceptance Criteria

- README leads with legal notice and explicit legal metadata examples.
- CLI help text is clear and legal-notice-first.
- Legal-notice model doc is accurate and cautious.
- Architecture docs match current modules and API behavior.
- Changelog accurately summarizes the implementation sequence.
- Crate metadata is release-valid.
- Public-facing docs avoid adversarial/security-overclaim terminology.
- Examples compile or run as documented.

## Risk Notes

Do not scrub all technical mentions of steganography. Stego is real functionality; it should be described as best-effort redundant evidence.

Do not say metadata proves infringement. Keep documentation factual.

Avoid large API renames in this docs pass unless a separate breaking-change decision has been made.
