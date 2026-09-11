use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use stegoeggo::{DmiValue, ImageOutputFormat, ProtectionLevel, ProtectionPreset, RightsPolicy};

#[derive(clap::Args, Debug, Clone)]
pub(crate) struct ProtectArgs {
    #[arg(
        value_name = "INPUT",
        required = true,
        help = "Input image file(s) or directories"
    )]
    pub(crate) input: Vec<PathBuf>,

    #[arg(
        short,
        long,
        help_heading = "Image output",
        help = "Output directory (for batch processing) or output file (for single file)"
    )]
    pub(crate) output: Option<PathBuf>,

    #[arg(
        long,
        hide = true,
        help = "Legacy alias for `inspect`; always exits successfully after reporting"
    )]
    pub(crate) verify: bool,

    #[arg(
        short,
        long,
        default_value = "standard",
        help_heading = "Compatibility (legacy)",
        help = "Protection level"
    )]
    pub(crate) level: ProtectionLevelArg,

    #[arg(
        short,
        long,
        default_value = "legal-notice",
        help_heading = "Compatibility (legacy)",
        help = "Evidence profile: legal-notice, legal-notice-stego, authenticated-provenance, maximal"
    )]
    pub(crate) profile: ProfileArg,

    #[arg(
        short,
        long,
        default_value = "0.5",
        help_heading = "Execution",
        help = "Protection intensity (0.0-1.0)"
    )]
    pub(crate) intensity: f32,

    #[arg(
        short,
        long,
        help_heading = "Execution",
        help = "Seed for reproducible results"
    )]
    pub(crate) seed: Option<u64>,

    #[arg(
        short,
        long,
        help_heading = "Image output",
        help = "Output format (png|jpg|webp) - defaults to preserving input format"
    )]
    pub(crate) format: Option<OutputFormatArg>,

    #[arg(
        long,
        default_value = "2",
        help_heading = "Image output",
        help = "Stego embedding redundancy (1-10). Higher = more robust, lower = faster"
    )]
    pub(crate) stego_redundancy: usize,

    #[arg(
        long,
        default_value = "90",
        help_heading = "Image output",
        help = "JPEG encoding quality (1-100). Only applies when output is JPEG"
    )]
    pub(crate) jpeg_quality: u8,

    #[arg(
        long,
        help_heading = "Image output",
        help = "Use progressive JPEG encoding. Progressive JPEGs render faster on slow connections"
    )]
    pub(crate) progressive: bool,

    #[arg(short, long, help_heading = "Execution", help = "Print verbose output")]
    pub(crate) verbose: bool,

    #[arg(
        short,
        long,
        help_heading = "Compatibility (legacy)",
        help = "AI-training restriction metadata (IPTC DMI value)"
    )]
    pub(crate) dmi: Option<DmiArg>,

    #[arg(
        long,
        help_heading = "Compatibility (legacy)",
        help = "Inject metadata (seed, DMI). Default: true for Light and Standard"
    )]
    pub(crate) metadata: Option<bool>,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "Inject legal claims (copyright, usage terms) into image metadata — only for content you own"
    )]
    pub(crate) legal_claims: bool,

    #[arg(
        long,
        alias = "copyright-holder",
        help_heading = "Rights metadata",
        help = "Copyright notice text (e.g., '© 2024 Jane Doe. All rights reserved.')"
    )]
    pub(crate) copyright_notice: Option<String>,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "Creator/author name (e.g., 'Jane Doe')"
    )]
    pub(crate) creator: Option<String>,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "Contact email or URL for rights inquiries"
    )]
    pub(crate) contact: Option<String>,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "URL to full usage terms or license text"
    )]
    pub(crate) rights_url: Option<String>,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "Brief usage terms summary (e.g., 'All rights reserved')"
    )]
    pub(crate) usage_terms: Option<String>,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "AI-specific constraints (e.g., 'No training, no generation')"
    )]
    pub(crate) ai_constraints: Option<String>,

    #[arg(
        long,
        help_heading = "Compatibility (legacy)",
        help = "Shorthand: prohibit AI/ML training and set default AI constraints"
    )]
    pub(crate) no_ai_training: bool,

    #[arg(
        long,
        help_heading = "Compatibility (legacy)",
        help = "Shorthand: prohibit generative AI training only"
    )]
    pub(crate) no_genai_training: bool,

    #[arg(
        long,
        help_heading = "Compatibility (legacy)",
        help = "Shorthand: reserve text and data mining rights [DEPRECATED: TDMRep deployment artifacts deferred; sets DMI ProhibitedSeeConstraints instead]"
    )]
    pub(crate) tdm_reserved: bool,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "Required credit line text (e.g., 'Photo by Jane Doe / Acme Corp')"
    )]
    pub(crate) credit_line: Option<String>,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "Copyright owner name (distinct from copyright holder notice text)"
    )]
    pub(crate) copyright_owner: Option<String>,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "Licensor name for PLUS structured rights"
    )]
    pub(crate) licensor_name: Option<String>,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "Licensor email for PLUS structured rights"
    )]
    pub(crate) licensor_email: Option<String>,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "Licensor URL for PLUS structured rights"
    )]
    pub(crate) licensor_url: Option<String>,

    #[arg(
        long,
        help_heading = "Rights metadata",
        help = "Content creation date (ISO 8601, e.g., '2024-01-15')"
    )]
    pub(crate) content_created_at: Option<String>,

    #[arg(
        long,
        help_heading = "Policy and evidence",
        help = "Cryptographic key for HMAC authentication. Accepts: hex string, @/path/to/file (hex in file), - (stdin), or env STEGOEGGO_KEY"
    )]
    pub(crate) key: Option<String>,

    #[arg(
        short = 'j',
        long = "jobs",
        default_value = "1",
        help_heading = "Execution",
        help = "Number of parallel jobs for batch processing"
    )]
    pub(crate) jobs: usize,

    #[arg(
        long,
        help_heading = "Execution",
        help = "Exit with error if any warnings have error severity for the active evidence profile"
    )]
    pub(crate) strict: bool,

    #[arg(
        long,
        help_heading = "Execution",
        help = "Output results as JSON (machine-readable)"
    )]
    pub(crate) json: bool,

    #[arg(
        long,
        value_enum,
        help_heading = "Policy and evidence",
        help = "Explicit rights policy (canonical; replaces --dmi)"
    )]
    pub(crate) rights_policy: Option<RightsPolicyArg>,

    #[arg(
        long,
        value_enum,
        help_heading = "Policy and evidence",
        help = "Evidence preset (canonical; replaces --level + --profile)"
    )]
    pub(crate) preset: Option<PresetArg>,

    #[arg(
        long,
        value_enum,
        help_heading = "Policy and evidence",
        help = "Hidden marker mode (canonical)"
    )]
    pub(crate) hidden_marker: Option<HiddenMarkerArg>,

    #[arg(
        long,
        value_enum,
        help_heading = "Policy and evidence",
        help = "Authentication mode (canonical)"
    )]
    pub(crate) authentication: Option<AuthenticationArg>,

    #[arg(
        long,
        help_heading = "Execution",
        help = "Dry run: show resolved plan without processing"
    )]
    pub(crate) dry_run: bool,
}

#[derive(Parser, Debug)]
#[command(name = "stegoeggo")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Protect and inspect image rights metadata", long_about = None)]
pub(crate) struct Args {
    #[command(flatten)]
    pub(crate) protect: ProtectArgs,
}

#[derive(Parser, Debug)]
#[command(name = "stegoeggo")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Protect and inspect image rights metadata", long_about = None)]
pub(crate) struct RootArgs {
    #[arg(long, hide = true)]
    pub(crate) json: bool,
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    #[command(about = "Protect images with rights metadata and optional hidden markers")]
    Protect(Box<ProtectArgs>),

    #[command(about = "Inspect rights metadata and evidence without changing the image")]
    Inspect(InspectArgs),

    #[command(about = "Assert that protection evidence is intact; exits 3 on integrity failure")]
    Verify(VerifyArgs),

    #[command(about = "Print the stegoeggo version")]
    Version,

    #[command(about = "Update the stegoeggo installation from a verified release asset")]
    Update(UpdateArgs),

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

        #[arg(long, hide = true, help = "Output verification results as JSON")]
        json: bool,
    },
}

#[derive(clap::Args, Debug, Clone)]
pub(crate) struct UpdateArgs {}

#[derive(clap::Args, Debug)]
pub(crate) struct InspectArgs {
    #[arg(value_name = "IMAGE", help = "Image to inspect")]
    pub(crate) image: PathBuf,
    #[arg(long, help = "HMAC key for authenticated marker inspection")]
    pub(crate) key: Option<String>,
    #[arg(long, help = "Output the compatibility JSON report")]
    pub(crate) json: bool,
    #[arg(short, long, help = "Print progress information")]
    pub(crate) verbose: bool,
}

#[derive(clap::Args, Debug)]
pub(crate) struct VerifyArgs {
    #[arg(
        value_name = "IMAGE",
        help = "Image whose protection evidence should be verified"
    )]
    pub(crate) image: PathBuf,
    #[arg(long, help = "HMAC key for authenticated marker verification")]
    pub(crate) key: Option<String>,
    #[arg(long, help = "Output the compatibility JSON report")]
    pub(crate) json: bool,
    #[arg(short, long, help = "Print progress information")]
    pub(crate) verbose: bool,
}

#[derive(Debug, Clone, PartialEq, ValueEnum)]
pub(crate) enum ProtectionLevelArg {
    Disabled,
    Light,
    Standard,
}

#[derive(Debug, Clone, ValueEnum)]
pub(crate) enum OutputFormatArg {
    Png,
    Jpg,
    WebP,
}

#[derive(Debug, Clone, ValueEnum)]
pub(crate) enum DmiArg {
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
    pub(crate) fn into_dmi_value(self) -> Option<DmiValue> {
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
pub(crate) enum ProfileArg {
    LegalNotice,
    LegalNoticeStego,
    AuthenticatedProvenance,
    Maximal,
}

#[allow(deprecated)]
impl From<ProfileArg> for stegoeggo::EvidenceProfile {
    fn from(arg: ProfileArg) -> Self {
        match arg {
            ProfileArg::LegalNotice => stegoeggo::EvidenceProfile::LegalNotice,
            ProfileArg::LegalNoticeStego => stegoeggo::EvidenceProfile::LegalNoticeWithStego,
            ProfileArg::AuthenticatedProvenance => {
                stegoeggo::EvidenceProfile::AuthenticatedProvenance
            }
            ProfileArg::Maximal => stegoeggo::EvidenceProfile::Maximal,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub(crate) enum RightsPolicyArg {
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
pub(crate) enum PresetArg {
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
pub(crate) enum HiddenMarkerArg {
    Disabled,
    BestEffort,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub(crate) enum AuthenticationArg {
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
