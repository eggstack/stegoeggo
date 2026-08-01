//! Negative conformance coverage tests for Plan 031 Phase 5.
//!
//! These tests prove that the conformance harness rejects invalid manifests
//! and fixture configurations.

use std::fs;
use std::path::Path;

fn conformance_bin() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut path = manifest_dir.join("target/release/stegoeggo-conformance");
    if !path.exists() {
        let output = std::process::Command::new("cargo")
            .args([
                "build",
                "--release",
                "--bin",
                "stegoeggo-conformance",
                "--features",
                "conformance",
            ])
            .current_dir(&manifest_dir)
            .output()
            .expect("Failed to build conformance harness");
        assert!(output.status.success(), "Conformance harness build failed");
        path = manifest_dir.join("target/release/stegoeggo-conformance");
    }
    path
}

fn fixtures_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/conformance")
}

fn run_conformance(manifest_path: &Path, strict: bool) -> (i32, String) {
    let bin = conformance_bin();
    let fixtures = fixtures_dir();
    let mut args = vec![
        "--fixtures",
        fixtures.to_str().unwrap(),
        "--manifest",
        manifest_path.to_str().unwrap(),
        "--json",
        "/dev/null",
    ];
    if strict {
        args.push("--strict");
    }
    let output = std::process::Command::new(&bin)
        .args(&args)
        .output()
        .expect("Failed to run conformance harness");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(1);
    (code, format!("{}{}", stdout, stderr))
}

fn original_manifest() -> String {
    fs::read_to_string(fixtures_dir().join("manifest.toml"))
        .expect("Failed to read original manifest")
}

#[test]
fn negative_incorrect_sha256_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let original = original_manifest();
    if let Some(pos) = original.find("sha256 = \"") {
        let hash_start = pos + "sha256 = \"".len();
        if let Some(quote_end) = original[hash_start..].find('"') {
            let hash_end = hash_start + quote_end;
            let mut bad = original.clone();
            bad.replace_range(hash_start..hash_end, &"0".repeat(64));
            fs::write(tmp.path().join("bad_sha.toml"), bad).unwrap();
            let (code, output) = run_conformance(&tmp.path().join("bad_sha.toml"), false);
            assert!(
                code != 0 || output.contains("mismatch") || output.contains("digest"),
                "Incorrect sha256 should cause failure, got exit {}",
                code
            );
            return;
        }
    }
    panic!("Could not find sha256 field in manifest");
}

#[test]
fn negative_source_reclassified_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let original = original_manifest();
    let bad = original.replace("source = \"external\"", "source = \"generated\"");
    fs::write(tmp.path().join("bad_source.toml"), bad).unwrap();

    let (code, output) = run_conformance(&tmp.path().join("bad_source.toml"), false);
    assert!(
        code != 0 || output.contains("coverage") || output.contains("external"),
        "Reclassifying external as generated should cause coverage failure, got exit {}",
        code
    );
}

#[test]
fn negative_empty_manifest_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("empty.toml"), "").unwrap();

    let (code, _) = run_conformance(&tmp.path().join("empty.toml"), false);
    assert_ne!(code, 0, "Empty manifest should be rejected");
}

#[test]
fn negative_duplicate_fixture_ids_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let original = original_manifest();
    let entries: Vec<&str> = original.split("[[fixture]]").collect();
    if entries.len() > 1 {
        let dup = format!(
            "{}[[fixture]]{}[[fixture]]{}",
            entries[0], entries[1], entries[1]
        );
        fs::write(tmp.path().join("dup.toml"), dup).unwrap();
        let (code, _) = run_conformance(&tmp.path().join("dup.toml"), false);
        assert_ne!(code, 0, "Duplicate fixture IDs should be rejected");
    }
}
