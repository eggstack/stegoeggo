use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use stegoeggo::conformance::{
    CheckSeverity, ConformanceReport, ExternalExtraction, InternalExtraction,
};

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

fn exiftool_extract(file: &Path, exiftool: &Path, tag: &str) -> Option<String> {
    let output = Command::new(exiftool)
        .arg("-s3")
        .arg(tag)
        .arg(file)
        .output()
        .ok()?;
    if output.status.success() {
        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if val.is_empty() {
            None
        } else {
            Some(val)
        }
    } else {
        None
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

fn extract_xmp_from_image(file: &Path) -> Option<String> {
    let bytes = std::fs::read(file).ok()?;
    detect_format(&bytes).and_then(|fmt| match fmt.as_str() {
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
                    return String::from_utf8(val.to_vec()).ok();
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

fn detect_format(bytes: &[u8]) -> Option<String> {
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

fn external_extract(file: &Path, exiftool: &Path) -> ExternalExtraction {
    ExternalExtraction {
        tool: "exiftool".to_string(),
        version: exiftool_version(exiftool),
        copyright: exiftool_extract(file, exiftool, "-Copyright")
            .or_else(|| exiftool_extract(file, exiftool, "-XMP-dc:Rights")),
        creators: exiftool_extract(file, exiftool, "-Creator")
            .or_else(|| exiftool_extract(file, exiftool, "-XMP-dc:Creator"))
            .map(|v| v.split(", ").map(|s| s.to_string()).collect())
            .unwrap_or_default(),
        usage_terms: exiftool_extract(file, exiftool, "-UsageTerms")
            .or_else(|| exiftool_extract(file, exiftool, "-XMP-xmpRights:UsageTerms")),
        rights_url: exiftool_extract(file, exiftool, "-WebStatement")
            .or_else(|| exiftool_extract(file, exiftool, "-XMP-xmpRights:WebStatement")),
        credit_line: exiftool_extract(file, exiftool, "-Credit")
            .or_else(|| exiftool_extract(file, exiftool, "-photoshop:Credit")),
        copyright_owner: exiftool_extract(file, exiftool, "-CopyrightOwner"),
        licensor_name: exiftool_extract(file, exiftool, "-LicensorName"),
        licensor_email: exiftool_extract(file, exiftool, "-LicensorEmail"),
        licensor_url: exiftool_extract(file, exiftool, "-LicensorUrl"),
        content_creation_date: exiftool_extract(file, exiftool, "-ContentCreationDate")
            .or_else(|| exiftool_extract(file, exiftool, "-DateTimeOriginal")),
        ai_constraints: exiftool_extract(file, exiftool, "-AIConstraints"),
        canonical_data_mining: exiftool_extract(file, exiftool, "-XMP-plus:DataMining"),
        legacy_data_mining: exiftool_extract(file, exiftool, "-XMP-iptcExt:DMI-Prohibited")
            .map(|v| vec![v])
            .unwrap_or_default(),
        tdm_reserved: exiftool_extract(file, exiftool, "-TDMReserve").and_then(|v| v.parse().ok()),
        extra: HashMap::new(),
    }
}

fn compare_extractions(
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
    check(
        "canonical_dmi",
        &internal.canonical_data_mining,
        &external.canonical_data_mining,
        report,
    );

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

fn run_harness(
    fixtures_dir: &Path,
    strict: bool,
    json_path: Option<&Path>,
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

    let entries: Vec<_> = std::fs::read_dir(fixtures_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| matches!(ext, "png" | "jpg" | "jpeg" | "webp"))
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        eprintln!("No fixture images found in {}", fixtures_dir.display());
        return reports;
    }

    for entry in &entries {
        let file = entry.path();
        let file_name = file.file_name().unwrap().to_string_lossy().to_string();

        let bytes = match std::fs::read(&file) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Warning: cannot read {}: {}", file.display(), e);
                continue;
            }
        };

        let format = match detect_format(&bytes) {
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

        if let Some(xmp) = extract_xmp_from_image(&file) {
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
        }

        if let Some(internal) = internal_extract(&file) {
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

        let external = external_extract(&file, &exiftool);
        report.external.push(external.clone());
        report.add_check(
            "external_extraction",
            CheckSeverity::Pass,
            "External extraction succeeded",
        );

        compare_extractions(&report.internal.clone(), &external, &mut report);

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
            "--format" | "--all-formats" => {
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    let dir = fixtures_dir.unwrap_or_else(|| PathBuf::from("tests/fixtures/conformance"));
    let reports = run_harness(&dir, strict, json_path.as_deref());

    let total = reports.len();
    let passed = reports.iter().filter(|r| r.passed).count();
    let failed = total - passed;

    for report in &reports {
        println!("{}", report.summary());
        println!();
    }

    eprintln!("=== Conformance Summary ===");
    eprintln!("Total: {}, Passed: {}, Failed: {}", total, passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }
}
