use crate::error::{Error, Result};
use crate::traits::Protector;
use crate::types::{ImageOutputFormat, MetadataUpdatePolicy, ProtectionContext, ProtectionLevel};
use image::DynamicImage;
use std::borrow::Cow;

mod common;
mod jpeg;
mod notice;
mod png;
mod webp;

pub(crate) use self::common::current_timestamp_iso8601;

#[cfg(test)]
use self::common::{
    current_date_iso, date_parts_from_secs, is_leap_year, timestamp_iso8601_from_secs,
};
#[cfg(test)]
use self::notice::{has_copyright_prefix, xml_escape};
#[cfg(test)]
use crate::types::{DmiValue, LegalMetadata, RightsNotice};
#[cfg(test)]
use crc32fast::Hasher as Crc32Hasher;

/// Metadata injection protector for the Light protection level.
///
/// Injects rights-reservation markers into image headers: tEXt/iTXt chunks for PNG,
/// COM/XMP markers for JPEG, and EXIF/XML for WebP. Also embeds the protection
/// seed in metadata for later extraction.
///
/// When used through the pipeline at `Light` level, operates on bytes (encode,
/// inject metadata, decode) so that metadata survives in the byte output.
pub struct RightsMetadataProtector;

impl RightsMetadataProtector {
    /// Create a new metadata trap protector.
    pub fn new() -> Self {
        Self
    }
}

impl RightsMetadataProtector {
    pub(crate) fn has_stego_owned_metadata(
        &self,
        img_bytes: &[u8],
        format: ImageOutputFormat,
    ) -> bool {
        match format {
            ImageOutputFormat::Png => self.png_has_stego_metadata(img_bytes),
            ImageOutputFormat::Jpeg => self.jpeg_has_stego_metadata(img_bytes),
            ImageOutputFormat::WebP => self.webp_has_stego_metadata(img_bytes),
        }
    }
}

impl Default for RightsMetadataProtector {
    fn default() -> Self {
        Self::new()
    }
}

impl Protector for RightsMetadataProtector {
    /// # Warning
    ///
    /// This method returns the image **unchanged**. Metadata injection operates at the
    /// byte level (PNG tEXt chunks, JPEG COM/APP markers, WebP XMP chunks) and cannot
    /// be preserved through the `DynamicImage` encode/decode cycle.
    ///
    /// Use [`apply_bytes`](Protector::apply_bytes) for byte-level metadata injection,
    /// or use the [`ProtectionPipeline`](crate::ProtectionPipeline) which routes
    /// metadata correctly via `process_bytes` or `apply_light_bytes`.
    fn apply<'a>(
        &self,
        img: &'a DynamicImage,
        _ctx: &ProtectionContext,
    ) -> Result<Cow<'a, DynamicImage>> {
        Ok(Cow::Borrowed(img))
    }

    fn apply_bytes(&self, img_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>> {
        self.inject_bytes(img_bytes, ctx)
    }

    fn name(&self) -> &'static str {
        "metadata_trap"
    }

    fn protection_level(&self) -> ProtectionLevel {
        ProtectionLevel::Light
    }

    fn estimated_latency_ms(&self) -> u32 {
        2
    }

    fn requires_bytes_level(&self) -> bool {
        true
    }
}

impl RightsMetadataProtector {
    /// Extract the protection seed from image metadata.
    ///
    /// Parses PNG tEXt/iTXt chunks, JPEG COM/XMP markers, or WebP metadata
    /// looking for the `X-Protection-Seed` key. Returns `None` if no seed is found
    /// or the image format is unrecognized.
    pub fn extract_seed_from_image(img_bytes: &[u8]) -> Option<u64> {
        Self::extract_seed_from_image_with_limits(img_bytes, None)
    }

    /// Extract the protection seed from image metadata, with optional resource limits.
    ///
    /// When limits are provided, chunk/segment iteration is bounded to prevent
    /// resource exhaustion on malicious inputs.
    pub fn extract_seed_from_image_with_limits(
        img_bytes: &[u8],
        limits: Option<&crate::ResourceLimits>,
    ) -> Option<u64> {
        Self::extract_seed_from_image_with_limits_truncated(img_bytes, limits).0
    }

    /// Extract the protection seed from image metadata, reporting limit truncation.
    ///
    /// Like [`extract_seed_from_image_with_limits`](Self::extract_seed_from_image_with_limits),
    /// but also reports whether a resource limit truncated the scan before a
    /// seed could be found. When `truncated` is true, `None` must not be read
    /// as "unprotected" — the seed-bearing marker may lie past the limit.
    pub fn extract_seed_from_image_with_limits_truncated(
        img_bytes: &[u8],
        limits: Option<&crate::ResourceLimits>,
    ) -> (Option<u64>, bool) {
        if img_bytes.len() < 8 {
            return (None, false);
        }

        if img_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            Self::extract_seed_from_png_truncated(img_bytes, limits)
        } else if img_bytes.starts_with(&[0xFF, 0xD8]) {
            Self::extract_seed_from_jpeg_truncated(img_bytes, limits)
        } else if img_bytes.len() >= 12
            && &img_bytes[0..4] == b"RIFF"
            && &img_bytes[8..12] == b"WEBP"
        {
            Self::extract_seed_from_webp_truncated(img_bytes, limits)
        } else {
            (None, false)
        }
    }
}

impl RightsMetadataProtector {
    #[doc(hidden)]
    pub fn inject_bytes(&self, img_bytes: &[u8], ctx: &ProtectionContext) -> Result<Vec<u8>> {
        let should_inject_metadata =
            Self::should_inject_metadata(ctx.inject_metadata(), ctx.protection_level());

        let notice = ctx.normalize_rights_notice();

        let metadata = self.generate_rights_metadata_from_notice(
            &notice,
            should_inject_metadata,
            ctx.inject_legal_claims(),
        );

        if metadata.is_empty() {
            return Ok(img_bytes.to_vec());
        }

        let format = ctx
            .output_format()
            .or(ctx.input_format())
            .unwrap_or_else(|| {
                ImageOutputFormat::from_magic_bytes(img_bytes)
                    .unwrap_or(crate::types::DEFAULT_OUTPUT_FORMAT)
            });

        match ctx.metadata_update_policy() {
            MetadataUpdatePolicy::FailOnConflict => {
                if self.has_stego_owned_metadata(img_bytes, format) {
                    return Err(Error::Metadata(
                        "MetadataUpdatePolicy::FailOnConflict: \
                         image already contains StegoEggo metadata"
                            .to_string(),
                    ));
                }
            }
            MetadataUpdatePolicy::PreserveExisting => {
                if self.has_stego_owned_metadata(img_bytes, format) {
                    let existing_keys = match format {
                        ImageOutputFormat::Png => Self::collect_stego_owned_png_keys(img_bytes),
                        ImageOutputFormat::Jpeg => Self::collect_stego_owned_jpeg_keys(img_bytes),
                        ImageOutputFormat::WebP => Self::collect_stego_owned_webp_keys(img_bytes),
                    };
                    let metadata: Vec<_> = metadata
                        .into_iter()
                        .filter(|(k, _)| !existing_keys.contains(k))
                        .collect();
                    if metadata.is_empty() && notice.dmi().is_none() {
                        return Ok(img_bytes.to_vec());
                    }
                    return match format {
                        ImageOutputFormat::Png => self.inject_text_chunks_png(
                            img_bytes,
                            &metadata,
                            notice.dmi(),
                            notice.seed(),
                            Some(&ctx.resource_limits()),
                        ),
                        ImageOutputFormat::Jpeg => {
                            let (effective_dmi, emit_structured_com) =
                                Self::jpeg_preserve_existing_suppression(
                                    &existing_keys,
                                    notice.dmi(),
                                );
                            self.inject_text_chunks_jpeg_with_timestamp(
                                img_bytes,
                                &metadata,
                                effective_dmi,
                                notice.seed(),
                                Some(ctx),
                                notice.notice_applied_at(),
                                emit_structured_com,
                            )
                        }
                        ImageOutputFormat::WebP => {
                            self.inject_text_chunks_webp_from_notice(img_bytes, &notice)
                        }
                    };
                }
            }
            MetadataUpdatePolicy::ReplaceStegoOwned => {}
        }

        let stripped = match format {
            ImageOutputFormat::Png => Self::strip_stego_owned_png(img_bytes)?,
            ImageOutputFormat::Jpeg => Self::strip_stego_owned_jpeg(img_bytes)?,
            ImageOutputFormat::WebP => Self::strip_stego_owned_webp(img_bytes)?,
        };

        let with_metadata = match format {
            ImageOutputFormat::Png => self.inject_text_chunks_png(
                &stripped,
                &metadata,
                notice.dmi(),
                notice.seed(),
                Some(&ctx.resource_limits()),
            )?,
            ImageOutputFormat::Jpeg => self.inject_text_chunks_jpeg_with_timestamp(
                &stripped,
                &metadata,
                notice.dmi(),
                notice.seed(),
                Some(ctx),
                notice.notice_applied_at(),
                true,
            )?,
            ImageOutputFormat::WebP => {
                self.inject_text_chunks_webp_from_notice(&stripped, &notice)?
            }
        };

        Ok(with_metadata)
    }

    /// Inject metadata bytes using a resolved plan directly.
    ///
    /// This is the canonical plan-aware entry point that avoids reconstructing
    /// a full `ProtectionContext`. It uses `plan.effective_notice()` directly
    /// (already normalized during resolution) and reads format/policy/limits
    /// from the plan fields.
    pub fn inject_bytes_from_plan(
        &self,
        img_bytes: &[u8],
        plan: &crate::types::ResolvedProtectionPlan,
    ) -> Result<Vec<u8>> {
        let should_inject = plan.channels().rights_metadata;
        let notice = plan.effective_notice();
        let effective_dmi = plan.effective_dmi();

        let metadata = self.generate_rights_metadata_from_notice(notice, should_inject, None);

        if metadata.is_empty() && effective_dmi.is_none() {
            return Ok(img_bytes.to_vec());
        }

        if !should_inject {
            return Ok(img_bytes.to_vec());
        }

        let format = plan.output_format();
        let policy = plan.processing().metadata_update_policy;

        // Translation adapter: the plan-based path carries intensity, seed,
        // and resource limits, while the shared JPEG injection internals
        // (including the structured-COM renderer) still read those three
        // fields through the legacy context shape. No level/policy is set
        // here, so rendering matches the established plan-path bytes exactly.
        let limits_ctx = {
            let mut ctx = ProtectionContext::new(plan.intensity(), plan.seed());
            ctx = ctx.with_resource_limits(plan.resource_limits().clone());
            ctx
        };

        match policy {
            MetadataUpdatePolicy::FailOnConflict => {
                if self.has_stego_owned_metadata(img_bytes, format) {
                    return Err(Error::Metadata(
                        "MetadataUpdatePolicy::FailOnConflict: \
                         image already contains StegoEggo metadata"
                            .to_string(),
                    ));
                }
            }
            MetadataUpdatePolicy::PreserveExisting => {
                if self.has_stego_owned_metadata(img_bytes, format) {
                    let existing_keys = match format {
                        ImageOutputFormat::Png => Self::collect_stego_owned_png_keys(img_bytes),
                        ImageOutputFormat::Jpeg => Self::collect_stego_owned_jpeg_keys(img_bytes),
                        ImageOutputFormat::WebP => Self::collect_stego_owned_webp_keys(img_bytes),
                    };
                    let metadata: Vec<_> = metadata
                        .into_iter()
                        .filter(|(k, _)| !existing_keys.contains(k))
                        .collect();
                    if metadata.is_empty() && effective_dmi.is_none() {
                        return Ok(img_bytes.to_vec());
                    }
                    return match format {
                        ImageOutputFormat::Png => self.inject_text_chunks_png(
                            img_bytes,
                            &metadata,
                            effective_dmi,
                            notice.seed(),
                            Some(plan.resource_limits()),
                        ),
                        ImageOutputFormat::Jpeg => {
                            let (preserved_dmi, emit_structured_com) =
                                Self::jpeg_preserve_existing_suppression(
                                    &existing_keys,
                                    effective_dmi,
                                );
                            self.inject_text_chunks_jpeg_with_timestamp(
                                img_bytes,
                                &metadata,
                                preserved_dmi,
                                notice.seed(),
                                Some(&limits_ctx),
                                notice.notice_applied_at(),
                                emit_structured_com,
                            )
                        }
                        ImageOutputFormat::WebP => {
                            self.inject_text_chunks_webp_from_notice(img_bytes, notice)
                        }
                    };
                }
            }
            MetadataUpdatePolicy::ReplaceStegoOwned => {}
        }

        let stripped = match format {
            ImageOutputFormat::Png => Self::strip_stego_owned_png(img_bytes)?,
            ImageOutputFormat::Jpeg => Self::strip_stego_owned_jpeg(img_bytes)?,
            ImageOutputFormat::WebP => Self::strip_stego_owned_webp(img_bytes)?,
        };

        let with_metadata = match format {
            ImageOutputFormat::Png => self.inject_text_chunks_png(
                &stripped,
                &metadata,
                effective_dmi,
                notice.seed(),
                Some(plan.resource_limits()),
            )?,
            ImageOutputFormat::Jpeg => self.inject_text_chunks_jpeg_with_timestamp(
                &stripped,
                &metadata,
                effective_dmi,
                notice.seed(),
                Some(&limits_ctx),
                notice.notice_applied_at(),
                true,
            )?,
            ImageOutputFormat::WebP => {
                self.inject_text_chunks_webp_from_notice(&stripped, notice)?
            }
        };

        Ok(with_metadata)
    }
}

#[deprecated(note = "Use RightsMetadataProtector")]
#[allow(missing_docs)]
pub type MetadataTrapProtector = RightsMetadataProtector;

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use crate::traits::Protector;
    use crate::types::ProtectionLevel;
    use image::DynamicImage;

    fn make_test_image() -> DynamicImage {
        DynamicImage::ImageRgba8(image::ImageBuffer::from_fn(64, 64, |x, y| {
            image::Rgba([(x * 3) as u8, (y * 5) as u8, ((x + y) * 7) as u8, 255])
        }))
    }

    fn encode_png(img: &DynamicImage) -> Vec<u8> {
        crate::util::image::encode_image(img, image::ImageFormat::Png).unwrap()
    }

    fn encode_jpeg(img: &DynamicImage) -> Vec<u8> {
        crate::util::image::encode_image(img, image::ImageFormat::Jpeg).unwrap()
    }

    fn encode_webp(img: &DynamicImage) -> Vec<u8> {
        crate::util::image::encode_image(img, image::ImageFormat::WebP).unwrap()
    }

    // ── Date computation ──────────────────────────────────────────────

    #[test]
    fn is_leap_year_divisible_by_400() {
        assert!(is_leap_year(2000));
    }

    #[test]
    fn is_leap_year_divisible_by_100_not_400() {
        assert!(!is_leap_year(1900));
    }

    #[test]
    fn is_leap_year_divisible_by_4_not_100() {
        assert!(is_leap_year(2024));
    }

    #[test]
    fn is_leap_year_not_divisible_by_4() {
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn current_date_iso_format() {
        let date = current_date_iso();
        assert_eq!(date.len(), 10);
        assert_eq!(date.as_bytes()[4], b'-');
        assert_eq!(date.as_bytes()[7], b'-');
    }

    #[test]
    fn date_parts_from_secs_epoch() {
        assert_eq!(date_parts_from_secs(0), (1970, 1, 1, 0));
        assert_eq!(date_parts_from_secs(86400), (1970, 1, 2, 0));
        assert_eq!(date_parts_from_secs(86400 + 3600), (1970, 1, 2, 3600));
    }

    #[test]
    fn date_parts_from_secs_known_dates() {
        assert_eq!(timestamp_iso8601_from_secs(0), "1970-01-01T00:00:00Z");
        assert_eq!(
            timestamp_iso8601_from_secs(946684800),
            "2000-01-01T00:00:00Z"
        );
        assert_eq!(
            timestamp_iso8601_from_secs(951782400),
            "2000-02-29T00:00:00Z"
        );
        assert_eq!(
            timestamp_iso8601_from_secs(951868800),
            "2000-03-01T00:00:00Z"
        );
        assert_eq!(
            timestamp_iso8601_from_secs(1582934400),
            "2020-02-29T00:00:00Z"
        );
        assert_eq!(
            timestamp_iso8601_from_secs(1583020800),
            "2020-03-01T00:00:00Z"
        );
    }

    #[test]
    fn date_parts_from_secs_leap_year_boundaries() {
        assert_eq!(date_parts_from_secs(68169600), (1972, 2, 29, 0));
        let (y, m, d, _) = date_parts_from_secs(68169600 + 86400);
        assert_eq!((y, m, d), (1972, 3, 1));
        assert_eq!(date_parts_from_secs(4102444800), (2100, 1, 1, 0));
    }

    #[test]
    fn timestamp_iso8601_from_secs_midday() {
        assert_eq!(
            timestamp_iso8601_from_secs(946684800 + 12 * 3600 + 34 * 60 + 56),
            "2000-01-01T12:34:56Z"
        );
    }

    #[test]
    fn malformed_short_jpeg_comment_does_not_scan_the_tail() {
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xFE, 0x00, 0x01];
        jpeg.extend_from_slice(b"X-Protection-Seed: 42");

        assert!(!RightsMetadataProtector::new()
            .has_stego_owned_metadata(&jpeg, ImageOutputFormat::Jpeg,));
    }

    // ── Metadata generation ───────────────────────────────────────────

    #[test]
    fn generate_rights_metadata_with_seed() {
        let protector = RightsMetadataProtector::new();
        let metadata = protector.generate_rights_metadata(
            None,
            Some(ProtectionLevel::Standard),
            Some(42),
            None,
            None,
            None,
        );
        let seed_entry = metadata.iter().find(|(k, _)| k == b"X-Protection-Seed");
        assert!(seed_entry.is_some());
        assert_eq!(seed_entry.unwrap().1, b"42");
    }

    #[test]
    fn generate_rights_metadata_dmi_auto_mapping_removed() {
        let protector = RightsMetadataProtector::new();

        let light = protector.generate_rights_metadata(
            None,
            Some(ProtectionLevel::Light),
            None,
            None,
            None,
            None,
        );
        let dmi = light.iter().find(|(k, _)| k == b"DMI-PROHIBITED");
        assert!(
            dmi.is_none(),
            "DMI-PROHIBITED should not be emitted in new output"
        );

        let standard = protector.generate_rights_metadata(
            None,
            Some(ProtectionLevel::Standard),
            None,
            None,
            None,
            None,
        );
        let dmi = standard.iter().find(|(k, _)| k == b"DMI-PROHIBITED");
        assert!(
            dmi.is_none(),
            "DMI-PROHIBITED should not be emitted in new output"
        );
    }

    #[test]
    fn xmp_stego_detection_matches_stegoeggo_namespace_without_plus_property() {
        let xmp = b"<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
                     <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
                     <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
                     <rdf:Description rdf:about=\"\" \
                     xmlns:stegoeggo=\"https://github.com/eggstack/stegoeggo\" \
                     stegoeggo:ProtectionSeed=\"42\"/>\
                     </rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";
        assert!(RightsMetadataProtector::xmp_has_stego_properties(xmp));

        let plain_xmp =
            b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"/></x:xmpmeta>";
        assert!(!RightsMetadataProtector::xmp_has_stego_properties(
            plain_xmp
        ));
    }

    #[test]
    fn webp_stego_detection_matches_xmp_seed_marker_only() {
        let protector = RightsMetadataProtector::new();
        let xmp = b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
                     <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
                     <rdf:Description rdf:about=\"\" \
                     xmlns:stegoeggo=\"https://github.com/eggstack/stegoeggo\" \
                     stegoeggo:ProtectionSeed=\"42\"/>\
                     </rdf:RDF></x:xmpmeta>";
        let xmp_chunk_len = (xmp.len() as u32).to_le_bytes();
        let mut webp = Vec::new();
        webp.extend_from_slice(b"RIFF");
        webp.extend_from_slice(&((12 + 8 + xmp.len()) as u32).to_le_bytes());
        webp.extend_from_slice(b"WEBP");
        webp.extend_from_slice(b"XMP ");
        webp.extend_from_slice(&xmp_chunk_len);
        webp.extend_from_slice(xmp);
        if xmp.len() & 1 != 0 {
            webp.push(0);
        }
        assert!(protector.webp_has_stego_metadata(&webp));
    }

    #[test]
    fn generate_rights_metadata_no_private_markers() {
        let protector = RightsMetadataProtector::new();
        let metadata = protector.generate_rights_metadata(
            None,
            Some(ProtectionLevel::Light),
            Some(42),
            None,
            None,
            None,
        );
        let noai = metadata.iter().find(|(k, _)| k == b"noai");
        assert!(noai.is_none(), "noai should not be emitted in new output");
        let dmi = metadata.iter().find(|(k, _)| k == b"DMI-PROHIBITED");
        assert!(
            dmi.is_none(),
            "DMI-PROHIBITED should not be emitted in new output"
        );
    }

    #[test]
    fn generate_rights_metadata_disabled_skips_injection() {
        let protector = RightsMetadataProtector::new();
        let metadata = protector.generate_rights_metadata(
            None,
            Some(ProtectionLevel::Disabled),
            Some(42),
            None,
            None,
            None,
        );
        assert!(metadata.is_empty());
    }

    #[test]
    fn generate_rights_metadata_explicit_dmi_no_private_marker() {
        let protector = RightsMetadataProtector::new();
        let metadata = protector.generate_rights_metadata(
            Some(DmiValue::Allowed),
            Some(ProtectionLevel::Standard),
            None,
            None,
            None,
            None,
        );
        let dmi = metadata.iter().find(|(k, _)| k == b"DMI-PROHIBITED");
        assert!(
            dmi.is_none(),
            "DMI-PROHIBITED should not be emitted in new output"
        );
    }

    #[test]
    fn generate_rights_metadata_legal_claims() {
        let protector = RightsMetadataProtector::new();
        let legal = LegalMetadata::new()
            .with_copyright_holder("Test Corp")
            .with_contact_email("legal@test.com");
        let metadata = protector.generate_rights_metadata(
            None,
            Some(ProtectionLevel::Standard),
            None,
            Some(&legal),
            None,
            Some(true),
        );
        let copyright = metadata.iter().find(|(k, _)| k == b"Copyright");
        assert!(copyright.is_some());
        let copyright_str = String::from_utf8_lossy(&copyright.unwrap().1);
        assert!(copyright_str.contains("Test Corp"));

        let contact = metadata.iter().find(|(k, _)| k == b"Contact");
        assert_eq!(contact.unwrap().1, b"legal@test.com");
    }

    #[test]
    fn generate_rights_metadata_ai_constraints_and_web_statement() {
        let protector = RightsMetadataProtector::new();
        let legal = LegalMetadata::new()
            .with_ai_constraints("no-training")
            .with_web_statement_of_rights("https://example.com/rights");
        let metadata = protector.generate_rights_metadata(
            None,
            Some(ProtectionLevel::Standard),
            None,
            Some(&legal),
            None,
            Some(true),
        );
        let constraints = metadata.iter().find(|(k, _)| k == b"AIConstraints");
        assert_eq!(constraints.unwrap().1, b"no-training");

        let statement = metadata.iter().find(|(k, _)| k == b"WebStatementOfRights");
        assert_eq!(statement.unwrap().1, b"https://example.com/rights");
    }

    #[test]
    fn generate_rights_metadata_omits_constraints_when_none() {
        let protector = RightsMetadataProtector::new();
        let legal = LegalMetadata::new().with_copyright_holder("Test");
        let metadata = protector.generate_rights_metadata(
            None,
            Some(ProtectionLevel::Standard),
            None,
            Some(&legal),
            None,
            Some(true),
        );
        assert!(
            !metadata.iter().any(|(k, _)| k == b"AIConstraints"),
            "AIConstraints should not be present when not set"
        );
        assert!(
            !metadata.iter().any(|(k, _)| k == b"WebStatementOfRights"),
            "WebStatementOfRights should not be present when not set"
        );
    }

    // ── PNG injection + extraction ────────────────────────────────────

    #[test]
    fn png_inject_produces_valid_png() {
        let protector = RightsMetadataProtector::new();
        let png = encode_png(&make_test_image());
        let metadata = vec![(b"Test-Key".to_vec(), b"Test-Value".to_vec())];
        let result = protector
            .inject_text_chunks_png(&png, &metadata, None, None, None)
            .unwrap();
        assert!(result.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
        assert!(result.len() > png.len());
    }

    #[test]
    fn png_injected_chunks_have_valid_crc() {
        let protector = RightsMetadataProtector::new();
        let png = encode_png(&make_test_image());
        let metadata = vec![(b"Test-Key".to_vec(), b"Test-Value".to_vec())];
        let result = protector
            .inject_text_chunks_png(
                &png,
                &metadata,
                Some(DmiValue::ProhibitedAiMlTraining),
                Some(42),
                None,
            )
            .unwrap();

        let mut pos = 8;
        let mut checked_injected_chunk = false;
        while pos + 12 <= result.len() {
            let chunk_len = u32::from_be_bytes([
                result[pos],
                result[pos + 1],
                result[pos + 2],
                result[pos + 3],
            ]) as usize;
            let chunk_type = &result[pos + 4..pos + 8];
            let data_start = pos + 8;
            let data_end = data_start + chunk_len;
            let crc_start = data_end;
            let crc_end = crc_start + 4;
            assert!(crc_end <= result.len(), "PNG chunk extends past file end");

            if chunk_type == b"tEXt" || chunk_type == b"iTXt" {
                let stored = u32::from_be_bytes([
                    result[crc_start],
                    result[crc_start + 1],
                    result[crc_start + 2],
                    result[crc_start + 3],
                ]);
                let mut hasher = Crc32Hasher::new();
                hasher.update(chunk_type);
                hasher.update(&result[data_start..data_end]);
                assert_eq!(stored, hasher.finalize());
                checked_injected_chunk = true;
            }

            if chunk_type == b"IEND" {
                break;
            }
            pos = crc_end;
        }

        assert!(checked_injected_chunk);
    }

    #[test]
    fn png_inject_empty_metadata_returns_original() {
        let protector = RightsMetadataProtector::new();
        let png = encode_png(&make_test_image());
        let result = protector
            .inject_text_chunks_png(&png, &[], None, None, None)
            .unwrap();
        assert_eq!(result, png);
    }

    #[test]
    fn png_inject_invalid_signature_errors() {
        let protector = RightsMetadataProtector::new();
        let result = protector.inject_text_chunks_png(
            b"NOTAPNG",
            &[(b"key".to_vec(), b"val".to_vec())],
            None,
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn png_seed_roundtrip() {
        let protector = RightsMetadataProtector::new();
        let png = encode_png(&make_test_image());
        let metadata = vec![];
        let result = protector
            .inject_text_chunks_png(&png, &metadata, None, None, None)
            .unwrap();
        let extracted = RightsMetadataProtector::extract_seed_from_png(&result, None);
        assert!(extracted.is_none());

        // Now with seed in metadata
        let metadata_with_seed = vec![
            (b"X-Protection-Seed".to_vec(), b"12345".to_vec()),
            (b"Other".to_vec(), b"Value".to_vec()),
        ];
        let result = protector
            .inject_text_chunks_png(&png, &metadata_with_seed, None, Some(12345), None)
            .unwrap();
        let extracted = RightsMetadataProtector::extract_seed_from_png(&result, None);
        assert_eq!(extracted, Some(12345));
    }

    #[test]
    fn png_dmi_injects_xmp_chunk() {
        let protector = RightsMetadataProtector::new();
        let png = encode_png(&make_test_image());
        let result = protector
            .inject_text_chunks_png(
                &png,
                &[],
                Some(DmiValue::ProhibitedAiMlTraining),
                None,
                None,
            )
            .unwrap();
        // XMP is injected as iTXt chunk with "XML:com.adobe.xmp" keyword
        assert!(result.len() > png.len());
    }

    // ── JPEG injection + extraction ───────────────────────────────────

    #[test]
    fn jpeg_inject_produces_valid_jpeg() {
        let protector = RightsMetadataProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let metadata = vec![(b"Test-Key".to_vec(), b"Test-Value".to_vec())];
        let result = protector
            .inject_text_chunks_jpeg(&jpeg, &metadata, None, None, None)
            .unwrap();
        assert!(result.starts_with(&[0xFF, 0xD8]));
        assert!(result.ends_with(&[0xFF, 0xD9]));
    }

    #[test]
    fn jpeg_seed_roundtrip() {
        let protector = RightsMetadataProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let metadata = vec![(b"X-Protection-Seed".to_vec(), b"54321".to_vec())];
        let result = protector
            .inject_text_chunks_jpeg(&jpeg, &metadata, None, Some(54321), None)
            .unwrap();
        let extracted = RightsMetadataProtector::extract_seed_from_jpeg(&result, None).unwrap();
        assert_eq!(extracted, 54321);
    }

    #[test]
    fn jpeg_seed_scan_reports_truncation_past_segment_limit() {
        fn com_segment(payload: &[u8]) -> Vec<u8> {
            let mut seg = vec![0xFF, 0xFE];
            let len = (payload.len() + 2) as u16;
            seg.extend_from_slice(&len.to_be_bytes());
            seg.extend_from_slice(payload);
            seg
        }
        let mut jpeg = vec![0xFF, 0xD8];
        for i in 0..4u8 {
            jpeg.extend_from_slice(&com_segment(&[b'f', b'i', b'l', b'l', b'0' + i]));
        }
        jpeg.extend_from_slice(&com_segment(b"X-Protection-Seed: 777"));
        jpeg.extend_from_slice(&[0xFF, 0xD9]);
        let tight = crate::ResourceLimits::builder()
            .max_jpeg_segments(4)
            .build();
        let (seed, truncated) =
            RightsMetadataProtector::extract_seed_from_jpeg_truncated(&jpeg, Some(&tight));
        assert_eq!(seed, None);
        assert!(truncated);
        let (seed, truncated) =
            RightsMetadataProtector::extract_seed_from_jpeg_truncated(&jpeg, None);
        assert_eq!(seed, Some(777));
        assert!(!truncated);
    }

    #[test]
    fn dmi_caption_markers_match_dmi_values() {
        use crate::types::DmiValue;
        for value in [
            DmiValue::Unspecified,
            DmiValue::Allowed,
            DmiValue::ProhibitedAiMlTraining,
            DmiValue::ProhibitedGenAiMlTraining,
            DmiValue::ProhibitedExceptSearchEngineIndexing,
            DmiValue::Prohibited,
            DmiValue::ProhibitedSeeConstraints,
        ] {
            let caption = format!("DMI: {}", value.as_str());
            assert!(
                RightsMetadataProtector::jpeg_data_has_dmi_caption(caption.as_bytes()),
                "caption for {:?} must be detected",
                value
            );
        }
        assert!(!RightsMetadataProtector::jpeg_data_has_dmi_caption(
            b"Sunset over the harbor"
        ));
    }

    #[test]
    fn legacy_generate_xmp_notice_forwards_all_legal_fields() {
        use crate::types::LegalMetadata;
        let legal = LegalMetadata::new()
            .with_copyright_holder("Holder")
            .with_creator("Creator")
            .with_usage_terms("Terms")
            .with_ai_constraints("No AI")
            .with_web_statement_of_rights("https://example.com/rights")
            .with_contact_email("legal@example.com")
            .with_license_url("https://example.com/license")
            .with_credit_line("Credit")
            .with_creation_date("2025-01-01")
            .with_copyright_owner("Owner")
            .with_licensor_name("Licensor")
            .with_licensor_email("lic@example.com")
            .with_licensor_url("https://example.com/licensor")
            .with_metadata_date("2025-06-01")
            .with_notice_applied_at("2025-06-15T12:00:00Z");
        let xmp = RightsMetadataProtector::generate_xmp_notice(
            crate::types::DmiValue::ProhibitedAiMlTraining,
            Some(7),
            Some(&legal),
        );
        let xmp_str = String::from_utf8_lossy(&xmp);
        for needle in [
            "stegoeggo:CopyrightOwner",
            "stegoeggo:LicensorName",
            "stegoeggo:LicensorEmail",
            "stegoeggo:LicensorURL",
            "xmp:MetadataDate",
            "stegoeggo:NoticeAppliedAt",
        ] {
            assert!(
                xmp_str.contains(needle),
                "legacy XMP notice must contain {needle}"
            );
        }
    }

    #[test]
    fn jpeg_inject_contains_markers() {
        let protector = RightsMetadataProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let metadata = vec![(b"Test".to_vec(), b"Val".to_vec())];
        let result = protector
            .inject_text_chunks_jpeg(&jpeg, &metadata, None, None, None)
            .unwrap();

        // Should contain COM marker (0xFFFE)
        assert!(result.windows(2).any(|w| w == [0xFF, 0xFE]));
    }

    #[test]
    fn jpeg_invalid_input_errors() {
        let protector = RightsMetadataProtector::new();
        let result = protector.inject_text_chunks_jpeg(
            b"NOTJPEG",
            &[(b"key".to_vec(), b"val".to_vec())],
            None,
            None,
            None,
        );
        assert!(result.is_err());
    }

    /// Regression test for an out-of-bounds slice panic discovered by the fuzz
    /// harness in `fuzz/fuzz_targets/pipeline_bytes.rs`. A JPEG with an unknown
    /// marker (FF 0A) whose segment_len field (0x3100) exceeds the buffer must
    /// return an `Error::ImageTruncated` / `Error::Metadata`, not panic.
    #[test]
    fn jpeg_malformed_segment_length_does_not_panic() {
        let protector = RightsMetadataProtector::new();
        let malformed: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0x0A, // unknown marker
            0x31, 0x00, // segment_len = 0x3100 (12544), but only 3 bytes remain
            0x08, 0x00, 0x7A, 0x00, 0xEF,
        ];
        let result = protector.inject_text_chunks_jpeg(
            malformed,
            &[(b"key".to_vec(), b"val".to_vec())],
            None,
            None,
            None,
        );
        assert!(result.is_err(), "malformed segment length must error");
    }

    #[test]
    fn jpeg_segment_length_at_u16_max_does_not_panic() {
        let protector = RightsMetadataProtector::new();
        let malformed: &[u8] = &[
            0xFF, 0xD8, // SOI
            0xFF, 0xFE, // COM marker
            0xFF, 0xFF, // segment_len = 0xFFFF (max u16)
            0x00,
        ];
        let result = protector.inject_text_chunks_jpeg(
            malformed,
            &[(b"key".to_vec(), b"val".to_vec())],
            None,
            None,
            None,
        );
        assert!(result.is_err(), "segment_len near u16::MAX must error");
    }

    // ── WebP injection + extraction ───────────────────────────────────

    #[test]
    fn webp_inject_produces_valid_webp() {
        let protector = RightsMetadataProtector::new();
        let webp = encode_webp(&make_test_image());
        let metadata = vec![(b"Test-Key".to_vec(), b"Test-Value".to_vec())];
        let result = protector
            .inject_text_chunks_webp(&webp, &metadata, None, None, None)
            .unwrap();
        assert!(result.starts_with(b"RIFF"));
        assert!(&result[8..12] == b"WEBP");
    }

    #[test]
    fn webp_seed_roundtrip() {
        let protector = RightsMetadataProtector::new();
        let webp = encode_webp(&make_test_image());
        let metadata = vec![(b"X-Protection-Seed".to_vec(), b"99999".to_vec())];
        let result = protector
            .inject_text_chunks_webp(&webp, &metadata, None, Some(99999), None)
            .unwrap();
        let extracted = RightsMetadataProtector::extract_seed_from_webp(&result, None);
        assert_eq!(extracted, Some(99999));
    }

    #[test]
    fn webp_invalid_input_errors() {
        let protector = RightsMetadataProtector::new();
        let result = protector.inject_text_chunks_webp(
            b"NOTWEBP",
            &[(b"key".to_vec(), b"val".to_vec())],
            None,
            None,
            None,
        );
        assert!(result.is_err());
    }

    fn build_webp_with_stego_chunks() -> Vec<u8> {
        let base = encode_webp(&make_test_image());
        assert_eq!(&base[0..4], b"RIFF");
        assert!(base.len() >= 12);
        // The image crate emits simple (non-VP8X) lossless WebP; append a
        // VP8X header chunk so VP8X flag fix-up can be exercised.
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(b"WEBP");
        let vp8x_data = [0x0Cu8, 0, 0, 0, 63, 0, 0, 63, 0, 0];
        out.extend_from_slice(b"VP8X");
        out.extend_from_slice(&(vp8x_data.len() as u32).to_le_bytes());
        out.extend_from_slice(&vp8x_data);
        out.extend_from_slice(&base[12..]);
        out.extend_from_slice(&RightsMetadataProtector::create_webp_xmp_chunk(
            &RightsMetadataProtector::generate_xmp_dmi(DmiValue::ProhibitedAiMlTraining, Some(7)),
        ));
        out.extend_from_slice(&RightsMetadataProtector::create_webp_exif_chunk(7));
        let riff_size = (out.len() - 8) as u32;
        out[4] = riff_size as u8;
        out[5] = (riff_size >> 8) as u8;
        out[6] = (riff_size >> 16) as u8;
        out[7] = (riff_size >> 24) as u8;
        out
    }

    #[test]
    fn strip_stego_owned_webp_removes_legacy_exif_seed() {
        let webp = build_webp_with_stego_chunks();

        let stripped = RightsMetadataProtector::strip_stego_owned_webp(&webp).unwrap();

        let mut saw_xmp = false;
        let mut saw_exif = false;
        let mut pos = 12;
        while pos + 8 <= stripped.len() {
            let id = &stripped[pos..pos + 4];
            let size = u32::from_le_bytes([
                stripped[pos + 4],
                stripped[pos + 5],
                stripped[pos + 6],
                stripped[pos + 7],
            ]) as usize;
            if id == b"XMP " {
                saw_xmp = true;
            }
            if id == b"EXIF" {
                saw_exif = true;
            }
            pos += 8 + size + (size & 1);
        }
        assert!(
            !saw_exif,
            "legacy EXIF seed chunk must not survive stripping"
        );
        assert!(
            saw_xmp,
            "XMP is replaced at the description level during injection, not stripped"
        );

        assert_eq!(&stripped[12..16], b"VP8X");
        let flags = stripped[20];
        assert_eq!(
            flags & 0x08,
            0,
            "VP8X EXIF flag must be cleared after stripping"
        );
        assert_ne!(flags & 0x04, 0, "unrelated VP8X XMP flag must be preserved");

        let riff_size =
            u32::from_le_bytes([stripped[4], stripped[5], stripped[6], stripped[7]]) as usize;
        assert_eq!(riff_size, stripped.len() - 8);
    }

    // ── Seed extraction dispatch ──────────────────────────────────────

    #[test]
    fn extract_seed_dispatches_png() {
        let protector = RightsMetadataProtector::new();
        let png = encode_png(&make_test_image());
        let metadata = vec![(b"X-Protection-Seed".to_vec(), b"100".to_vec())];
        let injected = protector
            .inject_text_chunks_png(&png, &metadata, None, Some(100), None)
            .unwrap();
        assert_eq!(
            RightsMetadataProtector::extract_seed_from_image(&injected),
            Some(100)
        );
    }

    #[test]
    fn extract_seed_dispatches_jpeg() {
        let protector = RightsMetadataProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let metadata = vec![(b"X-Protection-Seed".to_vec(), b"200".to_vec())];
        let injected = protector
            .inject_text_chunks_jpeg(&jpeg, &metadata, None, Some(200), None)
            .unwrap();
        assert_eq!(
            RightsMetadataProtector::extract_seed_from_image(&injected),
            Some(200)
        );
    }

    #[test]
    fn extract_seed_dispatches_webp() {
        let protector = RightsMetadataProtector::new();
        let webp = encode_webp(&make_test_image());
        let metadata = vec![(b"X-Protection-Seed".to_vec(), b"300".to_vec())];
        let injected = protector
            .inject_text_chunks_webp(&webp, &metadata, None, Some(300), None)
            .unwrap();
        assert_eq!(
            RightsMetadataProtector::extract_seed_from_image(&injected),
            Some(300)
        );
    }

    #[test]
    fn extract_seed_unknown_format_returns_none() {
        assert_eq!(
            RightsMetadataProtector::extract_seed_from_image(b"GARBAGE"),
            None
        );
    }

    #[test]
    fn extract_seed_too_short_returns_none() {
        assert_eq!(
            RightsMetadataProtector::extract_seed_from_image(&[0; 4]),
            None
        );
    }

    // ── Protector trait ───────────────────────────────────────────────

    #[test]
    fn protector_name() {
        let p = RightsMetadataProtector::new();
        assert_eq!(p.name(), "metadata_trap");
    }

    #[test]
    fn protector_level() {
        let p = RightsMetadataProtector::new();
        assert_eq!(p.protection_level(), ProtectionLevel::Light);
    }

    #[test]
    fn protector_modifies_pixels() {
        let p = RightsMetadataProtector::new();
        // Default is true; RightsMetadataProtector doesn't override.
        // apply() re-encodes the image which may alter pixel data.
        assert!(p.modifies_pixels());
    }

    #[test]
    fn protector_apply_preserves_dimensions() {
        let p = RightsMetadataProtector::new();
        let img = make_test_image();
        let ctx = ProtectionContext::new(0.5, 42);
        let (w, h) = (img.width(), img.height());
        let result = p.apply(&img, &ctx).unwrap();
        assert_eq!(result.width(), w);
        assert_eq!(result.height(), h);
    }

    #[test]
    fn protector_apply_bytes_preserves_metadata() {
        let p = RightsMetadataProtector::new();
        let img = make_test_image();
        let png = encode_png(&img);
        let ctx = ProtectionContext::new(0.5, 42);
        let result = p.apply_bytes(&png, &ctx).unwrap();

        // Metadata should be preserved in byte output
        let extracted = RightsMetadataProtector::extract_seed_from_image(&result);
        assert_eq!(extracted, Some(42));
    }

    #[test]
    fn protector_apply_bytes_disabled_returns_original() {
        let p = RightsMetadataProtector::new();
        let img = make_test_image();
        let png = encode_png(&img);
        // Pass inject_metadata=false to skip metadata injection
        let ctx = ProtectionContext::new(0.5, 42).with_metadata_injection(false);
        let result = p.apply_bytes(&png, &ctx).unwrap();
        assert_eq!(result, png);
    }

    // ── Legal metadata injection ──────────────────────────────────────

    #[test]
    fn legal_metadata_all_fields() {
        let legal = LegalMetadata::new()
            .with_copyright_holder("Author")
            .with_contact_email("a@b.com")
            .with_license_url("https://example.com/license")
            .with_usage_terms("No AI")
            .with_creation_date("2024-01-01");
        let mut metadata = Vec::new();
        RightsMetadataProtector::add_legal_metadata(&mut metadata, Some(&legal));

        let keys: Vec<&[u8]> = metadata.iter().map(|(k, _)| k.as_slice()).collect();
        assert!(keys.iter().any(|k| *k == b"Copyright"));
        assert!(keys.iter().any(|k| *k == b"Contact"));
        assert!(keys.iter().any(|k| *k == b"License"));
        assert!(keys.iter().any(|k| *k == b"UsageTerms"));
        assert!(keys.iter().any(|k| *k == b"DateCreated"));
    }

    #[test]
    fn legal_metadata_partial_fields() {
        let legal = LegalMetadata::new().with_copyright_holder("Author");
        let mut metadata = Vec::new();
        RightsMetadataProtector::add_legal_metadata(&mut metadata, Some(&legal));

        let copyright = metadata.iter().find(|(k, _)| k == b"Copyright").unwrap();
        assert!(String::from_utf8_lossy(&copyright.1).contains("Author"));

        // Contact should not be present
        assert!(!metadata.iter().any(|(k, _)| k == b"Contact"));
    }

    #[test]
    fn legal_metadata_none_produces_no_output() {
        let mut metadata = Vec::new();
        RightsMetadataProtector::add_legal_metadata(&mut metadata, None);

        assert!(
            metadata.is_empty(),
            "no fields should be emitted when legal metadata is None"
        );
    }

    // ── Edge cases ────────────────────────────────────────────────────

    #[test]
    fn inject_empty_metadata_jpeg() {
        let protector = RightsMetadataProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let result = protector
            .inject_text_chunks_jpeg(&jpeg, &[], None, None, None)
            .unwrap();
        assert_eq!(result, jpeg);
    }

    #[test]
    fn inject_empty_metadata_webp() {
        let protector = RightsMetadataProtector::new();
        let webp = encode_webp(&make_test_image());
        let result = protector
            .inject_text_chunks_webp(&webp, &[], None, None, None)
            .unwrap();
        assert_eq!(result, webp);
    }

    // ── Standards compliance ──────────────────────────────────────────

    #[test]
    fn webp_xmp_chunk_uses_correct_fourcc() {
        let xmp_data = b"test xmp data";
        let chunk = RightsMetadataProtector::create_webp_xmp_chunk(xmp_data);
        assert_eq!(&chunk[0..4], b"XMP ");
    }

    #[test]
    fn webp_xmp_chunk_contains_seed_in_xmp_content() {
        let dmi = DmiValue::ProhibitedAiMlTraining;
        let seed = Some(42u64);
        let xmp = RightsMetadataProtector::generate_xmp_dmi(dmi, seed);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(xmp_str.contains("stegoeggo:ProtectionSeed=\"42\""));
    }

    #[test]
    fn webp_xmp_chunk_no_seed_omits_attribute() {
        let dmi = DmiValue::ProhibitedAiMlTraining;
        let xmp = RightsMetadataProtector::generate_xmp_dmi(dmi, None);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(!xmp_str.contains("stegoeggo:ProtectionSeed"));
    }

    #[test]
    fn webp_xmp_legal_children_after_description_start_tag() {
        let legal = LegalMetadata::new()
            .with_copyright_holder("Test Corp")
            .with_creator("Test Author")
            .with_usage_terms("All rights reserved");
        let xmp = RightsMetadataProtector::generate_xmp_notice(
            DmiValue::ProhibitedAiMlTraining,
            Some(42),
            Some(&legal),
        );
        let xmp_str = String::from_utf8_lossy(&xmp);

        let desc_open = xmp_str
            .find("<rdf:Description")
            .expect("rdf:Description must be present");
        let desc_close_offset = xmp_str[desc_open..]
            .find('>')
            .expect("rdf:Description must have a closing >");
        let desc_close = desc_open + desc_close_offset;

        for needle in ["<dc:creator>", "<dc:rights>", "<xmpRights:UsageTerms>"] {
            let pos = xmp_str
                .find(needle)
                .unwrap_or_else(|| panic!("expected {needle} in XMP"));
            assert!(
                desc_close < pos,
                "{needle} must appear after the > of <rdf:Description ...> (desc_close={desc_close}, pos={pos})"
            );
        }

        let end_desc = xmp_str
            .find("</rdf:Description>")
            .expect("must close rdf:Description");
        for needle in ["<dc:creator>", "<dc:rights>", "<xmpRights:UsageTerms>"] {
            let pos = xmp_str.find(needle).unwrap();
            assert!(
                pos < end_desc,
                "{needle} must appear before </rdf:Description> (pos={pos}, end_desc={end_desc})"
            );
        }
    }

    #[test]
    fn webp_xmp_rdf_description_is_well_ordered() {
        let legal = LegalMetadata::new().with_copyright_holder("Test Corp");
        let xmp = RightsMetadataProtector::generate_xmp_notice(
            DmiValue::ProhibitedAiMlTraining,
            Some(7),
            Some(&legal),
        );
        let xmp_str = String::from_utf8_lossy(&xmp);

        let rdf_desc_open = xmp_str.find("<rdf:Description").unwrap();
        assert!(
            rdf_desc_open < xmp_str.find("plus:DataMining").unwrap(),
            "<rdf:Description must precede plus:DataMining"
        );

        let desc_close_rel = xmp_str[rdf_desc_open..].find('>').unwrap();
        let desc_close = rdf_desc_open + desc_close_rel;
        assert!(
            desc_close < xmp_str.find("<dc:rights>").unwrap(),
            "<rdf:Description ...> must close before any legal child element"
        );
    }

    #[test]
    fn webp_xmp_dc_rights_uses_rdf_alt() {
        let legal = LegalMetadata::new().with_copyright_holder("Test Corp");
        let xmp = RightsMetadataProtector::generate_xmp_notice(
            DmiValue::ProhibitedAiMlTraining,
            None,
            Some(&legal),
        );
        let xmp_str = String::from_utf8_lossy(&xmp);

        let rights_start = xmp_str.find("<dc:rights>").unwrap();
        let rights_end = xmp_str.find("</dc:rights>").unwrap();
        let slice = &xmp_str[rights_start..rights_end];
        assert!(
            slice.contains("<rdf:Alt>"),
            "dc:rights must contain <rdf:Alt> container"
        );
        assert!(
            slice.contains("xml:lang=\"x-default\""),
            "rdf:Alt must include xml:lang x-default"
        );
        assert!(
            slice.contains("Copyright (c) Test Corp"),
            "rdf:Alt must wrap the copyright text"
        );
    }

    #[test]
    fn webp_xmp_usage_terms_uses_rdf_alt() {
        let legal = LegalMetadata::new().with_usage_terms("All rights reserved");
        let xmp = RightsMetadataProtector::generate_xmp_notice(
            DmiValue::ProhibitedAiMlTraining,
            None,
            Some(&legal),
        );
        let xmp_str = String::from_utf8_lossy(&xmp);

        let start = xmp_str.find("<xmpRights:UsageTerms>").unwrap();
        let end = xmp_str.find("</xmpRights:UsageTerms>").unwrap();
        let slice = &xmp_str[start..end];
        assert!(slice.contains("<rdf:Alt>"));
        assert!(slice.contains("All rights reserved"));
    }

    #[test]
    fn webp_xmp_exiftool_tag_shape_regression() {
        let legal = LegalMetadata::new()
            .with_copyright_holder("Test Corp")
            .with_creator("Test Author")
            .with_contact_email("legal@test.com")
            .with_web_statement_of_rights("https://example.com/rights")
            .with_usage_terms("All rights reserved")
            .with_ai_constraints("No AI training");
        let xmp = RightsMetadataProtector::generate_xmp_notice(
            DmiValue::ProhibitedAiMlTraining,
            Some(99),
            Some(&legal),
        );
        let xmp_str = String::from_utf8_lossy(&xmp);

        for required in [
            "<rdf:Description",
            "<dc:creator>",
            "<rdf:Seq>",
            "<dc:rights>",
            "<rdf:Alt>",
            "<xmpRights:UsageTerms>",
            "<xmpRights:WebStatement>",
            "<stegoeggo:AIConstraints>",
            "stegoeggo:ProtectionSeed=\"99\"",
            "plus:DataMining=\"http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING\"",
            "</rdf:Description>",
        ] {
            assert!(
                xmp_str.contains(required),
                "XMP must contain {required}, got: {xmp_str}"
            );
        }

        let desc_close = xmp_str.find("</rdf:Description>").unwrap();
        let start_tag_close = xmp_str.find("<rdf:Description").unwrap();
        let after_open = xmp_str[start_tag_close..]
            .find('>')
            .map(|p| start_tag_close + p)
            .unwrap();
        assert!(
            after_open < desc_close,
            "rdf:Description opening tag must close before its closing tag"
        );
    }

    #[test]
    fn jpeg_exif_marker_has_tiff_header() {
        let exif_data = b"Exif test data";
        let marker = RightsMetadataProtector::create_jpeg_exif_marker(exif_data).unwrap();
        assert_eq!(marker[0], 0xFF);
        assert_eq!(marker[1], 0xE1);
        let exif_pos = marker
            .windows(6)
            .position(|w| w == b"Exif\x00\x00")
            .unwrap();
        let tiff_start = exif_pos + 6;
        assert_eq!(&marker[tiff_start..tiff_start + 2], b"II");
        assert_eq!(marker[tiff_start + 2], 42);
        assert_eq!(marker[tiff_start + 3], 0);
    }

    #[test]
    fn jpeg_exif_marker_length_matches_payload() {
        let exif_data = b"DMI: ProhibitedAiMlTraining";
        let marker = RightsMetadataProtector::create_jpeg_exif_marker(exif_data).unwrap();
        let segment_len = u16::from_be_bytes([marker[2], marker[3]]) as usize;
        assert_eq!(segment_len, marker.len() - 2);
    }

    #[test]
    fn jpeg_exif_marker_has_ifd_usercomment() {
        let exif_data = b"DMI: ProhibitedAiMlTraining";
        let marker = RightsMetadataProtector::create_jpeg_exif_marker(exif_data).unwrap();
        let exif_pos = marker
            .windows(6)
            .position(|w| w == b"Exif\x00\x00")
            .unwrap();
        let ifd_start = exif_pos + 6 + 8;
        let entry_count = u16::from_le_bytes([marker[ifd_start], marker[ifd_start + 1]]);
        assert_eq!(entry_count, 1);
        let tag = u16::from_le_bytes([marker[ifd_start + 2], marker[ifd_start + 3]]);
        assert_eq!(tag, 0x9286);
    }

    #[test]
    fn jpeg_iptc_marker_has_photoshop_resource_envelope() {
        let iptc_data = vec![0x1C, 0x02, 0x78, 0x00, 0x05, b'D', b'M', b'I', b':', b' '];
        let marker = RightsMetadataProtector::create_jpeg_iptc_marker(&iptc_data).unwrap();
        assert_eq!(marker[0], 0xFF);
        assert_eq!(marker[1], 0xED);
        let photoshop_pos = marker
            .windows(14)
            .position(|w| w == b"Photoshop 3.0\x00")
            .unwrap();
        let resource_start = photoshop_pos + 14;
        let resource_id = u16::from_be_bytes([marker[resource_start], marker[resource_start + 1]]);
        assert_eq!(resource_id, 0x0404);
    }

    #[test]
    fn xmp_contains_stegoeggo_namespace() {
        let xmp = RightsMetadataProtector::generate_xmp_dmi(DmiValue::ProhibitedAiMlTraining, None);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(xmp_str.contains("xmlns:stegoeggo=\"https://github.com/eggstack/stegoeggo\""));
    }

    #[test]
    fn xmp_contains_tdm_reservation_prohibit() {
        let xmp = RightsMetadataProtector::generate_xmp_dmi(DmiValue::ProhibitedAiMlTraining, None);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(
            !xmp_str.contains("tdm:reserve_tdm"),
            "TDM reservation should not be present in default output"
        );
        assert!(
            xmp_str.contains(
                "plus:DataMining=\"http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-AIMLTRAINING\""
            ),
            "XMP should contain canonical plus:DataMining URI"
        );
    }

    #[test]
    fn xmp_contains_tdm_reservation_allow() {
        let xmp = RightsMetadataProtector::generate_xmp_dmi(DmiValue::Allowed, None);
        let xmp_str = String::from_utf8_lossy(&xmp);
        assert!(
            !xmp_str.contains("tdm:reserve_tdm"),
            "TDM reservation should not be present in default output"
        );
        assert!(
            xmp_str.contains("plus:DataMining=\"http://ns.useplus.org/ldf/vocab/DMI-ALLOWED\""),
            "XMP should contain canonical plus:DataMining URI"
        );
    }

    #[test]
    fn structured_com_marker_roundtrip() {
        let protector = RightsMetadataProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let ctx =
            ProtectionContext::new(0.7, 42).with_format(crate::types::ImageOutputFormat::Jpeg);
        let metadata = protector.generate_rights_metadata(
            Some(DmiValue::ProhibitedAiMlTraining),
            Some(ProtectionLevel::Standard),
            Some(42),
            None,
            None,
            None,
        );
        let injected = protector
            .inject_text_chunks_jpeg(
                &jpeg,
                &metadata,
                Some(DmiValue::ProhibitedAiMlTraining),
                Some(42),
                Some(&ctx),
            )
            .unwrap();

        let extracted_seed = RightsMetadataProtector::extract_seed_from_image(&injected);
        assert_eq!(extracted_seed, Some(42));
    }

    #[test]
    fn structured_com_marker_parse_roundtrip() {
        let ctx =
            ProtectionContext::new(0.5, 12345).with_format(crate::types::ImageOutputFormat::Jpeg);
        let marker = RightsMetadataProtector::generate_structured_com_marker(
            Some(DmiValue::Prohibited),
            None,
            &ctx,
        );
        let payload = &marker[4..];
        let parsed = RightsMetadataProtector::parse_structured_com_payload(payload);
        assert!(parsed.is_some());
        let (seed, level, intensity) = parsed.unwrap();
        assert_eq!(seed, 12345);
        assert_eq!(level, 2);
        assert_eq!(intensity, 50);
    }

    // ── Redundant metadata injection ─────────────────────────────────

    #[test]
    fn png_seed_extractable_from_description_chunk() {
        let protector = RightsMetadataProtector::new();
        let img = make_test_image();
        let png = encode_png(&img);
        let ctx = ProtectionContext::new(0.5, 42);
        let injected = protector.inject_bytes(&png, &ctx).unwrap();

        let mut pos = 8;
        let mut found_description = false;
        while pos + 12 <= injected.len() {
            let chunk_len = u32::from_be_bytes([
                injected[pos],
                injected[pos + 1],
                injected[pos + 2],
                injected[pos + 3],
            ]) as usize;
            let chunk_type = &injected[pos + 4..pos + 8];
            if chunk_type == b"IEND" {
                break;
            }
            if chunk_type == b"tEXt" {
                let data_start = pos + 8;
                let data_end = (data_start + chunk_len).min(injected.len());
                let data = &injected[data_start..data_end];
                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    let key = &data[..null_pos];
                    if key == b"Description" {
                        found_description = true;
                        let value = &data[null_pos + 1..];
                        let value_str = String::from_utf8_lossy(value);
                        assert!(value_str.contains("Protected image. Seed: 42"));
                    }
                }
            }
            pos += 12 + chunk_len;
        }
        assert!(found_description, "Description chunk not found");
    }

    #[test]
    fn jpeg_iptc_has_object_name_seed() {
        let protector = RightsMetadataProtector::new();
        let img = make_test_image();
        let jpeg = encode_jpeg(&img);
        let ctx = ProtectionContext::new(0.5, 42).with_dmi(DmiValue::ProhibitedAiMlTraining);
        let injected = protector.inject_bytes(&jpeg, &ctx).unwrap();

        assert_eq!(
            RightsMetadataProtector::extract_seed_from_image(&injected),
            Some(42)
        );
    }

    #[test]
    fn iptc_detection_recognizes_generated_records() {
        let data = RightsMetadataProtector::generate_iptc_iim_dmi(
            DmiValue::ProhibitedAiMlTraining,
            Some(42),
        );
        assert!(RightsMetadataProtector::iptc_has_stego_properties(&data));

        let dmi_only =
            RightsMetadataProtector::generate_iptc_iim_dmi(DmiValue::ProhibitedAiMlTraining, None);
        assert!(RightsMetadataProtector::iptc_has_stego_properties(
            &dmi_only
        ));
    }

    #[test]
    fn truncated_iptc_record_is_rejected_without_panicking() {
        assert!(!RightsMetadataProtector::iptc_has_stego_properties(&[
            0x1C, 0x02, 0x78, 0x00, 0x10
        ]));
        assert!(!RightsMetadataProtector::iptc_has_stego_properties(&[
            0x1C, 0x02, 0x78, 0x00
        ]));
    }

    #[test]
    fn odd_sized_trailing_iptc_resource_does_not_panic() {
        let mut data = Vec::new();
        data.extend_from_slice(b"Photoshop 3.0\0");
        data.extend_from_slice(&0x0404u16.to_be_bytes());
        data.push(0);
        data.push(0);
        let iptc: [u8; 5] = [0x1C, 0x02, 0x78, 0x00, 0x41];
        data.extend_from_slice(&(iptc.len() as u32).to_be_bytes());
        data.extend_from_slice(&iptc);
        assert!(!RightsMetadataProtector::jpeg_payload_has_stego_properties(
            &data
        ));
    }

    #[test]
    fn repeated_protection_replaces_generated_app13_segment() {
        let protector = RightsMetadataProtector::new();
        let jpeg = encode_jpeg(&make_test_image());
        let ctx = ProtectionContext::new(0.5, 42).with_dmi(DmiValue::ProhibitedAiMlTraining);
        let once = protector.inject_bytes(&jpeg, &ctx).unwrap();
        let twice = protector.inject_bytes(&once, &ctx).unwrap();

        let count_once = once
            .windows(14)
            .filter(|w| *w == *b"Photoshop 3.0\0")
            .count();
        let count_twice = twice
            .windows(14)
            .filter(|w| *w == *b"Photoshop 3.0\0")
            .count();
        assert!(count_once >= 1);
        assert_eq!(count_once, count_twice);
    }

    #[test]
    fn png_text_chunk_rejects_invalid_keyword_lengths() {
        assert!(RightsMetadataProtector::create_png_text_chunk(b"", b"value", None).is_err());
        let long_key = [b'k'; 80];
        assert!(RightsMetadataProtector::create_png_text_chunk(&long_key, b"value", None).is_err());
        let max_key = [b'k'; 79];
        assert!(RightsMetadataProtector::create_png_text_chunk(&max_key, b"value", None).is_ok());
    }

    #[test]
    fn jpeg_auto_dmi_injects_standard_markers() {
        let protector = RightsMetadataProtector::new();
        let img = make_test_image();
        let jpeg = encode_jpeg(&img);
        let mut ctx =
            ProtectionContext::new(0.5, 42).with_format(crate::types::ImageOutputFormat::Jpeg);
        ctx.set_protection_level(ProtectionLevel::Standard);

        let injected = protector.inject_bytes(&jpeg, &ctx).unwrap();

        assert!(
            injected.windows(6).any(|w| w == b"Exif\x00\x00"),
            "auto DMI should create EXIF metadata"
        );
        assert!(
            injected.windows(14).any(|w| w == b"Photoshop 3.0\x00"),
            "auto DMI should create IPTC metadata"
        );
        assert!(
            injected.windows(15).any(|w| w == b"plus:DataMining"),
            "auto DMI should create XMP/PLUS metadata"
        );
    }

    #[test]
    fn webp_exif_chunk_has_seed() {
        let protector = RightsMetadataProtector::new();
        let img = make_test_image();
        let webp = encode_webp(&img);
        let ctx = ProtectionContext::new(0.5, 42).with_dmi(DmiValue::ProhibitedAiMlTraining);
        let injected = protector.inject_bytes(&webp, &ctx).unwrap();

        assert_eq!(
            RightsMetadataProtector::extract_seed_from_image(&injected),
            Some(42)
        );
    }

    #[test]
    fn png_seed_survives_description_only() {
        let protector = RightsMetadataProtector::new();
        let img = make_test_image();
        let png = encode_png(&img);
        let ctx = ProtectionContext::new(0.5, 99);
        let injected = protector.inject_bytes(&png, &ctx).unwrap();

        let mut output = Vec::new();
        output.extend_from_slice(&injected[0..8]);
        let mut pos = 8;
        while pos + 12 <= injected.len() {
            let chunk_len = u32::from_be_bytes([
                injected[pos],
                injected[pos + 1],
                injected[pos + 2],
                injected[pos + 3],
            ]) as usize;
            let chunk_type = &injected[pos + 4..pos + 8];
            let keep = chunk_type == b"IEND"
                || chunk_type == b"tEXt" && {
                    let data_start = pos + 8;
                    let data_end = (data_start + chunk_len).min(injected.len());
                    let data = &injected[data_start..data_end];
                    if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                        &data[..null_pos] == b"Description"
                    } else {
                        false
                    }
                };
            if chunk_type == b"IEND" {
                let text_chunk = RightsMetadataProtector::create_png_text_chunk(
                    b"Description",
                    b"Protected image. Seed: 99",
                    None,
                )
                .unwrap();
                output.extend_from_slice(&text_chunk);
            }
            if keep || chunk_type == b"IEND" {
                output.extend_from_slice(&injected[pos..pos + 12 + chunk_len]);
            }
            pos += 12 + chunk_len;
        }
        assert_eq!(
            RightsMetadataProtector::extract_seed_from_image(&output),
            Some(99)
        );
    }

    #[test]
    fn xml_escape_ampersand() {
        assert_eq!(xml_escape("A & B"), "A &amp; B");
    }

    #[test]
    fn xml_escape_less_than() {
        assert_eq!(xml_escape("a < b"), "a &lt; b");
    }

    #[test]
    fn xml_escape_greater_than() {
        assert_eq!(xml_escape("a > b"), "a &gt; b");
    }

    #[test]
    fn xml_escape_quotes() {
        assert_eq!(
            xml_escape(r#"he said "hello""#),
            "he said &quot;hello&quot;"
        );
    }

    #[test]
    fn xml_escape_apostrophe() {
        assert_eq!(xml_escape("it's"), "it&apos;s");
    }

    #[test]
    fn xml_escape_mixed_special_chars() {
        let input = "Copyright (c) 2024 <Corp> & \"Friends\" — it's fine";
        let escaped = xml_escape(input);
        assert!(escaped.contains("&amp;"));
        assert!(escaped.contains("&lt;"));
        assert!(escaped.contains("&gt;"));
        assert!(escaped.contains("&quot;"));
        assert!(escaped.contains("&apos;"));
    }

    #[test]
    fn xml_escape_no_change_for_plain_text() {
        let input = "Simple copyright notice 2024";
        assert_eq!(xml_escape(input), input);
    }

    #[test]
    fn webp_injection_rejects_xml_illegal_controls() {
        let protector = RightsMetadataProtector::new();
        let notice = RightsNotice::new().with_creator("invalid\u{0001}value");
        let webp = encode_webp(&make_test_image());

        assert!(protector
            .inject_text_chunks_webp_from_notice(&webp, &notice)
            .is_err());
    }

    #[test]
    fn copyright_prefix_detection_is_case_insensitive_and_bounded() {
        assert!(has_copyright_prefix("COPYRIGHT (c) ACME"));
        assert!(has_copyright_prefix("Copyright: ACME"));
        assert!(!has_copyright_prefix("Copyrightable Designs LLC"));
        assert!(!has_copyright_prefix("ACME Copyright 2024"));
    }

    #[test]
    fn webp_xmp_round_trip_special_chars() {
        let protector = RightsMetadataProtector::new();
        let img = make_test_image();
        let webp = encode_webp(&img);

        let legal = LegalMetadata::new()
            .with_copyright_holder("A < B & C")
            .with_usage_terms("Terms with \"quotes\" and 'apostrophes'")
            .with_creator("O'Brien & Sons");

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::WebP)
            .with_legal_metadata(legal)
            .with_metadata_injection(true);

        let result = protector.inject_bytes(&webp, &ctx).unwrap();
        let report = crate::verify_legal_notice(&result, b"");

        assert_eq!(
            report.copyright_holder(),
            Some("A < B & C"),
            "Copyright should survive XML escaping round-trip"
        );
        assert_eq!(
            report.usage_terms(),
            Some("Terms with \"quotes\" and 'apostrophes'"),
            "Usage terms should survive XML escaping round-trip"
        );
        assert_eq!(
            report.creator(),
            Some("O'Brien & Sons"),
            "Creator should survive XML escaping round-trip"
        );
    }

    #[test]
    fn png_text_round_trip_special_chars() {
        let protector = RightsMetadataProtector::new();
        let img = make_test_image();
        let png = encode_png(&img);

        let legal = LegalMetadata::new()
            .with_copyright_holder("A < B & C")
            .with_usage_terms("Terms with \"quotes\"");

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Png)
            .with_legal_metadata(legal)
            .with_metadata_injection(true);

        let result = protector.inject_bytes(&png, &ctx).unwrap();
        let report = crate::verify_legal_notice(&result, b"");

        assert_eq!(
            report.copyright_holder(),
            Some("A < B & C"),
            "Copyright should survive tEXt round-trip"
        );
        assert_eq!(
            report.usage_terms(),
            Some("Terms with \"quotes\""),
            "Usage terms should survive tEXt round-trip"
        );
    }

    #[test]
    fn jpeg_com_round_trip_special_chars() {
        let protector = RightsMetadataProtector::new();
        let img = make_test_image();
        let jpeg = encode_jpeg(&img);

        let legal = LegalMetadata::new()
            .with_copyright_holder("A < B & C")
            .with_usage_terms("Terms with \"quotes\"");

        let ctx = ProtectionContext::new(0.5, 42)
            .with_format(ImageOutputFormat::Jpeg)
            .with_legal_metadata(legal)
            .with_metadata_injection(true);

        let result = protector.inject_bytes(&jpeg, &ctx).unwrap();
        let report = crate::verify_legal_notice(&result, b"");

        assert_eq!(
            report.copyright_holder(),
            Some("A < B & C"),
            "Copyright should survive COM round-trip"
        );
        assert_eq!(
            report.usage_terms(),
            Some("Terms with \"quotes\""),
            "Usage terms should survive COM round-trip"
        );
    }

    #[test]
    fn malformed_xmp_non_utf8_causes_error() {
        let bad_xmp: &[u8] = &[0xFF, 0xFE, 0x00, 0x3C];
        let result = crate::xmp::filter_xmp_packet(bad_xmp);
        assert!(
            result.is_err(),
            "non-UTF8 XMP must fail, not silently discard"
        );
    }

    #[test]
    fn malformed_xmp_no_description_returns_empty() {
        let no_desc_xmp = br#"<?xml version="1.0"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<other>nothing here</other>
</x:xmpmeta>"#;
        let result = crate::xmp::filter_xmp_packet(no_desc_xmp).expect("should parse");
        assert!(
            result.is_empty(),
            "XMP without rdf:Description returns no descriptions"
        );
    }

    #[test]
    fn malformed_xmp_unclosed_description_causes_error() {
        let bad_xmp = br#"<?xml version="1.0"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:RDF>
<rdf:Description rdf:about="">
</rdf:RDF>
</x:xmpmeta>"#;
        let result = crate::xmp::filter_xmp_packet(bad_xmp);
        assert!(
            result.is_err(),
            "XMP with unclosed rdf:Description must fail"
        );
    }
}
