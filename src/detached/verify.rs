use image::GenericImageView;
use sha2::{Digest, Sha256};

use crate::detached::manifest::DetachedManifest;
use crate::resource_limits::ResourceLimits;
use crate::verification::report::{FieldSource, SignatureVerification, VerificationReport};

/// Callback function type for trust evaluation.
///
/// Receives a key identifier and returns `true` if the key is trusted.
pub type TrustCallbackFn = dyn Fn(&[u8]) -> bool + Send + Sync;

/// A caller-owned trusted verifying key.
///
/// Unlike [`TrustPolicy::TrustKeys`], which trusts only a key identifier,
/// this type binds trust to the exact 32-byte Ed25519 public key supplied
/// by the caller. A manifest cannot substitute alternative key bytes under
/// a trusted key ID when this policy is used.
///
/// `key_id` is used to match against signature records; `key` is the
/// caller-owned public key used for cryptographic verification.
#[cfg(feature = "signatures")]
#[derive(Clone)]
pub struct TrustedVerifyingKey {
    /// Key identifier used to match signature records.
    pub key_id: Vec<u8>,
    /// Caller-owned Ed25519 verifying key.
    pub key: crate::signing::VerifyingKey,
}

#[cfg(feature = "signatures")]
impl std::fmt::Debug for TrustedVerifyingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrustedVerifyingKey")
            .field("key_id", &hex::encode(&self.key_id))
            .finish_non_exhaustive()
    }
}

/// Trust policy for evaluating detached manifest signatures.
///
/// Controls which public key identifiers are considered trusted
/// during verification. The library ships no implicit trust store;
/// trust is always caller-owned.
///
/// # Trust modes
///
/// - [`TrustPolicy::TrustNone`]: No key is trusted. Signature validity is
///   reported but `trusted` is always `false`.
/// - [`TrustPolicy::TrustKeys`]: Trusts key identifiers only. **Does not
///   bind external key material** — a manifest can supply arbitrary public
///   key bytes under a trusted key ID. Use only for legacy compatibility.
/// - [`TrustPolicy::TrustCallback`]: Trusts keys for which the callback
///   returns `true`.
///
/// For caller-owned public key verification (binding key ID to exact key
/// bytes), use [`DetachedVerificationOptions`] with
/// [`verify_detached_manifest_with_options`].
pub enum TrustPolicy {
    /// Never trust any key. Signature validity is reported but `trusted` is always false.
    TrustNone,
    /// Trust an exact set of key identifiers.
    ///
    /// **Does not bind external key material.** A manifest can supply
    /// arbitrary public key bytes under a trusted key ID. Use only for
    /// legacy compatibility.
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

/// Options for detached manifest verification with caller-owned key support.
///
/// This additive type allows the caller to supply verifying keys that bind
/// a key ID to exact Ed25519 public key bytes. A manifest cannot substitute
/// alternative key bytes under a trusted key ID when caller-owned keys are
/// provided.
///
/// Use [`verify_detached_manifest_with_options`] to pass these options.
#[derive(Debug, Default)]
pub struct DetachedVerificationOptions<'a> {
    /// Trust policy controlling which key identifiers are trusted.
    /// When `None`, [`TrustPolicy::TrustNone`] is used.
    pub trust_policy: Option<&'a TrustPolicy>,
    /// Caller-owned verifying keys that bind key IDs to exact public key bytes.
    #[cfg(feature = "signatures")]
    pub caller_verifying_keys: &'a [TrustedVerifyingKey],
    /// Optional HMAC key for embedded payload verification.
    pub payload_mac_key: Option<&'a [u8]>,
    /// Optional resource limits.
    pub limits: Option<&'a ResourceLimits>,
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
    #[deprecated(note = "use PresentValid")]
    Present,
    /// The manifest declares a reference and a valid stego payload was found in the image.
    PresentValid,
    /// The manifest declares an HMAC-protected reference but no MAC key is available.
    AuthenticationKeyMissing,
    /// The manifest declares an HMAC-protected reference and verification failed.
    AuthenticationFailed,
    /// The manifest declares a reference but the payload version is not supported.
    UnsupportedVersion,
}

/// Overall status of detached manifest verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetachedOverallStatus {
    /// Manifest is valid, binding matches, at least one signature is cryptographically valid,
    /// and a caller-trusted key produced a valid signature.
    VerifiedTrusted,
    /// Manifest is valid, binding matches, at least one signature is cryptographically valid,
    /// but no caller-trusted key produced a valid signature.
    VerifiedUntrusted,
    /// Manifest failed to parse or resource limits were exceeded.
    InvalidConfiguration,
    /// Image instance digest does not match the manifest claim.
    BindingFailure,
    /// No signature was cryptographically valid.
    SignatureFailure,
    /// Embedded reference check failed (stripped, version mismatch, digest mismatch, etc.).
    EmbeddedReferenceFailure,
    /// A caller-supplied verifying key matched the key ID but the manifest
    /// key bytes differ from the caller-owned key. This is an integrity/
    /// configuration failure, not ordinary absence of trust.
    KeyMaterialMismatch,
}

impl DetachedOverallStatus {
    /// Map this status to a CLI exit code.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::VerifiedTrusted => 0,
            Self::VerifiedUntrusted => 4,
            Self::InvalidConfiguration => 2,
            Self::BindingFailure
            | Self::SignatureFailure
            | Self::EmbeddedReferenceFailure
            | Self::KeyMaterialMismatch => 3,
        }
    }
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

impl ManifestVerification {
    /// Compute the overall verification status.
    ///
    /// Priority: InvalidConfiguration > BindingFailure > SignatureFailure > EmbeddedReferenceFailure > KeyMaterialMismatch > Verified.
    #[must_use]
    pub fn overall_status(&self) -> DetachedOverallStatus {
        if !self.manifest_valid {
            return DetachedOverallStatus::InvalidConfiguration;
        }
        if !self.instance_digest_match {
            return DetachedOverallStatus::BindingFailure;
        }
        if !self
            .report
            .signatures()
            .iter()
            .any(|s| s.cryptographically_valid())
        {
            return DetachedOverallStatus::SignatureFailure;
        }
        match self.embedded_reference_status {
            #[allow(deprecated)]
            EmbeddedReferenceStatus::NotProvided
            | EmbeddedReferenceStatus::Present
            | EmbeddedReferenceStatus::PresentValid => {
                // Key-material mismatch takes priority over ordinary untrusted:
                // a caller-supplied key matched the ID but manifest bytes differ.
                let has_key_material_mismatch = self
                    .report
                    .signatures()
                    .iter()
                    .any(|s| s.key_id_matched() && !s.key_material_matched());
                if has_key_material_mismatch {
                    DetachedOverallStatus::KeyMaterialMismatch
                } else if self.report.trust().trusted() {
                    DetachedOverallStatus::VerifiedTrusted
                } else {
                    DetachedOverallStatus::VerifiedUntrusted
                }
            }
            _ => DetachedOverallStatus::EmbeddedReferenceFailure,
        }
    }
}

/// Verify a detached manifest against image bytes using a [`TrustPolicy`].
///
/// Checks:
/// 1. Image SHA-256 matches the claim's `instance_digest`.
/// 2. Signatures verify against public keys in the manifest.
/// 3. Trust is evaluated according to the supplied policy.
/// 4. Trust metadata from the manifest is reported if present.
/// 5. Embedded payload reference is verified with the optional payload MAC key.
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
    verify_detached_manifest_with_limits_and_mac(image_bytes, manifest, trust, limits, None)
}

/// Verify a detached manifest with resource limits and an optional payload MAC key.
///
/// Like [`verify_detached_manifest_with_limits`], but also verifies
/// embedded HMAC payload references when a MAC key is provided.
///
/// # Arguments
///
/// * `image_bytes` - Raw image bytes.
/// * `manifest` - The detached manifest to verify.
/// * `trust` - Trust policy controlling which keys are trusted.
/// * `limits` - Optional resource limits. When `None`, default limits are used.
/// * `payload_mac_key` - Optional HMAC key for embedded payload verification.
///   When `Some`, HMAC-protected payloads are verified with this key.
///   When `None` and an HMAC payload is found, `EmbeddedReferenceStatus::AuthenticationKeyMissing`
///   is returned. When `Some` and the key is wrong,
///   `EmbeddedReferenceStatus::AuthenticationFailed` is returned.
///
/// # Returns
///
/// A [`ManifestVerification`] with structured results.
#[must_use]
pub fn verify_detached_manifest_with_limits_and_mac(
    image_bytes: &[u8],
    manifest: &DetachedManifest,
    trust: &TrustPolicy,
    limits: Option<&ResourceLimits>,
    payload_mac_key: Option<&[u8]>,
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

    verify_detached_manifest_inner(image_bytes, manifest, trust, payload_mac_key, &[])
}

#[allow(unused_variables)]
fn verify_detached_manifest_inner(
    image_bytes: &[u8],
    manifest: &DetachedManifest,
    trust: &TrustPolicy,
    payload_mac_key: Option<&[u8]>,
    #[cfg(feature = "signatures")] caller_verifying_keys: &[TrustedVerifyingKey],
    #[cfg(not(feature = "signatures"))] caller_verifying_keys: &[()],
) -> ManifestVerification {
    let mut builder = VerificationReport::builder();

    // 0. Validate manifest structure before signature evaluation.
    // Duplicate/conflicting key records must be rejected before any
    // signature is evaluated.
    let manifest_valid = manifest.validate().is_ok();

    // 1. Verify instance digest
    let mut hasher = Sha256::new();
    hasher.update(image_bytes);
    let image_hash = hasher.finalize();
    let image_digest = format!("sha256:{}", hex::encode(image_hash));
    let instance_digest_match = image_digest == manifest.claim.instance_digest;

    // 2. Verify signatures
    let mut _any_signature_valid = false;
    let mut _any_signature_trusted = false;

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

        // Determine whether a caller-owned trusted verifying key matches this
        // signature's key ID. When present, verification uses the caller-owned
        // key bytes directly and requires manifest key bytes (if any) to match.
        #[cfg(feature = "signatures")]
        let trusted_vk: Option<&crate::signing::VerifyingKey> = caller_verifying_keys
            .iter()
            .find(|t| t.key_id == sig_record.key_id)
            .map(|t| &t.key);
        #[cfg(not(feature = "signatures"))]
        let trusted_vk: Option<()> = None;

        #[cfg(feature = "signatures")]
        {
            // Decide which key bytes to use for verification and whether key
            // material binding holds.
            let (verify_key_bytes, key_material_matched): (Option<[u8; 32]>, bool) =
                if let Some(vk) = trusted_vk {
                    // Caller-owned key: verify with caller bytes, require manifest
                    // bytes (if present) to match.
                    let caller_bytes = *vk.as_bytes();
                    let matched = if let Some(entry) = matching_key {
                        if entry.algorithm != "ed25519" {
                            false
                        } else if let Ok(manifest_bytes) = hex::decode(&entry.key_bytes) {
                            manifest_bytes.as_slice() == caller_bytes.as_slice()
                        } else {
                            false
                        }
                    } else {
                        // No manifest key entry — caller key used directly.
                        true
                    };
                    (Some(caller_bytes), matched)
                } else if let Some(entry) = matching_key {
                    // Key-ID-only trust or no caller key: use manifest bytes.
                    if entry.algorithm != "ed25519" {
                        (None, true)
                    } else if let Ok(pub_bytes_vec) = hex::decode(&entry.key_bytes) {
                        if pub_bytes_vec.len() == 32 {
                            let mut raw = [0u8; 32];
                            raw.copy_from_slice(&pub_bytes_vec);
                            (Some(raw), true)
                        } else {
                            (None, true)
                        }
                    } else {
                        (None, true)
                    }
                } else {
                    (None, true)
                };

            if let Some(raw_pub) = verify_key_bytes {
                let vk =
                    crate::signing::VerifyingKey::from_bytes(raw_pub, sig_record.key_id.clone());

                let claim_bytes = manifest.claim.canonical_bytes();
                let result = vk.verify(&claim_bytes, &sig_bytes);

                let is_valid = result == crate::signing::SignatureResult::Valid;
                _any_signature_valid = _any_signature_valid || is_valid;

                let key_id_matched = match trust {
                    TrustPolicy::TrustNone => false,
                    TrustPolicy::TrustKeys(keys) => keys.iter().any(|t| t == &sig_record.key_id),
                    TrustPolicy::TrustCallback(f) => f(&sig_record.key_id),
                } || {
                    #[cfg(feature = "signatures")]
                    {
                        caller_verifying_keys
                            .iter()
                            .any(|t| t.key_id == sig_record.key_id)
                    }
                    #[cfg(not(feature = "signatures"))]
                    {
                        false
                    }
                };

                // Trusted requires: key ID matched AND cryptographically valid
                // AND (if caller-owned key was used) key material binding holds.
                let sig_trusted = key_id_matched && is_valid && key_material_matched;
                _any_signature_trusted = _any_signature_trusted || sig_trusted;

                builder = builder.add_signature(
                    SignatureVerification::builder()
                        .present(true)
                        .structurally_valid(true)
                        .cryptographically_valid(is_valid)
                        .public_key_id(sig_record.key_id.clone())
                        .key_id_matched(key_id_matched)
                        .key_material_matched(key_material_matched)
                        .trusted(sig_trusted)
                        .source(FieldSource::DetachedManifest)
                        .build(),
                );
            } else {
                builder = builder.add_signature(
                    SignatureVerification::builder()
                        .present(true)
                        .structurally_valid(false)
                        .key_material_matched(key_material_matched)
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
    }

    // 3. Trust evaluation is derived solely from the caller-supplied TrustPolicy.
    // The manifest's trust_metadata is never used to set the trust outcome.
    // A malicious manifest claiming `trusted: true` must not influence the
    // report. The overall trust reflects whether any signature was both
    // cryptographically valid AND matched a trusted key via the caller policy.
    let overall_trusted = _any_signature_trusted;
    if let Some(ref trust) = manifest.trust_metadata {
        builder = builder.with_trust(
            crate::verification::report::TrustEvaluation::builder()
                .trust_model(&trust.trust_model)
                .trusted(overall_trusted)
                .reason(if overall_trusted {
                    "caller-trusted key produced valid signature"
                } else {
                    "trust_metadata from manifest is informational only; no caller-trusted key produced a valid signature"
                })
                .build(),
        );
    } else if overall_trusted {
        builder = builder.with_trust(
            crate::verification::report::TrustEvaluation::builder()
                .trust_model("caller")
                .trusted(true)
                .reason("caller-trusted key produced valid signature")
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
            let mac_key = payload_mac_key.unwrap_or(&[]);

            // Use verify_and_extract_raw_from_bytes to get both the status and raw
            // payload bytes. This allows us to inspect the v3 header to distinguish
            // between missing and wrong HMAC keys.
            let (status, raw_bytes) =
                extractor.verify_and_extract_raw_from_bytes(image_bytes, mac_key);

            match status {
                crate::VerificationStatus::Verified => {
                    // Payload verified. Extract to check version and digest.
                    if let Some(payload) =
                        extractor.extract_payload_from_bytes_with_key(image_bytes, mac_key)
                    {
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
                                    EmbeddedReferenceStatus::PresentValid
                                }
                            }
                            None => EmbeddedReferenceStatus::Malformed,
                        }
                    } else {
                        EmbeddedReferenceStatus::Malformed
                    }
                }
                crate::VerificationStatus::Invalid => {
                    // Payload found but verification failed.
                    // Use raw bytes to determine if this is due to missing/wrong HMAC key.
                    if let Some(raw) = raw_bytes {
                        // Check if this is a v3 payload with HMAC auth
                        if raw.len() > 30 && raw[0] == 0x53 && raw[1] == 0x45 {
                            let auth_algo = raw[29];
                            if auth_algo == 2 {
                                // HMAC payload
                                if mac_key.is_empty() {
                                    EmbeddedReferenceStatus::AuthenticationKeyMissing
                                } else {
                                    EmbeddedReferenceStatus::AuthenticationFailed
                                }
                            } else {
                                // CRC payload — verification failed for other reasons
                                EmbeddedReferenceStatus::Malformed
                            }
                        } else {
                            EmbeddedReferenceStatus::Malformed
                        }
                    } else {
                        // No raw bytes available — payload not found
                        EmbeddedReferenceStatus::Stripped
                    }
                }
                crate::VerificationStatus::NotFound => EmbeddedReferenceStatus::Stripped,
            }
        }
    };

    ManifestVerification {
        report,
        instance_digest_match,
        manifest_valid,
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
    verify_detached_manifest_with_keys_and_mac(image_bytes, manifest, expected_keys, None)
}

/// Like [`verify_detached_manifest_with_keys`], but also verifies
/// embedded HMAC payload references when a MAC key is provided.
///
/// # Arguments
///
/// * `image_bytes` - Raw image bytes.
/// * `manifest` - The detached manifest to verify.
/// * `expected_keys` - Optional list of trusted public key identifiers.
///   If `None`, [`TrustPolicy::TrustNone`] is used.
/// * `payload_mac_key` - Optional HMAC key for embedded payload verification.
///
/// # Returns
///
/// A [`ManifestVerification`] with structured results.
#[must_use]
pub fn verify_detached_manifest_with_keys_and_mac(
    image_bytes: &[u8],
    manifest: &DetachedManifest,
    expected_keys: Option<&[Vec<u8>]>,
    payload_mac_key: Option<&[u8]>,
) -> ManifestVerification {
    let policy = match expected_keys {
        Some(keys) => TrustPolicy::TrustKeys(keys.to_vec()),
        None => TrustPolicy::TrustNone,
    };
    let limits = ResourceLimits::default();
    verify_detached_manifest_with_limits_and_mac(
        image_bytes,
        manifest,
        &policy,
        Some(&limits),
        payload_mac_key,
    )
}

/// Verify a detached manifest with full caller-owned key support.
///
/// This additive function accepts [`DetachedVerificationOptions`] which allows
/// the caller to supply:
/// - A [`TrustPolicy`] for key-ID-based trust evaluation
/// - Caller-owned verifying keys that bind key IDs to exact Ed25519 public key bytes
/// - An optional HMAC key for embedded payload verification
/// - Optional resource limits
///
/// When caller-owned verifying keys are provided, the manifest cannot substitute
/// alternative key bytes under a trusted key ID. This is the recommended function
/// for `verify-manifest --key` and similar CLI operations.
///
/// # Arguments
///
/// * `image_bytes` - Raw image bytes.
/// * `manifest` - The detached manifest to verify.
/// * `options` - Verification options including trust policy, caller keys, and limits.
///
/// # Returns
///
/// A [`ManifestVerification`] with structured results.
#[must_use]
pub fn verify_detached_manifest_with_options(
    image_bytes: &[u8],
    manifest: &DetachedManifest,
    options: &DetachedVerificationOptions<'_>,
) -> ManifestVerification {
    let default_limits = ResourceLimits::default();
    let limits = options.limits.unwrap_or(&default_limits);

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

    let default_policy = TrustPolicy::TrustNone;
    let trust = options.trust_policy.unwrap_or(&default_policy);

    #[cfg(feature = "signatures")]
    {
        verify_detached_manifest_inner(
            image_bytes,
            manifest,
            trust,
            options.payload_mac_key,
            options.caller_verifying_keys,
        )
    }
    #[cfg(not(feature = "signatures"))]
    {
        verify_detached_manifest_inner(image_bytes, manifest, trust, options.payload_mac_key, &[])
    }
}
