use sha2::{Digest, Sha256};

use crate::detached::manifest::DetachedManifest;
use crate::verification::report::{FieldSource, SignatureVerification, VerificationReport};

/// Callback function type for trust evaluation.
///
/// Receives a key identifier and returns `true` if the key is trusted.
pub type TrustCallbackFn = dyn Fn(&[u8]) -> bool + Send + Sync;

/// Trust policy for evaluating detached manifest signatures.
///
/// Controls which public key identifiers are considered trusted
/// during verification. The library ships no implicit trust store;
/// trust is always caller-owned.
pub enum TrustPolicy {
    /// Never trust any key. Signature validity is reported but `trusted` is always false.
    TrustNone,
    /// Trust an exact set of key identifiers.
    TrustKeys(Vec<Vec<u8>>),
    /// Trust keys for which the callback returns `true`.
    ///
    /// The callback receives the key identifier from each signature record.
    /// Returning `true` marks the key as trusted (combined with cryptographic validity).
    TrustCallback(Box<TrustCallbackFn>),
}

impl std::fmt::Debug for TrustPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustPolicy::TrustNone => write!(f, "TrustNone"),
            TrustPolicy::TrustKeys(keys) => f.debug_tuple("TrustKeys").field(keys).finish(),
            TrustPolicy::TrustCallback(_) => write!(f, "TrustCallback(<function>)"),
        }
    }
}

/// Status of the embedded payload reference in a detached manifest.
///
/// When a manifest declares an `embedded_reference`, this status indicates
/// whether the referenced payload was found in the image. A `Stripped` status
/// means only detached evidence remains — the embedded stego channel has been
/// removed or was never present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddedReferenceStatus {
    /// The manifest does not declare an embedded reference.
    NotProvided,
    /// The manifest declares a reference but no stego payload was found in the image.
    /// Only detached evidence remains.
    Stripped,
    /// The manifest declares a reference and a stego payload was found in the image.
    Present,
}

/// Result of verifying a detached manifest against an image.
#[derive(Debug, Clone)]
pub struct ManifestVerification {
    /// The aggregated verification report.
    pub report: VerificationReport,
    /// Whether the image instance digest matches the claim.
    pub instance_digest_match: bool,
    /// Whether the manifest was deserialized successfully.
    pub manifest_valid: bool,
    /// Status of the embedded payload reference.
    pub embedded_reference_status: EmbeddedReferenceStatus,
}

/// Verify a detached manifest against image bytes using a [`TrustPolicy`].
///
/// Checks:
/// 1. Image SHA-256 matches the claim's `instance_digest`.
/// 2. Signatures verify against public keys in the manifest.
/// 3. Trust is evaluated according to the supplied policy.
/// 4. Trust metadata from the manifest is reported if present.
///
/// # Arguments
///
/// * `image_bytes` - Raw image bytes.
/// * `manifest` - The detached manifest to verify.
/// * `trust` - Trust policy controlling which keys are trusted.
///
/// # Returns
///
/// A [`ManifestVerification`] with structured results.
#[must_use]
pub fn verify_detached_manifest(
    image_bytes: &[u8],
    manifest: &DetachedManifest,
    trust: &TrustPolicy,
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

                    let key_id_matched = match trust {
                        TrustPolicy::TrustNone => false,
                        TrustPolicy::TrustKeys(keys) => {
                            keys.iter().any(|t| t == &sig_record.key_id)
                        }
                        TrustPolicy::TrustCallback(f) => f(&sig_record.key_id),
                    };

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

    let embedded_reference_status = match manifest.embedded_reference {
        None => EmbeddedReferenceStatus::NotProvided,
        Some(_) => {
            let extractor = crate::protected::steganography::SteganographyProtector::new();
            let img = match crate::util::image::load_image_from_bytes(image_bytes) {
                Ok(img) => img,
                Err(_) => {
                    return ManifestVerification {
                        report,
                        instance_digest_match,
                        manifest_valid: true,
                        embedded_reference_status: EmbeddedReferenceStatus::Stripped,
                    };
                }
            };
            match extractor.extract_payload(&img) {
                Some(_) => EmbeddedReferenceStatus::Present,
                None => EmbeddedReferenceStatus::Stripped,
            }
        }
    };

    ManifestVerification {
        report,
        instance_digest_match,
        manifest_valid: true,
        embedded_reference_status,
    }
}

/// Verify a detached manifest against image bytes using a flat key-ID set.
///
/// This is a backward-compatible wrapper around [`verify_detached_manifest`]
/// that accepts the legacy `expected_keys` parameter.
///
/// # Arguments
///
/// * `image_bytes` - Raw image bytes.
/// * `manifest` - The detached manifest to verify.
/// * `expected_keys` - Optional list of trusted public key identifiers.
///   If `None`, [`TrustPolicy::TrustNone`] is used.
///
/// # Returns
///
/// A [`ManifestVerification`] with structured results.
#[must_use]
pub fn verify_detached_manifest_with_keys(
    image_bytes: &[u8],
    manifest: &DetachedManifest,
    expected_keys: Option<&[Vec<u8>]>,
) -> ManifestVerification {
    let policy = match expected_keys {
        Some(keys) => TrustPolicy::TrustKeys(keys.to_vec()),
        None => TrustPolicy::TrustNone,
    };
    verify_detached_manifest(image_bytes, manifest, &policy)
}
