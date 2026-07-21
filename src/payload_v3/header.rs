use crate::payload_v3::errors::PayloadV3ParseError;
use crate::payload_v3::types::{
    AuthAlgorithm, V3_CORE_SIZE, V3_MAGIC, V3_MAX_EMBEDDED_SIZE, V3_MAX_KEY_ID_LEN,
    V3_PAYLOAD_VERSION,
};

/// V3 payload header with parsed fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadV3Header {
    /// Magic bytes (`"SE"`).
    pub magic: [u8; 2],
    /// Payload format version.
    pub version: u8,
    /// Total header length in bytes (core + key ID + extensions).
    pub header_length: u8,
    /// Total embedded payload length in bytes.
    pub total_length: u16,
    /// Header flags bitfield.
    pub flags: u16,
    /// Protection channels bitfield.
    pub channels: u16,
    /// Data-mining policy discriminant.
    pub dmi_policy: u8,
    /// PRNG seed for steganographic embedding.
    pub seed: u64,
    /// Embedding intensity (0–10000, where 10000 = 100.0%).
    pub intensity: u16,
    /// Content hash truncated to 8 bytes.
    pub content_hash: [u8; 8],
    /// Authentication algorithm byte.
    pub auth_algorithm: u8,
    /// Authentication tag length in bytes.
    pub auth_tag_len: u8,
    /// Key identifier length in bytes.
    pub key_id_len: u8,
}

impl PayloadV3Header {
    /// Total core header size including the key ID.
    #[must_use]
    pub fn total_core_size(&self) -> usize {
        V3_CORE_SIZE + self.key_id_len as usize
    }

    /// Parse the auth algorithm byte into an [`AuthAlgorithm`] enum.
    #[must_use]
    pub fn auth_algorithm_enum(&self) -> Option<AuthAlgorithm> {
        AuthAlgorithm::from_byte(self.auth_algorithm)
    }

    /// Convert the stored intensity to a 0.0–100.0 float.
    #[must_use]
    pub fn intensity_f32(&self) -> f32 {
        self.intensity as f32 / 100.0
    }

    /// Serialize the core header to bytes (32 bytes).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(V3_CORE_SIZE);
        buf.extend_from_slice(&self.magic);
        buf.push(self.version);
        buf.push(self.header_length);
        buf.extend_from_slice(&self.total_length.to_le_bytes());
        buf.extend_from_slice(&self.flags.to_le_bytes());
        buf.extend_from_slice(&self.channels.to_le_bytes());
        buf.push(self.dmi_policy);
        buf.extend_from_slice(&self.seed.to_le_bytes());
        buf.extend_from_slice(&self.intensity.to_le_bytes());
        buf.extend_from_slice(&self.content_hash);
        buf.push(self.auth_algorithm);
        buf.push(self.auth_tag_len);
        buf.push(self.key_id_len);
        debug_assert_eq!(buf.len(), V3_CORE_SIZE);
        buf
    }

    /// Deserialize a header from a byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Self, PayloadV3ParseError> {
        if data.len() < V3_CORE_SIZE {
            return Err(PayloadV3ParseError::TooShort {
                min: V3_CORE_SIZE,
                actual: data.len(),
            });
        }

        let magic = [data[0], data[1]];
        if magic != V3_MAGIC {
            return Err(PayloadV3ParseError::InvalidMagic(magic));
        }

        let version = data[2];
        if version != V3_PAYLOAD_VERSION {
            return Err(PayloadV3ParseError::UnsupportedVersion(version));
        }

        let header_length = data[3];
        let total_length = u16::from_le_bytes([data[4], data[5]]);
        let flags = u16::from_le_bytes([data[6], data[7]]);
        let channels = u16::from_le_bytes([data[8], data[9]]);
        let dmi_policy = data[10];

        if dmi_policy > 6 {
            return Err(PayloadV3ParseError::InvalidDmiPolicy(dmi_policy));
        }

        let seed = u64::from_le_bytes([
            data[11], data[12], data[13], data[14], data[15], data[16], data[17], data[18],
        ]);

        let intensity = u16::from_le_bytes([data[19], data[20]]);

        let mut content_hash = [0u8; 8];
        content_hash.copy_from_slice(&data[21..29]);

        let auth_algorithm = data[29];
        if AuthAlgorithm::from_byte(auth_algorithm).is_none() {
            return Err(PayloadV3ParseError::InvalidAuthAlgorithm(auth_algorithm));
        }

        let auth_tag_len = data[30];
        let key_id_len = data[31];

        if key_id_len as usize > V3_MAX_KEY_ID_LEN {
            return Err(PayloadV3ParseError::KeyIdTooLong {
                key_id_len: key_id_len as usize,
                max: V3_MAX_KEY_ID_LEN,
            });
        }

        let total_core = V3_CORE_SIZE + key_id_len as usize;
        if (header_length as usize) < total_core {
            return Err(PayloadV3ParseError::HeaderExceedsTotal {
                header: header_length as usize,
                total: total_core,
            });
        }

        if total_length as usize > V3_MAX_EMBEDDED_SIZE {
            return Err(PayloadV3ParseError::Oversized {
                size: total_length as usize,
                max: V3_MAX_EMBEDDED_SIZE,
            });
        }

        Ok(Self {
            magic,
            version,
            header_length,
            total_length,
            flags,
            channels,
            dmi_policy,
            seed,
            intensity,
            content_hash,
            auth_algorithm,
            auth_tag_len,
            key_id_len,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_header() -> PayloadV3Header {
        PayloadV3Header {
            magic: V3_MAGIC,
            version: V3_PAYLOAD_VERSION,
            header_length: V3_CORE_SIZE as u8,
            total_length: V3_CORE_SIZE as u16,
            flags: 0,
            channels: 0x0003,
            dmi_policy: 2,
            seed: 0x0102030405060708,
            intensity: 5000,
            content_hash: [0xAA; 8],
            auth_algorithm: 0,
            auth_tag_len: 0,
            key_id_len: 0,
        }
    }

    #[test]
    fn test_header_roundtrip() {
        let header = make_test_header();
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), V3_CORE_SIZE);

        let parsed = PayloadV3Header::from_bytes(&bytes).unwrap();
        assert_eq!(header, parsed);
    }

    #[test]
    fn test_header_with_key_id() {
        let mut header = make_test_header();
        header.key_id_len = 16;
        header.flags = 0x0002;
        header.header_length = (V3_CORE_SIZE + 16) as u8;
        header.total_length = (V3_CORE_SIZE + 16) as u16;

        let mut bytes = header.to_bytes();
        bytes.extend_from_slice(&[0xBB; 16]);

        let parsed = PayloadV3Header::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.key_id_len, 16);
    }

    #[test]
    fn test_header_invalid_magic() {
        let mut bytes = make_test_header().to_bytes();
        bytes[0] = 0xFF;
        assert!(matches!(
            PayloadV3Header::from_bytes(&bytes),
            Err(PayloadV3ParseError::InvalidMagic([0xFF, 0x45]))
        ));
    }

    #[test]
    fn test_header_unsupported_version() {
        let mut bytes = make_test_header().to_bytes();
        bytes[2] = 1;
        assert!(matches!(
            PayloadV3Header::from_bytes(&bytes),
            Err(PayloadV3ParseError::UnsupportedVersion(1))
        ));
    }

    #[test]
    fn test_header_too_short() {
        assert!(matches!(
            PayloadV3Header::from_bytes(&[0u8; 16]),
            Err(PayloadV3ParseError::TooShort {
                min: 32,
                actual: 16
            })
        ));
    }

    #[test]
    fn test_header_key_id_too_long() {
        let mut header = make_test_header();
        header.key_id_len = 33;
        let bytes = header.to_bytes();
        assert!(matches!(
            PayloadV3Header::from_bytes(&bytes),
            Err(PayloadV3ParseError::KeyIdTooLong {
                key_id_len: 33,
                max: 32
            })
        ));
    }

    #[test]
    fn test_header_intensity_f32() {
        let mut header = make_test_header();
        header.intensity = 5000;
        assert!((header.intensity_f32() - 50.0).abs() < f32::EPSILON);
    }
}
