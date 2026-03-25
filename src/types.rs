use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// IPTC Photo Metadata Standard 2023.1 - DMI (Data Mining) tags for AI exclusion.
/// These tags communicate whether content may be used for AI/ML training.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum DmiValue {
    #[default]
    Unspecified,
    Allowed,
    ProhibitedAiMlTraining,
    ProhibitedGenAiMlTraining,
    ProhibitedExceptSearchEngineIndexing,
    Prohibited,
    ProhibitedSeeConstraints,
}

impl DmiValue {
    pub fn as_str(&self) -> &'static str {
        match self {
            DmiValue::Unspecified => "Unspecified",
            DmiValue::Allowed => "Allowed",
            DmiValue::ProhibitedAiMlTraining => "ProhibitedAiMlTraining",
            DmiValue::ProhibitedGenAiMlTraining => "ProhibitedGenAiMlTraining",
            DmiValue::ProhibitedExceptSearchEngineIndexing => {
                "ProhibitedExceptSearchEngineIndexing"
            }
            DmiValue::Prohibited => "Prohibited",
            DmiValue::ProhibitedSeeConstraints => "ProhibitedSeeConstraints",
        }
    }

    /// Returns the IPTC XMP property name for this DMI value.
    ///
    /// Note: The IPTC Photo Metadata Standard defines only two property names:
    /// `Iptc4xmpExt:DMI-Allowed` and `Iptc4xmpExt:DMI-Prohibited`.
    /// The specific prohibition granularity (`ProhibitedAiMlTraining`,
    /// `ProhibitedGenAiMlTraining`, etc.) is conveyed via the *value* of the
    /// property (returned by `as_str()`), not the property name itself.
    pub fn to_iptc_property(&self) -> &'static str {
        match self {
            DmiValue::Unspecified => "Iptc4xmpExt:DMI",
            DmiValue::Allowed => "Iptc4xmpExt:DMI-Allowed",
            DmiValue::ProhibitedAiMlTraining => "Iptc4xmpExt:DMI-Prohibited",
            DmiValue::ProhibitedGenAiMlTraining => "Iptc4xmpExt:DMI-Prohibited",
            DmiValue::ProhibitedExceptSearchEngineIndexing => "Iptc4xmpExt:DMI-Prohibited",
            DmiValue::Prohibited => "Iptc4xmpExt:DMI-Prohibited",
            DmiValue::ProhibitedSeeConstraints => "Iptc4xmpExt:DMI-Prohibited",
        }
    }
}

/// Protection level determining the protection strategy applied to images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum ProtectionLevel {
    Disabled,
    Light,
    #[default]
    Standard,
    Enhanced,
    Strong,
}

impl ProtectionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtectionLevel::Disabled => "disabled",
            ProtectionLevel::Light => "light",
            ProtectionLevel::Standard => "standard",
            ProtectionLevel::Enhanced => "enhanced",
            ProtectionLevel::Strong => "strong",
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            ProtectionLevel::Disabled => 0,
            ProtectionLevel::Light => 1,
            ProtectionLevel::Standard => 2,
            ProtectionLevel::Enhanced => 3,
            ProtectionLevel::Strong => 4,
        }
    }

    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(ProtectionLevel::Disabled),
            1 => Some(ProtectionLevel::Light),
            2 => Some(ProtectionLevel::Standard),
            3 => Some(ProtectionLevel::Enhanced),
            4 => Some(ProtectionLevel::Strong),
            _ => None,
        }
    }
}

/// Image output format for encoding protected images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum ImageOutputFormat {
    #[default]
    Png,
    Jpeg,
    WebP,
}

/// Default output format used when none is specified.
pub const DEFAULT_OUTPUT_FORMAT: ImageOutputFormat = ImageOutputFormat::Png;

impl ImageOutputFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "png" => Some(ImageOutputFormat::Png),
            "jpg" | "jpeg" => Some(ImageOutputFormat::Jpeg),
            "webp" => Some(ImageOutputFormat::WebP),
            _ => None,
        }
    }

    pub fn from_magic_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }
        if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return Some(ImageOutputFormat::Png);
        }
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(ImageOutputFormat::Jpeg);
        }
        if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            return Some(ImageOutputFormat::WebP);
        }
        None
    }

    pub fn is_png(bytes: &[u8]) -> bool {
        bytes.len() >= 4 && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47])
    }

    pub fn is_jpeg(bytes: &[u8]) -> bool {
        bytes.len() >= 3 && bytes.starts_with(&[0xFF, 0xD8, 0xFF])
    }

    pub fn is_webp(bytes: &[u8]) -> bool {
        bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
    }

    /// Returns the canonical file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            ImageOutputFormat::Png => "png",
            ImageOutputFormat::Jpeg => "jpg",
            ImageOutputFormat::WebP => "webp",
        }
    }

    pub fn to_image_format(self) -> image::ImageFormat {
        match self {
            ImageOutputFormat::Png => image::ImageFormat::Png,
            ImageOutputFormat::Jpeg => image::ImageFormat::Jpeg,
            ImageOutputFormat::WebP => image::ImageFormat::WebP,
        }
    }
}

/// Legal metadata for copyright and AI training restrictions.
/// This information is embedded in the image for legal discovery and proof of intent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalMetadata {
    copyright_holder: Option<String>,
    contact_email: Option<String>,
    license_url: Option<String>,
    usage_terms: Option<String>,
    creation_date: Option<String>,
    ai_constraints: Option<String>,
    web_statement_of_rights: Option<String>,
}

impl LegalMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn copyright_holder(&self) -> Option<&str> {
        self.copyright_holder.as_deref()
    }

    pub fn contact_email(&self) -> Option<&str> {
        self.contact_email.as_deref()
    }

    pub fn license_url(&self) -> Option<&str> {
        self.license_url.as_deref()
    }

    pub fn usage_terms(&self) -> Option<&str> {
        self.usage_terms.as_deref()
    }

    pub fn creation_date(&self) -> Option<&str> {
        self.creation_date.as_deref()
    }

    pub fn ai_constraints(&self) -> Option<&str> {
        self.ai_constraints.as_deref()
    }

    pub fn web_statement_of_rights(&self) -> Option<&str> {
        self.web_statement_of_rights.as_deref()
    }

    #[must_use]
    pub fn with_copyright_holder(mut self, holder: impl Into<String>) -> Self {
        self.copyright_holder = Some(holder.into());
        self
    }

    #[must_use]
    pub fn with_contact_email(mut self, email: impl Into<String>) -> Self {
        self.contact_email = Some(email.into());
        self
    }

    #[must_use]
    pub fn with_license_url(mut self, url: impl Into<String>) -> Self {
        self.license_url = Some(url.into());
        self
    }

    #[must_use]
    pub fn with_usage_terms(mut self, terms: impl Into<String>) -> Self {
        self.usage_terms = Some(terms.into());
        self
    }

    #[must_use]
    pub fn with_creation_date(mut self, date: impl Into<String>) -> Self {
        self.creation_date = Some(date.into());
        self
    }

    #[must_use]
    pub fn with_ai_constraints(mut self, constraints: impl Into<String>) -> Self {
        self.ai_constraints = Some(constraints.into());
        self
    }

    #[must_use]
    pub fn with_web_statement_of_rights(mut self, statement: impl Into<String>) -> Self {
        self.web_statement_of_rights = Some(statement.into());
        self
    }
}

/// Heavy configuration that is shared across requests via `Arc`.
/// Create once, reuse across many image processing calls.
/// This avoids per-request heap allocation of large fields.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtectionConfig {
    /// MAC key for cryptographic payload verification.
    ///
    /// # Security
    ///
    /// Without a MAC key, steganographic payload verification uses a trivial
    /// additive checksum that provides no cryptographic assurance. Always set a
    /// MAC key in adversarial settings to enable HMAC-SHA256 verification.
    pub mac_key: Option<Vec<u8>>,
    /// Legal metadata for copyright and AI training restrictions.
    pub legal_metadata: Option<LegalMetadata>,
}

impl ProtectionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_mac_key(mut self, key: Vec<u8>) -> Self {
        self.mac_key = Some(key);
        self
    }

    #[must_use]
    pub fn with_legal_metadata(mut self, metadata: LegalMetadata) -> Self {
        self.legal_metadata = Some(metadata);
        self
    }
}

/// Context for protection operations containing intensity and configuration.
///
/// Cheap to clone (heavy fields are in `Arc<ProtectionConfig>`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionContext {
    intensity: f32,
    seed: u64,
    input_format: Option<ImageOutputFormat>,
    output_format: Option<ImageOutputFormat>,
    protection_level: Option<ProtectionLevel>,
    dmi_value: Option<DmiValue>,
    max_dimension: Option<u32>,
    inject_metadata: Option<bool>,
    inject_legal_claims: Option<bool>,
    stego_redundancy: usize,
    jpeg_quality: u8,
    progressive_jpeg: bool,
    #[serde(skip)]
    config: Option<Arc<ProtectionConfig>>,
}

/// # Security
///
/// The default seed is generated from the system clock via
/// `generate_random_seed()` and is **not cryptographically secure**.
/// An attacker who knows the approximate request time can predict the seed.
/// Use `ProtectionContext::new(intensity, seed)` with a CSPRNG-generated
/// seed if unpredictability is required.
impl Default for ProtectionContext {
    fn default() -> Self {
        let seed = crate::util::seed::generate_random_seed();
        Self {
            intensity: 0.5,
            seed,
            input_format: None,
            output_format: None,
            protection_level: None,
            dmi_value: None,
            max_dimension: None,
            inject_metadata: None,
            inject_legal_claims: None,
            stego_redundancy: 2,
            jpeg_quality: 90,
            progressive_jpeg: false,
            config: None,
        }
    }
}

impl ProtectionContext {
    /// Create a new ProtectionContext with the specified intensity and seed.
    ///
    /// Intensity is clamped to the range [0.0, 1.0].
    pub fn new(intensity: f32, seed: u64) -> Self {
        Self {
            intensity: intensity.clamp(0.0, 1.0),
            seed,
            input_format: None,
            output_format: None,
            protection_level: None,
            dmi_value: None,
            max_dimension: None,
            inject_metadata: None,
            inject_legal_claims: None,
            stego_redundancy: 2,
            jpeg_quality: 90,
            progressive_jpeg: false,
            config: None,
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
    pub fn mac_key(&self) -> Option<&[u8]> {
        self.config.as_ref().and_then(|c| c.mac_key.as_deref())
    }

    /// Access the legal metadata, if set.
    pub fn legal_metadata(&self) -> Option<&LegalMetadata> {
        self.config.as_ref().and_then(|c| c.legal_metadata.as_ref())
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
    #[must_use]
    pub fn with_dmi(mut self, dmi: DmiValue) -> Self {
        self.dmi_value = Some(dmi);
        self
    }

    /// Enable metadata injection (seed, DMI). This is the default for Standard+ levels.
    #[must_use]
    pub fn with_metadata_injection(mut self, enable: bool) -> Self {
        self.inject_metadata = Some(enable);
        self
    }

    /// Enable legal claim injection (copyright, artist).
    /// WARNING: Only enable for content you own. May create legal liability otherwise.
    #[must_use]
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

    /// Set the stego embedding redundancy (1-5). Higher values are more robust
    /// for verification but slower. Default is 2.
    #[must_use]
    pub fn with_stego_redundancy(mut self, redundancy: usize) -> Self {
        self.stego_redundancy = redundancy.clamp(1, 5);
        self
    }

    /// Set the JPEG encoding quality (1-100). Default is 90.
    #[must_use]
    pub fn with_jpeg_quality(mut self, quality: u8) -> Self {
        self.jpeg_quality = quality.clamp(1, 100);
        self
    }

    /// Enable progressive JPEG encoding. Progressive JPEGs render faster on
    /// slow connections as the image appears progressively. Default is false.
    #[must_use]
    pub fn with_progressive_jpeg(mut self, progressive: bool) -> Self {
        self.progressive_jpeg = progressive;
        self
    }

    /// Get the intensity value.
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// Get the seed value.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Get the input format hint.
    pub fn input_format(&self) -> Option<ImageOutputFormat> {
        self.input_format
    }

    /// Get the output format.
    pub fn output_format(&self) -> Option<ImageOutputFormat> {
        self.output_format
    }

    /// Get the protection level.
    pub fn protection_level(&self) -> Option<ProtectionLevel> {
        self.protection_level
    }

    /// Get the DMI value.
    pub fn dmi_value(&self) -> Option<DmiValue> {
        self.dmi_value
    }

    /// Get the maximum dimension limit.
    pub fn max_dimension(&self) -> Option<u32> {
        self.max_dimension
    }

    /// Get whether metadata injection is enabled.
    pub fn inject_metadata(&self) -> Option<bool> {
        self.inject_metadata
    }

    /// Get whether legal claim injection is enabled.
    pub fn inject_legal_claims(&self) -> Option<bool> {
        self.inject_legal_claims
    }

    /// Get the stego embedding redundancy.
    pub fn stego_redundancy(&self) -> usize {
        self.stego_redundancy
    }

    /// Get the JPEG encoding quality.
    pub fn jpeg_quality(&self) -> u8 {
        self.jpeg_quality
    }

    /// Get whether progressive JPEG encoding is enabled.
    pub fn progressive_jpeg(&self) -> bool {
        self.progressive_jpeg
    }

    /// Set the input format hint (non-consuming).
    pub fn set_input_format(&mut self, format: ImageOutputFormat) {
        self.input_format = Some(format);
    }

    /// Set the protection level (non-consuming, crate-internal).
    pub(crate) fn set_protection_level(&mut self, level: ProtectionLevel) {
        self.protection_level = Some(level);
    }
}

/// Precomputed protected variant for fast CDN edge deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedVariant {
    variant_id: uuid::Uuid,
    original_hash: String,
    protection_level: ProtectionLevel,
    perturbation_data: Vec<u8>,
    intensity: f32,
    width: u32,
    height: u32,
}

impl ProtectedVariant {
    /// Create a new ProtectedVariant with the specified parameters.
    ///
    /// The `original_hash` is the SHA-256 hash of the original image.
    /// The `perturbation_data` contains the precomputed adversarial perturbations.
    pub fn new(
        original_hash: String,
        protection_level: ProtectionLevel,
        perturbation_data: Vec<u8>,
        intensity: f32,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            variant_id: uuid::Uuid::new_v4(),
            original_hash,
            protection_level,
            perturbation_data,
            intensity,
            width,
            height,
        }
    }

    /// Generate a cache key for this variant based on hash, level, and intensity.
    pub fn cache_key(&self) -> String {
        // Round intensity to 4 decimal places to avoid f32 representation mismatches
        // (e.g., 0.1 + 0.2 formatting as "0.30000001").
        let intensity_rounded = (self.intensity * 10000.0).round() / 10000.0;
        format!(
            "{}_{}_{}",
            self.original_hash,
            self.protection_level.as_str(),
            intensity_rounded
        )
    }

    pub fn variant_id(&self) -> uuid::Uuid {
        self.variant_id
    }

    pub fn original_hash(&self) -> &str {
        &self.original_hash
    }

    pub fn protection_level(&self) -> ProtectionLevel {
        self.protection_level
    }

    pub fn perturbation_data(&self) -> &[u8] {
        &self.perturbation_data
    }

    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_chain() {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_stego_redundancy(3);
        assert_eq!(ctx.intensity(), 0.5);
        assert_eq!(ctx.seed(), 42);
        assert_eq!(ctx.stego_redundancy(), 3);
    }

    #[test]
    fn intensity_clamped() {
        let ctx = ProtectionContext::new(2.0, 42);
        assert_eq!(ctx.intensity(), 1.0);

        let ctx = ProtectionContext::new(-1.0, 42);
        assert_eq!(ctx.intensity(), 0.0);
    }

    #[test]
    fn seed_roundtrip_through_serde() {
        let ctx = ProtectionContext::new(0.7, 12345);
        let json = serde_json::to_string(&ctx).unwrap();
        let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.seed(), 12345);
        assert_eq!(restored.intensity(), 0.7);
    }
}
