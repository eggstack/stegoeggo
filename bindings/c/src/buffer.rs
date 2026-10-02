use crate::handles::stegoeggo_v1_buffer_t;

#[no_mangle]
pub extern "C" fn stegoeggo_v1_buffer_data(buffer: *const stegoeggo_v1_buffer_t) -> *const u8 {
    if buffer.is_null() {
        return core::ptr::null();
    }
    let bytes = unsafe { &(*buffer).bytes };
    if bytes.is_empty() {
        return core::ptr::null();
    }
    bytes.as_ptr()
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_buffer_len(buffer: *const stegoeggo_v1_buffer_t) -> usize {
    if buffer.is_null() {
        return 0;
    }
    let bytes = unsafe { &(*buffer).bytes };
    bytes.len()
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_buffer_free(buffer: *mut stegoeggo_v1_buffer_t) {
    if buffer.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(buffer));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_round_trip() {
        let handle = Box::into_raw(Box::new(stegoeggo_v1_buffer_t {
            bytes: vec![1, 2, 3],
        }));
        assert_eq!(stegoeggo_v1_buffer_len(handle), 3);
        let data = stegoeggo_v1_buffer_data(handle);
        assert!(!data.is_null());
        assert_eq!(unsafe { core::slice::from_raw_parts(data, 3) }, &[1, 2, 3]);
        stegoeggo_v1_buffer_free(handle);
    }

    #[test]
    fn null_buffer_sentinels() {
        assert!(stegoeggo_v1_buffer_data(core::ptr::null()).is_null());
        assert_eq!(stegoeggo_v1_buffer_len(core::ptr::null()), 0);
        stegoeggo_v1_buffer_free(core::ptr::null_mut());
    }
}
