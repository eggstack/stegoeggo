use crate::protected::notice_verification as notice;
use crate::protected::steganography::CandidateOutcome;
use crate::protected::steganography::SteganographyProtector;
use crate::resource_limits::ResourceLimits;
use crate::types::{
    DmiValue, EvidenceChannel, EvidenceStrength, NoticeVerification, RightsSignalKind,
    VerificationResult, VerificationStatus,
};
use crate::verification::report::{
    AuthenticationVerification, BindingVerification, Diagnostic, DiagnosticLevel, FieldSource,
    HiddenMarkerVerification, RightsVerification, TrustEvaluation, VerificationReport,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanonicalOutcomeKind {
    Verified,
    InvalidCorrupted,
    MalformedV3,
    UnsupportedVersion,
    AuthKeyMissing,
    AuthFailed,
    ResourceLimitExceeded,
    NotFound,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct CanonicalFacts {
    pub copyright_holder: Option<String>,
    pub creator: Option<String>,
    pub contact: Option<String>,
    pub rights_url: Option<String>,
    pub usage_terms: Option<String>,
    pub ai_constraints: Option<String>,
    pub dmi: Option<DmiValue>,
    pub tdm_reserved: Option<bool>,
    pub rights_signal_kind: RightsSignalKind,
    pub canonical_dmi: Option<DmiValue>,
    pub legacy_dmi: Option<DmiValue>,
    pub protection_seed: Option<u64>,
    pub channels: Vec<EvidenceChannel>,
    pub license_url: Option<String>,
    pub web_statement_of_rights: Option<String>,
    pub credit_line: Option<String>,
    pub copyright_owner: Option<String>,
    pub licensor_name: Option<String>,
    pub licensor_email: Option<String>,
    pub licensor_url: Option<String>,
    pub metadata_date: Option<String>,
    pub notice_applied_at: Option<String>,
    pub has_notice: bool,
    pub stego_status: VerificationStatus,
    pub stego_payload: Option<crate::StegoPayload>,
    pub raw_payload: Option<Vec<u8>>,
    pub payload_version: Option<u8>,
    pub stego_seed: Option<u64>,
    pub intensity: Option<f32>,
    pub tiled: bool,
    pub marker_source: FieldSource,
    pub auth_attempted: bool,
    pub hmac_status: Option<VerificationStatus>,
    pub key_matched: bool,
    pub auth_algorithm: String,
    pub authenticated: bool,
    pub evidence_strength: EvidenceStrength,
    pub outcome_kind: CanonicalOutcomeKind,
    pub unsupported_version: Option<u8>,
    pub resource_limit_exceeded: bool,
    pub report: VerificationReport,
}

fn outcome_kind_of(outcome: &CandidateOutcome) -> (CanonicalOutcomeKind, Option<u8>) {
    match outcome {
        CandidateOutcome::Valid(_) => (CanonicalOutcomeKind::Verified, None),
        CandidateOutcome::Invalid(_) => (CanonicalOutcomeKind::InvalidCorrupted, None),
        CandidateOutcome::MalformedV3 => (CanonicalOutcomeKind::MalformedV3, None),
        CandidateOutcome::UnsupportedVersion(v) => {
            (CanonicalOutcomeKind::UnsupportedVersion, Some(*v))
        }
        CandidateOutcome::AuthenticationKeyMissing(_) => {
            (CanonicalOutcomeKind::AuthKeyMissing, None)
        }
        CandidateOutcome::AuthenticationFailed(_) => (CanonicalOutcomeKind::AuthFailed, None),
        CandidateOutcome::ResourceLimitExceeded => {
            (CanonicalOutcomeKind::ResourceLimitExceeded, None)
        }
        CandidateOutcome::NotFound => (CanonicalOutcomeKind::NotFound, None),
    }
}

fn status_of_kind(kind: CanonicalOutcomeKind) -> VerificationStatus {
    match kind {
        CanonicalOutcomeKind::Verified => VerificationStatus::Verified,
        CanonicalOutcomeKind::NotFound => VerificationStatus::NotFound,
        CanonicalOutcomeKind::InvalidCorrupted
        | CanonicalOutcomeKind::MalformedV3
        | CanonicalOutcomeKind::UnsupportedVersion
        | CanonicalOutcomeKind::AuthKeyMissing
        | CanonicalOutcomeKind::AuthFailed
        | CanonicalOutcomeKind::ResourceLimitExceeded => VerificationStatus::Invalid,
    }
}

fn raw_of_outcome(outcome: &CandidateOutcome) -> Option<Vec<u8>> {
    match outcome {
        CandidateOutcome::Valid(b)
        | CandidateOutcome::Invalid(b)
        | CandidateOutcome::AuthenticationKeyMissing(b)
        | CandidateOutcome::AuthenticationFailed(b) => Some(b.clone()),
        CandidateOutcome::MalformedV3
        | CandidateOutcome::UnsupportedVersion(_)
        | CandidateOutcome::ResourceLimitExceeded
        | CandidateOutcome::NotFound => None,
    }
}

fn is_hmac_payload(raw: &[u8]) -> bool {
    raw.len() > 30 && raw[0] == 0x53 && raw[1] == 0x45 && raw[2] == 3 && raw[29] == 2
}

fn marker_source_for_version(version: Option<u8>) -> FieldSource {
    match version {
        Some(1) => FieldSource::EmbeddedPayloadV1,
        Some(2) => FieldSource::EmbeddedPayloadV2,
        Some(3) => FieldSource::EmbeddedPayloadV3,
        _ => FieldSource::Xmp,
    }
}

fn rights_source_for_channels(channels: &[EvidenceChannel]) -> FieldSource {
    let has_xmp = channels.iter().any(|c| {
        matches!(
            c,
            EvidenceChannel::PngXmp | EvidenceChannel::JpegXmp | EvidenceChannel::WebPXmp
        )
    });
    if has_xmp || channels.is_empty() {
        FieldSource::Xmp
    } else {
        FieldSource::Legacy
    }
}

pub(crate) fn verify_canonical(img_bytes: &[u8], mac_key: &[u8]) -> CanonicalFacts {
    verify_canonical_with_limits(img_bytes, mac_key, &ResourceLimits::default())
}

pub(crate) fn verify_canonical_with_limits(
    img_bytes: &[u8],
    mac_key: &[u8],
    limits: &ResourceLimits,
) -> CanonicalFacts {
    let (
        copyright_holder,
        creator,
        contact,
        rights_url,
        usage_terms,
        ai_constraints,
        license_url,
        web_statement_of_rights,
        credit_line,
        copyright_owner,
        licensor_name,
        licensor_email,
        licensor_url,
        metadata_date,
        notice_applied_at,
        mut channels,
        seed,
        dmi,
        tdm_reserved,
        canonical_dmi,
        legacy_dmi,
        rights_signal_kind_opt,
    ) = extract_rights(img_bytes, limits);

    let has_notice = copyright_holder.is_some()
        || creator.is_some()
        || contact.is_some()
        || rights_url.is_some()
        || usage_terms.is_some()
        || ai_constraints.is_some()
        || dmi.is_some()
        || license_url.is_some()
        || web_statement_of_rights.is_some()
        || credit_line.is_some()
        || copyright_owner.is_some()
        || licensor_name.is_some()
        || licensor_email.is_some()
        || licensor_url.is_some()
        || metadata_date.is_some()
        || notice_applied_at.is_some();

    let protector = SteganographyProtector::with_resource_limits(limits.clone());
    let outcome = protector.verify_payload_from_bytes_outcome(img_bytes, mac_key, true);
    let (outcome_kind, unsupported_version) = outcome_kind_of(&outcome);
    let stego_status = status_of_kind(outcome_kind);
    let raw_payload = raw_of_outcome(&outcome);

    let mut stego_payload: Option<crate::StegoPayload> = None;
    let mut payload_version: Option<u8> = None;
    let mut stego_seed: Option<u64> = None;
    let mut intensity: Option<f32> = None;

    if let Some(ref raw) = raw_payload {
        if let Some(parsed) = SteganographyProtector::parse_verified_payload(raw) {
            payload_version = Some(parsed.version());
            stego_seed = Some(parsed.seed());
            intensity = Some(parsed.intensity());
            if matches!(
                outcome_kind,
                CanonicalOutcomeKind::Verified
                    | CanonicalOutcomeKind::InvalidCorrupted
                    | CanonicalOutcomeKind::AuthKeyMissing
                    | CanonicalOutcomeKind::AuthFailed
            ) {
                stego_payload = Some(parsed);
            }
        }
    }
    if payload_version.is_none() && matches!(outcome_kind, CanonicalOutcomeKind::UnsupportedVersion)
    {
        payload_version = unsupported_version;
    }

    let is_jpeg = img_bytes.starts_with(&[0xFF, 0xD8]);
    if stego_status == VerificationStatus::Verified {
        if is_jpeg {
            if !channels.contains(&EvidenceChannel::DctPayload) {
                channels.push(EvidenceChannel::DctPayload);
            }
        } else if !channels.contains(&EvidenceChannel::LsbPayload) {
            channels.push(EvidenceChannel::LsbPayload);
        }
    }

    let hmac_expected = raw_payload.as_ref().is_some_and(|raw| is_hmac_payload(raw));
    let legacy_hmac_attempt = !hmac_expected
        && !mac_key.is_empty()
        && raw_payload.is_some()
        && stego_status != VerificationStatus::NotFound;

    let (auth_attempted, hmac_status, key_matched, auth_algorithm, authenticated) =
        match outcome_kind {
            CanonicalOutcomeKind::Verified if hmac_expected => (
                true,
                Some(VerificationStatus::Verified),
                true,
                "hmac-sha256".to_string(),
                true,
            ),
            CanonicalOutcomeKind::Verified => {
                if legacy_hmac_attempt {
                    (
                        true,
                        Some(VerificationStatus::Verified),
                        true,
                        "hmac-sha256".to_string(),
                        true,
                    )
                } else {
                    (false, None, false, "crc32".to_string(), false)
                }
            }
            CanonicalOutcomeKind::AuthKeyMissing => (
                true,
                Some(VerificationStatus::NotFound),
                false,
                "hmac-sha256".to_string(),
                false,
            ),
            CanonicalOutcomeKind::AuthFailed => (
                true,
                Some(VerificationStatus::Invalid),
                false,
                "hmac-sha256".to_string(),
                false,
            ),
            CanonicalOutcomeKind::InvalidCorrupted if hmac_expected => (
                true,
                Some(VerificationStatus::Invalid),
                false,
                "hmac-sha256".to_string(),
                false,
            ),
            CanonicalOutcomeKind::InvalidCorrupted if legacy_hmac_attempt => (
                true,
                Some(VerificationStatus::Invalid),
                false,
                "hmac-sha256".to_string(),
                false,
            ),
            CanonicalOutcomeKind::InvalidCorrupted
            | CanonicalOutcomeKind::MalformedV3
            | CanonicalOutcomeKind::UnsupportedVersion
            | CanonicalOutcomeKind::ResourceLimitExceeded
            | CanonicalOutcomeKind::NotFound => (false, None, false, "crc32".to_string(), false),
        };

    let has_stego = stego_status == VerificationStatus::Verified;
    let evidence_strength = match (has_notice, has_stego, authenticated) {
        (true, true, true) => EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance,
        (true, true, false) => EvidenceStrength::MetadataNoticeAndBestEffortStego,
        (true, false, _) => EvidenceStrength::MetadataNoticeOnly,
        (false, _, _) => EvidenceStrength::NoNoticeFound,
    };

    let marker_source = marker_source_for_version(payload_version);
    let rights_source = rights_source_for_channels(&channels);

    let mut rights_builder = RightsVerification::builder()
        .found(has_notice)
        .source(rights_source)
        .channels(channels.clone());
    if let Some(ref v) = copyright_holder {
        rights_builder = rights_builder.copyright_holder(v.clone());
    }
    if let Some(ref v) = creator {
        rights_builder = rights_builder.creator(v.clone());
    }
    if let Some(ref v) = contact {
        rights_builder = rights_builder.contact(v.clone());
    }
    if let Some(ref v) = rights_url {
        rights_builder = rights_builder.rights_url(v.clone());
    }
    if let Some(ref v) = usage_terms {
        rights_builder = rights_builder.usage_terms(v.clone());
    }
    if let Some(ref v) = ai_constraints {
        rights_builder = rights_builder.ai_constraints(v.clone());
    }
    if let Some(v) = dmi {
        rights_builder = rights_builder.dmi(v as u8);
    }
    let rights = rights_builder.build();

    let mut marker_builder = HiddenMarkerVerification::builder()
        .status(stego_status)
        .source(marker_source);
    if let Some(v) = payload_version {
        marker_builder = marker_builder.payload_version(v);
    }
    if let Some(v) = stego_seed {
        marker_builder = marker_builder.seed(v);
    } else if let Some(v) = seed {
        if stego_status != VerificationStatus::NotFound {
            marker_builder = marker_builder.seed(v);
        }
    }
    if let Some(v) = intensity {
        marker_builder = marker_builder.intensity(v);
    }
    let hidden_marker = marker_builder.build();

    let mut auth_builder = AuthenticationVerification::builder().attempted(auth_attempted);
    if let Some(s) = hmac_status {
        auth_builder = auth_builder.hmac_status(s);
    }
    auth_builder = auth_builder
        .algorithm(auth_algorithm.clone())
        .key_matched(key_matched);
    let authentication = auth_builder.build();

    let trust = TrustEvaluation::builder()
        .trust_model("caller-owned")
        .trusted(false)
        .reason("Trust evaluation requires an explicit key/trust policy".to_string())
        .build();

    let mut builder = VerificationReport::builder()
        .with_rights(rights)
        .with_hidden_marker(hidden_marker)
        .with_authentication(authentication)
        .with_bindings(BindingVerification::builder().build())
        .with_trust(trust);

    builder = match outcome_kind {
        CanonicalOutcomeKind::MalformedV3 => builder.add_diagnostic(
            Diagnostic::builder()
                .level(DiagnosticLevel::Error)
                .message("Malformed v3 payload header".to_string())
                .source("hidden-marker".to_string())
                .build(),
        ),
        CanonicalOutcomeKind::UnsupportedVersion => builder.add_diagnostic(
            Diagnostic::builder()
                .level(DiagnosticLevel::Error)
                .message(format!(
                    "Unsupported payload version {}",
                    unsupported_version.unwrap_or(0)
                ))
                .source("hidden-marker".to_string())
                .build(),
        ),
        CanonicalOutcomeKind::AuthKeyMissing => builder.add_diagnostic(
            Diagnostic::builder()
                .level(DiagnosticLevel::Warning)
                .message("HMAC payload found but no MAC key supplied".to_string())
                .source("authentication".to_string())
                .build(),
        ),
        CanonicalOutcomeKind::AuthFailed => builder.add_diagnostic(
            Diagnostic::builder()
                .level(DiagnosticLevel::Error)
                .message("HMAC authentication failed".to_string())
                .source("authentication".to_string())
                .build(),
        ),
        CanonicalOutcomeKind::InvalidCorrupted => builder.add_diagnostic(
            Diagnostic::builder()
                .level(DiagnosticLevel::Error)
                .message("Payload found but integrity check failed".to_string())
                .source("hidden-marker".to_string())
                .build(),
        ),
        CanonicalOutcomeKind::ResourceLimitExceeded => builder.add_diagnostic(
            Diagnostic::builder()
                .level(DiagnosticLevel::Error)
                .message("Resource limit exceeded during verification".to_string())
                .source("hidden-marker".to_string())
                .build(),
        ),
        CanonicalOutcomeKind::Verified | CanonicalOutcomeKind::NotFound => builder,
    };

    if outcome_kind == CanonicalOutcomeKind::NotFound && has_notice {
        builder = builder.add_diagnostic(
            Diagnostic::builder()
                .level(DiagnosticLevel::Info)
                .message("Metadata-only evidence; no hidden marker found".to_string())
                .source("rights".to_string())
                .build(),
        );
    }

    let report = builder.build();

    CanonicalFacts {
        copyright_holder,
        creator,
        contact,
        rights_url,
        usage_terms,
        ai_constraints,
        dmi,
        tdm_reserved,
        rights_signal_kind: rights_signal_kind_opt.unwrap_or(RightsSignalKind::Unknown),
        canonical_dmi,
        legacy_dmi,
        protection_seed: seed,
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
        has_notice,
        stego_status,
        stego_payload,
        raw_payload,
        payload_version,
        stego_seed,
        intensity,
        tiled: false,
        marker_source,
        auth_attempted,
        hmac_status,
        key_matched,
        auth_algorithm,
        authenticated,
        evidence_strength,
        outcome_kind,
        unsupported_version,
        resource_limit_exceeded: outcome_kind == CanonicalOutcomeKind::ResourceLimitExceeded,
        report,
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn extract_rights(
    img_bytes: &[u8],
    limits: &ResourceLimits,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Vec<EvidenceChannel>,
    Option<u64>,
    Option<DmiValue>,
    Option<bool>,
    Option<DmiValue>,
    Option<DmiValue>,
    Option<RightsSignalKind>,
) {
    if img_bytes.len() < 8 {
        return (
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            None,
            None,
            None,
            None,
            None,
        );
    }
    let format = notice::detect_format(img_bytes);
    let mut channels = Vec::new();
    let mut seed: Option<u64> = None;
    let mut dmi: Option<DmiValue> = None;
    let mut tdm_reserved: Option<bool> = None;
    let mut canonical_dmi: Option<DmiValue> = None;
    let mut legacy_dmi: Option<DmiValue> = None;
    let mut kind: Option<RightsSignalKind> = None;
    let fields = match format {
        Some(notice::Format::Png) => {
            let result = notice::extract_png_notice(img_bytes, &mut channels, &mut seed);
            notice::extract_xmp_dmi_from_png_with_limits(
                img_bytes,
                &mut dmi,
                &mut tdm_reserved,
                &mut canonical_dmi,
                &mut legacy_dmi,
                &mut kind,
                limits,
            );
            result
        }
        Some(notice::Format::Jpeg) => {
            let result = notice::extract_jpeg_notice(img_bytes, &mut channels, &mut seed);
            notice::extract_xmp_dmi_from_jpeg_with_limits(
                img_bytes,
                &mut dmi,
                &mut tdm_reserved,
                &mut canonical_dmi,
                &mut legacy_dmi,
                &mut kind,
                limits,
            );
            result
        }
        Some(notice::Format::WebP) => {
            let result = notice::extract_webp_notice(img_bytes, &mut channels, &mut seed);
            notice::extract_xmp_dmi_from_webp_with_limits(
                img_bytes,
                &mut dmi,
                &mut tdm_reserved,
                &mut canonical_dmi,
                &mut legacy_dmi,
                &mut kind,
                limits,
            );
            result
        }
        None => {
            return (
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Vec::new(),
                None,
                None,
                None,
                None,
                None,
                None,
            );
        }
    };
    (
        fields.0,
        fields.1,
        fields.2,
        fields.3,
        fields.4,
        fields.5,
        fields.6,
        fields.7,
        fields.8,
        fields.9,
        fields.10,
        fields.11,
        fields.12,
        fields.13,
        fields.14,
        channels,
        seed,
        dmi,
        tdm_reserved,
        canonical_dmi,
        legacy_dmi,
        kind,
    )
}

pub(crate) fn project_status_from_canonical(facts: &CanonicalFacts) -> VerificationStatus {
    facts.stego_status
}

pub(crate) fn project_result_from_canonical(facts: &CanonicalFacts) -> VerificationResult {
    match (facts.stego_status, &facts.stego_payload) {
        (VerificationStatus::Verified, Some(payload)) => VerificationResult::Verified {
            payload: payload.clone(),
        },
        (VerificationStatus::Invalid, Some(payload)) => VerificationResult::Corrupted {
            payload: payload.clone(),
        },
        _ => {
            if let Some(s) = facts.protection_seed {
                VerificationResult::MetadataOnly { seed: s }
            } else {
                VerificationResult::NotFound
            }
        }
    }
}

pub(crate) fn project_notice_from_canonical(facts: &CanonicalFacts) -> NoticeVerification {
    NoticeVerification::builder()
        .copyright_holder(facts.copyright_holder.clone())
        .creator(facts.creator.clone())
        .contact(facts.contact.clone())
        .rights_url(facts.rights_url.clone())
        .usage_terms(facts.usage_terms.clone())
        .ai_constraints(facts.ai_constraints.clone())
        .dmi(facts.dmi)
        .tdm_reserved(facts.tdm_reserved)
        .rights_signal_kind(facts.rights_signal_kind)
        .canonical_dmi(facts.canonical_dmi)
        .legacy_dmi(facts.legacy_dmi)
        .protection_seed(facts.protection_seed)
        .stego_status(facts.stego_status)
        .stego_payload(facts.stego_payload.clone())
        .authenticated(facts.authenticated)
        .evidence_strength(facts.evidence_strength)
        .channels(facts.channels.clone())
        .license_url(facts.license_url.clone())
        .web_statement_of_rights(facts.web_statement_of_rights.clone())
        .credit_line(facts.credit_line.clone())
        .copyright_owner(facts.copyright_owner.clone())
        .licensor_name(facts.licensor_name.clone())
        .licensor_email(facts.licensor_email.clone())
        .licensor_url(facts.licensor_url.clone())
        .metadata_date(facts.metadata_date.clone())
        .notice_applied_at(facts.notice_applied_at.clone())
        .build()
}

pub(crate) fn project_report_from_canonical(facts: &CanonicalFacts) -> VerificationReport {
    facts.report.clone()
}
