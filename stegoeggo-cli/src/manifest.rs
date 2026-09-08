#[cfg(feature = "signatures")]
use crate::keys::resolve_key_input;
#[cfg(feature = "signatures")]
use crate::output::config_err;
#[cfg(feature = "signatures")]
use crate::protect::write_atomic;
#[cfg(feature = "signatures")]
use std::fs;
#[cfg(feature = "signatures")]
use std::path::PathBuf;

#[cfg(feature = "signatures")]
pub(crate) fn handle_keygen(
    output_dir: &PathBuf,
    key_id: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    use stegoeggo::signing::SigningKey;

    let key = SigningKey::generate()?;
    let verifying_key = key.verifying_key();

    let key_id_hex = key_id
        .as_deref()
        .map(|id| id.to_string())
        .unwrap_or_else(|| hex::encode(key.key_id()));

    let private_path = output_dir.join("key_private.pem");
    let public_path = output_dir.join("key_public.pem");

    fs::create_dir_all(output_dir)?;

    let private_pem = format!(
        "-----BEGIN STEGOEGGO PRIVATE KEY-----\nkey_id:{}\n{}\n-----END STEGOEGGO PRIVATE KEY-----\n",
        key_id_hex,
        hex::encode(key.to_bytes())
    );
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&private_path)?;
        file.write_all(private_pem.as_bytes())?;
    }
    #[cfg(not(unix))]
    fs::write(&private_path, private_pem.as_bytes())?;

    let public_pem = format!(
        "-----BEGIN STEGOEGGO PUBLIC KEY-----\nkey_id:{}\n{}\n-----END STEGOEGGO PUBLIC KEY-----\n",
        key_id_hex,
        hex::encode(verifying_key.as_bytes())
    );
    fs::write(&public_path, public_pem.as_bytes())?;

    println!("Key pair generated:");
    println!("  Private key: {}", private_path.display());
    println!("  Public key:  {}", public_path.display());
    println!("  Key ID:      {}", key_id_hex);

    Ok(())
}

#[cfg(feature = "signatures")]
pub(crate) fn handle_sign(
    manifest_path: &PathBuf,
    key_path: &PathBuf,
    output: &Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    use stegoeggo::detached::{DetachedManifest, PublicKeyEntry, SignatureRecord};
    use stegoeggo::resource_limits::ResourceLimits;
    use stegoeggo::signing::SigningKey;

    let key_bytes = fs::read(key_path)?;
    let key_str = String::from_utf8_lossy(&key_bytes);

    let hex_key = extract_pem_field(&key_str, "BEGIN STEGOEGGO PRIVATE KEY")
        .and_then(|block| {
            let key_id = block
                .lines()
                .find(|l| l.starts_with("key_id:"))
                .map(|l| l.strip_prefix("key_id:").unwrap_or("").to_string());
            let key_hex = block
                .lines()
                .find(|l| !l.starts_with("key_id:"))
                .map(String::from);
            key_hex.map(|k| (k, key_id.unwrap_or_default()))
        })
        .unwrap_or_else(|| {
            (
                String::from_utf8_lossy(&key_bytes).trim().to_string(),
                String::new(),
            )
        });

    let key_bytes_vec = hex::decode(&hex_key.0).map_err(|e| {
        config_err(format!(
            "Invalid hex key data in {}: {}",
            key_path.display(),
            e
        ))
    })?;
    if key_bytes_vec.len() != 32 {
        return Err(config_err(format!(
            "Private key must be 32 bytes, got {}",
            key_bytes_vec.len()
        )));
    }
    let mut raw_key = [0u8; 32];
    raw_key.copy_from_slice(&key_bytes_vec);

    let signing_key = SigningKey::from_bytes(raw_key, hex_key.1.into_bytes())
        .map_err(|e| config_err(format!("Invalid signing key: {}", e)))?;

    let manifest_bytes = fs::read(manifest_path)?;
    let limits = ResourceLimits::default();
    let mut manifest = DetachedManifest::from_json_with_limits(&manifest_bytes, &limits)
        .map_err(|e| config_err(format!("Manifest parsing failed: {}", e)))?;

    let claim_bytes = manifest.claim.canonical_bytes();
    let signature_bytes = signing_key.sign(&claim_bytes);
    let signature_hex = hex::encode(&signature_bytes);

    let key_id = signing_key.verifying_key().key_id().to_vec();

    let sig_record = SignatureRecord {
        algorithm: "ed25519".to_string(),
        key_id,
        signature: signature_hex,
    };
    manifest = manifest.with_signature(sig_record)?;

    let public_key = signing_key.verifying_key();
    let pub_entry = PublicKeyEntry {
        key_id: public_key.key_id().to_vec(),
        algorithm: "ed25519".to_string(),
        key_bytes: hex::encode(public_key.as_bytes()),
    };
    manifest = manifest.with_public_key(pub_entry)?;

    let signed_json = serde_json::to_string_pretty(&manifest)?;
    let out_path = output.as_ref().unwrap_or(manifest_path);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_atomic(out_path, signed_json.as_bytes())?;

    println!("Manifest signed: {}", out_path.display());
    Ok(())
}

#[cfg(feature = "signatures")]
pub(crate) fn handle_verify_manifest(
    manifest_path: &PathBuf,
    image_path: &PathBuf,
    key_path: &Option<PathBuf>,
    payload_key: Option<String>,
    json_output: bool,
) -> Result<i32, Box<dyn std::error::Error>> {
    use stegoeggo::detached::verify::{
        verify_detached_manifest_with_options, DetachedVerificationOptions, EmbeddedReferenceStatus,
    };
    use stegoeggo::detached::DetachedManifest;
    use stegoeggo::resource_limits::ResourceLimits;
    use stegoeggo::signing::VerifyingKey;

    let manifest_bytes = fs::read(manifest_path)?;
    let limits = ResourceLimits::default();
    let manifest = DetachedManifest::from_json_with_limits(&manifest_bytes, &limits)
        .map_err(|e| config_err(format!("Manifest parsing failed: {}", e)))?;

    let image_bytes = fs::read(image_path)?;

    let caller_keys: Vec<stegoeggo::detached::TrustedVerifyingKey> = if let Some(ref key_file) =
        key_path
    {
        let pub_key_bytes = fs::read(key_file)?;
        let pub_key_str = String::from_utf8_lossy(&pub_key_bytes);

        let (hex_pub, key_id_hex) = extract_pem_field(&pub_key_str, "BEGIN STEGOEGGO PUBLIC KEY")
            .and_then(|block| {
                let key_id = block
                    .lines()
                    .find(|l| l.starts_with("key_id:"))
                    .map(|l| l.strip_prefix("key_id:").unwrap_or("").to_string());
                let key_hex = block
                    .lines()
                    .find(|l| !l.starts_with("key_id:"))
                    .map(String::from);
                key_hex.map(|k| (k, key_id.unwrap_or_default()))
            })
            .unwrap_or_else(|| {
                (
                    String::from_utf8_lossy(&pub_key_bytes).trim().to_string(),
                    String::new(),
                )
            });

        let pub_bytes_vec = hex::decode(&hex_pub)
            .map_err(|e| config_err(format!("Invalid hex in public key file: {}", e)))?;
        if pub_bytes_vec.len() != 32 {
            return Err(config_err(format!(
                "Public key must be 32 bytes, got {}",
                pub_bytes_vec.len()
            )));
        }
        let mut raw_pub = [0u8; 32];
        raw_pub.copy_from_slice(&pub_bytes_vec);
        let vk = VerifyingKey::from_bytes(raw_pub, key_id_hex.into_bytes())
            .map_err(|e| config_err(format!("Invalid public key: {e}")))?;
        vec![stegoeggo::detached::TrustedVerifyingKey {
            key_id: vk.key_id().to_vec(),
            key: vk,
        }]
    } else {
        Vec::new()
    };

    let payload_mac_key = resolve_key_input(&payload_key, "")?;

    let options = DetachedVerificationOptions {
        trust_policy: None,
        caller_verifying_keys: &caller_keys,
        payload_mac_key: payload_mac_key.as_deref(),
        limits: Some(&limits),
    };
    let result = verify_detached_manifest_with_options(&image_bytes, &manifest, &options);

    let overall = result.overall_status();

    if json_output {
        let status_str = match overall {
            stegoeggo::detached::DetachedOverallStatus::VerifiedTrusted => "verified_trusted",
            stegoeggo::detached::DetachedOverallStatus::VerifiedUntrusted => "verified_untrusted",
            stegoeggo::detached::DetachedOverallStatus::InvalidConfiguration => {
                "invalid_configuration"
            }
            stegoeggo::detached::DetachedOverallStatus::BindingFailure => "binding_failure",
            stegoeggo::detached::DetachedOverallStatus::SignatureFailure => "signature_failure",
            stegoeggo::detached::DetachedOverallStatus::EmbeddedReferenceFailure => {
                "embedded_reference_failure"
            }
            stegoeggo::detached::DetachedOverallStatus::KeyMaterialMismatch => {
                "key_material_mismatch"
            }
        };

        #[derive(serde::Serialize)]
        struct JsonSignatureDetail {
            key_id: String,
            cryptographically_valid: bool,
            key_id_matched: bool,
            key_material_matched: bool,
            trusted: bool,
        }

        #[derive(serde::Serialize)]
        struct JsonManifestVerify {
            schema_version: u32,
            overall_status: &'static str,
            trust_mode: &'static str,
            instance_digest_match: bool,
            manifest_valid: bool,
            embedded_reference: &'static str,
            signatures_valid: bool,
            trusted: bool,
            evidence_strength: String,
            signatures: Vec<JsonSignatureDetail>,
        }

        let embedded_ref = match result.embedded_reference_status {
            EmbeddedReferenceStatus::NotProvided => "not_provided",
            EmbeddedReferenceStatus::Stripped => "stripped",
            EmbeddedReferenceStatus::VersionMismatch => "version_mismatch",
            EmbeddedReferenceStatus::DigestMismatch => "digest_mismatch",
            EmbeddedReferenceStatus::Malformed => "malformed",
            #[allow(deprecated)]
            EmbeddedReferenceStatus::Present => "present",
            EmbeddedReferenceStatus::PresentValid => "present_valid",
            EmbeddedReferenceStatus::AuthenticationKeyMissing => "authentication_key_missing",
            EmbeddedReferenceStatus::AuthenticationFailed => "authentication_failed",
            EmbeddedReferenceStatus::UnsupportedVersion => "unsupported_version",
        };

        let sigs_valid = result
            .report
            .signatures()
            .iter()
            .any(|s| s.cryptographically_valid());
        let trusted = result.report.trust().trusted();

        let sig_details: Vec<JsonSignatureDetail> = result
            .report
            .signatures()
            .iter()
            .map(|s| JsonSignatureDetail {
                key_id: hex::encode(s.public_key_id().unwrap_or(&[])),
                cryptographically_valid: s.cryptographically_valid(),
                key_id_matched: s.key_id_matched(),
                key_material_matched: s.key_material_matched(),
                trusted: s.trusted(),
            })
            .collect();

        let trust_mode = if !caller_keys.is_empty() {
            "caller_verifying_key"
        } else {
            "none"
        };

        let json = JsonManifestVerify {
            schema_version: manifest.schema_version as u32,
            overall_status: status_str,
            trust_mode,
            instance_digest_match: result.instance_digest_match,
            manifest_valid: result.manifest_valid,
            embedded_reference: embedded_ref,
            signatures_valid: sigs_valid,
            trusted,
            evidence_strength: format!("{:?}", result.report.evidence_strength()),
            signatures: sig_details,
        };
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Manifest schema version: {}", manifest.schema_version);
        println!("Claim ID: {}", hex::encode(manifest.claim.claim_id));
        println!("Instance digest: {}", manifest.claim.instance_digest);
        println!("Format: {}", manifest.claim.format);
        println!(
            "Dimensions: {}x{}",
            manifest.claim.width, manifest.claim.height
        );
        println!("File size: {} bytes", manifest.claim.file_size);
        println!("Rights policy: {}", manifest.claim.rights_policy);
        println!("Software: {}", manifest.claim.software);

        if result.instance_digest_match {
            println!("\nImage digest: MATCH");
        } else {
            println!("\nImage digest: MISMATCH");
        }

        println!(
            "\nManifest valid: {}",
            if result.manifest_valid { "YES" } else { "NO" }
        );

        if manifest.signatures.is_empty() {
            println!("Signatures: None");
        } else {
            println!("Signatures: {} total", manifest.signatures.len());
            for (i, sig) in result.report.signatures().iter().enumerate() {
                println!("  [{}] algorithm: ed25519", i);
                println!(
                    "      key_id: {}",
                    hex::encode(sig.public_key_id().unwrap_or(&[]))
                );
                println!(
                    "      cryptographically_valid: {}",
                    sig.cryptographically_valid()
                );
                println!("      key_id_matched: {}", sig.key_id_matched());
                println!("      key_material_matched: {}", sig.key_material_matched());
                println!("      trusted: {}", sig.trusted());
            }
        }

        println!(
            "\nEmbedded reference: {:?}",
            result.embedded_reference_status
        );
        let trust_mode_str = if !caller_keys.is_empty() {
            "caller_verifying_key"
        } else {
            "none"
        };
        println!(
            "Trust: {} (mode: {})",
            if result.report.trust().trusted() {
                "TRUSTED"
            } else {
                "UNTRUSTED"
            },
            trust_mode_str
        );
        println!("Evidence strength: {:?}", result.report.evidence_strength());
        println!("Overall status: {:?}", overall);

        for diag in result.report.diagnostics() {
            println!("  [{:?}] {}", diag.level(), diag.message());
        }
    }

    Ok(overall.exit_code())
}

#[cfg(feature = "signatures")]
pub(crate) fn extract_pem_field(pem_str: &str, begin_tag: &str) -> Option<String> {
    let start_marker = format!("-----{}-----", begin_tag);
    let end_marker = start_marker.replacen("BEGIN", "END", 1);

    let start = pem_str.find(&start_marker)? + start_marker.len();
    let end = pem_str[start..].find(&end_marker)? + start;
    Some(pem_str[start..end].trim().to_string())
}

#[cfg(all(test, feature = "signatures"))]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pem_field_end_before_begin_returns_none() {
        let pem = "-----END STEGOEGGO PRIVATE KEY-----\nabc\n";
        assert_eq!(extract_pem_field(pem, "BEGIN STEGOEGGO PRIVATE KEY"), None);
    }

    #[test]
    fn test_extract_pem_field_normal_block() {
        let pem = "-----BEGIN STEGOEGGO PRIVATE KEY-----\nkey_id:abcd\n1234\n-----END STEGOEGGO PRIVATE KEY-----\n";
        assert_eq!(
            extract_pem_field(pem, "BEGIN STEGOEGGO PRIVATE KEY"),
            Some("key_id:abcd\n1234".to_string())
        );
    }
}
