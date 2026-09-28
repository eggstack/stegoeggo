use crate::error::{config_error, Result};
use napi_derive::napi;
use stegoeggo::{
    AuthenticationMode as RustAuthentication, DmiValue as RustDmi,
    EvidenceChannel as RustEvidenceChannel, HiddenMarkerMode as RustHiddenMarker,
    ImageOutputFormat as RustFormat, MetadataUpdatePolicy as RustMetadataUpdatePolicy,
    ProtectionPreset as RustPreset, ProtectionWarning as RustWarning,
    RightsPolicy as RustRightsPolicy, VerificationStatus as RustStatus,
};

/// The rights policy written into the output metadata.
///
/// This is the canonical `RightsPolicy` domain model; it is never inferred
/// from intensity, format, or channel selection.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightsPolicy {
    /// No restriction expressed.
    #[napi(value = "UNSPECIFIED")]
    Unspecified,
    /// Use is allowed.
    #[napi(value = "ALLOWED")]
    Allowed,
    /// Prohibited for AI/ML training.
    #[napi(value = "PROHIBITED_AI_ML_TRAINING")]
    ProhibitedAiMlTraining,
    /// Prohibited for generative AI training.
    #[napi(value = "PROHIBITED_GENERATIVE_AI_TRAINING")]
    ProhibitedGenerativeAiTraining,
    /// Prohibited except for search indexing.
    #[napi(value = "PROHIBITED_EXCEPT_SEARCH_INDEXING")]
    ProhibitedExceptSearchIndexing,
    /// All data mining prohibited.
    #[napi(value = "PROHIBITED_ALL_DATA_MINING")]
    ProhibitedAllDataMining,
    /// Prohibited; see the supplied constraints.
    #[napi(value = "PROHIBITED_SEE_CONSTRAINTS")]
    ProhibitedSeeConstraints,
}

impl From<RustRightsPolicy> for RightsPolicy {
    fn from(value: RustRightsPolicy) -> Self {
        match value {
            RustRightsPolicy::Unspecified => Self::Unspecified,
            RustRightsPolicy::Allowed => Self::Allowed,
            RustRightsPolicy::ProhibitedAiMlTraining => Self::ProhibitedAiMlTraining,
            RustRightsPolicy::ProhibitedGenerativeAiTraining => {
                Self::ProhibitedGenerativeAiTraining
            }
            RustRightsPolicy::ProhibitedExceptSearchIndexing => {
                Self::ProhibitedExceptSearchIndexing
            }
            RustRightsPolicy::ProhibitedAllDataMining => Self::ProhibitedAllDataMining,
            RustRightsPolicy::ProhibitedSeeConstraints => Self::ProhibitedSeeConstraints,
            #[allow(unreachable_patterns)]
            _ => Self::Unspecified,
        }
    }
}

impl From<RightsPolicy> for RustRightsPolicy {
    fn from(value: RightsPolicy) -> Self {
        match value {
            RightsPolicy::Unspecified => Self::Unspecified,
            RightsPolicy::Allowed => Self::Allowed,
            RightsPolicy::ProhibitedAiMlTraining => Self::ProhibitedAiMlTraining,
            RightsPolicy::ProhibitedGenerativeAiTraining => Self::ProhibitedGenerativeAiTraining,
            RightsPolicy::ProhibitedExceptSearchIndexing => Self::ProhibitedExceptSearchIndexing,
            RightsPolicy::ProhibitedAllDataMining => Self::ProhibitedAllDataMining,
            RightsPolicy::ProhibitedSeeConstraints => Self::ProhibitedSeeConstraints,
        }
    }
}

/// The DMI (Data Mining Implementation) value serialized into metadata.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmiValue {
    /// No DMI restriction expressed.
    #[napi(value = "UNSPECIFIED")]
    Unspecified,
    /// DMI is allowed.
    #[napi(value = "ALLOWED")]
    Allowed,
    /// Prohibited for AI/ML training.
    #[napi(value = "PROHIBITED_AI_ML_TRAINING")]
    ProhibitedAiMlTraining,
    /// Prohibited for generative AI/ML training.
    #[napi(value = "PROHIBITED_GEN_AI_ML_TRAINING")]
    ProhibitedGenAiMlTraining,
    /// Prohibited except for search engine indexing.
    #[napi(value = "PROHIBITED_EXCEPT_SEARCH_ENGINE_INDEXING")]
    ProhibitedExceptSearchEngineIndexing,
    /// Prohibited for every use.
    #[napi(value = "PROHIBITED")]
    Prohibited,
    /// Prohibited; see the supplied constraints.
    #[napi(value = "PROHIBITED_SEE_CONSTRAINTS")]
    ProhibitedSeeConstraints,
}

impl From<RustDmi> for DmiValue {
    fn from(value: RustDmi) -> Self {
        match value {
            RustDmi::Unspecified => Self::Unspecified,
            RustDmi::Allowed => Self::Allowed,
            RustDmi::ProhibitedAiMlTraining => Self::ProhibitedAiMlTraining,
            RustDmi::ProhibitedGenAiMlTraining => Self::ProhibitedGenAiMlTraining,
            RustDmi::ProhibitedExceptSearchEngineIndexing => {
                Self::ProhibitedExceptSearchEngineIndexing
            }
            RustDmi::Prohibited => Self::Prohibited,
            RustDmi::ProhibitedSeeConstraints => Self::ProhibitedSeeConstraints,
            #[allow(unreachable_patterns)]
            _ => Self::Unspecified,
        }
    }
}

impl From<DmiValue> for RustDmi {
    fn from(value: DmiValue) -> Self {
        match value {
            DmiValue::Unspecified => Self::Unspecified,
            DmiValue::Allowed => Self::Allowed,
            DmiValue::ProhibitedAiMlTraining => Self::ProhibitedAiMlTraining,
            DmiValue::ProhibitedGenAiMlTraining => Self::ProhibitedGenAiMlTraining,
            DmiValue::ProhibitedExceptSearchEngineIndexing => {
                Self::ProhibitedExceptSearchEngineIndexing
            }
            DmiValue::Prohibited => Self::Prohibited,
            DmiValue::ProhibitedSeeConstraints => Self::ProhibitedSeeConstraints,
        }
    }
}

/// The encoded output container format.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageOutputFormat {
    /// Portable Network Graphics.
    #[napi(value = "PNG")]
    Png,
    /// Joint Photographic Experts Group.
    #[napi(value = "JPEG")]
    Jpeg,
    /// WebP image format.
    #[napi(value = "WEBP")]
    WebP,
}

impl ImageOutputFormat {
    /// The canonical lowercase file extension for this format. Mirrors
    /// `stegoeggo::ImageOutputFormat::extension()`, which returns `jpg` for
    /// JPEG.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
        }
    }
}

impl From<RustFormat> for ImageOutputFormat {
    fn from(value: RustFormat) -> Self {
        match value {
            RustFormat::Png => Self::Png,
            RustFormat::Jpeg => Self::Jpeg,
            RustFormat::WebP => Self::WebP,
            #[allow(unreachable_patterns)]
            _ => Self::Png,
        }
    }
}

impl From<ImageOutputFormat> for RustFormat {
    fn from(value: ImageOutputFormat) -> Self {
        match value {
            ImageOutputFormat::Png => Self::Png,
            ImageOutputFormat::Jpeg => Self::Jpeg,
            ImageOutputFormat::WebP => Self::WebP,
        }
    }
}

/// How existing StegoEggo-owned metadata is treated on write.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataUpdatePolicy {
    /// Replace StegoEggo-owned properties, preserve unrelated metadata.
    #[napi(value = "REPLACE_STEGO_OWNED")]
    ReplaceStegoOwned,
    /// Fail when conflicting StegoEggo metadata already exists.
    #[napi(value = "FAIL_ON_CONFLICT")]
    FailOnConflict,
    /// Preserve existing StegoEggo metadata and only add new fields.
    #[napi(value = "PRESERVE_EXISTING")]
    PreserveExisting,
}

impl From<RustMetadataUpdatePolicy> for MetadataUpdatePolicy {
    fn from(value: RustMetadataUpdatePolicy) -> Self {
        match value {
            RustMetadataUpdatePolicy::ReplaceStegoOwned => Self::ReplaceStegoOwned,
            RustMetadataUpdatePolicy::FailOnConflict => Self::FailOnConflict,
            RustMetadataUpdatePolicy::PreserveExisting => Self::PreserveExisting,
            #[allow(unreachable_patterns)]
            _ => Self::ReplaceStegoOwned,
        }
    }
}

impl From<MetadataUpdatePolicy> for RustMetadataUpdatePolicy {
    fn from(value: MetadataUpdatePolicy) -> Self {
        match value {
            MetadataUpdatePolicy::ReplaceStegoOwned => Self::ReplaceStegoOwned,
            MetadataUpdatePolicy::FailOnConflict => Self::FailOnConflict,
            MetadataUpdatePolicy::PreserveExisting => Self::PreserveExisting,
        }
    }
}

/// A named bundle of protection channels.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionPreset {
    /// Metadata notice only. No hidden marker, no MAC.
    #[napi(value = "LEGAL_NOTICE")]
    LegalNotice,
    /// Metadata notice plus best-effort hidden marker.
    #[napi(value = "LEGAL_NOTICE_WITH_STEGO")]
    LegalNoticeWithStego,
    /// Metadata plus hidden marker plus HMAC authentication.
    #[napi(value = "AUTHENTICATED_PROVENANCE")]
    AuthenticatedProvenance,
    /// Every available channel. Requires a MAC key.
    #[napi(value = "MAXIMAL")]
    Maximal,
}

impl From<RustPreset> for ProtectionPreset {
    fn from(value: RustPreset) -> Self {
        match value {
            RustPreset::LegalNotice => Self::LegalNotice,
            RustPreset::LegalNoticeWithStego => Self::LegalNoticeWithStego,
            RustPreset::AuthenticatedProvenance => Self::AuthenticatedProvenance,
            RustPreset::Maximal => Self::Maximal,
            #[allow(unreachable_patterns)]
            _ => Self::LegalNotice,
        }
    }
}

impl From<ProtectionPreset> for RustPreset {
    fn from(value: ProtectionPreset) -> Self {
        match value {
            ProtectionPreset::LegalNotice => Self::LegalNotice,
            ProtectionPreset::LegalNoticeWithStego => Self::LegalNoticeWithStego,
            ProtectionPreset::AuthenticatedProvenance => Self::AuthenticatedProvenance,
            ProtectionPreset::Maximal => Self::Maximal,
        }
    }
}

/// Payload authentication mode for the hidden marker.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationMode {
    /// CRC32 integrity only; forgeable.
    #[napi(value = "NONE")]
    None,
    /// HMAC-SHA256 authentication.
    #[napi(value = "HMAC")]
    Hmac,
}

impl From<RustAuthentication> for AuthenticationMode {
    fn from(value: RustAuthentication) -> Self {
        match value {
            RustAuthentication::None => Self::None,
            RustAuthentication::Hmac => Self::Hmac,
            #[allow(unreachable_patterns)]
            _ => Self::None,
        }
    }
}

impl From<AuthenticationMode> for RustAuthentication {
    fn from(value: AuthenticationMode) -> Self {
        match value {
            AuthenticationMode::None => Self::None,
            AuthenticationMode::Hmac => Self::Hmac,
        }
    }
}

/// The hidden-marker strategy discriminator.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenMarkerKind {
    /// No hidden marker work at all.
    #[napi(value = "disabled")]
    Disabled,
    /// Minimal seed-only marker (PNG/WebP LSB seed, JPEG Q-table seed).
    #[napi(value = "seedOnly")]
    SeedOnly,
    /// Non-tiled LSB/DCT hidden marker.
    #[napi(value = "bestEffort")]
    BestEffort,
    /// Crop-resistant tiled hidden marker.
    #[napi(value = "tiled")]
    Tiled,
}

/// Hidden-marker configuration.
///
/// Use the static factories rather than a bare object literal: `tileSize` is
/// only meaningful for the `tiled` kind and is validated against the canonical
/// `32..=1024` range.
#[napi]
#[derive(Clone, Debug)]
pub struct HiddenMarkerMode {
    /// The canonical Rust value owned by this mode.
    pub(crate) inner: RustHiddenMarker,
}

#[napi]
impl HiddenMarkerMode {
    /// No hidden-marker work at all.
    #[napi(factory)]
    pub fn disabled() -> Self {
        Self {
            inner: RustHiddenMarker::Disabled,
        }
    }

    /// Minimal seed-only marker: PNG/WebP LSB seed, JPEG Q-table seed.
    #[napi(factory)]
    pub fn seed_only() -> Self {
        Self {
            inner: RustHiddenMarker::SeedOnly,
        }
    }

    /// Non-tiled LSB/DCT hidden marker.
    #[napi(factory)]
    pub fn best_effort() -> Self {
        Self {
            inner: RustHiddenMarker::BestEffort,
        }
    }

    /// Crop-resistant tiled hidden marker. `tileSize` must be within the
    /// canonical `32..=1024` range.
    #[napi(factory)]
    pub fn tiled(tile_size: f64) -> Result<Self> {
        let tile_size = crate::numeric::to_u32("hiddenMarkerMode.tileSize", tile_size)?;
        if !(32..=1024).contains(&tile_size) {
            return Err(config_error(format!(
                "hiddenMarkerMode.tileSize {tile_size} is out of range 32..=1024"
            )));
        }
        Ok(Self {
            inner: RustHiddenMarker::Tiled { tile_size },
        })
    }

    /// The hidden-marker strategy.
    #[napi(getter)]
    pub fn kind(&self) -> HiddenMarkerKind {
        match self.inner {
            RustHiddenMarker::Disabled => HiddenMarkerKind::Disabled,
            RustHiddenMarker::SeedOnly => HiddenMarkerKind::SeedOnly,
            RustHiddenMarker::BestEffort => HiddenMarkerKind::BestEffort,
            RustHiddenMarker::Tiled { .. } => HiddenMarkerKind::Tiled,
            #[allow(unreachable_patterns)]
            _ => HiddenMarkerKind::BestEffort,
        }
    }

    /// The tile dimension in pixels, or `null` for non-tiled modes.
    #[napi(getter, ts_return_type = "number | null")]
    pub fn tile_size(&self) -> Option<u32> {
        match self.inner {
            RustHiddenMarker::Tiled { tile_size } => Some(tile_size),
            _ => None,
        }
    }
}

impl From<RustHiddenMarker> for HiddenMarkerMode {
    fn from(value: RustHiddenMarker) -> Self {
        Self { inner: value }
    }
}

/// A degradation or advisory condition raised during protection.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionWarning {
    /// No MAC key was configured; integrity is CRC32-only and forgeable.
    #[napi(value = "MISSING_MAC_KEY")]
    MissingMacKey,
    /// Metadata injection was disabled.
    #[napi(value = "METADATA_INJECTION_DISABLED")]
    MetadataInjectionDisabled,
    /// Progressive JPEG fell back to Q-table seed only.
    #[napi(value = "PROGRESSIVE_JPEG_FALLBACK")]
    ProgressiveJpegFallback,
    /// JPEG output is fragile under downstream re-encoding.
    #[napi(value = "JPEG_REENCODE_FRAGILE")]
    JpegReencodeFragile,
    /// Image too small for LSB hidden-marker embedding.
    #[napi(value = "LSB_CAPACITY_SKIPPED")]
    LsbCapacitySkipped,
    /// JPEG DCT coefficients insufficient for full embedding.
    #[napi(value = "DCT_CAPACITY_INSUFFICIENT")]
    DctCapacityInsufficient,
    /// Legal claims disabled while legal metadata is present.
    #[napi(value = "CONTRADICTORY_LEGAL_CLAIMS")]
    ContradictoryLegalClaims,
    /// `PROHIBITED_SEE_CONSTRAINTS` selected without constraints.
    #[napi(value = "MISSING_RIGHTS_CONSTRAINTS")]
    MissingRightsConstraints,
}

impl From<RustWarning> for ProtectionWarning {
    fn from(value: RustWarning) -> Self {
        match value {
            RustWarning::MissingMacKey => Self::MissingMacKey,
            RustWarning::MetadataInjectionDisabled => Self::MetadataInjectionDisabled,
            RustWarning::ProgressiveJpegFallback => Self::ProgressiveJpegFallback,
            RustWarning::JpegReencodeFragile => Self::JpegReencodeFragile,
            RustWarning::LsbCapacitySkipped => Self::LsbCapacitySkipped,
            RustWarning::DctCapacityInsufficient => Self::DctCapacityInsufficient,
            RustWarning::ContradictoryLegalClaims => Self::ContradictoryLegalClaims,
            RustWarning::MissingRightsConstraints => Self::MissingRightsConstraints,
            #[allow(unreachable_patterns)]
            _ => Self::MissingMacKey,
        }
    }
}

/// Coarse verification status for the hidden marker.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatus {
    /// Payload extracted and verified.
    #[napi(value = "VERIFIED")]
    Verified,
    /// Payload found but failed verification.
    #[napi(value = "INVALID")]
    Invalid,
    /// No payload found.
    #[napi(value = "NOT_FOUND")]
    NotFound,
}

impl From<RustStatus> for VerificationStatus {
    fn from(value: RustStatus) -> Self {
        match value {
            RustStatus::Verified => Self::Verified,
            RustStatus::Invalid => Self::Invalid,
            RustStatus::NotFound => Self::NotFound,
            #[allow(unreachable_patterns)]
            _ => Self::NotFound,
        }
    }
}

/// Combined strength of the evidence found in an image.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStrength {
    /// No rights-reservation metadata was found.
    #[napi(value = "NO_NOTICE_FOUND")]
    NoNoticeFound,
    /// Metadata notice only, no verified hidden marker.
    #[napi(value = "METADATA_NOTICE_ONLY")]
    MetadataNoticeOnly,
    /// Metadata notice plus a non-authenticated hidden marker.
    #[napi(value = "METADATA_NOTICE_AND_BEST_EFFORT_STEGO")]
    MetadataNoticeAndBestEffortStego,
    /// Metadata notice plus an authenticated hidden marker.
    #[napi(value = "METADATA_NOTICE_AND_AUTHENTICATED_PROVENANCE")]
    MetadataNoticeAndAuthenticatedProvenance,
}

impl From<stegoeggo::EvidenceStrength> for EvidenceStrength {
    fn from(value: stegoeggo::EvidenceStrength) -> Self {
        match value {
            stegoeggo::EvidenceStrength::NoNoticeFound => Self::NoNoticeFound,
            stegoeggo::EvidenceStrength::MetadataNoticeOnly => Self::MetadataNoticeOnly,
            stegoeggo::EvidenceStrength::MetadataNoticeAndBestEffortStego => {
                Self::MetadataNoticeAndBestEffortStego
            }
            stegoeggo::EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance => {
                Self::MetadataNoticeAndAuthenticatedProvenance
            }
            #[allow(unreachable_patterns)]
            _ => Self::NoNoticeFound,
        }
    }
}

/// Where a verification fact was read from.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSource {
    /// Extracted from XMP metadata.
    #[napi(value = "xmp")]
    Xmp,
    /// Extracted from legacy non-XMP metadata.
    #[napi(value = "legacy")]
    Legacy,
    /// Extracted from an embedded V1 payload.
    #[napi(value = "embeddedPayloadV1")]
    EmbeddedPayloadV1,
    /// Extracted from an embedded V2 payload.
    #[napi(value = "embeddedPayloadV2")]
    EmbeddedPayloadV2,
    /// Extracted from an embedded V3 payload.
    #[napi(value = "embeddedPayloadV3")]
    EmbeddedPayloadV3,
    /// Extracted from a detached manifest.
    #[napi(value = "detachedManifest")]
    DetachedManifest,
    /// Extracted from JPEG quantization table seed bits.
    #[napi(value = "qTableSeed")]
    QTableSeed,
}

impl From<stegoeggo::verification::FieldSource> for FieldSource {
    fn from(value: stegoeggo::verification::FieldSource) -> Self {
        match value {
            stegoeggo::verification::FieldSource::Xmp => Self::Xmp,
            stegoeggo::verification::FieldSource::Legacy => Self::Legacy,
            stegoeggo::verification::FieldSource::EmbeddedPayloadV1 => Self::EmbeddedPayloadV1,
            stegoeggo::verification::FieldSource::EmbeddedPayloadV2 => Self::EmbeddedPayloadV2,
            stegoeggo::verification::FieldSource::EmbeddedPayloadV3 => Self::EmbeddedPayloadV3,
            stegoeggo::verification::FieldSource::DetachedManifest => Self::DetachedManifest,
            #[allow(unreachable_patterns)]
            _ => Self::Xmp,
        }
    }
}

/// A metadata location or steganographic technique that carried evidence.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceChannel {
    /// PNG `tEXt`/`iTXt` text chunk.
    #[napi(value = "pngText")]
    PngText,
    /// PNG `iTXt` chunk carrying XMP.
    #[napi(value = "pngXmp")]
    PngXmp,
    /// JPEG `COM` comment marker.
    #[napi(value = "jpegComment")]
    JpegComment,
    /// JPEG `APP1` marker carrying XMP.
    #[napi(value = "jpegXmp")]
    JpegXmp,
    /// JPEG `APP13` marker carrying IPTC-IIM.
    #[napi(value = "jpegIptc")]
    JpegIptc,
    /// WebP RIFF chunk carrying XMP.
    #[napi(value = "webpXmp")]
    WebPXmp,
    /// WebP RIFF chunk carrying EXIF.
    #[napi(value = "webpExif")]
    WebpExif,
    /// LSB hidden marker in pixel data.
    #[napi(value = "lsbPayload")]
    LsbPayload,
    /// F5-style DCT hidden marker in JPEG coefficients.
    #[napi(value = "dctPayload")]
    DctPayload,
    /// Seed stored in JPEG quantization table LSBs.
    #[napi(value = "qTableSeed")]
    QTableSeed,
}

impl From<RustEvidenceChannel> for EvidenceChannel {
    fn from(value: RustEvidenceChannel) -> Self {
        match value {
            RustEvidenceChannel::PngText => Self::PngText,
            RustEvidenceChannel::PngXmp => Self::PngXmp,
            RustEvidenceChannel::JpegComment => Self::JpegComment,
            RustEvidenceChannel::JpegXmp => Self::JpegXmp,
            RustEvidenceChannel::JpegIptc => Self::JpegIptc,
            RustEvidenceChannel::WebPXmp => Self::WebPXmp,
            RustEvidenceChannel::WebPExif => Self::WebpExif,
            RustEvidenceChannel::LsbPayload => Self::LsbPayload,
            RustEvidenceChannel::DctPayload => Self::DctPayload,
            #[allow(unreachable_patterns)]
            _ => Self::QTableSeed,
        }
    }
}

/// Severity of a verification diagnostic.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    /// Informational.
    #[napi(value = "info")]
    Info,
    /// Warning.
    #[napi(value = "warning")]
    Warning,
    /// Error.
    #[napi(value = "error")]
    Error,
}

impl From<stegoeggo::verification::DiagnosticLevel> for DiagnosticLevel {
    fn from(value: stegoeggo::verification::DiagnosticLevel) -> Self {
        match value {
            stegoeggo::verification::DiagnosticLevel::Info => Self::Info,
            stegoeggo::verification::DiagnosticLevel::Warning => Self::Warning,
            stegoeggo::verification::DiagnosticLevel::Error => Self::Error,
            #[allow(unreachable_patterns)]
            _ => Self::Info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rights_policy_round_trips() {
        for policy in [
            RustRightsPolicy::Unspecified,
            RustRightsPolicy::Allowed,
            RustRightsPolicy::ProhibitedAiMlTraining,
            RustRightsPolicy::ProhibitedGenerativeAiTraining,
            RustRightsPolicy::ProhibitedExceptSearchIndexing,
            RustRightsPolicy::ProhibitedAllDataMining,
            RustRightsPolicy::ProhibitedSeeConstraints,
        ] {
            assert_eq!(RustRightsPolicy::from(RightsPolicy::from(policy)), policy);
        }
    }

    #[test]
    fn dmi_value_round_trips() {
        for dmi in [
            RustDmi::Unspecified,
            RustDmi::Allowed,
            RustDmi::ProhibitedAiMlTraining,
            RustDmi::ProhibitedGenAiMlTraining,
            RustDmi::ProhibitedExceptSearchEngineIndexing,
            RustDmi::Prohibited,
            RustDmi::ProhibitedSeeConstraints,
        ] {
            assert_eq!(RustDmi::from(DmiValue::from(dmi)), dmi);
        }
    }

    #[test]
    fn format_extension_matches_rust() {
        for format in [RustFormat::Png, RustFormat::Jpeg, RustFormat::WebP] {
            assert_eq!(
                ImageOutputFormat::from(format).extension(),
                format.extension()
            );
        }
    }

    #[test]
    fn hidden_marker_tile_size_is_validated() {
        assert!(HiddenMarkerMode::tiled(64.0).is_ok());
        assert!(HiddenMarkerMode::tiled(31.0).is_err());
        assert!(HiddenMarkerMode::tiled(1025.0).is_err());
        assert!(HiddenMarkerMode::tiled(64.5).is_err());
    }

    #[test]
    fn hidden_marker_kinds_project() {
        assert_eq!(
            HiddenMarkerMode::disabled().kind(),
            HiddenMarkerKind::Disabled
        );
        assert_eq!(
            HiddenMarkerMode::seed_only().kind(),
            HiddenMarkerKind::SeedOnly
        );
        assert_eq!(
            HiddenMarkerMode::best_effort().kind(),
            HiddenMarkerKind::BestEffort
        );
        assert_eq!(HiddenMarkerMode::disabled().tile_size(), None);
    }

    #[test]
    fn hidden_marker_round_trips() {
        for mode in [
            HiddenMarkerMode::disabled(),
            HiddenMarkerMode::seed_only(),
            HiddenMarkerMode::best_effort(),
            HiddenMarkerMode::tiled(128.0).unwrap(),
        ] {
            let rust = mode.inner;
            assert_eq!(HiddenMarkerMode::from(rust).inner, rust);
        }
    }

    #[test]
    fn warnings_project_from_rust() {
        assert_eq!(
            ProtectionWarning::from(RustWarning::MissingMacKey),
            ProtectionWarning::MissingMacKey
        );
        assert_eq!(
            ProtectionWarning::from(RustWarning::MissingRightsConstraints),
            ProtectionWarning::MissingRightsConstraints
        );
    }
}
