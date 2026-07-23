use std::fmt;

/// Limits applied to externally reachable parsers to prevent resource exhaustion.
///
/// Defaults are chosen to be safe for web-facing services while allowing
/// normal photographs. Higher limits can be set explicitly via the builder.
///
/// # Denial-of-service implications
///
/// Parsers validate declared lengths and counts before allocating. Exceeding
/// defaults returns structured errors (`Error::InputTooLarge`,
/// `Error::ContainerLimitExceeded`, etc.) rather than panicking or allocating
/// unbounded memory.
///
/// # Examples
///
/// ```rust
/// use stegoeggo::ResourceLimits;
///
/// let limits = ResourceLimits::builder()
///     .max_input_bytes(50 * 1024 * 1024)
///     .max_png_chunks(1000)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    max_input_bytes: usize,
    max_width: u32,
    max_height: u32,
    max_png_chunks: usize,
    max_png_chunk_bytes: usize,
    max_jpeg_segments: usize,
    max_jpeg_segment_bytes: usize,
    max_webp_riff_chunks: usize,
    max_webp_riff_bytes: usize,
    max_xmp_bytes: usize,
    max_xml_depth: usize,
    max_xml_properties: usize,
    max_metadata_fields: usize,
    max_metadata_field_bytes: usize,
    max_payload_bytes: usize,
    max_detached_manifest_bytes: usize,
    max_tile_extraction_origins: usize,
    max_verification_seeds: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 100 * 1024 * 1024,
            max_width: 16384,
            max_height: 16384,
            max_png_chunks: 500,
            max_png_chunk_bytes: 16 * 1024 * 1024,
            max_jpeg_segments: 256,
            max_jpeg_segment_bytes: 65535,
            max_webp_riff_chunks: 500,
            max_webp_riff_bytes: 16 * 1024 * 1024,
            max_xmp_bytes: 65535,
            max_xml_depth: 32,
            max_xml_properties: 256,
            max_metadata_fields: 64,
            max_metadata_field_bytes: 8192,
            max_payload_bytes: 256,
            max_detached_manifest_bytes: 1024 * 1024,
            max_tile_extraction_origins: 16,
            max_verification_seeds: 32,
        }
    }
}

impl ResourceLimits {
    /// Create a builder for resource limits.
    pub fn builder() -> ResourceLimitsBuilder {
        ResourceLimitsBuilder(ResourceLimits::default())
    }

    /// Maximum input size in bytes.
    pub fn max_input_bytes(&self) -> usize {
        self.max_input_bytes
    }

    /// Maximum image width in pixels.
    pub fn max_width(&self) -> u32 {
        self.max_width
    }

    /// Maximum image height in pixels.
    pub fn max_height(&self) -> u32 {
        self.max_height
    }

    /// Maximum number of PNG chunks to scan.
    pub fn max_png_chunks(&self) -> usize {
        self.max_png_chunks
    }

    /// Maximum PNG chunk payload size in bytes.
    pub fn max_png_chunk_bytes(&self) -> usize {
        self.max_png_chunk_bytes
    }

    /// Maximum number of JPEG segments to scan.
    pub fn max_jpeg_segments(&self) -> usize {
        self.max_jpeg_segments
    }

    /// Maximum JPEG segment payload size in bytes.
    pub fn max_jpeg_segment_bytes(&self) -> usize {
        self.max_jpeg_segment_bytes
    }

    /// Maximum number of WebP RIFF chunks to scan.
    pub fn max_webp_riff_chunks(&self) -> usize {
        self.max_webp_riff_chunks
    }

    /// Maximum WebP RIFF chunk payload size in bytes.
    pub fn max_webp_riff_bytes(&self) -> usize {
        self.max_webp_riff_bytes
    }

    /// Maximum XMP packet size in bytes.
    pub fn max_xmp_bytes(&self) -> usize {
        self.max_xmp_bytes
    }

    /// Maximum XML nesting depth.
    pub fn max_xml_depth(&self) -> usize {
        self.max_xml_depth
    }

    /// Maximum number of XML properties to parse.
    pub fn max_xml_properties(&self) -> usize {
        self.max_xml_properties
    }

    /// Maximum number of metadata fields to extract.
    pub fn max_metadata_fields(&self) -> usize {
        self.max_metadata_fields
    }

    /// Maximum per-field metadata size in bytes.
    pub fn max_metadata_field_bytes(&self) -> usize {
        self.max_metadata_field_bytes
    }

    /// Maximum embedded payload size in bytes.
    pub fn max_payload_bytes(&self) -> usize {
        self.max_payload_bytes
    }

    /// Maximum detached manifest size in bytes.
    pub fn max_detached_manifest_bytes(&self) -> usize {
        self.max_detached_manifest_bytes
    }

    /// Maximum number of tile extraction origins to check.
    pub fn max_tile_extraction_origins(&self) -> usize {
        self.max_tile_extraction_origins
    }

    /// Maximum number of verification seeds to try.
    pub fn max_verification_seeds(&self) -> usize {
        self.max_verification_seeds
    }

    /// Check that input size is within limits.
    pub fn check_input_size(&self, size: usize) -> crate::Result<()> {
        if size > self.max_input_bytes {
            return Err(crate::Error::InputTooLarge {
                size,
                limit: self.max_input_bytes,
            });
        }
        Ok(())
    }

    /// Check that image dimensions are within limits.
    pub fn check_dimensions(&self, width: u32, height: u32) -> crate::Result<()> {
        if width > self.max_width || height > self.max_height {
            return Err(crate::Error::DimensionsExceeded {
                width,
                height,
                max_width: self.max_width,
                max_height: self.max_height,
            });
        }
        Ok(())
    }

    /// Check that a container element count is within limits.
    pub fn check_container_count(
        &self,
        kind: &'static str,
        count: usize,
        limit: usize,
    ) -> crate::Result<()> {
        if count > limit {
            return Err(crate::Error::ContainerLimitExceeded { kind, count, limit });
        }
        Ok(())
    }

    /// Check that a metadata payload size is within limits.
    pub fn check_metadata_size(
        &self,
        kind: &'static str,
        size: usize,
        limit: usize,
    ) -> crate::Result<()> {
        if size > limit {
            return Err(crate::Error::MetadataLimitExceeded { kind, size, limit });
        }
        Ok(())
    }

    /// Check that the number of metadata fields is within limits.
    pub fn check_metadata_field_count(&self, count: usize) -> crate::Result<()> {
        if count > self.max_metadata_fields {
            return Err(crate::Error::ContainerLimitExceeded {
                kind: "metadata fields",
                count,
                limit: self.max_metadata_fields,
            });
        }
        Ok(())
    }
}

/// Builder for [`ResourceLimits`].
pub struct ResourceLimitsBuilder(ResourceLimits);

impl ResourceLimitsBuilder {
    /// Set the maximum input size in bytes.
    pub fn max_input_bytes(mut self, val: usize) -> Self {
        self.0.max_input_bytes = val;
        self
    }

    /// Set the maximum image width in pixels.
    pub fn max_width(mut self, val: u32) -> Self {
        self.0.max_width = val;
        self
    }

    /// Set the maximum image height in pixels.
    pub fn max_height(mut self, val: u32) -> Self {
        self.0.max_height = val;
        self
    }

    /// Set the maximum number of PNG chunks to scan.
    pub fn max_png_chunks(mut self, val: usize) -> Self {
        self.0.max_png_chunks = val;
        self
    }

    /// Set the maximum PNG chunk payload size in bytes.
    pub fn max_png_chunk_bytes(mut self, val: usize) -> Self {
        self.0.max_png_chunk_bytes = val;
        self
    }

    /// Set the maximum number of JPEG segments to scan.
    pub fn max_jpeg_segments(mut self, val: usize) -> Self {
        self.0.max_jpeg_segments = val;
        self
    }

    /// Set the maximum JPEG segment payload size in bytes.
    pub fn max_jpeg_segment_bytes(mut self, val: usize) -> Self {
        self.0.max_jpeg_segment_bytes = val;
        self
    }

    /// Set the maximum number of WebP RIFF chunks to scan.
    pub fn max_webp_riff_chunks(mut self, val: usize) -> Self {
        self.0.max_webp_riff_chunks = val;
        self
    }

    /// Set the maximum WebP RIFF chunk payload size in bytes.
    pub fn max_webp_riff_bytes(mut self, val: usize) -> Self {
        self.0.max_webp_riff_bytes = val;
        self
    }

    /// Set the maximum XMP packet size in bytes.
    pub fn max_xmp_bytes(mut self, val: usize) -> Self {
        self.0.max_xmp_bytes = val;
        self
    }

    /// Set the maximum XML nesting depth.
    pub fn max_xml_depth(mut self, val: usize) -> Self {
        self.0.max_xml_depth = val;
        self
    }

    /// Set the maximum number of XML properties to parse.
    pub fn max_xml_properties(mut self, val: usize) -> Self {
        self.0.max_xml_properties = val;
        self
    }

    /// Set the maximum number of metadata fields to extract.
    pub fn max_metadata_fields(mut self, val: usize) -> Self {
        self.0.max_metadata_fields = val;
        self
    }

    /// Set the maximum per-field metadata size in bytes.
    pub fn max_metadata_field_bytes(mut self, val: usize) -> Self {
        self.0.max_metadata_field_bytes = val;
        self
    }

    /// Set the maximum embedded payload size in bytes.
    pub fn max_payload_bytes(mut self, val: usize) -> Self {
        self.0.max_payload_bytes = val;
        self
    }

    /// Set the maximum detached manifest size in bytes.
    pub fn max_detached_manifest_bytes(mut self, val: usize) -> Self {
        self.0.max_detached_manifest_bytes = val;
        self
    }

    /// Set the maximum number of tile extraction origins to check.
    pub fn max_tile_extraction_origins(mut self, val: usize) -> Self {
        self.0.max_tile_extraction_origins = val;
        self
    }

    /// Set the maximum number of verification seeds to try.
    pub fn max_verification_seeds(mut self, val: usize) -> Self {
        self.0.max_verification_seeds = val;
        self
    }

    /// Build the resource limits.
    pub fn build(self) -> ResourceLimits {
        self.0
    }
}

/// Budget accounting for a single processing operation.
///
/// Tracks observed counts and bytes so operators can tune limits.
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// Total input bytes processed.
    pub input_bytes: usize,
    /// Number of PNG chunks scanned.
    pub png_chunks_scanned: usize,
    /// Number of JPEG segments scanned.
    pub jpeg_segments_scanned: usize,
    /// Number of WebP RIFF chunks scanned.
    pub webp_riff_chunks_scanned: usize,
    /// Total XMP bytes parsed.
    pub xmp_bytes_parsed: usize,
    /// Number of metadata fields extracted.
    pub metadata_fields_extracted: usize,
    /// Total metadata bytes copied.
    pub metadata_bytes_copied: usize,
    /// Number of tile origins checked during extraction.
    pub tile_origins_checked: usize,
    /// Number of verification seeds tried.
    pub verification_seeds_tried: usize,
    /// Peak allocation size observed.
    pub peak_allocations_bytes: usize,
}

impl ResourceUsage {
    /// Record the start of a processing operation with the given input size.
    pub fn begin(input_bytes: usize) -> Self {
        Self {
            input_bytes,
            peak_allocations_bytes: input_bytes,
            ..Default::default()
        }
    }

    /// Update peak allocation tracking if the given size exceeds the current peak.
    pub fn track_allocation(&mut self, size: usize) {
        if size > self.peak_allocations_bytes {
            self.peak_allocations_bytes = size;
        }
    }

    /// Record that PNG chunks were scanned.
    pub fn record_png_chunks(&mut self, count: usize) {
        self.png_chunks_scanned = count;
    }

    /// Record that JPEG segments were scanned.
    pub fn record_jpeg_segments(&mut self, count: usize) {
        self.jpeg_segments_scanned = count;
    }

    /// Record that WebP RIFF chunks were scanned.
    pub fn record_webp_riff_chunks(&mut self, count: usize) {
        self.webp_riff_chunks_scanned = count;
    }

    /// Record XMP bytes parsed.
    pub fn record_xmp_bytes(&mut self, bytes: usize) {
        self.xmp_bytes_parsed = bytes;
    }

    /// Record metadata extraction results.
    pub fn record_metadata(&mut self, fields: usize, bytes: usize) {
        self.metadata_fields_extracted = fields;
        self.metadata_bytes_copied = bytes;
    }

    /// Record tile origin checks.
    pub fn record_tile_origins(&mut self, count: usize) {
        self.tile_origins_checked = count;
    }

    /// Record verification seed attempts.
    pub fn record_verification_seeds(&mut self, count: usize) {
        self.verification_seeds_tried = count;
    }
}

impl fmt::Display for ResourceUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "input={}B png_chunks={} jpeg_segments={} webp_chunks={} xmp_bytes={} \
             metadata_fields={} metadata_bytes={} tile_origins={} seeds_tried={} peak_alloc={}B",
            self.input_bytes,
            self.png_chunks_scanned,
            self.jpeg_segments_scanned,
            self.webp_riff_chunks_scanned,
            self.xmp_bytes_parsed,
            self.metadata_fields_extracted,
            self.metadata_bytes_copied,
            self.tile_origins_checked,
            self.verification_seeds_tried,
            self.peak_allocations_bytes,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_are_conservative() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_input_bytes(), 100 * 1024 * 1024);
        assert_eq!(limits.max_width(), 16384);
        assert_eq!(limits.max_height(), 16384);
        assert_eq!(limits.max_png_chunks(), 500);
        assert_eq!(limits.max_jpeg_segments(), 256);
        assert_eq!(limits.max_webp_riff_chunks(), 500);
        assert_eq!(limits.max_xmp_bytes(), 65535);
        assert_eq!(limits.max_metadata_fields(), 64);
        assert_eq!(limits.max_payload_bytes(), 256);
        assert_eq!(limits.max_tile_extraction_origins(), 16);
        assert_eq!(limits.max_verification_seeds(), 32);
    }

    #[test]
    fn builder_overrides_defaults() {
        let limits = ResourceLimits::builder()
            .max_input_bytes(50 * 1024 * 1024)
            .max_width(8192)
            .max_png_chunks(1000)
            .build();
        assert_eq!(limits.max_input_bytes(), 50 * 1024 * 1024);
        assert_eq!(limits.max_width(), 8192);
        assert_eq!(limits.max_png_chunks(), 1000);
        assert_eq!(limits.max_height(), 16384);
    }

    #[test]
    fn check_input_size_passes() {
        let limits = ResourceLimits::default();
        assert!(limits.check_input_size(1024).is_ok());
    }

    #[test]
    fn check_input_size_fails() {
        let limits = ResourceLimits::default();
        let err = limits.check_input_size(200 * 1024 * 1024).unwrap_err();
        match err {
            crate::Error::InputTooLarge { size, limit } => {
                assert_eq!(size, 200 * 1024 * 1024);
                assert_eq!(limit, 100 * 1024 * 1024);
            }
            _ => panic!("Expected InputTooLarge"),
        }
    }

    #[test]
    fn check_dimensions_passes() {
        let limits = ResourceLimits::default();
        assert!(limits.check_dimensions(4096, 4096).is_ok());
    }

    #[test]
    fn check_dimensions_fails() {
        let limits = ResourceLimits::default();
        let err = limits.check_dimensions(32768, 32768).unwrap_err();
        match err {
            crate::Error::DimensionsExceeded {
                width,
                height,
                max_width,
                max_height,
            } => {
                assert_eq!(width, 32768);
                assert_eq!(height, 32768);
                assert_eq!(max_width, 16384);
                assert_eq!(max_height, 16384);
            }
            _ => panic!("Expected DimensionsExceeded"),
        }
    }

    #[test]
    fn check_container_count_fails() {
        let limits = ResourceLimits::default();
        let err = limits
            .check_container_count("PNG chunks", 600, 500)
            .unwrap_err();
        match err {
            crate::Error::ContainerLimitExceeded { kind, count, limit } => {
                assert_eq!(kind, "PNG chunks");
                assert_eq!(count, 600);
                assert_eq!(limit, 500);
            }
            _ => panic!("Expected ContainerLimitExceeded"),
        }
    }

    #[test]
    fn resource_usage_default() {
        let usage = ResourceUsage::default();
        assert_eq!(usage.input_bytes, 0);
        assert_eq!(usage.png_chunks_scanned, 0);
    }

    #[test]
    fn resource_usage_display() {
        let usage = ResourceUsage {
            input_bytes: 1024,
            png_chunks_scanned: 10,
            ..Default::default()
        };
        let s = usage.to_string();
        assert!(s.contains("1024"));
        assert!(s.contains("10"));
    }

    #[test]
    fn jpeg_segment_count_exceeds_limit() {
        let limits = ResourceLimits::builder().max_jpeg_segments(3).build();
        let mut data = vec![0xFF, 0xD8]; // SOI
        for i in 0..5u8 {
            data.extend_from_slice(&[0xFF, 0xFE]); // COM marker
            data.extend_from_slice(&[0x00, 0x06]); // length = 6
            data.extend_from_slice(&[i, 0, 0, 0]); // payload
        }
        data.extend_from_slice(&[0xFF, 0xD9]); // EOI
        let result = crate::jpeg_transcoder::header::JpegHeader::parse_with_limits(&data, &limits);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("segment count"),
            "Error should mention segment count: {err}"
        );
    }

    #[test]
    fn jpeg_segment_bytes_exceeds_limit() {
        let limits = ResourceLimits::builder().max_jpeg_segment_bytes(10).build();
        let mut data = vec![0xFF, 0xD8]; // SOI
        data.extend_from_slice(&[0xFF, 0xFE]); // COM marker
        data.extend_from_slice(&[0x00, 0x20]); // length = 32 (exceeds 10)
        data.extend_from_slice(&[0; 30]); // payload
        data.extend_from_slice(&[0xFF, 0xD9]); // EOI
        let result = crate::jpeg_transcoder::header::JpegHeader::parse_with_limits(&data, &limits);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("segment size"),
            "Error should mention segment size: {err}"
        );
    }

    #[test]
    fn png_chunk_count_exceeds_limit() {
        use crate::protected::metadata_trap::MetadataTrapProtector;
        use crate::types::{ImageOutputFormat, ProtectionContext};

        let img = image::DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(16, 16, |x, y| {
            image::Rgb([x as u8, y as u8, 0])
        }));
        let png = crate::util::image::encode_image(&img, image::ImageFormat::Png).unwrap();

        let limits = ResourceLimits::builder().max_png_chunks(2).build();
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_resource_limits(limits);
        let protector = MetadataTrapProtector::new();
        let result = protector.inject_bytes(&png, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn png_chunk_bytes_exceeds_limit() {
        use crate::protected::metadata_trap::MetadataTrapProtector;
        use crate::types::{ImageOutputFormat, ProtectionContext};

        let img = image::DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(16, 16, |x, y| {
            image::Rgb([x as u8, y as u8, 0])
        }));
        let png = crate::util::image::encode_image(&img, image::ImageFormat::Png).unwrap();

        let limits = ResourceLimits::builder().max_png_chunk_bytes(10).build();
        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_resource_limits(limits);
        let protector = MetadataTrapProtector::new();
        let result = protector.inject_bytes(&png, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn input_size_exceeds_limit() {
        let limits = ResourceLimits::builder().max_input_bytes(100).build();
        let err = limits.check_input_size(200).unwrap_err();
        match err {
            crate::Error::InputTooLarge { size, limit } => {
                assert_eq!(size, 200);
                assert_eq!(limit, 100);
            }
            _ => panic!("Expected InputTooLarge"),
        }
    }

    #[test]
    fn dimensions_exceeds_limit() {
        let limits = ResourceLimits::builder()
            .max_width(100)
            .max_height(100)
            .build();
        let err = limits.check_dimensions(200, 200).unwrap_err();
        match err {
            crate::Error::DimensionsExceeded {
                width,
                height,
                max_width,
                max_height,
            } => {
                assert_eq!(width, 200);
                assert_eq!(height, 200);
                assert_eq!(max_width, 100);
                assert_eq!(max_height, 100);
            }
            _ => panic!("Expected DimensionsExceeded"),
        }
    }

    #[test]
    fn metadata_field_bytes_exceeds_limit() {
        let limits = ResourceLimits::builder()
            .max_metadata_field_bytes(10)
            .build();
        let err = limits
            .check_metadata_size("XMP", 20, limits.max_metadata_field_bytes())
            .unwrap_err();
        match err {
            crate::Error::MetadataLimitExceeded { kind, size, limit } => {
                assert_eq!(kind, "XMP");
                assert_eq!(size, 20);
                assert_eq!(limit, 10);
            }
            _ => panic!("Expected MetadataLimitExceeded"),
        }
    }
}
