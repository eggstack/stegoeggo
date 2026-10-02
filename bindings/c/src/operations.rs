use crate::codes::{
    stegoeggo_v1_image_format_t, stegoeggo_v1_status_t, STEGOEGGO_V1_FORMAT_UNKNOWN,
};
use crate::error::ErrorDto;
use crate::handles::{
    stegoeggo_v1_buffer_t, stegoeggo_v1_error_t, stegoeggo_v1_execution_report_t,
    stegoeggo_v1_request_t, stegoeggo_v1_resource_limits_t, stegoeggo_v1_verification_report_t,
};
use crate::input::{borrow_bytes, read_optional_bytes, require_handle, require_out};
use crate::panic::invoke;

#[no_mangle]
pub extern "C" fn stegoeggo_v1_detect_format(
    data: *const u8,
    data_len: usize,
    out_format: *mut stegoeggo_v1_image_format_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        if out_format.is_null() {
            return Err(ErrorDto::invalid_argument("out_format must be non-NULL"));
        }
        let bytes = borrow_bytes(data, data_len, "data")?;
        let code = match stegoeggo::ImageOutputFormat::from_magic_bytes(bytes) {
            Some(format) => crate::codes::format_to_code(format),
            None => STEGOEGGO_V1_FORMAT_UNKNOWN,
        };
        unsafe {
            *out_format = code;
        }
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_protect(
    data: *const u8,
    data_len: usize,
    request: *const stegoeggo_v1_request_t,
    out_data: *mut *mut stegoeggo_v1_buffer_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let slot = require_out(out_data, "out_data must be non-NULL")?;
        let bytes = borrow_bytes(data, data_len, "data")?;
        let handle = require_handle(request, "request must be non-NULL")?;
        let canonical = handle.to_request();
        match stegoeggo::process_request_bytes(bytes, &canonical) {
            Ok(protected) => {
                *slot = Box::into_raw(Box::new(stegoeggo_v1_buffer_t { bytes: protected }));
                Ok(())
            }
            Err(error) => Err(ErrorDto::from_rust(&error)),
        }
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_protect_with_report(
    data: *const u8,
    data_len: usize,
    request: *const stegoeggo_v1_request_t,
    out_data: *mut *mut stegoeggo_v1_buffer_t,
    out_report: *mut *mut stegoeggo_v1_execution_report_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let data_slot = require_out(out_data, "out_data must be non-NULL")?;
        let report_slot = require_out(out_report, "out_report must be non-NULL")?;
        let bytes = borrow_bytes(data, data_len, "data")?;
        let handle = require_handle(request, "request must be non-NULL")?;
        let canonical = handle.to_request();
        match stegoeggo::process_request_bytes_with_report(bytes, &canonical) {
            Ok((protected, report)) => {
                *data_slot = Box::into_raw(Box::new(stegoeggo_v1_buffer_t { bytes: protected }));
                *report_slot =
                    Box::into_raw(Box::new(stegoeggo_v1_execution_report_t { inner: report }));
                Ok(())
            }
            Err(error) => Err(ErrorDto::from_rust(&error)),
        }
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_verify(
    data: *const u8,
    data_len: usize,
    mac_key: *const u8,
    mac_key_len: usize,
    limits: *const stegoeggo_v1_resource_limits_t,
    out_report: *mut *mut stegoeggo_v1_verification_report_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let slot = require_out(out_report, "out_report must be non-NULL")?;
        let bytes = borrow_bytes(data, data_len, "data")?;
        let key = read_optional_bytes(mac_key, mac_key_len, "mac key")?.unwrap_or(&[]);
        let report = if limits.is_null() {
            stegoeggo::verify_image_bytes_report(bytes, key)
        } else {
            let handle = require_handle(limits, "limits must be non-NULL")?;
            stegoeggo::verify_image_bytes_report_with_limits(bytes, key, &handle.inner)
        };
        *slot = Box::into_raw(Box::new(stegoeggo_v1_verification_report_t {
            inner: report,
        }));
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::STEGOEGGO_V1_OK;

    fn tiny_png() -> Vec<u8> {
        let img = image::ImageBuffer::from_fn(16, 16, |x, y| image::Rgb([x as u8, y as u8, 128]));
        let mut bytes = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
        use image::ImageEncoder as _;
        encoder
            .write_image(&img, 16, 16, image::ExtendedColorType::Rgb8)
            .expect("encode fixture");
        bytes
    }

    #[test]
    fn detect_format_png_and_unknown() {
        let png = tiny_png();
        let mut format = crate::codes::STEGOEGGO_V1_FORMAT_UNKNOWN;
        let code =
            stegoeggo_v1_detect_format(png.as_ptr(), png.len(), &mut format, core::ptr::null_mut());
        assert_eq!(code, STEGOEGGO_V1_OK);
        assert_eq!(format, crate::codes::STEGOEGGO_V1_FORMAT_PNG);

        let junk = [0u8; 8];
        let mut unknown = crate::codes::STEGOEGGO_V1_FORMAT_JPEG;
        let code = stegoeggo_v1_detect_format(
            junk.as_ptr(),
            junk.len(),
            &mut unknown,
            core::ptr::null_mut(),
        );
        assert_eq!(code, STEGOEGGO_V1_OK);
        assert_eq!(unknown, STEGOEGGO_V1_FORMAT_UNKNOWN);
    }

    #[test]
    fn protect_then_verify_round_trip() {
        let png = tiny_png();
        let notice = stegoeggo::RightsNotice::new().with_copyright_holder("C ABI parity");
        let handle = stegoeggo_v1_request_t::new(
            notice,
            stegoeggo::RightsPolicy::ProhibitedAiMlTraining,
            stegoeggo::ProtectionChannels::metadata_only(),
        );
        let request_ptr = &handle as *const stegoeggo_v1_request_t;
        let mut out: *mut stegoeggo_v1_buffer_t = core::ptr::null_mut();
        let code = stegoeggo_v1_protect(
            png.as_ptr(),
            png.len(),
            request_ptr,
            &mut out,
            core::ptr::null_mut(),
        );
        assert_eq!(code, STEGOEGGO_V1_OK);
        assert!(!out.is_null());
        let protected = unsafe {
            core::slice::from_raw_parts(
                crate::buffer::stegoeggo_v1_buffer_data(out),
                crate::buffer::stegoeggo_v1_buffer_len(out),
            )
        }
        .to_vec();

        let mut report: *mut stegoeggo_v1_verification_report_t = core::ptr::null_mut();
        let code = stegoeggo_v1_verify(
            protected.as_ptr(),
            protected.len(),
            core::ptr::null(),
            0,
            core::ptr::null(),
            &mut report,
            core::ptr::null_mut(),
        );
        assert_eq!(code, STEGOEGGO_V1_OK);
        assert!(!report.is_null());
        let status = crate::report::stegoeggo_v1_verification_report_status(report);
        assert_eq!(status, crate::codes::STEGOEGGO_V1_VERIFY_NOT_FOUND);
        let rights = crate::report::stegoeggo_v1_verification_report_rights_found(report);
        assert_eq!(rights, 1);
        unsafe {
            drop(Box::from_raw(out));
            drop(Box::from_raw(report));
        }
    }

    #[test]
    fn rust_and_c_protect_agree_byte_for_byte() {
        let png = tiny_png();
        let notice = stegoeggo::RightsNotice::new().with_copyright_holder("C ABI parity");
        let mut staged = stegoeggo_v1_request_t::new(
            notice.clone(),
            stegoeggo::RightsPolicy::ProhibitedAiMlTraining,
            stegoeggo::ProtectionChannels::with_hidden_marker(),
        );
        staged.seed = Some(42);
        staged.timestamp_override = Some("2026-01-01T00:00:00Z".to_string());
        let canonical = staged.to_request();
        let expected = stegoeggo::process_request_bytes(&png, &canonical).expect("rust protect");

        let request_ptr = &staged as *const stegoeggo_v1_request_t;
        let mut out: *mut stegoeggo_v1_buffer_t = core::ptr::null_mut();
        let code = stegoeggo_v1_protect(
            png.as_ptr(),
            png.len(),
            request_ptr,
            &mut out,
            core::ptr::null_mut(),
        );
        assert_eq!(code, STEGOEGGO_V1_OK);
        let actual = unsafe {
            core::slice::from_raw_parts(
                crate::buffer::stegoeggo_v1_buffer_data(out),
                crate::buffer::stegoeggo_v1_buffer_len(out),
            )
        };
        assert_eq!(actual, expected.as_slice());
        unsafe {
            drop(Box::from_raw(out));
        }
    }
}
