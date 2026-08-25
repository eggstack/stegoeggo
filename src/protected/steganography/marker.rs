//! Current StegoEggo application-marker construction.

use super::*;

impl SteganographyProtector {
    pub(crate) fn payload_bits_for_context(ctx: &ProtectionContext) -> usize {
        if ctx.mac_key().is_some() {
            V3_HMAC_PAYLOAD_BITS
        } else {
            V3_CRC_PAYLOAD_BITS
        }
    }

    /// Generates the steganography payload containing protection metadata.
    ///
    /// The payload uses the v3 wire format with a TLV header:
    /// - Magic bytes (3 bytes: 0x53 0x45 0x03)
    /// - Protection level byte
    /// - Seed (8 bytes, little-endian)
    /// - Intensity (2 bytes, scaled by 100)
    /// - Flags and extension count
    /// - Optional TLV extensions
    /// - Authentication tag (HMAC-SHA256 truncated, or CRC32 + ECC)
    pub(crate) fn generate_payload(
        &self,
        emission: &PayloadEmissionContext,
        ctx: &ProtectionContext,
    ) -> Vec<u8> {
        let intensity_val = (ctx.intensity() * 100.0) as u16;

        let dmi_byte = ctx
            .dmi_value()
            .map(|d| match d {
                crate::types::DmiValue::Unspecified => 0u8,
                crate::types::DmiValue::Allowed => 1,
                crate::types::DmiValue::ProhibitedAiMlTraining => 2,
                crate::types::DmiValue::ProhibitedGenAiMlTraining => 3,
                crate::types::DmiValue::ProhibitedExceptSearchEngineIndexing => 4,
                crate::types::DmiValue::Prohibited => 5,
                crate::types::DmiValue::ProhibitedSeeConstraints => 6,
            })
            .unwrap_or(0);

        let content_hash_8 = ctx
            .content_hash()
            .map(|h| {
                let mut buf = [0u8; 8];
                buf[..4].copy_from_slice(&h);
                buf
            })
            .unwrap_or([0u8; 8]);

        let has_mac = emission.has_mac();

        let flags = crate::payload_v3::types::PayloadFlags {
            has_extensions: !emission.extensions.is_empty(),
            has_key_id: emission.key_id.is_some(),
            tiled: emission.tiled,
            progressive_jpeg: emission.progressive_output,
            critical_extension: false,
            signed: false,
            reserved: 0,
        };

        let channels = crate::payload_v3::types::ProtectionChannels {
            rights_metadata: emission.rights_metadata_planned,
            hidden_marker: true,
            authentication: has_mac,
        };

        let (auth_algo, auth_tag_len) = if has_mac {
            (AuthAlgorithm::HmacSha256Truncated, 16u8)
        } else {
            (AuthAlgorithm::Crc32, 4u8)
        };

        let mut buf =
            Vec::with_capacity(crate::payload_v3::types::V3_CORE_SIZE + auth_tag_len as usize);

        buf.extend_from_slice(&V3_MAGIC);
        buf.push(V3_PAYLOAD_VERSION);
        buf.push(crate::payload_v3::types::V3_CORE_SIZE as u8);
        let total_length = crate::payload_v3::types::V3_CORE_SIZE + auth_tag_len as usize;
        buf.extend_from_slice(&(total_length as u16).to_le_bytes());
        buf.extend_from_slice(&flags.to_bits().to_le_bytes());
        buf.extend_from_slice(&channels.to_bits().to_le_bytes());
        buf.push(dmi_byte);
        buf.extend_from_slice(&ctx.seed().to_le_bytes());
        buf.extend_from_slice(&intensity_val.to_le_bytes());
        buf.extend_from_slice(&content_hash_8);
        buf.push(auth_algo as u8);
        buf.push(auth_tag_len);
        buf.push(0);

        debug_assert!(
            emission.extensions.is_empty() && emission.key_id.is_none(),
            "V3 writer does not emit extensions or key IDs; flags must not advertise them"
        );
        debug_assert_eq!(buf.len(), crate::payload_v3::types::V3_CORE_SIZE);

        let auth_tag = if let Some(key) = ctx.mac_key() {
            Self::compute_payload_mac_v3(&buf, key).to_vec()
        } else {
            Self::compute_checksum(&buf).to_vec()
        };
        buf.extend_from_slice(&auth_tag);

        buf
    }

    pub(crate) fn generate_payload_for_plan(
        &self,
        emission: &PayloadEmissionContext,
        plan: &crate::types::ResolvedProtectionPlan,
    ) -> Vec<u8> {
        use crate::payload_v3::types::{AuthAlgorithm, V3_CORE_SIZE, V3_MAGIC, V3_PAYLOAD_VERSION};

        let intensity_val = (plan.intensity() * 100.0) as u16;

        let dmi_byte = plan
            .effective_dmi()
            .map(|d| match d {
                crate::types::DmiValue::Unspecified => 0u8,
                crate::types::DmiValue::Allowed => 1,
                crate::types::DmiValue::ProhibitedAiMlTraining => 2,
                crate::types::DmiValue::ProhibitedGenAiMlTraining => 3,
                crate::types::DmiValue::ProhibitedExceptSearchEngineIndexing => 4,
                crate::types::DmiValue::Prohibited => 5,
                crate::types::DmiValue::ProhibitedSeeConstraints => 6,
            })
            .unwrap_or(0);

        let content_hash_8 = plan
            .content_hash()
            .map(|h| {
                let mut buf = [0u8; 8];
                buf[..4].copy_from_slice(&h);
                buf
            })
            .unwrap_or([0u8; 8]);

        let has_mac = emission.has_mac();

        let flags = crate::payload_v3::types::PayloadFlags {
            has_extensions: !emission.extensions.is_empty(),
            has_key_id: emission.key_id.is_some(),
            tiled: emission.tiled,
            progressive_jpeg: emission.progressive_output,
            critical_extension: false,
            signed: false,
            reserved: 0,
        };

        let channels = crate::payload_v3::types::ProtectionChannels {
            rights_metadata: emission.rights_metadata_planned,
            hidden_marker: true,
            authentication: has_mac,
        };

        let (auth_algo, auth_tag_len) = if has_mac {
            (AuthAlgorithm::HmacSha256Truncated, 16u8)
        } else {
            (AuthAlgorithm::Crc32, 4u8)
        };

        let mut buf = Vec::with_capacity(V3_CORE_SIZE + auth_tag_len as usize);

        buf.extend_from_slice(&V3_MAGIC);
        buf.push(V3_PAYLOAD_VERSION);
        buf.push(V3_CORE_SIZE as u8);
        let total_length = V3_CORE_SIZE + auth_tag_len as usize;
        buf.extend_from_slice(&(total_length as u16).to_le_bytes());
        buf.extend_from_slice(&flags.to_bits().to_le_bytes());
        buf.extend_from_slice(&channels.to_bits().to_le_bytes());
        buf.push(dmi_byte);
        buf.extend_from_slice(&plan.seed().to_le_bytes());
        buf.extend_from_slice(&intensity_val.to_le_bytes());
        buf.extend_from_slice(&content_hash_8);
        buf.push(auth_algo as u8);
        buf.push(auth_tag_len);
        buf.push(0);

        debug_assert!(
            emission.extensions.is_empty() && emission.key_id.is_none(),
            "V3 writer does not emit extensions or key IDs; flags must not advertise them"
        );
        debug_assert_eq!(buf.len(), V3_CORE_SIZE);

        let auth_tag = if let Some(key) = plan.mac_key() {
            Self::compute_payload_mac_v3(&buf, key).to_vec()
        } else {
            Self::compute_checksum(&buf).to_vec()
        };
        buf.extend_from_slice(&auth_tag);

        buf
    }

    /// Test-only wrapper: generate payload from a [`ProtectionContext`].
    ///
    /// Derives the emission context from the context's fields. For tests
    /// that call `generate_payload` directly rather than through the
    /// pipeline.
    #[cfg(test)]
    pub(crate) fn generate_payload_from_ctx(&self, ctx: &ProtectionContext) -> Vec<u8> {
        self.generate_payload_for_context(ctx)
    }

    /// Generate the V3 stego payload for a given context.
    ///
    /// Exposed for testing channel flags and payload structure without
    /// requiring a full image embed/extract cycle.
    #[doc(hidden)]
    pub fn generate_payload_for_context(&self, ctx: &ProtectionContext) -> Vec<u8> {
        let embed_path = if ctx.is_tile_mode_enabled() {
            if ctx.input_format() == Some(crate::types::ImageOutputFormat::Jpeg) {
                crate::stego::EmbedPath::DctF5Tiled
            } else {
                crate::stego::EmbedPath::LsbTiled
            }
        } else if ctx.input_format() == Some(crate::types::ImageOutputFormat::Jpeg) {
            crate::stego::EmbedPath::DctF5
        } else {
            crate::stego::EmbedPath::Lsb
        };
        let emission = PayloadEmissionContext::from_plan_for_context(ctx, embed_path);
        self.generate_payload(&emission, ctx)
    }
}
