use stegoeggo::Error;

pub(crate) const EXIT_OK: i32 = 0;
pub(crate) const EXIT_ERROR: i32 = 1;
pub(crate) const EXIT_CONFIG: i32 = 2;
pub(crate) const EXIT_INTEGRITY: i32 = 3;
pub(crate) const EXIT_INTERNAL: i32 = 5;

pub(crate) fn config_err(msg: impl std::fmt::Display) -> Box<dyn std::error::Error> {
    Box::new(Error::Config(msg.to_string()))
}

pub(crate) fn classify_error(e: &(dyn std::error::Error + 'static)) -> i32 {
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
            | Error::Image(_)
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
pub(crate) struct JsonOutput {
    pub(crate) schema_version: u32,
    pub(crate) status: String,
    pub(crate) output_path: Option<String>,
    pub(crate) warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) report: Option<JsonExecutionReport>,
}

#[derive(serde::Serialize)]
pub(crate) struct JsonExecutionReport {
    pub(crate) effective_policy: String,
    pub(crate) effective_dmi: Option<String>,
    pub(crate) metadata_injected: bool,
    pub(crate) stego_attempted: bool,
    pub(crate) stego_succeeded: bool,
    pub(crate) format_transcoded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) embed_summary: Option<JsonEmbedOutcomeSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) resource_usage: Option<JsonResourceUsage>,
}

#[derive(serde::Serialize)]
pub(crate) struct JsonEmbedOutcomeSummary {
    pub(crate) status: String,
    pub(crate) embedding_path: String,
    pub(crate) payload_bytes: usize,
    pub(crate) required_capacity: usize,
    pub(crate) available_capacity: usize,
}

pub(crate) fn embed_path_label(path: stegoeggo::EmbedPath) -> &'static str {
    match path {
        stegoeggo::EmbedPath::Lsb => "lsb",
        stegoeggo::EmbedPath::LsbTiled => "lsb-tiled",
        stegoeggo::EmbedPath::DctF5 => "dct-f5",
        stegoeggo::EmbedPath::DctF5Tiled => "dct-f5-tiled",
        stegoeggo::EmbedPath::QTableSeedOnly => "q-table-seed-only",
    }
}

#[derive(serde::Serialize)]
pub(crate) struct JsonResourceUsage {
    pub(crate) input_bytes: usize,
    pub(crate) png_chunks_scanned: usize,
    pub(crate) jpeg_segments_scanned: usize,
    pub(crate) webp_riff_chunks_scanned: usize,
    pub(crate) xmp_bytes_parsed: usize,
    pub(crate) metadata_fields_extracted: usize,
    pub(crate) metadata_bytes_copied: usize,
    pub(crate) tile_origins_checked: usize,
    pub(crate) verification_seeds_tried: usize,
    pub(crate) peak_allocations_bytes: usize,
}

#[derive(serde::Serialize)]
pub(crate) struct JsonVerifyOutput {
    pub(crate) schema_version: u32,
    pub(crate) status: String,
    pub(crate) copyright_holder: Option<String>,
    pub(crate) rights_url: Option<String>,
    pub(crate) ai_constraints: Option<String>,
    pub(crate) stego_status: String,
    pub(crate) evidence_strength: String,
}
