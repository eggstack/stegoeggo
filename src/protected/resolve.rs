use crate::error::{Error, Result};
use crate::types::*;

pub fn resolve_request(
    request: &ProtectionRequest,
    input_format: ImageOutputFormat,
) -> Result<ResolvedProtectionPlan> {
    let mut warnings = Vec::new();

    validate_channels(request.channels(), request.mac_key())?;

    if request.policy() != RightsPolicy::Unspecified && !request.channels().rights_metadata {
        return Err(Error::Config(
            "A non-Unspecified rights policy requires rights_metadata to be enabled".into(),
        ));
    }

    if request.policy() == RightsPolicy::ProhibitedSeeConstraints {
        let notice = request.notice();
        let has_constraints =
            notice.ai_constraints().is_some() || notice.web_statement_of_rights().is_some();
        if !has_constraints {
            if let Some(meta) = request.legal_metadata() {
                if meta.ai_constraints().is_some() || meta.web_statement_of_rights().is_some() {
                    // Legal metadata provides constraints
                } else {
                    return Err(Error::Config(
                        "ProhibitedSeeConstraints requires ai_constraints or web_statement_of_rights"
                            .to_string(),
                    ));
                }
            } else {
                return Err(Error::Config(
                    "ProhibitedSeeConstraints requires ai_constraints or web_statement_of_rights"
                        .to_string(),
                ));
            }
        }
    }

    let effective_dmi = match request.policy() {
        RightsPolicy::Unspecified => None,
        policy => Some(DmiValue::from(policy)),
    };

    let seed = request
        .seed()
        .unwrap_or_else(crate::util::seed::generate_random_seed);

    let output_format = request.processing().output_format.unwrap_or(input_format);

    let effective_notice = request.notice().clone();

    if request.channels().authentication == AuthenticationMode::Hmac && request.mac_key().is_none()
    {
        warnings.push(ProtectionWarning::MissingMacKey);
    }

    if !request.channels().rights_metadata {
        warnings.push(ProtectionWarning::MetadataInjectionDisabled);
    }

    Ok(ResolvedProtectionPlan::new(
        request.policy(),
        effective_dmi,
        effective_notice,
        request.channels().clone(),
        request.processing().clone(),
        seed,
        request.intensity(),
        input_format,
        output_format,
        request.legal_metadata().cloned(),
        request.mac_key().map(|k| k.to_vec()),
        warnings,
        request.resource_limits().cloned().unwrap_or_default(),
    ))
}

fn validate_channels(channels: &ProtectionChannels, mac_key: Option<&[u8]>) -> Result<()> {
    if channels.authentication == AuthenticationMode::Hmac
        && matches!(channels.hidden_marker, HiddenMarkerMode::Disabled)
    {
        return Err(Error::Config(
            "HMAC authentication requires an enabled hidden marker".to_string(),
        ));
    }
    if channels.authentication == AuthenticationMode::Hmac && mac_key.is_none() {
        return Err(Error::Config(
            "HMAC authentication requires a MAC key".to_string(),
        ));
    }
    Ok(())
}
