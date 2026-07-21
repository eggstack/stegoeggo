use crate::payload_v3::errors::PayloadV3ParseError;
use crate::payload_v3::header::PayloadV3Header;
use crate::payload_v3::types::{
    ExtensionEntry, V3_CORE_SIZE, V3_MAGIC, V3_MAX_EXTENSION_COUNT, V3_MAX_EXTENSION_SIZE,
    V3_PAYLOAD_VERSION,
};

/// Parsed payload from any supported version (V1, V2, or V3).
#[derive(Debug)]
pub enum ParsedPayload {
    /// A V1 payload (24-byte legacy format).
    V1(V1Payload),
    /// A V2 payload (32-byte format with HMAC).
    V2(V2Payload),
    /// A V3 payload (TLV extensions with domain-separated authentication).
    V3(V3Payload),
}

/// V1 legacy payload (24-byte format).
#[derive(Debug)]
pub struct V1Payload {
    /// Protection level byte.
    pub protection_level: u8,
    /// PRNG seed for steganographic embedding.
    pub seed: u64,
    /// Embedding intensity.
    pub intensity: u16,
    /// Timestamp of processing.
    pub timestamp: u64,
}

/// V2 payload (32-byte format with HMAC and DMI policy).
#[derive(Debug)]
pub struct V2Payload {
    /// Protection level byte.
    pub protection_level: u8,
    /// PRNG seed for steganographic embedding.
    pub seed: u64,
    /// Embedding intensity.
    pub intensity: u16,
    /// Timestamp of processing.
    pub timestamp: u64,
    /// Content hash truncated to 4 bytes.
    pub content_hash: [u8; 4],
    /// Data-mining policy discriminant.
    pub dmi_value: u8,
    /// Payload flags byte.
    pub flags: u8,
}

/// V3 payload with TLV extensions.
#[derive(Debug)]
pub struct V3Payload {
    /// Parsed V3 header.
    pub header: PayloadV3Header,
    /// Key identifier bytes.
    pub key_id: Vec<u8>,
    /// Parsed TLV extension entries.
    pub extensions: Vec<ExtensionEntry>,
}

/// Parse a stego payload from raw bytes, auto-detecting the version.
pub fn parse_payload(data: &[u8]) -> Result<ParsedPayload, PayloadV3ParseError> {
    if data.len() >= 3
        && data[0] == V3_MAGIC[0]
        && data[1] == V3_MAGIC[1]
        && data[2] == V3_PAYLOAD_VERSION
    {
        return parse_v3(data);
    }

    if !data.is_empty() {
        match data[0] {
            2 => return parse_v2(data),
            1 => return parse_v1(data),
            _ => {}
        }
    }

    if data.len() >= 2 {
        return Err(PayloadV3ParseError::InvalidMagic([data[0], data[1]]));
    }
    Err(PayloadV3ParseError::TooShort {
        min: 1,
        actual: data.len(),
    })
}

fn parse_v1(data: &[u8]) -> Result<ParsedPayload, PayloadV3ParseError> {
    const V1_MIN_SIZE: usize = 24;
    if data.len() < V1_MIN_SIZE {
        return Err(PayloadV3ParseError::TooShort {
            min: V1_MIN_SIZE,
            actual: data.len(),
        });
    }

    if data[0] != 1 {
        return Err(PayloadV3ParseError::UnsupportedVersion(data[0]));
    }

    let protection_level = data[1];
    let seed = u64::from_le_bytes([
        data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
    ]);
    let intensity = u16::from_le_bytes([data[10], data[11]]);
    let timestamp = u64::from_le_bytes([
        data[12], data[13], data[14], data[15], data[16], data[17], data[18], data[19],
    ]);

    Ok(ParsedPayload::V1(V1Payload {
        protection_level,
        seed,
        intensity,
        timestamp,
    }))
}

fn parse_v2(data: &[u8]) -> Result<ParsedPayload, PayloadV3ParseError> {
    const V2_SIZE: usize = 32;
    if data.len() < V2_SIZE {
        return Err(PayloadV3ParseError::TooShort {
            min: V2_SIZE,
            actual: data.len(),
        });
    }

    if data[0] != 2 {
        return Err(PayloadV3ParseError::UnsupportedVersion(data[0]));
    }

    let protection_level = data[1];
    let seed = u64::from_le_bytes([
        data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
    ]);
    let intensity = u16::from_le_bytes([data[10], data[11]]);
    let timestamp = u64::from_le_bytes([
        data[12], data[13], data[14], data[15], data[16], data[17], data[18], data[19],
    ]);

    let mut content_hash = [0u8; 4];
    content_hash.copy_from_slice(&data[20..24]);

    let dmi_value = data[24];
    let flags = data[25];

    Ok(ParsedPayload::V2(V2Payload {
        protection_level,
        seed,
        intensity,
        timestamp,
        content_hash,
        dmi_value,
        flags,
    }))
}

fn parse_v3(data: &[u8]) -> Result<ParsedPayload, PayloadV3ParseError> {
    let header = PayloadV3Header::from_bytes(data)?;

    let key_id_start = V3_CORE_SIZE;
    let key_id_end = key_id_start + header.key_id_len as usize;
    if key_id_end > data.len() {
        return Err(PayloadV3ParseError::TooShort {
            min: key_id_end,
            actual: data.len(),
        });
    }
    let key_id = data[key_id_start..key_id_end].to_vec();

    let mut extensions = Vec::new();
    let ext_start = key_id_end;
    let ext_end = header.header_length as usize;

    if ext_end > data.len() {
        return Err(PayloadV3ParseError::TooShort {
            min: ext_end,
            actual: data.len(),
        });
    }

    if ext_end > ext_start {
        let ext_data = &data[ext_start..ext_end];
        extensions = parse_extensions(ext_data)?;
    }

    if header.total_length as usize > data.len() {
        return Err(PayloadV3ParseError::TooShort {
            min: header.total_length as usize,
            actual: data.len(),
        });
    }

    Ok(ParsedPayload::V3(V3Payload {
        header,
        key_id,
        extensions,
    }))
}

fn parse_extensions(data: &[u8]) -> Result<Vec<ExtensionEntry>, PayloadV3ParseError> {
    let mut extensions = Vec::new();
    let mut total_ext_size = 0usize;
    let mut seen_types = [false; 256];
    let mut offset = 0usize;

    while offset + 4 <= data.len() {
        let ext_type = u16::from_le_bytes([data[offset], data[offset + 1]]);
        let ext_len = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);

        if ext_type == 0xFFFF {
            break;
        }

        let ext_len = ext_len as usize;
        if offset + 4 + ext_len > data.len() {
            return Err(PayloadV3ParseError::ExtensionsTooLarge);
        }

        total_ext_size += 4 + ext_len;
        if total_ext_size > V3_MAX_EXTENSION_SIZE {
            return Err(PayloadV3ParseError::ExtensionsTooLarge);
        }

        if extensions.len() >= V3_MAX_EXTENSION_COUNT {
            break;
        }

        if ext_type < 0x0100 {
            let idx = ext_type as usize;
            if idx < seen_types.len() && seen_types[idx] {
                return Err(PayloadV3ParseError::DuplicateExtension(ext_type));
            }
            if idx < seen_types.len() {
                seen_types[idx] = true;
            }
        }

        let ext_data = data[offset + 4..offset + 4 + ext_len].to_vec();
        extensions.push(ExtensionEntry {
            extension_type: ext_type,
            critical: crate::payload_v3::types::ExtensionType::is_critical(ext_type),
            data: ext_data,
        });

        offset += 4 + ext_len;
    }

    Ok(extensions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload_v3::types::V3_MAGIC;

    #[test]
    fn test_parse_v1_payload() {
        let mut data = vec![1u8; 32];
        data[0] = 1;
        data[1] = 2;
        data[2..10].copy_from_slice(&42u64.to_le_bytes());
        data[10..12].copy_from_slice(&5000u16.to_le_bytes());
        data[12..20].copy_from_slice(&12345u64.to_le_bytes());

        let parsed = parse_payload(&data).unwrap();
        match parsed {
            ParsedPayload::V1(v1) => {
                assert_eq!(v1.protection_level, 2);
                assert_eq!(v1.seed, 42);
                assert_eq!(v1.intensity, 5000);
            }
            _ => panic!("Expected V1"),
        }
    }

    #[test]
    fn test_parse_v2_payload() {
        let mut data = vec![0u8; 32];
        data[0] = 2;
        data[1] = 2;
        data[2..10].copy_from_slice(&99u64.to_le_bytes());
        data[10..12].copy_from_slice(&7500u16.to_le_bytes());
        data[12..20].copy_from_slice(&67890u64.to_le_bytes());
        data[24] = 3;
        data[25] = 0;

        let parsed = parse_payload(&data).unwrap();
        match parsed {
            ParsedPayload::V2(v2) => {
                assert_eq!(v2.protection_level, 2);
                assert_eq!(v2.seed, 99);
                assert_eq!(v2.intensity, 7500);
                assert_eq!(v2.dmi_value, 3);
            }
            _ => panic!("Expected V2"),
        }
    }

    #[test]
    fn test_parse_v3_minimal() {
        let mut data = vec![0u8; V3_CORE_SIZE];
        data[0] = V3_MAGIC[0];
        data[1] = V3_MAGIC[1];
        data[2] = V3_PAYLOAD_VERSION;
        data[3] = V3_CORE_SIZE as u8;
        data[4..6].copy_from_slice(&(V3_CORE_SIZE as u16).to_le_bytes());
        data[11..19].copy_from_slice(&42u64.to_le_bytes());
        data[19..21].copy_from_slice(&5000u16.to_le_bytes());

        let parsed = parse_payload(&data).unwrap();
        match parsed {
            ParsedPayload::V3(v3) => {
                assert_eq!(v3.header.seed, 42);
                assert_eq!(v3.header.intensity, 5000);
                assert!(v3.extensions.is_empty());
                assert!(v3.key_id.is_empty());
            }
            _ => panic!("Expected V3"),
        }
    }

    #[test]
    fn test_parse_v3_with_key_id() {
        let key_id = vec![0xAA; 16];
        let total_size = V3_CORE_SIZE + 16;
        let mut data = vec![0u8; total_size];
        data[0] = V3_MAGIC[0];
        data[1] = V3_MAGIC[1];
        data[2] = V3_PAYLOAD_VERSION;
        data[3] = total_size as u8;
        data[4..6].copy_from_slice(&(total_size as u16).to_le_bytes());
        data[11..19].copy_from_slice(&42u64.to_le_bytes());
        data[19..21].copy_from_slice(&5000u16.to_le_bytes());
        data[31] = 16;
        data[V3_CORE_SIZE..].copy_from_slice(&key_id);

        let parsed = parse_payload(&data).unwrap();
        match parsed {
            ParsedPayload::V3(v3) => {
                assert_eq!(v3.key_id, vec![0xAA; 16]);
                assert_eq!(v3.header.key_id_len, 16);
            }
            _ => panic!("Expected V3"),
        }
    }

    #[test]
    fn test_parse_unknown_version() {
        let data = vec![99u8; 32];
        assert!(parse_payload(&data).is_err());
    }

    #[test]
    fn test_parse_empty() {
        assert!(parse_payload(&[]).is_err());
    }
}
