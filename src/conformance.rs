//! Machine-readable conformance reporting for independent interoperability testing.
//!
//! Provides structured types for the conformance harness to report check results,
//! external parser extractions, and normalized comparisons between internal and
//! external metadata observations.

use serde::{Deserialize, Serialize};
use std::fmt;

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
