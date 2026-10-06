use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn cli_bin() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_BIN_EXE_stegoeggo"));
    if !path.exists() {
        let output = Command::new("cargo")
            .args(["build", "-p", "stegoeggo-cli"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("Failed to build CLI");
        assert!(output.status.success(), "CLI build failed");
        path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/stegoeggo");
    }
    path
}

fn create_test_png(path: &Path) {
    let img = image::DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(64, 64, |x, y| {
        image::Rgb([
            ((x * 7 + y * 3) % 256) as u8,
            ((x * 11 + y * 5) % 256) as u8,
            ((x * 13 + y * 9) % 256) as u8,
        ])
    }));
    let file = fs::File::create(path).unwrap();
    image::ImageEncoder::write_image(
        image::codecs::png::PngEncoder::new(file),
        &img.to_rgb8(),
        64,
        64,
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
}

fn protect(args: &[&std::ffi::OsStr]) -> std::process::Output {
    Command::new(cli_bin())
        .args(args)
        .output()
        .expect("run CLI")
}

#[test]
fn batch_output_naming_a_file_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a.png");
    let b = tmp.path().join("b.png");
    create_test_png(&a);
    create_test_png(&b);
    let requested_file = tmp.path().join("out.png");

    let result = protect(&[
        a.as_os_str(),
        b.as_os_str(),
        "-o".as_ref(),
        requested_file.as_os_str(),
    ]);

    assert!(
        !result.status.success(),
        "a batch run cannot write one file per input into a single file path"
    );
    assert!(
        !requested_file.exists(),
        "the rejected output path must not be created as a directory or a file"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("batch"),
        "the error should explain the batch/directory mismatch, got: {stderr}"
    );
}

#[test]
fn batch_output_naming_an_existing_file_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a.png");
    let b = tmp.path().join("b.png");
    create_test_png(&a);
    create_test_png(&b);
    let existing = tmp.path().join("already.png");
    fs::write(&existing, b"not an image").unwrap();

    let result = protect(&[
        a.as_os_str(),
        b.as_os_str(),
        "-o".as_ref(),
        existing.as_os_str(),
    ]);

    assert!(!result.status.success());
    assert_eq!(
        fs::read(&existing).unwrap(),
        b"not an image".to_vec(),
        "the pre-existing file must be left untouched"
    );
}

#[test]
fn batch_output_directory_is_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a.png");
    let b = tmp.path().join("b.png");
    create_test_png(&a);
    create_test_png(&b);
    let out = tmp.path().join("out");

    let result = protect(&[a.as_os_str(), b.as_os_str(), "-o".as_ref(), out.as_os_str()]);

    assert!(
        result.status.success(),
        "directory output must keep working: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(out.join("a_protected.png").is_file());
    assert!(out.join("b_protected.png").is_file());
}

#[test]
fn single_file_output_with_a_non_image_extension_is_written_as_a_file() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("a.png");
    create_test_png(&input);
    let requested = tmp.path().join("report.json");

    let result = protect(&[input.as_os_str(), "-o".as_ref(), requested.as_os_str()]);

    assert!(
        result.status.success(),
        "a mistyped extension must not silently become a directory: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        requested.is_file(),
        "the requested output path must be the protected image itself"
    );
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert_eq!(stdout.trim(), requested.display().to_string());
}

#[test]
fn single_file_output_without_an_extension_is_still_a_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("a.png");
    create_test_png(&input);
    let out = tmp.path().join("out");

    let result = protect(&[input.as_os_str(), "-o".as_ref(), out.as_os_str()]);

    assert!(result.status.success());
    assert!(out.is_dir());
    assert!(out.join("a_protected.png").is_file());
}

#[cfg(unix)]
#[test]
fn written_output_follows_the_process_umask() {
    use std::os::unix::fs::PermissionsExt as _;

    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("a.png");
    create_test_png(&input);
    let written = tmp.path().join("written.png");
    let reference = tmp.path().join("reference.png");

    let result = protect(&[input.as_os_str(), "-o".as_ref(), written.as_os_str()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    // A plain create in the same directory is the reference for the umask.
    fs::File::create(&reference).unwrap();
    let expected = fs::metadata(&reference).unwrap().permissions().mode() & 0o777;
    let actual = fs::metadata(&written).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        actual, expected,
        "protected output must be as readable as a normally created file"
    );
    assert_ne!(actual, 0o600, "output must not be forced to owner-only");
}

#[cfg(unix)]
#[test]
fn overwriting_preserves_the_existing_mode() {
    use std::os::unix::fs::PermissionsExt as _;

    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("a.png");
    create_test_png(&input);
    let target = tmp.path().join("existing.png");
    fs::write(&target, b"old").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();

    let result = protect(&[input.as_os_str(), "-o".as_ref(), target.as_os_str()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o644, "an overwritten file keeps its own mode");
}

#[test]
fn batch_json_emits_one_parseable_document() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a.png");
    let b = tmp.path().join("b.png");
    create_test_png(&a);
    create_test_png(&b);
    let out = tmp.path().join("out");

    let result = protect(&[
        a.as_os_str(),
        b.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
        "--json".as_ref(),
    ]);

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8_lossy(&result.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("batch --json stdout must be JSON ({e}), got: {stdout}"));
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["status"], "ok");

    let files = json["files"]
        .as_array()
        .expect("batch --json must carry a files array");
    assert_eq!(files.len(), 2, "one entry per input: {stdout}");
    for entry in files {
        assert_eq!(entry["status"], "ok");
        let output = entry["output_path"].as_str().expect("output_path");
        assert!(
            Path::new(output).is_file(),
            "reported output {output} must exist"
        );
    }
}

#[test]
fn batch_json_records_per_file_failures() {
    let tmp = tempfile::tempdir().unwrap();
    let good = tmp.path().join("good.png");
    create_test_png(&good);
    let bad = tmp.path().join("bad.png");
    fs::write(&bad, b"\x89PNG\r\n\x1a\n").unwrap();
    let out = tmp.path().join("out");

    let result = protect(&[
        good.as_os_str(),
        bad.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
        "--json".as_ref(),
    ]);

    let stdout = String::from_utf8_lossy(&result.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("stdout must be JSON ({e})"));
    assert_eq!(json["status"], "error");
    let files = json["files"].as_array().expect("files array");
    assert_eq!(files.len(), 2);
    let failed = files
        .iter()
        .find(|f| f["status"] == "error")
        .expect("the corrupt input must be reported");
    assert!(
        failed["error"].as_str().is_some_and(|e| !e.is_empty()),
        "a failed entry must carry an error string"
    );
}

#[cfg(feature = "signatures")]
#[test]
fn keygen_refuses_to_destroy_an_existing_private_key() {
    let tmp = tempfile::tempdir().unwrap();
    let key_dir = tmp.path().join("keys");
    let keygen = || {
        Command::new(cli_bin())
            .arg("keygen")
            .arg("--output-dir")
            .arg(&key_dir)
            .output()
            .expect("run CLI")
    };

    let first = keygen();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let private_path = key_dir.join("key_private.pem");
    let before = fs::read(&private_path).unwrap();

    let second = keygen();
    assert!(
        !second.status.success(),
        "a second keygen must not report success after refusing"
    );
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("refusing to overwrite"),
        "the error must explain the refusal, got: {stderr}"
    );
    assert!(
        !String::from_utf8_lossy(&second.stdout).contains("Key pair generated"),
        "no success banner may be printed when nothing was written"
    );
    assert_eq!(
        fs::read(&private_path).unwrap(),
        before,
        "the existing private key must be byte-identical"
    );
}

#[test]
fn jobs_flag_runs_the_batch_and_matches_serial_output() {
    let tmp = tempfile::tempdir().unwrap();
    let inputs: Vec<PathBuf> = (0..4)
        .map(|i| tmp.path().join(format!("in{i}.png")))
        .collect();
    for input in &inputs {
        create_test_png(input);
    }

    let run_with_jobs = |out: &Path, jobs: &str| {
        let mut args: Vec<&std::ffi::OsStr> = inputs.iter().map(|p| p.as_os_str()).collect();
        args.push("-o".as_ref());
        args.push(out.as_os_str());
        args.push("-j".as_ref());
        args.push(OsStr::new(jobs));
        args.push("-s".as_ref());
        args.push(OsStr::new("42"));
        protect(&args)
    };

    let serial_out = tmp.path().join("serial");
    let serial = run_with_jobs(&serial_out, "1");
    let parallel_out = tmp.path().join("parallel");
    let parallel = run_with_jobs(&parallel_out, "4");

    assert!(
        serial.status.success(),
        "{}",
        String::from_utf8_lossy(&serial.stderr)
    );
    assert!(
        parallel.status.success(),
        "{}",
        String::from_utf8_lossy(&parallel.stderr)
    );

    for input in &inputs {
        let name = format!(
            "{}_protected.png",
            input.file_stem().unwrap().to_string_lossy()
        );
        assert_eq!(
            fs::read(serial_out.join(&name)).unwrap(),
            fs::read(parallel_out.join(&name)).unwrap(),
            "{name} must not depend on the worker count"
        );
    }
}
