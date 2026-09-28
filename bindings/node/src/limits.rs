use crate::error::Result;
use napi_derive::napi;
use stegoeggo::ResourceLimits as RustLimits;

/// Parser hardening limits applied to untrusted input.
///
/// `ResourceLimits.defaults()` returns the canonical defaults; a
/// `ResourceLimitsBuilder` derives a customized set.
#[derive(Clone, Debug)]
#[napi]
pub struct ResourceLimits {
    inner: RustLimits,
}

#[napi]
impl ResourceLimits {
    /// Returns the canonical default limits.
    #[napi(factory)]
    pub fn defaults() -> Self {
        Self {
            inner: RustLimits::default(),
        }
    }

    /// Starts a builder. Like the canonical `ResourceLimits::builder()`, the
    /// new builder starts from the canonical defaults rather than from this
    /// value.
    #[must_use]
    #[napi]
    pub fn to_builder(&self) -> ResourceLimitsBuilder {
        ResourceLimitsBuilder::new()
    }

    /// Maximum accepted input size in bytes.
    #[napi(getter)]
    pub fn max_input_bytes(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_input_bytes())
    }

    /// Maximum accepted image width in pixels.
    #[napi(getter)]
    pub fn max_width(&self) -> u32 {
        self.inner.max_width()
    }

    /// Maximum accepted image height in pixels.
    #[napi(getter)]
    pub fn max_height(&self) -> u32 {
        self.inner.max_height()
    }

    /// Maximum number of PNG chunks scanned.
    #[napi(getter)]
    pub fn max_png_chunks(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_png_chunks())
    }

    /// Maximum PNG chunk payload size in bytes.
    #[napi(getter)]
    pub fn max_png_chunk_bytes(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_png_chunk_bytes())
    }

    /// Maximum number of JPEG segments scanned.
    #[napi(getter)]
    pub fn max_jpeg_segments(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_jpeg_segments())
    }

    /// Maximum JPEG segment payload size in bytes.
    #[napi(getter)]
    pub fn max_jpeg_segment_bytes(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_jpeg_segment_bytes())
    }

    /// Maximum number of WebP RIFF chunks scanned.
    #[napi(getter)]
    pub fn max_webp_riff_chunks(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_webp_riff_chunks())
    }

    /// Maximum WebP RIFF chunk payload size in bytes.
    #[napi(getter)]
    pub fn max_webp_riff_bytes(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_webp_riff_bytes())
    }

    /// Maximum XMP packet size in bytes.
    #[napi(getter)]
    pub fn max_xmp_bytes(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_xmp_bytes())
    }

    /// Maximum XML nesting depth.
    #[napi(getter)]
    pub fn max_xml_depth(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_xml_depth())
    }

    /// Maximum number of XML properties parsed.
    #[napi(getter)]
    pub fn max_xml_properties(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_xml_properties())
    }

    /// Maximum number of metadata fields extracted.
    #[napi(getter)]
    pub fn max_metadata_fields(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_metadata_fields())
    }

    /// Maximum per-field metadata size in bytes.
    #[napi(getter)]
    pub fn max_metadata_field_bytes(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_metadata_field_bytes())
    }

    /// Maximum embedded payload size in bytes.
    #[napi(getter)]
    pub fn max_payload_bytes(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_payload_bytes())
    }

    /// Maximum detached manifest size in bytes.
    #[napi(getter)]
    pub fn max_detached_manifest_bytes(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_detached_manifest_bytes())
    }

    /// Maximum number of tile extraction origins checked.
    #[napi(getter)]
    pub fn max_tile_extraction_origins(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_tile_extraction_origins())
    }

    /// Maximum number of verification seeds tried.
    #[napi(getter)]
    pub fn max_verification_seeds(&self) -> f64 {
        crate::numeric::from_usize(self.inner.max_verification_seeds())
    }
}

impl From<&ResourceLimits> for RustLimits {
    fn from(value: &ResourceLimits) -> Self {
        value.inner.clone()
    }
}

/// Chainable builder for [`ResourceLimits`].
///
/// The builder is immutable: every `with*` call returns a new builder, so a
/// partially configured builder can be shared safely. Limits that are never set
/// fall back to the canonical defaults.
#[derive(Debug)]
#[napi]
pub struct ResourceLimitsBuilder {
    overrides: LimitOverrides,
}

#[derive(Debug, Clone, Default)]
struct LimitOverrides {
    max_input_bytes: Option<usize>,
    max_width: Option<u32>,
    max_height: Option<u32>,
    max_png_chunks: Option<usize>,
    max_png_chunk_bytes: Option<usize>,
    max_jpeg_segments: Option<usize>,
    max_jpeg_segment_bytes: Option<usize>,
    max_webp_riff_chunks: Option<usize>,
    max_webp_riff_bytes: Option<usize>,
    max_xmp_bytes: Option<usize>,
    max_xml_depth: Option<usize>,
    max_xml_properties: Option<usize>,
    max_metadata_fields: Option<usize>,
    max_metadata_field_bytes: Option<usize>,
    max_payload_bytes: Option<usize>,
    max_detached_manifest_bytes: Option<usize>,
    max_tile_extraction_origins: Option<usize>,
    max_verification_seeds: Option<usize>,
}

#[napi]
impl ResourceLimitsBuilder {
    /// Starts a builder seeded with the canonical default limits.
    #[napi(factory)]
    pub fn new() -> Self {
        Self {
            overrides: LimitOverrides::default(),
        }
    }

    /// Sets the `maxInputBytes` limit.
    #[napi]
    pub fn with_max_input_bytes(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_input_bytes = Some(crate::numeric::to_usize("maxInputBytes", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxWidth` limit.
    #[napi]
    pub fn with_max_width(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_width = Some(crate::numeric::to_u32("maxWidth", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxHeight` limit.
    #[napi]
    pub fn with_max_height(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_height = Some(crate::numeric::to_u32("maxHeight", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxPngChunks` limit.
    #[napi]
    pub fn with_max_png_chunks(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_png_chunks = Some(crate::numeric::to_usize("maxPngChunks", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxPngChunkBytes` limit.
    #[napi]
    pub fn with_max_png_chunk_bytes(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_png_chunk_bytes = Some(crate::numeric::to_usize("maxPngChunkBytes", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxJpegSegments` limit.
    #[napi]
    pub fn with_max_jpeg_segments(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_jpeg_segments = Some(crate::numeric::to_usize("maxJpegSegments", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxJpegSegmentBytes` limit.
    #[napi]
    pub fn with_max_jpeg_segment_bytes(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_jpeg_segment_bytes =
            Some(crate::numeric::to_usize("maxJpegSegmentBytes", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxWebpRiffChunks` limit.
    #[napi]
    pub fn with_max_webp_riff_chunks(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_webp_riff_chunks =
            Some(crate::numeric::to_usize("maxWebpRiffChunks", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxWebpRiffBytes` limit.
    #[napi]
    pub fn with_max_webp_riff_bytes(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_webp_riff_bytes = Some(crate::numeric::to_usize("maxWebpRiffBytes", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxXmpBytes` limit.
    #[napi]
    pub fn with_max_xmp_bytes(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_xmp_bytes = Some(crate::numeric::to_usize("maxXmpBytes", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxXmlDepth` limit.
    #[napi]
    pub fn with_max_xml_depth(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_xml_depth = Some(crate::numeric::to_usize("maxXmlDepth", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxXmlProperties` limit.
    #[napi]
    pub fn with_max_xml_properties(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_xml_properties = Some(crate::numeric::to_usize("maxXmlProperties", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxMetadataFields` limit.
    #[napi]
    pub fn with_max_metadata_fields(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_metadata_fields = Some(crate::numeric::to_usize("maxMetadataFields", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxMetadataFieldBytes` limit.
    #[napi]
    pub fn with_max_metadata_field_bytes(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_metadata_field_bytes =
            Some(crate::numeric::to_usize("maxMetadataFieldBytes", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxPayloadBytes` limit.
    #[napi]
    pub fn with_max_payload_bytes(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_payload_bytes = Some(crate::numeric::to_usize("maxPayloadBytes", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxDetachedManifestBytes` limit.
    #[napi]
    pub fn with_max_detached_manifest_bytes(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_detached_manifest_bytes =
            Some(crate::numeric::to_usize("maxDetachedManifestBytes", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxTileExtractionOrigins` limit.
    #[napi]
    pub fn with_max_tile_extraction_origins(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_tile_extraction_origins =
            Some(crate::numeric::to_usize("maxTileExtractionOrigins", value)?);
        Ok(Self { overrides })
    }

    /// Sets the `maxVerificationSeeds` limit.
    #[napi]
    pub fn with_max_verification_seeds(&self, value: f64) -> Result<Self> {
        let mut overrides = self.overrides.clone();
        overrides.max_verification_seeds =
            Some(crate::numeric::to_usize("maxVerificationSeeds", value)?);
        Ok(Self { overrides })
    }

    /// Builds the resource limits.
    #[must_use]
    #[napi]
    pub fn build(&self) -> ResourceLimits {
        let mut builder = RustLimits::builder();
        if let Some(value) = self.overrides.max_input_bytes {
            builder = builder.max_input_bytes(value);
        }

        if let Some(value) = self.overrides.max_width {
            builder = builder.max_width(value);
        }

        if let Some(value) = self.overrides.max_height {
            builder = builder.max_height(value);
        }

        if let Some(value) = self.overrides.max_png_chunks {
            builder = builder.max_png_chunks(value);
        }

        if let Some(value) = self.overrides.max_png_chunk_bytes {
            builder = builder.max_png_chunk_bytes(value);
        }

        if let Some(value) = self.overrides.max_jpeg_segments {
            builder = builder.max_jpeg_segments(value);
        }

        if let Some(value) = self.overrides.max_jpeg_segment_bytes {
            builder = builder.max_jpeg_segment_bytes(value);
        }

        if let Some(value) = self.overrides.max_webp_riff_chunks {
            builder = builder.max_webp_riff_chunks(value);
        }

        if let Some(value) = self.overrides.max_webp_riff_bytes {
            builder = builder.max_webp_riff_bytes(value);
        }

        if let Some(value) = self.overrides.max_xmp_bytes {
            builder = builder.max_xmp_bytes(value);
        }

        if let Some(value) = self.overrides.max_xml_depth {
            builder = builder.max_xml_depth(value);
        }

        if let Some(value) = self.overrides.max_xml_properties {
            builder = builder.max_xml_properties(value);
        }

        if let Some(value) = self.overrides.max_metadata_fields {
            builder = builder.max_metadata_fields(value);
        }

        if let Some(value) = self.overrides.max_metadata_field_bytes {
            builder = builder.max_metadata_field_bytes(value);
        }

        if let Some(value) = self.overrides.max_payload_bytes {
            builder = builder.max_payload_bytes(value);
        }

        if let Some(value) = self.overrides.max_detached_manifest_bytes {
            builder = builder.max_detached_manifest_bytes(value);
        }

        if let Some(value) = self.overrides.max_tile_extraction_origins {
            builder = builder.max_tile_extraction_origins(value);
        }

        if let Some(value) = self.overrides.max_verification_seeds {
            builder = builder.max_verification_seeds(value);
        }
        ResourceLimits {
            inner: builder.build(),
        }
    }
}

impl Default for ResourceLimitsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
