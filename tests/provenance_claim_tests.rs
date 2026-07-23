use stegoeggo::provenance::{canonical_json, verify_canonical_stability, ProvenanceClaim};

#[test]
fn test_claim_new_random_id() {
    let claim1 = ProvenanceClaim::new(0);
    let claim2 = ProvenanceClaim::new(0);

    assert_ne!(claim1.claim_id, claim2.claim_id);
}

#[test]
fn test_claim_builder() {
    let claim = ProvenanceClaim::new(2)
        .with_content_code("iscc:abcd1234".to_string())
        .with_creation_time(1700000000)
        .with_source_facts("png", 1920, 1080, 524288)
        .with_issuer_id("issuer123".to_string())
        .with_software("stegoeggo/0.2.2")
        .with_parent_claim("parent_claim_id".to_string())
        .with_statement_uri("https://example.com/license");

    assert_eq!(claim.schema_version, 1);
    assert_eq!(claim.rights_policy, 2);
    assert_eq!(claim.content_code, "iscc:abcd1234");
    assert_eq!(claim.created_at, 1700000000);
    assert_eq!(claim.format, "png");
    assert_eq!(claim.width, 1920);
    assert_eq!(claim.height, 1080);
    assert_eq!(claim.file_size, 524288);
    assert_eq!(claim.issuer_id, "issuer123");
    assert_eq!(claim.software, "stegoeggo/0.2.2");
    assert_eq!(claim.parent_claim_id, Some("parent_claim_id".to_string()));
    assert_eq!(
        claim.statement_uri,
        Some("https://example.com/license".to_string())
    );
}

#[test]
fn test_claim_canonical_bytes_stable() {
    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:test".to_string())
        .with_creation_time(1000)
        .with_source_facts("jpeg", 640, 480, 1024)
        .with_software("test/1.0");

    let bytes1 = claim.canonical_bytes();
    let bytes2 = claim.canonical_bytes();

    assert_eq!(bytes1, bytes2);
    assert!(verify_canonical_stability(&claim));
}

#[test]
fn test_claim_digest_deterministic() {
    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:deterministic".to_string())
        .with_creation_time(2000)
        .with_source_facts("png", 100, 100, 512)
        .with_software("test/1.0");

    let digest1 = claim.digest();
    let digest2 = claim.digest();

    assert_eq!(digest1, digest2);
}

#[test]
fn test_claim_with_rights_notice() {
    let claim = ProvenanceClaim::new(3).with_notice_digest(b"All Rights Reserved. No AI training.");

    assert!(claim.notice_digest.starts_with("sha256:"));
    assert_eq!(claim.notice_digest.len(), "sha256:".len() + 64);
}

#[test]
fn test_claim_with_instance_digest() {
    let file_bytes = b"fake image data for testing";
    let claim = ProvenanceClaim::new(0).with_instance_digest(file_bytes);

    assert!(claim.instance_digest.starts_with("sha256:"));
    assert_eq!(claim.instance_digest.len(), "sha256:".len() + 64);

    let same_claim = ProvenanceClaim::new(0).with_instance_digest(file_bytes);
    assert_eq!(claim.instance_digest, same_claim.instance_digest);
}

#[test]
fn test_claim_serialization_roundtrip() {
    let claim = ProvenanceClaim::new(1)
        .with_content_code("iscc:roundtrip".to_string())
        .with_creation_time(3000)
        .with_source_facts("webp", 800, 600, 2048)
        .with_software("stegoeggo/0.2.2")
        .with_issuer_id("test-issuer".to_string())
        .with_statement_uri("https://example.com/license");

    let json = serde_json::to_string(&claim).unwrap();
    let deserialized: ProvenanceClaim = serde_json::from_str(&json).unwrap();

    assert_eq!(claim.claim_id, deserialized.claim_id);
    assert_eq!(claim.content_code, deserialized.content_code);
    assert_eq!(claim.created_at, deserialized.created_at);
    assert_eq!(claim.file_size, deserialized.file_size);
    assert_eq!(claim.format, deserialized.format);
    assert_eq!(claim.width, deserialized.width);
    assert_eq!(claim.height, deserialized.height);
    assert_eq!(claim.rights_policy, deserialized.rights_policy);
    assert_eq!(claim.schema_version, deserialized.schema_version);
    assert_eq!(claim.software, deserialized.software);
    assert_eq!(claim.issuer_id, deserialized.issuer_id);
    assert_eq!(claim.statement_uri, deserialized.statement_uri);
}

#[test]
fn test_claim_default_optional_fields_none() {
    let claim = ProvenanceClaim::new(0);

    assert!(claim.parent_claim_id.is_none());
    assert!(claim.statement_uri.is_none());
    assert!(claim.notice_digest.is_empty());
    assert!(claim.content_code.is_empty());
    assert!(claim.instance_digest.is_empty());
}

#[test]
fn test_claim_with_notice_digest_raw() {
    let claim = ProvenanceClaim::new(0).with_notice_digest_raw("sha256:abc123".to_string());

    assert_eq!(claim.notice_digest, "sha256:abc123");
}

#[test]
fn test_claim_with_instance_digest_raw() {
    let claim = ProvenanceClaim::new(0).with_instance_digest_raw("sha256:def456".to_string());

    assert_eq!(claim.instance_digest, "sha256:def456");
}

#[test]
fn test_canonical_json_excludes_claim_id() {
    let claim = ProvenanceClaim::new(0)
        .with_content_code("iscc:test".to_string())
        .with_software("test/1.0");

    let canonical = canonical_json(&claim);
    let canonical_str = String::from_utf8(canonical).unwrap();

    assert!(!canonical_str.contains("claim_id"));
    assert!(canonical_str.contains("content_code"));
    assert!(canonical_str.contains("schema_version"));
}

#[test]
fn test_claim_digest_differs_for_different_claims() {
    let claim1 = ProvenanceClaim::new(0).with_content_code("iscc:aaa".to_string());
    let claim2 = ProvenanceClaim::new(0).with_content_code("iscc:bbb".to_string());

    assert_ne!(claim1.digest(), claim2.digest());
}
