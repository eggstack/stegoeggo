use super::report::*;
use crate::types::EvidenceStrength;

/// Fluent builder for constructing a [`VerificationReport`].
pub struct VerificationReportBuilder {
    rights: RightsVerification,
    hidden_marker: HiddenMarkerVerification,
    authentication: AuthenticationVerification,
    signatures: Vec<SignatureVerification>,
    bindings: BindingVerification,
    trust: TrustEvaluation,
    diagnostics: Vec<Diagnostic>,
}

impl VerificationReportBuilder {
    /// Create a new builder with default (empty) values.
    pub fn new() -> Self {
        Self {
            rights: RightsVerification::builder().build(),
            hidden_marker: HiddenMarkerVerification::builder().build(),
            authentication: AuthenticationVerification::builder().build(),
            signatures: Vec::new(),
            bindings: BindingVerification::builder().build(),
            trust: TrustEvaluation::builder().build(),
            diagnostics: Vec::new(),
        }
    }

    /// Set the rights verification results.
    #[must_use]
    pub fn with_rights(mut self, rights: RightsVerification) -> Self {
        self.rights = rights;
        self
    }

    /// Set the hidden marker verification results.
    #[must_use]
    pub fn with_hidden_marker(mut self, marker: HiddenMarkerVerification) -> Self {
        self.hidden_marker = marker;
        self
    }

    /// Set the authentication verification results.
    #[must_use]
    pub fn with_authentication(mut self, auth: AuthenticationVerification) -> Self {
        self.authentication = auth;
        self
    }

    /// Add a signature verification result.
    #[must_use]
    pub fn add_signature(mut self, sig: SignatureVerification) -> Self {
        self.signatures.push(sig);
        self
    }

    /// Set the binding verification results.
    #[must_use]
    pub fn with_bindings(mut self, bindings: BindingVerification) -> Self {
        self.bindings = bindings;
        self
    }

    /// Set the trust evaluation.
    #[must_use]
    pub fn with_trust(mut self, trust: TrustEvaluation) -> Self {
        self.trust = trust;
        self
    }

    /// Add a diagnostic message.
    #[must_use]
    pub fn add_diagnostic(mut self, diag: Diagnostic) -> Self {
        self.diagnostics.push(diag);
        self
    }

    /// Build the [`VerificationReport`], computing evidence strength automatically.
    #[must_use]
    pub fn build(self) -> VerificationReport {
        let mut report = VerificationReport {
            rights: self.rights,
            hidden_marker: self.hidden_marker,
            authentication: self.authentication,
            signatures: self.signatures,
            bindings: self.bindings,
            trust: self.trust,
            evidence_strength: EvidenceStrength::NoNoticeFound,
            diagnostics: self.diagnostics,
        };
        report.evidence_strength = report.compute_evidence_strength();
        report
    }
}

/// Default implementation delegates to [`VerificationReportBuilder::new`].
impl Default for VerificationReportBuilder {
    fn default() -> Self {
        Self::new()
    }
}
