use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// IPTC Photo Metadata Standard 2023.1 - DMI (Data Mining) tags for AI exclusion.
/// These tags communicate whether content may be used for AI/ML training.
///
/// When injected into XMP metadata, the canonical PLUS controlled-vocabulary URI
/// is emitted as the `plus:DataMining` property value (e.g.
/// `http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING`).
/// `Unspecified` emits no `plus:DataMining` property.
///
/// Legacy `tdm:reserve_tdm` properties are parsed for backward compatibility
/// but are not emitted in current output.
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

    /// Returns the canonical PLUS controlled-vocabulary key identifier for this DMI value.
    ///
    /// This is the bare key portion (e.g. `"DMI-ALLOWED"`) used internally and in
    /// legacy IPTC properties. For the full XMP-ready URI, use [`plus_vocab_uri`](Self::plus_vocab_uri).
    #[must_use]
    pub fn plus_vocab_key(self) -> &'static str {
        match self {
            DmiValue::Unspecified => "DMI-UNSPECIFIED",
            DmiValue::Allowed => "DMI-ALLOWED",
            DmiValue::ProhibitedAiMlTraining => "DMI-PROHIBITED-AIMLTRAINING",
            DmiValue::ProhibitedGenAiMlTraining => "DMI-PROHIBITED-GENAIMLTRAINING",
            DmiValue::ProhibitedExceptSearchEngineIndexing => {
                "DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING"
            }
            DmiValue::Prohibited => "DMI-PROHIBITED",
            DmiValue::ProhibitedSeeConstraints => "DMI-PROHIBITED-SEECONSTRAINT",
        }
    }

    /// Returns the full canonical PLUS controlled-vocabulary URI for this DMI value,
    /// suitable for use as the `plus:DataMining` XMP attribute value.
    ///
    /// Returns `None` for `Unspecified` — no `plus:DataMining` property should be
    /// emitted for an unspecified policy.
    #[must_use]
    pub fn plus_vocab_uri(self) -> Option<&'static str> {
        match self {
            DmiValue::Unspecified => None,
            DmiValue::Allowed => Some("http://ns.useplus.org/ldf/vocab/DMI-ALLOWED"),
            DmiValue::ProhibitedAiMlTraining => {
                Some("http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING")
            }
            DmiValue::ProhibitedGenAiMlTraining => {
                Some("http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-GENAIMLTRAINING")
            }
            DmiValue::ProhibitedExceptSearchEngineIndexing => {
                Some("http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING")
            }
            DmiValue::Prohibited => Some("http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED"),
            DmiValue::ProhibitedSeeConstraints => {
                Some("http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-SEECONSTRAINT")
            }
        }
    }

    /// Parse a full canonical PLUS vocabulary URI into a `DmiValue`.
    ///
    /// Only accepts URIs beginning with [`PLUS_VOCAB_PREFIX`] followed by a
    /// recognized bare key. Rejects empty suffixes, embedded slashes, query
    /// strings, fragments, whitespace, and unknown keys. Does not accept
    /// bare keys or URIs from other origins.
    #[must_use]
    pub fn from_plus_vocab_uri(value: &str) -> Option<Self> {
        let key = value.strip_prefix(PLUS_VOCAB_PREFIX)?;
        if key.is_empty()
            || key.contains('/')
            || key.contains('?')
            || key.contains('#')
            || key.chars().any(|c| c.is_whitespace())
        {
            return None;
        }
        Self::from_plus_vocab_key_only(key)
    }

    /// Parse a bare PLUS vocabulary key into a `DmiValue`.
    ///
    /// Accepts only bare keys (e.g. `"DMI-ALLOWED"`). Rejects values
    /// containing `/`, `:`, `?`, `#`, or leading/trailing whitespace.
    /// Returns `None` for unknown or malformed values.
    #[must_use]
    pub fn from_plus_vocab_key(key: &str) -> Option<Self> {
        if key.contains('/') || key.contains(':') || key.contains('?') || key.contains('#') {
            return None;
        }
        let trimmed = key.trim();
        if trimmed != key {
            return None;
        }
        Self::from_plus_vocab_key_only(key)
    }

    fn from_plus_vocab_key_only(key: &str) -> Option<Self> {
        match key {
            "DMI-UNSPECIFIED" => Some(DmiValue::Unspecified),
            "DMI-ALLOWED" => Some(DmiValue::Allowed),
            "DMI-PROHIBITED-AIMLTRAINING" => Some(DmiValue::ProhibitedAiMlTraining),
            "DMI-PROHIBITED-GENAIMLTRAINING" => Some(DmiValue::ProhibitedGenAiMlTraining),
            "DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING" => {
                Some(DmiValue::ProhibitedExceptSearchEngineIndexing)
            }
            "DMI-PROHIBITED" => Some(DmiValue::Prohibited),
            "DMI-PROHIBITED-SEECONSTRAINT" => Some(DmiValue::ProhibitedSeeConstraints),
            _ => None,
        }
    }
}

/// PLUS LDF namespace URI for the `plus` prefix.
pub const PLUS_NAMESPACE: &str = "http://ns.useplus.org/ldf/xmp/1.0/";
/// PLUS Data Mining property name (without prefix).
pub const PLUS_DATA_MINING_PROPERTY: &str = "plus:DataMining";
/// PLUS controlled-vocabulary URI prefix for Data Mining values.
///
/// Full canonical URIs have the form `{PLUS_VOCAB_PREFIX}{key}`, e.g.
/// `http://ns.useplus.org/ldf/vocab/DMI-ALLOWED`.
pub const PLUS_VOCAB_PREFIX: &str = "http://ns.useplus.org/ldf/vocab/";

/// Evidence profile controlling the interpretation of protection warnings
/// and the default evidence posture.
///
/// An evidence profile answers the question "what evidence model is the caller
/// trying to express?" while [`ProtectionLevel`] answers "how much processing
/// should occur?"
///
/// - [`LegalNotice`](Self::LegalNotice): Standards-aligned metadata notice.
///   No MAC key required.
/// - [`LegalNoticeWithStego`](Self::LegalNoticeWithStego): Metadata notice
///   plus best-effort hidden marker. No MAC key required.
/// - [`AuthenticatedProvenance`](Self::AuthenticatedProvenance): Cryptographic
///   proof that a hidden payload was generated by a party with the configured
///   key. MAC key expected.
/// - [`Maximal`](Self::Maximal): All available legal notice and evidence channels.
#[deprecated(
    since = "0.4.0",
    note = "Use ProtectionPreset instead. EvidenceProfile only changes warning interpretation; ProtectionPreset controls actual processing behavior."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum EvidenceProfile {
    /// Standards-aligned metadata notice. No MAC key required.
    /// Missing MAC is not a warning.
    #[default]
    LegalNotice,
    /// Metadata notice plus best-effort hidden marker.
    /// No MAC key required. Stego capacity warnings are best-effort, not
    /// legal-notice failures.
    LegalNoticeWithStego,
    /// Cryptographic proof that a hidden payload was generated by a party
    /// with the configured key. MAC key expected; missing MAC is a warning.
    AuthenticatedProvenance,
    /// All available legal notice and evidence channels.
    /// MAC key used if provided; missing MAC is informational.
    Maximal,
}

#[allow(deprecated)]
impl EvidenceProfile {
    /// Returns the lowercase string representation of this evidence profile.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            EvidenceProfile::LegalNotice => "legal-notice",
            EvidenceProfile::LegalNoticeWithStego => "legal-notice-stego",
            EvidenceProfile::AuthenticatedProvenance => "authenticated-provenance",
            EvidenceProfile::Maximal => "maximal",
        }
    }
}

/// Policy for updating metadata on repeated image processing.
///
/// Controls how the protection pipeline handles existing metadata when
/// re-processing an already-protected image.
///
/// # Caveats
///
/// Ownership on PNG is detected by `tEXt`/`iTXt` keyword alone (no namespace).
/// Common keywords like `Copyright`, `Creator`, `UsageTerms`, and
/// `License` may already appear in user-authored PNGs produced by other
/// tools. With `ReplaceStegoOwned`, those chunks are removed before
/// re-injection; with `FailOnConflict`, processing aborts. JPEG and WebP
/// ownership is more precise (structured COM, namespaced XMP, RIFF chunk
/// IDs) and is not affected by this caveat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MetadataUpdatePolicy {
    /// Replace all StegoEggo-owned metadata properties. Preserve unrelated
    /// metadata (camera EXIF, color profiles, etc.). This is the default.
    #[default]
    ReplaceStegoOwned,
    /// Fail with an error if conflicting StegoEggo metadata already exists.
    FailOnConflict,
    /// Preserve existing StegoEggo metadata and only add new fields.
    /// Never overwrites existing values.
    PreserveExisting,
}

impl MetadataUpdatePolicy {
    /// Returns the lowercase string representation of this policy.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            MetadataUpdatePolicy::ReplaceStegoOwned => "replace-stego-owned",
            MetadataUpdatePolicy::FailOnConflict => "fail-on-conflict",
            MetadataUpdatePolicy::PreserveExisting => "preserve-existing",
        }
    }
}

impl std::fmt::Display for MetadataUpdatePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataUpdatePolicy::ReplaceStegoOwned => write!(f, "ReplaceStegoOwned"),
            MetadataUpdatePolicy::FailOnConflict => write!(f, "FailOnConflict"),
            MetadataUpdatePolicy::PreserveExisting => write!(f, "PreserveExisting"),
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

    /// Returns the default [`RightsPolicy`] for this legacy protection level.
    ///
    /// This is the single canonical compatibility mapping used by both the CLI
    /// and the library when translating a level into policy. `Light` maps to
    /// `Unspecified` because processing intensity must not silently create an
    /// all-data-mining legal restriction.
    #[must_use]
    pub fn default_policy(self) -> RightsPolicy {
        match self {
            ProtectionLevel::Disabled | ProtectionLevel::Light => RightsPolicy::Unspecified,
            ProtectionLevel::Standard => RightsPolicy::ProhibitedAiMlTraining,
        }
    }

    /// Converts this legacy protection level into a [`ProtectionRequest`] template.
    ///
    /// This is a compatibility adapter for callers migrating from the level-based API.
    /// The returned request has default processing options and no legal metadata —
    /// callers should chain builder methods to add notice, policy, and metadata.
    #[must_use]
    pub fn to_request(&self, notice: RightsNotice, policy: RightsPolicy) -> ProtectionRequest {
        match self {
            ProtectionLevel::Disabled => {
                // Disabled: metadata-only with no channels
                ProtectionRequest::new(
                    notice,
                    policy,
                    ProtectionChannels {
                        rights_metadata: false,
                        hidden_marker: HiddenMarkerMode::Disabled,
                        authentication: AuthenticationMode::None,
                    },
                )
            }
            ProtectionLevel::Light => {
                // Light: metadata + minimal seed stego (Q-table for JPEG, fixed-position LSB for PNG/WebP)
                ProtectionRequest::new(
                    notice,
                    policy,
                    ProtectionChannels {
                        rights_metadata: true,
                        hidden_marker: HiddenMarkerMode::SeedOnly,
                        authentication: AuthenticationMode::None,
                    },
                )
            }
            ProtectionLevel::Standard => {
                // Standard: full stego + metadata
                ProtectionRequest::new(notice, policy, ProtectionChannels::with_hidden_marker())
            }
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

/// A text value with an associated language tag.
///
/// Used for metadata fields that support localization, such as
/// `xmpRights:UsageTerms`. The default language is `"x-default"`.
///
/// # Examples
///
/// ```no_run
/// use stegoeggo::LocalizedText;
///
/// let terms = LocalizedText::new("All rights reserved.");
/// assert_eq!(terms.text(), "All rights reserved.");
/// assert_eq!(terms.lang(), "x-default");
///
/// let french = LocalizedText::with_lang("Tous droits réservés.", "fr");
/// assert_eq!(french.lang(), "fr");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalizedText {
    text: String,
    #[serde(default = "default_lang")]
    lang: String,
}

fn default_lang() -> String {
    "x-default".to_string()
}

impl LocalizedText {
    /// Creates a new `LocalizedText` with the default language (`"x-default"`).
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            lang: default_lang(),
        }
    }

    /// Creates a new `LocalizedText` with an explicit language tag.
    #[must_use]
    pub fn with_lang(text: impl Into<String>, lang: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            lang: lang.into(),
        }
    }

    /// Returns the text content.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the language tag.
    #[must_use]
    pub fn lang(&self) -> &str {
        &self.lang
    }
}

impl From<String> for LocalizedText {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for LocalizedText {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for LocalizedText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

/// Normalized rights notice produced by the pipeline before format encoding.
///
/// All format writers (PNG tEXt, JPEG COM, WebP XMP) consume the same
/// `RightsNotice` instance, ensuring semantically equivalent metadata
/// regardless of output format.
///
/// Created by [`ProtectionContext::normalize_rights_notice`].
#[derive(Debug, Clone, Default)]
pub struct RightsNotice {
    copyright_holder: Option<String>,
    contact_email: Option<String>,
    license_url: Option<String>,
    usage_terms: Option<String>,
    usage_terms_lang: Option<String>,
    creation_date: Option<String>,
    ai_constraints: Option<String>,
    web_statement_of_rights: Option<String>,
    creator: Option<String>,
    credit_line: Option<String>,
    copyright_owner: Option<String>,
    licensor_name: Option<String>,
    licensor_email: Option<String>,
    licensor_url: Option<String>,
    metadata_date: Option<String>,
    notice_applied_at: Option<String>,
    dmi: Option<DmiValue>,
    seed: Option<u64>,
}

impl RightsNotice {
    /// Creates a new empty `RightsNotice`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the copyright holder name, if set.
    #[must_use]
    pub fn copyright_holder(&self) -> Option<&str> {
        self.copyright_holder.as_deref()
    }

    /// Returns the contact email, if set.
    #[must_use]
    pub fn contact_email(&self) -> Option<&str> {
        self.contact_email.as_deref()
    }

    /// Returns the license URL, if set.
    #[must_use]
    pub fn license_url(&self) -> Option<&str> {
        self.license_url.as_deref()
    }

    /// Returns the usage terms, if set.
    #[must_use]
    pub fn usage_terms(&self) -> Option<&str> {
        self.usage_terms.as_deref()
    }

    /// Returns the usage terms language tag, if set.
    #[must_use]
    pub fn usage_terms_lang(&self) -> Option<&str> {
        self.usage_terms_lang.as_deref()
    }

    /// Returns the creation date, if set.
    #[must_use]
    pub fn creation_date(&self) -> Option<&str> {
        self.creation_date.as_deref()
    }

    /// Returns the AI constraints, if set.
    #[must_use]
    pub fn ai_constraints(&self) -> Option<&str> {
        self.ai_constraints.as_deref()
    }

    /// Returns the web statement of rights URL, if set.
    #[must_use]
    pub fn web_statement_of_rights(&self) -> Option<&str> {
        self.web_statement_of_rights.as_deref()
    }

    /// Returns the creator name, if set.
    #[must_use]
    pub fn creator(&self) -> Option<&str> {
        self.creator.as_deref()
    }

    /// Returns the credit line, if set.
    #[must_use]
    pub fn credit_line(&self) -> Option<&str> {
        self.credit_line.as_deref()
    }

    /// Returns the copyright owner name, if set.
    #[must_use]
    pub fn copyright_owner(&self) -> Option<&str> {
        self.copyright_owner.as_deref()
    }

    /// Returns the licensor name, if set.
    #[must_use]
    pub fn licensor_name(&self) -> Option<&str> {
        self.licensor_name.as_deref()
    }

    /// Returns the licensor email, if set.
    #[must_use]
    pub fn licensor_email(&self) -> Option<&str> {
        self.licensor_email.as_deref()
    }

    /// Returns the licensor URL, if set.
    #[must_use]
    pub fn licensor_url(&self) -> Option<&str> {
        self.licensor_url.as_deref()
    }

    /// Returns the metadata date, if set.
    #[must_use]
    pub fn metadata_date(&self) -> Option<&str> {
        self.metadata_date.as_deref()
    }

    /// Returns the notice-applied-at timestamp, if set.
    #[must_use]
    pub fn notice_applied_at(&self) -> Option<&str> {
        self.notice_applied_at.as_deref()
    }

    /// Returns the resolved DMI value, if any.
    #[must_use]
    pub fn dmi(&self) -> Option<DmiValue> {
        self.dmi
    }

    /// Returns the protection seed, if any.
    #[must_use]
    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    /// Returns `true` if any legal text field is set (ignores DMI).
    ///
    /// This checks only the 16 textual legal fields and ignores the `dmi`
    /// policy. Use `RightsNotice::has_notice()` (which includes DMI) when
    /// deciding whether to emit `PLUS:DataMining`.
    #[must_use]
    pub fn has_legal_content(&self) -> bool {
        self.copyright_holder.is_some()
            || self.contact_email.is_some()
            || self.license_url.is_some()
            || self.usage_terms.is_some()
            || self.usage_terms_lang.is_some()
            || self.creation_date.is_some()
            || self.ai_constraints.is_some()
            || self.web_statement_of_rights.is_some()
            || self.creator.is_some()
            || self.credit_line.is_some()
            || self.copyright_owner.is_some()
            || self.licensor_name.is_some()
            || self.licensor_email.is_some()
            || self.licensor_url.is_some()
            || self.metadata_date.is_some()
            || self.notice_applied_at.is_some()
    }

    pub(crate) fn validate(&self) -> crate::Result<()> {
        for (name, value) in [
            ("copyright_holder", self.copyright_holder.as_deref()),
            ("contact_email", self.contact_email.as_deref()),
            ("license_url", self.license_url.as_deref()),
            ("usage_terms", self.usage_terms.as_deref()),
            ("usage_terms_lang", self.usage_terms_lang.as_deref()),
            ("creation_date", self.creation_date.as_deref()),
            ("ai_constraints", self.ai_constraints.as_deref()),
            (
                "web_statement_of_rights",
                self.web_statement_of_rights.as_deref(),
            ),
            ("creator", self.creator.as_deref()),
            ("credit_line", self.credit_line.as_deref()),
            ("copyright_owner", self.copyright_owner.as_deref()),
            ("licensor_name", self.licensor_name.as_deref()),
            ("licensor_email", self.licensor_email.as_deref()),
            ("licensor_url", self.licensor_url.as_deref()),
            ("metadata_date", self.metadata_date.as_deref()),
            ("notice_applied_at", self.notice_applied_at.as_deref()),
        ] {
            if let Some(value) = value {
                if name == "usage_terms_lang" {
                    validate_language_tag(name, value)?;
                } else {
                    validate_legal_field(name, value)?;
                }
            }
        }
        Ok(())
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

    /// Sets the creator name.
    #[must_use]
    pub fn with_creator(mut self, creator: impl Into<String>) -> Self {
        self.creator = Some(creator.into());
        self
    }

    /// Sets the credit line.
    #[must_use]
    pub fn with_credit_line(mut self, line: impl Into<String>) -> Self {
        self.credit_line = Some(line.into());
        self
    }

    /// Sets the copyright owner name.
    #[must_use]
    pub fn with_copyright_owner(mut self, owner: impl Into<String>) -> Self {
        self.copyright_owner = Some(owner.into());
        self
    }

    /// Sets the licensor name.
    #[must_use]
    pub fn with_licensor_name(mut self, name: impl Into<String>) -> Self {
        self.licensor_name = Some(name.into());
        self
    }

    /// Sets the licensor email.
    #[must_use]
    pub fn with_licensor_email(mut self, email: impl Into<String>) -> Self {
        self.licensor_email = Some(email.into());
        self
    }

    /// Sets the licensor URL.
    #[must_use]
    pub fn with_licensor_url(mut self, url: impl Into<String>) -> Self {
        self.licensor_url = Some(url.into());
        self
    }

    /// Sets the metadata date.
    #[must_use]
    pub fn with_metadata_date(mut self, date: impl Into<String>) -> Self {
        self.metadata_date = Some(date.into());
        self
    }

    /// Sets the notice-applied-at timestamp.
    #[must_use]
    pub fn with_notice_applied_at(mut self, timestamp: impl Into<String>) -> Self {
        self.notice_applied_at = Some(timestamp.into());
        self
    }

    /// Sets the DMI value.
    #[must_use]
    pub fn with_dmi(mut self, dmi: DmiValue) -> Self {
        self.dmi = Some(dmi);
        self
    }

    /// Sets the seed.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Merge fields from [`LegalMetadata`] into this notice.
    ///
    /// Only sets fields that are `Some` in the legal metadata; existing
    /// notice fields are preserved when the legal metadata field is `None`.
    #[must_use]
    pub fn with_legal_metadata_fields(mut self, legal: &LegalMetadata) -> Self {
        if let Some(v) = legal.copyright_holder() {
            self.copyright_holder = Some(v.to_string());
        }
        if let Some(v) = legal.contact_email() {
            self.contact_email = Some(v.to_string());
        }
        if let Some(v) = legal.license_url() {
            self.license_url = Some(v.to_string());
        }
        if let Some(v) = legal.usage_terms() {
            self.usage_terms = Some(v.to_string());
        }
        if let Some(v) = legal.usage_terms_lang() {
            self.usage_terms_lang = Some(v.to_string());
        }
        if let Some(v) = legal.creation_date() {
            self.creation_date = Some(v.to_string());
        }
        if let Some(v) = legal.ai_constraints() {
            self.ai_constraints = Some(v.to_string());
        }
        if let Some(v) = legal.web_statement_of_rights() {
            self.web_statement_of_rights = Some(v.to_string());
        }
        if let Some(v) = legal.creator() {
            self.creator = Some(v.to_string());
        }
        if let Some(v) = legal.credit_line() {
            self.credit_line = Some(v.to_string());
        }
        if let Some(v) = legal.copyright_owner() {
            self.copyright_owner = Some(v.to_string());
        }
        if let Some(v) = legal.licensor_name() {
            self.licensor_name = Some(v.to_string());
        }
        if let Some(v) = legal.licensor_email() {
            self.licensor_email = Some(v.to_string());
        }
        if let Some(v) = legal.licensor_url() {
            self.licensor_url = Some(v.to_string());
        }
        if let Some(v) = legal.metadata_date() {
            self.metadata_date = Some(v.to_string());
        }
        if let Some(v) = legal.notice_applied_at() {
            self.notice_applied_at = Some(v.to_string());
        }
        self
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
    usage_terms_lang: Option<String>,
    creation_date: Option<String>,
    ai_constraints: Option<String>,
    web_statement_of_rights: Option<String>,
    creator: Option<String>,
    credit_line: Option<String>,
    copyright_owner: Option<String>,
    licensor_name: Option<String>,
    licensor_email: Option<String>,
    licensor_url: Option<String>,
    metadata_date: Option<String>,
    notice_applied_at: Option<String>,
}

impl LegalMetadata {
    /// Maximum byte length for any single metadata field.
    ///
    /// This limit ensures field values fit safely within JPEG segment length fields
    /// (u16: 65535 bytes max) and PNG chunk length fields (u32: 4 GiB max) after
    /// accounting for overhead bytes in the marker/chunk structure. The 8 KiB limit
    /// is generous for all practical metadata fields while preventing overflow.
    pub const MAX_FIELD_LEN: usize = 8192;

    /// Creates a new `LegalMetadata` with all fields unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates that all set fields are within the allowed byte length.
    ///
    /// Returns `Ok(())` if all fields are valid, or `Err` with a description
    /// of the first invalid field found.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Config`] if any field exceeds [`Self::MAX_FIELD_LEN`] bytes.
    ///
    /// URL fields (`license_url`, `web_statement_of_rights`, `licensor_url`)
    /// are also validated for basic syntactic correctness (scheme + authority).
    /// The `usage_terms_lang` field is validated as a BCP 47 language tag.
    pub fn validate(&self) -> crate::Result<()> {
        let check = |name: &str, val: &Option<String>| -> crate::Result<()> {
            if let Some(v) = val {
                validate_legal_field(name, v)?;
            }
            Ok(())
        };
        let check_url = |name: &str, val: &Option<String>| -> crate::Result<()> {
            if let Some(v) = val {
                Self::validate_url_syntax(name, v)?;
            }
            Ok(())
        };
        check("copyright_holder", &self.copyright_holder)?;
        check("contact_email", &self.contact_email)?;
        check("license_url", &self.license_url)?;
        check("usage_terms", &self.usage_terms)?;
        check("creation_date", &self.creation_date)?;
        check("ai_constraints", &self.ai_constraints)?;
        check("web_statement_of_rights", &self.web_statement_of_rights)?;
        check("creator", &self.creator)?;
        check("credit_line", &self.credit_line)?;
        check("copyright_owner", &self.copyright_owner)?;
        check("licensor_name", &self.licensor_name)?;
        check("licensor_email", &self.licensor_email)?;
        check("licensor_url", &self.licensor_url)?;
        check("metadata_date", &self.metadata_date)?;
        check("notice_applied_at", &self.notice_applied_at)?;
        if let Some(value) = &self.usage_terms_lang {
            validate_language_tag("usage_terms_lang", value)?;
        }

        check_url("license_url", &self.license_url)?;
        check_url("web_statement_of_rights", &self.web_statement_of_rights)?;
        check_url("licensor_url", &self.licensor_url)?;

        let check_date = |name: &str, val: &Option<String>| -> crate::Result<()> {
            if let Some(v) = val {
                Self::validate_date(name, v)?;
            }
            Ok(())
        };
        check_date("creation_date", &self.creation_date)?;
        check_date("metadata_date", &self.metadata_date)?;
        check_date("notice_applied_at", &self.notice_applied_at)?;

        Ok(())
    }

    fn validate_url_syntax(field_name: &str, url: &str) -> crate::Result<()> {
        if url.is_empty() {
            return Err(crate::Error::Config(format!(
                "URL field '{}' must not be empty",
                field_name
            )));
        }
        let Some(scheme_end) = url.find("://") else {
            return Err(crate::Error::Config(format!(
                "URL field '{}' must include a scheme (e.g., https://): {}",
                field_name, url
            )));
        };
        let after_scheme = &url[scheme_end + 3..];
        if after_scheme.is_empty() {
            return Err(crate::Error::Config(format!(
                "URL field '{}' must include an authority after the scheme: {}",
                field_name, url
            )));
        }
        Ok(())
    }

    fn validate_date(field_name: &str, value: &str) -> crate::Result<()> {
        if value.is_empty() {
            return Err(crate::Error::Config(format!(
                "Date field '{}' must not be empty",
                field_name
            )));
        }
        let valid = match value.len() {
            10 => {
                // YYYY-MM-DD
                value.as_bytes()[4] == b'-'
                    && value.as_bytes()[7] == b'-'
                    && value.bytes().enumerate().all(|(i, b)| match i {
                        4 | 7 => b == b'-',
                        _ => b.is_ascii_digit(),
                    })
            }
            20 => {
                // YYYY-MM-DDTHH:MM:SSZ
                value.as_bytes()[4] == b'-'
                    && value.as_bytes()[7] == b'-'
                    && value.as_bytes()[10] == b'T'
                    && value.as_bytes()[13] == b':'
                    && value.as_bytes()[16] == b':'
                    && value.as_bytes()[19] == b'Z'
                    && value.bytes().enumerate().all(|(i, b)| match i {
                        4 | 7 => b == b'-',
                        10 => b == b'T',
                        13 | 16 => b == b':',
                        19 => b == b'Z',
                        _ => b.is_ascii_digit(),
                    })
            }
            25 => {
                // YYYY-MM-DDTHH:MM:SS+HH:MM
                value.as_bytes()[4] == b'-'
                    && value.as_bytes()[7] == b'-'
                    && value.as_bytes()[10] == b'T'
                    && value.as_bytes()[13] == b':'
                    && value.as_bytes()[16] == b':'
                    && (value.as_bytes()[19] == b'+' || value.as_bytes()[19] == b'-')
                    && value.as_bytes()[22] == b':'
                    && value.bytes().enumerate().all(|(i, b)| match i {
                        4 | 7 => b == b'-',
                        10 => b == b'T',
                        13 | 16 | 22 => b == b':',
                        19 => b == b'+' || b == b'-',
                        _ => b.is_ascii_digit(),
                    })
            }
            _ => false,
        };
        if !valid {
            return Err(crate::Error::Config(format!(
                "Date field '{}' must be ISO 8601 format (YYYY-MM-DD, YYYY-MM-DDTHH:MM:SSZ, \
                 or YYYY-MM-DDTHH:MM:SS+HH:MM): {}",
                field_name, value
            )));
        }

        let year: u32 = value[0..4].parse().unwrap_or(0);
        let month: u32 = value[5..7].parse().unwrap_or(0);
        let day: u32 = value[8..10].parse().unwrap_or(0);

        if !(1..=9999).contains(&year) {
            return Err(crate::Error::Config(format!(
                "Date field '{}' has invalid year {}: must be 0001-9999",
                field_name, year
            )));
        }
        if !(1..=12).contains(&month) {
            return Err(crate::Error::Config(format!(
                "Date field '{}' has invalid month {}: must be 01-12",
                field_name, month
            )));
        }

        let max_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
                {
                    29
                } else {
                    28
                }
            }
            _ => unreachable!("month validated to 1..=12 above"),
        };
        if day < 1 || day > max_day {
            return Err(crate::Error::Config(format!(
                "Date field '{}' has invalid day {} for month {}: must be 01-{}",
                field_name, day, month, max_day
            )));
        }

        if value.len() >= 20 {
            let hour: u32 = value[11..13].parse().unwrap_or(0);
            let minute: u32 = value[14..16].parse().unwrap_or(0);
            let second: u32 = value[17..19].parse().unwrap_or(0);
            if hour > 23 {
                return Err(crate::Error::Config(format!(
                    "Date field '{}' has invalid hour {}: must be 00-23",
                    field_name, hour
                )));
            }
            if minute > 59 {
                return Err(crate::Error::Config(format!(
                    "Date field '{}' has invalid minute {}: must be 00-59",
                    field_name, minute
                )));
            }
            if second > 59 {
                return Err(crate::Error::Config(format!(
                    "Date field '{}' has invalid second {}: must be 00-59",
                    field_name, second
                )));
            }
        }

        if value.len() == 25 {
            let offset_hour: u32 = value[20..22].parse().unwrap_or(0);
            let offset_min: u32 = value[23..25].parse().unwrap_or(0);
            if offset_hour > 23 || offset_min > 59 {
                return Err(crate::Error::Config(format!(
                    "Date field '{}' has invalid UTC offset {}: must be +HH:MM or -HH:MM with HH<=23, MM<=59",
                    field_name, &value[19..25]
                )));
            }
        }

        Ok(())
    }

    /// Returns `true` if any legal metadata field is set (ignores DMI).
    ///
    /// Like `RightsNotice::has_legal_content`, this checks only textual
    /// fields. DMI is tracked separately via `LegalMetadata::has_content`
    /// vs `RightsNotice::has_notice`.
    #[must_use]
    pub fn has_content(&self) -> bool {
        self.copyright_holder.is_some()
            || self.contact_email.is_some()
            || self.license_url.is_some()
            || self.usage_terms.is_some()
            || self.usage_terms_lang.is_some()
            || self.creation_date.is_some()
            || self.ai_constraints.is_some()
            || self.web_statement_of_rights.is_some()
            || self.creator.is_some()
            || self.credit_line.is_some()
            || self.copyright_owner.is_some()
            || self.licensor_name.is_some()
            || self.licensor_email.is_some()
            || self.licensor_url.is_some()
            || self.metadata_date.is_some()
            || self.notice_applied_at.is_some()
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

    /// Returns the usage terms language tag, if set.
    ///
    /// Defaults to `"x-default"` when using [`LegalMetadata::with_usage_terms_localized`].
    #[must_use]
    pub fn usage_terms_lang(&self) -> Option<&str> {
        self.usage_terms_lang.as_deref()
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

    /// Returns the creator name, if set.
    #[must_use]
    pub fn creator(&self) -> Option<&str> {
        self.creator.as_deref()
    }

    /// Returns the credit line, if set.
    #[must_use]
    pub fn credit_line(&self) -> Option<&str> {
        self.credit_line.as_deref()
    }

    /// Returns the copyright owner name, if set.
    #[must_use]
    pub fn copyright_owner(&self) -> Option<&str> {
        self.copyright_owner.as_deref()
    }

    /// Returns the licensor name, if set.
    #[must_use]
    pub fn licensor_name(&self) -> Option<&str> {
        self.licensor_name.as_deref()
    }

    /// Returns the licensor email, if set.
    #[must_use]
    pub fn licensor_email(&self) -> Option<&str> {
        self.licensor_email.as_deref()
    }

    /// Returns the licensor URL, if set.
    #[must_use]
    pub fn licensor_url(&self) -> Option<&str> {
        self.licensor_url.as_deref()
    }

    /// Returns the metadata date, if set.
    #[must_use]
    pub fn metadata_date(&self) -> Option<&str> {
        self.metadata_date.as_deref()
    }

    /// Returns the notice-applied-at timestamp, if set.
    #[must_use]
    pub fn notice_applied_at(&self) -> Option<&str> {
        self.notice_applied_at.as_deref()
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

    /// Sets the usage terms with an explicit language tag.
    ///
    /// The language tag is emitted as `xml:lang` in XMP `rdf:Alt` containers.
    /// Defaults to `"x-default"` if not specified.
    #[must_use]
    pub fn with_usage_terms_localized(mut self, terms: impl Into<LocalizedText>) -> Self {
        let lt = terms.into();
        self.usage_terms = Some(lt.text().to_string());
        self.usage_terms_lang = Some(lt.lang().to_string());
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

    /// Sets the creator name.
    #[must_use]
    pub fn with_creator(mut self, creator: impl Into<String>) -> Self {
        self.creator = Some(creator.into());
        self
    }

    /// Sets the credit line.
    #[must_use]
    pub fn with_credit_line(mut self, line: impl Into<String>) -> Self {
        self.credit_line = Some(line.into());
        self
    }

    /// Sets the copyright owner name.
    #[must_use]
    pub fn with_copyright_owner(mut self, owner: impl Into<String>) -> Self {
        self.copyright_owner = Some(owner.into());
        self
    }

    /// Sets the licensor name.
    #[must_use]
    pub fn with_licensor_name(mut self, name: impl Into<String>) -> Self {
        self.licensor_name = Some(name.into());
        self
    }

    /// Sets the licensor email.
    #[must_use]
    pub fn with_licensor_email(mut self, email: impl Into<String>) -> Self {
        self.licensor_email = Some(email.into());
        self
    }

    /// Sets the licensor URL.
    #[must_use]
    pub fn with_licensor_url(mut self, url: impl Into<String>) -> Self {
        self.licensor_url = Some(url.into());
        self
    }

    /// Sets the metadata date.
    #[must_use]
    pub fn with_metadata_date(mut self, date: impl Into<String>) -> Self {
        self.metadata_date = Some(date.into());
        self
    }

    /// Sets the notice-applied-at timestamp.
    #[must_use]
    pub fn with_notice_applied_at(mut self, ts: impl Into<String>) -> Self {
        self.notice_applied_at = Some(ts.into());
        self
    }
}

fn validate_legal_field(name: &str, value: &str) -> crate::Result<()> {
    if value.len() > LegalMetadata::MAX_FIELD_LEN {
        return Err(crate::Error::Config(format!(
            "Legal metadata field '{}' exceeds maximum length of {} bytes (got {})",
            name,
            LegalMetadata::MAX_FIELD_LEN,
            value.len()
        )));
    }
    if value
        .chars()
        .any(|c| !matches!(c, '\u{9}' | '\u{A}' | '\u{D}') && (c < '\u{20}' || c == '\u{7F}'))
    {
        return Err(crate::Error::Config(format!(
            "Legal metadata field '{}' contains XML-illegal control characters",
            name
        )));
    }
    Ok(())
}

fn validate_language_tag(name: &str, value: &str) -> crate::Result<()> {
    if value.is_empty() {
        return Err(crate::Error::Config(format!(
            "Legal metadata field '{}' must be a valid BCP 47 language tag",
            name
        )));
    }

    let mut subtags = value.split('-');
    let Some(primary) = subtags.next() else {
        return Err(crate::Error::Config(format!(
            "Legal metadata field '{}' must be a valid BCP 47 language tag",
            name
        )));
    };

    let valid_subtag = |subtag: &str, min_len: usize| {
        (min_len..=8).contains(&subtag.len())
            && subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
    };

    if primary.eq_ignore_ascii_case("x") {
        if subtags.clone().next().is_none() || subtags.any(|subtag| !valid_subtag(subtag, 1)) {
            return Err(crate::Error::Config(format!(
                "Legal metadata field '{}' must be a valid BCP 47 language tag",
                name
            )));
        }
        return Ok(());
    }

    if !(2..=8).contains(&primary.len())
        || !primary.bytes().all(|byte| byte.is_ascii_alphabetic())
        || subtags.any(|subtag| !valid_subtag(subtag, 1))
    {
        return Err(crate::Error::Config(format!(
            "Legal metadata field '{}' must be a valid BCP 47 language tag",
            name
        )));
    }

    Ok(())
}

pub(crate) fn validate_stego_redundancy(redundancy: usize) -> crate::Result<()> {
    if (1..=10).contains(&redundancy) {
        Ok(())
    } else {
        Err(crate::Error::Config(format!(
            "stego redundancy must be in 1..=10, got {redundancy}"
        )))
    }
}

pub(crate) fn validate_jpeg_quality(quality: u8) -> crate::Result<()> {
    if (1..=100).contains(&quality) {
        Ok(())
    } else {
        Err(crate::Error::Config(format!(
            "JPEG quality must be in 1..=100, got {quality}"
        )))
    }
}

pub(crate) fn validate_tile_size(size: u32) -> crate::Result<()> {
    if size == 0 || (32..=1024).contains(&size) {
        Ok(())
    } else {
        Err(crate::Error::Config(format!(
            "tile size must be 0 or in 32..=1024, got {size}"
        )))
    }
}

pub(crate) fn validate_tile_extraction_max_origins(origins: u32) -> crate::Result<()> {
    if (1..=4096).contains(&origins) {
        Ok(())
    } else {
        Err(crate::Error::Config(format!(
            "maximum tile extraction origins must be in 1..=4096, got {origins}"
        )))
    }
}

pub(crate) fn validate_intensity(intensity: f32) -> crate::Result<()> {
    if !intensity.is_finite() {
        return Err(crate::Error::Config(format!(
            "intensity must be finite, got {intensity}"
        )));
    }
    if !(0.0..=1.0).contains(&intensity) {
        return Err(crate::Error::Config(format!(
            "intensity must be in 0.0..=1.0, got {intensity}"
        )));
    }
    Ok(())
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
///
/// Serialization loses the MAC key, legal metadata, resource limits, and
/// timestamp override (`#[serde(skip)]`; keys must not serialize). The
/// serialized form carries a `_config_dropped_warning` field when config was
/// present. After deserialization re-attach via `with_mac_key()`,
/// `with_legal_metadata()`, `with_config()`, or `with_resource_limits()`;
/// check `mac_key().is_none()` when authentication matters, since use without
/// a key falls back to CRC32 with a `MissingMacKey` warning.
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
}

impl Serialize for ProtectionContext {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut fields = 17;
        if self.config.is_some() {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

/// Strength of legal-notice evidence found in an image.
///
/// Evidence strength increases as more independent verification channels agree.
/// This enum is oriented toward legal deterrence, not cryptographic security.
///
/// # Interpretation
///
/// - [`NoNoticeFound`](Self::NoNoticeFound): No rights-reservation metadata detected.
/// - [`MetadataNoticeOnly`](Self::MetadataNoticeOnly): Legal-notice fields found in
///   metadata but no verified steganographic payload.
/// - [`MetadataNoticeAndBestEffortStego`](Self::MetadataNoticeAndBestEffortStego):
///   Legal-notice metadata plus a steganographic payload verified without
///   cryptographic authentication (CRC32 or unmatched MAC).
/// - [`MetadataNoticeAndAuthenticatedProvenance`](Self::MetadataNoticeAndAuthenticatedProvenance):
///   Legal-notice metadata plus a steganographic payload verified with
///   HMAC-SHA256 using the caller's MAC key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum EvidenceStrength {
    /// No rights-reservation metadata found in the image.
    NoNoticeFound,
    /// Legal-notice metadata found but no verified steganographic payload.
    MetadataNoticeOnly,
    /// Legal-notice metadata plus a non-authenticated steganographic payload.
    MetadataNoticeAndBestEffortStego,
    /// Legal-notice metadata plus a MAC-authenticated steganographic payload.
    MetadataNoticeAndAuthenticatedProvenance,
}

impl std::fmt::Display for EvidenceStrength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceStrength::NoNoticeFound => write!(f, "NoNoticeFound"),
            EvidenceStrength::MetadataNoticeOnly => write!(f, "MetadataNoticeOnly"),
            EvidenceStrength::MetadataNoticeAndBestEffortStego => {
                write!(f, "MetadataNoticeAndBestEffortStego")
            }
            EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance => {
                write!(f, "MetadataNoticeAndAuthenticatedProvenance")
            }
        }
    }
}

/// A channel through which legal-notice or steganographic evidence was detected.
///
/// Each variant corresponds to a specific metadata location or steganographic
/// technique used by the protection pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum EvidenceChannel {
    /// PNG tEXt/iTXt text chunk containing a key-value pair.
    PngText,
    /// PNG iTXt chunk containing XMP metadata.
    PngXmp,
    /// JPEG COM (comment) marker.
    JpegComment,
    /// JPEG APP1 marker containing XMP metadata.
    JpegXmp,
    /// JPEG APP13 marker containing IPTC-IIM data.
    JpegIptc,
    /// WebP RIFF chunk containing XMP metadata.
    WebPXmp,
    /// WebP RIFF chunk containing EXIF data.
    WebPExif,
    /// LSB steganographic payload embedded in pixel data.
    LsbPayload,
    /// F5-style DCT steganographic payload embedded in JPEG coefficients.
    DctPayload,
    /// Seed stored in JPEG quantization table LSBs.
    /// Reserved for future use — currently not emitted by `verify_legal_notice()`.
    QTableSeed,
}

impl std::fmt::Display for EvidenceChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceChannel::PngText => write!(f, "PngText"),
            EvidenceChannel::PngXmp => write!(f, "PngXmp"),
            EvidenceChannel::JpegComment => write!(f, "JpegComment"),
            EvidenceChannel::JpegXmp => write!(f, "JpegXmp"),
            EvidenceChannel::JpegIptc => write!(f, "JpegIptc"),
            EvidenceChannel::WebPXmp => write!(f, "WebPXmp"),
            EvidenceChannel::WebPExif => write!(f, "WebPExif"),
            EvidenceChannel::LsbPayload => write!(f, "LsbPayload"),
            EvidenceChannel::DctPayload => write!(f, "DctPayload"),
            EvidenceChannel::QTableSeed => write!(f, "QTableSeed"),
        }
    }
}

/// Classification of the source and conformance of an extracted rights signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RightsSignalKind {
    /// Canonical `plus:DataMining` property with a recognized PLUS vocabulary URI.
    CanonicalPlusDataMining,
    /// Legacy bare PLUS vocabulary key in `plus:DataMining` (backward-compatible).
    LegacyBarePlusVocabularyKey,
    /// Legacy StegoEggo `Iptc4xmpExt:DMI-*` property (v0.2 era).
    LegacyStegoEggoDmi,
    /// Legacy `tdm:reserve_tdm` property.
    LegacyTdmReservation,
    /// Unknown property or unrecognized value.
    Unknown,
}

impl std::fmt::Display for RightsSignalKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RightsSignalKind::CanonicalPlusDataMining => write!(f, "CanonicalPlusDataMining"),
            RightsSignalKind::LegacyBarePlusVocabularyKey => {
                write!(f, "LegacyBarePlusVocabularyKey")
            }
            RightsSignalKind::LegacyStegoEggoDmi => write!(f, "LegacyStegoEggoDmi"),
            RightsSignalKind::LegacyTdmReservation => write!(f, "LegacyTdmReservation"),
            RightsSignalKind::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Classification of a parsed `plus:DataMining` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParsedDmiRepresentation {
    /// Full canonical PLUS vocabulary URI.
    CanonicalUri(DmiValue),
    /// Legacy bare PLUS vocabulary key (backward-compatible).
    LegacyBareKey(DmiValue),
    /// Unknown or unrecognized value.
    Unknown,
}

/// Classify a `plus:DataMining` value into its representation form.
///
/// Canonical URIs must begin with [`PLUS_VOCAB_PREFIX`]. Bare keys are
/// recognized for backward compatibility but classified as legacy.
pub(crate) fn classify_plus_data_mining_value(value: &str) -> ParsedDmiRepresentation {
    if let Some(v) = DmiValue::from_plus_vocab_uri(value) {
        return ParsedDmiRepresentation::CanonicalUri(v);
    }
    if let Some(v) = DmiValue::from_plus_vocab_key(value) {
        return ParsedDmiRepresentation::LegacyBareKey(v);
    }
    ParsedDmiRepresentation::Unknown
}

/// Legal-notice verification report for a protected image.
///
/// This struct reports the legal-notice metadata and steganographic status
/// of an image, enabling callers to present a structured evidence report
/// without interpreting legal conclusions.
///
/// # Fields
///
/// All metadata fields are `Option<String>`: `None` means the field was not
/// found in the image. An empty string means the field was found but empty.
///
/// # Examples
///
/// ```no_run
/// let img_bytes = std::fs::read("protected.png").unwrap();
/// let report = stegoeggo::verify_legal_notice(&img_bytes, b"my-mac-key");
/// println!("Evidence strength: {}", report.evidence_strength());
/// ```
#[derive(Debug, Clone)]
pub struct NoticeVerification {
    /// Copyright holder extracted from the image metadata.
    copyright_holder: Option<String>,
    /// Creator name extracted from the image metadata.
    creator: Option<String>,
    /// Contact email extracted from the image metadata.
    contact: Option<String>,
    /// Rights URL or web statement of rights extracted from the image metadata.
    rights_url: Option<String>,
    /// Usage terms extracted from the image metadata.
    usage_terms: Option<String>,
    /// AI training constraints extracted from the image metadata.
    ai_constraints: Option<String>,
    /// DMI (Data Mining) restriction value extracted from the image metadata.
    dmi: Option<DmiValue>,
    /// Whether TDM reservation was found in XMP metadata.
    tdm_reserved: Option<bool>,
    /// Classification of the rights signal source.
    rights_signal_kind: RightsSignalKind,
    /// DMI value from canonical `plus:DataMining` property.
    canonical_dmi: Option<DmiValue>,
    /// DMI value from legacy `Iptc4xmpExt:DMI-*` property.
    legacy_dmi: Option<DmiValue>,
    /// Protection seed extracted from metadata or steganographic payload.
    protection_seed: Option<u64>,
    /// Steganographic payload verification status.
    stego_status: VerificationStatus,
    /// The extracted steganographic payload, if verified.
    stego_payload: Option<crate::StegoPayload>,
    /// Whether the steganographic payload was authenticated via HMAC.
    authenticated: bool,
    /// Overall evidence strength combining metadata and stego channels.
    evidence_strength: EvidenceStrength,
    /// Evidence channels through which data was detected.
    channels: Vec<EvidenceChannel>,
    /// License URL extracted from the image metadata.
    license_url: Option<String>,
    /// Web statement of rights URL extracted from the image metadata.
    web_statement_of_rights: Option<String>,
    /// Credit line extracted from the image metadata.
    credit_line: Option<String>,
    /// Copyright owner extracted from the image metadata.
    copyright_owner: Option<String>,
    /// Licensor name extracted from the image metadata.
    licensor_name: Option<String>,
    /// Licensor email extracted from the image metadata.
    licensor_email: Option<String>,
    /// Licensor URL extracted from the image metadata.
    licensor_url: Option<String>,
    /// Metadata date extracted from the image metadata.
    metadata_date: Option<String>,
    /// Notice-applied-at timestamp extracted from the image metadata.
    notice_applied_at: Option<String>,
}

impl NoticeVerification {
    /// Returns the copyright holder, if found.
    #[must_use]
    pub fn copyright_holder(&self) -> Option<&str> {
        self.copyright_holder.as_deref()
    }

    /// Returns the creator name, if found.
    #[must_use]
    pub fn creator(&self) -> Option<&str> {
        self.creator.as_deref()
    }

    /// Returns the contact email, if found.
    #[must_use]
    pub fn contact(&self) -> Option<&str> {
        self.contact.as_deref()
    }

    /// Returns the rights URL, if found.
    #[must_use]
    pub fn rights_url(&self) -> Option<&str> {
        self.rights_url
            .as_deref()
            .or(self.web_statement_of_rights.as_deref())
            .or(self.license_url.as_deref())
    }

    /// Returns the usage terms, if found.
    #[must_use]
    pub fn usage_terms(&self) -> Option<&str> {
        self.usage_terms.as_deref()
    }

    /// Returns the AI training constraints, if found.
    #[must_use]
    pub fn ai_constraints(&self) -> Option<&str> {
        self.ai_constraints.as_deref()
    }

    /// Returns the DMI restriction value, if found.
    #[must_use]
    pub fn dmi(&self) -> Option<DmiValue> {
        self.dmi
    }

    /// Returns whether TDM reservation was found.
    #[must_use]
    pub fn tdm_reserved(&self) -> Option<bool> {
        self.tdm_reserved
    }

    /// Returns the classification of the rights signal source.
    #[must_use]
    pub fn rights_signal_kind(&self) -> RightsSignalKind {
        self.rights_signal_kind
    }

    /// Returns the DMI value from canonical `plus:DataMining`, if found.
    #[must_use]
    pub fn canonical_dmi(&self) -> Option<DmiValue> {
        self.canonical_dmi
    }

    /// Returns the DMI value from legacy `Iptc4xmpExt:DMI-*`, if found.
    #[must_use]
    pub fn legacy_dmi(&self) -> Option<DmiValue> {
        self.legacy_dmi
    }

    /// Returns true if canonical and legacy DMI values were both found and disagree.
    #[must_use]
    pub fn has_dmi_conflict(&self) -> bool {
        if let (Some(canonical), Some(legacy)) = (self.canonical_dmi, self.legacy_dmi) {
            canonical != legacy
        } else {
            false
        }
    }

    /// Returns the protection seed, if found.
    #[must_use]
    pub fn protection_seed(&self) -> Option<u64> {
        self.protection_seed
    }

    /// Returns the steganographic verification status.
    #[must_use]
    pub fn stego_status(&self) -> VerificationStatus {
        self.stego_status
    }

    /// Returns the extracted steganographic payload, if verified.
    #[must_use]
    pub fn stego_payload(&self) -> Option<&crate::StegoPayload> {
        self.stego_payload.as_ref()
    }

    /// Returns whether the steganographic payload was authenticated.
    #[must_use]
    pub fn authenticated(&self) -> bool {
        self.authenticated
    }

    /// Returns the evidence strength.
    #[must_use]
    pub fn evidence_strength(&self) -> EvidenceStrength {
        self.evidence_strength
    }

    /// Returns the evidence channels detected.
    #[must_use]
    pub fn channels(&self) -> &[EvidenceChannel] {
        &self.channels
    }

    /// Returns the license URL, if found.
    #[must_use]
    pub fn license_url(&self) -> Option<&str> {
        self.license_url.as_deref()
    }

    /// Returns the web statement of rights URL, if found.
    #[must_use]
    pub fn web_statement_of_rights(&self) -> Option<&str> {
        self.web_statement_of_rights.as_deref()
    }

    /// Returns the credit line, if found.
    #[must_use]
    pub fn credit_line(&self) -> Option<&str> {
        self.credit_line.as_deref()
    }

    /// Returns the copyright owner, if found.
    #[must_use]
    pub fn copyright_owner(&self) -> Option<&str> {
        self.copyright_owner.as_deref()
    }

    /// Returns the licensor name, if found.
    #[must_use]
    pub fn licensor_name(&self) -> Option<&str> {
        self.licensor_name.as_deref()
    }

    /// Returns the licensor email, if found.
    #[must_use]
    pub fn licensor_email(&self) -> Option<&str> {
        self.licensor_email.as_deref()
    }

    /// Returns the licensor URL, if found.
    #[must_use]
    pub fn licensor_url(&self) -> Option<&str> {
        self.licensor_url.as_deref()
    }

    /// Returns the metadata date, if found.
    #[must_use]
    pub fn metadata_date(&self) -> Option<&str> {
        self.metadata_date.as_deref()
    }

    /// Returns the notice-applied-at timestamp, if found.
    #[must_use]
    pub fn notice_applied_at(&self) -> Option<&str> {
        self.notice_applied_at.as_deref()
    }

    /// Returns `true` if any legal-notice metadata was found.
    #[must_use]
    pub fn has_notice(&self) -> bool {
        self.copyright_holder.is_some()
            || self.creator.is_some()
            || self.contact.is_some()
            || self.rights_url.is_some()
            || self.usage_terms.is_some()
            || self.ai_constraints.is_some()
            || self.dmi.is_some()
            || self.license_url.is_some()
            || self.web_statement_of_rights.is_some()
            || self.credit_line.is_some()
            || self.copyright_owner.is_some()
            || self.licensor_name.is_some()
            || self.licensor_email.is_some()
            || self.licensor_url.is_some()
            || self.metadata_date.is_some()
            || self.notice_applied_at.is_some()
    }

    #[deprecated(since = "0.2.2", note = "use NoticeVerificationBuilder instead")]
    #[allow(clippy::too_many_arguments, dead_code)]
    pub(crate) fn new(
        copyright_holder: Option<String>,
        creator: Option<String>,
        contact: Option<String>,
        rights_url: Option<String>,
        usage_terms: Option<String>,
        ai_constraints: Option<String>,
        dmi: Option<DmiValue>,
        tdm_reserved: Option<bool>,
        rights_signal_kind: RightsSignalKind,
        canonical_dmi: Option<DmiValue>,
        legacy_dmi: Option<DmiValue>,
        protection_seed: Option<u64>,
        stego_status: VerificationStatus,
        stego_payload: Option<crate::StegoPayload>,
        authenticated: bool,
        evidence_strength: EvidenceStrength,
        channels: Vec<EvidenceChannel>,
        license_url: Option<String>,
        web_statement_of_rights: Option<String>,
        credit_line: Option<String>,
        copyright_owner: Option<String>,
        licensor_name: Option<String>,
        licensor_email: Option<String>,
        licensor_url: Option<String>,
        metadata_date: Option<String>,
        notice_applied_at: Option<String>,
    ) -> Self {
        Self {
            copyright_holder,
            creator,
            contact,
            rights_url,
            usage_terms,
            ai_constraints,
            dmi,
            tdm_reserved,
            rights_signal_kind,
            canonical_dmi,
            legacy_dmi,
            protection_seed,
            stego_status,
            stego_payload,
            authenticated,
            evidence_strength,
            channels,
            license_url,
            web_statement_of_rights,
            credit_line,
            copyright_owner,
            licensor_name,
            licensor_email,
            licensor_url,
            metadata_date,
            notice_applied_at,
        }
    }

    /// Creates a new [`NoticeVerificationBuilder`] with default values.
    #[must_use]
    pub fn builder() -> NoticeVerificationBuilder {
        NoticeVerificationBuilder::default()
    }
}

/// Builder for [`NoticeVerification`].
///
/// Construct via [`NoticeVerification::builder()`], chain setter methods, then
/// call [`build()`](NoticeVerificationBuilder::build).
#[derive(Debug, Clone)]
pub struct NoticeVerificationBuilder {
    copyright_holder: Option<String>,
    creator: Option<String>,
    contact: Option<String>,
    rights_url: Option<String>,
    usage_terms: Option<String>,
    ai_constraints: Option<String>,
    dmi: Option<DmiValue>,
    tdm_reserved: Option<bool>,
    rights_signal_kind: RightsSignalKind,
    canonical_dmi: Option<DmiValue>,
    legacy_dmi: Option<DmiValue>,
    protection_seed: Option<u64>,
    stego_status: VerificationStatus,
    stego_payload: Option<crate::StegoPayload>,
    authenticated: bool,
    evidence_strength: EvidenceStrength,
    channels: Vec<EvidenceChannel>,
    license_url: Option<String>,
    web_statement_of_rights: Option<String>,
    credit_line: Option<String>,
    copyright_owner: Option<String>,
    licensor_name: Option<String>,
    licensor_email: Option<String>,
    licensor_url: Option<String>,
    metadata_date: Option<String>,
    notice_applied_at: Option<String>,
}

impl Default for NoticeVerificationBuilder {
    fn default() -> Self {
        Self {
            copyright_holder: None,
            creator: None,
            contact: None,
            rights_url: None,
            usage_terms: None,
            ai_constraints: None,
            dmi: None,
            tdm_reserved: None,
            rights_signal_kind: RightsSignalKind::Unknown,
            canonical_dmi: None,
            legacy_dmi: None,
            protection_seed: None,
            stego_status: VerificationStatus::NotFound,
            stego_payload: None,
            authenticated: false,
            evidence_strength: EvidenceStrength::NoNoticeFound,
            channels: Vec::new(),
            license_url: None,
            web_statement_of_rights: None,
            credit_line: None,
            copyright_owner: None,
            licensor_name: None,
            licensor_email: None,
            licensor_url: None,
            metadata_date: None,
            notice_applied_at: None,
        }
    }
}

impl NoticeVerificationBuilder {
    /// Sets the copyright holder.
    #[must_use]
    pub fn copyright_holder(mut self, v: Option<String>) -> Self {
        self.copyright_holder = v;
        self
    }

    /// Sets the creator name.
    #[must_use]
    pub fn creator(mut self, v: Option<String>) -> Self {
        self.creator = v;
        self
    }

    /// Sets the contact email.
    #[must_use]
    pub fn contact(mut self, v: Option<String>) -> Self {
        self.contact = v;
        self
    }

    /// Sets the rights URL.
    #[must_use]
    pub fn rights_url(mut self, v: Option<String>) -> Self {
        self.rights_url = v;
        self
    }

    /// Sets the usage terms.
    #[must_use]
    pub fn usage_terms(mut self, v: Option<String>) -> Self {
        self.usage_terms = v;
        self
    }

    /// Sets the AI training constraints.
    #[must_use]
    pub fn ai_constraints(mut self, v: Option<String>) -> Self {
        self.ai_constraints = v;
        self
    }

    /// Sets the DMI restriction value.
    #[must_use]
    pub fn dmi(mut self, v: Option<DmiValue>) -> Self {
        self.dmi = v;
        self
    }

    /// Sets the TDM reservation flag.
    #[must_use]
    pub fn tdm_reserved(mut self, v: Option<bool>) -> Self {
        self.tdm_reserved = v;
        self
    }

    /// Sets the rights signal kind.
    #[must_use]
    pub fn rights_signal_kind(mut self, v: RightsSignalKind) -> Self {
        self.rights_signal_kind = v;
        self
    }

    /// Sets the canonical DMI value.
    #[must_use]
    pub fn canonical_dmi(mut self, v: Option<DmiValue>) -> Self {
        self.canonical_dmi = v;
        self
    }

    /// Sets the legacy DMI value.
    #[must_use]
    pub fn legacy_dmi(mut self, v: Option<DmiValue>) -> Self {
        self.legacy_dmi = v;
        self
    }

    /// Sets the protection seed.
    #[must_use]
    pub fn protection_seed(mut self, v: Option<u64>) -> Self {
        self.protection_seed = v;
        self
    }

    /// Sets the steganographic verification status.
    #[must_use]
    pub fn stego_status(mut self, v: VerificationStatus) -> Self {
        self.stego_status = v;
        self
    }

    /// Sets the extracted steganographic payload.
    #[must_use]
    pub fn stego_payload(mut self, v: Option<crate::StegoPayload>) -> Self {
        self.stego_payload = v;
        self
    }

    /// Sets whether the payload was authenticated via HMAC.
    #[must_use]
    pub fn authenticated(mut self, v: bool) -> Self {
        self.authenticated = v;
        self
    }

    /// Sets the overall evidence strength.
    #[must_use]
    pub fn evidence_strength(mut self, v: EvidenceStrength) -> Self {
        self.evidence_strength = v;
        self
    }

    /// Sets the evidence channels.
    #[must_use]
    pub fn channels(mut self, v: Vec<EvidenceChannel>) -> Self {
        self.channels = v;
        self
    }

    /// Sets the license URL.
    #[must_use]
    pub fn license_url(mut self, v: Option<String>) -> Self {
        self.license_url = v;
        self
    }

    /// Sets the web statement of rights URL.
    #[must_use]
    pub fn web_statement_of_rights(mut self, v: Option<String>) -> Self {
        self.web_statement_of_rights = v;
        self
    }

    /// Sets the credit line.
    #[must_use]
    pub fn credit_line(mut self, v: Option<String>) -> Self {
        self.credit_line = v;
        self
    }

    /// Sets the copyright owner.
    #[must_use]
    pub fn copyright_owner(mut self, v: Option<String>) -> Self {
        self.copyright_owner = v;
        self
    }

    /// Sets the licensor name.
    #[must_use]
    pub fn licensor_name(mut self, v: Option<String>) -> Self {
        self.licensor_name = v;
        self
    }

    /// Sets the licensor email.
    #[must_use]
    pub fn licensor_email(mut self, v: Option<String>) -> Self {
        self.licensor_email = v;
        self
    }

    /// Sets the licensor URL.
    #[must_use]
    pub fn licensor_url(mut self, v: Option<String>) -> Self {
        self.licensor_url = v;
        self
    }

    /// Sets the metadata date.
    #[must_use]
    pub fn metadata_date(mut self, v: Option<String>) -> Self {
        self.metadata_date = v;
        self
    }

    /// Sets the notice-applied-at timestamp.
    #[must_use]
    pub fn notice_applied_at(mut self, v: Option<String>) -> Self {
        self.notice_applied_at = v;
        self
    }

    /// Builds the [`NoticeVerification`] from the accumulated fields.
    #[must_use]
    pub fn build(self) -> NoticeVerification {
        NoticeVerification {
            copyright_holder: self.copyright_holder,
            creator: self.creator,
            contact: self.contact,
            rights_url: self.rights_url,
            usage_terms: self.usage_terms,
            ai_constraints: self.ai_constraints,
            dmi: self.dmi,
            tdm_reserved: self.tdm_reserved,
            rights_signal_kind: self.rights_signal_kind,
            canonical_dmi: self.canonical_dmi,
            legacy_dmi: self.legacy_dmi,
            protection_seed: self.protection_seed,
            stego_status: self.stego_status,
            stego_payload: self.stego_payload,
            authenticated: self.authenticated,
            evidence_strength: self.evidence_strength,
            channels: self.channels,
            license_url: self.license_url,
            web_statement_of_rights: self.web_statement_of_rights,
            credit_line: self.credit_line,
            copyright_owner: self.copyright_owner,
            licensor_name: self.licensor_name,
            licensor_email: self.licensor_email,
            licensor_url: self.licensor_url,
            metadata_date: self.metadata_date,
            notice_applied_at: self.notice_applied_at,
        }
    }
}

/// Resolved context for steganographic payload generation.
///
/// Created after output format and embed path selection to carry the actual
/// resolved flags that go into the v3 payload header. This avoids deriving
/// payload fields from the generic mutable [`ProtectionContext`] and makes
/// the relationship between resolved plan and emitted payload explicit and
/// testable.
///
/// All fields reflect the actual embed attempt — not hypothetical or
/// hard-coded generic state.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct PayloadEmissionContext {
    /// Whether rights metadata was planned for this operation.
    pub rights_metadata_planned: bool,
    /// The embedding path that will be used.
    pub embed_path: crate::stego::EmbedPath,
    /// Whether tiled embedding is selected.
    pub tiled: bool,
    /// Whether the actual output mode is progressive JPEG.
    ///
    /// Must be `false` when the embed path fell back to Q-table seed only
    /// (e.g. progressive JPEG fallback), so the payload does not falsely
    /// claim DCT embedding.
    pub progressive_output: bool,
    /// The authentication mode used.
    pub authentication: AuthenticationMode,
    /// Key ID, if present in the payload.
    pub key_id: Option<Vec<u8>>,
    /// Additional TLV extensions to embed.
    pub extensions: Vec<PayloadExtension>,
}

impl PayloadEmissionContext {
    /// Build a [`PayloadEmissionContext`] from a resolved plan and the
    /// determined embed path.
    ///
    /// `progressive_output` is set based on the plan's progressive JPEG
    /// setting. The caller must override this to `false` if the actual
    /// embed path fell back to Q-table seed only.
    #[must_use]
    #[allow(dead_code)]
    pub(crate) fn from_plan(
        plan: &crate::types::ResolvedProtectionPlan,
        embed_path: crate::stego::EmbedPath,
    ) -> Self {
        let tiled = matches!(
            embed_path,
            crate::stego::EmbedPath::LsbTiled | crate::stego::EmbedPath::DctF5Tiled
        );
        let authentication = if plan.mac_key().is_some() {
            AuthenticationMode::Hmac
        } else {
            AuthenticationMode::None
        };
        Self {
            rights_metadata_planned: plan.channels().rights_metadata,
            embed_path,
            tiled,
            progressive_output: plan.processing().progressive_jpeg,
            authentication,
            key_id: None,
            extensions: Vec::new(),
        }
    }

    /// Whether the payload is authenticated with HMAC.
    #[must_use]
    pub(crate) fn has_mac(&self) -> bool {
        self.authentication == AuthenticationMode::Hmac
    }

    /// Build a [`PayloadEmissionContext`] from a [`ProtectionContext`] for
    /// backward-compatible callers.
    ///
    /// This derives the emission context from the context's fields rather
    /// than from a resolved plan. Prefer [`from_plan`](Self::from_plan) in
    /// new code.
    #[must_use]
    pub(crate) fn from_plan_for_context(
        ctx: &ProtectionContext,
        embed_path: crate::stego::EmbedPath,
    ) -> Self {
        let tiled = matches!(
            embed_path,
            crate::stego::EmbedPath::LsbTiled | crate::stego::EmbedPath::DctF5Tiled
        );
        let authentication = if ctx.mac_key().is_some() {
            AuthenticationMode::Hmac
        } else {
            AuthenticationMode::None
        };
        Self {
            rights_metadata_planned: ctx.effective_metadata_injection(),
            embed_path,
            tiled,
            progressive_output: ctx.progressive_jpeg(),
            authentication,
            key_id: None,
            extensions: Vec::new(),
        }
    }
}

/// An additional TLV extension embedded in the v3 payload.
///
/// Currently unused — reserved for future key-ID and extension support.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct PayloadExtension {
    /// Extension type identifier.
    pub extension_type: u16,
    /// Extension value bytes.
    pub value: Vec<u8>,
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
    /// All available channels. MAC used if provided.
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

#[cfg(test)]
#[allow(deprecated)]
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
    fn invalid_numeric_context_values_are_preserved_for_validation() {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_stego_redundancy(0)
            .with_jpeg_quality(0);
        assert_eq!(ctx.stego_redundancy_field(), Some(0));
        assert_eq!(ctx.jpeg_quality(), 0);
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn effective_redundancy_clamps_out_of_range_values() {
        let ctx = ProtectionContext::new(0.5, 42).with_stego_redundancy(0);
        assert_eq!(ctx.stego_redundancy(), 1);
        assert!(ctx.validate().is_err());
        let ctx = ProtectionContext::new(0.5, 42).with_stego_redundancy(99);
        assert_eq!(ctx.stego_redundancy(), 10);
        assert!(ctx.validate().is_err());
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
    fn with_tile_size_preserves_below_minimum_for_validation() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(8);
        assert_eq!(ctx.tile_size(), Some(8));
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn with_tile_size_preserves_above_maximum_for_validation() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(4096);
        assert_eq!(ctx.tile_size(), Some(4096));
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn with_tile_extraction_max_origins_defaults_to_64() {
        let ctx = ProtectionContext::new(0.5, 42);
        assert_eq!(ctx.tile_extraction_max_origins(), 64);
    }

    #[test]
    fn with_tile_extraction_max_origins_preserves_zero_for_validation() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_extraction_max_origins(0);
        assert_eq!(ctx.tile_extraction_max_origins(), 0);
        assert!(ctx.validate().is_err());
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

    #[test]
    fn evidence_profile_default_is_legal_notice() {
        let ctx = ProtectionContext::new(0.5, 42);
        assert_eq!(ctx.evidence_profile(), EvidenceProfile::LegalNotice);
    }

    #[test]
    fn with_evidence_profile_sets_and_retrieves() {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_evidence_profile(EvidenceProfile::AuthenticatedProvenance);
        assert_eq!(
            ctx.evidence_profile(),
            EvidenceProfile::AuthenticatedProvenance
        );
    }

    #[test]
    fn evidence_profile_serialization_roundtrip() {
        let profiles = [
            EvidenceProfile::LegalNotice,
            EvidenceProfile::LegalNoticeWithStego,
            EvidenceProfile::AuthenticatedProvenance,
            EvidenceProfile::Maximal,
        ];
        for profile in &profiles {
            let json = serde_json::to_string(profile).unwrap();
            let restored: EvidenceProfile = serde_json::from_str(&json).unwrap();
            assert_eq!(&restored, profile);
        }
    }

    #[test]
    fn evidence_profile_as_str() {
        assert_eq!(EvidenceProfile::LegalNotice.as_str(), "legal-notice");
        assert_eq!(
            EvidenceProfile::LegalNoticeWithStego.as_str(),
            "legal-notice-stego"
        );
        assert_eq!(
            EvidenceProfile::AuthenticatedProvenance.as_str(),
            "authenticated-provenance"
        );
        assert_eq!(EvidenceProfile::Maximal.as_str(), "maximal");
    }

    #[test]
    fn evidence_profile_serde_roundtrip_in_context() {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_evidence_profile(EvidenceProfile::AuthenticatedProvenance);
        let json = serde_json::to_string(&ctx).unwrap();
        let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(
            restored.evidence_profile(),
            EvidenceProfile::AuthenticatedProvenance
        );
    }

    #[test]
    fn evidence_profile_default_context_backward_compatible() {
        let ctx = ProtectionContext::new(0.5, 42);
        let json = serde_json::to_string(&ctx).unwrap();
        let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.evidence_profile(), EvidenceProfile::LegalNotice);
        assert_eq!(restored.intensity(), 0.5);
        assert_eq!(restored.seed(), 42);
    }

    #[test]
    fn helper_constructors_set_correct_profile() {
        assert_eq!(
            ProtectionContext::legal_notice().evidence_profile(),
            EvidenceProfile::LegalNotice
        );
        assert_eq!(
            ProtectionContext::legal_notice_with_stego().evidence_profile(),
            EvidenceProfile::LegalNoticeWithStego
        );
        assert_eq!(
            ProtectionContext::authenticated_provenance().evidence_profile(),
            EvidenceProfile::AuthenticatedProvenance
        );
        assert_eq!(
            ProtectionContext::maximal().evidence_profile(),
            EvidenceProfile::Maximal
        );
    }

    #[test]
    fn warning_category_mapping() {
        assert_eq!(
            ProtectionWarning::MissingMacKey.category(),
            WarningCategory::AuthenticatedProvenance
        );
        assert_eq!(
            ProtectionWarning::MetadataInjectionDisabled.category(),
            WarningCategory::LegalNotice
        );
        assert_eq!(
            ProtectionWarning::ProgressiveJpegFallback.category(),
            WarningCategory::FormatFragility
        );
        assert_eq!(
            ProtectionWarning::JpegReencodeFragile.category(),
            WarningCategory::FormatFragility
        );
        assert_eq!(
            ProtectionWarning::LsbCapacitySkipped.category(),
            WarningCategory::BestEffortStego
        );
        assert_eq!(
            ProtectionWarning::DctCapacityInsufficient.category(),
            WarningCategory::BestEffortStego
        );
    }

    #[test]
    fn missing_mac_key_severity_by_profile() {
        let w = ProtectionWarning::MissingMacKey;
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::AuthenticatedProvenance),
            WarningSeverity::Warning
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::Maximal),
            WarningSeverity::Warning
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::LegalNotice),
            WarningSeverity::Info
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::LegalNoticeWithStego),
            WarningSeverity::Info
        );
    }

    #[test]
    fn metadata_injection_disabled_severity_by_profile() {
        let w = ProtectionWarning::MetadataInjectionDisabled;
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::LegalNotice),
            WarningSeverity::Error
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::LegalNoticeWithStego),
            WarningSeverity::Error
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::AuthenticatedProvenance),
            WarningSeverity::Warning
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::Maximal),
            WarningSeverity::Warning
        );
    }

    #[test]
    fn format_fragility_severity_is_always_warning() {
        for w in [
            ProtectionWarning::ProgressiveJpegFallback,
            ProtectionWarning::JpegReencodeFragile,
        ] {
            for profile in [
                EvidenceProfile::LegalNotice,
                EvidenceProfile::LegalNoticeWithStego,
                EvidenceProfile::AuthenticatedProvenance,
                EvidenceProfile::Maximal,
            ] {
                assert_eq!(
                    w.severity_for_profile(profile),
                    WarningSeverity::Warning,
                    "{:?} should be Warning for {:?}",
                    w,
                    profile
                );
            }
        }
    }

    #[test]
    fn stego_capacity_severity_by_profile() {
        for w in [
            ProtectionWarning::LsbCapacitySkipped,
            ProtectionWarning::DctCapacityInsufficient,
        ] {
            assert_eq!(
                w.severity_for_profile(EvidenceProfile::LegalNotice),
                WarningSeverity::Info,
                "{:?} should be Info for LegalNotice",
                w
            );
            assert_eq!(
                w.severity_for_profile(EvidenceProfile::LegalNoticeWithStego),
                WarningSeverity::Warning,
                "{:?} should be Warning for LegalNoticeWithStego",
                w
            );
            assert_eq!(
                w.severity_for_profile(EvidenceProfile::AuthenticatedProvenance),
                WarningSeverity::Warning,
                "{:?} should be Warning for AuthenticatedProvenance",
                w
            );
            assert_eq!(
                w.severity_for_profile(EvidenceProfile::Maximal),
                WarningSeverity::Warning,
                "{:?} should be Warning for Maximal",
                w
            );
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod plus_mapping_tests {
    use super::*;

    #[test]
    fn all_variants_have_plus_vocab_key() {
        let variants = [
            DmiValue::Unspecified,
            DmiValue::Allowed,
            DmiValue::ProhibitedAiMlTraining,
            DmiValue::ProhibitedGenAiMlTraining,
            DmiValue::ProhibitedExceptSearchEngineIndexing,
            DmiValue::Prohibited,
            DmiValue::ProhibitedSeeConstraints,
        ];
        for v in variants {
            let key = v.plus_vocab_key();
            assert!(key.starts_with("DMI-"), "key must start with DMI-: {key}");
            assert_eq!(DmiValue::from_plus_vocab_key(key), Some(v));
        }
    }

    #[test]
    fn from_plus_vocab_key_rejects_unknown() {
        assert_eq!(DmiValue::from_plus_vocab_key("DMI-UNKNOWN"), None);
        assert_eq!(DmiValue::from_plus_vocab_key(""), None);
        assert_eq!(DmiValue::from_plus_vocab_key("Prohibited"), None);
    }

    #[test]
    fn plus_vocab_keys_match_exiftool() {
        assert_eq!(
            DmiValue::ProhibitedSeeConstraints.plus_vocab_key(),
            "DMI-PROHIBITED-SEECONSTRAINT"
        );
        assert_eq!(
            DmiValue::ProhibitedAiMlTraining.plus_vocab_key(),
            "DMI-PROHIBITED-AIMLTRAINING"
        );
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod rights_policy_tests {
    use super::*;

    #[test]
    fn unspecified_to_dmi_returns_none() {
        assert_eq!(RightsPolicy::Unspecified.to_dmi_value(), None);
    }

    #[test]
    fn allowed_to_dmi() {
        assert_eq!(
            RightsPolicy::Allowed.to_dmi_value(),
            Some(DmiValue::Allowed)
        );
    }

    #[test]
    fn prohibited_ai_ml_training_to_dmi() {
        assert_eq!(
            RightsPolicy::ProhibitedAiMlTraining.to_dmi_value(),
            Some(DmiValue::ProhibitedAiMlTraining)
        );
    }

    #[test]
    fn prohibited_generative_ai_training_to_dmi() {
        assert_eq!(
            RightsPolicy::ProhibitedGenerativeAiTraining.to_dmi_value(),
            Some(DmiValue::ProhibitedGenAiMlTraining)
        );
    }

    #[test]
    fn prohibited_except_search_indexing_to_dmi() {
        assert_eq!(
            RightsPolicy::ProhibitedExceptSearchIndexing.to_dmi_value(),
            Some(DmiValue::ProhibitedExceptSearchEngineIndexing)
        );
    }

    #[test]
    fn prohibited_all_data_mining_to_dmi() {
        assert_eq!(
            RightsPolicy::ProhibitedAllDataMining.to_dmi_value(),
            Some(DmiValue::Prohibited)
        );
    }

    #[test]
    fn prohibited_see_constraints_to_dmi() {
        assert_eq!(
            RightsPolicy::ProhibitedSeeConstraints.to_dmi_value(),
            Some(DmiValue::ProhibitedSeeConstraints)
        );
    }

    #[test]
    fn from_dmi_unspecified() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::Unspecified),
            RightsPolicy::Unspecified
        );
    }

    #[test]
    fn from_dmi_allowed() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::Allowed),
            RightsPolicy::Allowed
        );
    }

    #[test]
    fn from_dmi_prohibited_ai_ml_training() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::ProhibitedAiMlTraining),
            RightsPolicy::ProhibitedAiMlTraining
        );
    }

    #[test]
    fn from_dmi_prohibited_gen_ai_ml_training() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::ProhibitedGenAiMlTraining),
            RightsPolicy::ProhibitedGenerativeAiTraining
        );
    }

    #[test]
    fn from_dmi_prohibited_except_search_engine_indexing() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::ProhibitedExceptSearchEngineIndexing),
            RightsPolicy::ProhibitedExceptSearchIndexing
        );
    }

    #[test]
    fn from_dmi_prohibited() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::Prohibited),
            RightsPolicy::ProhibitedAllDataMining
        );
    }

    #[test]
    fn from_dmi_prohibited_see_constraints() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::ProhibitedSeeConstraints),
            RightsPolicy::ProhibitedSeeConstraints
        );
    }

    #[test]
    fn roundtrip_to_dmi_from_dmi() {
        let policies = [
            RightsPolicy::Allowed,
            RightsPolicy::ProhibitedAiMlTraining,
            RightsPolicy::ProhibitedGenerativeAiTraining,
            RightsPolicy::ProhibitedExceptSearchIndexing,
            RightsPolicy::ProhibitedAllDataMining,
            RightsPolicy::ProhibitedSeeConstraints,
        ];
        for policy in policies {
            let dmi = policy.to_dmi_value().unwrap();
            let roundtripped = RightsPolicy::from_dmi_value(dmi);
            assert_eq!(roundtripped, policy);
        }
    }

    #[test]
    fn roundtrip_from_dmi_to_dmi() {
        let dmi_values = [
            DmiValue::Allowed,
            DmiValue::ProhibitedAiMlTraining,
            DmiValue::ProhibitedGenAiMlTraining,
            DmiValue::ProhibitedExceptSearchEngineIndexing,
            DmiValue::Prohibited,
            DmiValue::ProhibitedSeeConstraints,
        ];
        for dmi in dmi_values {
            let policy = RightsPolicy::from_dmi_value(dmi);
            let roundtripped = policy.to_dmi_value().unwrap();
            assert_eq!(roundtripped, dmi);
        }
    }

    #[test]
    fn from_trait_matches_function() {
        for dmi in [
            DmiValue::Allowed,
            DmiValue::ProhibitedAiMlTraining,
            DmiValue::ProhibitedGenAiMlTraining,
        ] {
            let via_from: RightsPolicy = dmi.into();
            let via_fn = RightsPolicy::from_dmi_value(dmi);
            assert_eq!(via_from, via_fn);
        }
    }

    #[test]
    fn into_trait_matches_to_dmi_value() {
        for policy in [
            RightsPolicy::Allowed,
            RightsPolicy::ProhibitedAiMlTraining,
            RightsPolicy::ProhibitedGenerativeAiTraining,
        ] {
            let via_into: DmiValue = policy.into();
            let via_fn = policy.to_dmi_value().unwrap();
            assert_eq!(via_into, via_fn);
        }
    }

    #[test]
    fn requires_constraints_only_for_see_constraints() {
        assert!(!RightsPolicy::Allowed.requires_constraints());
        assert!(!RightsPolicy::ProhibitedAiMlTraining.requires_constraints());
        assert!(RightsPolicy::ProhibitedSeeConstraints.requires_constraints());
    }

    #[test]
    fn as_str_matches_variant_name() {
        assert_eq!(RightsPolicy::Unspecified.as_str(), "Unspecified");
        assert_eq!(RightsPolicy::Allowed.as_str(), "Allowed");
        assert_eq!(
            RightsPolicy::ProhibitedAllDataMining.as_str(),
            "ProhibitedAllDataMining"
        );
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod protection_preset_tests {
    use super::*;

    #[test]
    fn legal_notice_expands_to_metadata_only() {
        let channels = ProtectionPreset::LegalNotice.to_channels();
        assert!(channels.rights_metadata);
        assert_eq!(channels.hidden_marker, HiddenMarkerMode::Disabled);
        assert_eq!(channels.authentication, AuthenticationMode::None);
        assert!(!channels.has_stego());
    }

    #[test]
    fn legal_notice_with_stego_expands_correctly() {
        let channels = ProtectionPreset::LegalNoticeWithStego.to_channels();
        assert!(channels.rights_metadata);
        assert_eq!(channels.hidden_marker, HiddenMarkerMode::BestEffort);
        assert_eq!(channels.authentication, AuthenticationMode::None);
        assert!(channels.has_stego());
    }

    #[test]
    fn authenticated_provenance_expands_correctly() {
        let channels = ProtectionPreset::AuthenticatedProvenance.to_channels();
        assert!(channels.rights_metadata);
        assert_eq!(channels.hidden_marker, HiddenMarkerMode::BestEffort);
        assert_eq!(channels.authentication, AuthenticationMode::Hmac);
        assert!(channels.has_stego());
    }

    #[test]
    fn maximal_expands_correctly() {
        let channels = ProtectionPreset::Maximal.to_channels();
        assert!(channels.rights_metadata);
        assert_eq!(channels.hidden_marker, HiddenMarkerMode::BestEffort);
        assert_eq!(channels.authentication, AuthenticationMode::Hmac);
        assert!(channels.has_stego());
    }

    #[test]
    fn requires_mac_key_only_for_authenticated_presets() {
        assert!(!ProtectionPreset::LegalNotice.requires_mac_key());
        assert!(!ProtectionPreset::LegalNoticeWithStego.requires_mac_key());
        assert!(ProtectionPreset::AuthenticatedProvenance.requires_mac_key());
        assert!(ProtectionPreset::Maximal.requires_mac_key());
    }

    #[test]
    fn as_str_returns_lowercase() {
        assert_eq!(ProtectionPreset::LegalNotice.as_str(), "legal-notice");
        assert_eq!(
            ProtectionPreset::LegalNoticeWithStego.as_str(),
            "legal-notice-stego"
        );
        assert_eq!(
            ProtectionPreset::AuthenticatedProvenance.as_str(),
            "authenticated-provenance"
        );
        assert_eq!(ProtectionPreset::Maximal.as_str(), "maximal");
    }

    #[test]
    fn from_preset_uses_preset_channels() {
        let notice = RightsNotice::new();
        let request = ProtectionRequest::from_preset(
            ProtectionPreset::LegalNotice,
            notice,
            RightsPolicy::Allowed,
        );
        assert!(request.channels().rights_metadata);
        assert_eq!(request.channels().hidden_marker, HiddenMarkerMode::Disabled);
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod url_validation_tests {
    use super::*;

    #[test]
    fn valid_https_url_passes() {
        let meta = LegalMetadata::new().with_license_url("https://example.com/license");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn valid_http_url_passes() {
        let meta = LegalMetadata::new().with_license_url("http://example.com/license");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn valid_ftp_url_passes() {
        let meta = LegalMetadata::new().with_license_url("ftp://files.example.com/doc");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn missing_scheme_fails() {
        let meta = LegalMetadata::new().with_license_url("example.com/license");
        let err = meta.validate().unwrap_err();
        assert!(
            err.to_string().contains("must include a scheme"),
            "Expected scheme error, got: {}",
            err
        );
    }

    #[test]
    fn empty_url_fails() {
        let meta = LegalMetadata::new().with_license_url("");
        let err = meta.validate().unwrap_err();
        assert!(
            err.to_string().contains("must not be empty"),
            "Expected empty error, got: {}",
            err
        );
    }

    #[test]
    fn scheme_only_fails() {
        let meta = LegalMetadata::new().with_license_url("https://");
        let err = meta.validate().unwrap_err();
        assert!(
            err.to_string().contains("must include an authority"),
            "Expected authority error, got: {}",
            err
        );
    }

    #[test]
    fn web_statement_validates() {
        let meta = LegalMetadata::new().with_web_statement_of_rights("not-a-url");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("web_statement_of_rights"));
    }

    #[test]
    fn licensor_url_validates() {
        let meta = LegalMetadata::new().with_licensor_url("missing-scheme");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("licensor_url"));
    }

    #[test]
    fn non_url_fields_not_affected() {
        let meta = LegalMetadata::new()
            .with_copyright_holder("Test")
            .with_creator("Author");
        assert!(meta.validate().is_ok());
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod localized_text_tests {
    use super::*;

    #[test]
    fn new_defaults_to_x_default() {
        let lt = LocalizedText::new("All rights reserved");
        assert_eq!(lt.text(), "All rights reserved");
        assert_eq!(lt.lang(), "x-default");
    }

    #[test]
    fn with_lang_sets_language() {
        let lt = LocalizedText::with_lang("Tous droits réservés.", "fr");
        assert_eq!(lt.text(), "Tous droits réservés.");
        assert_eq!(lt.lang(), "fr");
    }

    #[test]
    fn from_string_uses_default_lang() {
        let lt: LocalizedText = "test".into();
        assert_eq!(lt.lang(), "x-default");
    }

    #[test]
    fn display_returns_text() {
        let lt = LocalizedText::new("hello");
        assert_eq!(format!("{}", lt), "hello");
    }

    #[test]
    fn usage_terms_localized_sets_both_fields() {
        let meta = LegalMetadata::new()
            .with_usage_terms_localized(LocalizedText::with_lang("Tous droits réservés.", "fr"));
        assert_eq!(meta.usage_terms(), Some("Tous droits réservés."));
        assert_eq!(meta.usage_terms_lang(), Some("fr"));
    }

    #[test]
    fn usage_terms_localized_from_string_uses_default_lang() {
        let meta = LegalMetadata::new().with_usage_terms_localized("All rights reserved");
        assert_eq!(meta.usage_terms(), Some("All rights reserved"));
        assert_eq!(meta.usage_terms_lang(), Some("x-default"));
    }

    #[test]
    fn usage_terms_plain_has_no_lang() {
        let meta = LegalMetadata::new().with_usage_terms("All rights reserved");
        assert_eq!(meta.usage_terms(), Some("All rights reserved"));
        assert_eq!(meta.usage_terms_lang(), None);
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod date_validation_tests {
    use super::*;

    #[test]
    fn valid_date_only() {
        let meta = LegalMetadata::new().with_creation_date("2024-01-15");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn valid_datetime_utc() {
        let meta = LegalMetadata::new().with_notice_applied_at("2024-01-15T12:30:45Z");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn valid_datetime_offset() {
        let meta = LegalMetadata::new().with_metadata_date("2024-01-15T12:30:45+05:30");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn valid_datetime_negative_offset() {
        let meta = LegalMetadata::new().with_creation_date("2024-01-15T12:30:45-08:00");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn invalid_date_too_short() {
        let meta = LegalMetadata::new().with_creation_date("2024-01");
        let err = meta.validate().unwrap_err();
        assert!(
            err.to_string().contains("ISO 8601"),
            "Expected ISO 8601 error, got: {}",
            err
        );
    }

    #[test]
    fn invalid_date_wrong_separator() {
        let meta = LegalMetadata::new().with_creation_date("2024/01/15");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("ISO 8601"));
    }

    #[test]
    fn invalid_datetime_missing_t() {
        let meta = LegalMetadata::new().with_notice_applied_at("2024-01-15 12:30:45Z");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("ISO 8601"));
    }

    #[test]
    fn empty_date_fails() {
        let meta = LegalMetadata::new().with_creation_date("");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn date_only_with_time_component_fails() {
        let meta = LegalMetadata::new().with_creation_date("2024-01-15T");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("ISO 8601"));
    }

    #[test]
    fn all_date_fields_valid() {
        let meta = LegalMetadata::new()
            .with_creation_date("2024-01-15")
            .with_metadata_date("2024-01-15T12:30:45Z")
            .with_notice_applied_at("2024-01-15T12:30:45+05:30");
        assert!(meta.validate().is_ok());
    }
}
