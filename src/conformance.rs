//! Machine-readable conformance reporting for independent interoperability testing.
//!
//! Provides structured types for the conformance harness to report check results,
//! external parser extractions, and normalized comparisons between internal and
//! external metadata observations.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// Severity of a conformance check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckSeverity {
    /// Check passed.
    Pass,
    /// Check passed with a warning (e.g., field found externally but not internally).
    Warn,
    /// Check failed (e.g., field mismatch or missing required field).
    Fail,
}

impl fmt::Display for CheckSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckSeverity::Pass => write!(f, "PASS"),
            CheckSeverity::Warn => write!(f, "WARN"),
            CheckSeverity::Fail => write!(f, "FAIL"),
        }
    }
}

/// A single conformance check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Check identifier (e.g., "copyright", "creators", "canonical_dmi").
    pub name: String,
    /// Severity of this check.
    pub severity: CheckSeverity,
    /// Human-readable description of the result.
    pub message: String,
    /// Optional technical details (e.g., conflicting values).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Format-specific metadata extracted by an external parser (e.g., ExifTool).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalExtraction {
    /// Name of the external tool (e.g., "exiftool").
    pub tool: String,
    /// Tool version string, if available.
    pub version: Option<String>,
    /// Copyright notice.
    pub copyright: Option<String>,
    /// List of creators.
    pub creators: Vec<String>,
    /// Usage terms.
    pub usage_terms: Option<String>,
    /// Rights URL (web statement of rights).
    pub rights_url: Option<String>,
    /// Credit line.
    pub credit_line: Option<String>,
    /// Copyright owner.
    pub copyright_owner: Option<String>,
    /// Licensor name.
    pub licensor_name: Option<String>,
    /// Licensor email.
    pub licensor_email: Option<String>,
    /// Licensor URL.
    pub licensor_url: Option<String>,
    /// Content creation date.
    pub content_creation_date: Option<String>,
    /// AI constraints text.
    pub ai_constraints: Option<String>,
    /// Canonical PLUS DataMining value (e.g., "DMI-PROHIBITED-AIMLTRAINING").
    pub canonical_data_mining: Option<String>,
    /// Legacy IPTC DMI values.
    pub legacy_data_mining: Vec<String>,
    /// TDM reservation status.
    pub tdm_reserved: Option<bool>,
    /// Additional fields captured by the external parser.
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, String>,
}

/// Normalized metadata from internal extraction via `verify_legal_notice`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InternalExtraction {
    /// Copyright holder.
    pub copyright_holder: Option<String>,
    /// List of creators.
    pub creators: Vec<String>,
    /// Copyright owner.
    pub copyright_owner: Option<String>,
    /// Usage terms.
    pub usage_terms: Option<String>,
    /// Web statement of rights (rights URL).
    pub web_statement_of_rights: Option<String>,
    /// Credit line.
    pub credit_line: Option<String>,
    /// Licensor name.
    pub licensor_name: Option<String>,
    /// Licensor email.
    pub licensor_email: Option<String>,
    /// Licensor URL.
    pub licensor_url: Option<String>,
    /// Content creation date.
    pub content_creation_date: Option<String>,
    /// AI constraints text.
    pub ai_constraints: Option<String>,
    /// Canonical PLUS DataMining value.
    pub canonical_data_mining: Option<String>,
    /// Legacy IPTC DMI values.
    pub legacy_data_mining: Vec<String>,
    /// TDM reservation status.
    pub tdm_reserved: Option<bool>,
    /// Protection seed, if extracted.
    pub seed: Option<u64>,
    /// Evidence channels used for extraction.
    pub evidence_channels: Vec<String>,
    /// Overall evidence strength rating.
    pub evidence_strength: Option<String>,
}

/// Complete conformance report for one image fixture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceReport {
    /// Fixture filename.
    pub fixture: String,
    /// Detected image format (png, jpeg, webp).
    pub format: String,
    /// Tool that generated this report.
    pub generated_by: String,
    /// Whether the image decodes successfully.
    pub decode_valid: bool,
    /// Whether XMP is well-formed XML (if XMP was found).
    pub xmp_valid: Option<bool>,
    /// Normalized metadata from internal extraction.
    pub internal: InternalExtraction,
    /// Metadata extracted by external parsers.
    pub external: Vec<ExternalExtraction>,
    /// Individual check results.
    pub checks: Vec<CheckResult>,
    /// Detected conflicts between internal and external values.
    pub conflicts: Vec<String>,
    /// Overall pass/fail status (false if any check has Fail severity).
    pub passed: bool,
}

impl ConformanceReport {
    /// Create a new empty report for a fixture.
    #[must_use]
    pub fn new(fixture: &str, format: &str) -> Self {
        Self {
            fixture: fixture.to_string(),
            format: format.to_string(),
            generated_by: "stegoeggo-conformance".to_string(),
            decode_valid: false,
            xmp_valid: None,
            internal: InternalExtraction::default(),
            external: Vec::new(),
            checks: Vec::new(),
            conflicts: Vec::new(),
            passed: false,
        }
    }

    /// Add a check result.
    pub fn add_check(&mut self, name: &str, severity: CheckSeverity, message: &str) {
        self.checks.push(CheckResult {
            name: name.to_string(),
            severity,
            message: message.to_string(),
            details: None,
        });
    }

    /// Add a check with additional technical details.
    pub fn add_check_with_details(
        &mut self,
        name: &str,
        severity: CheckSeverity,
        message: &str,
        details: &str,
    ) {
        self.checks.push(CheckResult {
            name: name.to_string(),
            severity,
            message: message.to_string(),
            details: Some(details.to_string()),
        });
    }

    /// Record a conflict between internal and external observations.
    pub fn add_conflict(&mut self, conflict: &str) {
        self.conflicts.push(conflict.to_string());
    }

    /// Evaluate pass/fail based on checks. Sets `self.passed` to true only
    /// if no checks have `Fail` severity.
    pub fn evaluate(&mut self) {
        self.passed = !self
            .checks
            .iter()
            .any(|c| c.severity == CheckSeverity::Fail);
    }

    /// Human-readable summary of the report.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        let status = if self.passed { "PASS" } else { "FAIL" };
        lines.push(format!(
            "Fixture: {} ({}) — {}",
            self.fixture, self.format, status
        ));
        for check in &self.checks {
            lines.push(format!(
                "  [{}] {}: {}",
                check.severity, check.name, check.message
            ));
        }
        if !self.conflicts.is_empty() {
            lines.push("Conflicts:".to_string());
            for c in &self.conflicts {
                lines.push(format!("  - {}", c));
            }
        }
        lines.join("\n")
    }
}

/// Detect image format from magic bytes.
///
/// Returns `"png"`, `"jpeg"`, or `"webp"`, or `None` if unrecognized.
#[must_use]
pub fn detect_format(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 4 {
        return None;
    }
    if bytes.starts_with(b"\x89PNG") {
        Some("png".to_string())
    } else if bytes.starts_with(b"\xFF\xD8\xFF") {
        Some("jpeg".to_string())
    } else if bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
        Some("webp".to_string())
    } else {
        None
    }
}

/// Normalize a DMI value string for comparison.
///
/// Internal extraction returns `DmiValue::as_str()` values (e.g., "ProhibitedAiMlTraining").
/// ExifTool returns PLUS vocab keys (e.g., "DMI-PROHIBITED-AIMLTRAINING") or display values
/// (e.g., "Prohibited for AI/ML training"). This normalizes all forms for comparison.
#[must_use]
pub fn normalize_dmi_value(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower.contains("prohibited") && (lower.contains("ai") || lower.contains("aiml")) {
        "DMI-PROHIBITED-AIMLTRAINING".to_string()
    } else if lower.contains("prohibited") && lower.contains("gen") {
        "DMI-PROHIBITED-GENAIMLTRAINING".to_string()
    } else if lower.contains("prohibited") && lower.contains("search") {
        "DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING".to_string()
    } else if lower.contains("prohibited") && lower.contains("see") {
        "DMI-PROHIBITED-SEECONSTRAINT".to_string()
    } else if lower.contains("prohibited") {
        "DMI-PROHIBITED".to_string()
    } else if lower.contains("allowed") || lower.contains("permitted") {
        "DMI-ALLOWED".to_string()
    } else {
        s.to_string()
    }
}

/// Compare internal and external metadata extractions, adding check results
/// to the report.
pub fn compare_extractions(
    internal: &InternalExtraction,
    external: &ExternalExtraction,
    report: &mut ConformanceReport,
) {
    let check = |name: &str,
                 internal_val: &Option<String>,
                 external_val: &Option<String>,
                 report: &mut ConformanceReport| {
        match (internal_val, external_val) {
            (Some(i), Some(e)) => {
                if i == e {
                    report.add_check(name, CheckSeverity::Pass, "Internal and external agree");
                } else if e.contains(i) || i.contains(e) {
                    report.add_check(
                        name,
                        CheckSeverity::Pass,
                        "Values overlap (format-specific wrapping)",
                    );
                } else {
                    report.add_check_with_details(
                        name,
                        CheckSeverity::Fail,
                        "Internal and external disagree",
                        &format!("internal={:?}, external={:?}", i, e),
                    );
                }
            }
            (Some(i), None) => {
                report.add_check_with_details(
                    name,
                    CheckSeverity::Warn,
                    "Found internally but not via external parser",
                    &format!("internal={:?}", i),
                );
            }
            (None, Some(e)) => {
                report.add_check_with_details(
                    name,
                    CheckSeverity::Warn,
                    "Found via external parser but not internally",
                    &format!("external={:?}", e),
                );
            }
            (None, None) => {
                report.add_check(name, CheckSeverity::Pass, "Both absent");
            }
        }
    };

    check(
        "copyright",
        &internal.copyright_holder,
        &external.copyright,
        report,
    );
    check(
        "usage_terms",
        &internal.usage_terms,
        &external.usage_terms,
        report,
    );
    check(
        "rights_url",
        &internal.web_statement_of_rights,
        &external.rights_url,
        report,
    );
    check(
        "credit_line",
        &internal.credit_line,
        &external.credit_line,
        report,
    );
    check(
        "ai_constraints",
        &internal.ai_constraints,
        &external.ai_constraints,
        report,
    );

    match (
        &internal.canonical_data_mining,
        &external.canonical_data_mining,
    ) {
        (Some(i), Some(e)) => {
            let ni = normalize_dmi_value(i);
            let ne = normalize_dmi_value(e);
            if ni == ne {
                report.add_check(
                    "canonical_dmi",
                    CheckSeverity::Pass,
                    "DMI values agree (normalized)",
                );
            } else {
                report.add_check_with_details(
                    "canonical_dmi",
                    CheckSeverity::Fail,
                    "DMI values disagree",
                    &format!(
                        "internal={:?} (normalized={:?}), external={:?} (normalized={:?})",
                        i, ni, e, ne
                    ),
                );
            }
        }
        (Some(i), None) => {
            report.add_check_with_details(
                "canonical_dmi",
                CheckSeverity::Warn,
                "DMI found internally but not externally",
                &format!("internal={:?}", i),
            );
        }
        (None, Some(e)) => {
            report.add_check_with_details(
                "canonical_dmi",
                CheckSeverity::Warn,
                "DMI found externally but not internally",
                &format!("external={:?}", e),
            );
        }
        (None, None) => {
            report.add_check("canonical_dmi", CheckSeverity::Pass, "Both absent");
        }
    }

    if internal.creators != external.creators {
        report.add_check_with_details(
            "creators",
            CheckSeverity::Warn,
            "Creator lists differ",
            &format!(
                "internal={:?}, external={:?}",
                internal.creators, external.creators
            ),
        );
    } else {
        report.add_check("creators", CheckSeverity::Pass, "Creator lists match");
    }
}

/// Recursively collect image fixture files from a directory.
///
/// Returns files matching supported image extensions (png, jpg, jpeg, webp),
/// optionally filtered by format name.
#[must_use]
pub fn collect_fixture_files(dir: &Path, format_filter: &Option<String>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_fixture_files(&path, format_filter));
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let fmt = match ext {
                    "png" => Some("png"),
                    "jpg" | "jpeg" => Some("jpeg"),
                    "webp" => Some("webp"),
                    _ => None,
                };
                if let Some(f) = fmt {
                    if format_filter.as_ref().is_none_or(|filter| filter == f) {
                        files.push(path);
                    }
                }
            }
        }
    }
    files
}
