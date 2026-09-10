//! Opaque prepared-JPEG carrier for repeated generic operations.
//!
//! [`PreparedJpeg`] retains one coefficient decode for repeated capacity,
//! extraction, and strict-embedding operations against the same encoded
//! JPEG. Parser headers, coefficient maps, Huffman state, and F5
//! implementation objects stay private; only operation-level methods are
//! exposed.
//!
//! One-shot functions in [`crate::jpeg`] delegate to the same
//! decoded-state helpers, so prepared and one-shot results agree exactly.
//!
//! # Capacity units
//!
//! Capacity is reported in **eligible AC coefficients with `|coef| >= 2`
//! after canonicalization**, matching [`crate::jpeg::capacity`].
//!
//! # Examples
//!
//! ```rust,no_run
//! use stegoeggo_stego::jpeg::JpegConfig;
//! use stegoeggo_stego::prepared::PreparedJpeg;
//!
//! let jpeg_bytes = std::fs::read("photo.jpg").unwrap();
//! let prepared = PreparedJpeg::new(&jpeg_bytes).unwrap();
//! let config = JpegConfig::new(42);
//! let report = prepared.capacity(100, &config).unwrap();
//! if report.is_sufficient() {
//!     let payload = prepared.extract_framed(&config).unwrap();
//!     println!("{} bytes", payload.len());
//! }
//! ```

use crate::error::StegoError;
use crate::jpeg::{
    capacity_from_decoded, decode_supported_carrier, embed_strict_from_decoded,
    extract_framed_from_decoded, extract_from_decoded, extract_tiled_framed_from_decoded,
    extract_tiled_from_decoded, DecodedJpegCarrier, JpegConfig, JpegSupport,
};
use crate::types::TileConfig;

/// A borrowed JPEG carrier decoded once for repeated operations.
///
/// Construct with [`PreparedJpeg::new`]. Read-only operations reuse the
/// single coefficient decode from construction; strict embedding clones
/// only the coefficient structures needed for the output, so a failed
/// embed never poisons the prepared state. The constructor borrows the
/// caller's encoded bytes and never copies the full JPEG.
pub struct PreparedJpeg<'a> {
    source: &'a [u8],
    support: JpegSupport,
    decoded: Option<DecodedJpegCarrier>,
}

impl<'a> PreparedJpeg<'a> {
    /// Prepare a JPEG for repeated carrier operations.
    ///
    /// Parsing and support classification match the one-shot functions:
    /// malformed inputs return [`StegoError::MalformedInput`]. Supported
    /// inputs decode coefficients once; well-formed but unsupported inputs
    /// (progressive, multi-scan, restart intervals, and the other
    /// [`JpegSupport::Unsupported`] reasons) still construct successfully
    /// so callers can probe support without parsing twice, and
    /// coefficient operations on them return
    /// [`StegoError::UnsupportedJpeg`].
    pub fn new(encoded: &'a [u8]) -> Result<Self, StegoError> {
        let (support, decoded) = match decode_supported_carrier(encoded) {
            Ok(decoded) => (JpegSupport::Supported, Some(decoded)),
            Err(StegoError::UnsupportedJpeg(reason)) => (JpegSupport::Unsupported(reason), None),
            Err(other) => return Err(other),
        };
        Ok(Self {
            source: encoded,
            support,
            decoded,
        })
    }

    /// The DCT support classification for the prepared JPEG.
    #[must_use]
    pub fn support(&self) -> JpegSupport {
        self.support.clone()
    }

    fn decoded_or_unsupported(&self) -> Result<&DecodedJpegCarrier, StegoError> {
        self.decoded.as_ref().ok_or_else(|| match &self.support {
            JpegSupport::Unsupported(reason) => StegoError::UnsupportedJpeg(*reason),
            JpegSupport::Supported => {
                StegoError::MalformedInput("prepared JPEG has no decoded coefficients".to_string())
            }
        })
    }

    /// Query the available DCT capacity, reusing the prepared decode.
    ///
    /// Matches [`crate::jpeg::capacity`] exactly.
    pub fn capacity(
        &self,
        payload_len: usize,
        config: &JpegConfig,
    ) -> Result<crate::CapacityReport, StegoError> {
        capacity_from_decoded(
            self.decoded_or_unsupported()?,
            payload_len,
            config.redundancy(),
        )
    }

    /// Extract raw bytes, reusing the prepared decode.
    ///
    /// Matches [`crate::jpeg::extract`] exactly, including the
    /// `actual_redundancy` contract.
    pub fn extract(
        &self,
        payload_len: usize,
        config: &JpegConfig,
        actual_redundancy: usize,
    ) -> Result<Vec<u8>, StegoError> {
        extract_from_decoded(
            self.decoded_or_unsupported()?,
            payload_len,
            config.seed(),
            actual_redundancy,
        )
    }

    /// Extract a framed payload, reusing the prepared decode.
    ///
    /// Matches [`crate::jpeg::extract_framed`] exactly.
    pub fn extract_framed(&self, config: &JpegConfig) -> Result<Vec<u8>, StegoError> {
        extract_framed_from_decoded(self.decoded_or_unsupported()?, config)
    }

    /// Extract tiled raw bytes, reusing the prepared decode.
    ///
    /// Matches [`crate::jpeg::extract_tiled`] exactly, including the
    /// bounded `max_origins` search.
    pub fn extract_tiled(
        &self,
        payload_len: usize,
        config: &TileConfig,
        max_origins: u32,
    ) -> Result<Vec<u8>, StegoError> {
        extract_tiled_from_decoded(
            self.decoded_or_unsupported()?,
            payload_len,
            config,
            max_origins,
        )
    }

    /// Extract a framed tiled payload, reusing the prepared decode.
    ///
    /// Matches [`crate::jpeg::extract_tiled_framed`] exactly.
    pub fn extract_tiled_framed(
        &self,
        config: &TileConfig,
        max_origins: u32,
    ) -> Result<Vec<u8>, StegoError> {
        extract_tiled_framed_from_decoded(self.decoded_or_unsupported()?, config, max_origins)
    }

    /// Strict-embed at exactly the requested redundancy.
    ///
    /// Matches [`crate::jpeg::embed_strict`] exactly, including the
    /// container-preserving encode against the borrowed source bytes.
    /// Only the structures needed for the output are cloned, so a failed
    /// attempt leaves the prepared state unchanged and reusable.
    pub fn embed_strict(
        &self,
        payload: &[u8],
        config: &JpegConfig,
    ) -> Result<crate::EmbedReport, StegoError> {
        embed_strict_from_decoded(self.decoded_or_unsupported()?, self.source, payload, config)
    }

    /// Strict-embed a framed payload at exactly the requested redundancy.
    ///
    /// Matches [`crate::jpeg::embed_framed_strict`] exactly.
    pub fn embed_framed_strict(
        &self,
        payload: &[u8],
        config: &JpegConfig,
    ) -> Result<crate::EmbedReport, StegoError> {
        let framed = crate::frame::encode(payload)?;
        self.embed_strict(&framed, config)
    }

    /// Read the quantization-table seed hint from the source JPEG.
    ///
    /// Matches [`crate::jpeg::extract_seed_hint`]; available even for
    /// unsupported structures because it is header-only.
    pub fn seed_hint(&self) -> Result<Option<u64>, StegoError> {
        super::jpeg::extract_seed_hint(self.source)
    }
}

impl std::fmt::Debug for PreparedJpeg<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedJpeg")
            .field("support", &self.support)
            .field("has_coefficients", &self.decoded.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jpeg::{
        self, capacity, embed_framed, embed_strict, embed_tiled, embed_tiled_framed, extract,
        extract_framed, extract_tiled, extract_tiled_framed,
    };

    fn make_test_jpeg(w: u32, h: u32) -> Vec<u8> {
        let mut img = image::RgbImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                img.put_pixel(
                    x,
                    y,
                    image::Rgb([
                        ((x * 7 + y * 13) % 256) as u8,
                        ((x * 11 + y * 3) % 256) as u8,
                        ((x * 5 + y * 17) % 256) as u8,
                    ]),
                );
            }
        }
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
        image::DynamicImage::ImageRgb8(img)
            .write_with_encoder(encoder)
            .unwrap();
        buf
    }

    #[test]
    fn prepared_rejects_malformed_input_like_one_shot() {
        assert!(matches!(
            PreparedJpeg::new(b"not a jpeg"),
            Err(StegoError::MalformedInput(_))
        ));
        assert!(matches!(
            PreparedJpeg::new(&[]),
            Err(StegoError::MalformedInput(_))
        ));
    }

    #[test]
    fn prepared_constructs_for_unsupported_jpeg() {
        use jpeg_encoder::Encoder as JpegEnc;
        let rgb = image::DynamicImage::new_rgb8(64, 64).to_rgb8();
        let mut progressive_buf = Vec::new();
        {
            let mut enc = JpegEnc::new(&mut progressive_buf, 90);
            enc.set_progressive(true);
            enc.encode(rgb.as_raw(), 64, 64, jpeg_encoder::ColorType::Rgb)
                .unwrap();
        }
        let one_shot = jpeg::probe_support(&progressive_buf).unwrap();
        let prepared = PreparedJpeg::new(&progressive_buf).unwrap();
        assert_eq!(prepared.support(), one_shot);
        assert!(matches!(prepared.support(), JpegSupport::Unsupported(_)));
        let config = JpegConfig::new(42);
        assert!(matches!(
            prepared.capacity(4, &config),
            Err(StegoError::UnsupportedJpeg(_))
        ));
        assert!(matches!(
            prepared.extract(4, &config, 1),
            Err(StegoError::UnsupportedJpeg(_))
        ));
        assert!(matches!(
            prepared.embed_strict(b"payload", &config),
            Err(StegoError::UnsupportedJpeg(_))
        ));
    }

    #[test]
    fn prepared_repeated_reads_match_one_shot_and_decode_once() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let config = JpegConfig::new(42);
        let payload = b"prepared parity payload";
        let report = embed_strict(&jpeg_bytes, payload, &config).unwrap();

        crate::jpeg::reset_decode_count();
        let prepared = PreparedJpeg::new(&report.output).unwrap();
        assert_eq!(crate::jpeg::decode_count(), 1);
        assert_eq!(prepared.support(), JpegSupport::Supported);

        let one_shot_capacity = capacity(&report.output, payload.len(), &config).unwrap();
        assert_eq!(
            prepared.capacity(payload.len(), &config).unwrap(),
            one_shot_capacity
        );
        crate::jpeg::reset_decode_count();
        for redundancy in [1usize, 2, 3] {
            let _ = extract(&report.output, payload.len(), &config, redundancy);
        }
        for redundancy in [1usize, 2, 3] {
            let one_shot = extract(&report.output, payload.len(), &config, redundancy);
            let prepared_extract = prepared.extract(payload.len(), &config, redundancy);
            assert_eq!(prepared_extract.is_ok(), one_shot.is_ok());
            if let Ok(expected) = one_shot {
                assert_eq!(prepared_extract.unwrap(), expected);
            }
        }
        assert_eq!(crate::jpeg::decode_count(), 6);

        let framed = embed_framed(&jpeg_bytes, b"framed prepared", &config).unwrap();
        let prepared_framed = PreparedJpeg::new(&framed.output).unwrap();
        assert_eq!(
            prepared_framed.extract_framed(&config).unwrap(),
            extract_framed(&framed.output, &config).unwrap()
        );

        let tile_config = TileConfig::try_new(42, 64).unwrap();
        let tiled = embed_tiled(&jpeg_bytes, payload, &tile_config).unwrap();
        let prepared_tiled = PreparedJpeg::new(&tiled.output).unwrap();
        assert_eq!(
            prepared_tiled
                .extract_tiled(payload.len(), &tile_config, 64)
                .unwrap(),
            extract_tiled(&tiled.output, payload.len(), &tile_config, 64).unwrap()
        );
        let tiled_framed = embed_tiled_framed(&jpeg_bytes, b"tiled framed", &tile_config).unwrap();
        let prepared_tiled_framed = PreparedJpeg::new(&tiled_framed.output).unwrap();
        assert_eq!(
            prepared_tiled_framed
                .extract_tiled_framed(&tile_config, 64)
                .unwrap(),
            extract_tiled_framed(&tiled_framed.output, &tile_config, 64).unwrap()
        );
    }

    #[test]
    fn prepared_reads_use_exactly_one_decode() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let config = JpegConfig::new(42);
        let framed = embed_framed(&jpeg_bytes, b"one decode", &config).unwrap();
        crate::jpeg::reset_decode_count();
        let prepared = PreparedJpeg::new(&framed.output).unwrap();
        let _ = prepared.capacity(11, &config);
        let _ = prepared.extract_framed(&config);
        let tile_config = TileConfig::try_new(42, 64).unwrap();
        let _ = prepared.extract_tiled_framed(&tile_config, 8);
        let _ = prepared.seed_hint();
        assert_eq!(crate::jpeg::decode_count(), 1);
    }

    #[test]
    fn prepared_strict_embed_matches_one_shot_bytes() {
        let jpeg_bytes = make_test_jpeg(256, 256);
        let config = JpegConfig::new(42);
        let prepared = PreparedJpeg::new(&jpeg_bytes).unwrap();
        let payload = b"prepared strict embed";
        let prepared_report = prepared.embed_strict(payload, &config).unwrap();
        let one_shot_report = embed_strict(&jpeg_bytes, payload, &config).unwrap();
        assert_eq!(prepared_report.output, one_shot_report.output);
        assert_eq!(
            prepared_report.actual_redundancy,
            one_shot_report.actual_redundancy
        );
        let framed_prepared = prepared.embed_framed_strict(payload, &config).unwrap();
        let framed_one_shot =
            crate::jpeg::embed_framed_strict(&jpeg_bytes, payload, &config).unwrap();
        assert_eq!(framed_prepared.output, framed_one_shot.output);
    }

    #[test]
    fn prepared_failed_embed_leaves_state_reusable() {
        let jpeg_bytes = make_test_jpeg(64, 64);
        let config = JpegConfig::new(42);
        let prepared = PreparedJpeg::new(&jpeg_bytes).unwrap();
        let available = prepared.capacity(1, &config).unwrap().available;
        let oversized = vec![0xA5; available + 64];
        assert!(matches!(
            prepared.embed_strict(&oversized, &config),
            Err(StegoError::InsufficientCapacity { .. })
        ));
        let payload = b"still works";
        let report = prepared.embed_strict(payload, &config).unwrap();
        assert!(report.embedded);
        let reprepared = PreparedJpeg::new(&report.output).unwrap();
        assert_eq!(
            reprepared
                .extract(payload.len(), &config, config.redundancy())
                .unwrap(),
            payload
        );
        assert_eq!(
            prepared
                .extract_tiled_framed(&TileConfig::try_new(42, 64).unwrap(), 1)
                .is_ok(),
            extract_tiled_framed(&jpeg_bytes, &TileConfig::try_new(42, 64).unwrap(), 1).is_ok()
        );
    }

    #[test]
    fn prepared_borrows_source_without_copy() {
        let jpeg_bytes = make_test_jpeg(64, 64);
        let prepared = PreparedJpeg::new(&jpeg_bytes).unwrap();
        assert!(std::ptr::eq(prepared.source, jpeg_bytes.as_slice()));
        let debug = format!("{:?}", prepared);
        assert!(debug.contains("PreparedJpeg"));
        assert!(debug.contains("Supported"));
    }

    #[test]
    fn prepared_seed_hint_matches_one_shot() {
        let jpeg_bytes = make_test_jpeg(128, 128);
        let hinted = jpeg::embed_seed_hint(&jpeg_bytes, 0xCAFE_F00D).unwrap();
        let prepared = PreparedJpeg::new(&hinted).unwrap();
        assert_eq!(prepared.seed_hint().unwrap(), Some(0xCAFE_F00D));
        assert_eq!(
            prepared.seed_hint().unwrap(),
            jpeg::extract_seed_hint(&hinted).unwrap()
        );
    }

    #[test]
    fn prepared_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PreparedJpeg<'static>>();
    }
}
