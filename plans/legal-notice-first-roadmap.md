# Legal-Notice-First Roadmap

## Purpose

This roadmap shifts `stegoeggo` toward its intended product center: legal notice, rights reservation, and evidentiary deterrence for copyrighted images that should not be used for AI/ML training without permission.

The project should not be framed primarily as forensic watermarking, anti-tamper watermarking, or security-grade image authentication. Its core claim is narrower and more useful: a distributed image can carry clear, redundant, standards-aligned metadata that communicates copyright ownership and AI-training restrictions. If a scraper, dataset builder, or model vendor later uses that image despite the embedded notice, the project should help show that the notice was present, discoverable, and intentionally communicated.

Steganographic payloads remain useful as a redundant best-effort evidence channel, but they are secondary to the legal notice. MAC/HMAC-backed payloads should be treated as optional authenticated provenance, not as the baseline requirement for the primary legal-deterrence workflow.

## Current State Summary

The current repository already has much of the needed implementation shape:

- A Rust library and CLI for image metadata injection and steganographic marking.
- Byte-oriented APIs that preserve or explicitly convert image formats.
- PNG/WebP metadata injection and LSB payload support.
- JPEG metadata, quantization-table seed, and DCT/F5-style payload support.
- Verification APIs that distinguish verified payloads, corrupted payloads, metadata-only evidence, and not-found results.
- A robustness matrix that already states the tool is a deterrent rather than a forensic watermark.
- Warning APIs that can report degraded stego or metadata conditions.

The main issue is framing and default workflow. Public docs and some API/CLI behavior still lean toward cryptographic or security-oriented language. The legal-notice use case should become the default, and security/provenance options should become clearly optional advanced modes.

## Target Product Posture

The project should communicate the following model everywhere:

`stegoeggo` embeds rights-reservation metadata into images. It is designed to make copyright ownership and AI-training restrictions visible to scrapers, metadata-aware tooling, archive systems, discovery workflows, and humans inspecting image files. It can also add hidden best-effort steganographic markers, but it does not claim that those markers survive arbitrary resizing, re-encoding, screenshots, compression, cropping, metadata stripping, or laundering through external image pipelines.

The project should be explicit that metadata stripping is not necessarily a failure under this model. If a served image contained standardized notice and a downstream pipeline stripped or ignored it, that may still support the intended evidentiary theory: the restriction was discoverable at distribution time.

## Non-Goals

The roadmap should avoid turning the project into one of the following:

- A forensic watermarking system.
- A tamper-proof watermarking system.
- A DRM system.
- A cryptographic proof that a model trained on a specific image.
- An adversarial data-poisoning project.
- A security product whose primary claim depends on secret keys.

## Strategic Principles

1. Metadata is primary. Stego is redundant.
2. Legal notice mode must not require MAC keys.
3. Warnings must be profile-aware: missing MAC is relevant to authenticated provenance, not to basic rights notice.
4. CLI defaults should guide non-experts toward real rights metadata: copyright holder, contact/rights URL, AI-training restriction, and standards-aligned DMI/TDM fields.
5. External metadata conformance matters more than internal self-parsing.
6. Public terminology should be neutral and legally defensible. Avoid words such as `poison` in public docs and new code paths.
7. Existing API users should get compatibility aliases where practical, with deprecation notes rather than abrupt breakage.

## Roadmap Phases

### Phase 1: Documentation and Terminology Reframe

Move the legal-notice threat model to the front of the README, crate docs, CLI help, and supporting documentation. Replace security-heavy or adversarial language with rights-reservation language. Add a plain `What this is / is not` section. Rename or deprecate public-facing uses of `poison`, `trap`, and `anti-scraping` where they are not necessary.

Deliverables:

- Updated README opening section.
- Updated crate-level docs in `src/lib.rs`.
- Updated CLI help text in `stegoeggo-cli/src/main.rs`.
- New `docs/legal_notice_model.md`.
- Terminology audit notes in the changelog.

### Phase 2: CLI Legal Metadata First Pass

Make the CLI able to express actual legal notice fields rather than only toggling generic legal claims. Add flags for copyright holder, creator, contact, rights URL, usage terms, AI constraints, and no-training presets.

Deliverables:

- CLI flags for legal metadata fields.
- Mapping from CLI flags to `LegalMetadata` and `DmiValue`.
- Default examples that show legal-notice workflows first.
- Strict validation for obviously empty or malformed rights metadata where appropriate.
- Tests for CLI argument parsing and resulting metadata injection.

### Phase 3: Evidence Profiles

Add explicit profiles or builder helpers that encode the intended posture:

- `LegalNotice`: metadata-only, no MAC required.
- `LegalNoticeWithStego`: metadata plus best-effort hidden marker, no MAC required.
- `AuthenticatedProvenance`: hidden payload with MAC/HMAC expected.
- `Maximal`: all available legal notice and stego channels, with authentication if configured.

Deliverables:

- New enum or builder helpers.
- Profile-aware defaults for metadata, DMI, stego, and warning behavior.
- Backward-compatible behavior for existing `ProtectionLevel` users.
- Tests proving missing MAC is not a legal-notice warning.

### Phase 4: Warning Policy Cleanup

Split warnings into legal-notice warnings and stego/provenance warnings. Missing MAC should only be emitted when the caller selected authenticated provenance or a policy that requires cryptographic payload verification.

Deliverables:

- Revised `ProtectionWarning` variants or added severity/category fields.
- Warning categorization by evidence profile.
- CLI display grouping for legal-notice warnings versus optional stego/provenance warnings.
- Regression tests for each profile.

### Phase 5: Verification and Conformance

Make verification report legal evidence channels explicitly. Add external metadata conformance scripts or tests so generated sample files can be validated by third-party metadata tools.

Deliverables:

- New legal-notice verification struct/report.
- CLI output that reports rights fields independently from stego payload status.
- Optional `scripts/verify_metadata_conformance.sh` using exiftool or equivalent.
- Tiny generated fixture workflow for PNG/JPEG/WebP metadata inspection.
- Documentation of expected third-party parser visibility.

### Later Phases

Additional work after the first five handoff plans:

- Internal rename/deprecation pass for `MetadataTrapProtector` and `generate_poison_metadata`.
- Release migration guide for 0.2.x to the legal-notice-first API.
- Optional cargo features for external conformance tests.
- Expanded standards validation against IPTC examples and TDM/DMI parser expectations.
- Better operational logging examples for WAF/CDN integrations.

## Success Criteria

The shift is complete when a new user can open the README and immediately understand that the tool embeds copyright and AI-training restriction notice, not forensic watermarking. A user should be able to run one CLI command with real copyright holder and rights URL fields, verify the output, and see the legal-notice fields reported clearly.

The repo should still support steganographic markers and authenticated provenance, but those should read as optional evidence layers rather than the center of the product.

## Validation Checklist

- README first screen communicates legal notice and rights reservation.
- CLI `--help` shows legal metadata flags before advanced provenance flags.
- Missing MAC is not a warning in legal-notice mode.
- Legal-notice verification reports copyright, rights URL, DMI/TDM, and AI constraints.
- External metadata tooling can see expected fields in generated PNG/JPEG/WebP samples.
- Existing public API either remains compatible or has clear deprecation paths.
- Changelog explains the terminology and behavior shift.
