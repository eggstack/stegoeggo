use super::observe_metadata_work;
use super::stego;
use super::{load_image_from_bytes, RightsMetadataProtector, SteganographyProtector};
use crate::error::{Error, Result};
use crate::stego::EmbedOutcomeSummary;
use crate::types::{
    HiddenMarkerMode, ImageOutputFormat, ProtectionWarning, ResolvedProtectionPlan,
};
use image::{DynamicImage, GenericImageView};
use std::io::Cursor;

/// Internal pipeline output that carries both the processed bytes and the
/// structured embedding outcome. Public functions extract just the bytes;
/// `process_request_bytes_with_report` uses the full result.
pub(crate) struct PipelineResult {
    pub bytes: Vec<u8>,
    pub embed_summary: Option<EmbedOutcomeSummary>,
}

pub(crate) fn warnings_from_embed_outcome(
    summary: &crate::stego::EmbedOutcomeSummary,
) -> Vec<ProtectionWarning> {
    let mut warnings = Vec::new();
    match summary.status {
        crate::stego::EmbedStatus::SkippedCapacity => match summary.path {
            crate::stego::EmbedPath::Lsb | crate::stego::EmbedPath::LsbTiled => {
                warnings.push(ProtectionWarning::LsbCapacitySkipped);
            }
            crate::stego::EmbedPath::DctF5 | crate::stego::EmbedPath::DctF5Tiled => {
                warnings.push(ProtectionWarning::DctCapacityInsufficient);
            }
            crate::stego::EmbedPath::QTableSeedOnly => {}
        },
        crate::stego::EmbedStatus::UnsupportedProgressive => {
            warnings.push(ProtectionWarning::ProgressiveJpegFallback);
        }
        crate::stego::EmbedStatus::Embedded => {}
    }
    warnings
}

pub(crate) fn process_plan_bytes(
    img_bytes: &[u8],
    plan: &ResolvedProtectionPlan,
    budget: &mut crate::resource_limits::OperationObserver,
) -> Result<PipelineResult> {
    let limits = plan.resource_limits();
    limits.check_input_size(img_bytes.len())?;

    if plan.input_format() == ImageOutputFormat::Jpeg {
        let info = stego::jpeg::inspect(
            img_bytes,
            limits.max_jpeg_segments(),
            limits.max_jpeg_segment_bytes(),
        )?;
        limits.check_dimensions(info.width, info.height)?;
        if let Some(max_dim) = plan.processing().max_dimension {
            if info.width > max_dim || info.height > max_dim {
                return Err(Error::ImageDecode(format!(
                    "Image dimensions {}x{} exceed max_dimension {}",
                    info.width, info.height, max_dim
                )));
            }
        }
    } else {
        if let Ok(reader) = image::ImageReader::new(Cursor::new(img_bytes)).with_guessed_format() {
            if let Ok((width, height)) = reader.into_dimensions() {
                limits.check_dimensions(width, height)?;
                if let Some(max_dim) = plan.processing().max_dimension {
                    if width > max_dim || height > max_dim {
                        return Err(Error::ImageDecode(format!(
                            "Image dimensions {width}x{height} exceed max_dimension {max_dim}"
                        )));
                    }
                }
            }
        }
    }

    if plan.is_metadata_only()
        || matches!(plan.channels().hidden_marker, HiddenMarkerMode::Disabled)
    {
        let bytes = execute_metadata_only(img_bytes, plan, budget)?;
        return Ok(PipelineResult {
            bytes,
            embed_summary: None,
        });
    }

    let input_format = plan.input_format();
    let output_format = plan.output_format();
    let steganography = SteganographyProtector::new();
    let metadata_trap = RightsMetadataProtector::new();

    match plan.channels().hidden_marker {
        HiddenMarkerMode::Disabled => {
            debug_assert!(false, "Disabled hidden marker is handled above");
            let bytes = execute_metadata_only(img_bytes, plan, budget)?;
            Ok(PipelineResult {
                bytes,
                embed_summary: None,
            })
        }
        HiddenMarkerMode::SeedOnly => execute_seed_only_and_metadata(
            img_bytes,
            plan,
            input_format,
            output_format,
            &steganography,
            &metadata_trap,
            budget,
        ),
        HiddenMarkerMode::BestEffort => execute_full_marker_and_metadata(
            img_bytes,
            plan,
            None,
            &steganography,
            &metadata_trap,
            budget,
        ),
        HiddenMarkerMode::Tiled { tile_size } => execute_full_marker_and_metadata(
            img_bytes,
            plan,
            Some(tile_size),
            &steganography,
            &metadata_trap,
            budget,
        ),
    }
}

pub(crate) fn execute_metadata_only(
    img_bytes: &[u8],
    plan: &ResolvedProtectionPlan,
    budget: &mut crate::resource_limits::OperationObserver,
) -> Result<Vec<u8>> {
    let input_format = plan.input_format();
    let output_format = plan.output_format();
    let metadata_trap = RightsMetadataProtector::new();

    if input_format != output_format {
        let img = load_image_from_bytes(img_bytes)?;
        let encoded = crate::util::image::encode_image_with_options(
            &img,
            Some(output_format),
            plan.processing().progressive_jpeg,
            plan.processing().jpeg_quality,
        )?;
        let result = metadata_trap.inject_bytes_from_plan(&encoded, plan)?;
        observe_metadata_work(&result, output_format, budget)?;
        return Ok(result);
    }

    let result = metadata_trap.inject_bytes_from_plan(img_bytes, plan)?;
    observe_metadata_work(&result, output_format, budget)?;
    Ok(result)
}

pub(crate) fn execute_full_marker_and_metadata(
    img_bytes: &[u8],
    plan: &ResolvedProtectionPlan,
    tile_size: Option<u32>,
    steganography: &SteganographyProtector,
    metadata_trap: &RightsMetadataProtector,
    budget: &mut crate::resource_limits::OperationObserver,
) -> Result<PipelineResult> {
    let input_format = plan.input_format();
    let output_format = plan.output_format();
    if input_format == ImageOutputFormat::Jpeg && output_format == ImageOutputFormat::Jpeg {
        let with_stego =
            steganography.apply_dct_stego_bytes_from_plan(img_bytes, plan, tile_size)?;
        let (output, embed_summary) = with_stego.into_parts();
        let bytes = metadata_trap.inject_bytes_from_plan(&output, plan)?;
        observe_metadata_work(&bytes, output_format, budget)?;
        return Ok(PipelineResult {
            bytes,
            embed_summary: Some(embed_summary),
        });
    }

    let img = load_image_from_bytes(img_bytes)?;
    let (width, height) = img.dimensions();
    plan.resource_limits().check_dimensions(width, height)?;

    if output_format == ImageOutputFormat::Jpeg {
        let jpeg_bytes = crate::util::image::encode_image_with_options(
            &img,
            Some(output_format),
            plan.processing().progressive_jpeg,
            plan.processing().jpeg_quality,
        )?;
        let with_stego =
            steganography.apply_dct_stego_bytes_from_plan(&jpeg_bytes, plan, tile_size)?;
        let (output, embed_summary) = with_stego.into_parts();
        let bytes = metadata_trap.inject_bytes_from_plan(&output, plan)?;
        observe_metadata_work(&bytes, output_format, budget)?;
        return Ok(PipelineResult {
            bytes,
            embed_summary: Some(embed_summary),
        });
    }

    let (stego_img, embed_summary) =
        steganography.apply_lsb_to_image_with_summary_from_plan(&img, plan, tile_size)?;
    let encoded = crate::util::image::encode_image_with_options(
        &stego_img,
        Some(output_format),
        plan.processing().progressive_jpeg,
        plan.processing().jpeg_quality,
    )?;
    let bytes = metadata_trap.inject_bytes_from_plan(&encoded, plan)?;
    observe_metadata_work(&bytes, output_format, budget)?;
    Ok(PipelineResult {
        bytes,
        embed_summary,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_seed_only_and_metadata(
    img_bytes: &[u8],
    plan: &ResolvedProtectionPlan,
    input_format: ImageOutputFormat,
    output_format: ImageOutputFormat,
    steganography: &SteganographyProtector,
    metadata_trap: &RightsMetadataProtector,
    budget: &mut crate::resource_limits::OperationObserver,
) -> Result<PipelineResult> {
    if input_format == ImageOutputFormat::Jpeg && output_format == ImageOutputFormat::Jpeg {
        let with_seed = steganography.apply_qtable_seed_bytes(img_bytes, plan.seed())?;
        let bytes = metadata_trap.inject_bytes_from_plan(&with_seed, plan)?;
        observe_metadata_work(&bytes, output_format, budget)?;
        return Ok(PipelineResult {
            bytes,
            embed_summary: None,
        });
    }

    if output_format == ImageOutputFormat::Jpeg {
        let img = load_image_from_bytes(img_bytes)?;
        let (width, height) = img.dimensions();
        plan.resource_limits().check_dimensions(width, height)?;
        let jpeg_bytes = crate::util::image::encode_image_with_options(
            &img,
            Some(output_format),
            plan.processing().progressive_jpeg,
            plan.processing().jpeg_quality,
        )?;
        let with_metadata = metadata_trap.inject_bytes_from_plan(&jpeg_bytes, plan)?;
        let with_seed = steganography.apply_qtable_seed_bytes(&with_metadata, plan.seed())?;
        observe_metadata_work(&with_seed, output_format, budget)?;
        return Ok(PipelineResult {
            bytes: with_seed,
            embed_summary: None,
        });
    }

    let img = load_image_from_bytes(img_bytes)?;
    let (width, height) = img.dimensions();
    plan.resource_limits().check_dimensions(width, height)?;
    let mut rgba = img.to_rgba8();
    SteganographyProtector::embed_seed_lsb_fallback_pub(&mut rgba, plan.seed());
    let stego_img = DynamicImage::ImageRgba8(rgba);
    let encoded = crate::util::image::encode_image_with_options(
        &stego_img,
        Some(output_format),
        plan.processing().progressive_jpeg,
        plan.processing().jpeg_quality,
    )?;
    let bytes = metadata_trap.inject_bytes_from_plan(&encoded, plan)?;
    observe_metadata_work(&bytes, output_format, budget)?;
    Ok(PipelineResult {
        bytes,
        embed_summary: None,
    })
}
