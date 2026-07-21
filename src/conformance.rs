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

/// Error from an external tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalToolError {
    /// Name of the tool (e.g., "exiftool").
    pub tool: String,
    /// Path to the executable.
    pub executable: String,
    /// Process exit status code, if available.
    pub exit_status: Option<i32>,
    /// Summary of stderr output.
    pub stderr_summary: String,
    /// Whether stdout was empty.
    pub output_empty: bool,
    /// Whether JSON parsing failed.
    pub json_parse_failed: bool,
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
    /// Manifest fixture ID, if matched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixture_id: Option<String>,
    /// Fixture category from manifest, if matched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Fixture source classification, if matched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
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
            fixture_id: None,
            category: None,
            source: None,
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
    match s {
        "DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING" => {
            return "DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING".to_string()
        }
        "DMI-PROHIBITED-GENAIMLTRAINING" => return "DMI-PROHIBITED-GENAIMLTRAINING".to_string(),
        "DMI-PROHIBITED-AIMLTRAINING" => return "DMI-PROHIBITED-AIMLTRAINING".to_string(),
        "DMI-PROHIBITED-SEECONSTRAINT" => return "DMI-PROHIBITED-SEECONSTRAINT".to_string(),
        "DMI-PROHIBITED" => return "DMI-PROHIBITED".to_string(),
        "DMI-ALLOWED" => return "DMI-ALLOWED".to_string(),
        _ => {}
    }
    let lower = s.to_lowercase();
    if lower.contains("prohibited") && lower.contains("search") {
        "DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING".to_string()
    } else if lower.contains("prohibited") && lower.contains("gen") && lower.contains("ai") {
        "DMI-PROHIBITED-GENAIMLTRAINING".to_string()
    } else if lower.contains("prohibited") && (lower.contains("ai") || lower.contains("aiml")) {
        "DMI-PROHIBITED-AIMLTRAINING".to_string()
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
                        CheckSeverity::Warn,
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

/// Expected decode outcome for a fixture.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecodeExpectation {
    /// Image should decode successfully.
    #[default]
    Pass,
    /// Image should fail to decode.
    Fail,
    /// Either outcome is acceptable.
    Either,
}

/// Expected XMP validity for a fixture.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XmpExpectation {
    /// XMP should be present and valid XML.
    #[default]
    Valid,
    /// XMP should be present but invalid XML.
    Invalid,
    /// No XMP should be present.
    Absent,
    /// Any XMP state is acceptable.
    Either,
}

/// Expected extraction outcome for a fixture.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionExpectation {
    /// Extraction should succeed with metadata.
    #[default]
    Success,
    /// Extraction should find no notice.
    NoNotice,
    /// Fixture should be rejected.
    Reject,
}

/// Legal field values expected for a fixture.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExpectedLegalFields {
    /// Expected copyright holder.
    pub copyright_holder: Option<String>,
    /// Expected creator.
    pub creator: Option<String>,
    /// Expected copyright owner.
    pub copyright_owner: Option<String>,
    /// Expected usage terms.
    pub usage_terms: Option<String>,
    /// Expected web statement of rights.
    pub web_statement_of_rights: Option<String>,
    /// Expected AI constraints text.
    pub ai_constraints: Option<String>,
    /// Expected credit line.
    pub credit_line: Option<String>,
    /// Expected licensor name.
    pub licensor_name: Option<String>,
    /// Expected licensor email.
    pub licensor_email: Option<String>,
    /// Expected licensor URL.
    pub licensor_url: Option<String>,
}

/// A single fixture entry in the TOML manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureEntry {
    /// Unique identifier for this fixture.
    pub id: String,
    /// Relative path from the fixtures directory root.
    pub path: String,
    /// Detected image format (png, jpeg, webp).
    pub format: String,
    /// Fixture category (canonical, legacy, conflicting, malformed, preservation).
    pub category: String,
    /// Tool used to generate this fixture.
    pub authoring_tool: String,
    /// Version of the authoring tool.
    pub authoring_tool_version: String,
    /// Command used to generate this fixture.
    pub generation_command: String,
    /// Source of the fixture (generated, external).
    pub source: String,
    /// License identifier.
    pub license: String,
    /// SHA-256 hex digest of the fixture file.
    pub sha256: String,
    /// Expected DMI value string.
    pub expected_dmi: String,
    /// Whether this fixture is expected to have conflicting metadata.
    pub expected_conflict: bool,
    /// Expected legal field values.
    #[serde(default)]
    pub expected_legal_fields: ExpectedLegalFields,
    /// Whether this fixture is expected to be malformed (legacy, use expected_decode instead).
    pub expected_malformed: bool,
    /// Expected decode outcome.
    #[serde(default)]
    pub expected_decode: DecodeExpectation,
    /// Expected XMP validity.
    #[serde(default)]
    pub expected_xmp: XmpExpectation,
    /// Expected internal extraction outcome.
    #[serde(default)]
    pub expected_internal: ExtractionExpectation,
    /// Expected external extraction outcome.
    #[serde(default)]
    pub expected_external: ExtractionExpectation,
    /// Fields required to be present in external extraction.
    #[serde(default)]
    pub required_external_fields: Vec<String>,
    /// Expected preserved field names after re-processing.
    #[serde(default)]
    pub expected_preservation: Vec<String>,
}

/// The full fixture manifest loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureManifest {
    /// All fixture entries.
    #[serde(default = "Vec::new", rename = "fixture")]
    pub entries: Vec<FixtureEntry>,
}

impl FixtureManifest {
    /// Compute SHA-256 hex digest for a file on disk.
    pub fn compute_sha256(path: &Path) -> std::io::Result<String> {
        use sha2::{Digest, Sha256};
        let bytes = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        Ok(hex::encode(hasher.finalize()))
    }

    /// Find a fixture entry by its relative path within the fixtures directory.
    #[must_use]
    pub fn find_by_path(&self, path: &str) -> Option<&FixtureEntry> {
        self.entries.iter().find(|e| e.path == path)
    }

    /// Build a path-to-entry index for O(1) lookups.
    #[must_use]
    pub fn path_index(&self) -> std::collections::HashMap<String, &FixtureEntry> {
        self.entries.iter().map(|e| (e.path.clone(), e)).collect()
    }

    /// Return all entries belonging to a given category.
    #[must_use]
    pub fn entries_by_category(&self, category: &str) -> Vec<&FixtureEntry> {
        self.entries
            .iter()
            .filter(|e| e.category == category)
            .collect()
    }

    /// Return all entries matching a given format.
    #[must_use]
    pub fn entries_by_format(&self, format: &str) -> Vec<&FixtureEntry> {
        self.entries.iter().filter(|e| e.format == format).collect()
    }

    /// Count entries grouped by authoring tool.
    #[must_use]
    pub fn count_by_authoring_tool(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for entry in &self.entries {
            *counts.entry(entry.authoring_tool.clone()).or_insert(0) += 1;
        }
        counts
    }
}

/// Load a fixture manifest from a TOML file.
pub fn load_manifest(path: &Path) -> Result<FixtureManifest, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read manifest {}: {}", path.display(), e))?;
    let manifest: FixtureManifest =
        toml::from_str(&content).map_err(|e| format!("Failed to parse manifest: {}", e))?;
    Ok(manifest)
}

/// Validate manifest structure before processing fixtures.
///
/// Checks for duplicate IDs, duplicate paths, empty IDs, path traversal,
/// unsupported formats/categories, missing SHA-256, and other structural issues.
pub fn validate_manifest(manifest: &FixtureManifest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    let mut seen_paths = std::collections::HashSet::new();

    let valid_formats = ["png", "jpeg", "webp"];
    let valid_categories = [
        "canonical",
        "legacy",
        "conflicting",
        "malformed",
        "preservation",
    ];
    let valid_sources = [
        "generated",
        "external",
        "historical",
        "generated-negative",
        "current-generated",
    ];

    for entry in &manifest.entries {
        if entry.id.is_empty() {
            errors.push(format!("Fixture at '{}' has empty ID", entry.path));
        }
        if !seen_ids.insert(&entry.id) {
            errors.push(format!("Duplicate fixture ID: '{}'", entry.id));
        }
        if !seen_paths.insert(&entry.path) {
            errors.push(format!("Duplicate fixture path: '{}'", entry.path));
        }
        if entry.path.starts_with('/') || entry.path.starts_with('\\') {
            errors.push(format!("Fixture '{}' has absolute path", entry.id));
        }
        if entry.path.contains("..") {
            errors.push(format!(
                "Fixture '{}' contains path traversal (..)",
                entry.id
            ));
        }
        if !valid_formats.contains(&entry.format.as_str()) {
            errors.push(format!(
                "Fixture '{}' has unsupported format: '{}'",
                entry.id, entry.format
            ));
        }
        if !valid_categories.contains(&entry.category.as_str()) {
            errors.push(format!(
                "Fixture '{}' has unsupported category: '{}'",
                entry.id, entry.category
            ));
        }
        if !valid_sources.contains(&entry.source.as_str()) {
            errors.push(format!(
                "Fixture '{}' has unsupported source: '{}'",
                entry.id, entry.source
            ));
        }
        if entry.sha256.is_empty() {
            errors.push(format!("Fixture '{}' has empty SHA-256", entry.id));
        } else if entry.sha256.len() != 64 || !entry.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            errors.push(format!(
                "Fixture '{}' has invalid SHA-256: expected 64 hex characters",
                entry.id
            ));
        }
        if entry.source == "external" && entry.authoring_tool_version.is_empty() {
            // Allow empty version for external tools but log it
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Result of SHA-256 digest verification for a single fixture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestCheckResult {
    /// Relative path of the fixture file.
    pub fixture_path: String,
    /// Expected SHA-256 hex digest from the manifest.
    pub expected_sha256: String,
    /// Actual SHA-256 hex digest computed from the file.
    pub actual_sha256: String,
    /// Whether the digests match.
    pub matches: bool,
}

/// Verify SHA-256 digests of all fixtures referenced by the manifest.
///
/// Returns a `DigestCheckResult` for each entry. Callers should check
/// `matches` to determine if verification passed.
pub fn verify_fixtures(manifest: &FixtureManifest, fixtures_dir: &Path) -> Vec<DigestCheckResult> {
    manifest
        .entries
        .iter()
        .map(|entry| {
            let full_path = fixtures_dir.join(&entry.path);
            let actual = FixtureManifest::compute_sha256(&full_path).unwrap_or_default();
            DigestCheckResult {
                fixture_path: entry.path.clone(),
                expected_sha256: entry.sha256.clone(),
                actual_sha256: actual.clone(),
                matches: actual == entry.sha256,
            }
        })
        .collect()
}

/// Minimum counts required per category/format for coverage enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageMinimums {
    /// Minimum canonical PNG fixtures required.
    pub canonical_png: usize,
    /// Minimum canonical JPEG fixtures required.
    pub canonical_jpeg: usize,
    /// Minimum canonical WebP fixtures required.
    pub canonical_webp: usize,
    /// Minimum total legacy fixtures required.
    pub legacy_min: usize,
    /// Minimum distinct formats required in legacy category.
    pub legacy_formats: usize,
    /// Minimum conflicting fixtures required.
    pub conflict_min: usize,
    /// Minimum malformed fixtures required.
    pub malformed_min: usize,
    /// Minimum malformed fixtures per format (png, jpeg, webp).
    pub malformed_per_format: usize,
    /// Minimum preservation fixtures required.
    pub preservation_min: usize,
    /// Minimum distinct formats required in preservation category.
    pub preservation_formats: usize,
}

impl Default for CoverageMinimums {
    fn default() -> Self {
        Self {
            canonical_png: 1,
            canonical_jpeg: 1,
            canonical_webp: 1,
            legacy_min: 3,
            legacy_formats: 2,
            conflict_min: 3,
            malformed_min: 4,
            malformed_per_format: 1,
            preservation_min: 3,
            preservation_formats: 3,
        }
    }
}

/// Result of coverage enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageCheckResult {
    /// Whether all coverage minimums are met.
    pub passed: bool,
    /// List of coverage violation descriptions.
    pub violations: Vec<String>,
}

/// Enforce coverage minimums against a manifest.
#[must_use]
pub fn check_coverage(
    manifest: &FixtureManifest,
    minimums: &CoverageMinimums,
) -> CoverageCheckResult {
    let mut violations = Vec::new();

    let canonical = manifest.entries_by_category("canonical");
    let canonical_png = canonical.iter().filter(|e| e.format == "png").count();
    let canonical_jpeg = canonical.iter().filter(|e| e.format == "jpeg").count();
    let canonical_webp = canonical.iter().filter(|e| e.format == "webp").count();

    if canonical_png < minimums.canonical_png {
        violations.push(format!(
            "canonical PNG: {} < {}",
            canonical_png, minimums.canonical_png
        ));
    }
    if canonical_jpeg < minimums.canonical_jpeg {
        violations.push(format!(
            "canonical JPEG: {} < {}",
            canonical_jpeg, minimums.canonical_jpeg
        ));
    }
    if canonical_webp < minimums.canonical_webp {
        violations.push(format!(
            "canonical WebP: {} < {}",
            canonical_webp, minimums.canonical_webp
        ));
    }

    let legacy = manifest.entries_by_category("legacy");
    let legacy_format_count = legacy
        .iter()
        .map(|e| e.format.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    if legacy.len() < minimums.legacy_min {
        violations.push(format!(
            "legacy: {} < {}",
            legacy.len(),
            minimums.legacy_min
        ));
    }
    if legacy_format_count < minimums.legacy_formats {
        violations.push(format!(
            "legacy formats: {} < {}",
            legacy_format_count, minimums.legacy_formats
        ));
    }

    let conflict = manifest.entries_by_category("conflicting");
    if conflict.len() < minimums.conflict_min {
        violations.push(format!(
            "conflict: {} < {}",
            conflict.len(),
            minimums.conflict_min
        ));
    }

    let malformed = manifest.entries_by_category("malformed");
    if malformed.len() < minimums.malformed_min {
        violations.push(format!(
            "malformed: {} < {}",
            malformed.len(),
            minimums.malformed_min
        ));
    }
    let malformed_png = malformed.iter().filter(|e| e.format == "png").count();
    let malformed_jpeg = malformed.iter().filter(|e| e.format == "jpeg").count();
    let malformed_webp = malformed.iter().filter(|e| e.format == "webp").count();
    if minimums.malformed_per_format > 0 {
        if malformed_png < minimums.malformed_per_format {
            violations.push(format!(
                "malformed PNG: {} < {}",
                malformed_png, minimums.malformed_per_format
            ));
        }
        if malformed_jpeg < minimums.malformed_per_format {
            violations.push(format!(
                "malformed JPEG: {} < {}",
                malformed_jpeg, minimums.malformed_per_format
            ));
        }
        if malformed_webp < minimums.malformed_per_format {
            violations.push(format!(
                "malformed WebP: {} < {}",
                malformed_webp, minimums.malformed_per_format
            ));
        }
    }

    let preservation = manifest.entries_by_category("preservation");
    let preservation_format_count = preservation
        .iter()
        .map(|e| e.format.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    if preservation.len() < minimums.preservation_min {
        violations.push(format!(
            "preservation: {} < {}",
            preservation.len(),
            minimums.preservation_min
        ));
    }
    if preservation_format_count < minimums.preservation_formats {
        violations.push(format!(
            "preservation formats: {} < {}",
            preservation_format_count, minimums.preservation_formats
        ));
    }

    CoverageCheckResult {
        passed: violations.is_empty(),
        violations,
    }
}

/// Aggregate summary across multiple conformance reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceSummary {
    /// Total number of reports.
    pub total: usize,
    /// Number of passing reports.
    pub passed: usize,
    /// Number of failing reports.
    pub failed: usize,
    /// Report counts grouped by image format.
    pub by_format: std::collections::HashMap<String, usize>,
    /// Report counts grouped by fixture category.
    pub by_category: std::collections::HashMap<String, usize>,
    /// Digest verification results, if performed.
    pub digest_verification: Option<Vec<DigestCheckResult>>,
    /// Coverage check results, if performed.
    pub coverage: Option<CoverageCheckResult>,
    /// Coverage minimums used for this run, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_minimums: Option<CoverageMinimums>,
}

impl ConformanceSummary {
    /// Build a summary from a slice of conformance reports.
    #[must_use]
    pub fn from_reports(reports: &[ConformanceReport]) -> Self {
        let total = reports.len();
        let passed = reports.iter().filter(|r| r.passed).count();
        let failed = total - passed;

        let mut by_format = std::collections::HashMap::new();
        for report in reports {
            *by_format.entry(report.format.clone()).or_insert(0) += 1;
        }

        Self {
            total,
            passed,
            failed,
            by_format,
            by_category: std::collections::HashMap::new(),
            digest_verification: None,
            coverage: None,
            coverage_minimums: None,
        }
    }

    /// Attach digest verification results.
    pub fn with_digest_verification(&mut self, results: Vec<DigestCheckResult>) {
        self.digest_verification = Some(results);
    }

    /// Attach coverage check results.
    pub fn with_coverage(&mut self, result: CoverageCheckResult) {
        self.coverage = Some(result);
    }

    /// Attach coverage minimums for the report envelope.
    pub fn with_coverage_minimums(&mut self, minimums: CoverageMinimums) {
        self.coverage_minimums = Some(minimums);
    }

    /// Human-readable summary string.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "Conformance Summary: {} total, {} passed, {} failed",
            self.total, self.passed, self.failed
        ));

        if !self.by_format.is_empty() {
            lines.push("By format:".to_string());
            for (fmt, count) in &self.by_format {
                lines.push(format!("  {}: {}", fmt, count));
            }
        }

        if let Some(ref digest) = self.digest_verification {
            let matching = digest.iter().filter(|d| d.matches).count();
            lines.push(format!(
                "Digest verification: {}/{} passed",
                matching,
                digest.len()
            ));
        }

        if let Some(ref coverage) = self.coverage {
            if coverage.passed {
                lines.push("Coverage: PASS".to_string());
            } else {
                lines.push("Coverage: FAIL".to_string());
                for v in &coverage.violations {
                    lines.push(format!("  - {}", v));
                }
            }
        }

        lines.join("\n")
    }
}

/// Detect whether a JPEG file uses progressive encoding.
#[must_use]
pub fn is_progressive_jpeg(bytes: &[u8]) -> bool {
    if bytes.len() < 4 || !bytes.starts_with(b"\xFF\xD8\xFF") {
        return false;
    }
    let mut pos = 2;
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            break;
        }
        let marker = bytes[pos + 1];
        if marker == 0xD8 || marker == 0xD9 {
            pos += 2;
            continue;
        }
        if marker == 0xC0 || marker == 0xC2 {
            return marker == 0xC2;
        }
        let length = u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize;
        pos += 2 + length;
    }
    false
}
