use core::ffi::c_char;

use crate::codes::stegoeggo_v1_status_t;
use crate::handles::{stegoeggo_v1_error_t, stegoeggo_v1_notice_t};
use crate::input::{read_optional_text, require_flag, require_handle_mut, require_out};
use crate::panic::invoke;

fn set_text_field(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
    assign: impl FnOnce(&mut stegoeggo_v1_notice_t, Option<String>),
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(notice, "notice must be non-NULL")?;
        let value = read_optional_text(text, text_len, "text")?;
        assign(handle, value);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_create(
    out_notice: *mut *mut stegoeggo_v1_notice_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let slot = require_out(out_notice, "out_notice must be non-NULL")?;
        let handle = Box::new(stegoeggo_v1_notice_t::new());
        *slot = Box::into_raw(handle);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_free(notice: *mut stegoeggo_v1_notice_t) {
    if notice.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(notice));
    }
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_copyright_holder(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.copyright_holder = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_contact_email(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.contact_email = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_license_url(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.license_url = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_usage_terms(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.usage_terms = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_usage_terms_lang(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.usage_terms_lang = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_creation_date(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.creation_date = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_ai_constraints(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.ai_constraints = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_web_statement_of_rights(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.web_statement_of_rights = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_creator(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.creator = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_credit_line(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.credit_line = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_copyright_owner(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.copyright_owner = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_licensor_name(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.licensor_name = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_licensor_email(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.licensor_email = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_licensor_url(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.licensor_url = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_metadata_date(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.metadata_date = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_notice_applied_at(
    notice: *mut stegoeggo_v1_notice_t,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_text_field(notice, text, text_len, out_error, |handle, value| {
        handle.notice_applied_at = value;
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_dmi(
    notice: *mut stegoeggo_v1_notice_t,
    dmi: crate::codes::stegoeggo_v1_dmi_value_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(notice, "notice must be non-NULL")?;
        let value = crate::codes::dmi_from_code(dmi)
            .ok_or_else(|| crate::error::ErrorDto::invalid_argument("unknown DMI value"))?;
        handle.dmi = Some(value);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_notice_set_seed(
    notice: *mut stegoeggo_v1_notice_t,
    has_seed: u8,
    seed: u64,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(notice, "notice must be non-NULL")?;
        let present = require_flag(has_seed, "has_seed must be 0 or 1")?;
        handle.seed = if present { Some(seed) } else { None };
        Ok(())
    })
}
