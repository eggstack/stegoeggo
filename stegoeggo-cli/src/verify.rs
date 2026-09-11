use crate::keys::resolve_key_input;
use crate::output::JsonVerifyOutput;
use std::fs;
use std::path::{Path, PathBuf};
use stegoeggo::{verify_legal_notice, Error, ProtectionLevel, StegoPayload, VerificationStatus};

pub(crate) fn print_payload_info(payload: &StegoPayload) {
    let level_str = ProtectionLevel::from_byte(payload.protection_level())
        .map(|l: ProtectionLevel| l.as_str())
        .unwrap_or("Unknown");
    println!("Level: {} (id: {})", level_str, payload.protection_level());
    println!("Seed: {}", payload.seed());
    println!("Intensity: {:.2}", payload.intensity());
    println!("Version: {}", payload.version());
}

pub(crate) fn run_legacy_verify(
    input_path: &Path,
    output: &Option<PathBuf>,
    key: &Option<String>,
    json: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    run_report(input_path, output.as_deref(), key, json, verbose, false)
}

pub(crate) fn run_inspect(
    input_path: &Path,
    key: &Option<String>,
    json: bool,
    verbose: bool,
    assert_protected: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    run_report(input_path, None, key, json, verbose, assert_protected)
}

fn run_report(
    input_path: &Path,
    output: Option<&Path>,
    key: &Option<String>,
    json: bool,
    verbose: bool,
    assert_protected: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes_to_verify = if let Some(output_path) = output {
        if verbose {
            eprintln!("Verifying explicit output file");
        }
        fs::read(output_path)?
    } else {
        if verbose {
            eprintln!("Verifying input file");
        }
        fs::read(input_path)?
    };

    let mac_key = resolve_key_input(key, "STEGOEGGO_KEY")?.unwrap_or_default();

    let notice = verify_legal_notice(&bytes_to_verify, &mac_key);

    if json {
        let json_output = JsonVerifyOutput {
            schema_version: 1,
            status: if assert_protected && verification_failed(&notice) {
                "failed".to_string()
            } else {
                "ok".to_string()
            },
            copyright_holder: notice.copyright_holder().map(String::from),
            rights_url: notice.rights_url().map(String::from),
            ai_constraints: notice.ai_constraints().map(String::from),
            stego_status: format!("{:?}", notice.stego_status()),
            evidence_strength: notice.evidence_strength().to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else {
        println!(
            "Rights notice: {}",
            if notice.has_notice() {
                "Found"
            } else {
                "Not found"
            }
        );
        if let Some(holder) = notice.copyright_holder() {
            println!("Copyright holder: {}", holder);
        }
        if let Some(creator) = notice.creator() {
            println!("Creator: {}", creator);
        }
        if let Some(contact) = notice.contact() {
            println!("Contact: {}", contact);
        }
        if let Some(url) = notice.rights_url() {
            println!("Rights URL: {}", url);
        }
        if let Some(dmi) = notice.dmi() {
            println!("AI training restriction: {}", dmi.as_str());
        }
        if let Some(canonical) = notice.canonical_dmi() {
            println!("Canonical DMI: {}", canonical.as_str());
        }
        if let Some(legacy) = notice.legacy_dmi() {
            println!("Legacy DMI: {}", legacy.as_str());
        }
        if notice.has_dmi_conflict() {
            println!("DMI conflict: YES (canonical and legacy values disagree)");
        }
        if let Some(reserved) = notice.tdm_reserved() {
            println!(
                "TDM reservation: {}",
                if reserved { "reserved" } else { "not reserved" }
            );
        }
        if let Some(terms) = notice.usage_terms() {
            println!("Usage terms: {}", terms);
        }
        if let Some(line) = notice.credit_line() {
            println!("Credit line: {}", line);
        }
        if let Some(owner) = notice.copyright_owner() {
            println!("Copyright owner: {}", owner);
        }
        if let Some(name) = notice.licensor_name() {
            println!("Licensor name: {}", name);
        }
        if let Some(email) = notice.licensor_email() {
            println!("Licensor email: {}", email);
        }
        if let Some(url) = notice.licensor_url() {
            println!("Licensor URL: {}", url);
        }
        if let Some(date) = notice.metadata_date() {
            println!("Metadata date: {}", date);
        }
        if let Some(ts) = notice.notice_applied_at() {
            println!("Notice applied at: {}", ts);
        }
        if let Some(seed) = notice.protection_seed() {
            println!("Protection seed: {}", seed);
        }

        println!();

        match notice.stego_status() {
            stegoeggo::VerificationStatus::Verified => {
                println!("Stego marker: Found, checksum verified");
            }
            stegoeggo::VerificationStatus::Invalid => {
                println!("Stego marker: Found, but integrity check failed");
            }
            stegoeggo::VerificationStatus::NotFound => {
                println!("Stego marker: Not found");
            }
        }

        if notice.authenticated() {
            println!("Authenticated provenance: Verified");
        } else if notice.stego_status() == stegoeggo::VerificationStatus::Invalid {
            println!("Authenticated provenance: Not verified (integrity check failed)");
        } else {
            println!("Authenticated provenance: Not configured");
        }

        println!("Evidence strength: {}", notice.evidence_strength());

        if let Some(payload) = notice.stego_payload() {
            println!();
            print_payload_info(payload);
        }
    }

    if assert_protected && verification_failed(&notice) {
        return Err(Box::new(Error::PayloadVerification(
            if notice.stego_status() == VerificationStatus::Invalid {
                "protection marker integrity or authentication verification failed".to_string()
            } else {
                "no protection evidence found".to_string()
            },
        )));
    }

    Ok(())
}

fn verification_failed(notice: &stegoeggo::NoticeVerification) -> bool {
    notice.stego_status() == VerificationStatus::Invalid
        || (!notice.has_notice() && notice.stego_status() == VerificationStatus::NotFound)
}
