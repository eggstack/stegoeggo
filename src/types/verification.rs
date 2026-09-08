use super::*;

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
