use crate::error::ErrorDto;

pub(crate) fn require_handle<'a, T>(ptr: *const T, name: &'static str) -> Result<&'a T, ErrorDto> {
    if ptr.is_null() {
        return Err(ErrorDto::invalid_argument(name));
    }
    Ok(unsafe { &*ptr })
}

pub(crate) fn require_handle_mut<'a, T>(
    ptr: *mut T,
    name: &'static str,
) -> Result<&'a mut T, ErrorDto> {
    if ptr.is_null() {
        return Err(ErrorDto::invalid_argument(name));
    }
    Ok(unsafe { &mut *ptr })
}

pub(crate) fn require_out<'a, T>(
    ptr: *mut *mut T,
    name: &'static str,
) -> Result<&'a mut *mut T, ErrorDto> {
    if ptr.is_null() {
        return Err(ErrorDto::invalid_argument(name));
    }
    unsafe {
        *ptr = core::ptr::null_mut();
    }
    Ok(unsafe { &mut *ptr })
}

pub(crate) fn borrow_bytes<'a>(
    data: *const u8,
    len: usize,
    name: &'static str,
) -> Result<&'a [u8], ErrorDto> {
    if len == 0 {
        return Ok(&[]);
    }
    if data.is_null() {
        return Err(ErrorDto::invalid_argument(name));
    }
    if len > isize::MAX as usize {
        return Err(ErrorDto::invalid_argument(name));
    }
    Ok(unsafe { core::slice::from_raw_parts(data, len) })
}

pub(crate) fn read_text(
    text: *const core::ffi::c_char,
    len: usize,
    name: &'static str,
) -> Result<String, ErrorDto> {
    let bytes = borrow_bytes(text as *const u8, len, name)?;
    if bytes.contains(&0) {
        return Err(ErrorDto::invalid_argument(name));
    }
    if bytes.len() > stegoeggo::LegalMetadata::MAX_FIELD_LEN {
        return Err(ErrorDto::invalid_argument(name));
    }
    core::str::from_utf8(bytes)
        .map(ToString::to_string)
        .map_err(|_| ErrorDto::invalid_argument(name))
}

pub(crate) fn read_optional_text(
    text: *const core::ffi::c_char,
    len: usize,
    name: &'static str,
) -> Result<Option<String>, ErrorDto> {
    if text.is_null() {
        if len == 0 {
            return Ok(None);
        }
        return Err(ErrorDto::invalid_argument(name));
    }
    if len == 0 {
        return Err(ErrorDto::invalid_argument(name));
    }
    read_text(text, len, name).map(Some)
}

pub(crate) fn read_optional_bytes<'a>(
    data: *const u8,
    len: usize,
    name: &'static str,
) -> Result<Option<&'a [u8]>, ErrorDto> {
    if data.is_null() {
        if len == 0 {
            return Ok(None);
        }
        return Err(ErrorDto::invalid_argument(name));
    }
    if len == 0 {
        return Err(ErrorDto::invalid_argument(name));
    }
    borrow_bytes(data, len, name).map(Some)
}

pub(crate) fn require_flag(flag: u8, name: &'static str) -> Result<bool, ErrorDto> {
    match flag {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ErrorDto::invalid_argument(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_handle_rejected() {
        let result: Result<&u8, ErrorDto> =
            require_handle(core::ptr::null(), "handle must be non-NULL");
        let err = result.expect_err("NULL handle must fail");
        assert_eq!(err.code, crate::codes::STEGOEGGO_V1_ERR_INVALID_ARGUMENT);
    }

    #[test]
    fn null_with_nonzero_length_rejected() {
        assert!(borrow_bytes(core::ptr::null(), 4, "data").is_err());
        assert!(borrow_bytes(core::ptr::null(), 0, "data").is_ok());
    }

    #[test]
    fn text_rejects_nul_and_bad_utf8() {
        let with_nul = b"ab\0cd";
        assert!(read_text(with_nul.as_ptr() as *const core::ffi::c_char, 5, "t").is_err());
        let bad = [0xFF, 0xFE];
        assert!(read_text(bad.as_ptr() as *const core::ffi::c_char, 2, "t").is_err());
        let ok = b"hello";
        assert_eq!(
            read_text(ok.as_ptr() as *const core::ffi::c_char, 5, "t").expect("valid"),
            "hello"
        );
    }

    #[test]
    fn optional_text_null_rules() {
        assert_eq!(
            read_optional_text(core::ptr::null(), 0, "t").expect("none"),
            None
        );
        assert!(read_optional_text(core::ptr::null(), 3, "t").is_err());
        let bytes = b"x";
        assert!(read_optional_text(bytes.as_ptr() as *const core::ffi::c_char, 0, "t").is_err());
    }

    #[test]
    fn flags_strict() {
        assert!(!require_flag(0, "f").expect("flag"));
        assert!(require_flag(1, "f").expect("flag"));
        assert!(require_flag(2, "f").is_err());
    }
}
