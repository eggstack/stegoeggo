use super::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Heavy configuration that is shared across requests via `Arc`.
/// Create once, reuse across many image processing calls.
/// This avoids per-request heap allocation of large fields.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtectionConfig {
    /// MAC key for cryptographic payload verification.
    ///
    /// # Security
    ///
    /// Without a MAC key, steganographic payload verification uses a non-cryptographic
    /// CRC32 checksum that provides no cryptographic assurance. Always set a
    /// MAC key in adversarial settings to enable HMAC-SHA256 verification.
    mac_key: Option<Vec<u8>>,
    /// Legal metadata for copyright and AI training restrictions.
    legal_metadata: Option<LegalMetadata>,
}

impl ProtectionConfig {
    /// Creates a new `ProtectionConfig` with no MAC key or legal metadata.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the MAC key for cryptographic payload verification.
    #[must_use]
    pub fn with_mac_key(mut self, key: Vec<u8>) -> Self {
        self.mac_key = Some(key);
        self
    }

    /// Sets the legal metadata for content ownership claims.
    #[must_use]
    pub fn with_legal_metadata(mut self, metadata: LegalMetadata) -> Self {
        self.legal_metadata = Some(metadata);
        self
    }

    /// Returns the MAC key, if set.
    #[must_use]
    pub fn mac_key(&self) -> Option<&[u8]> {
        self.mac_key.as_deref()
    }

    /// Returns the legal metadata, if set.
    #[must_use]
    pub fn legal_metadata(&self) -> Option<&LegalMetadata> {
        self.legal_metadata.as_ref()
    }
}

/// Context for protection operations containing intensity and configuration.
///
/// Cheap to clone (heavy fields are in `Arc<ProtectionConfig>`).
///
/// Serialization loses the MAC key, legal metadata, resource limits, and
/// timestamp override (`#[serde(skip)]`; keys must not serialize). The
/// serialized form carries a `_config_dropped_warning` field when config was
/// present, which is captured on deserialization (see
/// [`ProtectionContext::config_dropped_in_serialization`]). After
/// deserialization re-attach via `with_mac_key()`,
/// `with_legal_metadata()`, `with_config()`, or `with_resource_limits()`;
/// check `mac_key().is_none()` when authentication matters, since legacy
/// `ProtectionContext` entry points without a key fall back to CRC32 with a
/// `MissingMacKey` warning, while a canonical [`ProtectionRequest`] with
/// [`AuthenticationMode::Hmac`] and no key fails resolution with
/// `Error::Config`.
#[allow(deprecated)]
#[derive(Debug, Clone, Deserialize)]
pub struct ProtectionContext {
    intensity: f32,
    seed: u64,
    input_format: Option<ImageOutputFormat>,
    output_format: Option<ImageOutputFormat>,
    protection_level: Option<ProtectionLevel>,
    evidence_profile: Option<EvidenceProfile>,
    dmi_value: Option<DmiValue>,
    max_dimension: Option<u32>,
    /// Three-state control for metadata injection (seed, DMI values).
    ///
    /// - `None` (default): use level-based defaults — metadata is injected for
    ///   all protection levels except `Disabled`.
    /// - `Some(true)`: force-enable metadata injection, overriding the level default.
    /// - `Some(false)`: force-disable metadata injection, overriding the level default.
    ///
    /// Omitting `with_metadata_injection()` (leaving this `None`) differs from
    /// calling `.with_metadata_injection(false)` for non-`Disabled` levels:
    /// the former injects metadata; the latter suppresses it.
    inject_metadata: Option<bool>,
    /// Three-state control for legal claim injection (copyright, artist).
    ///
    /// - `None` (default): automatically inject explicitly supplied legal
    ///   fields when [`LegalMetadata`] is present; otherwise inject none.
    /// - `Some(true)`: force-enable legal claim injection.
    /// - `Some(false)`: force-disable legal claim injection.
    ///
    /// Legal claims require `LegalMetadata` to be set via
    /// [`with_legal_metadata`](ProtectionContext::with_legal_metadata).
    /// WARNING: Only enable for content you own. May create legal liability otherwise.
    inject_legal_claims: Option<bool>,
    stego_redundancy: Option<usize>,
    jpeg_quality: u8,
    progressive_jpeg: bool,
    /// Tile size for crop-resistant stego embedding, in pixels.
    ///
    /// - `None` (default): tiling is disabled. Behavior matches the non-tiled
    ///   baseline, which survives common image transformations (resize,
    ///   recompression, format conversion) but is destroyed by cropping.
    /// - `Some(0)`: treated as disabled, same as `None`.
    /// - `Some(n)` with `n > 0`: each `n × n` pixel tile embeds a full copy of
    ///   the payload. The extractor scans candidate tile origins so the
    ///   payload is recoverable from any crop that contains at least one
    ///   intact tile. Valid range: 32..=1024. Smaller tiles fail ECC capacity
    ///   in non-MAC mode; larger tiles shrink the protected image's usable
    ///   embed region.
    ///
    /// Tiled mode multiplies total embed work by the tile count and is
    /// **opt-in** because the capacity and embedding-time costs are real.
    tile_size: Option<u32>,
    /// Maximum number of candidate tile origins the extractor will try before
    /// giving up. Bounds extraction time on very large images at the cost of
    /// potentially missing a successful tile when the crop is small or
    /// misaligned with the tile grid. Default 64.
    tile_extraction_max_origins: u32,
    /// Truncated content hash (4 bytes) for linking the protected image to its original.
    ///
    /// Derived from the ISCC content code or a truncated SHA-256 of the image pixels.
    /// Embedded in v2 payloads for provenance tracking. When not set, the hash is
    /// zeroed in the payload (v2 payloads without a content hash still carry the
    /// DMI value and flags fields).
    content_hash: Option<[u8; 4]>,
    /// Policy for updating metadata when re-processing an already-protected image.
    metadata_update_policy: Option<MetadataUpdatePolicy>,
    /// Override for auto-computed timestamps (notice_applied_at).
    ///
    /// When set, this value is used instead of `current_timestamp_iso8601()`.
    /// Intended for testing; not serialized.
    #[serde(skip)]
    timestamp_override: Option<String>,
    #[serde(skip)]
    config: Option<Arc<ProtectionConfig>>,
    #[serde(skip)]
    resource_limits: Option<crate::resource_limits::ResourceLimits>,
    #[serde(rename = "_config_dropped_warning", default)]
    config_dropped_warning: Option<String>,
}

impl Serialize for ProtectionContext {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut fields = 17;
        if self.config.is_some() || self.config_dropped_warning.is_some() {
            fields += 1;
        }
        let mut s = serializer.serialize_struct("ProtectionContext", fields)?;
        s.serialize_field("intensity", &self.intensity)?;
        s.serialize_field("seed", &self.seed)?;
        s.serialize_field("input_format", &self.input_format)?;
        s.serialize_field("output_format", &self.output_format)?;
        s.serialize_field("protection_level", &self.protection_level)?;
        s.serialize_field("evidence_profile", &self.evidence_profile)?;
        s.serialize_field("dmi_value", &self.dmi_value)?;
        s.serialize_field("max_dimension", &self.max_dimension)?;
        s.serialize_field("inject_metadata", &self.inject_metadata)?;
        s.serialize_field("inject_legal_claims", &self.inject_legal_claims)?;
        s.serialize_field("stego_redundancy", &self.stego_redundancy)?;
        s.serialize_field("jpeg_quality", &self.jpeg_quality)?;
        s.serialize_field("progressive_jpeg", &self.progressive_jpeg)?;
        s.serialize_field("tile_size", &self.tile_size)?;
        s.serialize_field(
            "tile_extraction_max_origins",
            &self.tile_extraction_max_origins,
        )?;
        s.serialize_field("content_hash", &self.content_hash)?;
        s.serialize_field("metadata_update_policy", &self.metadata_update_policy)?;
        if self.config.is_some() {
            s.serialize_field(
                "_config_dropped_warning",
                "ProtectionContext.config is not serialized; MAC key and legal metadata will be lost on roundtrip. Set them again after deserialization.",
            )?;
        } else if let Some(warning) = &self.config_dropped_warning {
            s.serialize_field("_config_dropped_warning", warning)?;
        }
        s.end()
    }
}

/// The default seed is generated via `getrandom` (OS CSPRNG).
/// For reproducible protection, use `ProtectionContext::new(intensity, seed)`.
impl Default for ProtectionContext {
    fn default() -> Self {
        let seed = crate::util::seed::generate_random_seed();
        Self {
            intensity: 0.5,
            seed,
            input_format: None,
            output_format: None,
            protection_level: None,
            evidence_profile: None,
            dmi_value: None,
            max_dimension: None,
            inject_metadata: None,
            inject_legal_claims: None,
            stego_redundancy: None,
            jpeg_quality: 90,
            progressive_jpeg: false,
            tile_size: None,
            tile_extraction_max_origins: 64,
            content_hash: None,
            metadata_update_policy: None,
            timestamp_override: None,
            config: None,
            resource_limits: None,
            config_dropped_warning: None,
        }
    }
}

impl ProtectionContext {
    pub(crate) fn validate(&self) -> crate::Result<()> {
        validate_intensity(self.intensity)?;
        if let Some(redundancy) = self.stego_redundancy {
            validate_stego_redundancy(redundancy)?;
        }
        validate_jpeg_quality(self.jpeg_quality)?;
        if let Some(tile_size) = self.tile_size {
            validate_tile_size(tile_size)?;
        }
        validate_tile_extraction_max_origins(self.tile_extraction_max_origins)
    }

    /// Create a new ProtectionContext with the specified intensity and seed.
    ///
    /// Intensity is clamped to the range [0.0, 1.0].
    ///
    /// **Production use requires a MAC key.** Without one, steganographic payloads use
    /// a non-cryptographic CRC32 checksum that can be trivially forged. Call `.with_mac_key()`
    /// for adversarial or production deployments.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use stegoeggo::{ProtectionContext, ProtectionLevel, process_image};
    /// use image::DynamicImage;
    ///
    /// let img = DynamicImage::new_rgb8(64, 64);
    /// let ctx = ProtectionContext::new(0.5, 42);
    /// let protected = process_image(img, ProtectionLevel::Standard, &ctx).unwrap();
    /// ```
    pub fn new(intensity: f32, seed: u64) -> Self {
        Self {
            intensity: intensity.clamp(0.0, 1.0),
            seed,
            input_format: None,
            output_format: None,
            protection_level: None,
            evidence_profile: None,
            dmi_value: None,
            max_dimension: None,
            inject_metadata: None,
            inject_legal_claims: None,
            stego_redundancy: None,
            jpeg_quality: 90,
            progressive_jpeg: false,
            tile_size: None,
            tile_extraction_max_origins: 64,
            content_hash: None,
            metadata_update_policy: None,
            timestamp_override: None,
            config: None,
            resource_limits: None,
            config_dropped_warning: None,
        }
    }

    /// Set the shared configuration (legal metadata, MAC key).
    #[must_use]
    pub fn with_config(mut self, config: Arc<ProtectionConfig>) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the MAC key for cryptographic payload verification.
    /// Creates a `ProtectionConfig` internally.
    #[must_use]
    pub fn with_mac_key(mut self, key: Vec<u8>) -> Self {
        let config = self
            .config
            .get_or_insert_with(|| Arc::new(ProtectionConfig::new()));
        let mut builder = (**config).clone();
        builder.mac_key = Some(key);
        self.config = Some(Arc::new(builder));
        self
    }

    /// Set the legal metadata for this context.
    /// This should only be used for content you own.
    #[must_use]
    pub fn with_legal_metadata(mut self, metadata: LegalMetadata) -> Self {
        let config = self
            .config
            .get_or_insert_with(|| Arc::new(ProtectionConfig::new()));
        let mut builder = (**config).clone();
        builder.legal_metadata = Some(metadata);
        self.config = Some(Arc::new(builder));
        self
    }

    /// Access the MAC key, if set.
    #[must_use]
    pub fn mac_key(&self) -> Option<&[u8]> {
        self.config.as_ref().and_then(|c| c.mac_key.as_deref())
    }

    /// Access the legal metadata, if set.
    #[must_use]
    pub fn legal_metadata(&self) -> Option<&LegalMetadata> {
        self.config.as_ref().and_then(|c| c.legal_metadata.as_ref())
    }

    /// Returns `true` if this context was deserialized from a form that had
    /// dropped its config (MAC key, legal metadata, resource limits,
    /// timestamp override) during serialization.
    ///
    /// Check this after deserialization when authentication matters: a `true`
    /// value means `mac_key()` and `legal_metadata()` are `None` because they
    /// were lost in the roundtrip, not because they were never set.
    /// Re-attach via `with_mac_key()`, `with_legal_metadata()`,
    /// `with_config()`, or `with_resource_limits()`.
    #[must_use]
    pub fn config_dropped_in_serialization(&self) -> bool {
        self.config_dropped_warning.is_some()
    }

    /// Set the maximum image dimension limit.
    #[must_use]
    pub fn with_max_dimension(mut self, max: u32) -> Self {
        self.max_dimension = Some(max);
        self
    }

    /// Set the output format for this context. When set, images will be encoded
    /// in this format. If not set, defaults to PNG or matches input format.
    #[must_use]
    pub fn with_format(mut self, format: ImageOutputFormat) -> Self {
        self.output_format = Some(format);
        self
    }

    /// Set the input format hint for this context.
    /// Usually auto-detected from magic bytes, so this is rarely needed.
    #[must_use]
    pub fn with_input_format(mut self, format: ImageOutputFormat) -> Self {
        self.input_format = Some(format);
        self
    }

    /// Set the DMI value for this context, returning a new context.
    #[deprecated(
        since = "0.4.0",
        note = "Use RightsPolicy in ProtectionRequest instead. See ProtectionRequest::new()."
    )]
    #[must_use]
    pub fn with_dmi(mut self, dmi: DmiValue) -> Self {
        self.dmi_value = Some(dmi);
        self
    }

    /// Set the evidence profile for this context.
    ///
    /// The evidence profile controls how protection warnings are interpreted
    /// and the default evidence posture. It does not directly change the
    /// processing pipeline — use [`ProtectionLevel`] for that.
    ///
    /// When not set, the profile defaults to [`EvidenceProfile::LegalNotice`]
    /// for warning interpretation purposes.
    #[allow(deprecated)]
    #[must_use]
    pub fn with_evidence_profile(mut self, profile: EvidenceProfile) -> Self {
        self.evidence_profile = Some(profile);
        self
    }

    /// Get the evidence profile.
    ///
    /// Returns the caller's explicit profile, if any. When `None`, the
    /// pipeline treats the context as [`EvidenceProfile::LegalNotice`] for
    /// warning interpretation.
    #[allow(deprecated)]
    #[must_use]
    pub fn evidence_profile(&self) -> EvidenceProfile {
        self.evidence_profile
            .unwrap_or(EvidenceProfile::LegalNotice)
    }

    /// Create a context pre-configured for legal notice (metadata only, no MAC required).
    #[allow(deprecated)]
    #[must_use]
    pub fn legal_notice() -> Self {
        Self::default().with_evidence_profile(EvidenceProfile::LegalNotice)
    }

    /// Create a context pre-configured for legal notice with steganographic markers.
    #[allow(deprecated)]
    #[must_use]
    pub fn legal_notice_with_stego() -> Self {
        Self::default().with_evidence_profile(EvidenceProfile::LegalNoticeWithStego)
    }

    /// Create a context pre-configured for authenticated provenance (MAC key expected).
    #[allow(deprecated)]
    #[must_use]
    pub fn authenticated_provenance() -> Self {
        Self::default().with_evidence_profile(EvidenceProfile::AuthenticatedProvenance)
    }

    /// Create a context pre-configured for maximal protection (all channels).
    #[allow(deprecated)]
    #[must_use]
    pub fn maximal() -> Self {
        Self::default().with_evidence_profile(EvidenceProfile::Maximal)
    }

    /// Override the level-based default for metadata injection.
    ///
    /// When `enable` is `true`, metadata (seed, DMI values) is injected
    /// regardless of protection level. When `enable` is `false`, metadata
    /// injection is suppressed even for levels that would normally inject it.
    ///
    /// If this method is **not** called, the default behavior depends on the
    /// protection level: metadata is injected for all levels except `Disabled`.
    /// This means `.with_metadata_injection(true)` on a `Standard` context is
    /// a no-op (metadata was already on), while `.with_metadata_injection(false)`
    /// suppresses it — a meaningful behavioral difference.
    ///
    /// When `enable` is `false` and legal metadata is present, the library
    /// path (`process_image_bytes_with_warnings` and `resolve_request`) emits a
    /// [`ContradictoryLegalClaims`](ProtectionWarning::ContradictoryLegalClaims)
    /// warning and the legal metadata is not emitted
    /// (`generate_rights_metadata_from_notice` no-ops when
    /// `should_inject_metadata == false`). The CLI rejects the same
    /// combination as a hard configuration error (exit code 2) for stricter
    /// policy enforcement. This library/CLI inconsistency is intentional for
    /// backward compatibility — callers that do not inspect warnings will
    /// silently get an image without the expected legal claims.
    #[deprecated(
        since = "0.4.0",
        note = "Use ProtectionChannels::metadata_only() or ProtectionRequest builder instead."
    )]
    #[must_use]
    pub fn with_metadata_injection(mut self, enable: bool) -> Self {
        self.inject_metadata = Some(enable);
        self
    }

    /// Override the default for legal claim injection.
    ///
    /// When `enable` is `true`, legal claims (copyright, artist) are injected
    /// into the image metadata. When `enable` is `false`, legal claim injection
    /// is disabled even if [`LegalMetadata`] is present.
    ///
    /// Legal claims require [`LegalMetadata`] to be set via
    /// [`with_legal_metadata`](ProtectionContext::with_legal_metadata).
    ///
    /// If this method is **not** called, legal claims are automatically
    /// enabled when [`LegalMetadata`] is present, and disabled otherwise.
    ///
    /// # Deprecated
    ///
    /// This method is deprecated. Legal claims are now automatically
    /// enabled when [`LegalMetadata`] is provided. Calling this method
    /// with `true` is redundant, and calling it with `false` while
    /// legal metadata is present produces a
    /// [`ContradictoryLegalClaims`](ProtectionWarning::ContradictoryLegalClaims)
    /// warning.
    ///
    /// # Warning
    ///
    /// Only enable for content you own. May create legal liability otherwise.
    #[must_use]
    #[deprecated(
        since = "0.2.2",
        note = "Legal claims are auto-enabled when LegalMetadata is present. \
                This method is redundant for the normal case and produces a \
                ContradictoryLegalClaims warning when used with `false` \
                while legal metadata is set."
    )]
    pub fn with_legal_claims(mut self, enable: bool) -> Self {
        self.inject_legal_claims = Some(enable);
        self
    }

    /// Set the intensity for this context, returning a new context.
    #[must_use]
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set the seed for this context, returning a new context.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set the stego embedding redundancy (1-10). Higher values are more robust
    /// for verification but slower. Invalid values are rejected when the context
    /// is used. When not set, redundancy is derived from `intensity` via the
    /// internal `effective_redundancy()` helper.
    #[must_use]
    pub fn with_stego_redundancy(mut self, redundancy: usize) -> Self {
        self.stego_redundancy = Some(redundancy);
        self
    }

    /// Set the JPEG encoding quality (1-100). Invalid values are rejected when
    /// the context is used. Default is 90.
    #[must_use]
    pub fn with_jpeg_quality(mut self, quality: u8) -> Self {
        self.jpeg_quality = quality;
        self
    }

    /// Enable progressive JPEG encoding. Progressive JPEGs render faster on
    /// slow connections as the image appears progressively. Default is false.
    #[must_use]
    pub fn with_progressive_jpeg(mut self, progressive: bool) -> Self {
        self.progressive_jpeg = progressive;
        self
    }

    /// Enable tiled stego embedding for crop resistance.
    ///
    /// Each `size × size` pixel tile embeds a full copy of the payload. The
    /// extractor scans candidate tile origins so the payload is recoverable
    /// from any crop that contains at least one intact tile.
    ///
    /// Pass `0` to disable tiling (same as never calling this method).
    /// Valid range for non-zero values: 32..=1024. Values outside that range
    /// are rejected when the context is used. The most common choice is 64 (matches the LSB tile
    /// capacity for the default ECC payload).
    ///
    /// Tiled embedding multiplies total embed work by the tile count, so
    /// consider the capacity and embedding-time costs. For adversarial
    /// settings where cropping is a known attack vector, opt in via
    /// `with_tile_size(64)`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use stegoeggo::{ProtectionContext, ProtectionLevel, process_image_bytes};
    ///
    /// let bytes: Vec<u8> = Vec::new();
    /// let ctx = ProtectionContext::new(0.7, 42).with_tile_size(64);
    /// let _protected = process_image_bytes(&bytes, ProtectionLevel::Standard, &ctx);
    /// ```
    #[must_use]
    pub fn with_tile_size(mut self, size: u32) -> Self {
        self.tile_size = Some(size);
        self
    }

    /// Set the maximum number of candidate tile origins the extractor will
    /// try (1..=4096). Invalid values are rejected when the context is used.
    /// Default is 64. Higher values increase extraction time but improve
    /// recovery from small or misaligned crops.
    #[must_use]
    pub fn with_tile_extraction_max_origins(mut self, n: u32) -> Self {
        self.tile_extraction_max_origins = n;
        self
    }

    /// Set a content hash for provenance tracking (v2 payloads).
    ///
    /// The 4-byte hash is embedded in v2 payload headers and can be used
    /// to link a protected image back to its original, even after metadata
    /// stripping. Typically derived from a truncated ISCC content code or
    /// SHA-256 of the image pixels.
    ///
    /// When not set, the hash is zeroed in the payload (v2 payloads without
    /// a content hash still carry the DMI value and flags fields).
    #[must_use]
    pub fn with_content_hash(mut self, hash: [u8; 4]) -> Self {
        self.content_hash = Some(hash);
        self
    }

    /// Set the metadata update policy for repeated image processing.
    ///
    /// Controls how the pipeline handles existing StegoEggo metadata when
    /// re-processing an already-protected image.
    #[must_use]
    pub fn with_metadata_update_policy(mut self, policy: MetadataUpdatePolicy) -> Self {
        self.metadata_update_policy = Some(policy);
        self
    }

    /// Get the metadata update policy.
    ///
    /// Returns the caller's explicit policy, if any. Defaults to
    /// [`MetadataUpdatePolicy::ReplaceStegoOwned`] when not set.
    #[must_use]
    pub fn metadata_update_policy(&self) -> MetadataUpdatePolicy {
        self.metadata_update_policy
            .unwrap_or(MetadataUpdatePolicy::ReplaceStegoOwned)
    }

    /// Override the auto-computed `notice_applied_at` timestamp.
    ///
    /// When set, this value replaces the wall-clock timestamp that would
    /// otherwise be auto-computed. Without an override and without an explicit
    /// `LegalMetadata::notice_applied_at`, `resolve_request` injects the
    /// current wall-clock time, so a second `resolve_request` over the same
    /// `ProtectionRequest` will produce a different `notice_applied_at`.
    /// Callers that need a deterministic `effective_notice` (for example
    /// conformance snapshots or CI golden-file comparisons) must set
    /// `timestamp_override` or supply an explicit
    /// `LegalMetadata::notice_applied_at`. Not serialized.
    #[must_use]
    pub fn with_timestamp_override(mut self, ts: impl Into<String>) -> Self {
        self.timestamp_override = Some(ts.into());
        self
    }

    /// Returns the timestamp override, if set.
    #[must_use]
    pub(crate) fn timestamp_override(&self) -> Option<&str> {
        self.timestamp_override.as_deref()
    }

    /// Set resource limits for parser hardening.
    ///
    /// Limits are applied to externally reachable parsers (PNG chunk walker,
    /// JPEG segment parser, WebP RIFF parser, XMP extraction, stego extraction)
    /// to prevent resource exhaustion from malformed or adversarial inputs.
    ///
    /// When not set, the library uses conservative defaults suitable for
    /// web-facing services. Explicit limits override all defaults.
    #[must_use]
    pub fn with_resource_limits(mut self, limits: crate::resource_limits::ResourceLimits) -> Self {
        self.resource_limits = Some(limits);
        self
    }

    /// Get the resource limits.
    ///
    /// Returns caller-specified limits, or conservative defaults.
    #[must_use]
    pub fn resource_limits(&self) -> crate::resource_limits::ResourceLimits {
        self.resource_limits.clone().unwrap_or_default()
    }

    /// Get the intensity value.
    #[must_use]
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// Get the seed value.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Get the input format hint.
    #[must_use]
    pub fn input_format(&self) -> Option<ImageOutputFormat> {
        self.input_format
    }

    /// Get the output format.
    #[must_use]
    pub fn output_format(&self) -> Option<ImageOutputFormat> {
        self.output_format
    }

    /// Get the protection level.
    #[must_use]
    pub fn protection_level(&self) -> Option<ProtectionLevel> {
        self.protection_level
    }

    /// Get the DMI value.
    #[must_use]
    pub fn dmi_value(&self) -> Option<DmiValue> {
        self.dmi_value
    }

    /// Get the maximum dimension limit.
    #[must_use]
    pub fn max_dimension(&self) -> Option<u32> {
        self.max_dimension
    }

    /// Get whether metadata injection is enabled.
    ///
    /// Returns the caller's explicit override, if any. `None` means the
    /// pipeline will apply the level-based default (inject unless `Disabled`).
    /// The pipeline resolves this by calling
    /// `inject_metadata.unwrap_or(!matches!(level, Disabled))`.
    #[must_use]
    pub fn inject_metadata(&self) -> Option<bool> {
        self.inject_metadata
    }

    /// Determine whether metadata will effectively be emitted for this context.
    ///
    /// Returns `true` when the caller has not explicitly disabled metadata
    /// injection and the protection level is not `Disabled`. This is the
    /// single source of truth for the payload `rights_metadata` channel flag.
    #[must_use]
    pub fn effective_metadata_injection(&self) -> bool {
        match self.inject_metadata {
            Some(false) => false,
            _ => !matches!(self.protection_level, Some(ProtectionLevel::Disabled)),
        }
    }

    /// Get whether legal claim injection is explicitly overridden.
    ///
    /// Returns the caller's explicit override, if any. `None` means the
    /// pipeline will auto-enable legal claims when [`LegalMetadata`] is
    /// present and disable them otherwise.
    #[must_use]
    pub fn inject_legal_claims(&self) -> Option<bool> {
        self.inject_legal_claims
    }

    /// Get the effective stego redundancy.
    ///
    /// When the user has explicitly set `stego_redundancy` via
    /// `with_stego_redundancy()`, that value is returned clamped to `1..=10`
    /// as defense-in-depth (out-of-range values are still rejected by
    /// `validate()` / `resolve_request()`). Otherwise,
    /// the redundancy is derived from the current `intensity`:
    /// - `intensity < 0.3` → 1 (minimal embedding)
    /// - `intensity < 0.7` → 2 (standard)
    /// - `intensity >= 0.7` → 3 (heavy)
    #[must_use]
    pub fn stego_redundancy(&self) -> usize {
        self.effective_redundancy()
    }

    /// Returns the explicitly-set stego redundancy field, if any.
    ///
    /// Distinct from [`Self::stego_redundancy`] which always returns the
    /// effective value. Returns `None` when the caller did not call
    /// `with_stego_redundancy`, allowing the canonical path to fall back
    /// to the intensity-derived redundancy.
    #[must_use]
    pub fn stego_redundancy_field(&self) -> Option<usize> {
        self.stego_redundancy
    }

    pub(crate) fn effective_redundancy(&self) -> usize {
        if let Some(r) = self.stego_redundancy {
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

    /// Get the JPEG encoding quality.
    #[must_use]
    pub fn jpeg_quality(&self) -> u8 {
        self.jpeg_quality
    }

    /// Get whether progressive JPEG encoding is enabled.
    #[must_use]
    pub fn progressive_jpeg(&self) -> bool {
        self.progressive_jpeg
    }

    /// Get the tile size for crop-resistant stego embedding.
    ///
    /// Returns the configured value if set, otherwise `None`. Note that
    /// `Some(0)` and `None` both indicate that tiling is disabled — callers
    /// that need a single on/off decision should use
    /// [`is_tile_mode_enabled`](Self::is_tile_mode_enabled) instead.
    #[must_use]
    pub fn tile_size(&self) -> Option<u32> {
        self.tile_size
    }

    /// Returns `true` when tiled embedding is active.
    ///
    /// Treats both `Some(0)` and `None` as "tiling disabled" so callers
    /// don't need to special-case the sentinel.
    #[must_use]
    pub fn is_tile_mode_enabled(&self) -> bool {
        matches!(self.tile_size, Some(n) if n > 0)
    }

    /// Get the maximum number of candidate tile origins the extractor will
    /// try.
    #[must_use]
    pub fn tile_extraction_max_origins(&self) -> u32 {
        self.tile_extraction_max_origins
    }

    /// Get the content hash, if set.
    #[must_use]
    pub fn content_hash(&self) -> Option<[u8; 4]> {
        self.content_hash
    }

    /// Set the input format hint (non-consuming).
    pub fn set_input_format(&mut self, format: ImageOutputFormat) {
        self.input_format = Some(format);
    }

    #[cfg(test)]
    pub(crate) fn set_protection_level(&mut self, level: ProtectionLevel) {
        self.protection_level = Some(level);
    }

    /// Normalize legal metadata and context into a format-independent [`RightsNotice`].
    ///
    /// This is called once per processing invocation. All format writers
    /// (PNG tEXt, JPEG COM, WebP XMP) consume the same `RightsNotice`,
    /// ensuring semantically equivalent metadata regardless of output format.
    ///
    /// The normalization resolves DMI defaults, applies auto-computed timestamps,
    /// and merges `LegalMetadata` fields with context-level overrides.
    #[must_use]
    pub fn normalize_rights_notice(&self) -> RightsNotice {
        let legal = self.legal_metadata();
        let dmi = self
            .dmi_value()
            .or_else(|| {
                self.protection_level().and_then(|level| match level {
                    ProtectionLevel::Standard => Some(DmiValue::ProhibitedAiMlTraining),
                    _ => None,
                })
            })
            .filter(|v| *v != DmiValue::Unspecified);

        let notice_applied_at =
            legal
                .and_then(|l| l.notice_applied_at().map(String::from))
                .or_else(|| {
                    if legal.is_some() {
                        Some(self.timestamp_override.clone().unwrap_or_else(
                            crate::protected::metadata_trap::current_timestamp_iso8601,
                        ))
                    } else {
                        None
                    }
                });

        RightsNotice {
            copyright_holder: legal.and_then(|l| l.copyright_holder().map(String::from)),
            contact_email: legal.and_then(|l| l.contact_email().map(String::from)),
            license_url: legal.and_then(|l| l.license_url().map(String::from)),
            usage_terms: legal.and_then(|l| l.usage_terms().map(String::from)),
            usage_terms_lang: legal.and_then(|l| l.usage_terms_lang().map(String::from)),
            creation_date: legal.and_then(|l| l.creation_date().map(String::from)),
            ai_constraints: legal.and_then(|l| l.ai_constraints().map(String::from)),
            web_statement_of_rights: legal
                .and_then(|l| l.web_statement_of_rights().map(String::from)),
            creator: legal.and_then(|l| l.creator().map(String::from)),
            credit_line: legal.and_then(|l| l.credit_line().map(String::from)),
            copyright_owner: legal.and_then(|l| l.copyright_owner().map(String::from)),
            licensor_name: legal.and_then(|l| l.licensor_name().map(String::from)),
            licensor_email: legal.and_then(|l| l.licensor_email().map(String::from)),
            licensor_url: legal.and_then(|l| l.licensor_url().map(String::from)),
            metadata_date: legal.and_then(|l| l.metadata_date().map(String::from)),
            notice_applied_at,
            dmi,
            seed: Some(self.seed()),
        }
    }
}
