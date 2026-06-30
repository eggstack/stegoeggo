use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// IPTC Photo Metadata Standard 2023.1 - DMI (Data Mining) tags for AI exclusion.
/// These tags communicate whether content may be used for AI/ML training.
///
/// When injected into XMP metadata, the TDM Reservation Protocol (ISO/IEC 21000-21)
/// property `tdm:reserve_tdm` is also included: `"1"` for all prohibition values,
/// `"0"` for `Allowed`. This is the standard referenced by the EU AI Act (2024).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum DmiValue {
    /// No DMI restriction specified (default).
    #[default]
    Unspecified,
    /// Content may be used for AI/ML training.
    Allowed,
    /// Prohibited for AI/ML training.
    ProhibitedAiMlTraining,
    /// Prohibited for generative AI training.
    ProhibitedGenAiMlTraining,
    /// Prohibited except for search engine indexing.
    ProhibitedExceptSearchEngineIndexing,
    /// All uses prohibited.
    Prohibited,
    /// Prohibited, see constraints for details.
    ProhibitedSeeConstraints,
}

impl DmiValue {
    /// Returns the string representation of this DMI value.
    #[must_use]
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
    /// No protection applied.
    Disabled,
    /// Metadata injection with minimal steganography.
    Light,
    /// Full steganography + metadata injection (default).
    #[default]
    Standard,
}

impl ProtectionLevel {
    /// Returns the lowercase string representation of this protection level.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtectionLevel::Disabled => "disabled",
            ProtectionLevel::Light => "light",
            ProtectionLevel::Standard => "standard",
        }
    }

    /// Encodes this protection level as a single byte for payload serialization.
    #[must_use]
    pub fn to_byte(&self) -> u8 {
        match self {
            ProtectionLevel::Disabled => 0,
            ProtectionLevel::Light => 1,
            ProtectionLevel::Standard => 2,
        }
    }

    /// Decodes a protection level from a byte. Returns `None` for unknown values.
    #[must_use]
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(ProtectionLevel::Disabled),
            1 => Some(ProtectionLevel::Light),
            2 => Some(ProtectionLevel::Standard),
            _ => None,
        }
    }
}

/// Image output format for encoding protected images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum ImageOutputFormat {
    /// Portable Network Graphics (default).
    #[default]
    Png,
    /// Joint Photographic Experts Group.
    Jpeg,
    /// WebP image format.
    WebP,
}

/// Default output format used when none is specified.
pub const DEFAULT_OUTPUT_FORMAT: ImageOutputFormat = ImageOutputFormat::Png;

impl ImageOutputFormat {
    /// Parses an image format from a file extension (case-insensitive).
    ///
    /// Recognizes `"png"`, `"jpg"`, `"jpeg"`, and `"webp"`.
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "png" => Some(ImageOutputFormat::Png),
            "jpg" | "jpeg" => Some(ImageOutputFormat::Jpeg),
            "webp" => Some(ImageOutputFormat::WebP),
            _ => None,
        }
    }

    /// Detects the image format from file magic bytes.
    ///
    /// Returns `None` if the bytes are too short or the format is unrecognized.
    #[must_use]
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

    /// Returns `true` if the bytes start with the PNG magic number.
    #[must_use]
    pub fn is_png(bytes: &[u8]) -> bool {
        bytes.len() >= 4 && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47])
    }

    /// Returns `true` if the bytes start with the JPEG magic number.
    #[must_use]
    pub fn is_jpeg(bytes: &[u8]) -> bool {
        bytes.len() >= 3 && bytes.starts_with(&[0xFF, 0xD8, 0xFF])
    }

    /// Returns `true` if the bytes start with the RIFF/WEBP magic number.
    #[must_use]
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

    /// Converts to the corresponding `image::ImageFormat` variant.
    #[must_use]
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
    /// Creates a new `LegalMetadata` with all fields unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the copyright holder name, if set.
    #[must_use]
    pub fn copyright_holder(&self) -> Option<&str> {
        self.copyright_holder.as_deref()
    }

    /// Returns the contact email for IP claims, if set.
    #[must_use]
    pub fn contact_email(&self) -> Option<&str> {
        self.contact_email.as_deref()
    }

    /// Returns the license URL, if set.
    #[must_use]
    pub fn license_url(&self) -> Option<&str> {
        self.license_url.as_deref()
    }

    /// Returns the usage terms string, if set.
    #[must_use]
    pub fn usage_terms(&self) -> Option<&str> {
        self.usage_terms.as_deref()
    }

    /// Returns the creation date string, if set.
    #[must_use]
    pub fn creation_date(&self) -> Option<&str> {
        self.creation_date.as_deref()
    }

    /// Returns the AI training constraints string, if set.
    #[must_use]
    pub fn ai_constraints(&self) -> Option<&str> {
        self.ai_constraints.as_deref()
    }

    /// Returns the web statement of rights URL, if set.
    #[must_use]
    pub fn web_statement_of_rights(&self) -> Option<&str> {
        self.web_statement_of_rights.as_deref()
    }

    /// Sets the copyright holder name.
    #[must_use]
    pub fn with_copyright_holder(mut self, holder: impl Into<String>) -> Self {
        self.copyright_holder = Some(holder.into());
        self
    }

    /// Sets the contact email for IP claims.
    #[must_use]
    pub fn with_contact_email(mut self, email: impl Into<String>) -> Self {
        self.contact_email = Some(email.into());
        self
    }

    /// Sets the license URL.
    #[must_use]
    pub fn with_license_url(mut self, url: impl Into<String>) -> Self {
        self.license_url = Some(url.into());
        self
    }

    /// Sets the usage terms (e.g., "All Rights Reserved").
    #[must_use]
    pub fn with_usage_terms(mut self, terms: impl Into<String>) -> Self {
        self.usage_terms = Some(terms.into());
        self
    }

    /// Sets the creation date string.
    #[must_use]
    pub fn with_creation_date(mut self, date: impl Into<String>) -> Self {
        self.creation_date = Some(date.into());
        self
    }

    /// Sets the AI training constraints (e.g., "No AI training permitted").
    #[must_use]
    pub fn with_ai_constraints(mut self, constraints: impl Into<String>) -> Self {
        self.ai_constraints = Some(constraints.into());
        self
    }

    /// Sets the web statement of rights URL.
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
#[derive(Debug, Clone, Deserialize)]
pub struct ProtectionContext {
    intensity: f32,
    seed: u64,
    input_format: Option<ImageOutputFormat>,
    output_format: Option<ImageOutputFormat>,
    protection_level: Option<ProtectionLevel>,
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
    /// - `None` (default): never inject legal claims (level default is off).
    /// - `Some(true)`: force-enable legal claim injection.
    /// - `Some(false)`: force-disable legal claim injection (same as `None`).
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
    #[serde(skip)]
    config: Option<Arc<ProtectionConfig>>,
}

impl Serialize for ProtectionContext {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut fields = 15;
        if self.config.is_some() {
            fields += 1;
        }
        let mut s = serializer.serialize_struct("ProtectionContext", fields)?;
        s.serialize_field("intensity", &self.intensity)?;
        s.serialize_field("seed", &self.seed)?;
        s.serialize_field("input_format", &self.input_format)?;
        s.serialize_field("output_format", &self.output_format)?;
        s.serialize_field("protection_level", &self.protection_level)?;
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
        if self.config.is_some() {
            s.serialize_field(
                "_config_dropped_warning",
                "ProtectionContext.config is not serialized; MAC key and legal metadata will be lost on roundtrip. Set them again after deserialization.",
            )?;
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
            config: None,
        }
    }
}

impl ProtectionContext {
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
    #[must_use]
    pub fn mac_key(&self) -> Option<&[u8]> {
        self.config.as_ref().and_then(|c| c.mac_key.as_deref())
    }

    /// Access the legal metadata, if set.
    #[must_use]
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
    #[must_use]
    pub fn with_metadata_injection(mut self, enable: bool) -> Self {
        self.inject_metadata = Some(enable);
        self
    }

    /// Override the default for legal claim injection.
    ///
    /// When `enable` is `true`, legal claims (copyright, artist) are injected
    /// into the image metadata. When `enable` is `false`, legal claim injection
    /// is disabled (same as the default).
    ///
    /// Legal claims require [`LegalMetadata`] to be set via
    /// [`with_legal_metadata`](ProtectionContext::with_legal_metadata).
    ///
    /// If this method is **not** called, legal claims are never injected
    /// regardless of protection level.
    ///
    /// # Warning
    ///
    /// Only enable for content you own. May create legal liability otherwise.
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

    /// Set the stego embedding redundancy (1-10). Higher values are more robust
    /// for verification but slower. When not set, redundancy is derived from
    /// `intensity` via the internal `effective_redundancy()` helper.
    #[must_use]
    pub fn with_stego_redundancy(mut self, redundancy: usize) -> Self {
        self.stego_redundancy = Some(redundancy.clamp(1, 10));
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

    /// Enable tiled stego embedding for crop resistance.
    ///
    /// Each `size × size` pixel tile embeds a full copy of the payload. The
    /// extractor scans candidate tile origins so the payload is recoverable
    /// from any crop that contains at least one intact tile.
    ///
    /// Pass `0` to disable tiling (same as never calling this method).
    /// Valid range for non-zero values: 32..=1024. Values outside that range
    /// are clamped. The most common choice is 64 (matches the LSB tile
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
        if size == 0 {
            self.tile_size = Some(0);
        } else {
            self.tile_size = Some(size.clamp(32, 1024));
        }
        self
    }

    /// Set the maximum number of candidate tile origins the extractor will
    /// try. Default is 64. Higher values increase extraction time but improve
    /// recovery from small or misaligned crops.
    #[must_use]
    pub fn with_tile_extraction_max_origins(mut self, n: u32) -> Self {
        self.tile_extraction_max_origins = n.clamp(1, 4096);
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

    /// Get whether legal claim injection is enabled.
    ///
    /// Returns the caller's explicit override, if any. `None` means the
    /// pipeline will **not** inject legal claims (default is off).
    /// The pipeline resolves this by calling `inject_legal_claims.unwrap_or(false)`.
    #[must_use]
    pub fn inject_legal_claims(&self) -> Option<bool> {
        self.inject_legal_claims
    }

    /// Get the effective stego redundancy.
    ///
    /// When the user has explicitly set `stego_redundancy` via
    /// `with_stego_redundancy()`, that value is returned. Otherwise,
    /// the redundancy is derived from the current `intensity`:
    /// - `intensity < 0.3` → 1 (minimal embedding)
    /// - `intensity < 0.7` → 2 (standard)
    /// - `intensity >= 0.7` → 3 (heavy)
    #[must_use]
    pub fn stego_redundancy(&self) -> usize {
        self.effective_redundancy()
    }

    pub(crate) fn effective_redundancy(&self) -> usize {
        if let Some(r) = self.stego_redundancy {
            return r;
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
    /// try. Always at least 1.
    #[must_use]
    pub fn tile_extraction_max_origins(&self) -> u32 {
        self.tile_extraction_max_origins.max(1)
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

    /// Set the protection level (non-consuming, crate-internal).
    pub(crate) fn set_protection_level(&mut self, level: ProtectionLevel) {
        self.protection_level = Some(level);
    }
}

/// Detailed result of image protection verification.
///
/// Returned by [`verify_image_bytes_detailed`](crate::verify_image_bytes_detailed).
/// Provides richer information than the `Option<bool>` return of
/// [`verify_image_bytes`](crate::verify_image_bytes).
#[derive(Debug, Clone)]
pub enum VerificationResult {
    /// Protection data found and integrity check passed.
    ///
    /// Contains the extracted [`StegoPayload`](crate::StegoPayload) with
    /// protection metadata (seed, intensity, version, content hash, DMI value).
    Verified {
        /// The extracted payload from the protected image.
        payload: crate::StegoPayload,
    },
    /// Protection data found but integrity check failed.
    ///
    /// The payload was extracted but either the CRC32 checksum is invalid
    /// (non-MAC mode) or the HMAC-SHA256 verification failed (MAC mode).
    /// This may indicate corruption, wrong MAC key, or tampering.
    Corrupted {
        /// The partially extracted payload (may contain valid metadata).
        payload: crate::StegoPayload,
    },
    /// Metadata markers were found, but no steganographic payload could be
    /// integrity-verified.
    ///
    /// This is useful evidence that the image passed through the protection
    /// pipeline, but it is weaker than [`Verified`](Self::Verified). Metadata
    /// can be stripped, copied, or forged more easily than a MAC-verified
    /// steganographic payload.
    MetadataOnly {
        /// Protection seed recovered from metadata.
        seed: u64,
    },
    /// No protection data found in the image.
    ///
    /// The extraction chain exhausted all seed sources (metadata, LSB fallback,
    /// tiled extraction) without finding a valid payload.
    NotFound,
}

impl VerificationResult {
    /// Returns `true` if verification succeeded.
    #[must_use]
    pub fn is_verified(&self) -> bool {
        matches!(self, VerificationResult::Verified { .. })
    }

    /// Returns `true` if protection data was found (whether valid or corrupted).
    #[must_use]
    pub fn is_found(&self) -> bool {
        !matches!(self, VerificationResult::NotFound)
    }

    /// Returns the payload if verification succeeded.
    #[must_use]
    pub fn payload(&self) -> Option<&crate::StegoPayload> {
        match self {
            VerificationResult::Verified { payload } => Some(payload),
            _ => None,
        }
    }

    /// Returns the metadata seed when the result is metadata-only evidence.
    #[must_use]
    pub fn metadata_seed(&self) -> Option<u64> {
        match self {
            VerificationResult::MetadataOnly { seed } => Some(*seed),
            _ => None,
        }
    }
}

/// Simple verification status for quick checks.
///
/// Returned by [`verify_image_bytes`](crate::verify_image_bytes) and
/// [`SteganographyProtector::verify_payload_with_key`](crate::SteganographyProtector::verify_payload_with_key).
/// For richer information, use [`VerificationResult`] via
/// [`verify_image_bytes_detailed`](crate::verify_image_bytes_detailed).
///
/// # Examples
///
/// ```no_run
/// use stegoeggo::VerificationStatus;
///
/// let img_bytes: Vec<u8> = std::fs::read("protected.png").unwrap();
/// match stegoeggo::verify_image_bytes(&img_bytes, b"key") {
///     VerificationStatus::Verified => println!("Protected and verified"),
///     VerificationStatus::Invalid => println!("Protected but verification failed"),
///     VerificationStatus::NotFound => println!("No protection found"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerificationStatus {
    /// Protection data found and integrity check passed.
    Verified,
    /// Protection data found but integrity check failed.
    ///
    /// The payload was extracted but either the CRC32 checksum is invalid
    /// (non-MAC mode) or the HMAC-SHA256 verification failed (MAC mode).
    /// This may indicate corruption, wrong MAC key, or tampering.
    Invalid,
    /// No protection data found in the image.
    NotFound,
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationStatus::Verified => write!(f, "Verified"),
            VerificationStatus::Invalid => write!(f, "Invalid"),
            VerificationStatus::NotFound => write!(f, "NotFound"),
        }
    }
}

impl From<Option<bool>> for VerificationStatus {
    fn from(val: Option<bool>) -> Self {
        match val {
            Some(true) => VerificationStatus::Verified,
            Some(false) => VerificationStatus::Invalid,
            None => VerificationStatus::NotFound,
        }
    }
}

impl From<VerificationStatus> for Option<bool> {
    fn from(val: VerificationStatus) -> Self {
        match val {
            VerificationStatus::Verified => Some(true),
            VerificationStatus::Invalid => Some(false),
            VerificationStatus::NotFound => None,
        }
    }
}

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
    /// WebP lossy re-encoding will destroy steganographic payloads.
    ///
    /// WebP lossy compression modifies pixel values, which destroys LSB
    /// steganographic data. Use lossless WebP or another format to preserve
    /// steganographic protection.
    WebpLossyReencodeDestructive,
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
            ProtectionWarning::WebpLossyReencodeDestructive => write!(
                f,
                "WebP lossy re-encoding will destroy LSB steganographic payloads. \
                 Use lossless WebP or another format to preserve steganographic protection."
            ),
        }
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

    #[test]
    fn serialize_emits_warning_when_config_set() {
        let ctx = ProtectionContext::new(0.5, 99).with_mac_key(b"key".to_vec());
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(
            json.contains("_config_dropped_warning"),
            "Serialized JSON should contain a warning field when config is set: {json}"
        );
        assert!(
            json.contains("MAC key"),
            "Warning should mention the MAC key: {json}"
        );

        let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.seed(), 99);
        assert_eq!(restored.intensity(), 0.5);
        assert!(
            restored.mac_key().is_none(),
            "MAC key should be lost after serde roundtrip even when warning is emitted"
        );
    }

    #[test]
    fn serialize_no_warning_when_config_none() {
        let ctx = ProtectionContext::new(0.5, 99);
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(
            !json.contains("_config_dropped_warning"),
            "No warning should be emitted when config is None: {json}"
        );
    }

    // ── Tile size configuration ───────────────────────────────────────

    #[test]
    fn tile_size_default_is_none() {
        let ctx = ProtectionContext::new(0.5, 42);
        assert_eq!(ctx.tile_size(), None);
        assert!(!ctx.is_tile_mode_enabled());
    }

    #[test]
    fn with_tile_size_zero_disables_tiling() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(0);
        assert_eq!(ctx.tile_size(), Some(0));
        assert!(!ctx.is_tile_mode_enabled());
    }

    #[test]
    fn with_tile_size_enables_tiling() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(64);
        assert_eq!(ctx.tile_size(), Some(64));
        assert!(ctx.is_tile_mode_enabled());
    }

    #[test]
    fn with_tile_size_clamps_below_minimum() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(8);
        assert_eq!(ctx.tile_size(), Some(32), "values below 32 clamp up to 32");
    }

    #[test]
    fn with_tile_size_clamps_above_maximum() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(4096);
        assert_eq!(
            ctx.tile_size(),
            Some(1024),
            "values above 1024 clamp down to 1024"
        );
    }

    #[test]
    fn with_tile_extraction_max_origins_defaults_to_64() {
        let ctx = ProtectionContext::new(0.5, 42);
        assert_eq!(ctx.tile_extraction_max_origins(), 64);
    }

    #[test]
    fn with_tile_extraction_max_origins_zero_clamps_to_one() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_extraction_max_origins(0);
        assert_eq!(ctx.tile_extraction_max_origins(), 1);
    }

    #[test]
    fn tile_settings_survive_serde_roundtrip() {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_tile_size(64)
            .with_tile_extraction_max_origins(128);
        let json = serde_json::to_string(&ctx).unwrap();
        let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tile_size(), Some(64));
        assert_eq!(restored.tile_extraction_max_origins(), 128);
    }

    #[test]
    fn protection_level_byte_roundtrip() {
        let levels = [
            ProtectionLevel::Disabled,
            ProtectionLevel::Light,
            ProtectionLevel::Standard,
        ];
        for level in &levels {
            let byte = level.to_byte();
            let restored = ProtectionLevel::from_byte(byte);
            assert_eq!(restored.as_ref(), Some(level));
        }
    }

    #[test]
    fn protection_level_from_invalid_byte() {
        assert!(ProtectionLevel::from_byte(3).is_none());
        assert!(ProtectionLevel::from_byte(255).is_none());
    }

    #[test]
    fn dmi_value_iptc_property_mapping() {
        use crate::types::DmiValue;

        let allowed = DmiValue::Allowed;
        assert!(allowed.to_iptc_property().contains("DMI-Allowed"));

        let prohibited_training = DmiValue::ProhibitedAiMlTraining;
        assert!(prohibited_training
            .to_iptc_property()
            .contains("DMI-Prohibited"));

        let prohibited_gen = DmiValue::ProhibitedGenAiMlTraining;
        assert!(prohibited_gen.to_iptc_property().contains("DMI-Prohibited"));

        let prohibited_all = DmiValue::Prohibited;
        assert!(prohibited_all.to_iptc_property().contains("DMI-Prohibited"));

        let prohibited_se = DmiValue::ProhibitedExceptSearchEngineIndexing;
        assert!(prohibited_se.to_iptc_property().contains("DMI-Prohibited"));

        let prohibited_see = DmiValue::ProhibitedSeeConstraints;
        assert!(prohibited_see.to_iptc_property().contains("DMI-Prohibited"));

        let unspecified = DmiValue::Unspecified;
        assert!(unspecified.to_iptc_property().contains("DMI"));
    }
}
