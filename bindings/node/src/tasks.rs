use crate::enums::ProtectionWarning;
use crate::error::NativeError;
use crate::limits::ResourceLimits;
use crate::report::{ExecutionReport, VerificationReport};
use crate::request::ProtectionRequest;
use napi::bindgen_prelude::{AsyncTask, Buffer, Env, Result, Task};
use napi_derive::napi;
use std::panic::{catch_unwind, AssertUnwindSafe};
use stegoeggo::{
    process_request_bytes, process_request_bytes_with_report, process_request_bytes_with_warnings,
    verify_image_bytes_report, verify_image_bytes_report_with_limits,
    ProtectionRequest as RustRequest, ResourceLimits as RustLimits,
};

/// Runs a closure on a libuv worker and converts an unwind into a structured
/// internal failure instead of aborting the host process.
///
/// The binding is built with `panic = "unwind"` in both dev and release, so
/// the canonical call can be contained here without changing core Rust
/// semantics.
#[allow(clippy::result_large_err)]
fn guarded<T>(operation: impl FnOnce() -> T) -> std::result::Result<T, NativeError> {
    catch_unwind(AssertUnwindSafe(operation))
        .map_err(|_| NativeError::internal("native operation panicked"))
}

/// Result of a `protect` call: the protected bytes, or a structured failure.
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct ProtectOutcome {
    /// Protected encoded bytes, or `null` on failure.
    pub data: Option<Buffer>,
    /// Structured failure, or `null` on success.
    pub error: Option<NativeError>,
}

/// Result of a `protectWithWarnings` call.
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct ProtectWithWarningsOutcome {
    /// Protected encoded bytes, or `null` on failure.
    pub data: Option<Buffer>,
    /// Degradation and advisory conditions, or `null` on failure.
    pub warnings: Option<Vec<ProtectionWarning>>,
    /// Structured failure, or `null` on success.
    pub error: Option<NativeError>,
}

/// Result of a `protectWithReport` call.
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct ProtectWithReportOutcome {
    /// Protected encoded bytes, or `null` on failure.
    pub data: Option<Buffer>,
    /// Execution report, or `null` on failure.
    pub report: Option<ExecutionReport>,
    /// Structured failure, or `null` on success.
    pub error: Option<NativeError>,
}

/// Result of a `verify` call.
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct VerifyOutcome {
    /// Canonical verification report, or `null` on failure.
    pub report: Option<VerificationReport>,
    /// Structured failure, or `null` on success.
    pub error: Option<NativeError>,
}

pub struct ProtectTask {
    data: Vec<u8>,
    request: RustRequest,
}

#[napi]
impl Task for ProtectTask {
    type Output = ProtectOutcome;
    type JsValue = ProtectOutcome;

    fn compute(&mut self) -> Result<Self::Output> {
        let data = std::mem::take(&mut self.data);
        let request = self.request.clone();
        let outcome = guarded(move || process_request_bytes(&data, &request));
        Ok(match outcome {
            Ok(Ok(bytes)) => ProtectOutcome {
                data: Some(bytes.into()),
                error: None,
            },
            Ok(Err(error)) => ProtectOutcome {
                data: None,
                error: Some(NativeError::from_rust(&error)),
            },
            Err(error) => ProtectOutcome {
                data: None,
                error: Some(error),
            },
        })
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

pub struct ProtectWithWarningsTask {
    data: Vec<u8>,
    request: RustRequest,
}

#[napi]
impl Task for ProtectWithWarningsTask {
    type Output = ProtectWithWarningsOutcome;
    type JsValue = ProtectWithWarningsOutcome;

    fn compute(&mut self) -> Result<Self::Output> {
        let data = std::mem::take(&mut self.data);
        let request = self.request.clone();
        let outcome = guarded(move || process_request_bytes_with_warnings(&data, &request));
        Ok(match outcome {
            Ok(Ok((bytes, warnings))) => ProtectWithWarningsOutcome {
                data: Some(bytes.into()),
                warnings: Some(warnings.into_iter().map(Into::into).collect()),
                error: None,
            },
            Ok(Err(error)) => ProtectWithWarningsOutcome {
                data: None,
                warnings: None,
                error: Some(NativeError::from_rust(&error)),
            },
            Err(error) => ProtectWithWarningsOutcome {
                data: None,
                warnings: None,
                error: Some(error),
            },
        })
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

pub struct ProtectWithReportTask {
    data: Vec<u8>,
    request: RustRequest,
}

#[napi]
impl Task for ProtectWithReportTask {
    type Output = ProtectWithReportOutcome;
    type JsValue = ProtectWithReportOutcome;

    fn compute(&mut self) -> Result<Self::Output> {
        let data = std::mem::take(&mut self.data);
        let request = self.request.clone();
        let outcome = guarded(move || process_request_bytes_with_report(&data, &request));
        Ok(match outcome {
            Ok(Ok((bytes, report))) => ProtectWithReportOutcome {
                data: Some(bytes.into()),
                report: Some(ExecutionReport::from(&report)),
                error: None,
            },
            Ok(Err(error)) => ProtectWithReportOutcome {
                data: None,
                report: None,
                error: Some(NativeError::from_rust(&error)),
            },
            Err(error) => ProtectWithReportOutcome {
                data: None,
                report: None,
                error: Some(error),
            },
        })
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

pub struct VerifyTask {
    data: Vec<u8>,
    mac_key: Vec<u8>,
    resource_limits: Option<RustLimits>,
}

#[napi]
impl Task for VerifyTask {
    type Output = VerifyOutcome;
    type JsValue = VerifyOutcome;

    fn compute(&mut self) -> Result<Self::Output> {
        let data = std::mem::take(&mut self.data);
        let mac_key = std::mem::take(&mut self.mac_key);
        let limits = self.resource_limits.take();
        let outcome = guarded(move || match &limits {
            Some(limits) => verify_image_bytes_report_with_limits(&data, &mac_key, limits),
            None => verify_image_bytes_report(&data, &mac_key),
        });
        Ok(match outcome {
            Ok(report) => VerifyOutcome {
                report: Some(VerificationReport::from(&report)),
                error: None,
            },
            Err(error) => VerifyOutcome {
                report: None,
                error: Some(error),
            },
        })
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

/// Copies JavaScript-owned bytes into Rust-owned storage before the worker is
/// scheduled. Mutating the caller's `Buffer`/`Uint8Array` after this call
/// cannot change the bytes the worker processes.
#[must_use]
pub fn owned_bytes(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}

/// Native entry point behind the public `protect` Promise. Copies the
/// JavaScript bytes into Rust-owned storage before scheduling the worker.
#[napi(js_name = "protectInternal")]
pub fn protect_internal(data: &[u8], request: &ProtectionRequest) -> AsyncTask<ProtectTask> {
    AsyncTask::new(ProtectTask {
        data: owned_bytes(data),
        request: RustRequest::from(request),
    })
}

/// Native entry point behind the public `protectWithWarnings` Promise.
#[napi(js_name = "protectWithWarningsInternal")]
pub fn protect_with_warnings_internal(
    data: &[u8],
    request: &ProtectionRequest,
) -> AsyncTask<ProtectWithWarningsTask> {
    AsyncTask::new(ProtectWithWarningsTask {
        data: owned_bytes(data),
        request: RustRequest::from(request),
    })
}

/// Native entry point behind the public `protectWithReport` Promise.
#[napi(js_name = "protectWithReportInternal")]
pub fn protect_with_report_internal(
    data: &[u8],
    request: &ProtectionRequest,
) -> AsyncTask<ProtectWithReportTask> {
    AsyncTask::new(ProtectWithReportTask {
        data: owned_bytes(data),
        request: RustRequest::from(request),
    })
}

/// Native entry point behind the public `verify` Promise.
#[napi(js_name = "verifyInternal")]
pub fn verify_internal(
    data: &[u8],
    mac_key: Option<&[u8]>,
    resource_limits: Option<&ResourceLimits>,
) -> AsyncTask<VerifyTask> {
    AsyncTask::new(VerifyTask {
        data: owned_bytes(data),
        mac_key: mac_key.map_or_else(Vec::new, ToOwned::to_owned),
        resource_limits: resource_limits.map(RustLimits::from),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use stegoeggo::RightsNotice;

    fn request() -> RustRequest {
        RustRequest::metadata_only(
            RightsNotice::new().with_copyright_holder("Acme"),
            stegoeggo::RightsPolicy::ProhibitedAiMlTraining,
        )
    }

    #[test]
    fn owned_bytes_is_independent_of_the_source() {
        let mut source = vec![1_u8, 2, 3];
        let copy = owned_bytes(&source);
        source[0] = 9;
        assert_eq!(copy, vec![1, 2, 3]);
    }

    #[test]
    fn protect_task_reports_structured_failure() {
        let mut task = ProtectTask {
            data: b"not a real image".to_vec(),
            request: request(),
        };
        let outcome = task.compute().unwrap();
        assert!(outcome.data.is_none());
        let error = outcome.error.unwrap();
        assert_eq!(error.code, crate::error::ErrorCode::InvalidFormat);
    }

    #[test]
    fn truncated_input_yields_a_structured_failure() {
        let mut task = ProtectTask {
            data: b"\x89PNG\r\n\x1a\n".to_vec(),
            request: request(),
        };
        let outcome = task.compute().unwrap();
        assert!(outcome.data.is_none());
        assert!(outcome.error.is_some());
    }

    #[test]
    fn verify_task_never_fails_for_untrusted_bytes() {
        let mut task = VerifyTask {
            data: b"not a real image".to_vec(),
            mac_key: Vec::new(),
            resource_limits: None,
        };
        let outcome = task.compute().unwrap();
        assert!(outcome.error.is_none());
        assert!(outcome.report.is_some());
    }

    #[test]
    fn panics_become_internal_failures() {
        let outcome: std::result::Result<Vec<u8>, NativeError> = guarded(|| panic!("boom"));
        assert!(outcome.is_err());
    }
}
