use crate::error::StegoError;

pub(crate) const DEFAULT_MAX_INPUT_BYTES: usize = 100 * 1024 * 1024;
pub(crate) const DEFAULT_MAX_WIDTH: u32 = 16384;
pub(crate) const DEFAULT_MAX_HEIGHT: u32 = 16384;
pub(crate) const DEFAULT_MAX_PIXELS: u64 = 16384 * 16384;
pub(crate) const DEFAULT_MAX_JPEG_SEGMENTS: usize = 256;
pub(crate) const DEFAULT_MAX_JPEG_SEGMENT_BYTES: usize = 65535;
pub(crate) const DEFAULT_MAX_TILED_ORIGINS: u32 = crate::types::MAX_TILED_ORIGINS;

fn default_max_frame_bytes() -> usize {
    crate::frame::MAX_FRAME_PAYLOAD + crate::frame::FRAME_HEADER_SIZE
}

/// Bounded untrusted-input contract for generic carrier operations.
///
/// Carrier-owned limits over encoded-input size, JPEG segment structure,
/// decoded dimensions/pixels, framed-payload size, and tiled-search extent.
/// All fields are private with getters and a builder so the type can evolve
/// semver-safely. Limit failures map to
/// [`StegoError::ResourceLimitExceeded`] without exposing secret or input
/// bytes.
///
/// Defaults preserve current one-shot behavior for ordinary inputs while
/// bounding adversarial amplification: 100 MiB input, 16384×16384
/// dimensions/pixels, 256 JPEG segments of at most 65535 bytes each, framed
/// totals up to 16 MiB plus the 11-byte header, and at most
/// [`crate::types::MAX_TILED_ORIGINS`] tiled origins.
#[derive(Debug, Clone)]
pub struct CarrierLimits {
    max_input_bytes: usize,
    max_width: u32,
    max_height: u32,
    max_pixels: u64,
    max_jpeg_segments: usize,
    max_jpeg_segment_bytes: usize,
    max_frame_bytes: usize,
    max_tiled_origins: u32,
}

impl Default for CarrierLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: DEFAULT_MAX_INPUT_BYTES,
            max_width: DEFAULT_MAX_WIDTH,
            max_height: DEFAULT_MAX_HEIGHT,
            max_pixels: DEFAULT_MAX_PIXELS,
            max_jpeg_segments: DEFAULT_MAX_JPEG_SEGMENTS,
            max_jpeg_segment_bytes: DEFAULT_MAX_JPEG_SEGMENT_BYTES,
            max_frame_bytes: default_max_frame_bytes(),
            max_tiled_origins: DEFAULT_MAX_TILED_ORIGINS,
        }
    }
}

impl CarrierLimits {
    pub(crate) fn legacy_compatibility() -> Self {
        let structural_max = u64::from(u16::MAX)
            .checked_mul(u64::from(u16::MAX))
            .expect("JPEG structural pixel maximum fits in u64");
        Self {
            max_input_bytes: usize::MAX,
            max_width: u32::from(u16::MAX),
            max_height: u32::from(u16::MAX),
            max_pixels: structural_max,
            ..Self::default()
        }
    }

    pub fn builder() -> CarrierLimitsBuilder {
        CarrierLimitsBuilder(CarrierLimits::default())
    }

    #[must_use]
    pub fn max_input_bytes(&self) -> usize {
        self.max_input_bytes
    }

    #[must_use]
    pub fn max_width(&self) -> u32 {
        self.max_width
    }

    #[must_use]
    pub fn max_height(&self) -> u32 {
        self.max_height
    }

    #[must_use]
    pub fn max_pixels(&self) -> u64 {
        self.max_pixels
    }

    #[must_use]
    pub fn max_jpeg_segments(&self) -> usize {
        self.max_jpeg_segments
    }

    #[must_use]
    pub fn max_jpeg_segment_bytes(&self) -> usize {
        self.max_jpeg_segment_bytes
    }

    #[must_use]
    pub fn max_frame_bytes(&self) -> usize {
        self.max_frame_bytes
    }

    #[must_use]
    pub fn max_tiled_origins(&self) -> u32 {
        self.max_tiled_origins
    }

    pub(crate) fn parse_limits(&self) -> crate::jpeg_transcoder::header::ParseLimits {
        crate::jpeg_transcoder::header::ParseLimits {
            max_jpeg_segments: self.max_jpeg_segments,
            max_jpeg_segment_bytes: self.max_jpeg_segment_bytes,
        }
    }

    pub(crate) fn check_input_bytes(&self, len: usize) -> Result<(), StegoError> {
        if len > self.max_input_bytes {
            return Err(StegoError::ResourceLimitExceeded(format!(
                "input size {len} exceeds limit {}",
                self.max_input_bytes
            )));
        }
        Ok(())
    }

    pub(crate) fn check_dimensions(&self, width: u32, height: u32) -> Result<(), StegoError> {
        if width > self.max_width || height > self.max_height {
            return Err(StegoError::ResourceLimitExceeded(format!(
                "dimensions {width}x{height} exceed limit {}x{}",
                self.max_width, self.max_height
            )));
        }
        let pixels = width as u64 * height as u64;
        if pixels > self.max_pixels {
            return Err(StegoError::ResourceLimitExceeded(format!(
                "pixel count {pixels} exceeds limit {}",
                self.max_pixels
            )));
        }
        Ok(())
    }

    pub(crate) fn check_frame_bytes(&self, total_len: usize) -> Result<(), StegoError> {
        if total_len > self.max_frame_bytes {
            return Err(StegoError::ResourceLimitExceeded(format!(
                "frame size {total_len} exceeds limit {}",
                self.max_frame_bytes
            )));
        }
        Ok(())
    }

    pub(crate) fn check_tiled_origins(&self, max_origins: u32) -> Result<(), StegoError> {
        crate::types::validate_max_origins(max_origins)?;
        if max_origins > self.max_tiled_origins {
            return Err(StegoError::ResourceLimitExceeded(format!(
                "tiled origins {max_origins} exceed limit {}",
                self.max_tiled_origins
            )));
        }
        Ok(())
    }
}

/// Builder for [`CarrierLimits`].
#[derive(Debug, Clone)]
pub struct CarrierLimitsBuilder(CarrierLimits);

impl CarrierLimitsBuilder {
    #[must_use]
    pub fn max_input_bytes(mut self, value: usize) -> Self {
        self.0.max_input_bytes = value;
        self
    }

    #[must_use]
    pub fn max_width(mut self, value: u32) -> Self {
        self.0.max_width = value;
        self
    }

    #[must_use]
    pub fn max_height(mut self, value: u32) -> Self {
        self.0.max_height = value;
        self
    }

    #[must_use]
    pub fn max_pixels(mut self, value: u64) -> Self {
        self.0.max_pixels = value;
        self
    }

    #[must_use]
    pub fn max_jpeg_segments(mut self, value: usize) -> Self {
        self.0.max_jpeg_segments = value;
        self
    }

    #[must_use]
    pub fn max_jpeg_segment_bytes(mut self, value: usize) -> Self {
        self.0.max_jpeg_segment_bytes = value;
        self
    }

    #[must_use]
    pub fn max_frame_bytes(mut self, value: usize) -> Self {
        self.0.max_frame_bytes = value;
        self
    }

    #[must_use]
    pub fn max_tiled_origins(mut self, value: u32) -> Self {
        self.0.max_tiled_origins = value;
        self
    }

    #[must_use]
    pub fn build(self) -> CarrierLimits {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_documented_bounds() {
        let limits = CarrierLimits::default();
        assert_eq!(limits.max_input_bytes(), 100 * 1024 * 1024);
        assert_eq!(limits.max_width(), 16384);
        assert_eq!(limits.max_height(), 16384);
        assert_eq!(limits.max_pixels(), 16384 * 16384);
        assert_eq!(limits.max_jpeg_segments(), 256);
        assert_eq!(limits.max_jpeg_segment_bytes(), 65535);
        assert_eq!(
            limits.max_frame_bytes(),
            crate::frame::MAX_FRAME_PAYLOAD + crate::frame::FRAME_HEADER_SIZE
        );
        assert_eq!(limits.max_tiled_origins(), crate::types::MAX_TILED_ORIGINS);
    }

    #[test]
    fn legacy_profile_keeps_parser_and_structural_bounds() {
        let limits = CarrierLimits::legacy_compatibility();
        assert_eq!(limits.max_input_bytes(), usize::MAX);
        assert_eq!(limits.max_width(), u32::from(u16::MAX));
        assert_eq!(limits.max_height(), u32::from(u16::MAX));
        assert_eq!(
            limits.max_pixels(),
            u64::from(u16::MAX)
                .checked_mul(u64::from(u16::MAX))
                .unwrap()
        );
        assert_eq!(limits.max_jpeg_segments(), DEFAULT_MAX_JPEG_SEGMENTS);
        assert_eq!(
            limits.max_jpeg_segment_bytes(),
            DEFAULT_MAX_JPEG_SEGMENT_BYTES
        );
        assert_eq!(limits.max_tiled_origins(), DEFAULT_MAX_TILED_ORIGINS);
        assert!(limits
            .check_dimensions(u32::from(u16::MAX), u32::from(u16::MAX))
            .is_ok());
    }

    #[test]
    fn builder_round_trips_custom_values() {
        let limits = CarrierLimits::builder()
            .max_input_bytes(1024)
            .max_width(64)
            .max_height(64)
            .max_pixels(4096)
            .max_jpeg_segments(8)
            .max_jpeg_segment_bytes(1024)
            .max_frame_bytes(128)
            .max_tiled_origins(4)
            .build();
        assert_eq!(limits.max_input_bytes(), 1024);
        assert_eq!(limits.max_width(), 64);
        assert_eq!(limits.max_height(), 64);
        assert_eq!(limits.max_pixels(), 4096);
        assert_eq!(limits.max_jpeg_segments(), 8);
        assert_eq!(limits.max_jpeg_segment_bytes(), 1024);
        assert_eq!(limits.max_frame_bytes(), 128);
        assert_eq!(limits.max_tiled_origins(), 4);
    }

    #[test]
    fn checks_map_to_resource_limit_without_input_dump() {
        let limits = CarrierLimits::builder()
            .max_input_bytes(8)
            .max_width(16)
            .max_height(16)
            .max_pixels(255)
            .max_frame_bytes(11)
            .max_tiled_origins(2)
            .build();
        assert!(matches!(
            limits.check_input_bytes(9),
            Err(StegoError::ResourceLimitExceeded(_))
        ));
        assert!(matches!(
            limits.check_dimensions(17, 4),
            Err(StegoError::ResourceLimitExceeded(_))
        ));
        assert!(matches!(
            limits.check_dimensions(16, 16),
            Err(StegoError::ResourceLimitExceeded(_))
        ));
        assert!(matches!(
            limits.check_frame_bytes(12),
            Err(StegoError::ResourceLimitExceeded(_))
        ));
        assert!(matches!(
            limits.check_tiled_origins(3),
            Err(StegoError::ResourceLimitExceeded(_))
        ));
        assert!(matches!(
            limits.check_tiled_origins(0),
            Err(StegoError::InvalidConfig(_))
        ));
    }
}
