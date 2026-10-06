use crate::output::config_err;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use stegoeggo::{process_request_bytes_with_warnings, Error, ImageOutputFormat, ProtectionWarning};

pub(crate) fn collect_input_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, Error> {
    let mut files = Vec::new();
    for input in inputs {
        if input.is_dir() {
            for entry in fs::read_dir(input).map_err(Error::Io)? {
                let entry = entry.map_err(Error::Io)?;
                let path = entry.path();
                if is_image_file(&path) {
                    files.push(path);
                } else {
                    eprintln!("Warning: skipping non-image file {}", path.display());
                }
            }
        } else if !input.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("input does not exist: {}", input.display()),
            )));
        } else if is_image_file(input) {
            files.push(input.clone());
        } else {
            eprintln!("Warning: skipping non-image input {}", input.display());
        }
    }
    files.sort();
    Ok(files)
}

/// Longest prefix any supported magic signature needs.
const MAGIC_PREFIX_LEN: usize = 16;

/// Read just enough leading bytes to classify a file.
///
/// `ImageOutputFormat::from_magic_bytes` only inspects a short signature, so a
/// full `fs::read` here would buffer an entire large non-image (video, archive,
/// sparse file) just to reject it.
pub(crate) fn read_magic_prefix(path: &Path) -> Result<Vec<u8>, Error> {
    use std::io::Read as _;
    let mut file = fs::File::open(path)?;
    let mut buf = vec![0u8; MAGIC_PREFIX_LEN];
    let mut filled = 0;
    while filled < MAGIC_PREFIX_LEN {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(Error::Io(e)),
        }
    }
    buf.truncate(filled);
    Ok(buf)
}

pub(crate) fn is_image_file(path: &Path) -> bool {
    path.is_file()
        && read_magic_prefix(path)
            .ok()
            .and_then(|bytes| ImageOutputFormat::from_magic_bytes(&bytes))
            .is_some()
}

#[cfg(unix)]
fn existing_mode(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::metadata(path)
        .ok()
        .map(|meta| meta.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
fn existing_mode(_path: &Path) -> Option<u32> {
    None
}

/// Mode a plain `File::create` would produce under the current umask.
///
/// `tempfile::NamedTempFile` always creates `0600`, so the atomic staging file
/// is re-permissioned to this value after it is renamed into place.
#[cfg(unix)]
fn default_output_mode() -> Option<u32> {
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
    use std::sync::OnceLock;

    static MODE: OnceLock<Option<u32>> = OnceLock::new();
    *MODE.get_or_init(|| {
        let directory = std::env::temp_dir();
        (0..8).find_map(|attempt| {
            let probe =
                directory.join(format!(".stegoeggo-umask-{}-{attempt}", std::process::id()));
            let file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o666)
                .open(&probe)
                .ok()?;
            let mode = file.metadata().ok()?.permissions().mode() & 0o777;
            drop(file);
            let _ = fs::remove_file(&probe);
            Some(mode)
        })
    })
}

#[cfg(not(unix))]
fn default_output_mode() -> Option<u32> {
    None
}

fn apply_output_mode(path: &Path, previous_mode: Option<u32>) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if let Some(mode) = previous_mode.or_else(default_output_mode) {
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
        }
    }
    #[cfg(not(unix))]
    let _ = (path, previous_mode);
}

pub(crate) fn write_atomic(path: &Path, data: &[u8]) -> Result<(), Error> {
    let dir = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let previous_mode = existing_mode(path);
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("create temp file: {e}"),
        ))
    })?;
    std::io::Write::write_all(&mut temp, data).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("write temp file: {e}"),
        ))
    })?;
    temp.persist(path).map_err(|e| {
        Error::Io(std::io::Error::new(
            e.error.kind(),
            format!("persist temp file: {}", e.error),
        ))
    })?;
    apply_output_mode(path, previous_mode);
    Ok(())
}

pub(crate) fn check_input_output_disjoint(input: &Path, output: &Path) -> Result<(), Error> {
    let input_canonical = input.canonicalize().map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("resolve input path: {e}"),
        ))
    })?;
    let output_canonical = match output.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            let output_parent = match output.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent,
                _ => Path::new("."),
            };
            let parent = output_parent.canonicalize().map_err(|e| {
                Error::Io(std::io::Error::new(
                    e.kind(),
                    format!("resolve output path: {e}"),
                ))
            })?;
            parent.join(
                output
                    .file_name()
                    .ok_or_else(|| Error::Config("Output path has no file name".to_string()))?,
            )
        }
    };
    if input_canonical == output_canonical {
        return Err(Error::Config(
            "Input and output paths resolve to the same file; use --output to specify a different path".to_string(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if let (Ok(input_meta), Ok(output_meta)) =
            (std::fs::metadata(input), std::fs::metadata(output))
        {
            if input_meta.dev() == output_meta.dev() && input_meta.ino() == output_meta.ino() {
                return Err(Error::Config(
                    "Input and output paths resolve to the same file; use --output to specify a different path".to_string(),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn compute_output_path(
    input_path: &Path,
    output_dir: &Option<PathBuf>,
    output_format: ImageOutputFormat,
    seen: &mut HashMap<PathBuf, usize>,
) -> PathBuf {
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();
    let ext = output_format.extension();

    let filename = format!("{}_protected.{}", stem, ext);
    let base_path = output_dir
        .as_ref()
        .map_or_else(|| PathBuf::from(&filename), |dir| dir.join(&filename));
    let count = seen.entry(base_path.clone()).or_insert(0);
    if *count > 0 {
        let out_path = if let Some(ref dir) = output_dir {
            dir.join(format!("{}_protected_{}.{}", stem, count, ext))
        } else {
            PathBuf::from(format!("{}_protected_{}.{}", stem, count, ext))
        };
        *count += 1;
        out_path
    } else {
        *count = 1;
        base_path
    }
}

pub(crate) fn output_looks_like_file(out: &Path) -> bool {
    out.is_file() || (!out.is_dir() && out.extension().is_some())
}

#[allow(dead_code)]
#[allow(clippy::ptr_arg)]
pub(crate) fn process_single_file(
    input_path: &PathBuf,
    output_dir: &Option<PathBuf>,
    output_format: Option<ImageOutputFormat>,
    request: &stegoeggo::ProtectionRequest,
    verbose: bool,
    override_output: Option<PathBuf>,
) -> Result<(PathBuf, Vec<ProtectionWarning>), Error> {
    let input_bytes = fs::read(input_path).map_err(Error::Io)?;
    process_single_file_with_bytes(
        input_path,
        &input_bytes,
        output_dir,
        output_format,
        request,
        verbose,
        override_output,
    )
}

#[allow(clippy::ptr_arg)]
pub(crate) fn process_single_file_with_bytes(
    input_path: &PathBuf,
    input_bytes: &[u8],
    output_dir: &Option<PathBuf>,
    output_format: Option<ImageOutputFormat>,
    request: &stegoeggo::ProtectionRequest,
    verbose: bool,
    override_output: Option<PathBuf>,
) -> Result<(PathBuf, Vec<ProtectionWarning>), Error> {
    let detected_format = ImageOutputFormat::from_magic_bytes(input_bytes)
        .unwrap_or(stegoeggo::DEFAULT_OUTPUT_FORMAT);

    if verbose {
        if let Some(fmt) = output_format {
            if fmt != detected_format {
                eprintln!(
                    "Warning: output format {:?} differs from detected format {:?}",
                    fmt, detected_format
                );
            }
        }
    }

    let (output_bytes, warnings) = process_request_bytes_with_warnings(input_bytes, request)?;

    let effective_format = output_format.unwrap_or(detected_format);

    let output_path = if let Some(override_path) = override_output {
        if let Some(parent) = override_path.parent() {
            fs::create_dir_all(parent)?;
        }
        check_input_output_disjoint(input_path, &override_path)?;
        write_atomic(&override_path, &output_bytes)?;
        override_path
    } else {
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let ext = effective_format.extension();
        let filename = format!("{}_protected.{}", stem, ext);

        if let Some(ref dir) = output_dir {
            let out_path = if output_looks_like_file(dir) {
                if let Some(parent) = dir.parent() {
                    fs::create_dir_all(parent)?;
                }
                dir.clone()
            } else {
                fs::create_dir_all(dir)?;
                dir.join(&filename)
            };
            check_input_output_disjoint(input_path, &out_path)?;
            write_atomic(&out_path, &output_bytes)?;
            out_path
        } else {
            let output_path = PathBuf::from(filename);
            check_input_output_disjoint(input_path, &output_path)?;
            write_atomic(&output_path, &output_bytes)?;
            output_path
        }
    };

    Ok((output_path, warnings))
}

pub(crate) fn batch_output_for_file(
    out: &Option<PathBuf>,
    _files: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(ref out_path) = out {
        if output_looks_like_file(out_path) {
            return Err(config_err(format!(
                "--output names a file but a batch run writes one file per input; \
                 use a directory --output such as '{}'",
                out_path.display()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn collect_input_files_returns_sorted_paths() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("b.png");
        let second = temp.path().join("a.png");
        fs::write(&first, b"\x89PNG\r\n\x1a\n").unwrap();
        fs::write(&second, b"\x89PNG\r\n\x1a\n").unwrap();

        assert_eq!(
            collect_input_files(&[temp.path().to_path_buf()]).unwrap(),
            vec![second, first]
        );
    }

    #[test]
    fn collect_input_files_uses_magic_bytes_and_rejects_missing_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let image = temp.path().join("without_extension");
        fs::write(&image, b"\x89PNG\r\n\x1a\n").unwrap();
        assert_eq!(
            collect_input_files(std::slice::from_ref(&image)).unwrap(),
            vec![image]
        );

        let missing = temp.path().join("missing.png");
        assert!(collect_input_files(&[missing]).is_err());
    }

    #[test]
    fn output_directory_with_image_extension_is_not_a_file() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("output.png");
        fs::create_dir(&directory).unwrap();
        assert!(!output_looks_like_file(&directory));
    }

    #[test]
    fn bare_relative_output_path_resolves_against_the_current_directory() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("photo.png");
        fs::write(&input, b"\x89PNG\r\n\x1a\n").unwrap();

        // A bare relative output name has an empty parent; it must resolve against
        // the current directory instead of failing to canonicalize.
        assert!(check_input_output_disjoint(&input, Path::new("photo_protected.png")).is_ok());
        assert!(check_input_output_disjoint(&input, Path::new("./photo_protected.png")).is_ok());

        // An absolute path in a missing directory must still surface the real error.
        let missing_dir = temp.path().join("no-such-dir").join("out.png");
        assert!(check_input_output_disjoint(&input, &missing_dir).is_err());
    }

    #[test]
    fn write_atomic_writes_bare_relative_names_into_the_current_directory() {
        let temp = tempfile::tempdir().unwrap();
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let result = write_atomic(Path::new("out.bin"), b"payload");

        std::env::set_current_dir(previous).unwrap();
        result.unwrap();

        assert_eq!(
            fs::read(temp.path().join("out.bin")).unwrap(),
            b"payload".to_vec()
        );
    }

    #[test]
    fn same_file_input_and_output_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let image = temp.path().join("photo.png");
        fs::write(&image, b"\x89PNG\r\n\x1a\n").unwrap();

        assert!(check_input_output_disjoint(&image, &image).is_err());
    }

    #[test]
    fn compute_output_path_deduplicates_candidate_output_paths() {
        let temp = tempfile::tempdir().unwrap();
        let input = PathBuf::from("photo.png");
        let output_dir = Some(temp.path().to_path_buf());
        let mut seen = HashMap::new();

        assert_eq!(
            compute_output_path(&input, &output_dir, ImageOutputFormat::Png, &mut seen),
            temp.path().join("photo_protected.png")
        );
        assert_eq!(
            compute_output_path(&input, &output_dir, ImageOutputFormat::Png, &mut seen),
            temp.path().join("photo_protected_1.png")
        );
    }

    #[test]
    fn compute_output_path_deduplicates_stems_from_different_directories() {
        let temp = tempfile::tempdir().unwrap();
        let output_dir = Some(temp.path().to_path_buf());
        let mut seen = HashMap::new();

        assert_eq!(
            compute_output_path(
                Path::new("a/photo.png"),
                &output_dir,
                ImageOutputFormat::Png,
                &mut seen,
            ),
            temp.path().join("photo_protected.png")
        );
        assert_eq!(
            compute_output_path(
                Path::new("b/photo.png"),
                &output_dir,
                ImageOutputFormat::Png,
                &mut seen,
            ),
            temp.path().join("photo_protected_1.png")
        );
    }
}
