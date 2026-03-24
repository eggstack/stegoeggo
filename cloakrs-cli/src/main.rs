use clap::{Parser, ValueEnum};
use cloakrs::Error;
use cloakrs::{
    process_image_bytes, DmiValue, ImageOutputFormat, MetadataTrapProtector, ProtectionContext,
    ProtectionLevel, SteganographyProtector, TargetModel, DEFAULT_OUTPUT_FORMAT,
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "cloakrs")]
#[command(about = "Image protection CLI for protecting against AI scraping", long_about = None)]
struct Args {
    #[arg(help = "Input image file(s). Use multiple files or a directory for batch processing")]
    input: Vec<PathBuf>,

    #[arg(
        short,
        long,
        help = "Output directory (for batch processing) or output file (for single file)"
    )]
    output: Option<PathBuf>,

    #[arg(
        short = 'V',
        long,
        help = "Verify if image contains protection signature"
    )]
    verify: bool,

    #[arg(short, long, default_value = "standard", help = "Protection level")]
    level: ProtectionLevelArg,

    #[arg(short, long, help = "Target AI model")]
    target: Option<TargetModelArg>,

    #[arg(
        short,
        long,
        default_value = "0.5",
        help = "Protection intensity (0.0-1.0)"
    )]
    intensity: f32,

    #[arg(short, long, help = "Seed for reproducible results")]
    seed: Option<u64>,

    #[arg(
        short,
        long,
        help = "Output format (png|jpg|webp) - defaults to input format"
    )]
    format: Option<OutputFormatArg>,

    #[arg(
        long,
        default_value = "2",
        help = "Stego embedding redundancy (1-5). Higher = more robust, lower = faster"
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
        help = "DMI metadata value (auto|unspecified|allowed|prohibited-ai|prohibited-genai|prohibited-se|prohibited|prohibited-constraints)"
    )]
    dmi: Option<DmiArg>,

    #[arg(
        long,
        help = "Inject metadata (seed, DMI). Default: true for Standard+, false for Light"
    )]
    metadata: Option<bool>,

    #[arg(
        long,
        help = "Inject legal claims (copyright, artist). WARNING: only for content you own"
    )]
    legal_claims: bool,

    #[arg(
        long,
        help = "Cryptographic key for keyed perturbations (hex string). Provides extra protection."
    )]
    key: Option<String>,

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
    Enhanced,
    Strong,
}

#[derive(Debug, Clone, ValueEnum)]
enum TargetModelArg {
    #[clap(name = "sd15")]
    StableDiffusion15,
    #[clap(name = "sd21")]
    StableDiffusion21,
    #[clap(name = "sdxl")]
    StableDiffusionXL,
    #[clap(name = "dalle")]
    DallE,
    Midjourney,
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
    fn to_dmi_value(self) -> Option<DmiValue> {
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
            ProtectionLevelArg::Enhanced => ProtectionLevel::Enhanced,
            ProtectionLevelArg::Strong => ProtectionLevel::Strong,
        }
    }
}

impl From<TargetModelArg> for TargetModel {
    fn from(arg: TargetModelArg) -> Self {
        match arg {
            TargetModelArg::StableDiffusion15 => TargetModel::StableDiffusion15,
            TargetModelArg::StableDiffusion21 => TargetModel::StableDiffusion21,
            TargetModelArg::StableDiffusionXL => TargetModel::StableDiffusionXL,
            TargetModelArg::DallE => TargetModel::DallE,
            TargetModelArg::Midjourney => TargetModel::Midjourney,
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
        matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" | "tiff" | "tif"
        )
    } else {
        false
    }
}

fn process_single_file(
    input_path: &PathBuf,
    output_dir: &Option<PathBuf>,
    output_format: &Option<ImageOutputFormat>,
    ctx_base: &ProtectionContext,
    protection_level: ProtectionLevel,
    verbose: bool,
) -> Result<PathBuf, Error> {
    let input_bytes = fs::read(input_path).map_err(Error::Io)?;

    let detected_format =
        ImageOutputFormat::from_magic_bytes(&input_bytes).unwrap_or(DEFAULT_OUTPUT_FORMAT);

    let output_fmt = match output_format {
        Some(fmt) => {
            if verbose && *fmt != detected_format {
                eprintln!(
                    "Warning: --format {:?} differs from detected format {:?}",
                    fmt, detected_format
                );
            }
            *fmt
        }
        None => detected_format,
    };

    let mut ctx = ctx_base.clone();
    ctx.input_format = Some(detected_format);

    let output_bytes = process_image_bytes(&input_bytes, protection_level, &ctx)?;

    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = output_fmt.extension();
    let filename = format!("{}_protected.{}", stem, ext);

    let output_path = if let Some(ref dir) = output_dir {
        let out_path = dir.join(&filename);
        fs::create_dir_all(dir)?;
        fs::write(&out_path, &output_bytes)?;
        out_path
    } else {
        let output_path = PathBuf::from(filename);
        fs::write(&output_path, &output_bytes)?;
        output_path
    };

    Ok(output_path)
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
        println!("cloakrs CLI");
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

        let img = image::load_from_memory(&bytes_to_verify)
            .map_err(|e| format!("Failed to load image: {}", e))?;

        let stego = SteganographyProtector::new();

        if args.verbose {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            eprintln!("Image dimensions: {}x{}", w, h);
        }

        let metadata_seed = MetadataTrapProtector::extract_seed_from_image(&bytes_to_verify);

        let is_jpeg = bytes_to_verify.starts_with(&[0xFF, 0xD8]);

        if args.verbose {
            if let Some(seed) = metadata_seed {
                eprintln!("Found seed in metadata: {}", seed);
                if is_jpeg {
                    eprintln!("JPEG detected - using metadata verification");
                }
            } else {
                eprintln!("No seed found in metadata, using stego-only verification");
            }
        }

        let verified = if let Some(seed) = metadata_seed {
            println!("Protected: Yes (verified via metadata)");
            println!("Seed: {}", seed);
            if is_jpeg {
                println!("Note: JPEG - stego may not survive re-encoding");
            }
            true
        } else {
            let p = stego.extract_payload(&img);
            p.is_some()
        };

        if verified {
            if metadata_seed.is_none() {
                println!("Protected: Yes");
                if let Some(payload) = stego.extract_payload(&img) {
                    let level_str = ProtectionLevel::from_byte(payload.protection_level)
                        .map(|l: ProtectionLevel| l.as_str())
                        .unwrap_or("Unknown");
                    println!("Level: {} (id: {})", level_str, payload.protection_level);
                    println!("Seed: {}", payload.seed);
                    println!("Intensity: {:.2}", payload.intensity);
                    println!("Version: {}", payload.version);
                }
            }
        } else {
            println!("Protected: No");
            println!("This image does not contain a protection signature.");
        }
        return Ok(());
    }

    let target = args.target.map(TargetModel::from).unwrap_or_default();

    let seed = args.seed.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    let mac_key = args
        .key
        .as_ref()
        .map(|k| hex::decode(k).map_err(|e| format!("Invalid hex key '{}': {}", k, e)))
        .transpose()?;

    let output_format = args.format.map(|f| match f {
        OutputFormatArg::Png => ImageOutputFormat::Png,
        OutputFormatArg::Jpg => ImageOutputFormat::Jpeg,
        OutputFormatArg::WebP => ImageOutputFormat::WebP,
    });

    let protection_level = ProtectionLevel::from(args.level);

    let dmi_value = args.dmi.and_then(|d| {
        d.to_dmi_value().or_else(|| {
            // Auto-select DMI based on protection level
            Some(match protection_level {
                ProtectionLevel::Disabled | ProtectionLevel::Light => DmiValue::Unspecified,
                _ => DmiValue::ProhibitedAiMlTraining,
            })
        })
    });

    let mut ctx = ProtectionContext::new(target, args.intensity.clamp(0.0, 1.0), seed)
        .with_format(output_format.unwrap_or(cloakrs::ImageOutputFormat::Png))
        .with_stego_redundancy(args.stego_redundancy.clamp(1, 5))
        .with_jpeg_quality(args.jpeg_quality.clamp(1, 100))
        .with_progressive_jpeg(args.progressive);

    if let Some(dmi) = dmi_value {
        ctx = ctx.with_dmi(dmi);
    }
    if let Some(metadata) = args.metadata {
        ctx = ctx.with_metadata_injection(metadata);
    }
    if args.legal_claims {
        ctx = ctx.with_legal_claims(true);
    }
    if let Some(key) = mac_key {
        ctx = ctx.with_mac_key(key);
    }

    if args.verbose {
        println!("Protection level: {:?}", protection_level);
        println!("Target model: {}", ctx.target.as_str());
        println!("Intensity: {}", ctx.intensity);
        println!("Seed: {}", ctx.seed);
        println!("Stego redundancy: {}", ctx.stego_redundancy);
        if let Some(ref format) = ctx.output_format {
            println!("Output format: {:?}", format);
        }
        println!("JPEG quality: {}", ctx.jpeg_quality);
        println!("Progressive JPEG: {}", ctx.progressive_jpeg);
        println!("Inject metadata: {:?}", ctx.inject_metadata);
        println!("Inject legal claims: {:?}", ctx.inject_legal_claims);
        println!(
            "MAC key: {}",
            if ctx.mac_key().is_some() {
                "set"
            } else {
                "none"
            }
        );
        if let Some(ref dmi) = ctx.dmi_value {
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
            rayon::ThreadPoolBuilder::new()
                .num_threads(args.jobs)
                .build_global()
                .unwrap_or(());
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

        let results: Vec<Result<(PathBuf, PathBuf), String>> = if args.jobs > 1 {
            input_files
                .par_iter()
                .with_max_len(1)
                .map(|input_path| {
                    process_single_file(
                        input_path,
                        &output_dir,
                        &output_format,
                        &ctx,
                        protection_level,
                        args.verbose,
                    )
                    .map(|output| (input_path.clone(), output))
                    .map_err(|e| e.to_string())
                })
                .collect()
        } else {
            input_files
                .iter()
                .map(|input_path| {
                    process_single_file(
                        input_path,
                        &output_dir,
                        &output_format,
                        &ctx,
                        protection_level,
                        args.verbose,
                    )
                    .map(|output| (input_path.clone(), output))
                    .map_err(|e| e.to_string())
                })
                .collect()
        };

        let mut success_count = 0;
        let mut failed_count = 0;

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
                Err(e) => {
                    failed_count += 1;
                    eprintln!("Error: {}", e);
                }
            }
        }

        if args.verbose || failed_count > 0 {
            println!(
                "\nCompleted: {} succeeded, {} failed",
                success_count, failed_count
            );
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
            &output_format,
            &ctx,
            protection_level,
            args.verbose,
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
