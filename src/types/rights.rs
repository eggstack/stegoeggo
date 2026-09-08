use serde::{Deserialize, Serialize};

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
