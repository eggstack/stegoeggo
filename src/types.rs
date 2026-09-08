mod compat;
mod context;
mod legal;
mod request;
mod rights;
mod verification;
mod warnings;

pub use self::compat::*;
pub use self::context::*;
pub use self::legal::*;
pub use self::request::*;
pub use self::rights::*;
pub use self::verification::*;
pub use self::warnings::*;

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn builder_chain() {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_stego_redundancy(3);
        assert_eq!(ctx.intensity(), 0.5);
        assert_eq!(ctx.seed(), 42);
        assert_eq!(ctx.stego_redundancy(), 3);
    }

    #[test]
    fn invalid_numeric_context_values_are_preserved_for_validation() {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_stego_redundancy(0)
            .with_jpeg_quality(0);
        assert_eq!(ctx.stego_redundancy_field(), Some(0));
        assert_eq!(ctx.jpeg_quality(), 0);
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn effective_redundancy_clamps_out_of_range_values() {
        let ctx = ProtectionContext::new(0.5, 42).with_stego_redundancy(0);
        assert_eq!(ctx.stego_redundancy(), 1);
        assert!(ctx.validate().is_err());
        let ctx = ProtectionContext::new(0.5, 42).with_stego_redundancy(99);
        assert_eq!(ctx.stego_redundancy(), 10);
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn intensity_clamped() {
        let ctx = ProtectionContext::new(2.0, 42);
        assert_eq!(ctx.intensity(), 1.0);

        let ctx = ProtectionContext::new(-1.0, 42);
        assert_eq!(ctx.intensity(), 0.0);
    }

    #[test]
    fn seed_roundtrip_through_serde() {
        let ctx = ProtectionContext::new(0.7, 12345);
        let json = serde_json::to_string(&ctx).unwrap();
        let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.seed(), 12345);
        assert_eq!(restored.intensity(), 0.7);
    }

    #[test]
    fn serialize_emits_warning_when_config_set() {
        let ctx = ProtectionContext::new(0.5, 99).with_mac_key(b"key".to_vec());
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(
            json.contains("_config_dropped_warning"),
            "Serialized JSON should contain a warning field when config is set: {json}"
        );
        assert!(
            json.contains("MAC key"),
            "Warning should mention the MAC key: {json}"
        );

        let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.seed(), 99);
        assert_eq!(restored.intensity(), 0.5);
        assert!(
            restored.mac_key().is_none(),
            "MAC key should be lost after serde roundtrip even when warning is emitted"
        );
    }

    #[test]
    fn serialize_no_warning_when_config_none() {
        let ctx = ProtectionContext::new(0.5, 99);
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(
            !json.contains("_config_dropped_warning"),
            "No warning should be emitted when config is None: {json}"
        );
    }

    // ── Tile size configuration ───────────────────────────────────────

    #[test]
    fn tile_size_default_is_none() {
        let ctx = ProtectionContext::new(0.5, 42);
        assert_eq!(ctx.tile_size(), None);
        assert!(!ctx.is_tile_mode_enabled());
    }

    #[test]
    fn with_tile_size_zero_disables_tiling() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(0);
        assert_eq!(ctx.tile_size(), Some(0));
        assert!(!ctx.is_tile_mode_enabled());
    }

    #[test]
    fn with_tile_size_enables_tiling() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(64);
        assert_eq!(ctx.tile_size(), Some(64));
        assert!(ctx.is_tile_mode_enabled());
    }

    #[test]
    fn with_tile_size_preserves_below_minimum_for_validation() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(8);
        assert_eq!(ctx.tile_size(), Some(8));
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn with_tile_size_preserves_above_maximum_for_validation() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_size(4096);
        assert_eq!(ctx.tile_size(), Some(4096));
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn with_tile_extraction_max_origins_defaults_to_64() {
        let ctx = ProtectionContext::new(0.5, 42);
        assert_eq!(ctx.tile_extraction_max_origins(), 64);
    }

    #[test]
    fn with_tile_extraction_max_origins_preserves_zero_for_validation() {
        let ctx = ProtectionContext::new(0.5, 42).with_tile_extraction_max_origins(0);
        assert_eq!(ctx.tile_extraction_max_origins(), 0);
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn tile_settings_survive_serde_roundtrip() {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_tile_size(64)
            .with_tile_extraction_max_origins(128);
        let json = serde_json::to_string(&ctx).unwrap();
        let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tile_size(), Some(64));
        assert_eq!(restored.tile_extraction_max_origins(), 128);
    }

    #[test]
    fn protection_level_byte_roundtrip() {
        let levels = [
            ProtectionLevel::Disabled,
            ProtectionLevel::Light,
            ProtectionLevel::Standard,
        ];
        for level in &levels {
            let byte = level.to_byte();
            let restored = ProtectionLevel::from_byte(byte);
            assert_eq!(restored.as_ref(), Some(level));
        }
    }

    #[test]
    fn protection_level_from_invalid_byte() {
        assert!(ProtectionLevel::from_byte(3).is_none());
        assert!(ProtectionLevel::from_byte(255).is_none());
    }

    #[test]
    fn dmi_value_iptc_property_mapping() {
        use crate::types::DmiValue;

        let allowed = DmiValue::Allowed;
        assert!(allowed.to_iptc_property().contains("DMI-Allowed"));

        let prohibited_training = DmiValue::ProhibitedAiMlTraining;
        assert!(prohibited_training
            .to_iptc_property()
            .contains("DMI-Prohibited"));

        let prohibited_gen = DmiValue::ProhibitedGenAiMlTraining;
        assert!(prohibited_gen.to_iptc_property().contains("DMI-Prohibited"));

        let prohibited_all = DmiValue::Prohibited;
        assert!(prohibited_all.to_iptc_property().contains("DMI-Prohibited"));

        let prohibited_se = DmiValue::ProhibitedExceptSearchEngineIndexing;
        assert!(prohibited_se.to_iptc_property().contains("DMI-Prohibited"));

        let prohibited_see = DmiValue::ProhibitedSeeConstraints;
        assert!(prohibited_see.to_iptc_property().contains("DMI-Prohibited"));

        let unspecified = DmiValue::Unspecified;
        assert!(unspecified.to_iptc_property().contains("DMI"));
    }

    #[test]
    fn evidence_profile_default_is_legal_notice() {
        let ctx = ProtectionContext::new(0.5, 42);
        assert_eq!(ctx.evidence_profile(), EvidenceProfile::LegalNotice);
    }

    #[test]
    fn with_evidence_profile_sets_and_retrieves() {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_evidence_profile(EvidenceProfile::AuthenticatedProvenance);
        assert_eq!(
            ctx.evidence_profile(),
            EvidenceProfile::AuthenticatedProvenance
        );
    }

    #[test]
    fn evidence_profile_serialization_roundtrip() {
        let profiles = [
            EvidenceProfile::LegalNotice,
            EvidenceProfile::LegalNoticeWithStego,
            EvidenceProfile::AuthenticatedProvenance,
            EvidenceProfile::Maximal,
        ];
        for profile in &profiles {
            let json = serde_json::to_string(profile).unwrap();
            let restored: EvidenceProfile = serde_json::from_str(&json).unwrap();
            assert_eq!(&restored, profile);
        }
    }

    #[test]
    fn evidence_profile_as_str() {
        assert_eq!(EvidenceProfile::LegalNotice.as_str(), "legal-notice");
        assert_eq!(
            EvidenceProfile::LegalNoticeWithStego.as_str(),
            "legal-notice-stego"
        );
        assert_eq!(
            EvidenceProfile::AuthenticatedProvenance.as_str(),
            "authenticated-provenance"
        );
        assert_eq!(EvidenceProfile::Maximal.as_str(), "maximal");
    }

    #[test]
    fn evidence_profile_serde_roundtrip_in_context() {
        let ctx = ProtectionContext::new(0.5, 42)
            .with_evidence_profile(EvidenceProfile::AuthenticatedProvenance);
        let json = serde_json::to_string(&ctx).unwrap();
        let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(
            restored.evidence_profile(),
            EvidenceProfile::AuthenticatedProvenance
        );
    }

    #[test]
    fn evidence_profile_default_context_backward_compatible() {
        let ctx = ProtectionContext::new(0.5, 42);
        let json = serde_json::to_string(&ctx).unwrap();
        let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.evidence_profile(), EvidenceProfile::LegalNotice);
        assert_eq!(restored.intensity(), 0.5);
        assert_eq!(restored.seed(), 42);
    }

    #[test]
    fn helper_constructors_set_correct_profile() {
        assert_eq!(
            ProtectionContext::legal_notice().evidence_profile(),
            EvidenceProfile::LegalNotice
        );
        assert_eq!(
            ProtectionContext::legal_notice_with_stego().evidence_profile(),
            EvidenceProfile::LegalNoticeWithStego
        );
        assert_eq!(
            ProtectionContext::authenticated_provenance().evidence_profile(),
            EvidenceProfile::AuthenticatedProvenance
        );
        assert_eq!(
            ProtectionContext::maximal().evidence_profile(),
            EvidenceProfile::Maximal
        );
    }

    #[test]
    fn warning_category_mapping() {
        assert_eq!(
            ProtectionWarning::MissingMacKey.category(),
            WarningCategory::AuthenticatedProvenance
        );
        assert_eq!(
            ProtectionWarning::MetadataInjectionDisabled.category(),
            WarningCategory::LegalNotice
        );
        assert_eq!(
            ProtectionWarning::ProgressiveJpegFallback.category(),
            WarningCategory::FormatFragility
        );
        assert_eq!(
            ProtectionWarning::JpegReencodeFragile.category(),
            WarningCategory::FormatFragility
        );
        assert_eq!(
            ProtectionWarning::LsbCapacitySkipped.category(),
            WarningCategory::BestEffortStego
        );
        assert_eq!(
            ProtectionWarning::DctCapacityInsufficient.category(),
            WarningCategory::BestEffortStego
        );
    }

    #[test]
    fn missing_mac_key_severity_by_profile() {
        let w = ProtectionWarning::MissingMacKey;
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::AuthenticatedProvenance),
            WarningSeverity::Warning
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::Maximal),
            WarningSeverity::Warning
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::LegalNotice),
            WarningSeverity::Info
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::LegalNoticeWithStego),
            WarningSeverity::Info
        );
    }

    #[test]
    fn metadata_injection_disabled_severity_by_profile() {
        let w = ProtectionWarning::MetadataInjectionDisabled;
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::LegalNotice),
            WarningSeverity::Error
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::LegalNoticeWithStego),
            WarningSeverity::Error
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::AuthenticatedProvenance),
            WarningSeverity::Warning
        );
        assert_eq!(
            w.severity_for_profile(EvidenceProfile::Maximal),
            WarningSeverity::Warning
        );
    }

    #[test]
    fn format_fragility_severity_is_always_warning() {
        for w in [
            ProtectionWarning::ProgressiveJpegFallback,
            ProtectionWarning::JpegReencodeFragile,
        ] {
            for profile in [
                EvidenceProfile::LegalNotice,
                EvidenceProfile::LegalNoticeWithStego,
                EvidenceProfile::AuthenticatedProvenance,
                EvidenceProfile::Maximal,
            ] {
                assert_eq!(
                    w.severity_for_profile(profile),
                    WarningSeverity::Warning,
                    "{:?} should be Warning for {:?}",
                    w,
                    profile
                );
            }
        }
    }

    #[test]
    fn stego_capacity_severity_by_profile() {
        for w in [
            ProtectionWarning::LsbCapacitySkipped,
            ProtectionWarning::DctCapacityInsufficient,
        ] {
            assert_eq!(
                w.severity_for_profile(EvidenceProfile::LegalNotice),
                WarningSeverity::Info,
                "{:?} should be Info for LegalNotice",
                w
            );
            assert_eq!(
                w.severity_for_profile(EvidenceProfile::LegalNoticeWithStego),
                WarningSeverity::Warning,
                "{:?} should be Warning for LegalNoticeWithStego",
                w
            );
            assert_eq!(
                w.severity_for_profile(EvidenceProfile::AuthenticatedProvenance),
                WarningSeverity::Warning,
                "{:?} should be Warning for AuthenticatedProvenance",
                w
            );
            assert_eq!(
                w.severity_for_profile(EvidenceProfile::Maximal),
                WarningSeverity::Warning,
                "{:?} should be Warning for Maximal",
                w
            );
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod plus_mapping_tests {
    use super::*;

    #[test]
    fn all_variants_have_plus_vocab_key() {
        let variants = [
            DmiValue::Unspecified,
            DmiValue::Allowed,
            DmiValue::ProhibitedAiMlTraining,
            DmiValue::ProhibitedGenAiMlTraining,
            DmiValue::ProhibitedExceptSearchEngineIndexing,
            DmiValue::Prohibited,
            DmiValue::ProhibitedSeeConstraints,
        ];
        for v in variants {
            let key = v.plus_vocab_key();
            assert!(key.starts_with("DMI-"), "key must start with DMI-: {key}");
            assert_eq!(DmiValue::from_plus_vocab_key(key), Some(v));
        }
    }

    #[test]
    fn from_plus_vocab_key_rejects_unknown() {
        assert_eq!(DmiValue::from_plus_vocab_key("DMI-UNKNOWN"), None);
        assert_eq!(DmiValue::from_plus_vocab_key(""), None);
        assert_eq!(DmiValue::from_plus_vocab_key("Prohibited"), None);
    }

    #[test]
    fn plus_vocab_keys_match_exiftool() {
        assert_eq!(
            DmiValue::ProhibitedSeeConstraints.plus_vocab_key(),
            "DMI-PROHIBITED-SEECONSTRAINT"
        );
        assert_eq!(
            DmiValue::ProhibitedAiMlTraining.plus_vocab_key(),
            "DMI-PROHIBITED-AIMLTRAINING"
        );
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod rights_policy_tests {
    use super::*;

    #[test]
    fn unspecified_to_dmi_returns_none() {
        assert_eq!(RightsPolicy::Unspecified.to_dmi_value(), None);
    }

    #[test]
    fn allowed_to_dmi() {
        assert_eq!(
            RightsPolicy::Allowed.to_dmi_value(),
            Some(DmiValue::Allowed)
        );
    }

    #[test]
    fn prohibited_ai_ml_training_to_dmi() {
        assert_eq!(
            RightsPolicy::ProhibitedAiMlTraining.to_dmi_value(),
            Some(DmiValue::ProhibitedAiMlTraining)
        );
    }

    #[test]
    fn prohibited_generative_ai_training_to_dmi() {
        assert_eq!(
            RightsPolicy::ProhibitedGenerativeAiTraining.to_dmi_value(),
            Some(DmiValue::ProhibitedGenAiMlTraining)
        );
    }

    #[test]
    fn prohibited_except_search_indexing_to_dmi() {
        assert_eq!(
            RightsPolicy::ProhibitedExceptSearchIndexing.to_dmi_value(),
            Some(DmiValue::ProhibitedExceptSearchEngineIndexing)
        );
    }

    #[test]
    fn prohibited_all_data_mining_to_dmi() {
        assert_eq!(
            RightsPolicy::ProhibitedAllDataMining.to_dmi_value(),
            Some(DmiValue::Prohibited)
        );
    }

    #[test]
    fn prohibited_see_constraints_to_dmi() {
        assert_eq!(
            RightsPolicy::ProhibitedSeeConstraints.to_dmi_value(),
            Some(DmiValue::ProhibitedSeeConstraints)
        );
    }

    #[test]
    fn from_dmi_unspecified() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::Unspecified),
            RightsPolicy::Unspecified
        );
    }

    #[test]
    fn from_dmi_allowed() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::Allowed),
            RightsPolicy::Allowed
        );
    }

    #[test]
    fn from_dmi_prohibited_ai_ml_training() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::ProhibitedAiMlTraining),
            RightsPolicy::ProhibitedAiMlTraining
        );
    }

    #[test]
    fn from_dmi_prohibited_gen_ai_ml_training() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::ProhibitedGenAiMlTraining),
            RightsPolicy::ProhibitedGenerativeAiTraining
        );
    }

    #[test]
    fn from_dmi_prohibited_except_search_engine_indexing() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::ProhibitedExceptSearchEngineIndexing),
            RightsPolicy::ProhibitedExceptSearchIndexing
        );
    }

    #[test]
    fn from_dmi_prohibited() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::Prohibited),
            RightsPolicy::ProhibitedAllDataMining
        );
    }

    #[test]
    fn from_dmi_prohibited_see_constraints() {
        assert_eq!(
            RightsPolicy::from_dmi_value(DmiValue::ProhibitedSeeConstraints),
            RightsPolicy::ProhibitedSeeConstraints
        );
    }

    #[test]
    fn roundtrip_to_dmi_from_dmi() {
        let policies = [
            RightsPolicy::Allowed,
            RightsPolicy::ProhibitedAiMlTraining,
            RightsPolicy::ProhibitedGenerativeAiTraining,
            RightsPolicy::ProhibitedExceptSearchIndexing,
            RightsPolicy::ProhibitedAllDataMining,
            RightsPolicy::ProhibitedSeeConstraints,
        ];
        for policy in policies {
            let dmi = policy.to_dmi_value().unwrap();
            let roundtripped = RightsPolicy::from_dmi_value(dmi);
            assert_eq!(roundtripped, policy);
        }
    }

    #[test]
    fn roundtrip_from_dmi_to_dmi() {
        let dmi_values = [
            DmiValue::Allowed,
            DmiValue::ProhibitedAiMlTraining,
            DmiValue::ProhibitedGenAiMlTraining,
            DmiValue::ProhibitedExceptSearchEngineIndexing,
            DmiValue::Prohibited,
            DmiValue::ProhibitedSeeConstraints,
        ];
        for dmi in dmi_values {
            let policy = RightsPolicy::from_dmi_value(dmi);
            let roundtripped = policy.to_dmi_value().unwrap();
            assert_eq!(roundtripped, dmi);
        }
    }

    #[test]
    fn from_trait_matches_function() {
        for dmi in [
            DmiValue::Allowed,
            DmiValue::ProhibitedAiMlTraining,
            DmiValue::ProhibitedGenAiMlTraining,
        ] {
            let via_from: RightsPolicy = dmi.into();
            let via_fn = RightsPolicy::from_dmi_value(dmi);
            assert_eq!(via_from, via_fn);
        }
    }

    #[test]
    fn into_trait_matches_to_dmi_value() {
        for policy in [
            RightsPolicy::Allowed,
            RightsPolicy::ProhibitedAiMlTraining,
            RightsPolicy::ProhibitedGenerativeAiTraining,
        ] {
            let via_into: DmiValue = policy.into();
            let via_fn = policy.to_dmi_value().unwrap();
            assert_eq!(via_into, via_fn);
        }
    }

    #[test]
    fn requires_constraints_only_for_see_constraints() {
        assert!(!RightsPolicy::Allowed.requires_constraints());
        assert!(!RightsPolicy::ProhibitedAiMlTraining.requires_constraints());
        assert!(RightsPolicy::ProhibitedSeeConstraints.requires_constraints());
    }

    #[test]
    fn as_str_matches_variant_name() {
        assert_eq!(RightsPolicy::Unspecified.as_str(), "Unspecified");
        assert_eq!(RightsPolicy::Allowed.as_str(), "Allowed");
        assert_eq!(
            RightsPolicy::ProhibitedAllDataMining.as_str(),
            "ProhibitedAllDataMining"
        );
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod protection_preset_tests {
    use super::*;

    #[test]
    fn legal_notice_expands_to_metadata_only() {
        let channels = ProtectionPreset::LegalNotice.to_channels();
        assert!(channels.rights_metadata);
        assert_eq!(channels.hidden_marker, HiddenMarkerMode::Disabled);
        assert_eq!(channels.authentication, AuthenticationMode::None);
        assert!(!channels.has_stego());
    }

    #[test]
    fn legal_notice_with_stego_expands_correctly() {
        let channels = ProtectionPreset::LegalNoticeWithStego.to_channels();
        assert!(channels.rights_metadata);
        assert_eq!(channels.hidden_marker, HiddenMarkerMode::BestEffort);
        assert_eq!(channels.authentication, AuthenticationMode::None);
        assert!(channels.has_stego());
    }

    #[test]
    fn authenticated_provenance_expands_correctly() {
        let channels = ProtectionPreset::AuthenticatedProvenance.to_channels();
        assert!(channels.rights_metadata);
        assert_eq!(channels.hidden_marker, HiddenMarkerMode::BestEffort);
        assert_eq!(channels.authentication, AuthenticationMode::Hmac);
        assert!(channels.has_stego());
    }

    #[test]
    fn maximal_expands_correctly() {
        let channels = ProtectionPreset::Maximal.to_channels();
        assert!(channels.rights_metadata);
        assert_eq!(channels.hidden_marker, HiddenMarkerMode::BestEffort);
        assert_eq!(channels.authentication, AuthenticationMode::Hmac);
        assert!(channels.has_stego());
    }

    #[test]
    fn requires_mac_key_only_for_authenticated_presets() {
        assert!(!ProtectionPreset::LegalNotice.requires_mac_key());
        assert!(!ProtectionPreset::LegalNoticeWithStego.requires_mac_key());
        assert!(ProtectionPreset::AuthenticatedProvenance.requires_mac_key());
        assert!(ProtectionPreset::Maximal.requires_mac_key());
    }

    #[test]
    fn as_str_returns_lowercase() {
        assert_eq!(ProtectionPreset::LegalNotice.as_str(), "legal-notice");
        assert_eq!(
            ProtectionPreset::LegalNoticeWithStego.as_str(),
            "legal-notice-stego"
        );
        assert_eq!(
            ProtectionPreset::AuthenticatedProvenance.as_str(),
            "authenticated-provenance"
        );
        assert_eq!(ProtectionPreset::Maximal.as_str(), "maximal");
    }

    #[test]
    fn from_preset_uses_preset_channels() {
        let notice = RightsNotice::new();
        let request = ProtectionRequest::from_preset(
            ProtectionPreset::LegalNotice,
            notice,
            RightsPolicy::Allowed,
        );
        assert!(request.channels().rights_metadata);
        assert_eq!(request.channels().hidden_marker, HiddenMarkerMode::Disabled);
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod url_validation_tests {
    use super::*;

    #[test]
    fn valid_https_url_passes() {
        let meta = LegalMetadata::new().with_license_url("https://example.com/license");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn valid_http_url_passes() {
        let meta = LegalMetadata::new().with_license_url("http://example.com/license");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn valid_ftp_url_passes() {
        let meta = LegalMetadata::new().with_license_url("ftp://files.example.com/doc");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn missing_scheme_fails() {
        let meta = LegalMetadata::new().with_license_url("example.com/license");
        let err = meta.validate().unwrap_err();
        assert!(
            err.to_string().contains("must include a scheme"),
            "Expected scheme error, got: {}",
            err
        );
    }

    #[test]
    fn empty_url_fails() {
        let meta = LegalMetadata::new().with_license_url("");
        let err = meta.validate().unwrap_err();
        assert!(
            err.to_string().contains("must not be empty"),
            "Expected empty error, got: {}",
            err
        );
    }

    #[test]
    fn scheme_only_fails() {
        let meta = LegalMetadata::new().with_license_url("https://");
        let err = meta.validate().unwrap_err();
        assert!(
            err.to_string().contains("must include an authority"),
            "Expected authority error, got: {}",
            err
        );
    }

    #[test]
    fn web_statement_validates() {
        let meta = LegalMetadata::new().with_web_statement_of_rights("not-a-url");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("web_statement_of_rights"));
    }

    #[test]
    fn licensor_url_validates() {
        let meta = LegalMetadata::new().with_licensor_url("missing-scheme");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("licensor_url"));
    }

    #[test]
    fn non_url_fields_not_affected() {
        let meta = LegalMetadata::new()
            .with_copyright_holder("Test")
            .with_creator("Author");
        assert!(meta.validate().is_ok());
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod localized_text_tests {
    use super::*;

    #[test]
    fn new_defaults_to_x_default() {
        let lt = LocalizedText::new("All rights reserved");
        assert_eq!(lt.text(), "All rights reserved");
        assert_eq!(lt.lang(), "x-default");
    }

    #[test]
    fn with_lang_sets_language() {
        let lt = LocalizedText::with_lang("Tous droits réservés.", "fr");
        assert_eq!(lt.text(), "Tous droits réservés.");
        assert_eq!(lt.lang(), "fr");
    }

    #[test]
    fn from_string_uses_default_lang() {
        let lt: LocalizedText = "test".into();
        assert_eq!(lt.lang(), "x-default");
    }

    #[test]
    fn display_returns_text() {
        let lt = LocalizedText::new("hello");
        assert_eq!(format!("{}", lt), "hello");
    }

    #[test]
    fn usage_terms_localized_sets_both_fields() {
        let meta = LegalMetadata::new()
            .with_usage_terms_localized(LocalizedText::with_lang("Tous droits réservés.", "fr"));
        assert_eq!(meta.usage_terms(), Some("Tous droits réservés."));
        assert_eq!(meta.usage_terms_lang(), Some("fr"));
    }

    #[test]
    fn usage_terms_localized_from_string_uses_default_lang() {
        let meta = LegalMetadata::new().with_usage_terms_localized("All rights reserved");
        assert_eq!(meta.usage_terms(), Some("All rights reserved"));
        assert_eq!(meta.usage_terms_lang(), Some("x-default"));
    }

    #[test]
    fn usage_terms_plain_has_no_lang() {
        let meta = LegalMetadata::new().with_usage_terms("All rights reserved");
        assert_eq!(meta.usage_terms(), Some("All rights reserved"));
        assert_eq!(meta.usage_terms_lang(), None);
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod date_validation_tests {
    use super::*;

    #[test]
    fn valid_date_only() {
        let meta = LegalMetadata::new().with_creation_date("2024-01-15");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn valid_datetime_utc() {
        let meta = LegalMetadata::new().with_notice_applied_at("2024-01-15T12:30:45Z");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn valid_datetime_offset() {
        let meta = LegalMetadata::new().with_metadata_date("2024-01-15T12:30:45+05:30");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn valid_datetime_negative_offset() {
        let meta = LegalMetadata::new().with_creation_date("2024-01-15T12:30:45-08:00");
        assert!(meta.validate().is_ok());
    }

    #[test]
    fn invalid_date_too_short() {
        let meta = LegalMetadata::new().with_creation_date("2024-01");
        let err = meta.validate().unwrap_err();
        assert!(
            err.to_string().contains("ISO 8601"),
            "Expected ISO 8601 error, got: {}",
            err
        );
    }

    #[test]
    fn invalid_date_wrong_separator() {
        let meta = LegalMetadata::new().with_creation_date("2024/01/15");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("ISO 8601"));
    }

    #[test]
    fn invalid_datetime_missing_t() {
        let meta = LegalMetadata::new().with_notice_applied_at("2024-01-15 12:30:45Z");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("ISO 8601"));
    }

    #[test]
    fn empty_date_fails() {
        let meta = LegalMetadata::new().with_creation_date("");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn date_only_with_time_component_fails() {
        let meta = LegalMetadata::new().with_creation_date("2024-01-15T");
        let err = meta.validate().unwrap_err();
        assert!(err.to_string().contains("ISO 8601"));
    }

    #[test]
    fn all_date_fields_valid() {
        let meta = LegalMetadata::new()
            .with_creation_date("2024-01-15")
            .with_metadata_date("2024-01-15T12:30:45Z")
            .with_notice_applied_at("2024-01-15T12:30:45+05:30");
        assert!(meta.validate().is_ok());
    }
}
