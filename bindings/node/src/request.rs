use crate::enums::{
    AuthenticationMode, HiddenMarkerMode, ImageOutputFormat, MetadataUpdatePolicy,
    ProtectionPreset, RightsPolicy,
};
use crate::error::Result;
use crate::limits::ResourceLimits;
use crate::notice::{ProcessingOptions, RightsNotice};
use napi::bindgen_prelude::BigInt;
use napi_derive::napi;
use stegoeggo::{ProtectionChannels, ProtectionRequest as RustRequest};

/// The canonical protection request.
///
/// Every protection operation delegates to `ProtectionRequest` and the
/// canonical `process_request_bytes*` byte API. Deprecated `ProtectionLevel`,
/// `ProtectionContext`, and `EvidenceProfile` adapters are intentionally not
/// projected here.
#[derive(Clone, Debug)]
#[napi]
pub struct ProtectionRequest {
    inner: RustRequest,
}

#[napi]
impl ProtectionRequest {
    /// Metadata-only protection: the fastest path.
    #[napi(factory)]
    pub fn metadata_only(notice: &RightsNotice, policy: RightsPolicy) -> Self {
        Self {
            inner: RustRequest::metadata_only(notice.into(), policy.into()),
        }
    }

    /// Metadata plus a best-effort hidden marker.
    #[napi(factory)]
    pub fn with_hidden_marker(notice: &RightsNotice, policy: RightsPolicy) -> Self {
        Self {
            inner: RustRequest::with_hidden_marker(notice.into(), policy.into()),
        }
    }

    /// Protection from a named channel preset.
    #[napi(factory)]
    pub fn from_preset(
        preset: ProtectionPreset,
        notice: &RightsNotice,
        policy: RightsPolicy,
    ) -> Self {
        Self {
            inner: RustRequest::from_preset(preset.into(), notice.into(), policy.into()),
        }
    }

    /// Sets the steganographic seed as a full-width `bigint`.
    ///
    /// `0` is a valid seed. Values above `Number.MAX_SAFE_INTEGER` are
    /// preserved exactly because the value never passes through a JavaScript
    /// `number`.
    #[napi]
    pub fn with_seed(&self, seed: BigInt) -> Result<Self> {
        Ok(Self {
            inner: self
                .inner
                .clone()
                .with_seed(crate::numeric::to_u64("seed", &seed)?),
        })
    }

    /// Sets the embedding intensity in `0.0..=1.0`.
    #[napi]
    pub fn with_intensity(&self, intensity: f64) -> Result<Self> {
        Ok(Self {
            inner: self
                .inner
                .clone()
                .with_intensity(crate::numeric::to_unit_f32("intensity", intensity)?),
        })
    }

    /// Sets the encoded output format.
    #[must_use]
    #[napi]
    pub fn with_output_format(&self, format: ImageOutputFormat) -> Self {
        Self {
            inner: self.inner.clone().with_output_format(format.into()),
        }
    }

    /// Sets the JPEG quality.
    #[napi]
    pub fn with_jpeg_quality(&self, quality: f64) -> Result<Self> {
        Ok(Self {
            inner: self
                .inner
                .clone()
                .with_jpeg_quality(crate::numeric::to_u8("jpegQuality", quality)?),
        })
    }

    /// Enables progressive JPEG output.
    #[must_use]
    #[napi]
    pub fn with_progressive_jpeg(&self) -> Self {
        Self {
            inner: self.inner.clone().with_progressive_jpeg(),
        }
    }

    /// Sets the maximum output dimension.
    #[napi]
    pub fn with_max_dimension(&self, max_dimension: f64) -> Result<Self> {
        Ok(Self {
            inner: self
                .inner
                .clone()
                .with_max_dimension(crate::numeric::to_u32("maxDimension", max_dimension)?),
        })
    }

    /// Sets the metadata update policy.
    #[must_use]
    #[napi]
    pub fn with_metadata_update_policy(&self, policy: MetadataUpdatePolicy) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .with_metadata_update_policy(policy.into()),
        }
    }

    /// Sets the stego redundancy override (`1..=10`).
    #[napi]
    pub fn with_stego_redundancy(&self, redundancy: f64) -> Result<Self> {
        Ok(Self {
            inner: self
                .inner
                .clone()
                .with_stego_redundancy(crate::numeric::to_usize("stegoRedundancy", redundancy)?),
        })
    }

    /// Sets the 4-byte provenance content hash.
    #[napi]
    pub fn with_content_hash(&self, hash: &[u8]) -> Result<Self> {
        let bytes: [u8; 4] = hash
            .try_into()
            .map_err(|_| crate::error::config_error("contentHash must be exactly 4 bytes"))?;
        Ok(Self {
            inner: self.inner.clone().with_content_hash(bytes),
        })
    }

    /// Sets the deterministic notice-applied-at timestamp override.
    ///
    /// Without an override the canonical resolver uses wall-clock time, so two
    /// invocations of the same request are not byte-identical.
    #[must_use]
    #[napi]
    pub fn with_timestamp_override(&self, timestamp: String) -> Self {
        Self {
            inner: self.inner.clone().with_timestamp_override(timestamp),
        }
    }

    /// Replaces the whole processing-options block.
    #[must_use]
    #[napi]
    pub fn with_processing(&self, processing: &ProcessingOptions) -> Self {
        Self {
            inner: self.inner.clone().with_processing(processing.into()),
        }
    }

    /// Sets the HMAC/MAC key. The bytes are never echoed back into reports,
    /// errors, or generated declarations.
    #[must_use]
    #[napi]
    pub fn with_mac_key(&self, key: &[u8]) -> Self {
        Self {
            inner: self.inner.clone().with_mac_key(key.to_vec()),
        }
    }

    /// Applies parser hardening limits for untrusted input.
    #[must_use]
    #[napi]
    pub fn with_resource_limits(&self, limits: &ResourceLimits) -> Self {
        Self {
            inner: self.inner.clone().with_resource_limits(limits.into()),
        }
    }

    /// Replaces the hidden-marker strategy, leaving the other channels intact.
    #[must_use]
    #[napi]
    pub fn with_hidden_marker_mode(&self, mode: &HiddenMarkerMode) -> Self {
        let mut channels = self.inner.channels().clone();
        channels.hidden_marker = mode.inner;
        Self {
            inner: with_channels(&self.inner, channels),
        }
    }

    /// Replaces the authentication mode, leaving the other channels intact.
    #[must_use]
    #[napi]
    pub fn with_authentication(&self, mode: AuthenticationMode) -> Self {
        let mut channels = self.inner.channels().clone();
        channels.authentication = mode.into();
        Self {
            inner: with_channels(&self.inner, channels),
        }
    }

    /// Whether rights metadata is requested.
    #[napi(getter)]
    pub fn rights_metadata_enabled(&self) -> bool {
        self.inner.channels().rights_metadata
    }

    /// The configured hidden-marker strategy.
    #[napi(getter)]
    pub fn hidden_marker_mode(&self) -> HiddenMarkerMode {
        HiddenMarkerMode::from(self.inner.channels().hidden_marker)
    }

    /// The configured authentication mode.
    #[napi(getter)]
    pub fn authentication_mode(&self) -> AuthenticationMode {
        self.inner.channels().authentication.into()
    }

    /// The rights policy.
    #[napi(getter)]
    pub fn policy(&self) -> RightsPolicy {
        self.inner.policy().into()
    }

    /// The steganographic seed as a full-width `bigint`, or `null` when unset.
    #[napi(getter, ts_return_type = "bigint | null")]
    pub fn seed(&self) -> Option<u64> {
        self.inner.seed()
    }

    /// The embedding intensity.
    #[napi(getter)]
    pub fn intensity(&self) -> f64 {
        f64::from(self.inner.intensity())
    }

    /// Whether a MAC/HMAC key is configured. The key bytes are never exposed.
    #[napi(getter)]
    pub fn has_mac_key(&self) -> bool {
        self.inner.mac_key().is_some()
    }

    /// The timestamp override, or `null` when unset.
    #[napi(getter, ts_return_type = "string | null")]
    pub fn timestamp_override(&self) -> Option<String> {
        self.inner.timestamp_override().map(ToString::to_string)
    }
}

/// Rebuilds a canonical request with a different channel configuration while
/// preserving every other canonical field.
fn with_channels(request: &RustRequest, channels: ProtectionChannels) -> RustRequest {
    let mut next = RustRequest::new(request.notice().clone(), request.policy(), channels)
        .with_processing(request.processing().clone())
        .with_intensity(request.intensity());
    if let Some(seed) = request.seed() {
        next = next.with_seed(seed);
    }
    if let Some(legal) = request.legal_metadata() {
        next = next.with_legal_metadata(legal.clone());
    }
    if let Some(key) = request.mac_key() {
        next = next.with_mac_key(key.to_vec());
    }
    if let Some(limits) = request.resource_limits() {
        next = next.with_resource_limits(limits.clone());
    }
    next
}

impl From<&ProtectionRequest> for RustRequest {
    fn from(value: &ProtectionRequest) -> Self {
        value.inner.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice() -> RightsNotice {
        RightsNotice::new().with_copyright_holder("Acme".to_string())
    }

    #[test]
    fn metadata_only_is_immutable() {
        let base =
            ProtectionRequest::metadata_only(&notice(), RightsPolicy::ProhibitedAiMlTraining);
        let seeded = base.with_seed(BigInt::from(42_u64)).unwrap();
        assert_eq!(base.seed(), None);
        assert_eq!(seeded.seed(), Some(42));
        assert_eq!(base.policy(), RightsPolicy::ProhibitedAiMlTraining);
        assert!(base.rights_metadata_enabled());
    }

    #[test]
    fn hidden_marker_factory_enables_channel() {
        let request =
            ProtectionRequest::with_hidden_marker(&notice(), RightsPolicy::ProhibitedAiMlTraining);
        assert_eq!(
            request.hidden_marker_mode().kind(),
            crate::enums::HiddenMarkerKind::BestEffort
        );
    }

    #[test]
    fn preset_sets_channels() {
        let request = ProtectionRequest::from_preset(
            ProtectionPreset::AuthenticatedProvenance,
            &notice(),
            RightsPolicy::ProhibitedSeeConstraints,
        );
        assert_eq!(request.authentication_mode(), AuthenticationMode::Hmac);
    }

    #[test]
    fn hidden_marker_mode_can_be_replaced() {
        let request =
            ProtectionRequest::with_hidden_marker(&notice(), RightsPolicy::ProhibitedAiMlTraining);
        let tiled = request.with_hidden_marker_mode(&HiddenMarkerMode::tiled(128.0).unwrap());
        assert_eq!(tiled.hidden_marker_mode().tile_size(), Some(128));
        assert_eq!(
            request.hidden_marker_mode().kind(),
            crate::enums::HiddenMarkerKind::BestEffort
        );
    }

    #[test]
    fn seed_preserves_full_width() {
        let request = ProtectionRequest::metadata_only(&notice(), RightsPolicy::Allowed)
            .with_seed(BigInt::from(u64::MAX))
            .unwrap();
        assert_eq!(request.seed(), Some(u64::MAX));
    }

    #[test]
    fn seed_rejects_invalid_bigint() {
        let request = ProtectionRequest::metadata_only(&notice(), RightsPolicy::Allowed);
        let negative = BigInt {
            sign_bit: true,
            words: vec![7],
        };
        assert!(request.with_seed(negative).is_err());
    }

    #[test]
    fn mac_key_is_never_readable() {
        let request = ProtectionRequest::metadata_only(&notice(), RightsPolicy::Allowed)
            .with_mac_key(b"super-secret-mac-key");
        assert!(request.has_mac_key());
        assert!(!format!("{request:?}").contains("super-secret-mac-key"));
    }

    #[test]
    fn channels_helper_preserves_every_other_field() {
        let request = RustRequest::with_hidden_marker(
            stegoeggo::RightsNotice::new().with_copyright_holder("Acme"),
            stegoeggo::RightsPolicy::ProhibitedAiMlTraining,
        )
        .with_seed(42)
        .with_intensity(0.25)
        .with_mac_key(vec![1, 2, 3])
        .with_timestamp_override("2026-01-01T00:00:00Z");
        let mut channels = request.channels().clone();
        channels.authentication = stegoeggo::AuthenticationMode::Hmac;
        let next = with_channels(&request, channels);
        assert_eq!(next.seed(), Some(42));
        assert!((next.intensity() - 0.25).abs() < f32::EPSILON);
        assert_eq!(next.mac_key(), Some(&[1_u8, 2, 3][..]));
        assert_eq!(next.timestamp_override(), Some("2026-01-01T00:00:00Z"));
        assert_eq!(
            next.policy(),
            stegoeggo::RightsPolicy::ProhibitedAiMlTraining
        );
        assert_eq!(next.notice().copyright_holder(), Some("Acme"));
        assert_eq!(
            next.channels().authentication,
            stegoeggo::AuthenticationMode::Hmac
        );
    }

    #[test]
    fn channels_type_is_public() {
        let channels = ProtectionChannels::metadata_only();
        assert!(channels.rights_metadata);
    }
}
