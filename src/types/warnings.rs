use super::*;

/// Warning about degraded protection during image processing.
///
/// Returned by [`process_image_bytes_with_info`](crate::process_image_bytes_with_info)
/// and [`process_image_bytes_with_warnings`](crate::process_image_bytes_with_warnings)
/// when protection was applied with reduced effectiveness or with an advisory
/// configuration.
/// For legal defense use cases, callers should check for warnings to understand
/// what level of protection was actually applied.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProtectionWarning {
    /// No MAC key was configured.
    ///
    /// The embedded payload can still detect accidental corruption via CRC32,
    /// but it is forgeable. Reverse proxies serving adversarial traffic should
    /// configure a MAC key and verify with the same key.
    MissingMacKey,
    /// Metadata injection was disabled.
    ///
    /// The steganographic payload may still be present, but visible legal/DMI
    /// markers will not be available to scrapers or downstream evidence tools.
    MetadataInjectionDisabled,
    /// Progressive JPEG detected — fell back to Q-table seed only.
    ///
    /// Full F5 DCT steganography was not applied because the JPEG uses
    /// progressive encoding, which the transcoder cannot decode. Only the
    /// seed was stored in quantization tables. This provides weaker protection
    /// than the standard DCT steganography path.
    ProgressiveJpegFallback,
    /// JPEG output was requested.
    ///
    /// The protection is efficient for byte-preserving JPEG serving through the
    /// stegoeggo fast path, but generic downstream JPEG re-encoding destroys
    /// COM/APP metadata, Q-table seed bits, and DCT payload evidence.
    JpegReencodeFragile,
    /// Image is too small for LSB steganographic embedding.
    ///
    /// The payload requires more pixels than the image provides. No LSB payload
    /// was embedded. Only metadata markers (and Q-table seeds for JPEG) were applied.
    /// Use a larger image or a smaller payload to enable steganographic protection.
    LsbCapacitySkipped,
    /// JPEG DCT coefficients insufficient for full F5 embedding.
    ///
    /// The image has too few DCT coefficients (e.g., a very small or heavily
    /// compressed JPEG) to embed the full payload. Only the seed was stored in
    /// quantization tables. This provides weaker protection than the standard
    /// DCT steganography path.
    DctCapacityInsufficient,
    /// Legal claims were explicitly disabled while legal metadata is present.
    ///
    /// The caller set `inject_legal_claims` to `false` but also provided
    /// non-empty [`LegalMetadata`]. This is contradictory: legal metadata
    /// should not be provided if injection is not desired. The legal metadata
    /// will be silently ignored.
    ContradictoryLegalClaims,
    /// `ProhibitedSeeConstraints` policy was selected without providing constraints.
    ///
    /// The DMI value is `ProhibitedSeeConstraints` but no `ai_constraints` or
    /// `web_statement_of_rights` was provided. The output will emit the
    /// `plus:DataMining` URI but no `plus:OtherConstraints` property. For strict
    /// evidence profiles this should be treated as an error.
    MissingRightsConstraints,
}

impl std::fmt::Display for ProtectionWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtectionWarning::MissingMacKey => write!(
                f,
                "No MAC key configured: payload integrity is CRC32-only and forgeable."
            ),
            ProtectionWarning::MetadataInjectionDisabled => write!(
                f,
                "Metadata injection disabled: visible DMI/legal evidence will not be emitted."
            ),
            ProtectionWarning::ProgressiveJpegFallback => write!(
                f,
                "Progressive JPEG detected: fell back to Q-table seed only. \
                 Full F5 DCT steganography was not applied."
            ),
            ProtectionWarning::JpegReencodeFragile => write!(
                f,
                "JPEG output is fragile under downstream re-encoding; serve byte-identical \
                 output or expect metadata/Q-table/DCT evidence loss."
            ),
            ProtectionWarning::LsbCapacitySkipped => write!(
                f,
                "Image too small for LSB steganographic embedding: no payload embedded. \
                 Only metadata markers were applied."
            ),
            ProtectionWarning::DctCapacityInsufficient => write!(
                f,
                "JPEG DCT coefficients insufficient for full F5 embedding: \
                 fell back to Q-table seed only. Weaker protection applied."
            ),
            ProtectionWarning::ContradictoryLegalClaims => write!(
                f,
                "Legal claims explicitly disabled but legal metadata is present: \
                 the legal metadata will be ignored. Remove the legal metadata or \
                 stop disabling legal claims."
            ),
            ProtectionWarning::MissingRightsConstraints => write!(
                f,
                "ProhibitedSeeConstraints policy selected without constraints: \
                 no ai_constraints or web_statement_of_rights was provided. \
                 The output will emit the prohibition URI but no companion constraint text."
            ),
        }
    }
}

/// Categorizes protection warnings by their relevance to evidence profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarningCategory {
    /// Warnings relevant to legal-notice evidence models.
    LegalNotice,
    /// Warnings about steganographic capacity limitations (best-effort).
    BestEffortStego,
    /// Warnings relevant to authenticated provenance models.
    AuthenticatedProvenance,
    /// Warnings about format-specific fragility or fallbacks.
    FormatFragility,
}

/// Severity level for a protection warning within a specific evidence profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarningSeverity {
    /// Informational — no action required; expected behavior for this profile.
    Info,
    /// Warning — protection is degraded; caller should be aware.
    Warning,
    /// Error — the evidence model cannot be satisfied with current configuration.
    Error,
}

impl std::fmt::Display for WarningSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WarningSeverity::Info => write!(f, "info"),
            WarningSeverity::Warning => write!(f, "warning"),
            WarningSeverity::Error => write!(f, "error"),
        }
    }
}

impl ProtectionWarning {
    /// Returns the category this warning belongs to.
    #[must_use]
    pub fn category(&self) -> WarningCategory {
        match self {
            ProtectionWarning::MissingMacKey => WarningCategory::AuthenticatedProvenance,
            ProtectionWarning::MetadataInjectionDisabled => WarningCategory::LegalNotice,
            ProtectionWarning::ProgressiveJpegFallback => WarningCategory::FormatFragility,
            ProtectionWarning::JpegReencodeFragile => WarningCategory::FormatFragility,
            ProtectionWarning::LsbCapacitySkipped => WarningCategory::BestEffortStego,
            ProtectionWarning::DctCapacityInsufficient => WarningCategory::BestEffortStego,
            ProtectionWarning::ContradictoryLegalClaims => WarningCategory::LegalNotice,
            ProtectionWarning::MissingRightsConstraints => WarningCategory::LegalNotice,
        }
    }

    /// Returns the severity of this warning for the given evidence profile.
    #[allow(deprecated)]
    #[must_use]
    pub fn severity_for_profile(&self, profile: EvidenceProfile) -> WarningSeverity {
        match self {
            ProtectionWarning::MissingMacKey => match profile {
                EvidenceProfile::AuthenticatedProvenance | EvidenceProfile::Maximal => {
                    WarningSeverity::Warning
                }
                _ => WarningSeverity::Info,
            },
            ProtectionWarning::MetadataInjectionDisabled => match profile {
                EvidenceProfile::LegalNotice | EvidenceProfile::LegalNoticeWithStego => {
                    WarningSeverity::Error
                }
                _ => WarningSeverity::Warning,
            },
            ProtectionWarning::ProgressiveJpegFallback | ProtectionWarning::JpegReencodeFragile => {
                WarningSeverity::Warning
            }
            ProtectionWarning::LsbCapacitySkipped | ProtectionWarning::DctCapacityInsufficient => {
                match profile {
                    EvidenceProfile::LegalNotice => WarningSeverity::Info,
                    _ => WarningSeverity::Warning,
                }
            }
            ProtectionWarning::ContradictoryLegalClaims => WarningSeverity::Warning,
            ProtectionWarning::MissingRightsConstraints => WarningSeverity::Error,
        }
    }
}
