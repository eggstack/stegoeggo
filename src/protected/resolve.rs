use crate::error::{Error, Result};
use crate::types::*;

pub fn resolve_request(
    request: &ProtectionRequest,
    input_format: ImageOutputFormat,
) -> Result<ResolvedProtectionPlan> {
    let mut warnings = Vec::new();

    if let Some(redundancy) = request.processing().stego_redundancy {
        validate_stego_redundancy(redundancy)?;
    }
    validate_jpeg_quality(request.processing().jpeg_quality)?;
    if let HiddenMarkerMode::Tiled { tile_size } = request.channels().hidden_marker {
        if !(32..=1024).contains(&tile_size) {
            return Err(Error::Config(format!(
                "tile size must be in 32..=1024, got {tile_size}"
            )));
        }
    }

    validate_channels(request.channels(), request.mac_key())?;

    if let Some(legal) = request.legal_metadata() {
        legal.validate()?;
    }

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
            let meta_provides_constraints = request.legal_metadata().is_some_and(|meta| {
                meta.ai_constraints().is_some() || meta.web_statement_of_rights().is_some()
            });
            if !meta_provides_constraints {
                warnings.push(ProtectionWarning::MissingRightsConstraints);
            }
        }
    }

    let effective_dmi = match request.policy() {
        RightsPolicy::Unspecified => None,
        policy => Some(DmiValue::from(policy)),
    };

    let seed = match request.seed() {
        Some(seed) => seed,
        None => crate::util::seed::try_generate_random_seed().map_err(|error| {
            Error::Crypto(format!("failed to generate protection seed: {error}"))
        })?,
    };

    let output_format = request.processing().output_format.unwrap_or(input_format);

    let effective_notice = {
        let base = request.notice().clone().with_seed(seed);
        let with_legal = if let Some(legal) = request.legal_metadata() {
            base.with_legal_metadata_fields(legal)
        } else {
            base
        };
        let override_ts = request.timestamp_override();
        let legal_has_explicit_ts = request
            .legal_metadata()
            .and_then(|m| m.notice_applied_at())
            .is_some();
        apply_timestamp_override(
            with_legal,
            override_ts,
            request.legal_metadata().is_some() && !legal_has_explicit_ts,
        )
    };
    effective_notice.validate()?;

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

fn apply_timestamp_override(
    notice: RightsNotice,
    override_ts: Option<&str>,
    auto_compute: bool,
) -> RightsNotice {
    if notice.notice_applied_at().is_some() {
        return notice;
    }
    if let Some(ts) = override_ts {
        return notice.with_notice_applied_at(ts.to_string());
    }
    if auto_compute {
        return notice
            .with_notice_applied_at(crate::protected::metadata_trap::current_timestamp_iso8601());
    }
    notice
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
