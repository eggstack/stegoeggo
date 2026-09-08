use crate::output::config_err;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use stegoeggo::{process_request_bytes_with_warnings, Error, ImageOutputFormat, ProtectionWarning};

pub(crate) fn collect_input_files(inputs: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for input in inputs {
        if input.is_dir() {
            if let Ok(entries) = fs::read_dir(input) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if is_image_file(&path) {
                        files.push(path);
                    }
                }
            }
        } else if is_image_file(input) {
            files.push(input.clone());
        }
    }
    files.sort();
    files
}

pub(crate) fn is_image_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp")
    } else {
        false
    }
}

pub(crate) fn write_atomic(path: &Path, data: &[u8]) -> Result<(), Error> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
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
            let output_parent = output.parent().unwrap_or_else(|| Path::new("."));
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

pub(crate) fn has_duplicate_stems(files: &[PathBuf]) -> bool {
    let mut seen = HashSet::new();
    files.iter().any(|f| {
        let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        !seen.insert(stem.to_string())
    })
}

pub(crate) fn output_looks_like_file(out: &Path) -> bool {
    out.is_file() || (out.extension().is_some() && is_image_file(out))
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
    files: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(ref out_path) = out {
        if output_looks_like_file(out_path) && has_duplicate_stems(files) {
            return Err(config_err(
                "--output names a file but the batch input contains duplicate file stems; \
                 use a directory --output or rename inputs",
            ));
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
        fs::write(&first, []).unwrap();
        fs::write(&second, []).unwrap();

        assert_eq!(
            collect_input_files(&[temp.path().to_path_buf()]),
            vec![second, first]
        );
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
