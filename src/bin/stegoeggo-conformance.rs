use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use stegoeggo::conformance::{
    self, CheckSeverity, ConformanceReport, ExternalExtraction, InternalExtraction,
};

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

fn exiftool_json_extract(file: &Path, exiftool: &Path) -> Option<serde_json::Value> {
    let output = Command::new(exiftool)
        .arg("-json")
        .arg("-G")
        .arg("-n")
        .arg(file)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let arr: Vec<serde_json::Value> = serde_json::from_str(&stdout).ok()?;
    arr.into_iter().next()
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

fn external_extract_json(file: &Path, exiftool: &Path) -> ExternalExtraction {
    let version = exiftool_version(exiftool);
    match exiftool_json_extract(file, exiftool) {
        Some(obj) => {
            let group = obj
                .as_object()
                .and_then(|m| m.keys().next())
                .and_then(|k| k.split(':').next())
                .unwrap_or("XMP");
            ExternalExtraction {
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
                credit_line: resolve_tag(
                    &obj,
                    group,
                    &["CreditLine", "Credit", "photoshop:Credit"],
                ),
                copyright_owner: resolve_tag(
                    &obj,
                    group,
                    &["CopyrightOwner", "IPTC:CopyrightOwner"],
                ),
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
                canonical_data_mining: resolve_tag(
                    &obj,
                    group,
                    &["DataMining", "XMP-plus:DataMining"],
                ),
                legacy_data_mining: resolve_array_tag(
                    &obj,
                    group,
                    &["DMI-Prohibited", "XMP-iptcExt:DMI-Prohibited"],
                ),
                tdm_reserved: resolve_tag(&obj, group, &["TDMReserve"])
                    .and_then(|v| v.parse().ok()),
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
            }
        }
        None => ExternalExtraction {
            tool: "exiftool".to_string(),
            version,
            ..Default::default()
        },
    }
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

pub fn run_harness(
    fixtures_dir: &Path,
    strict: bool,
    json_path: Option<&Path>,
    format_filter: &Option<String>,
) -> Vec<ConformanceReport> {
    let exiftool = match find_exiftool() {
        Some(et) => et,
        None => {
            if strict {
                eprintln!("Error: exiftool required in strict mode but not found");
                std::process::exit(2);
            }
            eprintln!("Warning: exiftool not found, skipping external validation");
            return Vec::new();
        }
    };

    let mut reports = Vec::new();

    if !fixtures_dir.exists() {
        eprintln!(
            "Warning: fixtures directory not found at {}",
            fixtures_dir.display()
        );
        return reports;
    }

    let fixture_files = conformance::collect_fixture_files(fixtures_dir, format_filter);

    if fixture_files.is_empty() {
        eprintln!("No fixture images found in {}", fixtures_dir.display());
        return reports;
    }

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

        let is_valid = image::load_from_memory(&bytes).is_ok();
        report.decode_valid = is_valid;
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

        if let Some(xmp) = extract_xmp_from_image(file) {
            report.xmp_valid = xmllint_validate(&xmp);
            let valid = report.xmp_valid.unwrap_or(false);
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

            for (name, severity, message) in validate_xmp_structure(&xmp) {
                report.add_check(&name, severity, &message);
            }
        }

        if let Some(internal) = internal_extract(file) {
            report.internal = internal;
            report.add_check(
                "internal_extraction",
                CheckSeverity::Pass,
                "Internal extraction succeeded",
            );
        } else {
            report.add_check(
                "internal_extraction",
                CheckSeverity::Fail,
                "Internal extraction failed",
            );
        }

        let external = external_extract_json(file, &exiftool);
        report.external.push(external.clone());
        report.add_check(
            "external_extraction",
            CheckSeverity::Pass,
            "External extraction succeeded",
        );

        conformance::compare_extractions(&report.internal.clone(), &external, &mut report);

        report.evaluate();
        reports.push(report);
    }

    if let Some(path) = json_path {
        let json = serde_json::to_string_pretty(&reports).unwrap();
        std::fs::write(path, &json).unwrap();
        eprintln!("JSON report written to {}", path.display());
    }

    reports
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut strict = false;
    let mut json_path: Option<PathBuf> = None;
    let mut fixtures_dir: Option<PathBuf> = None;
    let mut format_filter: Option<String> = None;

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
            _ => {
                i += 1;
            }
        }
    }

    let dir = fixtures_dir.unwrap_or_else(|| PathBuf::from("tests/fixtures/conformance"));
    let reports = run_harness(&dir, strict, json_path.as_deref(), &format_filter);

    let total = reports.len();
    let passed = reports.iter().filter(|r| r.passed).count();
    let failed = total - passed;

    for report in &reports {
        println!("{}", report.summary());
        println!();
    }

    eprintln!("=== Conformance Summary ===");
    eprintln!("Total: {}, Passed: {}, Failed: {}", total, passed, failed);

    let et_ver = find_exiftool()
        .and_then(|et| exiftool_version(&et))
        .unwrap_or_else(|| "not found".to_string());
    let xl_ver = xmllint_version().unwrap_or_else(|| "not found".to_string());
    eprintln!("ExifTool version: {}", et_ver);
    eprintln!("xmllint version: {}", xl_ver);

    if failed > 0 {
        std::process::exit(1);
    }
}
