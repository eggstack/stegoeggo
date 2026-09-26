use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyTuple};
use std::ops::DerefMut;

use stegoeggo::verification::VerificationReport as RustVerificationReport;
use stegoeggo::{
    process_request_bytes, process_request_bytes_with_report, process_request_bytes_with_warnings,
    verify_image_bytes_report, verify_image_bytes_report_with_limits, AuthenticationMode,
    DmiValue as RustDmiValue, Error as RustError, EvidenceStrength, HiddenMarkerMode as RustHidden,
    ImageOutputFormat as RustFormat, MetadataUpdatePolicy as RustMetadataPolicy,
    ProcessingOptions as RustProcessingOptions, ProtectionPreset as RustPreset,
    ProtectionRequest as RustRequest, ProtectionWarning as RustWarning,
    resource_limits::ResourceLimitsBuilder as RustLimitsBuilder, ResourceLimits as RustLimits,
    ResourceUsage, RightsNotice as RustRightsNotice, RightsPolicy as RustRightsPolicy,
    VerificationStatus as RustStatus,
};

create_exception!(
    stegoeggo._native,
    StegoEggoError,
    PyException,
    "Base exception for stegoeggo binding errors."
);
create_exception!(
    stegoeggo._native,
    InvalidConfigError,
    StegoEggoError,
    "Configuration or request validation error."
);
create_exception!(
    stegoeggo._native,
    InvalidFormatError,
    StegoEggoError,
    "Image format could not be determined or is unsupported."
);
create_exception!(
    stegoeggo._native,
    EncodeDecodeError,
    StegoEggoError,
    "Image decoding or encoding failed."
);
create_exception!(
    stegoeggo._native,
    MetadataError,
    StegoEggoError,
    "Metadata operation failed."
);
create_exception!(
    stegoeggo._native,
    SteganographyError,
    StegoEggoError,
    "Steganographic embedding or extraction failed."
);
create_exception!(
    stegoeggo._native,
    InsufficientCapacityError,
    StegoEggoError,
    "Carrier lacks capacity for the requested payload."
);
create_exception!(
    stegoeggo._native,
    VerificationError,
    StegoEggoError,
    "Payload verification (CRC32 or HMAC) failed."
);
create_exception!(
    stegoeggo._native,
    ResourceLimitError,
    StegoEggoError,
    "Configured resource limit exceeded during parsing or extraction."
);

fn map_error(err: RustError) -> PyErr {
    match &err {
        RustError::Config(_) => PyErr::new::<InvalidConfigError, _>(err.to_string()),
        RustError::InvalidFormat(_) => PyErr::new::<InvalidFormatError, _>(err.to_string()),
        RustError::ImageDecode(_) | RustError::ImageEncode(_) | RustError::Image(_) => {
            PyErr::new::<EncodeDecodeError, _>(err.to_string())
        }
        RustError::Metadata(_) => PyErr::new::<MetadataError, _>(err.to_string()),
        RustError::Steganography(_) => PyErr::new::<SteganographyError, _>(err.to_string()),
        RustError::InsufficientCapacity { .. } => {
            PyErr::new::<InsufficientCapacityError, _>(err.to_string())
        }
        RustError::PayloadVerification(_) | RustError::Crypto(_) => {
            PyErr::new::<VerificationError, _>(err.to_string())
        }
        RustError::InputTooLarge { .. }
        | RustError::DimensionsExceeded { .. }
        | RustError::ContainerLimitExceeded { .. }
        | RustError::MetadataLimitExceeded { .. }
        | RustError::VerificationBudgetExceeded { .. } => {
            PyErr::new::<ResourceLimitError, _>(err.to_string())
        }
        _ => PyErr::new::<StegoEggoError, _>(err.to_string()),
    }
}

fn opt_string(value: Option<&str>) -> Option<String> {
    value.map(|s| s.to_string())
}

fn rust_dmi_name(d: RustDmiValue) -> &'static str {
    #[allow(unreachable_patterns)]
    match d {
        RustDmiValue::Unspecified => "UNSPECIFIED",
        RustDmiValue::Allowed => "ALLOWED",
        RustDmiValue::ProhibitedAiMlTraining => "PROHIBITED_AI_ML_TRAINING",
        RustDmiValue::ProhibitedGenAiMlTraining => "PROHIBITED_GEN_AI_ML_TRAINING",
        RustDmiValue::ProhibitedExceptSearchEngineIndexing => {
            "PROHIBITED_EXCEPT_SEARCH_ENGINE_INDEXING"
        }
        RustDmiValue::Prohibited => "PROHIBITED",
        RustDmiValue::ProhibitedSeeConstraints => "PROHIBITED_SEE_CONSTRAINTS",
        _ => "UNKNOWN",
    }
}

#[pyclass(name = "RightsPolicy", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyRightsPolicy {
    Unspecified = 0,
    Allowed = 1,
    ProhibitedAiMlTraining = 2,
    ProhibitedGenerativeAiTraining = 3,
    ProhibitedExceptSearchIndexing = 4,
    ProhibitedAllDataMining = 5,
    ProhibitedSeeConstraints = 6,
}

#[pymethods]
impl PyRightsPolicy {
    #[new]
    fn new() -> Self {
        Self::Unspecified
    }

    fn __repr__(&self) -> String {
        format!("RightsPolicy.{}", self.name())
    }

    #[getter]
    fn name(&self) -> &'static str {
        #[allow(unreachable_patterns)]
        match self {
            Self::Unspecified => "UNSPECIFIED",
            Self::Allowed => "ALLOWED",
            Self::ProhibitedAiMlTraining => "PROHIBITED_AI_ML_TRAINING",
            Self::ProhibitedGenerativeAiTraining => "PROHIBITED_GENERATIVE_AI_TRAINING",
            Self::ProhibitedExceptSearchIndexing => "PROHIBITED_EXCEPT_SEARCH_INDEXING",
            Self::ProhibitedAllDataMining => "PROHIBITED_ALL_DATA_MINING",
            Self::ProhibitedSeeConstraints => "PROHIBITED_SEE_CONSTRAINTS",
            _ => "UNKNOWN",
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }
}

impl From<PyRightsPolicy> for RustRightsPolicy {
    fn from(v: PyRightsPolicy) -> Self {
        match v {
            PyRightsPolicy::Unspecified => Self::Unspecified,
            PyRightsPolicy::Allowed => Self::Allowed,
            PyRightsPolicy::ProhibitedAiMlTraining => Self::ProhibitedAiMlTraining,
            PyRightsPolicy::ProhibitedGenerativeAiTraining => Self::ProhibitedGenerativeAiTraining,
            PyRightsPolicy::ProhibitedExceptSearchIndexing => Self::ProhibitedExceptSearchIndexing,
            PyRightsPolicy::ProhibitedAllDataMining => Self::ProhibitedAllDataMining,
            PyRightsPolicy::ProhibitedSeeConstraints => Self::ProhibitedSeeConstraints,
            #[allow(unreachable_patterns)]
            _ => Self::Unspecified,
        }
    }
}

impl From<RustRightsPolicy> for PyRightsPolicy {
    fn from(p: RustRightsPolicy) -> Self {
        match p {
            RustRightsPolicy::Unspecified => Self::Unspecified,
            RustRightsPolicy::Allowed => Self::Allowed,
            RustRightsPolicy::ProhibitedAiMlTraining => Self::ProhibitedAiMlTraining,
            RustRightsPolicy::ProhibitedGenerativeAiTraining => {
                Self::ProhibitedGenerativeAiTraining
            }
            RustRightsPolicy::ProhibitedExceptSearchIndexing => {
                Self::ProhibitedExceptSearchIndexing
            }
            RustRightsPolicy::ProhibitedAllDataMining => Self::ProhibitedAllDataMining,
            RustRightsPolicy::ProhibitedSeeConstraints => Self::ProhibitedSeeConstraints,
            #[allow(unreachable_patterns)]
            _ => Self::Unspecified,
        }
    }
}

#[pyclass(name = "DmiValue", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyDmiValue {
    Unspecified = 0,
    Allowed = 1,
    ProhibitedAiMlTraining = 2,
    ProhibitedGenAiMlTraining = 3,
    ProhibitedExceptSearchEngineIndexing = 4,
    Prohibited = 5,
    ProhibitedSeeConstraints = 6,
}

#[pymethods]
impl PyDmiValue {
    #[new]
    fn new() -> Self {
        Self::Unspecified
    }

    fn __repr__(&self) -> String {
        format!("DmiValue.{}", self.name())
    }

    #[getter]
    fn name(&self) -> &'static str {
        #[allow(unreachable_patterns)]
        match self {
            Self::Unspecified => "UNSPECIFIED",
            Self::Allowed => "ALLOWED",
            Self::ProhibitedAiMlTraining => "PROHIBITED_AI_ML_TRAINING",
            Self::ProhibitedGenAiMlTraining => "PROHIBITED_GEN_AI_ML_TRAINING",
            Self::ProhibitedExceptSearchEngineIndexing => "PROHIBITED_EXCEPT_SEARCH_ENGINE_INDEXING",
            Self::Prohibited => "PROHIBITED",
            Self::ProhibitedSeeConstraints => "PROHIBITED_SEE_CONSTRAINTS",
            _ => "UNKNOWN",
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }
}

impl From<PyDmiValue> for RustDmiValue {
    fn from(v: PyDmiValue) -> Self {
        match v {
            PyDmiValue::Unspecified => Self::Unspecified,
            PyDmiValue::Allowed => Self::Allowed,
            PyDmiValue::ProhibitedAiMlTraining => Self::ProhibitedAiMlTraining,
            PyDmiValue::ProhibitedGenAiMlTraining => Self::ProhibitedGenAiMlTraining,
            PyDmiValue::ProhibitedExceptSearchEngineIndexing => {
                Self::ProhibitedExceptSearchEngineIndexing
            }
            PyDmiValue::Prohibited => Self::Prohibited,
            PyDmiValue::ProhibitedSeeConstraints => Self::ProhibitedSeeConstraints,
            #[allow(unreachable_patterns)]
            _ => Self::Unspecified,
        }
    }
}

impl From<RustDmiValue> for PyDmiValue {
    fn from(d: RustDmiValue) -> Self {
        match d {
            RustDmiValue::Unspecified => Self::Unspecified,
            RustDmiValue::Allowed => Self::Allowed,
            RustDmiValue::ProhibitedAiMlTraining => Self::ProhibitedAiMlTraining,
            RustDmiValue::ProhibitedGenAiMlTraining => Self::ProhibitedGenAiMlTraining,
            RustDmiValue::ProhibitedExceptSearchEngineIndexing => {
                Self::ProhibitedExceptSearchEngineIndexing
            }
            RustDmiValue::Prohibited => Self::Prohibited,
            RustDmiValue::ProhibitedSeeConstraints => Self::ProhibitedSeeConstraints,
            #[allow(unreachable_patterns)]
            _ => Self::Unspecified,
        }
    }
}

#[pyclass(name = "ImageOutputFormat", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyImageOutputFormat {
    Png = 0,
    Jpeg = 1,
    WebP = 2,
}

#[pymethods]
impl PyImageOutputFormat {
    #[new]
    fn new() -> Self {
        Self::Png
    }

    fn __repr__(&self) -> String {
        format!("ImageOutputFormat.{}", self.name())
    }

    #[getter]
    fn name(&self) -> &'static str {
        #[allow(unreachable_patterns)]
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::WebP => "WEBP",
            _ => "UNKNOWN",
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }

    #[staticmethod]
    fn from_extension(ext: &str) -> Option<Self> {
        RustFormat::from_extension(ext).map(|f| f.into())
    }
}

impl From<RustFormat> for PyImageOutputFormat {
    fn from(f: RustFormat) -> Self {
        match f {
            RustFormat::Png => Self::Png,
            RustFormat::Jpeg => Self::Jpeg,
            RustFormat::WebP => Self::WebP,
            #[allow(unreachable_patterns)]
            _ => Self::Png,
        }
    }
}

impl From<PyImageOutputFormat> for RustFormat {
    fn from(f: PyImageOutputFormat) -> Self {
        match f {
            PyImageOutputFormat::Png => Self::Png,
            PyImageOutputFormat::Jpeg => Self::Jpeg,
            PyImageOutputFormat::WebP => Self::WebP,
            #[allow(unreachable_patterns)]
            _ => Self::Png,
        }
    }
}

#[pyclass(name = "MetadataUpdatePolicy", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyMetadataUpdatePolicy {
    ReplaceStegoOwned = 0,
    FailOnConflict = 1,
    PreserveExisting = 2,
}

#[pymethods]
impl PyMetadataUpdatePolicy {
    #[new]
    fn new() -> Self {
        Self::ReplaceStegoOwned
    }

    fn __repr__(&self) -> String {
        format!("MetadataUpdatePolicy.{}", self.name())
    }

    #[getter]
    fn name(&self) -> &'static str {
        #[allow(unreachable_patterns)]
        match self {
            Self::ReplaceStegoOwned => "REPLACE_STEGO_OWNED",
            Self::FailOnConflict => "FAIL_ON_CONFLICT",
            Self::PreserveExisting => "PRESERVE_EXISTING",
            _ => "UNKNOWN",
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }
}

impl From<PyMetadataUpdatePolicy> for RustMetadataPolicy {
    fn from(p: PyMetadataUpdatePolicy) -> Self {
        match p {
            PyMetadataUpdatePolicy::ReplaceStegoOwned => Self::ReplaceStegoOwned,
            PyMetadataUpdatePolicy::FailOnConflict => Self::FailOnConflict,
            PyMetadataUpdatePolicy::PreserveExisting => Self::PreserveExisting,
            #[allow(unreachable_patterns)]
            _ => Self::ReplaceStegoOwned,
        }
    }
}

#[pyclass(name = "ProtectionPreset", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyProtectionPreset {
    LegalNotice = 0,
    LegalNoticeWithStego = 1,
    AuthenticatedProvenance = 2,
    Maximal = 3,
}

#[pymethods]
impl PyProtectionPreset {
    #[new]
    fn new() -> Self {
        Self::LegalNotice
    }

    fn __repr__(&self) -> String {
        format!("ProtectionPreset.{}", self.name())
    }

    #[getter]
    fn name(&self) -> &'static str {
        #[allow(unreachable_patterns)]
        match self {
            Self::LegalNotice => "LEGAL_NOTICE",
            Self::LegalNoticeWithStego => "LEGAL_NOTICE_WITH_STEGO",
            Self::AuthenticatedProvenance => "AUTHENTICATED_PROVENANCE",
            Self::Maximal => "MAXIMAL",
            _ => "UNKNOWN",
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }
}

impl From<PyProtectionPreset> for RustPreset {
    fn from(p: PyProtectionPreset) -> Self {
        match p {
            PyProtectionPreset::LegalNotice => Self::LegalNotice,
            PyProtectionPreset::LegalNoticeWithStego => Self::LegalNoticeWithStego,
            PyProtectionPreset::AuthenticatedProvenance => Self::AuthenticatedProvenance,
            PyProtectionPreset::Maximal => Self::Maximal,
            #[allow(unreachable_patterns)]
            _ => Self::LegalNotice,
        }
    }
}

#[pyclass(name = "AuthenticationMode", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyAuthenticationMode {
    None = 0,
    Hmac = 1,
}

#[pymethods]
impl PyAuthenticationMode {
    #[new]
    fn new() -> Self {
        Self::None
    }

    fn __repr__(&self) -> String {
        format!("AuthenticationMode.{}", self.name())
    }

    #[getter]
    fn name(&self) -> &'static str {
        #[allow(unreachable_patterns)]
        match self {
            Self::None => "NONE",
            Self::Hmac => "HMAC",
            _ => "UNKNOWN",
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }
}

impl From<PyAuthenticationMode> for AuthenticationMode {
    fn from(m: PyAuthenticationMode) -> Self {
        match m {
            PyAuthenticationMode::None => Self::None,
            PyAuthenticationMode::Hmac => Self::Hmac,
            #[allow(unreachable_patterns)]
            _ => Self::None,
        }
    }
}

impl From<AuthenticationMode> for PyAuthenticationMode {
    fn from(m: AuthenticationMode) -> Self {
        match m {
            AuthenticationMode::None => Self::None,
            AuthenticationMode::Hmac => Self::Hmac,
            #[allow(unreachable_patterns)]
            _ => Self::None,
        }
    }
}

#[pyclass(name = "HiddenMarkerMode")]
#[derive(Clone, Debug)]
pub struct PyHiddenMarkerMode {
    inner: RustHidden,
}

#[pymethods]
impl PyHiddenMarkerMode {
    #[staticmethod]
    fn disabled() -> Self {
        Self {
            inner: RustHidden::Disabled,
        }
    }

    #[staticmethod]
    fn seed_only() -> Self {
        Self {
            inner: RustHidden::SeedOnly,
        }
    }

    #[staticmethod]
    fn best_effort() -> Self {
        Self {
            inner: RustHidden::BestEffort,
        }
    }

    #[staticmethod]
    fn tiled(tile_size: u32) -> PyResult<Self> {
        if !(32..=1024).contains(&tile_size) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "tile_size {tile_size} out of range 32..=1024"
            )));
        }
        Ok(Self {
            inner: RustHidden::Tiled { tile_size },
        })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            RustHidden::Disabled => "HiddenMarkerMode.disabled()".to_string(),
            RustHidden::SeedOnly => "HiddenMarkerMode.seed_only()".to_string(),
            RustHidden::BestEffort => "HiddenMarkerMode.best_effort()".to_string(),
            RustHidden::Tiled { tile_size } => {
                format!("HiddenMarkerMode.tiled({tile_size})")
            }
            _ => "HiddenMarkerMode(<unknown>)".to_string(),
        }
    }

    fn __eq__(&self, other: &PyHiddenMarkerMode) -> bool {
        self.inner == other.inner
    }

    #[getter]
    fn is_tiled(&self) -> bool {
        matches!(self.inner, RustHidden::Tiled { .. })
    }

    #[getter]
    fn tile_size(&self) -> Option<u32> {
        match self.inner {
            RustHidden::Tiled { tile_size } => Some(tile_size),
            _ => None,
        }
    }
}

impl From<RustHidden> for PyHiddenMarkerMode {
    fn from(h: RustHidden) -> Self {
        Self { inner: h }
    }
}

fn notice_repr(n: &RustRightsNotice) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = n.copyright_holder() {
        parts.push(format!("copyright_holder={v:?}"));
    }
    if let Some(v) = n.contact_email() {
        parts.push(format!("contact_email={v:?}"));
    }
    if let Some(v) = n.license_url() {
        parts.push(format!("license_url={v:?}"));
    }
    if let Some(v) = n.usage_terms() {
        parts.push(format!("usage_terms={v:?}"));
    }
    if let Some(v) = n.creation_date() {
        parts.push(format!("creation_date={v:?}"));
    }
    if let Some(v) = n.ai_constraints() {
        parts.push(format!("ai_constraints={v:?}"));
    }
    if let Some(v) = n.web_statement_of_rights() {
        parts.push(format!("web_statement_of_rights={v:?}"));
    }
    if let Some(v) = n.creator() {
        parts.push(format!("creator={v:?}"));
    }
    if let Some(v) = n.credit_line() {
        parts.push(format!("credit_line={v:?}"));
    }
    if let Some(v) = n.copyright_owner() {
        parts.push(format!("copyright_owner={v:?}"));
    }
    if let Some(v) = n.licensor_name() {
        parts.push(format!("licensor_name={v:?}"));
    }
    if let Some(v) = n.licensor_email() {
        parts.push(format!("licensor_email={v:?}"));
    }
    if let Some(v) = n.licensor_url() {
        parts.push(format!("licensor_url={v:?}"));
    }
    if let Some(v) = n.metadata_date() {
        parts.push(format!("metadata_date={v:?}"));
    }
    if let Some(v) = n.notice_applied_at() {
        parts.push(format!("notice_applied_at={v:?}"));
    }
    if let Some(v) = n.dmi() {
        parts.push(format!("dmi=DmiValue.{}", rust_dmi_name(v)));
    }
    format!("RightsNotice({})", parts.join(", "))
}

#[pyclass(name = "RightsNotice")]
#[derive(Clone, Debug)]
pub struct PyRightsNotice {
    inner: RustRightsNotice,
}

#[pymethods]
impl PyRightsNotice {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustRightsNotice::new(),
        }
    }

    fn with_copyright_holder(mut slf: PyRefMut<'_, Self>, holder: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_copyright_holder(holder);
        slf
    }

    fn with_copyright_owner(mut slf: PyRefMut<'_, Self>, owner: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_copyright_owner(owner);
        slf
    }

    fn with_contact_email(mut slf: PyRefMut<'_, Self>, email: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_contact_email(email);
        slf
    }

    fn with_license_url(mut slf: PyRefMut<'_, Self>, url: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_license_url(url);
        slf
    }

    fn with_usage_terms(mut slf: PyRefMut<'_, Self>, terms: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_usage_terms(terms);
        slf
    }

    fn with_creation_date(mut slf: PyRefMut<'_, Self>, date: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_creation_date(date);
        slf
    }

    fn with_ai_constraints(mut slf: PyRefMut<'_, Self>, c: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_ai_constraints(c);
        slf
    }

    fn with_web_statement_of_rights(
        mut slf: PyRefMut<'_,
        Self>,
        s: String,
    ) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_web_statement_of_rights(s);
        slf
    }

    fn with_creator(mut slf: PyRefMut<'_, Self>, c: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_creator(c);
        slf
    }

    fn with_credit_line(mut slf: PyRefMut<'_, Self>, l: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_credit_line(l);
        slf
    }

    fn with_licensor_name(mut slf: PyRefMut<'_, Self>, n: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_licensor_name(n);
        slf
    }

    fn with_licensor_email(mut slf: PyRefMut<'_, Self>, e: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_licensor_email(e);
        slf
    }

    fn with_licensor_url(mut slf: PyRefMut<'_, Self>, u: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_licensor_url(u);
        slf
    }

    fn with_metadata_date(mut slf: PyRefMut<'_, Self>, d: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_metadata_date(d);
        slf
    }

    fn with_notice_applied_at(mut slf: PyRefMut<'_, Self>, t: String) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_notice_applied_at(t);
        slf
    }

    fn with_dmi(mut slf: PyRefMut<'_, Self>, d: PyDmiValue) -> PyRefMut<'_, Self> {
        slf.inner = std::mem::take(&mut slf.inner).with_dmi(d.into());
        slf
    }

    fn __repr__(&self) -> String {
        notice_repr(&self.inner)
    }

    #[getter]
    fn copyright_holder(&self) -> Option<String> {
        opt_string(self.inner.copyright_holder())
    }

    #[getter]
    fn copyright_owner(&self) -> Option<String> {
        opt_string(self.inner.copyright_owner())
    }

    #[getter]
    fn contact_email(&self) -> Option<String> {
        opt_string(self.inner.contact_email())
    }

    #[getter]
    fn license_url(&self) -> Option<String> {
        opt_string(self.inner.license_url())
    }

    #[getter]
    fn usage_terms(&self) -> Option<String> {
        opt_string(self.inner.usage_terms())
    }

    #[getter]
    fn creation_date(&self) -> Option<String> {
        opt_string(self.inner.creation_date())
    }

    #[getter]
    fn ai_constraints(&self) -> Option<String> {
        opt_string(self.inner.ai_constraints())
    }

    #[getter]
    fn web_statement_of_rights(&self) -> Option<String> {
        opt_string(self.inner.web_statement_of_rights())
    }

    #[getter]
    fn creator(&self) -> Option<String> {
        opt_string(self.inner.creator())
    }

    #[getter]
    fn credit_line(&self) -> Option<String> {
        opt_string(self.inner.credit_line())
    }

    #[getter]
    fn licensor_name(&self) -> Option<String> {
        opt_string(self.inner.licensor_name())
    }

    #[getter]
    fn licensor_email(&self) -> Option<String> {
        opt_string(self.inner.licensor_email())
    }

    #[getter]
    fn licensor_url(&self) -> Option<String> {
        opt_string(self.inner.licensor_url())
    }

    #[getter]
    fn metadata_date(&self) -> Option<String> {
        opt_string(self.inner.metadata_date())
    }

    #[getter]
    fn notice_applied_at(&self) -> Option<String> {
        opt_string(self.inner.notice_applied_at())
    }

    #[getter]
    fn dmi(&self) -> Option<PyDmiValue> {
        self.inner.dmi().map(PyDmiValue::from)
    }

    #[getter]
    fn seed(&self) -> Option<u64> {
        self.inner.seed()
    }

    #[getter]
    fn has_legal_content(&self) -> bool {
        self.inner.has_legal_content()
    }
}

#[pyclass(name = "ProcessingOptions")]
#[derive(Clone, Debug)]
pub struct PyProcessingOptions {
    inner: RustProcessingOptions,
}

#[pymethods]
impl PyProcessingOptions {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustProcessingOptions::default(),
        }
    }

    fn with_output_format(
        mut slf: PyRefMut<'_,
        Self>,
        format: PyImageOutputFormat,
    ) -> PyRefMut<'_, Self> {
        slf.inner.output_format = Some(format.into());
        slf
    }

    fn with_jpeg_quality(mut slf: PyRefMut<'_, Self>, quality: u8) -> PyRefMut<'_, Self> {
        slf.inner.jpeg_quality = quality;
        slf
    }

    fn with_progressive_jpeg(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner.progressive_jpeg = true;
        slf
    }

    fn with_max_dimension(mut slf: PyRefMut<'_, Self>, max: u32) -> PyRefMut<'_, Self> {
        slf.inner.max_dimension = Some(max);
        slf
    }

    fn with_metadata_update_policy(
        mut slf: PyRefMut<'_,
        Self>,
        policy: PyMetadataUpdatePolicy,
    ) -> PyRefMut<'_, Self> {
        slf.inner.metadata_update_policy = policy.into();
        slf
    }

    fn with_stego_redundancy(mut slf: PyRefMut<'_, Self>, redundancy: usize) -> PyRefMut<'_, Self> {
        slf.inner.stego_redundancy = Some(redundancy);
        slf
    }

    fn with_content_hash(
        mut slf: PyRefMut<'_,
        Self>,
        hash: [u8; 4],
    ) -> PyRefMut<'_, Self> {
        slf.inner.content_hash = Some(hash);
        slf
    }

    fn with_timestamp_override(
        mut slf: PyRefMut<'_,
        Self>,
        timestamp: String,
    ) -> PyRefMut<'_, Self> {
        slf.inner.timestamp_override = Some(timestamp);
        slf
    }

    fn __repr__(&self) -> String {
        format!(
            "ProcessingOptions(output_format={:?}, jpeg_quality={}, progressive={}, max_dim={:?}, redundancy={:?})",
            self.inner.output_format.map(|f| f.extension()),
            self.inner.jpeg_quality,
            self.inner.progressive_jpeg,
            self.inner.max_dimension,
            self.inner.stego_redundancy,
        )
    }
}

#[pyclass(name = "ResourceLimits")]
#[derive(Clone, Debug)]
pub struct PyResourceLimits {
    inner: RustLimits,
}

#[pymethods]
impl PyResourceLimits {
    #[staticmethod]
    fn defaults() -> Self {
        Self {
            inner: RustLimits::default(),
        }
    }

    #[staticmethod]
    fn builder() -> PyResourceLimitsBuilder {
        PyResourceLimitsBuilder {
            inner: Some(RustLimits::builder()),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ResourceLimits(max_input_bytes={}, max_dim={}x{}, payload={}, tile_origins={})",
            self.inner.max_input_bytes(),
            self.inner.max_width(),
            self.inner.max_height(),
            self.inner.max_payload_bytes(),
            self.inner.max_tile_extraction_origins(),
        )
    }
}

#[pyclass(name = "ResourceLimitsBuilder")]
pub struct PyResourceLimitsBuilder {
    inner: Option<RustLimitsBuilder>,
}

#[pymethods]
impl PyResourceLimitsBuilder {
    fn with_max_input_bytes(mut slf: PyRefMut<'_, Self>, val: usize) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_input_bytes(val));
        slf
    }

    fn with_max_width(mut slf: PyRefMut<'_, Self>, val: u32) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_width(val));
        slf
    }

    fn with_max_height(mut slf: PyRefMut<'_, Self>, val: u32) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_height(val));
        slf
    }

    fn with_max_png_chunks(mut slf: PyRefMut<'_, Self>, val: usize) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_png_chunks(val));
        slf
    }

    fn with_max_jpeg_segments(mut slf: PyRefMut<'_, Self>, val: usize) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_jpeg_segments(val));
        slf
    }

    fn with_max_webp_riff_chunks(mut slf: PyRefMut<'_, Self>, val: usize) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_webp_riff_chunks(val));
        slf
    }

    fn with_max_xmp_bytes(mut slf: PyRefMut<'_, Self>, val: usize) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_xmp_bytes(val));
        slf
    }

    fn with_max_metadata_fields(mut slf: PyRefMut<'_, Self>, val: usize) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_metadata_fields(val));
        slf
    }

    fn with_max_metadata_field_bytes(mut slf: PyRefMut<'_, Self>, val: usize) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_metadata_field_bytes(val));
        slf
    }

    fn with_max_payload_bytes(mut slf: PyRefMut<'_, Self>, val: usize) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_payload_bytes(val));
        slf
    }

    fn with_max_tile_extraction_origins(mut slf: PyRefMut<'_, Self>, val: usize) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_tile_extraction_origins(val));
        slf
    }

    fn with_max_verification_seeds(mut slf: PyRefMut<'_, Self>, val: usize) -> PyRefMut<'_, Self> {
        let b = slf.inner.take().expect("builder initialized");
        slf.inner = Some(b.max_verification_seeds(val));
        slf
    }

    fn build(&mut self) -> PyResourceLimits {
        let b = self.inner.take().expect("builder initialized");
        PyResourceLimits {
            inner: b.build(),
        }
    }
}

#[pyclass(name = "ProtectionRequest")]
#[derive(Clone, Debug)]
pub struct PyProtectionRequest {
    inner: Option<RustRequest>,
}

#[pymethods]
impl PyProtectionRequest {
    #[staticmethod]
    fn metadata_only(notice: &PyRightsNotice, policy: PyRightsPolicy) -> Self {
        Self {
            inner: Some(RustRequest::metadata_only(notice.inner.clone(), policy.into())),
        }
    }

    #[staticmethod]
    fn with_hidden_marker(notice: &PyRightsNotice, policy: PyRightsPolicy) -> Self {
        Self {
            inner: Some(RustRequest::with_hidden_marker(
                notice.inner.clone(),
                policy.into(),
            )),
        }
    }

    #[staticmethod]
    fn from_preset(
        preset: PyProtectionPreset,
        notice: &PyRightsNotice,
        policy: PyRightsPolicy,
    ) -> Self {
        Self {
            inner: Some(RustRequest::from_preset(
                preset.into(),
                notice.inner.clone(),
                policy.into(),
            )),
        }
    }

    fn with_seed(mut slf: PyRefMut<'_, Self>, seed: u64) -> PyRefMut<'_, Self> {
        let r = slf.inner.take().expect("request initialized");
        slf.inner = Some(r.with_seed(seed));
        slf
    }

    fn with_intensity(mut slf: PyRefMut<'_, Self>, intensity: f32) -> PyRefMut<'_, Self> {
        let r = slf.inner.take().expect("request initialized");
        slf.inner = Some(r.with_intensity(intensity));
        slf
    }

    fn with_output_format(
        mut slf: PyRefMut<'_,
        Self>,
        format: PyImageOutputFormat,
    ) -> PyRefMut<'_, Self> {
        let r = slf.inner.take().expect("request initialized");
        slf.inner = Some(r.with_output_format(format.into()));
        slf
    }

    fn with_jpeg_quality(mut slf: PyRefMut<'_, Self>, quality: u8) -> PyRefMut<'_, Self> {
        let r = slf.inner.take().expect("request initialized");
        slf.inner = Some(r.with_jpeg_quality(quality));
        slf
    }

    fn with_progressive_jpeg(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        let r = slf.inner.take().expect("request initialized");
        slf.inner = Some(r.with_progressive_jpeg());
        slf
    }

    fn with_max_dimension(mut slf: PyRefMut<'_, Self>, max: u32) -> PyRefMut<'_, Self> {
        let r = slf.inner.take().expect("request initialized");
        slf.inner = Some(r.with_max_dimension(max));
        slf
    }

    fn with_metadata_update_policy(
        mut slf: PyRefMut<'_,
        Self>,
        policy: PyMetadataUpdatePolicy,
    ) -> PyRefMut<'_, Self> {
        let r = slf.inner.take().expect("request initialized");
        slf.inner = Some(r.with_metadata_update_policy(policy.into()));
        slf
    }

    fn with_stego_redundancy(mut slf: PyRefMut<'_, Self>, r: usize) -> PyRefMut<'_, Self> {
        let req = slf.inner.take().expect("request initialized");
        slf.inner = Some(req.with_stego_redundancy(r));
        slf
    }

    fn with_content_hash(mut slf: PyRefMut<'_, Self>, hash: [u8; 4]) -> PyRefMut<'_, Self> {
        let r = slf.inner.take().expect("request initialized");
        slf.inner = Some(r.with_content_hash(hash));
        slf
    }

    fn with_timestamp_override(mut slf: PyRefMut<'_, Self>, t: String) -> PyRefMut<'_, Self> {
        let r = slf.inner.take().expect("request initialized");
        slf.inner = Some(r.with_timestamp_override(t));
        slf
    }

    fn with_processing<'py>(
        mut slf: PyRefMut<'py, Self>,
        p: &PyProcessingOptions,
    ) -> PyRefMut<'py, Self> {
        let r = slf.deref_mut().inner.take().expect("request initialized");
        slf.deref_mut().inner = Some(r.with_processing(p.inner.clone()));
        slf
    }

    fn with_mac_key(mut slf: PyRefMut<'_, Self>, key: Vec<u8>) -> PyRefMut<'_, Self> {
        let r = slf.deref_mut().inner.take().expect("request initialized");
        slf.deref_mut().inner = Some(r.with_mac_key(key));
        slf
    }

    fn with_resource_limits<'py>(
        mut slf: PyRefMut<'py, Self>,
        l: &PyResourceLimits,
    ) -> PyRefMut<'py, Self> {
        let r = slf.deref_mut().inner.take().expect("request initialized");
        slf.deref_mut().inner = Some(r.with_resource_limits(l.inner.clone()));
        slf
    }

    fn __repr__(&self) -> String {
        let r = self.inner.as_ref().expect("request initialized");
        format!(
            "ProtectionRequest(policy={}, intensity={}, seed={:?})",
            rust_policy_name(r.policy()),
            r.intensity(),
            r.seed(),
        )
    }

    #[getter]
    fn policy(&self) -> PyRightsPolicy {
        self.inner.as_ref().expect("request initialized").policy().into()
    }

    #[getter]
    fn seed(&self) -> Option<u64> {
        self.inner.as_ref().expect("request initialized").seed()
    }

    #[getter]
    fn intensity(&self) -> f32 {
        self.inner.as_ref().expect("request initialized").intensity()
    }

    #[getter]
    fn has_mac_key(&self) -> bool {
        self.inner.as_ref().expect("request initialized").mac_key().is_some()
    }
}

fn rust_policy_name(p: RustRightsPolicy) -> &'static str {
    match p {
        RustRightsPolicy::Unspecified => "UNSPECIFIED",
        RustRightsPolicy::Allowed => "ALLOWED",
        RustRightsPolicy::ProhibitedAiMlTraining => "PROHIBITED_AI_ML_TRAINING",
        RustRightsPolicy::ProhibitedGenerativeAiTraining => "PROHIBITED_GENERATIVE_AI_TRAINING",
        RustRightsPolicy::ProhibitedExceptSearchIndexing => "PROHIBITED_EXCEPT_SEARCH_INDEXING",
        RustRightsPolicy::ProhibitedAllDataMining => "PROHIBITED_ALL_DATA_MINING",
        RustRightsPolicy::ProhibitedSeeConstraints => "PROHIBITED_SEE_CONSTRAINTS",
        _ => "UNKNOWN",
    }
}

#[pyclass(name = "ProtectionWarning", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyProtectionWarning {
    MissingMacKey = 0,
    MetadataInjectionDisabled = 1,
    ProgressiveJpegFallback = 2,
    JpegReencodeFragile = 3,
    LsbCapacitySkipped = 4,
    DctCapacityInsufficient = 5,
    ContradictoryLegalClaims = 6,
    MissingRightsConstraints = 7,
}

#[pymethods]
impl PyProtectionWarning {
    fn __repr__(&self) -> String {
        format!("ProtectionWarning.{}", self.name())
    }

    #[getter]
    fn name(&self) -> &'static str {
        #[allow(unreachable_patterns)]
        match self {
            Self::MissingMacKey => "MISSING_MAC_KEY",
            Self::MetadataInjectionDisabled => "METADATA_INJECTION_DISABLED",
            Self::ProgressiveJpegFallback => "PROGRESSIVE_JPEG_FALLBACK",
            Self::JpegReencodeFragile => "JPEG_REENCODE_FRAGILE",
            Self::LsbCapacitySkipped => "LSB_CAPACITY_SKIPPED",
            Self::DctCapacityInsufficient => "DCT_CAPACITY_INSUFFICIENT",
            Self::ContradictoryLegalClaims => "CONTRADICTORY_LEGAL_CLAIMS",
            Self::MissingRightsConstraints => "MISSING_RIGHTS_CONSTRAINTS",
            _ => "UNKNOWN",
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }
}

impl From<RustWarning> for PyProtectionWarning {
    fn from(w: RustWarning) -> Self {
        match w {
            RustWarning::MissingMacKey => Self::MissingMacKey,
            RustWarning::MetadataInjectionDisabled => Self::MetadataInjectionDisabled,
            RustWarning::ProgressiveJpegFallback => Self::ProgressiveJpegFallback,
            RustWarning::JpegReencodeFragile => Self::JpegReencodeFragile,
            RustWarning::LsbCapacitySkipped => Self::LsbCapacitySkipped,
            RustWarning::DctCapacityInsufficient => Self::DctCapacityInsufficient,
            RustWarning::ContradictoryLegalClaims => Self::ContradictoryLegalClaims,
            RustWarning::MissingRightsConstraints => Self::MissingRightsConstraints,
            _ => Self::MissingMacKey,
        }
    }
}

#[pyclass(name = "ExecutionReport")]
#[derive(Clone, Debug)]
pub struct PyExecutionReport {
    inner: stegoeggo::ExecutionReport,
}

#[pymethods]
impl PyExecutionReport {
    #[getter]
    fn effective_policy(&self) -> PyRightsPolicy {
        self.inner.effective_policy().into()
    }

    #[getter]
    fn effective_dmi(&self) -> Option<PyDmiValue> {
        self.inner.effective_dmi().map(PyDmiValue::from)
    }

    #[getter]
    fn metadata_injected(&self) -> bool {
        self.inner.metadata_injected()
    }

    #[getter]
    fn stego_attempted(&self) -> bool {
        self.inner.stego_attempted()
    }

    #[getter]
    fn stego_succeeded(&self) -> bool {
        self.inner.stego_succeeded()
    }

    #[getter]
    fn format_transcoded(&self) -> bool {
        self.inner.format_transcoded()
    }

    #[getter]
    fn warnings<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let list = PyList::empty(py);
        for w in self.inner.warnings() {
            let pyw: PyProtectionWarning = (*w).clone().into();
            list.append(pyw)?;
        }
        Ok(list)
    }

    #[getter]
    fn has_degradation(&self) -> bool {
        self.inner.has_degradation()
    }

    #[getter]
    fn any_succeeded(&self) -> bool {
        self.inner.any_succeeded()
    }

    #[getter]
    fn resource_usage(&self) -> Option<PyResourceUsage> {
        self.inner.resource_usage().cloned().map(PyResourceUsage::from)
    }

    fn __repr__(&self) -> String {
        format!(
            "ExecutionReport(policy={}, metadata_injected={}, stego_succeeded={}, format_transcoded={})",
            rust_policy_name(self.inner.effective_policy()),
            self.inner.metadata_injected(),
            self.inner.stego_succeeded(),
            self.inner.format_transcoded(),
        )
    }
}

#[pyclass(name = "ResourceUsage")]
#[derive(Clone, Debug)]
pub struct PyResourceUsage {
    inner: ResourceUsage,
}

impl From<ResourceUsage> for PyResourceUsage {
    fn from(r: ResourceUsage) -> Self {
        Self { inner: r }
    }
}

#[pymethods]
impl PyResourceUsage {
    #[getter]
    fn input_bytes(&self) -> usize {
        self.inner.input_bytes
    }

    #[getter]
    fn png_chunks_scanned(&self) -> usize {
        self.inner.png_chunks_scanned
    }

    #[getter]
    fn jpeg_segments_scanned(&self) -> usize {
        self.inner.jpeg_segments_scanned
    }

    #[getter]
    fn webp_riff_chunks_scanned(&self) -> usize {
        self.inner.webp_riff_chunks_scanned
    }

    #[getter]
    fn xmp_bytes_parsed(&self) -> usize {
        self.inner.xmp_bytes_parsed
    }

    #[getter]
    fn metadata_fields_extracted(&self) -> usize {
        self.inner.metadata_fields_extracted
    }

    #[getter]
    fn metadata_bytes_copied(&self) -> usize {
        self.inner.metadata_bytes_copied
    }

    #[getter]
    fn tile_origins_checked(&self) -> usize {
        self.inner.tile_origins_checked
    }

    #[getter]
    fn verification_seeds_tried(&self) -> usize {
        self.inner.verification_seeds_tried
    }

    #[getter]
    fn peak_allocations_bytes(&self) -> usize {
        self.inner.peak_allocations_bytes
    }

    fn __repr__(&self) -> String {
        format!(
            "ResourceUsage(input={}, png_chunks={}, jpeg_segments={}, webp_chunks={}, xmp={}, peak_alloc={})",
            self.inner.input_bytes,
            self.inner.png_chunks_scanned,
            self.inner.jpeg_segments_scanned,
            self.inner.webp_riff_chunks_scanned,
            self.inner.xmp_bytes_parsed,
            self.inner.peak_allocations_bytes,
        )
    }
}

#[pyclass(name = "VerificationStatus", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyVerificationStatus {
    Verified = 0,
    Invalid = 1,
    NotFound = 2,
}

#[pymethods]
impl PyVerificationStatus {
    fn __repr__(&self) -> String {
        format!("VerificationStatus.{}", self.name())
    }

    #[getter]
    fn name(&self) -> &'static str {
        #[allow(unreachable_patterns)]
        match self {
            Self::Verified => "VERIFIED",
            Self::Invalid => "INVALID",
            Self::NotFound => "NOT_FOUND",
            _ => "UNKNOWN",
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }
}

impl From<RustStatus> for PyVerificationStatus {
    fn from(s: RustStatus) -> Self {
        #[allow(unreachable_patterns)]
        match s {
            RustStatus::Verified => Self::Verified,
            RustStatus::Invalid => Self::Invalid,
            RustStatus::NotFound => Self::NotFound,
            _ => Self::NotFound,
        }
    }
}

#[pyclass(name = "EvidenceStrength", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyEvidenceStrength {
    NoNoticeFound = 0,
    MetadataNoticeOnly = 1,
    MetadataNoticeAndBestEffortStego = 2,
    MetadataNoticeAndAuthenticatedProvenance = 3,
}

#[pymethods]
impl PyEvidenceStrength {
    fn __repr__(&self) -> String {
        format!("EvidenceStrength.{}", self.name())
    }

    #[getter]
    fn name(&self) -> &'static str {
        #[allow(unreachable_patterns)]
        match self {
            Self::NoNoticeFound => "NO_NOTICE_FOUND",
            Self::MetadataNoticeOnly => "METADATA_NOTICE_ONLY",
            Self::MetadataNoticeAndBestEffortStego => "METADATA_NOTICE_AND_BEST_EFFORT_STEGO",
            Self::MetadataNoticeAndAuthenticatedProvenance => {
                "METADATA_NOTICE_AND_AUTHENTICATED_PROVENANCE"
            }
            _ => "UNKNOWN",
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }
}

impl From<EvidenceStrength> for PyEvidenceStrength {
    fn from(s: EvidenceStrength) -> Self {
        match s {
            EvidenceStrength::NoNoticeFound => Self::NoNoticeFound,
            EvidenceStrength::MetadataNoticeOnly => Self::MetadataNoticeOnly,
            EvidenceStrength::MetadataNoticeAndBestEffortStego => {
                Self::MetadataNoticeAndBestEffortStego
            }
            EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance => {
                Self::MetadataNoticeAndAuthenticatedProvenance
            }
            _ => Self::NoNoticeFound,
        }
    }
}

#[pyclass(name = "VerificationReport")]
#[derive(Clone, Debug)]
pub struct PyVerificationReport {
    inner: RustVerificationReport,
}

#[pymethods]
impl PyVerificationReport {
    #[getter]
    fn status(&self) -> PyVerificationStatus {
        self.inner.summary_status().into()
    }

    #[getter]
    fn evidence_strength(&self) -> PyEvidenceStrength {
        self.inner.evidence_strength().into()
    }

    #[getter]
    fn has_errors(&self) -> bool {
        self.inner.has_errors()
    }

    #[getter]
    fn hidden_marker_status(&self) -> PyVerificationStatus {
        self.inner.hidden_marker().status().into()
    }

    #[getter]
    fn hidden_marker_seed(&self) -> Option<u64> {
        self.inner.hidden_marker().seed()
    }

    #[getter]
    fn hidden_marker_payload_version(&self) -> Option<u8> {
        self.inner.hidden_marker().payload_version()
    }

    #[getter]
    fn hidden_marker_intensity(&self) -> Option<f32> {
        self.inner.hidden_marker().intensity()
    }

    #[getter]
    fn hidden_marker_tiled(&self) -> bool {
        self.inner.hidden_marker().tiled()
    }

    #[getter]
    fn rights_found(&self) -> bool {
        self.inner.rights().found()
    }

    #[getter]
    fn copyright_holder(&self) -> Option<String> {
        opt_string(self.inner.rights().copyright_holder())
    }

    #[getter]
    fn creator(&self) -> Option<String> {
        opt_string(self.inner.rights().creator())
    }

    #[getter]
    fn contact(&self) -> Option<String> {
        opt_string(self.inner.rights().contact())
    }

    #[getter]
    fn rights_url(&self) -> Option<String> {
        opt_string(self.inner.rights().rights_url())
    }

    #[getter]
    fn license_url(&self) -> Option<String> {
        opt_string(self.inner.rights().rights_url())
    }

    #[getter]
    fn usage_terms(&self) -> Option<String> {
        opt_string(self.inner.rights().usage_terms())
    }

    #[getter]
    fn ai_constraints(&self) -> Option<String> {
        opt_string(self.inner.rights().ai_constraints())
    }

    #[getter]
    fn rights_dmi(&self) -> Option<u8> {
        self.inner.rights().dmi()
    }

    #[getter]
    fn authentication_attempted(&self) -> bool {
        self.inner.authentication().attempted()
    }

    #[getter]
    fn authentication_key_matched(&self) -> bool {
        self.inner.authentication().key_matched()
    }

    #[getter]
    fn authentication_hmac_status(&self) -> Option<PyVerificationStatus> {
        self.inner
            .authentication()
            .hmac_status()
            .map(PyVerificationStatus::from)
    }

    #[getter]
    fn trust_trusted(&self) -> bool {
        self.inner.trust().trusted()
    }

    #[getter]
    fn trust_reason(&self) -> String {
        self.inner.trust().reason().to_string()
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let json = serde_json::to_string(&self.inner).map_err(|e| {
            PyErr::new::<StegoEggoError, _>(format!("serialization failed: {e}"))
        })?;
        let parsed: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
            PyErr::new::<StegoEggoError, _>(format!("json parse failed: {e}"))
        })?;
        json_to_pydict(py, &parsed)
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyErr::new::<StegoEggoError, _>(format!("serialization failed: {e}")))
    }

    fn __repr__(&self) -> String {
        format!(
            "VerificationReport(status={:?}, evidence_strength={:?}, rights_found={})",
            self.inner.summary_status(),
            self.inner.evidence_strength(),
            self.inner.rights().found(),
        )
    }
}

fn json_to_pydict<'py>(py: Python<'py>, value: &serde_json::Value) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let pv = json_to_pyobject(py, v)?;
                dict.set_item(k, pv)?;
            }
            Ok(dict)
        }
        _ => {
            let pv = json_to_pyobject(py, value)?;
            dict.set_item("value", pv)?;
            Ok(dict)
        }
    }
}

fn json_to_pyobject<'py>(py: Python<'py>, value: &serde_json::Value) -> PyResult<Bound<'py, PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None().into_bound(py)),
        serde_json::Value::Bool(b) => {
            let builtins = py.import("builtins")?;
            let cls = builtins.getattr(if *b { "True" } else { "False" })?;
            Ok(cls)
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any())
            } else if let Some(u) = n.as_u64() {
                Ok(u.into_pyobject(py)?.into_any())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.into_any())
            } else {
                Ok(py.None().into_bound(py))
            }
        }
        serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into_any()),
        serde_json::Value::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(json_to_pyobject(py, item)?)?;
            }
            Ok(list.into_any())
        }
        serde_json::Value::Object(_) => Ok(json_to_pydict(py, value)?.into_any()),
    }
}

#[pyfunction]
fn detect_format(data: &[u8]) -> Option<&'static str> {
    RustFormat::from_magic_bytes(data).map(|f| f.extension())
}

#[pyfunction]
fn protect<'py>(
    py: Python<'py>,
    data: &[u8],
    request: &PyProtectionRequest,
) -> PyResult<Bound<'py, PyBytes>> {
    let req = request.inner.as_ref().expect("request initialized").clone();
    let bytes = py
        .detach(|| process_request_bytes(data, &req))
        .map_err(map_error)?;
    Ok(PyBytes::new(py, &bytes))
}

#[pyfunction]
fn protect_with_warnings<'py>(
    py: Python<'py>,
    data: &[u8],
    request: &PyProtectionRequest,
) -> PyResult<Bound<'py, PyTuple>> {
    let req = request.inner.as_ref().expect("request initialized").clone();
    let (bytes, warnings) = py
        .detach(|| process_request_bytes_with_warnings(data, &req))
        .map_err(map_error)?;
    let py_bytes: Bound<'py, PyAny> = PyBytes::new(py, &bytes).into_any();
    let py_warnings = PyList::empty(py);
    for w in warnings {
        let pw: PyProtectionWarning = w.into();
        py_warnings.append(pw)?;
    }
    Ok(PyTuple::new(py, [py_bytes, py_warnings.into_any()])?)
}

#[pyfunction]
fn protect_with_report<'py>(
    py: Python<'py>,
    data: &[u8],
    request: &PyProtectionRequest,
) -> PyResult<Bound<'py, PyTuple>> {
    let req = request.inner.as_ref().expect("request initialized").clone();
    let (bytes, report) = py
        .detach(|| process_request_bytes_with_report(data, &req))
        .map_err(map_error)?;
    let py_bytes: Bound<'py, PyAny> = PyBytes::new(py, &bytes).into_any();
    let py_report = PyExecutionReport { inner: report };
    Ok(PyTuple::new(
        py,
        [py_bytes, Bound::new(py, py_report)?.into_any()],
    )?)
}

#[pyfunction]
#[pyo3(signature = (data, mac_key=None, resource_limits=None))]
fn verify<'py>(
    py: Python<'py>,
    data: &[u8],
    mac_key: Option<Vec<u8>>,
    resource_limits: Option<&PyResourceLimits>,
) -> PyResult<PyVerificationReport> {
    let key_slice: &[u8] = mac_key.as_deref().unwrap_or(&[]);
    let lim_owned = resource_limits.map(|l| l.inner.clone());
    let report = py.detach(|| {
        if let Some(lim) = lim_owned.as_ref() {
            verify_image_bytes_report_with_limits(data, key_slice, lim)
        } else {
            verify_image_bytes_report(data, key_slice)
        }
    });
    Ok(PyVerificationReport { inner: report })
}

fn path_to_bytes(path: &str) -> PyResult<Vec<u8>> {
    std::fs::read(path).map_err(|e| {
        PyErr::new::<StegoEggoError, _>(format!("failed to read {path}: {e}"))
    })
}

#[pyfunction]
fn protect_file(path: &str, request: &PyProtectionRequest) -> PyResult<()> {
    let data = path_to_bytes(path)?;
    let req = request.inner.as_ref().expect("request initialized").clone();
    let protected = process_request_bytes(&data, &req).map_err(map_error)?;
    std::fs::write(path, &protected).map_err(|e| {
        PyErr::new::<StegoEggoError, _>(format!("failed to write {path}: {e}"))
    })?;
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (path, mac_key=None, resource_limits=None))]
fn verify_file(
    py: Python<'_>,
    path: &str,
    mac_key: Option<Vec<u8>>,
    resource_limits: Option<&PyResourceLimits>,
) -> PyResult<PyVerificationReport> {
    let data = path_to_bytes(path)?;
    verify(py, data.as_slice(), mac_key, resource_limits)
}

#[pymodule(name = "_native")]
fn _native_module(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__stegoeggo_version__", env!("CARGO_PKG_VERSION"))?;
    m.add("RightsPolicy", py.get_type::<PyRightsPolicy>())?;
    m.add("DmiValue", py.get_type::<PyDmiValue>())?;
    m.add("ImageOutputFormat", py.get_type::<PyImageOutputFormat>())?;
    m.add("MetadataUpdatePolicy", py.get_type::<PyMetadataUpdatePolicy>())?;
    m.add("ProtectionPreset", py.get_type::<PyProtectionPreset>())?;
    m.add("AuthenticationMode", py.get_type::<PyAuthenticationMode>())?;
    m.add("HiddenMarkerMode", py.get_type::<PyHiddenMarkerMode>())?;
    m.add("RightsNotice", py.get_type::<PyRightsNotice>())?;
    m.add("ProcessingOptions", py.get_type::<PyProcessingOptions>())?;
    m.add("ResourceLimits", py.get_type::<PyResourceLimits>())?;
    m.add("ResourceLimitsBuilder", py.get_type::<PyResourceLimitsBuilder>())?;
    m.add("ProtectionRequest", py.get_type::<PyProtectionRequest>())?;
    m.add("ProtectionWarning", py.get_type::<PyProtectionWarning>())?;
    m.add("ExecutionReport", py.get_type::<PyExecutionReport>())?;
    m.add("ResourceUsage", py.get_type::<PyResourceUsage>())?;
    m.add("VerificationStatus", py.get_type::<PyVerificationStatus>())?;
    m.add("EvidenceStrength", py.get_type::<PyEvidenceStrength>())?;
    m.add("VerificationReport", py.get_type::<PyVerificationReport>())?;
    m.add("StegoEggoError", py.get_type::<StegoEggoError>())?;
    m.add("InvalidConfigError", py.get_type::<InvalidConfigError>())?;
    m.add("InvalidFormatError", py.get_type::<InvalidFormatError>())?;
    m.add("EncodeDecodeError", py.get_type::<EncodeDecodeError>())?;
    m.add("MetadataError", py.get_type::<MetadataError>())?;
    m.add("SteganographyError", py.get_type::<SteganographyError>())?;
    m.add("InsufficientCapacityError", py.get_type::<InsufficientCapacityError>())?;
    m.add("VerificationError", py.get_type::<VerificationError>())?;
    m.add("ResourceLimitError", py.get_type::<ResourceLimitError>())?;

    m.add_function(wrap_pyfunction!(protect, m)?)?;
    m.add_function(wrap_pyfunction!(protect_with_warnings, m)?)?;
    m.add_function(wrap_pyfunction!(protect_with_report, m)?)?;
    m.add_function(wrap_pyfunction!(verify, m)?)?;
    m.add_function(wrap_pyfunction!(protect_file, m)?)?;
    m.add_function(wrap_pyfunction!(verify_file, m)?)?;
    m.add_function(wrap_pyfunction!(detect_format, m)?)?;

    Ok(())
}
