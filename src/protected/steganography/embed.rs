//! Application-level carrier selection and embedding dispatch.

use super::*;

impl SteganographyProtector {
    fn outcome_from_report<T>(
        report: crate::stego::EmbedReport<T>,
        path: crate::stego::EmbedPath,
    ) -> crate::stego::EmbedOutcome<T> {
        if report.embedded {
            crate::stego::EmbedOutcome::Embedded {
                output: report.output,
                payload_bytes: report.payload_bytes,
                required_capacity: report.required_capacity,
                available_capacity: report.available_capacity,
                path,
            }
        } else {
            crate::stego::EmbedOutcome::SkippedCapacity {
                output: report.output,
                payload_bytes: report.payload_bytes,
                required_capacity: report.required_capacity,
                available_capacity: report.available_capacity,
                path,
            }
        }
    }

    /// Application seed-only fallback for unsupported JPEG structures.
    ///
    /// Stores the seed hint when the tables hold it. When even the hint
    /// does not fit (short or missing quantization tables), degrades
    /// truthfully to an unmodified passthrough: the outcome still reports
    /// `UnsupportedProgressive` (no DCT payload) with the existing
    /// `ProgressiveJpegFallback` warning, and the pipeline continues with
    /// metadata injection. Other errors (malformed input) still fail.
    fn progressive_fallback(
        jpeg_bytes: &[u8],
        seed: u64,
    ) -> Result<crate::stego::EmbedOutcome<Vec<u8>>> {
        match carrier_jpeg::embed_seed_hint(jpeg_bytes, seed) {
            Ok(output) => Ok(crate::stego::EmbedOutcome::UnsupportedProgressive { output }),
            Err(crate::stego::StegoError::InsufficientCapacity { .. }) => {
                Ok(crate::stego::EmbedOutcome::UnsupportedProgressive {
                    output: jpeg_bytes.to_vec(),
                })
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Application best-effort DCT embedding.
    ///
    /// StegoEggo application policy, owned by this layer: uses the generic
    /// best-effort carrier (`carrier_jpeg::embed`, which may lower redundancy
    /// and degrade to a seed hint) and maps unsupported structures to the
    /// explicit seed-only fallback above. A short quantization table now
    /// propagates as an error instead of an unrecoverable partial hint.
    /// Callers needing exact redundancy must use `carrier_jpeg::embed_strict`.
    fn embed_dct_payload(
        jpeg_bytes: &[u8],
        payload: &[u8],
        seed: u64,
        redundancy: usize,
    ) -> Result<crate::stego::EmbedOutcome<Vec<u8>>> {
        let config = carrier_jpeg::JpegConfig::try_new(seed, redundancy)?;
        match carrier_jpeg::embed(jpeg_bytes, payload, &config) {
            Ok(report) => Ok(Self::outcome_from_report(
                report,
                crate::stego::EmbedPath::DctF5,
            )),
            Err(crate::stego::StegoError::UnsupportedJpeg(_)) => {
                Self::progressive_fallback(jpeg_bytes, seed)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn embed_dct_tiled_payload(
        jpeg_bytes: &[u8],
        payload: &[u8],
        seed: u64,
        tile_size: u32,
    ) -> Result<crate::stego::EmbedOutcome<Vec<u8>>> {
        let config = crate::stego::TileConfig::try_new(seed, tile_size)?;
        match carrier_jpeg::embed_tiled(jpeg_bytes, payload, &config) {
            Ok(report) => Ok(Self::outcome_from_report(
                report,
                crate::stego::EmbedPath::DctF5Tiled,
            )),
            Err(crate::stego::StegoError::UnsupportedJpeg(_)) => {
                Self::progressive_fallback(jpeg_bytes, seed)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn inplace_summary(
        report: crate::stego::InPlaceEmbedReport,
        path: crate::stego::EmbedPath,
    ) -> crate::stego::EmbedOutcomeSummary {
        crate::stego::EmbedOutcomeSummary {
            status: if report.embedded {
                crate::stego::EmbedStatus::Embedded
            } else {
                crate::stego::EmbedStatus::SkippedCapacity
            },
            path,
            payload_bytes: report.payload_bytes,
            required_capacity: report.required_capacity,
            available_capacity: report.available_capacity,
        }
    }

    fn embed_raster_with_seed_fallback(
        &self,
        rgba: &mut RgbaImage,
        payload: &[u8],
        seed: u64,
        redundancy: usize,
        tile_size: Option<u32>,
    ) -> Result<crate::stego::EmbedOutcomeSummary> {
        let summary = match tile_size.filter(|&s| s > 0) {
            Some(ts) => {
                let report = self.embed_lsb_tiled_in_place(rgba, payload, seed, ts)?;
                Self::inplace_summary(report, crate::stego::EmbedPath::LsbTiled)
            }
            None => {
                let report = self.embed_lsb_v2_in_place(rgba, payload, seed, redundancy)?;
                Self::inplace_summary(report, crate::stego::EmbedPath::Lsb)
            }
        };
        if summary.is_embedded() {
            Self::embed_seed_lsb_fallback(rgba, seed);
        }
        Ok(summary)
    }

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
        Self::embed_dct_payload(jpeg_bytes, &payload, ctx.seed(), ctx.effective_redundancy())
    }

    /// Embed only the seed in JPEG quantization tables (no DCT coefficient modification).
    /// Used for Light level JPEG protection — the seed is recoverable when the
    /// quantization tables themselves are preserved.
    ///
    /// Unlike the best-effort progressive fallback, a short table fails here:
    /// the seed hint is Light's only channel, so an unstoreable hint must not
    /// silently become no protection.
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
        Self::embed_dct_tiled_payload(jpeg_bytes, &payload, ctx.seed(), tile_size)
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
        match tile_size.filter(|&size| size > 0) {
            Some(size) => Self::embed_dct_tiled_payload(jpeg_bytes, &payload, plan.seed(), size),
            None => Self::embed_dct_payload(
                jpeg_bytes,
                &payload,
                plan.seed(),
                Self::effective_redundancy_for_plan(plan),
            ),
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
        let summary = self.embed_raster_with_seed_fallback(
            &mut rgba,
            &payload,
            plan.seed(),
            Self::effective_redundancy_for_plan(plan),
            tile_size,
        )?;
        Ok((DynamicImage::ImageRgba8(rgba), Some(summary)))
    }

    /// Embed payload using the corrected V2 carrier scheme.
    ///
    /// The V2 scheme operates over `width * height * 3` RGB carrier slots.
    /// Each payload bit is spread across `STEGO_SPREAD_FACTOR * redundancy`
    /// slots selected by the byte-frozen V2 cycle-walking mapping. Full-domain
    /// injectivity is verified by carrier tests for the documented small and
    /// medium domains; outside those domains distinct slots are an operational
    /// assumption, not a proven invariant:
    /// - Exact capacity model: `required = payload_bits * STEGO_SPREAD_FACTOR * redundancy`
    /// - No slot collisions observed within one embedding in tested domains
    /// - V2 mapping is byte-stable for compatibility and is not claimed to be
    ///   a proven bijection over the full domain
    #[allow(dead_code)]
    pub(crate) fn embed_lsb_v2(
        &self,
        img: &RgbaImage,
        payload: &[u8],
        seed: u64,
        redundancy: usize,
    ) -> crate::stego::EmbedOutcome<RgbaImage> {
        let config = match carrier_lsb::LsbConfig::try_new(seed, redundancy) {
            Ok(config) => config,
            Err(_) => {
                return crate::stego::EmbedOutcome::SkippedCapacity {
                    output: img.clone(),
                    payload_bytes: payload.len(),
                    required_capacity: usize::MAX,
                    available_capacity: 0,
                    path: crate::stego::EmbedPath::Lsb,
                };
            }
        };
        match carrier_lsb::embed(img, payload, &config) {
            Ok(report) => Self::outcome_from_report(report, crate::stego::EmbedPath::Lsb),
            Err(_) => crate::stego::EmbedOutcome::SkippedCapacity {
                output: img.clone(),
                payload_bytes: payload.len(),
                required_capacity: usize::MAX,
                available_capacity: 0,
                path: crate::stego::EmbedPath::Lsb,
            },
        }
    }

    pub(crate) fn embed_lsb_v2_in_place(
        &self,
        img: &mut RgbaImage,
        payload: &[u8],
        seed: u64,
        redundancy: usize,
    ) -> Result<crate::stego::InPlaceEmbedReport> {
        let config = carrier_lsb::LsbConfig::try_new(seed, redundancy)?;
        carrier_lsb::embed_in_place(img, payload, &config).map_err(Into::into)
    }

    pub(crate) fn embed_seed_lsb_fallback(img: &mut RgbaImage, seed: u64) {
        carrier_support::seed_fallback_embed(img, seed);
    }

    pub(crate) fn embed_seed_lsb_fallback_pub(img: &mut RgbaImage, seed: u64) {
        Self::embed_seed_lsb_fallback(img, seed);
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
    ) -> Result<crate::stego::InPlaceEmbedReport> {
        let config = crate::stego::TileConfig::try_new(master_seed, tile_size)?;
        carrier_lsb::embed_tiled_in_place(img, payload, &config).map_err(Into::into)
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
            _ => {
                let summary = self.embed_raster_with_seed_fallback(
                    &mut rgba,
                    &payload,
                    ctx.seed(),
                    redundancy,
                    ctx.tile_size(),
                )?;
                Ok((DynamicImage::ImageRgba8(rgba), Some(summary)))
            }
        }
    }
}

#[cfg(test)]
mod embed_helper_tests {
    use super::*;

    #[test]
    fn outcome_from_report_maps_embedded_and_skipped() {
        let embedded = crate::stego::EmbedReport {
            embedded: true,
            output: vec![1u8, 2, 3],
            payload_bytes: 3,
            required_capacity: 10,
            available_capacity: 20,
            actual_redundancy: 2,
        };
        let (output, summary) =
            SteganographyProtector::outcome_from_report(embedded, crate::stego::EmbedPath::Lsb)
                .into_parts();
        assert_eq!(output, vec![1u8, 2, 3]);
        assert!(summary.is_embedded());
        assert_eq!(summary.path, crate::stego::EmbedPath::Lsb);
        assert_eq!(summary.payload_bytes, 3);

        let skipped = crate::stego::EmbedReport {
            embedded: false,
            output: vec![9u8],
            payload_bytes: 1,
            required_capacity: 100,
            available_capacity: 5,
            actual_redundancy: 0,
        };
        let (output, summary) =
            SteganographyProtector::outcome_from_report(skipped, crate::stego::EmbedPath::DctF5)
                .into_parts();
        assert_eq!(output, vec![9u8]);
        assert!(!summary.is_embedded());
        assert_eq!(summary.path, crate::stego::EmbedPath::DctF5);
        assert_eq!(summary.required_capacity, 100);
    }

    #[test]
    fn inplace_summary_maps_status_and_path() {
        let report = crate::stego::InPlaceEmbedReport {
            embedded: true,
            payload_bytes: 36,
            required_capacity: 1440,
            available_capacity: 12288,
            actual_redundancy: 1,
        };
        let summary =
            SteganographyProtector::inplace_summary(report, crate::stego::EmbedPath::LsbTiled);
        assert!(summary.is_embedded());
        assert_eq!(summary.path, crate::stego::EmbedPath::LsbTiled);

        let report = crate::stego::InPlaceEmbedReport {
            embedded: false,
            payload_bytes: 36,
            required_capacity: 99999,
            available_capacity: 10,
            actual_redundancy: 0,
        };
        let summary = SteganographyProtector::inplace_summary(report, crate::stego::EmbedPath::Lsb);
        assert!(!summary.is_embedded());
        assert_eq!(summary.path, crate::stego::EmbedPath::Lsb);
    }

    #[test]
    fn raster_helper_embeds_plain_and_tiled_paths() {
        let protector = SteganographyProtector::new();
        let payload = vec![0xA5u8; 36];
        let mut plain = RgbaImage::from_fn(64, 64, |x, y| {
            image::Rgba([
                (x as u8).wrapping_mul(3),
                (y as u8).wrapping_mul(5),
                128,
                255,
            ])
        });
        let summary = protector
            .embed_raster_with_seed_fallback(&mut plain, &payload, 42, 1, None)
            .unwrap();
        assert!(summary.is_embedded());
        assert_eq!(summary.path, crate::stego::EmbedPath::Lsb);

        let mut tiled = RgbaImage::from_fn(64, 64, |x, y| {
            image::Rgba([
                (x as u8).wrapping_mul(3),
                (y as u8).wrapping_mul(5),
                128,
                255,
            ])
        });
        let summary = protector
            .embed_raster_with_seed_fallback(&mut tiled, &payload, 42, 1, Some(32))
            .unwrap();
        assert_eq!(summary.path, crate::stego::EmbedPath::LsbTiled);
    }
}
