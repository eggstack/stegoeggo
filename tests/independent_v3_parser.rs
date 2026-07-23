//! Independent v3 parser test fixture.
//!
//! This test constructs known v3 payloads using the public writer API and
//! validates they parse correctly through the independent parser. It serves
//! as a cross-check that writer and parser agree on the wire format.

use stegoeggo::payload_v3::{
    parse_payload, AuthAlgorithm, ExtensionEntry, ParsedPayload, PayloadBuilder, PayloadFlags,
    ProtectionChannels, V3_CORE_SIZE, V3_MAGIC, V3_PAYLOAD_VERSION,
};

// ============================================================================
// Known-answer vectors: minimal v3 payload
// ============================================================================

#[test]
fn test_independent_minimal_v3_roundtrip() {
    let payload = PayloadBuilder::new()
        .seed(0xDEAD_BEEF_CAFE_BABE)
        .intensity(7500)
        .dmi_policy(3)
        .channels(ProtectionChannels {
            rights_metadata: true,
            hidden_marker: false,
            authentication: false,
        })
        .build()
        .expect("build minimal v3 payload");

    assert_eq!(payload.len(), V3_CORE_SIZE);
    assert_eq!(&payload[0..2], &V3_MAGIC);
    assert_eq!(payload[2], V3_PAYLOAD_VERSION);

    let parsed = parse_payload(&payload).expect("parse minimal v3 payload");
    match parsed {
        ParsedPayload::V3(v3) => {
            assert_eq!(v3.header.seed, 0xDEAD_BEEF_CAFE_BABE);
            assert_eq!(v3.header.intensity, 7500);
            assert_eq!(v3.header.dmi_policy, 3);
            assert!(v3.key_id.is_empty());
            assert!(v3.extensions.is_empty());
        }
        _ => panic!("Expected V3 payload"),
    }
}

// ============================================================================
// Known-answer vectors: v3 with key ID
// ============================================================================

#[test]
fn test_independent_v3_with_key_id() {
    let key_id = vec![0x42; 16];
    let payload = PayloadBuilder::new()
        .seed(12345)
        .intensity(5000)
        .key_id(key_id.clone())
        .channels(ProtectionChannels {
            rights_metadata: true,
            hidden_marker: true,
            authentication: true,
        })
        .build()
        .expect("build v3 with key_id");

    assert_eq!(payload.len(), V3_CORE_SIZE + 16);

    let parsed = parse_payload(&payload).expect("parse v3 with key_id");
    match parsed {
        ParsedPayload::V3(v3) => {
            assert_eq!(v3.key_id, key_id);
            assert_eq!(v3.header.key_id_len, 16);
            assert_eq!(v3.header.seed, 12345);
            let flags = PayloadFlags::from_bits(v3.header.flags);
            assert!(flags.has_key_id);
        }
        _ => panic!("Expected V3 payload"),
    }
}

// ============================================================================
// Known-answer vectors: v3 with extensions
// ============================================================================

#[test]
fn test_independent_v3_with_extensions() {
    let ext = ExtensionEntry {
        extension_type: 0x0001,
        critical: false,
        data: vec![0xAA, 0xBB, 0xCC, 0xDD],
    };

    let payload = PayloadBuilder::new()
        .seed(99)
        .extension(ext.clone())
        .build()
        .expect("build v3 with extension");

    let ext_size = 4 + ext.data.len();
    assert_eq!(payload.len(), V3_CORE_SIZE + ext_size);

    let parsed = parse_payload(&payload).expect("parse v3 with extension");
    match parsed {
        ParsedPayload::V3(v3) => {
            assert_eq!(v3.extensions.len(), 1);
            assert_eq!(v3.extensions[0].extension_type, 0x0001);
            assert_eq!(v3.extensions[0].data, ext.data);
            let flags = PayloadFlags::from_bits(v3.header.flags);
            assert!(flags.has_extensions);
        }
        _ => panic!("Expected V3 payload"),
    }
}

// ============================================================================
// Known-answer vectors: v3 with CRC32 auth
// ============================================================================

#[test]
fn test_independent_v3_with_crc32() {
    let payload = PayloadBuilder::new()
        .seed(42)
        .auth_algorithm(AuthAlgorithm::Crc32)
        .auth_tag(vec![0x12, 0x34, 0x56, 0x78])
        .build()
        .expect("build v3 with CRC32");

    assert_eq!(payload.len(), V3_CORE_SIZE + 4);

    let parsed = parse_payload(&payload).expect("parse v3 with CRC32");
    match parsed {
        ParsedPayload::V3(v3) => {
            assert_eq!(v3.header.auth_algorithm, AuthAlgorithm::Crc32 as u8);
            assert_eq!(v3.header.auth_tag_len, 4);
        }
        _ => panic!("Expected V3 payload"),
    }
}

// ============================================================================
// Known-answer vectors: v3 with HMAC-SHA256 (128-bit tag)
// ============================================================================

#[test]
fn test_independent_v3_with_hmac_128bit() {
    let hmac_tag = vec![0xAB; 16];
    let payload = PayloadBuilder::new()
        .seed(0xCAFEBABE)
        .intensity(3000)
        .dmi_policy(1)
        .key_id(vec![0x01; 8])
        .auth_algorithm(AuthAlgorithm::HmacSha256Truncated)
        .auth_tag(hmac_tag.clone())
        .build()
        .expect("build v3 with HMAC");

    assert_eq!(payload.len(), V3_CORE_SIZE + 8 + 16);

    let parsed = parse_payload(&payload).expect("parse v3 with HMAC");
    match parsed {
        ParsedPayload::V3(v3) => {
            assert_eq!(
                v3.header.auth_algorithm,
                AuthAlgorithm::HmacSha256Truncated as u8
            );
            assert_eq!(v3.header.auth_tag_len, 16);
            assert_eq!(v3.key_id, vec![0x01; 8]);
        }
        _ => panic!("Expected V3 payload"),
    }
}

// ============================================================================
// Known-answer vectors: v3 with tiled and progressive flags
// ============================================================================

#[test]
fn test_independent_v3_flags_tiled_and_progressive() {
    let payload = PayloadBuilder::new()
        .seed(100)
        .tiled(true)
        .progressive_jpeg(true)
        .build()
        .expect("build v3 with flags");

    let parsed = parse_payload(&payload).expect("parse v3 with flags");
    match parsed {
        ParsedPayload::V3(v3) => {
            let flags = PayloadFlags::from_bits(v3.header.flags);
            assert!(flags.tiled);
            assert!(flags.progressive_jpeg);
        }
        _ => panic!("Expected V3 payload"),
    }
}

// ============================================================================
// Known-answer vectors: full-featured v3 payload
// ============================================================================

#[test]
fn test_independent_v3_full_featured() {
    let payload = PayloadBuilder::new()
        .seed(0x0102_0304_0506_0708)
        .intensity(8500)
        .dmi_policy(4)
        .channels(ProtectionChannels {
            rights_metadata: true,
            hidden_marker: true,
            authentication: true,
        })
        .key_id(vec![0xFE; 24])
        .content_hash([0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88])
        .auth_algorithm(AuthAlgorithm::HmacSha256Truncated)
        .auth_tag(vec![0xCD; 16])
        .tiled(true)
        .extension(ExtensionEntry {
            extension_type: 0x0005,
            critical: false,
            data: b"stegoeggo/0.2.2".to_vec(),
        })
        .build()
        .expect("build full-featured v3");

    let parsed = parse_payload(&payload).expect("parse full-featured v3");
    match parsed {
        ParsedPayload::V3(v3) => {
            assert_eq!(v3.header.seed, 0x0102_0304_0506_0708);
            assert_eq!(v3.header.intensity, 8500);
            assert_eq!(v3.header.dmi_policy, 4);
            assert_eq!(v3.key_id, vec![0xFE; 24]);
            assert_eq!(
                v3.header.content_hash,
                [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]
            );
            assert_eq!(v3.header.auth_tag_len, 16);
            assert_eq!(v3.extensions.len(), 1);
            assert_eq!(v3.extensions[0].data, b"stegoeggo/0.2.2");
            let flags = PayloadFlags::from_bits(v3.header.flags);
            assert!(flags.tiled);
            assert!(flags.has_key_id);
            assert!(flags.has_extensions);
        }
        _ => panic!("Expected V3 payload"),
    }
}

// ============================================================================
// Negative tests: writer rejects invalid inputs
// ============================================================================

#[test]
fn test_independent_v3_writer_rejects_mismatched_hmac_tag_length() {
    let result = PayloadBuilder::new()
        .auth_algorithm(AuthAlgorithm::HmacSha256Truncated)
        .auth_tag(vec![0u8; 8]) // wrong: expects 16 bytes
        .build();
    assert!(result.is_err());
}

#[test]
fn test_independent_v3_writer_rejects_mismatched_ed25519_tag_length() {
    let result = PayloadBuilder::new()
        .auth_algorithm(AuthAlgorithm::Ed25519)
        .auth_tag(vec![0u8; 32]) // wrong: expects 64 bytes
        .build();
    assert!(result.is_err());
}

#[test]
fn test_independent_v3_writer_rejects_extension_too_large() {
    let result = PayloadBuilder::new()
        .extension(ExtensionEntry {
            extension_type: 0x0001,
            critical: false,
            data: vec![0u8; 129], // exceeds V3_MAX_EXTENSION_SIZE (128)
        })
        .build();
    assert!(result.is_err());
}

// ============================================================================
// Cross-check: parse_payload dispatches correctly across versions
// ============================================================================

#[test]
fn test_independent_version_dispatch_v1() {
    let mut data = vec![0u8; 24];
    data[0] = 1; // v1 marker
    let parsed = parse_payload(&data).expect("parse v1");
    assert!(matches!(parsed, ParsedPayload::V1(_)));
}

#[test]
fn test_independent_version_dispatch_v2() {
    let mut data = vec![0u8; 32];
    data[0] = 2; // v2 marker
    let parsed = parse_payload(&data).expect("parse v2");
    assert!(matches!(parsed, ParsedPayload::V2(_)));
}

#[test]
fn test_independent_version_dispatch_v3() {
    let payload = PayloadBuilder::new().seed(1).build().unwrap();
    let parsed = parse_payload(&payload).expect("parse v3");
    assert!(matches!(parsed, ParsedPayload::V3(_)));
}
