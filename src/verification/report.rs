use crate::types::{EvidenceChannel, EvidenceStrength, VerificationStatus};
use serde::{Deserialize, Serialize};

/// Source from which a verification field was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldSource {
    /// Extracted from XMP metadata.
    Xmp,
    /// Extracted from legacy (non-XMP) metadata.
    Legacy,
    /// Extracted from an embedded V1 payload.
    EmbeddedPayloadV1,
    /// Extracted from an embedded V2 payload.
    EmbeddedPayloadV2,
    /// Extracted from an embedded V3 payload.
    EmbeddedPayloadV3,
    /// Extracted from a detached manifest.
    DetachedManifest,
    /// Supplied directly by the caller.
    CallerSupplied,
    /// Computed or derived.
    Computed,
}

/// Verification result for rights and legal-notice metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightsVerification {
    found: bool,
    copyright_holder: Option<String>,
    creator: Option<String>,
    contact: Option<String>,
    rights_url: Option<String>,
    usage_terms: Option<String>,
    ai_constraints: Option<String>,
    dmi: Option<u8>,
    source: FieldSource,
    channels: Vec<EvidenceChannel>,
}

impl RightsVerification {
    /// Whether any rights metadata was found.
    #[must_use]
    pub fn found(&self) -> bool {
        self.found
    }

    /// Copyright holder text, if present.
    #[must_use]
    pub fn copyright_holder(&self) -> Option<&str> {
        self.copyright_holder.as_deref()
    }

    /// Creator text, if present.
    #[must_use]
    pub fn creator(&self) -> Option<&str> {
        self.creator.as_deref()
    }

    /// Contact information, if present.
    #[must_use]
    pub fn contact(&self) -> Option<&str> {
        self.contact.as_deref()
    }

    /// Rights URL, if present.
    #[must_use]
    pub fn rights_url(&self) -> Option<&str> {
        self.rights_url.as_deref()
    }

    /// Usage terms text, if present.
    #[must_use]
    pub fn usage_terms(&self) -> Option<&str> {
        self.usage_terms.as_deref()
    }

    /// AI constraints text, if present.
    #[must_use]
    pub fn ai_constraints(&self) -> Option<&str> {
        self.ai_constraints.as_deref()
    }

    /// Data-mining policy byte, if present.
    #[must_use]
    pub fn dmi(&self) -> Option<u8> {
        self.dmi
    }

    /// Source from which this verification was obtained.
    #[must_use]
    pub fn source(&self) -> FieldSource {
        self.source
    }

    /// Evidence channels that contributed to this verification.
    #[must_use]
    pub fn channels(&self) -> &[EvidenceChannel] {
        &self.channels
    }

    /// Create a new builder for `RightsVerification`.
    pub fn builder() -> RightsVerificationBuilder {
        RightsVerificationBuilder {
            found: false,
            copyright_holder: None,
            creator: None,
            contact: None,
            rights_url: None,
            usage_terms: None,
            ai_constraints: None,
            dmi: None,
            source: FieldSource::Xmp,
            channels: Vec::new(),
        }
    }
}

/// Builder for [`RightsVerification`].
pub struct RightsVerificationBuilder {
    found: bool,
    copyright_holder: Option<String>,
    creator: Option<String>,
    contact: Option<String>,
    rights_url: Option<String>,
    usage_terms: Option<String>,
    ai_constraints: Option<String>,
    dmi: Option<u8>,
    source: FieldSource,
    channels: Vec<EvidenceChannel>,
}

impl RightsVerificationBuilder {
    /// Set whether rights metadata was found.
    #[must_use]
    pub fn found(mut self, found: bool) -> Self {
        self.found = found;
        self
    }

    /// Set the copyright holder text.
    #[must_use]
    pub fn copyright_holder(mut self, val: impl Into<String>) -> Self {
        self.copyright_holder = Some(val.into());
        self
    }

    /// Set the creator text.
    #[must_use]
    pub fn creator(mut self, val: impl Into<String>) -> Self {
        self.creator = Some(val.into());
        self
    }

    /// Set the contact information.
    #[must_use]
    pub fn contact(mut self, val: impl Into<String>) -> Self {
        self.contact = Some(val.into());
        self
    }

    /// Set the rights URL.
    #[must_use]
    pub fn rights_url(mut self, val: impl Into<String>) -> Self {
        self.rights_url = Some(val.into());
        self
    }

    /// Set the usage terms text.
    #[must_use]
    pub fn usage_terms(mut self, val: impl Into<String>) -> Self {
        self.usage_terms = Some(val.into());
        self
    }

    /// Set the AI constraints text.
    #[must_use]
    pub fn ai_constraints(mut self, val: impl Into<String>) -> Self {
        self.ai_constraints = Some(val.into());
        self
    }

    /// Set the data-mining policy byte.
    #[must_use]
    pub fn dmi(mut self, val: u8) -> Self {
        self.dmi = Some(val);
        self
    }

    /// Set the source from which this verification was obtained.
    #[must_use]
    pub fn source(mut self, source: FieldSource) -> Self {
        self.source = source;
        self
    }

    /// Set the evidence channels.
    #[must_use]
    pub fn channels(mut self, channels: Vec<EvidenceChannel>) -> Self {
        self.channels = channels;
        self
    }

    /// Build the [`RightsVerification`].
    #[must_use]
    pub fn build(self) -> RightsVerification {
        RightsVerification {
            found: self.found,
            copyright_holder: self.copyright_holder,
            creator: self.creator,
            contact: self.contact,
            rights_url: self.rights_url,
            usage_terms: self.usage_terms,
            ai_constraints: self.ai_constraints,
            dmi: self.dmi,
            source: self.source,
            channels: self.channels,
        }
    }
}

/// Verification result for the hidden steganographic marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenMarkerVerification {
    status: VerificationStatus,
    payload_version: Option<u8>,
    seed: Option<u64>,
    intensity: Option<f32>,
    source: FieldSource,
    tiled: bool,
}

impl HiddenMarkerVerification {
    /// Verification status of the stego payload.
    #[must_use]
    pub fn status(&self) -> VerificationStatus {
        self.status
    }

    /// Payload version, if detected.
    #[must_use]
    pub fn payload_version(&self) -> Option<u8> {
        self.payload_version
    }

    /// PRNG seed, if extracted.
    #[must_use]
    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    /// Embedding intensity, if extracted.
    #[must_use]
    pub fn intensity(&self) -> Option<f32> {
        self.intensity
    }

    /// Source from which this verification was obtained.
    #[must_use]
    pub fn source(&self) -> FieldSource {
        self.source
    }

    /// Whether tiled steganography was detected.
    #[must_use]
    pub fn tiled(&self) -> bool {
        self.tiled
    }

    /// Create a new builder for `HiddenMarkerVerification`.
    pub fn builder() -> HiddenMarkerVerificationBuilder {
        HiddenMarkerVerificationBuilder {
            status: VerificationStatus::NotFound,
            payload_version: None,
            seed: None,
            intensity: None,
            source: FieldSource::Xmp,
            tiled: false,
        }
    }
}

/// Builder for [`HiddenMarkerVerification`].
pub struct HiddenMarkerVerificationBuilder {
    status: VerificationStatus,
    payload_version: Option<u8>,
    seed: Option<u64>,
    intensity: Option<f32>,
    source: FieldSource,
    tiled: bool,
}

impl HiddenMarkerVerificationBuilder {
    /// Set the verification status.
    #[must_use]
    pub fn status(mut self, status: VerificationStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the payload version.
    #[must_use]
    pub fn payload_version(mut self, version: u8) -> Self {
        self.payload_version = Some(version);
        self
    }

    /// Set the extracted seed.
    #[must_use]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the embedding intensity.
    #[must_use]
    pub fn intensity(mut self, intensity: f32) -> Self {
        self.intensity = Some(intensity);
        self
    }

    /// Set the source.
    #[must_use]
    pub fn source(mut self, source: FieldSource) -> Self {
        self.source = source;
        self
    }

    /// Set whether tiled steganography was detected.
    #[must_use]
    pub fn tiled(mut self, tiled: bool) -> Self {
        self.tiled = tiled;
        self
    }

    /// Build the [`HiddenMarkerVerification`].
    #[must_use]
    pub fn build(self) -> HiddenMarkerVerification {
        HiddenMarkerVerification {
            status: self.status,
            payload_version: self.payload_version,
            seed: self.seed,
            intensity: self.intensity,
            source: self.source,
            tiled: self.tiled,
        }
    }
}

/// Verification result for payload authentication (HMAC or Ed25519).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationVerification {
    attempted: bool,
    hmac_status: Option<VerificationStatus>,
    key_id: Option<Vec<u8>>,
    algorithm: String,
    key_matched: bool,
}

impl AuthenticationVerification {
    /// Whether authentication was attempted.
    #[must_use]
    pub fn attempted(&self) -> bool {
        self.attempted
    }

    /// HMAC verification status, if applicable.
    #[must_use]
    pub fn hmac_status(&self) -> Option<VerificationStatus> {
        self.hmac_status
    }

    /// Key identifier, if present.
    #[must_use]
    pub fn key_id(&self) -> Option<&[u8]> {
        self.key_id.as_deref()
    }

    /// Authentication algorithm name.
    #[must_use]
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// Whether the supplied key matched the expected key.
    #[must_use]
    pub fn key_matched(&self) -> bool {
        self.key_matched
    }

    /// Create a new builder for `AuthenticationVerification`.
    pub fn builder() -> AuthenticationVerificationBuilder {
        AuthenticationVerificationBuilder {
            attempted: false,
            hmac_status: None,
            key_id: None,
            algorithm: String::new(),
            key_matched: false,
        }
    }
}

/// Builder for [`AuthenticationVerification`].
pub struct AuthenticationVerificationBuilder {
    attempted: bool,
    hmac_status: Option<VerificationStatus>,
    key_id: Option<Vec<u8>>,
    algorithm: String,
    key_matched: bool,
}

impl AuthenticationVerificationBuilder {
    /// Set whether authentication was attempted.
    #[must_use]
    pub fn attempted(mut self, attempted: bool) -> Self {
        self.attempted = attempted;
        self
    }

    /// Set the HMAC verification status.
    #[must_use]
    pub fn hmac_status(mut self, status: VerificationStatus) -> Self {
        self.hmac_status = Some(status);
        self
    }

    /// Set the key identifier.
    #[must_use]
    pub fn key_id(mut self, key_id: Vec<u8>) -> Self {
        self.key_id = Some(key_id);
        self
    }

    /// Set the authentication algorithm name.
    #[must_use]
    pub fn algorithm(mut self, algorithm: impl Into<String>) -> Self {
        self.algorithm = algorithm.into();
        self
    }

    /// Set whether the supplied key matched.
    #[must_use]
    pub fn key_matched(mut self, matched: bool) -> Self {
        self.key_matched = matched;
        self
    }

    /// Build the [`AuthenticationVerification`].
    #[must_use]
    pub fn build(self) -> AuthenticationVerification {
        AuthenticationVerification {
            attempted: self.attempted,
            hmac_status: self.hmac_status,
            key_id: self.key_id,
            algorithm: self.algorithm,
            key_matched: self.key_matched,
        }
    }
}

/// Verification result for Ed25519 signatures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureVerification {
    present: bool,
    structurally_valid: bool,
    cryptographically_valid: bool,
    public_key_id: Option<Vec<u8>>,
    expected_key_id: Option<Vec<u8>>,
    key_id_matched: bool,
    trusted: bool,
    claim: Option<Vec<u8>>,
    source: FieldSource,
}

impl SignatureVerification {
    /// Whether a signature was present.
    #[must_use]
    pub fn present(&self) -> bool {
        self.present
    }

    /// Whether the signature is structurally valid (correct length, etc.).
    #[must_use]
    pub fn structurally_valid(&self) -> bool {
        self.structurally_valid
    }

    /// Whether the signature is cryptographically valid.
    #[must_use]
    pub fn cryptographically_valid(&self) -> bool {
        self.cryptographically_valid
    }

    /// Public key identifier from the signature, if present.
    #[must_use]
    pub fn public_key_id(&self) -> Option<&[u8]> {
        self.public_key_id.as_deref()
    }

    /// Expected key identifier, if supplied.
    #[must_use]
    pub fn expected_key_id(&self) -> Option<&[u8]> {
        self.expected_key_id.as_deref()
    }

    /// Whether the public key ID matched the expected key ID.
    #[must_use]
    pub fn key_id_matched(&self) -> bool {
        self.key_id_matched
    }

    /// Whether the signature is trusted.
    #[must_use]
    pub fn trusted(&self) -> bool {
        self.trusted
    }

    /// Raw claim bytes from the signature, if present.
    #[must_use]
    pub fn claim(&self) -> Option<&[u8]> {
        self.claim.as_deref()
    }

    /// Source from which this verification was obtained.
    #[must_use]
    pub fn source(&self) -> FieldSource {
        self.source
    }

    /// Create a new builder for `SignatureVerification`.
    pub fn builder() -> SignatureVerificationBuilder {
        SignatureVerificationBuilder {
            present: false,
            structurally_valid: false,
            cryptographically_valid: false,
            public_key_id: None,
            expected_key_id: None,
            key_id_matched: false,
            trusted: false,
            claim: None,
            source: FieldSource::Xmp,
        }
    }
}

/// Builder for [`SignatureVerification`].
pub struct SignatureVerificationBuilder {
    present: bool,
    structurally_valid: bool,
    cryptographically_valid: bool,
    public_key_id: Option<Vec<u8>>,
    expected_key_id: Option<Vec<u8>>,
    key_id_matched: bool,
    trusted: bool,
    claim: Option<Vec<u8>>,
    source: FieldSource,
}

impl SignatureVerificationBuilder {
    /// Set whether a signature was present.
    #[must_use]
    pub fn present(mut self, present: bool) -> Self {
        self.present = present;
        self
    }

    /// Set structural validity.
    #[must_use]
    pub fn structurally_valid(mut self, valid: bool) -> Self {
        self.structurally_valid = valid;
        self
    }

    /// Set cryptographic validity.
    #[must_use]
    pub fn cryptographically_valid(mut self, valid: bool) -> Self {
        self.cryptographically_valid = valid;
        self
    }

    /// Set the public key identifier.
    #[must_use]
    pub fn public_key_id(mut self, key_id: Vec<u8>) -> Self {
        self.public_key_id = Some(key_id);
        self
    }

    /// Set the expected key identifier.
    #[must_use]
    pub fn expected_key_id(mut self, key_id: Vec<u8>) -> Self {
        self.expected_key_id = Some(key_id);
        self
    }

    /// Set whether the key ID matched.
    #[must_use]
    pub fn key_id_matched(mut self, matched: bool) -> Self {
        self.key_id_matched = matched;
        self
    }

    /// Set trust status.
    #[must_use]
    pub fn trusted(mut self, trusted: bool) -> Self {
        self.trusted = trusted;
        self
    }

    /// Set the raw claim bytes.
    #[must_use]
    pub fn claim(mut self, claim: Vec<u8>) -> Self {
        self.claim = Some(claim);
        self
    }

    /// Set the source.
    #[must_use]
    pub fn source(mut self, source: FieldSource) -> Self {
        self.source = source;
        self
    }

    /// Build the [`SignatureVerification`].
    #[must_use]
    pub fn build(self) -> SignatureVerification {
        SignatureVerification {
            present: self.present,
            structurally_valid: self.structurally_valid,
            cryptographically_valid: self.cryptographically_valid,
            public_key_id: self.public_key_id,
            expected_key_id: self.expected_key_id,
            key_id_matched: self.key_id_matched,
            trusted: self.trusted,
            claim: self.claim,
            source: self.source,
        }
    }
}

/// Verification result for instance-digest and content-hash binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingVerification {
    instance_digest_present: bool,
    instance_digest_valid: bool,
    content_hash_present: bool,
    content_hash_valid: bool,
    source: FieldSource,
}

impl BindingVerification {
    /// Whether an instance digest was found.
    #[must_use]
    pub fn instance_digest_present(&self) -> bool {
        self.instance_digest_present
    }

    /// Whether the instance digest is valid.
    #[must_use]
    pub fn instance_digest_valid(&self) -> bool {
        self.instance_digest_valid
    }

    /// Whether a content hash was found.
    #[must_use]
    pub fn content_hash_present(&self) -> bool {
        self.content_hash_present
    }

    /// Whether the content hash is valid.
    #[must_use]
    pub fn content_hash_valid(&self) -> bool {
        self.content_hash_valid
    }

    /// Source from which this verification was obtained.
    #[must_use]
    pub fn source(&self) -> FieldSource {
        self.source
    }

    /// Create a new builder for `BindingVerification`.
    pub fn builder() -> BindingVerificationBuilder {
        BindingVerificationBuilder {
            instance_digest_present: false,
            instance_digest_valid: false,
            content_hash_present: false,
            content_hash_valid: false,
            source: FieldSource::Xmp,
        }
    }
}

/// Builder for [`BindingVerification`].
pub struct BindingVerificationBuilder {
    instance_digest_present: bool,
    instance_digest_valid: bool,
    content_hash_present: bool,
    content_hash_valid: bool,
    source: FieldSource,
}

impl BindingVerificationBuilder {
    /// Set whether an instance digest was found.
    #[must_use]
    pub fn instance_digest_present(mut self, present: bool) -> Self {
        self.instance_digest_present = present;
        self
    }

    /// Set whether the instance digest is valid.
    #[must_use]
    pub fn instance_digest_valid(mut self, valid: bool) -> Self {
        self.instance_digest_valid = valid;
        self
    }

    /// Set whether a content hash was found.
    #[must_use]
    pub fn content_hash_present(mut self, present: bool) -> Self {
        self.content_hash_present = present;
        self
    }

    /// Set whether the content hash is valid.
    #[must_use]
    pub fn content_hash_valid(mut self, valid: bool) -> Self {
        self.content_hash_valid = valid;
        self
    }

    /// Set the source.
    #[must_use]
    pub fn source(mut self, source: FieldSource) -> Self {
        self.source = source;
        self
    }

    /// Build the [`BindingVerification`].
    #[must_use]
    pub fn build(self) -> BindingVerification {
        BindingVerification {
            instance_digest_present: self.instance_digest_present,
            instance_digest_valid: self.instance_digest_valid,
            content_hash_present: self.content_hash_present,
            content_hash_valid: self.content_hash_valid,
            source: self.source,
        }
    }
}

/// Trust evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEvaluation {
    trust_model: String,
    trusted: bool,
    reason: String,
}

impl TrustEvaluation {
    /// Trust model name.
    #[must_use]
    pub fn trust_model(&self) -> &str {
        &self.trust_model
    }

    /// Whether the claim is trusted.
    #[must_use]
    pub fn trusted(&self) -> bool {
        self.trusted
    }

    /// Human-readable trust reason.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Create a new builder for `TrustEvaluation`.
    pub fn builder() -> TrustEvaluationBuilder {
        TrustEvaluationBuilder {
            trust_model: String::new(),
            trusted: false,
            reason: String::new(),
        }
    }
}

/// Builder for [`TrustEvaluation`].
pub struct TrustEvaluationBuilder {
    trust_model: String,
    trusted: bool,
    reason: String,
}

impl TrustEvaluationBuilder {
    /// Set the trust model name.
    #[must_use]
    pub fn trust_model(mut self, model: impl Into<String>) -> Self {
        self.trust_model = model.into();
        self
    }

    /// Set trust status.
    #[must_use]
    pub fn trusted(mut self, trusted: bool) -> Self {
        self.trusted = trusted;
        self
    }

    /// Set the trust reason.
    #[must_use]
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }

    /// Build the [`TrustEvaluation`].
    #[must_use]
    pub fn build(self) -> TrustEvaluation {
        TrustEvaluation {
            trust_model: self.trust_model,
            trusted: self.trusted,
            reason: self.reason,
        }
    }
}

/// A diagnostic message attached to a verification report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    level: DiagnosticLevel,
    message: String,
    source: String,
}

impl Diagnostic {
    /// Diagnostic severity level.
    #[must_use]
    pub fn level(&self) -> DiagnosticLevel {
        self.level
    }

    /// Human-readable diagnostic message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Source component that produced this diagnostic.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Create a new builder for `Diagnostic`.
    pub fn builder() -> DiagnosticBuilder {
        DiagnosticBuilder {
            level: DiagnosticLevel::Info,
            message: String::new(),
            source: String::new(),
        }
    }
}

/// Builder for [`Diagnostic`].
pub struct DiagnosticBuilder {
    level: DiagnosticLevel,
    message: String,
    source: String,
}

impl DiagnosticBuilder {
    /// Set the severity level.
    #[must_use]
    pub fn level(mut self, level: DiagnosticLevel) -> Self {
        self.level = level;
        self
    }

    /// Set the diagnostic message.
    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Set the source component.
    #[must_use]
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// Build the [`Diagnostic`].
    #[must_use]
    pub fn build(self) -> Diagnostic {
        Diagnostic {
            level: self.level,
            message: self.message,
            source: self.source,
        }
    }
}

/// Severity level for a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    /// Informational message.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

/// Structured verification report aggregating all verification channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub(crate) rights: RightsVerification,
    pub(crate) hidden_marker: HiddenMarkerVerification,
    pub(crate) authentication: AuthenticationVerification,
    pub(crate) signatures: Vec<SignatureVerification>,
    pub(crate) bindings: BindingVerification,
    pub(crate) trust: TrustEvaluation,
    pub(crate) evidence_strength: EvidenceStrength,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl VerificationReport {
    /// Rights and legal-notice verification results.
    #[must_use]
    pub fn rights(&self) -> &RightsVerification {
        &self.rights
    }

    /// Hidden steganographic marker verification results.
    #[must_use]
    pub fn hidden_marker(&self) -> &HiddenMarkerVerification {
        &self.hidden_marker
    }

    /// Authentication verification results.
    #[must_use]
    pub fn authentication(&self) -> &AuthenticationVerification {
        &self.authentication
    }

    /// Signature verification results.
    #[must_use]
    pub fn signatures(&self) -> &[SignatureVerification] {
        &self.signatures
    }

    /// Instance-digest and content-hash binding verification results.
    #[must_use]
    pub fn bindings(&self) -> &BindingVerification {
        &self.bindings
    }

    /// Trust evaluation result.
    #[must_use]
    pub fn trust(&self) -> &TrustEvaluation {
        &self.trust
    }

    /// Computed evidence strength across all channels.
    #[must_use]
    pub fn evidence_strength(&self) -> EvidenceStrength {
        self.evidence_strength
    }

    /// Diagnostic messages produced during verification.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Recompute evidence strength from the current verification results.
    pub fn compute_evidence_strength(&self) -> EvidenceStrength {
        let has_notice = self.rights.found;
        let stego_verified = self.hidden_marker.status() == VerificationStatus::Verified;
        let authenticated = self.authentication.attempted()
            && self.authentication.hmac_status() == Some(VerificationStatus::Verified)
            && self.authentication.key_matched();

        match (has_notice, stego_verified, authenticated) {
            (true, true, true) => EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance,
            (true, true, false) => EvidenceStrength::MetadataNoticeAndBestEffortStego,
            (true, false, _) => EvidenceStrength::MetadataNoticeOnly,
            (false, _, _) => EvidenceStrength::NoNoticeFound,
        }
    }

    /// Compute the summary verification status from all channels.
    pub fn summary_status(&self) -> VerificationStatus {
        match self.hidden_marker.status() {
            VerificationStatus::Verified => VerificationStatus::Verified,
            VerificationStatus::Invalid => VerificationStatus::Invalid,
            VerificationStatus::NotFound => {
                if self.rights.found() {
                    VerificationStatus::Verified
                } else {
                    VerificationStatus::NotFound
                }
            }
        }
    }

    /// Returns `true` if any diagnostic has `Error` severity or if verification failed.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.level() == DiagnosticLevel::Error)
            || self.hidden_marker.status() == VerificationStatus::Invalid
            || (self.authentication.attempted()
                && self.authentication.hmac_status() == Some(VerificationStatus::Invalid))
    }

    /// Create a new [`VerificationReportBuilder`].
    pub fn builder() -> crate::verification::VerificationReportBuilder {
        crate::verification::VerificationReportBuilder::new()
    }
}
