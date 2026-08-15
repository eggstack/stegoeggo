//! Compatibility-only V1/V2 payload decoding and ECC adapters.

use super::*;

impl SteganographyProtector {
    pub(crate) fn parse_stego_payload_v1(payload: &[u8]) -> Option<StegoPayload> {
        let protection_level = payload[1];

        let extracted_seed = u64::from_le_bytes([
            payload[2], payload[3], payload[4], payload[5], payload[6], payload[7], payload[8],
            payload[9],
        ]);

        let intensity_raw = u16::from_le_bytes([payload[10], payload[11]]);
        let intensity = intensity_raw as f32 / 100.0;

        Some(StegoPayload {
            protection_level,
            seed: extracted_seed,
            intensity,
            version: 1,
            content_hash: None,
            dmi_value: None,
            raw_payload: None,
        })
    }

    pub(crate) fn parse_stego_payload_v2(payload: &[u8]) -> Option<StegoPayload> {
        if payload.len() < V2_HEADER_SIZE {
            return None;
        }

        let protection_level = payload[1];

        let extracted_seed = u64::from_le_bytes([
            payload[2], payload[3], payload[4], payload[5], payload[6], payload[7], payload[8],
            payload[9],
        ]);

        let intensity_raw = u16::from_le_bytes([payload[10], payload[11]]);
        let intensity = intensity_raw as f32 / 100.0;

        let content_hash = {
            let mut hash = [0u8; 4];
            hash.copy_from_slice(&payload[20..24]);
            if hash != [0, 0, 0, 0] {
                Some(hash)
            } else {
                None
            }
        };

        let dmi_value = match payload[24] {
            0 => None,
            1 => Some(crate::types::DmiValue::Allowed),
            2 => Some(crate::types::DmiValue::ProhibitedAiMlTraining),
            3 => Some(crate::types::DmiValue::ProhibitedGenAiMlTraining),
            4 => Some(crate::types::DmiValue::ProhibitedExceptSearchEngineIndexing),
            5 => Some(crate::types::DmiValue::Prohibited),
            6 => Some(crate::types::DmiValue::ProhibitedSeeConstraints),
            _ => None,
        };

        Some(StegoPayload {
            protection_level,
            seed: extracted_seed,
            intensity,
            version: 2,
            content_hash,
            dmi_value,
            raw_payload: None,
        })
    }

    pub(crate) fn try_ecc_decode(payload: &[u8]) -> Option<Vec<u8>> {
        // Try v2 data length first (32 bytes), then v1 (24 bytes)
        for &data_len in &[V2_HEADER_SIZE, 24usize] {
            let ecc_len = data_len * ecc::REPLICATION_FACTOR;
            if payload.len() >= ecc_len {
                if let Some(decoded) = ecc::ecc_decode(payload, data_len) {
                    if decoded.len() >= data_len {
                        let checksum_start = ecc_len;
                        let expected = Self::compute_checksum(&payload[..ecc_len]);
                        if payload[checksum_start] == expected[0]
                            && payload[checksum_start + 1] == expected[1]
                            && payload[checksum_start + 2] == expected[2]
                            && payload[checksum_start + 3] == expected[3]
                        {
                            return Some(decoded);
                        }
                    }
                }
            }
        }
        None
    }
}
