use core::ffi::c_char;

use crate::codes::{
    stegoeggo_v1_authentication_mode_t, stegoeggo_v1_hidden_marker_mode_t,
    stegoeggo_v1_image_format_t, stegoeggo_v1_metadata_update_policy_t, stegoeggo_v1_preset_t,
    stegoeggo_v1_rights_policy_t, stegoeggo_v1_status_t,
};
use crate::handles::{
    stegoeggo_v1_error_t, stegoeggo_v1_notice_t, stegoeggo_v1_request_t,
    stegoeggo_v1_resource_limits_t,
};
use crate::input::{
    borrow_bytes, read_optional_bytes, read_optional_text, require_flag, require_handle,
    require_handle_mut, require_out,
};
use crate::panic::invoke;

fn build_request(
    notice: *const stegoeggo_v1_notice_t,
    policy_code: u32,
    preset_channels: Option<stegoeggo::ProtectionChannels>,
    out_request: *mut *mut stegoeggo_v1_request_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let slot = require_out(out_request, "out_request must be non-NULL")?;
        let notice_handle = require_handle(notice, "notice must be non-NULL")?;
        let policy = crate::codes::policy_from_code(policy_code)
            .ok_or_else(|| crate::error::ErrorDto::invalid_argument("unknown policy"))?;
        let channels = match preset_channels {
            Some(channels) => channels,
            None => stegoeggo::ProtectionChannels::metadata_only(),
        };
        let handle = stegoeggo_v1_request_t::new(notice_handle.to_notice(), policy, channels);
        *slot = Box::into_raw(Box::new(handle));
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_metadata_only(
    notice: *const stegoeggo_v1_notice_t,
    policy: stegoeggo_v1_rights_policy_t,
    out_request: *mut *mut stegoeggo_v1_request_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    build_request(notice, policy, None, out_request, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_with_hidden_marker(
    notice: *const stegoeggo_v1_notice_t,
    policy: stegoeggo_v1_rights_policy_t,
    out_request: *mut *mut stegoeggo_v1_request_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    build_request(
        notice,
        policy,
        Some(stegoeggo::ProtectionChannels::with_hidden_marker()),
        out_request,
        out_error,
    )
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_from_preset(
    preset: stegoeggo_v1_preset_t,
    notice: *const stegoeggo_v1_notice_t,
    policy: stegoeggo_v1_rights_policy_t,
    out_request: *mut *mut stegoeggo_v1_request_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    let channels = match crate::codes::preset_from_code(preset) {
        Some(value) => value.to_channels(),
        None => {
            return invoke(out_error, || {
                require_out(out_request, "out_request must be non-NULL")?;
                Err(crate::error::ErrorDto::invalid_argument("unknown preset"))
            });
        }
    };
    build_request(notice, policy, Some(channels), out_request, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_free(request: *mut stegoeggo_v1_request_t) {
    if request.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(request));
    }
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_seed(
    request: *mut stegoeggo_v1_request_t,
    has_seed: u8,
    seed: u64,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        let present = require_flag(has_seed, "has_seed must be 0 or 1")?;
        handle.seed = if present { Some(seed) } else { None };
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_intensity(
    request: *mut stegoeggo_v1_request_t,
    intensity: f32,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        if !intensity.is_finite() || !(0.0..=1.0).contains(&intensity) {
            return Err(crate::error::ErrorDto::invalid_argument(
                "intensity must be finite 0.0..=1.0",
            ));
        }
        handle.intensity = intensity;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_output_format(
    request: *mut stegoeggo_v1_request_t,
    has_format: u8,
    format: stegoeggo_v1_image_format_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        let present = require_flag(has_format, "has_format must be 0 or 1")?;
        if !present {
            handle.output_format = None;
            return Ok(());
        }
        let value = crate::codes::format_from_code(format)
            .ok_or_else(|| crate::error::ErrorDto::invalid_argument("unknown format"))?;
        handle.output_format = Some(value);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_jpeg_quality(
    request: *mut stegoeggo_v1_request_t,
    quality: u8,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        if quality == 0 {
            return Err(crate::error::ErrorDto::invalid_argument(
                "jpeg quality 0 is invalid",
            ));
        }
        handle.jpeg_quality = quality;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_progressive_jpeg(
    request: *mut stegoeggo_v1_request_t,
    enabled: u8,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        handle.progressive_jpeg = require_flag(enabled, "enabled must be 0 or 1")?;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_max_dimension(
    request: *mut stegoeggo_v1_request_t,
    has_max: u8,
    max_dimension: u32,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        let present = require_flag(has_max, "has_max must be 0 or 1")?;
        if !present {
            handle.max_dimension = None;
            return Ok(());
        }
        if max_dimension == 0 {
            return Err(crate::error::ErrorDto::invalid_argument(
                "max dimension 0 with presence is invalid",
            ));
        }
        handle.max_dimension = Some(max_dimension);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_metadata_update_policy(
    request: *mut stegoeggo_v1_request_t,
    policy: stegoeggo_v1_metadata_update_policy_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        let value = crate::codes::update_policy_from_code(policy)
            .ok_or_else(|| crate::error::ErrorDto::invalid_argument("unknown update policy"))?;
        handle.metadata_update_policy = value;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_stego_redundancy(
    request: *mut stegoeggo_v1_request_t,
    has_redundancy: u8,
    redundancy: u32,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        let present = require_flag(has_redundancy, "has_redundancy must be 0 or 1")?;
        if !present {
            handle.stego_redundancy = None;
            return Ok(());
        }
        if !(1..=10).contains(&redundancy) {
            return Err(crate::error::ErrorDto::invalid_argument(
                "redundancy must be 1..=10",
            ));
        }
        handle.stego_redundancy = Some(redundancy as usize);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_content_hash(
    request: *mut stegoeggo_v1_request_t,
    has_hash: u8,
    hash: *const u8,
    hash_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        let present = require_flag(has_hash, "has_hash must be 0 or 1")?;
        if !present {
            handle.content_hash = None;
            return Ok(());
        }
        let bytes = borrow_bytes(hash, hash_len, "content hash")?;
        if bytes.len() != 4 {
            return Err(crate::error::ErrorDto::invalid_argument(
                "content hash must be exactly 4 bytes",
            ));
        }
        let mut value = [0u8; 4];
        value.copy_from_slice(bytes);
        handle.content_hash = Some(value);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_timestamp_override(
    request: *mut stegoeggo_v1_request_t,
    has_ts: u8,
    text: *const c_char,
    text_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        let present = require_flag(has_ts, "has_ts must be 0 or 1")?;
        if !present {
            handle.timestamp_override = None;
            return Ok(());
        }
        let value = read_optional_text(text, text_len, "timestamp")?
            .ok_or_else(|| crate::error::ErrorDto::invalid_argument("timestamp requires text"))?;
        if value.len() > 256 {
            return Err(crate::error::ErrorDto::invalid_argument(
                "timestamp override exceeds 256 bytes",
            ));
        }
        handle.timestamp_override = Some(value);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_mac_key(
    request: *mut stegoeggo_v1_request_t,
    key: *const u8,
    key_len: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        let value = read_optional_bytes(key, key_len, "mac key")?;
        handle.mac_key = value.map(<[u8]>::to_vec);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_resource_limits(
    request: *mut stegoeggo_v1_request_t,
    limits: *const stegoeggo_v1_resource_limits_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        if limits.is_null() {
            handle.resource_limits = None;
            return Ok(());
        }
        let snapshot = unsafe { (*limits).inner.clone() };
        handle.resource_limits = Some(snapshot);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_hidden_marker_mode(
    request: *mut stegoeggo_v1_request_t,
    mode: stegoeggo_v1_hidden_marker_mode_t,
    tile_size: u32,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        let value = crate::codes::marker_from_code(mode, tile_size).ok_or_else(|| {
            crate::error::ErrorDto::invalid_argument("unknown marker mode or tile pairing")
        })?;
        handle.hidden_marker = value;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_request_set_authentication_mode(
    request: *mut stegoeggo_v1_request_t,
    mode: stegoeggo_v1_authentication_mode_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(request, "request must be non-NULL")?;
        let value = crate::codes::auth_from_code(mode)
            .ok_or_else(|| crate::error::ErrorDto::invalid_argument("unknown auth mode"))?;
        handle.authentication = value;
        Ok(())
    })
}
