use core::ffi::c_char;

pub type stegoeggo_v1_status_t = u32;
#[allow(dead_code)]
pub type stegoeggo_v1_error_code_t = u32;
#[allow(dead_code)]
pub type stegoeggo_v1_resource_code_t = u32;
pub type stegoeggo_v1_rights_policy_t = u32;
pub type stegoeggo_v1_dmi_value_t = u32;
pub type stegoeggo_v1_image_format_t = u32;
pub type stegoeggo_v1_metadata_update_policy_t = u32;
pub type stegoeggo_v1_preset_t = u32;
pub type stegoeggo_v1_authentication_mode_t = u32;
pub type stegoeggo_v1_hidden_marker_mode_t = u32;
pub type stegoeggo_v1_verification_status_t = u32;
pub type stegoeggo_v1_evidence_strength_t = u32;
pub type stegoeggo_v1_warning_t = u32;

pub const STEGOEGGO_V1_OK: u32 = 0;
pub const STEGOEGGO_V1_ERR_INVALID_ARGUMENT: u32 = 1;
pub const STEGOEGGO_V1_ERR_INVALID_CONFIGURATION: u32 = 2;
pub const STEGOEGGO_V1_ERR_INVALID_FORMAT: u32 = 3;
pub const STEGOEGGO_V1_ERR_ENCODE_DECODE: u32 = 4;
pub const STEGOEGGO_V1_ERR_METADATA: u32 = 5;
pub const STEGOEGGO_V1_ERR_STEGANOGRAPHY: u32 = 6;
pub const STEGOEGGO_V1_ERR_INSUFFICIENT_CAPACITY: u32 = 7;
pub const STEGOEGGO_V1_ERR_VERIFICATION: u32 = 8;
pub const STEGOEGGO_V1_ERR_RESOURCE_LIMIT: u32 = 9;
pub const STEGOEGGO_V1_ERR_INTERNAL: u32 = 10;

pub const STEGOEGGO_V1_RESOURCE_NONE: u32 = 0;
pub const STEGOEGGO_V1_RESOURCE_INPUT_BYTES: u32 = 1;
pub const STEGOEGGO_V1_RESOURCE_DIMENSIONS: u32 = 2;
pub const STEGOEGGO_V1_RESOURCE_CONTAINER: u32 = 3;
pub const STEGOEGGO_V1_RESOURCE_METADATA: u32 = 4;
pub const STEGOEGGO_V1_RESOURCE_VERIFICATION_BUDGET: u32 = 5;
pub const STEGOEGGO_V1_RESOURCE_CARRIER: u32 = 6;

pub const STEGOEGGO_V1_POLICY_UNSPECIFIED: u32 = 0;
pub const STEGOEGGO_V1_POLICY_ALLOWED: u32 = 1;
pub const STEGOEGGO_V1_POLICY_PROHIBITED_AI_ML_TRAINING: u32 = 2;
pub const STEGOEGGO_V1_POLICY_PROHIBITED_GENERATIVE_AI_TRAINING: u32 = 3;
pub const STEGOEGGO_V1_POLICY_PROHIBITED_EXCEPT_SEARCH_INDEXING: u32 = 4;
pub const STEGOEGGO_V1_POLICY_PROHIBITED_ALL_DATA_MINING: u32 = 5;
pub const STEGOEGGO_V1_POLICY_PROHIBITED_SEE_CONSTRAINTS: u32 = 6;

pub const STEGOEGGO_V1_DMI_UNSPECIFIED: u32 = 0;
pub const STEGOEGGO_V1_DMI_ALLOWED: u32 = 1;
pub const STEGOEGGO_V1_DMI_PROHIBITED_AI_ML_TRAINING: u32 = 2;
pub const STEGOEGGO_V1_DMI_PROHIBITED_GEN_AI_ML_TRAINING: u32 = 3;
pub const STEGOEGGO_V1_DMI_PROHIBITED_EXCEPT_SEARCH_ENGINE_INDEXING: u32 = 4;
pub const STEGOEGGO_V1_DMI_PROHIBITED: u32 = 5;
pub const STEGOEGGO_V1_DMI_PROHIBITED_SEE_CONSTRAINTS: u32 = 6;

pub const STEGOEGGO_V1_FORMAT_UNKNOWN: u32 = 0;
pub const STEGOEGGO_V1_FORMAT_PNG: u32 = 1;
pub const STEGOEGGO_V1_FORMAT_JPEG: u32 = 2;
pub const STEGOEGGO_V1_FORMAT_WEBP: u32 = 3;

pub const STEGOEGGO_V1_UPDATE_REPLACE_STEGO_OWNED: u32 = 0;
pub const STEGOEGGO_V1_UPDATE_FAIL_ON_CONFLICT: u32 = 1;
pub const STEGOEGGO_V1_UPDATE_PRESERVE_EXISTING: u32 = 2;

pub const STEGOEGGO_V1_PRESET_LEGAL_NOTICE: u32 = 0;
pub const STEGOEGGO_V1_PRESET_LEGAL_NOTICE_WITH_STEGO: u32 = 1;
pub const STEGOEGGO_V1_PRESET_AUTHENTICATED_PROVENANCE: u32 = 2;
pub const STEGOEGGO_V1_PRESET_MAXIMAL: u32 = 3;

pub const STEGOEGGO_V1_AUTH_NONE: u32 = 0;
pub const STEGOEGGO_V1_AUTH_HMAC: u32 = 1;

pub const STEGOEGGO_V1_MARKER_DISABLED: u32 = 0;
pub const STEGOEGGO_V1_MARKER_SEED_ONLY: u32 = 1;
pub const STEGOEGGO_V1_MARKER_BEST_EFFORT: u32 = 2;
pub const STEGOEGGO_V1_MARKER_TILED: u32 = 3;

pub const STEGOEGGO_V1_VERIFY_VERIFIED: u32 = 0;
pub const STEGOEGGO_V1_VERIFY_INVALID: u32 = 1;
pub const STEGOEGGO_V1_VERIFY_NOT_FOUND: u32 = 2;

pub const STEGOEGGO_V1_EVIDENCE_NO_NOTICE_FOUND: u32 = 0;
pub const STEGOEGGO_V1_EVIDENCE_METADATA_NOTICE_ONLY: u32 = 1;
pub const STEGOEGGO_V1_EVIDENCE_METADATA_NOTICE_AND_BEST_EFFORT_STEGO: u32 = 2;
pub const STEGOEGGO_V1_EVIDENCE_METADATA_NOTICE_AND_AUTHENTICATED_PROVENANCE: u32 = 3;

pub const STEGOEGGO_V1_WARNING_MISSING_MAC_KEY: u32 = 0;
pub const STEGOEGGO_V1_WARNING_METADATA_INJECTION_DISABLED: u32 = 1;
pub const STEGOEGGO_V1_WARNING_PROGRESSIVE_JPEG_FALLBACK: u32 = 2;
pub const STEGOEGGO_V1_WARNING_JPEG_REENCODE_FRAGILE: u32 = 3;
pub const STEGOEGGO_V1_WARNING_LSB_CAPACITY_SKIPPED: u32 = 4;
pub const STEGOEGGO_V1_WARNING_DCT_CAPACITY_INSUFFICIENT: u32 = 5;
pub const STEGOEGGO_V1_WARNING_CONTRADICTORY_LEGAL_CLAIMS: u32 = 6;
pub const STEGOEGGO_V1_WARNING_MISSING_RIGHTS_CONSTRAINTS: u32 = 7;

pub const STEGOEGGO_V1_ABI_MAJOR: u32 = 1;
pub const STEGOEGGO_V1_ABI_MINOR: u32 = 0;

const SOURCE_VERSION_BYTES: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();

#[no_mangle]
pub extern "C" fn stegoeggo_abi_version_major() -> u32 {
    STEGOEGGO_V1_ABI_MAJOR
}

#[no_mangle]
pub extern "C" fn stegoeggo_abi_version_minor() -> u32 {
    STEGOEGGO_V1_ABI_MINOR
}

#[no_mangle]
pub extern "C" fn stegoeggo_source_version() -> *const c_char {
    SOURCE_VERSION_BYTES.as_ptr() as *const c_char
}

pub(crate) fn policy_from_code(code: u32) -> Option<stegoeggo::RightsPolicy> {
    use stegoeggo::RightsPolicy as P;
    match code {
        STEGOEGGO_V1_POLICY_UNSPECIFIED => Some(P::Unspecified),
        STEGOEGGO_V1_POLICY_ALLOWED => Some(P::Allowed),
        STEGOEGGO_V1_POLICY_PROHIBITED_AI_ML_TRAINING => Some(P::ProhibitedAiMlTraining),
        STEGOEGGO_V1_POLICY_PROHIBITED_GENERATIVE_AI_TRAINING => {
            Some(P::ProhibitedGenerativeAiTraining)
        }
        STEGOEGGO_V1_POLICY_PROHIBITED_EXCEPT_SEARCH_INDEXING => {
            Some(P::ProhibitedExceptSearchIndexing)
        }
        STEGOEGGO_V1_POLICY_PROHIBITED_ALL_DATA_MINING => Some(P::ProhibitedAllDataMining),
        STEGOEGGO_V1_POLICY_PROHIBITED_SEE_CONSTRAINTS => Some(P::ProhibitedSeeConstraints),
        _ => None,
    }
}

pub(crate) fn policy_to_code(policy: stegoeggo::RightsPolicy) -> u32 {
    use stegoeggo::RightsPolicy as P;
    match policy {
        P::Unspecified => STEGOEGGO_V1_POLICY_UNSPECIFIED,
        P::Allowed => STEGOEGGO_V1_POLICY_ALLOWED,
        P::ProhibitedAiMlTraining => STEGOEGGO_V1_POLICY_PROHIBITED_AI_ML_TRAINING,
        P::ProhibitedGenerativeAiTraining => STEGOEGGO_V1_POLICY_PROHIBITED_GENERATIVE_AI_TRAINING,
        P::ProhibitedExceptSearchIndexing => STEGOEGGO_V1_POLICY_PROHIBITED_EXCEPT_SEARCH_INDEXING,
        P::ProhibitedAllDataMining => STEGOEGGO_V1_POLICY_PROHIBITED_ALL_DATA_MINING,
        P::ProhibitedSeeConstraints => STEGOEGGO_V1_POLICY_PROHIBITED_SEE_CONSTRAINTS,
        _ => STEGOEGGO_V1_POLICY_UNSPECIFIED,
    }
}

pub(crate) fn dmi_from_code(code: u32) -> Option<stegoeggo::DmiValue> {
    use stegoeggo::DmiValue as D;
    match code {
        STEGOEGGO_V1_DMI_UNSPECIFIED => Some(D::Unspecified),
        STEGOEGGO_V1_DMI_ALLOWED => Some(D::Allowed),
        STEGOEGGO_V1_DMI_PROHIBITED_AI_ML_TRAINING => Some(D::ProhibitedAiMlTraining),
        STEGOEGGO_V1_DMI_PROHIBITED_GEN_AI_ML_TRAINING => Some(D::ProhibitedGenAiMlTraining),
        STEGOEGGO_V1_DMI_PROHIBITED_EXCEPT_SEARCH_ENGINE_INDEXING => {
            Some(D::ProhibitedExceptSearchEngineIndexing)
        }
        STEGOEGGO_V1_DMI_PROHIBITED => Some(D::Prohibited),
        STEGOEGGO_V1_DMI_PROHIBITED_SEE_CONSTRAINTS => Some(D::ProhibitedSeeConstraints),
        _ => None,
    }
}

pub(crate) fn dmi_to_code(dmi: stegoeggo::DmiValue) -> u32 {
    use stegoeggo::DmiValue as D;
    match dmi {
        D::Unspecified => STEGOEGGO_V1_DMI_UNSPECIFIED,
        D::Allowed => STEGOEGGO_V1_DMI_ALLOWED,
        D::ProhibitedAiMlTraining => STEGOEGGO_V1_DMI_PROHIBITED_AI_ML_TRAINING,
        D::ProhibitedGenAiMlTraining => STEGOEGGO_V1_DMI_PROHIBITED_GEN_AI_ML_TRAINING,
        D::ProhibitedExceptSearchEngineIndexing => {
            STEGOEGGO_V1_DMI_PROHIBITED_EXCEPT_SEARCH_ENGINE_INDEXING
        }
        D::Prohibited => STEGOEGGO_V1_DMI_PROHIBITED,
        D::ProhibitedSeeConstraints => STEGOEGGO_V1_DMI_PROHIBITED_SEE_CONSTRAINTS,
        _ => STEGOEGGO_V1_DMI_UNSPECIFIED,
    }
}

pub(crate) fn format_from_code(code: u32) -> Option<stegoeggo::ImageOutputFormat> {
    use stegoeggo::ImageOutputFormat as F;
    match code {
        STEGOEGGO_V1_FORMAT_PNG => Some(F::Png),
        STEGOEGGO_V1_FORMAT_JPEG => Some(F::Jpeg),
        STEGOEGGO_V1_FORMAT_WEBP => Some(F::WebP),
        _ => None,
    }
}

pub(crate) fn format_to_code(format: stegoeggo::ImageOutputFormat) -> u32 {
    use stegoeggo::ImageOutputFormat as F;
    match format {
        F::Png => STEGOEGGO_V1_FORMAT_PNG,
        F::Jpeg => STEGOEGGO_V1_FORMAT_JPEG,
        F::WebP => STEGOEGGO_V1_FORMAT_WEBP,
        _ => STEGOEGGO_V1_FORMAT_UNKNOWN,
    }
}

pub(crate) fn update_policy_from_code(code: u32) -> Option<stegoeggo::MetadataUpdatePolicy> {
    use stegoeggo::MetadataUpdatePolicy as M;
    match code {
        STEGOEGGO_V1_UPDATE_REPLACE_STEGO_OWNED => Some(M::ReplaceStegoOwned),
        STEGOEGGO_V1_UPDATE_FAIL_ON_CONFLICT => Some(M::FailOnConflict),
        STEGOEGGO_V1_UPDATE_PRESERVE_EXISTING => Some(M::PreserveExisting),
        _ => None,
    }
}

pub(crate) fn preset_from_code(code: u32) -> Option<stegoeggo::ProtectionPreset> {
    use stegoeggo::ProtectionPreset as P;
    match code {
        STEGOEGGO_V1_PRESET_LEGAL_NOTICE => Some(P::LegalNotice),
        STEGOEGGO_V1_PRESET_LEGAL_NOTICE_WITH_STEGO => Some(P::LegalNoticeWithStego),
        STEGOEGGO_V1_PRESET_AUTHENTICATED_PROVENANCE => Some(P::AuthenticatedProvenance),
        STEGOEGGO_V1_PRESET_MAXIMAL => Some(P::Maximal),
        _ => None,
    }
}

pub(crate) fn auth_from_code(code: u32) -> Option<stegoeggo::AuthenticationMode> {
    use stegoeggo::AuthenticationMode as A;
    match code {
        STEGOEGGO_V1_AUTH_NONE => Some(A::None),
        STEGOEGGO_V1_AUTH_HMAC => Some(A::Hmac),
        _ => None,
    }
}

pub(crate) fn marker_from_code(mode: u32, tile_size: u32) -> Option<stegoeggo::HiddenMarkerMode> {
    use stegoeggo::HiddenMarkerMode as M;
    match mode {
        STEGOEGGO_V1_MARKER_DISABLED => {
            if tile_size == 0 {
                Some(M::Disabled)
            } else {
                None
            }
        }
        STEGOEGGO_V1_MARKER_SEED_ONLY => {
            if tile_size == 0 {
                Some(M::SeedOnly)
            } else {
                None
            }
        }
        STEGOEGGO_V1_MARKER_BEST_EFFORT => {
            if tile_size == 0 {
                Some(M::BestEffort)
            } else {
                None
            }
        }
        STEGOEGGO_V1_MARKER_TILED => {
            if (32..=1024).contains(&tile_size) {
                Some(M::Tiled { tile_size })
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn status_to_code(status: stegoeggo::VerificationStatus) -> u32 {
    use stegoeggo::VerificationStatus as S;
    match status {
        S::Verified => STEGOEGGO_V1_VERIFY_VERIFIED,
        S::Invalid => STEGOEGGO_V1_VERIFY_INVALID,
        S::NotFound => STEGOEGGO_V1_VERIFY_NOT_FOUND,
    }
}

pub(crate) fn strength_to_code(strength: stegoeggo::EvidenceStrength) -> u32 {
    use stegoeggo::EvidenceStrength as E;
    match strength {
        E::NoNoticeFound => STEGOEGGO_V1_EVIDENCE_NO_NOTICE_FOUND,
        E::MetadataNoticeOnly => STEGOEGGO_V1_EVIDENCE_METADATA_NOTICE_ONLY,
        E::MetadataNoticeAndBestEffortStego => {
            STEGOEGGO_V1_EVIDENCE_METADATA_NOTICE_AND_BEST_EFFORT_STEGO
        }
        E::MetadataNoticeAndAuthenticatedProvenance => {
            STEGOEGGO_V1_EVIDENCE_METADATA_NOTICE_AND_AUTHENTICATED_PROVENANCE
        }
        _ => STEGOEGGO_V1_EVIDENCE_NO_NOTICE_FOUND,
    }
}

pub(crate) fn warning_to_code(warning: &stegoeggo::ProtectionWarning) -> u32 {
    use stegoeggo::ProtectionWarning as W;
    match warning {
        W::MissingMacKey => STEGOEGGO_V1_WARNING_MISSING_MAC_KEY,
        W::MetadataInjectionDisabled => STEGOEGGO_V1_WARNING_METADATA_INJECTION_DISABLED,
        W::ProgressiveJpegFallback => STEGOEGGO_V1_WARNING_PROGRESSIVE_JPEG_FALLBACK,
        W::JpegReencodeFragile => STEGOEGGO_V1_WARNING_JPEG_REENCODE_FRAGILE,
        W::LsbCapacitySkipped => STEGOEGGO_V1_WARNING_LSB_CAPACITY_SKIPPED,
        W::DctCapacityInsufficient => STEGOEGGO_V1_WARNING_DCT_CAPACITY_INSUFFICIENT,
        W::ContradictoryLegalClaims => STEGOEGGO_V1_WARNING_CONTRADICTORY_LEGAL_CLAIMS,
        W::MissingRightsConstraints => STEGOEGGO_V1_WARNING_MISSING_RIGHTS_CONSTRAINTS,
        _ => u32::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_versions_match_contract() {
        assert_eq!(stegoeggo_abi_version_major(), 1);
        assert_eq!(stegoeggo_abi_version_minor(), 0);
        let ptr = stegoeggo_source_version();
        assert!(!ptr.is_null());
        let version = unsafe { core::ffi::CStr::from_ptr(ptr) }
            .to_str()
            .expect("source version is ASCII");
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.5.0");
    }

    #[test]
    fn unknown_codes_rejected() {
        assert!(policy_from_code(7).is_none());
        assert!(policy_from_code(u32::MAX).is_none());
        assert!(dmi_from_code(7).is_none());
        assert!(format_from_code(STEGOEGGO_V1_FORMAT_UNKNOWN).is_none());
        assert!(format_from_code(4).is_none());
        assert!(update_policy_from_code(3).is_none());
        assert!(preset_from_code(4).is_none());
        assert!(auth_from_code(2).is_none());
        assert!(marker_from_code(4, 0).is_none());
        assert!(marker_from_code(STEGOEGGO_V1_MARKER_TILED, 0).is_none());
        assert!(marker_from_code(STEGOEGGO_V1_MARKER_TILED, 31).is_none());
        assert!(marker_from_code(STEGOEGGO_V1_MARKER_TILED, 1025).is_none());
        assert!(marker_from_code(STEGOEGGO_V1_MARKER_DISABLED, 64).is_none());
        assert!(marker_from_code(STEGOEGGO_V1_MARKER_BEST_EFFORT, 1).is_none());
    }

    #[test]
    fn known_codes_round_trip() {
        for code in 0..=6 {
            let policy = policy_from_code(code).expect("known policy code");
            assert_eq!(policy_to_code(policy), code);
        }
        for code in 0..=6 {
            let dmi = dmi_from_code(code).expect("known dmi code");
            assert_eq!(dmi_to_code(dmi), code);
        }
        assert!(marker_from_code(STEGOEGGO_V1_MARKER_TILED, 32).is_some());
        assert!(marker_from_code(STEGOEGGO_V1_MARKER_TILED, 1024).is_some());
    }

    #[test]
    fn output_projections_cover_variants() {
        assert_eq!(
            status_to_code(stegoeggo::VerificationStatus::Verified),
            STEGOEGGO_V1_VERIFY_VERIFIED
        );
        assert_eq!(
            status_to_code(stegoeggo::VerificationStatus::Invalid),
            STEGOEGGO_V1_VERIFY_INVALID
        );
        assert_eq!(
            status_to_code(stegoeggo::VerificationStatus::NotFound),
            STEGOEGGO_V1_VERIFY_NOT_FOUND
        );
        assert_eq!(
            strength_to_code(stegoeggo::EvidenceStrength::NoNoticeFound),
            STEGOEGGO_V1_EVIDENCE_NO_NOTICE_FOUND
        );
        assert_eq!(
            warning_to_code(&stegoeggo::ProtectionWarning::MissingMacKey),
            STEGOEGGO_V1_WARNING_MISSING_MAC_KEY
        );
        assert_eq!(
            warning_to_code(&stegoeggo::ProtectionWarning::MissingRightsConstraints),
            STEGOEGGO_V1_WARNING_MISSING_RIGHTS_CONSTRAINTS
        );
    }
}
