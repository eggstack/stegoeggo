use crate::codes::stegoeggo_v1_status_t;
use crate::handles::{stegoeggo_v1_error_t, stegoeggo_v1_resource_limits_t};
use crate::input::{require_handle_mut, require_out};
use crate::panic::invoke;

fn set_limit(
    limits: *mut stegoeggo_v1_resource_limits_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
    apply: impl FnOnce(stegoeggo::ResourceLimits) -> stegoeggo::ResourceLimits,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let handle = require_handle_mut(limits, "limits must be non-NULL")?;
        let current = core::mem::take(&mut handle.inner);
        handle.inner = apply(current);
        Ok(())
    })
}

fn rebuild(
    current: &stegoeggo::ResourceLimits,
    field: LimitField,
    usize_value: usize,
    u32_value: u32,
) -> stegoeggo::ResourceLimits {
    let builder = stegoeggo::ResourceLimits::builder()
        .max_input_bytes(current.max_input_bytes())
        .max_width(current.max_width())
        .max_height(current.max_height())
        .max_png_chunks(current.max_png_chunks())
        .max_png_chunk_bytes(current.max_png_chunk_bytes())
        .max_jpeg_segments(current.max_jpeg_segments())
        .max_jpeg_segment_bytes(current.max_jpeg_segment_bytes())
        .max_webp_riff_chunks(current.max_webp_riff_chunks())
        .max_webp_riff_bytes(current.max_webp_riff_bytes())
        .max_xmp_bytes(current.max_xmp_bytes())
        .max_xml_depth(current.max_xml_depth())
        .max_xml_properties(current.max_xml_properties())
        .max_metadata_fields(current.max_metadata_fields())
        .max_metadata_field_bytes(current.max_metadata_field_bytes())
        .max_payload_bytes(current.max_payload_bytes())
        .max_detached_manifest_bytes(current.max_detached_manifest_bytes())
        .max_tile_extraction_origins(current.max_tile_extraction_origins())
        .max_verification_seeds(current.max_verification_seeds());
    let builder = match field {
        LimitField::InputBytes => builder.max_input_bytes(usize_value),
        LimitField::Width => builder.max_width(u32_value),
        LimitField::Height => builder.max_height(u32_value),
        LimitField::PngChunks => builder.max_png_chunks(usize_value),
        LimitField::PngChunkBytes => builder.max_png_chunk_bytes(usize_value),
        LimitField::JpegSegments => builder.max_jpeg_segments(usize_value),
        LimitField::JpegSegmentBytes => builder.max_jpeg_segment_bytes(usize_value),
        LimitField::WebpRiffChunks => builder.max_webp_riff_chunks(usize_value),
        LimitField::WebpRiffBytes => builder.max_webp_riff_bytes(usize_value),
        LimitField::XmpBytes => builder.max_xmp_bytes(usize_value),
        LimitField::XmlDepth => builder.max_xml_depth(usize_value),
        LimitField::XmlProperties => builder.max_xml_properties(usize_value),
        LimitField::MetadataFields => builder.max_metadata_fields(usize_value),
        LimitField::MetadataFieldBytes => builder.max_metadata_field_bytes(usize_value),
        LimitField::PayloadBytes => builder.max_payload_bytes(usize_value),
        LimitField::DetachedManifestBytes => builder.max_detached_manifest_bytes(usize_value),
        LimitField::TileExtractionOrigins => builder.max_tile_extraction_origins(usize_value),
        LimitField::VerificationSeeds => builder.max_verification_seeds(usize_value),
    };
    builder.build()
}

#[derive(Debug, Clone, Copy)]
enum LimitField {
    InputBytes,
    Width,
    Height,
    PngChunks,
    PngChunkBytes,
    JpegSegments,
    JpegSegmentBytes,
    WebpRiffChunks,
    WebpRiffBytes,
    XmpBytes,
    XmlDepth,
    XmlProperties,
    MetadataFields,
    MetadataFieldBytes,
    PayloadBytes,
    DetachedManifestBytes,
    TileExtractionOrigins,
    VerificationSeeds,
}

fn set_usize_field(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    field: LimitField,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_limit(limits, out_error, |current| {
        rebuild(&current, field, value, 0)
    })
}

fn set_u32_field(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: u32,
    field: LimitField,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_limit(limits, out_error, |current| {
        rebuild(&current, field, 0, value)
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_create(
    out_limits: *mut *mut stegoeggo_v1_resource_limits_t,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    invoke(out_error, || {
        let slot = require_out(out_limits, "out_limits must be non-NULL")?;
        let handle = stegoeggo_v1_resource_limits_t {
            inner: stegoeggo::ResourceLimits::default(),
        };
        *slot = Box::into_raw(Box::new(handle));
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_free(limits: *mut stegoeggo_v1_resource_limits_t) {
    if limits.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(limits));
    }
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_input_bytes(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::InputBytes, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_width(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: u32,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_u32_field(limits, value, LimitField::Width, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_height(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: u32,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_u32_field(limits, value, LimitField::Height, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_png_chunks(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::PngChunks, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_png_chunk_bytes(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::PngChunkBytes, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_jpeg_segments(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::JpegSegments, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_jpeg_segment_bytes(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::JpegSegmentBytes, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_webp_riff_chunks(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::WebpRiffChunks, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_webp_riff_bytes(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::WebpRiffBytes, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_xmp_bytes(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::XmpBytes, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_xml_depth(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::XmlDepth, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_xml_properties(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::XmlProperties, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_metadata_fields(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::MetadataFields, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_metadata_field_bytes(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::MetadataFieldBytes, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_payload_bytes(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::PayloadBytes, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_detached_manifest_bytes(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::DetachedManifestBytes, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_tile_extraction_origins(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::TileExtractionOrigins, out_error)
}

#[no_mangle]
pub extern "C" fn stegoeggo_v1_resource_limits_set_max_verification_seeds(
    limits: *mut stegoeggo_v1_resource_limits_t,
    value: usize,
    out_error: *mut *mut stegoeggo_v1_error_t,
) -> stegoeggo_v1_status_t {
    set_usize_field(limits, value, LimitField::VerificationSeeds, out_error)
}
