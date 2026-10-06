//! Authentication reporting must reflect a check that actually ran.
//!
//! A CRC32 payload carries an unkeyed integrity tag. Supplying a MAC key to
//! the verifier must never turn that payload into an authenticated one, and
//! the two public status projections must never contradict each other.

use stegoeggo::{
    encode_image, process_image_bytes, verify_image_bytes, verify_image_bytes_detailed,
    verify_image_bytes_report, verify_legal_notice, EvidenceStrength, ImageOutputFormat,
    LegalMetadata, ProtectionContext, ProtectionLevel, VerificationResult, VerificationStatus,
};

fn test_image() -> image::DynamicImage {
    image::DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(256, 256, |x, y| {
        image::Rgb([
            (x as u8).wrapping_mul(3),
            (y as u8).wrapping_mul(5),
            ((x + y) as u8).wrapping_mul(7),
        ])
    }))
}

fn legal_metadata() -> LegalMetadata {
    LegalMetadata::new()
        .with_copyright_holder("Jane Artist")
        .with_creator("Jane Artist")
        .with_usage_terms("No AI training")
}

fn protect_with_key(mac_key: Option<&[u8]>) -> Vec<u8> {
    let mut ctx = ProtectionContext::new(0.5, 42)
        .with_format(ImageOutputFormat::Png)
        .with_legal_metadata(legal_metadata());
    if let Some(key) = mac_key {
        ctx = ctx.with_mac_key(key.to_vec());
    }
    process_image_bytes(
        &encode_image(&test_image(), image::ImageFormat::Png).unwrap(),
        ProtectionLevel::Standard,
        &ctx,
    )
    .expect("protection should succeed")
}

#[test]
fn crc_payload_is_never_authenticated_even_when_a_key_is_supplied() {
    let protected = protect_with_key(None);

    for key in [&b"test-mac-key"[..], &b"wrong-key"[..], &b""[..], &b"x"[..]] {
        let report = verify_image_bytes_report(&protected, key);
        let auth = report.authentication();

        assert!(
            !auth.attempted(),
            "a CRC32 payload declares no HMAC, so nothing may be reported as attempted (key: {key:?})"
        );
        assert!(
            !auth.key_matched(),
            "an unkeyed CRC32 tag can never match a supplied key (key: {key:?})"
        );
        assert_eq!(
            auth.algorithm(),
            "crc32",
            "algorithm must name the real check"
        );
        assert_eq!(auth.hmac_status(), None);
        assert_eq!(
            report.evidence_strength(),
            EvidenceStrength::MetadataNoticeAndBestEffortStego,
            "an unkeyed marker must stay in the best-effort tier (key: {key:?})"
        );

        let notice = verify_legal_notice(&protected, key);
        assert!(!notice.authenticated());
        assert_eq!(
            notice.evidence_strength(),
            EvidenceStrength::MetadataNoticeAndBestEffortStego
        );
    }
}

#[test]
fn crc_payload_verdict_does_not_depend_on_key_presence() {
    let protected = protect_with_key(None);
    let unkeyed = verify_image_bytes_report(&protected, &[]);
    let keyed = verify_image_bytes_report(&protected, b"an-entirely-unrelated-key");

    assert_eq!(unkeyed.evidence_strength(), keyed.evidence_strength());
    assert_eq!(
        unkeyed.hidden_marker().status(),
        keyed.hidden_marker().status()
    );
    assert_eq!(
        unkeyed.authentication().attempted(),
        keyed.authentication().attempted()
    );
    assert_eq!(
        unkeyed.authentication().key_matched(),
        keyed.authentication().key_matched()
    );
    assert_eq!(unkeyed.summary_status(), keyed.summary_status());
    assert_eq!(unkeyed.has_errors(), keyed.has_errors());
}

#[test]
fn crc_payload_verified_with_a_key_still_reports_metadata_and_marker() {
    let protected = protect_with_key(None);
    let notice = verify_legal_notice(&protected, b"some-key");

    assert!(notice.has_notice());
    assert_eq!(notice.copyright_holder(), Some("Jane Artist"));
    assert_eq!(notice.stego_status(), VerificationStatus::Verified);
    assert!(!notice.authenticated());
}

#[test]
fn hmac_payload_is_authenticated_only_with_the_matching_key() {
    let key = b"test-mac-key";
    let protected = protect_with_key(Some(key));

    let matched = verify_image_bytes_report(&protected, key);
    assert!(matched.authentication().attempted());
    assert!(matched.authentication().key_matched());
    assert_eq!(matched.authentication().algorithm(), "hmac-sha256");
    assert_eq!(
        matched.authentication().hmac_status(),
        Some(VerificationStatus::Verified)
    );
    assert_eq!(
        matched.evidence_strength(),
        EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance
    );
    assert!(verify_legal_notice(&protected, key).authenticated());

    for wrong in [&b"wrong-key"[..], &b""[..]] {
        let unmatched = verify_image_bytes_report(&protected, wrong);
        assert!(!unmatched.authentication().key_matched());
        assert_ne!(
            unmatched.evidence_strength(),
            EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance
        );
        assert!(!verify_legal_notice(&protected, wrong).authenticated());
    }
}

#[test]
fn hmac_payload_without_any_key_reports_missing_key_not_verified() {
    let protected = protect_with_key(Some(b"test-mac-key"));
    let report = verify_image_bytes_report(&protected, &[]);

    assert!(report.authentication().attempted());
    assert!(!report.authentication().key_matched());
    assert_eq!(
        report.authentication().hmac_status(),
        Some(VerificationStatus::NotFound)
    );
    assert_ne!(
        report.evidence_strength(),
        EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance
    );
}

#[test]
fn coarse_and_detailed_status_projections_never_contradict() {
    let cases: Vec<(Vec<u8>, Vec<&[u8]>)> = vec![
        (protect_with_key(None), vec![&[], b"key", b"other"]),
        (protect_with_key(Some(b"key")), vec![&[], b"key", b"other"]),
    ];

    for (protected, keys) in cases {
        for key in keys {
            let status = verify_image_bytes(&protected, key);
            match verify_image_bytes_detailed(&protected, key) {
                VerificationResult::Verified { .. } => assert_eq!(
                    status,
                    VerificationStatus::Verified,
                    "detailed says verified, coarse says {status:?} (key: {key:?})"
                ),
                other => assert_ne!(
                    status,
                    VerificationStatus::Verified,
                    "coarse says verified but detailed says {other:?} (key: {key:?})"
                ),
            }
        }
    }
}

#[test]
fn an_unprotected_image_reports_no_integrity_algorithm() {
    let plain = encode_image(&test_image(), image::ImageFormat::Png).unwrap();
    let report = verify_image_bytes_report(&plain, b"any-key");

    assert_eq!(
        report.hidden_marker().status(),
        VerificationStatus::NotFound,
        "the fixture must be genuinely unprotected"
    );
    assert!(!report.authentication().attempted());
    assert_eq!(report.authentication().hmac_status(), None);
    assert!(
        report.authentication().algorithm().is_empty(),
        "no integrity check ran, so no algorithm may be named (got {:?})",
        report.authentication().algorithm()
    );
    assert_eq!(report.evidence_strength(), EvidenceStrength::NoNoticeFound);
}

#[test]
fn legacy_notice_projection_never_overstates_its_own_facts() {
    // A caller-built notice can claim any evidence tier. The structured
    // report derived from it must recompute the tier from the facts it holds
    // rather than copying the claim.
    let overstated = stegoeggo::NoticeVerification::builder()
        .copyright_holder(Some("Jane Artist".to_string()))
        .stego_status(VerificationStatus::NotFound)
        .authenticated(true)
        .evidence_strength(EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance)
        .build();

    let report = stegoeggo::verification::VerificationReport::from_notice_verification(&overstated);

    assert!(report.authentication().key_matched());
    assert_eq!(
        report.evidence_strength(),
        report.compute_evidence_strength(),
        "the projected tier must match the projected facts"
    );
    assert_ne!(
        report.evidence_strength(),
        EvidenceStrength::MetadataNoticeAndAuthenticatedProvenance,
        "no hidden marker was verified, so the authenticated tier is unreachable"
    );
    assert_eq!(
        report.evidence_strength(),
        EvidenceStrength::MetadataNoticeOnly
    );
}
