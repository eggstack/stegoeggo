use crate::enums::{
    DiagnosticLevel, DmiValue, EvidenceChannel, EvidenceStrength, FieldSource, ProtectionWarning,
    RightsPolicy, VerificationStatus,
};
use napi_derive::napi;
use stegoeggo::verification::{
    AuthenticationVerification as RustAuthentication, BindingVerification as RustBinding,
    Diagnostic as RustDiagnostic, HiddenMarkerVerification as RustHidden,
    RightsVerification as RustRights, TrustEvaluation as RustTrust,
    VerificationReport as RustVerificationReport,
};
use stegoeggo::{ExecutionReport as RustExecutionReport, ResourceUsage as RustResourceUsage};

/// Observed resource usage for a single operation.
#[napi(object, object_from_js = false)]
pub struct ResourceUsage {
    /// Total input bytes processed.
    pub input_bytes: f64,
    /// Number of PNG chunks scanned.
    pub png_chunks_scanned: f64,
    /// Number of JPEG segments scanned.
    pub jpeg_segments_scanned: f64,
    /// Number of WebP RIFF chunks scanned.
    pub webp_riff_chunks_scanned: f64,
    /// Total XMP bytes parsed.
    pub xmp_bytes_parsed: f64,
    /// Number of metadata fields extracted.
    pub metadata_fields_extracted: f64,
    /// Total metadata bytes copied.
    pub metadata_bytes_copied: f64,
    /// Number of tile origins checked during extraction.
    pub tile_origins_checked: f64,
    /// Number of verification seeds tried.
    pub verification_seeds_tried: f64,
    /// Largest single allocation observed.
    pub peak_allocations_bytes: f64,
}

impl From<&RustResourceUsage> for ResourceUsage {
    fn from(value: &RustResourceUsage) -> Self {
        Self {
            input_bytes: crate::numeric::from_usize(value.input_bytes),
            png_chunks_scanned: crate::numeric::from_usize(value.png_chunks_scanned),
            jpeg_segments_scanned: crate::numeric::from_usize(value.jpeg_segments_scanned),
            webp_riff_chunks_scanned: crate::numeric::from_usize(value.webp_riff_chunks_scanned),
            xmp_bytes_parsed: crate::numeric::from_usize(value.xmp_bytes_parsed),
            metadata_fields_extracted: crate::numeric::from_usize(value.metadata_fields_extracted),
            metadata_bytes_copied: crate::numeric::from_usize(value.metadata_bytes_copied),
            tile_origins_checked: crate::numeric::from_usize(value.tile_origins_checked),
            verification_seeds_tried: crate::numeric::from_usize(value.verification_seeds_tried),
            peak_allocations_bytes: crate::numeric::from_usize(value.peak_allocations_bytes),
        }
    }
}

/// Which channels ran, which degraded, and what the effective policy was.
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct ExecutionReport {
    /// Effective rights policy after resolution.
    pub effective_policy: RightsPolicy,
    /// DMI value serialized into metadata, or `null`.
    pub effective_dmi: Option<DmiValue>,
    /// Whether rights metadata was injected.
    pub metadata_injected: bool,
    /// Whether hidden-marker embedding was attempted.
    pub stego_attempted: bool,
    /// Whether hidden-marker embedding succeeded.
    pub stego_succeeded: bool,
    /// Whether the output format differs from the input format.
    pub format_transcoded: bool,
    /// Degradation and advisory conditions raised during protection.
    pub warnings: Vec<ProtectionWarning>,
    /// Whether any requested channel executed successfully.
    pub any_succeeded: bool,
    /// Whether any requested channel was degraded or skipped.
    pub has_degradation: bool,
    /// Observed resource usage, or `null` when untracked.
    pub resource_usage: Option<ResourceUsage>,
}

impl From<&RustExecutionReport> for ExecutionReport {
    fn from(value: &RustExecutionReport) -> Self {
        Self {
            effective_policy: value.effective_policy().into(),
            effective_dmi: value.effective_dmi().map(DmiValue::from),
            metadata_injected: value.metadata_injected(),
            stego_attempted: value.stego_attempted(),
            stego_succeeded: value.stego_succeeded(),
            format_transcoded: value.format_transcoded(),
            warnings: value.warnings().iter().cloned().map(Into::into).collect(),
            any_succeeded: value.any_succeeded(),
            has_degradation: value.has_degradation(),
            resource_usage: value.resource_usage().map(ResourceUsage::from),
        }
    }
}

/// Rights and legal-notice verification facts.
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct RightsVerification {
    /// Whether any rights notice was found.
    pub found: bool,
    /// Detected copyright holder, or `null`.
    pub copyright_holder: Option<String>,
    /// Detected creator, or `null`.
    pub creator: Option<String>,
    /// Detected contact, or `null`.
    pub contact: Option<String>,
    /// Detected rights URL, or `null`.
    pub rights_url: Option<String>,
    /// Detected license URL, or `null`.
    pub license_url: Option<String>,
    /// Detected usage terms, or `null`.
    pub usage_terms: Option<String>,
    /// Detected AI training constraints, or `null`.
    pub ai_constraints: Option<String>,
    /// Detected DMI byte value, or `null`.
    pub dmi: Option<u8>,
    /// Where the notice was read from.
    pub source: FieldSource,
    /// Metadata locations that carried evidence.
    pub channels: Vec<EvidenceChannel>,
}

impl From<&RustRights> for RightsVerification {
    fn from(value: &RustRights) -> Self {
        Self {
            found: value.found(),
            copyright_holder: value.copyright_holder().map(ToString::to_string),
            creator: value.creator().map(ToString::to_string),
            contact: value.contact().map(ToString::to_string),
            rights_url: value.rights_url().map(ToString::to_string),
            license_url: value.rights_url().map(ToString::to_string),
            usage_terms: value.usage_terms().map(ToString::to_string),
            ai_constraints: value.ai_constraints().map(ToString::to_string),
            dmi: value.dmi(),
            source: value.source().into(),
            channels: value.channels().iter().copied().map(Into::into).collect(),
        }
    }
}

/// Hidden steganographic marker verification facts.
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct HiddenMarkerVerification {
    /// Status of the hidden marker.
    pub status: VerificationStatus,
    /// Detected payload version, or `null`.
    pub payload_version: Option<u8>,
    /// Extracted seed as a full-width `bigint`, or `null`.
    pub seed: Option<u64>,
    /// Extracted embedding intensity, or `null`.
    pub intensity: Option<f64>,
    /// Whether tiled steganography was detected.
    pub tiled: bool,
    /// Where the marker was read from.
    pub source: FieldSource,
}

impl From<&RustHidden> for HiddenMarkerVerification {
    fn from(value: &RustHidden) -> Self {
        Self {
            status: value.status().into(),
            payload_version: value.payload_version(),
            seed: value.seed(),
            intensity: value.intensity().map(f64::from),
            tiled: value.tiled(),
            source: value.source().into(),
        }
    }
}

/// HMAC authentication verification facts.
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct AuthenticationVerification {
    /// Whether authentication was attempted.
    pub attempted: bool,
    /// HMAC status, or `null` when not applicable.
    pub hmac_status: Option<VerificationStatus>,
    /// Authentication algorithm name.
    pub algorithm: String,
    /// Whether the supplied key matched the expected key.
    pub key_matched: bool,
}

impl From<&RustAuthentication> for AuthenticationVerification {
    fn from(value: &RustAuthentication) -> Self {
        Self {
            attempted: value.attempted(),
            hmac_status: value.hmac_status().map(Into::into),
            algorithm: value.algorithm().to_string(),
            key_matched: value.key_matched(),
        }
    }
}

/// Instance-digest and content-hash binding verification facts.
#[napi(object, object_from_js = false)]
pub struct BindingVerification {
    /// Whether an instance digest was found.
    pub instance_digest_present: bool,
    /// Whether the instance digest is valid.
    pub instance_digest_valid: bool,
    /// Whether a content hash was found.
    pub content_hash_present: bool,
    /// Whether the content hash is valid.
    pub content_hash_valid: bool,
    /// Whether the claimed format matches the actual format.
    pub format_valid: bool,
    /// Whether the claimed dimensions match the actual dimensions.
    pub dimensions_valid: bool,
    /// Whether the claimed file size matches the actual file size.
    pub file_size_valid: bool,
}

impl From<&RustBinding> for BindingVerification {
    fn from(value: &RustBinding) -> Self {
        Self {
            instance_digest_present: value.instance_digest_present(),
            instance_digest_valid: value.instance_digest_valid(),
            content_hash_present: value.content_hash_present(),
            content_hash_valid: value.content_hash_valid(),
            format_valid: value.format_valid(),
            dimensions_valid: value.dimensions_valid(),
            file_size_valid: value.file_size_valid(),
        }
    }
}

/// Trust evaluation of the discovered evidence.
#[napi(object, object_from_js = false)]
pub struct TrustEvaluation {
    /// Trust model name.
    pub trust_model: String,
    /// Whether the evidence is trusted.
    pub trusted: bool,
    /// Human-readable trust reason.
    pub reason: String,
}

impl From<&RustTrust> for TrustEvaluation {
    fn from(value: &RustTrust) -> Self {
        Self {
            trust_model: value.trust_model().to_string(),
            trusted: value.trusted(),
            reason: value.reason().to_string(),
        }
    }
}

/// A verification diagnostic message.
#[napi(object, object_from_js = false)]
pub struct Diagnostic {
    /// Diagnostic severity.
    pub level: DiagnosticLevel,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Component that produced the diagnostic.
    pub source: String,
}

impl From<&RustDiagnostic> for Diagnostic {
    fn from(value: &RustDiagnostic) -> Self {
        Self {
            level: value.level().into(),
            message: value.message().to_string(),
            source: value.source().to_string(),
        }
    }
}

/// The canonical structured verification report.
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct VerificationReport {
    /// Summary status across all channels.
    pub status: VerificationStatus,
    /// Combined strength of the discovered evidence.
    pub evidence_strength: EvidenceStrength,
    /// Whether any error-level diagnostic or failure was recorded.
    pub has_errors: bool,
    /// Rights and legal-notice facts.
    pub rights: RightsVerification,
    /// Hidden-marker facts.
    pub hidden_marker: HiddenMarkerVerification,
    /// HMAC authentication facts.
    pub authentication: AuthenticationVerification,
    /// Instance-digest and content-hash binding facts.
    pub bindings: BindingVerification,
    /// Trust evaluation of the discovered evidence.
    pub trust: TrustEvaluation,
    /// Diagnostics produced during verification.
    pub diagnostics: Vec<Diagnostic>,
}

impl From<&RustVerificationReport> for VerificationReport {
    fn from(value: &RustVerificationReport) -> Self {
        Self {
            status: value.summary_status().into(),
            evidence_strength: value.evidence_strength().into(),
            has_errors: value.has_errors(),
            rights: value.rights().into(),
            hidden_marker: value.hidden_marker().into(),
            authentication: value.authentication().into(),
            bindings: value.bindings().into(),
            trust: value.trust().into(),
            diagnostics: value.diagnostics().iter().map(Into::into).collect(),
        }
    }
}
