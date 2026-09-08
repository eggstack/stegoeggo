use super::*;
use serde::{Deserialize, Serialize};

/// Explicit rights policy expressing data-mining intent.
///
/// This is the caller-facing representation of data-mining restrictions.
/// It maps to [`DmiValue`] for serialization but is never inferred from
/// processing intensity, output format, or channel selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RightsPolicy {
    /// No DMI claim is emitted.
    #[default]
    Unspecified,
    /// Explicit permission for data mining.
    Allowed,
    /// Prohibited for AI/ML training.
    ProhibitedAiMlTraining,
    /// Prohibited for generative AI training.
    ProhibitedGenerativeAiTraining,
    /// Prohibited except for search engine indexing.
    ProhibitedExceptSearchIndexing,
    /// All data mining prohibited.
    ProhibitedAllDataMining,
    /// Prohibited, see constraints for details.
    ProhibitedSeeConstraints,
}

impl RightsPolicy {
    /// Returns the string representation of this policy.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            RightsPolicy::Unspecified => "Unspecified",
            RightsPolicy::Allowed => "Allowed",
            RightsPolicy::ProhibitedAiMlTraining => "ProhibitedAiMlTraining",
            RightsPolicy::ProhibitedGenerativeAiTraining => "ProhibitedGenerativeAiTraining",
            RightsPolicy::ProhibitedExceptSearchIndexing => "ProhibitedExceptSearchIndexing",
            RightsPolicy::ProhibitedAllDataMining => "ProhibitedAllDataMining",
            RightsPolicy::ProhibitedSeeConstraints => "ProhibitedSeeConstraints",
        }
    }

    /// Converts to the corresponding [`DmiValue`], if any.
    #[must_use]
    pub fn to_dmi_value(&self) -> Option<DmiValue> {
        match self {
            RightsPolicy::Unspecified => None,
            RightsPolicy::Allowed => Some(DmiValue::Allowed),
            RightsPolicy::ProhibitedAiMlTraining => Some(DmiValue::ProhibitedAiMlTraining),
            RightsPolicy::ProhibitedGenerativeAiTraining => {
                Some(DmiValue::ProhibitedGenAiMlTraining)
            }
            RightsPolicy::ProhibitedExceptSearchIndexing => {
                Some(DmiValue::ProhibitedExceptSearchEngineIndexing)
            }
            RightsPolicy::ProhibitedAllDataMining => Some(DmiValue::Prohibited),
            RightsPolicy::ProhibitedSeeConstraints => Some(DmiValue::ProhibitedSeeConstraints),
        }
    }

    /// Creates a `RightsPolicy` from the corresponding [`DmiValue`].
    #[must_use]
    pub fn from_dmi_value(dmi: DmiValue) -> Self {
        match dmi {
            DmiValue::Unspecified => RightsPolicy::Unspecified,
            DmiValue::Allowed => RightsPolicy::Allowed,
            DmiValue::ProhibitedAiMlTraining => RightsPolicy::ProhibitedAiMlTraining,
            DmiValue::ProhibitedGenAiMlTraining => RightsPolicy::ProhibitedGenerativeAiTraining,
            DmiValue::ProhibitedExceptSearchEngineIndexing => {
                RightsPolicy::ProhibitedExceptSearchIndexing
            }
            DmiValue::Prohibited => RightsPolicy::ProhibitedAllDataMining,
            DmiValue::ProhibitedSeeConstraints => RightsPolicy::ProhibitedSeeConstraints,
        }
    }

    /// Returns `true` if this policy requires constraint details.
    #[must_use]
    pub fn requires_constraints(&self) -> bool {
        matches!(self, RightsPolicy::ProhibitedSeeConstraints)
    }
}

/// Controls steganographic hidden-marker embedding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum HiddenMarkerMode {
    /// No LSB, DCT, Q-table, or tiled hidden marker work.
    Disabled,
    /// Minimal seed-only marker:
    /// - PNG / lossless WebP: fixed-position LSB seed in the first 64 RGB
    ///   channels (no full v3 payload)
    /// - JPEG: seed stored in quantization-table LSBs (no full DCT payload)
    ///
    /// This gives a canonical name to the existing legacy `Light` behavior
    /// so the canonical plan path can represent seed-only intent without a
    /// legacy side channel.
    SeedOnly,
    /// Existing non-tiled LSB/DCT behavior.
    BestEffort,
    /// Crop-resistant tiled mode with validated tile size.
    Tiled {
        /// Tile dimension in pixels. Must be in the range 32..=1024.
        tile_size: u32,
    },
}

/// Controls payload authentication mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AuthenticationMode {
    /// Non-cryptographic CRC32 checksum.
    None,
    /// HMAC-SHA256 cryptographic authentication.
    Hmac,
}

/// Explicit configuration of protection channels.
///
/// Each channel maps to concrete pipeline work. Invalid combinations
/// are rejected during resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionChannels {
    /// Whether to emit/merge canonical rights metadata.
    pub rights_metadata: bool,
    /// Hidden marker embedding mode.
    pub hidden_marker: HiddenMarkerMode,
    /// Authentication mode for steganographic payloads.
    pub authentication: AuthenticationMode,
}

impl ProtectionChannels {
    /// Creates a metadata-only configuration (no hidden marker, no authentication).
    #[must_use]
    pub fn metadata_only() -> Self {
        Self {
            rights_metadata: true,
            hidden_marker: HiddenMarkerMode::Disabled,
            authentication: AuthenticationMode::None,
        }
    }

    /// Creates a metadata + best-effort hidden marker configuration.
    #[must_use]
    pub fn with_hidden_marker() -> Self {
        Self {
            rights_metadata: true,
            hidden_marker: HiddenMarkerMode::BestEffort,
            authentication: AuthenticationMode::None,
        }
    }

    /// Creates a metadata + hidden marker + HMAC authentication configuration.
    #[must_use]
    pub fn authenticated() -> Self {
        Self {
            rights_metadata: true,
            hidden_marker: HiddenMarkerMode::BestEffort,
            authentication: AuthenticationMode::Hmac,
        }
    }

    /// Returns true if this configuration performs any steganographic work.
    #[must_use]
    pub fn has_stego(&self) -> bool {
        !matches!(self.hidden_marker, HiddenMarkerMode::Disabled)
    }
}

/// Image processing options for the protection pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOptions {
    /// Override output format. `None` means same as input.
    pub output_format: Option<ImageOutputFormat>,
    /// JPEG quality (1-100, default 90).
    pub jpeg_quality: u8,
    /// Whether to produce progressive JPEG output.
    pub progressive_jpeg: bool,
    /// Maximum image dimension in pixels.
    pub max_dimension: Option<u32>,
    /// Metadata update policy for re-processing.
    pub metadata_update_policy: MetadataUpdatePolicy,
    /// Caller-supplied steganographic redundancy override (1..=10).
    ///
    /// When `None`, the executor derives redundancy from intensity via
    /// [`ResolvedProtectionPlan::effective_redundancy`] (the same derivation
    /// the legacy `ProtectionContext::effective_redundancy` used).
    pub stego_redundancy: Option<usize>,
    /// Caller-supplied 4-byte truncated content hash for provenance linkage.
    ///
    /// Embedded in v3 payload generation so the same payload that survives
    /// lossy recompression also carries the provenance link. When `None`,
    /// the v3 payload zeroes the content-hash slot (matching legacy
    /// non-content-hash fixtures).
    pub content_hash: Option<[u8; 4]>,
    /// Caller-supplied override for the auto-computed `notice_applied_at`
    /// timestamp.
    ///
    /// `None` means "use wall-clock at execution time". Carried through
    /// [`ResolvedProtectionPlan`] so deterministic test fixtures and the
    /// legacy `ProtectionContext::with_timestamp_override` path produce
    /// identical notice timestamps.
    pub timestamp_override: Option<String>,
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            output_format: None,
            jpeg_quality: 90,
            progressive_jpeg: false,
            max_dimension: None,
            metadata_update_policy: MetadataUpdatePolicy::default(),
            stego_redundancy: None,
            content_hash: None,
            timestamp_override: None,
        }
    }
}

/// A validated protection request combining rights notice, policy, channels, and processing options.
///
/// This is the primary entry point for the new request-based API.
#[derive(Debug, Clone)]
pub struct ProtectionRequest {
    notice: RightsNotice,
    policy: RightsPolicy,
    channels: ProtectionChannels,
    processing: ProcessingOptions,
    seed: Option<u64>,
    intensity: f32,
    legal_metadata: Option<LegalMetadata>,
    mac_key: Option<Vec<u8>>,
    resource_limits: Option<crate::resource_limits::ResourceLimits>,
}

impl ProtectionRequest {
    /// Creates a new protection request.
    #[must_use]
    pub fn new(notice: RightsNotice, policy: RightsPolicy, channels: ProtectionChannels) -> Self {
        Self {
            notice,
            policy,
            channels,
            processing: ProcessingOptions::default(),
            seed: None,
            intensity: 0.5,
            legal_metadata: None,
            mac_key: None,
            resource_limits: None,
        }
    }

    /// Creates a metadata-only protection request (fastest path).
    #[must_use]
    pub fn metadata_only(notice: RightsNotice, policy: RightsPolicy) -> Self {
        Self::new(notice, policy, ProtectionChannels::metadata_only())
    }

    /// Creates a request with best-effort hidden marker.
    #[must_use]
    pub fn with_hidden_marker(notice: RightsNotice, policy: RightsPolicy) -> Self {
        Self::new(notice, policy, ProtectionChannels::with_hidden_marker())
    }

    /// Sets processing options.
    #[must_use]
    pub fn with_processing(mut self, processing: ProcessingOptions) -> Self {
        self.processing = processing;
        self
    }

    /// Sets the random seed for steganographic embedding.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Sets the embedding intensity (0.0-1.0).
    #[must_use]
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Sets legal metadata for the request.
    #[must_use]
    pub fn with_legal_metadata(mut self, metadata: LegalMetadata) -> Self {
        self.legal_metadata = Some(metadata);
        self
    }

    /// Sets the MAC key for HMAC authentication.
    #[must_use]
    pub fn with_mac_key(mut self, key: Vec<u8>) -> Self {
        self.mac_key = Some(key);
        self
    }

    /// Sets custom resource limits for parser safety.
    #[must_use]
    pub fn with_resource_limits(mut self, limits: crate::resource_limits::ResourceLimits) -> Self {
        self.resource_limits = Some(limits);
        self
    }

    /// Sets the output format.
    #[must_use]
    pub fn with_output_format(mut self, format: ImageOutputFormat) -> Self {
        self.processing.output_format = Some(format);
        self
    }

    /// Sets JPEG quality. Invalid values are rejected during request resolution.
    #[must_use]
    pub fn with_jpeg_quality(mut self, quality: u8) -> Self {
        self.processing.jpeg_quality = quality;
        self
    }

    /// Enables progressive JPEG output.
    #[must_use]
    pub fn with_progressive_jpeg(mut self) -> Self {
        self.processing.progressive_jpeg = true;
        self
    }

    /// Sets maximum image dimension.
    #[must_use]
    pub fn with_max_dimension(mut self, max: u32) -> Self {
        self.processing.max_dimension = Some(max);
        self
    }

    /// Sets the metadata update policy.
    #[must_use]
    pub fn with_metadata_update_policy(mut self, policy: MetadataUpdatePolicy) -> Self {
        self.processing.metadata_update_policy = policy;
        self
    }

    /// Sets the caller-supplied stego redundancy override. Invalid values are
    /// rejected during request resolution.
    ///
    /// When set, this value reaches the canonical executor unchanged instead
    /// of being derived from intensity. Range 1..=10.
    #[must_use]
    pub fn with_stego_redundancy(mut self, redundancy: usize) -> Self {
        self.processing.stego_redundancy = Some(redundancy);
        self
    }

    /// Sets the caller-supplied 4-byte content hash for provenance linkage.
    ///
    /// Reaches the v3 payload generator unchanged. When not set, the v3
    /// payload uses zero in the content-hash slot.
    #[must_use]
    pub fn with_content_hash(mut self, hash: [u8; 4]) -> Self {
        self.processing.content_hash = Some(hash);
        self
    }

    /// Sets the caller-supplied timestamp override for the notice-applied-at
    /// timestamp. Replaces the auto-computed wall-clock timestamp.
    ///
    /// Without an override and without an explicit
    /// `LegalMetadata::notice_applied_at`, `resolve_request` injects the
    /// current wall-clock time, so a second `resolve_request` over the same
    /// `ProtectionRequest` will produce a different `notice_applied_at`.
    /// Callers that need a deterministic `effective_notice` (for example
    /// conformance snapshots or CI golden-file comparisons) must set
    /// `timestamp_override` or supply an explicit
    /// `LegalMetadata::notice_applied_at`.
    #[must_use]
    pub fn with_timestamp_override(mut self, ts: impl Into<String>) -> Self {
        self.processing.timestamp_override = Some(ts.into());
        self
    }

    /// Returns the rights notice.
    #[must_use]
    pub fn notice(&self) -> &RightsNotice {
        &self.notice
    }

    /// Returns the rights policy.
    #[must_use]
    pub fn policy(&self) -> RightsPolicy {
        self.policy
    }

    /// Returns the protection channels.
    #[must_use]
    pub fn channels(&self) -> &ProtectionChannels {
        &self.channels
    }

    /// Returns the processing options.
    #[must_use]
    pub fn processing(&self) -> &ProcessingOptions {
        &self.processing
    }

    /// Returns the seed, if set.
    #[must_use]
    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    /// Returns the intensity.
    #[must_use]
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// Returns the legal metadata, if set.
    #[must_use]
    pub fn legal_metadata(&self) -> Option<&LegalMetadata> {
        self.legal_metadata.as_ref()
    }

    /// Returns the MAC key, if set.
    #[must_use]
    pub fn mac_key(&self) -> Option<&[u8]> {
        self.mac_key.as_deref()
    }

    /// Returns the resource limits, if set.
    #[must_use]
    pub fn resource_limits(&self) -> Option<&crate::resource_limits::ResourceLimits> {
        self.resource_limits.as_ref()
    }

    /// Returns the timestamp override, if set.
    #[must_use]
    pub fn timestamp_override(&self) -> Option<&str> {
        self.processing.timestamp_override.as_deref()
    }

    /// Creates a protection request from a preset, notice, and policy.
    ///
    /// The preset determines the channel configuration. Additional options
    /// can be chained with builder methods.
    #[must_use]
    pub fn from_preset(
        preset: ProtectionPreset,
        notice: RightsNotice,
        policy: RightsPolicy,
    ) -> Self {
        Self::new(notice, policy, preset.to_channels())
    }
}

/// An immutable, validated execution plan produced by resolving a [`ProtectionRequest`].
///
/// Pipeline stages consume this plan rather than repeatedly querying
/// mutable/optional context fields.
#[derive(Debug, Clone)]
pub struct ResolvedProtectionPlan {
    effective_policy: RightsPolicy,
    effective_dmi: Option<DmiValue>,
    effective_notice: RightsNotice,
    channels: ProtectionChannels,
    processing: ProcessingOptions,
    seed: u64,
    intensity: f32,
    input_format: ImageOutputFormat,
    output_format: ImageOutputFormat,
    legal_metadata: Option<LegalMetadata>,
    mac_key: Option<Vec<u8>>,
    warnings: Vec<ProtectionWarning>,
    resource_limits: crate::resource_limits::ResourceLimits,
}

impl ResolvedProtectionPlan {
    /// Returns the effective rights policy.
    #[must_use]
    pub fn effective_policy(&self) -> RightsPolicy {
        self.effective_policy
    }

    /// Returns the effective DMI value for serialization.
    #[must_use]
    pub fn effective_dmi(&self) -> Option<DmiValue> {
        self.effective_dmi
    }

    /// Returns the effective rights notice.
    #[must_use]
    pub fn effective_notice(&self) -> &RightsNotice {
        &self.effective_notice
    }

    /// Returns the resolved channels.
    #[must_use]
    pub fn channels(&self) -> &ProtectionChannels {
        &self.channels
    }

    /// Returns the processing options.
    #[must_use]
    pub fn processing(&self) -> &ProcessingOptions {
        &self.processing
    }

    /// Returns the resolved seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Returns the intensity.
    #[must_use]
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// Returns the input format.
    #[must_use]
    pub fn input_format(&self) -> ImageOutputFormat {
        self.input_format
    }

    /// Returns the output format.
    #[must_use]
    pub fn output_format(&self) -> ImageOutputFormat {
        self.output_format
    }

    /// Returns the legal metadata.
    #[must_use]
    pub fn legal_metadata(&self) -> Option<&LegalMetadata> {
        self.legal_metadata.as_ref()
    }

    /// Returns the MAC key.
    #[must_use]
    pub fn mac_key(&self) -> Option<&[u8]> {
        self.mac_key.as_deref()
    }

    /// Returns any warnings generated during resolution.
    #[must_use]
    pub fn warnings(&self) -> &[ProtectionWarning] {
        &self.warnings
    }

    /// Returns the resource limits for this plan.
    #[must_use]
    pub fn resource_limits(&self) -> &crate::resource_limits::ResourceLimits {
        &self.resource_limits
    }

    /// Returns true if any pixel-modifying work is required.
    #[must_use]
    pub fn modifies_pixels(&self) -> bool {
        self.channels.has_stego()
    }

    /// Returns true if this is a metadata-only plan.
    #[must_use]
    pub fn is_metadata_only(&self) -> bool {
        !self.channels.has_stego() && self.channels.rights_metadata
    }

    /// Returns the effective stego redundancy used by this plan.
    ///
    /// When the caller supplied an explicit `stego_redundancy` via
    /// [`ProtectionRequest::with_stego_redundancy`] / the
    /// [`ProcessingOptions::stego_redundancy`] field, that value is used
    /// clamped to `1..=10` as defense-in-depth (plans are validated at
    /// resolution time, so out-of-range values are rejected before this).
    /// Otherwise redundancy is derived from [`Self::intensity`] using the
    /// same thresholds as the legacy `ProtectionContext::effective_redundancy`:
    /// - `intensity < 0.3` → 1
    /// - `intensity < 0.7` → 2
    /// - `intensity >= 0.7` → 3
    #[must_use]
    pub fn effective_redundancy(&self) -> usize {
        if let Some(r) = self.processing.stego_redundancy {
            return r.clamp(1, 10);
        }
        let i = self.intensity;
        if i < 0.3 {
            1
        } else if i < 0.7 {
            2
        } else {
            3
        }
    }

    /// Returns the resolved 4-byte content hash, if any.
    #[must_use]
    pub fn content_hash(&self) -> Option<[u8; 4]> {
        self.processing.content_hash
    }

    /// Returns the resolved timestamp override, if any.
    #[must_use]
    pub fn timestamp_override(&self) -> Option<&str> {
        self.processing.timestamp_override.as_deref()
    }

    /// Build a [`PayloadEmissionContext`] for the given embed path.
    ///
    /// The caller must provide the actual [`crate::stego::EmbedPath`] after output format
    /// and embed path selection. The `progressive_output` field is set from
    /// the plan's progressive JPEG setting — the caller must override it to
    /// `false` if the actual embed path fell back to Q-table seed only.
    #[must_use]
    #[allow(dead_code)]
    pub(crate) fn payload_emission_context(
        &self,
        embed_path: crate::stego::EmbedPath,
    ) -> PayloadEmissionContext {
        PayloadEmissionContext::from_plan(self, embed_path)
    }

    /// Construct a resolved plan from validated parts.
    ///
    /// This is crate-internal — external code should use [`resolve_request`](crate::resolve_request).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        effective_policy: RightsPolicy,
        effective_dmi: Option<DmiValue>,
        effective_notice: RightsNotice,
        channels: ProtectionChannels,
        processing: ProcessingOptions,
        seed: u64,
        intensity: f32,
        input_format: ImageOutputFormat,
        output_format: ImageOutputFormat,
        legal_metadata: Option<LegalMetadata>,
        mac_key: Option<Vec<u8>>,
        warnings: Vec<ProtectionWarning>,
        resource_limits: crate::resource_limits::ResourceLimits,
    ) -> Self {
        Self {
            effective_policy,
            effective_dmi,
            effective_notice,
            channels,
            processing,
            seed,
            intensity,
            input_format,
            output_format,
            legal_metadata,
            mac_key,
            warnings,
            resource_limits,
        }
    }
}

/// Executable presets that expand into concrete channel configurations.
///
/// Each preset deterministically maps to [`ProtectionChannels`] plus
/// validation expectations. This replaces the non-executable
/// [`EvidenceProfile`] for new request-based API usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ProtectionPreset {
    /// Standards-aligned metadata notice. No hidden marker, no MAC.
    LegalNotice,
    /// Metadata notice plus best-effort hidden marker. No MAC required.
    LegalNoticeWithStego,
    /// Metadata + hidden marker + HMAC authentication. MAC key required.
    AuthenticatedProvenance,
    /// All available channels. MAC key required: a request built from this
    /// preset without a key fails resolution with `Error::Config`.
    /// (Unlike the legacy `EvidenceProfile::Maximal`, which degrades to a
    /// `MissingMacKey` warning when no key is present.)
    Maximal,
}

impl ProtectionPreset {
    /// Expands this preset into concrete [`ProtectionChannels`].
    #[must_use]
    pub fn to_channels(&self) -> ProtectionChannels {
        match self {
            ProtectionPreset::LegalNotice => ProtectionChannels::metadata_only(),
            ProtectionPreset::LegalNoticeWithStego => ProtectionChannels::with_hidden_marker(),
            ProtectionPreset::AuthenticatedProvenance => ProtectionChannels::authenticated(),
            ProtectionPreset::Maximal => ProtectionChannels {
                rights_metadata: true,
                hidden_marker: HiddenMarkerMode::BestEffort,
                authentication: AuthenticationMode::Hmac,
            },
        }
    }

    /// Returns the lowercase string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtectionPreset::LegalNotice => "legal-notice",
            ProtectionPreset::LegalNoticeWithStego => "legal-notice-stego",
            ProtectionPreset::AuthenticatedProvenance => "authenticated-provenance",
            ProtectionPreset::Maximal => "maximal",
        }
    }

    /// Returns `true` if this preset requires a MAC key.
    #[must_use]
    pub fn requires_mac_key(&self) -> bool {
        matches!(
            self,
            ProtectionPreset::AuthenticatedProvenance | ProtectionPreset::Maximal
        )
    }
}

/// Describes which channels were requested, executed, and degraded
/// during processing. Returned alongside the processed bytes.
#[derive(Debug, Clone, Default)]
pub struct ExecutionReport {
    /// The effective rights policy after resolution.
    pub effective_policy: RightsPolicy,
    /// The DMI value serialized into metadata, if any.
    pub effective_dmi: Option<DmiValue>,
    /// Whether metadata was injected.
    pub metadata_injected: bool,
    /// Whether steganographic embedding was attempted.
    pub stego_attempted: bool,
    /// Whether steganographic embedding succeeded.
    pub stego_succeeded: bool,
    /// Whether the output format differs from input.
    pub format_transcoded: bool,
    /// Warnings generated during execution.
    pub warnings: Vec<ProtectionWarning>,
    /// Observed resource usage during processing, if tracked.
    pub resource_usage: Option<crate::resource_limits::ResourceUsage>,
    /// Structured embed outcome summary, if steganographic embedding was attempted.
    pub embed_summary: Option<crate::stego::EmbedOutcomeSummary>,
}

impl ExecutionReport {
    /// The effective rights policy after resolution.
    #[must_use]
    pub fn effective_policy(&self) -> RightsPolicy {
        self.effective_policy
    }

    /// The DMI value serialized into metadata, if any.
    #[must_use]
    pub fn effective_dmi(&self) -> Option<DmiValue> {
        self.effective_dmi
    }

    /// Whether metadata was injected.
    #[must_use]
    pub fn metadata_injected(&self) -> bool {
        self.metadata_injected
    }

    /// Whether steganographic embedding was attempted.
    #[must_use]
    pub fn stego_attempted(&self) -> bool {
        self.stego_attempted
    }

    /// Whether steganographic embedding succeeded.
    #[must_use]
    pub fn stego_succeeded(&self) -> bool {
        self.stego_succeeded
    }

    /// Whether the output format differs from input.
    #[must_use]
    pub fn format_transcoded(&self) -> bool {
        self.format_transcoded
    }

    /// Warnings generated during execution.
    #[must_use]
    pub fn warnings(&self) -> &[ProtectionWarning] {
        &self.warnings
    }

    /// Observed resource usage during processing, if tracked.
    #[must_use]
    pub fn resource_usage(&self) -> Option<&crate::resource_limits::ResourceUsage> {
        self.resource_usage.as_ref()
    }

    /// Structured embed outcome summary, if steganographic embedding was attempted.
    #[must_use]
    pub fn embed_summary(&self) -> Option<&crate::stego::EmbedOutcomeSummary> {
        self.embed_summary.as_ref()
    }

    /// Returns true if any channel executed successfully.
    #[must_use]
    pub fn any_succeeded(&self) -> bool {
        self.metadata_injected || self.stego_succeeded
    }

    /// Returns true if any requested channel was degraded or skipped.
    #[allow(deprecated)]
    #[must_use]
    pub fn has_degradation(&self) -> bool {
        self.warnings.iter().any(|w| {
            matches!(
                w.severity_for_profile(EvidenceProfile::LegalNotice),
                WarningSeverity::Warning | WarningSeverity::Error
            )
        })
    }
}

impl From<DmiValue> for RightsPolicy {
    fn from(dmi: DmiValue) -> Self {
        RightsPolicy::from_dmi_value(dmi)
    }
}

impl From<RightsPolicy> for DmiValue {
    fn from(policy: RightsPolicy) -> Self {
        match policy {
            RightsPolicy::Unspecified => DmiValue::Unspecified,
            RightsPolicy::Allowed => DmiValue::Allowed,
            RightsPolicy::ProhibitedAiMlTraining => DmiValue::ProhibitedAiMlTraining,
            RightsPolicy::ProhibitedGenerativeAiTraining => DmiValue::ProhibitedGenAiMlTraining,
            RightsPolicy::ProhibitedExceptSearchIndexing => {
                DmiValue::ProhibitedExceptSearchEngineIndexing
            }
            RightsPolicy::ProhibitedAllDataMining => DmiValue::Prohibited,
            RightsPolicy::ProhibitedSeeConstraints => DmiValue::ProhibitedSeeConstraints,
        }
    }
}
