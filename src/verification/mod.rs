/// Verification report builder with fluent API.
pub mod builder;
/// Verification report types: rights, stego, authentication, binding, trust, and diagnostics.
pub mod report;

pub use builder::VerificationReportBuilder;
pub use report::{
    AuthenticationVerification, BindingVerification, Diagnostic, DiagnosticLevel, FieldSource,
    HiddenMarkerVerification, RightsVerification, SignatureVerification, TrustEvaluation,
    VerificationReport,
};
