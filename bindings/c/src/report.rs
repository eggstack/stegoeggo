use crate::codes::{
    stegoeggo_v1_dmi_value_t, stegoeggo_v1_rights_policy_t, stegoeggo_v1_verification_status_t,
    stegoeggo_v1_warning_t, STEGOEGGO_V1_DMI_UNSPECIFIED,
};
use crate::handles::{
    stegoeggo_v1_buffer_t, stegoeggo_v1_execution_report_t, stegoeggo_v1_verification_report_t,
};

#[no_mangle]
pub extern "C" fn stegoeggo_v1_execution_report_effective_policy(
    report: *const stegoeggo_v1_execution_report_t,
) -> stegoeggo_v1_rights_policy_t {
    if report.is_null() {
        return crate::codes::STEGOEGGO_V1_POLICY_UNSPECIFIED;
    }
    let inner = unsafe { &(*report).inner };
    let policy = inner.effective_policy();
    crate::codes::policy_to_code(policy)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_execution_report_has_dmi(
    report: *const stegoeggo_v1_execution_report_t,
) -> u8 {
    if report.is_null() {
        return 0;
    }
    u8::from(unsafe { &(*report).inner }.effective_dmi().is_some())
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_execution_report_effective_dmi(
    report: *const stegoeggo_v1_execution_report_t,
) -> stegoeggo_v1_dmi_value_t {
    if report.is_null() {
        return STEGOEGGO_V1_DMI_UNSPECIFIED;
    }
    match unsafe { &(*report).inner }.effective_dmi() {
        Some(dmi) => crate::codes::dmi_to_code(dmi),
        None => STEGOEGGO_V1_DMI_UNSPECIFIED,
    }
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_execution_report_metadata_injected(
    report: *const stegoeggo_v1_execution_report_t,
) -> u8 {
    if report.is_null() {
        return 0;
    }
    u8::from(unsafe { &(*report).inner }.metadata_injected())
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_execution_report_stego_attempted(
    report: *const stegoeggo_v1_execution_report_t,
) -> u8 {
    if report.is_null() {
        return 0;
    }
    u8::from(unsafe { &(*report).inner }.stego_attempted())
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_execution_report_stego_succeeded(
    report: *const stegoeggo_v1_execution_report_t,
) -> u8 {
    if report.is_null() {
        return 0;
    }
    u8::from(unsafe { &(*report).inner }.stego_succeeded())
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_execution_report_format_transcoded(
    report: *const stegoeggo_v1_execution_report_t,
) -> u8 {
    if report.is_null() {
        return 0;
    }
    u8::from(unsafe { &(*report).inner }.format_transcoded())
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_execution_report_warning_count(
    report: *const stegoeggo_v1_execution_report_t,
) -> usize {
    if report.is_null() {
        return 0;
    }
    unsafe { &(*report).inner }.warnings().len()
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_execution_report_warning_at(
    report: *const stegoeggo_v1_execution_report_t,
    index: usize,
) -> stegoeggo_v1_warning_t {
    if report.is_null() {
        return u32::MAX;
    }
    match unsafe { &(*report).inner }.warnings().get(index) {
        Some(warning) => crate::codes::warning_to_code(warning),
        None => u32::MAX,
    }
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_execution_report_free(report: *mut stegoeggo_v1_execution_report_t) {
    if report.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(report));
    }
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_verification_report_status(
    report: *const stegoeggo_v1_verification_report_t,
) -> stegoeggo_v1_verification_status_t {
    if report.is_null() {
        return crate::codes::STEGOEGGO_V1_VERIFY_NOT_FOUND;
    }
    let inner = unsafe { &(*report).inner };
    let status = inner.hidden_marker().status();
    crate::codes::status_to_code(status)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_verification_report_evidence_strength(
    report: *const stegoeggo_v1_verification_report_t,
) -> crate::codes::stegoeggo_v1_evidence_strength_t {
    if report.is_null() {
        return crate::codes::STEGOEGGO_V1_EVIDENCE_NO_NOTICE_FOUND;
    }
    let inner = unsafe { &(*report).inner };
    let strength = inner.evidence_strength();
    crate::codes::strength_to_code(strength)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_verification_report_rights_found(
    report: *const stegoeggo_v1_verification_report_t,
) -> u8 {
    if report.is_null() {
        return 0;
    }
    u8::from(unsafe { &(*report).inner }.rights().found())
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_verification_report_authenticated(
    report: *const stegoeggo_v1_verification_report_t,
) -> u8 {
    if report.is_null() {
        return 0;
    }
    let inner = unsafe { &(*report).inner };
    let authenticated = inner.authentication().attempted()
        && inner.authentication().hmac_status() == Some(stegoeggo::VerificationStatus::Verified)
        && inner.authentication().key_matched();
    u8::from(authenticated)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_verification_report_json(
    report: *const stegoeggo_v1_verification_report_t,
) -> *mut stegoeggo_v1_buffer_t {
    if report.is_null() {
        return core::ptr::null_mut();
    }
    let inner = unsafe { &(*report).inner };
    match serde_json::to_vec(inner) {
        Ok(bytes) => Box::into_raw(Box::new(stegoeggo_v1_buffer_t { bytes })),
        Err(_) => core::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_verification_report_free(
    report: *mut stegoeggo_v1_verification_report_t,
) {
    if report.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(report));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_reports_use_sentinels() {
        assert_eq!(
            stegoeggo_v1_execution_report_effective_policy(core::ptr::null()),
            crate::codes::STEGOEGGO_V1_POLICY_UNSPECIFIED
        );
        assert_eq!(stegoeggo_v1_execution_report_has_dmi(core::ptr::null()), 0);
        assert_eq!(
            stegoeggo_v1_execution_report_effective_dmi(core::ptr::null()),
            STEGOEGGO_V1_DMI_UNSPECIFIED
        );
        assert_eq!(
            stegoeggo_v1_execution_report_metadata_injected(core::ptr::null()),
            0
        );
        assert_eq!(
            stegoeggo_v1_execution_report_warning_count(core::ptr::null()),
            0
        );
        assert_eq!(
            stegoeggo_v1_execution_report_warning_at(core::ptr::null(), 0),
            u32::MAX
        );
        stegoeggo_v1_execution_report_free(core::ptr::null_mut());
        assert_eq!(
            stegoeggo_v1_verification_report_status(core::ptr::null()),
            crate::codes::STEGOEGGO_V1_VERIFY_NOT_FOUND
        );
        assert_eq!(
            stegoeggo_v1_verification_report_evidence_strength(core::ptr::null()),
            crate::codes::STEGOEGGO_V1_EVIDENCE_NO_NOTICE_FOUND
        );
        assert_eq!(
            stegoeggo_v1_verification_report_rights_found(core::ptr::null()),
            0
        );
        assert_eq!(
            stegoeggo_v1_verification_report_authenticated(core::ptr::null()),
            0
        );
        assert!(stegoeggo_v1_verification_report_json(core::ptr::null()).is_null());
        stegoeggo_v1_verification_report_free(core::ptr::null_mut());
    }

    #[test]
    fn execution_getters_project_report() {
        let report = stegoeggo::ExecutionReport {
            effective_policy: stegoeggo::RightsPolicy::Allowed,
            effective_dmi: Some(stegoeggo::DmiValue::Allowed),
            metadata_injected: true,
            stego_attempted: true,
            stego_succeeded: true,
            format_transcoded: false,
            warnings: vec![stegoeggo::ProtectionWarning::MissingMacKey],
            resource_usage: None,
            embed_summary: None,
        };
        let handle = Box::into_raw(Box::new(stegoeggo_v1_execution_report_t { inner: report }));
        assert_eq!(
            stegoeggo_v1_execution_report_effective_policy(handle),
            crate::codes::STEGOEGGO_V1_POLICY_ALLOWED
        );
        assert_eq!(stegoeggo_v1_execution_report_has_dmi(handle), 1);
        assert_eq!(
            stegoeggo_v1_execution_report_effective_dmi(handle),
            crate::codes::STEGOEGGO_V1_DMI_ALLOWED
        );
        assert_eq!(stegoeggo_v1_execution_report_metadata_injected(handle), 1);
        assert_eq!(stegoeggo_v1_execution_report_warning_count(handle), 1);
        assert_eq!(
            stegoeggo_v1_execution_report_warning_at(handle, 0),
            crate::codes::STEGOEGGO_V1_WARNING_MISSING_MAC_KEY
        );
        assert_eq!(
            stegoeggo_v1_execution_report_warning_at(handle, 1),
            u32::MAX
        );
        stegoeggo_v1_execution_report_free(handle);
    }
}
