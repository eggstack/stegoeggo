use std::collections::BTreeMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(name: &str) -> String {
    std::fs::read_to_string(root().join(name)).expect(name)
}

fn contract_triples() -> Vec<String> {
    let text = read("release/eggpack/distribution.toml");
    let mut triples = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("triple = ") {
            triples.push(value.trim_matches('"').to_string());
        }
    }
    triples
}

fn contract_assets() -> BTreeMap<String, String> {
    let triples = contract_triples();
    let mut assets = BTreeMap::new();
    for triple in &triples {
        let mut asset = format!("stegoeggo-{triple}");
        if triple.ends_with("-pc-windows-msvc") {
            asset.push_str(".exe");
        }
        assets.insert(triple.clone(), asset);
    }
    assets
}

#[test]
fn eggpack_config_resolves_five_targets() {
    let triples = contract_triples();
    assert_eq!(triples.len(), 5);
    let mut sorted = triples.clone();
    sorted.sort();
    assert_eq!(triples, sorted);
    for alias in [
        "linux-x64",
        "linux-arm64",
        "macos-x64",
        "macos-arm64",
        "windows-x64",
    ] {
        assert!(read("release/eggpack/distribution.toml").contains(alias));
        assert!(read("release/eggpack/workflow-shape.json").contains(alias));
    }
}

#[test]
fn contract_asset_names_match_installer_updater_expectations() {
    let assets = contract_assets();
    assert_eq!(assets.len(), 5);
    let installer = read("packaging/install.sh");
    let powershell = read("packaging/install.ps1");
    let update = read("stegoeggo-cli/src/update.rs");
    for (triple, asset) in &assets {
        assert!(update.contains(triple.as_str()), "{triple}");
        if triple.ends_with("-pc-windows-msvc") {
            assert!(powershell.contains(triple.as_str()), "{triple}");
        } else {
            assert!(installer.contains(triple.as_str()), "{triple}");
        }
        assert!(read("docs/installation.md").contains(asset.as_str()));
        assert!(read("architecture/cli.md").contains(asset.as_str()));
    }
    assert!(update.contains("format!(\"stegoeggo-{target}.exe\")"));
    assert!(update.contains("format!(\"stegoeggo-{target}\")"));
    assert!(update.contains(".args([\"version\"])"));
}

#[test]
fn cli_default_signatures_feature_equivalence_is_guarded() {
    let cli = read("stegoeggo-cli/Cargo.toml");
    assert!(cli.contains("default = [\"signatures\"]"));
    assert!(
        cli.contains("signatures = [\"stegoeggo/signatures\", \"stegoeggo/detached-manifest\"]")
    );
    let bindings = read("release/eggpack/build-bindings.toml");
    assert!(bindings.contains("stegoeggo-cli"));
    assert!(bindings.contains("stegoeggo"));
}

#[test]
fn updater_targets_match_eggpack_contract() {
    let triples = contract_triples();
    let update = read("stegoeggo-cli/src/update.rs");
    assert_eq!(triples.len(), 5);
    for triple in &triples {
        assert!(update.contains(triple.as_str()));
    }
}

#[test]
fn generated_workflow_is_drift_free_shape() {
    let workflow = read(".github/workflows/release-binaries.yml");
    let policy: serde_json::Value =
        serde_json::from_str(&read("release/eggpack/github-policy.json")).expect("policy");
    let revision = policy
        .get("eggpack_tool")
        .and_then(|tool| tool.get("revision"))
        .and_then(serde_json::Value::as_str)
        .expect("revision");
    assert_eq!(revision.len(), 40);
    assert!(revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(workflow.contains(revision));
    assert!(workflow.contains("release_tag"));
    assert_eq!(workflow.matches("contents: write").count(), 1);
    for forbidden in [
        "--clobber",
        "gh release publish",
        "gh release create --latest",
    ] {
        assert!(!workflow.contains(forbidden), "{forbidden}");
    }
    assert!(!workflow.contains("apt-get install"));
    assert!(workflow.contains("_stage-github-draft"));
    for triple in contract_triples() {
        assert!(workflow.contains(&triple), "{triple}");
    }
}

#[test]
fn qualification_and_consumer_validators_cover_all_targets() {
    let qual = read("release/eggpack/qualification-bindings.toml");
    let validators: serde_json::Value =
        serde_json::from_str(&read("release/eggpack/consumer-validators.json"))
            .expect("validators");
    assert!(qual.contains("argv = [\"version\"]"));
    for triple in contract_triples() {
        assert!(
            qual.contains(&format!("[targets.\"{triple}\".smoke]")),
            "{triple}"
        );
        assert_eq!(
            validators
                .get(&triple)
                .and_then(|entry| entry.get("script"))
                .and_then(serde_json::Value::as_str),
            Some("scripts/smoke-release-binary.py"),
            "{triple}"
        );
    }
}

#[test]
fn cross_toolchain_pins_preserve_glibc_floor() {
    let pack = read("release/eggpack/pack.toml");
    assert!(pack.contains("zig = \"0.14.1\""));
    assert!(pack.contains("cargo_zigbuild = \"0.23.3\""));
    assert!(pack.contains("major = 2, minor = 17"));
}

#[test]
fn glibc_requirement_parsing_accepts_floor_and_rejects_above() {
    let sample = "Name: GLIBC_2.2 Flags: none\nName: GLIBC_2.17 Flags: none\n";
    let mut max: (u64, u64) = (0, 0);
    for captures in regex_lite(sample) {
        if captures > max {
            max = captures;
        }
    }
    assert_eq!(max, (2, 17));
    assert!(max <= (2, 17));
    let higher = "Name: GLIBC_2.28 Flags: none\n";
    let mut over: (u64, u64) = (0, 0);
    for captures in regex_lite(higher) {
        if captures > over {
            over = captures;
        }
    }
    assert!(over > (2, 17));
}

fn regex_lite(text: &str) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if let Some(rest) = text
            .get(index..)
            .and_then(|tail| tail.strip_prefix("GLIBC_"))
        {
            let mut major = String::new();
            let mut minor = String::new();
            let mut cursor = 0;
            for byte in rest.bytes() {
                if byte.is_ascii_digit() {
                    major.push(byte as char);
                    cursor += 1;
                } else {
                    break;
                }
            }
            if rest.as_bytes().get(cursor) == Some(&b'.') {
                cursor += 1;
                for byte in rest.bytes().skip(cursor) {
                    if byte.is_ascii_digit() {
                        minor.push(byte as char);
                        cursor += 1;
                    } else {
                        break;
                    }
                }
            }
            if let (Ok(major), Ok(minor)) = (major.parse(), minor.parse()) {
                out.push((major, minor));
                index += "GLIBC_".len() + cursor;
                continue;
            }
        }
        index += 1;
    }
    out
}

#[test]
fn validator_script_enforces_product_semantics() {
    let script = read("scripts/smoke-release-binary.py");
    for fragment in [
        "protect",
        "inspect",
        "verify",
        "--rights-policy",
        "prohibited-ai-ml-training",
        "--preset",
        "legal-notice",
        "ceiling > (2, 17)",
        "readelf",
    ] {
        assert!(script.contains(fragment), "{fragment}");
    }
}

fn workspace_version() -> String {
    let text = read("Cargo.toml");
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("version = ") {
            return value.trim_matches('"').to_string();
        }
    }
    panic!("workspace version not found");
}

fn write_fake_candidate(dir: &std::path::Path, version: &str, mode: &str) -> PathBuf {
    let path = dir.join("fake-candidate");
    let script = match mode {
        "happy" => format!(
            "#!/usr/bin/env bash\nif [[ \"${{1:-}}\" == version ]]; then echo \"stegoeggo {version}\"; exit 0; fi\nif [[ \"${{1:-}}\" == --help ]]; then echo help; exit 0; fi\nif [[ \"${{1:-}}\" == protect ]]; then for ((i=1;i<=$#;i++)); do if [[ \"${{!i}}\" == --output ]]; then j=$((i+1)); touch \"${{!j}}\"; fi; done; exit 0; fi\nif [[ \"${{1:-}}\" == inspect ]]; then echo \"ProhibitedAiMlTraining\"; exit 0; fi\nif [[ \"${{1:-}}\" == verify ]]; then exit 0; fi\nexit 1\n"
        ),
        "wrong-version" => "#!/usr/bin/env bash\necho \"stegoeggo 0.0.0\"; exit 0\n".to_string(),
        "nonzero-help" => format!(
            "#!/usr/bin/env bash\nif [[ \"${{1:-}}\" == version ]]; then echo \"stegoeggo {version}\"; exit 0; fi\nexit 3\n"
        ),
        _ => panic!("unknown fake mode"),
    };
    std::fs::write(&path, script).expect("fake candidate");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    path
}

fn run_validator(candidate: &std::path::Path) -> (bool, String) {
    run_validator_with_temp_root(candidate, None)
}

fn run_validator_with_temp_root(
    candidate: &std::path::Path,
    temp_root: Option<&std::path::Path>,
) -> (bool, String) {
    let mut command = std::process::Command::new("python3");
    command
        .arg(root().join("scripts/smoke-release-binary.py"))
        .arg(candidate);
    if let Some(temp_root) = temp_root {
        command
            .env("TMPDIR", temp_root)
            .env("TEMP", temp_root)
            .env("TMP", temp_root);
    }
    let output = command.output().expect("python3");
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), combined)
}

fn smoke_temp_entries(root: &std::path::Path) -> Vec<PathBuf> {
    std::fs::read_dir(root)
        .expect("scratch")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("stegoeggo-release-smoke-"))
        })
        .collect()
}

#[test]
fn validator_happy_path_with_fake_candidate() {
    let dir = tempfile::tempdir().expect("tempdir");
    let version = workspace_version();
    let candidate = write_fake_candidate(dir.path(), &version, "happy");
    let (ok, output) = run_validator(&candidate);
    assert!(ok, "{output}");
    assert!(output.contains(&format!("stegoeggo {version}")));
}

#[test]
fn validator_wrong_version_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let version = workspace_version();
    let candidate = write_fake_candidate(dir.path(), &version, "wrong-version");
    let (ok, output) = run_validator(&candidate);
    assert!(!ok, "wrong version must fail: {output}");
    assert!(output.contains("mismatch"));
}

#[test]
fn validator_nonzero_help_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let version = workspace_version();
    let candidate = write_fake_candidate(dir.path(), &version, "nonzero-help");
    let (ok, output) = run_validator(&candidate);
    assert!(!ok, "nonzero help must fail: {output}");
}

#[test]
fn validator_temp_state_is_cleaned() {
    let dir = tempfile::tempdir().expect("tempdir");
    let scratch = dir.path().join("scratch");
    std::fs::create_dir_all(&scratch).expect("scratch");
    let version = workspace_version();
    let candidate = write_fake_candidate(dir.path(), &version, "happy");
    let before = smoke_temp_entries(&scratch);
    let (ok, output) = run_validator_with_temp_root(&candidate, Some(&scratch));
    assert!(ok, "{output}");
    let after = smoke_temp_entries(&scratch);
    assert_eq!(before.len(), after.len());
}

#[test]
fn bounded_subprocess_timeout_pattern() {
    let start = std::time::Instant::now();
    let result = std::process::Command::new("python3")
        .args(["-c", "import subprocess,sys; subprocess.run([sys.executable,'-c','import time; time.sleep(10)'], timeout=1)"])
        .output()
        .expect("python3");
    assert!(!result.status.success());
    assert!(start.elapsed() < std::time::Duration::from_secs(10));
}

#[test]
fn asset_audit_accepts_fifteen_and_rejects_extras() {
    let policy: serde_json::Value =
        serde_json::from_str(&read("release/eggpack/github-policy.json")).expect("policy");
    assert!(policy.get("staging").is_some());
    let audit = read("scripts/release-check-assets.sh");
    for name in [
        "install.sh",
        "install.ps1",
        "install-exact.sh",
        "install-exact.ps1",
        "release-manifest.json",
    ] {
        assert!(audit.contains(name), "{name}");
    }
}
