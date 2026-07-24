use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use stegoeggo::Error;
#[allow(deprecated)]
use stegoeggo::{
    generate_random_seed, process_image_bytes_with_warnings, process_request_bytes_with_warnings,
    verify_legal_notice, DmiValue, EvidenceProfile, HiddenMarkerMode, ImageOutputFormat,
    ProtectionChannels, ProtectionContext, ProtectionLevel, ProtectionPreset, ProtectionWarning,
    RightsPolicy, StegoPayload, WarningSeverity, DEFAULT_OUTPUT_FORMAT,
};

const EXIT_OK: i32 = 0;
const EXIT_ERROR: i32 = 1;
const EXIT_CONFIG: i32 = 2;
const EXIT_INTEGRITY: i32 = 3;
const EXIT_INTERNAL: i32 = 5;

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

    #[arg(
        long,
        help = "Verify legal-notice report: check metadata fields, stego integrity, evidence strength, and channels"
    )]
    verify: bool,

    #[arg(short, long, default_value = "standard", help = "Protection level")]
    level: ProtectionLevelArg,

    #[arg(
        short,
        long,
        default_value = "legal-notice",
        help = "Evidence profile: legal-notice, legal-notice-stego, authenticated-provenance, maximal"
    )]
    profile: ProfileArg,

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
        help = "Output format (png|jpg|webp) - defaults to preserving input format"
    )]
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

    #[arg(
        long,
        alias = "copyright-holder",
        help = "Copyright notice text (e.g., '© 2024 Jane Doe. All rights reserved.')"
    )]
    copyright_notice: Option<String>,

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

    #[arg(
        long,
        help = "Shorthand: reserve text and data mining rights [DEPRECATED: TDMRep deployment artifacts deferred; sets DMI ProhibitedSeeConstraints instead]"
    )]
    tdm_reserved: bool,

    #[arg(
        long,
        help = "Required credit line text (e.g., 'Photo by Jane Doe / Acme Corp')"
    )]
    credit_line: Option<String>,

    #[arg(
        long,
        help = "Copyright owner name (distinct from copyright holder notice text)"
    )]
    copyright_owner: Option<String>,

    #[arg(long, help = "Licensor name for PLUS structured rights")]
    licensor_name: Option<String>,

    #[arg(long, help = "Licensor email for PLUS structured rights")]
    licensor_email: Option<String>,

    #[arg(long, help = "Licensor URL for PLUS structured rights")]
    licensor_url: Option<String>,

    #[arg(long, help = "Content creation date (ISO 8601, e.g., '2024-01-15')")]
    content_created_at: Option<String>,

    #[arg(
        long,
        help = "Cryptographic key for HMAC authentication. Accepts: hex string, @/path/to/file (hex in file), - (stdin), or env STEGOEGGO_KEY"
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

    #[arg(
        long,
        help = "Exit with error if any warnings have error severity for the active evidence profile"
    )]
    strict: bool,

    #[arg(long, help = "Output results as JSON (machine-readable)")]
    json: bool,

    #[arg(
        long,
        value_enum,
        help = "Explicit rights policy (new API, replaces --dmi)"
    )]
    rights_policy: Option<RightsPolicyArg>,

    #[arg(
        long,
        value_enum,
        help = "Executable preset (new API, replaces --level + --profile)"
    )]
    preset: Option<PresetArg>,

    #[arg(long, value_enum, help = "Hidden marker mode (new API)")]
    hidden_marker: Option<HiddenMarkerArg>,

    #[arg(long, value_enum, help = "Authentication mode (new API)")]
    authentication: Option<AuthenticationArg>,

    #[arg(long, help = "Dry run: show resolved plan without processing")]
    dry_run: bool,

    #[cfg(feature = "signatures")]
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[cfg(feature = "signatures")]
    #[command(about = "Generate a new Ed25519 key pair")]
    Keygen {
        #[arg(long, default_value = ".", help = "Directory to write key files")]
        output_dir: PathBuf,

        #[arg(long, help = "Optional key identifier label")]
        key_id: Option<String>,
    },

    #[cfg(feature = "signatures")]
    #[command(about = "Sign a detached manifest")]
    Sign {
        #[arg(long, help = "Path to the detached manifest JSON")]
        manifest: PathBuf,

        #[arg(long, help = "Path to the private key file")]
        key: PathBuf,

        #[arg(long, help = "Output file (default: overwrite manifest)")]
        output: Option<PathBuf>,
    },

    #[cfg(feature = "signatures")]
    #[command(about = "Verify a detached manifest")]
    VerifyManifest {
        #[arg(long, help = "Path to the detached manifest JSON")]
        manifest: PathBuf,

        #[arg(long, help = "Path to the image file")]
        image: PathBuf,

        #[arg(long, help = "Path to public key file for signature verification")]
        key: Option<PathBuf>,

        #[arg(long, help = "Hex-encoded HMAC key for embedded payload verification")]
        payload_key: Option<String>,
    },
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

#[derive(Debug, Clone, ValueEnum)]
enum ProfileArg {
    LegalNotice,
    LegalNoticeStego,
    AuthenticatedProvenance,
    Maximal,
}

#[allow(deprecated)]
impl From<ProfileArg> for EvidenceProfile {
    fn from(arg: ProfileArg) -> Self {
        match arg {
            ProfileArg::LegalNotice => EvidenceProfile::LegalNotice,
            ProfileArg::LegalNoticeStego => EvidenceProfile::LegalNoticeWithStego,
            ProfileArg::AuthenticatedProvenance => EvidenceProfile::AuthenticatedProvenance,
            ProfileArg::Maximal => EvidenceProfile::Maximal,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum RightsPolicyArg {
    Unspecified,
    Allowed,
    ProhibitedAiMlTraining,
    ProhibitedGenerativeAiTraining,
    ProhibitedExceptSearchIndexing,
    ProhibitedAllDataMining,
    ProhibitedSeeConstraints,
}

impl From<RightsPolicyArg> for RightsPolicy {
    fn from(arg: RightsPolicyArg) -> Self {
        match arg {
            RightsPolicyArg::Unspecified => RightsPolicy::Unspecified,
            RightsPolicyArg::Allowed => RightsPolicy::Allowed,
            RightsPolicyArg::ProhibitedAiMlTraining => RightsPolicy::ProhibitedAiMlTraining,
            RightsPolicyArg::ProhibitedGenerativeAiTraining => {
                RightsPolicy::ProhibitedGenerativeAiTraining
            }
            RightsPolicyArg::ProhibitedExceptSearchIndexing => {
                RightsPolicy::ProhibitedExceptSearchIndexing
            }
            RightsPolicyArg::ProhibitedAllDataMining => RightsPolicy::ProhibitedAllDataMining,
            RightsPolicyArg::ProhibitedSeeConstraints => RightsPolicy::ProhibitedSeeConstraints,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum PresetArg {
    LegalNotice,
    LegalNoticeWithStego,
    AuthenticatedProvenance,
    Maximal,
}

impl From<PresetArg> for ProtectionPreset {
    fn from(arg: PresetArg) -> Self {
        match arg {
            PresetArg::LegalNotice => ProtectionPreset::LegalNotice,
            PresetArg::LegalNoticeWithStego => ProtectionPreset::LegalNoticeWithStego,
            PresetArg::AuthenticatedProvenance => ProtectionPreset::AuthenticatedProvenance,
            PresetArg::Maximal => ProtectionPreset::Maximal,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum HiddenMarkerArg {
    Disabled,
    BestEffort,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum AuthenticationArg {
    None,
    Hmac,
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

/// Resolve a key from multiple sources with the following priority:
/// 1. Explicit CLI argument (--key <hex>)
/// 2. File path (--key @/path/to/file, reads raw hex from file)
/// 3. Environment variable (STEGOEGGO_KEY)
/// 4. Stdin (when --key is "-")
fn resolve_key_input(
    key_arg: &Option<String>,
    env_var: &str,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    if let Some(ref key_str) = key_arg {
        if key_str == "-" {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let hex_key = input.trim();
            return Ok(Some(
                hex::decode(hex_key).map_err(|e| format!("Invalid hex key from stdin: {}", e))?,
            ));
        }
        if let Some(path_str) = key_str.strip_prefix('@') {
            let path = Path::new(path_str);
            if !path.exists() {
                return Err(format!("Key file not found: {}", path_str).into());
            }
            let contents = fs::read_to_string(path)
                .map_err(|e| format!("Failed to read key file '{}': {}", path_str, e))?;
            let hex_key = contents.trim().replace(['\n', '\r'], "");
            return Ok(Some(
                hex::decode(&hex_key).map_err(|e| format!("Invalid hex key in file: {}", e))?,
            ));
        }
        return Ok(Some(
            hex::decode(key_str).map_err(|e| format!("Invalid hex key: {}", e))?,
        ));
    }

    if let Ok(env_val) = std::env::var(env_var) {
        if !env_val.is_empty() {
            return Ok(Some(hex::decode(&env_val).map_err(|e| {
                format!("Invalid hex key from {}: {}", env_var, e)
            })?));
        }
    }

    Ok(None)
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

fn write_atomic(path: &Path, data: &[u8]) -> Result<(), Error> {
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

fn check_input_output_disjoint(input: &Path, output: &Path) -> Result<(), Error> {
    let input_canonical = input.canonicalize().map_err(|e| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("resolve input path: {e}"),
        ))
    })?;
    let output_parent = output.parent().unwrap_or_else(|| Path::new("."));
    let output_canonical = output
        .canonicalize()
        .or_else(|_| output_parent.canonicalize())
        .map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("resolve output path: {e}"),
            ))
        })?;
    if input_canonical == output_canonical {
        return Err(Error::Config(
            "Input and output paths resolve to the same file; use --output to specify a different path".to_string(),
        ));
    }
    Ok(())
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

fn display_warnings(warnings: &[ProtectionWarning], ctx: &ProtectionContext, verbose: bool) {
    if warnings.is_empty() {
        return;
    }
    let profile = ctx.evidence_profile();
    for w in warnings {
        let severity = w.severity_for_profile(profile);
        let prefix = match severity {
            WarningSeverity::Error => "Error",
            WarningSeverity::Warning => "Warning",
            WarningSeverity::Info => "Info",
        };
        if verbose || severity != WarningSeverity::Info {
            eprintln!("[{}] {}", prefix, w);
        }
    }
}

fn process_single_file(
    input_path: &PathBuf,
    output_dir: &Option<PathBuf>,
    output_format: Option<ImageOutputFormat>,
    ctx_base: &ProtectionContext,
    protection_level: ProtectionLevel,
    verbose: bool,
    override_output: Option<PathBuf>,
) -> Result<(PathBuf, Vec<ProtectionWarning>), Error> {
    let input_bytes = fs::read(input_path).map_err(Error::Io)?;

    let detected_format =
        ImageOutputFormat::from_magic_bytes(&input_bytes).unwrap_or(DEFAULT_OUTPUT_FORMAT);

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

    let mut ctx = ctx_base.clone();
    ctx.set_input_format(detected_format);

    let (output_bytes, warnings) =
        process_image_bytes_with_warnings(&input_bytes, protection_level, &ctx)?;

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
            let out_path = if dir.is_file() || (dir.extension().is_some() && is_image_file(dir)) {
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

fn process_single_file_request(
    input_path: &PathBuf,
    output_dir: &Option<PathBuf>,
    output_format: Option<ImageOutputFormat>,
    request: &stegoeggo::ProtectionRequest,
    verbose: bool,
    override_output: Option<PathBuf>,
) -> Result<(PathBuf, Vec<ProtectionWarning>), Error> {
    let input_bytes = fs::read(input_path).map_err(Error::Io)?;

    let detected_format =
        ImageOutputFormat::from_magic_bytes(&input_bytes).unwrap_or(DEFAULT_OUTPUT_FORMAT);

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

    let (output_bytes, warnings) = process_request_bytes_with_warnings(&input_bytes, request)?;

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
            let out_path = if dir.is_file() || (dir.extension().is_some() && is_image_file(dir)) {
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
    let has_legal_flags = args.copyright_notice.is_some()
        || args.creator.is_some()
        || args.contact.is_some()
        || args.rights_url.is_some()
        || args.usage_terms.is_some()
        || args.ai_constraints.is_some()
        || args.no_ai_training
        || args.no_genai_training
        || args.tdm_reserved
        || args.credit_line.is_some()
        || args.copyright_owner.is_some()
        || args.licensor_name.is_some()
        || args.licensor_email.is_some()
        || args.licensor_url.is_some()
        || args.content_created_at.is_some();

    if !has_legal_flags {
        return (None, None);
    }

    let mut meta = stegoeggo::LegalMetadata::default();
    let mut dmi_override: Option<DmiValue> = None;

    if let Some(ref v) = args.copyright_notice {
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
    if let Some(ref v) = args.credit_line {
        meta = meta.with_credit_line(v);
    }
    if let Some(ref v) = args.copyright_owner {
        meta = meta.with_copyright_owner(v);
    }
    if let Some(ref v) = args.licensor_name {
        meta = meta.with_licensor_name(v);
    }
    if let Some(ref v) = args.licensor_email {
        meta = meta.with_licensor_email(v);
    }
    if let Some(ref v) = args.licensor_url {
        meta = meta.with_licensor_url(v);
    }
    if let Some(ref v) = args.content_created_at {
        meta = meta.with_creation_date(v);
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

#[cfg(feature = "signatures")]
fn handle_keygen(
    output_dir: &PathBuf,
    key_id: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    use stegoeggo::signing::SigningKey;

    let key = SigningKey::generate();
    let verifying_key = key.verifying_key();

    let key_id_hex = key_id
        .as_deref()
        .map(|id| id.to_string())
        .unwrap_or_else(|| hex::encode(key.key_id()));

    let private_path = output_dir.join("key_private.pem");
    let public_path = output_dir.join("key_public.pem");

    fs::create_dir_all(output_dir)?;

    let private_pem = format!(
        "-----BEGIN STEGOEGGO PRIVATE KEY-----\nkey_id:{}\n{}\n-----END STEGOEGGO PRIVATE KEY-----\n",
        key_id_hex,
        hex::encode(key.to_bytes())
    );
    fs::write(&private_path, private_pem.as_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&private_path, fs::Permissions::from_mode(0o600))?;
    }

    let public_pem = format!(
        "-----BEGIN STEGOEGGO PUBLIC KEY-----\nkey_id:{}\n{}\n-----END STEGOEGGO PUBLIC KEY-----\n",
        key_id_hex,
        hex::encode(verifying_key.as_bytes())
    );
    fs::write(&public_path, public_pem.as_bytes())?;

    println!("Key pair generated:");
    println!("  Private key: {}", private_path.display());
    println!("  Public key:  {}", public_path.display());
    println!("  Key ID:      {}", key_id_hex);

    Ok(())
}

#[cfg(feature = "signatures")]
fn handle_sign(
    manifest_path: &PathBuf,
    key_path: &PathBuf,
    output: &Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    use stegoeggo::detached::{DetachedManifest, PublicKeyEntry, SignatureRecord};
    use stegoeggo::resource_limits::ResourceLimits;
    use stegoeggo::signing::SigningKey;

    let key_bytes = fs::read(key_path)?;
    let key_str = String::from_utf8_lossy(&key_bytes);

    let hex_key = extract_pem_field(&key_str, "BEGIN STEGOEGGO PRIVATE KEY")
        .and_then(|block| {
            let key_id = block
                .lines()
                .find(|l| l.starts_with("key_id:"))
                .map(|l| l.strip_prefix("key_id:").unwrap_or("").to_string());
            let key_hex = block
                .lines()
                .find(|l| !l.starts_with("key_id:"))
                .map(String::from);
            key_hex.map(|k| (k, key_id.unwrap_or_default()))
        })
        .unwrap_or_else(|| {
            (
                String::from_utf8_lossy(&key_bytes).trim().to_string(),
                String::new(),
            )
        });

    let key_bytes_vec = hex::decode(&hex_key.0)
        .map_err(|e| format!("Invalid hex key data in {}: {}", key_path.display(), e))?;
    if key_bytes_vec.len() != 32 {
        return Err(format!("Private key must be 32 bytes, got {}", key_bytes_vec.len()).into());
    }
    let mut raw_key = [0u8; 32];
    raw_key.copy_from_slice(&key_bytes_vec);

    let signing_key = SigningKey::from_bytes(raw_key, hex_key.1.into_bytes())
        .map_err(|e| format!("Invalid signing key: {}", e))?;

    let manifest_bytes = fs::read(manifest_path)?;
    let limits = ResourceLimits::default();
    let mut manifest = DetachedManifest::from_json_with_limits(&manifest_bytes, &limits)
        .map_err(|e| format!("Manifest parsing failed: {}", e))?;

    let claim_bytes = manifest.claim.canonical_bytes();
    let signature_bytes = signing_key.sign(&claim_bytes);
    let signature_hex = hex::encode(&signature_bytes);

    let key_id = signing_key.verifying_key().key_id().to_vec();

    let sig_record = SignatureRecord {
        algorithm: "ed25519".to_string(),
        key_id,
        signature: signature_hex,
    };
    manifest = manifest.with_signature(sig_record);

    let public_key = signing_key.verifying_key();
    let pub_entry = PublicKeyEntry {
        key_id: public_key.key_id().to_vec(),
        algorithm: "ed25519".to_string(),
        key_bytes: hex::encode(public_key.as_bytes()),
    };
    manifest = manifest.with_public_key(pub_entry);

    let signed_json = serde_json::to_string_pretty(&manifest)?;
    let out_path = output.as_ref().unwrap_or(manifest_path);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_atomic(out_path, signed_json.as_bytes())?;

    println!("Manifest signed: {}", out_path.display());
    Ok(())
}

#[cfg(feature = "signatures")]
fn handle_verify_manifest(
    manifest_path: &PathBuf,
    image_path: &PathBuf,
    key_path: &Option<PathBuf>,
    _payload_key: Option<String>,
    json_output: bool,
) -> Result<i32, Box<dyn std::error::Error>> {
    use stegoeggo::detached::verify::{
        verify_detached_manifest, EmbeddedReferenceStatus, TrustPolicy,
    };
    use stegoeggo::detached::DetachedManifest;
    use stegoeggo::resource_limits::ResourceLimits;
    use stegoeggo::signing::VerifyingKey;

    let manifest_bytes = fs::read(manifest_path)?;
    let limits = ResourceLimits::default();
    let manifest = DetachedManifest::from_json_with_limits(&manifest_bytes, &limits)
        .map_err(|e| format!("Manifest parsing failed: {}", e))?;

    let image_bytes = fs::read(image_path)?;

    let trust = if let Some(ref key_file) = key_path {
        let pub_key_bytes = fs::read(key_file)?;
        let pub_key_str = String::from_utf8_lossy(&pub_key_bytes);

        let (hex_pub, key_id_hex) = extract_pem_field(&pub_key_str, "BEGIN STEGOEGGO PUBLIC KEY")
            .and_then(|block| {
                let key_id = block
                    .lines()
                    .find(|l| l.starts_with("key_id:"))
                    .map(|l| l.strip_prefix("key_id:").unwrap_or("").to_string());
                let key_hex = block
                    .lines()
                    .find(|l| !l.starts_with("key_id:"))
                    .map(String::from);
                key_hex.map(|k| (k, key_id.unwrap_or_default()))
            })
            .unwrap_or_else(|| {
                (
                    String::from_utf8_lossy(&pub_key_bytes).trim().to_string(),
                    String::new(),
                )
            });

        let pub_bytes_vec =
            hex::decode(&hex_pub).map_err(|e| format!("Invalid hex in public key file: {}", e))?;
        if pub_bytes_vec.len() != 32 {
            return Err(format!("Public key must be 32 bytes, got {}", pub_bytes_vec.len()).into());
        }
        let mut raw_pub = [0u8; 32];
        raw_pub.copy_from_slice(&pub_bytes_vec);
        let vk = VerifyingKey::from_bytes(raw_pub, key_id_hex.into_bytes());
        TrustPolicy::TrustKeys(vec![vk.key_id().to_vec()])
    } else {
        TrustPolicy::TrustNone
    };

    let result = verify_detached_manifest(&image_bytes, &manifest, &trust);

    let overall = result.overall_status();

    if json_output {
        let status_str = match overall {
            stegoeggo::detached::DetachedOverallStatus::VerifiedTrusted => "verified_trusted",
            stegoeggo::detached::DetachedOverallStatus::VerifiedUntrusted => "verified_untrusted",
            stegoeggo::detached::DetachedOverallStatus::InvalidConfiguration => {
                "invalid_configuration"
            }
            stegoeggo::detached::DetachedOverallStatus::BindingFailure => "binding_failure",
            stegoeggo::detached::DetachedOverallStatus::SignatureFailure => "signature_failure",
            stegoeggo::detached::DetachedOverallStatus::EmbeddedReferenceFailure => {
                "embedded_reference_failure"
            }
        };

        #[derive(serde::Serialize)]
        struct JsonManifestVerify {
            schema_version: u32,
            overall_status: &'static str,
            instance_digest_match: bool,
            manifest_valid: bool,
            embedded_reference: &'static str,
            signatures_valid: bool,
            trusted: bool,
            evidence_strength: String,
        }

        let embedded_ref = match result.embedded_reference_status {
            EmbeddedReferenceStatus::NotProvided => "not_provided",
            EmbeddedReferenceStatus::Stripped => "stripped",
            EmbeddedReferenceStatus::VersionMismatch => "version_mismatch",
            EmbeddedReferenceStatus::DigestMismatch => "digest_mismatch",
            EmbeddedReferenceStatus::Malformed => "malformed",
            #[allow(deprecated)]
            EmbeddedReferenceStatus::Present => "present",
            EmbeddedReferenceStatus::PresentValid => "present_valid",
            EmbeddedReferenceStatus::AuthenticationKeyMissing => "authentication_key_missing",
            EmbeddedReferenceStatus::AuthenticationFailed => "authentication_failed",
            EmbeddedReferenceStatus::UnsupportedVersion => "unsupported_version",
        };

        let sigs_valid = result
            .report
            .signatures()
            .iter()
            .any(|s| s.cryptographically_valid());
        let trusted = result.report.trust().trusted();

        let json = JsonManifestVerify {
            schema_version: manifest.schema_version as u32,
            overall_status: status_str,
            instance_digest_match: result.instance_digest_match,
            manifest_valid: result.manifest_valid,
            embedded_reference: embedded_ref,
            signatures_valid: sigs_valid,
            trusted,
            evidence_strength: format!("{:?}", result.report.evidence_strength()),
        };
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Manifest schema version: {}", manifest.schema_version);
        println!("Claim ID: {}", hex::encode(manifest.claim.claim_id));
        println!("Instance digest: {}", manifest.claim.instance_digest);
        println!("Format: {}", manifest.claim.format);
        println!(
            "Dimensions: {}x{}",
            manifest.claim.width, manifest.claim.height
        );
        println!("File size: {} bytes", manifest.claim.file_size);
        println!("Rights policy: {}", manifest.claim.rights_policy);
        println!("Software: {}", manifest.claim.software);

        if result.instance_digest_match {
            println!("\nImage digest: MATCH");
        } else {
            println!("\nImage digest: MISMATCH");
        }

        println!(
            "\nManifest valid: {}",
            if result.manifest_valid { "YES" } else { "NO" }
        );

        if manifest.signatures.is_empty() {
            println!("Signatures: None");
        } else {
            println!("Signatures: {} total", manifest.signatures.len());
            for (i, sig) in result.report.signatures().iter().enumerate() {
                println!("  [{}] algorithm: ed25519", i);
                println!(
                    "      key_id: {}",
                    hex::encode(sig.public_key_id().unwrap_or(&[]))
                );
                println!(
                    "      cryptographically_valid: {}",
                    sig.cryptographically_valid()
                );
                println!("      trusted: {}", sig.trusted());
            }
        }

        println!(
            "\nEmbedded reference: {:?}",
            result.embedded_reference_status
        );
        println!(
            "Trust: {}",
            if result.report.trust().trusted() {
                "TRUSTED"
            } else {
                "UNTRUSTED"
            }
        );
        println!("Evidence strength: {:?}", result.report.evidence_strength());
        println!("Overall status: {:?}", overall);

        for diag in result.report.diagnostics() {
            println!("  [{:?}] {}", diag.level(), diag.message());
        }
    }

    Ok(overall.exit_code())
}

#[cfg(feature = "signatures")]
fn extract_pem_field(pem_str: &str, begin_tag: &str) -> Option<String> {
    let start_marker = format!("-----{}-----", begin_tag);
    let end_marker = start_marker.replacen("BEGIN", "END", 1);

    let start = pem_str.find(&start_marker)? + start_marker.len();
    let end = pem_str.find(&end_marker)?;
    Some(pem_str[start..end].trim().to_string())
}

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

fn classify_error(e: &(dyn std::error::Error + 'static)) -> i32 {
    if let Some(e) = e.downcast_ref::<stegoeggo::Error>() {
        match e {
            Error::Config(_) => EXIT_CONFIG,
            Error::InputTooLarge { .. } | Error::DimensionsExceeded { .. } => EXIT_CONFIG,
            Error::ContainerLimitExceeded { .. } | Error::MetadataLimitExceeded { .. } => {
                EXIT_CONFIG
            }
            Error::PayloadVerification(_) | Error::Crypto(_) => EXIT_INTEGRITY,
            Error::ImageDecode(_)
            | Error::ImageEncode(_)
            | Error::ImageTruncated(_)
            | Error::Steganography(_)
            | Error::InvalidFormat(_) => EXIT_ERROR,
            Error::Metadata(_) => EXIT_ERROR,
            Error::Io(_) => EXIT_ERROR,
            Error::Serialization(_) => EXIT_CONFIG,
            Error::Iscc(_) => EXIT_ERROR,
            Error::VerificationBudgetExceeded { .. } => EXIT_CONFIG,
            _ => EXIT_INTERNAL,
        }
    } else {
        EXIT_ERROR
    }
}

#[derive(serde::Serialize)]
struct JsonOutput {
    schema_version: u32,
    status: String,
    output_path: Option<String>,
    warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<JsonExecutionReport>,
}

#[derive(serde::Serialize)]
struct JsonExecutionReport {
    effective_policy: String,
    effective_dmi: Option<String>,
    metadata_injected: bool,
    stego_attempted: bool,
    stego_succeeded: bool,
    format_transcoded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_usage: Option<JsonResourceUsage>,
}

#[derive(serde::Serialize)]
struct JsonResourceUsage {
    input_bytes: usize,
    png_chunks_scanned: usize,
    jpeg_segments_scanned: usize,
    webp_riff_chunks_scanned: usize,
    xmp_bytes_parsed: usize,
    metadata_fields_extracted: usize,
    metadata_bytes_copied: usize,
    tile_origins_checked: usize,
    verification_seeds_tried: usize,
    peak_allocations_bytes: usize,
}

#[derive(serde::Serialize)]
struct JsonVerifyOutput {
    schema_version: u32,
    status: String,
    copyright_holder: Option<String>,
    rights_url: Option<String>,
    ai_constraints: Option<String>,
    stego_status: String,
    evidence_strength: String,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    #[cfg(feature = "signatures")]
    if let Some(ref cmd) = args.command {
        return match cmd {
            Command::Keygen { output_dir, key_id } => handle_keygen(output_dir, key_id),
            Command::Sign {
                manifest,
                key,
                output,
            } => handle_sign(manifest, key, output),
            Command::VerifyManifest {
                manifest,
                image,
                key,
                payload_key,
            } => {
                let exit_code =
                    handle_verify_manifest(manifest, image, key, payload_key.clone(), args.json)?;
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

        let mac_key = args
            .key
            .as_ref()
            .map(|k| hex::decode(k).map_err(|e| format!("Invalid hex key: {}", e)))
            .transpose()?
            .unwrap_or_default();

        let notice = verify_legal_notice(&bytes_to_verify, &mac_key);

        if args.json {
            let json_output = JsonVerifyOutput {
                schema_version: 1,
                status: "ok".to_string(),
                copyright_holder: notice.copyright_holder().map(String::from),
                rights_url: notice.rights_url().map(String::from),
                ai_constraints: notice.ai_constraints().map(String::from),
                stego_status: format!("{:?}", notice.stego_status()),
                evidence_strength: notice.evidence_strength().to_string(),
            };
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        } else {
            println!(
                "Rights notice: {}",
                if notice.has_notice() {
                    "Found"
                } else {
                    "Not found"
                }
            );
            if let Some(holder) = notice.copyright_holder() {
                println!("Copyright holder: {}", holder);
            }
            if let Some(creator) = notice.creator() {
                println!("Creator: {}", creator);
            }
            if let Some(contact) = notice.contact() {
                println!("Contact: {}", contact);
            }
            if let Some(url) = notice.rights_url() {
                println!("Rights URL: {}", url);
            }
            if let Some(dmi) = notice.dmi() {
                println!("AI training restriction: {}", dmi.as_str());
            }
            if let Some(canonical) = notice.canonical_dmi() {
                println!("Canonical DMI: {}", canonical.as_str());
            }
            if let Some(legacy) = notice.legacy_dmi() {
                println!("Legacy DMI: {}", legacy.as_str());
            }
            if notice.has_dmi_conflict() {
                println!("DMI conflict: YES (canonical and legacy values disagree)");
            }
            if let Some(reserved) = notice.tdm_reserved() {
                println!(
                    "TDM reservation: {}",
                    if reserved { "reserved" } else { "not reserved" }
                );
            }
            if let Some(terms) = notice.usage_terms() {
                println!("Usage terms: {}", terms);
            }
            if let Some(line) = notice.credit_line() {
                println!("Credit line: {}", line);
            }
            if let Some(owner) = notice.copyright_owner() {
                println!("Copyright owner: {}", owner);
            }
            if let Some(name) = notice.licensor_name() {
                println!("Licensor name: {}", name);
            }
            if let Some(email) = notice.licensor_email() {
                println!("Licensor email: {}", email);
            }
            if let Some(url) = notice.licensor_url() {
                println!("Licensor URL: {}", url);
            }
            if let Some(date) = notice.metadata_date() {
                println!("Metadata date: {}", date);
            }
            if let Some(ts) = notice.notice_applied_at() {
                println!("Notice applied at: {}", ts);
            }
            if let Some(seed) = notice.protection_seed() {
                println!("Protection seed: {}", seed);
            }

            println!();

            match notice.stego_status() {
                stegoeggo::VerificationStatus::Verified => {
                    println!("Stego marker: Found, checksum verified");
                }
                stegoeggo::VerificationStatus::Invalid => {
                    println!("Stego marker: Found, but integrity check failed");
                }
                stegoeggo::VerificationStatus::NotFound => {
                    println!("Stego marker: Not found");
                }
            }

            if notice.authenticated() {
                println!("Authenticated provenance: Verified");
            } else if args.key.is_some() {
                println!(
                    "Authenticated provenance: Not verified (key provided but HMAC check failed)"
                );
            } else {
                println!("Authenticated provenance: Not configured");
            }

            println!("Evidence strength: {}", notice.evidence_strength());

            if let Some(payload) = notice.stego_payload() {
                println!();
                print_payload_info(payload);
            }
        }

        return Ok(());
    }

    // New request-based API path
    if args.rights_policy.is_some()
        || args.preset.is_some()
        || args.hidden_marker.is_some()
        || args.authentication.is_some()
        || args.dry_run
    {
        let seed = args.seed.unwrap_or_else(generate_random_seed);
        let mac_key = resolve_key_input(&args.key, "STEGOEGGO_KEY")?;

        let (legal_metadata, _) = build_legal_metadata(&args);

        let policy = args
            .rights_policy
            .map(RightsPolicy::from)
            .unwrap_or(RightsPolicy::Unspecified);

        let notice = stegoeggo::RightsNotice::default();

        let channels = if let Some(preset_arg) = args.preset {
            let preset: ProtectionPreset = preset_arg.into();
            preset.to_channels()
        } else {
            let hidden = args
                .hidden_marker
                .map(|h| match h {
                    HiddenMarkerArg::Disabled => HiddenMarkerMode::Disabled,
                    HiddenMarkerArg::BestEffort => HiddenMarkerMode::BestEffort,
                })
                .unwrap_or(HiddenMarkerMode::Disabled);

            let auth = args
                .authentication
                .map(|a| match a {
                    AuthenticationArg::None => stegoeggo::AuthenticationMode::None,
                    AuthenticationArg::Hmac => stegoeggo::AuthenticationMode::Hmac,
                })
                .unwrap_or(stegoeggo::AuthenticationMode::None);

            let rights_metadata = policy != RightsPolicy::Unspecified
                || notice.has_legal_content()
                || legal_metadata.is_some()
                || !matches!(hidden, HiddenMarkerMode::Disabled);

            ProtectionChannels {
                rights_metadata,
                hidden_marker: hidden,
                authentication: auth,
            }
        };

        let output_format: Option<ImageOutputFormat> = args.format.map(ImageOutputFormat::from);

        let input_files = collect_input_files(&args.input);
        if input_files.is_empty() {
            eprintln!("Error: No input files found");
            std::process::exit(EXIT_CONFIG);
        }

        let mut request = stegoeggo::ProtectionRequest::new(notice, policy, channels)
            .with_seed(seed)
            .with_intensity(args.intensity.clamp(0.0, 1.0))
            .with_jpeg_quality(args.jpeg_quality.clamp(1, 100));

        if let Some(fmt) = output_format {
            request = request.with_output_format(fmt);
        }

        if args.progressive {
            request = request.with_progressive_jpeg();
        }
        if let Some(meta) = legal_metadata {
            request = request.with_legal_metadata(meta);
        }
        if let Some(key) = mac_key {
            request = request.with_mac_key(key);
        }

        let is_batch = input_files.len() > 1 || args.input.iter().any(|p| p.is_dir());

        if is_batch {
            use rayon::prelude::*;

            #[allow(deprecated)]
            let ctx_for_display = ProtectionContext::new(args.intensity.clamp(0.0, 1.0), seed)
                .with_evidence_profile(EvidenceProfile::LegalNotice);

            #[allow(clippy::type_complexity)]
            let results: Vec<
                Result<(PathBuf, PathBuf, Vec<ProtectionWarning>), (PathBuf, String)>,
            > = if args.jobs > 1 {
                let seen_paths: std::sync::Mutex<HashMap<PathBuf, usize>> =
                    std::sync::Mutex::new(HashMap::new());

                input_files
                    .par_iter()
                    .with_max_len(1)
                    .map(|input_path| {
                        let input_bytes_preview = fs::read(input_path)
                            .map_err(|e| (input_path.clone(), e.to_string()))?;
                        let detected = ImageOutputFormat::from_magic_bytes(&input_bytes_preview)
                            .unwrap_or(DEFAULT_OUTPUT_FORMAT);
                        let effective_format = output_format.unwrap_or(detected);

                        let mut seen = seen_paths.lock().unwrap();
                        let override_output = compute_output_path(
                            input_path,
                            &args.output,
                            effective_format,
                            &mut seen,
                        );
                        drop(seen);

                        process_single_file_request(
                            input_path,
                            &args.output,
                            output_format,
                            &request,
                            args.verbose,
                            override_output,
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
                        let detected = ImageOutputFormat::from_magic_bytes(
                            &fs::read(input_path).unwrap_or_default(),
                        )
                        .unwrap_or(DEFAULT_OUTPUT_FORMAT);
                        let effective_format = output_format.unwrap_or(detected);

                        let override_output = compute_output_path(
                            input_path,
                            &args.output,
                            effective_format,
                            &mut seen,
                        );

                        process_single_file_request(
                            input_path,
                            &args.output,
                            output_format,
                            &request,
                            args.verbose,
                            override_output,
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
                        display_warnings(&warnings, &ctx_for_display, args.verbose);
                        if args.strict
                            && warnings.iter().any(|w| {
                                w.severity_for_profile(ctx_for_display.evidence_profile())
                                    == WarningSeverity::Error
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
                    "Strict mode: one or more warnings with error severity (see warnings above)"
                        .into(),
                );
            }

            return Ok(());
        }

        let input_path = &input_files[0];
        let input_bytes = fs::read(input_path)?;

        if args.dry_run {
            let input_format = stegoeggo::ImageOutputFormat::from_magic_bytes(&input_bytes)
                .unwrap_or(DEFAULT_OUTPUT_FORMAT);
            let plan = stegoeggo::resolve_request(&request, input_format)?;
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
            return Ok(());
        }

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
            if dir.is_file() || (dir.extension().is_some() && is_image_file(dir)) {
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

            #[allow(deprecated)]
            let ctx = ProtectionContext::new(args.intensity.clamp(0.0, 1.0), seed)
                .with_evidence_profile(EvidenceProfile::LegalNotice);
            display_warnings(&warnings, &ctx, args.verbose);

            if args.verbose {
                println!("Output: {:?}", output_path);
                println!("Done!");
            } else {
                println!("{}", output_path.display());
            }

            if args.strict
                && warnings.iter().any(|w| {
                    w.severity_for_profile(ctx.evidence_profile()) == WarningSeverity::Error
                })
            {
                return Err(
                    "Strict mode: one or more warnings with error severity (see warnings above)"
                        .into(),
                );
            }
        }

        return Ok(());
    }

    let seed = args.seed.unwrap_or_else(generate_random_seed);

    let mac_key = resolve_key_input(&args.key, "STEGOEGGO_KEY")?;

    let (legal_metadata, legal_dmi_override) = build_legal_metadata(&args);

    let output_format: Option<ImageOutputFormat> = args.format.map(ImageOutputFormat::from);

    let protection_level = ProtectionLevel::from(args.level);
    #[allow(deprecated)]
    let evidence_profile = EvidenceProfile::from(args.profile);

    let dmi_value = args.dmi.as_ref().and_then(|d| {
        d.clone().into_dmi_value().or({
            // Auto-select DMI based on protection level
            Some(match protection_level {
                ProtectionLevel::Disabled | ProtectionLevel::Light => DmiValue::Unspecified,
                _ => DmiValue::ProhibitedAiMlTraining,
            })
        })
    });

    if args.metadata == Some(false) && legal_metadata.is_some() {
        eprintln!(
            "Error: Cannot use --no-metadata (or -m false) together with legal metadata flags \
             (--copyright-notice, --creator, --contact, --rights-url, --usage-terms, \
             --ai-constraints, --no-ai-training, --no-genai-training, --tdm-reserved). \
             Legal metadata requires metadata injection to be enabled."
        );
        std::process::exit(EXIT_CONFIG);
    }

    #[allow(deprecated)]
    let mut ctx = ProtectionContext::new(args.intensity.clamp(0.0, 1.0), seed)
        .with_stego_redundancy(args.stego_redundancy.clamp(1, 10))
        .with_jpeg_quality(args.jpeg_quality.clamp(1, 100))
        .with_progressive_jpeg(args.progressive)
        .with_evidence_profile(evidence_profile);

    if let Some(fmt) = output_format {
        ctx = ctx.with_format(fmt);
    }

    let effective_dmi = legal_dmi_override.or(dmi_value);
    #[allow(deprecated)]
    if let Some(dmi) = effective_dmi {
        ctx = ctx.with_dmi(dmi);
    }
    #[allow(deprecated)]
    if let Some(val) = args.metadata {
        ctx = ctx.with_metadata_injection(val);
    } else if legal_metadata.is_some() {
        #[allow(deprecated)]
        {
            ctx = ctx.with_metadata_injection(true);
        }
    }
    #[allow(deprecated)]
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
        println!("Evidence profile: {:?}", evidence_profile);
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

        #[allow(clippy::type_complexity)]
        let results: Vec<
            Result<(PathBuf, PathBuf, Vec<ProtectionWarning>), (PathBuf, String)>,
        > = if args.jobs > 1 {
            let seen_paths: std::sync::Mutex<HashMap<PathBuf, usize>> =
                std::sync::Mutex::new(HashMap::new());

            input_files
                .par_iter()
                .with_max_len(1)
                .map(|input_path| {
                    let input_bytes_preview =
                        fs::read(input_path).map_err(|e| (input_path.clone(), e.to_string()))?;
                    let detected = ImageOutputFormat::from_magic_bytes(&input_bytes_preview)
                        .unwrap_or(DEFAULT_OUTPUT_FORMAT);
                    let effective_format = output_format.unwrap_or(detected);

                    let mut seen = seen_paths.lock().unwrap();
                    let override_output =
                        compute_output_path(input_path, &output_dir, effective_format, &mut seen);
                    drop(seen);

                    process_single_file(
                        input_path,
                        &output_dir,
                        output_format,
                        &ctx,
                        protection_level,
                        args.verbose,
                        override_output,
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
                    let detected = ImageOutputFormat::from_magic_bytes(
                        &fs::read(input_path).unwrap_or_default(),
                    )
                    .unwrap_or(DEFAULT_OUTPUT_FORMAT);
                    let effective_format = output_format.unwrap_or(detected);

                    let override_output =
                        compute_output_path(input_path, &output_dir, effective_format, &mut seen);

                    process_single_file(
                        input_path,
                        &output_dir,
                        output_format,
                        &ctx,
                        protection_level,
                        args.verbose,
                        override_output,
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
                    display_warnings(&warnings, &ctx, args.verbose);
                    if args.strict
                        && warnings.iter().any(|w| {
                            w.severity_for_profile(ctx.evidence_profile()) == WarningSeverity::Error
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

        if args.strict && has_errors {
            return Err(
                "Strict mode: one or more files produced errors (see warnings above)".into(),
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

        let (output_path, warnings) = process_single_file(
            input_path,
            &args.output,
            output_format,
            &ctx,
            protection_level,
            args.verbose,
            None,
        )?;

        display_warnings(&warnings, &ctx, args.verbose);

        if args.verbose {
            println!("Output: {:?}", output_path);
            println!("Done!");
        } else {
            println!("{}", output_path.display());
        }

        if args.strict
            && warnings
                .iter()
                .any(|w| w.severity_for_profile(ctx.evidence_profile()) == WarningSeverity::Error)
        {
            return Err(
                "Strict mode: one or more warnings with error severity (see warnings above)".into(),
            );
        }
    }

    Ok(())
}
