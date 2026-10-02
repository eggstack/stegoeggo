use crate::error::{clear_error, emit_error, ErrorDto};
use crate::handles::stegoeggo_v1_error_t;

const PANIC_MESSAGE: &str = "Internal error: unexpected failure in C ABI call";

pub(crate) fn invoke(
    out_error: *mut *mut stegoeggo_v1_error_t,
    operation: impl FnOnce() -> Result<(), ErrorDto>,
) -> u32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(Ok(())) => {
            clear_error(out_error);
            crate::codes::STEGOEGGO_V1_OK
        }
        Ok(Err(dto)) => emit_error(out_error, dto),
        Err(_) => emit_error(out_error, ErrorDto::internal(PANIC_MESSAGE)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deliberate_panic_maps_to_internal_without_payload() {
        let mut error: *mut stegoeggo_v1_error_t = core::ptr::null_mut();
        let out_error: *mut *mut stegoeggo_v1_error_t = &mut error;
        let code = invoke(out_error, || {
            panic!("secret-payload-marker-xyz");
        });
        assert_eq!(code, crate::codes::STEGOEGGO_V1_ERR_INTERNAL);
        assert!(!error.is_null());
        let dto = unsafe { (*error).inner.clone() };
        assert_eq!(dto.resource, crate::codes::STEGOEGGO_V1_RESOURCE_NONE);
        assert!(!dto.message.contains("secret-payload-marker-xyz"));
        assert!(!dto.details_json.contains("secret-payload-marker-xyz"));
        unsafe {
            drop(Box::from_raw(error));
        }
    }

    #[test]
    fn success_clears_error_slot() {
        let mut error: *mut stegoeggo_v1_error_t =
            Box::into_raw(Box::new(crate::handles::stegoeggo_v1_error_t {
                inner: ErrorDto::internal("stale"),
            }));
        let code = invoke(&mut error, || Ok(()));
        assert_eq!(code, crate::codes::STEGOEGGO_V1_OK);
        assert!(error.is_null());
    }

    #[test]
    fn null_error_slot_still_returns_code() {
        let code = invoke(core::ptr::null_mut(), || {
            Err(ErrorDto::invalid_argument("probe"))
        });
        assert_eq!(code, crate::codes::STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    }
}
