use super::*;
use serde::{Deserialize, Serialize};

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
    pub(crate) copyright_holder: Option<String>,
    pub(crate) contact_email: Option<String>,
    pub(crate) license_url: Option<String>,
    pub(crate) usage_terms: Option<String>,
    pub(crate) usage_terms_lang: Option<String>,
    pub(crate) creation_date: Option<String>,
    pub(crate) ai_constraints: Option<String>,
    pub(crate) web_statement_of_rights: Option<String>,
    pub(crate) creator: Option<String>,
    pub(crate) credit_line: Option<String>,
    pub(crate) copyright_owner: Option<String>,
    pub(crate) licensor_name: Option<String>,
    pub(crate) licensor_email: Option<String>,
    pub(crate) licensor_url: Option<String>,
    pub(crate) metadata_date: Option<String>,
    pub(crate) notice_applied_at: Option<String>,
    pub(crate) dmi: Option<DmiValue>,
    pub(crate) seed: Option<u64>,
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
    /// deciding whether to emit `PLUS:DataMining`. A bare
    /// `usage_terms_lang` without `usage_terms` does not count (a language
    /// tag alone is a qualifier, not content).
    #[must_use]
    pub fn has_legal_content(&self) -> bool {
        self.copyright_holder.is_some()
            || self.contact_email.is_some()
            || self.license_url.is_some()
            || self.usage_terms.is_some()
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

    fn parse_date_component(field_name: &str, what: &str, text: &str) -> crate::Result<u32> {
        text.parse().map_err(|_| {
            crate::Error::Config(format!(
                "Date field '{}' has invalid {} '{}': must be numeric",
                field_name, what, text
            ))
        })
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

        let year: u32 = Self::parse_date_component(field_name, "year", &value[0..4])?;
        let month: u32 = Self::parse_date_component(field_name, "month", &value[5..7])?;
        let day: u32 = Self::parse_date_component(field_name, "day", &value[8..10])?;

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
            _ => {
                return Err(crate::Error::Config(format!(
                    "Date field '{}' has invalid month {}: must be 01-12",
                    field_name, month
                )));
            }
        };
        if day < 1 || day > max_day {
            return Err(crate::Error::Config(format!(
                "Date field '{}' has invalid day {} for month {}: must be 01-{}",
                field_name, day, month, max_day
            )));
        }

        if value.len() >= 20 {
            let hour: u32 = Self::parse_date_component(field_name, "hour", &value[11..13])?;
            let minute: u32 = Self::parse_date_component(field_name, "minute", &value[14..16])?;
            let second: u32 = Self::parse_date_component(field_name, "second", &value[17..19])?;
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
            let offset_hour: u32 =
                Self::parse_date_component(field_name, "UTC offset hour", &value[20..22])?;
            let offset_min: u32 =
                Self::parse_date_component(field_name, "UTC offset minute", &value[23..25])?;
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
    /// vs `RightsNotice::has_notice`. A bare `usage_terms_lang` without
    /// `usage_terms` does not count.
    #[must_use]
    pub fn has_content(&self) -> bool {
        self.copyright_holder.is_some()
            || self.contact_email.is_some()
            || self.license_url.is_some()
            || self.usage_terms.is_some()
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
