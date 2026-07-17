use image::{DynamicImage, ImageEncoder};
use stegoeggo::{
    process_image_bytes, LegalMetadata, MetadataTrapProtector, MetadataUpdatePolicy,
    ProtectionContext, ProtectionLevel, VerificationStatus,
};

fn create_test_png(width: u32, height: u32) -> Vec<u8> {
    let img = DynamicImage::new_rgb8(width, height);
    let mut buffer = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
    encoder
        .write_image(
            &img.to_rgb8(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    buffer
}

fn create_png_with_custom_text(png_bytes: &[u8], key: &str, value: &str) -> Vec<u8> {
    let mut output = Vec::with_capacity(png_bytes.len() + 100);
    output.extend_from_slice(&png_bytes[0..8]);
    let mut i = 8;
    while i + 8 <= png_bytes.len() {
        let length = u32::from_be_bytes([
            png_bytes[i],
            png_bytes[i + 1],
            png_bytes[i + 2],
            png_bytes[i + 3],
        ]) as usize;
        let chunk_type = &png_bytes[i + 4..i + 8];

        if chunk_type == b"IEND" {
            output.extend_from_slice(&png_bytes[i..i + 8 + length + 4]);
            let chunk_len = key.len() + 1 + value.len();
            let chunk_len_bytes = (chunk_len as u32).to_be_bytes();
            output.extend_from_slice(&chunk_len_bytes);
            output.extend_from_slice(b"tEXt");
            output.extend_from_slice(key.as_bytes());
            output.push(0);
            output.extend_from_slice(value.as_bytes());
            let mut crc = crc32fast::Hasher::new();
            crc.update(b"tEXt");
            crc.update(key.as_bytes());
            crc.update(&[0u8]);
            crc.update(value.as_bytes());
            let checksum = crc.finalize().to_be_bytes();
            output.extend_from_slice(&checksum);
        } else {
            output.extend_from_slice(&png_bytes[i..i + 8 + length + 4]);
        }

        i += 8 + length + 4;
    }
    output
}

fn has_text_chunk(png_bytes: &[u8], key: &str) -> bool {
    let mut i = 8;
    while i + 8 <= png_bytes.len() {
        let length = u32::from_be_bytes([
            png_bytes[i],
            png_bytes[i + 1],
            png_bytes[i + 2],
            png_bytes[i + 3],
        ]) as usize;
        let chunk_type = &png_bytes[i + 4..i + 8];
        if chunk_type == b"tEXt" && length > key.len() {
            let chunk_data = &png_bytes[i + 8..i + 8 + length];
            let null_pos = chunk_data.iter().position(|&b| b == 0);
            if let Some(pos) = null_pos {
                let chunk_key = &chunk_data[..pos];
                if chunk_key == key.as_bytes() {
                    return true;
                }
            }
        }
        i += 8 + length + 4;
    }
    false
}

fn get_text_value(png_bytes: &[u8], key: &str) -> Option<String> {
    let mut i = 8;
    while i + 8 <= png_bytes.len() {
        let length = u32::from_be_bytes([
            png_bytes[i],
            png_bytes[i + 1],
            png_bytes[i + 2],
            png_bytes[i + 3],
        ]) as usize;
        let chunk_type = &png_bytes[i + 4..i + 8];
        if chunk_type == b"tEXt" && length > key.len() {
            let chunk_data = &png_bytes[i + 8..i + 8 + length];
            let null_pos = chunk_data.iter().position(|&b| b == 0);
            if let Some(pos) = null_pos {
                let chunk_key = &chunk_data[..pos];
                if chunk_key == key.as_bytes() {
                    let val = &chunk_data[pos + 1..];
                    return Some(String::from_utf8_lossy(val).into_owned());
                }
            }
        }
        i += 8 + length + 4;
    }
    None
}

fn count_stego_text_chunks(png_bytes: &[u8]) -> usize {
    let stego_keys: &[&[u8]] = &[
        b"Copyright",
        b"Creator",
        b"Contact",
        b"UsageTerms",
        b"AIConstraints",
        b"X-Protection-Seed",
        b"LicenseURL",
        b"WebStatement",
    ];
    let mut count = 0;
    let mut i = 8;
    while i + 8 <= png_bytes.len() {
        let length = u32::from_be_bytes([
            png_bytes[i],
            png_bytes[i + 1],
            png_bytes[i + 2],
            png_bytes[i + 3],
        ]) as usize;
        let chunk_type = &png_bytes[i + 4..i + 8];
        if chunk_type == b"tEXt" && length > 0 {
            let chunk_data = &png_bytes[i + 8..i + 8 + length];
            let null_pos = chunk_data.iter().position(|&b| b == 0);
            if let Some(pos) = null_pos {
                let chunk_key = &chunk_data[..pos];
                if stego_keys.contains(&chunk_key) {
                    count += 1;
                }
            }
        }
        i += 8 + length + 4;
    }
    count
}

fn legal_metadata() -> LegalMetadata {
    LegalMetadata::new()
        .with_copyright_holder("Test Corp")
        .with_creator("Test Author")
        .with_usage_terms("All Rights Reserved")
}

#[test]
fn default_policy_is_replace_stego_owned() {
    let ctx = ProtectionContext::new(0.5, 42);
    assert_eq!(
        ctx.metadata_update_policy(),
        MetadataUpdatePolicy::ReplaceStegoOwned
    );
}

#[test]
fn policy_as_str_roundtrip() {
    assert_eq!(
        MetadataUpdatePolicy::ReplaceStegoOwned.as_str(),
        "replace-stego-owned"
    );
    assert_eq!(
        MetadataUpdatePolicy::FailOnConflict.as_str(),
        "fail-on-conflict"
    );
    assert_eq!(
        MetadataUpdatePolicy::PreserveExisting.as_str(),
        "preserve-existing"
    );
}

#[test]
fn with_metadata_update_policy_sets_and_retrieves() {
    let ctx = ProtectionContext::new(0.5, 42)
        .with_metadata_update_policy(MetadataUpdatePolicy::PreserveExisting);
    assert_eq!(
        ctx.metadata_update_policy(),
        MetadataUpdatePolicy::PreserveExisting
    );
}

#[test]
fn replace_stego_owned_idempotent_metadata_count() {
    let png1 = create_test_png(64, 64);
    let ctx = ProtectionContext::new(0.5, 42)
        .with_legal_metadata(legal_metadata())
        .with_metadata_injection(true);

    let protected1 = process_image_bytes(&png1, ProtectionLevel::Light, &ctx).unwrap();
    let protected2 = process_image_bytes(&protected1, ProtectionLevel::Light, &ctx).unwrap();

    let count1 = count_stego_text_chunks(&protected1);
    let count2 = count_stego_text_chunks(&protected2);

    assert!(count1 > 0, "First pass should inject stego metadata");
    assert_eq!(
        count1, count2,
        "Second pass with ReplaceStegoOwned should not increase stego chunk count: \
         first={count1}, second={count2}"
    );
}

#[test]
fn replace_stego_owned_updates_values() {
    let png1 = create_test_png(64, 64);
    let ctx1 = ProtectionContext::new(0.5, 42)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Original Corp")
                .with_creator("Original Author"),
        )
        .with_metadata_injection(true);

    let protected1 = process_image_bytes(&png1, ProtectionLevel::Light, &ctx1).unwrap();

    let ctx2 = ProtectionContext::new(0.5, 99)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Updated Corp")
                .with_creator("Updated Author"),
        )
        .with_metadata_injection(true);

    let protected2 = process_image_bytes(&protected1, ProtectionLevel::Light, &ctx2).unwrap();

    assert_eq!(
        get_text_value(&protected2, "Copyright"),
        Some("Copyright (c) Updated Corp".to_string()),
        "Copyright should be updated on second pass"
    );
    assert_eq!(
        get_text_value(&protected2, "Creator"),
        Some("Updated Author".to_string()),
        "Creator should be updated on second pass"
    );
    assert_ne!(
        get_text_value(&protected2, "Copyright"),
        Some("Copyright (c) Original Corp".to_string()),
        "Old copyright should not remain"
    );
}

#[test]
fn custom_text_chunk_survives_metadata_injection() {
    let png = create_test_png(64, 64);
    let png_with_author = create_png_with_custom_text(&png, "Author", "Alice");

    let ctx = ProtectionContext::new(0.5, 42)
        .with_legal_metadata(legal_metadata())
        .with_metadata_injection(true);

    let trap = MetadataTrapProtector::new();
    let injected = trap.inject_bytes(&png_with_author, &ctx).unwrap();

    assert!(
        has_text_chunk(&injected, "Author"),
        "Custom 'Author' tEXt chunk should survive metadata injection"
    );
    assert_eq!(
        get_text_value(&injected, "Author"),
        Some("Alice".to_string()),
        "Custom 'Author' value should be preserved"
    );

    let injected2 = trap.inject_bytes(&injected, &ctx).unwrap();
    assert!(
        has_text_chunk(&injected2, "Author"),
        "Custom 'Author' tEXt chunk should survive second injection"
    );
    assert_eq!(
        get_text_value(&injected2, "Author"),
        Some("Alice".to_string()),
        "Custom 'Author' value should be preserved after second injection"
    );
}

#[test]
fn preserve_existing_currently_replaces_like_default() {
    let png1 = create_test_png(64, 64);
    let ctx1 = ProtectionContext::new(0.5, 42)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("Original Corp")
                .with_creator("Original Author"),
        )
        .with_metadata_injection(true);

    let protected1 = process_image_bytes(&png1, ProtectionLevel::Light, &ctx1).unwrap();

    let ctx2 = ProtectionContext::new(0.5, 99)
        .with_legal_metadata(
            LegalMetadata::new()
                .with_copyright_holder("New Corp")
                .with_creator("New Author"),
        )
        .with_metadata_injection(true)
        .with_metadata_update_policy(MetadataUpdatePolicy::PreserveExisting);

    let protected2 = process_image_bytes(&protected1, ProtectionLevel::Light, &ctx2).unwrap();

    assert_eq!(
        get_text_value(&protected2, "Copyright"),
        Some("Copyright (c) New Corp".to_string()),
        "Pipeline replaces metadata even with PreserveExisting (policy not yet enforced)"
    );
    assert_eq!(
        get_text_value(&protected2, "Creator"),
        Some("New Author".to_string()),
        "Pipeline replaces metadata even with PreserveExisting (policy not yet enforced)"
    );
}

#[test]
fn policy_serde_roundtrip() {
    let ctx = ProtectionContext::new(0.5, 42)
        .with_metadata_update_policy(MetadataUpdatePolicy::PreserveExisting);
    let json = serde_json::to_string(&ctx).unwrap();
    let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
    assert_eq!(
        restored.metadata_update_policy(),
        MetadataUpdatePolicy::PreserveExisting
    );
}

#[test]
fn fail_on_conflict_currently_does_not_error() {
    let png1 = create_test_png(64, 64);
    let ctx1 = ProtectionContext::new(0.5, 42)
        .with_legal_metadata(legal_metadata())
        .with_metadata_injection(true);

    let protected1 = process_image_bytes(&png1, ProtectionLevel::Light, &ctx1).unwrap();

    let ctx2 = ProtectionContext::new(0.5, 99)
        .with_legal_metadata(legal_metadata())
        .with_metadata_injection(true)
        .with_metadata_update_policy(MetadataUpdatePolicy::FailOnConflict);

    let result = process_image_bytes(&protected1, ProtectionLevel::Light, &ctx2);
    assert!(
        result.is_ok(),
        "FailOnConflict does not yet error (policy not enforced): {:?}",
        result.err()
    );
}

#[test]
fn policy_default_deserialize_backward_compatible() {
    let ctx = ProtectionContext::new(0.5, 42);
    let json = serde_json::to_string(&ctx).unwrap();
    let restored: ProtectionContext = serde_json::from_str(&json).unwrap();
    assert_eq!(
        restored.metadata_update_policy(),
        MetadataUpdatePolicy::ReplaceStegoOwned
    );
}

#[test]
fn verification_succeeds_after_double_process() {
    let png1 = create_test_png(64, 64);
    let ctx = ProtectionContext::new(0.7, 42)
        .with_mac_key(b"test-key".to_vec())
        .with_legal_metadata(legal_metadata())
        .with_metadata_injection(true);

    let protected1 = process_image_bytes(&png1, ProtectionLevel::Standard, &ctx).unwrap();
    let protected2 = process_image_bytes(&protected1, ProtectionLevel::Standard, &ctx).unwrap();

    let status = stegoeggo::verify_image_bytes(&protected2, b"test-key");
    assert_eq!(
        status,
        VerificationStatus::Verified,
        "Verification should succeed after double processing with ReplaceStegoOwned"
    );
}
