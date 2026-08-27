//! Seed discovery, bounded carrier search, and extraction orchestration.

use super::*;

impl SteganographyProtector {
    /// Extract and validate payload candidates from tiled F5 DCT stego.
    #[doc(hidden)]
    pub fn extract_f5_tiled_candidates(
        &self,
        jpeg_bytes: &[u8],
        master_seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        if max_origins == 0 {
            return None;
        }
        let prefix_bits = V3_PREFIX_BYTES * 8;
        let search =
            stegoeggo_stego::application_support::TiledJpegSearch::new(jpeg_bytes, tile_size)?;
        let candidates = search.prefix_candidates(master_seed, max_origins, prefix_bits);
        if candidates.is_empty() {
            return None;
        }
        match self.evaluate_tiled_candidates(candidates, mac_key, |candidate, bits| {
            search.extract_candidate(master_seed, candidate, bits)
        }) {
            CandidateOutcome::Valid(payload) => Some(payload),
            CandidateOutcome::Invalid(_)
            | CandidateOutcome::MalformedV3
            | CandidateOutcome::UnsupportedVersion(_)
            | CandidateOutcome::AuthenticationKeyMissing(_)
            | CandidateOutcome::AuthenticationFailed(_)
            | CandidateOutcome::ResourceLimitExceeded
            | CandidateOutcome::NotFound => None,
        }
    }

    pub(crate) fn evaluate_tiled_candidates<I, F>(
        &self,
        candidates: I,
        mac_key: &[u8],
        mut extract: F,
    ) -> CandidateOutcome
    where
        I: IntoIterator<
            Item = (
                stegoeggo_stego::application_support::TiledJpegCandidateKey,
                Vec<u8>,
            ),
        >,
        F: FnMut(
            stegoeggo_stego::application_support::TiledJpegCandidateKey,
            usize,
        ) -> Option<Vec<u8>>,
    {
        let mut last_outcome = None;
        for (candidate, prefix) in candidates {
            let outcome = match Self::classify_v3_prefix(&prefix, Some(&self.limits)) {
                V3PrefixResult::Detected {
                    header_length,
                    total_length,
                } => {
                    let Some(header) = extract(candidate, header_length * 8) else {
                        continue;
                    };
                    if Self::validate_v3_header(&header, Some(&self.limits)).is_err() {
                        CandidateOutcome::MalformedV3
                    } else {
                        let Some(full) = extract(candidate, total_length * 8) else {
                            continue;
                        };
                        if Self::verify_payload_integrity(&full, mac_key) {
                            CandidateOutcome::Valid(Self::truncate_to_actual_payload(&full))
                        } else {
                            Self::classify_auth_failure(&full, mac_key)
                        }
                    }
                }
                V3PrefixResult::NotV3 => {
                    let mut legacy_outcome = None;
                    for bits in [ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        let Some(payload) = extract(candidate, bits) else {
                            continue;
                        };
                        if Self::try_ecc_decode(&payload).is_some() {
                            return CandidateOutcome::Valid(payload);
                        }
                        if legacy_outcome.is_none() {
                            legacy_outcome = Some(Self::classify_auth_failure(&payload, mac_key));
                        }
                    }
                    legacy_outcome.unwrap_or(CandidateOutcome::NotFound)
                }
                V3PrefixResult::Malformed(_) => CandidateOutcome::MalformedV3,
                V3PrefixResult::ResourceLimitExceeded => CandidateOutcome::ResourceLimitExceeded,
                V3PrefixResult::UnsupportedVersion(version) => {
                    CandidateOutcome::UnsupportedVersion(version)
                }
            };
            if matches!(outcome, CandidateOutcome::Valid(_)) {
                return outcome;
            }
            if last_outcome.is_none() && !matches!(outcome, CandidateOutcome::NotFound) {
                last_outcome = Some(outcome);
            }
        }
        last_outcome.unwrap_or(CandidateOutcome::NotFound)
    }

    pub(crate) fn verify_extract_f5_tiled(
        &self,
        jpeg_bytes: &[u8],
        master_seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        if max_origins == 0 {
            return CandidateOutcome::NotFound;
        }
        let prefix_bits = V3_PREFIX_BYTES * 8;
        let Some(search) =
            stegoeggo_stego::application_support::TiledJpegSearch::new(jpeg_bytes, tile_size)
        else {
            return CandidateOutcome::NotFound;
        };
        let candidates = search.prefix_candidates(master_seed, max_origins, prefix_bits);
        self.evaluate_tiled_candidates(candidates, mac_key, |candidate, bits| {
            search.extract_candidate(master_seed, candidate, bits)
        })
    }

    pub(crate) fn extract_with_redundancy(
        &self,
        img: &RgbaImage,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        let prefix_bits = 6 * 8;
        for redundancy in 1..=10 {
            if let Some(result) =
                self.extract_payload_at_seed_v2(img, prefix_bits, seed, mac_key, redundancy)
            {
                return Some(result);
            }
        }
        if let Some(result) = self.extract_payload_at_seed_legacy(img, prefix_bits, seed, mac_key) {
            return Some(result);
        }
        for pass in 0..5 {
            let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));
            for redundancy in 1..=10 {
                if let Some(result) = self.extract_payload_at_seed_v2(
                    img,
                    prefix_bits,
                    offset_seed,
                    mac_key,
                    redundancy,
                ) {
                    return Some(result);
                }
            }
            if let Some(result) =
                self.extract_payload_at_seed_legacy(img, prefix_bits, offset_seed, mac_key)
            {
                return Some(result);
            }
        }
        None
    }

    pub(crate) fn extract_payload_at_seed_v2(
        &self,
        img: &RgbaImage,
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
        redundancy: usize,
    ) -> Option<Vec<u8>> {
        if let Some(prefix) = self.extract_lsb_v2(img, prefix_bits, seed, redundancy) {
            match Self::classify_v3_probe(&prefix, Some(&self.limits)) {
                V3ProbeResult::V3Detected { total_bits, .. } => {
                    if let Some(full) = self.extract_lsb_v2(img, total_bits, seed, redundancy) {
                        if Self::verify_payload_integrity(&full, mac_key) {
                            return Some(Self::truncate_to_actual_payload(&full));
                        }
                    }
                }
                V3ProbeResult::NotV3 => {
                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        if let Some(payload) = self.extract_lsb_v2(img, ecc_bits, seed, redundancy)
                        {
                            if Self::try_ecc_decode(&payload).is_some() {
                                return Some(payload);
                            }
                            if Self::verify_payload_integrity(&payload, mac_key) {
                                return Some(Self::truncate_to_actual_payload(&payload));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub(crate) fn extract_payload_at_seed_legacy(
        &self,
        img: &RgbaImage,
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        if let Some(prefix) = self.extract_lsb(img, prefix_bits, seed) {
            match Self::classify_v3_probe(&prefix, Some(&self.limits)) {
                V3ProbeResult::V3Detected { total_bits, .. } => {
                    if let Some(full) = self.extract_lsb(img, total_bits, seed) {
                        if Self::verify_payload_integrity(&full, mac_key) {
                            return Some(Self::truncate_to_actual_payload(&full));
                        }
                    }
                }
                V3ProbeResult::NotV3 => {
                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        if let Some(payload) = self.extract_lsb(img, ecc_bits, seed) {
                            if Self::try_ecc_decode(&payload).is_some() {
                                return Some(payload);
                            }
                            if Self::verify_payload_integrity(&payload, mac_key) {
                                return Some(Self::truncate_to_actual_payload(&payload));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Verification-path variant of `extract_with_redundancy` that returns a
    /// tri-state. Used by [`Self::verify_payload_from_bytes_with_key`] so that
    /// a structurally plausible payload whose integrity check fails (e.g.
    /// wrong MAC key, bit corruption) can be reported as
    /// [`VerificationStatus::Invalid`] instead of falling through to `NotFound`.
    pub(crate) fn verify_extract_with_redundancy(
        &self,
        img: &RgbaImage,
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        let mut last_outcome: Option<CandidateOutcome> = None;
        let prefix_bits = 6 * 8;
        for redundancy in 1..=10 {
            let v2_corrected =
                self.verify_extract_at_seed_v2(img, prefix_bits, seed, mac_key, redundancy);
            match &v2_corrected {
                CandidateOutcome::Valid(_)
                | CandidateOutcome::Invalid(_)
                | CandidateOutcome::AuthenticationKeyMissing(_)
                | CandidateOutcome::AuthenticationFailed(_)
                | CandidateOutcome::ResourceLimitExceeded => {
                    if Self::candidate_seed_matches(&v2_corrected, seed) {
                        return v2_corrected;
                    }
                    if last_outcome.is_none() {
                        last_outcome = Some(v2_corrected);
                    }
                }
                _ => {}
            }
        }
        let legacy_primary = self.verify_extract_at_seed_legacy(img, prefix_bits, seed, mac_key);
        match &legacy_primary {
            CandidateOutcome::Valid(_)
            | CandidateOutcome::Invalid(_)
            | CandidateOutcome::AuthenticationKeyMissing(_)
            | CandidateOutcome::AuthenticationFailed(_)
            | CandidateOutcome::MalformedV3
            | CandidateOutcome::UnsupportedVersion(_)
            | CandidateOutcome::ResourceLimitExceeded => {
                if Self::candidate_seed_matches(&legacy_primary, seed) {
                    return legacy_primary;
                }
                if last_outcome.is_none() {
                    last_outcome = Some(legacy_primary);
                }
            }
            _ => {}
        }
        for pass in 0..5 {
            let offset_seed = seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));
            for redundancy in 1..=10 {
                let v2_outcome = self.verify_extract_at_seed_v2(
                    img,
                    prefix_bits,
                    offset_seed,
                    mac_key,
                    redundancy,
                );
                match &v2_outcome {
                    CandidateOutcome::Valid(_)
                    | CandidateOutcome::Invalid(_)
                    | CandidateOutcome::AuthenticationKeyMissing(_)
                    | CandidateOutcome::AuthenticationFailed(_)
                    | CandidateOutcome::ResourceLimitExceeded => {
                        if Self::candidate_seed_matches(&v2_outcome, seed) {
                            return v2_outcome;
                        }
                        if last_outcome.is_none() {
                            last_outcome = Some(v2_outcome);
                        }
                    }
                    _ => {}
                }
            }

            let legacy_outcome =
                self.verify_extract_at_seed_legacy(img, prefix_bits, offset_seed, mac_key);
            match &legacy_outcome {
                CandidateOutcome::Valid(_)
                | CandidateOutcome::Invalid(_)
                | CandidateOutcome::AuthenticationKeyMissing(_)
                | CandidateOutcome::AuthenticationFailed(_)
                | CandidateOutcome::MalformedV3
                | CandidateOutcome::UnsupportedVersion(_)
                | CandidateOutcome::ResourceLimitExceeded => {
                    if Self::candidate_seed_matches(&legacy_outcome, seed) {
                        return legacy_outcome;
                    }
                    if last_outcome.is_none() {
                        last_outcome = Some(legacy_outcome);
                    }
                }
                _ => {}
            }
        }
        match last_outcome {
            Some(outcome) => outcome,
            None => CandidateOutcome::NotFound,
        }
    }

    pub(crate) fn verify_extract_at_seed_v2(
        &self,
        img: &RgbaImage,
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
        redundancy: usize,
    ) -> CandidateOutcome {
        let mut last_outcome: Option<CandidateOutcome> = None;
        if let Some(prefix) = self.extract_lsb_v2(img, prefix_bits, seed, redundancy) {
            match Self::classify_v3_probe(&prefix, Some(&self.limits)) {
                V3ProbeResult::V3Detected { total_bits, .. } => {
                    if let Some(full) = self.extract_lsb_v2(img, total_bits, seed, redundancy) {
                        if Self::verify_payload_integrity(&full, mac_key) {
                            return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                &full,
                            ));
                        }
                        if last_outcome.is_none() {
                            last_outcome = Some(Self::classify_auth_failure(&full, mac_key));
                        }
                    }
                }
                V3ProbeResult::NotV3 => {
                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        if let Some(payload) = self.extract_lsb_v2(img, ecc_bits, seed, redundancy)
                        {
                            if Self::try_ecc_decode(&payload).is_some() {
                                return CandidateOutcome::Valid(payload);
                            }
                            if Self::verify_payload_integrity(&payload, mac_key) {
                                return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                    &payload,
                                ));
                            }
                            if last_outcome.is_none() {
                                last_outcome = Some(Self::classify_auth_failure(&payload, mac_key));
                            }
                        }
                    }
                }
                V3ProbeResult::MalformedV3 if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::MalformedV3);
                }
                V3ProbeResult::UnsupportedVersion(v) if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::UnsupportedVersion(v));
                }
                V3ProbeResult::ResourceLimitExceeded if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::ResourceLimitExceeded);
                }
                _ => {}
            }
        }
        match last_outcome {
            Some(outcome) => outcome,
            None => CandidateOutcome::NotFound,
        }
    }

    pub(crate) fn verify_extract_at_seed_legacy(
        &self,
        img: &RgbaImage,
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        let mut last_outcome: Option<CandidateOutcome> = None;
        if let Some(prefix) = self.extract_lsb(img, prefix_bits, seed) {
            match Self::classify_v3_probe(&prefix, Some(&self.limits)) {
                V3ProbeResult::V3Detected { total_bits, .. } => {
                    if let Some(full) = self.extract_lsb(img, total_bits, seed) {
                        if Self::verify_payload_integrity(&full, mac_key) {
                            return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                &full,
                            ));
                        }
                        if last_outcome.is_none() {
                            last_outcome = Some(Self::classify_auth_failure(&full, mac_key));
                        }
                    }
                }
                V3ProbeResult::NotV3 => {
                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        if let Some(payload) = self.extract_lsb(img, ecc_bits, seed) {
                            if Self::try_ecc_decode(&payload).is_some() {
                                return CandidateOutcome::Valid(payload);
                            }
                            if Self::verify_payload_integrity(&payload, mac_key) {
                                return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                    &payload,
                                ));
                            }
                            if last_outcome.is_none() {
                                last_outcome = Some(Self::classify_auth_failure(&payload, mac_key));
                            }
                        }
                    }
                }
                V3ProbeResult::MalformedV3 if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::MalformedV3);
                }
                V3ProbeResult::UnsupportedVersion(v) if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::UnsupportedVersion(v));
                }
                V3ProbeResult::ResourceLimitExceeded if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::ResourceLimitExceeded);
                }
                _ => {}
            }
        }
        match last_outcome {
            Some(outcome) => outcome,
            None => CandidateOutcome::NotFound,
        }
    }

    /// Try tiled extraction and verify the embedded seed matches the caller's
    /// expected seed. Returns `true` on success. Used by the crop-resistant
    /// fallback in the verification chain.
    pub(crate) fn try_tiled_extraction_verify(
        &self,
        rgba: &RgbaImage,
        seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> bool {
        let Some(payload) =
            self.extract_lsb_tiled_candidates(rgba, seed, tile_size, max_origins, mac_key)
        else {
            return false;
        };
        Self::verify_embedded_seed_matches(&payload, seed)
    }

    /// Tri-state variant of the non-tiled LSB extraction used by verification.
    pub(crate) fn verify_payload_with_seed_outcome(
        &self,
        img: &DynamicImage,
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        let rgba = img.to_rgba8();

        match self.verify_extract_with_redundancy(&rgba, seed, mac_key) {
            CandidateOutcome::Valid(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    return CandidateOutcome::Valid(payload);
                }
            }
            CandidateOutcome::Invalid(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    return CandidateOutcome::Invalid(payload);
                }
            }
            CandidateOutcome::AuthenticationKeyMissing(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    return CandidateOutcome::AuthenticationKeyMissing(payload);
                }
            }
            CandidateOutcome::AuthenticationFailed(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    return CandidateOutcome::AuthenticationFailed(payload);
                }
            }
            CandidateOutcome::ResourceLimitExceeded => {
                return CandidateOutcome::ResourceLimitExceeded;
            }
            CandidateOutcome::MalformedV3 | CandidateOutcome::UnsupportedVersion(_) => {}
            CandidateOutcome::NotFound => {}
        }

        if let Ok(encoded) = crate::util::image::encode_image(img, image::ImageFormat::Png) {
            if let Some(metadata_seed) =
                RightsMetadataProtector::extract_seed_from_image_with_limits(
                    &encoded,
                    Some(&self.limits),
                )
            {
                if metadata_seed != seed {
                    match self.verify_extract_with_redundancy(&rgba, metadata_seed, mac_key) {
                        CandidateOutcome::Valid(payload) => {
                            if Self::verify_embedded_seed_matches(&payload, seed) {
                                return CandidateOutcome::Valid(payload);
                            }
                        }
                        CandidateOutcome::Invalid(payload) => {
                            if Self::verify_embedded_seed_matches(&payload, seed) {
                                return CandidateOutcome::Invalid(payload);
                            }
                        }
                        CandidateOutcome::AuthenticationKeyMissing(payload) => {
                            if Self::verify_embedded_seed_matches(&payload, seed) {
                                return CandidateOutcome::AuthenticationKeyMissing(payload);
                            }
                        }
                        CandidateOutcome::AuthenticationFailed(payload) => {
                            if Self::verify_embedded_seed_matches(&payload, seed) {
                                return CandidateOutcome::AuthenticationFailed(payload);
                            }
                        }
                        CandidateOutcome::ResourceLimitExceeded => {
                            return CandidateOutcome::ResourceLimitExceeded;
                        }
                        CandidateOutcome::MalformedV3 | CandidateOutcome::UnsupportedVersion(_) => {
                        }
                        CandidateOutcome::NotFound => {}
                    }
                }
            }
        }

        self.verify_tiled_extraction_outcome(
            &rgba,
            seed,
            DEFAULT_TILE_SIZE,
            self.limits.max_tile_extraction_origins() as u32,
            mac_key,
        )
    }

    /// Tri-state variant of `try_tiled_extraction_verify`.
    pub(crate) fn verify_tiled_extraction_outcome(
        &self,
        rgba: &RgbaImage,
        seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        let outcome = self.verify_extract_lsb_tiled(rgba, seed, tile_size, max_origins, mac_key);
        match outcome {
            CandidateOutcome::Valid(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    CandidateOutcome::Valid(payload)
                } else {
                    CandidateOutcome::NotFound
                }
            }
            CandidateOutcome::Invalid(payload) => {
                if Self::verify_embedded_seed_matches(&payload, seed) {
                    CandidateOutcome::Invalid(payload)
                } else {
                    CandidateOutcome::NotFound
                }
            }
            CandidateOutcome::MalformedV3
            | CandidateOutcome::UnsupportedVersion(_)
            | CandidateOutcome::AuthenticationKeyMissing(_)
            | CandidateOutcome::AuthenticationFailed(_)
            | CandidateOutcome::ResourceLimitExceeded => outcome,
            CandidateOutcome::NotFound => CandidateOutcome::NotFound,
        }
    }

    pub(crate) fn dct_candidates(
        &self,
        jpeg_bytes: &[u8],
        seed: u64,
        payload_bits: usize,
    ) -> Vec<Vec<u8>> {
        let payload_len = payload_bits.div_ceil(8);
        (1..=10)
            .filter_map(|redundancy| {
                carrier_support::jpeg_extract(jpeg_bytes, payload_len, seed, redundancy).ok()
            })
            .filter(|payload| payload.len() >= payload_len)
            .collect()
    }

    pub(crate) fn dct_outcome_with_seed(
        &self,
        jpeg_bytes: &[u8],
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        let prefix_bits = V3_PREFIX_BYTES * 8;
        let mut last_outcome = None;
        for prefix in self.dct_candidates(jpeg_bytes, seed, prefix_bits) {
            match Self::classify_v3_probe(&prefix, Some(&self.limits)) {
                V3ProbeResult::V3Detected { total_bits, .. } => {
                    for full in self.dct_candidates(jpeg_bytes, seed, total_bits) {
                        if Self::verify_payload_integrity(&full, mac_key) {
                            return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                &full,
                            ));
                        }
                        if last_outcome.is_none() {
                            last_outcome = Some(Self::classify_auth_failure(&full, mac_key));
                        }
                    }
                }
                V3ProbeResult::NotV3 => {
                    for bits in [ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                        for payload in self.dct_candidates(jpeg_bytes, seed, bits) {
                            if Self::try_ecc_decode(&payload).is_some() {
                                return CandidateOutcome::Valid(payload);
                            }
                            if Self::verify_payload_integrity(&payload, mac_key) {
                                return CandidateOutcome::Valid(Self::truncate_to_actual_payload(
                                    &payload,
                                ));
                            }
                            if last_outcome.is_none() {
                                last_outcome = Some(Self::classify_auth_failure(&payload, mac_key));
                            }
                        }
                    }
                }
                V3ProbeResult::MalformedV3 if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::MalformedV3);
                }
                V3ProbeResult::UnsupportedVersion(version) if last_outcome.is_none() => {
                    last_outcome = Some(CandidateOutcome::UnsupportedVersion(version));
                }
                V3ProbeResult::ResourceLimitExceeded => {
                    if last_outcome.is_none() {
                        last_outcome = Some(CandidateOutcome::ResourceLimitExceeded);
                    }
                }
                V3ProbeResult::InsufficientCapacity => {}
                V3ProbeResult::MalformedV3 | V3ProbeResult::UnsupportedVersion(_) => {
                    if last_outcome.is_none() {
                        last_outcome = Some(CandidateOutcome::MalformedV3);
                    }
                }
            }
        }
        last_outcome.unwrap_or(CandidateOutcome::NotFound)
    }

    pub(crate) fn verify_extract_dct_with_seed(
        &self,
        jpeg_bytes: &[u8],
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        if !jpeg_bytes.starts_with(&[0xFF, 0xD8]) {
            return CandidateOutcome::NotFound;
        }
        let dct_outcome = self.dct_outcome_with_seed(jpeg_bytes, seed, mac_key);
        if matches!(dct_outcome, CandidateOutcome::Valid(_)) {
            return dct_outcome;
        }
        let tiled_outcome = self.verify_extract_f5_tiled(
            jpeg_bytes,
            seed,
            DEFAULT_TILE_SIZE,
            self.limits.max_tile_extraction_origins() as u32,
            mac_key,
        );
        if matches!(tiled_outcome, CandidateOutcome::Valid(_)) {
            return tiled_outcome;
        }
        match (dct_outcome, tiled_outcome) {
            (CandidateOutcome::Invalid(payload), _) | (_, CandidateOutcome::Invalid(payload)) => {
                CandidateOutcome::Invalid(payload)
            }
            (CandidateOutcome::AuthenticationKeyMissing(payload), _)
            | (_, CandidateOutcome::AuthenticationKeyMissing(payload)) => {
                CandidateOutcome::AuthenticationKeyMissing(payload)
            }
            (CandidateOutcome::AuthenticationFailed(payload), _)
            | (_, CandidateOutcome::AuthenticationFailed(payload)) => {
                CandidateOutcome::AuthenticationFailed(payload)
            }
            (CandidateOutcome::MalformedV3, _) | (_, CandidateOutcome::MalformedV3) => {
                CandidateOutcome::MalformedV3
            }
            (CandidateOutcome::UnsupportedVersion(version), _)
            | (_, CandidateOutcome::UnsupportedVersion(version)) => {
                CandidateOutcome::UnsupportedVersion(version)
            }
            (CandidateOutcome::ResourceLimitExceeded, _)
            | (_, CandidateOutcome::ResourceLimitExceeded) => {
                CandidateOutcome::ResourceLimitExceeded
            }
            _ => CandidateOutcome::NotFound,
        }
    }

    pub(crate) fn extract_verified_dct_payload(
        &self,
        jpeg_bytes: &[u8],
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        let seed = carrier_jpeg::extract_seed_hint(jpeg_bytes).ok().flatten()?;
        match self.dct_outcome_with_seed(jpeg_bytes, seed, mac_key) {
            CandidateOutcome::Valid(payload) => Some(payload),
            _ => self.extract_f5_tiled_candidates(
                jpeg_bytes,
                seed,
                DEFAULT_TILE_SIZE,
                self.limits.max_tile_extraction_origins() as u32,
                mac_key,
            ),
        }
    }

    pub(crate) fn verify_extract_verified_dct(
        &self,
        jpeg_bytes: &[u8],
        mac_key: &[u8],
    ) -> CandidateOutcome {
        let Some(seed) = carrier_jpeg::extract_seed_hint(jpeg_bytes).ok().flatten() else {
            return CandidateOutcome::NotFound;
        };
        self.verify_extract_dct_with_seed(jpeg_bytes, seed, mac_key)
    }

    pub(crate) fn candidate_seed_matches(outcome: &CandidateOutcome, expected_seed: u64) -> bool {
        match outcome {
            CandidateOutcome::Valid(payload)
            | CandidateOutcome::Invalid(payload)
            | CandidateOutcome::AuthenticationKeyMissing(payload)
            | CandidateOutcome::AuthenticationFailed(payload) => {
                Self::verify_embedded_seed_matches(payload, expected_seed)
            }
            _ => false,
        }
    }

    pub(crate) fn extract_embedded_seed(header: &[u8]) -> Option<u64> {
        if header.len() < 10 {
            return None;
        }
        // V3: check magic bytes and read seed from v3 offset
        if header.len() >= 19 && header[0] == V3_MAGIC[0] && header[1] == V3_MAGIC[1] {
            return Some(u64::from_le_bytes([
                header[11], header[12], header[13], header[14], header[15], header[16], header[17],
                header[18],
            ]));
        }
        // V1/V2: seed at bytes 2-9
        Some(u64::from_le_bytes([
            header[2], header[3], header[4], header[5], header[6], header[7], header[8], header[9],
        ]))
    }

    pub(crate) fn extract_lsb(
        &self,
        img: &RgbaImage,
        expected_bits: usize,
        seed: u64,
    ) -> Option<Vec<u8>> {
        carrier_support::legacy_lsb_extract(img, expected_bits, seed)
    }

    /// Extract LSBs from a specific bit range `[offset, offset + count)`.
    ///
    /// Uses the same permutation as [`extract_lsb`] (same seed, same
    /// `expected_bits`), but only decodes bits in the requested range. The
    /// caller must ensure `offset + count <= expected_bits` and that the
    /// image has sufficient pixels for the full `expected_bits` spread.
    #[allow(dead_code)]
    pub(crate) fn extract_lsb_range(
        &self,
        img: &RgbaImage,
        expected_bits: usize,
        offset: usize,
        count: usize,
        seed: u64,
    ) -> Option<Vec<u8>> {
        carrier_support::legacy_lsb_extract_range(img, expected_bits, offset, count, seed)
    }

    /// Extract payload using the corrected V2 carrier scheme.
    ///
    /// Mirrors [`embed_lsb_v2`] — uses the same permutation, slot mapping,
    /// and majority-vote logic over `width * height * 3` RGB carrier slots.
    /// Each bit reads `STEGO_SPREAD_FACTOR * redundancy` slots via majority vote.
    pub(crate) fn extract_lsb_v2(
        &self,
        img: &RgbaImage,
        expected_bits: usize,
        seed: u64,
        redundancy: usize,
    ) -> Option<Vec<u8>> {
        carrier_support::corrected_lsb_extract(img, expected_bits, seed, redundancy)
    }

    pub(crate) fn extract_seed_lsb_fallback(img: &RgbaImage) -> Option<u64> {
        carrier_support::seed_fallback_extract(img)
    }

    /// Extract payload from a sub-image trying V2 then legacy scheme.
    ///
    /// Used by tiled extraction where each tile is an independent sub-image.
    /// Tries V2 (corrected carrier) first, then falls back to legacy.
    pub(crate) fn extract_from_sub_image(
        &self,
        sub: &RgbaImage,
        _expected_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        let prefix_bits = 6 * 8;

        if let Some(prefix) = self.extract_lsb_v2(sub, prefix_bits, seed, 1) {
            if let Some(result) =
                self.probe_payload_from_prefix_v2(sub, &prefix, prefix_bits, seed, mac_key)
            {
                return Some(result);
            }
        }

        if let Some(prefix) = self.extract_lsb(sub, prefix_bits, seed) {
            if let Some(result) =
                self.probe_payload_from_prefix_legacy(sub, &prefix, prefix_bits, seed, mac_key)
            {
                return Some(result);
            }
        }

        None
    }

    pub(crate) fn probe_payload_from_prefix_v2(
        &self,
        img: &RgbaImage,
        prefix: &[u8],
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        match Self::classify_v3_prefix(prefix, Some(&self.limits)) {
            V3PrefixResult::Detected {
                header_length,
                total_length,
            } => {
                let header_bits = header_length * 8;
                let header_bytes = if header_bits <= prefix_bits {
                    prefix[..header_length].to_vec()
                } else {
                    self.extract_lsb_v2(img, header_bits, seed, 1)?
                };
                if Self::validate_v3_header(&header_bytes, Some(&self.limits)).is_err() {
                    return None;
                }
                let total_bits = total_length * 8;
                let full = if total_bits <= prefix_bits {
                    prefix[..total_length].to_vec()
                } else {
                    self.extract_lsb_v2(img, total_bits, seed, 1)?
                };
                if Self::verify_payload_integrity(&full, mac_key) {
                    return Some(Self::truncate_to_actual_payload(&full));
                }
            }
            V3PrefixResult::NotV3 => {
                for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                    let payload = if ecc_bits <= prefix_bits {
                        prefix.to_vec()
                    } else {
                        match self.extract_lsb_v2(img, ecc_bits, seed, 1) {
                            Some(p) => p,
                            None => continue,
                        }
                    };
                    if Self::try_ecc_decode(&payload).is_some() {
                        return Some(payload);
                    }
                }
            }
            V3PrefixResult::Malformed(_)
            | V3PrefixResult::UnsupportedVersion(_)
            | V3PrefixResult::ResourceLimitExceeded => {}
        }
        None
    }

    pub(crate) fn probe_payload_from_prefix_legacy(
        &self,
        img: &RgbaImage,
        prefix: &[u8],
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        match Self::classify_v3_prefix(prefix, Some(&self.limits)) {
            V3PrefixResult::Detected {
                header_length,
                total_length,
            } => {
                let header_bits = header_length * 8;
                let header_bytes = if header_bits <= prefix_bits {
                    prefix[..header_length].to_vec()
                } else {
                    self.extract_lsb(img, header_bits, seed)?
                };
                if Self::validate_v3_header(&header_bytes, Some(&self.limits)).is_err() {
                    return None;
                }
                let total_bits = total_length * 8;
                let full = if total_bits <= prefix_bits {
                    prefix[..total_length].to_vec()
                } else {
                    self.extract_lsb(img, total_bits, seed)?
                };
                if Self::verify_payload_integrity(&full, mac_key) {
                    return Some(Self::truncate_to_actual_payload(&full));
                }
            }
            V3PrefixResult::NotV3 => {
                for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                    let payload = if ecc_bits <= prefix_bits {
                        prefix.to_vec()
                    } else {
                        match self.extract_lsb(img, ecc_bits, seed) {
                            Some(p) => p,
                            None => continue,
                        }
                    };
                    if Self::try_ecc_decode(&payload).is_some() {
                        return Some(payload);
                    }
                }
            }
            V3PrefixResult::Malformed(_)
            | V3PrefixResult::UnsupportedVersion(_)
            | V3PrefixResult::ResourceLimitExceeded => {}
        }
        None
    }

    /// Verification-path variant of probe_payload_from_prefix that returns a tri-state.
    pub(crate) fn verify_probe_payload_from_prefix(
        &self,
        img: &RgbaImage,
        prefix: &[u8],
        prefix_bits: usize,
        seed: u64,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        match Self::classify_v3_prefix(prefix, Some(&self.limits)) {
            V3PrefixResult::Detected {
                header_length,
                total_length,
            } => {
                let header_bits = header_length * 8;
                let header_bytes = if header_bits <= prefix_bits {
                    prefix[..header_length].to_vec()
                } else if let Some(h) = self.extract_lsb_v2(img, header_bits, seed, 1) {
                    h
                } else {
                    return CandidateOutcome::NotFound;
                };
                if Self::validate_v3_header(&header_bytes, Some(&self.limits)).is_err() {
                    return CandidateOutcome::NotFound;
                }
                let total_bits = total_length * 8;
                let full = if total_bits <= prefix_bits {
                    prefix[..total_length].to_vec()
                } else if let Some(f) = self.extract_lsb_v2(img, total_bits, seed, 1) {
                    f
                } else {
                    return CandidateOutcome::NotFound;
                };
                if Self::verify_payload_integrity(&full, mac_key) {
                    return CandidateOutcome::Valid(Self::truncate_to_actual_payload(&full));
                }
                Self::classify_auth_failure(&full, mac_key)
            }
            V3PrefixResult::NotV3 => {
                for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                    let payload = if ecc_bits <= prefix_bits {
                        prefix.to_vec()
                    } else if let Some(p) = self.extract_lsb_v2(img, ecc_bits, seed, 1) {
                        p
                    } else {
                        continue;
                    };
                    if Self::try_ecc_decode(&payload).is_some() {
                        return CandidateOutcome::Valid(payload);
                    }
                    if Self::verify_payload_integrity(&payload, mac_key) {
                        return CandidateOutcome::Valid(Self::truncate_to_actual_payload(&payload));
                    }
                    return Self::classify_auth_failure(&payload, mac_key);
                }
                CandidateOutcome::NotFound
            }
            V3PrefixResult::Malformed(_) => CandidateOutcome::MalformedV3,
            V3PrefixResult::UnsupportedVersion(v) => CandidateOutcome::UnsupportedVersion(v),
            V3PrefixResult::ResourceLimitExceeded => CandidateOutcome::ResourceLimitExceeded,
        }
    }

    /// Extract payload from tiled candidates, trying V2 then legacy per tile.
    pub(crate) fn extract_lsb_tiled_candidates(
        &self,
        img: &RgbaImage,
        master_seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> Option<Vec<u8>> {
        if tile_size == 0 {
            return None;
        }
        let (width, height) = img.dimensions();
        if width < tile_size || height < tile_size {
            return None;
        }

        let stride = (tile_size / 2).max(1);
        let mut origins: Vec<(u32, u32)> = Vec::new();
        let mut y = 0u32;
        while y + tile_size <= height {
            let mut x = 0u32;
            while x + tile_size <= width {
                origins.push((x, y));
                if origins.len() as u32 >= max_origins {
                    break;
                }
                x = x.saturating_add(stride);
            }
            if origins.len() as u32 >= max_origins {
                break;
            }
            y = y.saturating_add(stride);
        }

        let max_grid = 16u32;

        for &(x0, y0) in &origins {
            let sub = Self::crop_rgba(img, x0, y0, tile_size, tile_size);
            let base_x = x0 / tile_size;
            let base_y = y0 / tile_size;
            for dy in 0..=2u32 {
                if base_y + dy >= max_grid {
                    break;
                }
                for dx in 0..=2u32 {
                    if base_x + dx >= max_grid {
                        break;
                    }
                    let local_seed = tile_seed(master_seed, base_x + dx, base_y + dy);

                    for pass in 0..5 {
                        let offset_seed =
                            local_seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));

                        if let Some(payload) = self.extract_from_sub_image(
                            &sub,
                            V3_PREFIX_BYTES * 8,
                            offset_seed,
                            mac_key,
                        ) {
                            return Some(payload);
                        }
                    }
                }
            }
        }
        None
    }

    /// Verification-path variant of `extract_lsb_tiled_candidates` that
    /// returns a tri-state. See [`Self::verify_extract_with_redundancy`].
    pub(crate) fn verify_extract_lsb_tiled(
        &self,
        img: &RgbaImage,
        master_seed: u64,
        tile_size: u32,
        max_origins: u32,
        mac_key: &[u8],
    ) -> CandidateOutcome {
        if tile_size == 0 {
            return CandidateOutcome::NotFound;
        }
        let (width, height) = img.dimensions();
        if width < tile_size || height < tile_size {
            return CandidateOutcome::NotFound;
        }

        let stride = (tile_size / 2).max(1);
        let mut origins: Vec<(u32, u32)> = Vec::new();
        let mut y = 0u32;
        while y + tile_size <= height {
            let mut x = 0u32;
            while x + tile_size <= width {
                origins.push((x, y));
                if origins.len() as u32 >= max_origins {
                    break;
                }
                x = x.saturating_add(stride);
            }
            if origins.len() as u32 >= max_origins {
                break;
            }
            y = y.saturating_add(stride);
        }
        let max_grid = 16u32;
        let mut last_outcome: Option<CandidateOutcome> = None;

        for &(x0, y0) in &origins {
            let sub = Self::crop_rgba(img, x0, y0, tile_size, tile_size);
            let base_x = x0 / tile_size;
            let base_y = y0 / tile_size;
            for dy in 0..=2u32 {
                if base_y + dy >= max_grid {
                    break;
                }
                for dx in 0..=2u32 {
                    if base_x + dx >= max_grid {
                        break;
                    }
                    let local_seed = tile_seed(master_seed, base_x + dx, base_y + dy);

                    for pass in 0..5 {
                        let offset_seed =
                            local_seed.wrapping_mul(STEGO_OFFSET_SEED_1.wrapping_add(pass as u64));
                        let prefix_bits = V3_PREFIX_BYTES * 8;

                        if let Some(prefix) = self.extract_lsb_v2(&sub, prefix_bits, offset_seed, 1)
                        {
                            let outcome = self.verify_probe_payload_from_prefix(
                                &sub,
                                &prefix,
                                prefix_bits,
                                offset_seed,
                                mac_key,
                            );
                            match &outcome {
                                CandidateOutcome::Valid(_) => return outcome,
                                CandidateOutcome::Invalid(_)
                                | CandidateOutcome::AuthenticationKeyMissing(_)
                                | CandidateOutcome::AuthenticationFailed(_) => {
                                    if last_outcome.is_none() {
                                        last_outcome = Some(outcome);
                                    }
                                }
                                CandidateOutcome::MalformedV3 if last_outcome.is_none() => {
                                    last_outcome = Some(outcome);
                                }
                                CandidateOutcome::UnsupportedVersion(_)
                                    if last_outcome.is_none() =>
                                {
                                    last_outcome = Some(outcome);
                                }
                                CandidateOutcome::ResourceLimitExceeded
                                    if last_outcome.is_none() =>
                                {
                                    last_outcome = Some(outcome);
                                }
                                _ => {}
                            }
                        }

                        if let Some(prefix) = self.extract_lsb(&sub, prefix_bits, offset_seed) {
                            match Self::classify_v3_prefix(&prefix, Some(&self.limits)) {
                                V3PrefixResult::Detected {
                                    header_length,
                                    total_length,
                                } => {
                                    let header_bits = header_length * 8;
                                    let header_bytes = if header_bits <= prefix_bits {
                                        prefix[..header_length].to_vec()
                                    } else if let Some(h) =
                                        self.extract_lsb(&sub, header_bits, offset_seed)
                                    {
                                        h
                                    } else {
                                        continue;
                                    };
                                    if Self::validate_v3_header(&header_bytes, Some(&self.limits))
                                        .is_err()
                                    {
                                        continue;
                                    }
                                    let total_bits = total_length * 8;
                                    let full = if total_bits <= prefix_bits {
                                        prefix[..total_length].to_vec()
                                    } else if let Some(f) =
                                        self.extract_lsb(&sub, total_bits, offset_seed)
                                    {
                                        f
                                    } else {
                                        continue;
                                    };
                                    if Self::verify_payload_integrity(&full, mac_key) {
                                        return CandidateOutcome::Valid(
                                            Self::truncate_to_actual_payload(&full),
                                        );
                                    }
                                    if last_outcome.is_none() {
                                        last_outcome =
                                            Some(Self::classify_auth_failure(&full, mac_key));
                                    }
                                }
                                V3PrefixResult::Malformed(_) => {
                                    if last_outcome.is_none() {
                                        last_outcome = Some(CandidateOutcome::MalformedV3);
                                    }
                                }
                                V3PrefixResult::UnsupportedVersion(v) => {
                                    if last_outcome.is_none() {
                                        last_outcome =
                                            Some(CandidateOutcome::UnsupportedVersion(v));
                                    }
                                }
                                V3PrefixResult::ResourceLimitExceeded => {
                                    if last_outcome.is_none() {
                                        last_outcome =
                                            Some(CandidateOutcome::ResourceLimitExceeded);
                                    }
                                }
                                V3PrefixResult::NotV3 => {
                                    for &ecc_bits in &[ECC_PAYLOAD_BITS_V2, ECC_PAYLOAD_BITS] {
                                        let payload = if ecc_bits <= prefix_bits {
                                            prefix.clone()
                                        } else {
                                            match self.extract_lsb(&sub, ecc_bits, offset_seed) {
                                                Some(p) => p,
                                                None => continue,
                                            }
                                        };
                                        if Self::try_ecc_decode(&payload).is_some() {
                                            return CandidateOutcome::Valid(payload);
                                        }
                                        if last_outcome.is_none() {
                                            last_outcome = Some(Self::classify_auth_failure(
                                                &payload, mac_key,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        match last_outcome {
            Some(outcome) => outcome,
            None => CandidateOutcome::NotFound,
        }
    }

    pub(crate) fn crop_rgba(src: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
        carrier_support::crop_image_region(src, x, y, w, h)
    }

    /// Extract and parse a protected payload directly from encoded image bytes.
    pub fn extract_payload_from_bytes_with_key(
        &self,
        img_bytes: &[u8],
        mac_key: &[u8],
    ) -> Option<StegoPayload> {
        let metadata_seed = RightsMetadataProtector::extract_seed_from_image_with_limits(
            img_bytes,
            Some(&self.limits),
        );

        // JPEG: try DCT extraction first (avoids pixel decode)
        if img_bytes.starts_with(&[0xFF, 0xD8]) {
            if let Some(payload_bytes) = self.extract_verified_dct_payload(img_bytes, mac_key) {
                if !self.payload_within_limits(&payload_bytes) {
                    return None;
                }
                if let Some(decoded) = Self::try_ecc_decode(&payload_bytes) {
                    if let Some(payload) = Self::parse_stego_payload(&decoded) {
                        return Some(payload);
                    }
                }
                if Self::verify_payload_integrity(&payload_bytes, mac_key) {
                    return Self::parse_stego_payload(&payload_bytes);
                }
            }

            // Tiled F5 fallback for JPEG
            if let Some(metadata_seed) = metadata_seed {
                if let Some(payload_bytes) = self.extract_f5_tiled_candidates(
                    img_bytes,
                    metadata_seed,
                    DEFAULT_TILE_SIZE,
                    self.limits.max_tile_extraction_origins() as u32,
                    mac_key,
                ) {
                    if !self.payload_within_limits(&payload_bytes) {
                        return None;
                    }
                    if let Some(decoded) = Self::try_ecc_decode(&payload_bytes) {
                        if let Some(payload) = Self::parse_stego_payload(&decoded) {
                            return Some(payload);
                        }
                    }
                    if Self::verify_payload_integrity(&payload_bytes, mac_key) {
                        return Self::parse_stego_payload(&payload_bytes);
                    }
                }
            }

            // JPEG stego is coefficient-based in this crate. If DCT extraction
            // failed, decoding to pixels and trying LSB extraction is wasted
            // work for production byte-level verification.
            return None;
        }

        // Try metadata seed extraction from bytes (works for PNG, JPEG, WebP)
        if let Some(metadata_seed) = metadata_seed {
            if let Ok(img) = image::load_from_memory(img_bytes) {
                if let Some(payload) =
                    self.extract_payload_with_seed_and_key(&img, metadata_seed, mac_key)
                {
                    return Some(payload);
                }
            }
        }

        // LSB fallback seed
        if let Ok(img) = image::load_from_memory(img_bytes) {
            let rgba = img.to_rgba8();
            if let Some(fallback_seed) = Self::extract_seed_lsb_fallback(&rgba) {
                if let Some(payload) =
                    self.extract_payload_with_seed_and_key(&img, fallback_seed, mac_key)
                {
                    return Some(payload);
                }
            }
        }

        // Tiled LSB fallback
        #[cfg(feature = "test-seeds")]
        if let Ok(img) = image::load_from_memory(img_bytes) {
            let rgba = img.to_rgba8();
            for &seed in FALLBACK_SEEDS
                .iter()
                .take(self.limits.max_verification_seeds())
            {
                if let Some(payload) = self.extract_lsb_tiled_candidates(
                    &rgba,
                    seed,
                    DEFAULT_TILE_SIZE,
                    self.limits.max_tile_extraction_origins() as u32,
                    mac_key,
                ) {
                    if let Some(decoded) = Self::try_ecc_decode(&payload) {
                        if let Some(payload) = Self::parse_stego_payload(&decoded) {
                            return Some(payload);
                        }
                    }
                    if Self::verify_payload_integrity(&payload, mac_key) {
                        return Self::parse_stego_payload(&payload);
                    }
                }
            }
        }

        None
    }

    /// Extract a protected payload using a known seed and MAC key.
    pub fn extract_payload_with_seed_and_key(
        &self,
        img: &DynamicImage,
        seed: u64,
        mac_key: &[u8],
    ) -> Option<StegoPayload> {
        let rgba = img.to_rgba8();
        if let Some(payload) = self.extract_with_redundancy(&rgba, seed, mac_key) {
            if !self.payload_within_limits(&payload) {
                return None;
            }
            if let Some(decoded) = Self::try_ecc_decode(&payload) {
                let mut sp = Self::parse_stego_payload(&decoded)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
            if Self::verify_payload_integrity(&payload, mac_key) {
                let mut sp = Self::parse_stego_payload(&payload)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
        }

        // Crop-resistant fallback: try tiled extraction. This recovers the
        // payload from any crop that contains at least one intact tile, even
        // when the non-tiled path's pixel positions are completely scrambled
        // by the crop offset.
        if let Some(payload) = self.extract_lsb_tiled_candidates(
            &rgba,
            seed,
            DEFAULT_TILE_SIZE,
            self.limits.max_tile_extraction_origins() as u32,
            mac_key,
        ) {
            if let Some(decoded) = Self::try_ecc_decode(&payload) {
                let mut sp = Self::parse_stego_payload(&decoded)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
            if Self::verify_payload_integrity(&payload, mac_key) {
                let mut sp = Self::parse_stego_payload(&payload)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
        }

        None
    }

    /// Extract a protected payload using a known seed in checksum mode.
    pub fn extract_payload_with_seed(&self, img: &DynamicImage, seed: u64) -> Option<StegoPayload> {
        let rgba = img.to_rgba8();
        if let Some(payload) = self.extract_with_redundancy(&rgba, seed, &[]) {
            if let Some(decoded) = Self::try_ecc_decode(&payload) {
                let mut sp = Self::parse_stego_payload(&decoded)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
            let mut sp = Self::parse_stego_payload(&payload)?;
            sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
            return Some(sp);
        }

        // Crop-resistant fallback: tiled extraction.
        if let Some(payload) = self.extract_lsb_tiled_candidates(
            &rgba,
            seed,
            DEFAULT_TILE_SIZE,
            self.limits.max_tile_extraction_origins() as u32,
            &[],
        ) {
            if let Some(decoded) = Self::try_ecc_decode(&payload) {
                let mut sp = Self::parse_stego_payload(&decoded)?;
                sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
                return Some(sp);
            }
            let mut sp = Self::parse_stego_payload(&payload)?;
            sp.raw_payload = Some(Self::truncate_to_actual_payload(&payload));
            return Some(sp);
        }

        None
    }

    /// Extract the steganographic payload from an image.
    pub fn extract_payload(&self, img: &DynamicImage) -> Option<StegoPayload> {
        self.extract_payload_with_key(img, &[])
    }

    /// Extract the steganographic payload and verify it with a MAC key.
    pub fn extract_payload_with_key(
        &self,
        img: &DynamicImage,
        mac_key: &[u8],
    ) -> Option<StegoPayload> {
        // Try LSB fallback seed (fixed-position LSB pattern)
        let rgba = img.to_rgba8();
        if let Some(fallback_seed) = Self::extract_seed_lsb_fallback(&rgba) {
            if let Some(payload) =
                self.extract_payload_with_seed_and_key(img, fallback_seed, mac_key)
            {
                return Some(payload);
            }
        }

        // Fallback: try common seeds when metadata is unavailable.
        #[cfg(feature = "test-seeds")]
        for &seed in FALLBACK_SEEDS
            .iter()
            .take(self.limits.max_verification_seeds())
        {
            if let Some(payload) = self.extract_payload_with_seed_and_key(img, seed, mac_key) {
                return Some(payload);
            }
        }

        None
    }
}
