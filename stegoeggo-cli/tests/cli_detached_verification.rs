#[cfg(feature = "signatures")]
mod cli_detached_verification {
    use image::{GenericImage, GenericImageView};
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    fn cli_bin() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_BIN_EXE_stegoeggo"));
        if !path.exists() {
            let output = Command::new("cargo")
                .args(["build", "-p", "stegoeggo-cli", "--features", "signatures"])
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .output()
                .expect("Failed to build CLI");
            assert!(output.status.success(), "CLI build failed");
            path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/stegoeggo");
        }
        path
    }

    fn create_test_png(path: &std::path::Path) {
        let img = image::DynamicImage::new_rgb8(64, 64);
        let mut rgb = img.to_rgb8();
        for y in 0..64u32 {
            for x in 0..64u32 {
                let r = ((x * 7 + y * 3) % 256) as u8;
                let g = ((x * 11 + y * 5) % 256) as u8;
                let b = ((x * 13 + y * 9) % 256) as u8;
                rgb.put_pixel(x, y, image::Rgb([r, g, b]));
            }
        }
        let dyn_img = image::DynamicImage::ImageRgb8(rgb);
        let file = fs::File::create(path).unwrap();
        let encoder = image::codecs::png::PngEncoder::new(file);
        image::ImageEncoder::write_image(
            encoder,
            &dyn_img.to_rgb8(),
            64,
            64,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    }

    fn compute_digest(path: &std::path::Path) -> String {
        use sha2::{Digest, Sha256};
        let bytes = fs::read(path).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        format!("sha256:{}", hex::encode(hasher.finalize()))
    }

    fn create_manifest_json(
        image_path: &std::path::Path,
        output_path: &std::path::Path,
        embedded_reference: Option<serde_json::Value>,
    ) {
        let bytes = fs::read(image_path).unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        let (w, h) = img.dimensions();
        let digest = compute_digest(image_path);

        let claim_id = "00000000000000000000000000000000";

        let claim = serde_json::json!({
            "claim_id": claim_id,
            "content_code": "",
            "created_at": 0u64,
            "file_size": bytes.len() as u64,
            "format": "png",
            "height": h,
            "instance_digest": digest,
            "issuer_id": "",
            "notice_digest": "",
            "rights_policy": 0u8,
            "schema_version": 1u8,
            "software": "stegoeggo-test",
            "width": w,
        });

        if let Some(er) = embedded_reference {
            let manifest = serde_json::json!({
                "schema_version": 1,
                "claim": claim,
                "signatures": [],
                "public_keys": [],
                "embedded_reference": er,
            });
            fs::write(
                output_path,
                serde_json::to_string_pretty(&manifest).unwrap(),
            )
            .unwrap();
        } else {
            let manifest = serde_json::json!({
                "schema_version": 1,
                "claim": claim,
                "signatures": [],
                "public_keys": [],
            });
            fs::write(
                output_path,
                serde_json::to_string_pretty(&manifest).unwrap(),
            )
            .unwrap();
        }
    }

    fn run_cli(args: &[&str]) -> (i32, String, String) {
        let output = Command::new(cli_bin())
            .args(args)
            .output()
            .expect("Failed to execute CLI");
        let code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        (code, stdout, stderr)
    }

    // B5.1: keygen creates a private/public pair
    #[test]
    fn b5_1_keygen_creates_key_pair() {
        let tmp = tempfile::tempdir().unwrap();
        let key_dir = tmp.path().join("keys");

        let (code, stdout, _) = run_cli(&[
            "keygen",
            "--output-dir",
            key_dir.to_str().unwrap(),
            "--key-id",
            "test-key",
        ]);
        assert_eq!(code, 0, "keygen should succeed");
        assert!(stdout.contains("test-key"), "should print key ID");

        assert!(
            key_dir.join("key_private.pem").exists(),
            "private key file should exist"
        );
        assert!(
            key_dir.join("key_public.pem").exists(),
            "public key file should exist"
        );

        let priv_key = fs::read_to_string(key_dir.join("key_private.pem")).unwrap();
        assert!(
            priv_key.contains("BEGIN STEGOEGGO PRIVATE KEY"),
            "private key should be PEM format"
        );
        let pub_key = fs::read_to_string(key_dir.join("key_public.pem")).unwrap();
        assert!(
            pub_key.contains("BEGIN STEGOEGGO PUBLIC KEY"),
            "public key should be PEM format"
        );
    }

    // B5.2-3: Create manifest, sign it, verify with correct key → exit 0
    #[test]
    fn b5_2_sign_and_verify_exits_0() {
        let tmp = tempfile::tempdir().unwrap();
        let image_path = tmp.path().join("test.png");
        let manifest_path = tmp.path().join("manifest.json");
        let signed_manifest_path = tmp.path().join("manifest_signed.json");
        let key_dir = tmp.path().join("keys");

        create_test_png(&image_path);

        let (code, _, _) = run_cli(&["keygen", "--output-dir", key_dir.to_str().unwrap()]);
        assert_eq!(code, 0, "keygen should succeed");

        create_manifest_json(&image_path, &manifest_path, None);

        // sign
        let (code, stdout, stderr) = run_cli(&[
            "sign",
            "--manifest",
            manifest_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_private.pem").to_str().unwrap(),
            "--output",
            signed_manifest_path.to_str().unwrap(),
        ]);
        assert_eq!(code, 0, "sign should succeed: {}", stderr);
        assert!(stdout.contains("signed"), "should confirm signing");

        // verify with correct key → exit 0
        let (code, stdout, _) = run_cli(&[
            "verify-manifest",
            "--manifest",
            signed_manifest_path.to_str().unwrap(),
            "--image",
            image_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_public.pem").to_str().unwrap(),
        ]);
        assert_eq!(code, 0, "verify-manifest with correct key should exit 0");
        assert!(
            stdout.contains("TRUSTED"),
            "should report TRUSTED: {}",
            stdout
        );
    }

    // B5.5: Verification without --key → exit 4 (untrusted)
    #[test]
    fn b5_3_verify_without_key_exits_4() {
        let tmp = tempfile::tempdir().unwrap();
        let image_path = tmp.path().join("test.png");
        let manifest_path = tmp.path().join("manifest.json");
        let signed_manifest_path = tmp.path().join("manifest_signed.json");
        let key_dir = tmp.path().join("keys");

        create_test_png(&image_path);

        let (code, _, _) = run_cli(&["keygen", "--output-dir", key_dir.to_str().unwrap()]);
        assert_eq!(code, 0);

        create_manifest_json(&image_path, &manifest_path, None);

        let (code, _, _) = run_cli(&[
            "sign",
            "--manifest",
            manifest_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_private.pem").to_str().unwrap(),
            "--output",
            signed_manifest_path.to_str().unwrap(),
        ]);
        assert_eq!(code, 0);

        // verify without --key → exit 4 (untrusted)
        let (code, stdout, _) = run_cli(&[
            "verify-manifest",
            "--manifest",
            signed_manifest_path.to_str().unwrap(),
            "--image",
            image_path.to_str().unwrap(),
        ]);
        assert_eq!(
            code, 4,
            "verify-manifest without --key should exit 4 (untrusted)"
        );
        assert!(
            stdout.contains("UNTRUSTED"),
            "should report UNTRUSTED: {}",
            stdout
        );
    }

    // B5.6: Verification with wrong public key → exit 3 or 4
    #[test]
    fn b5_4_verify_with_wrong_key_exits_nonzero() {
        let tmp = tempfile::tempdir().unwrap();
        let image_path = tmp.path().join("test.png");
        let manifest_path = tmp.path().join("manifest.json");
        let signed_manifest_path = tmp.path().join("manifest_signed.json");
        let key_dir1 = tmp.path().join("keys1");
        let key_dir2 = tmp.path().join("keys2");

        create_test_png(&image_path);

        let (code, _, _) = run_cli(&["keygen", "--output-dir", key_dir1.to_str().unwrap()]);
        assert_eq!(code, 0);
        let (code, _, _) = run_cli(&["keygen", "--output-dir", key_dir2.to_str().unwrap()]);
        assert_eq!(code, 0);

        create_manifest_json(&image_path, &manifest_path, None);

        let (code, _, _) = run_cli(&[
            "sign",
            "--manifest",
            manifest_path.to_str().unwrap(),
            "--key",
            key_dir1.join("key_private.pem").to_str().unwrap(),
            "--output",
            signed_manifest_path.to_str().unwrap(),
        ]);
        assert_eq!(code, 0);

        let (code, _, _) = run_cli(&[
            "verify-manifest",
            "--manifest",
            signed_manifest_path.to_str().unwrap(),
            "--image",
            image_path.to_str().unwrap(),
            "--key",
            key_dir2.join("key_public.pem").to_str().unwrap(),
        ]);
        assert!(
            code != 0,
            "verify-manifest with wrong key should not exit 0, got {}",
            code
        );
        assert!(
            code == 3 || code == 4,
            "expected exit 3 (integrity) or 4 (trust), got {}",
            code
        );
    }

    // B5.7: Tampered manifest signature → not exit 0
    #[test]
    fn b5_5_tampered_signature_not_exit_0() {
        let tmp = tempfile::tempdir().unwrap();
        let image_path = tmp.path().join("test.png");
        let manifest_path = tmp.path().join("manifest.json");
        let signed_manifest_path = tmp.path().join("manifest_signed.json");
        let tampered_manifest_path = tmp.path().join("manifest_tampered.json");
        let key_dir = tmp.path().join("keys");

        create_test_png(&image_path);

        let (code, _, _) = run_cli(&["keygen", "--output-dir", key_dir.to_str().unwrap()]);
        assert_eq!(code, 0);

        create_manifest_json(&image_path, &manifest_path, None);

        let (code, _, _) = run_cli(&[
            "sign",
            "--manifest",
            manifest_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_private.pem").to_str().unwrap(),
            "--output",
            signed_manifest_path.to_str().unwrap(),
        ]);
        assert_eq!(code, 0);

        // tamper with the signature bytes
        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&signed_manifest_path).unwrap()).unwrap();
        if let Some(sigs) = manifest["signatures"].as_array_mut() {
            if let Some(sig) = sigs.first_mut() {
                let sig_str = sig["signature"].as_str().unwrap();
                let mut chars: Vec<char> = sig_str.chars().collect();
                let last = chars.last_mut().unwrap();
                *last = if *last == 'a' { 'b' } else { 'a' };
                sig["signature"] = serde_json::Value::String(chars.into_iter().collect());
            }
        }
        fs::write(
            &tampered_manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let (code, _, _) = run_cli(&[
            "verify-manifest",
            "--manifest",
            tampered_manifest_path.to_str().unwrap(),
            "--image",
            image_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_public.pem").to_str().unwrap(),
        ]);
        assert!(
            code != 0,
            "tampered manifest should not verify, got exit {}",
            code
        );
    }

    // B5.8: Modified image → exit 3 (binding failure)
    #[test]
    fn b5_6_modified_image_exits_3() {
        let tmp = tempfile::tempdir().unwrap();
        let image_path = tmp.path().join("test.png");
        let modified_path = tmp.path().join("modified.png");
        let manifest_path = tmp.path().join("manifest.json");
        let signed_manifest_path = tmp.path().join("manifest_signed.json");
        let key_dir = tmp.path().join("keys");

        create_test_png(&image_path);

        let img = image::DynamicImage::new_rgb8(64, 64);
        let mut rgb = img.to_rgb8();
        for y in 0..64u32 {
            for x in 0..64u32 {
                rgb.put_pixel(x, y, image::Rgb([255, 0, 0]));
            }
        }
        let dyn_img = image::DynamicImage::ImageRgb8(rgb);
        let file = fs::File::create(&modified_path).unwrap();
        let encoder = image::codecs::png::PngEncoder::new(file);
        image::ImageEncoder::write_image(
            encoder,
            &dyn_img.to_rgb8(),
            64,
            64,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();

        let (code, _, _) = run_cli(&["keygen", "--output-dir", key_dir.to_str().unwrap()]);
        assert_eq!(code, 0);

        create_manifest_json(&image_path, &manifest_path, None);

        let (code, _, _) = run_cli(&[
            "sign",
            "--manifest",
            manifest_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_private.pem").to_str().unwrap(),
            "--output",
            signed_manifest_path.to_str().unwrap(),
        ]);
        assert_eq!(code, 0);

        let (code, _, _) = run_cli(&[
            "verify-manifest",
            "--manifest",
            signed_manifest_path.to_str().unwrap(),
            "--image",
            modified_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_public.pem").to_str().unwrap(),
        ]);
        assert_eq!(
            code, 3,
            "modified image should cause binding failure (exit 3), got {}",
            code
        );
    }

    // B5.9: HMAC embedded reference without --payload-key → exit 3 or 4
    #[test]
    fn b5_7_hmac_embedded_reference_without_key_exits_3() {
        let tmp = tempfile::tempdir().unwrap();
        let image_path = tmp.path().join("test.png");
        let manifest_path = tmp.path().join("manifest.json");
        let signed_manifest_path = tmp.path().join("manifest_signed.json");
        let key_dir = tmp.path().join("keys");

        create_test_png(&image_path);

        let (code, _, _) = run_cli(&["keygen", "--output-dir", key_dir.to_str().unwrap()]);
        assert_eq!(code, 0);

        create_manifest_json(
            &image_path,
            &manifest_path,
            Some(serde_json::json!({
                "payload_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                "payload_version": 3,
            })),
        );

        let (code, _, _) = run_cli(&[
            "sign",
            "--manifest",
            manifest_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_private.pem").to_str().unwrap(),
            "--output",
            signed_manifest_path.to_str().unwrap(),
        ]);
        assert_eq!(code, 0);

        let (code, _, _) = run_cli(&[
            "verify-manifest",
            "--manifest",
            signed_manifest_path.to_str().unwrap(),
            "--image",
            image_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_public.pem").to_str().unwrap(),
        ]);
        assert!(
            code != 0,
            "HMAC embedded reference without --payload-key should fail, got exit {}",
            code
        );
        assert!(code == 3 || code == 4, "expected exit 3 or 4, got {}", code);
    }

    // B5.13: JSON and human-readable overall outcomes agree
    #[test]
    fn b5_8_json_and_human_outcomes_agree() {
        let tmp = tempfile::tempdir().unwrap();
        let image_path = tmp.path().join("test.png");
        let manifest_path = tmp.path().join("manifest.json");
        let signed_manifest_path = tmp.path().join("manifest_signed.json");
        let key_dir = tmp.path().join("keys");

        create_test_png(&image_path);

        let (code, _, _) = run_cli(&["keygen", "--output-dir", key_dir.to_str().unwrap()]);
        assert_eq!(code, 0);

        create_manifest_json(&image_path, &manifest_path, None);

        let (code, _, _) = run_cli(&[
            "sign",
            "--manifest",
            manifest_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_private.pem").to_str().unwrap(),
            "--output",
            signed_manifest_path.to_str().unwrap(),
        ]);
        assert_eq!(code, 0);

        // human-readable
        let (code_h, stdout_h, _) = run_cli(&[
            "verify-manifest",
            "--manifest",
            signed_manifest_path.to_str().unwrap(),
            "--image",
            image_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_public.pem").to_str().unwrap(),
        ]);

        // JSON
        let (code_j, stdout_j, _) = run_cli(&[
            "--json",
            "verify-manifest",
            "--manifest",
            signed_manifest_path.to_str().unwrap(),
            "--image",
            image_path.to_str().unwrap(),
            "--key",
            key_dir.join("key_public.pem").to_str().unwrap(),
        ]);

        assert_eq!(code_h, 0, "human-readable verify should exit 0");
        assert_eq!(code_j, 0, "JSON verify should exit 0");

        let json_val: serde_json::Value =
            serde_json::from_str(&stdout_j).expect("JSON output should be valid JSON");
        assert!(
            json_val.get("overall_status").is_some(),
            "JSON should have overall_status field"
        );
        assert!(
            json_val["overall_status"] == "verified_trusted"
                || json_val["overall_status"] == "VerifiedTrusted",
            "JSON overall_status should be VerifiedTrusted, got {:?}",
            json_val["overall_status"]
        );
        assert!(
            stdout_h.contains("TRUSTED"),
            "human output should say TRUSTED"
        );
    }
}
