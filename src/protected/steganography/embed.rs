//! Application-level carrier selection and embedding dispatch.

use super::*;

impl SteganographyProtector {
    /// Apply DCT-based steganography to JPEG bytes.
    ///
    /// For baseline JPEGs, performs full F5 embedding in DCT coefficients and stores
    /// the seed in quantization tables. For progressive JPEGs (which cannot be decoded
    /// to DCT coefficients), falls back to seed-in-quantization-tables only.
    ///
    /// This is the JPEG fast path — it avoids pixel decode/encode cycles and preserves
    /// quality. Used internally by the pipeline for JPEG-in/JPEG-out flows.
    pub(crate) fn apply_dct_stego_bytes(
        &self,
        jpeg_bytes: &[u8],
        ctx: &ProtectionContext,
    ) -> Result<crate::stego::EmbedOutcome<Vec<u8>>> {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return Err(Error::Steganography("Not a valid JPEG".to_string()));
        }

        if let Some(tile_size) = ctx.tile_size().filter(|&s| s > 0) {
            return self.apply_dct_stego_bytes_tiled(jpeg_bytes, ctx, tile_size);
        }

        let payload = self.generate_payload(
            &crate::types::PayloadEmissionContext::from_plan_for_context(
                ctx,
                crate::stego::EmbedPath::DctF5,
            ),
            ctx,
        );
        match carrier_support::jpeg_embed(
            jpeg_bytes,
            &payload,
            ctx.seed(),
            ctx.effective_redundancy(),
        ) {
            Ok(outcome) => Ok(outcome),
            Err(crate::stego::StegoError::UnsupportedJpeg(_)) => {
                Ok(crate::stego::EmbedOutcome::UnsupportedProgressive {
                    output: carrier_jpeg::embed_seed_hint(jpeg_bytes, ctx.seed())?,
                })
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) fn lsb_pixels_needed(ctx: &ProtectionContext) -> usize {
        let payload_bits = Self::payload_bits_for_context(ctx);
        carrier_support::legacy_lsb_required_slots(payload_bits)
    }

    /// Embed only the seed in JPEG quantization tables (no DCT coefficient modification).
    /// Used for Light level JPEG protection — the seed is recoverable when the
    /// quantization tables themselves are preserved.
    pub(crate) fn apply_qtable_seed_bytes(&self, jpeg_bytes: &[u8], seed: u64) -> Result<Vec<u8>> {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return Err(Error::Steganography("Not a valid JPEG".to_string()));
        }

        carrier_jpeg::embed_seed_hint(jpeg_bytes, seed).map_err(Into::into)
    }

    /// Embed the full payload per tile using F5-style DCT coefficient
    /// manipulation for crop resistance.
    ///
    /// Each `tile_size × tile_size` pixel region (mapped to DCT blocks)
    /// embeds the full payload using a tile-specific seed. Redundancy is
    /// fixed at 1 because the tile grid itself is the redundancy.
    ///
    /// Returns the re-encoded JPEG bytes, or an error if embedding fails.
    pub(crate) fn apply_dct_stego_bytes_tiled(
        &self,
        jpeg_bytes: &[u8],
        ctx: &ProtectionContext,
        tile_size: u32,
    ) -> Result<crate::stego::EmbedOutcome<Vec<u8>>> {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return Err(Error::Steganography("Not a valid JPEG".to_string()));
        }
        let payload = self.generate_payload(
            &crate::types::PayloadEmissionContext::from_plan_for_context(
                ctx,
                crate::stego::EmbedPath::DctF5Tiled,
            ),
            ctx,
        );
        match carrier_support::jpeg_embed_tiled(jpeg_bytes, &payload, ctx.seed(), tile_size) {
            Ok(outcome) => Ok(outcome),
            Err(crate::stego::StegoError::UnsupportedJpeg(_)) => {
                Ok(crate::stego::EmbedOutcome::UnsupportedProgressive {
                    output: carrier_jpeg::embed_seed_hint(jpeg_bytes, ctx.seed())?,
                })
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) fn effective_redundancy_for_plan(
        plan: &crate::types::ResolvedProtectionPlan,
    ) -> usize {
        plan.effective_redundancy()
    }

    /// Legacy capacity estimate for tEXt capacity checks.
    ///
    /// Computes `payload_bits * STEGO_SPREAD_FACTOR` without accounting for
    /// future V3 `key_id` / TLV extension bytes. Safe today because the V3
    /// writer never emits them (`assert!(emission.extensions.is_empty())`
    /// in `marker.rs:95`), but will under-count if extensions are ever
    /// emitted. When extension support lands, derive `payload_bits` from the
    /// actual emitted payload size instead of the fixed CRC/HMAC constants.
    #[allow(dead_code)]
    pub(crate) fn lsb_pixels_needed_from_plan(
        plan: &crate::types::ResolvedProtectionPlan,
    ) -> usize {
        let payload_bits = if plan.mac_key().is_some() {
            V3_HMAC_PAYLOAD_BITS
        } else {
            V3_CRC_PAYLOAD_BITS
        };
        carrier_support::legacy_lsb_required_slots(payload_bits)
    }

    pub(crate) fn apply_dct_stego_bytes_from_plan(
        &self,
        jpeg_bytes: &[u8],
        plan: &crate::types::ResolvedProtectionPlan,
        tile_size: Option<u32>,
    ) -> Result<crate::stego::EmbedOutcome<Vec<u8>>> {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return Err(Error::Steganography("Not a valid JPEG".to_string()));
        }
        let path = if tile_size.filter(|&size| size > 0).is_some() {
            crate::stego::EmbedPath::DctF5Tiled
        } else {
            crate::stego::EmbedPath::DctF5
        };
        let emission = PayloadEmissionContext::from_plan(plan, path);
        let payload = self.generate_payload_for_plan(&emission, plan);
        let outcome = match tile_size.filter(|&size| size > 0) {
            Some(size) => {
                carrier_support::jpeg_embed_tiled(jpeg_bytes, &payload, plan.seed(), size)
            }
            None => carrier_support::jpeg_embed(
                jpeg_bytes,
                &payload,
                plan.seed(),
                Self::effective_redundancy_for_plan(plan),
            ),
        };
        match outcome {
            Ok(outcome) => Ok(outcome),
            Err(crate::stego::StegoError::UnsupportedJpeg(_)) => {
                Ok(crate::stego::EmbedOutcome::UnsupportedProgressive {
                    output: carrier_jpeg::embed_seed_hint(jpeg_bytes, plan.seed())?,
                })
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) fn apply_lsb_to_image_with_summary_from_plan(
        &self,
        img: &DynamicImage,
        plan: &crate::types::ResolvedProtectionPlan,
        tile_size: Option<u32>,
    ) -> Result<(DynamicImage, Option<crate::stego::EmbedOutcomeSummary>)> {
        let is_tiled = tile_size.filter(|&s| s > 0).is_some();
        let embed_path = if is_tiled {
            crate::stego::EmbedPath::LsbTiled
        } else {
            crate::stego::EmbedPath::Lsb
        };

        let emission = PayloadEmissionContext::from_plan(plan, embed_path);
        let payload = self.generate_payload_for_plan(&emission, plan);
        let mut rgba = img.to_rgba8();
        let seed = plan.seed();
        let redundancy = Self::effective_redundancy_for_plan(plan);

        if let Some(ts) = tile_size.filter(|&s| s > 0) {
            let report = self.embed_lsb_tiled_in_place(&mut rgba, &payload, seed, ts);
            let summary = Self::lsb_tiled_in_place_summary(report);
            if summary.is_embedded() {
                Self::embed_seed_lsb_fallback(&mut rgba, seed);
            }
            Ok((DynamicImage::ImageRgba8(rgba), Some(summary)))
        } else {
            let report = self.embed_lsb_v2_in_place(&mut rgba, &payload, seed, redundancy)?;
            let summary = Self::lsb_in_place_summary(report);
            if report.embedded {
                Self::embed_seed_lsb_fallback(&mut rgba, seed);
            }
            Ok((DynamicImage::ImageRgba8(rgba), Some(summary)))
        }
    }

    /// Embed payload using the corrected V2 carrier scheme.
    ///
    /// The V2 scheme operates over `width * height * 3` RGB carrier slots.
    /// Each payload bit is spread across `STEGO_SPREAD_FACTOR * redundancy`
    /// distinct slots selected by a single true bijection permutation.
    /// All replicas of the same bit use consecutive logical indices through
    /// one permutation, guaranteeing no inter-replica collisions:
    /// - Exact capacity model: `required = payload_bits * STEGO_SPREAD_FACTOR * redundancy`
    /// - No slot collisions within one embedding
    /// - True bijection for arbitrary (including non-power-of-two) slot counts
    #[allow(dead_code)]
    pub(crate) fn embed_lsb_v2(
        &self,
        img: &RgbaImage,
        payload: &[u8],
        seed: u64,
        redundancy: usize,
    ) -> crate::stego::EmbedOutcome<RgbaImage> {
        carrier_support::corrected_lsb_embed(img, payload, seed, redundancy)
    }

    pub(crate) fn embed_lsb_v2_in_place(
        &self,
        img: &mut RgbaImage,
        payload: &[u8],
        seed: u64,
        redundancy: usize,
    ) -> Result<crate::stego::InPlaceEmbedReport> {
        carrier_support::corrected_lsb_embed_in_place(img, payload, seed, redundancy)
            .map_err(Into::into)
    }

    fn lsb_in_place_summary(
        report: crate::stego::InPlaceEmbedReport,
    ) -> crate::stego::EmbedOutcomeSummary {
        crate::stego::EmbedOutcomeSummary {
            status: if report.embedded {
                crate::stego::EmbedStatus::Embedded
            } else {
                crate::stego::EmbedStatus::SkippedCapacity
            },
            path: crate::stego::EmbedPath::Lsb,
            payload_bytes: report.payload_bytes,
            required_capacity: report.required_capacity,
            available_capacity: report.available_capacity,
        }
    }

    pub(crate) fn embed_seed_lsb_fallback(img: &mut RgbaImage, seed: u64) {
        carrier_support::seed_fallback_embed(img, seed);
    }

    pub(crate) fn embed_seed_lsb_fallback_pub(img: &mut RgbaImage, seed: u64) {
        carrier_support::seed_fallback_embed(img, seed);
    }

    /// Embed the full payload once per tile for crop resistance.
    ///
    /// Each `tile_size × tile_size` pixel region embeds the full payload using
    /// a tile-specific seed (see [`tile_seed`]). Redundancy is fixed at 1
    /// because the tile *grid* itself is the redundancy — multiple tiles
    /// already cover the same payload.
    ///
    /// Tiles do not overlap; right/bottom edge tiles may be partial and the
    /// embed is silently skipped for those. At least one full interior tile
    /// will survive any reasonable crop.
    ///
    /// When `tile_size == 0` the image is left unchanged — this is the
    /// "tiling disabled" sentinel, and the caller is expected to route
    /// through the non-tiled path instead.
    pub(crate) fn embed_lsb_tiled_in_place(
        &self,
        img: &mut RgbaImage,
        payload: &[u8],
        master_seed: u64,
        tile_size: u32,
    ) -> crate::stego::InPlaceEmbedReport {
        carrier_support::tiled_lsb_embed_in_place(img, payload, master_seed, tile_size)
    }

    fn lsb_tiled_in_place_summary(
        report: crate::stego::InPlaceEmbedReport,
    ) -> crate::stego::EmbedOutcomeSummary {
        crate::stego::EmbedOutcomeSummary {
            status: if report.embedded {
                crate::stego::EmbedStatus::Embedded
            } else {
                crate::stego::EmbedStatus::SkippedCapacity
            },
            path: crate::stego::EmbedPath::LsbTiled,
            payload_bytes: report.payload_bytes,
            required_capacity: report.required_capacity,
            available_capacity: report.available_capacity,
        }
    }

    pub(crate) fn apply_to_image_owned(
        &self,
        img: &DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<DynamicImage> {
        let (image, _summary) = self.apply_to_image_with_summary(img, ctx)?;
        Ok(image)
    }

    pub(crate) fn apply_to_image_with_summary(
        &self,
        img: &DynamicImage,
        ctx: &ProtectionContext,
    ) -> Result<(DynamicImage, Option<crate::stego::EmbedOutcomeSummary>)> {
        let format = ctx
            .input_format()
            .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT);

        let is_tiled = ctx.tile_size().filter(|&s| s > 0).is_some();
        let embed_path = match format {
            crate::types::ImageOutputFormat::Jpeg => {
                if is_tiled {
                    crate::stego::EmbedPath::DctF5Tiled
                } else {
                    crate::stego::EmbedPath::DctF5
                }
            }
            _ => {
                if is_tiled {
                    crate::stego::EmbedPath::LsbTiled
                } else {
                    crate::stego::EmbedPath::Lsb
                }
            }
        };

        let emission = crate::types::PayloadEmissionContext::from_plan_for_context(ctx, embed_path);
        let payload = self.generate_payload(&emission, ctx);
        let mut rgba = img.to_rgba8();

        let redundancy = ctx.effective_redundancy();

        match format {
            crate::types::ImageOutputFormat::Png => {
                if let Some(tile_size) = ctx.tile_size().filter(|&s| s > 0) {
                    let report =
                        self.embed_lsb_tiled_in_place(&mut rgba, &payload, ctx.seed(), tile_size);
                    let summary = Self::lsb_tiled_in_place_summary(report);
                    if summary.is_embedded() {
                        Self::embed_seed_lsb_fallback(&mut rgba, ctx.seed());
                    }
                    Ok((DynamicImage::ImageRgba8(rgba), Some(summary)))
                } else {
                    let report =
                        self.embed_lsb_v2_in_place(&mut rgba, &payload, ctx.seed(), redundancy)?;
                    let summary = Self::lsb_in_place_summary(report);
                    if summary.is_embedded() {
                        Self::embed_seed_lsb_fallback(&mut rgba, ctx.seed());
                    }
                    Ok((DynamicImage::ImageRgba8(rgba), Some(summary)))
                }
            }
            crate::types::ImageOutputFormat::Jpeg => {
                let jpeg_bytes = crate::util::image::encode_image_with_options(
                    img,
                    Some(crate::types::ImageOutputFormat::Jpeg),
                    ctx.progressive_jpeg(),
                    ctx.jpeg_quality(),
                )?;
                let with_stego = self.apply_dct_stego_bytes(&jpeg_bytes, ctx)?;
                let (output, summary) = with_stego.into_parts();
                Ok((image::load_from_memory(&output)?, Some(summary)))
            }
            crate::types::ImageOutputFormat::WebP => {
                if let Some(tile_size) = ctx.tile_size().filter(|&s| s > 0) {
                    let report =
                        self.embed_lsb_tiled_in_place(&mut rgba, &payload, ctx.seed(), tile_size);
                    let summary = Self::lsb_tiled_in_place_summary(report);
                    if summary.is_embedded() {
                        Self::embed_seed_lsb_fallback(&mut rgba, ctx.seed());
                    }
                    Ok((DynamicImage::ImageRgba8(rgba), Some(summary)))
                } else {
                    let report =
                        self.embed_lsb_v2_in_place(&mut rgba, &payload, ctx.seed(), redundancy)?;
                    let summary = Self::lsb_in_place_summary(report);
                    if summary.is_embedded() {
                        Self::embed_seed_lsb_fallback(&mut rgba, ctx.seed());
                    }
                    Ok((DynamicImage::ImageRgba8(rgba), Some(summary)))
                }
            }
        }
    }
}
