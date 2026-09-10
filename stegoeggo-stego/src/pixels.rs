//! Borrowed packed/strided pixel buffers for the LSB carrier.
//!
//! [`PixelView`] and [`PixelViewMut`] let callers use caller-owned RGB/RGBA
//! bytes — including strided camera, video, GUI, and FFI buffers — without
//! converting into an `image::RgbaImage`. The logical carrier mapping is
//! identical to the `RgbaImage` path: pixel-major RGB order over
//! `width * height * 3` slots, so equivalent pixels produce byte-identical
//! carrier channels.
//!
//! Alpha bytes (RGBA) and row padding (strided rows) are never carriers and
//! are never mutated. All geometry arithmetic is checked; malformed views
//! return [`StegoError::InvalidConfig`]. Capacity failure is atomic: a
//! failed embed leaves the buffer byte-for-byte unchanged.
//!
//! # Examples
//!
//! ```rust
//! use stegoeggo_stego::lsb::LsbConfig;
//! use stegoeggo_stego::pixels::{PixelLayout, PixelViewMut};
//!
//! let mut bytes = vec![0x7Fu8; 64 * 64 * 3];
//! let mut view = PixelViewMut::new(&mut bytes, 64, 64, PixelLayout::Rgb8, 64 * 3).unwrap();
//! let config = LsbConfig::new(42);
//! let report = view.embed(b"payload", &config).unwrap();
//! assert!(report.embedded);
//! let view = view.as_view();
//! assert_eq!(view.extract(b"payload".len(), &config).unwrap(), b"payload");
//! ```

use crate::error::StegoError;
use crate::lsb::LsbConfig;
use crate::lsb_internal::{
    apply_lsb_bit, checked_lsb_available_slots, embed_tiled_carrier, embed_v2_in_place_carrier,
    extract_tiled_carrier, extract_tiled_framed_carrier, extract_v2_carrier,
    lsb_required_capacity_v2, PixelCarrier, PixelCarrierMut,
};
use crate::types::TileConfig;

/// Packed pixel layout of a borrowed buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PixelLayout {
    /// Three bytes per pixel (R, G, B); every byte is a carrier slot.
    Rgb8,
    /// Four bytes per pixel (R, G, B, A); RGB bytes are carrier slots and
    /// the alpha byte is never read as a carrier nor mutated.
    Rgba8,
}

impl PixelLayout {
    /// Bytes per pixel for this layout.
    #[must_use]
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            PixelLayout::Rgb8 => 3,
            PixelLayout::Rgba8 => 4,
        }
    }
}

fn validate_geometry(
    backing_len: usize,
    width: u32,
    height: u32,
    layout: PixelLayout,
    stride: usize,
) -> Result<BufferGeometry, StegoError> {
    let bytes_per_pixel = layout.bytes_per_pixel();
    let row_len = (width as usize)
        .checked_mul(bytes_per_pixel)
        .ok_or_else(|| {
            StegoError::InvalidConfig(format!("pixel width {width} overflows the row calculation"))
        })?;
    if stride < row_len {
        return Err(StegoError::InvalidConfig(format!(
            "row stride {stride} is smaller than the packed row length {row_len}"
        )));
    }
    let geometry = BufferGeometry {
        width,
        height,
        bytes_per_pixel,
        stride,
    };
    if width == 0 || height == 0 {
        return Ok(geometry);
    }
    let last_row_start = ((height as usize) - 1).checked_mul(stride).ok_or_else(|| {
        StegoError::InvalidConfig(format!(
            "pixel dimensions {width}x{height} with stride {stride} overflow"
        ))
    })?;
    let required = last_row_start.checked_add(row_len).ok_or_else(|| {
        StegoError::InvalidConfig(format!(
            "pixel dimensions {width}x{height} with stride {stride} overflow"
        ))
    })?;
    if backing_len < required {
        return Err(StegoError::InvalidConfig(format!(
            "pixel buffer holds {backing_len} bytes but {required} are required"
        )));
    }
    Ok(geometry)
}

#[derive(Clone, Copy, Debug)]
struct BufferGeometry {
    width: u32,
    height: u32,
    bytes_per_pixel: usize,
    stride: usize,
}

impl BufferGeometry {
    fn channel_offset(&self, backing_len: usize, x: u32, y: u32, channel: usize) -> Option<usize> {
        if x >= self.width || y >= self.height || channel >= 3 {
            return None;
        }
        let offset = (y as usize)
            .checked_mul(self.stride)?
            .checked_add((x as usize).checked_mul(self.bytes_per_pixel)?)?
            .checked_add(channel)?;
        (offset < backing_len).then_some(offset)
    }
}

/// Immutable borrowed packed/strided RGB/RGBA buffer for LSB extraction.
#[derive(Clone, Copy)]
pub struct PixelView<'a> {
    bytes: &'a [u8],
    layout: PixelLayout,
    geometry: BufferGeometry,
}

impl<'a> PixelView<'a> {
    /// Borrow a packed (`stride == width * bytes_per_pixel`) or strided
    /// buffer. Row padding beyond the packed row length is never read as a
    /// carrier.
    pub fn new(
        bytes: &'a [u8],
        width: u32,
        height: u32,
        layout: PixelLayout,
        stride: usize,
    ) -> Result<Self, StegoError> {
        let geometry = validate_geometry(bytes.len(), width, height, layout, stride)?;
        Ok(Self {
            bytes,
            layout,
            geometry,
        })
    }

    /// Buffer width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.geometry.width
    }

    /// Buffer height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.geometry.height
    }

    /// Pixel layout of the buffer.
    #[must_use]
    pub const fn layout(&self) -> PixelLayout {
        self.layout
    }

    /// Row stride in bytes.
    #[must_use]
    pub const fn stride(&self) -> usize {
        self.geometry.stride
    }

    /// Query the available LSB capacity, matching `lsb::capacity`.
    #[must_use = "capacity reports should be inspected"]
    pub fn capacity(
        &self,
        payload_len: usize,
        config: &LsbConfig,
    ) -> Result<crate::CapacityReport, StegoError> {
        let available = checked_lsb_available_slots(self.geometry.width, self.geometry.height)?;
        let payload_bits = payload_len.saturating_mul(8);
        let required = lsb_required_capacity_v2(payload_bits, config.redundancy());
        Ok(crate::CapacityReport {
            required,
            available,
        })
    }

    /// Extract raw bytes, matching `lsb::extract`.
    pub fn extract(&self, payload_len: usize, config: &LsbConfig) -> Result<Vec<u8>, StegoError> {
        let bits = payload_len
            .checked_mul(8)
            .ok_or_else(|| StegoError::ResourceLimitExceeded("payload length overflow".into()))?;
        checked_lsb_available_slots(self.geometry.width, self.geometry.height)?;
        extract_v2_carrier(self, bits, config.seed(), config.redundancy())
            .ok_or_else(|| StegoError::MalformedInput("extraction returned no data".into()))
    }

    /// Extract a framed payload, matching `lsb::extract_framed`.
    pub fn extract_framed(&self, config: &LsbConfig) -> Result<Vec<u8>, StegoError> {
        let prefix_capacity = self.capacity(crate::frame::FRAME_HEADER_SIZE, config)?;
        if !prefix_capacity.is_sufficient() {
            return Err(StegoError::InsufficientCapacity {
                required: prefix_capacity.required,
                available: prefix_capacity.available,
            });
        }
        let prefix = self.extract(crate::frame::FRAME_HEADER_SIZE, config)?;
        let (_, total_len) = crate::frame::decode_prefix(&prefix)?;
        let frame_capacity = self.capacity(total_len, config)?;
        if !frame_capacity.is_sufficient() {
            return Err(StegoError::InsufficientCapacity {
                required: frame_capacity.required,
                available: frame_capacity.available,
            });
        }
        let framed = self.extract(total_len, config)?;
        let (_, payload) = crate::frame::decode(&framed)?;
        Ok(payload)
    }

    /// Extract tiled raw bytes, matching `lsb::extract_tiled`.
    pub fn extract_tiled(
        &self,
        payload_len: usize,
        config: &TileConfig,
        max_origins: u32,
    ) -> Result<Vec<u8>, StegoError> {
        extract_tiled_carrier(
            self,
            payload_len,
            config.seed(),
            config.tile_size(),
            max_origins,
        )
    }

    /// Extract a framed tiled payload, matching `lsb::extract_tiled_framed`.
    pub fn extract_tiled_framed(
        &self,
        config: &TileConfig,
        max_origins: u32,
    ) -> Result<Vec<u8>, StegoError> {
        extract_tiled_framed_carrier(self, config.seed(), config.tile_size(), max_origins)
    }
}

impl std::fmt::Debug for PixelView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PixelView")
            .field("width", &self.geometry.width)
            .field("height", &self.geometry.height)
            .field("layout", &self.layout)
            .field("stride", &self.geometry.stride)
            .finish()
    }
}

/// Mutable borrowed packed/strided RGB/RGBA buffer for in-place LSB embedding.
pub struct PixelViewMut<'a> {
    bytes: &'a mut [u8],
    layout: PixelLayout,
    geometry: BufferGeometry,
}

impl<'a> PixelViewMut<'a> {
    /// Borrow a packed (`stride == width * bytes_per_pixel`) or strided
    /// buffer mutably. Alpha bytes and row padding are never mutated.
    pub fn new(
        bytes: &'a mut [u8],
        width: u32,
        height: u32,
        layout: PixelLayout,
        stride: usize,
    ) -> Result<Self, StegoError> {
        let geometry = validate_geometry(bytes.len(), width, height, layout, stride)?;
        Ok(Self {
            bytes,
            layout,
            geometry,
        })
    }

    /// Buffer width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.geometry.width
    }

    /// Buffer height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.geometry.height
    }

    /// Pixel layout of the buffer.
    #[must_use]
    pub const fn layout(&self) -> PixelLayout {
        self.layout
    }

    /// Row stride in bytes.
    #[must_use]
    pub const fn stride(&self) -> usize {
        self.geometry.stride
    }

    /// Reborrow as an immutable view for extraction.
    pub fn as_view(&self) -> PixelView<'_> {
        PixelView {
            bytes: self.bytes,
            layout: self.layout,
            geometry: self.geometry,
        }
    }

    /// Query the available LSB capacity, matching `lsb::capacity`.
    #[must_use = "capacity reports should be inspected"]
    pub fn capacity(
        &self,
        payload_len: usize,
        config: &LsbConfig,
    ) -> Result<crate::CapacityReport, StegoError> {
        self.as_view().capacity(payload_len, config)
    }

    /// Embed raw bytes in place, matching `lsb::embed_in_place`.
    ///
    /// Capacity is checked before the first mutation, so an insufficient
    /// carrier is left unchanged.
    pub fn embed(
        &mut self,
        payload: &[u8],
        config: &LsbConfig,
    ) -> Result<crate::InPlaceEmbedReport, StegoError> {
        if self.geometry.width == 0 && self.geometry.height == 0 {
            return Err(StegoError::EmptyCarrier);
        }
        checked_lsb_available_slots(self.geometry.width, self.geometry.height)?;
        Ok(embed_v2_in_place_carrier(
            self,
            payload,
            config.seed(),
            config.redundancy(),
        ))
    }

    /// Embed a framed payload in place, matching `lsb::embed_framed`.
    pub fn embed_framed(
        &mut self,
        payload: &[u8],
        config: &LsbConfig,
    ) -> Result<crate::InPlaceEmbedReport, StegoError> {
        let framed = crate::frame::encode(payload)?;
        self.embed(&framed, config)
    }

    /// Embed raw bytes once per tile in place, matching
    /// `lsb::embed_tiled_in_place`.
    pub fn embed_tiled(
        &mut self,
        payload: &[u8],
        config: &TileConfig,
    ) -> Result<crate::InPlaceEmbedReport, StegoError> {
        if self.geometry.width == 0 && self.geometry.height == 0 {
            return Err(StegoError::EmptyCarrier);
        }
        Ok(embed_tiled_carrier(
            self,
            payload,
            config.seed(),
            config.tile_size(),
        ))
    }

    /// Embed a framed payload once per tile in place, matching
    /// `lsb::embed_tiled_framed`.
    pub fn embed_tiled_framed(
        &mut self,
        payload: &[u8],
        config: &TileConfig,
    ) -> Result<crate::InPlaceEmbedReport, StegoError> {
        let framed = crate::frame::encode(payload)?;
        self.embed_tiled(&framed, config)
    }

    /// Extract raw bytes, matching `lsb::extract`.
    pub fn extract(&self, payload_len: usize, config: &LsbConfig) -> Result<Vec<u8>, StegoError> {
        self.as_view().extract(payload_len, config)
    }

    /// Extract a framed payload, matching `lsb::extract_framed`.
    pub fn extract_framed(&self, config: &LsbConfig) -> Result<Vec<u8>, StegoError> {
        self.as_view().extract_framed(config)
    }
}

impl std::fmt::Debug for PixelViewMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PixelViewMut")
            .field("width", &self.geometry.width)
            .field("height", &self.geometry.height)
            .field("layout", &self.layout)
            .field("stride", &self.geometry.stride)
            .finish()
    }
}

impl PixelCarrier for PixelView<'_> {
    fn carrier_width(&self) -> u32 {
        self.geometry.width
    }

    fn carrier_height(&self) -> u32 {
        self.geometry.height
    }

    fn read_channel(&self, x: u32, y: u32, channel: usize) -> Option<u8> {
        let offset = self
            .geometry
            .channel_offset(self.bytes.len(), x, y, channel)?;
        self.bytes.get(offset).copied()
    }
}

impl PixelCarrier for PixelViewMut<'_> {
    fn carrier_width(&self) -> u32 {
        self.geometry.width
    }

    fn carrier_height(&self) -> u32 {
        self.geometry.height
    }

    fn read_channel(&self, x: u32, y: u32, channel: usize) -> Option<u8> {
        let offset = self
            .geometry
            .channel_offset(self.bytes.len(), x, y, channel)?;
        self.bytes.get(offset).copied()
    }
}

impl PixelCarrierMut for PixelViewMut<'_> {
    fn write_channel_bit(&mut self, x: u32, y: u32, channel: usize, bit: u8) -> Option<()> {
        let offset = self
            .geometry
            .channel_offset(self.bytes.len(), x, y, channel)?;
        let old_val = *self.bytes.get(offset)?;
        if (old_val & 1) != bit {
            *self.bytes.get_mut(offset)? = apply_lsb_bit(old_val, x, y);
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsb;

    fn pattern_rgba(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::with_capacity((width as usize) * (height as usize) * 4);
        for y in 0..height {
            for x in 0..width {
                bytes.extend_from_slice(&[
                    ((x * 7 + y * 13) % 256) as u8,
                    ((x * 11 + y * 3) % 256) as u8,
                    ((x * 5 + y * 17) % 256) as u8,
                    0xA0,
                ]);
            }
        }
        bytes
    }

    fn pattern_rgb(width: u32, height: u32, stride: usize) -> Vec<u8> {
        let mut bytes = vec![0xCCu8; (height as usize) * stride];
        for y in 0..height {
            for x in 0..width {
                let base = (y as usize) * stride + (x as usize) * 3;
                bytes[base] = ((x * 7 + y * 13) % 256) as u8;
                bytes[base + 1] = ((x * 11 + y * 3) % 256) as u8;
                bytes[base + 2] = ((x * 5 + y * 17) % 256) as u8;
            }
        }
        bytes
    }

    #[test]
    fn packed_rgba_view_matches_rgba_image_bytes() {
        let (width, height) = (48u32, 32u32);
        let backing = pattern_rgba(width, height);
        let mut image = image::RgbaImage::from_raw(width, height, backing.clone()).unwrap();
        let mut view_bytes = backing.clone();
        let config = LsbConfig::new(99).with_redundancy_value(crate::Redundancy::new(3).unwrap());
        let payload = b"view equivalence payload";

        let image_report = lsb::embed_in_place(&mut image, payload, &config).unwrap();
        let mut view = PixelViewMut::new(
            &mut view_bytes,
            width,
            height,
            PixelLayout::Rgba8,
            (width as usize) * 4,
        )
        .unwrap();
        let view_report = view.embed(payload, &config).unwrap();

        assert!(image_report.embedded);
        assert!(view_report.embedded);
        assert_eq!(
            image_report.required_capacity,
            view_report.required_capacity
        );
        assert_eq!(image.as_raw(), &view_bytes);
        let view = PixelView::new(
            &view_bytes,
            width,
            height,
            PixelLayout::Rgba8,
            (width as usize) * 4,
        )
        .unwrap();
        assert_eq!(view.extract(payload.len(), &config).unwrap(), payload);
        assert!(view_bytes.chunks_exact(4).all(|pixel| pixel[3] == 0xA0));
    }

    #[test]
    fn rgb_view_uses_same_logical_slots_as_rgba() {
        let (width, height) = (40u32, 24u32);
        let config = LsbConfig::new(7);
        let payload = b"logical slot parity";
        let mut rgb = pattern_rgb(width, height, (width as usize) * 3);
        let mut view = PixelViewMut::new(
            &mut rgb,
            width,
            height,
            PixelLayout::Rgb8,
            (width as usize) * 3,
        )
        .unwrap();
        assert!(view.embed(payload, &config).unwrap().embedded);
        assert_eq!(
            view.as_view().extract(payload.len(), &config).unwrap(),
            payload
        );

        let mut rgba = pattern_rgba(width, height);
        let mut rgba_view = PixelViewMut::new(
            &mut rgba,
            width,
            height,
            PixelLayout::Rgba8,
            (width as usize) * 4,
        )
        .unwrap();
        assert!(rgba_view.embed(payload, &config).unwrap().embedded);
        for y in 0..height {
            for x in 0..width {
                let rgb_base = (y as usize) * (width as usize) * 3 + (x as usize) * 3;
                let rgba_base = (y as usize) * (width as usize) * 4 + (x as usize) * 4;
                assert_eq!(
                    &rgb[rgb_base..rgb_base + 3],
                    &rgba[rgba_base..rgba_base + 3],
                    "RGB channels must match at pixel ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn strided_padding_and_alpha_are_never_mutated() {
        let (width, height) = (32u32, 32u32);
        let stride = (width as usize) * 3 + 37;
        let mut bytes = pattern_rgb(width, height, stride);
        let padding_before: Vec<u8> = bytes
            .chunks(stride)
            .flat_map(|row| row[(width as usize) * 3..].to_vec())
            .collect();
        let config = LsbConfig::new(1234);
        let recovered = {
            let mut view =
                PixelViewMut::new(&mut bytes, width, height, PixelLayout::Rgb8, stride).unwrap();
            assert!(
                view.embed(b"padding must survive", &config)
                    .unwrap()
                    .embedded
            );
            view.as_view().extract(20, &config).unwrap()
        };
        let padding_after: Vec<u8> = bytes
            .chunks(stride)
            .flat_map(|row| row[(width as usize) * 3..].to_vec())
            .collect();
        assert_eq!(padding_before, padding_after);
        assert!(bytes
            .chunks(stride)
            .all(|row| row[(width as usize) * 3..].iter().all(|&b| b == 0xCC)));
        assert_eq!(recovered, b"padding must survive");

        let (aw, ah) = (24u32, 24u32);
        let astride = (aw as usize) * 4 + 8;
        let mut rgba = vec![0xCCu8; (ah as usize) * astride];
        for y in 0..ah {
            for x in 0..aw {
                let base = (y as usize) * astride + (x as usize) * 4;
                rgba[base] = ((x * 7 + y * 13) % 256) as u8;
                rgba[base + 1] = ((x * 11 + y * 3) % 256) as u8;
                rgba[base + 2] = ((x * 5 + y * 17) % 256) as u8;
                rgba[base + 3] = 0xA0;
            }
        }
        let recovered = {
            let mut rgba_view =
                PixelViewMut::new(&mut rgba, aw, ah, PixelLayout::Rgba8, astride).unwrap();
            assert!(
                rgba_view
                    .embed(b"alpha must survive", &config)
                    .unwrap()
                    .embedded
            );
            rgba_view.as_view().extract(18, &config).unwrap()
        };
        assert!(rgba.chunks(astride).all(|row| {
            row.chunks_exact(4)
                .take(aw as usize)
                .all(|pixel| pixel[3] == 0xA0)
                && row[(aw as usize) * 4..].iter().all(|&b| b == 0xCC)
        }));
        assert_eq!(recovered, b"alpha must survive");
    }

    #[test]
    fn invalid_geometry_is_rejected() {
        let bytes = vec![0u8; 300];
        assert!(PixelView::new(&bytes, 10, 10, PixelLayout::Rgb8, 30).is_ok());
        assert!(PixelView::new(&bytes, 10, 10, PixelLayout::Rgb8, 29).is_err());
        assert!(PixelView::new(&bytes, 10, 10, PixelLayout::Rgb8, usize::MAX).is_err());
        assert!(PixelView::new(&bytes, 10, 10, PixelLayout::Rgba8, 40).is_err());
        assert!(PixelView::new(&bytes, u32::MAX, u32::MAX, PixelLayout::Rgb8, usize::MAX).is_err());
        assert!(PixelView::new(&bytes, u32::MAX, 2, PixelLayout::Rgb8, usize::MAX).is_err());
        assert!(PixelView::new(&[], 0, 0, PixelLayout::Rgb8, 0).is_ok());
        let mut small = vec![0u8; 100];
        assert!(PixelViewMut::new(&mut small, 4, 4, PixelLayout::Rgb8, 12).is_ok());
        assert!(PixelViewMut::new(&mut small, 4, 4, PixelLayout::Rgb8, 11).is_err());
    }

    #[test]
    fn empty_view_reports_empty_carrier() {
        let mut bytes = Vec::new();
        let config = LsbConfig::new(42);
        let view = PixelView::new(&bytes, 0, 0, PixelLayout::Rgb8, 0).unwrap();
        let capacity = view.capacity(4, &config).unwrap();
        assert_eq!(capacity.available, 0);
        assert!(!capacity.is_sufficient());
        let mut empty = PixelViewMut::new(&mut bytes, 0, 0, PixelLayout::Rgb8, 0).unwrap();
        assert!(matches!(
            empty.embed(b"x", &config),
            Err(StegoError::EmptyCarrier)
        ));
    }

    #[test]
    fn capacity_shortage_is_atomic_before_mutation() {
        let (width, height) = (16u32, 16u32);
        let mut bytes = pattern_rgb(width, height, (width as usize) * 3);
        let before = bytes.clone();
        let config = LsbConfig::new(42);
        let available = PixelView::new(
            &bytes,
            width,
            height,
            PixelLayout::Rgb8,
            (width as usize) * 3,
        )
        .unwrap()
        .capacity(1, &config)
        .unwrap()
        .available;
        let oversized = vec![0xA5; available + 64];
        let mut view = PixelViewMut::new(
            &mut bytes,
            width,
            height,
            PixelLayout::Rgb8,
            (width as usize) * 3,
        )
        .unwrap();
        let report = view.embed(&oversized, &config).unwrap();
        assert!(!report.embedded);
        assert_eq!(bytes, before);
    }

    #[test]
    fn framed_and_tiled_view_round_trips() {
        let (width, height) = (128u32, 128u32);
        let config = LsbConfig::new(4242);
        let tile = TileConfig::try_new(4242, 64).unwrap();

        let mut framed_bytes = pattern_rgba(width, height);
        let mut framed_view = PixelViewMut::new(
            &mut framed_bytes,
            width,
            height,
            PixelLayout::Rgba8,
            (width as usize) * 4,
        )
        .unwrap();
        assert!(
            framed_view
                .embed_framed(b"framed view", &config)
                .unwrap()
                .embedded
        );
        assert_eq!(
            framed_view.as_view().extract_framed(&config).unwrap(),
            b"framed view"
        );

        let mut tiled_bytes = pattern_rgb(width, height, (width as usize) * 3 + 16);
        let stride = (width as usize) * 3 + 16;
        let mut tiled_view =
            PixelViewMut::new(&mut tiled_bytes, width, height, PixelLayout::Rgb8, stride).unwrap();
        let payload = vec![0x5Au8; 24];
        assert!(tiled_view.embed_tiled(&payload, &tile).unwrap().embedded);
        assert_eq!(
            tiled_view
                .as_view()
                .extract_tiled(payload.len(), &tile, 64)
                .unwrap(),
            payload
        );

        let mut tiled_framed_bytes = pattern_rgba(width, height);
        let mut tiled_framed_view = PixelViewMut::new(
            &mut tiled_framed_bytes,
            width,
            height,
            PixelLayout::Rgba8,
            (width as usize) * 4,
        )
        .unwrap();
        assert!(
            tiled_framed_view
                .embed_tiled_framed(b"tiled framed view", &tile)
                .unwrap()
                .embedded
        );
        assert_eq!(
            tiled_framed_view
                .as_view()
                .extract_tiled_framed(&tile, 64)
                .unwrap(),
            b"tiled framed view"
        );
    }

    #[test]
    fn tiled_view_matches_rgba_image_path() {
        let (width, height) = (128u32, 128u32);
        let backing = pattern_rgba(width, height);
        let image = image::RgbaImage::from_raw(width, height, backing.clone()).unwrap();
        let tile = TileConfig::try_new(99, 64).unwrap();
        let payload = vec![0x3Cu8; 20];

        let image_report = lsb::embed_tiled(&image, &payload, &tile).unwrap();
        let mut view_bytes = backing.clone();
        let (view_report, view_recovered) = {
            let mut view = PixelViewMut::new(
                &mut view_bytes,
                width,
                height,
                PixelLayout::Rgba8,
                (width as usize) * 4,
            )
            .unwrap();
            let view_report = view.embed_tiled(&payload, &tile).unwrap();
            let view_recovered = view
                .as_view()
                .extract_tiled(payload.len(), &tile, 64)
                .unwrap();
            (view_report, view_recovered)
        };
        assert!(image_report.embedded);
        assert!(view_report.embedded);
        assert_eq!(image_report.output.as_raw(), &view_bytes);
        assert_eq!(
            view_recovered,
            lsb::extract_tiled(&image_report.output, payload.len(), &tile, 64).unwrap()
        );
    }

    #[test]
    fn view_redundancy_values_use_validated_type() {
        let (width, height) = (48u32, 48u32);
        for redundancy in 1..=10u8 {
            let config = LsbConfig::from_redundancy(5, crate::Redundancy::new(redundancy).unwrap());
            let mut bytes = pattern_rgb(width, height, (width as usize) * 3);
            let mut view = PixelViewMut::new(
                &mut bytes,
                width,
                height,
                PixelLayout::Rgb8,
                (width as usize) * 3,
            )
            .unwrap();
            let report = view.embed(b"redundant", &config).unwrap();
            assert!(report.embedded);
            assert_eq!(report.actual_redundancy, redundancy as usize);
        }
    }
}
