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
    assert_eq!(AuthAlgorithm::HmacSha256Truncated.tag_length(), Some(16));
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

// ============================================================================
// Negative tests: Truncated payloads
// ============================================================================

#[test]
fn test_v3_truncated_header() {
    let mut data = vec![0u8; 16];
    data[0] = V3_MAGIC[0];
    data[1] = V3_MAGIC[1];
    data[2] = V3_PAYLOAD_VERSION;
    assert!(parse_payload(&data).is_err());
}

#[test]
fn test_v3_truncated_after_magic() {
    let data = vec![V3_MAGIC[0], V3_MAGIC[1], V3_PAYLOAD_VERSION];
    assert!(parse_payload(&data).is_err());
}

#[test]
fn test_v2_truncated() {
    let data = vec![2u8; 16];
    assert!(parse_payload(&data).is_err());
}

#[test]
fn test_v1_truncated() {
    let data = vec![1u8; 10];
    assert!(parse_payload(&data).is_err());
}

// ============================================================================
// Negative tests: Invalid magic bytes
// ============================================================================

#[test]
fn test_v3_invalid_magic_byte_0() {
    let mut data = vec![0u8; V3_CORE_SIZE];
    data[0] = 0x00;
    data[1] = 0x45;
    data[2] = V3_PAYLOAD_VERSION;
    data[3] = V3_CORE_SIZE as u8;
    assert!(parse_payload(&data).is_err());
}

#[test]
fn test_v3_invalid_magic_byte_1() {
    let mut data = vec![0u8; V3_CORE_SIZE];
    data[0] = 0x53;
    data[1] = 0x00;
    data[2] = V3_PAYLOAD_VERSION;
    data[3] = V3_CORE_SIZE as u8;
    assert!(parse_payload(&data).is_err());
}

// ============================================================================
// Negative tests: Invalid DMI policy
// ============================================================================

#[test]
fn test_v3_invalid_dmi_policy() {
    let mut header = make_test_header();
    header.dmi_policy = 7;
    let bytes = header.to_bytes();
    assert!(PayloadV3Header::from_bytes(&bytes).is_err());
}

// ============================================================================
// Negative tests: Invalid auth algorithm
// ============================================================================

#[test]
fn test_v3_invalid_auth_algorithm() {
    let mut header = make_test_header();
    header.auth_algorithm = 4;
    let bytes = header.to_bytes();
    assert!(PayloadV3Header::from_bytes(&bytes).is_err());
}

#[test]
fn test_v3_invalid_auth_algorithm_255() {
    let mut header = make_test_header();
    header.auth_algorithm = 255;
    let bytes = header.to_bytes();
    assert!(PayloadV3Header::from_bytes(&bytes).is_err());
}

// ============================================================================
// Negative tests: Key ID too long
// ============================================================================

#[test]
fn test_v3_key_id_too_long() {
    let mut header = make_test_header();
    header.key_id_len = 33;
    let bytes = header.to_bytes();
    assert!(PayloadV3Header::from_bytes(&bytes).is_err());
}

// ============================================================================
// Negative tests: Single byte payloads
// ============================================================================

#[test]
fn test_single_byte_zero() {
    assert!(parse_payload(&[0x00]).is_err());
}

#[test]
fn test_single_byte_one() {
    assert!(parse_payload(&[0x01]).is_err());
}

#[test]
fn test_single_byte_two() {
    assert!(parse_payload(&[0x02]).is_err());
}

// ============================================================================
// Negative tests: Extension parsing
// ============================================================================

#[test]
fn test_v3_duplicate_extension_rejected() {
    let mut header = make_test_header();
    let ext1_data = b"first";
    let ext2_data = b"second";
    let ext_section_len = (4 + ext1_data.len()) + (4 + ext2_data.len());

    header.flags = 0x0001;
    header.header_length = (V3_CORE_SIZE + ext_section_len) as u8;
    header.total_length = (V3_CORE_SIZE + ext_section_len) as u16;

    let mut bytes = header.to_bytes();
    // Extension 1: type 0x0001
    bytes.extend_from_slice(&0x0001u16.to_le_bytes());
    bytes.extend_from_slice(&(ext1_data.len() as u16).to_le_bytes());
    bytes.extend_from_slice(ext1_data);
    // Extension 2: type 0x0001 (duplicate)
    bytes.extend_from_slice(&0x0001u16.to_le_bytes());
    bytes.extend_from_slice(&(ext2_data.len() as u16).to_le_bytes());
    bytes.extend_from_slice(ext2_data);

    let result = parse_payload(&bytes);
    assert!(result.is_err());
}

#[test]
fn test_v3_extension_truncated() {
    let mut header = make_test_header();
    header.flags = 0x0001;
    header.header_length = (V3_CORE_SIZE + 10) as u8;
    header.total_length = (V3_CORE_SIZE + 10) as u16;

    let mut bytes = header.to_bytes();
    // Extension type but no length or data
    bytes.extend_from_slice(&0x0001u16.to_le_bytes());
    bytes.extend_from_slice(&100u16.to_le_bytes());
    // Only 2 bytes of data instead of 100

    let result = parse_payload(&bytes);
    assert!(result.is_err());
}

// ============================================================================
// Compatibility matrix: v1 CRC payload
// ============================================================================

#[test]
fn test_v1_crc_payload_parsing() {
    let mut data = vec![0u8; 32];
    data[0] = 1; // version
    data[1] = 2; // protection_level = Standard
    data[2..10].copy_from_slice(&12345u64.to_le_bytes()); // seed
    data[10..12].copy_from_slice(&7500u16.to_le_bytes()); // intensity
    data[12..20].copy_from_slice(&99999u64.to_le_bytes()); // timestamp

    let parsed = parse_payload(&data).unwrap();
    match parsed {
        ParsedPayload::V1(v1) => {
            assert_eq!(v1.protection_level, 2);
            assert_eq!(v1.seed, 12345);
            assert_eq!(v1.intensity, 7500);
            assert_eq!(v1.timestamp, 99999);
        }
        _ => panic!("Expected V1 payload"),
    }
}

#[test]
fn test_v1_crc_payload_minimal_values() {
    let mut data = vec![0u8; 32];
    data[0] = 1;
    data[1] = 0; // Disabled
                 // seed = 0, intensity = 0, timestamp = 0

    let parsed = parse_payload(&data).unwrap();
    match parsed {
        ParsedPayload::V1(v1) => {
            assert_eq!(v1.protection_level, 0);
            assert_eq!(v1.seed, 0);
            assert_eq!(v1.intensity, 0);
            assert_eq!(v1.timestamp, 0);
        }
        _ => panic!("Expected V1 payload"),
    }
}

#[test]
fn test_v1_crc_payload_maximal_values() {
    let mut data = vec![0u8; 32];
    data[0] = 1;
    data[1] = 2; // Standard
    data[2..10].copy_from_slice(&u64::MAX.to_le_bytes());
    data[10..12].copy_from_slice(&u16::MAX.to_le_bytes());
    data[12..20].copy_from_slice(&u64::MAX.to_le_bytes());

    let parsed = parse_payload(&data).unwrap();
    match parsed {
        ParsedPayload::V1(v1) => {
            assert_eq!(v1.protection_level, 2);
            assert_eq!(v1.seed, u64::MAX);
            assert_eq!(v1.intensity, u16::MAX);
            assert_eq!(v1.timestamp, u64::MAX);
        }
        _ => panic!("Expected V1 payload"),
    }
}

// ============================================================================
// Compatibility matrix: v2 CRC payload
// ============================================================================

#[test]
fn test_v2_crc_payload_parsing() {
    let mut data = vec![0u8; 32];
    data[0] = 2; // version
    data[1] = 2; // protection_level = Standard
    data[2..10].copy_from_slice(&54321u64.to_le_bytes()); // seed
    data[10..12].copy_from_slice(&8000u16.to_le_bytes()); // intensity
    data[12..20].copy_from_slice(&11111u64.to_le_bytes()); // timestamp
    data[20..24].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]); // content_hash
    data[24] = 2; // dmi_value
    data[25] = 0; // flags (CRC, no HMAC)

    let parsed = parse_payload(&data).unwrap();
    match parsed {
        ParsedPayload::V2(v2) => {
            assert_eq!(v2.protection_level, 2);
            assert_eq!(v2.seed, 54321);
            assert_eq!(v2.intensity, 8000);
            assert_eq!(v2.timestamp, 11111);
            assert_eq!(v2.content_hash, [0xAA, 0xBB, 0xCC, 0xDD]);
            assert_eq!(v2.dmi_value, 2);
            assert_eq!(v2.flags, 0);
        }
        _ => panic!("Expected V2 payload"),
    }
}

#[test]
fn test_v2_hmac_payload_parsing() {
    let mut data = vec![0u8; 32];
    data[0] = 2; // version
    data[1] = 2; // protection_level = Standard
    data[2..10].copy_from_slice(&77777u64.to_le_bytes()); // seed
    data[10..12].copy_from_slice(&9000u16.to_le_bytes()); // intensity
    data[12..20].copy_from_slice(&22222u64.to_le_bytes()); // timestamp
    data[20..24].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]); // content_hash
    data[24] = 4; // dmi_value
    data[25] = 1; // flags (HMAC bit set)

    let parsed = parse_payload(&data).unwrap();
    match parsed {
        ParsedPayload::V2(v2) => {
            assert_eq!(v2.protection_level, 2);
            assert_eq!(v2.seed, 77777);
            assert_eq!(v2.intensity, 9000);
            assert_eq!(v2.content_hash, [0x11, 0x22, 0x33, 0x44]);
            assert_eq!(v2.dmi_value, 4);
            assert_eq!(v2.flags, 1);
        }
        _ => panic!("Expected V2 payload"),
    }
}

#[test]
fn test_v2_dmi_value_range() {
    for dmi in 0..=6u8 {
        let mut data = vec![0u8; 32];
        data[0] = 2;
        data[24] = dmi;

        let parsed = parse_payload(&data).unwrap();
        match parsed {
            ParsedPayload::V2(v2) => {
                assert_eq!(v2.dmi_value, dmi);
            }
            _ => panic!("Expected V2 payload for dmi={}", dmi),
        }
    }
}

// ============================================================================
// Compatibility matrix: v3 with Ed25519 signature
// ============================================================================

#[cfg(feature = "signatures")]
#[test]
fn test_v3_ed25519_signature_payload_roundtrip() {
    use stegoeggo::payload_v3::writer::PayloadBuilder;
    use stegoeggo::signing::SigningKey;

    let sk = SigningKey::from_bytes([0xABu8; 32], b"compat-test-key".to_vec());
    let vk = sk.verifying_key();
    let claim = b"compatibility matrix claim data";

    let payload = PayloadBuilder::new()
        .seed(88888)
        .intensity(6000)
        .dmi_policy(5)
        .channels(stegoeggo::payload_v3::ProtectionChannels {
            rights_metadata: true,
            hidden_marker: true,
            authentication: true,
        })
        .embed_signature(&sk, claim)
        .build()
        .unwrap();

    assert!(payload.len() <= V3_MAX_EMBEDDED_SIZE);
    assert_eq!(&payload[0..2], V3_MAGIC);
    assert_eq!(payload[2], V3_PAYLOAD_VERSION);

    let parsed = parse_payload(&payload).unwrap();
    match parsed {
        ParsedPayload::V3(v3) => {
            assert_eq!(v3.header.seed, 88888);
            assert_eq!(v3.header.intensity, 6000);
            assert_eq!(v3.header.auth_algorithm, AuthAlgorithm::Ed25519 as u8);
            assert!(v3.header.flags & 0x0200 != 0); // SIGNED flag
            assert_eq!(v3.key_id, b"compat-test-key");

            let sig_ext = v3.extensions.iter().find(|e| e.extension_type == 0x0011);
            assert!(sig_ext.is_some());
            let sig_data = &sig_ext.unwrap().data;
            assert_eq!(sig_data.len(), 64);

            let result = vk.verify(claim, sig_data);
            assert_eq!(result, stegoeggo::signing::SignatureResult::Valid);
        }
        _ => panic!("Expected V3 payload"),
    }
}

// ============================================================================
// Compatibility matrix: v3 with HMAC-128
// ============================================================================

#[test]
fn test_v3_hmac128_payload_roundtrip() {
    use stegoeggo::payload_v3::writer::PayloadBuilder;

    let payload = PayloadBuilder::new()
        .seed(55555)
        .intensity(4000)
        .dmi_policy(1)
        .key_id(b"hmac-key-id".to_vec())
        .auth_algorithm(AuthAlgorithm::HmacSha256Truncated)
        .auth_tag(vec![0xAA; 16])
        .build()
        .unwrap();

    assert!(payload.len() <= V3_MAX_EMBEDDED_SIZE);

    let parsed = parse_payload(&payload).unwrap();
    match parsed {
        ParsedPayload::V3(v3) => {
            assert_eq!(v3.header.seed, 55555);
            assert_eq!(v3.header.intensity, 4000);
            assert_eq!(
                v3.header.auth_algorithm,
                AuthAlgorithm::HmacSha256Truncated as u8
            );
            assert_eq!(v3.header.auth_tag_len, 16);
            assert_eq!(v3.key_id, b"hmac-key-id");
        }
        _ => panic!("Expected V3 payload"),
    }
}

// ============================================================================
// Compatibility matrix: version dispatch boundary tests
// ============================================================================

#[test]
fn test_version_dispatch_boundary_zero() {
    let data = vec![0u8; 32];
    let result = parse_payload(&data);
    assert!(result.is_err());
}

#[test]
fn test_version_dispatch_boundary_three() {
    let mut data = vec![0u8; V3_CORE_SIZE];
    data[0] = V3_MAGIC[0];
    data[1] = V3_MAGIC[1];
    data[2] = 3;
    data[3] = V3_CORE_SIZE as u8;
    data[4..6].copy_from_slice(&(V3_CORE_SIZE as u16).to_le_bytes());

    let parsed = parse_payload(&data).unwrap();
    match parsed {
        ParsedPayload::V3(v3) => {
            assert_eq!(v3.header.version, 3);
        }
        _ => panic!("Expected V3 payload"),
    }
}

#[test]
fn test_version_dispatch_boundary_four_unknown() {
    let mut data = vec![0u8; 32];
    data[0] = 4; // unknown version

    let result = parse_payload(&data);
    assert!(result.is_err());
}

#[test]
fn test_endianness_explicit_byte_order() {
    let header = make_test_header();
    let bytes = header.to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[4], bytes[5]]),
        header.total_length,
        "total_length must be little-endian"
    );
    assert_eq!(
        u16::from_le_bytes([bytes[6], bytes[7]]),
        header.flags,
        "flags must be little-endian"
    );
    assert_eq!(
        u16::from_le_bytes([bytes[8], bytes[9]]),
        header.channels,
        "channels must be little-endian"
    );
    assert_eq!(
        u64::from_le_bytes(bytes[11..19].try_into().unwrap()),
        header.seed,
        "seed must be little-endian"
    );
    assert_eq!(
        u16::from_le_bytes([bytes[19], bytes[20]]),
        header.intensity,
        "intensity must be little-endian"
    );
}
