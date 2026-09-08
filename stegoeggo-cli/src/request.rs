use crate::args::{
    Args, AuthenticationArg, DmiArg, HiddenMarkerArg, PresetArg, ProfileArg, ProtectionLevelArg,
};
use crate::keys::resolve_key_input;
use crate::output::config_err;
use stegoeggo::{
    generate_random_seed, DmiValue, HiddenMarkerMode, ImageOutputFormat, ProtectionChannels,
    ProtectionLevel, ProtectionRequest, ProtectionWarning, RightsPolicy, WarningSeverity,
};

pub(crate) fn build_legal_metadata(
    args: &Args,
) -> (Option<stegoeggo::LegalMetadata>, Option<DmiValue>) {
    let has_legal_flags = args.copyright_notice.is_some()
        || args.creator.is_some()
        || args.contact.is_some()
        || args.rights_url.is_some()
        || args.usage_terms.is_some()
        || args.ai_constraints.is_some()
        || args.no_ai_training
        || args.no_genai_training
        || args.tdm_reserved
        || args.credit_line.is_some()
        || args.copyright_owner.is_some()
        || args.licensor_name.is_some()
        || args.licensor_email.is_some()
        || args.licensor_url.is_some()
        || args.content_created_at.is_some();

    if !has_legal_flags {
        return (None, None);
    }

    let mut meta = stegoeggo::LegalMetadata::default();
    let mut dmi_override: Option<DmiValue> = None;

    if let Some(ref v) = args.copyright_notice {
        meta = meta.with_copyright_holder(v);
    }
    if let Some(ref v) = args.creator {
        meta = meta.with_creator(v);
    }
    if let Some(ref v) = args.contact {
        meta = meta.with_contact_email(v);
    }
    if let Some(ref v) = args.rights_url {
        meta = meta.with_web_statement_of_rights(v);
    }
    if let Some(ref v) = args.usage_terms {
        meta = meta.with_usage_terms(v);
    }
    if let Some(ref v) = args.ai_constraints {
        meta = meta.with_ai_constraints(v);
    }
    if let Some(ref v) = args.credit_line {
        meta = meta.with_credit_line(v);
    }
    if let Some(ref v) = args.copyright_owner {
        meta = meta.with_copyright_owner(v);
    }
    if let Some(ref v) = args.licensor_name {
        meta = meta.with_licensor_name(v);
    }
    if let Some(ref v) = args.licensor_email {
        meta = meta.with_licensor_email(v);
    }
    if let Some(ref v) = args.licensor_url {
        meta = meta.with_licensor_url(v);
    }
    if let Some(ref v) = args.content_created_at {
        meta = meta.with_creation_date(v);
    }

    if args.no_ai_training {
        dmi_override = Some(DmiValue::ProhibitedAiMlTraining);
        if args.ai_constraints.is_none() {
            meta = meta.with_ai_constraints(
                "Training for artificial intelligence and machine learning is prohibited",
            );
        }
    } else if args.no_genai_training {
        dmi_override = Some(DmiValue::ProhibitedGenAiMlTraining);
        if args.ai_constraints.is_none() {
            meta = meta.with_ai_constraints(
                "Training for generative artificial intelligence is prohibited",
            );
        }
    } else if args.tdm_reserved {
        dmi_override = Some(DmiValue::ProhibitedSeeConstraints);
        if args.ai_constraints.is_none() {
            meta = meta.with_ai_constraints("Text and data mining rights reserved");
        }
    }

    (Some(meta), dmi_override)
}

pub(crate) fn has_new_style_flags(args: &Args) -> bool {
    args.rights_policy.is_some()
        || args.preset.is_some()
        || args.hidden_marker.is_some()
        || args.authentication.is_some()
}

#[allow(deprecated)]
#[cfg(test)]
pub(crate) fn build_protection_request(
    args: &Args,
) -> Result<ProtectionRequest, Box<dyn std::error::Error>> {
    build_protection_request_with_explicit_options(args, false, false)
}

pub(crate) fn build_protection_request_with_explicit_options(
    args: &Args,
    level_explicit: bool,
    profile_explicit: bool,
) -> Result<ProtectionRequest, Box<dyn std::error::Error>> {
    let is_new_style = has_new_style_flags(args);

    if is_new_style
        && args.preset.is_some()
        && (level_explicit
            || profile_explicit
            || args.level != ProtectionLevelArg::Standard
            || args.profile != ProfileArg::LegalNotice)
    {
        return Err(config_err(
            "Cannot combine --preset with --level/--profile; use --preset alone or --level/--profile alone",
        ));
    }

    if is_new_style
        && args.preset.is_some()
        && (args.hidden_marker.is_some() || args.authentication.is_some())
    {
        return Err(config_err(
            "Cannot combine --preset with --hidden-marker/--authentication; use --preset alone or explicit channel flags alone",
        ));
    }

    if is_new_style
        && args.preset.is_none()
        && (args.hidden_marker.is_some() || args.authentication.is_some())
        && (level_explicit || profile_explicit)
    {
        return Err(config_err(
            "Cannot combine --hidden-marker/--authentication with --level/--profile; use explicit channel flags alone or --level/--profile alone",
        ));
    }

    let (legal_metadata, legal_dmi_override) = build_legal_metadata(args);

    let legal_metadata = if let Some(ref dmi_arg) = args.dmi {
        if matches!(dmi_arg, DmiArg::ProhibitedConstraints) {
            if args.ai_constraints.is_none() && args.rights_url.is_none() {
                match legal_metadata {
                    Some(meta) => {
                        Some(meta.with_ai_constraints("Text and data mining rights reserved"))
                    }
                    None => {
                        let mut meta = stegoeggo::LegalMetadata::default();
                        meta = meta.with_ai_constraints("Text and data mining rights reserved");
                        Some(meta)
                    }
                }
            } else {
                legal_metadata
            }
        } else {
            legal_metadata
        }
    } else {
        legal_metadata
    };

    if args.metadata == Some(false) && legal_metadata.is_some() {
        return Err(config_err(
            "Cannot use --metadata false with legal metadata flags (--copyright-notice, --creator, etc.). Legal metadata requires metadata injection",
        ));
    }

    let seed = args.seed.unwrap_or_else(generate_random_seed);

    let mac_key = resolve_key_input(&args.key, "STEGOEGGO_KEY")?;

    let output_format: Option<ImageOutputFormat> = args.format.as_ref().map(|f| f.clone().into());

    let (policy, channels) = if is_new_style {
        build_new_style_request(args, &legal_metadata, legal_dmi_override)?
    } else {
        build_legacy_style_request(args, &legal_metadata, legal_dmi_override)?
    };

    if args.metadata == Some(false) && channels.rights_metadata {
        return Err(config_err(
            "Cannot use --metadata false with a preset/channel selection that injects rights metadata; omit --metadata false or choose a metadata-free configuration",
        ));
    }

    if matches!(channels.authentication, stegoeggo::AuthenticationMode::Hmac) && mac_key.is_none() {
        return Err(config_err(
            "HMAC authentication requires a key (--key hex, --key @file, --key -, or env STEGOEGGO_KEY)",
        ));
    }

    if matches!(channels.authentication, stegoeggo::AuthenticationMode::Hmac)
        && matches!(channels.hidden_marker, HiddenMarkerMode::Disabled)
    {
        return Err(config_err(
            "HMAC authentication requires a non-disabled hidden marker (--hidden-marker best-effort)",
        ));
    }

    if !args.intensity.is_finite() {
        return Err(config_err(format!(
            "--intensity must be finite, got {}",
            args.intensity
        )));
    }
    let notice = stegoeggo::RightsNotice::default();

    let mut request = stegoeggo::ProtectionRequest::new(notice, policy, channels)
        .with_seed(seed)
        .with_intensity(args.intensity.clamp(0.0, 1.0))
        .with_jpeg_quality(args.jpeg_quality.clamp(1, 100));

    if !(1..=10).contains(&args.stego_redundancy) {
        return Err(config_err(format!(
            "--stego-redundancy must be between 1 and 10, got {}",
            args.stego_redundancy
        )));
    }
    request = request.with_stego_redundancy(args.stego_redundancy);

    if let Some(fmt) = output_format {
        request = request.with_output_format(fmt);
    }
    if args.progressive {
        request = request.with_progressive_jpeg();
    }
    if let Some(meta) = legal_metadata {
        request = request.with_legal_metadata(meta);
    }
    if let Some(key) = mac_key {
        request = request.with_mac_key(key);
    }

    Ok(request)
}

#[allow(deprecated)]
fn build_new_style_request(
    args: &Args,
    _legal_metadata: &Option<stegoeggo::LegalMetadata>,
    legal_dmi_override: Option<DmiValue>,
) -> Result<(RightsPolicy, ProtectionChannels), Box<dyn std::error::Error>> {
    let mut policy = args
        .rights_policy
        .map(RightsPolicy::from)
        .unwrap_or(RightsPolicy::Unspecified);
    let rights_policy_set = args.rights_policy.is_some();
    let shorthand_set = legal_dmi_override.is_some();

    if let Some(dmi_val) = legal_dmi_override {
        let dmi_policy = RightsPolicy::from_dmi_value(dmi_val);
        if rights_policy_set && dmi_policy != policy {
            return Err(config_err(format!(
                "Conflicting policy: --rights-policy {:?} contradicts --no-ai-training/--no-genai-training/--tdm-reserved",
                args.rights_policy
            )));
        }
        policy = dmi_policy;
    }

    if let Some(ref dmi_arg) = args.dmi {
        if let Some(dmi_val) = dmi_arg.clone().into_dmi_value() {
            let dmi_policy = RightsPolicy::from_dmi_value(dmi_val);
            if (rights_policy_set || shorthand_set) && dmi_policy != policy {
                if rights_policy_set {
                    return Err(config_err(format!(
                        "Conflicting policy: --rights-policy {:?} contradicts --dmi {:?}",
                        args.rights_policy, dmi_arg
                    )));
                }
                return Err(config_err(format!(
                    "Conflicting policy: --dmi {:?} contradicts --no-ai-training/--no-genai-training/--tdm-reserved",
                    dmi_arg
                )));
            }
            policy = dmi_policy;
        }
    }

    let channels = if let Some(preset_arg) = args.preset {
        let preset: stegoeggo::ProtectionPreset = preset_arg.into();
        preset.to_channels()
    } else {
        let hidden = args
            .hidden_marker
            .map(|h| match h {
                HiddenMarkerArg::Disabled => HiddenMarkerMode::Disabled,
                HiddenMarkerArg::BestEffort => HiddenMarkerMode::BestEffort,
            })
            .unwrap_or(HiddenMarkerMode::Disabled);

        let auth = args
            .authentication
            .map(|a| match a {
                AuthenticationArg::None => stegoeggo::AuthenticationMode::None,
                AuthenticationArg::Hmac => stegoeggo::AuthenticationMode::Hmac,
            })
            .unwrap_or(stegoeggo::AuthenticationMode::None);

        let mut rights_metadata = policy != RightsPolicy::Unspecified
            || _legal_metadata.is_some()
            || !matches!(hidden, HiddenMarkerMode::Disabled);

        if args.legal_claims {
            rights_metadata = true;
        }

        ProtectionChannels {
            rights_metadata,
            hidden_marker: hidden,
            authentication: auth,
        }
    };

    Ok((policy, channels))
}

fn resolve_legacy_dmi(args: &Args, level: ProtectionLevel) -> Option<DmiValue> {
    match args.dmi.as_ref() {
        None => {
            let policy = level.default_policy();
            if policy == RightsPolicy::Unspecified {
                Some(DmiValue::Unspecified)
            } else {
                Some(DmiValue::from(policy))
            }
        }
        Some(dmi_arg) => {
            let dmi_val = dmi_arg.clone().into_dmi_value();
            Some(dmi_val.unwrap_or_else(|| {
                let policy = level.default_policy();
                if policy == RightsPolicy::Unspecified {
                    DmiValue::Unspecified
                } else {
                    DmiValue::from(policy)
                }
            }))
        }
    }
}

#[allow(deprecated)]
fn build_legacy_style_request(
    args: &Args,
    _legal_metadata: &Option<stegoeggo::LegalMetadata>,
    legal_dmi_override: Option<DmiValue>,
) -> Result<(RightsPolicy, ProtectionChannels), Box<dyn std::error::Error>> {
    let protection_level = ProtectionLevel::from(args.level.clone());

    let dmi_from_arg = resolve_legacy_dmi(args, protection_level);
    let effective_dmi = legal_dmi_override.or(dmi_from_arg);
    let policy = effective_dmi
        .map(RightsPolicy::from_dmi_value)
        .unwrap_or(RightsPolicy::Unspecified);

    let hidden_marker = match protection_level {
        ProtectionLevel::Disabled => HiddenMarkerMode::Disabled,
        ProtectionLevel::Light => HiddenMarkerMode::SeedOnly,
        ProtectionLevel::Standard => HiddenMarkerMode::BestEffort,
        _ => HiddenMarkerMode::Disabled,
    };

    let authentication = match args.profile {
        ProfileArg::AuthenticatedProvenance | ProfileArg::Maximal => {
            stegoeggo::AuthenticationMode::Hmac
        }
        _ => stegoeggo::AuthenticationMode::None,
    };

    let mut rights_metadata = !matches!(protection_level, ProtectionLevel::Disabled);
    if let Some(meta) = _legal_metadata {
        if meta.has_content() {
            rights_metadata = true;
        }
    }
    if args.metadata == Some(false) {
        rights_metadata = false;
    }
    if args.legal_claims {
        rights_metadata = true;
    }

    let channels = ProtectionChannels {
        rights_metadata,
        hidden_marker,
        authentication,
    };

    Ok((policy, channels))
}

#[allow(deprecated)]
pub(crate) fn evidence_profile_for_display(args: &Args) -> stegoeggo::EvidenceProfile {
    if let Some(preset_arg) = args.preset {
        return match preset_arg {
            PresetArg::LegalNotice => stegoeggo::EvidenceProfile::LegalNotice,
            PresetArg::LegalNoticeWithStego => stegoeggo::EvidenceProfile::LegalNoticeWithStego,
            PresetArg::AuthenticatedProvenance => {
                stegoeggo::EvidenceProfile::AuthenticatedProvenance
            }
            PresetArg::Maximal => stegoeggo::EvidenceProfile::Maximal,
        };
    }
    if args.dry_run {
        return stegoeggo::EvidenceProfile::LegalNotice;
    }
    if args.authentication.is_some() || args.hidden_marker.is_some() || args.rights_policy.is_some()
    {
        if matches!(args.authentication, Some(AuthenticationArg::Hmac)) {
            return stegoeggo::EvidenceProfile::AuthenticatedProvenance;
        }
        if matches!(args.hidden_marker, Some(HiddenMarkerArg::BestEffort)) {
            return stegoeggo::EvidenceProfile::LegalNoticeWithStego;
        }
        return stegoeggo::EvidenceProfile::LegalNotice;
    }
    stegoeggo::EvidenceProfile::from(args.profile.clone())
}

#[allow(deprecated)]
pub(crate) fn display_warnings(
    warnings: &[ProtectionWarning],
    profile: stegoeggo::EvidenceProfile,
    verbose: bool,
) {
    if warnings.is_empty() {
        return;
    }
    for w in warnings {
        let severity = w.severity_for_profile(profile);
        let prefix = match severity {
            WarningSeverity::Error => "Error",
            WarningSeverity::Warning => "Warning",
            WarningSeverity::Info => "Info",
        };
        if verbose || severity != WarningSeverity::Info {
            eprintln!("[{}] {}", prefix, w);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{AuthenticationArg, HiddenMarkerArg, PresetArg, RightsPolicyArg};
    use std::path::PathBuf;

    pub(crate) fn default_args() -> Args {
        Args {
            input: vec![PathBuf::from("test.png")],
            output: None,
            verify: false,
            level: ProtectionLevelArg::Standard,
            profile: ProfileArg::LegalNotice,
            intensity: 0.5,
            seed: Some(42),
            format: None,
            stego_redundancy: 2,
            jpeg_quality: 90,
            progressive: false,
            verbose: false,
            dmi: None,
            metadata: None,
            legal_claims: false,
            copyright_notice: None,
            creator: None,
            contact: None,
            rights_url: None,
            usage_terms: None,
            ai_constraints: None,
            no_ai_training: false,
            no_genai_training: false,
            tdm_reserved: false,
            credit_line: None,
            copyright_owner: None,
            licensor_name: None,
            licensor_email: None,
            licensor_url: None,
            content_created_at: None,
            key: None,
            jobs: 1,
            strict: false,
            json: false,
            rights_policy: None,
            preset: None,
            hidden_marker: None,
            authentication: None,
            dry_run: false,
            #[cfg(feature = "signatures")]
            command: None,
        }
    }

    #[test]
    fn test_legacy_default_standard_is_prohibited() {
        let args = default_args();
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedAiMlTraining);
        assert!(req.channels().rights_metadata);
        assert_eq!(req.channels().hidden_marker, HiddenMarkerMode::BestEffort);
    }

    #[test]
    fn test_legacy_default_standard_dmi_auto_matches_omitted() {
        let mut args_omitted = default_args();
        args_omitted.dmi = None;
        let req_omitted = build_protection_request(&args_omitted).unwrap();

        let mut args_auto = default_args();
        args_auto.dmi = Some(DmiArg::Auto);
        let req_auto = build_protection_request(&args_auto).unwrap();

        assert_eq!(req_omitted.policy(), req_auto.policy());
        assert_eq!(
            req_omitted.channels().rights_metadata,
            req_auto.channels().rights_metadata
        );
        assert_eq!(
            req_omitted.channels().hidden_marker,
            req_auto.channels().hidden_marker
        );
    }

    #[test]
    fn test_legacy_dmi_unspecified_is_distinct_from_default() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::Unspecified);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::Unspecified);
    }

    #[test]
    fn test_legacy_light_level() {
        let mut args = default_args();
        args.level = ProtectionLevelArg::Light;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::Unspecified);
        assert!(req.channels().rights_metadata);
        assert_eq!(req.channels().hidden_marker, HiddenMarkerMode::SeedOnly);
    }

    #[test]
    fn test_stego_redundancy_is_applied_to_request() {
        let mut args = default_args();
        args.stego_redundancy = 8;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.processing().stego_redundancy, Some(8));
    }

    #[test]
    fn test_stego_redundancy_out_of_range_is_config_error() {
        for out_of_range in [0usize, 11, 100] {
            let mut args = default_args();
            args.stego_redundancy = out_of_range;
            let err = build_protection_request(&args).unwrap_err();
            let downcast = err.downcast_ref::<stegoeggo::Error>().expect(
                "redundancy validation must be stegoeggo::Error for exit-code classification",
            );
            assert!(
                matches!(downcast, stegoeggo::Error::Config(_)),
                "expected Config error for redundancy {out_of_range}, got: {downcast:?}"
            );
        }
    }

    #[test]
    fn test_legacy_disabled_level() {
        let mut args = default_args();
        args.level = ProtectionLevelArg::Disabled;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::Unspecified);
        assert!(!req.channels().rights_metadata);
        assert_eq!(req.channels().hidden_marker, HiddenMarkerMode::Disabled);
    }

    #[test]
    fn test_no_ai_training_shorthand_sets_policy() {
        let mut args = default_args();
        args.no_ai_training = true;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedAiMlTraining);
    }

    #[test]
    fn test_no_genai_training_shorthand_sets_policy() {
        let mut args = default_args();
        args.no_genai_training = true;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedGenerativeAiTraining);
    }

    #[test]
    fn test_tdm_reserved_shorthand_sets_policy() {
        let mut args = default_args();
        args.tdm_reserved = true;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedSeeConstraints);
    }

    #[test]
    fn test_explicit_rights_policy_prohibited() {
        let mut args = default_args();
        args.rights_policy = Some(RightsPolicyArg::ProhibitedAiMlTraining);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedAiMlTraining);
    }

    #[test]
    fn test_explicit_rights_policy_allowed() {
        let mut args = default_args();
        args.rights_policy = Some(RightsPolicyArg::Allowed);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::Allowed);
    }

    #[test]
    fn test_conflicting_dmi_and_rights_policy() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::Allowed);
        args.rights_policy = Some(RightsPolicyArg::ProhibitedAiMlTraining);
        let result = build_protection_request(&args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Conflicting"), "Error: {}", err);
    }

    #[test]
    fn test_conflicting_shorthand_and_rights_policy() {
        let mut args = default_args();
        args.no_ai_training = true;
        args.rights_policy = Some(RightsPolicyArg::Allowed);
        let result = build_protection_request(&args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Conflicting"), "Error: {}", err);
    }

    #[test]
    fn test_metadata_false_with_legal_fields() {
        let mut args = default_args();
        args.metadata = Some(false);
        args.copyright_notice = Some("test".to_string());
        let result = build_protection_request(&args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Cannot use --metadata false"),
            "Error: {}",
            err
        );
    }

    #[test]
    fn test_metadata_false_with_preset_is_config_error() {
        let mut args = default_args();
        args.metadata = Some(false);
        args.preset = Some(PresetArg::LegalNotice);
        let result = build_protection_request(&args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Cannot use --metadata false"),
            "Error: {}",
            err
        );
    }

    #[test]
    fn test_hmac_without_key_is_error() {
        let mut args = default_args();
        args.preset = Some(PresetArg::AuthenticatedProvenance);
        let result = build_protection_request(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_hmac_with_disabled_marker_is_error() {
        let mut args = default_args();
        args.hidden_marker = Some(HiddenMarkerArg::Disabled);
        args.authentication = Some(AuthenticationArg::Hmac);
        args.key = Some("deadbeef01234567deadbeef01234567".to_string());
        let result = build_protection_request(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_preset_and_level_conflict() {
        let mut args = default_args();
        args.preset = Some(PresetArg::Maximal);
        args.level = ProtectionLevelArg::Light;
        let result = build_protection_request(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_preset_and_explicit_default_level_conflict() {
        let mut args = default_args();
        args.preset = Some(PresetArg::Maximal);
        let result = build_protection_request_with_explicit_options(&args, true, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_hidden_marker_and_explicit_level_conflict() {
        let mut args = default_args();
        args.hidden_marker = Some(HiddenMarkerArg::BestEffort);
        let result = build_protection_request_with_explicit_options(&args, true, false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Cannot combine"), "Error: {}", err);
    }

    #[test]
    fn test_authentication_and_explicit_profile_conflict() {
        let mut args = default_args();
        args.authentication = Some(AuthenticationArg::Hmac);
        args.key = Some("deadbeef01234567deadbeef01234567".to_string());
        let result = build_protection_request_with_explicit_options(&args, false, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_hidden_marker_without_level_explicit_succeeds() {
        let mut args = default_args();
        args.hidden_marker = Some(HiddenMarkerArg::BestEffort);
        let req = build_protection_request_with_explicit_options(&args, false, false).unwrap();
        assert_eq!(req.channels().hidden_marker, HiddenMarkerMode::BestEffort);
    }

    #[test]
    fn test_policy_conflict_errors_are_config_errors() {
        let mut args = default_args();
        args.preset = Some(PresetArg::Maximal);
        args.level = ProtectionLevelArg::Light;
        let err = build_protection_request(&args).unwrap_err();
        let downcast = err
            .downcast_ref::<stegoeggo::Error>()
            .expect("policy conflicts must be stegoeggo::Error for exit-code classification");
        assert!(
            matches!(downcast, stegoeggo::Error::Config(_)),
            "expected Config error, got: {downcast:?}"
        );
    }

    #[test]
    fn test_seed_and_intensity_preserved() {
        let mut args = default_args();
        args.seed = Some(99);
        args.intensity = 0.8;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.seed(), Some(99));
        assert_eq!(req.intensity(), 0.8);
    }

    #[test]
    fn test_jpeg_quality_preserved() {
        let mut args = default_args();
        args.jpeg_quality = 75;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.processing().jpeg_quality, 75);
    }

    #[test]
    fn test_legacy_dmi_prohibited_ai_matches_rights_policy() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::ProhibitedAi);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedAiMlTraining);
    }

    #[test]
    fn test_legacy_dmi_prohibited_gen_ai() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::ProhibitedGenAi);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedGenerativeAiTraining);
    }

    #[test]
    fn test_legacy_dmi_allowed() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::Allowed);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::Allowed);
    }

    #[test]
    fn test_legacy_dmi_prohibited() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::Prohibited);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedAllDataMining);
    }

    #[test]
    fn test_preset_legal_notice_request() {
        let mut args = default_args();
        args.preset = Some(PresetArg::LegalNotice);
        let req = build_protection_request(&args).unwrap();
        assert!(req.channels().rights_metadata);
        assert_eq!(req.channels().hidden_marker, HiddenMarkerMode::Disabled);
    }

    #[test]
    fn test_preset_legal_notice_stego_request() {
        let mut args = default_args();
        args.preset = Some(PresetArg::LegalNoticeWithStego);
        let req = build_protection_request(&args).unwrap();
        assert!(req.channels().rights_metadata);
        assert_eq!(req.channels().hidden_marker, HiddenMarkerMode::BestEffort);
    }

    #[test]
    fn test_preset_and_hidden_marker_conflict() {
        let mut args = default_args();
        args.preset = Some(PresetArg::LegalNotice);
        args.hidden_marker = Some(HiddenMarkerArg::BestEffort);
        let result = build_protection_request(&args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Cannot combine --preset"), "Error: {}", err);
    }

    #[test]
    fn test_preset_and_authentication_conflict() {
        let mut args = default_args();
        args.preset = Some(PresetArg::Maximal);
        args.authentication = Some(AuthenticationArg::Hmac);
        args.key = Some("deadbeef01234567deadbeef01234567".to_string());
        let result = build_protection_request(&args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Cannot combine --preset"), "Error: {}", err);
    }

    #[test]
    fn test_explicit_channel_best_effort_matches_legacy_standard() {
        let mut legacy = default_args();
        legacy.level = ProtectionLevelArg::Standard;
        let legacy_req = build_protection_request(&legacy).unwrap();

        let mut modern = default_args();
        modern.rights_policy = Some(RightsPolicyArg::ProhibitedAiMlTraining);
        modern.hidden_marker = Some(HiddenMarkerArg::BestEffort);
        let modern_req = build_protection_request(&modern).unwrap();

        assert_eq!(legacy_req.policy(), modern_req.policy());
        assert_eq!(
            legacy_req.channels().hidden_marker,
            modern_req.channels().hidden_marker
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_display_profile_reflects_channel_flags() {
        let mut args = default_args();
        args.rights_policy = Some(RightsPolicyArg::Allowed);
        assert_eq!(
            evidence_profile_for_display(&args),
            stegoeggo::EvidenceProfile::LegalNotice
        );

        let mut args = default_args();
        args.rights_policy = Some(RightsPolicyArg::ProhibitedSeeConstraints);
        args.hidden_marker = Some(HiddenMarkerArg::BestEffort);
        assert_eq!(
            evidence_profile_for_display(&args),
            stegoeggo::EvidenceProfile::LegalNoticeWithStego
        );

        let mut args = default_args();
        args.rights_policy = Some(RightsPolicyArg::Unspecified);
        args.hidden_marker = Some(HiddenMarkerArg::Disabled);
        args.authentication = Some(AuthenticationArg::Hmac);
        assert_eq!(
            evidence_profile_for_display(&args),
            stegoeggo::EvidenceProfile::AuthenticatedProvenance
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_display_profile_preset_and_dry_run_and_legacy() {
        let mut args = default_args();
        args.preset = Some(PresetArg::Maximal);
        assert_eq!(
            evidence_profile_for_display(&args),
            stegoeggo::EvidenceProfile::Maximal
        );

        let mut args = default_args();
        args.dry_run = true;
        assert_eq!(
            evidence_profile_for_display(&args),
            stegoeggo::EvidenceProfile::LegalNotice
        );

        let mut args = default_args();
        args.profile = ProfileArg::AuthenticatedProvenance;
        assert_eq!(
            evidence_profile_for_display(&args),
            stegoeggo::EvidenceProfile::AuthenticatedProvenance
        );
    }
}
