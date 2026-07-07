use clap::{Parser, ValueEnum};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use stegoeggo::Error;
use stegoeggo::{
    generate_random_seed, process_image_bytes, verify_image_bytes_detailed, DmiValue,
    ImageOutputFormat, ProtectionContext, ProtectionLevel, StegoPayload, VerificationResult,
    DEFAULT_OUTPUT_FORMAT,
};

#[derive(Parser, Debug)]
#[command(name = "stegoeggo")]
#[command(about = "Embed legal-notice and rights-reservation metadata into images, with optional steganographic markers", long_about = None)]
struct Args {
    #[arg(help = "Input image file(s). Use multiple files or a directory for batch processing")]
    input: Vec<PathBuf>,

    #[arg(
        short,
        long,
        help = "Output directory (for batch processing) or output file (for single file)"
    )]
    output: Option<PathBuf>,

    #[arg(long, help = "Verify if image contains protection signature")]
    verify: bool,

    #[arg(short, long, default_value = "standard", help = "Protection level")]
    level: ProtectionLevelArg,

    #[arg(
        short,
        long,
        default_value = "0.5",
        help = "Protection intensity (0.0-1.0)"
    )]
    intensity: f32,

    #[arg(short, long, help = "Seed for reproducible results")]
    seed: Option<u64>,

    #[arg(short, long, help = "Output format (png|jpg|webp) - defaults to png")]
    format: Option<OutputFormatArg>,

    #[arg(
        long,
        default_value = "2",
        help = "Stego embedding redundancy (1-10). Higher = more robust, lower = faster"
    )]
    stego_redundancy: usize,

    #[arg(
        long,
        default_value = "90",
        help = "JPEG encoding quality (1-100). Only applies when output is JPEG"
    )]
    jpeg_quality: u8,

    #[arg(
        long,
        help = "Use progressive JPEG encoding. Progressive JPEGs render faster on slow connections"
    )]
    progressive: bool,

    #[arg(short, long, help = "Print verbose output")]
    verbose: bool,

    #[arg(
        short,
        long,
        help = "AI-training restriction metadata (IPTC DMI value)"
    )]
    dmi: Option<DmiArg>,

    #[arg(
        long,
        help = "Inject metadata (seed, DMI). Default: true for Light and Standard"
    )]
    metadata: Option<bool>,

    #[arg(
        long,
        help = "Inject legal claims (copyright, usage terms) into image metadata — only for content you own"
    )]
    legal_claims: bool,

    #[arg(long, help = "Copyright holder name (e.g., 'Jane Doe' or 'Acme Corp')")]
    copyright_holder: Option<String>,

    #[arg(long, help = "Creator/author name (e.g., 'Jane Doe')")]
    creator: Option<String>,

    #[arg(long, help = "Contact email or URL for rights inquiries")]
    contact: Option<String>,

    #[arg(long, help = "URL to full usage terms or license text")]
    rights_url: Option<String>,

    #[arg(long, help = "Brief usage terms summary (e.g., 'All rights reserved')")]
    usage_terms: Option<String>,

    #[arg(
        long,
        help = "AI-specific constraints (e.g., 'No training, no generation')"
    )]
    ai_constraints: Option<String>,

    #[arg(
        long,
        help = "Shorthand: prohibit AI/ML training and set default AI constraints"
    )]
    no_ai_training: bool,

    #[arg(long, help = "Shorthand: prohibit generative AI training only")]
    no_genai_training: bool,

    #[arg(long, help = "Shorthand: reserve text and data mining rights")]
    tdm_reserved: bool,

    #[arg(
        long,
        help = "Optional cryptographic key for HMAC-verified steganographic payloads (authenticated provenance mode)"
    )]
    key: Option<String>,

    #[arg(
        long,
        help = "Additional seeds to try during verification (comma-separated u64 values)"
    )]
    known_seeds: Option<String>,

    #[arg(
        short = 'j',
        long = "jobs",
        default_value = "1",
        help = "Number of parallel jobs for batch processing"
    )]
    jobs: usize,
}

#[derive(Debug, Clone, ValueEnum)]
enum ProtectionLevelArg {
    Disabled,
    Light,
    Standard,
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormatArg {
    Png,
    Jpg,
    WebP,
}

#[derive(Debug, Clone, ValueEnum)]
enum DmiArg {
    Auto,
    Unspecified,
    Allowed,
    ProhibitedAi,
    ProhibitedGenAi,
    ProhibitedSe,
    Prohibited,
    ProhibitedConstraints,
}

impl DmiArg {
    /// Convert CLI DMI arg to library DMI value.
    /// Returns `None` for `Auto`, meaning the caller should auto-select based on protection level.
    fn into_dmi_value(self) -> Option<DmiValue> {
        match self {
            DmiArg::Auto => None,
            DmiArg::Unspecified => Some(DmiValue::Unspecified),
            DmiArg::Allowed => Some(DmiValue::Allowed),
            DmiArg::ProhibitedAi => Some(DmiValue::ProhibitedAiMlTraining),
            DmiArg::ProhibitedGenAi => Some(DmiValue::ProhibitedGenAiMlTraining),
            DmiArg::ProhibitedSe => Some(DmiValue::ProhibitedExceptSearchEngineIndexing),
            DmiArg::Prohibited => Some(DmiValue::Prohibited),
            DmiArg::ProhibitedConstraints => Some(DmiValue::ProhibitedSeeConstraints),
        }
    }
}

impl From<ProtectionLevelArg> for ProtectionLevel {
    fn from(arg: ProtectionLevelArg) -> Self {
        match arg {
            ProtectionLevelArg::Disabled => ProtectionLevel::Disabled,
            ProtectionLevelArg::Light => ProtectionLevel::Light,
            ProtectionLevelArg::Standard => ProtectionLevel::Standard,
        }
    }
}

impl From<OutputFormatArg> for ImageOutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::Png => ImageOutputFormat::Png,
            OutputFormatArg::Jpg => ImageOutputFormat::Jpeg,
            OutputFormatArg::WebP => ImageOutputFormat::WebP,
        }
    }
}

fn collect_input_files(inputs: &[PathBuf]) -> Vec<PathBuf> {
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
    files
}

fn is_image_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp")
    } else {
        false
    }
}

fn compute_output_path(
    input_path: &Path,
    output_dir: &Option<PathBuf>,
    output_format: ImageOutputFormat,
    seen: &mut HashMap<PathBuf, usize>,
) -> Option<PathBuf> {
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();
    let ext = output_format.extension();

    let count = seen.entry(PathBuf::from(&stem)).or_insert(0);
    if *count > 0 {
        let out_path = if let Some(ref dir) = output_dir {
            dir.join(format!("{}_protected_{}.{}", stem, count, ext))
        } else {
            PathBuf::from(format!("{}_protected_{}.{}", stem, count, ext))
        };
        *count += 1;
        Some(out_path)
    } else {
        *count = 1;
        None
    }
}

fn process_single_file(
    input_path: &PathBuf,
    output_dir: &Option<PathBuf>,
    output_format: ImageOutputFormat,
    ctx_base: &ProtectionContext,
    protection_level: ProtectionLevel,
    verbose: bool,
    override_output: Option<PathBuf>,
) -> Result<PathBuf, Error> {
    let input_bytes = fs::read(input_path).map_err(Error::Io)?;

    let detected_format =
        ImageOutputFormat::from_magic_bytes(&input_bytes).unwrap_or(DEFAULT_OUTPUT_FORMAT);

    if verbose && output_format != detected_format {
        eprintln!(
            "Warning: output format {:?} differs from detected format {:?}",
            output_format, detected_format
        );
    }

    let mut ctx = ctx_base.clone();
    ctx.set_input_format(detected_format);

    let output_bytes = process_image_bytes(&input_bytes, protection_level, &ctx)?;

    let output_path = if let Some(override_path) = override_output {
        if let Some(parent) = override_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&override_path, &output_bytes)?;
        override_path
    } else {
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let ext = output_format.extension();
        let filename = format!("{}_protected.{}", stem, ext);

        if let Some(ref dir) = output_dir {
            let out_path = dir.join(&filename);
            fs::create_dir_all(dir)?;
            fs::write(&out_path, &output_bytes)?;
            out_path
        } else {
            let output_path = PathBuf::from(filename);
            fs::write(&output_path, &output_bytes)?;
            output_path
        }
    };

    Ok(output_path)
}

fn print_payload_info(payload: &StegoPayload) {
    let level_str = ProtectionLevel::from_byte(payload.protection_level())
        .map(|l: ProtectionLevel| l.as_str())
        .unwrap_or("Unknown");
    println!("Level: {} (id: {})", level_str, payload.protection_level());
    println!("Seed: {}", payload.seed());
    println!("Intensity: {:.2}", payload.intensity());
    println!("Version: {}", payload.version());
}

fn build_legal_metadata(args: &Args) -> (Option<stegoeggo::LegalMetadata>, Option<DmiValue>) {
    let has_legal_flags = args.copyright_holder.is_some()
        || args.creator.is_some()
        || args.contact.is_some()
        || args.rights_url.is_some()
        || args.usage_terms.is_some()
        || args.ai_constraints.is_some()
        || args.no_ai_training
        || args.no_genai_training
        || args.tdm_reserved;

    if !has_legal_flags {
        return (None, None);
    }

    let mut meta = stegoeggo::LegalMetadata::default();
    let mut dmi_override: Option<DmiValue> = None;

    if let Some(ref v) = args.copyright_holder {
        meta = meta.with_copyright_holder(v);
    }
    if let Some(ref v) = args.creator {
        meta = meta.with_creator(v);
    }
    if let Some(ref v) = args.contact {
        meta = meta.with_contact_email(v);
    }
    if let Some(ref v) = args.rights_url {
        meta = meta.with_web_statement_of_rights(v);
    }
    if let Some(ref v) = args.usage_terms {
        meta = meta.with_usage_terms(v);
    }
    if let Some(ref v) = args.ai_constraints {
        meta = meta.with_ai_constraints(v);
    }

    // DMI presets (--no-ai-training, --no-genai-training, --tdm-reserved)
    if args.no_ai_training {
        dmi_override = Some(DmiValue::ProhibitedAiMlTraining);
        if args.ai_constraints.is_none() {
            meta = meta.with_ai_constraints(
                "Training for artificial intelligence and machine learning is prohibited",
            );
        }
    } else if args.no_genai_training {
        dmi_override = Some(DmiValue::ProhibitedGenAiMlTraining);
        if args.ai_constraints.is_none() {
            meta = meta.with_ai_constraints(
                "Training for generative artificial intelligence is prohibited",
            );
        }
    } else if args.tdm_reserved {
        dmi_override = Some(DmiValue::ProhibitedSeeConstraints);
        if args.ai_constraints.is_none() {
            meta = meta.with_ai_constraints("Text and data mining rights reserved");
        }
    }

    (Some(meta), dmi_override)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let input_files = collect_input_files(&args.input);

    if input_files.is_empty() {
        eprintln!("Error: No input files found");
        std::process::exit(1);
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
            std::process::exit(1);
        }

        let input_path = &input_files[0];
        let bytes_to_verify = if let Some(ref output_path) = args.output {
            if args.verbose {
                eprintln!("Verifying explicit output file");
            }
            fs::read(output_path)?
        } else {
            if args.verbose {
                eprintln!("Verifying input file");
            }
            fs::read(input_path)?
        };

        if args.verbose {
            if let Ok(img) = image::load_from_memory(&bytes_to_verify) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                eprintln!("Image dimensions: {}x{}", w, h);
            }
        }

        let result = verify_image_bytes_detailed(&bytes_to_verify, &[]);

        if args.verbose {
            match &result {
                VerificationResult::Verified { .. } => {
                    eprintln!("Verified steganographic payload found")
                }
                VerificationResult::Corrupted { .. } => {
                    eprintln!("Protection payload found but integrity check failed")
                }
                VerificationResult::MetadataOnly { seed } => {
                    eprintln!("Metadata-only evidence found (seed {})", seed)
                }
                VerificationResult::NotFound => eprintln!("No protection evidence found"),
            }
        }

        match result {
            VerificationResult::Verified { payload } => {
                println!("Protected: Yes (verified)");
                print_payload_info(&payload);
            }
            VerificationResult::Corrupted { payload } => {
                println!("Protected: Yes (corrupted)");
                print_payload_info(&payload);
            }
            VerificationResult::MetadataOnly { seed } => {
                println!("Protected: Maybe (metadata-only — steganographic payload not verified)");
                println!("Seed: {}", seed);
            }
            VerificationResult::NotFound => {
                println!("Protected: No");
                println!("This image does not contain a protection signature.");
            }
        }
        return Ok(());
    }

    let seed = args.seed.unwrap_or_else(generate_random_seed);

    let mac_key = args
        .key
        .as_ref()
        .map(|k| hex::decode(k).map_err(|e| format!("Invalid hex key '{}': {}", k, e)))
        .transpose()?;

    let output_format = args.format.map(ImageOutputFormat::from);
    let effective_output_format = output_format.unwrap_or(DEFAULT_OUTPUT_FORMAT);

    let protection_level = ProtectionLevel::from(args.level);

    let dmi_value = args.dmi.and_then(|d| {
        d.into_dmi_value().or({
            // Auto-select DMI based on protection level
            Some(match protection_level {
                ProtectionLevel::Disabled | ProtectionLevel::Light => DmiValue::Unspecified,
                _ => DmiValue::ProhibitedAiMlTraining,
            })
        })
    });

    let (legal_metadata, legal_dmi_override) = build_legal_metadata(&args);

    if args.metadata == Some(false) && legal_metadata.is_some() {
        eprintln!(
            "Error: Cannot use --no-metadata (or -m false) together with legal metadata flags \
             (--copyright-holder, --creator, --contact, --rights-url, --usage-terms, \
             --ai-constraints, --no-ai-training, --no-genai-training, --tdm-reserved). \
             Legal metadata requires metadata injection to be enabled."
        );
        std::process::exit(1);
    }

    let mut ctx = ProtectionContext::new(args.intensity.clamp(0.0, 1.0), seed)
        .with_format(effective_output_format)
        .with_stego_redundancy(args.stego_redundancy.clamp(1, 10))
        .with_jpeg_quality(args.jpeg_quality.clamp(1, 100))
        .with_progressive_jpeg(args.progressive);

    let effective_dmi = legal_dmi_override.or(dmi_value);
    if let Some(dmi) = effective_dmi {
        ctx = ctx.with_dmi(dmi);
    }
    if args.metadata.is_some() {
        ctx = ctx.with_metadata_injection(args.metadata.unwrap());
    } else if legal_metadata.is_some() {
        ctx = ctx.with_metadata_injection(true);
    }
    if args.legal_claims {
        ctx = ctx.with_legal_claims(true);
    }
    if let Some(meta) = legal_metadata {
        ctx = ctx.with_legal_metadata(meta);
    }
    if let Some(key) = mac_key {
        ctx = ctx.with_mac_key(key);
    }

    if args.verbose {
        println!("Protection level: {:?}", protection_level);
        println!("Intensity: {}", ctx.intensity());
        println!("Seed: {}", ctx.seed());
        println!("Stego redundancy: {}", ctx.stego_redundancy());
        if let Some(ref format) = ctx.output_format() {
            println!("Output format: {:?}", format);
        }
        println!("JPEG quality: {}", ctx.jpeg_quality());
        println!("Progressive JPEG: {}", ctx.progressive_jpeg());
        println!("Inject metadata: {:?}", ctx.inject_metadata());
        println!("Inject legal claims: {:?}", ctx.inject_legal_claims());
        println!(
            "MAC key: {}",
            if ctx.mac_key().is_some() {
                "set"
            } else {
                "none"
            }
        );
        if let Some(ref dmi) = ctx.dmi_value() {
            let dmi_val: DmiValue = *dmi;
            println!("DMI: {}", dmi_val.as_str());
        }
        if is_batch {
            println!("Parallel jobs: {}", args.jobs);
        }
    }

    if is_batch {
        use rayon::prelude::*;

        if args.jobs > 1 {
            if let Err(e) = rayon::ThreadPoolBuilder::new()
                .num_threads(args.jobs)
                .build_global()
            {
                if args.verbose {
                    eprintln!(
                        "Warning: Could not set thread count to {}: {}",
                        args.jobs, e
                    );
                }
            }
        }

        let output_dir = args
            .output
            .filter(|p| p.is_dir() || !input_files.iter().any(|i| i == p));

        if args.verbose {
            println!(
                "Processing {} files with {} jobs...",
                input_files.len(),
                args.jobs
            );
        }

        let results: Vec<Result<(PathBuf, PathBuf), (PathBuf, String)>> = if args.jobs > 1 {
            let seen_paths: std::sync::Mutex<HashMap<PathBuf, usize>> =
                std::sync::Mutex::new(HashMap::new());

            input_files
                .par_iter()
                .with_max_len(1)
                .map(|input_path| {
                    let mut seen = seen_paths.lock().unwrap();
                    let override_output = compute_output_path(
                        input_path,
                        &output_dir,
                        effective_output_format,
                        &mut seen,
                    );
                    drop(seen);

                    process_single_file(
                        input_path,
                        &output_dir,
                        effective_output_format,
                        &ctx,
                        protection_level,
                        args.verbose,
                        override_output,
                    )
                    .map(|output| (input_path.clone(), output))
                    .map_err(|e| (input_path.clone(), e.to_string()))
                })
                .collect()
        } else {
            let mut seen: HashMap<PathBuf, usize> = HashMap::new();

            input_files
                .iter()
                .map(|input_path| {
                    let override_output = compute_output_path(
                        input_path,
                        &output_dir,
                        effective_output_format,
                        &mut seen,
                    );

                    process_single_file(
                        input_path,
                        &output_dir,
                        effective_output_format,
                        &ctx,
                        protection_level,
                        args.verbose,
                        override_output,
                    )
                    .map(|output| (input_path.clone(), output))
                    .map_err(|e| (input_path.clone(), e.to_string()))
                })
                .collect()
        };

        let mut success_count = 0;
        let mut failed_files: Vec<PathBuf> = Vec::new();

        for result in results {
            match result {
                Ok((input_path, output_path)) => {
                    success_count += 1;
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
            return Err(format!(
                "{} file(s) failed to process: {}",
                failed_files.len(),
                failed_files
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .into());
        }
    } else {
        let input_path = &input_files[0];

        if args.verbose {
            let input_bytes = fs::read(input_path)?;
            println!("Input size: {} bytes", input_bytes.len());
            let detected_format =
                ImageOutputFormat::from_magic_bytes(&input_bytes).unwrap_or(DEFAULT_OUTPUT_FORMAT);
            println!("Detected format: {:?}", detected_format);
        }

        let output_path = process_single_file(
            input_path,
            &args.output,
            effective_output_format,
            &ctx,
            protection_level,
            args.verbose,
            None,
        )?;

        if args.verbose {
            println!("Output: {:?}", output_path);
            println!("Done!");
        } else {
            println!("{}", output_path.display());
        }
    }

    Ok(())
}
