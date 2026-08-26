//! Application payload parsing, integrity checks, and verification classification.

use super::*;

impl SteganographyProtector {
    fn new_hmac(mac_key: &[u8]) -> HmacSha256 {
        HmacSha256::new_from_slice(mac_key)
            .unwrap_or_else(|_| unreachable!("HMAC accepts keys of every length"))
    }

    pub(crate) fn payload_within_limits(&self, bytes: &[u8]) -> bool {
        bytes.len() <= self.limits.max_payload_bytes()
    }

    /// Verify that an image contains a valid protection payload.
    ///
    /// Returns `VerificationStatus::Verified` if a payload is found and its
    /// checksum or HMAC is valid. For HMAC verification, use
    /// [`verify_payload_with_key`](Self::verify_payload_with_key).
    ///
    /// **Warning:** Without a MAC key, this method only checks a non-cryptographic
    /// CRC32 checksum that can be forged. For adversarial settings, always verify
    /// with a MAC key.
    pub fn verify_payload(&self, img: &DynamicImage) -> bool {
        self.verify_payload_with_key(img, &[]) == VerificationStatus::Verified
    }

    /// Verify protection using a MAC key for HMAC-SHA256 validation.
    ///
    /// Returns [`VerificationStatus::Verified`] if the payload is found and HMAC is valid,
    /// [`VerificationStatus::Invalid`] if found but HMAC doesn't match, or
    /// [`VerificationStatus::NotFound`] if no payload is found.
    ///
    /// Without a MAC key (empty `mac_key`), falls back to a non-cryptographic CRC32 checksum
    /// that provides no cryptographic protection. For production use, always provide a key.
    pub fn verify_payload_with_key(
        &self,
        img: &DynamicImage,
        mac_key: &[u8],
    ) -> VerificationStatus {
        // Encode once, delegate to bytes-aware method to avoid double-encoding.
        if let Ok(png_bytes) = crate::util::image::encode_image(img, image::ImageFormat::Png) {
            self.verify_payload_from_bytes_with_key(&png_bytes, mac_key)
        } else {
            VerificationStatus::NotFound
        }
    }

    /// Verify protection using raw image bytes — avoids unnecessary re-encoding.
    /// For JPEG bytes, checks DCT stego directly. For all formats, extracts
    /// metadata seed from the byte stream (preserves metadata that DynamicImage drops).
    pub fn verify_payload_from_bytes_with_key(
        &self,
        img_bytes: &[u8],
        mac_key: &[u8],
    ) -> VerificationStatus {
        match self.verify_payload_from_bytes_outcome(img_bytes, mac_key, false) {
            CandidateOutcome::Valid(_) => VerificationStatus::Verified,
            CandidateOutcome::Invalid(_)
            | CandidateOutcome::MalformedV3
            | CandidateOutcome::UnsupportedVersion(_)
            | CandidateOutcome::AuthenticationKeyMissing(_)
            | CandidateOutcome::AuthenticationFailed(_)
            | CandidateOutcome::ResourceLimitExceeded => VerificationStatus::Invalid,
            CandidateOutcome::NotFound => VerificationStatus::NotFound,
        }
    }

    fn verify_payload_from_bytes_outcome(
        &self,
        img_bytes: &[u8],
        mac_key: &[u8],
        suppress_unstructured_candidates: bool,
    ) -> CandidateOutcome {
        let metadata_seed = RightsMetadataProtector::extract_seed_from_image_with_limits(
            img_bytes,
            Some(&self.limits),
        );

        if img_bytes.starts_with(&[0xFF, 0xD8]) {
            let outcome = self.verify_extract_verified_dct(img_bytes, mac_key);
            if !matches!(&outcome, CandidateOutcome::NotFound) {
                return outcome;
            }

            if let Some(metadata_seed) = metadata_seed {
                let outcome = self.verify_extract_dct_with_seed(img_bytes, metadata_seed, mac_key);
                if !matches!(&outcome, CandidateOutcome::NotFound) {
                    return outcome;
                }
            }

            return CandidateOutcome::NotFound;
        }

        if let Ok(img) = image::load_from_memory(img_bytes) {
            if let Some(metadata_seed) = metadata_seed {
                let outcome = self.verify_payload_with_seed_outcome(&img, metadata_seed, mac_key);
                if !matches!(&outcome, CandidateOutcome::NotFound) {
                    return outcome;
                }
            }
            let rgba = img.to_rgba8();
            if let Some(fallback_seed) = Self::extract_seed_lsb_fallback(&rgba) {
                let outcome = self.verify_payload_with_seed_outcome(&img, fallback_seed, mac_key);
                let outcome = if suppress_unstructured_candidates {
                    match outcome {
                        CandidateOutcome::MalformedV3 | CandidateOutcome::UnsupportedVersion(_) => {
                            CandidateOutcome::NotFound
                        }
                        outcome => outcome,
                    }
                } else {
                    outcome
                };
                if !matches!(&outcome, CandidateOutcome::NotFound) {
                    return outcome;
                }
            }

            #[cfg(feature = "test-seeds")]
            for &seed in FALLBACK_SEEDS
                .iter()
                .take(self.limits.max_verification_seeds())
            {
                let outcome = self.verify_tiled_extraction_outcome(
                    &rgba,
                    seed,
                    DEFAULT_TILE_SIZE,
                    self.limits.max_tile_extraction_origins() as u32,
                    mac_key,
                );
                let outcome = if suppress_unstructured_candidates {
                    match outcome {
                        CandidateOutcome::Valid(payload)
                        | CandidateOutcome::Invalid(payload)
                        | CandidateOutcome::AuthenticationKeyMissing(payload)
                        | CandidateOutcome::AuthenticationFailed(payload)
                            if !Self::payload_is_structurally_plausible(&payload) =>
                        {
                            CandidateOutcome::NotFound
                        }
                        CandidateOutcome::MalformedV3 | CandidateOutcome::UnsupportedVersion(_) => {
                            CandidateOutcome::NotFound
                        }
                        outcome => outcome,
                    }
                } else {
                    outcome
                };
                if !matches!(&outcome, CandidateOutcome::NotFound) {
                    return outcome;
                }
            }
        }

        CandidateOutcome::NotFound
    }

    /// Verify protection and return raw payload bytes for embedded reference checks.
    ///
    /// Like [`Self::verify_payload_from_bytes_with_key`], but also returns the raw
    /// payload bytes when verification fails. This allows callers to inspect
    /// the v3 header (e.g., auth_algo) to distinguish between missing and
    /// wrong HMAC keys.
    ///
    /// Returns `(VerificationStatus, Option<Vec<u8>>)` where the second
    /// element contains the raw payload bytes when a payload was found but
    /// verification failed.
    pub fn verify_and_extract_raw_from_bytes(
        &self,
        img_bytes: &[u8],
        mac_key: &[u8],
    ) -> (VerificationStatus, Option<Vec<u8>>) {
        match self.verify_payload_from_bytes_outcome(img_bytes, mac_key, true) {
            CandidateOutcome::Valid(_) => (VerificationStatus::Verified, None),
            CandidateOutcome::Invalid(raw)
            | CandidateOutcome::AuthenticationKeyMissing(raw)
            | CandidateOutcome::AuthenticationFailed(raw) => {
                (VerificationStatus::Invalid, Some(raw))
            }
            CandidateOutcome::MalformedV3
            | CandidateOutcome::UnsupportedVersion(_)
            | CandidateOutcome::ResourceLimitExceeded => (VerificationStatus::Invalid, None),
            CandidateOutcome::NotFound => (VerificationStatus::NotFound, None),
        }
    }

    /// Verify protection from raw image bytes using a known seed.
    ///
    /// For JPEG bytes, verifies DCT payload integrity first, then falls back to
    /// pixel-based stego using the provided seed.
    pub fn verify_payload_from_bytes(&self, img_bytes: &[u8], seed: u64) -> bool {
        if img_bytes.starts_with(&[0xFF, 0xD8]) {
            if let Some(payload_bytes) = self.extract_verified_dct_payload(img_bytes, &[]) {
                let header = if let Some(decoded) = Self::try_ecc_decode(&payload_bytes) {
                    decoded
                } else {
                    payload_bytes
                };

                if let Some(embedded_seed) = Self::extract_embedded_seed(&header) {
                    if embedded_seed == seed {
                        return true;
                    }
                }
            }
        }

        if let Ok(img) = image::load_from_memory(img_bytes) {
            return self.verify_payload_with_seed(&img, seed);
        }

        false
    }

    /// Verify protection using a known seed.
    ///
    /// Extracts the LSB payload with the given seed and checks both the checksum
    /// and the embedded seed value. Metadata is only available through the byte APIs.
    pub fn verify_payload_with_seed(&self, img: &DynamicImage, seed: u64) -> bool {
        let rgba = img.to_rgba8();

        if let Some(payload) = self.extract_with_redundancy(&rgba, seed, &[]) {
            let header = if let Some(decoded) = Self::try_ecc_decode(&payload) {
                decoded
            } else {
                payload.clone()
            };
            if Self::verify_checksum(&payload) {
                if let Some(embedded_seed) = Self::extract_embedded_seed(&header) {
                    if embedded_seed == seed {
                        return true;
                    }
                }
            }
        }

        // Crop-resistant path: try tiled extraction as a final fallback.
        // Tiled embedding produces multiple full copies of the payload, so a
        // crop that destroys most pixels can still leave one intact tile.
        if self.try_tiled_extraction_verify(
            &rgba,
            seed,
            DEFAULT_TILE_SIZE,
            self.limits.max_tile_extraction_origins() as u32,
            &[],
        ) {
            return true;
        }

        false
    }

    /// Check if bytes begin with v3 magic `[0x53, 0x45, 0x03]`.
    pub(crate) fn has_v3_magic(bytes: &[u8]) -> bool {
        bytes.len() >= 3
            && bytes[0] == V3_MAGIC[0]
            && bytes[1] == V3_MAGIC[1]
            && bytes[2] == V3_PAYLOAD_VERSION
    }

    pub(crate) fn verify_embedded_seed_matches(payload: &[u8], expected_seed: u64) -> bool {
        let header = if let Some(decoded) = Self::try_ecc_decode(payload) {
            decoded
        } else {
            payload.to_vec()
        };
        if header.len() < 10 {
            return false;
        }

        if header.len() >= 3
            && header[0] == V3_MAGIC[0]
            && header[1] == V3_MAGIC[1]
            && header[2] == V3_PAYLOAD_VERSION
        {
            if header.len() < 19 {
                return false;
            }
            let embedded_seed = u64::from_le_bytes([
                header[11], header[12], header[13], header[14], header[15], header[16], header[17],
                header[18],
            ]);
            return embedded_seed == expected_seed;
        }

        let embedded_seed = u64::from_le_bytes([
            header[2], header[3], header[4], header[5], header[6], header[7], header[8], header[9],
        ]);
        embedded_seed == expected_seed
    }

    pub(crate) fn truncate_to_actual_payload(payload: &[u8]) -> Vec<u8> {
        if payload.len() >= 3
            && payload[0] == V3_MAGIC[0]
            && payload[1] == V3_MAGIC[1]
            && payload[2] == V3_PAYLOAD_VERSION
            && payload.len() >= crate::payload_v3::types::V3_CORE_SIZE
        {
            let total_length = u16::from_le_bytes([payload[4], payload[5]]) as usize;
            if total_length <= payload.len() {
                return payload[..total_length].to_vec();
            }
        }
        payload.to_vec()
    }

    pub(crate) fn parse_stego_payload(payload: &[u8]) -> Option<StegoPayload> {
        if payload.len() < 24 {
            return None;
        }

        // V3: check magic bytes first
        if payload.len() >= 3
            && payload[0] == V3_MAGIC[0]
            && payload[1] == V3_MAGIC[1]
            && payload[2] == V3_PAYLOAD_VERSION
        {
            return Self::parse_stego_payload_v3(payload);
        }

        let version = payload[0];

        for &supported in SUPPORTED_PAYLOAD_VERSIONS {
            if version == supported {
                return match supported {
                    1 => Self::parse_stego_payload_v1(payload),
                    2 => Self::parse_stego_payload_v2(payload),
                    _ => None,
                };
            }
        }

        None
    }

    #[cfg(feature = "test-seeds")]
    pub(crate) fn payload_is_structurally_plausible(payload: &[u8]) -> bool {
        let decoded = Self::try_ecc_decode(payload).unwrap_or_else(|| payload.to_vec());
        Self::parse_stego_payload(&decoded).is_some()
    }

    pub(crate) fn parse_stego_payload_v3(payload: &[u8]) -> Option<StegoPayload> {
        if payload.len() < crate::payload_v3::types::V3_CORE_SIZE {
            return None;
        }

        let extracted_seed = u64::from_le_bytes([
            payload[11],
            payload[12],
            payload[13],
            payload[14],
            payload[15],
            payload[16],
            payload[17],
            payload[18],
        ]);

        let intensity_raw = u16::from_le_bytes([payload[19], payload[20]]);
        let intensity = intensity_raw as f32 / 100.0;

        let content_hash = {
            let mut hash = [0u8; 4];
            hash.copy_from_slice(&payload[21..25]);
            if hash != [0, 0, 0, 0] {
                Some(hash)
            } else {
                None
            }
        };

        let dmi_value = match payload[10] {
            0 => None,
            1 => Some(crate::types::DmiValue::Allowed),
            2 => Some(crate::types::DmiValue::ProhibitedAiMlTraining),
            3 => Some(crate::types::DmiValue::ProhibitedGenAiMlTraining),
            4 => Some(crate::types::DmiValue::ProhibitedExceptSearchEngineIndexing),
            5 => Some(crate::types::DmiValue::Prohibited),
            6 => Some(crate::types::DmiValue::ProhibitedSeeConstraints),
            _ => None,
        };

        Some(StegoPayload {
            protection_level: 2,
            seed: extracted_seed,
            intensity,
            version: 3,
            content_hash,
            dmi_value,
            raw_payload: None,
        })
    }

    pub(crate) fn compute_payload_mac(payload_without_mac: &[u8], mac_key: &[u8]) -> [u8; 8] {
        let mut mac = Self::new_hmac(mac_key);
        mac.update(payload_without_mac);
        let result = mac.finalize().into_bytes();
        [
            result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7],
        ]
    }

    pub(crate) fn compute_payload_mac_v3(payload_without_mac: &[u8], mac_key: &[u8]) -> [u8; 16] {
        let mut mac = Self::new_hmac(mac_key);
        mac.update(crate::payload_v3::types::V3_DOMAIN_STRING);
        mac.update(payload_without_mac);
        let result = mac.finalize().into_bytes();
        let mut out = [0u8; 16];
        out.copy_from_slice(&result[..16]);
        out
    }

    pub(crate) fn verify_payload_mac(
        payload_without_mac: &[u8],
        mac_key: &[u8],
        expected_mac: &[u8],
    ) -> bool {
        let computed_mac = Self::compute_payload_mac(payload_without_mac, mac_key);
        computed_mac.ct_eq(expected_mac).into()
    }

    /// Computes a CRC32 checksum of the data, stored as 4 bytes (little-endian).
    ///
    /// CRC32 provides strong accidental corruption detection. For this library's
    /// legal deterrence use case, forgability is irrelevant — the goal is detection,
    /// not authentication. Use `verify_payload_integrity` with a non-empty `mac_key`
    /// for HMAC-based verification when needed.
    pub(crate) fn compute_checksum(data: &[u8]) -> [u8; 4] {
        let mut hasher = Crc32Hasher::new();
        hasher.update(data);
        hasher.finalize().to_le_bytes()
    }

    /// Verifies the CRC32 checksum of the ECC-encoded payload.
    /// Tries v2 ECC size (96 data + 4 checksum = 100 bytes) first,
    /// then v1 (72 data + 4 checksum = 76 bytes).
    pub(crate) fn verify_checksum(payload: &[u8]) -> bool {
        // V3 payload: CRC32 over core header
        if payload.len() >= 3
            && payload[0] == V3_MAGIC[0]
            && payload[1] == V3_MAGIC[1]
            && payload[2] == V3_PAYLOAD_VERSION
        {
            if payload.len() < crate::payload_v3::types::V3_CORE_SIZE + 4 {
                return false;
            }
            let total_length = u16::from_le_bytes([payload[4], payload[5]]) as usize;
            if total_length > payload.len() {
                return false;
            }
            let auth_tag_len = payload[30] as usize;
            if auth_tag_len < 4 {
                return false;
            }
            if total_length < crate::payload_v3::types::V3_CORE_SIZE + auth_tag_len {
                return false;
            }
            let core_and_ext = &payload[..total_length - auth_tag_len];
            let expected = Self::compute_checksum(core_and_ext);
            payload[total_length - auth_tag_len] == expected[0]
                && payload[total_length - auth_tag_len + 1] == expected[1]
                && payload[total_length - auth_tag_len + 2] == expected[2]
                && payload[total_length - auth_tag_len + 3] == expected[3]
        }
        // V2 ECC size first
        else {
            let v2_ecc_len = V2_HEADER_SIZE * ecc::REPLICATION_FACTOR;
            if payload.len() >= v2_ecc_len + 4 {
                let expected = Self::compute_checksum(&payload[..v2_ecc_len]);
                if payload[v2_ecc_len] == expected[0]
                    && payload[v2_ecc_len + 1] == expected[1]
                    && payload[v2_ecc_len + 2] == expected[2]
                    && payload[v2_ecc_len + 3] == expected[3]
                {
                    return true;
                }
            }
            // Try v1 ECC size
            if payload.len() >= ecc::TOTAL_ECC_LEN + 4 {
                let expected = Self::compute_checksum(&payload[..ecc::TOTAL_ECC_LEN]);
                if payload[ecc::TOTAL_ECC_LEN] == expected[0]
                    && payload[ecc::TOTAL_ECC_LEN + 1] == expected[1]
                    && payload[ecc::TOTAL_ECC_LEN + 2] == expected[2]
                    && payload[ecc::TOTAL_ECC_LEN + 3] == expected[3]
                {
                    return true;
                }
            }
            // Legacy: try 24-byte header + 4 checksum
            if payload.len() >= MIN_PAYLOAD_SIZE {
                let expected = Self::compute_checksum(&payload[..24]);
                if payload[24] == expected[0]
                    && payload[25] == expected[1]
                    && payload[26] == expected[2]
                    && payload[27] == expected[3]
                {
                    return true;
                }
            }
            false
        }
    }

    pub(crate) fn verify_payload_integrity(payload: &[u8], mac_key: &[u8]) -> bool {
        // V3 payload: check magic bytes
        if payload.len() >= 3
            && payload[0] == V3_MAGIC[0]
            && payload[1] == V3_MAGIC[1]
            && payload[2] == V3_PAYLOAD_VERSION
        {
            if payload.len() < crate::payload_v3::types::V3_CORE_SIZE {
                return false;
            }
            let total_length = u16::from_le_bytes([payload[4], payload[5]]) as usize;
            if total_length > payload.len() {
                return false;
            }
            let auth_algo = payload[29];
            let auth_tag_len = payload[30] as usize;
            if total_length < crate::payload_v3::types::V3_CORE_SIZE + auth_tag_len {
                return false;
            }
            // The declared tag length must match the algorithm exactly; a
            // zero-length or truncated HMAC tag would otherwise authenticate
            // with any key (empty prefix compares equal to an empty slice).
            match auth_algo {
                1 if auth_tag_len == 4 => {
                    let core_and_ext = &payload[..total_length - auth_tag_len];
                    let tag = &payload[total_length - auth_tag_len..total_length];
                    let expected = Self::compute_checksum(core_and_ext);
                    tag == expected
                }
                2 if auth_tag_len == 16 && !mac_key.is_empty() => {
                    let core_and_ext = &payload[..total_length - auth_tag_len];
                    let tag = &payload[total_length - auth_tag_len..total_length];
                    let expected = Self::compute_payload_mac_v3(core_and_ext, mac_key);
                    let Some(expected_prefix) = expected.get(..tag.len()) else {
                        return false;
                    };
                    let authenticated = expected_prefix.ct_eq(tag).into();
                    if authenticated {
                        true
                    } else {
                        let mut legacy_mac = Self::new_hmac(mac_key);
                        legacy_mac.update(core_and_ext);
                        let result = legacy_mac.finalize().into_bytes();
                        result
                            .get(..tag.len())
                            .is_some_and(|legacy_prefix| legacy_prefix.ct_eq(tag).into())
                    }
                }
                _ => false,
            }
        } else if mac_key.is_empty() {
            Self::verify_checksum(payload)
        } else {
            // Try v2 MAC: 32-byte header + 8-byte MAC = 40 bytes
            if payload.len() >= V2_HEADER_SIZE + 8 {
                Self::verify_payload_mac(
                    &payload[..V2_HEADER_SIZE],
                    mac_key,
                    &payload[V2_HEADER_SIZE..V2_HEADER_SIZE + 8],
                )
            }
            // Legacy v1 MAC: 24-byte header + 8-byte MAC = 32 bytes
            else if payload.len() >= 32 {
                Self::verify_payload_mac(&payload[..24], mac_key, &payload[24..32])
            } else {
                false
            }
        }
    }

    /// Classify an invalid payload based on v3 auth_algo and provided key.
    ///
    /// Returns `AuthenticationKeyMissing` if the payload uses HMAC but no key
    /// was provided, `AuthenticationFailed` if the payload uses HMAC but the
    /// key was wrong, or `Invalid` for other failures.
    pub(crate) fn classify_auth_failure(payload: &[u8], mac_key: &[u8]) -> CandidateOutcome {
        if payload.len() > 30 && Self::has_v3_magic(payload) {
            let auth_algo = payload[29];
            if auth_algo == 2 {
                if mac_key.is_empty() {
                    CandidateOutcome::AuthenticationKeyMissing(payload.to_vec())
                } else {
                    CandidateOutcome::AuthenticationFailed(payload.to_vec())
                }
            } else {
                CandidateOutcome::Invalid(payload.to_vec())
            }
        } else {
            CandidateOutcome::Invalid(payload.to_vec())
        }
    }

    /// Classify the first 6 bytes of a potential v3 payload.
    ///
    /// Reads only magic (2), version (1), header_length (1), total_length (2).
    /// Returns `Detected` with declared sizes when the prefix is valid.
    /// Does NOT validate the full header — call `validate_v3_header` next.
    pub(crate) fn classify_v3_prefix(
        prefix: &[u8],
        limits: Option<&crate::resource_limits::ResourceLimits>,
    ) -> V3PrefixResult {
        if prefix.len() < V3_PREFIX_BYTES {
            return V3PrefixResult::NotV3;
        }
        if prefix[0] != V3_MAGIC[0] || prefix[1] != V3_MAGIC[1] {
            return V3PrefixResult::NotV3;
        }
        if prefix[2] != V3_PAYLOAD_VERSION {
            if prefix[2] > V3_PAYLOAD_VERSION {
                return V3PrefixResult::UnsupportedVersion(prefix[2]);
            }
            return V3PrefixResult::NotV3;
        }
        let header_length = prefix[3] as usize;
        let total_length = u16::from_le_bytes([prefix[4], prefix[5]]) as usize;
        if header_length < crate::payload_v3::types::V3_CORE_SIZE {
            return V3PrefixResult::Malformed(PayloadMalformedReason::HeaderTooShort);
        }
        if total_length < header_length {
            return V3PrefixResult::Malformed(PayloadMalformedReason::TotalLessThanHeader);
        }
        if total_length > crate::payload_v3::types::V3_MAX_EMBEDDED_SIZE {
            return V3PrefixResult::Malformed(PayloadMalformedReason::TotalExceedsWireMax);
        }
        if let Some(limits) = limits {
            if total_length > limits.max_payload_bytes() {
                return V3PrefixResult::ResourceLimitExceeded;
            }
        }
        V3PrefixResult::Detected {
            header_length,
            total_length,
        }
    }

    /// Validate a full v3 header (beyond the6-byte prefix).
    ///
    /// `header_bytes` must be exactly `header_length` bytes long (the full
    /// declared header, including the prefix). Returns a [`ValidatedV3Header`]
    /// on success, or a [`V3PrefixResult`] error on any validation failure.
    pub(crate) fn validate_v3_header(
        header_bytes: &[u8],
        limits: Option<&crate::resource_limits::ResourceLimits>,
    ) -> std::result::Result<ValidatedV3Header, V3PrefixResult> {
        if header_bytes.len() < V3_PREFIX_BYTES {
            return Err(V3PrefixResult::NotV3);
        }
        let header_length = header_bytes[3] as usize;
        let total_length = u16::from_le_bytes([header_bytes[4], header_bytes[5]]) as usize;
        if header_bytes.len() < header_length {
            return Err(V3PrefixResult::Malformed(
                PayloadMalformedReason::HeaderLengthMismatch,
            ));
        }
        if let Some(limits) = limits {
            if total_length > limits.max_payload_bytes() {
                return Err(V3PrefixResult::ResourceLimitExceeded);
            }
        }
        let total_bits = total_length
            .checked_mul(8)
            .ok_or(V3PrefixResult::Malformed(
                PayloadMalformedReason::BitOverflow,
            ))?;
        let _ = total_bits;

        let auth_tag_length = if header_bytes.len() > 30 {
            let tag_len = header_bytes[30] as usize;
            if total_length < crate::payload_v3::types::V3_CORE_SIZE + tag_len {
                return Err(V3PrefixResult::Malformed(
                    PayloadMalformedReason::AuthTagLengthInvalid,
                ));
            }
            tag_len
        } else {
            0
        };

        let key_id_length = if header_bytes.len() > 31 {
            let kid_len = header_bytes[31] as usize;
            if kid_len > crate::payload_v3::types::V3_MAX_KEY_ID_LEN {
                return Err(V3PrefixResult::Malformed(
                    PayloadMalformedReason::KeyIdTooLong,
                ));
            }
            if header_length < crate::payload_v3::types::V3_CORE_SIZE + kid_len {
                return Err(V3PrefixResult::Malformed(
                    PayloadMalformedReason::HeaderTooShortForKeyId,
                ));
            }
            kid_len
        } else {
            0
        };

        let auth_algorithm = if header_bytes.len() > 29 {
            let algo = header_bytes[29];
            AuthAlgorithm::from_byte(algo).ok_or(V3PrefixResult::Malformed(
                PayloadMalformedReason::InvalidAuthAlgorithm,
            ))?
        } else {
            AuthAlgorithm::Crc32
        };

        let channels = if header_bytes.len() >= 10 {
            let channel_bits = u16::from_le_bytes([header_bytes[8], header_bytes[9]]);
            ProtectionChannels::from_bits(channel_bits).ok_or(V3PrefixResult::Malformed(
                PayloadMalformedReason::InvalidChannels,
            ))?
        } else {
            ProtectionChannels {
                rights_metadata: true,
                hidden_marker: true,
                authentication: true,
            }
        };

        let flags = if header_bytes.len() >= 8 {
            let flag_bits = u16::from_le_bytes([header_bytes[6], header_bytes[7]]);
            crate::payload_v3::types::PayloadFlags::from_bits(flag_bits)
        } else {
            crate::payload_v3::types::PayloadFlags::from_bits(0)
        };

        let has_extension_bytes =
            header_length > crate::payload_v3::types::V3_CORE_SIZE + key_id_length;
        if flags.has_extensions != has_extension_bytes {
            return Err(V3PrefixResult::Malformed(
                PayloadMalformedReason::ExtensionFlagMismatch,
            ));
        }

        Ok(ValidatedV3Header {
            header_length,
            total_length,
            auth_algorithm,
            auth_tag_length,
            key_id_length,
            channels,
            flags,
        })
    }

    /// Classify a set of extracted bytes as v3 header, legacy, or malformed.
    ///
    /// The bytes must be at least 6 bytes (to read `total_length` from
    /// bytes 4-5). Returns the exact `header_length` and `total_length`
    /// when v3 magic is present and the header is structurally valid.
    ///
    /// When `limits` is provided, validates `total_length` against
    /// `limits.max_payload_bytes()` before returning `V3Detected`.
    pub(crate) fn classify_v3_probe(
        bytes: &[u8],
        limits: Option<&crate::resource_limits::ResourceLimits>,
    ) -> V3ProbeResult {
        if bytes.len() < 6 {
            return V3ProbeResult::NotV3;
        }
        if bytes[0] != V3_MAGIC[0] || bytes[1] != V3_MAGIC[1] {
            return V3ProbeResult::NotV3;
        }
        if bytes[2] != V3_PAYLOAD_VERSION {
            if bytes[2] > V3_PAYLOAD_VERSION {
                return V3ProbeResult::UnsupportedVersion(bytes[2]);
            }
            return V3ProbeResult::NotV3;
        }
        let header_length = bytes[3] as usize;
        let total_length = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        if header_length < crate::payload_v3::types::V3_CORE_SIZE {
            return V3ProbeResult::MalformedV3;
        }
        if total_length < header_length {
            return V3ProbeResult::MalformedV3;
        }
        if total_length > crate::payload_v3::types::V3_MAX_EMBEDDED_SIZE {
            return V3ProbeResult::MalformedV3;
        }
        if let Some(limits) = limits {
            if total_length > limits.max_payload_bytes() {
                return V3ProbeResult::ResourceLimitExceeded;
            }
        }
        let total_bits = match total_length.checked_mul(8) {
            Some(b) => b,
            None => return V3ProbeResult::MalformedV3,
        };
        if bytes.len() >= 30 {
            let auth_algo = bytes[29];
            if crate::payload_v3::types::AuthAlgorithm::from_byte(auth_algo).is_none() {
                return V3ProbeResult::MalformedV3;
            }
        }
        if bytes.len() >= 31 {
            let auth_tag_len = bytes[30] as usize;
            if total_length < crate::payload_v3::types::V3_CORE_SIZE + auth_tag_len {
                return V3ProbeResult::MalformedV3;
            }
        }
        if bytes.len() >= 32 {
            let key_id_len = bytes[31] as usize;
            if key_id_len > crate::payload_v3::types::V3_MAX_KEY_ID_LEN {
                return V3ProbeResult::MalformedV3;
            }
            if header_length < crate::payload_v3::types::V3_CORE_SIZE + key_id_len {
                return V3ProbeResult::MalformedV3;
            }
            let has_extension_bytes =
                header_length > crate::payload_v3::types::V3_CORE_SIZE + key_id_len;
            if (bytes[6] & 0x01 != 0) != has_extension_bytes {
                return V3ProbeResult::MalformedV3;
            }
        }
        V3ProbeResult::V3Detected {
            header_length,
            total_length,
            total_bits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v3_payload(total_length: usize, auth_algo: u8, auth_tag_len: u8) -> Vec<u8> {
        let mut payload = vec![0u8; total_length];
        payload[0] = V3_MAGIC[0];
        payload[1] = V3_MAGIC[1];
        payload[2] = V3_PAYLOAD_VERSION;
        payload[3] = crate::payload_v3::types::V3_CORE_SIZE as u8;
        payload[4..6].copy_from_slice(&(total_length as u16).to_le_bytes());
        payload[29] = auth_algo;
        payload[30] = auth_tag_len;
        payload
    }

    #[test]
    fn verify_checksum_rejects_undersized_auth_tag() {
        for tag_len in [0u8, 1, 2, 3] {
            let payload = v3_payload(36, 1, tag_len);
            assert!(!SteganographyProtector::verify_checksum(&payload));
        }
    }

    #[test]
    fn verify_integrity_rejects_oversized_hmac_tag() {
        for tag_len in [17u8, 32, 200] {
            let total = crate::payload_v3::types::V3_CORE_SIZE + tag_len as usize;
            let payload = v3_payload(total, 2, tag_len);
            assert!(!SteganographyProtector::verify_payload_integrity(
                &payload, b"mac-key"
            ));
        }
    }

    #[test]
    fn verify_accepts_wellformed_crc32_payload() {
        let mut payload = v3_payload(36, 1, 4);
        let checksum = SteganographyProtector::compute_checksum(&payload[..32]);
        payload[32..36].copy_from_slice(&checksum);
        assert!(SteganographyProtector::verify_checksum(&payload));
        assert!(SteganographyProtector::verify_payload_integrity(
            &payload,
            &[]
        ));
    }

    #[test]
    fn verify_accepts_wellformed_hmac_payload_and_rejects_wrong_key() {
        let key = b"secret-key";
        let mut payload = v3_payload(48, 2, 16);
        let mac = SteganographyProtector::compute_payload_mac_v3(&payload[..32], key);
        payload[32..48].copy_from_slice(&mac);
        assert!(SteganographyProtector::verify_payload_integrity(
            &payload, key
        ));

        payload[40] ^= 0xFF;
        assert!(!SteganographyProtector::verify_payload_integrity(
            &payload, key
        ));
    }

    #[test]
    fn verify_integrity_rejects_hmac_with_empty_tag_for_any_key() {
        let total = crate::payload_v3::types::V3_CORE_SIZE;
        let payload = v3_payload(total, 2, 0);
        for key in [&b"secret-key"[..], b"other-key", b""] {
            assert!(
                !SteganographyProtector::verify_payload_integrity(&payload, key),
                "HMAC payload with auth_tag_len=0 must not authenticate"
            );
        }
    }

    #[test]
    fn verify_integrity_rejects_truncated_hmac_tags() {
        let key = b"secret-key";
        for tag_len in [1u8, 8, 15] {
            let total = crate::payload_v3::types::V3_CORE_SIZE + tag_len as usize;
            let mut payload = v3_payload(total, 2, tag_len);
            let mac = SteganographyProtector::compute_payload_mac_v3(&payload[..32], key);
            let start = crate::payload_v3::types::V3_CORE_SIZE;
            payload[start..total].copy_from_slice(&mac[..tag_len as usize]);
            assert!(!SteganographyProtector::verify_payload_integrity(
                &payload, key
            ));
        }
    }

    #[test]
    fn verify_integrity_rejects_crc32_with_mismatched_tag_length() {
        let mut payload = v3_payload(36, 1, 4);
        let checksum = SteganographyProtector::compute_checksum(&payload[..32]);
        payload[32..36].copy_from_slice(&checksum);
        assert!(SteganographyProtector::verify_payload_integrity(
            &payload,
            &[]
        ));

        for tag_len in [0u8, 2, 5] {
            let total = crate::payload_v3::types::V3_CORE_SIZE + tag_len as usize;
            let payload = v3_payload(total, 1, tag_len);
            assert!(!SteganographyProtector::verify_payload_integrity(
                &payload,
                &[]
            ));
        }
    }

    #[test]
    fn validate_v3_header_rejects_reserved_channel_bits() {
        let mut header = v3_payload(crate::payload_v3::types::V3_CORE_SIZE, 0, 0);
        header[9] = 0x04;
        assert!(matches!(
            SteganographyProtector::validate_v3_header(&header, None),
            Err(V3PrefixResult::Malformed(
                PayloadMalformedReason::InvalidChannels
            ))
        ));
    }

    #[test]
    fn validate_v3_header_accepts_defined_channel_bits() {
        let header = v3_payload(crate::payload_v3::types::V3_CORE_SIZE, 0, 0);
        assert!(SteganographyProtector::validate_v3_header(&header, None).is_ok());
    }
}
