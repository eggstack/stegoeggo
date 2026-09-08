mod args;
mod keys;
mod manifest;
mod output;
mod protect;
mod request;
mod verify;

use args::Args;
#[cfg(feature = "signatures")]
use args::Command;
use clap::parser::ValueSource;
use clap::{CommandFactory, FromArgMatches};
use output::{
    classify_error, embed_path_label, JsonEmbedOutcomeSummary, JsonExecutionReport, JsonOutput,
    JsonResourceUsage, EXIT_CONFIG, EXIT_OK,
};
use protect::{
    batch_output_for_file, check_input_output_disjoint, collect_input_files, compute_output_path,
    output_looks_like_file, process_single_file_with_bytes, write_atomic,
};
use request::{
    build_protection_request_with_explicit_options, display_warnings, evidence_profile_for_display,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use stegoeggo::{ImageOutputFormat, ProtectionLevel, WarningSeverity, DEFAULT_OUTPUT_FORMAT};

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
    let matches = Args::command().get_matches();
    let args = Args::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    #[cfg(feature = "signatures")]
    if let Some(ref cmd) = args.command {
        return match cmd {
            Command::Keygen { output_dir, key_id } => manifest::handle_keygen(output_dir, key_id),
            Command::Sign {
                manifest,
                key,
                output,
            } => manifest::handle_sign(manifest, key, output),
            Command::VerifyManifest {
                manifest,
                image,
                key,
                payload_key,
            } => {
                let exit_code = manifest::handle_verify_manifest(
                    manifest,
                    image,
                    key,
                    payload_key.clone(),
                    args.json,
                )?;
                std::process::exit(exit_code);
            }
        };
    }

    if args.tdm_reserved {
        eprintln!(
            "Warning: --tdm-reserved is deprecated. TDMRep deployment artifacts (HTTP headers, \
             /.well-known/tdmrep.json) are deferred. This flag now sets DMI to \
             ProhibitedSeeConstraints with a default AI constraints message. Image-level \
             tdm:reserve_tdm metadata is no longer emitted."
        );
    }

    let input_files = collect_input_files(&args.input);

    if input_files.is_empty() {
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

    if args.verify {
        if is_batch {
            eprintln!("Error: Verify mode only works with single files");
            std::process::exit(EXIT_CONFIG);
        }

        let input_path = &input_files[0];
        return verify::run_verify(input_path, &args.output, &args.key, args.json, args.verbose);
    }

    let level_explicit = matches.value_source("level") == Some(ValueSource::CommandLine);
    let profile_explicit = matches.value_source("profile") == Some(ValueSource::CommandLine);
    let request =
        build_protection_request_with_explicit_options(&args, level_explicit, profile_explicit)?;

    let evidence_profile = evidence_profile_for_display(&args);

    if args.verbose {
        println!(
            "Protection level: {:?}",
            ProtectionLevel::from(args.level.clone())
        );
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
            let input_bytes = fs::read(input_path)?;
            let input_format = stegoeggo::ImageOutputFormat::from_magic_bytes(&input_bytes)
                .unwrap_or(DEFAULT_OUTPUT_FORMAT);
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
                            "internal error: batch input has neither bytes nor error".to_string(),
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
                .collect()
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
                    if args.verbose {
                        println!("  {} -> {}", input_path.display(), output_path.display());
                    } else {
                        println!("{}", output_path.display());
                    }
                }
                Err((path, msg)) => {
                    failed_files.push(path);
                    eprintln!("Error: {}", msg);
                }
            }
        }

        if args.verbose || !failed_files.is_empty() {
            println!(
                "\nCompleted: {} succeeded, {} failed",
                success_count,
                failed_files.len()
            );
        }

        if !failed_files.is_empty() {
            return Err(format!("{} file(s) failed processing", failed_files.len()).into());
        }

        if args.strict && has_errors {
            return Err(
                "Strict mode: one or more warnings with error severity (see warnings above)".into(),
            );
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

        let json_output = JsonOutput {
            schema_version: 1,
            status: "ok".to_string(),
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
        };
        println!("{}", serde_json::to_string_pretty(&json_output)?);
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
            return Err(
                "Strict mode: one or more warnings with error severity (see warnings above)".into(),
            );
        }
    }

    Ok(())
}
