use crate::codes;
use crate::handles::stegoeggo_v1_error_t;

#[derive(Debug, Clone)]
pub(crate) struct ErrorDto {
    pub(crate) code: u32,
    pub(crate) resource: u32,
    pub(crate) message: String,
    pub(crate) details_json: String,
}

impl ErrorDto {
    pub(crate) fn invalid_argument(detail: &'static str) -> Self {
        Self::bare(
            codes::STEGOEGGO_V1_ERR_INVALID_ARGUMENT,
            codes::STEGOEGGO_V1_RESOURCE_NONE,
            format!("Invalid argument: {detail}"),
        )
    }

    pub(crate) fn internal(message: &str) -> Self {
        Self::bare(
            codes::STEGOEGGO_V1_ERR_INTERNAL,
            codes::STEGOEGGO_V1_RESOURCE_NONE,
            message.to_string(),
        )
    }

    fn bare(code: u32, resource: u32, message: String) -> Self {
        let details_json = render_details(code, resource, &message, serde_json::Map::new());
        Self {
            code,
            resource,
            message,
            details_json,
        }
    }

    pub(crate) fn from_rust(error: &stegoeggo::Error) -> Self {
        use stegoeggo::Error as E;
        let message = error.to_string();
        let mut fields = serde_json::Map::new();
        let (code, resource) = match error {
            E::Config(_) => (
                codes::STEGOEGGO_V1_ERR_INVALID_CONFIGURATION,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            E::InvalidFormat(_) => (
                codes::STEGOEGGO_V1_ERR_INVALID_FORMAT,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            E::ImageDecode(_)
            | E::ImageEncode(_)
            | E::Image(_)
            | E::ImageTruncated(_)
            | E::Io(_) => (
                codes::STEGOEGGO_V1_ERR_ENCODE_DECODE,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            E::Metadata(_) => (
                codes::STEGOEGGO_V1_ERR_METADATA,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            E::Steganography(_) => (
                codes::STEGOEGGO_V1_ERR_STEGANOGRAPHY,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            E::InsufficientCapacity {
                required,
                available,
            } => {
                fields.insert(
                    "required".to_string(),
                    serde_json::Value::from(*required as u64),
                );
                fields.insert(
                    "available".to_string(),
                    serde_json::Value::from(*available as u64),
                );
                (
                    codes::STEGOEGGO_V1_ERR_INSUFFICIENT_CAPACITY,
                    codes::STEGOEGGO_V1_RESOURCE_NONE,
                )
            }
            E::PayloadVerification(_) | E::Crypto(_) => (
                codes::STEGOEGGO_V1_ERR_VERIFICATION,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            E::InputTooLarge { size, limit } => {
                fields.insert("size".to_string(), serde_json::Value::from(*size as u64));
                fields.insert("limit".to_string(), serde_json::Value::from(*limit as u64));
                (
                    codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                    codes::STEGOEGGO_V1_RESOURCE_INPUT_BYTES,
                )
            }
            E::DimensionsExceeded {
                width,
                height,
                max_width,
                max_height,
            } => {
                fields.insert("width".to_string(), serde_json::Value::from(*width));
                fields.insert("height".to_string(), serde_json::Value::from(*height));
                fields.insert("max_width".to_string(), serde_json::Value::from(*max_width));
                fields.insert(
                    "max_height".to_string(),
                    serde_json::Value::from(*max_height),
                );
                (
                    codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                    codes::STEGOEGGO_V1_RESOURCE_DIMENSIONS,
                )
            }
            E::ContainerLimitExceeded { kind, count, .. } => {
                fields.insert("kind".to_string(), serde_json::Value::from(*kind));
                fields.insert("count".to_string(), serde_json::Value::from(*count as u64));
                (
                    codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                    codes::STEGOEGGO_V1_RESOURCE_CONTAINER,
                )
            }
            E::MetadataLimitExceeded { kind, size, limit } => {
                fields.insert("kind".to_string(), serde_json::Value::from(*kind));
                fields.insert("size".to_string(), serde_json::Value::from(*size as u64));
                fields.insert("limit".to_string(), serde_json::Value::from(*limit as u64));
                (
                    codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                    codes::STEGOEGGO_V1_RESOURCE_METADATA,
                )
            }
            E::VerificationBudgetExceeded { kind, count, .. } => {
                fields.insert("kind".to_string(), serde_json::Value::from(*kind));
                fields.insert("count".to_string(), serde_json::Value::from(*count as u64));
                (
                    codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                    codes::STEGOEGGO_V1_RESOURCE_VERIFICATION_BUDGET,
                )
            }
            E::ResourceLimitExceeded(_) => (
                codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                codes::STEGOEGGO_V1_RESOURCE_CARRIER,
            ),
            _ => (
                codes::STEGOEGGO_V1_ERR_INTERNAL,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
        };
        let details_json = render_details(code, resource, &message, fields);
        Self {
            code,
            resource,
            message,
            details_json,
        }
    }
}

fn render_details(
    code: u32,
    resource: u32,
    message: &str,
    mut fields: serde_json::Map<String, serde_json::Value>,
) -> String {
    let mut object = serde_json::Map::with_capacity(fields.len() + 4);
    object.insert("schema_version".to_string(), serde_json::Value::from(1));
    object.insert("code".to_string(), serde_json::Value::from(code));
    object.insert("resource".to_string(), serde_json::Value::from(resource));
    object.insert("message".to_string(), serde_json::Value::from(message));
    object.append(&mut fields);
    serde_json::Value::Object(object).to_string()
}

pub(crate) fn emit_error(out_error: *mut *mut stegoeggo_v1_error_t, dto: ErrorDto) -> u32 {
    let code = dto.code;
    if !out_error.is_null() {
        let handle = Box::new(stegoeggo_v1_error_t { inner: dto });
        unsafe {
            *out_error = Box::into_raw(handle);
        }
    }
    code
}

pub(crate) fn clear_error(out_error: *mut *mut stegoeggo_v1_error_t) {
    if !out_error.is_null() {
        unsafe {
            *out_error = core::ptr::null_mut();
        }
    }
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_error_code(
    error: *const stegoeggo_v1_error_t,
) -> crate::codes::stegoeggo_v1_error_code_t {
    if error.is_null() {
        return codes::STEGOEGGO_V1_ERR_INVALID_ARGUMENT;
    }
    unsafe { (*error).inner.code }
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_error_resource(
    error: *const stegoeggo_v1_error_t,
) -> crate::codes::stegoeggo_v1_resource_code_t {
    if error.is_null() {
        return codes::STEGOEGGO_V1_RESOURCE_NONE;
    }
    unsafe { (*error).inner.resource }
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_error_message_data(
    error: *const stegoeggo_v1_error_t,
) -> *const core::ffi::c_char {
    if error.is_null() {
        return core::ptr::null();
    }
    let message = unsafe { &(*error).inner.message };
    message.as_ptr() as *const core::ffi::c_char
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_error_message_len(error: *const stegoeggo_v1_error_t) -> usize {
    if error.is_null() {
        return 0;
    }
    let message = unsafe { &(*error).inner.message };
    message.len()
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_error_details_json(
    error: *const stegoeggo_v1_error_t,
) -> *mut crate::handles::stegoeggo_v1_buffer_t {
    use crate::handles::stegoeggo_v1_buffer_t;
    if error.is_null() {
        return core::ptr::null_mut();
    }
    let bytes = unsafe { &(*error).inner.details_json }.clone().into_bytes();
    Box::into_raw(Box::new(stegoeggo_v1_buffer_t { bytes }))
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_error_free(error: *mut stegoeggo_v1_error_t) {
    if error.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(error));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rust_variant_maps_to_frozen_category() {
        let cases: Vec<(stegoeggo::Error, u32, u32)> = vec![
            (
                stegoeggo::Error::Config("c".to_string()),
                codes::STEGOEGGO_V1_ERR_INVALID_CONFIGURATION,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            (
                stegoeggo::Error::InvalidFormat("c".to_string()),
                codes::STEGOEGGO_V1_ERR_INVALID_FORMAT,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            (
                stegoeggo::Error::ImageDecode("c".to_string()),
                codes::STEGOEGGO_V1_ERR_ENCODE_DECODE,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            (
                stegoeggo::Error::ImageEncode("c".to_string()),
                codes::STEGOEGGO_V1_ERR_ENCODE_DECODE,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            (
                stegoeggo::Error::ImageTruncated("c".to_string()),
                codes::STEGOEGGO_V1_ERR_ENCODE_DECODE,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            (
                stegoeggo::Error::Metadata("c".to_string()),
                codes::STEGOEGGO_V1_ERR_METADATA,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            (
                stegoeggo::Error::Steganography("c".to_string()),
                codes::STEGOEGGO_V1_ERR_STEGANOGRAPHY,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            (
                stegoeggo::Error::InsufficientCapacity {
                    required: 100,
                    available: 10,
                },
                codes::STEGOEGGO_V1_ERR_INSUFFICIENT_CAPACITY,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            (
                stegoeggo::Error::PayloadVerification("c".to_string()),
                codes::STEGOEGGO_V1_ERR_VERIFICATION,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            (
                stegoeggo::Error::Crypto("c".to_string()),
                codes::STEGOEGGO_V1_ERR_VERIFICATION,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            (
                stegoeggo::Error::InputTooLarge { size: 9, limit: 1 },
                codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                codes::STEGOEGGO_V1_RESOURCE_INPUT_BYTES,
            ),
            (
                stegoeggo::Error::DimensionsExceeded {
                    width: 9,
                    height: 9,
                    max_width: 1,
                    max_height: 1,
                },
                codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                codes::STEGOEGGO_V1_RESOURCE_DIMENSIONS,
            ),
            (
                stegoeggo::Error::ContainerLimitExceeded {
                    kind: "PNG chunks",
                    count: 9,
                    limit: 1,
                },
                codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                codes::STEGOEGGO_V1_RESOURCE_CONTAINER,
            ),
            (
                stegoeggo::Error::MetadataLimitExceeded {
                    kind: "XMP",
                    size: 9,
                    limit: 1,
                },
                codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                codes::STEGOEGGO_V1_RESOURCE_METADATA,
            ),
            (
                stegoeggo::Error::VerificationBudgetExceeded {
                    kind: "seeds",
                    count: 9,
                    limit: 1,
                },
                codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                codes::STEGOEGGO_V1_RESOURCE_VERIFICATION_BUDGET,
            ),
            (
                stegoeggo::Error::ResourceLimitExceeded("carrier".to_string()),
                codes::STEGOEGGO_V1_ERR_RESOURCE_LIMIT,
                codes::STEGOEGGO_V1_RESOURCE_CARRIER,
            ),
            (
                stegoeggo::Error::Serialization(
                    serde_json::from_str::<serde_json::Value>("nope").unwrap_err(),
                ),
                codes::STEGOEGGO_V1_ERR_INTERNAL,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
            (
                stegoeggo::Error::Iscc("c".to_string()),
                codes::STEGOEGGO_V1_ERR_INTERNAL,
                codes::STEGOEGGO_V1_RESOURCE_NONE,
            ),
        ];
        for (error, code, resource) in cases {
            let dto = ErrorDto::from_rust(&error);
            assert_eq!(dto.code, code, "code for {error:?}");
            assert_eq!(dto.resource, resource, "resource for {error:?}");
            assert!(!dto.message.is_empty());
            let parsed: serde_json::Value =
                serde_json::from_str(&dto.details_json).expect("details must be JSON");
            assert_eq!(parsed["schema_version"], 1);
            assert_eq!(parsed["code"], code);
            assert_eq!(parsed["resource"], resource);
        }
    }

    #[test]
    fn capacity_details_carry_counts() {
        let dto = ErrorDto::from_rust(&stegoeggo::Error::InsufficientCapacity {
            required: 100,
            available: 10,
        });
        let parsed: serde_json::Value = serde_json::from_str(&dto.details_json).expect("JSON");
        assert_eq!(parsed["required"], 100);
        assert_eq!(parsed["available"], 10);
    }

    #[test]
    fn carrier_details_carry_base_fields_only() {
        let dto = ErrorDto::from_rust(&stegoeggo::Error::ResourceLimitExceeded(
            "carrier overflow".to_string(),
        ));
        let parsed: serde_json::Value = serde_json::from_str(&dto.details_json).expect("JSON");
        assert_eq!(parsed.as_object().expect("object").len(), 4);
        assert!(dto.message.contains("carrier overflow"));
    }

    #[test]
    fn null_error_accessors_use_sentinels() {
        assert_eq!(
            stegoeggo_v1_error_code(core::ptr::null()),
            codes::STEGOEGGO_V1_ERR_INVALID_ARGUMENT
        );
        assert_eq!(
            stegoeggo_v1_error_resource(core::ptr::null()),
            codes::STEGOEGGO_V1_RESOURCE_NONE
        );
        assert!(stegoeggo_v1_error_message_data(core::ptr::null()).is_null());
        assert_eq!(stegoeggo_v1_error_message_len(core::ptr::null()), 0);
        assert!(stegoeggo_v1_error_details_json(core::ptr::null()).is_null());
        stegoeggo_v1_error_free(core::ptr::null_mut());
    }
}
