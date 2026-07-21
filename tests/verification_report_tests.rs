use stegoeggo::types::{EvidenceChannel, EvidenceStrength, VerificationStatus};
use stegoeggo::verification::{
    AuthenticationVerification, BindingVerification, Diagnostic, DiagnosticLevel, FieldSource,
    HiddenMarkerVerification, RightsVerification, SignatureVerification, TrustEvaluation,
    VerificationReport, VerificationReportBuilder,
};

#[test]
fn test_report_builder() {
    let rights = RightsVerification::builder()
        .found(true)
        .copyright_holder("Test Corp")
        .creator("Author")
        .source(FieldSource::Xmp)
        .build();

    let hidden = HiddenMarkerVerification::builder()
        .status(VerificationStatus::Verified)
        .payload_version(2)
        .seed(42)
        .intensity(0.75)
        .source(FieldSource::EmbeddedPayloadV2)
        .build();

    let auth = AuthenticationVerification::builder()
        .attempted(true)
        .hmac_status(VerificationStatus::Verified)
        .key_id(vec![1, 2, 3])
        .algorithm("hmac-sha256")
        .key_matched(true)
        .build();

    let sig = SignatureVerification::builder()
        .present(true)
        .structurally_valid(true)
        .cryptographically_valid(true)
        .source(FieldSource::EmbeddedPayloadV3)
        .build();

    let bindings = BindingVerification::builder()
        .instance_digest_present(true)
        .instance_digest_valid(true)
        .content_hash_present(true)
        .content_hash_valid(true)
        .build();

    let trust = TrustEvaluation::builder()
        .trust_model("local")
        .trusted(true)
        .reason("All checks passed")
        .build();

    let diag = Diagnostic::builder()
        .level(DiagnosticLevel::Info)
        .message("Test diagnostic")
        .source("test")
        .build();

    let report = VerificationReport::builder()
        .with_rights(rights)
        .with_hidden_marker(hidden)
        .with_authentication(auth)
        .add_signature(sig)
        .with_bindings(bindings)
        .with_trust(trust)
        .add_diagnostic(diag)
        .build();

    assert!(report.rights().found());
    assert_eq!(report.rights().copyright_holder(), Some("Test Corp"));
    assert_eq!(report.rights().creator(), Some("Author"));
    assert_eq!(
        report.hidden_marker().status(),
        VerificationStatus::Verified
    );
    assert_eq!(report.hidden_marker().payload_version(), Some(2));
    assert_eq!(report.hidden_marker().seed(), Some(42));
    assert!(report.authentication().attempted());
    assert_eq!(report.authentication().algorithm(), "hmac-sha256");
    assert!(report.authentication().key_matched());
    assert_eq!(report.signatures().len(), 1);
    assert!(report.signatures()[0].present());
    assert!(report.bindings().instance_digest_present());
    assert!(report.bindings().instance_digest_valid());
    assert!(report.trust().trusted());
    assert_eq!(report.trust().reason(), "All checks passed");
    assert_eq!(report.diagnostics().len(), 1);
    assert_eq!(report.diagnostics()[0].level(), DiagnosticLevel::Info);
    assert_eq!(report.diagnostics()[0].message(), "Test diagnostic");
}

#[test]
fn test_report_evidence_strength() {
    let report = VerificationReport::builder()
        .with_rights(RightsVerification::builder().found(true).build())
        .with_hidden_marker(
            HiddenMarkerVerification::builder()
                .status(VerificationStatus::Verified)
                .build(),
        )
        .with_authentication(
            AuthenticationVerification::builder()
                .attempted(true)
                .hmac_status(VerificationStatus::Verified)
                .key_matched(true)
                .build(),
        )
        .build();

    assert_eq!(
        report.evidence_strength(),
        EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance
    );
}

#[test]
fn test_report_evidence_strength_notice_and_stego() {
    let report = VerificationReport::builder()
        .with_rights(RightsVerification::builder().found(true).build())
        .with_hidden_marker(
            HiddenMarkerVerification::builder()
                .status(VerificationStatus::Verified)
                .build(),
        )
        .build();

    assert_eq!(
        report.evidence_strength(),
        EvidenceStrength::MetadataNoticeAndBestEffortStego
    );
}

#[test]
fn test_report_evidence_strength_notice_only() {
    let report = VerificationReport::builder()
        .with_rights(RightsVerification::builder().found(true).build())
        .build();

    assert_eq!(
        report.evidence_strength(),
        EvidenceStrength::MetadataNoticeOnly
    );
}

#[test]
fn test_report_evidence_strength_no_notice() {
    let report = VerificationReport::builder().build();

    assert_eq!(report.evidence_strength(), EvidenceStrength::NoNoticeFound);
}

#[test]
fn test_report_summary_status_stego_verified() {
    let report = VerificationReport::builder()
        .with_hidden_marker(
            HiddenMarkerVerification::builder()
                .status(VerificationStatus::Verified)
                .build(),
        )
        .build();

    assert_eq!(report.summary_status(), VerificationStatus::Verified);
}

#[test]
fn test_report_summary_status_stego_invalid() {
    let report = VerificationReport::builder()
        .with_hidden_marker(
            HiddenMarkerVerification::builder()
                .status(VerificationStatus::Invalid)
                .build(),
        )
        .build();

    assert_eq!(report.summary_status(), VerificationStatus::Invalid);
}

#[test]
fn test_report_summary_status_notice_fallback() {
    let report = VerificationReport::builder()
        .with_hidden_marker(
            HiddenMarkerVerification::builder()
                .status(VerificationStatus::NotFound)
                .build(),
        )
        .with_rights(RightsVerification::builder().found(true).build())
        .build();

    assert_eq!(report.summary_status(), VerificationStatus::Verified);
}

#[test]
fn test_report_summary_status_not_found() {
    let report = VerificationReport::builder().build();
    assert_eq!(report.summary_status(), VerificationStatus::NotFound);
}

#[test]
fn test_report_has_errors_diagnostic() {
    let report = VerificationReport::builder()
        .add_diagnostic(
            Diagnostic::builder()
                .level(DiagnosticLevel::Error)
                .message("Something went wrong")
                .source("test")
                .build(),
        )
        .build();

    assert!(report.has_errors());
}

#[test]
fn test_report_has_errors_stego_invalid() {
    let report = VerificationReport::builder()
        .with_hidden_marker(
            HiddenMarkerVerification::builder()
                .status(VerificationStatus::Invalid)
                .build(),
        )
        .build();

    assert!(report.has_errors());
}

#[test]
fn test_report_has_errors_hmac_invalid() {
    let report = VerificationReport::builder()
        .with_authentication(
            AuthenticationVerification::builder()
                .attempted(true)
                .hmac_status(VerificationStatus::Invalid)
                .build(),
        )
        .build();

    assert!(report.has_errors());
}

#[test]
fn test_report_has_no_errors() {
    let report = VerificationReport::builder()
        .with_hidden_marker(
            HiddenMarkerVerification::builder()
                .status(VerificationStatus::Verified)
                .build(),
        )
        .build();

    assert!(!report.has_errors());
}

#[test]
fn test_report_default_builder_state() {
    let report = VerificationReport::builder().build();

    assert!(!report.rights().found());
    assert_eq!(
        report.hidden_marker().status(),
        VerificationStatus::NotFound
    );
    assert!(!report.authentication().attempted());
    assert!(report.signatures().is_empty());
    assert!(!report.bindings().instance_digest_present());
    assert!(!report.trust().trusted());
    assert!(report.diagnostics().is_empty());
    assert_eq!(report.evidence_strength(), EvidenceStrength::NoNoticeFound);
}

#[test]
fn test_report_builder_default_trait() {
    let report = VerificationReportBuilder::default().build();
    assert!(!report.rights().found());
}

#[test]
fn test_rights_verification_channels() {
    let rights = RightsVerification::builder()
        .found(true)
        .channels(vec![EvidenceChannel::JpegXmp, EvidenceChannel::PngXmp])
        .build();

    assert_eq!(rights.channels().len(), 2);
    assert!(rights.channels().contains(&EvidenceChannel::JpegXmp));
}

#[test]
fn test_hidden_marker_tiled() {
    let hidden = HiddenMarkerVerification::builder()
        .status(VerificationStatus::Verified)
        .tiled(true)
        .build();

    assert!(hidden.tiled());
}

#[test]
fn test_signature_verification_detailed() {
    let sig = SignatureVerification::builder()
        .present(true)
        .structurally_valid(true)
        .cryptographically_valid(true)
        .public_key_id(vec![1, 2, 3])
        .expected_key_id(vec![1, 2, 3])
        .key_id_matched(true)
        .trusted(true)
        .claim(vec![4, 5, 6])
        .source(FieldSource::DetachedManifest)
        .build();

    assert!(sig.present());
    assert!(sig.structurally_valid());
    assert!(sig.cryptographically_valid());
    assert_eq!(sig.public_key_id(), Some([1, 2, 3].as_slice()));
    assert_eq!(sig.expected_key_id(), Some([1, 2, 3].as_slice()));
    assert!(sig.key_id_matched());
    assert!(sig.trusted());
    assert_eq!(sig.claim(), Some([4, 5, 6].as_slice()));
    assert_eq!(sig.source(), FieldSource::DetachedManifest);
}

#[test]
fn test_trust_evaluation_fields() {
    let trust = TrustEvaluation::builder()
        .trust_model("web-of-trust")
        .trusted(false)
        .reason("Key not in trust store")
        .build();

    assert_eq!(trust.trust_model(), "web-of-trust");
    assert!(!trust.trusted());
    assert_eq!(trust.reason(), "Key not in trust store");
}
