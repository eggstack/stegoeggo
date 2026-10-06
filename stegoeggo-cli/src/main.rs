mod args;
mod keys;
mod manifest;
mod output;
mod protect;
mod request;
mod update;
mod verify;

use args::{Args, Command, RootArgs};
use clap::parser::ValueSource;
use clap::{ArgMatches, CommandFactory, FromArgMatches};
use output::{
    classify_error, embed_path_label, JsonBatchFile, JsonEmbedOutcomeSummary, JsonExecutionReport,
    JsonOutput, JsonResourceUsage, EXIT_CONFIG, EXIT_OK,
};
use protect::{
    batch_output_for_file, check_input_output_disjoint, collect_input_files, compute_output_path,
    output_looks_like_file, process_single_file_with_bytes, read_magic_prefix, write_atomic,
};
use request::{
    build_protection_request_with_explicit_options, display_warnings, evidence_profile_for_display,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use stegoeggo::{ImageOutputFormat, WarningSeverity, DEFAULT_OUTPUT_FORMAT};

fn main() {
    match run() {
        Ok(()) => std::process::exit(EXIT_OK),
        Err(e) => {
            let exit_code = classify_error(e.as_ref());
            eprintln!("Error: {}", e);
            std::process::exit(exit_code);
        }
    }
}

#[allow(deprecated)]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if uses_command_parser(&argv) {
        let matches = RootArgs::command().get_matches_from(&argv);
        let root = RootArgs::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
        return run_command(
            root.command,
            root.json,
            matches.subcommand_matches("protect"),
        );
    }

    let matches = Args::command().get_matches_from(&argv);
    let parsed = Args::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    run_protect(
        &parsed.protect,
        matches.value_source("level") == Some(ValueSource::CommandLine),
        matches.value_source("profile") == Some(ValueSource::CommandLine),
    )
}

/// Options that consume a following value token.
///
/// The command router must skip a value-taking option's argument, otherwise the
/// value is evaluated as a candidate command name and a later `verify` is
/// re-parsed as a legacy `protect` input path. `every_value_taking_option_is_routable`
/// pins this list against the clap definition so it cannot drift again.
const VALUE_OPTIONS: &[&str] = &[
    "-o",
    "--output",
    "-l",
    "--level",
    "-p",
    "--profile",
    "-i",
    "--intensity",
    "-s",
    "--seed",
    "-f",
    "--format",
    "-d",
    "--dmi",
    "--metadata",
    "--copyright-notice",
    "--copyright-holder",
    "--creator",
    "--contact",
    "--rights-url",
    "--usage-terms",
    "--ai-constraints",
    "--credit-line",
    "--copyright-owner",
    "--licensor-name",
    "--licensor-email",
    "--licensor-url",
    "--content-created-at",
    "--key",
    "-j",
    "--jobs",
    "--stego-redundancy",
    "--jpeg-quality",
    "--rights-policy",
    "--preset",
    "--hidden-marker",
    "--authentication",
];

/// Command names that must dispatch to the root parser.
///
/// `help` is clap's generated help subcommand on `RootArgs`; without it the
/// router falls through to the legacy parser and treats `help` as an input path.
const COMMANDS: &[&str] = &[
    "protect",
    "inspect",
    "verify",
    "version",
    "update",
    "keygen",
    "sign",
    "verify-manifest",
    "help",
];

const STRICT_ERROR: &str =
    "Strict mode: one or more warnings with error severity (see warnings above)";

fn uses_command_parser(argv: &[std::ffi::OsString]) -> bool {
    let mut expects_value = false;
    for arg in argv.iter().skip(1) {
        let Some(value) = arg.to_str() else {
            return false;
        };
        if expects_value {
            expects_value = false;
            continue;
        }
        if value == "--" {
            return false;
        }
        if value == "--help" || value == "-h" || value == "--version" {
            return true;
        }
        let option = value.split_once('=').map_or(value, |(name, _)| name);
        if VALUE_OPTIONS.contains(&option) {
            if !value.contains('=') {
                expects_value = true;
            }
            continue;
        }
        if value.starts_with('-') {
            continue;
        }
        return COMMANDS.contains(&value);
    }
    false
}

#[allow(deprecated)]
fn run_command(
    command: Option<Command>,
    root_json: bool,
    protect_matches: Option<&ArgMatches>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(command) = command else {
        return Err(output::config_err(
            "a command is required; use `stegoeggo --help` to see available commands",
        ));
    };

    match command {
        Command::Protect(args) => {
            let matches = protect_matches.ok_or_else(|| {
                output::config_err("internal error: missing protect command arguments")
            })?;
            let mut args = *args;
            if root_json {
                args.json = true;
            }
            run_protect(
                &args,
                matches.value_source("level") == Some(ValueSource::CommandLine),
                matches.value_source("profile") == Some(ValueSource::CommandLine),
            )
        }
        Command::Inspect(args) => verify::run_inspect(
            &args.image,
            &args.key,
            root_json || args.json,
            args.verbose,
            false,
        ),
        Command::Verify(args) => verify::run_inspect(
            &args.image,
            &args.key,
            root_json || args.json,
            args.verbose,
            true,
        ),
        Command::Version => {
            println!("stegoeggo {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Update(_) => update::run_update(),
        #[cfg(feature = "signatures")]
        Command::Keygen { output_dir, key_id } => manifest::handle_keygen(&output_dir, &key_id),
        #[cfg(feature = "signatures")]
        Command::Sign {
            manifest,
            key,
            output,
        } => manifest::handle_sign(&manifest, &key, &output),
        #[cfg(feature = "signatures")]
        Command::VerifyManifest {
            manifest,
            image,
            key,
            payload_key,
            json,
        } => {
            let exit_code = manifest::handle_verify_manifest(
                &manifest,
                &image,
                &key,
                payload_key,
                root_json || json,
            )?;
            std::process::exit(exit_code);
        }
    }
}

#[allow(deprecated)]
fn run_protect(
    args: &args::ProtectArgs,
    level_explicit: bool,
    profile_explicit: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if args.tdm_reserved {
        eprintln!(
            "Warning: --tdm-reserved is deprecated. TDMRep deployment artifacts (HTTP headers, \
             /.well-known/tdmrep.json) are deferred. This flag now sets DMI to \
             ProhibitedSeeConstraints with a default AI constraints message. Image-level \
             tdm:reserve_tdm metadata is no longer emitted."
        );
    }

    let input_files = collect_input_files(&args.input)?;

    if input_files.is_empty() {
        if args.json {
            let json_output = JsonOutput {
                schema_version: 1,
                status: "failed".to_string(),
                output_path: None,
                warnings: Vec::new(),
                report: None,
                files: None,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&json_output).unwrap_or_else(|_| "{}".to_string())
            );
        }
        eprintln!("Error: No input files found");
        std::process::exit(EXIT_CONFIG);
    }

    let is_batch = input_files.len() > 1 || args.input.iter().any(|p| p.is_dir());

    if args.verbose {
        println!("stegoeggo CLI");
        println!("==============");
        println!("Input files: {}", input_files.len());
        if is_batch {
            println!("Mode: Batch processing");
        } else {
            println!("Input: {:?}", input_files[0]);
        }
    }

    // `--verify` is the compatibility verify-only form: it inspects the file it
    // is given and never protects anything. Combination errors are reported
    // here rather than deferred into an I/O failure with a misleading exit
    // code, because the documented contract is that this path exits 0.
    if args.verify {
        if is_batch {
            emit_verify_config_error(args.json, "Verify mode only works with single files");
        }
        if args.output.is_some() {
            emit_verify_config_error(
                args.json,
                "--verify inspects an existing file and cannot write one; \
                 drop --output or use `stegoeggo verify`",
            );
        }

        let input_path = &input_files[0];
        return verify::run_legacy_verify(
            input_path,
            args.output.as_deref(),
            &args.key,
            args.json,
            args.verbose,
        );
    }

    let request =
        build_protection_request_with_explicit_options(args, level_explicit, profile_explicit)?;

    let evidence_profile = evidence_profile_for_display(args);

    if args.verbose {
        println!("Evidence profile: {:?}", evidence_profile);
        println!("Intensity: {}", request.intensity());
        println!("Seed: {:?}", request.seed());
        if let Some(ref fmt) = request.processing().output_format {
            println!("Output format: {:?}", fmt);
        }
        println!("JPEG quality: {}", request.processing().jpeg_quality);
        println!(
            "Progressive JPEG: {}",
            request.processing().progressive_jpeg
        );
        println!("Rights metadata: {}", request.channels().rights_metadata);
        println!("Hidden marker: {:?}", request.channels().hidden_marker);
        println!("Authentication: {:?}", request.channels().authentication);
        println!(
            "MAC key: {}",
            if request.mac_key().is_some() {
                "set"
            } else {
                "none"
            }
        );
        if let Some(dmi) = request.policy().to_dmi_value() {
            println!("DMI: {}", dmi.as_str());
        }
        if is_batch {
            println!("Parallel jobs: {}", args.jobs);
        }
    }

    if args.dry_run {
        for input_path in &input_files {
            let input_format = detect_input_format(input_path)?;
            let plan = stegoeggo::resolve_request(&request, input_format)?;
            if input_files.len() > 1 {
                println!("File: {}", input_path.display());
            }
            println!("Resolved Protection Plan:");
            println!("  Effective policy: {:?}", plan.effective_policy());
            println!("  Effective DMI: {:?}", plan.effective_dmi());
            println!(
                "  Channels: rights_metadata={}, hidden_marker={:?}, auth={:?}",
                plan.channels().rights_metadata,
                plan.channels().hidden_marker,
                plan.channels().authentication
            );
            println!("  Input format: {:?}", plan.input_format());
            println!("  Output format: {:?}", plan.output_format());
            println!("  Seed: {}", plan.seed());
            println!("  Intensity: {}", plan.intensity());
            println!("  Metadata-only: {}", plan.is_metadata_only());
            if !plan.warnings().is_empty() {
                println!("  Warnings:");
                for w in plan.warnings() {
                    println!("    - {}", w);
                }
            }
        }
        return Ok(());
    }

    let output_format: Option<ImageOutputFormat> = args.format.as_ref().map(|f| f.clone().into());

    if is_batch {
        use rayon::prelude::*;

        batch_output_for_file(&args.output, &input_files)?;

        #[allow(clippy::type_complexity)]
        let results: Vec<
            Result<(PathBuf, PathBuf, Vec<stegoeggo::ProtectionWarning>), (PathBuf, String)>,
        > = if args.jobs > 1 {
            let mut seen: HashMap<PathBuf, usize> = HashMap::new();
            let mut batch_inputs: Vec<(PathBuf, Option<Vec<u8>>, PathBuf, Option<String>)> =
                Vec::new();
            for input_path in &input_files {
                match fs::read(input_path) {
                    Ok(bytes) => {
                        let detected = ImageOutputFormat::from_magic_bytes(&bytes)
                            .unwrap_or(DEFAULT_OUTPUT_FORMAT);
                        let effective_format = output_format.unwrap_or(detected);
                        let out_path = compute_output_path(
                            input_path,
                            &args.output,
                            effective_format,
                            &mut seen,
                        );
                        batch_inputs.push((input_path.clone(), Some(bytes), out_path, None));
                    }
                    Err(e) => {
                        let effective_format = output_format.unwrap_or(DEFAULT_OUTPUT_FORMAT);
                        let out_path = compute_output_path(
                            input_path,
                            &args.output,
                            effective_format,
                            &mut seen,
                        );
                        batch_inputs.push((
                            input_path.clone(),
                            None,
                            out_path,
                            Some(e.to_string()),
                        ));
                    }
                }
            }

            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(args.jobs)
                .build()
                .map_err(|e| format!("cannot build a {}-thread worker pool: {e}", args.jobs))?;

            pool.install(|| {
                batch_inputs
                    .par_iter()
                    .with_max_len(1)
                    .map(|(input_path, maybe_bytes, override_output, maybe_err)| {
                        if let Some(err) = maybe_err {
                            return Err((input_path.clone(), err.clone()));
                        }
                        let Some(input_bytes) = maybe_bytes.as_ref() else {
                            return Err((
                                input_path.clone(),
                                "internal error: batch input has neither bytes nor error"
                                    .to_string(),
                            ));
                        };
                        process_single_file_with_bytes(
                            input_path,
                            input_bytes,
                            &args.output,
                            output_format,
                            &request,
                            args.verbose,
                            Some(override_output.clone()),
                        )
                        .map(|(output, warnings)| (input_path.clone(), output, warnings))
                        .map_err(|e| (input_path.clone(), e.to_string()))
                    })
                    .collect::<Vec<_>>()
            })
        } else {
            let mut seen: HashMap<PathBuf, usize> = HashMap::new();

            input_files
                .iter()
                .map(|input_path| {
                    let input_bytes_preview =
                        fs::read(input_path).map_err(|e| (input_path.clone(), e.to_string()))?;
                    let detected = ImageOutputFormat::from_magic_bytes(&input_bytes_preview)
                        .unwrap_or(DEFAULT_OUTPUT_FORMAT);
                    let effective_format = output_format.unwrap_or(detected);

                    let override_output =
                        compute_output_path(input_path, &args.output, effective_format, &mut seen);

                    process_single_file_with_bytes(
                        input_path,
                        &input_bytes_preview,
                        &args.output,
                        output_format,
                        &request,
                        args.verbose,
                        Some(override_output),
                    )
                    .map(|(output, warnings)| (input_path.clone(), output, warnings))
                    .map_err(|e| (input_path.clone(), e.to_string()))
                })
                .collect()
        };

        let mut success_count = 0;
        let mut failed_files: Vec<PathBuf> = Vec::new();
        let mut has_errors = false;
        let mut json_files: Vec<JsonBatchFile> = Vec::with_capacity(results.len());

        for result in results {
            match result {
                Ok((input_path, output_path, warnings)) => {
                    success_count += 1;
                    display_warnings(&warnings, evidence_profile, args.verbose);
                    if args.strict
                        && warnings.iter().any(|w| {
                            w.severity_for_profile(evidence_profile) == WarningSeverity::Error
                        })
                    {
                        has_errors = true;
                    }
                    if args.json {
                        json_files.push(JsonBatchFile {
                            input_path: input_path.display().to_string(),
                            status: "ok".to_string(),
                            output_path: Some(output_path.display().to_string()),
                            error: None,
                            warnings: warnings.iter().map(|w| w.to_string()).collect(),
                        });
                    } else if args.verbose {
                        println!("  {} -> {}", input_path.display(), output_path.display());
                    } else {
                        println!("{}", output_path.display());
                    }
                }
                Err((path, msg)) => {
                    failed_files.push(path.clone());
                    if args.json {
                        json_files.push(JsonBatchFile {
                            input_path: path.display().to_string(),
                            status: "error".to_string(),
                            output_path: None,
                            error: Some(msg.clone()),
                            warnings: Vec::new(),
                        });
                    }
                    eprintln!("Error: {}", msg);
                }
            }
        }

        if args.json {
            let json_output = JsonOutput {
                schema_version: 1,
                status: if failed_files.is_empty() && !has_errors {
                    "ok".to_string()
                } else {
                    "error".to_string()
                },
                output_path: None,
                warnings: Vec::new(),
                report: None,
                files: Some(json_files),
            };
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }

        if args.verbose || !failed_files.is_empty() {
            let summary = format!(
                "\nCompleted: {} succeeded, {} failed",
                success_count,
                failed_files.len()
            );
            if args.json {
                eprintln!("{}", summary);
            } else {
                println!("{}", summary);
            }
        }

        if !failed_files.is_empty() {
            return Err(format!("{} file(s) failed processing", failed_files.len()).into());
        }

        if args.strict && has_errors {
            return Err(STRICT_ERROR.into());
        }

        return Ok(());
    }

    let input_path = &input_files[0];
    let input_bytes = fs::read(input_path)?;

    let detected_format =
        ImageOutputFormat::from_magic_bytes(&input_bytes).unwrap_or(DEFAULT_OUTPUT_FORMAT);
    if args.verbose {
        if let Some(fmt) = output_format {
            if fmt != detected_format {
                eprintln!(
                    "Warning: output format {:?} differs from detected format {:?}",
                    fmt, detected_format
                );
            }
        }
    }

    let output_path = if let Some(ref dir) = args.output {
        if output_looks_like_file(dir) {
            if let Some(parent) = dir.parent() {
                fs::create_dir_all(parent)?;
            }
            dir.clone()
        } else {
            fs::create_dir_all(dir)?;
            let stem = input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let ext = output_format.unwrap_or(detected_format).extension();
            dir.join(format!("{}_protected.{}", stem, ext))
        }
    } else {
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let ext = output_format.unwrap_or(detected_format).extension();
        PathBuf::from(format!("{}_protected.{}", stem, ext))
    };

    if args.json {
        let (output_bytes, report) =
            stegoeggo::process_request_bytes_with_report(&input_bytes, &request)?;
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        check_input_output_disjoint(input_path, &output_path)?;
        write_atomic(&output_path, &output_bytes)?;

        let strict_failed = args.strict
            && report
                .warnings()
                .iter()
                .any(|w| w.severity_for_profile(evidence_profile) == WarningSeverity::Error);

        let json_output = JsonOutput {
            schema_version: 1,
            status: if strict_failed { "error" } else { "ok" }.to_string(),
            output_path: Some(output_path.display().to_string()),
            warnings: report.warnings().iter().map(|w| w.to_string()).collect(),
            report: Some(JsonExecutionReport {
                effective_policy: format!("{:?}", report.effective_policy()),
                effective_dmi: report.effective_dmi().map(|d| format!("{:?}", d)),
                metadata_injected: report.metadata_injected(),
                stego_attempted: report.stego_attempted(),
                stego_succeeded: report.stego_succeeded(),
                format_transcoded: report.format_transcoded(),
                embed_summary: report.embed_summary().map(|s| JsonEmbedOutcomeSummary {
                    status: format!("{}", s.status),
                    embedding_path: embed_path_label(s.path).to_string(),
                    payload_bytes: s.payload_bytes,
                    required_capacity: s.required_capacity,
                    available_capacity: s.available_capacity,
                }),
                resource_usage: report.resource_usage().map(|u| JsonResourceUsage {
                    input_bytes: u.input_bytes,
                    png_chunks_scanned: u.png_chunks_scanned,
                    jpeg_segments_scanned: u.jpeg_segments_scanned,
                    webp_riff_chunks_scanned: u.webp_riff_chunks_scanned,
                    xmp_bytes_parsed: u.xmp_bytes_parsed,
                    metadata_fields_extracted: u.metadata_fields_extracted,
                    metadata_bytes_copied: u.metadata_bytes_copied,
                    tile_origins_checked: u.tile_origins_checked,
                    verification_seeds_tried: u.verification_seeds_tried,
                    peak_allocations_bytes: u.peak_allocations_bytes,
                }),
            }),
            files: None,
        };
        println!("{}", serde_json::to_string_pretty(&json_output)?);

        if strict_failed {
            return Err(STRICT_ERROR.into());
        }
    } else {
        let (output_bytes, warnings) =
            stegoeggo::process_request_bytes_with_warnings(&input_bytes, &request)?;
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        check_input_output_disjoint(input_path, &output_path)?;
        write_atomic(&output_path, &output_bytes)?;

        display_warnings(&warnings, evidence_profile, args.verbose);

        if args.verbose {
            println!("Output: {:?}", output_path);
            println!("Done!");
        } else {
            println!("{}", output_path.display());
        }

        if args.strict
            && warnings
                .iter()
                .any(|w| w.severity_for_profile(evidence_profile) == WarningSeverity::Error)
        {
            return Err(STRICT_ERROR.into());
        }
    }

    Ok(())
}

/// Classify an input file from its magic prefix alone.
///
/// `--dry-run` only needs the format, so it must not buffer the whole image.
fn detect_input_format(
    input_path: &std::path::Path,
) -> Result<ImageOutputFormat, stegoeggo::Error> {
    let prefix = read_magic_prefix(input_path)?;
    Ok(ImageOutputFormat::from_magic_bytes(&prefix).unwrap_or(DEFAULT_OUTPUT_FORMAT))
}

/// Report a `--verify` combination rejection as a config error (exit 2)
/// instead of letting it surface later as an I/O failure with exit 1.
fn emit_verify_config_error(json: bool, message: &str) -> ! {
    if json {
        let json_output = JsonOutput {
            schema_version: 1,
            status: "failed".to_string(),
            output_path: None,
            warnings: Vec::new(),
            report: None,
            files: None,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&json_output).unwrap_or_else(|_| "{}".to_string())
        );
    }
    eprintln!("Error: {message}");
    std::process::exit(EXIT_CONFIG);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn argv(args: &[&str]) -> Vec<OsString> {
        std::iter::once("stegoeggo")
            .chain(args.iter().copied())
            .map(OsString::from)
            .collect()
    }

    /// Every value-taking non-positional option must be listed, otherwise the
    /// router reads its value as a candidate command name and re-parses a later
    /// `verify` as a legacy `protect` input path.
    #[test]
    fn every_value_taking_option_is_routable() {
        for arg in Args::command().get_arguments() {
            if arg.is_positional() || !arg.get_num_args().is_some_and(|r| r.takes_values()) {
                continue;
            }
            let mut names = Vec::new();
            if let Some(long) = arg.get_long() {
                names.push(format!("--{long}"));
            }
            if let Some(short) = arg.get_short() {
                names.push(format!("-{short}"));
            }
            names.extend(
                arg.get_all_aliases()
                    .unwrap_or_default()
                    .iter()
                    .map(|alias| format!("--{alias}")),
            );
            for name in names {
                assert!(
                    VALUE_OPTIONS.contains(&name.as_str()),
                    "{name} takes a value but is missing from VALUE_OPTIONS"
                );
            }
        }
    }

    #[test]
    fn router_skips_option_values_before_a_command_name() {
        for prefix in [
            vec![],
            vec!["--stego-redundancy", "5"],
            vec!["--jpeg-quality", "90"],
            vec!["--stego-redundancy=5"],
            vec!["--jpeg-quality=90"],
            vec!["--key", "abcd"],
            vec!["--rights-policy", "prohibited-ai-ml-training"],
        ] {
            let mut args = prefix.clone();
            args.extend(["verify", "plain.png"]);
            assert!(
                uses_command_parser(&argv(&args)),
                "router misrouted {args:?} to the legacy protect parser"
            );
        }
    }

    #[test]
    fn router_recognizes_every_command_including_help() {
        for command in COMMANDS {
            assert!(
                uses_command_parser(&argv(&[command])),
                "router does not recognize the {command} command"
            );
        }
    }

    /// A positional token that is not a command name stays on the legacy path.
    #[test]
    fn router_ignores_non_command_positionals() {
        assert!(!uses_command_parser(&argv(&["plain.png"])));
        assert!(!uses_command_parser(&argv(&[
            "--intensity",
            "0.5",
            "plain.png"
        ])));
        assert!(!uses_command_parser(&argv(&["--", "plain.png"])));
    }
}
