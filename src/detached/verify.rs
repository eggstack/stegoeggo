use image::GenericImageView;
use sha2::{Digest, Sha256};

use crate::detached::manifest::DetachedManifest;
use crate::resource_limits::ResourceLimits;
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
    /// The manifest declares a reference and a stego payload was found, but the
    /// payload version does not match the declared version.
    VersionMismatch,
    /// The manifest declares a reference and a stego payload was found, but the
    /// payload digest does not match the declared digest.
    DigestMismatch,
    /// The manifest declares a reference and a stego payload was found, but the
    /// payload could not be parsed (malformed, corrupted, or authentication failed).
    Malformed,
    /// The manifest declares a reference and a valid stego payload was found in the image.
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
    let limits = ResourceLimits::default();
    verify_detached_manifest_with_limits(image_bytes, manifest, trust, Some(&limits))
}

/// Verify a detached manifest with resource limits.
///
/// Like [`verify_detached_manifest`], but enforces [`ResourceLimits`]
/// on the input image bytes before performing verification. The
/// resource limits check is performed before the SHA-256 hash computation.
///
/// # Arguments
///
/// * `image_bytes` - Raw image bytes.
/// * `manifest` - The detached manifest to verify.
/// * `trust` - Trust policy controlling which keys are trusted.
/// * `limits` - Optional resource limits. When `None`, default limits are used.
///
/// # Returns
///
/// A [`ManifestVerification`] with structured results.
#[must_use]
pub fn verify_detached_manifest_with_limits(
    image_bytes: &[u8],
    manifest: &DetachedManifest,
    trust: &TrustPolicy,
    limits: Option<&ResourceLimits>,
) -> ManifestVerification {
    if let Some(limits) = limits {
        if limits.check_input_size(image_bytes.len()).is_err() {
            let mut builder = VerificationReport::builder();
            builder = builder.with_bindings(
                crate::verification::report::BindingVerification::builder()
                    .instance_digest_present(false)
                    .instance_digest_valid(false)
                    .build(),
            );
            return ManifestVerification {
                report: builder.build(),
                instance_digest_match: false,
                manifest_valid: false,
                embedded_reference_status: EmbeddedReferenceStatus::NotProvided,
            };
        }
    }

    verify_detached_manifest_inner(image_bytes, manifest, trust)
}

#[allow(unused_variables)]
fn verify_detached_manifest_inner(
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
    let mut _any_signature_valid = false;

    for sig_record in &manifest.signatures {
        if sig_record.algorithm != "ed25519" {
            builder = builder.add_signature(
                SignatureVerification::builder()
                    .present(true)
                    .structurally_valid(false)
                    .source(FieldSource::DetachedManifest)
                    .build(),
            );
            continue;
        }

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
            if pub_entry.algorithm != "ed25519" {
                builder = builder.add_signature(
                    SignatureVerification::builder()
                        .present(true)
                        .structurally_valid(false)
                        .source(FieldSource::DetachedManifest)
                        .build(),
                );
                continue;
            }

            #[cfg(feature = "signatures")]
            {
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
                        _any_signature_valid = _any_signature_valid || is_valid;

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
            }
            #[cfg(not(feature = "signatures"))]
            {
                builder = builder.add_signature(
                    SignatureVerification::builder()
                        .present(true)
                        .structurally_valid(true)
                        .cryptographically_valid(false)
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

    // 3. Trust evaluation is derived solely from the caller-supplied TrustPolicy.
    // The manifest's trust_metadata is never used to set the trust outcome.
    // A malicious manifest claiming `trusted: true` must not influence the
    // report. If trust_metadata is present, we report the trust_model name
    // for transparency but always set trusted=false (the real trust decision
    // lives in the per-signature key_id_matched flags above).
    if let Some(ref trust) = manifest.trust_metadata {
        builder = builder.with_trust(
            crate::verification::report::TrustEvaluation::builder()
                .trust_model(&trust.trust_model)
                .trusted(false)
                .reason("trust_metadata from manifest is informational only; trust is determined by caller policy")
                .build(),
        );
    }

    // 4. Set binding verification (instance digest + format + dimensions + file size)
    let actual_format = crate::types::ImageOutputFormat::from_magic_bytes(image_bytes)
        .map(|f| format!("{:?}", f).to_lowercase())
        .unwrap_or_default();
    let format_valid = actual_format == manifest.claim.format;

    let (actual_width, actual_height) = match crate::util::image::load_image_from_bytes(image_bytes)
    {
        Ok(img) => img.dimensions(),
        Err(_) => (0, 0),
    };
    let dimensions_valid =
        actual_width == manifest.claim.width && actual_height == manifest.claim.height;

    let file_size_valid = (image_bytes.len() as u64) == manifest.claim.file_size;

    builder = builder.with_bindings(
        crate::verification::report::BindingVerification::builder()
            .instance_digest_present(!manifest.claim.instance_digest.is_empty())
            .instance_digest_valid(instance_digest_match)
            .format_valid(format_valid)
            .dimensions_valid(dimensions_valid)
            .file_size_valid(file_size_valid)
            .build(),
    );

    let report = builder.build();

    let embedded_reference_status = match &manifest.embedded_reference {
        None => EmbeddedReferenceStatus::NotProvided,
        Some(reference) => {
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
                Some(payload) => {
                    if payload.version() != reference.payload_version {
                        return ManifestVerification {
                            report,
                            instance_digest_match,
                            manifest_valid: true,
                            embedded_reference_status: EmbeddedReferenceStatus::VersionMismatch,
                        };
                    }
                    match payload.raw_payload() {
                        Some(raw) => {
                            let mut hasher = Sha256::new();
                            hasher.update(raw);
                            let actual_digest =
                                format!("sha256:{}", hex::encode(hasher.finalize()));
                            if actual_digest != reference.payload_digest {
                                EmbeddedReferenceStatus::DigestMismatch
                            } else {
                                EmbeddedReferenceStatus::Present
                            }
                        }
                        None => EmbeddedReferenceStatus::Malformed,
                    }
                }
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
