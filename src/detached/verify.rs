use sha2::{Digest, Sha256};

use crate::detached::manifest::DetachedManifest;
use crate::verification::report::{FieldSource, SignatureVerification, VerificationReport};

/// Result of verifying a detached manifest against an image.
#[derive(Debug, Clone)]
pub struct ManifestVerification {
    /// The aggregated verification report.
    pub report: VerificationReport,
    /// Whether the image instance digest matches the claim.
    pub instance_digest_match: bool,
    /// Whether the manifest was deserialized successfully.
    pub manifest_valid: bool,
}

/// Verify a detached manifest against image bytes.
///
/// Checks:
/// 1. Manifest deserializes successfully.
/// 2. Image SHA-256 matches the claim's `instance_digest`.
/// 3. Signatures verify against provided public keys.
/// 4. Trust metadata is evaluated if present.
///
/// # Arguments
///
/// * `image_bytes` - Raw image bytes.
/// * `manifest` - The detached manifest to verify.
/// * `expected_keys` - Optional list of trusted public key identifiers.
///   If empty or None, signature validity is reported but not trusted.
///
/// # Returns
///
/// A [`ManifestVerification`] with structured results.
#[must_use]
pub fn verify_detached_manifest(
    image_bytes: &[u8],
    manifest: &DetachedManifest,
    expected_keys: Option<&[Vec<u8>]>,
) -> ManifestVerification {
    let mut builder = VerificationReport::builder();

    // 1. Verify instance digest
    let mut hasher = Sha256::new();
    hasher.update(image_bytes);
    let image_hash = hasher.finalize();
    let image_digest = format!("sha256:{}", hex::encode(image_hash));
    let instance_digest_match = image_digest == manifest.claim.instance_digest;

    // 2. Verify signatures
    let mut any_signature_valid = false;

    for sig_record in &manifest.signatures {
        let sig_bytes = match hex::decode(&sig_record.signature) {
            Ok(b) => b,
            Err(_) => {
                builder = builder.add_signature(
                    SignatureVerification::builder()
                        .present(true)
                        .structurally_valid(false)
                        .source(FieldSource::DetachedManifest)
                        .build(),
                );
                continue;
            }
        };

        // Find matching public key in manifest
        let matching_key = manifest
            .public_keys
            .iter()
            .find(|k| k.key_id == sig_record.key_id);

        if let Some(pub_entry) = matching_key {
            if let Ok(pub_bytes_vec) = hex::decode(&pub_entry.key_bytes) {
                if pub_bytes_vec.len() == 32 {
                    let mut raw_pub = [0u8; 32];
                    raw_pub.copy_from_slice(&pub_bytes_vec);
                    let vk = crate::signing::VerifyingKey::from_bytes(
                        raw_pub,
                        sig_record.key_id.clone(),
                    );

                    let claim_bytes = manifest.claim.canonical_bytes();
                    let result = vk.verify(&claim_bytes, &sig_bytes);

                    let is_valid = result == crate::signing::SignatureResult::Valid;
                    any_signature_valid = any_signature_valid || is_valid;

                    let key_id_matched = expected_keys
                        .is_some_and(|trusted| trusted.iter().any(|t| t == &sig_record.key_id));

                    builder = builder.add_signature(
                        SignatureVerification::builder()
                            .present(true)
                            .structurally_valid(true)
                            .cryptographically_valid(is_valid)
                            .public_key_id(sig_record.key_id.clone())
                            .key_id_matched(key_id_matched)
                            .trusted(key_id_matched && is_valid)
                            .source(FieldSource::DetachedManifest)
                            .build(),
                    );
                } else {
                    builder = builder.add_signature(
                        SignatureVerification::builder()
                            .present(true)
                            .structurally_valid(false)
                            .source(FieldSource::DetachedManifest)
                            .build(),
                    );
                }
            } else {
                builder = builder.add_signature(
                    SignatureVerification::builder()
                        .present(true)
                        .structurally_valid(false)
                        .source(FieldSource::DetachedManifest)
                        .build(),
                );
            }
        } else {
            builder = builder.add_signature(
                SignatureVerification::builder()
                    .present(true)
                    .structurally_valid(false)
                    .source(FieldSource::DetachedManifest)
                    .build(),
            );
        }
    }

    // 3. Set trust evaluation
    if let Some(ref trust) = manifest.trust_metadata {
        builder = builder.with_trust(
            crate::verification::report::TrustEvaluation::builder()
                .trust_model(&trust.trust_model)
                .trusted(trust.trusted)
                .reason(&trust.reason)
                .build(),
        );
    }

    // 4. Set binding verification
    builder = builder.with_bindings(
        crate::verification::report::BindingVerification::builder()
            .instance_digest_present(!manifest.claim.instance_digest.is_empty())
            .instance_digest_valid(instance_digest_match)
            .build(),
    );

    let report = builder.build();

    ManifestVerification {
        report,
        instance_digest_match,
        manifest_valid: true,
    }
}
