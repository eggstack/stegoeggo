use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const CRATES_API_URL: &str = "https://crates.io/api/v1/crates/stegoeggo-cli";
const RELEASES_URL: &str = "https://github.com/eggstack/stegoeggo/releases";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_COMMAND_OUTPUT: usize = 16 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(60);
const REDIRECT_MAX: usize = 10;
const REGISTRY_BODY_LIMIT: usize = 4 * 1024 * 1024;
const SIDECAR_BODY_LIMIT: usize = 8 * 1024;
const EXECUTABLE_BODY_LIMIT: usize = 64 * 1024 * 1024;

struct TargetSpec {
    os: &'static str,
    arch: &'static str,
    triple: &'static str,
}

const TARGETS: &[TargetSpec] = &[
    TargetSpec {
        os: "linux",
        arch: "x86_64",
        triple: "x86_64-unknown-linux-gnu",
    },
    TargetSpec {
        os: "linux",
        arch: "aarch64",
        triple: "aarch64-unknown-linux-gnu",
    },
    TargetSpec {
        os: "macos",
        arch: "x86_64",
        triple: "x86_64-apple-darwin",
    },
    TargetSpec {
        os: "macos",
        arch: "aarch64",
        triple: "aarch64-apple-darwin",
    },
    TargetSpec {
        os: "windows",
        arch: "x86_64",
        triple: "x86_64-pc-windows-msvc",
    },
];

#[derive(Debug)]
enum UpdateError {
    InvalidVersion(String),
    InvalidRegistryResponse(String),
    NoStableRelease,
    RequestBuild(String),
    Proxy(String),
    Transport(String),
    HttpStatus { url: String, status: u16 },
    BodyTooLarge { url: String, limit: usize },
    Io(io::Error),
    CommandFailed { command: String, detail: String },
    CommandTimedOut(String),
    Checksum(String),
    Candidate(String),
    Destination { path: PathBuf, detail: String },
    Replacement(io::Error),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVersion(value) => write!(f, "invalid stable version '{value}'"),
            Self::InvalidRegistryResponse(detail) => {
                write!(f, "invalid crates.io version response: {detail}")
            }
            Self::NoStableRelease => {
                write!(f, "crates.io reported no stable stegoeggo-cli release")
            }
            Self::RequestBuild(detail) => write!(f, "update request failed: {detail}"),
            Self::Proxy(detail) => write!(f, "update proxy configuration failed: {detail}"),
            Self::Transport(detail) => write!(f, "update transport failed: {detail}"),
            Self::HttpStatus { url, status } => {
                write!(f, "request to {url} returned HTTP {status}")
            }
            Self::BodyTooLarge { url, limit } => {
                write!(f, "response from {url} exceeded {limit} byte limit")
            }
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::CommandFailed { command, detail } => write!(f, "{command} failed: {detail}"),
            Self::CommandTimedOut(command) => write!(f, "{command} exceeded its timeout"),
            Self::Checksum(detail) => write!(f, "checksum verification failed: {detail}"),
            Self::Candidate(detail) => write!(f, "candidate validation failed: {detail}"),
            Self::Destination { path, detail } => {
                write!(
                    f,
                    "cannot replace executable '{}': {detail}",
                    path.display()
                )
            }
            Self::Replacement(error) => write!(f, "executable replacement failed: {error}"),
        }
    }
}

impl std::error::Error for UpdateError {}

impl From<io::Error> for UpdateError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct StableVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl fmt::Display for StableVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn parse_stable_version(value: &str) -> Result<StableVersion, UpdateError> {
    let mut components = value.split('.');
    let mut parse_component = || {
        let component = components
            .next()
            .ok_or_else(|| UpdateError::InvalidVersion(value.to_string()))?;
        if component.is_empty() || (component.len() > 1 && component.starts_with('0')) {
            return Err(UpdateError::InvalidVersion(value.to_string()));
        }
        component
            .parse()
            .map_err(|_| UpdateError::InvalidVersion(value.to_string()))
    };
    let major = parse_component()?;
    let minor = parse_component()?;
    let patch = parse_component()?;
    if components.next().is_some() {
        return Err(UpdateError::InvalidVersion(value.to_string()));
    }
    Ok(StableVersion {
        major,
        minor,
        patch,
    })
}

fn latest_stable_version_from_json(body: &[u8]) -> Result<StableVersion, UpdateError> {
    let value: serde_json::Value = serde_json::from_slice(body)
        .map_err(|error| UpdateError::InvalidRegistryResponse(error.to_string()))?;
    let versions = value
        .get("versions")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| UpdateError::InvalidRegistryResponse("missing versions array".into()))?;
    let mut latest = None;
    for version in versions {
        if version
            .get("yanked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        let Some(number) = version.get("num").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Ok(candidate) = parse_stable_version(number) else {
            continue;
        };
        if latest.is_none_or(|current| candidate > current) {
            latest = Some(candidate);
        }
    }
    latest.ok_or(UpdateError::NoStableRelease)
}

fn release_target() -> Option<&'static str> {
    TARGETS
        .iter()
        .find(|target| target.os == std::env::consts::OS && target.arch == std::env::consts::ARCH)
        .map(|target| target.triple)
}

fn asset_name(target: &str) -> String {
    if target.ends_with("-pc-windows-msvc") {
        format!("stegoeggo-{target}.exe")
    } else {
        format!("stegoeggo-{target}")
    }
}

fn checksum_name(asset: &str) -> String {
    format!("{asset}.sha256")
}

fn current_executable() -> Result<PathBuf, UpdateError> {
    let reported = std::env::current_exe()?;
    fs::canonicalize(&reported).map_err(UpdateError::Io)
}

fn ensure_replaceable(path: &Path) -> Result<(), UpdateError> {
    let metadata = fs::metadata(path).map_err(|error| UpdateError::Destination {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    if metadata.permissions().readonly() {
        return Err(UpdateError::Destination {
            path: path.to_path_buf(),
            detail: "the executable is read-only; rerun with appropriate privileges or use the bootstrap installer".into(),
        });
    }
    let parent = path.parent().ok_or_else(|| UpdateError::Destination {
        path: path.to_path_buf(),
        detail: "the executable has no parent directory".into(),
    })?;
    tempfile::NamedTempFile::new_in(parent).map_err(|error| UpdateError::Destination {
        path: path.to_path_buf(),
        detail: format!(
            "the destination directory is not writable; rerun with appropriate privileges or use the bootstrap installer ({error})"
        ),
    })?;
    Ok(())
}

fn read_limited<R: Read>(mut reader: R) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                if output.len() < MAX_COMMAND_OUTPUT {
                    let remaining = MAX_COMMAND_OUTPUT - output.len();
                    output.extend_from_slice(&buffer[..read.min(remaining)]);
                }
            }
            Err(_) => break,
        }
    }
    output
}

fn run_bounded(mut command: Command, label: &str) -> Result<Output, UpdateError> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| UpdateError::CommandFailed {
            command: label.to_string(),
            detail: error.to_string(),
        })?;
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");
    let stdout_thread = thread::spawn(move || read_limited(stdout));
    let stderr_thread = thread::spawn(move || read_limited(stderr));
    let deadline = Instant::now() + COMMAND_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().map_err(UpdateError::Io)? {
            let stdout = stdout_thread.join().unwrap_or_default();
            let stderr = stderr_thread.join().unwrap_or_default();
            return Ok(Output {
                status,
                stdout,
                stderr,
            });
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Err(UpdateError::CommandTimedOut(label.to_string()));
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn output_detail(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        stderr
    }
}

fn build_client() -> Result<eggfetch_core::Client, UpdateError> {
    build_client_with_env(
        &eggfetch_core::ProxyEnvironment::from_env(),
        CONNECT_TIMEOUT,
        TOTAL_TIMEOUT,
    )
}

fn build_client_with_env(
    env: &eggfetch_core::ProxyEnvironment,
    connect: Duration,
    total: Duration,
) -> Result<eggfetch_core::Client, UpdateError> {
    let timeout = eggfetch_core::Timeout::builder()
        .connect(connect)
        .total(total)
        .build();
    let redirect = eggfetch_core::RedirectPolicy::strict(REDIRECT_MAX);
    let builder = eggfetch_core::Client::builder()
        .user_agent(concat!("stegoeggo-cli/", env!("CARGO_PKG_VERSION")))
        .timeout(timeout)
        .redirect_policy(redirect)
        .max_decoded_body_size(EXECUTABLE_BODY_LIMIT)
        .automatic_decompression(false);
    let builder = builder
        .proxy_environment(env)
        .map_err(|error| UpdateError::Proxy(error.to_string()))?;
    Ok(builder.build())
}

fn map_fetch_error(url: &str, error: eggfetch_core::Error, limit: usize) -> UpdateError {
    match error {
        eggfetch_core::Error::DecodedBodyTooLarge => UpdateError::BodyTooLarge {
            url: url.to_string(),
            limit,
        },
        eggfetch_core::Error::InvalidProxyUrl(_) => UpdateError::Proxy(error.to_string()),
        eggfetch_core::Error::InvalidUrl(_)
        | eggfetch_core::Error::InvalidMethod(_)
        | eggfetch_core::Error::InvalidHeaderName(_)
        | eggfetch_core::Error::InvalidHeaderValue(_)
        | eggfetch_core::Error::RequestBuild(_) => UpdateError::RequestBuild(error.to_string()),
        other => UpdateError::Transport(other.to_string()),
    }
}

async fn fetch_to_file(
    client: &eggfetch_core::Client,
    url: &str,
    destination: &Path,
    max_bytes: usize,
) -> Result<u16, UpdateError> {
    let mut response = client
        .get(url)
        .map_err(|error| UpdateError::RequestBuild(error.to_string()))?
        .max_decoded_body_size(max_bytes)
        .send()
        .await
        .map_err(|error| map_fetch_error(url, error, max_bytes))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Ok(status);
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| map_fetch_error(url, error, max_bytes))?;
    fs::write(destination, &bytes).map_err(UpdateError::Io)?;
    Ok(status)
}

async fn download_required(
    client: &eggfetch_core::Client,
    url: &str,
    destination: &Path,
    max_bytes: usize,
) -> Result<(), UpdateError> {
    let status = fetch_to_file(client, url, destination, max_bytes).await?;
    if !(200..300).contains(&status) {
        return Err(UpdateError::HttpStatus {
            url: url.to_string(),
            status,
        });
    }
    Ok(())
}

fn parse_checksum(body: &[u8]) -> Result<[u8; 32], UpdateError> {
    let token = String::from_utf8_lossy(body)
        .split_whitespace()
        .next()
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| UpdateError::Checksum("sidecar is empty".into()))?;
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(UpdateError::Checksum(
            "sidecar does not contain a SHA-256 digest".into(),
        ));
    }
    let bytes = hex::decode(token).map_err(|error| UpdateError::Checksum(error.to_string()))?;
    bytes
        .try_into()
        .map_err(|_| UpdateError::Checksum("sidecar digest has the wrong length".into()))
}

fn verify_checksum(binary: &Path, sidecar: &Path) -> Result<(), UpdateError> {
    let expected = parse_checksum(&fs::read(sidecar).map_err(UpdateError::Io)?)?;
    let bytes = fs::read(binary).map_err(UpdateError::Io)?;
    let actual = Sha256::digest(bytes);
    if actual[..] != expected[..] {
        return Err(UpdateError::Checksum(format!(
            "digest mismatch for {}",
            binary.display()
        )));
    }
    Ok(())
}

fn candidate_version(path: &Path) -> Result<StableVersion, UpdateError> {
    let mut command = Command::new(path);
    command.arg("version");
    let output = run_bounded(command, "candidate version")?;
    if !output.status.success() {
        return Err(UpdateError::Candidate(format!(
            "candidate exited unsuccessfully: {}",
            output_detail(&output)
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout
        .lines()
        .next()
        .unwrap_or_default()
        .trim_end_matches('\r');
    let Some(version) = first_line.strip_prefix("stegoeggo ") else {
        return Err(UpdateError::Candidate(format!(
            "expected 'stegoeggo X.Y.Z', got '{first_line}'"
        )));
    };
    parse_stable_version(version).map_err(|_| {
        UpdateError::Candidate(format!("candidate reported an invalid version '{version}'"))
    })
}

fn release_base_url() -> String {
    std::env::var("STEGOEGGO_RELEASES_URL").unwrap_or_else(|_| RELEASES_URL.to_string())
}

fn registry_url() -> String {
    std::env::var("STEGOEGGO_CRATES_API_URL").unwrap_or_else(|_| CRATES_API_URL.to_string())
}

fn cargo_fallback(version: StableVersion) -> Result<(), UpdateError> {
    let mut probe = Command::new("cargo");
    probe.arg("--version");
    let output = run_bounded(probe, "cargo --version")?;
    if !output.status.success() {
        return Err(UpdateError::CommandFailed {
            command: "cargo --version".into(),
            detail: output_detail(&output),
        });
    }
    eprintln!("No compatible prebuilt binary was found; falling back to Cargo.");
    let mut command = Command::new("cargo");
    command.args(["install", "stegoeggo-cli", "--locked", "--version"]);
    command.arg(format!("={version}"));
    let output = run_bounded(command, "cargo install stegoeggo-cli")?;
    if !output.status.success() {
        return Err(UpdateError::CommandFailed {
            command: "cargo install stegoeggo-cli".into(),
            detail: output_detail(&output),
        });
    }
    println!("Cargo installed stegoeggo {version}.");
    Ok(())
}

fn fallback_allowed(status: Option<u16>) -> bool {
    status == Some(404)
}

async fn update_to(
    client: &eggfetch_core::Client,
    current: StableVersion,
    latest: StableVersion,
) -> Result<(), UpdateError> {
    if current >= latest {
        println!("stegoeggo {current} is up to date (latest stable {latest}).");
        return Ok(());
    }

    let executable = current_executable()?;
    ensure_replaceable(&executable)?;
    let Some(target) = release_target() else {
        return cargo_fallback(latest);
    };

    let asset = asset_name(target);
    let sidecar = checksum_name(&asset);
    let temp_dir = tempfile::tempdir()?;
    let candidate = temp_dir.path().join(&asset);
    let checksum = temp_dir.path().join(&sidecar);
    let base = release_base_url().trim_end_matches('/').to_string();
    let asset_url = format!("{base}/download/v{latest}/{asset}");
    let checksum_url = format!("{base}/download/v{latest}/{sidecar}");

    eprintln!("Latest stable: {latest}");
    eprintln!("Downloading verified release asset...");
    let asset_status = fetch_to_file(client, &asset_url, &candidate, EXECUTABLE_BODY_LIMIT).await?;
    if fallback_allowed(Some(asset_status)) {
        return cargo_fallback(latest);
    }
    if !(200..300).contains(&asset_status) {
        return Err(UpdateError::HttpStatus {
            url: asset_url,
            status: asset_status,
        });
    }
    download_required(client, &checksum_url, &checksum, SIDECAR_BODY_LIMIT).await?;
    verify_checksum(&candidate, &checksum)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&candidate)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&candidate, permissions)?;
    }

    let reported = candidate_version(&candidate)?;
    if reported != latest {
        return Err(UpdateError::Candidate(format!(
            "candidate reported {reported}, expected {latest}"
        )));
    }
    self_replace::self_replace(&candidate).map_err(UpdateError::Replacement)?;
    println!("Updated {current} -> {latest}.");
    Ok(())
}

async fn run_update_async() -> Result<(), Box<dyn std::error::Error>> {
    let client = build_client()?;
    let current = parse_stable_version(env!("CARGO_PKG_VERSION"))?;
    let temp_dir = tempfile::tempdir()?;
    let registry = temp_dir.path().join("registry.json");
    download_required(&client, &registry_url(), &registry, REGISTRY_BODY_LIMIT).await?;
    let latest = latest_stable_version_from_json(&fs::read(registry)?)?;
    update_to(&client, current, latest)
        .await
        .map_err(Into::into)
}

pub(crate) fn run_update() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            UpdateError::Transport(format!("failed to start update runtime: {error}"))
        })?;
    runtime.block_on(run_update_async())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn block_on<F>(future: F) -> F::Output
    where
        F: std::future::Future,
    {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime")
            .block_on(future)
    }

    fn test_client() -> eggfetch_core::Client {
        build_client_with_env(
            &eggfetch_core::ProxyEnvironment::new(),
            Duration::from_secs(10),
            Duration::from_secs(60),
        )
        .expect("test client")
    }

    fn http_response(status: u16, reason: &str, extra: &str, body: &[u8]) -> Vec<u8> {
        let mut out = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n",
            body.len()
        )
        .into_bytes();
        out.extend_from_slice(body);
        out
    }

    fn read_request(stream: &mut std::net::TcpStream) -> String {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut request = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    request.extend_from_slice(&chunk[..n]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                    if request.len() > 16384 {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        String::from_utf8_lossy(&request).into_owned()
    }

    fn spawn_single(response: Vec<u8>) -> String {
        spawn_single_with_delay(response, None)
    }

    fn spawn_single_with_delay(response: Vec<u8>, delay: Option<Duration>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        std::thread::spawn(move || {
            for _ in 0..16 {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let _ = read_request(&mut stream);
                if let Some(delay) = delay {
                    std::thread::sleep(delay);
                }
                if stream.write_all(&response).is_err() {
                    break;
                }
            }
        });
        format!("http://{addr}")
    }

    fn spawn_redirect_server(final_body: &[u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let final_body = final_body.to_vec();
        std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let request = read_request(&mut stream);
                let path = request
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .to_string();
                if path == "/redirect" {
                    let location = format!("http://{addr}/final");
                    let response = format!(
                        "HTTP/1.1 302 Found\r\nContent-Length: 0\r\nConnection: close\r\nLocation: {location}\r\n\r\n"
                    );
                    let _ = stream.write_all(response.as_bytes());
                } else {
                    let response = http_response(200, "OK", "", &final_body);
                    let _ = stream.write_all(&response);
                }
            }
        });
        format!("http://{addr}")
    }

    #[test]
    fn stable_version_parser_rejects_prerelease_when_required() {
        assert!(parse_stable_version("0.4.1-rc.1").is_err());
        assert!(parse_stable_version("0.4.1+build").is_err());
        assert!(parse_stable_version("0.4.01").is_err());
    }

    #[test]
    fn target_mapping_matches_release_contract() {
        let expected = [
            ("linux", "x86_64", "x86_64-unknown-linux-gnu"),
            ("linux", "aarch64", "aarch64-unknown-linux-gnu"),
            ("macos", "x86_64", "x86_64-apple-darwin"),
            ("macos", "aarch64", "aarch64-apple-darwin"),
            ("windows", "x86_64", "x86_64-pc-windows-msvc"),
        ];
        assert_eq!(TARGETS.len(), expected.len());
        for (target, (os, arch, triple)) in TARGETS.iter().zip(expected) {
            assert_eq!((target.os, target.arch, target.triple), (os, arch, triple));
        }
    }

    #[test]
    fn asset_name_matches_installer_contract() {
        assert_eq!(
            asset_name("x86_64-unknown-linux-gnu"),
            "stegoeggo-x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            asset_name("x86_64-pc-windows-msvc"),
            "stegoeggo-x86_64-pc-windows-msvc.exe"
        );
        assert_eq!(
            checksum_name("stegoeggo-aarch64-apple-darwin"),
            "stegoeggo-aarch64-apple-darwin.sha256"
        );
    }

    #[test]
    fn already_current_is_noop() {
        let current = parse_stable_version("0.4.1").unwrap();
        let latest = parse_stable_version("0.4.1").unwrap();
        assert!(current >= latest);
    }

    #[test]
    fn newer_stable_resolves_available() {
        let body = br#"{"versions":[{"num":"0.4.1-rc.1","yanked":false},{"num":"0.4.0","yanked":false},{"num":"0.4.2","yanked":true},{"num":"0.4.1","yanked":false}]}"#;
        assert_eq!(
            latest_stable_version_from_json(body).unwrap().to_string(),
            "0.4.1"
        );
    }

    #[test]
    fn checksum_mismatch_is_fatal() {
        let directory = tempfile::tempdir().unwrap();
        let binary = directory.path().join("candidate");
        let sidecar = directory.path().join("candidate.sha256");
        fs::write(&binary, b"candidate").unwrap();
        fs::write(&sidecar, format!("{}  candidate\n", "0".repeat(64))).unwrap();
        assert!(matches!(
            verify_checksum(&binary, &sidecar),
            Err(UpdateError::Checksum(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn candidate_wrong_program_is_fatal() {
        let directory = tempfile::tempdir().unwrap();
        let candidate = directory.path().join("candidate");
        fs::write(&candidate, b"#!/bin/sh\nprintf 'other 0.4.1\\n'\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&candidate, fs::Permissions::from_mode(0o755)).unwrap();
        let error = candidate_version(&candidate).unwrap_err();
        assert!(error.to_string().contains("expected 'stegoeggo X.Y.Z'"));
    }

    #[cfg(unix)]
    #[test]
    fn candidate_wrong_version_is_fatal() {
        let directory = tempfile::tempdir().unwrap();
        let candidate = directory.path().join("candidate");
        fs::write(&candidate, b"#!/bin/sh\nprintf 'stegoeggo 0.4.0\\n'\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&candidate, fs::Permissions::from_mode(0o755)).unwrap();
        let reported = candidate_version(&candidate).unwrap();
        assert_ne!(reported, parse_stable_version("0.4.1").unwrap());
    }

    #[test]
    fn asset_404_allows_cargo_fallback() {
        assert!(fallback_allowed(Some(404)));
    }

    #[test]
    fn network_failure_does_not_allow_fallback() {
        assert!(!fallback_allowed(None));
        assert!(!fallback_allowed(Some(500)));
    }

    #[test]
    fn unwritable_executable_fails_before_download() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("missing-parent").join("stegoeggo");
        let error = ensure_replaceable(&executable).unwrap_err();
        assert!(error
            .to_string()
            .contains(&executable.display().to_string()));
    }

    #[test]
    fn native_registry_200_resolves_latest() {
        let body =
            br#"{"versions":[{"num":"0.4.0","yanked":false},{"num":"0.4.1","yanked":false}]}"#;
        let base = spawn_single(http_response(200, "OK", "", body));
        let client = test_client();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("registry.json");
        let status = block_on(fetch_to_file(
            &client,
            &format!("{base}/api"),
            &destination,
            REGISTRY_BODY_LIMIT,
        ))
        .unwrap();
        assert_eq!(status, 200);
        let latest = latest_stable_version_from_json(&fs::read(&destination).unwrap()).unwrap();
        assert_eq!(latest.to_string(), "0.4.1");
    }

    #[test]
    fn native_asset_200_is_written() {
        let base = spawn_single(http_response(200, "OK", "", b"asset-bytes"));
        let client = test_client();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("asset");
        let status = block_on(fetch_to_file(
            &client,
            &format!("{base}/asset"),
            &destination,
            EXECUTABLE_BODY_LIMIT,
        ))
        .unwrap();
        assert_eq!(status, 200);
        assert_eq!(fs::read(&destination).unwrap(), b"asset-bytes");
    }

    #[test]
    fn native_sidecar_200_is_written() {
        let base = spawn_single(http_response(200, "OK", "", b"abcd  asset\n"));
        let client = test_client();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("asset.sha256");
        block_on(download_required(
            &client,
            &format!("{base}/asset.sha256"),
            &destination,
            SIDECAR_BODY_LIMIT,
        ))
        .unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"abcd  asset\n");
    }

    #[test]
    fn native_asset_404_allows_cargo_fallback() {
        let base = spawn_single(http_response(404, "Not Found", "", b"missing"));
        let client = test_client();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("asset");
        let status = block_on(fetch_to_file(
            &client,
            &format!("{base}/asset"),
            &destination,
            EXECUTABLE_BODY_LIMIT,
        ))
        .unwrap();
        assert_eq!(status, 404);
        assert!(fallback_allowed(Some(status)));
    }

    #[test]
    fn native_sidecar_404_is_hard_failure() {
        let base = spawn_single(http_response(404, "Not Found", "", b"missing"));
        let client = test_client();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("asset.sha256");
        let error = block_on(download_required(
            &client,
            &format!("{base}/asset.sha256"),
            &destination,
            SIDECAR_BODY_LIMIT,
        ))
        .unwrap_err();
        match error {
            UpdateError::HttpStatus { status, .. } => assert_eq!(status, 404),
            other => panic!("expected HttpStatus, got {other}"),
        }
    }

    #[test]
    fn native_500_is_hard_failure() {
        let base = spawn_single(http_response(500, "Internal Server Error", "", b"error"));
        let client = test_client();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("asset");
        let status = block_on(fetch_to_file(
            &client,
            &format!("{base}/asset"),
            &destination,
            EXECUTABLE_BODY_LIMIT,
        ))
        .unwrap();
        assert_eq!(status, 500);
        assert!(!fallback_allowed(Some(status)));
        let error = block_on(download_required(
            &client,
            &format!("{base}/asset"),
            &destination,
            EXECUTABLE_BODY_LIMIT,
        ))
        .unwrap_err();
        assert!(matches!(error, UpdateError::HttpStatus { status: 500, .. }));
    }

    #[test]
    fn native_timeout_is_hard_failure() {
        let base = spawn_single_with_delay(
            http_response(200, "OK", "", b"slow"),
            Some(Duration::from_millis(500)),
        );
        let client = build_client_with_env(
            &eggfetch_core::ProxyEnvironment::new(),
            Duration::from_millis(50),
            Duration::from_millis(100),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("registry.json");
        let error = block_on(fetch_to_file(
            &client,
            &format!("{base}/slow"),
            &destination,
            REGISTRY_BODY_LIMIT,
        ))
        .unwrap_err();
        assert!(matches!(error, UpdateError::Transport(_)));
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn native_oversized_registry_body_is_rejected() {
        let base = spawn_single(http_response(200, "OK", "", b"0123456789ABCDEF"));
        let client = test_client();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("registry.json");
        let error = block_on(fetch_to_file(
            &client,
            &format!("{base}/registry"),
            &destination,
            8,
        ))
        .unwrap_err();
        assert!(matches!(error, UpdateError::BodyTooLarge { .. }));
    }

    #[test]
    fn native_oversized_sidecar_is_rejected() {
        let base = spawn_single(http_response(200, "OK", "", b"0123456789ABCDEF"));
        let client = test_client();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("asset.sha256");
        let error = block_on(download_required(
            &client,
            &format!("{base}/asset.sha256"),
            &destination,
            4,
        ))
        .unwrap_err();
        assert!(matches!(error, UpdateError::BodyTooLarge { .. }));
    }

    #[test]
    fn native_oversized_asset_is_rejected() {
        let base = spawn_single(http_response(200, "OK", "", b"0123456789ABCDEF"));
        let client = test_client();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("asset");
        let error = block_on(fetch_to_file(
            &client,
            &format!("{base}/asset"),
            &destination,
            4,
        ))
        .unwrap_err();
        assert!(matches!(error, UpdateError::BodyTooLarge { .. }));
    }

    #[test]
    fn native_redirect_is_followed() {
        let base = spawn_redirect_server(b"final-bytes");
        let client = test_client();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("asset");
        let status = block_on(fetch_to_file(
            &client,
            &format!("{base}/redirect"),
            &destination,
            EXECUTABLE_BODY_LIMIT,
        ))
        .unwrap();
        assert_eq!(status, 200);
        assert_eq!(fs::read(&destination).unwrap(), b"final-bytes");
    }

    #[test]
    fn https_to_http_redirect_policy_is_strict() {
        let policy = eggfetch_core::RedirectPolicy::strict(REDIRECT_MAX);
        assert!(policy.follow);
        assert_eq!(policy.max_redirects, REDIRECT_MAX);
        assert_eq!(
            policy.downgrade,
            eggfetch_core::RedirectDowngradePolicy::Deny
        );
        let from: url::Url = "https://example.com/a".parse().unwrap();
        let to: url::Url = "http://example.com/b".parse().unwrap();
        assert!(eggfetch_core::is_https_downgrade(&from, &to));
        assert!(eggfetch_core::check_https_downgrade(
            &from,
            &to,
            eggfetch_core::RedirectDowngradePolicy::Deny
        )
        .is_err());
    }

    #[test]
    fn proxy_environment_is_applied_explicitly() {
        let env = eggfetch_core::ProxyEnvironment::from_map([(
            "HTTPS_PROXY",
            "http://proxy.example:8080",
        )]);
        assert!(!env.is_empty());
        let url: url::Url = "https://example.com/releases".parse().unwrap();
        let resolved = env.resolve(&url).unwrap();
        assert!(resolved.is_some());
        let lower_wins = eggfetch_core::ProxyEnvironment::from_map([
            ("HTTPS_PROXY", "http://upper.example:8080"),
            ("https_proxy", "http://lower.example:8080"),
        ]);
        let resolved = lower_wins.resolve(&url).unwrap().unwrap();
        assert!(resolved.uri().as_str().contains("lower.example"));
        let bypass = eggfetch_core::ProxyEnvironment::from_map([
            ("HTTPS_PROXY", "http://proxy.example:8080"),
            ("NO_PROXY", "example.com"),
        ]);
        assert!(bypass.resolve(&url).unwrap().is_none());
        let invalid = eggfetch_core::ProxyEnvironment::from_map([(
            "HTTPS_PROXY",
            "http://user:env-secret-9@[::1",
        )]);
        let error = match eggfetch_core::Client::builder().proxy_environment(&invalid) {
            Ok(_) => panic!("expected proxy configuration to fail"),
            Err(error) => error,
        };
        assert!(!error.to_string().contains("env-secret-9"));
        let empty = eggfetch_core::ProxyEnvironment::new();
        assert!(empty.is_empty());
    }

    #[test]
    fn native_network_failure_is_hard_failure() {
        let client = test_client();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("registry.json");
        let error = block_on(fetch_to_file(
            &client,
            "http://127.0.0.1:1/unreachable",
            &destination,
            REGISTRY_BODY_LIMIT,
        ))
        .unwrap_err();
        assert!(matches!(
            error,
            UpdateError::Transport(_) | UpdateError::Proxy(_)
        ));
    }
}
