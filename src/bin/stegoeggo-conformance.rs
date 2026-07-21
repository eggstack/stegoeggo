use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use stegoeggo::conformance::{
    self, CheckSeverity, ConformanceReport, ConformanceRunReport, ConformanceSummary,
    CoverageCheckResult, DecodeExpectation, DigestCheckResult, ExternalExtraction,
    ExtractionExpectation, InternalExtraction, ManifestReport, ToolReport, XmpExpectation,
};

#[allow(dead_code)]
const EXIT_PASS: i32 = 0;
const EXIT_FAIL: i32 = 1;
const EXIT_CONFIG: i32 = 2;
const EXIT_DIGEST: i32 = 3;
const EXIT_COVERAGE: i32 = 4;
#[allow(dead_code)]
const EXIT_INTERNAL: i32 = 5;

const MAX_XMP_BYTES: usize = 65536;
const MAX_PROPERTIES: usize = 100;
const MAX_TEXT_LEN: usize = 8192;

fn find_exiftool() -> Option<PathBuf> {
    Command::new("which")
        .arg("exiftool")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !path.is_empty() {
                    Some(PathBuf::from(path))
                } else {
                    None
                }
            } else {
                None
            }
        })
}

fn exiftool_version(exiftool: &Path) -> Option<String> {
    Command::new(exiftool)
        .arg("-ver")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}

fn xmllint_version() -> Option<String> {
    Command::new("xmllint")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            let err = String::from_utf8_lossy(&o.stderr);
            let line = err.lines().next()?;
            let ver = line.split_whitespace().last()?;
            Some(ver.to_string())
        })
}

fn imagemagick_version() -> Option<String> {
    Command::new("identify")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let line = stdout.lines().next()?;
                let ver = line.split_whitespace().nth(2)?;
                Some(ver.trim_end_matches('.').to_string())
            } else {
                None
            }
        })
}

fn libvips_version() -> Option<String> {
    Command::new("vips")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            let text = if !o.stdout.is_empty() {
                String::from_utf8_lossy(&o.stdout)
            } else {
                String::from_utf8_lossy(&o.stderr)
            };
            let line = text.lines().next()?;
            let ver = line.split_whitespace().nth(1)?;
            Some(ver.to_string())
        })
}

fn find_imagemagick() -> Option<PathBuf> {
    if let Ok(output) = Command::new("which").arg("identify").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    if let Ok(output) = Command::new("which").arg("magick").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

fn imagemagick_identify(file: &Path) -> Result<String, String> {
    let identify_cmd = if find_imagemagick().is_some() {
        if Command::new("magick")
            .arg("--version")
            .output()
            .ok()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            "magick".to_string()
        } else {
            "identify".to_string()
        }
    } else {
        "identify".to_string()
    };

    let args: Vec<String> = if identify_cmd == "magick" {
        vec![
            "identify".to_string(),
            "-format".to_string(),
            "%m %wx%h".to_string(),
            file.display().to_string(),
        ]
    } else {
        vec![
            "-format".to_string(),
            "%m %wx%h".to_string(),
            file.display().to_string(),
        ]
    };

    let output = Command::new(&identify_cmd)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", identify_cmd, e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "{} failed: {}",
            identify_cmd,
            stderr.lines().next().unwrap_or("unknown error")
        ))
    }
}

fn find_vipsheader() -> Option<PathBuf> {
    if let Ok(output) = Command::new("which").arg("vipsheader").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    if let Ok(output) = Command::new("vipsheader").arg("--help").output() {
        if output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty() {
            return Some(PathBuf::from("vipsheader"));
        }
    }
    None
}

fn vipsheader_validate(file: &Path) -> Result<String, String> {
    let vipsheader = find_vipsheader().ok_or_else(|| "vipsheader not found".to_string())?;
    let output = Command::new(&vipsheader)
        .arg(file)
        .output()
        .map_err(|e| format!("Failed to run vipsheader: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "vipsheader failed: {}",
            stderr.lines().next().unwrap_or("unknown error")
        ))
    }
}

fn xmllint_validate(xmp_content: &str) -> Option<bool> {
    use std::io::Write;
    let mut child = Command::new("xmllint")
        .arg("--noout")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(xmp_content.as_bytes());
        drop(stdin);
    }
    let result = child.wait_with_output().ok()?;
    Some(result.status.success())
}

fn exiftool_json_extract(
    file: &Path,
    exiftool: &Path,
) -> Result<serde_json::Value, conformance::ExternalToolError> {
    let output = Command::new(exiftool)
        .arg("-json")
        .arg("-G")
        .arg("-n")
        .arg(file)
        .output()
        .map_err(|e| conformance::ExternalToolError {
            tool: "exiftool".to_string(),
            executable: exiftool.display().to_string(),
            exit_status: None,
            stderr_summary: e.to_string(),
            output_empty: true,
            json_parse_failed: false,
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(conformance::ExternalToolError {
            tool: "exiftool".to_string(),
            executable: exiftool.display().to_string(),
            exit_status: output.status.code(),
            stderr_summary: stderr.lines().take(3).collect::<Vec<_>>().join("; "),
            output_empty: output.stdout.is_empty(),
            json_parse_failed: false,
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Err(conformance::ExternalToolError {
            tool: "exiftool".to_string(),
            executable: exiftool.display().to_string(),
            exit_status: output.status.code(),
            stderr_summary: String::new(),
            output_empty: true,
            json_parse_failed: false,
        });
    }
    let arr: Vec<serde_json::Value> =
        serde_json::from_str(&stdout).map_err(|_e| conformance::ExternalToolError {
            tool: "exiftool".to_string(),
            executable: exiftool.display().to_string(),
            exit_status: output.status.code(),
            stderr_summary: String::new(),
            output_empty: false,
            json_parse_failed: true,
        })?;
    arr.into_iter()
        .next()
        .ok_or_else(|| conformance::ExternalToolError {
            tool: "exiftool".to_string(),
            executable: exiftool.display().to_string(),
            exit_status: output.status.code(),
            stderr_summary: "Empty JSON array".to_string(),
            output_empty: true,
            json_parse_failed: false,
        })
}

fn resolve_tag(obj: &serde_json::Value, _group_prefix: &str, tag_names: &[&str]) -> Option<String> {
    if let Some(map) = obj.as_object() {
        for tag in tag_names {
            for (key, val) in map {
                if key.ends_with(&format!(":{}", tag)) || key == *tag {
                    if let Some(s) = val.as_str() {
                        if !s.is_empty() {
                            return Some(s.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn resolve_array_tag(
    obj: &serde_json::Value,
    _group_prefix: &str,
    tag_names: &[&str],
) -> Vec<String> {
    if let Some(map) = obj.as_object() {
        for tag in tag_names {
            for (key, val) in map {
                if key.ends_with(&format!(":{}", tag)) || key == *tag {
                    match val {
                        serde_json::Value::String(s) => {
                            if !s.is_empty() {
                                return s.split(", ").map(|s| s.to_string()).collect();
                            }
                        }
                        serde_json::Value::Array(arr) => {
                            return arr
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Vec::new()
}

fn external_extract_json(
    file: &Path,
    exiftool: &Path,
) -> Result<ExternalExtraction, conformance::ExternalToolError> {
    let version = exiftool_version(exiftool);
    let obj = exiftool_json_extract(file, exiftool)?;
    let group = obj
        .as_object()
        .and_then(|m| m.keys().next())
        .and_then(|k| k.split(':').next())
        .unwrap_or("XMP");
    Ok(ExternalExtraction {
        tool: "exiftool".to_string(),
        version,
        copyright: resolve_tag(&obj, group, &["Copyright", "Rights", "XMP-dc:Rights"]),
        creators: resolve_array_tag(&obj, group, &["Creator", "XMP-dc:Creator"]),
        usage_terms: resolve_tag(&obj, group, &["UsageTerms", "XMP-xmpRights:UsageTerms"]),
        rights_url: resolve_tag(
            &obj,
            group,
            &[
                "WebStatementOfRights",
                "WebStatement",
                "XMP-xmpRights:WebStatement",
            ],
        ),
        credit_line: resolve_tag(&obj, group, &["CreditLine", "Credit", "photoshop:Credit"]),
        copyright_owner: resolve_tag(&obj, group, &["CopyrightOwner", "IPTC:CopyrightOwner"]),
        licensor_name: resolve_tag(&obj, group, &["LicensorName", "IPTC:LicensorName"]),
        licensor_email: resolve_tag(&obj, group, &["LicensorEmail", "IPTC:LicensorEmail"]),
        licensor_url: resolve_tag(
            &obj,
            group,
            &["LicensorURL", "LicensorUrl", "IPTC:LicensorUrl"],
        ),
        content_creation_date: resolve_tag(
            &obj,
            group,
            &["DateCreated", "ContentCreationDate", "DateTimeOriginal"],
        ),
        ai_constraints: resolve_tag(
            &obj,
            group,
            &["AIConstraints", "XMP-stegoeggo:AIConstraints"],
        ),
        canonical_data_mining: resolve_tag(&obj, group, &["DataMining", "XMP-plus:DataMining"]),
        legacy_data_mining: resolve_array_tag(
            &obj,
            group,
            &["DMI-Prohibited", "XMP-iptcExt:DMI-Prohibited"],
        ),
        tdm_reserved: resolve_tag(&obj, group, &["TDMReserve"]).and_then(|v| v.parse().ok()),
        extra: {
            let mut extra = HashMap::new();
            if let Some(map) = obj.as_object() {
                for (k, v) in map {
                    if let Some(s) = v.as_str() {
                        if !s.is_empty() {
                            extra.insert(k.clone(), s.to_string());
                        }
                    }
                }
            }
            extra
        },
    })
}

fn validate_xmp_structure(xmp: &str) -> Vec<(String, CheckSeverity, String)> {
    let mut results = Vec::new();

    if xmp.len() > MAX_XMP_BYTES {
        results.push((
            "xmp_size".to_string(),
            CheckSeverity::Fail,
            format!(
                "XMP exceeds maximum size: {} > {} bytes",
                xmp.len(),
                MAX_XMP_BYTES
            ),
        ));
    } else {
        results.push((
            "xmp_size".to_string(),
            CheckSeverity::Pass,
            format!("XMP size within bounds: {} bytes", xmp.len()),
        ));
    }

    let depth = xmp.matches('<').count();
    if depth > MAX_PROPERTIES {
        results.push((
            "xmp_property_count".to_string(),
            CheckSeverity::Fail,
            format!(
                "XMP property count exceeds limit: {} > {}",
                depth, MAX_PROPERTIES
            ),
        ));
    } else {
        results.push((
            "xmp_property_count".to_string(),
            CheckSeverity::Pass,
            format!("XMP property count within bounds: {}", depth),
        ));
    }

    let has_rdf_rdf = xmp.contains("<rdf:RDF") || xmp.contains("<rdf:Description");
    if has_rdf_rdf {
        results.push((
            "xmp_rdf_structure".to_string(),
            CheckSeverity::Pass,
            "XMP contains RDF structure".to_string(),
        ));
    } else if xmp.contains("x:xmpmeta") {
        results.push((
            "xmp_rdf_structure".to_string(),
            CheckSeverity::Warn,
            "XMP has x:xmpmeta wrapper but no RDF structure detected".to_string(),
        ));
    }

    let nesting = xmp.matches("rdf:Description").count();
    if nesting > 5 {
        results.push((
            "xmp_depth".to_string(),
            CheckSeverity::Warn,
            format!("Unusual number of rdf:Description elements: {}", nesting),
        ));
    }

    for line in xmp.lines() {
        if line.len() > MAX_TEXT_LEN {
            results.push((
                "xmp_text_length".to_string(),
                CheckSeverity::Fail,
                format!(
                    "XMP text line exceeds maximum length: {} > {}",
                    line.len(),
                    MAX_TEXT_LEN
                ),
            ));
            break;
        }
    }

    let has_dmi_property = xmp.contains("DataMining") || xmp.contains("dataMining");
    let has_plus_namespace = xmp.contains("http://ns.useplus.org/ldf/xmp/1.0/");
    if has_dmi_property && !has_plus_namespace {
        results.push((
            "xmp_plus_namespace".to_string(),
            CheckSeverity::Fail,
            "XMP contains DataMining property but missing PLUS namespace URI".to_string(),
        ));
    } else if has_dmi_property && has_plus_namespace {
        results.push((
            "xmp_plus_namespace".to_string(),
            CheckSeverity::Pass,
            "PLUS namespace URI present for DataMining property".to_string(),
        ));
    }

    results
}

fn extract_xmp_from_image(file: &Path) -> Option<String> {
    let bytes = std::fs::read(file).ok()?;
    conformance::detect_format(&bytes).and_then(|fmt| match fmt.as_str() {
        "webp" => extract_xmp_from_webp(&bytes),
        "png" => extract_xmp_from_png(&bytes),
        "jpeg" => extract_xmp_from_jpeg(&bytes),
        _ => None,
    })
}

fn extract_xmp_from_webp(bytes: &[u8]) -> Option<String> {
    let marker = b"XMP ";
    let mut pos = 0;
    while pos + 8 <= bytes.len() {
        if &bytes[pos..pos + 4] == marker {
            let size = u32::from_le_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]) as usize;
            if pos + 8 + size <= bytes.len() {
                let xmp_data = &bytes[pos + 8..pos + 8 + size];
                let needle = b"<?xpacket begin=";
                if let Some(start) = xmp_data.windows(needle.len()).position(|w| w == needle) {
                    return String::from_utf8(xmp_data[start..].to_vec()).ok();
                }
            }
        }
        pos += 1;
    }
    None
}

fn extract_xmp_from_png(bytes: &[u8]) -> Option<String> {
    let mut pos = 8;
    while pos + 8 <= bytes.len() {
        let length =
            u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
                as usize;
        let chunk_type = &bytes[pos + 4..pos + 8];
        if chunk_type == b"tEXt" || chunk_type == b"iTXt" {
            let data = &bytes[pos + 8..pos + 8 + length];
            if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                let key = &data[..null_pos];
                if key == b"XML:com.adobe.xmp" {
                    let val = &data[null_pos + 1..];
                    let xmp_str = String::from_utf8(val.to_vec()).ok()?;
                    if let Some(start) = xmp_str.find("<?xpacket") {
                        return Some(xmp_str[start..].to_string());
                    }
                    return Some(xmp_str);
                }
            }
        }
        pos += 12 + length;
    }
    None
}

fn extract_xmp_from_jpeg(bytes: &[u8]) -> Option<String> {
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
        let length = u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize;
        if marker == 0xE1 {
            let data = &bytes[pos + 4..pos + 2 + length];
            if data.starts_with(b"http://ns.adobe.com/xap/1.0/") {
                let xmp_bytes = &data[29..];
                return String::from_utf8(xmp_bytes.to_vec()).ok();
            }
        }
        pos += 2 + length;
    }
    None
}

fn internal_extract(file: &Path) -> Option<InternalExtraction> {
    let bytes = std::fs::read(file).ok()?;
    let notice = stegoeggo::verify_legal_notice(&bytes, b"");
    Some(InternalExtraction {
        copyright_holder: notice.copyright_holder().map(|s| s.to_string()),
        creators: notice
            .creator()
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        copyright_owner: notice.copyright_owner().map(|s| s.to_string()),
        usage_terms: notice.usage_terms().map(|s| s.to_string()),
        web_statement_of_rights: notice.web_statement_of_rights().map(|s| s.to_string()),
        credit_line: notice.credit_line().map(|s| s.to_string()),
        licensor_name: notice.licensor_name().map(|s| s.to_string()),
        licensor_email: notice.licensor_email().map(|s| s.to_string()),
        licensor_url: notice.licensor_url().map(|s| s.to_string()),
        content_creation_date: notice.metadata_date().map(|s| s.to_string()),
        ai_constraints: notice.ai_constraints().map(|s| s.to_string()),
        canonical_data_mining: notice.canonical_dmi().map(|d| d.as_str().to_string()),
        legacy_data_mining: notice
            .legacy_dmi()
            .map(|d| vec![d.as_str().to_string()])
            .unwrap_or_default(),
        tdm_reserved: notice.tdm_reserved(),
        seed: notice.protection_seed(),
        evidence_channels: notice
            .channels()
            .iter()
            .map(|c| format!("{:?}", c))
            .collect(),
        evidence_strength: Some(format!("{:?}", notice.evidence_strength())),
    })
}

/// Return type from the harness, bundling reports with verification results.
pub struct HarnessResult {
    pub reports: Vec<ConformanceReport>,
    pub digest_results: Option<Vec<DigestCheckResult>>,
    pub coverage: Option<CoverageCheckResult>,
    pub coverage_minimums: Option<conformance::CoverageMinimums>,
    pub tools: Vec<ToolReport>,
    pub manifest_report: Option<ManifestReport>,
    pub incomplete_reasons: Vec<String>,
    pub strict: bool,
}

fn evaluate_manifest_expectations(
    entry: &conformance::FixtureEntry,
    report: &mut ConformanceReport,
) {
    if !entry.expected_dmi.is_empty() {
        let normalized_expected = conformance::normalize_dmi_value(&entry.expected_dmi);
        if let Some(ref canonical) = report.internal.canonical_data_mining {
            let normalized_actual = conformance::normalize_dmi_value(canonical);
            if normalized_expected != normalized_actual {
                report.add_check_with_details(
                    "expected_dmi",
                    CheckSeverity::Fail,
                    "Observed DMI differs from manifest expectation",
                    &format!(
                        "expected={:?}, observed={:?}",
                        entry.expected_dmi, canonical
                    ),
                );
            } else {
                report.add_check(
                    "expected_dmi",
                    CheckSeverity::Pass,
                    "DMI matches manifest expectation",
                );
            }
        } else if entry.expected_decode != DecodeExpectation::Fail {
            report.add_check(
                "expected_dmi",
                CheckSeverity::Fail,
                "Expected DMI but none found internally",
            );
        }
    }

    let has_conflict = !report.conflicts.is_empty();
    if entry.expected_conflict && !has_conflict {
        report.add_check(
            "expected_conflict",
            CheckSeverity::Fail,
            "Expected conflict but none detected",
        );
    } else if !entry.expected_conflict && has_conflict {
        report.add_check(
            "expected_conflict",
            CheckSeverity::Fail,
            "Unexpected conflict detected",
        );
    } else if entry.expected_conflict {
        report.add_check(
            "expected_conflict",
            CheckSeverity::Pass,
            "Conflict correctly detected",
        );
    }

    let ef = &entry.expected_legal_fields;

    macro_rules! check_expected_field {
        ($name:expr, $expected:expr, $actual:expr) => {
            if let Some(ref exp) = $expected {
                if let Some(ref act) = $actual {
                    if exp == act {
                        report.add_check($name, CheckSeverity::Pass, "Field matches expectation");
                    } else {
                        report.add_check_with_details(
                            $name,
                            CheckSeverity::Fail,
                            "Field differs from expectation",
                            &format!("expected={:?}, observed={:?}", exp, act),
                        );
                    }
                } else {
                    report.add_check_with_details(
                        $name,
                        CheckSeverity::Fail,
                        "Expected field not found",
                        &format!("expected={:?}", exp),
                    );
                }
            }
        };
    }

    check_expected_field!(
        "expected_copyright_holder",
        ef.copyright_holder,
        report.internal.copyright_holder
    );
    let creator_actual = report.internal.creators.first().cloned();
    check_expected_field!("expected_creator", ef.creator, creator_actual);
    check_expected_field!(
        "expected_copyright_owner",
        ef.copyright_owner,
        report.internal.copyright_owner
    );
    check_expected_field!(
        "expected_usage_terms",
        ef.usage_terms,
        report.internal.usage_terms
    );
    check_expected_field!(
        "expected_rights_url",
        ef.web_statement_of_rights,
        report.internal.web_statement_of_rights
    );
    check_expected_field!(
        "expected_ai_constraints",
        ef.ai_constraints,
        report.internal.ai_constraints
    );
    check_expected_field!(
        "expected_credit_line",
        ef.credit_line,
        report.internal.credit_line
    );
    check_expected_field!(
        "expected_licensor_name",
        ef.licensor_name,
        report.internal.licensor_name
    );
    check_expected_field!(
        "expected_licensor_email",
        ef.licensor_email,
        report.internal.licensor_email
    );
    check_expected_field!(
        "expected_licensor_url",
        ef.licensor_url,
        report.internal.licensor_url
    );
}

struct ToolState {
    name: String,
    path: Option<PathBuf>,
    version: Option<String>,
    discovered: bool,
    exercised: bool,
    invocations: u32,
    successes: u32,
    failures: u32,
}

impl ToolState {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            path: None,
            version: None,
            discovered: false,
            exercised: false,
            invocations: 0,
            successes: 0,
            failures: 0,
        }
    }

    fn record_invocation(&mut self, success: bool) {
        self.exercised = true;
        self.invocations += 1;
        if success {
            self.successes += 1;
        } else {
            self.failures += 1;
        }
    }

    fn to_report(&self) -> ToolReport {
        ToolReport {
            name: self.name.clone(),
            path: self.path.as_ref().map(|p| p.display().to_string()),
            version: self.version.clone(),
            discovered: self.discovered,
            exercised: self.exercised,
            invocations: self.invocations,
            successes: self.successes,
            failures: self.failures,
        }
    }
}

pub fn run_harness(
    fixtures_dir: &Path,
    strict: bool,
    _json_path: Option<&Path>,
    format_filter: &Option<String>,
    manifest_path: Option<&Path>,
) -> HarnessResult {
    let mut exiftool_state = ToolState::new("exiftool");
    let mut xmllint_state = ToolState::new("xmllint");
    let mut imagemagick_state = ToolState::new("imagemagick");
    let mut vips_state = ToolState::new("libvips");

    if let Some(path) = find_exiftool() {
        exiftool_state.discovered = true;
        exiftool_state.path = Some(path.clone());
        exiftool_state.version = exiftool_version(&path);
    }

    xmllint_state.discovered = Command::new("xmllint").arg("--version").output().is_ok();
    if xmllint_state.discovered {
        xmllint_state.version = xmllint_version();
        xmllint_state.path = Some(PathBuf::from("xmllint"));
    }

    if let Some(path) = find_imagemagick() {
        imagemagick_state.discovered = true;
        imagemagick_state.path = Some(path);
        imagemagick_state.version = imagemagick_version();
    }

    if let Some(path) = find_vipsheader() {
        vips_state.discovered = true;
        vips_state.path = Some(path);
        vips_state.version = libvips_version();
    }

    let mut incomplete_reasons = Vec::new();

    if !exiftool_state.discovered {
        if strict {
            eprintln!("Error: exiftool required in strict mode but not found");
            std::process::exit(EXIT_CONFIG);
        }
        incomplete_reasons.push("exiftool not found".to_string());
        eprintln!("Warning: exiftool not found, skipping external validation");
    }

    if strict && manifest_path.is_none() {
        eprintln!("Error: --manifest required in strict mode");
        std::process::exit(EXIT_CONFIG);
    }

    if !fixtures_dir.exists() {
        if strict {
            eprintln!(
                "Error: fixtures directory not found at {}",
                fixtures_dir.display()
            );
            std::process::exit(EXIT_CONFIG);
        }
        incomplete_reasons.push(format!(
            "fixtures directory not found at {}",
            fixtures_dir.display()
        ));
        eprintln!(
            "Warning: fixtures directory not found at {}",
            fixtures_dir.display()
        );
        return HarnessResult {
            reports: Vec::new(),
            digest_results: None,
            coverage: None,
            coverage_minimums: None,
            tools: vec![
                exiftool_state.to_report(),
                xmllint_state.to_report(),
                imagemagick_state.to_report(),
                vips_state.to_report(),
            ],
            manifest_report: None,
            incomplete_reasons,
            strict,
        };
    }

    let mut reports = Vec::new();

    let fixture_files = conformance::collect_fixture_files(fixtures_dir, format_filter);

    if fixture_files.is_empty() {
        if strict {
            eprintln!(
                "Error: no fixture images found in {}",
                fixtures_dir.display()
            );
            std::process::exit(EXIT_CONFIG);
        }
        incomplete_reasons.push(format!(
            "no fixture images found in {}",
            fixtures_dir.display()
        ));
        eprintln!("No fixture images found in {}", fixtures_dir.display());
        return HarnessResult {
            reports,
            digest_results: None,
            coverage: None,
            coverage_minimums: None,
            tools: vec![
                exiftool_state.to_report(),
                xmllint_state.to_report(),
                imagemagick_state.to_report(),
                vips_state.to_report(),
            ],
            manifest_report: None,
            incomplete_reasons,
            strict,
        };
    }

    let mut manifest_report = None;
    let manifest = if let Some(mpath) = manifest_path {
        let sha256 = conformance::FixtureManifest::compute_sha256(mpath).unwrap_or_default();
        match conformance::load_manifest(mpath) {
            Ok(m) => {
                if strict && m.entries.is_empty() {
                    eprintln!("Error: manifest contains no entries");
                    std::process::exit(EXIT_CONFIG);
                }
                let validation = conformance::validate_manifest(&m);
                let is_valid = validation.is_ok();
                if !is_valid {
                    if let Err(errors) = &validation {
                        for e in errors {
                            eprintln!("Manifest validation error: {}", e);
                        }
                    }
                    if strict {
                        std::process::exit(EXIT_DIGEST);
                    }
                }
                let entry_count = m.entries.len();
                let duplicate_count = {
                    let mut ids = std::collections::HashSet::new();
                    m.entries.iter().filter(|e| !ids.insert(&e.id)).count()
                };
                manifest_report = Some(ManifestReport {
                    requested_path: mpath.display().to_string(),
                    canonical_path: mpath.canonicalize().ok().map(|p| p.display().to_string()),
                    sha256,
                    entry_count,
                    validation: validation.clone(),
                    duplicate_count,
                    unlisted_count: 0,
                    unexercised_count: 0,
                });
                if is_valid {
                    eprintln!("Loaded and validated manifest with {} entries", entry_count);
                    Some(m)
                } else {
                    None
                }
            }
            Err(e) => {
                if strict {
                    eprintln!("Error: {}", e);
                    std::process::exit(EXIT_DIGEST);
                }
                incomplete_reasons.push(format!("manifest load error: {}", e));
                eprintln!("Warning: {}", e);
                None
            }
        }
    } else {
        None
    };

    let path_index = manifest.as_ref().map(|m| m.path_index());

    for file in &fixture_files {
        let file_name = file
            .strip_prefix(fixtures_dir)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();

        let bytes = match std::fs::read(file) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Warning: cannot read {}: {}", file.display(), e);
                continue;
            }
        };

        let format = match conformance::detect_format(&bytes) {
            Some(f) => f,
            None => {
                eprintln!("Warning: cannot detect format for {}", file.display());
                continue;
            }
        };

        let mut report = ConformanceReport::new(&file_name, &format);

        if let Some(ref index) = path_index {
            if let Some(entry) = index.get(&file_name) {
                report.fixture_id = Some(entry.id.clone());
                report.category = Some(entry.category.clone());
                report.source = Some(entry.source.clone());
            } else if strict {
                report.add_check(
                    "manifest_entry",
                    CheckSeverity::Fail,
                    "File has no manifest entry",
                );
            }
        }

        let is_valid = image::load_from_memory(&bytes).is_ok();
        report.decode_valid = is_valid;

        let manifest_entry = path_index.as_ref().and_then(|idx| idx.get(&file_name));

        let (
            expected_decode,
            expected_xmp,
            expected_internal,
            expected_external,
            required_ext_fields,
        ) = if let Some(entry) = manifest_entry {
            (
                entry.expected_decode,
                entry.expected_xmp,
                entry.expected_internal,
                entry.expected_external,
                entry.required_external_fields.clone(),
            )
        } else {
            (
                DecodeExpectation::Pass,
                XmpExpectation::Valid,
                ExtractionExpectation::Success,
                ExtractionExpectation::Success,
                Vec::new(),
            )
        };

        match expected_decode {
            DecodeExpectation::Pass => {
                report.add_check(
                    "decode",
                    if is_valid {
                        CheckSeverity::Pass
                    } else {
                        CheckSeverity::Fail
                    },
                    if is_valid {
                        "Image decodes successfully"
                    } else {
                        "Image failed to decode"
                    },
                );
            }
            DecodeExpectation::Fail => {
                if is_valid {
                    report.add_check(
                        "decode",
                        CheckSeverity::Fail,
                        "Malformed fixture decoded successfully (unexpected)",
                    );
                } else {
                    report.add_check(
                        "decode",
                        CheckSeverity::Pass,
                        "Expected decode failure for malformed fixture",
                    );
                }
            }
            DecodeExpectation::Either => {
                report.add_check(
                    "decode",
                    CheckSeverity::Pass,
                    if is_valid {
                        "Image decodes (either outcome acceptable)"
                    } else {
                        "Image fails to decode (either outcome acceptable)"
                    },
                );
            }
        }

        if let Some(xmp) = extract_xmp_from_image(file) {
            report.xmp_valid = xmllint_validate(&xmp);
            let valid = report.xmp_valid.unwrap_or(false);
            if report.xmp_valid.is_some() {
                xmllint_state.record_invocation(true);
            }
            match expected_xmp {
                XmpExpectation::Valid => {
                    report.add_check(
                        "xmp_well-formed",
                        if valid {
                            CheckSeverity::Pass
                        } else {
                            CheckSeverity::Fail
                        },
                        if valid {
                            "XMP is well-formed XML"
                        } else {
                            "XMP is not well-formed XML"
                        },
                    );
                }
                XmpExpectation::Invalid => {
                    if valid {
                        report.add_check(
                            "xmp_well-formed",
                            CheckSeverity::Fail,
                            "Expected invalid XMP but found valid XML",
                        );
                    } else {
                        report.add_check(
                            "xmp_well-formed",
                            CheckSeverity::Pass,
                            "Expected invalid XMP and found invalid XML",
                        );
                    }
                }
                XmpExpectation::Absent => {
                    report.add_check(
                        "xmp_well-formed",
                        CheckSeverity::Fail,
                        "Expected no XMP but found XMP content",
                    );
                }
                XmpExpectation::Either => {
                    report.add_check(
                        "xmp_well-formed",
                        CheckSeverity::Pass,
                        if valid {
                            "XMP present and valid (either acceptable)"
                        } else {
                            "XMP present but invalid (either acceptable)"
                        },
                    );
                }
            }

            for (name, severity, message) in validate_xmp_structure(&xmp) {
                report.add_check(&name, severity, &message);
            }
        } else if expected_xmp == XmpExpectation::Absent {
            report.add_check(
                "xmp_well-formed",
                CheckSeverity::Pass,
                "Expected no XMP and none found",
            );
        } else if expected_xmp == XmpExpectation::Valid || expected_xmp == XmpExpectation::Invalid {
            report.add_check(
                "xmp_well-formed",
                CheckSeverity::Fail,
                "Expected XMP but none found",
            );
        }

        if let Some(internal) = internal_extract(file) {
            report.internal = internal;
            match expected_internal {
                ExtractionExpectation::Success => {
                    report.add_check(
                        "internal_extraction",
                        CheckSeverity::Pass,
                        "Internal extraction succeeded",
                    );
                }
                ExtractionExpectation::NoNotice => {
                    if report.internal.has_notice_content() {
                        report.add_check(
                            "internal_extraction",
                            CheckSeverity::Fail,
                            "Internal extraction returned notice content but NoNotice expected",
                        );
                    } else {
                        report.add_check(
                            "internal_extraction",
                            CheckSeverity::Pass,
                            "Internal extraction returned no notice (expected)",
                        );
                    }
                }
                ExtractionExpectation::Reject => {
                    report.add_check(
                        "internal_extraction",
                        CheckSeverity::Fail,
                        "Expected rejection but extraction succeeded",
                    );
                }
            }
        } else {
            match expected_internal {
                ExtractionExpectation::Success => {
                    report.add_check(
                        "internal_extraction",
                        CheckSeverity::Fail,
                        "Internal extraction failed",
                    );
                }
                ExtractionExpectation::NoNotice => {
                    report.add_check(
                        "internal_extraction",
                        CheckSeverity::Fail,
                        "Internal extraction failed; parser failure is not equivalent to no notice",
                    );
                }
                ExtractionExpectation::Reject => {
                    report.add_check(
                        "internal_extraction",
                        CheckSeverity::Pass,
                        "Expected rejection and extraction failed",
                    );
                }
            }
        }

        let exiftool_path = exiftool_state
            .path
            .as_ref()
            .expect("exiftool path should be set");
        let external_result = external_extract_json(file, exiftool_path);
        let external = match &external_result {
            Ok(ext) => {
                exiftool_state.record_invocation(true);
                report.external.push(ext.clone());
                match expected_external {
                    ExtractionExpectation::Success => {
                        let tool_ran = ext.version.is_some();
                        if tool_ran {
                            report.add_check(
                                "external_extraction",
                                CheckSeverity::Pass,
                                "External extraction succeeded",
                            );
                        } else {
                            report.add_check(
                                "external_extraction",
                                CheckSeverity::Fail,
                                "External extraction tool did not run",
                            );
                        }
                    }
                    ExtractionExpectation::NoNotice => {
                        if ext.has_notice_content() {
                            report.add_check(
                                "external_extraction",
                                CheckSeverity::Fail,
                                "External extraction returned notice content but NoNotice expected",
                            );
                        } else {
                            report.add_check(
                                "external_extraction",
                                CheckSeverity::Pass,
                                "External extraction returned no notice (expected)",
                            );
                        }
                    }
                    ExtractionExpectation::Reject => {
                        report.add_check(
                            "external_extraction",
                            CheckSeverity::Fail,
                            "Expected rejection but extraction succeeded",
                        );
                    }
                }
                ext.clone()
            }
            Err(err) => {
                exiftool_state.record_invocation(false);
                let fallback = ExternalExtraction {
                    tool: err.tool.clone(),
                    version: None,
                    ..Default::default()
                };
                report.external.push(fallback.clone());
                match expected_external {
                    ExtractionExpectation::Reject => {
                        report.add_check(
                            "external_extraction",
                            CheckSeverity::Pass,
                            &format!("Expected rejection: {}", err.stderr_summary),
                        );
                    }
                    ExtractionExpectation::NoNotice => {
                        report.add_check(
                            "external_extraction",
                            CheckSeverity::Fail,
                            "External extraction failed; command failure is not equivalent to no notice",
                        );
                    }
                    _ => {
                        report.add_check_with_details(
                            "external_extraction",
                            CheckSeverity::Fail,
                            &format!("External extraction failed: {}", err.stderr_summary),
                            &format!(
                                "tool={}, exit={:?}, stderr_empty={}, json_failed={}",
                                err.tool, err.exit_status, err.output_empty, err.json_parse_failed
                            ),
                        );
                    }
                }
                fallback
            }
        };

        for field in &required_ext_fields {
            let found = match field.as_str() {
                "canonical_data_mining" => external.canonical_data_mining.is_some(),
                "copyright" => external.copyright.is_some(),
                "usage_terms" => external.usage_terms.is_some(),
                "rights_url" => external.rights_url.is_some(),
                "credit_line" => external.credit_line.is_some(),
                "copyright_owner" => external.copyright_owner.is_some(),
                "ai_constraints" => external.ai_constraints.is_some(),
                _ => external.extra.contains_key(field),
            };
            if !found {
                report.add_check_with_details(
                    "required_external_field",
                    CheckSeverity::Fail,
                    &format!("Required external field '{}' not found", field),
                    &format!("field={}", field),
                );
            }
        }

        conformance::compare_extractions(&report.internal.clone(), &external, &mut report);

        if is_valid {
            if strict && !imagemagick_state.discovered {
                report.add_check(
                    "imagemagick",
                    CheckSeverity::Fail,
                    "ImageMagick (identify) required in strict mode but not found",
                );
            } else if imagemagick_state.discovered {
                match imagemagick_identify(file) {
                    Ok(output) => {
                        imagemagick_state.record_invocation(true);
                        report.add_check(
                            "imagemagick",
                            CheckSeverity::Pass,
                            &format!("ImageMagick identify: {}", output),
                        );
                    }
                    Err(e) => {
                        imagemagick_state.record_invocation(false);
                        report.add_check_with_details(
                            "imagemagick",
                            CheckSeverity::Fail,
                            "ImageMagick identify failed",
                            &e,
                        );
                    }
                }
            }
            if strict && !vips_state.discovered {
                report.add_check(
                    "libvips",
                    CheckSeverity::Fail,
                    "libvips (vipsheader) required in strict mode but not found",
                );
            } else if vips_state.discovered {
                match vipsheader_validate(file) {
                    Ok(output) => {
                        vips_state.record_invocation(true);
                        report.add_check(
                            "libvips",
                            CheckSeverity::Pass,
                            &format!("vipsheader: {}", output),
                        );
                    }
                    Err(e) => {
                        vips_state.record_invocation(false);
                        report.add_check_with_details(
                            "libvips",
                            CheckSeverity::Fail,
                            "vipsheader failed",
                            &e,
                        );
                    }
                }
            }
        }

        if let Some(ref index) = path_index {
            if let Some(entry) = index.get(&file_name) {
                evaluate_manifest_expectations(entry, &mut report);
            }
        }

        report.evaluate();
        reports.push(report);
    }

    if let Some(ref m) = manifest {
        let exercised: std::collections::HashSet<_> =
            reports.iter().map(|r| r.fixture.as_str()).collect();
        let unexercised: Vec<_> = m
            .entries
            .iter()
            .filter(|entry| !exercised.contains(entry.path.as_str()))
            .cloned()
            .collect();
        for entry in &unexercised {
            if strict {
                eprintln!(
                    "Error: manifest entry '{}' ({}) was not exercised",
                    entry.id, entry.path
                );
                reports.push({
                    let mut r = ConformanceReport::new(&entry.path, &entry.format);
                    r.add_check(
                        "manifest_coverage",
                        CheckSeverity::Fail,
                        &format!("Manifest entry '{}' was not exercised", entry.id),
                    );
                    r.evaluate();
                    r
                });
            }
        }
    }

    let mut digest_results = None;
    let mut coverage = None;

    if let Some(ref m) = manifest {
        let dr = conformance::verify_fixtures(m, fixtures_dir);
        let all_match = dr.iter().all(|d| d.matches);
        if !all_match {
            let mismatch_count = dr.iter().filter(|d| !d.matches).count();
            eprintln!("Digest verification: {} mismatches", mismatch_count);
            if strict {
                for d in &dr {
                    if !d.matches {
                        eprintln!(
                            "  {} expected={} actual={}",
                            d.fixture_path, d.expected, d.observed
                        );
                    }
                }
                std::process::exit(EXIT_DIGEST);
            }
        } else {
            eprintln!("Digest verification: all {} digests match", dr.len());
        }
        digest_results = Some(dr);

        let cov = conformance::check_coverage(m, &conformance::CoverageMinimums::default());
        if !cov.passed {
            eprintln!("Coverage check failed:");
            for v in &cov.violations {
                eprintln!("  - {}", v);
            }
            if strict {
                std::process::exit(EXIT_COVERAGE);
            }
        } else {
            eprintln!("Coverage check: PASS");
        }
        coverage = Some(cov);

        let tool_counts = m.count_by_authoring_tool();
        eprintln!("Authoring tool breakdown:");
        for (tool, count) in &tool_counts {
            eprintln!("  {}: {}", tool, count);
        }

        if let Some(ref mut mr) = manifest_report {
            let exercised: std::collections::HashSet<_> =
                reports.iter().map(|r| r.fixture.as_str()).collect();
            mr.unexercised_count = m
                .entries
                .iter()
                .filter(|entry| !exercised.contains(entry.path.as_str()))
                .count();
        }
    }

    let coverage_minimums = conformance::CoverageMinimums::default();

    let tools = vec![
        exiftool_state.to_report(),
        xmllint_state.to_report(),
        imagemagick_state.to_report(),
        vips_state.to_report(),
    ];

    if !exiftool_state.discovered && strict {
        incomplete_reasons.push("exiftool not found".to_string());
    }
    if !imagemagick_state.discovered && strict {
        incomplete_reasons.push("imagemagick not found".to_string());
    }
    if !vips_state.discovered && strict {
        incomplete_reasons.push("libvips not found".to_string());
    }

    HarnessResult {
        reports,
        digest_results,
        coverage,
        coverage_minimums: Some(coverage_minimums),
        tools,
        manifest_report,
        incomplete_reasons,
        strict,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut strict = false;
    let mut json_path: Option<PathBuf> = None;
    let mut fixtures_dir: Option<PathBuf> = None;
    let mut format_filter: Option<String> = None;
    let mut manifest_path: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--strict" => {
                strict = true;
                i += 1;
            }
            "--json" => {
                i += 1;
                if i < args.len() {
                    json_path = Some(PathBuf::from(&args[i]));
                    i += 1;
                }
            }
            "--fixtures" => {
                i += 1;
                if i < args.len() {
                    fixtures_dir = Some(PathBuf::from(&args[i]));
                    i += 1;
                }
            }
            "--format" => {
                i += 1;
                if i < args.len() {
                    format_filter = Some(args[i].clone());
                    i += 1;
                }
            }
            "--all-formats" => {
                format_filter = None;
                i += 1;
            }
            "--manifest" => {
                i += 1;
                if i < args.len() {
                    manifest_path = Some(PathBuf::from(&args[i]));
                    i += 1;
                }
            }
            _ => {
                eprintln!(
                    "Error: unknown argument '{}'. Use --help for usage.",
                    args[i]
                );
                std::process::exit(EXIT_CONFIG);
            }
        }
    }

    let dir = fixtures_dir.unwrap_or_else(|| PathBuf::from("tests/fixtures/conformance"));
    let result = run_harness(
        &dir,
        strict,
        json_path.as_deref(),
        &format_filter,
        manifest_path.as_deref(),
    );

    let total = result.reports.len();
    let passed_count = result.reports.iter().filter(|r| r.passed).count();
    let failed = total - passed_count;

    for report in &result.reports {
        println!("{}", report.summary());
        println!();
    }

    let summary = ConformanceSummary::from_reports(&result.reports);

    let complete = result.incomplete_reasons.is_empty();
    let report_passed = complete && failed == 0;

    let run_report = ConformanceRunReport {
        schema_version: 1,
        generated_by: "stegoeggo-conformance".to_string(),
        crate_version: env!("CARGO_PKG_VERSION").to_string(),
        commit_sha: option_env!("GIT_COMMIT_SHA").map(|s| s.to_string()),
        strict: result.strict,
        complete,
        passed: report_passed,
        started_at: None,
        manifest: result.manifest_report,
        tools: result.tools,
        coverage_minimums: result.coverage_minimums.clone(),
        coverage: result.coverage.clone(),
        digest_verification: result.digest_results.clone().unwrap_or_default(),
        summary,
        incomplete_reasons: result.incomplete_reasons,
        fixtures: result.reports,
    };

    if let Some(ref path) = json_path {
        let json = serde_json::to_string_pretty(&run_report).unwrap();
        std::fs::write(path, &json).unwrap();
        eprintln!("JSON report written to {}", path.display());
    }

    eprintln!("=== Conformance Summary ===");
    eprintln!(
        "Total: {}, Passed: {}, Failed: {}",
        total, passed_count, failed
    );
    eprintln!("Complete: {}, Passed: {}", complete, report_passed);

    if failed > 0 {
        std::process::exit(EXIT_FAIL);
    }
}
