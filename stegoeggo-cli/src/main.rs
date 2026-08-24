use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use stegoeggo::Error;
#[allow(deprecated)]
use stegoeggo::{
    generate_random_seed, process_request_bytes_with_warnings, verify_legal_notice, DmiValue,
    EvidenceProfile, HiddenMarkerMode, ImageOutputFormat, ProtectionChannels, ProtectionLevel,
    ProtectionPreset, ProtectionRequest, ProtectionWarning, RightsPolicy, StegoPayload,
    WarningSeverity, DEFAULT_OUTPUT_FORMAT,
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

#[derive(Debug, Clone, PartialEq, ValueEnum)]
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

#[derive(Debug, Clone, PartialEq, ValueEnum)]
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

#[allow(deprecated)]
fn evidence_profile_for_display(args: &Args) -> EvidenceProfile {
    if let Some(preset_arg) = args.preset {
        return match preset_arg {
            PresetArg::LegalNotice => EvidenceProfile::LegalNotice,
            PresetArg::LegalNoticeWithStego => EvidenceProfile::LegalNoticeWithStego,
            PresetArg::AuthenticatedProvenance => EvidenceProfile::AuthenticatedProvenance,
            PresetArg::Maximal => EvidenceProfile::Maximal,
        };
    }
    if args.dry_run {
        return EvidenceProfile::LegalNotice;
    }
    if args.authentication.is_some() || args.hidden_marker.is_some() || args.rights_policy.is_some()
    {
        if matches!(args.authentication, Some(AuthenticationArg::Hmac)) {
            return EvidenceProfile::AuthenticatedProvenance;
        }
        if matches!(args.hidden_marker, Some(HiddenMarkerArg::BestEffort)) {
            return EvidenceProfile::LegalNoticeWithStego;
        }
        return EvidenceProfile::LegalNotice;
    }
    EvidenceProfile::from(args.profile.clone())
}

#[allow(deprecated)]
fn display_warnings(warnings: &[ProtectionWarning], profile: EvidenceProfile, verbose: bool) {
    if warnings.is_empty() {
        return;
    }
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

fn has_new_style_flags(args: &Args) -> bool {
    args.rights_policy.is_some()
        || args.preset.is_some()
        || args.hidden_marker.is_some()
        || args.authentication.is_some()
}

#[allow(deprecated)]
#[cfg(test)]
fn build_protection_request(args: &Args) -> Result<ProtectionRequest, Box<dyn std::error::Error>> {
    build_protection_request_with_explicit_options(args, false, false)
}

fn build_protection_request_with_explicit_options(
    args: &Args,
    level_explicit: bool,
    profile_explicit: bool,
) -> Result<ProtectionRequest, Box<dyn std::error::Error>> {
    let is_new_style = has_new_style_flags(args);

    if is_new_style
        && args.preset.is_some()
        && (level_explicit
            || profile_explicit
            || args.level != ProtectionLevelArg::Standard
            || args.profile != ProfileArg::LegalNotice)
    {
        return Err("Cannot combine --preset with --level/--profile; use --preset alone or --level/--profile alone".into());
    }

    if is_new_style
        && args.preset.is_some()
        && (args.hidden_marker.is_some() || args.authentication.is_some())
    {
        return Err("Cannot combine --preset with --hidden-marker/--authentication; use --preset alone or explicit channel flags alone".into());
    }

    let (legal_metadata, legal_dmi_override) = build_legal_metadata(args);

    let legal_metadata = if let Some(ref dmi_arg) = args.dmi {
        if matches!(dmi_arg, DmiArg::ProhibitedConstraints) {
            if args.ai_constraints.is_none() && args.rights_url.is_none() {
                match legal_metadata {
                    Some(meta) => {
                        Some(meta.with_ai_constraints("Text and data mining rights reserved"))
                    }
                    None => {
                        let mut meta = stegoeggo::LegalMetadata::default();
                        meta = meta.with_ai_constraints("Text and data mining rights reserved");
                        Some(meta)
                    }
                }
            } else {
                legal_metadata
            }
        } else {
            legal_metadata
        }
    } else {
        legal_metadata
    };

    if args.metadata == Some(false) && legal_metadata.is_some() {
        return Err("Cannot use --metadata false with legal metadata flags (--copyright-notice, --creator, etc.). Legal metadata requires metadata injection".into());
    }

    let seed = args.seed.unwrap_or_else(generate_random_seed);

    let mac_key = resolve_key_input(&args.key, "STEGOEGGO_KEY")?;

    let output_format: Option<ImageOutputFormat> = args.format.as_ref().map(|f| f.clone().into());

    let (policy, channels) = if is_new_style {
        build_new_style_request(args, &legal_metadata, legal_dmi_override)?
    } else {
        build_legacy_style_request(args, &legal_metadata, legal_dmi_override)?
    };

    if matches!(channels.authentication, stegoeggo::AuthenticationMode::Hmac) && mac_key.is_none() {
        return Err("HMAC authentication requires a key (--key hex, --key @file, --key -, or env STEGOEGGO_KEY)".into());
    }

    if matches!(channels.authentication, stegoeggo::AuthenticationMode::Hmac)
        && matches!(channels.hidden_marker, HiddenMarkerMode::Disabled)
    {
        return Err("HMAC authentication requires a non-disabled hidden marker (--hidden-marker best-effort)".into());
    }

    let notice = stegoeggo::RightsNotice::default();

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

    Ok(request)
}

#[allow(deprecated)]
fn build_new_style_request(
    args: &Args,
    _legal_metadata: &Option<stegoeggo::LegalMetadata>,
    legal_dmi_override: Option<DmiValue>,
) -> Result<(RightsPolicy, ProtectionChannels), Box<dyn std::error::Error>> {
    let mut policy = args
        .rights_policy
        .map(RightsPolicy::from)
        .unwrap_or(RightsPolicy::Unspecified);

    if let Some(dmi_val) = legal_dmi_override {
        let dmi_policy = RightsPolicy::from_dmi_value(dmi_val);
        if args.rights_policy.is_some() && dmi_policy != policy {
            return Err(format!(
                "Conflicting policy: --rights-policy {:?} contradicts --no-ai-training/--no-genai-training/--tdm-reserved",
                args.rights_policy
            ).into());
        }
        policy = dmi_policy;
    }

    if let Some(ref dmi_arg) = args.dmi {
        if let Some(dmi_val) = dmi_arg.clone().into_dmi_value() {
            let dmi_policy = RightsPolicy::from_dmi_value(dmi_val);
            if args.rights_policy.is_some() && dmi_policy != policy {
                return Err(format!(
                    "Conflicting policy: --rights-policy {:?} contradicts --dmi {:?}",
                    args.rights_policy, dmi_arg
                )
                .into());
            }
            policy = dmi_policy;
        }
    }

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

        let notice = stegoeggo::RightsNotice::default();
        let rights_metadata = policy != RightsPolicy::Unspecified
            || notice.has_legal_content()
            || _legal_metadata.is_some()
            || !matches!(hidden, HiddenMarkerMode::Disabled);

        ProtectionChannels {
            rights_metadata,
            hidden_marker: hidden,
            authentication: auth,
        }
    };

    Ok((policy, channels))
}

fn resolve_legacy_dmi(args: &Args, level: ProtectionLevel) -> Option<DmiValue> {
    match args.dmi.as_ref() {
        None => {
            let policy = level.default_policy();
            if policy == RightsPolicy::Unspecified {
                Some(DmiValue::Unspecified)
            } else {
                Some(DmiValue::from(policy))
            }
        }
        Some(dmi_arg) => {
            let dmi_val = dmi_arg.clone().into_dmi_value();
            Some(dmi_val.unwrap_or_else(|| {
                let policy = level.default_policy();
                if policy == RightsPolicy::Unspecified {
                    DmiValue::Unspecified
                } else {
                    DmiValue::from(policy)
                }
            }))
        }
    }
}

#[allow(deprecated)]
fn build_legacy_style_request(
    args: &Args,
    _legal_metadata: &Option<stegoeggo::LegalMetadata>,
    legal_dmi_override: Option<DmiValue>,
) -> Result<(RightsPolicy, ProtectionChannels), Box<dyn std::error::Error>> {
    let protection_level = ProtectionLevel::from(args.level.clone());

    let dmi_from_arg = resolve_legacy_dmi(args, protection_level);
    let effective_dmi = legal_dmi_override.or(dmi_from_arg);
    let policy = effective_dmi
        .map(RightsPolicy::from_dmi_value)
        .unwrap_or(RightsPolicy::Unspecified);

    let hidden_marker = match protection_level {
        ProtectionLevel::Disabled => HiddenMarkerMode::Disabled,
        ProtectionLevel::Light | ProtectionLevel::Standard => HiddenMarkerMode::BestEffort,
        _ => HiddenMarkerMode::Disabled,
    };

    let authentication = match args.profile {
        ProfileArg::AuthenticatedProvenance | ProfileArg::Maximal => {
            stegoeggo::AuthenticationMode::Hmac
        }
        _ => stegoeggo::AuthenticationMode::None,
    };

    let mut rights_metadata = !matches!(protection_level, ProtectionLevel::Disabled);
    if let Some(meta) = _legal_metadata {
        if meta.has_content() {
            rights_metadata = true;
        }
    }
    if args.metadata == Some(false) {
        rights_metadata = false;
    }
    if args.legal_claims {
        rights_metadata = true;
    }

    let channels = ProtectionChannels {
        rights_metadata,
        hidden_marker,
        authentication,
    };

    Ok((policy, channels))
}

fn process_single_file(
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
    payload_key: Option<String>,
    json_output: bool,
) -> Result<i32, Box<dyn std::error::Error>> {
    use stegoeggo::detached::verify::{
        verify_detached_manifest_with_options, DetachedVerificationOptions, EmbeddedReferenceStatus,
    };
    use stegoeggo::detached::DetachedManifest;
    use stegoeggo::resource_limits::ResourceLimits;
    use stegoeggo::signing::VerifyingKey;

    let manifest_bytes = fs::read(manifest_path)?;
    let limits = ResourceLimits::default();
    let manifest = DetachedManifest::from_json_with_limits(&manifest_bytes, &limits)
        .map_err(|e| format!("Manifest parsing failed: {}", e))?;

    let image_bytes = fs::read(image_path)?;

    let caller_keys: Vec<stegoeggo::detached::TrustedVerifyingKey> = if let Some(ref key_file) =
        key_path
    {
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
        vec![stegoeggo::detached::TrustedVerifyingKey {
            key_id: vk.key_id().to_vec(),
            key: vk,
        }]
    } else {
        Vec::new()
    };

    let payload_mac_key = resolve_key_input(&payload_key, "")?;

    let options = DetachedVerificationOptions {
        trust_policy: None,
        caller_verifying_keys: &caller_keys,
        payload_mac_key: payload_mac_key.as_deref(),
        limits: Some(&limits),
    };
    let result = verify_detached_manifest_with_options(&image_bytes, &manifest, &options);

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
            stegoeggo::detached::DetachedOverallStatus::KeyMaterialMismatch => {
                "key_material_mismatch"
            }
        };

        #[derive(serde::Serialize)]
        struct JsonSignatureDetail {
            key_id: String,
            cryptographically_valid: bool,
            key_id_matched: bool,
            key_material_matched: bool,
            trusted: bool,
        }

        #[derive(serde::Serialize)]
        struct JsonManifestVerify {
            schema_version: u32,
            overall_status: &'static str,
            trust_mode: &'static str,
            instance_digest_match: bool,
            manifest_valid: bool,
            embedded_reference: &'static str,
            signatures_valid: bool,
            trusted: bool,
            evidence_strength: String,
            signatures: Vec<JsonSignatureDetail>,
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

        let sig_details: Vec<JsonSignatureDetail> = result
            .report
            .signatures()
            .iter()
            .map(|s| JsonSignatureDetail {
                key_id: hex::encode(s.public_key_id().unwrap_or(&[])),
                cryptographically_valid: s.cryptographically_valid(),
                key_id_matched: s.key_id_matched(),
                key_material_matched: s.key_material_matched(),
                trusted: s.trusted(),
            })
            .collect();

        let trust_mode = if !caller_keys.is_empty() {
            "caller_verifying_key"
        } else {
            "none"
        };

        let json = JsonManifestVerify {
            schema_version: manifest.schema_version as u32,
            overall_status: status_str,
            trust_mode,
            instance_digest_match: result.instance_digest_match,
            manifest_valid: result.manifest_valid,
            embedded_reference: embedded_ref,
            signatures_valid: sigs_valid,
            trusted,
            evidence_strength: format!("{:?}", result.report.evidence_strength()),
            signatures: sig_details,
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
                println!("      key_id_matched: {}", sig.key_id_matched());
                println!("      key_material_matched: {}", sig.key_material_matched());
                println!("      trusted: {}", sig.trusted());
            }
        }

        println!(
            "\nEmbedded reference: {:?}",
            result.embedded_reference_status
        );
        let trust_mode_str = if !caller_keys.is_empty() {
            "caller_verifying_key"
        } else {
            "none"
        };
        println!(
            "Trust: {} (mode: {})",
            if result.report.trust().trusted() {
                "TRUSTED"
            } else {
                "UNTRUSTED"
            },
            trust_mode_str
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
    embed_summary: Option<JsonEmbedOutcomeSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_usage: Option<JsonResourceUsage>,
}

#[derive(serde::Serialize)]
struct JsonEmbedOutcomeSummary {
    status: String,
    embedding_path: String,
    payload_bytes: usize,
    required_capacity: usize,
    available_capacity: usize,
}

fn embed_path_label(path: stegoeggo::EmbedPath) -> &'static str {
    match path {
        stegoeggo::EmbedPath::Lsb => "lsb",
        stegoeggo::EmbedPath::LsbTiled => "lsb-tiled",
        stegoeggo::EmbedPath::DctF5 => "dct-f5",
        stegoeggo::EmbedPath::DctF5Tiled => "dct-f5-tiled",
        stegoeggo::EmbedPath::QTableSeedOnly => "q-table-seed-only",
    }
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

#[allow(deprecated)]
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

        let mac_key = resolve_key_input(&args.key, "STEGOEGGO_KEY")?.unwrap_or_default();

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

    let level_explicit = std::env::args().any(|arg| {
        arg == "--level"
            || arg.starts_with("--level=")
            || arg == "-l"
            || (arg.starts_with("-l") && arg.len() > 2)
    });
    let profile_explicit = std::env::args().any(|arg| {
        arg == "--profile"
            || arg.starts_with("--profile=")
            || arg == "-p"
            || (arg.starts_with("-p") && arg.len() > 2)
    });
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
        let input_path = &input_files[0];
        let input_bytes = fs::read(input_path)?;
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

    let output_format: Option<ImageOutputFormat> = args.format.as_ref().map(|f| f.clone().into());

    if is_batch {
        use rayon::prelude::*;

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

                    let mut seen = seen_paths.lock().unwrap_or_else(|e| e.into_inner());
                    let override_output =
                        compute_output_path(input_path, &args.output, effective_format, &mut seen);
                    drop(seen);

                    process_single_file(
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
                    let input_bytes_preview =
                        fs::read(input_path).map_err(|e| (input_path.clone(), e.to_string()))?;
                    let detected = ImageOutputFormat::from_magic_bytes(&input_bytes_preview)
                        .unwrap_or(DEFAULT_OUTPUT_FORMAT);
                    let effective_format = output_format.unwrap_or(detected);

                    let override_output =
                        compute_output_path(input_path, &args.output, effective_format, &mut seen);

                    process_single_file(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_args() -> Args {
        Args {
            input: vec![PathBuf::from("test.png")],
            output: None,
            verify: false,
            level: ProtectionLevelArg::Standard,
            profile: ProfileArg::LegalNotice,
            intensity: 0.5,
            seed: Some(42),
            format: None,
            stego_redundancy: 2,
            jpeg_quality: 90,
            progressive: false,
            verbose: false,
            dmi: None,
            metadata: None,
            legal_claims: false,
            copyright_notice: None,
            creator: None,
            contact: None,
            rights_url: None,
            usage_terms: None,
            ai_constraints: None,
            no_ai_training: false,
            no_genai_training: false,
            tdm_reserved: false,
            credit_line: None,
            copyright_owner: None,
            licensor_name: None,
            licensor_email: None,
            licensor_url: None,
            content_created_at: None,
            key: None,
            known_seeds: None,
            jobs: 1,
            strict: false,
            json: false,
            rights_policy: None,
            preset: None,
            hidden_marker: None,
            authentication: None,
            dry_run: false,
            #[cfg(feature = "signatures")]
            command: None,
        }
    }

    #[test]
    fn test_legacy_default_standard_is_prohibited() {
        let args = default_args();
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedAiMlTraining);
        assert!(req.channels().rights_metadata);
        assert_eq!(req.channels().hidden_marker, HiddenMarkerMode::BestEffort);
    }

    #[test]
    fn test_legacy_default_standard_dmi_auto_matches_omitted() {
        let mut args_omitted = default_args();
        args_omitted.dmi = None;
        let req_omitted = build_protection_request(&args_omitted).unwrap();

        let mut args_auto = default_args();
        args_auto.dmi = Some(DmiArg::Auto);
        let req_auto = build_protection_request(&args_auto).unwrap();

        assert_eq!(req_omitted.policy(), req_auto.policy());
        assert_eq!(
            req_omitted.channels().rights_metadata,
            req_auto.channels().rights_metadata
        );
        assert_eq!(
            req_omitted.channels().hidden_marker,
            req_auto.channels().hidden_marker
        );
    }

    #[test]
    fn test_legacy_dmi_unspecified_is_distinct_from_default() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::Unspecified);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::Unspecified);
    }

    #[test]
    fn test_legacy_light_level() {
        let mut args = default_args();
        args.level = ProtectionLevelArg::Light;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::Unspecified);
        assert!(req.channels().rights_metadata);
        assert_eq!(req.channels().hidden_marker, HiddenMarkerMode::BestEffort);
    }

    #[test]
    fn test_legacy_disabled_level() {
        let mut args = default_args();
        args.level = ProtectionLevelArg::Disabled;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::Unspecified);
        assert!(!req.channels().rights_metadata);
        assert_eq!(req.channels().hidden_marker, HiddenMarkerMode::Disabled);
    }

    #[test]
    fn test_no_ai_training_shorthand_sets_policy() {
        let mut args = default_args();
        args.no_ai_training = true;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedAiMlTraining);
    }

    #[test]
    fn test_no_genai_training_shorthand_sets_policy() {
        let mut args = default_args();
        args.no_genai_training = true;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedGenerativeAiTraining);
    }

    #[test]
    fn test_tdm_reserved_shorthand_sets_policy() {
        let mut args = default_args();
        args.tdm_reserved = true;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedSeeConstraints);
    }

    #[test]
    fn test_explicit_rights_policy_prohibited() {
        let mut args = default_args();
        args.rights_policy = Some(RightsPolicyArg::ProhibitedAiMlTraining);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedAiMlTraining);
    }

    #[test]
    fn test_explicit_rights_policy_allowed() {
        let mut args = default_args();
        args.rights_policy = Some(RightsPolicyArg::Allowed);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::Allowed);
    }

    #[test]
    fn test_conflicting_dmi_and_rights_policy() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::Allowed);
        args.rights_policy = Some(RightsPolicyArg::ProhibitedAiMlTraining);
        let result = build_protection_request(&args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Conflicting"), "Error: {}", err);
    }

    #[test]
    fn test_conflicting_shorthand_and_rights_policy() {
        let mut args = default_args();
        args.no_ai_training = true;
        args.rights_policy = Some(RightsPolicyArg::Allowed);
        let result = build_protection_request(&args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Conflicting"), "Error: {}", err);
    }

    #[test]
    fn test_metadata_false_with_legal_fields() {
        let mut args = default_args();
        args.metadata = Some(false);
        args.copyright_notice = Some("test".to_string());
        let result = build_protection_request(&args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Cannot use --metadata false"),
            "Error: {}",
            err
        );
    }

    #[test]
    fn test_hmac_without_key_is_error() {
        let mut args = default_args();
        args.preset = Some(PresetArg::AuthenticatedProvenance);
        let result = build_protection_request(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_hmac_with_disabled_marker_is_error() {
        let mut args = default_args();
        args.hidden_marker = Some(HiddenMarkerArg::Disabled);
        args.authentication = Some(AuthenticationArg::Hmac);
        args.key = Some("deadbeef01234567deadbeef01234567".to_string());
        let result = build_protection_request(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_preset_and_level_conflict() {
        let mut args = default_args();
        args.preset = Some(PresetArg::Maximal);
        args.level = ProtectionLevelArg::Light;
        let result = build_protection_request(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_preset_and_explicit_default_level_conflict() {
        let mut args = default_args();
        args.preset = Some(PresetArg::Maximal);
        let result = build_protection_request_with_explicit_options(&args, true, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_seed_and_intensity_preserved() {
        let mut args = default_args();
        args.seed = Some(99);
        args.intensity = 0.8;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.seed(), Some(99));
        assert_eq!(req.intensity(), 0.8);
    }

    #[test]
    fn test_jpeg_quality_preserved() {
        let mut args = default_args();
        args.jpeg_quality = 75;
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.processing().jpeg_quality, 75);
    }

    #[test]
    fn test_legacy_dmi_prohibited_ai_matches_rights_policy() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::ProhibitedAi);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedAiMlTraining);
    }

    #[test]
    fn test_legacy_dmi_prohibited_gen_ai() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::ProhibitedGenAi);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedGenerativeAiTraining);
    }

    #[test]
    fn test_legacy_dmi_allowed() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::Allowed);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::Allowed);
    }

    #[test]
    fn test_legacy_dmi_prohibited() {
        let mut args = default_args();
        args.dmi = Some(DmiArg::Prohibited);
        let req = build_protection_request(&args).unwrap();
        assert_eq!(req.policy(), RightsPolicy::ProhibitedAllDataMining);
    }

    #[test]
    fn test_preset_legal_notice_request() {
        let mut args = default_args();
        args.preset = Some(PresetArg::LegalNotice);
        let req = build_protection_request(&args).unwrap();
        assert!(req.channels().rights_metadata);
        assert_eq!(req.channels().hidden_marker, HiddenMarkerMode::Disabled);
    }

    #[test]
    fn test_preset_legal_notice_stego_request() {
        let mut args = default_args();
        args.preset = Some(PresetArg::LegalNoticeWithStego);
        let req = build_protection_request(&args).unwrap();
        assert!(req.channels().rights_metadata);
        assert_eq!(req.channels().hidden_marker, HiddenMarkerMode::BestEffort);
    }

    #[test]
    fn test_preset_and_hidden_marker_conflict() {
        let mut args = default_args();
        args.preset = Some(PresetArg::LegalNotice);
        args.hidden_marker = Some(HiddenMarkerArg::BestEffort);
        let result = build_protection_request(&args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Cannot combine --preset"), "Error: {}", err);
    }

    #[test]
    fn test_preset_and_authentication_conflict() {
        let mut args = default_args();
        args.preset = Some(PresetArg::Maximal);
        args.authentication = Some(AuthenticationArg::Hmac);
        args.key = Some("deadbeef01234567deadbeef01234567".to_string());
        let result = build_protection_request(&args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Cannot combine --preset"), "Error: {}", err);
    }

    #[test]
    #[allow(deprecated)]
    fn test_display_profile_reflects_channel_flags() {
        let mut args = default_args();
        args.rights_policy = Some(RightsPolicyArg::Allowed);
        assert_eq!(
            evidence_profile_for_display(&args),
            EvidenceProfile::LegalNotice
        );

        let mut args = default_args();
        args.rights_policy = Some(RightsPolicyArg::ProhibitedSeeConstraints);
        args.hidden_marker = Some(HiddenMarkerArg::BestEffort);
        assert_eq!(
            evidence_profile_for_display(&args),
            EvidenceProfile::LegalNoticeWithStego
        );

        let mut args = default_args();
        args.rights_policy = Some(RightsPolicyArg::Unspecified);
        args.hidden_marker = Some(HiddenMarkerArg::Disabled);
        args.authentication = Some(AuthenticationArg::Hmac);
        assert_eq!(
            evidence_profile_for_display(&args),
            EvidenceProfile::AuthenticatedProvenance
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_display_profile_preset_and_dry_run_and_legacy() {
        let mut args = default_args();
        args.preset = Some(PresetArg::Maximal);
        assert_eq!(
            evidence_profile_for_display(&args),
            EvidenceProfile::Maximal
        );

        let mut args = default_args();
        args.dry_run = true;
        assert_eq!(
            evidence_profile_for_display(&args),
            EvidenceProfile::LegalNotice
        );

        let mut args = default_args();
        args.profile = ProfileArg::AuthenticatedProvenance;
        assert_eq!(
            evidence_profile_for_display(&args),
            EvidenceProfile::AuthenticatedProvenance
        );
    }
}
