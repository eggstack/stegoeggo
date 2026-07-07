# Plan 004: Profile-Aware Warning Policy

## Goal

Revise warning generation so it matches the selected evidence model. Legal-notice mode should warn about degraded or missing rights-reservation metadata. Authenticated provenance mode should warn about missing MAC keys and cryptographic verification weaknesses.

The current warning surface is useful but too security/stego-centered for the primary legal-deterrence workflow. In particular, `MissingMacKey` should not be treated as a production warning for legal-notice use. It is relevant only when the caller wants authenticated hidden-payload provenance.

## Scope

This plan covers warning taxonomy, warning generation, CLI presentation, and tests. It assumes Plan 003 either added `EvidenceProfile` or at least added builder helpers/context state that warning code can inspect.

If Plan 003 has not landed, first implement the minimal profile field required for this plan.

## Files to Inspect

- `src/types.rs`
- `src/lib.rs`
- `src/protected/steganography.rs`
- `src/protected/metadata_trap.rs`
- `stegoeggo-cli/src/main.rs`
- Tests that mention `ProtectionWarning`
- README sections that describe warnings or production policy

## Desired Warning Categories

Warnings should be grouped by what they affect.

### Legal Notice Warnings

These indicate that the primary legal-notice evidence channel is incomplete or degraded.

Suggested variants:

```rust
NoRightsMetadataConfigured
NoCopyrightHolderConfigured
NoRightsUrlConfigured
NoDmiRestrictionConfigured
NoTdmReservationConfigured
MetadataInjectionDisabled
MetadataFormatUnsupported
MetadataInjectionSkipped
LegalClaimsMissingMetadata
```

Only add variants that can be reliably detected with current code. Do not add noisy warnings that fire in normal use without a clear remediation.

### Best-Effort Stego Warnings

These indicate that the optional hidden marker may be weak or absent.

Existing or suggested variants:

```rust
LsbCapacitySkipped
DctCapacityInsufficient
ProgressiveJpegFallback
JpegReencodeFragile
WebPLossyFragile
TileCapacityInsufficient
```

### Authenticated Provenance Warnings

These apply to cryptographic provenance claims.

Existing or suggested variants:

```rust
MissingMacKey
UnauthenticatedPayloadOnly
MacKeyProvidedButNoPayloadEmbedded
```

`MissingMacKey` belongs here, not in legal-notice mode.

## Warning Severity and Category

If changing the enum shape is acceptable, add helper methods rather than breaking variants:

```rust
impl ProtectionWarning {
    pub fn category(&self) -> WarningCategory { ... }
    pub fn severity_for_profile(&self, profile: EvidenceProfile) -> WarningSeverity { ... }
}
```

Suggested enums:

```rust
pub enum WarningCategory {
    LegalNotice,
    BestEffortStego,
    AuthenticatedProvenance,
    FormatFragility,
}

pub enum WarningSeverity {
    Info,
    Warning,
    Error,
}
```

If this is too much API churn, keep the enum unchanged and centralize profile-filtering in a private helper.

## Warning Generation Rules

### LegalNotice

Emit warnings for:

- Metadata disabled.
- No meaningful rights metadata configured, if legal notice profile expects explicit fields.
- No DMI/TDM restriction configured, if auto-DMI cannot resolve a restriction.
- Format does not support metadata injection or metadata injection fails.

Do not emit:

- `MissingMacKey`.
- DCT capacity warnings unless stego is also enabled and warnings are explicitly requested.

### LegalNoticeWithStego

Emit legal notice warnings and best-effort stego warnings.

Do not emit `MissingMacKey`.

If stego capacity is insufficient but metadata notice is successfully embedded, message should make clear that the legal notice still exists while the redundant marker is weaker.

### AuthenticatedProvenance

Emit:

- Missing MAC as warning or strict error.
- Stego capacity warnings.
- Metadata warnings if metadata is part of the selected workflow.

If the caller explicitly disables metadata in authenticated provenance mode, do not emit legal notice warnings unless the profile promises metadata.

### Maximal

Emit all relevant warnings, but distinguish legal-notice warnings from optional authentication warnings.

If severity support exists, missing MAC in `Maximal` may be `Info` or `Warning` depending on whether `Maximal` is defined as requiring authentication. Be explicit in docs.

## CLI Display

Switch CLI processing to `process_image_bytes_with_warnings()` if Plan 002 has not already done this.

Group warnings in output:

```text
Legal notice warnings:
  - No rights URL configured. Add --rights-url <URL>.

Optional stego/provenance warnings:
  - JPEG re-encoding may remove hidden payloads.
```

In non-verbose mode, consider printing only actionable warnings. In verbose mode, print all warnings.

If `--strict` is added, strict should fail on legal-notice warnings for legal-notice profile and on authenticated-provenance warnings for authenticated profile. Avoid strict failing on optional stego warnings in legal-notice mode unless the user selected a stego-required profile.

Suggested future strict semantics:

```text
--strict
    Fail when warnings compromise the selected evidence profile.
```

## Library API Behavior

`process_image_bytes_with_warnings()` should remain the primary warning API. Avoid making `process_image_bytes()` fail or warn; it should remain a compatibility helper.

Consider adding:

```rust
process_image_bytes_with_report(...)
```

but only if the warning API becomes too overloaded. Verification/report redesign belongs more directly in Plan 005.

## Implementation Steps

1. Review existing `ProtectionWarning` variants and all emission sites.
2. Add `EvidenceProfile` awareness to warning generation.
3. Suppress `MissingMacKey` outside authenticated provenance contexts.
4. Add legal-notice warnings that are cheap and reliable to detect.
5. Add helper methods for warning category/severity if feasible.
6. Update CLI to use and display warnings by category.
7. Update README warning guidance.
8. Add tests for each profile.

## Suggested Test Cases

Required:

1. LegalNotice profile without MAC does not emit `MissingMacKey`.
2. LegalNoticeWithStego profile without MAC does not emit `MissingMacKey`.
3. AuthenticatedProvenance without MAC emits `MissingMacKey`.
4. Metadata disabled in LegalNotice emits `MetadataInjectionDisabled` or equivalent.
5. Legal metadata flags with metadata disabled produce a CLI error or profile-compromising warning.
6. Small image with insufficient LSB capacity in LegalNoticeWithStego emits a stego warning but not a legal-notice failure if metadata succeeded.
7. JPEG output emits JPEG fragility warning only in profiles where that matters or as categorized info.
8. CLI `--strict --profile legal-notice` fails on legal-notice warnings but not missing MAC.

## Test Commands

Run:

```bash
cargo fmt --check
cargo test --all-features
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
```

Manual CLI checks:

```bash
cargo run -p stegoeggo-cli -- image.png -o /tmp/out.png --profile legal-notice --verbose
cargo run -p stegoeggo-cli -- image.png -o /tmp/out.png --profile authenticated-provenance --verbose
```

The first command should not warn about missing MAC. The second should warn unless `--key` is provided.

## Acceptance Criteria

- Missing MAC is no longer a legal-notice warning.
- Warning output is profile-aware and grouped clearly in CLI output.
- Legal-notice mode warns about missing or disabled rights metadata.
- Authenticated provenance mode warns about missing MAC.
- Tests cover all evidence profiles.
- README no longer says production legal-notice deployments must reject missing MAC.

## Risk Notes

Do not hide real metadata failures behind the new profile system. If metadata injection fails in legal-notice mode, that should be surfaced prominently.

Avoid making warnings too noisy. If a warning fires on every normal legal-notice invocation, it will train users to ignore all warnings.

Be precise about `Strict`: strict should be relative to the selected profile, not a global “every warning is fatal” mode unless explicitly documented.
