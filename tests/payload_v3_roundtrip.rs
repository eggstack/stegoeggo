use stegoeggo::payload_v3::{
    parse_payload, AuthAlgorithm, ParsedPayload, PayloadV3Header, V3_CORE_SIZE, V3_DOMAIN_STRING,
    V3_MAGIC, V3_MAX_EMBEDDED_SIZE, V3_PAYLOAD_VERSION,
};

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
fn test_v3_header_serialize_deserialize_roundtrip() {
    let header = make_test_header();
    let bytes = header.to_bytes();
    assert_eq!(bytes.len(), V3_CORE_SIZE);
    assert_eq!(&bytes[..2], V3_MAGIC);
    assert_eq!(bytes[2], V3_PAYLOAD_VERSION);

    let parsed = PayloadV3Header::from_bytes(&bytes).unwrap();
    assert_eq!(header.magic, parsed.magic);
    assert_eq!(header.version, parsed.version);
    assert_eq!(header.header_length, parsed.header_length);
    assert_eq!(header.total_length, parsed.total_length);
    assert_eq!(header.flags, parsed.flags);
    assert_eq!(header.channels, parsed.channels);
    assert_eq!(header.dmi_policy, parsed.dmi_policy);
    assert_eq!(header.seed, parsed.seed);
    assert_eq!(header.intensity, parsed.intensity);
    assert_eq!(header.content_hash, parsed.content_hash);
    assert_eq!(header.auth_algorithm, parsed.auth_algorithm);
    assert_eq!(header.auth_tag_len, parsed.auth_tag_len);
    assert_eq!(header.key_id_len, parsed.key_id_len);
}

#[test]
fn test_v3_parser_detects_v1() {
    let mut data = vec![0u8; 32];
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
        _ => panic!("Expected V1 payload"),
    }
}

#[test]
fn test_v3_parser_detects_v2() {
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
        _ => panic!("Expected V2 payload"),
    }
}

#[test]
fn test_v3_parser_detects_v3() {
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
        _ => panic!("Expected V3 payload"),
    }
}

#[test]
fn test_v3_header_valid_ranges() {
    let mut header = make_test_header();

    header.intensity = 0;
    assert_eq!(header.intensity_f32(), 0.0);

    header.intensity = 10000;
    assert_eq!(header.intensity_f32(), 100.0);

    header.dmi_policy = 0;
    let bytes = header.to_bytes();
    assert!(PayloadV3Header::from_bytes(&bytes).is_ok());

    header.dmi_policy = 6;
    let bytes = header.to_bytes();
    assert!(PayloadV3Header::from_bytes(&bytes).is_ok());

    header.auth_algorithm = AuthAlgorithm::None as u8;
    let bytes = header.to_bytes();
    assert!(PayloadV3Header::from_bytes(&bytes).is_ok());

    header.auth_algorithm = AuthAlgorithm::Ed25519 as u8;
    let bytes = header.to_bytes();
    assert!(PayloadV3Header::from_bytes(&bytes).is_ok());
}

#[test]
fn test_v3_header_rejects_invalid_magic() {
    let mut header = make_test_header();
    header.magic = [0xFF, 0xFF];
    let bytes = header.to_bytes();
    assert!(PayloadV3Header::from_bytes(&bytes).is_err());
}

#[test]
fn test_v3_header_rejects_oversized() {
    let mut header = make_test_header();
    header.total_length = V3_MAX_EMBEDDED_SIZE as u16 + 1;
    let bytes = header.to_bytes();
    assert!(PayloadV3Header::from_bytes(&bytes).is_err());
}

#[test]
fn test_v3_extension_roundtrip() {
    let mut header = make_test_header();
    let ext_data = b"test extension data";
    let ext_type: u16 = 0x0001;
    let ext_len = ext_data.len() as u16;
    let ext_section_len = 4 + ext_data.len();

    header.flags = 0x0001;
    header.header_length = (V3_CORE_SIZE + ext_section_len) as u8;
    header.total_length = (V3_CORE_SIZE + ext_section_len) as u16;

    let mut bytes = header.to_bytes();
    bytes.extend_from_slice(&ext_type.to_le_bytes());
    bytes.extend_from_slice(&ext_len.to_le_bytes());
    bytes.extend_from_slice(ext_data);

    let parsed = parse_payload(&bytes).unwrap();
    match parsed {
        ParsedPayload::V3(v3) => {
            assert_eq!(v3.extensions.len(), 1);
            assert_eq!(v3.extensions[0].extension_type, 0x0001);
            assert_eq!(v3.extensions[0].data, ext_data);
        }
        _ => panic!("Expected V3 payload"),
    }
}

#[test]
fn test_v3_auth_algorithm_coverage() {
    let variants = [
        (AuthAlgorithm::None, 0u8),
        (AuthAlgorithm::Crc32, 1),
        (AuthAlgorithm::HmacSha256Truncated, 2),
        (AuthAlgorithm::Ed25519, 3),
    ];

    for (expected, byte_val) in &variants {
        let algo = AuthAlgorithm::from_byte(*byte_val).unwrap();
        assert_eq!(algo, *expected);
        assert_eq!(algo as u8, *byte_val);
    }

    assert!(AuthAlgorithm::from_byte(4).is_none());
    assert!(AuthAlgorithm::from_byte(255).is_none());

    assert_eq!(AuthAlgorithm::None.tag_length(), Some(0));
    assert_eq!(AuthAlgorithm::Crc32.tag_length(), Some(4));
    assert_eq!(AuthAlgorithm::HmacSha256Truncated.tag_length(), Some(8));
    assert_eq!(AuthAlgorithm::Ed25519.tag_length(), Some(64));
}

#[test]
fn test_v3_domain_string() {
    assert_eq!(V3_DOMAIN_STRING, b"StegoEggo-v3");
}

#[test]
fn test_v3_header_with_key_id() {
    let mut header = make_test_header();
    let key_id = vec![0xBB; 16];
    header.key_id_len = 16;
    header.flags = 0x0002;
    header.header_length = (V3_CORE_SIZE + 16) as u8;
    header.total_length = (V3_CORE_SIZE + 16) as u16;

    let mut bytes = header.to_bytes();
    bytes.extend_from_slice(&key_id);

    let parsed = PayloadV3Header::from_bytes(&bytes).unwrap();
    assert_eq!(parsed.key_id_len, 16);
    assert_eq!(parsed.total_core_size(), V3_CORE_SIZE + 16);
}

#[test]
fn test_v3_parse_empty_payload() {
    assert!(parse_payload(&[]).is_err());
}

#[test]
fn test_v3_parse_unknown_version() {
    let data = vec![99u8; 32];
    assert!(parse_payload(&data).is_err());
}
